use super::*;

impl<'a> FunctionCompiler<'a> {
    pub(super) fn consume_prepared_async_yield_delegate_generator_promise_outcome(
        &mut self,
        prepared: PreparedAsyncDelegateConsumption,
    ) -> DirectResult<Option<StaticEvalOutcome>> {
        let PreparedAsyncDelegateConsumption {
            binding_name,
            current_static_index,
            index_local,
            property_name,
            plan,
            delegate_iterator_name,
            delegate_next_name,
            delegate_completion_name,
            delegate_iterator_expression,
            delegate_completion_expression,
            mut delegate_snapshot_bindings,
            scoped_snapshot_names,
            snapshot_current_argument,
            step_result_name,
            promise_value_name,
            promise_done_name,
        } = prepared;

        let delegate_property_expression = Expression::String(property_name.clone());
        let delegate_next_expression = Expression::Identifier(delegate_next_name.clone());
        let static_delegate_next_expression = delegate_snapshot_bindings
            .as_ref()
            .and_then(|snapshot_bindings| snapshot_bindings.get(delegate_next_name.as_str()))
            .cloned()
            .unwrap_or(delegate_next_expression.clone());
        let delegate_step_binding = match property_name.as_str() {
            "next" => self
                .resolve_function_binding_from_expression_with_context(
                    &static_delegate_next_expression,
                    Some(&plan.function_name),
                )
                .or_else(|| {
                    self.resolve_function_binding_from_expression_with_context(
                        &delegate_next_expression,
                        Some(&plan.function_name),
                    )
                }),
            "return" | "throw" => delegate_snapshot_bindings
                .as_ref()
                .and_then(|snapshot_bindings| {
                    snapshot_bindings.get(delegate_iterator_name.as_str())
                })
                .and_then(|delegate_iterator| {
                    self.resolve_member_function_binding(
                        delegate_iterator,
                        &delegate_property_expression,
                    )
                })
                .or_else(|| {
                    self.resolve_member_function_binding(
                        &delegate_iterator_expression,
                        &delegate_property_expression,
                    )
                }),
            _ => None,
        };
        let step_result_expression = Expression::Identifier(step_result_name.clone());
        let done_property = Expression::String("done".to_string());
        let value_property = Expression::String("value".to_string());
        let step_result_has_accessor_properties =
            |compiler: &FunctionCompiler<'a>, expression: &Expression| {
                compiler
                    .resolve_member_getter_binding(expression, &done_property)
                    .is_some()
                    || compiler
                        .resolve_member_getter_binding(expression, &value_property)
                        .is_some()
            };
        let static_step_result_has_accessor_properties =
            step_result_has_accessor_properties(self, &step_result_expression);
        let mut snapshot_delegate_step_binding = delegate_step_binding.clone();
        let mut delegate_step_method_missing = false;
        let mut delegate_step_method_non_callable = false;
        let mut delegate_step_method_throw = None;
        let delegate_step_getter_resolution =
            if matches!(property_name.as_str(), "return" | "throw") {
                self.resolve_member_getter_binding(
                    &delegate_iterator_expression,
                    &delegate_property_expression,
                )
                .map(|binding| (binding, delegate_iterator_expression.clone()))
                .or_else(|| {
                    self.resolve_member_getter_binding(
                        &plan.delegate_expression,
                        &delegate_property_expression,
                    )
                    .map(|binding| (binding, plan.delegate_expression.clone()))
                })
            } else {
                None
            };
        if matches!(property_name.as_str(), "return" | "throw")
            && (snapshot_delegate_step_binding.is_none()
                || delegate_step_getter_resolution.is_some())
            && let Some(snapshot_bindings) = delegate_snapshot_bindings.as_mut()
        {
            let delegate_step_member = Expression::Member {
                object: Box::new(Expression::Identifier(delegate_iterator_name.clone())),
                property: Box::new(delegate_property_expression.clone()),
            };
            if std::env::var_os("AYY_TRACE_ASYNC_DELEGATES").is_some() {
                eprintln!(
                    "async_delegate_step_method_lookup property={} iterator_snapshot={:?} identifier_getter={:?}",
                    property_name,
                    snapshot_bindings.get(delegate_iterator_name.as_str()),
                    self.resolve_member_getter_binding(
                        &delegate_iterator_expression,
                        &delegate_property_expression,
                    )
                );
            }
            let mut resolved_method_value = None;
            if let Some((getter_binding, getter_this_expression)) = delegate_step_getter_resolution
            {
                match self.resolve_bound_snapshot_function_outcome_with_arguments_and_this(
                    &getter_binding,
                    snapshot_bindings,
                    &[],
                    &getter_this_expression,
                ) {
                    Some((StaticEvalOutcome::Value(method_value), updated_bindings)) => {
                        Self::merge_bound_snapshot_updated_bindings(
                            snapshot_bindings,
                            updated_bindings,
                        );
                        resolved_method_value = Some(method_value);
                    }
                    Some((StaticEvalOutcome::Throw(throw_value), updated_bindings)) => {
                        Self::merge_bound_snapshot_updated_bindings(
                            snapshot_bindings,
                            updated_bindings,
                        );
                        delegate_step_method_throw = Some(throw_value);
                    }
                    None => {}
                }
            } else {
                resolved_method_value = self.evaluate_bound_snapshot_expression(
                    &delegate_step_member,
                    snapshot_bindings,
                    Some(&plan.function_name),
                );
            }
            if let Some(delegate_step_expression) = resolved_method_value {
                match delegate_step_expression {
                    Expression::Null | Expression::Undefined => {
                        delegate_step_method_missing = true;
                    }
                    delegate_step_expression => {
                        snapshot_delegate_step_binding = self
                            .resolve_function_binding_from_expression_with_context(
                                &delegate_step_expression,
                                Some(&plan.function_name),
                            )
                            .or_else(|| {
                                self.resolve_function_binding_from_expression(
                                    &delegate_step_expression,
                                )
                            });
                        if snapshot_delegate_step_binding.is_none() {
                            delegate_step_method_non_callable = true;
                        }
                    }
                }
            }
        }

