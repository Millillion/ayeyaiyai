use super::*;

enum DelegateMethodSnapshotResolution {
    Missing {
        bindings: HashMap<String, Expression>,
    },
    Nullish {
        bindings: HashMap<String, Expression>,
    },
    Bound {
        function_binding: LocalFunctionBinding,
        bindings: HashMap<String, Expression>,
    },
    Throw {
        throw_value: StaticThrowValue,
        bindings: HashMap<String, Expression>,
    },
}

impl<'a> FunctionCompiler<'a> {
    fn resolve_async_delegate_object_member_value(
        &self,
        expression: &Expression,
        property: &Expression,
    ) -> Option<Expression> {
        self.resolve_object_binding_from_expression(expression)
            .and_then(|object_binding| {
                object_binding_lookup_value(&object_binding, property).cloned()
            })
    }

    fn resolve_async_delegate_method_binding_from_value(
        &self,
        method_value: &Expression,
        function_name: &str,
    ) -> Result<Option<LocalFunctionBinding>, StaticThrowValue> {
        if matches!(method_value, Expression::Undefined | Expression::Null) {
            return Ok(None);
        }
        self.resolve_function_binding_from_expression_with_context(method_value, Some(function_name))
            .map(Some)
            .ok_or(StaticThrowValue::NamedError("TypeError"))
    }

    fn async_delegate_iterator_method_result_is_object(&self, value: &Expression) -> bool {
        matches!(value, Expression::Object(_) | Expression::Array(_))
            || self.resolve_object_binding_from_expression(value).is_some()
            || self.resolve_array_binding_from_expression(value).is_some()
            || self
                .resolve_function_binding_from_expression_with_context(value, self.current_function_name())
                .is_some()
            || matches!(
                self.infer_value_kind(value),
                Some(StaticValueKind::Object | StaticValueKind::Function)
            )
    }

    fn resolve_async_delegate_operand_outcome_with_context(
        &self,
        expression: &Expression,
        function_name: &str,
    ) -> Option<StaticEvalOutcome> {
        if let Expression::Call { callee, arguments } = expression {
            let materialized_callee = self.materialize_static_expression(callee);
            let binding = self
                .resolve_function_binding_from_expression_with_context(callee, Some(function_name))
                .or_else(|| {
                    (!static_expression_matches(&materialized_callee, callee)).then(|| {
                        self.resolve_function_binding_from_expression_with_context(
                            &materialized_callee,
                            Some(function_name),
                        )
                    })?
                });
            if std::env::var_os("AYY_TRACE_ASYNC_DELEGATES").is_some() {
                eprintln!(
                    "async_delegate_operand_outcome expression={expression:?} callee={callee:?} materialized_callee={materialized_callee:?} binding={binding:?}"
                );
            }
            if let Some(binding) = binding
                && let Some(outcome) = self.resolve_static_function_outcome_from_binding_with_context(
                    &binding,
                    arguments,
                    Some(function_name),
                )
            {
                return match outcome {
                    StaticEvalOutcome::Value(value) => Some(
                        self.resolve_static_await_resolution_outcome(&value)
                            .unwrap_or(StaticEvalOutcome::Value(value)),
                    ),
                    StaticEvalOutcome::Throw(throw_value) => {
                        Some(StaticEvalOutcome::Throw(throw_value))
                    }
                };
            }
        }
        self.resolve_static_await_resolution_outcome(expression)
    }

