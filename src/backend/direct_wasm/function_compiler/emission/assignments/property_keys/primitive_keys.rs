use super::*;

impl<'a> FunctionCompiler<'a> {
    /// Mirrors `static_global_property_name_from_generator_call`: a property
    /// key built from calling a simple generator resolves to the generator's
    /// static return value so define-time and lookup-time keys agree.
    fn static_property_key_from_generator_call(&self, expression: &Expression) -> Option<String> {
        let Expression::Call { callee, arguments } = expression else {
            return None;
        };
        if !arguments.is_empty() {
            return None;
        }
        let binding = self.resolve_function_binding_from_expression_with_context(
            callee,
            self.current_function_name(),
        )?;
        let LocalFunctionBinding::User(function_name) = binding else {
            return None;
        };
        let function = self.user_function(&function_name)?;
        if !function.is_generator() {
            return None;
        }
        let return_value = function.inline_summary.as_ref()?.return_value.as_ref()?;
        static_property_name_from_expression(&self.materialize_static_expression(return_value))
            .or_else(|| static_property_name_from_expression(return_value))
    }

    pub(in crate::backend::direct_wasm) fn resolve_primitive_property_key_expression(
        &self,
        expression: &Expression,
    ) -> Option<Expression> {
        if let Expression::Sequence(expressions) = expression {
            return self.resolve_primitive_property_key_expression(expressions.last()?);
        }
        if let Expression::Conditional {
            condition,
            then_expression,
            else_expression,
        } = expression
            && let Some(key) = self.resolve_primitive_conditional_assignment_property_key(
                condition,
                then_expression,
                else_expression,
            )
        {
            return Some(key);
        }
        if let Expression::Conditional {
            condition,
            then_expression,
            else_expression,
        } = expression
            && let Some(take_then) = self.resolve_static_boolean_expression(condition)
        {
            let selected = if take_then {
                then_expression
            } else {
                else_expression
            };
            if let Some(key) = self.resolve_primitive_property_key_expression(selected) {
                return Some(key);
            }
        }
        if let Expression::Identifier(name) = expression
            && name.starts_with("__ayy_class_field_name_")
        {
            let mut candidates = Vec::new();
            if let Some(hidden_name) = self.resolve_user_function_capture_hidden_name(name)
                && let Some(value) = self.global_value_binding(&hidden_name)
            {
                candidates.push(value.clone());
            }
            if let Some((resolved_name, _)) = self.resolve_current_local_binding(name)
                && resolved_name != *name
                && let Some(value) = self
                    .state
                    .speculation
                    .static_semantics
                    .local_value_binding(&resolved_name)
            {
                candidates.push(value.clone());
            }
            if let Some(value) = self
                .state
                .speculation
                .static_semantics
                .local_value_binding(name)
            {
                candidates.push(value.clone());
            }
            if let Some(value) = self.global_value_binding(name) {
                candidates.push(value.clone());
            }
            for candidate in candidates {
                if static_expression_matches(&candidate, expression) {
                    continue;
                }
                if let Some(key) =
                    self.resolve_class_field_name_primitive_property_key_candidate(&candidate)
                {
                    return Some(key);
                }
                let materialized = self.materialize_static_expression(&candidate);
                if !static_expression_matches(&materialized, &candidate)
                    && let Some(key) = self
                        .resolve_class_field_name_primitive_property_key_candidate(&materialized)
                {
                    return Some(key);
                }
            }
        }
        if let Expression::Binary {
            op: op @ (BinaryOp::LogicalAnd | BinaryOp::LogicalOr | BinaryOp::NullishCoalescing),
            left,
            right,
        } = expression
            && let (
                Expression::Identifier(left_name),
                Expression::Assign {
                    name: assigned_name,
                    ..
                },
            ) = (left.as_ref(), right.as_ref())
            && left_name == assigned_name
            && let Some(result) =
                self.resolve_primitive_logical_assignment_property_key_result(*op, left, right)
            && let Some(key) = self.resolve_primitive_property_key_expression(&result)
        {
            return Some(key);
        }
        if let Some(property_name) = static_property_name_from_expression(expression) {
            return Some(Expression::String(property_name));
        }
        // An assignment expression used as a property key evaluates to its
        // assigned value; the side effect is emitted separately by the
        // property-key effects path.
        if let Expression::Assign { value, .. } = expression {
            let key = self.resolve_primitive_property_key_expression(value);
            if crate::ayy_env_flag!("AYY_TRACE_ASSIGN_KEYS") {
                eprintln!(
                    "assign_key value={value:?} key={key:?} materialized={:?}",
                    self.materialize_static_expression(value)
                );
            }
            if let Some(key) = key {
                return Some(key);
            }
        }
        if let Expression::Call { callee, arguments } = expression
            && let Some(return_value) =
                self.resolve_effectful_call_return_metadata_value(callee, arguments)
            && !static_expression_matches(&return_value, expression)
            && let Some(key) = self.resolve_primitive_property_key_expression(&return_value)
        {
            return Some(key);
        }
        if let Some(property_name) = self.static_property_key_from_generator_call(expression) {
            return Some(Expression::String(property_name));
        }
        if self.well_known_symbol_name(expression).is_some() {
            return Some(expression.clone());
        }
        if let Some(symbol_identity) = self.resolve_symbol_identity_expression(expression) {
            return Some(symbol_identity);
        }
        if let Some(resolved) = self
            .resolve_bound_alias_expression(expression)
            .filter(|resolved| !static_expression_matches(resolved, expression))
        {
            if let Some(property_name) = static_property_name_from_expression(&resolved) {
                return Some(Expression::String(property_name));
            }
            if self.well_known_symbol_name(&resolved).is_some() {
                return Some(resolved);
            }
            if let Some(symbol_identity) = self.resolve_symbol_identity_expression(&resolved) {
                return Some(symbol_identity);
            }
            if let Some(key) = self.resolve_primitive_property_key_expression(&resolved) {
                return Some(key);
            }
        }
        if let Expression::Call { callee, arguments } = expression
            && matches!(callee.as_ref(), Expression::Identifier(name) if name == "String" && self.is_unshadowed_builtin_identifier(name))
        {
            let text = match arguments.first() {
                Some(CallArgument::Expression(argument)) | Some(CallArgument::Spread(argument)) => {
                    let materialized_argument = self.materialize_static_expression(argument);
                    let current_function_name = self.current_function_name();
                    if let Some(Expression::String(property_name)) = self
                        .resolve_primitive_property_key_expression(&materialized_argument)
                        .or_else(|| self.resolve_primitive_property_key_expression(argument))
                    {
                        property_name
                    } else if let Some(primitive) = self
                        .resolve_static_primitive_expression_with_context(
                            &materialized_argument,
                            current_function_name,
                        )
                        .or_else(|| {
                            self.resolve_static_primitive_expression_with_context(
                                argument,
                                current_function_name,
                            )
                        })
                        && let Some(property_name) =
                            static_property_name_from_expression(&primitive)
                    {
                        property_name
                    } else if let Some(binding) = self
                        .resolve_function_binding_from_expression_with_context(
                            &materialized_argument,
                            current_function_name,
                        )
                        .or_else(|| {
                            self.resolve_function_binding_from_expression_with_context(
                                argument,
                                current_function_name,
                            )
                        })
                    {
                        self.synthesize_static_function_binding_to_string(&binding)
                    } else {
                        self.resolve_static_symbol_to_string_value_with_context(
                            &materialized_argument,
                            current_function_name,
                        )
                        .or_else(|| {
                            self.resolve_static_symbol_to_string_value_with_context(
                                argument,
                                current_function_name,
                            )
                        })
                        .or_else(|| {
                            self.resolve_static_string_value_with_context(
                                &materialized_argument,
                                current_function_name,
                            )
                        })
                        .or_else(|| {
                            self.resolve_static_string_value_with_context(
                                argument,
                                current_function_name,
                            )
                        })?
                    }
                }
                None => String::new(),
            };
            return Some(Expression::String(text));
        }
        let materialized = self.materialize_static_expression(expression);
        if let Some(binding) = self
            .resolve_function_binding_from_expression_with_context(
                &materialized,
                self.current_function_name(),
            )
            .or_else(|| {
                self.resolve_function_binding_from_expression_with_context(
                    expression,
                    self.current_function_name(),
                )
            })
        {
            return Some(Expression::String(
                self.synthesize_static_function_binding_to_string(&binding),
            ));
        }
        if let Some(text) = self
            .resolve_static_string_value_with_context(&materialized, self.current_function_name())
        {
            return Some(Expression::String(text));
        }
        if let Some(text) =
            self.resolve_static_string_value_with_context(expression, self.current_function_name())
        {
            return Some(Expression::String(text));
        }
        if let Some(primitive) = self.resolve_static_primitive_expression_with_context(
            &materialized,
            self.current_function_name(),
        ) {
            if let Some(property_name) = static_property_name_from_expression(&primitive) {
                return Some(Expression::String(property_name));
            }
        }
        if let Some(primitive) = self.resolve_static_primitive_expression_with_context(
            expression,
            self.current_function_name(),
        ) {
            if let Some(property_name) = static_property_name_from_expression(&primitive) {
                return Some(Expression::String(property_name));
            }
        }
        if let Some(property_name) = static_property_name_from_expression(&materialized) {
            return Some(Expression::String(property_name));
        }
        if matches!(expression, Expression::Identifier(name) if name.starts_with("__ayy_generator_sent_"))
        {
            return Some(Expression::String("undefined".to_string()));
        }
        if self.well_known_symbol_name(&materialized).is_some() {
            return Some(materialized);
        }
        if self.well_known_symbol_name(expression).is_some() {
            return Some(expression.clone());
        }
        self.resolve_symbol_identity_expression(&materialized)
            .or_else(|| self.resolve_symbol_identity_expression(expression))
    }

