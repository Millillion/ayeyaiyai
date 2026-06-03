use super::*;

impl<'a> FunctionCompiler<'a> {
    fn static_object_key_may_match_string_property(key: &Expression, property_name: &str) -> bool {
        match key {
            Expression::String(key_name) => key_name == property_name,
            Expression::Sequence(expressions) => expressions.last().map_or(true, |key| {
                Self::static_object_key_may_match_string_property(key, property_name)
            }),
            Expression::Member { object, .. } if matches!(object.as_ref(), Expression::Identifier(name) if name == "Symbol") => {
                false
            }
            Expression::Number(_)
            | Expression::BigInt(_)
            | Expression::Bool(_)
            | Expression::Null
            | Expression::Undefined => false,
            _ => true,
        }
    }

    fn static_delegate_object_lacks_step_method(
        &self,
        expression: &Expression,
        property: &Expression,
    ) -> bool {
        if let Expression::Identifier(name) = expression {
            if let Some(object_binding) = self
                .state
                .speculation
                .static_semantics
                .local_object_binding(name)
                .or_else(|| self.global_object_binding(name))
            {
                return object_binding_lookup_value(object_binding, property).is_none()
                    && object_binding_lookup_descriptor(object_binding, property).is_none();
            }
            return false;
        }
        let Expression::Object(entries) = expression else {
            return false;
        };
        let Expression::String(property_name) = property else {
            return false;
        };
        for entry in entries {
            match entry {
                ObjectEntry::Data { key, .. }
                | ObjectEntry::Getter { key, .. }
                | ObjectEntry::Setter { key, .. } => {
                    if Self::static_object_key_may_match_string_property(key, property_name) {
                        return false;
                    }
                }
                ObjectEntry::Spread(_) => return false,
            }
        }
        true
    }

    fn static_delegate_snapshot_value_lacks_step_method(
        &self,
        expression: &Expression,
        property: &Expression,
        snapshot_bindings: &HashMap<String, Expression>,
    ) -> bool {
        if self.static_delegate_object_lacks_step_method(expression, property) {
            return true;
        }
        let Expression::Identifier(name) = expression else {
            return false;
        };
        snapshot_bindings
            .get(name)
            .or_else(|| {
                self.state
                    .speculation
                    .static_semantics
                    .local_value_binding(name)
            })
            .or_else(|| self.global_value_binding(name))
            .is_some_and(|value| {
                !static_expression_matches(value, expression)
                    && self.static_delegate_object_lacks_step_method(value, property)
            })
    }

    fn direct_static_snapshot_or_value_alias(
        &self,
        expression: &Expression,
        snapshot_bindings: Option<&HashMap<String, Expression>>,
    ) -> Option<Expression> {
        let Expression::Identifier(name) = expression else {
            return None;
        };
        snapshot_bindings
            .and_then(|bindings| bindings.get(name))
            .or_else(|| {
                self.state
                    .speculation
                    .static_semantics
                    .local_value_binding(name)
            })
            .or_else(|| self.global_value_binding(name))
            .filter(|value| !static_expression_matches(value, expression))
            .cloned()
    }

    fn resolve_static_promise_chain_await_outcome(
        &self,
        expression: &Expression,
        snapshot_bindings: Option<&HashMap<String, Expression>>,
    ) -> Option<StaticEvalOutcome> {
        self.resolve_static_promise_chain_await_outcome_with_depth(expression, snapshot_bindings, 0)
    }

