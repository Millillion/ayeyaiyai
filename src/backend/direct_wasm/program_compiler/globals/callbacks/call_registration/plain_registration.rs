use super::*;

impl DirectWasmCompiler {
    pub(in crate::backend::direct_wasm) fn register_callback_bindings_for_call(
        &self,
        callee: &Expression,
        arguments: &[CallArgument],
        aliases: &HashMap<String, Option<LocalFunctionBinding>>,
        bindings: &mut HashMap<String, HashMap<String, Option<LocalFunctionBinding>>>,
        array_bindings: &mut HashMap<String, HashMap<String, Option<ArrayValueBinding>>>,
        object_bindings: &mut HashMap<String, HashMap<String, Option<ObjectValueBinding>>>,
        current_function_name: Option<&str>,
    ) {
        let dynamic_import_callback =
            self.dynamic_import_then_callback_namespace_argument(callee, arguments, aliases);
        let (called_function_name, call_arguments) = if let Some((
            called_function_name,
            namespace_argument,
        )) = dynamic_import_callback
        {
            (called_function_name, vec![namespace_argument])
        } else {
            match callee {
                Expression::Member { object, property } if matches!(property.as_ref(), Expression::String(name) if name == "call") =>
                {
                    let Some(LocalFunctionBinding::User(called_function_name)) =
                        self.resolve_function_binding_from_expression_with_aliases(object, aliases)
                    else {
                        return;
                    };
                    (
                        called_function_name,
                        self.expanded_global_static_call_arguments(arguments)
                            .into_iter()
                            .skip(1)
                            .collect::<Vec<_>>(),
                    )
                }
                Expression::Member { object, property } if matches!(property.as_ref(), Expression::String(name) if name == "apply") =>
                {
                    let Some(LocalFunctionBinding::User(called_function_name)) =
                        self.resolve_function_binding_from_expression_with_aliases(object, aliases)
                    else {
                        return;
                    };
                    let expanded_arguments = self.expanded_global_static_call_arguments(arguments);
                    let apply_expression = expanded_arguments
                        .get(1)
                        .cloned()
                        .unwrap_or(Expression::Undefined);
                    let Some(call_arguments) = self
                        .expand_apply_parameter_call_arguments_from_expression(&apply_expression)
                    else {
                        return;
                    };
                    (called_function_name, call_arguments)
                }
                _ => {
                    let Some(LocalFunctionBinding::User(called_function_name)) =
                        self.resolve_function_binding_from_expression_with_aliases(callee, aliases)
                    else {
                        return;
                    };
                    (
                        called_function_name,
                        self.expanded_global_static_call_arguments(arguments),
                    )
                }
            }
        };
        self.register_plain_parameter_binding_candidates(
            &called_function_name,
            &call_arguments,
            aliases,
            bindings,
            array_bindings,
            object_bindings,
            current_function_name,
        );
    }

    pub(in crate::backend::direct_wasm) fn register_constructor_bindings_for_call(
        &self,
        callee: &Expression,
        arguments: &[CallArgument],
        aliases: &HashMap<String, Option<LocalFunctionBinding>>,
        bindings: &mut HashMap<String, HashMap<String, Option<LocalFunctionBinding>>>,
        array_bindings: &mut HashMap<String, HashMap<String, Option<ArrayValueBinding>>>,
        object_bindings: &mut HashMap<String, HashMap<String, Option<ObjectValueBinding>>>,
        current_function_name: Option<&str>,
    ) {
        let constructor_binding =
            self.resolve_parameter_analysis_constructor_binding(callee, aliases);
        let Some(LocalFunctionBinding::User(called_function_name)) = constructor_binding else {
            return;
        };
        let call_arguments = self.expanded_global_static_call_arguments(arguments);

        self.register_plain_parameter_binding_candidates(
            &called_function_name,
            &call_arguments,
            aliases,
            bindings,
            array_bindings,
            object_bindings,
            current_function_name,
        );
    }

