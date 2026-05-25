use super::*;

impl<'a> FunctionCompiler<'a> {
    pub(in crate::backend::direct_wasm) fn resolve_static_bound_has_own_property_result(
        &self,
        receiver: &Expression,
        property: &Expression,
    ) -> Option<bool> {
        self.resolve_static_has_own_property_call_result(&Expression::Call {
            callee: Box::new(Expression::Member {
                object: Box::new(receiver.clone()),
                property: Box::new(Expression::String("hasOwnProperty".to_string())),
            }),
            arguments: vec![CallArgument::Expression(property.clone())],
        })
    }

    pub(in crate::backend::direct_wasm) fn resolve_static_builtin_function_outcome(
        &self,
        function_name: &str,
        arguments: &[CallArgument],
        current_function_name: Option<&str>,
    ) -> Option<StaticEvalOutcome> {
        if let Some(target_name) = parse_bound_function_prototype_call_builtin_name(function_name)
            && let Some(value) = self.resolve_static_bound_function_prototype_call_value(
                target_name,
                arguments,
                current_function_name,
            )
        {
            return Some(StaticEvalOutcome::Value(value));
        }

        if let Some(value) = self.resolve_static_builtin_primitive_call_value(
            function_name,
            arguments,
            current_function_name,
        ) {
            return Some(StaticEvalOutcome::Value(value));
        }

        if parse_test262_realm_eval_builtin(function_name).is_some() {
            return self.resolve_static_indirect_eval_completion_outcome_with_context(
                arguments,
                Some(function_name),
            );
        }

        match function_name {
            "eval" => self.resolve_static_direct_eval_completion_outcome_with_context(
                arguments,
                current_function_name,
            ),
            "Object.getOwnPropertyDescriptor" => {
                let receiver = match arguments.first()? {
                    CallArgument::Expression(receiver) | CallArgument::Spread(receiver) => receiver,
                };
                let property = match arguments.get(1) {
                    Some(CallArgument::Expression(property) | CallArgument::Spread(property)) => {
                        property.clone()
                    }
                    None => Expression::Undefined,
                };
                let descriptor_call = Expression::Call {
                    callee: Box::new(Expression::Member {
                        object: Box::new(Expression::Identifier("Object".to_string())),
                        property: Box::new(Expression::String(
                            "getOwnPropertyDescriptor".to_string(),
                        )),
                    }),
                    arguments: vec![
                        CallArgument::Expression(receiver.clone()),
                        CallArgument::Expression(property.clone()),
                    ],
                };
                if self.runtime_object_property_shadow_deletion_may_affect_property(
                    receiver, &property,
                ) {
                    return None;
                }
                if let Some(descriptor) =
                    self.resolve_descriptor_binding_from_expression(&descriptor_call)
                {
                    return Some(StaticEvalOutcome::Value(object_binding_to_expression(
                        &self.object_binding_from_property_descriptor(&descriptor),
                    )));
                }
                if self
                    .resolve_function_binding_from_expression(receiver)
                    .is_some()
                    && self.resolve_function_object_has_own_property(receiver, &property)
                        == Some(false)
                {
                    return Some(StaticEvalOutcome::Value(Expression::Undefined));
                }
                if self.resolve_static_object_has_own_property_result(receiver, &property)
                    == Some(Some(false))
                {
                    return Some(StaticEvalOutcome::Value(Expression::Undefined));
                }
                None
            }
            "Reflect.has" => {
                let target = match arguments.first() {
                    Some(CallArgument::Expression(target) | CallArgument::Spread(target)) => target,
                    None => return None,
                };
                let property = match arguments.get(1) {
                    Some(CallArgument::Expression(property) | CallArgument::Spread(property)) => {
                        property.clone()
                    }
                    None => Expression::Undefined,
                };
                self.resolve_static_reflect_has_result(target, &property)
                    .map(Expression::Bool)
                    .map(StaticEvalOutcome::Value)
            }
            "Math.abs" => Some(StaticEvalOutcome::Value(Expression::Number(
                self.resolve_static_builtin_math_argument_number(
                    arguments.first()?,
                    current_function_name,
                )?
                .abs(),
            ))),
            "Math.atan" => Some(StaticEvalOutcome::Value(Expression::Number(
                self.resolve_static_builtin_math_argument_number(
                    arguments.first()?,
                    current_function_name,
                )?
                .atan(),
            ))),
            "Math.ceil" => Some(StaticEvalOutcome::Value(Expression::Number(
                self.resolve_static_builtin_math_argument_number(
                    arguments.first()?,
                    current_function_name,
                )?
                .ceil(),
            ))),
            "Math.exp" => Some(StaticEvalOutcome::Value(Expression::Number(
                self.resolve_static_builtin_math_argument_number(
                    arguments.first()?,
                    current_function_name,
                )?
                .exp(),
            ))),
            "Math.floor" => Some(StaticEvalOutcome::Value(Expression::Number(
                self.resolve_static_builtin_math_argument_number(
                    arguments.first()?,
                    current_function_name,
                )?
                .floor(),
            ))),
            "Math.max" => Some(StaticEvalOutcome::Value(Expression::Number(
                self.resolve_static_math_extremum(arguments, current_function_name, true)?,
            ))),
            "Math.min" => Some(StaticEvalOutcome::Value(Expression::Number(
                self.resolve_static_math_extremum(arguments, current_function_name, false)?,
            ))),
            "Math.pow" => Some(StaticEvalOutcome::Value(Expression::Number(
                self.resolve_static_builtin_math_argument_number(
                    arguments.first()?,
                    current_function_name,
                )?
                .powf(self.resolve_static_builtin_math_argument_number(
                    arguments.get(1)?,
                    current_function_name,
                )?),
            ))),
            "Math.sin" => Some(StaticEvalOutcome::Value(Expression::Number(
                self.resolve_static_builtin_math_argument_number(
                    arguments.first()?,
                    current_function_name,
                )?
                .sin(),
            ))),
            _ => None,
        }
    }

