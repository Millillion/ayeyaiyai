use super::*;

impl<'a> FunctionCompiler<'a> {
    #[allow(clippy::too_many_arguments)]
    pub(in crate::backend::direct_wasm) fn finalize_user_function_call(
        &mut self,
        user_function: &UserFunction,
        this_expression: &Expression,
        receiver_updated_via_parameter_writeback: bool,
        prepared_capture_bindings: &[PreparedCaptureBinding],
        assigned_nonlocal_bindings: &HashSet<String>,
        call_effect_nonlocal_bindings: &HashSet<String>,
        updated_nonlocal_bindings: &HashSet<String>,
        updated_bindings: Option<&HashMap<String, Expression>>,
        additional_call_effect_nonlocal_bindings: HashSet<String>,
        assigned_nonlocal_binding_results: Option<HashMap<String, Expression>>,
        saved_new_target_local: Option<u32>,
        saved_this_local: Option<u32>,
        saved_this_shadow_owner: Option<&str>,
        return_value_local: u32,
        argument_expressions: &[Expression],
    ) -> DirectResult<()> {
        self.sync_user_function_capture_source_bindings(
            prepared_capture_bindings,
            assigned_nonlocal_bindings,
            call_effect_nonlocal_bindings,
            updated_nonlocal_bindings,
            updated_bindings,
            saved_this_shadow_owner,
        )?;
        self.restore_user_function_capture_bindings(prepared_capture_bindings);
        let additional_call_effect_nonlocal_bindings = self
            .sync_snapshot_user_function_call_effect_bindings(
                &additional_call_effect_nonlocal_bindings,
                updated_bindings,
                updated_bindings
                    .map(|_| assigned_nonlocal_binding_results.as_ref())
                    .flatten(),
            )?;
        if !additional_call_effect_nonlocal_bindings.is_empty() {
            let preserved_kinds = additional_call_effect_nonlocal_bindings
                .iter()
                .filter(|name| !assigned_nonlocal_bindings.contains(*name))
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
        self.sync_consumed_iterator_bindings_for_user_call(user_function);
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
            let receiver_may_require_invalidation = assigned_nonlocal_bindings.contains("this")
                || updated_nonlocal_bindings.contains("this");
            self.finalize_user_function_runtime_this_shadow_state(
                user_function,
                this_expression,
                updated_bindings,
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