    fn register_plain_parameter_binding_candidates(
        &self,
        called_function_name: &str,
        call_arguments: &[Expression],
        aliases: &HashMap<String, Option<LocalFunctionBinding>>,
        bindings: &mut HashMap<String, HashMap<String, Option<LocalFunctionBinding>>>,
        array_bindings: &mut HashMap<String, HashMap<String, Option<ArrayValueBinding>>>,
        object_bindings: &mut HashMap<String, HashMap<String, Option<ObjectValueBinding>>>,
        current_function_name: Option<&str>,
    ) {
        let Some(user_function) = self.user_function(called_function_name) else {
            return;
        };
        let current_parameter_object_bindings = current_function_name
            .and_then(|name| object_bindings.get(name))
            .cloned()
            .unwrap_or_default();
        let Some(parameter_bindings) = bindings.get_mut(called_function_name) else {
            return;
        };
        let Some(parameter_array_bindings) = array_bindings.get_mut(called_function_name) else {
            return;
        };
        let Some(parameter_object_bindings) = object_bindings.get_mut(called_function_name) else {
            return;
        };
        let rest_parameter_index =
            self.registered_function(called_function_name)
                .and_then(|declaration| {
                    declaration
                        .params
                        .iter()
                        .position(|parameter| parameter.rest)
                });

        let mut register_candidate =
            |param_name: &str, candidate: Option<LocalFunctionBinding>| match candidate {
                None => {
                    parameter_bindings.insert(param_name.to_string(), None);
                }
                Some(binding) => match parameter_bindings.get(param_name) {
                    Some(None) => {}
                    Some(Some(existing)) if *existing == binding => {}
                    Some(Some(_)) => {
                        parameter_bindings.insert(param_name.to_string(), None);
                    }
                    None => {
                        parameter_bindings.insert(param_name.to_string(), Some(binding));
                    }
                },
            };
        let mut register_object_candidate =
            |param_name: &str, candidate: Option<ObjectValueBinding>| match candidate {
                None => {
                    parameter_object_bindings.insert(param_name.to_string(), None);
                }
                Some(binding) => match parameter_object_bindings.get(param_name) {
                    Some(None) => {}
                    Some(Some(existing)) if *existing == binding => {}
                    Some(Some(_)) => {
                        parameter_object_bindings.insert(param_name.to_string(), None);
                    }
                    None => {
                        parameter_object_bindings.insert(param_name.to_string(), Some(binding));
                    }
                },
            };
        let mut register_array_candidate =
            |param_name: &str, candidate: Option<ArrayValueBinding>| match candidate {
                None => {
                    parameter_array_bindings.insert(param_name.to_string(), None);
                }
                Some(binding) => match parameter_array_bindings.get(param_name) {
                    Some(None) => {}
                    Some(Some(existing)) if *existing == binding => {}
                    Some(Some(_)) => {
                        parameter_array_bindings.insert(param_name.to_string(), None);
                    }
                    None => {
                        parameter_array_bindings.insert(param_name.to_string(), Some(binding));
                    }
                },
            };

        let mut handled_rest_parameter = false;
        for (index, argument) in call_arguments.iter().enumerate() {
            if index >= user_function.params.len() {
                break;
            }
            let param_name = &user_function.params[index];
            if rest_parameter_index == Some(index) {
                let rest_candidate = self.rest_parameter_array_binding_candidate(
                    &call_arguments[index..],
                    &current_parameter_object_bindings,
                );
                register_candidate(param_name, None);
                register_object_candidate(param_name, None);
                if rest_candidate.is_some() {
                    register_array_candidate(param_name, rest_candidate);
                }
                handled_rest_parameter = true;
                break;
            }

            let candidate =
                self.resolve_function_binding_from_expression_with_aliases(argument, aliases);
            register_candidate(param_name, candidate);
            let global_bindings = self.snapshot_global_binding_environment();
            let materialized_argument = self
                .materialize_global_expression_with_state(
                    argument,
                    &HashMap::new(),
                    &global_bindings.value_bindings,
                    &global_bindings.object_bindings,
                )
                .unwrap_or_else(|| self.materialize_global_expression(argument));
            let stable_argument =
                Self::prepared_parameter_argument_is_stable(&materialized_argument);
            register_array_candidate(
                param_name,
                stable_argument
                    .then(|| {
                        self.infer_global_array_binding(&materialized_argument)
                            .or_else(|| self.infer_global_array_binding(argument))
                    })
                    .flatten(),
            );
            let object_candidate = if matches!(
                argument,
                Expression::Member { property, .. }
                    if matches!(property.as_ref(), Expression::String(name) if name == "prototype")
            ) {
                None
            } else if matches!(
                materialized_argument,
                Expression::Member { ref property, .. }
                    if matches!(property.as_ref(), Expression::String(name) if name == "prototype")
            ) {
                None
            } else if matches!(
                materialized_argument,
                Expression::Number(_)
                    | Expression::BigInt(_)
                    | Expression::String(_)
                    | Expression::Bool(_)
                    | Expression::Null
                    | Expression::Undefined
            ) {
                None
            } else if let Some(binding) = self
                .infer_current_or_global_object_binding(
                    argument,
                    &current_parameter_object_bindings,
                )
                .or_else(|| {
                    self.infer_current_or_global_object_binding(
                        &materialized_argument,
                        &current_parameter_object_bindings,
                    )
                })
            {
                Some(binding)
            } else if !stable_argument && !matches!(argument, Expression::Object(_)) {
                None
            } else {
                None
            };
            register_object_candidate(param_name, object_candidate);
        }

        if !handled_rest_parameter && call_arguments.len() < user_function.params.len() {
            for (index, param_name) in user_function
                .params
                .iter()
                .enumerate()
                .skip(call_arguments.len())
            {
                parameter_bindings.insert(param_name.to_string(), None);
                parameter_array_bindings.insert(
                    param_name.to_string(),
                    rest_parameter_index
                        .filter(|rest_index| *rest_index == index)
                        .map(|_| ArrayValueBinding { values: Vec::new() }),
                );
                parameter_object_bindings.insert(param_name.to_string(), None);
            }
        }
    }

