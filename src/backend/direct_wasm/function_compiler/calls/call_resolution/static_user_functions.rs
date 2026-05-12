use super::*;

impl<'a> FunctionCompiler<'a> {
    fn normalize_static_capture_source_binding(
        &self,
        source_name: &str,
        source_expression: Expression,
    ) -> Expression {
        match &source_expression {
            Expression::Identifier(name) if name == source_name => {
                let snapshot = self.snapshot_bound_capture_slot_expression(source_name);
                if static_expression_matches(&snapshot, &source_expression) {
                    source_expression
                } else {
                    snapshot
                }
            }
            _ => source_expression,
        }
    }

    fn seed_static_user_function_capture_bindings_with_sources(
        &self,
        function_name: &str,
        capture_source_bindings: Option<&HashMap<String, Expression>>,
        local_bindings: &mut HashMap<String, Expression>,
    ) {
        let snapshot_updated_bindings = self
            .state
            .speculation
            .static_semantics
            .last_bound_user_function_call
            .as_ref()
            .map(|snapshot| &snapshot.updated_bindings);
        if let Some(capture_bindings) = self.user_function_capture_bindings(function_name) {
            for (source_name, hidden_name) in capture_bindings {
                let source_expression = capture_source_bindings
                    .and_then(|bindings| bindings.get(&source_name).cloned())
                    .or_else(|| {
                        let snapshot = self.snapshot_bound_capture_slot_expression(&source_name);
                        (!static_expression_matches(
                            &snapshot,
                            &Expression::Identifier(source_name.clone()),
                        ))
                        .then_some(snapshot)
                    })
                    .or_else(|| self.global_value_binding(&hidden_name).cloned())
                    .or_else(|| {
                        snapshot_updated_bindings
                            .and_then(|bindings| bindings.get(&source_name).cloned())
                    })
                    .unwrap_or_else(|| Expression::Identifier(hidden_name.clone()));
                local_bindings.insert(
                    source_name.clone(),
                    self.normalize_static_capture_source_binding(&source_name, source_expression),
                );
            }
        }
    }

    fn expand_static_user_function_call_arguments(
        &self,
        arguments: &[CallArgument],
    ) -> Vec<CallArgument> {
        self.expand_call_arguments(arguments)
            .into_iter()
            .map(CallArgument::Expression)
            .collect()
    }

    fn static_user_function_arguments_binding(arguments: &[CallArgument]) -> Expression {
        Expression::Array(
            arguments
                .iter()
                .map(|argument| match argument {
                    CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                        ArrayElement::Expression(expression.clone())
                    }
                })
                .collect(),
        )
    }

    pub(in crate::backend::direct_wasm) fn prepare_static_user_function_execution(
        &self,
        function_name: &str,
        user_function: &UserFunction,
        arguments: &[CallArgument],
        this_binding: &Expression,
        capture_source_bindings: Option<&HashMap<String, Expression>>,
        extra_local_bindings: HashMap<String, Expression>,
        mut transform_statement: impl FnMut(Statement) -> Statement,
    ) -> Option<PreparedStaticUserFunctionExecution> {
        let function = self.resolve_registered_function_declaration(function_name)?;
        let call_arguments = self.expand_static_user_function_call_arguments(arguments);
        let arguments_binding = Self::static_user_function_arguments_binding(&call_arguments);
        let argument_values = call_arguments
            .iter()
            .map(|argument| match argument {
                CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                    expression.clone()
                }
            })
            .collect::<Vec<_>>();
        let substituted_body = function
            .body
            .iter()
            .map(|statement| {
                transform_statement(self.substitute_user_function_statement_call_frame_bindings(
                    statement,
                    user_function,
                    &call_arguments,
                    this_binding,
                    &arguments_binding,
                ))
            })
            .collect::<Vec<_>>();
        let mut local_bindings = extra_local_bindings;
        if !function.params.is_empty() {
            for (index, parameter) in function.params.iter().enumerate() {
                let value = if parameter.rest {
                    Expression::Array(
                        argument_values
                            .iter()
                            .skip(index)
                            .cloned()
                            .map(ArrayElement::Expression)
                            .collect(),
                    )
                } else {
                    argument_values
                        .get(index)
                        .cloned()
                        .unwrap_or(Expression::Undefined)
                };
                local_bindings.insert(parameter.name.clone(), value);
            }
        } else {
            for (index, parameter_name) in user_function.params.iter().enumerate() {
                let value = argument_values
                    .get(index)
                    .cloned()
                    .unwrap_or(Expression::Undefined);
                local_bindings.insert(parameter_name.clone(), value);
            }
        }
        self.seed_static_user_function_capture_bindings_with_sources(
            function_name,
            capture_source_bindings,
            &mut local_bindings,
        );
        let seeded_names = local_bindings.keys().cloned().collect::<Vec<_>>();
        let mut environment =
            self.snapshot_static_resolution_environment_with_local_bindings(local_bindings);
        for name in seeded_names {
            let Some(value) = environment.binding(&name).cloned() else {
                continue;
            };
            if let Some(object_binding) =
                self.resolve_object_binding_from_expression_with_state(&value, &mut environment)
            {
                environment.set_local_object_binding(name, object_binding);
            }
        }
        Some(PreparedStaticUserFunctionExecution {
            substituted_body,
            environment,
        })
    }
}
