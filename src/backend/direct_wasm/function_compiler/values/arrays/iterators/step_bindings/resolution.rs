use super::*;

impl<'a> FunctionCompiler<'a> {
    fn expression_may_alias_iterator_step_binding(expression: &Expression) -> bool {
        match expression {
            Expression::Identifier(name) => {
                name.starts_with("__ayy_array_step_") || name.starts_with("__ayy_for_of_step_")
            }
            Expression::Member { object, .. } => {
                Self::expression_may_alias_iterator_step_binding(object)
            }
            Expression::Conditional {
                then_expression,
                else_expression,
                ..
            } => {
                Self::expression_may_alias_iterator_step_binding(then_expression)
                    || Self::expression_may_alias_iterator_step_binding(else_expression)
            }
            Expression::Assign { value, .. }
            | Expression::AssignMember { value, .. }
            | Expression::AssignSuperMember { value, .. }
            | Expression::Await(value) => Self::expression_may_alias_iterator_step_binding(value),
            Expression::Sequence(expressions) => expressions
                .last()
                .is_some_and(Self::expression_may_alias_iterator_step_binding),
            _ => false,
        }
    }

    fn identifier_value_may_alias_iterator_step_binding(&self, name: &str) -> bool {
        if name.starts_with("__ayy_array_step_") || name.starts_with("__ayy_for_of_step_") {
            return true;
        }
        let resolved_name = self
            .resolve_current_local_binding(name)
            .map(|(resolved_name, _)| resolved_name);
        let candidates = [Some(name), resolved_name.as_deref()];
        candidates.into_iter().flatten().any(|candidate| {
            self.state
                .speculation
                .static_semantics
                .local_value_binding(candidate)
                .or_else(|| self.global_value_binding(candidate))
                .is_some_and(Self::expression_may_alias_iterator_step_binding)
        })
    }

    fn expression_references_iterator_step_temp(expression: &Expression) -> bool {
        let mut referenced_names = HashSet::new();
        collect_referenced_binding_names_from_expression(expression, &mut referenced_names);
        referenced_names.iter().any(|name| {
            name.starts_with("__ayy_array_step_")
                || name.starts_with("__ayy_for_of_step_")
                || name.starts_with("__ayy_array_iter_value_")
                || name.starts_with("__ayy_for_of_iter_value_")
                || name.starts_with("__ayy_array_iter_done_")
                || name.starts_with("__ayy_for_of_iter_done_")
                || name.starts_with("__ayy_binding_value_")
        })
    }

    fn materialize_non_iterator_step_expression(
        &self,
        expression: &Expression,
    ) -> Option<Expression> {
        (!Self::expression_references_iterator_step_temp(expression))
            .then(|| self.materialize_static_expression(expression))
    }

    fn resolve_static_iterator_step_condition_operand_value(
        &self,
        expression: &Expression,
    ) -> Option<Expression> {
        if let Expression::Identifier(name) = expression
            && (name.starts_with("__ayy_array_iter_done_")
                || name.starts_with("__ayy_for_of_iter_done_"))
            && let Some(value) = self
                .state
                .speculation
                .static_semantics
                .local_value_binding(name)
                .or_else(|| self.global_value_binding(name))
            && !static_expression_matches(value, expression)
        {
            return self.resolve_static_iterator_step_condition_operand_value(value);
        }
        self.resolve_static_iterator_step_member_value(expression, "done")
            .or_else(|| self.resolve_static_iterator_step_assignment_value(expression))
            .or_else(|| self.materialize_non_iterator_step_expression(expression))
    }

    fn resolve_static_iterator_step_member_value(
        &self,
        expression: &Expression,
        expected_property_name: &str,
    ) -> Option<Expression> {
        let Expression::Member { object, property } = expression else {
            return None;
        };
        let Expression::String(property_name) = property.as_ref() else {
            return None;
        };
        if property_name != expected_property_name {
            return None;
        }
        let IteratorStepBinding::Runtime {
            static_done,
            static_value,
            ..
        } = self.resolve_iterator_step_binding_from_expression(object)?;
        match expected_property_name {
            "done" => static_done
                .or_else(|| static_value.as_ref().map(|_| false))
                .map(Expression::Bool),
            "value" => static_value.and_then(|value| {
                (!self.iterator_step_static_value_requires_runtime_read(&value))
                    .then(|| self.materialize_static_expression(&value))
            }),
            _ => None,
        }
    }