    fn rest_parameter_array_binding_candidate(
        &self,
        arguments: &[Expression],
        current_parameter_object_bindings: &HashMap<String, Option<ObjectValueBinding>>,
    ) -> Option<ArrayValueBinding> {
        let values = arguments
            .iter()
            .map(|argument| {
                self.rest_parameter_array_element_expression(
                    argument,
                    current_parameter_object_bindings,
                )
                .map(Some)
            })
            .collect::<Option<Vec<_>>>()?;
        Some(ArrayValueBinding { values })
    }

    fn rest_parameter_array_element_expression(
        &self,
        argument: &Expression,
        current_parameter_object_bindings: &HashMap<String, Option<ObjectValueBinding>>,
    ) -> Option<Expression> {
        let global_bindings = self.snapshot_global_binding_environment();
        let materialized_argument = self
            .materialize_global_expression_with_state(
                argument,
                &HashMap::new(),
                &global_bindings.value_bindings,
                &global_bindings.object_bindings,
            )
            .unwrap_or_else(|| self.materialize_global_expression(argument));
        if let Some(object_binding) = self
            .infer_current_or_global_object_binding(argument, current_parameter_object_bindings)
            .or_else(|| {
                self.infer_current_or_global_object_binding(
                    &materialized_argument,
                    current_parameter_object_bindings,
                )
            })
        {
            return Some(object_binding_to_expression(&object_binding));
        }
        Self::prepared_parameter_argument_is_stable(&materialized_argument)
            .then_some(materialized_argument)
    }

    fn infer_current_or_global_object_binding(
        &self,
        expression: &Expression,
        current_parameter_object_bindings: &HashMap<String, Option<ObjectValueBinding>>,
    ) -> Option<ObjectValueBinding> {
        match expression {
            Expression::Identifier(name) => current_parameter_object_bindings
                .get(name)
                .cloned()
                .flatten()
                .or_else(|| self.infer_global_object_binding(expression)),
            _ => self.infer_global_object_binding(expression),
        }
    }
}
