use super::*;

impl<'a> FunctionCompiler<'a> {
    pub(in crate::backend::direct_wasm) fn prepare_bound_user_function_call_context(
        &mut self,
        user_function: &UserFunction,
        capture_slots: &BTreeMap<String, String>,
        new_target_value: i32,
        this_expression: &Expression,
    ) -> DirectResult<(
        Vec<PreparedBoundCaptureBinding>,
        HashSet<String>,
        Option<u32>,
        Option<u32>,
        Option<String>,
    )> {
        let prepared_capture_bindings =
            self.prepare_bound_user_function_capture_bindings(user_function, capture_slots)?;
        let synced_capture_source_bindings = self
            .synced_prepared_bound_user_function_capture_source_bindings(
                &prepared_capture_bindings,
            );
        self.sync_static_with_scope_member_assignment_effects(user_function);

        let saved_new_target_local = if user_function.lexical_this {
            None
        } else {
            let saved_local = self.allocate_temp_local();
            self.push_global_get(CURRENT_NEW_TARGET_GLOBAL_INDEX);
            self.push_local_set(saved_local);
            self.push_i32_const(new_target_value);
            self.push_global_set(CURRENT_NEW_TARGET_GLOBAL_INDEX);
            Some(saved_local)
        };
        let saved_this_local = if user_function.lexical_this {
            None
        } else {
            let saved_local = self.allocate_temp_local();
            let this_local = self.allocate_temp_local();
            self.push_global_get(CURRENT_THIS_GLOBAL_INDEX);
            self.push_local_set(saved_local);
            self.emit_numeric_expression(this_expression)?;
            self.push_local_set(this_local);
            self.push_local_get(this_local);
            self.push_global_set(CURRENT_THIS_GLOBAL_INDEX);
            Some(saved_local)
        };
        let saved_this_shadow_owner = if user_function.lexical_this {
            None
        } else {
            self.prepare_user_function_runtime_this_shadow_state(this_expression)?
        };

        Ok((
            prepared_capture_bindings,
            synced_capture_source_bindings,
            saved_new_target_local,
            saved_this_local,
            saved_this_shadow_owner,
        ))
    }

    pub(in crate::backend::direct_wasm) fn finalize_bound_user_function_call(
        &mut self,
        user_function: &UserFunction,
        this_expression: &Expression,
        receiver_updated_via_parameter_writeback: bool,
        prepared_capture_bindings: &[PreparedBoundCaptureBinding],
        updated_bindings: Option<HashMap<String, Expression>>,
        additional_call_effect_nonlocal_bindings: HashSet<String>,
        assigned_nonlocal_binding_results: Option<HashMap<String, Expression>>,
        saved_new_target_local: Option<u32>,
        saved_this_local: Option<u32>,
        saved_this_shadow_owner: Option<&str>,
        return_value_local: u32,
        argument_expressions: &[Expression],
    ) -> DirectResult<()> {
        self.sync_bound_user_function_capture_slots(
            user_function,
            prepared_capture_bindings,
            updated_bindings.as_ref(),
            saved_this_shadow_owner,
        )?;
        if let Some(saved_this_shadow_owner) = saved_this_shadow_owner
            && prepared_capture_bindings
                .iter()
                .any(|binding| binding.source_binding_name.as_deref() == Some("this"))
        {
            self.emit_runtime_object_property_shadow_copy("this", saved_this_shadow_owner)?;
        }
        self.restore_bound_user_function_capture_bindings(prepared_capture_bindings);

        let additional_call_effect_nonlocal_bindings = self
            .sync_snapshot_user_function_call_effect_bindings(
                &additional_call_effect_nonlocal_bindings,
                updated_bindings.as_ref(),
                updated_bindings
                    .as_ref()
                    .map(|_| assigned_nonlocal_binding_results.as_ref())
                    .flatten(),
            )?;
        if !additional_call_effect_nonlocal_bindings.is_empty() {
            let preserved_kinds = additional_call_effect_nonlocal_bindings
                .iter()
                .filter_map(|name| {
                    self.lookup_identifier_kind(name)
                        .map(|kind| (name.clone(), kind))
                })
                .collect::<HashMap<_, _>>();
            self.invalidate_static_binding_metadata_for_names_with_preserved_kinds(
                &additional_call_effect_nonlocal_bindings,
                &preserved_kinds,
            );
        }
        self.sync_static_with_scope_member_assignment_effects(user_function);

        self.sync_argument_iterator_bindings_for_user_call(user_function, argument_expressions);
        self.sync_direct_arguments_assignments_from_static_user_call(
            user_function,
            argument_expressions,
        );
        if !user_function.lexical_this {
            let allow_static_this_shadow_commit = self
                .user_function_call_allows_static_this_shadow_commit(
                    user_function,
                    this_expression,
                );
            let receiver_may_require_invalidation = updated_bindings
                .as_ref()
                .is_some_and(|bindings| bindings.contains_key("this"))
                || self
                    .collect_user_function_updated_nonlocal_bindings(user_function)
                    .contains("this");
            self.finalize_user_function_runtime_this_shadow_state(
                user_function,
                this_expression,
                updated_bindings.as_ref(),
                saved_this_shadow_owner,
                allow_static_this_shadow_commit,
                receiver_updated_via_parameter_writeback,
                receiver_may_require_invalidation,
            )?;
        }

        if let Some(saved_new_target_local) = saved_new_target_local {
            self.push_local_get(saved_new_target_local);
            self.push_global_set(CURRENT_NEW_TARGET_GLOBAL_INDEX);
        }
        if let Some(saved_this_local) = saved_this_local {
            self.push_local_get(saved_this_local);
            self.push_global_set(CURRENT_THIS_GLOBAL_INDEX);
        }
        if user_function.is_async() {
            self.push_global_get(THROW_TAG_GLOBAL_INDEX);
            self.push_i32_const(0);
            self.push_binary_op(BinaryOp::NotEqual)?;
            self.state.emission.output.instructions.push(0x04);
            self.state
                .emission
                .output
                .instructions
                .push(EMPTY_BLOCK_TYPE);
            self.push_control_frame();
            self.clear_global_throw_state();
            self.state.emission.output.instructions.push(0x0b);
            self.pop_control_frame();
            self.push_i32_const(JS_TYPEOF_OBJECT_TAG);
            return Ok(());
        }

        self.emit_check_global_throw_for_user_call()?;
        self.push_local_get(return_value_local);
        Ok(())
    }
}