    fn resolve_static_promise_chain_await_outcome_with_depth(
        &self,
        expression: &Expression,
        snapshot_bindings: Option<&HashMap<String, Expression>>,
        depth: usize,
    ) -> Option<StaticEvalOutcome> {
        if depth > 8 {
            return None;
        }
        if let Some(alias) =
            self.direct_static_snapshot_or_value_alias(expression, snapshot_bindings)
        {
            return self.resolve_static_promise_chain_await_outcome_with_depth(
                &alias,
                snapshot_bindings,
                depth + 1,
            );
        }
        let Expression::Call { callee, arguments } = expression else {
            return Some(StaticEvalOutcome::Value(expression.clone()));
        };
        let Expression::Member { object, property } = callee.as_ref() else {
            return Some(StaticEvalOutcome::Value(expression.clone()));
        };
        if matches!(object.as_ref(), Expression::Identifier(name) if name == "Promise") {
            let Expression::String(property_name) = property.as_ref() else {
                return Some(StaticEvalOutcome::Value(expression.clone()));
            };
            let argument = arguments
                .first()
                .map(|argument| argument.expression().clone())
                .unwrap_or(Expression::Undefined);
            return match property_name.as_str() {
                "resolve" => Some(StaticEvalOutcome::Value(argument)),
                "reject" => Some(StaticEvalOutcome::Throw(StaticThrowValue::Value(argument))),
                _ => Some(StaticEvalOutcome::Value(expression.clone())),
            };
        }
        let Expression::String(property_name) = property.as_ref() else {
            return Some(StaticEvalOutcome::Value(expression.clone()));
        };
        if property_name != "then" {
            return Some(StaticEvalOutcome::Value(expression.clone()));
        }
        let base_outcome = self.resolve_static_promise_chain_await_outcome_with_depth(
            object,
            snapshot_bindings,
            depth + 1,
        )?;
        let handler_index = match base_outcome {
            StaticEvalOutcome::Value(_) => 0,
            StaticEvalOutcome::Throw(_) => 1,
        };
        let Some(handler) = arguments.get(handler_index).map(CallArgument::expression) else {
            return Some(base_outcome);
        };
        if matches!(handler, Expression::Undefined | Expression::Null) {
            return Some(base_outcome);
        }
        let handler_argument = match &base_outcome {
            StaticEvalOutcome::Value(value) => value.clone(),
            StaticEvalOutcome::Throw(throw_value) => {
                self.resolve_static_throw_value_expression(throw_value)?
            }
        };
        let handler_binding = self.resolve_function_binding_from_expression(handler)?;
        let handler_outcome = self.resolve_static_function_outcome_from_binding_with_context(
            &handler_binding,
            &[CallArgument::Expression(handler_argument)],
            self.current_function_name(),
        )?;
        match handler_outcome {
            StaticEvalOutcome::Value(value) => self
                .resolve_static_promise_chain_await_outcome_with_depth(
                    &value,
                    snapshot_bindings,
                    depth + 1,
                )
                .or(Some(StaticEvalOutcome::Value(value))),
            StaticEvalOutcome::Throw(throw_value) => Some(StaticEvalOutcome::Throw(throw_value)),
        }
    }

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
                })
                .or_else(|| {
                    delegate_snapshot_bindings
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
                })
                .or_else(|| {
                    self.resolve_member_function_binding(
                        &delegate_iterator_expression,
                        &delegate_property_expression,
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
                if let Expression::Object(entries) = expression {
                    return entries.iter().any(|entry| {
                        let ObjectEntry::Getter { key, .. } = entry else {
                            return false;
                        };
                        compiler
                            .resolve_property_key_expression(key)
                            .is_some_and(|key| {
                                static_expression_matches(&key, &done_property)
                                    || static_expression_matches(&key, &value_property)
                            })
                    });
                }
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
            let snapshot_lacks_step_method = delegate_step_getter_resolution.is_none()
                && snapshot_delegate_step_binding.is_none()
                && (snapshot_bindings
                    .get(delegate_iterator_name.as_str())
                    .is_some_and(|delegate_iterator| {
                        self.static_delegate_snapshot_value_lacks_step_method(
                            delegate_iterator,
                            &delegate_property_expression,
                            snapshot_bindings,
                        )
                    })
                    || self.static_delegate_snapshot_value_lacks_step_method(
                        &delegate_iterator_expression,
                        &delegate_property_expression,
                        snapshot_bindings,
                    )
                    || self.static_delegate_snapshot_value_lacks_step_method(
                        &plan.delegate_expression,
                        &delegate_property_expression,
                        snapshot_bindings,
                    ));
            let mut resolved_method_value = None;
            if snapshot_lacks_step_method {
                delegate_step_method_missing = true;
            } else if let Some((getter_binding, getter_this_expression)) =
                delegate_step_getter_resolution.as_ref()
            {
                match self.resolve_bound_snapshot_function_outcome_with_arguments_and_this(
                    getter_binding,
                    snapshot_bindings,
                    &[],
                    getter_this_expression,
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
            let missing_return_value = match self.resolve_static_promise_chain_await_outcome(
                &snapshot_current_argument,
                delegate_snapshot_bindings.as_ref(),
            ) {
                Some(StaticEvalOutcome::Value(value)) => value,
                Some(StaticEvalOutcome::Throw(throw_value)) => {
                    self.persist_async_yield_delegate_generator_snapshot_state(
                        &binding_name,
                        Some(2),
                        delegate_snapshot_bindings,
                    );
                    self.sync_persisted_async_yield_delegate_generator_snapshot_state(
                        &binding_name,
                    )?;
                    self.pop_async_delegate_snapshot_scope_bindings(&scoped_snapshot_names);
                    return Ok(Some(StaticEvalOutcome::Throw(throw_value)));
                }
                None => snapshot_current_argument.clone(),
            };
            if let Some(snapshot_bindings) = delegate_snapshot_bindings.as_mut() {
                snapshot_bindings.insert(promise_done_name.clone(), Expression::Bool(true));
                snapshot_bindings.insert(promise_value_name.clone(), missing_return_value.clone());
                self.update_local_value_binding(&promise_done_name, &Expression::Bool(true));
                self.state
                    .speculation
                    .static_semantics
                    .set_local_value_binding(&promise_value_name, missing_return_value);
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
            static_step_result_expression,
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
        let mut pre_resolved_done_expression = None;
        let mut pre_resolved_done_throw = None;
        if let (Expression::Object(entries), Some(snapshot_bindings)) = (
            &static_step_result_expression,
            delegate_snapshot_bindings.as_mut(),
        ) {
            match self.resolve_bound_snapshot_object_member_outcome(
                entries,
                &done_property,
                snapshot_bindings,
                Some(&plan.function_name),
            ) {
                Some(StaticEvalOutcome::Value(done_value)) => {
                    snapshot_bindings.insert(promise_done_name.clone(), done_value.clone());
                    self.update_local_value_binding(&promise_done_name, &done_value);
                    pre_resolved_done_expression = Some(done_value);
                }
                Some(StaticEvalOutcome::Throw(throw_value)) => {
                    pre_resolved_done_throw = Some(throw_value);
                }
                None => {}
            }
        }
        if let Some(throw_value) = pre_resolved_done_throw {
            self.persist_async_yield_delegate_generator_snapshot_state(
                &binding_name,
                Some(2),
                delegate_snapshot_bindings,
            );
            self.sync_persisted_async_yield_delegate_generator_snapshot_state(&binding_name)?;
            self.pop_async_delegate_snapshot_scope_bindings(&scoped_snapshot_names);
            return Ok(Some(StaticEvalOutcome::Throw(throw_value)));
        }
        if let Some(done_expression) = pre_resolved_done_expression.or_else(|| {
            delegate_snapshot_bindings
                .as_ref()
                .and_then(|snapshot_bindings| snapshot_bindings.get(&promise_done_name))
                .cloned()
        }) {
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
        let mut static_done = self
            .resolve_static_boolean_expression(&Expression::Identifier(promise_done_name.clone()));
        if static_done.is_none() {
            let mut static_done_throw = None;
            let static_done_outcome =
                if let (Expression::Object(entries), Some(snapshot_bindings)) = (
                    &static_step_result_expression,
                    delegate_snapshot_bindings.as_mut(),
                ) {
                    self.resolve_bound_snapshot_object_member_outcome(
                        entries,
                        &done_property,
                        snapshot_bindings,
                        Some(&plan.function_name),
                    )
                } else {
                    self.resolve_static_property_get_outcome(
                        &static_step_result_expression,
                        &done_property,
                    )
                };
            match static_done_outcome {
                Some(StaticEvalOutcome::Value(done_value)) => {
                    static_done = self.resolve_static_boolean_expression(&done_value);
                    if static_done.is_some()
                        && let Some(snapshot_bindings) = delegate_snapshot_bindings.as_mut()
                    {
                        snapshot_bindings.insert(promise_done_name.clone(), done_value.clone());
                        self.update_local_value_binding(&promise_done_name, &done_value);
                    }
                }
                Some(StaticEvalOutcome::Throw(throw_value)) => {
                    static_done_throw = Some(throw_value);
                }
                None => {}
            }
            if let Some(throw_value) = static_done_throw {
                self.persist_async_yield_delegate_generator_snapshot_state(
                    &binding_name,
                    Some(2),
                    delegate_snapshot_bindings,
                );
                self.sync_persisted_async_yield_delegate_generator_snapshot_state(&binding_name)?;
                self.pop_async_delegate_snapshot_scope_bindings(&scoped_snapshot_names);
                return Ok(Some(StaticEvalOutcome::Throw(throw_value)));
            }
        }
        match static_done {
            Some(true) => {
                let static_value_outcome =
                    if let (Expression::Object(entries), Some(snapshot_bindings)) = (
                        &static_step_result_expression,
                        delegate_snapshot_bindings.as_mut(),
                    ) {
                        self.resolve_bound_snapshot_object_member_outcome(
                            entries,
                            &value_property,
                            snapshot_bindings,
                            Some(&plan.function_name),
                        )
                    } else {
                        self.resolve_static_property_get_outcome(
                            &static_step_result_expression,
                            &value_property,
                        )
                    };
                match static_value_outcome {
                    Some(StaticEvalOutcome::Value(value)) => {
                        if let Some(snapshot_bindings) = delegate_snapshot_bindings.as_mut() {
                            snapshot_bindings
                                .insert(delegate_completion_name.clone(), value.clone());
                            self.update_local_value_binding(&delegate_completion_name, &value);
                        }
                    }
                    Some(StaticEvalOutcome::Throw(throw_value)) => {
                        self.persist_async_yield_delegate_generator_snapshot_state(
                            &binding_name,
                            Some(2),
                            delegate_snapshot_bindings,
                        );
                        self.sync_persisted_async_yield_delegate_generator_snapshot_state(
                            &binding_name,
                        )?;
                        self.pop_async_delegate_snapshot_scope_bindings(&scoped_snapshot_names);
                        return Ok(Some(StaticEvalOutcome::Throw(throw_value)));
                    }
                    None => {}
                }
                if static_step_result_has_accessor_properties
                    && matches!(property_name.as_str(), "next" | "throw")
                    && let Some(snapshot_bindings) = delegate_snapshot_bindings.as_mut()
                    && snapshot_bindings.contains_key(&delegate_completion_name)
                {
                    self.execute_bound_snapshot_statements(
                        &plan.completion_effects,
                        snapshot_bindings,
                        Some(&plan.function_name),
                    );
                    let promise_value = self
                        .evaluate_bound_snapshot_expression(
                            &plan.completion_value,
                            snapshot_bindings,
                            Some(&plan.function_name),
                        )
                        .unwrap_or_else(|| plan.completion_value.clone());
                    snapshot_bindings.insert(promise_value_name.clone(), promise_value.clone());
                    self.update_local_value_binding(&promise_value_name, &promise_value);
                }
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
                )?
            }
            Some(false) => {
                let static_value_outcome =
                    if let (Expression::Object(entries), Some(snapshot_bindings)) = (
                        &static_step_result_expression,
                        delegate_snapshot_bindings.as_mut(),
                    ) {
                        self.resolve_bound_snapshot_object_member_outcome(
                            entries,
                            &value_property,
                            snapshot_bindings,
                            Some(&plan.function_name),
                        )
                    } else {
                        self.resolve_static_property_get_outcome(
                            &static_step_result_expression,
                            &value_property,
                        )
                    };
                match static_value_outcome {
                    Some(StaticEvalOutcome::Value(value)) => {
                        if let Some(snapshot_bindings) = delegate_snapshot_bindings.as_mut() {
                            snapshot_bindings.insert(promise_value_name.clone(), value.clone());
                            self.update_local_value_binding(&promise_value_name, &value);
                        }
                    }
                    Some(StaticEvalOutcome::Throw(throw_value)) => {
                        self.persist_async_yield_delegate_generator_snapshot_state(
                            &binding_name,
                            Some(2),
                            delegate_snapshot_bindings,
                        );
                        self.sync_persisted_async_yield_delegate_generator_snapshot_state(
                            &binding_name,
                        )?;
                        self.pop_async_delegate_snapshot_scope_bindings(&scoped_snapshot_names);
                        return Ok(Some(StaticEvalOutcome::Throw(throw_value)));
                    }
                    None => {}
                }
                self.emit_async_yield_delegate_not_done_branch(
                    delegate_snapshot_bindings.as_ref(),
                    &runtime_step_result_expression,
                    &step_result_name,
                    &promise_value_name,
                    &promise_done_name,
                )?
            }
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
