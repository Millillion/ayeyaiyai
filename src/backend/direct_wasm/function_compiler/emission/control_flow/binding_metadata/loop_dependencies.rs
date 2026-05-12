use super::*;

impl<'a> FunctionCompiler<'a> {
    fn resolve_static_loop_dependent_member_primitive(
        &self,
        object: &Expression,
        property: &Expression,
    ) -> Option<Expression> {
        let Expression::String(property_name) = property else {
            return None;
        };
        let step_binding = self.resolve_iterator_step_binding_from_expression(object)?;
        match (property_name.as_str(), step_binding) {
            (
                "done",
                IteratorStepBinding::Runtime {
                    static_done: Some(done),
                    ..
                },
            ) => Some(Expression::Bool(done)),
            (
                "value",
                IteratorStepBinding::Runtime {
                    static_value: Some(value),
                    ..
                },
            ) => Some(value),
            _ => None,
        }
    }

    fn resolve_static_loop_dependent_primitive(
        &self,
        expression: &Expression,
    ) -> Option<Expression> {
        match expression {
            Expression::Number(_)
            | Expression::BigInt(_)
            | Expression::String(_)
            | Expression::Bool(_)
            | Expression::Null
            | Expression::Undefined => Some(expression.clone()),
            Expression::Identifier(name)
                if name == "undefined" && self.is_unshadowed_builtin_identifier(name) =>
            {
                Some(Expression::Undefined)
            }
            Expression::Identifier(name) => self
                .state
                .speculation
                .static_semantics
                .local_value_binding(name)
                .or_else(|| self.global_value_binding(name))
                .filter(|value| !static_expression_matches(value, expression))
                .and_then(|value| self.resolve_static_loop_dependent_primitive(value)),
            Expression::Assign { value, .. }
            | Expression::AssignMember { value, .. }
            | Expression::AssignSuperMember { value, .. } => {
                self.resolve_static_loop_dependent_primitive(value)
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                let branch = if self.resolve_static_loop_dependent_condition_value(condition)? {
                    then_expression
                } else {
                    else_expression
                };
                self.resolve_static_loop_dependent_primitive(branch)
            }
            Expression::Member { object, property } => self
                .resolve_static_loop_dependent_member_primitive(object, property)
                .and_then(|value| self.resolve_static_loop_dependent_primitive(&value)),
            _ => None,
        }
    }

    fn resolve_static_loop_dependent_condition_value(
        &self,
        condition: &Expression,
    ) -> Option<bool> {
        let Expression::Binary { op, left, right } = condition else {
            return None;
        };
        let left = self.resolve_static_loop_dependent_primitive(left)?;
        let right = self.resolve_static_loop_dependent_primitive(right)?;
        let is_loose = matches!(op, BinaryOp::LooseEqual | BinaryOp::LooseNotEqual);
        let is_not_equal = matches!(op, BinaryOp::NotEqual | BinaryOp::LooseNotEqual);
        let equal = match (&left, &right) {
            (Expression::Bool(left), Expression::Bool(right)) => Some(left == right),
            (Expression::Number(left), Expression::Number(right)) => Some(left == right),
            (Expression::String(left), Expression::String(right)) => Some(left == right),
            (Expression::Null, Expression::Null)
            | (Expression::Undefined, Expression::Undefined) => Some(true),
            (Expression::Null, Expression::Undefined)
            | (Expression::Undefined, Expression::Null)
                if is_loose =>
            {
                Some(true)
            }
            (_, _) if !is_loose => Some(false),
            _ => None,
        }?;
        Some(equal ^ is_not_equal)
    }

    fn resolve_active_loop_indexed_array_member_primitive(
        &mut self,
        object: &Expression,
        property: &Expression,
        environment: &HashMap<String, i64>,
    ) -> Option<Expression> {
        let index = self.active_loop_integer_value(property, environment)?;
        if index < 0 {
            return None;
        }
        let array_binding = self.resolve_array_binding_from_expression(object)?;
        let value = array_binding
            .values
            .get(index as usize)
            .cloned()
            .flatten()
            .unwrap_or(Expression::Undefined);
        self.resolve_static_loop_dependent_primitive(&value)
            .or(Some(value))
    }

