use super::super::*;
use super::FunctionStaticEvalContext;

impl StaticSpecialExpressionSource for FunctionStaticEvalContext<'_, '_> {
    fn static_evaluate_special_expression(
        &self,
        expression: &Expression,
        environment: &mut Self::Environment,
    ) -> Option<Expression> {
        if let Expression::Call { callee, arguments } = expression
            && let Expression::Member { object, property } = callee.as_ref()
            && matches!(object.as_ref(), Expression::Identifier(name) if name == "Object" || name == "Reflect")
            && matches!(property.as_ref(), Expression::String(name) if name == "defineProperty")
        {
            let reflect_call =
                matches!(object.as_ref(), Expression::Identifier(name) if name == "Reflect");
            let [
                CallArgument::Expression(target),
                CallArgument::Expression(property),
                CallArgument::Expression(descriptor),
                ..,
            ] = arguments.as_slice()
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
                    self.evaluate_expression_with_state(target, environment)
                        .or_else(|| self.materialize_expression_with_state(target, environment))
                        .unwrap_or_else(|| target.clone())
                });
            };
            let property = self
                .evaluate_expression_with_state(property, environment)
                .or_else(|| self.materialize_expression_with_state(property, environment))?;
            let target_name = self
                .static_define_property_target_name(target, environment)
                .or_else(|| {
                    resolve_stateful_object_binding_name_in_environment(target, environment)
                });
            let Some(target_name) = target_name else {
                return Some(if reflect_call {
                    Expression::Bool(false)
                } else {
                    self.evaluate_expression_with_state(target, environment)
                        .or_else(|| self.materialize_expression_with_state(target, environment))
                        .unwrap_or_else(|| target.clone())
                });
            };

            if !environment.contains_object_binding(&target_name) {
                environment.set_object_binding(target_name.clone(), empty_object_value_binding());
            }

            let descriptor_binding =
                self.static_property_descriptor_binding(&descriptor, environment);
            let binding = environment.object_binding_mut(&target_name)?;
            if !object_binding_can_define_property(binding, &property) {
                return Some(if reflect_call {
                    Expression::Bool(false)
                } else {
                    Expression::Identifier(target_name)
                });
            }
            object_binding_define_property_descriptor(binding, property, descriptor_binding);
            return Some(if reflect_call {
                Expression::Bool(true)
            } else {
                Expression::Identifier(target_name)
            });
        }

        if let Expression::Call { callee, arguments } = expression
            && let Expression::Member { object, property } = callee.as_ref()
            && matches!(object.as_ref(), Expression::Identifier(name) if name == "Object" || name == "Reflect")
            && matches!(property.as_ref(), Expression::String(name) if name == "preventExtensions")
        {
            let reflect_call =
                matches!(object.as_ref(), Expression::Identifier(name) if name == "Reflect");
            let Some(CallArgument::Expression(target) | CallArgument::Spread(target)) =
                arguments.first()
            else {
                return Some(if reflect_call {
                    Expression::Bool(false)
                } else {
                    Expression::Undefined
                });
            };

            let resolved_target_name = match target {
                Expression::This => environment
                    .contains_object_binding(FunctionCompiler::STATIC_NEW_THIS_BINDING)
                    .then(|| FunctionCompiler::STATIC_NEW_THIS_BINDING.to_string()),
                _ => resolve_stateful_object_binding_name_in_environment(target, environment),
            };
            if let Some(target_name) = resolved_target_name
                && let Some(binding) = environment.object_binding_mut(&target_name)
            {
                object_binding_prevent_extensions(binding);
                return Some(if reflect_call {
                    Expression::Bool(true)
                } else {
                    Expression::Identifier(target_name)
                });
            }

            let evaluated_target = self
                .evaluate_expression_with_state(target, environment)
                .or_else(|| Some(self.materialize_expression(target)))?;
            return Some(if reflect_call {
                Expression::Bool(true)
            } else {
                evaluated_target
            });
        }

        if let Expression::Call { callee, arguments } = expression {
            let mut evaluated_arguments = Vec::with_capacity(arguments.len());
            for argument in arguments {
                let evaluated_argument = match argument {
                    CallArgument::Expression(expression) => self
                        .evaluate_expression_with_state(expression, environment)
                        .or_else(|| self.materialize_expression_with_state(expression, environment))
                        .map(CallArgument::Expression),
                    CallArgument::Spread(_) => None,
                };
                let Some(evaluated_argument) = evaluated_argument else {
                    return None;
                };
                evaluated_arguments.push(evaluated_argument);
            }
            let resolved_callee = self
                .evaluate_expression_with_state(callee, environment)
                .or_else(|| self.materialize_expression_with_state(callee, environment))
                .unwrap_or_else(|| callee.as_ref().clone());
            let binding_from_environment =
                |expression: &Expression| -> Option<LocalFunctionBinding> {
                    let Expression::Identifier(name) = expression else {
                        return None;
                    };
                    let value = environment.binding(name)?.clone();
                    self.resolve_function_binding(&value).or_else(|| {
                        self.materialize_expression_with_state(&value, environment)
                            .and_then(|value| self.resolve_function_binding(&value))
                    })
                };
            let binding = self
                .resolve_function_binding(&resolved_callee)
                .or_else(|| self.resolve_function_binding(callee))
                .or_else(|| binding_from_environment(&resolved_callee))
                .or_else(|| binding_from_environment(callee));
            let Some(binding) = binding else {
                return None;
            };
            let LocalFunctionBinding::User(function_name) = &binding else {
                let LocalFunctionBinding::Builtin(function_name) = &binding else {
                    return None;
                };
                return match self
                    .resolve_static_builtin_function_outcome(function_name, &evaluated_arguments)?
                {
                    StaticEvalOutcome::Value(value) => Some(value),
                    StaticEvalOutcome::Throw(_) => None,
                };
            };
            let user_function = self.user_function(function_name)?;
            if user_function.is_async()
                || user_function.is_generator()
                || !user_function
                    .inline_summary
                    .as_ref()
                    .is_some_and(|summary| summary.effects.is_empty())
            {
                return None;
            }
            return execute_static_user_function_binding_in_environment(
                self,
                &binding,
                &evaluated_arguments,
                environment,
                StaticFunctionEffectMode::Discard,
            );
        }

        let Expression::SuperCall { callee, arguments } = expression else {
            return None;
        };
        let trace_constructor = std::env::var_os("AYY_TRACE_CONSTRUCTOR_BINDINGS").is_some();
        if matches!(
            environment
                .local_bindings
                .get(FunctionCompiler::STATIC_NEW_THIS_INITIALIZED_BINDING),
            Some(Expression::Bool(true))
        ) {
            if trace_constructor {
                eprintln!("static_super_call:already_initialized callee={callee:?}");
            }
            return None;
        }
        let evaluated_arguments = arguments
            .iter()
            .map(|argument| match argument {
                CallArgument::Expression(expression) => self
                    .evaluate_expression_with_state(expression, environment)
                    .map(CallArgument::Expression),
                CallArgument::Spread(expression) => self
                    .evaluate_expression_with_state(expression, environment)
                    .map(CallArgument::Spread),
            })
            .collect::<Option<Vec<_>>>()?;
        if trace_constructor {
            eprintln!("static_super_call:arguments callee={callee:?} args={evaluated_arguments:?}");
        }
        let resolved_callee = self
            .evaluate_expression_with_state(callee, environment)
            .or_else(|| self.materialize_static_expression_with_state(callee, environment))
            .unwrap_or_else(|| callee.as_ref().clone());
        if trace_constructor {
            eprintln!("static_super_call:resolved callee={callee:?} resolved={resolved_callee:?}");
        }
        let Some(binding) = self.resolve_function_binding(&resolved_callee) else {
            if trace_constructor {
                eprintln!(
                    "static_super_call:no_binding callee={callee:?} resolved={resolved_callee:?}"
                );
            }
            return None;
        };
        let LocalFunctionBinding::User(function_name) = binding else {
            if trace_constructor {
                eprintln!("static_super_call:non_user binding={binding:?}");
            }
            return None;
        };
        let user_function = self.user_function(&function_name)?;
        let current_this_binding = environment
            .object_binding(FunctionCompiler::STATIC_NEW_THIS_BINDING)
            .cloned()
            .unwrap_or_else(empty_object_value_binding);
        let capture_source_bindings =
            self.resolve_constructor_capture_source_bindings(&resolved_callee);
        let super_return_expression = self
            .resolve_user_constructor_return_expression(
                user_function,
                &evaluated_arguments,
                capture_source_bindings.as_ref(),
            )
            .filter(|expression| {
                !matches!(
                    expression,
                    Expression::Identifier(name)
                        if name == FunctionCompiler::STATIC_NEW_THIS_BINDING
                ) && self
                    .resolve_object_binding_with_state(expression, environment)
                    .is_some()
            });
        let next_this_binding = self
            .resolve_user_constructor_object_binding(
                user_function,
                &evaluated_arguments,
                capture_source_bindings.as_ref(),
                current_this_binding.clone(),
            )
            .unwrap_or(current_this_binding);
        if trace_constructor {
            eprintln!(
                "static_super_call:next_this function={function_name} props={:?}",
                ordered_object_property_names(&next_this_binding)
            );
        }
        if let Some(updated_bindings) = self.resolve_user_constructor_updated_bindings(
            user_function,
            &evaluated_arguments,
            capture_source_bindings.as_ref(),
        ) {
            for (name, value) in updated_bindings {
                environment.assign_binding_value(name, value);
            }
        }
        environment.set_local_object_binding(
            FunctionCompiler::STATIC_NEW_THIS_BINDING.to_string(),
            next_this_binding,
        );
        if let Some(super_return_expression) = super_return_expression {
            environment.set_local_binding(
                FunctionCompiler::STATIC_NEW_THIS_BINDING.to_string(),
                super_return_expression,
            );
        }
        environment.set_local_binding(
            FunctionCompiler::STATIC_NEW_THIS_INITIALIZED_BINDING.to_string(),
            Expression::Bool(true),
        );
        Some(Expression::Identifier(
            FunctionCompiler::STATIC_NEW_THIS_BINDING.to_string(),
        ))
    }
}