        if let Some(throw_value) = delegate_step_method_throw {
            self.persist_async_yield_delegate_generator_snapshot_state(
                &binding_name,
                Some(2),
                delegate_snapshot_bindings,
            );
            self.sync_persisted_async_yield_delegate_generator_snapshot_state(&binding_name)?;
            self.pop_async_delegate_snapshot_scope_bindings(&scoped_snapshot_names);
            return Ok(Some(StaticEvalOutcome::Throw(throw_value)));
        }

        if delegate_step_method_missing && property_name == "return" {
            if let Some(snapshot_bindings) = delegate_snapshot_bindings.as_mut() {
                snapshot_bindings.insert(promise_done_name.clone(), Expression::Bool(true));
                snapshot_bindings.insert(
                    promise_value_name.clone(),
                    snapshot_current_argument.clone(),
                );
                self.update_local_value_binding(&promise_done_name, &Expression::Bool(true));
                self.update_local_value_binding(&promise_value_name, &snapshot_current_argument);
            }
            return self.finalize_async_yield_delegate_generator_outcome(
                &plan,
                property_name.as_str(),
                &step_result_name,
                &promise_done_name,
                &promise_value_name,
                &delegate_completion_expression,
                &binding_name,
                current_static_index,
                delegate_snapshot_bindings,
                &scoped_snapshot_names,
                false,
            );
        }

        if delegate_step_method_missing && property_name == "throw" {
            if let Some(snapshot_bindings) = delegate_snapshot_bindings.as_mut() {
                let return_property = Expression::String("return".to_string());
                let return_getter_resolution = self
                    .resolve_member_getter_binding(&delegate_iterator_expression, &return_property)
                    .map(|binding| (binding, delegate_iterator_expression.clone()))
                    .or_else(|| {
                        self.resolve_member_getter_binding(
                            &plan.delegate_expression,
                            &return_property,
                        )
                        .map(|binding| (binding, plan.delegate_expression.clone()))
                    });
                if let Some((return_getter_binding, return_getter_this_expression)) =
                    return_getter_resolution
                {
                    if let Some((_, updated_bindings)) = self
                        .resolve_bound_snapshot_function_outcome_with_arguments_and_this(
                            &return_getter_binding,
                            snapshot_bindings,
                            &[],
                            &return_getter_this_expression,
                        )
                    {
                        Self::merge_bound_snapshot_updated_bindings(
                            snapshot_bindings,
                            updated_bindings,
                        );
                    }
                } else {
                    let return_member = Expression::Member {
                        object: Box::new(Expression::Identifier(delegate_iterator_name.clone())),
                        property: Box::new(return_property),
                    };
                    let _ = self.evaluate_bound_snapshot_expression(
                        &return_member,
                        snapshot_bindings,
                        Some(&plan.function_name),
                    );
                }
            }
            self.persist_async_yield_delegate_generator_snapshot_state(
                &binding_name,
                Some(2),
                delegate_snapshot_bindings,
            );
            self.sync_persisted_async_yield_delegate_generator_snapshot_state(&binding_name)?;
            self.pop_async_delegate_snapshot_scope_bindings(&scoped_snapshot_names);
            return Ok(Some(StaticEvalOutcome::Throw(
                StaticThrowValue::NamedError("TypeError"),
            )));
        }

