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
        let trace_user_calls = std::env::var_os("AYY_TRACE_USER_CALLS").is_some();
        if trace_user_calls {
            eprintln!("finalize_user_call:start target={}", user_function.name);
        }
        self.sync_user_function_capture_source_bindings(
            prepared_capture_bindings,
            assigned_nonlocal_bindings,
            call_effect_nonlocal_bindings,
            updated_nonlocal_bindings,
            updated_bindings,
            saved_this_shadow_owner,
        )?;
        if trace_user_calls {
            eprintln!(
                "finalize_user_call:after_sync_captures target={}",
                user_function.name
            );
        }
        self.restore_user_function_capture_bindings(prepared_capture_bindings);
        if trace_user_calls {
            eprintln!(
                "finalize_user_call:after_restore_captures target={}",
                user_function.name
            );
        }
        self.invalidate_raw_assigned_global_metadata_after_user_call(user_function);
        if trace_user_calls {
            eprintln!(
                "finalize_user_call:after_invalidate_raw_assigned target={}",
                user_function.name
            );
        }
        let additional_call_effect_nonlocal_bindings = self
            .sync_snapshot_user_function_call_effect_bindings(
                &additional_call_effect_nonlocal_bindings,
                updated_bindings,
                updated_bindings
                    .map(|_| assigned_nonlocal_binding_results.as_ref())
                    .flatten(),
            )?;
        if trace_user_calls {
            eprintln!(
                "finalize_user_call:after_snapshot_effect_sync target={} additional={}",
                user_function.name,
                additional_call_effect_nonlocal_bindings.len()
            );
        }
        self.sync_current_function_capture_runtime_values_for_call_effects(
            call_effect_nonlocal_bindings,
        )?;
        if trace_user_calls {
            eprintln!(
                "finalize_user_call:after_current_capture_effects target={}",
                user_function.name
            );
        }
        if !additional_call_effect_nonlocal_bindings.is_empty() {
            self.invalidate_static_binding_metadata_for_names(
                &additional_call_effect_nonlocal_bindings,
            );
        }
        if trace_user_calls {
            eprintln!(
                "finalize_user_call:after_additional_invalidation target={}",
                user_function.name
            );
        }
        self.sync_static_with_scope_member_assignment_effects(user_function);
        if trace_user_calls {
            eprintln!(
                "finalize_user_call:after_scope_member_effects target={}",
                user_function.name
            );
        }
        self.sync_consumed_iterator_bindings_for_user_call(user_function);
        if trace_user_calls {
            eprintln!(
                "finalize_user_call:after_consumed_iterators target={}",
                user_function.name
            );
        }
        self.sync_argument_iterator_bindings_for_user_call(user_function, argument_expressions);
        if trace_user_calls {
            eprintln!(
                "finalize_user_call:after_argument_iterators target={}",
                user_function.name
            );
        }
        self.sync_direct_arguments_assignments_from_static_user_call(
            user_function,
            argument_expressions,
        );
        if trace_user_calls {
            eprintln!(
                "finalize_user_call:after_direct_arguments target={}",
                user_function.name
            );
        }
        if !user_function.lexical_this {
            let allow_static_this_shadow_commit = self
                .user_function_call_allows_static_this_shadow_commit(
                    user_function,
                    this_expression,
                );
            if trace_user_calls {
                eprintln!(
                    "finalize_user_call:before_this_shadow target={} allow_static={}",
                    user_function.name, allow_static_this_shadow_commit
                );
            }
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
                argument_expressions,
            )?;
            if trace_user_calls {
                eprintln!(
                    "finalize_user_call:after_this_shadow target={}",
                    user_function.name
                );
            }
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
        if trace_user_calls {
            eprintln!(
                "finalize_user_call:after_throw_check target={}",
                user_function.name
            );
        }
        self.push_local_get(return_value_local);
        if trace_user_calls {
            eprintln!("finalize_user_call:done target={}", user_function.name);
        }
        Ok(())
    }
}