impl FunctionStaticEvalContext<'_, '_> {
    fn static_define_property_target_name(
        &self,
        target: &Expression,
        environment: &mut StaticResolutionEnvironment,
    ) -> Option<String> {
        match target {
            Expression::This => environment
                .contains_object_binding(FunctionCompiler::STATIC_NEW_THIS_BINDING)
                .then(|| FunctionCompiler::STATIC_NEW_THIS_BINDING.to_string())
                .or_else(|| Some("this".to_string())),
            Expression::Identifier(name) => Some(name.clone()),
            _ => self
                .evaluate_expression_with_state(target, environment)
                .or_else(|| self.materialize_expression_with_state(target, environment))
                .and_then(|resolved| match resolved {
                    Expression::Identifier(name) => Some(name),
                    _ => None,
                }),
        }
    }

    fn static_property_descriptor_binding(
        &self,
        descriptor: &PropertyDescriptorDefinition,
        environment: &mut StaticResolutionEnvironment,
    ) -> PropertyDescriptorBinding {
        let descriptor_value =
            |expression: &Expression,
             context: &Self,
             environment: &mut StaticResolutionEnvironment| {
                context
                    .evaluate_expression_with_state(expression, environment)
                    .or_else(|| context.materialize_expression_with_state(expression, environment))
                    .unwrap_or_else(|| expression.clone())
            };

        let value = descriptor
            .value
            .as_ref()
            .map(|expression| descriptor_value(expression, self, environment));
        let getter = descriptor
            .getter
            .as_ref()
            .map(|expression| descriptor_value(expression, self, environment));
        let setter = descriptor
            .setter
            .as_ref()
            .map(|expression| descriptor_value(expression, self, environment));

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
        }
    }
}