    fn resolve_async_delegate_method_snapshot_resolution(
        &self,
        expression: &Expression,
        property: &Expression,
        bindings: HashMap<String, Expression>,
        function_name: &str,
    ) -> DelegateMethodSnapshotResolution {
        let awaited_expression = match self
            .resolve_async_delegate_operand_outcome_with_context(expression, function_name)
        {
            Some(StaticEvalOutcome::Value(value)) => Some(value),
            Some(StaticEvalOutcome::Throw(throw_value)) => {
                if std::env::var_os("AYY_TRACE_ASYNC_DELEGATES").is_some() {
                    eprintln!(
                        "async_delegate_method_resolution awaited_throw expression={expression:?} property={property:?}"
                    );
                }
                return DelegateMethodSnapshotResolution::Throw {
                    throw_value,
                    bindings,
                };
            }
            None => None,
        };
        let expression = awaited_expression.as_ref().unwrap_or(expression);
        if let Some(getter_binding) = self.resolve_member_getter_binding(expression, property) {
            if std::env::var_os("AYY_TRACE_ASYNC_DELEGATES").is_some() {
                eprintln!(
                    "async_delegate_method_resolution getter expression={expression:?} property={property:?} binding={getter_binding:?}"
                );
            }
            return match self.resolve_bound_snapshot_function_outcome_with_arguments_and_this(
                &getter_binding,
                &bindings,
                &[],
                expression,
            ) {
                Some((StaticEvalOutcome::Value(method_value), updated_bindings)) => {
                    if std::env::var_os("AYY_TRACE_ASYNC_DELEGATES").is_some() {
                        eprintln!(
                            "async_delegate_method_resolution getter_value method={method_value:?}"
                        );
                    }
                    match self.resolve_async_delegate_method_binding_from_value(
                        &method_value,
                        function_name,
                    ) {
                        Ok(Some(function_binding)) => DelegateMethodSnapshotResolution::Bound {
                            function_binding,
                            bindings: updated_bindings,
                        },
                        Ok(None) => DelegateMethodSnapshotResolution::Nullish {
                            bindings: updated_bindings,
                        },
                        Err(throw_value) => DelegateMethodSnapshotResolution::Throw {
                            throw_value,
                            bindings: updated_bindings,
                        },
                    }
                }
                Some((StaticEvalOutcome::Throw(throw_value), updated_bindings)) => {
                    if std::env::var_os("AYY_TRACE_ASYNC_DELEGATES").is_some() {
                        eprintln!("async_delegate_method_resolution getter_throw");
                    }
                    DelegateMethodSnapshotResolution::Throw {
                        throw_value,
                        bindings: updated_bindings,
                    }
                }
                None => match self.resolve_static_function_outcome_from_binding_with_context(
                    &getter_binding,
                    &[],
                    Some(function_name),
                ) {
                    Some(StaticEvalOutcome::Throw(throw_value)) => {
                        DelegateMethodSnapshotResolution::Throw {
                            throw_value,
                            bindings,
                        }
                    }
                    _ => DelegateMethodSnapshotResolution::Missing { bindings },
                },
            };
        }

        if let Some(function_binding) = self.resolve_member_function_binding(expression, property) {
            if std::env::var_os("AYY_TRACE_ASYNC_DELEGATES").is_some() {
                eprintln!(
                    "async_delegate_method_resolution member_function expression={expression:?} property={property:?} binding={function_binding:?}"
                );
            }
            return DelegateMethodSnapshotResolution::Bound {
                function_binding,
                bindings,
            };
        }

        if let Some(method_value) =
            self.resolve_async_delegate_object_member_value(expression, property)
        {
            if std::env::var_os("AYY_TRACE_ASYNC_DELEGATES").is_some() {
                eprintln!(
                    "async_delegate_method_resolution object_value expression={expression:?} property={property:?} method={method_value:?}"
                );
            }
            return match self
                .resolve_async_delegate_method_binding_from_value(&method_value, function_name)
            {
                Ok(Some(function_binding)) => DelegateMethodSnapshotResolution::Bound {
                    function_binding,
                    bindings,
                },
                Ok(None) => DelegateMethodSnapshotResolution::Nullish { bindings },
                Err(throw_value) => DelegateMethodSnapshotResolution::Throw {
                    throw_value,
                    bindings,
                },
            };
        }

        DelegateMethodSnapshotResolution::Missing { bindings }
    }