    fn resolve_primitive_conditional_assignment_property_key(
        &self,
        condition: &Expression,
        then_expression: &Expression,
        else_expression: &Expression,
    ) -> Option<Expression> {
        if let Some(key) = self.resolve_primitive_conditional_nullish_assignment_property_key(
            condition,
            then_expression,
            else_expression,
        ) {
            return Some(key);
        }

        let Expression::Identifier(condition_name) = condition else {
            return None;
        };

        let (op, assigned_value) = match (then_expression, else_expression) {
            (
                Expression::Assign {
                    name: assigned_name,
                    value,
                },
                Expression::Identifier(else_name),
            ) if assigned_name == condition_name && else_name == condition_name => {
                (BinaryOp::LogicalAnd, value.as_ref())
            }
            (
                Expression::Identifier(then_name),
                Expression::Assign {
                    name: assigned_name,
                    value,
                },
            ) if then_name == condition_name && assigned_name == condition_name => {
                (BinaryOp::LogicalOr, value.as_ref())
            }
            _ => return None,
        };

        let current_value = self.primitive_property_key_identifier_value(condition_name);
        let assigned_value = self.materialize_static_expression(assigned_value);
        let result = match op {
            BinaryOp::LogicalAnd => {
                let current_value = current_value?;
                if self
                    .resolve_static_boolean_expression(&current_value)
                    .or_else(|| self.resolve_static_boolean_expression(condition))?
                {
                    assigned_value
                } else {
                    current_value
                }
            }
            BinaryOp::LogicalOr => {
                let Some(current_value) = current_value else {
                    return self.resolve_primitive_property_key_expression(&assigned_value);
                };
                if self
                    .resolve_static_boolean_expression(&current_value)
                    .or_else(|| self.resolve_static_boolean_expression(condition))?
                {
                    current_value
                } else {
                    assigned_value
                }
            }
            _ => return None,
        };

        self.resolve_primitive_property_key_expression(&result)
    }

