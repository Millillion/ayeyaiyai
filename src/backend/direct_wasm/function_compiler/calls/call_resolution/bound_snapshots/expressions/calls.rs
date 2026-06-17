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
    fn evaluate_bound_snapshot_call_receiver(
        &self,
        callee: &Expression,
        bindings: &mut HashMap<String, Expression>,
        current_function_name: Option<&str>,
    ) -> Expression {
        match callee {
            Expression::Member { object, .. } => self
                .evaluate_bound_snapshot_expression(object, bindings, current_function_name)
                .unwrap_or_else(|| self.materialize_static_expression(object)),
            Expression::SuperMember { .. } => {
                bindings.get("this").cloned().unwrap_or(Expression::This)
            }
            _ => Expression::Undefined,
        }
    }

    fn evaluate_bound_snapshot_define_property_call(
        &self,
        reflect_call: bool,
        arguments: &[CallArgument],
        bindings: &mut HashMap<String, Expression>,
        current_function_name: Option<&str>,
    ) -> Option<Expression> {
        let [
            CallArgument::Expression(target),
            CallArgument::Expression(property),
            CallArgument::Expression(descriptor),
            ..,
        ] = arguments
        else {
            return Some(if reflect_call {
                Expression::Bool(false)
            } else {
                Expression::Undefined
            });
        };
        let Some(descriptor) = resolve_property_descriptor_definition(descriptor) else {
            return Some(if reflect_call {
                Expression::Bool(false)
            } else {
                self.evaluate_bound_snapshot_expression(target, bindings, current_function_name)
                    .unwrap_or_else(|| self.materialize_static_expression(target))
            });
        };

        let target_name = match target {
            Expression::Identifier(name) => Some(
                self.resolve_bound_snapshot_binding_name(name, bindings)
                    .to_string(),
            ),
            _ => self
                .evaluate_bound_snapshot_expression(target, bindings, current_function_name)
                .and_then(|value| match value {
                    Expression::Identifier(name) => Some(name),
                    _ => None,
                }),
        };
        let Some(target_name) = target_name else {
            return Some(if reflect_call {
                Expression::Bool(false)
            } else {
                self.evaluate_bound_snapshot_expression(target, bindings, current_function_name)
                    .unwrap_or_else(|| self.materialize_static_expression(target))
            });
        };

        let target_value = bindings
            .get(&target_name)
            .cloned()
            .unwrap_or_else(|| Expression::Identifier(target_name.clone()));
        let mut object_binding = self
            .resolve_object_binding_from_expression(&target_value)
            .unwrap_or_else(empty_object_value_binding);
        let property = self
            .evaluate_bound_snapshot_expression(property, bindings, current_function_name)
            .unwrap_or_else(|| self.materialize_static_expression(property));
        let property = self
            .resolve_property_key_expression(&property)
            .unwrap_or(property);

        if !object_binding_can_define_property(&object_binding, &property) {
            return Some(if reflect_call {
                Expression::Bool(false)
            } else {
                Expression::Identifier(target_name)
            });
        }

        let descriptor_value =
            |expression: &Expression,
             context: &FunctionCompiler<'a>,
             bindings: &mut HashMap<String, Expression>| {
                context
                    .evaluate_bound_snapshot_expression(expression, bindings, current_function_name)
                    .unwrap_or_else(|| context.materialize_static_expression(expression))
            };
        let value = descriptor
            .value
            .as_ref()
            .map(|expression| descriptor_value(expression, self, bindings));
        let getter = descriptor
            .getter
            .as_ref()
            .map(|expression| descriptor_value(expression, self, bindings));
        let setter = descriptor
            .setter
            .as_ref()
            .map(|expression| descriptor_value(expression, self, bindings));
        object_binding_define_property_descriptor(
            &mut object_binding,
            property,
            PropertyDescriptorBinding {
                value,
                configurable: descriptor.configurable.unwrap_or(false),
                enumerable: descriptor.enumerable.unwrap_or(false),
                writable: if descriptor.is_accessor() {
                    None
                } else {
                    Some(descriptor.writable.unwrap_or(false))
                },
                getter,
                setter,
                has_get: descriptor.getter.is_some(),
                has_set: descriptor.setter.is_some(),
            },
        );
        bindings.insert(
            target_name.clone(),
            object_binding_to_expression_with_descriptor_entries(&object_binding),
        );
        Some(if reflect_call {
            Expression::Bool(true)
        } else {
            Expression::Identifier(target_name)
        })
    }

    pub(super) fn evaluate_bound_snapshot_call_expression(
        &self,
        callee: &Expression,
        arguments: &[CallArgument],
        bindings: &mut HashMap<String, Expression>,
        current_function_name: Option<&str>,
    ) -> Option<Expression> {
        if let Expression::Member { object, property } = callee
            && matches!(object.as_ref(), Expression::Identifier(name) if name == "assert")
            && matches!(property.as_ref(), Expression::String(name) if name == "sameValue")
        {
            let [
                CallArgument::Expression(actual_expression),
                CallArgument::Expression(expected_expression),
                rest @ ..,
            ] = arguments
            else {
                return None;
            };
            let actual = self.evaluate_bound_snapshot_expression(
                actual_expression,
                bindings,
                current_function_name,
            )?;
            let expected = self.evaluate_bound_snapshot_expression(
                expected_expression,
                bindings,
                current_function_name,
            )?;
            let result = self.resolve_static_same_value_result_with_context(
                &actual,
                &expected,
                current_function_name,
            );
            let result = result.or_else(|| {
                matches!(
                    (&actual, expected_expression),
                    (Expression::Identifier(actual_name), Expression::Identifier(expected_name))
                        if actual_name == self.resolve_bound_snapshot_binding_name(expected_name, bindings)
                )
                .then_some(true)
            });
            if crate::ayy_env_flag!("AYY_TRACE_BOUND_SNAPSHOT") {
                eprintln!(
                    "bound_snapshot_assert_same_value actual={actual:?} expected={expected:?} result={result:?}"
                );
            }
            if !result? {
                return None;
            }
            for argument in rest {
                match argument {
                    CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                        self.evaluate_bound_snapshot_expression(
                            expression,
                            bindings,
                            current_function_name,
                        )?;
                    }
                }
            }
            return Some(Expression::Undefined);
        }
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
        if let Expression::Member { object, property } = effective_callee
            && matches!(object.as_ref(), Expression::Identifier(name) if name == "Promise" && self.is_unshadowed_builtin_identifier(name))
            && matches!(
                property.as_ref(),
                Expression::String(name)
                    if matches!(name.as_str(), "resolve" | "reject" | "all" | "withResolvers")
            )
        {
            return Some(Expression::Call {
                callee: Box::new(effective_callee.clone()),
                arguments: arguments.to_vec(),
            });
        }
        if let Expression::Member { object, property } = effective_callee
            && matches!(object.as_ref(), Expression::Identifier(name) if name == "Object" || name == "Reflect")
            && matches!(property.as_ref(), Expression::String(name) if name == "defineProperty")
        {
            let reflect_call =
                matches!(object.as_ref(), Expression::Identifier(name) if name == "Reflect");
            return self.evaluate_bound_snapshot_define_property_call(
                reflect_call,
                arguments,
                bindings,
                current_function_name,
            );
        }
        let binding = self.resolve_function_binding_from_expression_with_context(
            effective_callee,
            current_function_name,
        )?;
        if let LocalFunctionBinding::User(function_name) = &binding
            && self
                .user_function(function_name)
                .is_some_and(|function| function.is_generator())
        {
            return Some(Expression::Call {
                callee: Box::new(effective_callee.clone()),
                arguments: arguments.to_vec(),
            });
        }
        let call_receiver = self.evaluate_bound_snapshot_call_receiver(
            effective_callee,
            bindings,
            current_function_name,
        );
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
            LocalFunctionBinding::User(function_name) => {
                let user_function = self.user_function(function_name)?;
                if user_function.lexical_this {
                    bindings
                        .get("this")
                        .cloned()
                        .unwrap_or(Expression::Undefined)
                } else if self.should_box_sloppy_function_this(user_function, &call_receiver) {
                    Expression::This
                } else {
                    call_receiver.clone()
                }
            }
            LocalFunctionBinding::Builtin(_) => Expression::Undefined,
        };
        if let LocalFunctionBinding::Builtin(function_name) = &binding {
            let call_arguments = evaluated_arguments
                .iter()
                .cloned()
                .map(CallArgument::Expression)
                .collect::<Vec<_>>();
            return match self.resolve_static_builtin_function_outcome(
                function_name,
                &call_arguments,
                current_function_name,
            )? {
                StaticEvalOutcome::Value(value) => Some(value),
                StaticEvalOutcome::Throw(_) => None,
            };
        }
        let (result, updated_bindings) = self
            .resolve_bound_snapshot_function_result_with_arguments_and_this(
                &binding,
                bindings,
                &evaluated_arguments,
                &this_binding,
            )?;
        Self::merge_bound_snapshot_updated_bindings(bindings, updated_bindings);
        Some(result)
    }
}