    fn resolve_static_bound_function_prototype_call_value(
        &self,
        target_name: &str,
        arguments: &[CallArgument],
        current_function_name: Option<&str>,
    ) -> Option<Expression> {
        let Some(CallArgument::Expression(receiver) | CallArgument::Spread(receiver)) =
            arguments.first()
        else {
            return Some(Expression::Undefined);
        };
        let target_arguments = &arguments[1..];
        match target_name {
            "Array.prototype.join" => {
                let array_binding = self.resolve_array_binding_from_expression(receiver)?;
                let separator = match target_arguments.first() {
                    Some(
                        CallArgument::Expression(expression) | CallArgument::Spread(expression),
                    ) => {
                        self.resolve_static_string_concat_value(expression, current_function_name)?
                    }
                    None => ",".to_string(),
                };
                let mut parts = Vec::with_capacity(array_binding.values.len());
                for value in &array_binding.values {
                    let Some(value) = value else {
                        parts.push(String::new());
                        continue;
                    };
                    let materialized = self
                        .resolve_static_primitive_expression_with_context(
                            value,
                            current_function_name,
                        )
                        .unwrap_or_else(|| self.materialize_static_expression(value));
                    let text = match materialized {
                        Expression::Undefined | Expression::Null => String::new(),
                        _ => self.resolve_static_string_concat_value(
                            &materialized,
                            current_function_name,
                        )?,
                    };
                    parts.push(text);
                }
                Some(Expression::String(parts.join(&separator)))
            }
            "Object.prototype.hasOwnProperty" => {
                let Some(CallArgument::Expression(property) | CallArgument::Spread(property)) =
                    target_arguments.first()
                else {
                    return Some(Expression::Bool(false));
                };
                self.resolve_static_bound_has_own_property_result(receiver, property)
                    .map(Expression::Bool)
            }
            "Object.prototype.propertyIsEnumerable" => {
                let Some(CallArgument::Expression(property) | CallArgument::Spread(property)) =
                    target_arguments.first()
                else {
                    return Some(Expression::Bool(false));
                };
                self.resolve_bound_function_prototype_call_descriptor(receiver, property)
                    .map(|descriptor| Expression::Bool(descriptor.enumerable))
            }
            _ => None,
        }
    }