    #[allow(clippy::too_many_arguments)]
    pub(in crate::backend::direct_wasm) fn initialize_async_yield_delegate_snapshot_bindings(
        &mut self,
        plan: &AsyncYieldDelegateGeneratorPlan,
        async_iterator_property: &Expression,
        iterator_property: &Expression,
        delegate_iterator_method_name: &str,
        delegate_iterator_name: &str,
        delegate_next_name: &str,
    ) -> DirectResult<Option<InitialDelegateSnapshotBindings>> {
        self.with_restored_function_static_binding_metadata(|compiler| {
            let mut initial_snapshot_bindings = HashMap::new();
            for name in collect_referenced_binding_names_from_statements(&plan.prefix_effects) {
                if !compiler.should_sync_async_delegate_snapshot_binding(&name) {
                    continue;
                }
                initial_snapshot_bindings
                    .entry(name.clone())
                    .or_insert_with(|| {
                        compiler.materialize_static_expression(&Expression::Identifier(name))
                    });
            }
            compiler.execute_bound_snapshot_statements(
                &plan.prefix_effects,
                &mut initial_snapshot_bindings,
                Some(&plan.function_name),
            );
            let iterator_binding_snapshot = match compiler
                .resolve_async_delegate_method_snapshot_resolution(
                    &plan.delegate_expression,
                    async_iterator_property,
                    initial_snapshot_bindings,
                    &plan.function_name,
                ) {
                DelegateMethodSnapshotResolution::Throw {
                    throw_value,
                    bindings,
                } => {
                    return Ok(Some(InitialDelegateSnapshotBindings::Throw {
                        throw_value,
                        bindings,
                    }));
                }
                DelegateMethodSnapshotResolution::Bound {
                    function_binding,
                    bindings,
                } => Some((function_binding, bindings)),
                DelegateMethodSnapshotResolution::Missing { bindings }
                | DelegateMethodSnapshotResolution::Nullish { bindings } => {
                    match compiler.resolve_async_delegate_method_snapshot_resolution(
                        &plan.delegate_expression,
                        iterator_property,
                        bindings,
                        &plan.function_name,
                    ) {
                        DelegateMethodSnapshotResolution::Throw {
                            throw_value,
                            bindings,
                        } => {
                            return Ok(Some(InitialDelegateSnapshotBindings::Throw {
                                throw_value,
                                bindings,
                            }));
                        }
                        DelegateMethodSnapshotResolution::Bound {
                            function_binding,
                            bindings,
                        } => Some((function_binding, bindings)),
                        DelegateMethodSnapshotResolution::Missing { .. }
                        | DelegateMethodSnapshotResolution::Nullish { .. } => None,
                    }
                }
            };
            if std::env::var_os("AYY_TRACE_ASYNC_DELEGATES").is_some() {
                eprintln!(
                    "async_delegate_init function={} iterator_binding_snapshot={}",
                    plan.function_name,
                    iterator_binding_snapshot.is_some()
                );
            }
            if let Some((function_binding, iterator_snapshot_bindings)) = iterator_binding_snapshot
            {
                if let LocalFunctionBinding::User(function_name) = &function_binding {
                    let function_expression = Expression::Identifier(function_name.clone());
                    compiler.update_local_value_binding(
                        delegate_iterator_method_name,
                        &function_expression,
                    );
                    compiler.update_local_function_binding(
                        delegate_iterator_method_name,
                        &function_expression,
                    );
                }
                match compiler.resolve_bound_snapshot_function_outcome_with_arguments_and_this(
                    &function_binding,
                    &iterator_snapshot_bindings,
                    &[],
                    &plan.delegate_expression,
                ) {
                    Some((
                        StaticEvalOutcome::Value(static_delegate_iterator),
                        mut updated_bindings,
                    )) => {
                        if let Expression::Identifier(delegate_object_name) =
                            &plan.delegate_expression
                        {
                            updated_bindings.remove(delegate_object_name);
                        }
                        if !compiler.async_delegate_iterator_method_result_is_object(
                            &static_delegate_iterator,
                        ) {
                            return Ok(Some(InitialDelegateSnapshotBindings::Throw {
                                throw_value: StaticThrowValue::NamedError("TypeError"),
                                bindings: updated_bindings,
                            }));
                        }
                        updated_bindings.insert(
                            delegate_iterator_name.to_string(),
                            static_delegate_iterator.clone(),
                        );
                        if let Some(delegate_next_value) = compiler
                            .evaluate_bound_snapshot_expression(
                                &Expression::Member {
                                    object: Box::new(Expression::Identifier(
                                        delegate_iterator_name.to_string(),
                                    )),
                                    property: Box::new(Expression::String("next".to_string())),
                                },
                                &mut updated_bindings,
                                Some(&plan.function_name),
                            )
                        {
                            updated_bindings
                                .insert(delegate_next_name.to_string(), delegate_next_value.clone());
                            compiler.update_local_value_binding(
                                delegate_next_name,
                                &delegate_next_value,
                            );
                            compiler.update_local_function_binding(
                                delegate_next_name,
                                &delegate_next_value,
                            );
                        }
                        if let LocalFunctionBinding::User(function_name) = &function_binding {
                            compiler
                                .state
                                .speculation
                                .static_semantics
                                .last_bound_user_function_call =
                                Some(BoundUserFunctionCallSnapshot {
                                    function_name: function_name.clone(),
                                    source_expression: Some(Expression::Call {
                                        callee: Box::new(Expression::Identifier(
                                            delegate_iterator_method_name.to_string(),
                                        )),
                                        arguments: Vec::new(),
                                    }),
                                    result_expression: Some(static_delegate_iterator.clone()),
                                    prototype_source_expression: None,
                                    updated_bindings: updated_bindings.clone(),
                                });
                        }
                        compiler.update_local_value_binding(
                            delegate_iterator_name,
                            &static_delegate_iterator,
                        );
                        compiler.update_local_object_binding(
                            delegate_iterator_name,
                            &static_delegate_iterator,
                        );
                        compiler.update_object_literal_member_bindings_for_value(
                            delegate_iterator_name,
                            &static_delegate_iterator,
                        );
                        Ok(Some(InitialDelegateSnapshotBindings::Ready {
                            bindings: updated_bindings,
                        }))
                    }
                    Some((StaticEvalOutcome::Throw(throw_value), updated_bindings)) => {
                        Ok(Some(InitialDelegateSnapshotBindings::Throw {
                            throw_value,
                            bindings: updated_bindings,
                        }))
                    }
                    None => {
                        if std::env::var_os("AYY_TRACE_ASYNC_DELEGATES").is_some() {
                            eprintln!(
                                "async_delegate_init function={} final_snapshot_outcome=false",
                                plan.function_name
                            );
                        }
                        Ok(None)
                    }
                }
            } else {
                if std::env::var_os("AYY_TRACE_ASYNC_DELEGATES").is_some() {
                    eprintln!(
                        "async_delegate_init function={} final_snapshot_outcome=missing_iterator_binding",
                        plan.function_name
                    );
                }
                Ok(None)
            }
        })
    }
}