    pub(in crate::backend::direct_wasm) fn resolve_static_iterator_step_condition_value(
        &self,
        condition: &Expression,
    ) -> Option<bool> {
        let Expression::Binary { op, left, right } = condition else {
            return match self.resolve_static_iterator_step_condition_operand_value(condition)? {
                Expression::Bool(value) => Some(value),
                _ => None,
            };
        };
        if matches!(op, BinaryOp::LogicalOr | BinaryOp::LogicalAnd) {
            let left = self.resolve_static_iterator_step_condition_value(left)?;
            return match op {
                BinaryOp::LogicalOr if left => Some(true),
                BinaryOp::LogicalOr => self.resolve_static_iterator_step_condition_value(right),
                BinaryOp::LogicalAnd if !left => Some(false),
                BinaryOp::LogicalAnd => self.resolve_static_iterator_step_condition_value(right),
                _ => unreachable!("matched logical op above"),
            };
        }
        let left = self.resolve_static_iterator_step_condition_operand_value(left)?;
        let right = self.resolve_static_iterator_step_condition_operand_value(right)?;
        let loosely_equal_nullish = matches!(
            (&left, &right),
            (Expression::Null, Expression::Undefined) | (Expression::Undefined, Expression::Null)
        );
        let equal = static_expression_matches(&left, &right);
        match op {
            BinaryOp::Equal => Some(equal),
            BinaryOp::NotEqual => Some(!equal),
            BinaryOp::LooseEqual => Some(equal || loosely_equal_nullish),
            BinaryOp::LooseNotEqual => Some(!(equal || loosely_equal_nullish)),
            _ => None,
        }
    }

    pub(in crate::backend::direct_wasm) fn resolve_static_iterator_step_assignment_value(
        &self,
        value: &Expression,
    ) -> Option<Expression> {
        match value {
            Expression::Member { object, property } => self
                .resolve_static_iterator_step_member_value(value, "value")
                .or_else(|| {
                    let object = self.resolve_static_iterator_step_assignment_value(object)?;
                    Some(self.materialize_static_expression(&Expression::Member {
                        object: Box::new(object),
                        property: property.clone(),
                    }))
                }),
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                if self.resolve_static_iterator_step_condition_value(condition)? {
                    self.resolve_static_iterator_step_assignment_value(then_expression)
                        .or_else(|| self.materialize_non_iterator_step_expression(then_expression))
                } else {
                    self.materialize_non_iterator_step_expression(else_expression)
                }
            }
            _ => None,
        }
    }

    pub(in crate::backend::direct_wasm) fn resolve_iterator_step_binding_from_expression(
        &self,
        expression: &Expression,
    ) -> Option<IteratorStepBinding> {
        let trace = std::env::var_os("AYY_TRACE_ITERATOR_STEP").is_some();
        if let Expression::Identifier(name) = expression {
            if let Some(binding) = self
                .state
                .speculation
                .static_semantics
                .local_iterator_step_binding(name)
            {
                if trace {
                    eprintln!("iterator_step_resolve:direct_hit name={name}");
                }
                return Some(binding.clone());
            }
            if let Some((resolved_name, _)) = self.resolve_current_local_binding(name)
                && let Some(binding) = self
                    .state
                    .speculation
                    .static_semantics
                    .local_iterator_step_binding(&resolved_name)
            {
                if trace {
                    eprintln!(
                        "iterator_step_resolve:local_hit name={name} resolved={resolved_name}"
                    );
                }
                return Some(binding.clone());
            }
            if !self.identifier_value_may_alias_iterator_step_binding(name) {
                if trace {
                    eprintln!("iterator_step_resolve:no_alias expression={expression:?}");
                }
                return None;
            }
        }
        if matches!(expression, Expression::GetIterator(_)) {
            if trace {
                eprintln!("iterator_step_resolve:no_alias expression={expression:?}");
            }
            return None;
        }
        if !matches!(expression, Expression::Identifier(_))
            && !Self::expression_may_alias_iterator_step_binding(expression)
        {
            if trace {
                eprintln!("iterator_step_resolve:no_alias expression={expression:?}");
            }
            return None;
        }
        let Expression::Identifier(name) = self.resolve_bound_alias_expression(expression)? else {
            if trace {
                eprintln!("iterator_step_resolve:no_alias expression={expression:?}");
            }
            return None;
        };
        let binding = self
            .state
            .speculation
            .static_semantics
            .local_iterator_step_binding(&name)
            .cloned();
        if trace {
            eprintln!(
                "iterator_step_resolve:alias name={name} hit={}",
                binding.is_some()
            );
        }
        binding
    }
}