    pub(in crate::backend::direct_wasm) fn resolve_static_builtin_primitive_call_value(
        &self,
        function_name: &str,
        arguments: &[CallArgument],
        current_function_name: Option<&str>,
    ) -> Option<Expression> {
        match function_name {
            "String" => Some(Expression::String(match arguments.first() {
                Some(CallArgument::Expression(argument) | CallArgument::Spread(argument)) => self
                    .resolve_static_symbol_to_string_value_with_context(
                        argument,
                        current_function_name,
                    )
                    .or_else(|| {
                        self.resolve_static_string_concat_value(argument, current_function_name)
                    })?,
                None => String::new(),
            })),
            "JSON.stringify" => match arguments.first() {
                None => Some(Expression::Undefined),
                Some(CallArgument::Expression(argument) | CallArgument::Spread(argument)) => {
                    match self.resolve_static_primitive_expression_with_context(
                        argument,
                        current_function_name,
                    )? {
                        Expression::String(text) => {
                            Some(Expression::String(Self::escape_static_json_string(&text)))
                        }
                        Expression::Bool(value) => Some(Expression::String(if value {
                            "true".to_string()
                        } else {
                            "false".to_string()
                        })),
                        Expression::Null => Some(Expression::String("null".to_string())),
                        Expression::Number(value) => {
                            Some(Expression::String(if value.is_finite() {
                                self.resolve_static_string_concat_value(
                                    &Expression::Number(value),
                                    current_function_name,
                                )?
                            } else {
                                "null".to_string()
                            }))
                        }
                        Expression::Undefined => Some(Expression::Undefined),
                        Expression::BigInt(_) => None,
                        _ => None,
                    }
                }
            },
            "Boolean" => Some(Expression::Bool(match arguments.first() {
                Some(CallArgument::Expression(argument) | CallArgument::Spread(argument)) => {
                    self.resolve_static_boolean_expression(argument)?
                }
                None => false,
            })),
            "Math.abs" => Some(Expression::Number(
                self.resolve_static_builtin_math_argument_number(
                    arguments.first()?,
                    current_function_name,
                )?
                .abs(),
            )),
            "Math.atan" => Some(Expression::Number(
                self.resolve_static_builtin_math_argument_number(
                    arguments.first()?,
                    current_function_name,
                )?
                .atan(),
            )),
            "Math.ceil" => Some(Expression::Number(
                self.resolve_static_builtin_math_argument_number(
                    arguments.first()?,
                    current_function_name,
                )?
                .ceil(),
            )),
            "Math.exp" => Some(Expression::Number(
                self.resolve_static_builtin_math_argument_number(
                    arguments.first()?,
                    current_function_name,
                )?
                .exp(),
            )),
            "Math.floor" => Some(Expression::Number(
                self.resolve_static_builtin_math_argument_number(
                    arguments.first()?,
                    current_function_name,
                )?
                .floor(),
            )),
            "Math.max" => Some(Expression::Number(self.resolve_static_math_extremum(
                arguments,
                current_function_name,
                true,
            )?)),
            "Math.min" => Some(Expression::Number(self.resolve_static_math_extremum(
                arguments,
                current_function_name,
                false,
            )?)),
            "Math.pow" => Some(Expression::Number(
                self.resolve_static_builtin_math_argument_number(
                    arguments.first()?,
                    current_function_name,
                )?
                .powf(self.resolve_static_builtin_math_argument_number(
                    arguments.get(1)?,
                    current_function_name,
                )?),
            )),
            "Math.sin" => Some(Expression::Number(
                self.resolve_static_builtin_math_argument_number(
                    arguments.first()?,
                    current_function_name,
                )?
                .sin(),
            )),
            _ => None,
        }
    }
}