    fn resolve_primitive_conditional_nullish_assignment_property_key(
        &self,
        condition: &Expression,
        then_expression: &Expression,
        else_expression: &Expression,
    ) -> Option<Expression> {
        let Expression::Identifier(then_name) = then_expression else {
            return None;
        };
        let Expression::Assign {
            name: assigned_name,
            value,
        } = else_expression
        else {
            return None;
        };
        if then_name != assigned_name
            || !Self::conditional_nullish_assignment_condition_matches(condition, then_name)
        {
            return None;
        }

        let assigned_value = self.materialize_static_expression(value);
        let current_value = self.primitive_property_key_identifier_value(then_name);
        let result = match current_value {
            Some(Expression::Null | Expression::Undefined) | None => assigned_value,
            Some(value) => value,
        };

        self.resolve_primitive_property_key_expression(&result)
    }

    fn conditional_nullish_assignment_condition_matches(
        condition: &Expression,
        name: &str,
    ) -> bool {
        let Expression::Binary {
            op: BinaryOp::LogicalAnd,
            left,
            right,
        } = condition
        else {
            return false;
        };

        let left_matches =
            Self::conditional_nullish_assignment_comparison_matches(left, name, true);
        let right_matches =
            Self::conditional_nullish_assignment_comparison_matches(right, name, false);
        let swapped_left =
            Self::conditional_nullish_assignment_comparison_matches(left, name, false);
        let swapped_right =
            Self::conditional_nullish_assignment_comparison_matches(right, name, true);

        (left_matches && right_matches) || (swapped_left && swapped_right)
    }