    fn resolve_active_loop_indexed_expression_primitive(
        &mut self,
        expression: &Expression,
        environment: &HashMap<String, i64>,
    ) -> Option<Expression> {
        match expression {
            Expression::Member { object, property } => self
                .resolve_active_loop_indexed_array_member_primitive(object, property, environment),
            _ => self.resolve_static_loop_dependent_primitive(expression),
        }
    }

    fn resolve_static_equality_condition_value(
        &self,
        op: BinaryOp,
        left: &Expression,
        right: &Expression,
    ) -> Option<bool> {
        let is_loose = matches!(op, BinaryOp::LooseEqual | BinaryOp::LooseNotEqual);
        let is_not_equal = matches!(op, BinaryOp::NotEqual | BinaryOp::LooseNotEqual);
        let equal = match (left, right) {
            (Expression::Bool(left), Expression::Bool(right)) => Some(left == right),
            (Expression::Number(left), Expression::Number(right)) => Some(left == right),
            (Expression::String(left), Expression::String(right)) => Some(left == right),
            (Expression::Null, Expression::Null)
            | (Expression::Undefined, Expression::Undefined) => Some(true),
            (Expression::Null, Expression::Undefined)
            | (Expression::Undefined, Expression::Null)
                if is_loose =>
            {
                Some(true)
            }
            (_, _) if !is_loose => Some(false),
            _ => None,
        }?;
        Some(equal ^ is_not_equal)
    }

    pub(in crate::backend::direct_wasm) fn resolve_active_loop_indexed_member_if_condition_value(
        &mut self,
        condition: &Expression,
    ) -> Option<bool> {
        let Expression::Binary { op, left, right } = condition else {
            return None;
        };
        if !matches!(
            op,
            BinaryOp::Equal | BinaryOp::NotEqual | BinaryOp::LooseEqual | BinaryOp::LooseNotEqual
        ) {
            return None;
        }
        let environments = self.active_numeric_loop_environments()?;
        let mut condition_value = None;
        for environment in environments {
            let left = self.resolve_active_loop_indexed_expression_primitive(left, &environment)?;
            let right =
                self.resolve_active_loop_indexed_expression_primitive(right, &environment)?;
            let value = self.resolve_static_equality_condition_value(*op, &left, &right)?;
            if condition_value.is_some_and(|previous| previous != value) {
                return None;
            }
            condition_value = Some(value);
        }
        condition_value
    }

    pub(in crate::backend::direct_wasm) fn resolve_static_loop_dependent_if_condition_value(
        &self,
        condition: &Expression,
    ) -> Option<bool> {
        self.resolve_static_loop_dependent_condition_value(condition)
    }

    pub(in crate::backend::direct_wasm) fn expression_depends_on_active_loop_assignment(
        &self,
        expression: &Expression,
    ) -> bool {
        let mut referenced_names = HashSet::new();
        collect_referenced_binding_names_from_expression(expression, &mut referenced_names);
        self.state
            .emission
            .control_flow
            .loop_stack
            .iter()
            .rev()
            .any(|loop_context| {
                referenced_names.iter().any(|name| {
                    let source_name = scoped_binding_source_name(name).unwrap_or(name);
                    loop_context.assigned_bindings.contains(name)
                        || loop_context.assigned_bindings.contains(source_name)
                        || loop_context.numeric_binding_candidates.contains_key(name)
                        || loop_context
                            .numeric_binding_candidates
                            .contains_key(source_name)
                        || loop_context.numeric_spec.as_ref().is_some_and(|spec| {
                            spec.binding == *name || spec.binding == source_name
                        })
                })
            })
    }

