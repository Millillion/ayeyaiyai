use super::*;

thread_local! {
    static STATEFUL_OBJECT_BINDING_RESOLUTION_STACK: std::cell::RefCell<Vec<Expression>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

struct StatefulObjectBindingResolutionGuard;

impl StatefulObjectBindingResolutionGuard {
    fn enter(expression: &Expression) -> Option<Self> {
        let reentered = STATEFUL_OBJECT_BINDING_RESOLUTION_STACK.with(|stack| {
            stack
                .borrow()
                .iter()
                .any(|visited| static_expression_matches(visited, expression))
        });
        if reentered {
            return None;
        }
        STATEFUL_OBJECT_BINDING_RESOLUTION_STACK.with(|stack| {
            stack.borrow_mut().push(expression.clone());
        });
        Some(Self)
    }
}

impl Drop for StatefulObjectBindingResolutionGuard {
    fn drop(&mut self) {
        STATEFUL_OBJECT_BINDING_RESOLUTION_STACK.with(|stack| {
            stack.borrow_mut().pop();
        });
    }
}

impl<'a> FunctionCompiler<'a> {
    fn rematerialize_call_like_expression_with_state(
        &self,
        expression: &Expression,
        environment: &StaticResolutionEnvironment,
    ) -> Option<Expression> {
        match expression {
            Expression::Call { callee, arguments } => Some(Expression::Call {
                callee: Box::new(
                    self.materialize_static_expression_with_state(callee, environment)?,
                ),
                arguments: arguments
                    .iter()
                    .map(|argument| match argument {
                        CallArgument::Expression(expression) => self
                            .materialize_static_expression_with_state(expression, environment)
                            .map(CallArgument::Expression),
                        CallArgument::Spread(expression) => self
                            .materialize_static_expression_with_state(expression, environment)
                            .map(CallArgument::Spread),
                    })
                    .collect::<Option<Vec<_>>>()?,
            }),
            Expression::New { callee, arguments } => Some(Expression::New {
                callee: Box::new(
                    self.materialize_static_expression_with_state(callee, environment)?,
                ),
                arguments: arguments
                    .iter()
                    .map(|argument| match argument {
                        CallArgument::Expression(expression) => self
                            .materialize_static_expression_with_state(expression, environment)
                            .map(CallArgument::Expression),
                        CallArgument::Spread(expression) => self
                            .materialize_static_expression_with_state(expression, environment)
                            .map(CallArgument::Spread),
                    })
                    .collect::<Option<Vec<_>>>()?,
            }),
            _ => None,
        }
    }

    pub(in crate::backend::direct_wasm) fn resolve_object_binding_entries_with_state(
        &self,
        entries: &[ObjectEntry],
        environment: &mut StaticResolutionEnvironment,
    ) -> Option<ObjectValueBinding> {
        resolve_structural_object_binding(
            entries,
            environment,
            |expression, environment| {
                if matches!(
                    expression,
                    Expression::Identifier(name)
                        if self
                            .runtime_object_property_shadow_owner_name_for_identifier(name)
                            .is_some()
                ) {
                    return Some(expression.clone());
                }
                if self.resolve_iterator_source_kind(expression).is_some() {
                    return Some(expression.clone());
                }
                self.evaluate_static_expression_with_state(expression, environment)
                    .or_else(|| {
                        self.materialize_static_expression_with_state(expression, environment)
                    })
            },
            |expression, _environment| {
                self.iterator_step_member_static_value_binding_candidates(expression)
                    .iter()
                    .any(|candidate| {
                        matches!(candidate, Expression::Object(_) | Expression::Array(_))
                            || self
                                .resolve_object_binding_from_expression(candidate)
                                .is_some()
                    })
            },
            |spread_expression, _environment| {
                matches!(
                    spread_expression,
                    Expression::Identifier(name)
                        if name == "undefined"
                            && self.is_unshadowed_builtin_identifier(name)
                )
            },
            |spread_expression, environment| {
                let resolve_copy_data_properties =
                    |source: &Expression, environment: &mut StaticResolutionEnvironment| {
                        resolve_copy_data_properties_binding(
                            source,
                            environment,
                            |expression, environment| {
                                self.resolve_object_binding_from_expression_with_state(
                                    expression,
                                    environment,
                                )
                            },
                            |object, property, environment| {
                                let binding =
                                    self.resolve_member_getter_binding(object, property)?;
                                let context = self.static_eval_context();
                                execute_static_user_function_binding_in_environment(
                                    &context,
                                    &binding,
                                    &[],
                                    environment,
                                    StaticFunctionEffectMode::Commit,
                                )
                            },
                        )
                    };
                for candidate in
                    self.iterator_step_member_static_value_binding_candidates(spread_expression)
                {
                    let materialized_candidate = self
                        .materialize_static_expression_with_state(&candidate, environment)
                        .unwrap_or_else(|| self.materialize_static_expression(&candidate));
                    let binding =
                        resolve_copy_data_properties(&materialized_candidate, environment);
                    if let Some(binding) = binding {
                        return Some(binding);
                    }
                }
                let materialized_spread_expression = self
                    .materialize_static_expression_with_state(spread_expression, environment)
                    .unwrap_or_else(|| self.materialize_static_expression(spread_expression));
                resolve_copy_data_properties(&materialized_spread_expression, environment)
                    .or_else(|| resolve_copy_data_properties(spread_expression, environment))
            },
        )
    }

    fn resolve_raw_member_value_with_state(
        &self,
        object: &Expression,
        property: &Expression,
        environment: &StaticResolutionEnvironment,
    ) -> Option<Expression> {
        let Expression::Identifier(object_name) = object else {
            return None;
        };
        let property = self
            .materialize_static_expression_with_state(property, environment)
            .unwrap_or_else(|| self.materialize_static_expression(property));
        if let Some(object_binding) = environment.object_binding(object_name)
            && let Some(value) = object_binding_lookup_value(object_binding, &property)
        {
            return Some(value.clone());
        }
        let object_value = environment
            .local_binding(object_name)
            .or_else(|| environment.global_value_binding(object_name))?;
        let Expression::Object(entries) = object_value else {
            return None;
        };
        for entry in entries {
            let ObjectEntry::Data { key, value } = entry else {
                continue;
            };
            let key = self
                .materialize_static_expression_with_state(key, environment)
                .unwrap_or_else(|| self.materialize_static_expression(key));
            if static_expression_matches(&key, &property) {
                return Some(value.clone());
            }
        }
        None
    }

    pub(in crate::backend::direct_wasm) fn resolve_object_binding_from_expression_with_state(
        &self,
        expression: &Expression,
        environment: &mut StaticResolutionEnvironment,
    ) -> Option<ObjectValueBinding> {
        let _guard = StatefulObjectBindingResolutionGuard::enter(expression)?;

        if let Expression::Member { object, property } = expression {
            if let Some(value) =
                self.resolve_raw_member_value_with_state(object, property, environment)
                && !static_expression_matches(&value, expression)
            {
                if let Expression::Identifier(name) = &value
                    && let Some(binding) = self.resolve_runtime_shadow_object_binding(name)
                {
                    return Some(binding);
                }
                if let Some(binding) =
                    self.resolve_object_binding_from_expression_with_state(&value, environment)
                {
                    return Some(binding);
                }
            }
        }

        if let Expression::Await(value) = expression {
            if let Some(binding) =
                self.resolve_object_binding_from_expression_with_state(value, environment)
            {
                return Some(binding);
            }
            let materialized = self
                .materialize_static_expression_with_state(value, environment)
                .unwrap_or_else(|| self.materialize_static_expression(value));
            if let Some(binding) =
                self.resolve_object_binding_from_expression_with_state(&materialized, environment)
            {
                return Some(binding);
            }
            if let Some(StaticEvalOutcome::Value(value)) = self
                .resolve_static_await_resolution_outcome(&Expression::Await(Box::new(materialized)))
            {
                return self.resolve_object_binding_from_expression_with_state(&value, environment);
            }
        }

        if let Some(rematerialized) =
            self.rematerialize_call_like_expression_with_state(expression, environment)
            && !static_expression_matches(&rematerialized, expression)
        {
            return self
                .resolve_object_binding_from_expression_with_state(&rematerialized, environment);
        }

        if let Some(descriptor) =
            self.resolve_descriptor_binding_from_expression_with_state(expression, environment)
        {
            return Some(self.object_binding_from_property_descriptor(&descriptor));
        }

        resolve_stateful_object_binding_from_environment(
            expression,
            environment,
            &|expression, environment| {
                resolve_specialized_object_binding_expression(
                    expression,
                    environment,
                    |expression, _| self.resolve_array_binding_from_expression(expression),
                    |entries, environment| {
                        let mut environment = environment.fork();
                        self.resolve_object_binding_entries_with_state(entries, &mut environment)
                    },
                    |expression, environment| {
                        matches!(
                            expression,
                            Expression::Call { callee, .. }
                                if matches!(
                                    self.resolve_bound_alias_expression_with_state(
                                        callee,
                                        environment,
                                    )
                                    .as_ref(),
                                    Some(Expression::Member { object, property })
                                        if matches!(object.as_ref(), Expression::Identifier(name) if name == "Object")
                                            && matches!(property.as_ref(), Expression::String(name) if name == "create")
                                )
                        )
                    },
                    |expression, _| self.resolve_object_binding_from_expression(expression),
                )
            },
        )
        .or_else(|| self.resolve_object_binding_from_expression(expression))
    }
}
