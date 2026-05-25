use super::*;

#[path = "emission/fallback_path.rs"]
mod fallback_path;
#[path = "emission/state_setup.rs"]
mod state_setup;
#[path = "emission/summary_path.rs"]
mod summary_path;

struct InlineSummaryEmissionState {
    prepared_capture_bindings: Vec<PreparedCaptureBinding>,
    assigned_nonlocal_bindings: HashSet<String>,
    call_effect_nonlocal_bindings: HashSet<String>,
    assigned_nonlocal_binding_results: Option<HashMap<String, Expression>>,
    additional_call_effect_nonlocal_bindings: HashSet<String>,
    updated_nonlocal_bindings: HashSet<String>,
    updated_bindings: Option<HashMap<String, Expression>>,
    arguments_binding: Expression,
    call_arguments: Vec<CallArgument>,
    inline_parameter_scope_names: Vec<String>,
    inline_parameter_shadow_writebacks: Vec<(String, String)>,
}

impl<'a> FunctionCompiler<'a> {
    pub(in crate::backend::direct_wasm) fn emit_inline_user_function_summary_with_explicit_call_frame(
        &mut self,
        user_function: &UserFunction,
        arguments: &[Expression],
        this_binding: &Expression,
        result_local: u32,
    ) -> DirectResult<bool> {
        self.emit_inline_user_function_summary_with_explicit_call_frame_impl(
            user_function,
            arguments,
            this_binding,
            result_local,
            false,
        )
    }

    pub(in crate::backend::direct_wasm) fn emit_inline_primitive_effect_user_function_summary_with_explicit_call_frame(
        &mut self,
        user_function: &UserFunction,
        arguments: &[Expression],
        this_binding: &Expression,
        result_local: u32,
    ) -> DirectResult<bool> {
        self.emit_inline_user_function_summary_with_explicit_call_frame_impl(
            user_function,
            arguments,
            this_binding,
            result_local,
            true,
        )
    }

    fn emit_inline_user_function_summary_with_explicit_call_frame_impl(
        &mut self,
        user_function: &UserFunction,
        arguments: &[Expression],
        this_binding: &Expression,
        result_local: u32,
        preserve_emitted_effect_metadata: bool,
    ) -> DirectResult<bool> {
        if std::env::var_os("AYY_TRACE_INLINE_PROMISES").is_some() {
            eprintln!(
                "emit_inline_user_function_summary_with_explicit_call_frame:start name={}",
                user_function.name
            );
        }
        if std::env::var_os("AYY_TRACE_INLINE_PROMISES").is_some() {
            eprintln!(
                "emit_inline_user_function_summary_with_explicit_call_frame:check-iterator-consumption name={}",
                user_function.name
            );
        }
        let parameter_iterator_consumption_indices =
            self.user_function_parameter_iterator_consumption_indices(user_function);
        if std::env::var_os("AYY_TRACE_INLINE_PROMISES").is_some() {
            eprintln!(
                "emit_inline_user_function_summary_with_explicit_call_frame:check-iterator-consumption:result name={} consumed={parameter_iterator_consumption_indices:?}",
                user_function.name
            );
        }
        if !parameter_iterator_consumption_indices.is_empty() {
            return Ok(false);
        }
        if self.user_function_deletes_call_frame_arguments_member(user_function) {
            return Ok(false);
        }
        let mut state =
            self.prepare_inline_summary_emission_state(user_function, arguments, this_binding)?;
        if std::env::var_os("AYY_TRACE_INLINE_PROMISES").is_some() {
            eprintln!(
                "emit_inline_user_function_summary_with_explicit_call_frame:prepared name={}",
                user_function.name
            );
        }
        if self.try_emit_inline_summary_fast_path(
            user_function,
            arguments,
            &state,
            this_binding,
            result_local,
        )? {
            if std::env::var_os("AYY_TRACE_INLINE_PROMISES").is_some() {
                eprintln!(
                    "emit_inline_user_function_summary_with_explicit_call_frame:fast-path name={}",
                    user_function.name
                );
            }
            if preserve_emitted_effect_metadata {
                state.additional_call_effect_nonlocal_bindings.clear();
                state.updated_nonlocal_bindings.clear();
                state.assigned_nonlocal_binding_results = None;
            }
            self.finalize_inline_summary_emission_state(user_function, arguments, &mut state)?;
            return Ok(true);
        }
        if std::env::var_os("AYY_TRACE_INLINE_PROMISES").is_some() {
            eprintln!(
                "emit_inline_user_function_summary_with_explicit_call_frame:fallback-start name={}",
                user_function.name
            );
        }
        let emitted = self.try_emit_inline_summary_fallback_path(
            user_function,
            &state,
            this_binding,
            result_local,
        )?;
        if std::env::var_os("AYY_TRACE_INLINE_PROMISES").is_some() {
            eprintln!(
                "emit_inline_user_function_summary_with_explicit_call_frame:fallback-result name={} emitted={emitted}",
                user_function.name
            );
        }
        if !emitted {
            self.abort_inline_summary_emission_state(&state);
            return Ok(false);
        }
        if preserve_emitted_effect_metadata {
            state.additional_call_effect_nonlocal_bindings.clear();
            state.updated_nonlocal_bindings.clear();
            state.assigned_nonlocal_binding_results = None;
        }
        self.finalize_inline_summary_emission_state(user_function, arguments, &mut state)?;
        if std::env::var_os("AYY_TRACE_INLINE_PROMISES").is_some() {
            eprintln!(
                "emit_inline_user_function_summary_with_explicit_call_frame:finalized name={}",
                user_function.name
            );
        }
        Ok(true)
    }
}