        if delegate_step_method_non_callable {
            self.persist_async_yield_delegate_generator_snapshot_state(
                &binding_name,
                Some(2),
                delegate_snapshot_bindings,
            );
            self.sync_persisted_async_yield_delegate_generator_snapshot_state(&binding_name)?;
            self.pop_async_delegate_snapshot_scope_bindings(&scoped_snapshot_names);
            return Ok(Some(StaticEvalOutcome::Throw(
                StaticThrowValue::NamedError("TypeError"),
            )));
        }

        let (
            _static_step_result_expression,
            static_step_result_has_accessor_properties,
            needs_runtime_step_result_call,
        ) = if let Some(snapshot_bindings) = delegate_snapshot_bindings.as_mut() {
            let resolved_delegate_step_binding =
                snapshot_delegate_step_binding.clone().or_else(|| {
                    matches!(property_name.as_str(), "return" | "throw")
                        .then(|| {
                            self.evaluate_bound_snapshot_expression(
                                &Expression::Member {
                                    object: Box::new(Expression::Identifier(
                                        delegate_iterator_name.clone(),
                                    )),
                                    property: Box::new(delegate_property_expression.clone()),
                                },
                                snapshot_bindings,
                                Some(&plan.function_name),
                            )
                        })
                        .flatten()
                        .and_then(|delegate_step_expression| {
                            self.resolve_function_binding_from_expression(&delegate_step_expression)
                        })
                });
            let static_call_outcome =
                if let Some(function_binding) = resolved_delegate_step_binding.as_ref() {
                    self.resolve_bound_snapshot_function_outcome_with_arguments_and_this(
                        function_binding,
                        snapshot_bindings,
                        &[snapshot_current_argument.clone()],
                        &delegate_iterator_expression,
                    )
                } else {
                    None
                };
            if let Some((static_call_outcome, updated_bindings)) = static_call_outcome {
                Self::merge_bound_snapshot_updated_bindings(snapshot_bindings, updated_bindings);
                match static_call_outcome {
                    StaticEvalOutcome::Value(mut static_result) => {
                        match self.resolve_bound_snapshot_await_resolution_outcome(
                            &static_result,
                            snapshot_bindings,
                            Some(&plan.function_name),
                        ) {
                            Some(StaticEvalOutcome::Value(awaited_result)) => {
                                static_result = awaited_result;
                            }
                            Some(StaticEvalOutcome::Throw(throw_value)) => {
                                self.persist_async_yield_delegate_generator_snapshot_state(
                                    &binding_name,
                                    Some(2),
                                    Some(delegate_snapshot_bindings.clone().unwrap()),
                                );
                                self.sync_persisted_async_yield_delegate_generator_snapshot_state(
                                    &binding_name,
                                )?;
                                self.pop_async_delegate_snapshot_scope_bindings(
                                    &scoped_snapshot_names,
                                );
                                return Ok(Some(StaticEvalOutcome::Throw(throw_value)));
                            }
                            None => {}
                        }
                        if !self.static_expression_is_object_like(&static_result) {
                            self.persist_async_yield_delegate_generator_snapshot_state(
                                &binding_name,
                                Some(2),
                                Some(delegate_snapshot_bindings.clone().unwrap()),
                            );
                            self.sync_persisted_async_yield_delegate_generator_snapshot_state(
                                &binding_name,
                            )?;
                            self.pop_async_delegate_snapshot_scope_bindings(&scoped_snapshot_names);
                            return Ok(Some(StaticEvalOutcome::Throw(
                                StaticThrowValue::NamedError("TypeError"),
                            )));
                        }
                        let static_result_has_accessor_properties =
                            step_result_has_accessor_properties(self, &static_result);
                        snapshot_bindings.insert(step_result_name.clone(), static_result.clone());
                        self.update_local_value_binding(&step_result_name, &static_result);
                        self.update_local_function_binding(&step_result_name, &static_result);
                        self.update_local_object_binding(&step_result_name, &static_result);
                        self.update_object_literal_member_bindings_for_value(
                            &step_result_name,
                            &static_result,
                        );
                        (static_result, static_result_has_accessor_properties, false)
                    }
                    StaticEvalOutcome::Throw(throw_value) => {
                        self.persist_async_yield_delegate_generator_snapshot_state(
                            &binding_name,
                            Some(2),
                            Some(delegate_snapshot_bindings.clone().unwrap()),
                        );
                        self.sync_persisted_async_yield_delegate_generator_snapshot_state(
                            &binding_name,
                        )?;
                        self.pop_async_delegate_snapshot_scope_bindings(&scoped_snapshot_names);
                        return Ok(Some(StaticEvalOutcome::Throw(throw_value)));
                    }
                }
            } else {
                (
                    Expression::Identifier(step_result_name.clone()),
                    static_step_result_has_accessor_properties,
                    matches!(property_name.as_str(), "return" | "throw"),
                )
            }
        } else {
            (
                Expression::Identifier(step_result_name.clone()),
                static_step_result_has_accessor_properties,
                matches!(property_name.as_str(), "return" | "throw"),
            )
        };
        let runtime_step_result_expression = Expression::Identifier(step_result_name.clone());
        if needs_runtime_step_result_call {
            self.emit_statement(&Statement::Assign {
                name: step_result_name.clone(),
                value: Expression::Call {
                    callee: Box::new(Expression::Member {
                        object: Box::new(delegate_iterator_expression.clone()),
                        property: Box::new(delegate_property_expression.clone()),
                    }),
                    arguments: vec![CallArgument::Expression(snapshot_current_argument.clone())],
                },
            })?;
        }
        if let Some(snapshot_bindings) = delegate_snapshot_bindings.as_mut() {
            self.sync_async_yield_delegate_snapshot_after_step_result(
                &plan,
                snapshot_bindings,
                property_name.as_str(),
                &step_result_name,
                &promise_done_name,
                &promise_value_name,
                &delegate_completion_name,
                &delegate_iterator_name,
                static_step_result_has_accessor_properties,
            );
        }
        if let Some(done_expression) = delegate_snapshot_bindings
            .as_ref()
            .and_then(|snapshot_bindings| snapshot_bindings.get(&promise_done_name))
            .cloned()
        {
            self.emit_statement(&Statement::Assign {
                name: promise_done_name.clone(),
                value: done_expression,
            })?;
        } else if !self.emit_async_yield_delegate_step_result_getter_assignment(
            &step_result_name,
            &runtime_step_result_expression,
            &promise_done_name,
            "done",
            delegate_snapshot_bindings.as_mut(),
            Some(plan.function_name.as_str()),
        )? {
            self.emit_statement(&Statement::Assign {
                name: promise_done_name.clone(),
                value: Expression::Member {
                    object: Box::new(runtime_step_result_expression.clone()),
                    property: Box::new(Expression::String("done".to_string())),
                },
            })?;
        }
        let static_done = self
            .resolve_static_boolean_expression(&Expression::Identifier(promise_done_name.clone()));
        match static_done {
            Some(true) => self.emit_async_yield_delegate_done_branch(
                &plan,
                delegate_snapshot_bindings.as_ref(),
                &runtime_step_result_expression,
                &step_result_name,
                &delegate_completion_name,
                &delegate_completion_expression,
                &promise_value_name,
                &promise_done_name,
                property_name.as_str(),
                index_local,
            )?,
            Some(false) => self.emit_async_yield_delegate_not_done_branch(
                delegate_snapshot_bindings.as_ref(),
                &runtime_step_result_expression,
                &step_result_name,
                &promise_value_name,
                &promise_done_name,
            )?,
            None => {
                self.emit_numeric_expression(&Expression::Identifier(promise_done_name.clone()))?;
                self.state.emission.output.instructions.push(0x04);
                self.state
                    .emission
                    .output
                    .instructions
                    .push(EMPTY_BLOCK_TYPE);
                self.push_control_frame();
                self.emit_async_yield_delegate_done_branch(
                    &plan,
                    delegate_snapshot_bindings.as_ref(),
                    &runtime_step_result_expression,
                    &step_result_name,
                    &delegate_completion_name,
                    &delegate_completion_expression,
                    &promise_value_name,
                    &promise_done_name,
                    property_name.as_str(),
                    index_local,
                )?;
                self.state.emission.output.instructions.push(0x05);
                self.emit_async_yield_delegate_not_done_branch(
                    delegate_snapshot_bindings.as_ref(),
                    &runtime_step_result_expression,
                    &step_result_name,
                    &promise_value_name,
                    &promise_done_name,
                )?;
                self.state.emission.output.instructions.push(0x0b);
                self.pop_control_frame();
            }
        }

        self.finalize_async_yield_delegate_generator_outcome(
            &plan,
            property_name.as_str(),
            &step_result_name,
            &promise_done_name,
            &promise_value_name,
            &delegate_completion_expression,
            &binding_name,
            current_static_index,
            delegate_snapshot_bindings,
            &scoped_snapshot_names,
            static_step_result_has_accessor_properties,
        )
    }
}
