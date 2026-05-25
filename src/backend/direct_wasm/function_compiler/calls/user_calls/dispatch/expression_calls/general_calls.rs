use super::*;
mod finalization;
mod planning;
mod runtime_call;
use self::planning::GeneralUserFunctionCallPlan;

impl<'a> FunctionCompiler<'a> {
    pub(in crate::backend::direct_wasm) fn emit_user_function_call_with_new_target_and_this_expression(
        &mut self,
        user_function: &UserFunction,
        arguments: &[CallArgument],
        new_target_value: i32,
        this_expression: &Expression,
    ) -> DirectResult<()> {
        self.emit_user_function_call_with_new_target_and_this_expression_impl(
            user_function,
            arguments,
            new_target_value,
            this_expression,
            true,
            true,
        )
    }

    pub(in crate::backend::direct_wasm) fn emit_user_function_call_with_new_target_and_this_expression_without_static_snapshot(
        &mut self,
        user_function: &UserFunction,
        arguments: &[CallArgument],
        new_target_value: i32,
        this_expression: &Expression,
    ) -> DirectResult<()> {
        self.emit_user_function_call_with_new_target_and_this_expression_impl(
            user_function,
            arguments,
            new_target_value,
            this_expression,
            false,
            false,
        )
    }

    pub(in crate::backend::direct_wasm) fn emit_user_function_call_without_inline_with_new_target_and_this_expression(
        &mut self,
        user_function: &UserFunction,
        arguments: &[CallArgument],
        new_target_value: i32,
        this_expression: &Expression,
    ) -> DirectResult<()> {
        self.emit_user_function_call_with_new_target_and_this_expression_impl(
            user_function,
            arguments,
            new_target_value,
            this_expression,
            true,
            false,
        )
    }

    fn emit_user_function_call_with_new_target_and_this_expression_impl(
        &mut self,
        user_function: &UserFunction,
        arguments: &[CallArgument],
        new_target_value: i32,
        this_expression: &Expression,
        enable_static_snapshot: bool,
        allow_inline: bool,
    ) -> DirectResult<()> {
        let expanded_arguments = self.expand_call_arguments(arguments);
        let materialized_inline_arguments = expanded_arguments
            .iter()
            .map(|argument| self.materialize_static_expression(argument))
            .collect::<Vec<_>>();
        let static_this_expression = self
            .with_suspended_with_scopes_if_active_scope_object(this_expression, |compiler| {
                Ok(compiler.resolve_static_snapshot_this_expression(this_expression))
            })?;
        if self.emit_deferred_generator_call_result(user_function, &expanded_arguments)? {
            return Ok(());
        }
        if allow_inline {
            if self.emit_inline_lowered_pattern_user_function_with_arguments(
                user_function,
                &expanded_arguments,
                this_expression,
            )? {
                return Ok(());
            }
        }
        if allow_inline && self.can_inline_user_function_call(user_function, &expanded_arguments) {
            self.with_suspended_with_scopes_if_active_scope_object(this_expression, |compiler| {
                compiler.emit_numeric_expression(this_expression)
            })?;
            self.state.emission.output.instructions.push(0x1a);
            for argument in &expanded_arguments {
                self.emit_numeric_expression(argument)?;
                self.state.emission.output.instructions.push(0x1a);
            }
            if self.emit_inline_user_function_summary_with_arguments(
                user_function,
                &materialized_inline_arguments,
            )? {
                return Ok(());
            }
        }

        let GeneralUserFunctionCallPlan {
            expanded_arguments,
            prepared_capture_bindings,
            assigned_nonlocal_bindings,
            call_effect_nonlocal_bindings,
            updated_nonlocal_bindings,
            additional_call_effect_nonlocal_bindings,
            assigned_nonlocal_binding_results,
            updated_bindings,
        } = self.prepare_general_user_function_call_plan(
            user_function,
            expanded_arguments,
            new_target_value,
            &static_this_expression,
            enable_static_snapshot,
        )?;

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
            self.with_suspended_with_scopes_if_active_scope_object(this_expression, |compiler| {
                compiler.emit_numeric_expression(this_expression)
            })?;
            self.push_local_set(this_local);
            self.push_local_get(this_local);
            self.push_global_set(CURRENT_THIS_GLOBAL_INDEX);
            Some(saved_local)
        };
        let saved_this_shadow_owner = if user_function.lexical_this {
            None
        } else {
            self.with_suspended_with_scopes_if_active_scope_object(this_expression, |compiler| {
                compiler.prepare_user_function_runtime_this_shadow_state(this_expression)
            })?
        };

        self.emit_prepare_user_function_capture_globals(&user_function.name)?;

        let (
            return_value_local,
            parameter_object_shadow_writebacks,
            static_argument_member_writebacks,
        ) = self.emit_user_function_runtime_call_from_expanded_arguments(
            user_function,
            &expanded_arguments,
            updated_bindings.as_ref(),
        )?;
        let receiver_updated_via_parameter_writeback = self
            .receiver_shadow_updated_via_parameter_writebacks(
                this_expression,
                &parameter_object_shadow_writebacks,
            );
        self.finalize_user_function_call(
            user_function,
            this_expression,
            receiver_updated_via_parameter_writeback,
            &prepared_capture_bindings,
            &assigned_nonlocal_bindings,
            &call_effect_nonlocal_bindings,
            &updated_nonlocal_bindings,
            updated_bindings.as_ref(),
            additional_call_effect_nonlocal_bindings,
            assigned_nonlocal_binding_results,
            saved_new_target_local,
            saved_this_local,
            saved_this_shadow_owner.as_deref(),
            return_value_local,
            &expanded_arguments,
        )?;
        self.sync_static_argument_object_member_writeback_values(
            &static_argument_member_writebacks,
        );
        Ok(())
    }
}
