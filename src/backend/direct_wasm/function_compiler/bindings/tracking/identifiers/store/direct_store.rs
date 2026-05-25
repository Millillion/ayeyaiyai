use super::*;

fn is_internal_assignment_temp(name: &str) -> bool {
    name.starts_with("__ayy_optional_base_")
        || name.starts_with("__ayy_target_object_")
        || name.starts_with("__ayy_target_property_")
        || name.starts_with("__ayy_postfix_previous_")
}

impl<'a> FunctionCompiler<'a> {
    fn emit_store_local_binding_from_local(
        &mut self,
        resolved_name: &str,
        local_index: u32,
        value_local: u32,
    ) -> DirectResult<()> {
        if let Some(initialized_local) = self.local_lexical_initialized_local(resolved_name) {
            self.push_local_get(initialized_local);
            self.state.emission.output.instructions.push(0x04);
            self.state
                .emission
                .output
                .instructions
                .push(EMPTY_BLOCK_TYPE);
            self.push_control_frame();
            if self.local_binding_is_immutable(resolved_name) {
                self.emit_named_error_throw("TypeError")?;
            } else {
                self.push_local_get(value_local);
                self.push_local_set(local_index);
            }
            self.state.emission.output.instructions.push(0x05);
            self.emit_named_error_throw("ReferenceError")?;
            self.state.emission.output.instructions.push(0x0b);
            self.pop_control_frame();
            return Ok(());
        }

        self.push_local_get(value_local);
        self.push_local_set(local_index);
        Ok(())
    }

    pub(in crate::backend::direct_wasm) fn emit_store_declared_global_from_local(
        &mut self,
        name: &str,
        value_local: u32,
        global_index: u32,
    ) -> DirectResult<()> {
        let trace_identifier_store = std::env::var_os("AYY_TRACE_IDENTIFIER_STORE").is_some();
        if trace_identifier_store {
            eprintln!("identifier_direct_store:{name}:declared_global");
        }
        if let Some(binding) = self.backend.lexical_global_binding(name) {
            self.push_global_get(binding.initialized_index);
            self.state.emission.output.instructions.push(0x04);
            self.state
                .emission
                .output
                .instructions
                .push(EMPTY_BLOCK_TYPE);
            self.push_control_frame();
            if binding.mutable {
                self.push_local_get(value_local);
                self.push_global_set(global_index);
            } else {
                if trace_identifier_store {
                    eprintln!("identifier_direct_store:{name}:immutable_initialized_type_error");
                }
                self.emit_named_error_throw("TypeError")?;
            }
            self.state.emission.output.instructions.push(0x05);
            self.emit_named_error_throw("ReferenceError")?;
            self.state.emission.output.instructions.push(0x0b);
            self.pop_control_frame();
            return Ok(());
        }

        self.push_local_get(value_local);
        self.push_global_set(global_index);
        Ok(())
    }

    pub(in crate::backend::direct_wasm) fn emit_store_identifier_from_local(
        &mut self,
        name: &str,
        value_local: u32,
    ) -> DirectResult<()> {
        let trace_identifier_store = std::env::var_os("AYY_TRACE_IDENTIFIER_STORE").is_some();
        if trace_identifier_store {
            eprintln!("identifier_direct_store:{name}:start");
        }
        if self.assignment_targets_immutable_class_binding(name) {
            if trace_identifier_store {
                eprintln!("identifier_direct_store:{name}:immutable_class_binding_type_error");
            }
            self.emit_named_error_throw("TypeError")?;
            return Ok(());
        }
        if self.assignment_targets_immutable_function_self_binding(name) {
            if trace_identifier_store {
                eprintln!("identifier_direct_store:{name}:immutable_function_self_binding");
            }
            if self.state.speculation.execution_context.strict_mode {
                self.emit_named_error_throw("TypeError")?;
            }
            return Ok(());
        }
        let resolved_local_binding = self.resolve_current_local_binding(name);
        if let Some(parameter_scope_arguments_local) =
            self.parameter_scope_arguments_local_for(name)
        {
            self.push_local_get(value_local);
            self.push_local_set(parameter_scope_arguments_local);
        }
        if let Some((resolved_name, local_index)) = resolved_local_binding {
            self.emit_store_local_binding_from_local(&resolved_name, local_index, value_local)?;
        } else if is_internal_assignment_temp(name) {
            let local_index = self.ensure_named_internal_local(name, StaticValueKind::Unknown);
            self.push_local_get(value_local);
            self.push_local_set(local_index);
        } else if let Some(global_index) = self.backend.global_binding_index(name) {
            self.emit_store_declared_global_from_local(name, value_local, global_index)?;
        } else if self.emit_store_user_function_capture_binding_from_local(name, value_local)? {
        } else if self.emit_store_eval_local_function_binding_from_local(name, value_local)? {
        } else if let Some(binding) = self.backend.implicit_global_binding(name) {
            self.emit_store_implicit_global_from_local(binding, value_local)?;
        } else {
            let binding = self.ensure_implicit_global_binding(name);
            self.emit_store_implicit_global_from_local(binding, value_local)?;
        }
        self.state
            .emission
            .emitted_value_bindings
            .insert(name.to_string());
        Ok(())
    }

    pub(in crate::backend::direct_wasm) fn emit_sync_identifier_runtime_value_from_local(
        &mut self,
        name: &str,
        value_local: u32,
    ) -> DirectResult<()> {
        if let Some(parameter_scope_arguments_local) =
            self.parameter_scope_arguments_local_for(name)
        {
            self.push_local_get(value_local);
            self.push_local_set(parameter_scope_arguments_local);
        }
        if let Some((resolved_name, local_index)) = self.resolve_current_local_binding(name) {
            self.emit_store_local_binding_from_local(&resolved_name, local_index, value_local)?;
        } else if is_internal_assignment_temp(name) {
            let local_index = self.ensure_named_internal_local(name, StaticValueKind::Unknown);
            self.push_local_get(value_local);
            self.push_local_set(local_index);
        } else if let Some(global_index) = self.backend.global_binding_index(name) {
            self.emit_store_declared_global_from_local(name, value_local, global_index)?;
        } else if let Some(hidden_name) = self.resolve_user_function_capture_hidden_name(name) {
            self.sync_user_function_capture_static_metadata(name, &hidden_name);
            self.emit_store_user_function_capture_binding_from_local(name, value_local)?;
        } else if self.emit_store_eval_local_function_binding_from_local(name, value_local)? {
        } else if let Some(binding) = self.backend.implicit_global_binding(name) {
            self.emit_store_implicit_global_from_local(binding, value_local)?;
        } else {
            let binding = self.ensure_implicit_global_binding(name);
            self.emit_store_implicit_global_from_local(binding, value_local)?;
        }
        self.state
            .emission
            .emitted_value_bindings
            .insert(name.to_string());
        Ok(())
    }
}
