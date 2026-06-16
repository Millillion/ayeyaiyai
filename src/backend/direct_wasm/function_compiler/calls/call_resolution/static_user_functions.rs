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

    fn static_user_function_seed_object_binding(
        &self,
        name: &str,
        value: &Expression,
        environment: &mut StaticResolutionEnvironment,
    ) -> Option<ObjectValueBinding> {
        self.resolve_object_binding_from_expression_with_state(value, environment)
            .or_else(|| match value {
                Expression::Identifier(source_name) if source_name == name => self
                    .state
                    .speculation
                    .static_semantics
                    .local_object_binding(source_name)
                    .cloned()
                    .or_else(|| self.global_object_binding(source_name).cloned()),
                _ => None,
            })
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
                let normalized =
                    self.normalize_static_capture_source_binding(&source_name, source_expression);
                local_bindings.insert(source_name.clone(), normalized);
                if is_internal_user_function_identifier(&source_name)
                    && let Some(self_binding_source) = self
                        .resolve_registered_function_declaration(&source_name)
                        .and_then(|function| function.self_binding.as_deref())
                        .map(|self_binding| {
                            scoped_binding_source_name(self_binding).unwrap_or(self_binding)
                        })
                    && !self.bound_snapshot_current_function_declares_binding_source(
                        Some(function_name),
                        self_binding_source,
                    )
                {
                    local_bindings.insert(
                        self_binding_source.to_string(),
                        Expression::Identifier(source_name.clone()),
                    );
                }
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

    fn set_static_user_function_substitution_argument(
        arguments: &mut Vec<CallArgument>,
        index: usize,
        value: Expression,
    ) {
        while arguments.len() <= index {
            arguments.push(CallArgument::Expression(Expression::Undefined));
        }
        arguments[index] = CallArgument::Expression(value);
    }

    fn static_user_function_parameter_default_value(
        &self,
        default: &Expression,
        local_bindings: &mut HashMap<String, Expression>,
        function_name: &str,
    ) -> Option<Expression> {
        if !inline_summary_side_effect_free_expression(default) {
            return None;
        }
        self.evaluate_bound_snapshot_expression(default, local_bindings, Some(function_name))
            .or_else(|| {
                let substituted = self.substitute_expression_bindings(default, local_bindings);
                self.resolve_static_primitive_expression_with_context(
                    &substituted,
                    Some(function_name),
                )
                .or_else(|| Some(self.materialize_static_expression(&substituted)))
            })
    }

    fn static_call_frame_argument_preserves_parameter_identity(&self, value: &Expression) -> bool {
        if matches!(value, Expression::Identifier(_) | Expression::This) {
            return false;
        }
        matches!(
            value,
            Expression::Array(_) | Expression::Object(_) | Expression::New { .. }
        ) || self.resolve_object_binding_from_expression(value).is_some()
            || self.resolve_array_binding_from_expression(value).is_some()
    }

    fn static_call_frame_substitution_parameter_value(
        &self,
        parameter_name: &str,
        value: &Expression,
    ) -> Expression {
        if self.static_call_frame_argument_preserves_parameter_identity(value) {
            Expression::Identifier(parameter_name.to_string())
        } else {
            value.clone()
        }
    }

    fn static_execution_statement_contains_with(statement: &Statement) -> bool {
        struct WithFinder {
            found: bool,
        }
        impl crate::ir::visit::Visitor for WithFinder {
            fn visit_statement(&mut self, statement: &Statement) {
                if self.found {
                    return;
                }
                if matches!(statement, Statement::With { .. }) {
                    self.found = true;
                    return;
                }
                crate::ir::visit::walk_statement(self, statement);
            }
        }
        let mut finder = WithFinder { found: false };
        crate::ir::visit::Visitor::visit_statement(&mut finder, statement);
        finder.found
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
        // A `with` body resolves identifiers against the scope object at
        // runtime; the static executor and the define-property applier both
        // resolve lexically, so any static execution of a with-containing
        // body would mis-route scope-sensitive reads and writes.
        let trace_static_call_guard = crate::ayy_env_flag!("AYY_TRACE_STATIC_CALL_GUARD");
        if function
            .body
            .iter()
            .any(Self::static_execution_statement_contains_with)
        {
            if trace_static_call_guard {
                eprintln!("static_call_guard reject_with function={function_name}");
            }
            return None;
        }
        if Self::statements_contain_source_loop(&function.body) {
            if trace_static_call_guard {
                eprintln!("static_call_guard reject_source_loop function={function_name}");
            }
            return None;
        }
        if arguments.iter().any(|argument| {
            self.expression_contains_user_function_call_with_source_loop(argument.expression())
        }) {
            if trace_static_call_guard {
                eprintln!("static_call_guard reject_runtime_arg function={function_name}");
            }
            return None;
        }
        if trace_static_call_guard {
            eprintln!("static_call_guard allow function={function_name}");
        }
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
        let mut substitution_call_arguments = call_arguments.clone();
        let mut local_bindings = extra_local_bindings;
        self.seed_static_user_function_capture_bindings_with_sources(
            function_name,
            capture_source_bindings,
            &mut local_bindings,
        );
        if !function.params.is_empty() {
            for (index, parameter) in function.params.iter().enumerate() {
                let mut value = if parameter.rest {
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
                if !parameter.rest
                    && matches!(value, Expression::Undefined)
                    && let Some(default) = parameter.default.as_ref().or_else(|| {
                        user_function
                            .parameter_defaults
                            .get(index)
                            .and_then(Option::as_ref)
                    })
                {
                    value = self.static_user_function_parameter_default_value(
                        default,
                        &mut local_bindings,
                        function_name,
                    )?;
                }
                local_bindings.insert(parameter.name.clone(), value.clone());
                if !parameter.rest {
                    let substitution_value = self
                        .static_call_frame_substitution_parameter_value(&parameter.name, &value);
                    Self::set_static_user_function_substitution_argument(
                        &mut substitution_call_arguments,
                        index,
                        substitution_value,
                    );
                }
            }
        } else {
            for (index, parameter_name) in user_function.params.iter().enumerate() {
                let mut value = argument_values
                    .get(index)
                    .cloned()
                    .unwrap_or(Expression::Undefined);
                if matches!(value, Expression::Undefined)
                    && let Some(default) = user_function
                        .parameter_defaults
                        .get(index)
                        .and_then(Option::as_ref)
                {
                    value = self.static_user_function_parameter_default_value(
                        default,
                        &mut local_bindings,
                        function_name,
                    )?;
                }
                local_bindings.insert(parameter_name.clone(), value.clone());
                let substitution_value =
                    self.static_call_frame_substitution_parameter_value(parameter_name, &value);
                Self::set_static_user_function_substitution_argument(
                    &mut substitution_call_arguments,
                    index,
                    substitution_value,
                );
            }
        }
        let substituted_body = function
            .body
            .iter()
            .map(|statement| {
                transform_statement(self.substitute_user_function_statement_call_frame_bindings(
                    statement,
                    user_function,
                    &substitution_call_arguments,
                    this_binding,
                    &arguments_binding,
                ))
            })
            .collect::<Vec<_>>();
        let seeded_names = local_bindings.keys().cloned().collect::<Vec<_>>();
        let mut environment =
            self.snapshot_static_resolution_environment_with_local_bindings(local_bindings);
        for name in seeded_names {
            let Some(value) = environment.binding(&name).cloned() else {
                continue;
            };
            if let Some(object_binding) =
                self.static_user_function_seed_object_binding(&name, &value, &mut environment)
            {
                environment.set_local_object_binding(name, object_binding);
            }
        }
        Some(PreparedStaticUserFunctionExecution {
            substituted_body,
            environment,
        })
    }

    pub(in crate::backend::direct_wasm) fn user_function_uses_direct_arguments_object(
        &self,
        user_function: &UserFunction,
    ) -> bool {
        let Some(function) = self.resolve_registered_function_declaration(&user_function.name)
        else {
            return false;
        };
        if function.lexical_this {
            return false;
        }

        let parameter_default_expressions = function
            .params
            .iter()
            .filter_map(|parameter| parameter.default.as_ref());
        !collect_arguments_usage_from_statements_and_expressions(
            &function.body,
            parameter_default_expressions,
        )
        .indexed_slots
        .is_empty()
    }
}