    fn conditional_nullish_assignment_comparison_matches(
        expression: &Expression,
        name: &str,
        expect_undefined: bool,
    ) -> bool {
        let Expression::Binary { op, left, right } = expression else {
            return false;
        };
        if !matches!(op, BinaryOp::NotEqual | BinaryOp::LooseNotEqual) {
            return false;
        }

        let expected_matches = |expression: &Expression| {
            if expect_undefined {
                matches!(expression, Expression::Undefined)
                    || matches!(expression, Expression::Identifier(identifier) if identifier == "undefined")
            } else {
                matches!(expression, Expression::Null)
            }
        };
        let identifier_matches = |expression: &Expression| matches!(expression, Expression::Identifier(identifier) if identifier == name);

        (identifier_matches(left) && expected_matches(right))
            || (identifier_matches(right) && expected_matches(left))
    }

    fn primitive_property_key_identifier_value(&self, name: &str) -> Option<Expression> {
        let identifier = Expression::Identifier(name.to_string());
        let materialized = self.materialize_static_expression(&identifier);
        if !static_expression_matches(&materialized, &identifier) {
            return Some(materialized);
        }

        if let Some((resolved_name, _)) = self.resolve_current_local_binding(name)
            && resolved_name != name
            && let Some(value) = self
                .state
                .speculation
                .static_semantics
                .local_value_binding(&resolved_name)
            && !static_expression_matches(value, &identifier)
        {
            return Some(value.clone());
        }

        self.state
            .speculation
            .static_semantics
            .local_value_binding(name)
            .or_else(|| self.global_value_binding(name))
            .filter(|value| !static_expression_matches(value, &identifier))
            .cloned()
    }

    fn resolve_class_field_name_primitive_property_key_candidate(
        &self,
        expression: &Expression,
    ) -> Option<Expression> {
        if let Some(key) = self.resolve_primitive_property_key_expression(expression) {
            return Some(key);
        }

        let inner = match expression {
            Expression::Sequence(expressions) => expressions.last()?,
            other => other,
        };
        let Expression::Binary {
            op: BinaryOp::LogicalOr | BinaryOp::NullishCoalescing,
            left,
            right,
        } = inner
        else {
            return None;
        };
        let (
            Expression::Identifier(left_name),
            Expression::Assign {
                name: assigned_name,
                value,
            },
        ) = (left.as_ref(), right.as_ref())
        else {
            return None;
        };
        if left_name != assigned_name {
            return None;
        }

        self.resolve_primitive_property_key_expression(value)
    }

    fn resolve_primitive_logical_assignment_property_key_result(
        &self,
        op: BinaryOp,
        left: &Expression,
        right: &Expression,
    ) -> Option<Expression> {
        let materialized_left = if let Expression::Identifier(name) = left {
            self.state
                .speculation
                .static_semantics
                .local_value_binding(name)
                .or_else(|| self.global_value_binding(name))
                .or_else(|| self.backend.global_value_binding(name))
                .filter(|value| !static_expression_matches(value, left))
                .cloned()
                .unwrap_or_else(|| self.materialize_static_expression(left))
        } else {
            self.materialize_static_expression(left)
        };
        let assigned_value = match right {
            Expression::Assign { value, .. } => value.as_ref(),
            _ => right,
        };

        match op {
            BinaryOp::LogicalAnd => {
                let left_truthy = self
                    .resolve_static_boolean_expression(&materialized_left)
                    .or_else(|| self.resolve_static_boolean_expression(left))?;
                if left_truthy {
                    Some(self.materialize_static_expression(assigned_value))
                } else {
                    Some(materialized_left)
                }
            }
            BinaryOp::LogicalOr => {
                let Some(left_truthy) = self
                    .resolve_static_boolean_expression(&materialized_left)
                    .or_else(|| self.resolve_static_boolean_expression(left))
                else {
                    return Some(self.materialize_static_expression(assigned_value));
                };
                if left_truthy {
                    Some(materialized_left)
                } else {
                    Some(self.materialize_static_expression(assigned_value))
                }
            }
            BinaryOp::NullishCoalescing => {
                let Some(primitive_left) = self
                    .resolve_static_primitive_expression_with_context(
                        &materialized_left,
                        self.current_function_name(),
                    )
                    .or_else(|| {
                        self.resolve_static_primitive_expression_with_context(
                            left,
                            self.current_function_name(),
                        )
                    })
                else {
                    return Some(self.materialize_static_expression(assigned_value));
                };
                if matches!(primitive_left, Expression::Null | Expression::Undefined) {
                    Some(self.materialize_static_expression(assigned_value))
                } else {
                    Some(primitive_left)
                }
            }
            _ => None,
        }
    }
}
