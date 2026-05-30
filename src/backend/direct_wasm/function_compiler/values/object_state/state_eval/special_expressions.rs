use super::super::*;
use super::FunctionStaticEvalContext;

impl StaticSpecialExpressionSource for FunctionStaticEvalContext<'_, '_> {
    fn static_evaluate_special_expression(
        &self,
        expression: &Expression,
        environment: &mut Self::Environment,
    ) -> Option<Expression> {
        if let Some(value) = self.static_evaluate_assertion_call(expression, environment) {
            return Some(value);
        }

        if let Some(value) = self.static_evaluate_array_push_call(expression, environment) {
            return Some(value);
        }

        if let Some(value) = self.static_evaluate_array_index_of_call(expression, environment) {
            return Some(value);
        }

        if let Some(value) =
            self.static_evaluate_object_array_member_expression(expression, environment)
        {
            return Some(value);
        }

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
            let property = self
                .resolve_property_key_with_state(&property, environment)
                .unwrap_or(property);
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

        if let Expression::Call { callee, arguments } = expression
            && let Expression::Member { object, property } = callee.as_ref()
            && matches!(object.as_ref(), Expression::Identifier(name) if name == "Object")
            && matches!(property.as_ref(), Expression::String(name) if name == "freeze" || name == "seal")
        {
            let Some(CallArgument::Expression(target) | CallArgument::Spread(target)) =
                arguments.first()
            else {
                return Some(Expression::Undefined);
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
                object_binding_freeze(binding);
                return Some(Expression::Identifier(target_name));
            }

            return self
                .evaluate_expression_with_state(target, environment)
                .or_else(|| Some(self.materialize_expression(target)));
        }

        if let Expression::Call { callee, arguments } = expression {
            if let Some(binding) = self.static_builtin_object_array_call_binding_with_state(
                callee,
                arguments,
                environment,
            ) {
                return Some(array_value_binding_to_expression(binding));
            }

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
            if function_name.starts_with("__ayy_function_ctor_")
                && let Some(function) = self.registered_function_declaration(function_name)
                && let [Statement::Return(return_value)] = function.body.as_slice()
            {
                if matches!(return_value, Expression::This) {
                    return Some(if user_function.strict || user_function.lexical_this {
                        Expression::Undefined
                    } else {
                        Expression::Identifier("globalThis".to_string())
                    });
                }
                return Some(self.substitute_user_function_arguments(
                    return_value,
                    user_function,
                    &evaluated_arguments,
                ));
            }
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
    fn static_evaluate_assertion_call(
        &self,
        expression: &Expression,
        environment: &mut StaticResolutionEnvironment,
    ) -> Option<Expression> {
        let Expression::Call { callee, arguments } = expression else {
            return None;
        };
        if !Self::is_static_noop_assertion_callee(callee) {
            return None;
        }

        for argument in arguments {
            let CallArgument::Expression(expression) = argument else {
                return None;
            };
            if self
                .evaluate_expression_with_state(expression, environment)
                .or_else(|| self.materialize_expression_with_state(expression, environment))
                .is_none()
                && !inline_summary_side_effect_free_expression(expression)
            {
                return None;
            }
        }

        Some(Expression::Undefined)
    }

    fn is_static_noop_assertion_callee(callee: &Expression) -> bool {
        match callee {
            Expression::Identifier(name) => matches!(
                name.as_str(),
                "__assert"
                    | "__assertSameValue"
                    | "__assertNotSameValue"
                    | "__ayyAssertCompareArray"
            ),
            Expression::Member { object, property } => {
                matches!(object.as_ref(), Expression::Identifier(name) if name == "assert")
                    && matches!(
                        property.as_ref(),
                        Expression::String(name)
                            if matches!(name.as_str(), "sameValue" | "notSameValue" | "compareArray")
                    )
            }
            _ => false,
        }
    }

    fn static_evaluate_array_push_call(
        &self,
        expression: &Expression,
        environment: &mut StaticResolutionEnvironment,
    ) -> Option<Expression> {
        let Expression::Call { callee, arguments } = expression else {
            return None;
        };
        let Expression::Member { object, property } = callee.as_ref() else {
            return None;
        };
        if !matches!(property.as_ref(), Expression::String(name) if name == "push") {
            return None;
        }
        let target_name = self.static_mutable_array_target_name(object, environment)?;
        let mut binding = self.resolve_array_binding_with_state(
            &Expression::Identifier(target_name.clone()),
            environment,
        )?;

        for argument in arguments {
            match argument {
                CallArgument::Expression(expression) => {
                    let value = self
                        .evaluate_expression_with_state(expression, environment)
                        .or_else(|| self.materialize_expression_with_state(expression, environment))
                        .unwrap_or_else(|| expression.clone());
                    binding.values.push(Some(value));
                }
                CallArgument::Spread(expression) => {
                    let spread_value = self
                        .evaluate_expression_with_state(expression, environment)
                        .or_else(|| self.materialize_expression_with_state(expression, environment))
                        .unwrap_or_else(|| expression.clone());
                    let spread_binding =
                        self.resolve_array_binding_with_state(&spread_value, environment)?;
                    binding.values.extend(spread_binding.values);
                }
            }
        }

        environment.assign_binding_value(
            target_name.clone(),
            array_value_binding_to_expression(binding.clone()),
        );
        environment.sync_object_binding(
            &target_name,
            Some(object_binding_from_array_binding(&binding)),
        );
        Some(Expression::Number(binding.values.len() as f64))
    }

    fn static_evaluate_array_index_of_call(
        &self,
        expression: &Expression,
        environment: &mut StaticResolutionEnvironment,
    ) -> Option<Expression> {
        let Expression::Call { callee, arguments } = expression else {
            return None;
        };
        let Expression::Member { object, property } = callee.as_ref() else {
            return None;
        };
        if !matches!(property.as_ref(), Expression::String(name) if name == "indexOf") {
            return None;
        }
        let Some(
            CallArgument::Expression(search_expression) | CallArgument::Spread(search_expression),
        ) = arguments.first()
        else {
            return Some(Expression::Number(-1.0));
        };
        let search_value = self
            .evaluate_expression_with_state(search_expression, environment)
            .or_else(|| self.materialize_expression_with_state(search_expression, environment))
            .unwrap_or_else(|| search_expression.clone());
        let search_value = self
            .resolve_property_key_with_state(&search_value, environment)
            .unwrap_or(search_value);
        let array_binding = self.resolve_array_binding_with_state(object, environment)?;
        let found_index = array_binding
            .values
            .iter()
            .enumerate()
            .find_map(|(index, value)| {
                let value = value.as_ref()?;
                let value = self
                    .evaluate_expression_with_state(value, environment)
                    .or_else(|| self.materialize_expression_with_state(value, environment))
                    .unwrap_or_else(|| value.clone());
                let value = self
                    .resolve_property_key_with_state(&value, environment)
                    .unwrap_or(value);
                static_expression_matches(&value, &search_value).then_some(index as f64)
            })
            .unwrap_or(-1.0);
        Some(Expression::Number(found_index))
    }

    pub(super) fn static_mutable_array_target_name(
        &self,
        target: &Expression,
        environment: &mut StaticResolutionEnvironment,
    ) -> Option<String> {
        match target {
            Expression::Identifier(name) => Some(name.clone()),
            _ => self
                .evaluate_expression_with_state(target, environment)
                .or_else(|| self.materialize_expression_with_state(target, environment))
                .and_then(|target| match target {
                    Expression::Identifier(name) => Some(name),
                    _ => None,
                }),
        }
    }

    pub(super) fn resolve_array_binding_with_state(
        &self,
        expression: &Expression,
        environment: &StaticResolutionEnvironment,
    ) -> Option<ArrayValueBinding> {
        if let Expression::Identifier(name) = expression
            && let Some(object_binding) = environment.object_binding(name)
            && let Some(binding) = array_binding_from_object_binding(object_binding)
        {
            return Some(binding);
        }
        if let Expression::Identifier(name) = expression
            && let Some(value) = environment.binding(name)
            && !matches!(value, Expression::Identifier(alias) if alias == name)
            && let Some(binding) = self.resolve_array_binding(value)
        {
            return Some(binding);
        }
        self.resolve_array_binding(expression)
    }

    fn module_namespace_binding_module_index(binding: &ObjectValueBinding) -> Option<usize> {
        fn number_to_index(value: &Expression) -> Option<usize> {
            let Expression::Number(index) = value else {
                return None;
            };
            if index.is_finite() && *index >= 0.0 && index.fract() == 0.0 {
                Some(*index as usize)
            } else {
                None
            }
        }

        binding
            .string_properties
            .iter()
            .find_map(|(key, value)| {
                (key == "__ayy$module$namespace$moduleIndex")
                    .then(|| number_to_index(value))
                    .flatten()
            })
            .or_else(|| {
                binding
                    .property_descriptors
                    .iter()
                    .find_map(|(property, descriptor)| {
                        matches!(
                            property,
                            Expression::String(key)
                                if key == "__ayy$module$namespace$moduleIndex"
                        )
                        .then(|| descriptor.value.as_ref().and_then(number_to_index))
                        .flatten()
                    })
            })
    }

    fn module_namespace_module_index_with_state(
        &self,
        target: &Expression,
        environment: &mut StaticResolutionEnvironment,
    ) -> Option<usize> {
        self.namespace_like_module_index(target)
            .or_else(|| {
                let evaluated = self
                    .evaluate_expression_with_state(target, environment)
                    .or_else(|| self.materialize_expression_with_state(target, environment))?;
                self.namespace_like_module_index(&evaluated).or_else(|| {
                    let binding =
                        self.resolve_object_binding_with_state(&evaluated, environment)?;
                    FunctionCompiler::object_binding_has_module_namespace_marker(&binding)
                        .then(|| Self::module_namespace_binding_module_index(&binding))
                        .flatten()
                })
            })
            .or_else(|| {
                let binding = self.resolve_object_binding_with_state(target, environment)?;
                FunctionCompiler::object_binding_has_module_namespace_marker(&binding)
                    .then(|| Self::module_namespace_binding_module_index(&binding))
                    .flatten()
            })
    }

    fn static_evaluate_object_array_member_expression(
        &self,
        expression: &Expression,
        environment: &mut StaticResolutionEnvironment,
    ) -> Option<Expression> {
        let Expression::Member { object, property } = expression else {
            return None;
        };
        let property = self
            .evaluate_expression_with_state(property, environment)
            .or_else(|| self.materialize_expression_with_state(property, environment))
            .unwrap_or_else(|| property.as_ref().clone());
        if !matches!(property, Expression::String(ref name) if name == "length") {
            return None;
        }

        if let Expression::Call { callee, arguments } = object.as_ref()
            && let Some(binding) = self.static_builtin_object_array_call_binding_with_state(
                callee,
                arguments,
                environment,
            )
        {
            return Some(Expression::Number(binding.values.len() as f64));
        }

        let object = self
            .evaluate_expression_with_state(object, environment)
            .or_else(|| self.materialize_expression_with_state(object, environment))
            .unwrap_or_else(|| object.as_ref().clone());
        match object {
            Expression::Array(elements) => Some(Expression::Number(elements.len() as f64)),
            _ => self
                .resolve_array_binding_with_state(&object, environment)
                .map(|binding| Expression::Number(binding.values.len() as f64)),
        }
    }

    fn static_builtin_object_array_call_binding_with_state(
        &self,
        callee: &Expression,
        arguments: &[CallArgument],
        environment: &mut StaticResolutionEnvironment,
    ) -> Option<ArrayValueBinding> {
        let Expression::Member { object, property } = callee else {
            return None;
        };
        let Expression::Identifier(object_name) = object.as_ref() else {
            return None;
        };
        let [
            CallArgument::Expression(target) | CallArgument::Spread(target),
            ..,
        ] = arguments
        else {
            return None;
        };

        match (object_name.as_str(), property.as_ref()) {
            ("Object", Expression::String(name)) if name == "keys" => {
                self.static_enumerated_keys_binding_with_state(target, environment)
            }
            ("Object", Expression::String(name)) if name == "getOwnPropertyNames" => {
                self.static_own_property_names_binding_with_state(target, environment)
            }
            ("Object", Expression::String(name)) if name == "getOwnPropertySymbols" => {
                self.static_own_property_symbols_binding_with_state(target, environment)
            }
            ("Array", Expression::String(name)) if name == "from" => self
                .resolve_array_binding_with_state(target, environment)
                .or_else(|| self.resolve_static_typed_array_values(target)),
            ("Reflect", Expression::String(name)) if name == "ownKeys" => {
                let mut names =
                    self.static_own_property_names_binding_with_state(target, environment)?;
                if let Some(symbols) =
                    self.static_own_property_symbols_binding_with_state(target, environment)
                {
                    names.values.extend(symbols.values);
                }
                Some(names)
            }
            _ => None,
        }
    }

    fn static_enumerated_keys_binding_with_state(
        &self,
        target: &Expression,
        environment: &mut StaticResolutionEnvironment,
    ) -> Option<ArrayValueBinding> {
        if let Some(module_index) =
            self.module_namespace_module_index_with_state(target, environment)
        {
            return self
                .resolve_static_dynamic_import_namespace_own_property_names_binding(module_index);
        }
        if let Some(array_binding) = self.resolve_array_binding_with_state(target, environment) {
            return Some(enumerated_keys_from_array_binding(&array_binding));
        }
        self.resolve_object_binding_with_state(target, environment)
            .map(|binding| enumerated_keys_from_object_binding(&binding))
    }

    fn static_own_property_names_binding_with_state(
        &self,
        target: &Expression,
        environment: &mut StaticResolutionEnvironment,
    ) -> Option<ArrayValueBinding> {
        if let Some(module_index) =
            self.module_namespace_module_index_with_state(target, environment)
        {
            return self
                .resolve_static_dynamic_import_namespace_own_property_names_binding(module_index);
        }
        if let Some(array_binding) = self.resolve_array_binding_with_state(target, environment) {
            return Some(own_property_names_from_array_binding(&array_binding));
        }
        let object_binding = self.resolve_object_binding_with_state(target, environment);
        if self.resolve_function_binding(target).is_some()
            || matches!(
                target,
                Expression::Identifier(name)
                    if self.has_local_prototype_object_binding(name)
                        || self.has_global_prototype_object_binding(name)
            )
        {
            return Some(own_property_names_from_function_binding(
                object_binding.as_ref(),
            ));
        }
        object_binding.map(|binding| own_property_names_from_object_binding(&binding))
    }

    fn static_own_property_symbols_binding_with_state(
        &self,
        target: &Expression,
        environment: &mut StaticResolutionEnvironment,
    ) -> Option<ArrayValueBinding> {
        if let Some(module_index) =
            self.module_namespace_module_index_with_state(target, environment)
        {
            return Some(
                self.resolve_static_dynamic_import_namespace_own_property_symbols_binding(
                    module_index,
                ),
            );
        }
        self.resolve_object_binding_with_state(target, environment)
            .map(|binding| own_property_symbols_from_object_binding(&binding))
    }

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

pub(super) fn array_value_binding_to_expression(binding: ArrayValueBinding) -> Expression {
    Expression::Array(
        binding
            .values
            .into_iter()
            .map(|value| ArrayElement::Expression(value.unwrap_or(Expression::Undefined)))
            .collect(),
    )
}