    pub(in crate::backend::direct_wasm) fn if_condition_depends_on_active_loop_assignment(
        &self,
        condition: &Expression,
    ) -> bool {
        self.expression_depends_on_active_loop_assignment(condition)
            || self
                .iterator_domain()
                .depends_on_active_loop_assignment(condition)
    }

    pub(in crate::backend::direct_wasm) fn if_condition_depends_on_active_iterator_loop_assignment(
        &self,
        condition: &Expression,
    ) -> bool {
        self.iterator_domain()
            .depends_on_active_loop_assignment(condition)
    }

    pub(in crate::backend::direct_wasm) fn expression_has_dynamic_member_property_access(
        &self,
        expression: &Expression,
    ) -> bool {
        match expression {
            Expression::Member { object, property } => {
                !matches!(
                    property.as_ref(),
                    Expression::String(_) | Expression::Number(_)
                ) || self.expression_has_dynamic_member_property_access(object)
                    || self.expression_has_dynamic_member_property_access(property)
            }
            Expression::SuperMember { property } => {
                !matches!(
                    property.as_ref(),
                    Expression::String(_) | Expression::Number(_)
                ) || self.expression_has_dynamic_member_property_access(property)
            }
            Expression::Assign { value, .. }
            | Expression::Await(value)
            | Expression::EnumerateKeys(value)
            | Expression::GetIterator(value)
            | Expression::IteratorClose(value)
            | Expression::Unary {
                expression: value, ..
            } => self.expression_has_dynamic_member_property_access(value),
            Expression::AssignMember {
                object,
                property,
                value,
            } => {
                self.expression_has_dynamic_member_property_access(object)
                    || self.expression_has_dynamic_member_property_access(property)
                    || self.expression_has_dynamic_member_property_access(value)
            }
            Expression::AssignSuperMember { property, value } => {
                self.expression_has_dynamic_member_property_access(property)
                    || self.expression_has_dynamic_member_property_access(value)
            }
            Expression::Binary { left, right, .. } => {
                self.expression_has_dynamic_member_property_access(left)
                    || self.expression_has_dynamic_member_property_access(right)
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                self.expression_has_dynamic_member_property_access(condition)
                    || self.expression_has_dynamic_member_property_access(then_expression)
                    || self.expression_has_dynamic_member_property_access(else_expression)
            }
            Expression::Sequence(expressions) => expressions
                .iter()
                .any(|expression| self.expression_has_dynamic_member_property_access(expression)),
            Expression::Call { callee, arguments }
            | Expression::SuperCall { callee, arguments }
            | Expression::New { callee, arguments } => {
                self.expression_has_dynamic_member_property_access(callee)
                    || arguments.iter().any(|argument| match argument {
                        CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                            self.expression_has_dynamic_member_property_access(expression)
                        }
                    })
            }
            Expression::Array(elements) => elements.iter().any(|element| match element {
                ArrayElement::Expression(expression) | ArrayElement::Spread(expression) => {
                    self.expression_has_dynamic_member_property_access(expression)
                }
            }),
            Expression::Object(entries) => entries.iter().any(|entry| match entry {
                ObjectEntry::Data { key, value }
                | ObjectEntry::Getter { key, getter: value }
                | ObjectEntry::Setter { key, setter: value } => {
                    self.expression_has_dynamic_member_property_access(key)
                        || self.expression_has_dynamic_member_property_access(value)
                }
                ObjectEntry::Spread(expression) => {
                    self.expression_has_dynamic_member_property_access(expression)
                }
            }),
            Expression::Number(_)
            | Expression::BigInt(_)
            | Expression::String(_)
            | Expression::Bool(_)
            | Expression::Null
            | Expression::Undefined
            | Expression::NewTarget
            | Expression::Identifier(_)
            | Expression::This
            | Expression::Sent
            | Expression::Update { .. } => false,
        }
    }
}
