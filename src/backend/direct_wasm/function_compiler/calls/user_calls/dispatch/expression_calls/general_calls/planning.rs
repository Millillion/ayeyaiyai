use super::*;

pub(super) struct GeneralUserFunctionCallPlan {
    pub(super) expanded_arguments: Vec<Expression>,
    pub(super) prepared_capture_bindings: Vec<PreparedCaptureBinding>,
    pub(super) assigned_nonlocal_bindings: HashSet<String>,
    pub(super) call_effect_nonlocal_bindings: HashSet<String>,
    pub(super) updated_nonlocal_bindings: HashSet<String>,
    pub(super) additional_call_effect_nonlocal_bindings: HashSet<String>,
    pub(super) assigned_nonlocal_binding_results: Option<HashMap<String, Expression>>,
    pub(super) updated_bindings: Option<HashMap<String, Expression>>,
    pub(super) skip_static_argument_member_writebacks: bool,
}

impl<'a> FunctionCompiler<'a> {
    pub(super) fn prepare_general_user_function_call_plan(
        &mut self,
        user_function: &UserFunction,
        expanded_arguments: Vec<Expression>,
        new_target_value: i32,
        static_this_expression: &Expression,
        enable_static_snapshot: bool,
    ) -> DirectResult<GeneralUserFunctionCallPlan> {
        let allow_static_snapshot = enable_static_snapshot
            && !self.user_function_mentions_private_member_access(user_function)
            && !self
                .backend
                .function_registry
                .analysis
                .user_function_capture_bindings
                .get(&user_function.name)
                .is_some_and(|bindings| !bindings.is_empty());
        let runtime_only_parameter_iterator_call = user_function.has_lowered_pattern_parameters()
            || !self
                .user_function_parameter_iterator_consumption_indices(user_function)
                .is_empty();
        let arguments_contain_await = expanded_arguments
            .iter()
            .any(Self::expression_contains_await_for_user_call_runtime);
        let runtime_only_promise_chain_call = !enable_static_snapshot
            && self.registered_function_body_mentions_promise_like_chain(&user_function.name);
        let skip_static_call_effect_analysis =
            runtime_only_parameter_iterator_call || arguments_contain_await;
        let prepared_capture_bindings =
            self.prepare_user_function_capture_bindings(user_function)?;
        let synced_capture_source_bindings =
            self.synced_prepared_user_function_capture_source_bindings(&prepared_capture_bindings);
        let capture_snapshot =
            self.snapshot_user_function_capture_source_bindings(&prepared_capture_bindings);
        let static_result = (!runtime_only_parameter_iterator_call
            && !arguments_contain_await
            && allow_static_snapshot
            && new_target_value == JS_UNDEFINED_TAG)
            .then(|| {
                self.resolve_bound_snapshot_user_function_result_with_arguments_and_this(
                    &user_function.name,
                    &capture_snapshot,
                    &expanded_arguments,
                    static_this_expression,
                )
            })
            .flatten();
        let existing_snapshot = self
            .state
            .speculation
            .static_semantics
            .last_bound_user_function_call
            .clone()
            .filter(|snapshot| snapshot.function_name == user_function.name);
        let updated_bindings = static_result
            .as_ref()
            .map(|(_, updated_bindings)| updated_bindings.clone())
            .or_else(|| {
                allow_static_snapshot
                    .then(|| {
                        existing_snapshot
                            .as_ref()
                            .map(|snapshot| snapshot.updated_bindings.clone())
                            .filter(|bindings| !bindings.is_empty())
                    })
                    .flatten()
            });
        self.state
            .speculation
            .static_semantics
            .last_bound_user_function_call = if !runtime_only_parameter_iterator_call
            && !arguments_contain_await
            && allow_static_snapshot
        {
            Some(BoundUserFunctionCallSnapshot {
                function_name: user_function.name.clone(),
                source_expression: None,
                result_expression: static_result.as_ref().map(|(result, _)| result.clone()),
                prototype_source_expression: None,
                updated_bindings: updated_bindings
                    .clone()
                    .unwrap_or_else(|| capture_snapshot.clone()),
            })
        } else {
            existing_snapshot
        };

        let assigned_nonlocal_bindings = if skip_static_call_effect_analysis {
            HashSet::new()
        } else {
            self.collect_user_function_assigned_nonlocal_bindings(user_function)
        };
        let mut call_effect_nonlocal_bindings = if skip_static_call_effect_analysis {
            HashSet::new()
        } else {
            self.collect_user_function_call_effect_nonlocal_bindings(user_function)
        };
        if !skip_static_call_effect_analysis {
            call_effect_nonlocal_bindings.extend(
                self.collect_user_function_argument_call_effect_nonlocal_bindings(
                    user_function,
                    &expanded_arguments,
                ),
            );
        }
        let assigned_nonlocal_binding_results = if skip_static_call_effect_analysis {
            None
        } else {
            self.assigned_nonlocal_binding_results(&user_function.name)
                .cloned()
        };
        let mut additional_call_effect_nonlocal_bindings = if skip_static_call_effect_analysis {
            HashSet::new()
        } else {
            let mut names = call_effect_nonlocal_bindings
                .iter()
                .filter(|name| !synced_capture_source_bindings.contains(*name))
                .cloned()
                .collect::<HashSet<_>>();
            names.extend(self.collect_snapshot_updated_nonlocal_bindings(
                user_function,
                updated_bindings.as_ref(),
            ));
            names
        };
        additional_call_effect_nonlocal_bindings
            .retain(|name| !synced_capture_source_bindings.contains(name));
        let updated_nonlocal_bindings = if skip_static_call_effect_analysis {
            HashSet::new()
        } else {
            self.collect_user_function_updated_nonlocal_bindings(user_function)
        };
        if std::env::var_os("AYY_TRACE_CONSTRUCT_CALLS").is_some() {
            eprintln!(
                "call_plan:function={} new_target={} assigned={assigned_nonlocal_bindings:?} call_effect={call_effect_nonlocal_bindings:?} updated={updated_nonlocal_bindings:?} additional={additional_call_effect_nonlocal_bindings:?} static_snapshot={enable_static_snapshot}",
                user_function.name, new_target_value
            );
        }

        Ok(GeneralUserFunctionCallPlan {
            expanded_arguments,
            prepared_capture_bindings,
            assigned_nonlocal_bindings,
            call_effect_nonlocal_bindings,
            updated_nonlocal_bindings,
            additional_call_effect_nonlocal_bindings,
            assigned_nonlocal_binding_results,
            updated_bindings,
            skip_static_argument_member_writebacks: runtime_only_promise_chain_call,
        })
    }
}
