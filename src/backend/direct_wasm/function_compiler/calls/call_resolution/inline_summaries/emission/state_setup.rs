use super::*;

impl<'a> FunctionCompiler<'a> {
    pub(super) fn prepare_inline_summary_emission_state(
        &mut self,
        user_function: &UserFunction,
        arguments: &[Expression],
        _this_binding: &Expression,
    ) -> DirectResult<InlineSummaryEmissionState> {
        let prepared_capture_bindings =
            self.prepare_user_function_capture_bindings(user_function)?;
        let synced_capture_source_bindings =
            self.synced_prepared_user_function_capture_source_bindings(&prepared_capture_bindings);
        let capture_snapshot =
            self.snapshot_user_function_capture_source_bindings(&prepared_capture_bindings);
        let _capture_snapshot = capture_snapshot;
        // Explicit-call-frame inline emission already executes either substituted summary effects
        // or the lowered fallback statements directly. Precomputing bound-snapshot updates here can
        // recurse indefinitely on nested immediate-promise callback chains, so this path uses
        // conservative post-call invalidation instead of eager snapshot resolution.
        let updated_bindings = None;
        let assigned_nonlocal_bindings =
            self.collect_user_function_assigned_nonlocal_bindings(user_function);
        let mut call_effect_nonlocal_bindings =
            self.collect_user_function_call_effect_nonlocal_bindings(user_function);
        call_effect_nonlocal_bindings.extend(
            self.collect_user_function_argument_call_effect_nonlocal_bindings(
                user_function,
                arguments,
            ),
        );
        let assigned_nonlocal_binding_results = self
            .assigned_nonlocal_binding_results(&user_function.name)
            .cloned();
        let updated_nonlocal_bindings =
            self.collect_user_function_updated_nonlocal_bindings(user_function);
        let mut additional_call_effect_nonlocal_bindings = call_effect_nonlocal_bindings
            .iter()
            .filter(|name| !synced_capture_source_bindings.contains(*name))
            .cloned()
            .collect::<HashSet<_>>();
        for name in assigned_nonlocal_bindings
            .iter()
            .chain(updated_nonlocal_bindings.iter())
        {
            additional_call_effect_nonlocal_bindings.remove(name);
        }
        self.emit_prepare_user_function_capture_globals(&user_function.name)?;

        let arguments_binding = Expression::Array(
            arguments
                .iter()
                .cloned()
                .map(crate::ir::hir::ArrayElement::Expression)
                .collect(),
        );
        let (call_arguments, inline_parameter_scope_names, inline_parameter_shadow_writebacks) =
            self.prepare_inline_summary_call_arguments(
                user_function,
                arguments,
                &arguments_binding,
            )?;

        Ok(InlineSummaryEmissionState {
            prepared_capture_bindings,
            assigned_nonlocal_bindings,
            call_effect_nonlocal_bindings,
            assigned_nonlocal_binding_results,
            additional_call_effect_nonlocal_bindings,
            updated_nonlocal_bindings,
            updated_bindings,
            arguments_binding,
            call_arguments,
            inline_parameter_scope_names,
            inline_parameter_shadow_writebacks,
        })
    }

    fn prepare_inline_summary_call_arguments(
        &mut self,
        user_function: &UserFunction,
        arguments: &[Expression],
        arguments_binding: &Expression,
    ) -> DirectResult<(Vec<CallArgument>, Vec<String>, Vec<(String, String)>)> {
        let mut call_arguments = Vec::new();
        let mut inline_parameter_scope_names = Vec::new();
        let mut inline_parameter_shadow_writebacks = Vec::new();
        let visible_param_count = user_function.visible_param_count() as usize;
        for (param_index, param_name) in user_function
            .params
            .iter()
            .take(visible_param_count)
            .enumerate()
        {
            let argument = arguments
                .get(param_index)
                .cloned()
                .unwrap_or(Expression::Undefined);
            let hidden_name = self.allocate_named_hidden_local(
                &format!("inline_param_{param_name}"),
                self.infer_value_kind(&argument)
                    .unwrap_or(StaticValueKind::Unknown),
            );
            let hidden_local = self
                .state
                .runtime
                .locals
                .get(&hidden_name)
                .copied()
                .expect("inline parameter local must exist");
            self.emit_numeric_expression(&argument)?;
            self.push_local_set(hidden_local);
            self.update_capture_slot_binding_from_expression(&hidden_name, &argument)?;
            self.sync_capture_slot_runtime_object_shadows_from_expression(&hidden_name, &argument)?;
            self.state
                .emission
                .lexical_scopes
                .active_scoped_lexical_bindings
                .entry(param_name.clone())
                .or_default()
                .push(hidden_name.clone());
            match &argument {
                Expression::Identifier(source_name) => {
                    if std::env::var_os("AYY_TRACE_RUNTIME_SHADOWS").is_some() {
                        eprintln!(
                            "inline_param_writeback hidden={hidden_name} source_owner={source_name}"
                        );
                    }
                    inline_parameter_shadow_writebacks
                        .push((hidden_name.clone(), source_name.clone()));
                }
                Expression::This => {
                    if std::env::var_os("AYY_TRACE_RUNTIME_SHADOWS").is_some() {
                        eprintln!("inline_param_writeback hidden={hidden_name} source_owner=this");
                    }
                    inline_parameter_shadow_writebacks
                        .push((hidden_name.clone(), "this".to_string()));
                }
                _ => {}
            }
            call_arguments.push(CallArgument::Expression(Expression::Identifier(
                hidden_name,
            )));
            inline_parameter_scope_names.push(param_name.clone());
        }
        let arguments_shadowed = user_function.body_declares_arguments_binding
            || user_function
                .params
                .iter()
                .any(|param| param == "arguments");
        if !arguments_shadowed {
            let hidden_name =
                self.allocate_named_hidden_local("inline_arguments", StaticValueKind::Object);
            let hidden_local = self
                .state
                .runtime
                .locals
                .get(&hidden_name)
                .copied()
                .expect("inline arguments local must exist");
            self.emit_numeric_expression(arguments_binding)?;
            self.push_local_set(hidden_local);
            self.update_capture_slot_binding_from_expression(&hidden_name, arguments_binding)?;
            self.sync_capture_slot_runtime_object_shadows_from_expression(
                &hidden_name,
                arguments_binding,
            )?;
            self.state
                .emission
                .lexical_scopes
                .active_scoped_lexical_bindings
                .entry("arguments".to_string())
                .or_default()
                .push(hidden_name);
            inline_parameter_scope_names.push("arguments".to_string());
        }
        Ok((
            call_arguments,
            inline_parameter_scope_names,
            inline_parameter_shadow_writebacks,
        ))
    }

