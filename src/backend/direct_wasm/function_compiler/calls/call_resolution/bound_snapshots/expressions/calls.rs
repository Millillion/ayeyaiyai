use super::*;

fn bound_snapshot_builtin_number_argument(value: &Expression) -> Option<f64> {
    match value {
        Expression::Number(number) => Some(*number),
        Expression::Bool(value) => Some(if *value { 1.0 } else { 0.0 }),
        Expression::String(text) => Some(text.trim().parse::<f64>().unwrap_or(f64::NAN)),
        Expression::Null => Some(0.0),
        Expression::Undefined => Some(f64::NAN),
        Expression::Identifier(_) | Expression::Object(_) | Expression::Array(_) => Some(f64::NAN),
        _ => None,
    }
}

impl<'a> FunctionCompiler<'a> {
    pub(super) fn evaluate_bound_snapshot_call_expression(
        &self,
        callee: &Expression,
        arguments: &[CallArgument],
        bindings: &mut HashMap<String, Expression>,
        current_function_name: Option<&str>,
    ) -> Option<Expression> {
        if let Expression::Member { object, property } = callee
            && matches!(property.as_ref(), Expression::String(name) if name == "push")
        {
            return self.apply_bound_snapshot_array_push(
                object,
                arguments,
                bindings,
                current_function_name,
            );
        }
        let resolved_callee = if matches!(callee, Expression::Identifier(_)) {
            self.evaluate_bound_snapshot_expression(callee, bindings, current_function_name)
        } else {
            None
        };
        if let Some(Expression::Identifier(marker)) = resolved_callee.as_ref() {
            let stored_value = arguments
                .first()
                .and_then(|argument| match argument {
                    CallArgument::Expression(expression) | CallArgument::Spread(expression) => self
                        .evaluate_bound_snapshot_expression(
                            expression,
                            bindings,
                            current_function_name,
                        ),
                })
                .unwrap_or(Expression::Undefined);
            match marker.as_str() {
                SNAPSHOT_AWAIT_RESOLVE_BINDING => {
                    bindings.insert(SNAPSHOT_AWAIT_RESOLUTION_VALUE.to_string(), stored_value);
                    return Some(Expression::Undefined);
                }
                SNAPSHOT_AWAIT_REJECT_BINDING => {
                    bindings.insert(SNAPSHOT_AWAIT_REJECTION_VALUE.to_string(), stored_value);
                    return Some(Expression::Undefined);
                }
                _ => {}
            }
        }
        let effective_callee = resolved_callee.as_ref().unwrap_or(callee);
        if matches!(effective_callee, Expression::Identifier(name) if name == "Number" && self.is_unshadowed_builtin_identifier(name))
        {
            let value = arguments
                .first()
                .and_then(|argument| match argument {
                    CallArgument::Expression(expression) | CallArgument::Spread(expression) => self
                        .evaluate_bound_snapshot_expression(
                            expression,
                            bindings,
                            current_function_name,
                        ),
                })
                .unwrap_or(Expression::Number(0.0));
            return bound_snapshot_builtin_number_argument(&value).map(Expression::Number);
        }
        if matches!(effective_callee, Expression::Identifier(name) if name == "isNaN" && self.is_unshadowed_builtin_identifier(name))
        {
            let value = arguments
                .first()
                .and_then(|argument| match argument {
                    CallArgument::Expression(expression) | CallArgument::Spread(expression) => self
                        .evaluate_bound_snapshot_expression(
                            expression,
                            bindings,
                            current_function_name,
                        ),
                })
                .unwrap_or(Expression::Undefined);
            return bound_snapshot_builtin_number_argument(&value)
                .map(|number| Expression::Bool(number.is_nan()));
        }
        let binding = self.resolve_function_binding_from_expression_with_context(
            effective_callee,
            current_function_name,
        )?;
        let mut evaluated_arguments = Vec::new();
        for argument in arguments {
            match argument {
                CallArgument::Expression(expression) => {
                    evaluated_arguments.push(self.evaluate_bound_snapshot_expression(
                        expression,
                        bindings,
                        current_function_name,
                    )?);
                }
                CallArgument::Spread(expression) => {
                    let value = self.evaluate_bound_snapshot_expression(
                        expression,
                        bindings,
                        current_function_name,
                    )?;
                    let elements = self.bound_snapshot_array_expression(&value, bindings)?;
                    for element in elements {
                        match element {
                            ArrayElement::Expression(value) => evaluated_arguments.push(value),
                            ArrayElement::Spread(value) => {
                                let nested_value = self.evaluate_bound_snapshot_expression(
                                    &value,
                                    bindings,
                                    current_function_name,
                                )?;
                                let nested_elements =
                                    self.bound_snapshot_array_expression(&nested_value, bindings)?;
                                for nested_element in nested_elements {
                                    let ArrayElement::Expression(nested_value) = nested_element
                                    else {
                                        return None;
                                    };
                                    evaluated_arguments.push(nested_value);
                                }
                            }
                        }
                    }
                }
            }
        }
        let this_binding = match &binding {
            LocalFunctionBinding::User(function_name) => self
                .user_function(function_name)
                .filter(|user_function| user_function.lexical_this)
                .and_then(|_| bindings.get("this").cloned())
                .unwrap_or(Expression::Undefined),
            LocalFunctionBinding::Builtin(_) => Expression::Undefined,
        };
        let (result, updated_bindings) = self
            .resolve_bound_snapshot_function_result_with_arguments_and_this(
                &binding,
                bindings,
                &evaluated_arguments,
                &this_binding,
            )?;
        *bindings = updated_bindings;
        Some(result)
    }
}
