use super::*;

impl<'a> FunctionCompiler<'a> {
    pub(in crate::backend::direct_wasm) fn clear_eval_local_function_binding_metadata(
        &mut self,
        name: &str,
    ) {
        self.state.clear_eval_local_function_binding_metadata(name);
    }

    pub(in crate::backend::direct_wasm) fn clear_static_identifier_binding_metadata(
        &mut self,
        name: &str,
    ) {
        if std::env::var_os("AYY_TRACE_RUNTIME_SHADOWS").is_some() {
            eprintln!(
                "runtime_shadow_clear_identifier_metadata name={name} local_value={:?} local_object={} global_value={:?} global_object={}",
                self.state
                    .speculation
                    .static_semantics
                    .local_value_binding(name)
                    .cloned(),
                self.state
                    .speculation
                    .static_semantics
                    .has_local_object_binding(name),
                self.global_value_binding(name).cloned(),
                self.global_object_binding(name).is_some(),
            );
        }
        self.state.clear_local_static_binding_metadata(name);

        self.clear_global_binding_state(name);
        self.backend
            .clear_global_object_literal_member_bindings_for_name(name);
        if self.resolve_current_local_binding(name).is_none()
            && !self.state.runtime.locals.bindings.contains_key(name)
            && self.parameter_scope_arguments_local_for(name).is_none()
            && (self.global_has_binding(name)
                || self.global_has_implicit_binding(name)
                || self.backend.global_has_lexical_binding(name)
                || self
                    .backend
                    .shared_global_semantics
                    .global_names()
                    .kind(name)
                    .is_some()
                || self
                    .backend
                    .shared_global_semantics
                    .values
                    .value_binding(name)
                    .is_some())
        {
            self.backend
                .shared_global_semantics
                .clear_global_binding_state(name);
            self.backend
                .shared_global_semantics
                .clear_global_object_literal_member_bindings_for_name(name);
        }
    }

    pub(in crate::backend::direct_wasm) fn emit_delete_implicit_global_binding(
        &mut self,
        name: &str,
    ) -> DirectResult<bool> {
        let Some(binding) = self.backend.implicit_global_binding(name) else {
            return Ok(false);
        };
        self.clear_static_identifier_binding_metadata(name);
        self.push_i32_const(0);
        self.push_global_set(binding.present_index);
        self.push_i32_const(JS_UNDEFINED_TAG);
        self.push_global_set(binding.value_index);
        self.push_i32_const(1);
        Ok(true)
    }

    pub(in crate::backend::direct_wasm) fn emit_typeof_implicit_global_binding(
        &mut self,
        name: &str,
    ) -> DirectResult<bool> {
        if self.resolve_current_local_binding(name).is_some()
            || self.backend.global_binding_index(name).is_some()
        {
            return Ok(false);
        }
        let Some(binding) = self.backend.implicit_global_binding(name) else {
            return Ok(false);
        };
        let value_local = self.allocate_temp_local();

        self.push_global_get(binding.present_index);
        self.state.emission.output.instructions.push(0x04);
        self.state.emission.output.instructions.push(I32_TYPE);
        self.push_control_frame();
        self.push_global_get(binding.value_index);
        self.push_local_set(value_local);
        self.emit_runtime_typeof_tag_from_local(value_local)?;
        self.state.emission.output.instructions.push(0x05);
        self.push_i32_const(JS_TYPEOF_UNDEFINED_TAG);
        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();
        Ok(true)
    }

    pub(in crate::backend::direct_wasm) fn emit_store_implicit_global_from_local(
        &mut self,
        binding: ImplicitGlobalBinding,
        value_local: u32,
    ) -> DirectResult<()> {
        if self.state.speculation.execution_context.strict_mode {
            self.push_global_get(binding.present_index);
            self.state.emission.output.instructions.push(0x04);
            self.state
                .emission
                .output
                .instructions
                .push(EMPTY_BLOCK_TYPE);
            self.push_control_frame();
            self.push_local_get(value_local);
            self.push_global_set(binding.value_index);
            self.state.emission.output.instructions.push(0x05);
            self.emit_named_error_throw("ReferenceError")?;
            self.state.emission.output.instructions.push(0x0b);
            self.pop_control_frame();
            return Ok(());
        }

        self.push_local_get(value_local);
        self.push_global_set(binding.value_index);
        self.push_i32_const(1);
        self.push_global_set(binding.present_index);
        Ok(())
    }
}