    pub(super) fn abort_inline_summary_emission_state(
        &mut self,
        state: &InlineSummaryEmissionState,
    ) {
        self.pop_scoped_lexical_bindings(&state.inline_parameter_scope_names);
        self.restore_user_function_capture_bindings(&state.prepared_capture_bindings);
    }

    pub(super) fn finalize_inline_summary_emission_state(
        &mut self,
        user_function: &UserFunction,
        arguments: &[Expression],
        state: &mut InlineSummaryEmissionState,
    ) -> DirectResult<()> {
        let visible_param_count = user_function.visible_param_count() as usize;
        if std::env::var_os("AYY_TRACE_RUNTIME_SHADOWS").is_some() {
            eprintln!(
                "inline_param_writeback_finalize visible_param_count={visible_param_count} arguments_len={} call_arguments_len={}",
                arguments.len(),
                state.call_arguments.len(),
            );
        }
        for (argument, call_argument) in arguments
            .iter()
            .take(visible_param_count)
            .zip(state.call_arguments.iter().take(visible_param_count))
        {
            let CallArgument::Expression(Expression::Identifier(hidden_name)) = call_argument
            else {
                continue;
            };
            let source_owner = match argument {
                Expression::Identifier(source_name) => source_name.as_str(),
                Expression::This => "this",
                _ => continue,
            };
            if std::env::var_os("AYY_TRACE_RUNTIME_SHADOWS").is_some() {
                eprintln!(
                    "inline_param_writeback_commit hidden={hidden_name} source_owner={source_owner}"
                );
            }
            let alias_owners = self.runtime_object_reference_alias_owner_names(source_owner);
            self.emit_runtime_object_property_shadow_copy(hidden_name, source_owner)?;
            self.sync_runtime_object_shadow_owner_static_metadata_from_expression(
                source_owner,
                &Expression::Identifier(hidden_name.clone()),
            );
            for alias_owner in alias_owners {
                if std::env::var_os("AYY_TRACE_RUNTIME_SHADOWS").is_some() {
                    eprintln!(
                        "inline_param_writeback_alias hidden={hidden_name} alias_owner={alias_owner}"
                    );
                }
                self.emit_runtime_object_property_shadow_copy(hidden_name, &alias_owner)?;
                self.sync_runtime_object_shadow_owner_static_metadata_from_expression(
                    &alias_owner,
                    &Expression::Identifier(hidden_name.clone()),
                );
            }
        }
        self.pop_scoped_lexical_bindings(&state.inline_parameter_scope_names);
        self.sync_user_function_capture_source_bindings(
            &state.prepared_capture_bindings,
            &state.assigned_nonlocal_bindings,
            &state.call_effect_nonlocal_bindings,
            &state.updated_nonlocal_bindings,
            state.updated_bindings.as_ref(),
            None,
        )?;
        self.restore_user_function_capture_bindings(&state.prepared_capture_bindings);
        // Inline emission has just emitted these writes through the ordinary assignment/update
        // paths, so their static metadata already reflects the emitted code. The opaque runtime
        // call paths still invalidate raw globals after calls; doing it here loses exact values
        // needed by subsequent inline updates in the same top-level body.
        state.additional_call_effect_nonlocal_bindings = self
            .sync_snapshot_user_function_call_effect_bindings(
                &state.additional_call_effect_nonlocal_bindings,
                state.updated_bindings.as_ref(),
                state.assigned_nonlocal_binding_results.as_ref(),
            )?;
        self.sync_current_function_capture_runtime_values_for_call_effects(
            &state.call_effect_nonlocal_bindings,
        )?;
        if !state.additional_call_effect_nonlocal_bindings.is_empty() {
            self.invalidate_static_binding_metadata_for_names(
                &state.additional_call_effect_nonlocal_bindings,
            );
        }
        self.sync_argument_iterator_bindings_for_user_call(user_function, arguments);
        Ok(())
    }
}
