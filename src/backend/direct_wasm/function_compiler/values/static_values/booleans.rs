use super::*;

#[path = "booleans/builtin_calls.rs"]
mod builtin_calls;
#[path = "booleans/comparisons.rs"]
mod comparisons;
#[path = "booleans/logical_ops.rs"]
mod logical_ops;

impl<'a> FunctionCompiler<'a> {
    fn boolean_expression_reads_runtime_nonlocal_binding(&self, expression: &Expression) -> bool {
        if self.current_function_name().is_none() {
            return false;
        }

        let mut referenced_names = HashSet::new();
        collect_referenced_binding_names_from_expression(expression, &mut referenced_names);
        referenced_names.iter().any(|name| {
            let source_name = scoped_binding_source_name(name).unwrap_or(name);
            self.resolve_current_local_binding(source_name).is_none()
                && (self.global_has_binding(source_name)
                    || self.global_has_implicit_binding(source_name)
                    || self
                        .resolve_user_function_capture_hidden_name(source_name)
                        .is_some())
        })
    }

    pub(in crate::backend::direct_wasm) fn resolve_static_boolean_expression(
        &self,
        expression: &Expression,
    ) -> Option<bool> {
        if self.boolean_expression_reads_runtime_nonlocal_binding(expression) {
            return None;
        }

        let materialized = self.materialize_static_expression(expression);
        match materialized {
            Expression::Bool(value) => Some(value),
            Expression::Null | Expression::Undefined => Some(false),
            Expression::String(text) => Some(!text.is_empty()),
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                let branch = if self.resolve_static_if_condition_value(&condition)? {
                    &then_expression
                } else {
                    &else_expression
                };
                self.resolve_static_boolean_expression(branch)
            }
            Expression::Array(_)
            | Expression::Object(_)
            | Expression::New { .. }
            | Expression::This => Some(true),
            Expression::Identifier(name) => match name.as_str() {
                "undefined" => Some(false),
                "NaN" if self.is_unshadowed_builtin_identifier(name.as_str()) => Some(false),
                _ => {
                    let identifier = Expression::Identifier(name.clone());
                    if self
                        .resolve_object_binding_from_expression(&identifier)
                        .is_some()
                        || self
                            .resolve_array_binding_from_expression(&identifier)
                            .is_some()
                        || self
                            .resolve_arguments_binding_from_expression(&identifier)
                            .is_some()
                        || self
                            .resolve_proxy_binding_from_expression(&identifier)
                            .is_some()
                        || self
                            .resolve_function_binding_from_expression(&identifier)
                            .is_some()
                    {
                        Some(true)
                    } else {
                        match self.lookup_identifier_kind(&name) {
                            Some(StaticValueKind::Object)
                            | Some(StaticValueKind::Function)
                            | Some(StaticValueKind::Symbol) => Some(true),
                            Some(StaticValueKind::Null) | Some(StaticValueKind::Undefined) => {
                                Some(false)
                            }
                            _ => None,
                        }
                    }
                }
            },
            Expression::Unary {
                op: UnaryOp::Not,
                expression,
            } => Some(!self.resolve_static_boolean_expression(&expression)?),
            Expression::Binary { op, left, right } => match op {
                BinaryOp::LogicalAnd | BinaryOp::LogicalOr | BinaryOp::NullishCoalescing => self
                    .resolve_static_logical_result_expression(op, &left, &right)
                    .and_then(|value| self.resolve_static_boolean_expression(&value)),
                BinaryOp::Equal
                | BinaryOp::LooseEqual
                | BinaryOp::NotEqual
                | BinaryOp::LooseNotEqual
                | BinaryOp::LessThan
                | BinaryOp::LessThanOrEqual
                | BinaryOp::GreaterThan
                | BinaryOp::GreaterThanOrEqual => {
                    self.resolve_static_binary_boolean_result(&op, &left, &right)
                }
                BinaryOp::In => self.resolve_static_in_expression_result(&left, &right),
                _ => None,
            },
            Expression::Unary {
                op: UnaryOp::Plus,
                expression,
            }
            | Expression::Unary {
                op: UnaryOp::Negate,
                expression,
            } => {
                let number = self.resolve_static_number_value(&expression)?;
                Some(number != 0.0 && !number.is_nan())
            }
            Expression::Number(value) => Some(value != 0.0 && !value.is_nan()),
            Expression::Call { .. } => self
                .resolve_static_has_own_property_call_result(expression)
                .or_else(|| self.resolve_static_reflect_has_call_result(expression))
                .or_else(|| self.resolve_static_is_nan_call_result(expression))
                .or_else(|| self.resolve_static_object_is_call_result(expression))
                .or_else(|| self.resolve_static_array_is_array_call_result(expression)),
            Expression::Assign { value, .. } => self.resolve_static_boolean_expression(&value),
            _ => None,
        }
    }

    fn resolve_static_in_expression_result(
        &self,
        left: &Expression,
        right: &Expression,
    ) -> Option<bool> {
        if let Some(array_binding) = self.resolve_array_binding_from_expression(right) {
            if matches!(left, Expression::String(property_name) if property_name == "length") {
                return Some(true);
            }
            let materialized_left = self.materialize_static_expression(left);
            if let Some(index) = argument_index_from_expression(left)
                .or_else(|| argument_index_from_expression(&materialized_left))
            {
                return Some(
                    array_binding
                        .values
                        .get(index as usize)
                        .is_some_and(|value| value.is_some()),
                );
            }
            if let Expression::Member { object, .. } = left
                && let Some(key_binding) = self.resolve_array_binding_from_expression(object)
                && !key_binding.values.is_empty()
                && key_binding.values.iter().all(|value| {
                    matches!(
                        value,
                        Some(Expression::String(property_name))
                            if argument_index_from_expression(&Expression::String(property_name.clone()))
                                .is_some_and(|index| {
                                    array_binding
                                        .values
                                        .get(index as usize)
                                        .is_some_and(|value| value.is_some())
                                })
                    )
                })
            {
                return Some(true);
            }
        }

        let materialized_right = self.materialize_static_expression(right);
        let object_binding = self
            .resolve_object_binding_from_expression(right)
            .or_else(|| {
                (!static_expression_matches(&materialized_right, right))
                    .then(|| self.resolve_object_binding_from_expression(&materialized_right))?
            });
        if let Some(object_binding) = object_binding {
            let materialized_left = self.materialize_static_expression(left);
            if self.runtime_object_property_shadow_deletion_may_affect_property(
                right,
                &materialized_left,
            ) {
                return None;
            }
            return Some(
                self.resolve_object_binding_property_value_with_inherited(
                    right,
                    &object_binding,
                    &materialized_left,
                )
                .is_some(),
            );
        }

        if let Expression::Identifier(name) = right
            && let Expression::String(property_name) = left
        {
            return match name.as_str() {
                "Number" => Some(matches!(
                    property_name.as_str(),
                    "MAX_VALUE" | "MIN_VALUE" | "NaN" | "POSITIVE_INFINITY" | "NEGATIVE_INFINITY"
                )),
                _ => None,
            };
        }

        None
    }
}
