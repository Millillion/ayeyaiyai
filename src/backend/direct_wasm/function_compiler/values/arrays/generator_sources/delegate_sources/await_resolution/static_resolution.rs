use super::*;

impl<'a> FunctionCompiler<'a> {
    pub(in crate::backend::direct_wasm) fn resolve_static_await_resolution_outcome(
        &self,
        resolution: &Expression,
    ) -> Option<StaticEvalOutcome> {
        let current_function_name = self.current_function_name();
        if let Expression::Await(value) = resolution {
            let materialized = self.materialize_static_expression(value);
            return self
                .resolve_static_await_resolution_outcome(&materialized)
                .or(Some(StaticEvalOutcome::Value(materialized)));
        }
        if let Expression::New { callee, arguments } = resolution
            && matches!(callee.as_ref(), Expression::Identifier(name) if name == "Promise")
            && let Some(outcome) =
                self.resolve_static_promise_constructor_outcome(arguments, current_function_name)
        {
            return Some(outcome);
        }
        if let Expression::Call { callee, arguments } = resolution {
            if let Some(binding) = self.resolve_function_binding_from_expression_with_context(
                callee,
                current_function_name,
            ) {
                match &binding {
                    LocalFunctionBinding::Builtin(name) if name == "Promise.resolve" => {
                        let settled_argument = arguments.first().map(|argument| match argument {
                            CallArgument::Expression(expression)
                            | CallArgument::Spread(expression) => {
                                self.materialize_static_expression(expression)
                            }
                        });
                        return Some(match settled_argument {
                            Some(argument) => self
                                .resolve_static_await_resolution_outcome(&argument)
                                .unwrap_or(StaticEvalOutcome::Value(argument)),
                            None => StaticEvalOutcome::Value(Expression::Undefined),
                        });
                    }
                    LocalFunctionBinding::Builtin(name) if name == "Promise.reject" => {
                        let settled_argument = arguments.first().map(|argument| match argument {
                            CallArgument::Expression(expression)
                            | CallArgument::Spread(expression) => {
                                self.materialize_static_expression(expression)
                            }
                        });
                        return Some(StaticEvalOutcome::Throw(StaticThrowValue::Value(
                            settled_argument.unwrap_or(Expression::Undefined),
                        )));
                    }
                    LocalFunctionBinding::User(_) => {
                        let call_arguments = self.expand_call_arguments(arguments);
                        let this_binding = match callee.as_ref() {
                            Expression::Member { object, .. } => {
                                self.materialize_static_expression(object)
                            }
                            Expression::SuperMember { .. } => Expression::This,
                            _ => Expression::Undefined,
                        };
                        if let Some(value) = self
                            .resolve_function_binding_static_return_expression_with_call_frame(
                                &binding,
                                &call_arguments,
                                &this_binding,
                            )
                        {
                            return self
                                .resolve_static_await_resolution_outcome(&value)
                                .or(Some(StaticEvalOutcome::Value(value)));
                        }
                    }
                    _ => {}
                }
                if let Some(outcome) = self
                    .resolve_static_function_outcome_from_binding_with_context(
                        &binding,
                        arguments,
                        current_function_name,
                    )
                {
                    return Some(match outcome {
                        StaticEvalOutcome::Value(value) => self
                            .resolve_static_await_resolution_outcome(&value)
                            .unwrap_or(StaticEvalOutcome::Value(value)),
                        StaticEvalOutcome::Throw(throw_value) => {
                            StaticEvalOutcome::Throw(throw_value)
                        }
                    });
                }
            }
            if let Expression::Member { object, property } = callee.as_ref()
                && matches!(object.as_ref(), Expression::Identifier(name) if name == "Promise")
                && let Expression::String(property_name) = property.as_ref()
            {
                let settled_argument = arguments.first().map(|argument| match argument {
                    CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                        self.materialize_static_expression(expression)
                    }
                });
                match property_name.as_str() {
                    "resolve" => {
                        return Some(match settled_argument {
                            Some(argument) => self
                                .resolve_static_await_resolution_outcome(&argument)
                                .unwrap_or(StaticEvalOutcome::Value(argument)),
                            None => StaticEvalOutcome::Value(Expression::Undefined),
                        });
                    }
                    "reject" => {
                        return Some(StaticEvalOutcome::Throw(StaticThrowValue::Value(
                            settled_argument.unwrap_or(Expression::Undefined),
                        )));
                    }
                    _ => {}
                }
            }
            if let Some(result) = self.resolve_static_call_result_expression(callee, arguments) {
                return self
                    .resolve_static_await_resolution_outcome(&result)
                    .or(Some(StaticEvalOutcome::Value(result)));
            }
        }
        let materialized = self.materialize_static_expression(resolution);
        if !static_expression_matches(&materialized, resolution) {
            return self.resolve_static_await_resolution_outcome(&materialized);
        }
        if let Expression::Call { callee, arguments } = &materialized
            && let Expression::Member { object, property } = callee.as_ref()
            && matches!(object.as_ref(), Expression::Identifier(name) if name == "Promise")
            && let Expression::String(property_name) = property.as_ref()
        {
            let settled_argument = arguments.first().map(|argument| match argument {
                CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                    self.materialize_static_expression(expression)
                }
            });
            match property_name.as_str() {
                "resolve" => {
                    return Some(match settled_argument {
                        Some(argument) => self
                            .resolve_static_await_resolution_outcome(&argument)
                            .unwrap_or(StaticEvalOutcome::Value(argument)),
                        None => StaticEvalOutcome::Value(Expression::Undefined),
                    });
                }
                "reject" => {
                    return Some(StaticEvalOutcome::Throw(StaticThrowValue::Value(
                        settled_argument.unwrap_or(Expression::Undefined),
                    )));
                }
                _ => {}
            }
        }
        if self
            .resolve_static_primitive_expression_with_context(&materialized, current_function_name)
            .is_some()
        {
            return Some(StaticEvalOutcome::Value(materialized));
        }
        if !self.static_expression_is_object_like(&materialized) {
            return Some(StaticEvalOutcome::Value(materialized));
        }

        let then_property = Expression::String("then".to_string());
        let mut snapshot_bindings = HashMap::new();
        let then_outcome = match &materialized {
            Expression::Object(entries) => self
                .resolve_bound_snapshot_object_member_outcome(
                    entries,
                    &then_property,
                    &mut snapshot_bindings,
                    current_function_name,
                )
                .or_else(|| {
                    self.resolve_static_property_get_outcome(&materialized, &then_property)
                })?,
            _ => self.resolve_static_property_get_outcome(&materialized, &then_property)?,
        };
        match then_outcome {
            StaticEvalOutcome::Throw(throw_value) => Some(StaticEvalOutcome::Throw(throw_value)),
            StaticEvalOutcome::Value(then_value) => {
                if matches!(then_value, Expression::Undefined | Expression::Null) {
                    return Some(StaticEvalOutcome::Value(materialized));
                }
                let Some(binding) = self.resolve_function_binding_from_expression_with_context(
                    &then_value,
                    current_function_name,
                ) else {
                    return Some(StaticEvalOutcome::Value(materialized));
                };
                if let Some(outcome) = self.resolve_bound_snapshot_thenable_outcome(
                    &binding,
                    &materialized,
                    &mut snapshot_bindings,
                    current_function_name,
                ) {
                    return Some(outcome);
                }
                match self.resolve_static_function_outcome_from_binding_with_context(
                    &binding,
                    &[],
                    current_function_name,
                )? {
                    StaticEvalOutcome::Throw(throw_value) => {
                        Some(StaticEvalOutcome::Throw(throw_value))
                    }
                    StaticEvalOutcome::Value(_) => None,
                }
            }
        }
    }

    fn resolve_static_promise_constructor_outcome(
        &self,
        arguments: &[CallArgument],
        current_function_name: Option<&str>,
    ) -> Option<StaticEvalOutcome> {
        let executor = match arguments.first()? {
            CallArgument::Expression(expression) => expression,
            CallArgument::Spread(_) => return None,
        };
        let materialized_executor = self.materialize_static_expression(executor);
        let binding = self
            .resolve_function_binding_from_expression_with_context(executor, current_function_name)
            .or_else(|| {
                self.resolve_function_binding_from_expression_with_context(
                    &materialized_executor,
                    current_function_name,
                )
            })?;
        let LocalFunctionBinding::User(function_name) = binding else {
            return None;
        };
        let function = self.resolve_registered_function_declaration(&function_name)?;
        let resolve_name = function.params.first().map(|param| param.name.as_str())?;
        let reject_name = function.params.get(1).map(|param| param.name.as_str());

        for statement in &function.body {
            if let Some(outcome) =
                self.resolve_static_promise_executor_statement_outcome(
                    statement,
                    resolve_name,
                    reject_name,
                )
            {
                return Some(outcome);
            }
        }
        None
    }

    fn resolve_static_promise_executor_statement_outcome(
        &self,
        statement: &Statement,
        resolve_name: &str,
        reject_name: Option<&str>,
    ) -> Option<StaticEvalOutcome> {
        match statement {
            Statement::Expression(expression) | Statement::Return(expression) => self
                .resolve_static_promise_executor_expression_outcome(
                    expression,
                    resolve_name,
                    reject_name,
                ),
            Statement::Block { body } | Statement::Declaration { body } => {
                for statement in body {
                    if let Some(outcome) = self.resolve_static_promise_executor_statement_outcome(
                        statement,
                        resolve_name,
                        reject_name,
                    ) {
                        return Some(outcome);
                    }
                }
                None
            }
            _ => None,
        }
    }

    fn resolve_static_promise_executor_expression_outcome(
        &self,
        expression: &Expression,
        resolve_name: &str,
        reject_name: Option<&str>,
    ) -> Option<StaticEvalOutcome> {
        let Expression::Call { callee, arguments } = expression else {
            return None;
        };
        let Expression::Identifier(callee_name) = callee.as_ref() else {
            return None;
        };
        let settled_argument = arguments.first().map(|argument| match argument {
            CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                self.materialize_static_expression(expression)
            }
        });
        if callee_name == resolve_name {
            let value = settled_argument.unwrap_or(Expression::Undefined);
            return Some(
                self.resolve_static_await_resolution_outcome(&value)
                    .unwrap_or(StaticEvalOutcome::Value(value)),
            );
        }
        if reject_name.is_some_and(|reject_name| callee_name == reject_name) {
            return Some(StaticEvalOutcome::Throw(StaticThrowValue::Value(
                settled_argument.unwrap_or(Expression::Undefined),
            )));
        }
        None
    }
}
