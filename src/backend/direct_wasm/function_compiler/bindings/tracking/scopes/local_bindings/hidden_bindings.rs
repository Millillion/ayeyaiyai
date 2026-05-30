use super::*;

impl<'a> FunctionCompiler<'a> {
    fn emit_strict_deleted_member_source_check(
        &mut self,
        hidden_name: &str,
        fallback_source_name: Option<&str>,
    ) -> DirectResult<()> {
        if !self.state.speculation.execution_context.strict_mode {
            return Ok(());
        }

        let deleted_marker_name =
            Self::capture_slot_member_source_deleted_binding_name(hidden_name);
        let deleted_marker = self.ensure_implicit_global_binding(&deleted_marker_name);
        self.push_global_get(deleted_marker.present_index);
        self.state.emission.output.instructions.push(0x04);
        self.state
            .emission
            .output
            .instructions
            .push(EMPTY_BLOCK_TYPE);
        self.push_control_frame();
        self.emit_named_error_throw("ReferenceError")?;
        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();

        if let Some(source_name) = self.resolve_capture_slot_source_binding_name(hidden_name)
            && let Some((object_name, property_name)) =
                Self::capture_slot_member_source_key_parts(&source_name)
        {
            let object = Expression::Identifier(object_name);
            let property = Expression::String(property_name);
            if let Some(deleted_binding) =
                self.resolve_runtime_object_property_shadow_deleted_binding(&object, &property)
            {
                self.push_global_get(deleted_binding.present_index);
                self.state.emission.output.instructions.push(0x04);
                self.state
                    .emission
                    .output
                    .instructions
                    .push(EMPTY_BLOCK_TYPE);
                self.push_control_frame();
                self.emit_named_error_throw("ReferenceError")?;
                self.state.emission.output.instructions.push(0x0b);
                self.pop_control_frame();
            }
        }
        if let Some(source_name) = self
            .resolve_capture_slot_source_binding_name(hidden_name)
            .or_else(|| fallback_source_name.map(str::to_string))
            && Self::capture_slot_member_source_key_parts(&source_name).is_none()
            && (self.global_has_binding(&source_name)
                || self.backend.global_has_lexical_binding(&source_name)
                || self.global_has_implicit_binding(&source_name)
                || self.backend.global_function_binding(&source_name).is_some())
        {
            let property = Expression::String(source_name);
            if let Some(deleted_binding) = self
                .resolve_runtime_object_property_shadow_deleted_binding(
                    &Expression::This,
                    &property,
                )
            {
                self.push_global_get(deleted_binding.present_index);
                self.state.emission.output.instructions.push(0x04);
                self.state
                    .emission
                    .output
                    .instructions
                    .push(EMPTY_BLOCK_TYPE);
                self.push_control_frame();
                self.emit_named_error_throw("ReferenceError")?;
                self.state.emission.output.instructions.push(0x0b);
                self.pop_control_frame();
            }
        }

        Ok(())
    }

    pub(in crate::backend::direct_wasm) fn emit_eval_local_function_binding_read(
        &mut self,
        name: &str,
    ) -> DirectResult<bool> {
        let Some(hidden_name) = self.resolve_eval_local_function_hidden_name(name) else {
            return Ok(false);
        };
        let Some(binding) = self.hidden_implicit_global_binding(&hidden_name) else {
            return Ok(false);
        };

        self.emit_strict_deleted_member_source_check(&hidden_name, Some(name))?;

        self.push_global_get(binding.present_index);
        self.state.emission.output.instructions.push(0x04);
        self.state.emission.output.instructions.push(I32_TYPE);
        self.push_control_frame();
        self.push_global_get(binding.value_index);
        self.state.emission.output.instructions.push(0x05);
        self.emit_named_error_throw("ReferenceError")?;
        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();
        Ok(true)
    }

    pub(in crate::backend::direct_wasm) fn emit_user_function_capture_binding_read(
        &mut self,
        name: &str,
    ) -> DirectResult<bool> {
        let Some(hidden_name) = self.resolve_user_function_capture_hidden_name(name) else {
            return Ok(false);
        };
        let Some(binding) = self.hidden_implicit_global_binding(&hidden_name) else {
            return Ok(false);
        };

        self.emit_strict_deleted_member_source_check(&hidden_name, Some(name))?;
        let global_fallback_source =
            self.user_function_capture_global_fallback_source(name, &hidden_name);

        self.push_global_get(binding.present_index);
        self.state.emission.output.instructions.push(0x04);
        self.state.emission.output.instructions.push(I32_TYPE);
        self.push_control_frame();
        self.push_global_get(binding.value_index);
        self.state.emission.output.instructions.push(0x05);
        if let Some(source_name) = global_fallback_source {
            self.emit_global_capture_fallback_read(&source_name)?;
        } else {
            self.emit_named_error_throw("ReferenceError")?;
        }
        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();
        Ok(true)
    }

    pub(in crate::backend::direct_wasm) fn user_function_capture_read_would_throw_reference_error(
        &self,
        name: &str,
    ) -> bool {
        if !self
            .current_function_name()
            .is_some_and(|function_name| function_name.starts_with("__ayy_module_init_"))
        {
            return false;
        }
        if self.resolve_current_local_binding(name).is_some()
            || self.resolve_active_global_lexical_binding(name).is_some()
            || self.resolve_global_binding_index(name).is_some()
            || self
                .state
                .speculation
                .static_semantics
                .has_local_function_binding(name)
        {
            return false;
        }
        let Some(hidden_name) = self.resolve_user_function_capture_hidden_name(name) else {
            return false;
        };
        if self.hidden_implicit_global_binding(&hidden_name).is_none() {
            return false;
        }
        self.user_function_capture_global_fallback_source(name, &hidden_name)
            .is_none()
    }

    pub(in crate::backend::direct_wasm) fn user_function_capture_typeof_needs_runtime_check(
        &self,
        name: &str,
    ) -> bool {
        let Some(hidden_name) = self.resolve_user_function_capture_hidden_name(name) else {
            return false;
        };
        self.user_function_capture_global_fallback_source(name, &hidden_name)
            .is_none()
    }

    fn user_function_capture_global_fallback_source(
        &self,
        name: &str,
        hidden_name: &str,
    ) -> Option<String> {
        let current_function_name = self.current_function_name()?;
        let source_name = self
            .resolve_capture_slot_source_binding_name(hidden_name)
            .unwrap_or_else(|| name.to_string());
        if self.user_function_capture_originates_in_enclosing_local(
            current_function_name,
            &source_name,
        ) {
            return None;
        }
        (self.global_has_binding(&source_name)
            || self.global_has_implicit_binding(&source_name)
            || self.backend.global_has_lexical_binding(&source_name)
            || self.backend.global_function_binding(&source_name).is_some())
        .then_some(source_name)
    }

    fn emit_global_capture_fallback_read(&mut self, source_name: &str) -> DirectResult<()> {
        if let Some(global_index) = self.resolve_global_binding_index(source_name) {
            return self.emit_declared_global_binding_read(source_name, global_index);
        }
        if let Some(binding) = self.implicit_global_binding(source_name) {
            self.push_global_get(binding.present_index);
            self.state.emission.output.instructions.push(0x04);
            self.state.emission.output.instructions.push(I32_TYPE);
            self.push_control_frame();
            self.push_global_get(binding.value_index);
            self.state.emission.output.instructions.push(0x05);
            self.emit_named_error_throw("ReferenceError")?;
            self.state.emission.output.instructions.push(0x0b);
            self.pop_control_frame();
            return Ok(());
        }
        self.emit_named_error_throw("ReferenceError")
    }

    fn emit_global_capture_fallback_store_from_local(
        &mut self,
        source_name: &str,
        value_local: u32,
    ) -> DirectResult<()> {
        if let Some(global_index) = self.resolve_global_binding_index(source_name) {
            return self.emit_store_declared_global_from_local(
                source_name,
                value_local,
                global_index,
            );
        }
        if let Some(binding) = self.implicit_global_binding(source_name) {
            return self.emit_store_implicit_global_from_local(binding, value_local);
        }
        self.emit_named_error_throw("ReferenceError")
    }

    pub(in crate::backend::direct_wasm) fn emit_store_user_function_capture_binding_from_local(
        &mut self,
        name: &str,
        value_local: u32,
    ) -> DirectResult<bool> {
        let Some(hidden_name) = self.resolve_user_function_capture_hidden_name(name) else {
            return Ok(false);
        };
        let Some(binding) = self.hidden_implicit_global_binding(&hidden_name) else {
            return Ok(false);
        };
        self.emit_strict_deleted_member_source_check(&hidden_name, Some(name))?;
        let global_fallback_source =
            self.user_function_capture_global_fallback_source(name, &hidden_name);
        self.push_global_get(binding.present_index);
        self.state.emission.output.instructions.push(0x04);
        self.state
            .emission
            .output
            .instructions
            .push(EMPTY_BLOCK_TYPE);
        self.push_control_frame();
        if self.user_function_capture_binding_is_immutable(name) {
            if self.state.speculation.execution_context.strict_mode {
                self.emit_named_error_throw("TypeError")?;
            }
        } else {
            self.push_local_get(value_local);
            self.push_global_set(binding.value_index);
            self.push_i32_const(1);
            self.push_global_set(binding.present_index);
            if let Some(source_name) = global_fallback_source.as_deref() {
                self.emit_global_capture_fallback_store_from_local(source_name, value_local)?;
            }
        }
        self.state.emission.output.instructions.push(0x05);
        if let Some(source_name) = global_fallback_source {
            self.emit_global_capture_fallback_store_from_local(&source_name, value_local)?;
        } else {
            self.emit_named_error_throw("ReferenceError")?;
        }
        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();
        Ok(true)
    }

    pub(in crate::backend::direct_wasm) fn emit_store_eval_local_function_binding_from_local(
        &mut self,
        name: &str,
        value_local: u32,
    ) -> DirectResult<bool> {
        let Some(hidden_name) = self.resolve_eval_local_function_hidden_name(name) else {
            return Ok(false);
        };
        let Some(binding) = self.hidden_implicit_global_binding(&hidden_name) else {
            return Ok(false);
        };
        self.push_local_get(value_local);
        self.push_global_set(binding.value_index);
        self.push_i32_const(1);
        self.push_global_set(binding.present_index);
        Ok(true)
    }

    pub(in crate::backend::direct_wasm) fn emit_delete_eval_local_function_binding(
        &mut self,
        name: &str,
    ) -> DirectResult<bool> {
        let Some(hidden_name) = self.resolve_eval_local_function_hidden_name(name) else {
            return Ok(false);
        };
        let Some(binding) = self.hidden_implicit_global_binding(&hidden_name) else {
            return Ok(false);
        };
        self.push_i32_const(0);
        self.push_global_set(binding.present_index);
        self.push_i32_const(JS_UNDEFINED_TAG);
        self.push_global_set(binding.value_index);
        self.push_i32_const(1);
        Ok(true)
    }

    pub(in crate::backend::direct_wasm) fn emit_typeof_eval_local_function_binding(
        &mut self,
        name: &str,
    ) -> DirectResult<bool> {
        let Some(hidden_name) = self.resolve_eval_local_function_hidden_name(name) else {
            return Ok(false);
        };
        let Some(binding) = self.hidden_implicit_global_binding(&hidden_name) else {
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

    pub(in crate::backend::direct_wasm) fn emit_typeof_user_function_capture_binding(
        &mut self,
        name: &str,
    ) -> DirectResult<bool> {
        let Some(hidden_name) = self.resolve_user_function_capture_hidden_name(name) else {
            return Ok(false);
        };
        let Some(binding) = self.hidden_implicit_global_binding(&hidden_name) else {
            return Ok(false);
        };
        let value_local = self.allocate_temp_local();
        let global_fallback_source =
            self.user_function_capture_global_fallback_source(name, &hidden_name);

        self.push_global_get(binding.present_index);
        self.state.emission.output.instructions.push(0x04);
        self.state.emission.output.instructions.push(I32_TYPE);
        self.push_control_frame();
        self.push_global_get(binding.value_index);
        self.push_local_set(value_local);
        self.emit_runtime_typeof_tag_from_local(value_local)?;
        self.state.emission.output.instructions.push(0x05);
        if let Some(source_name) = global_fallback_source {
            self.emit_global_capture_fallback_read(&source_name)?;
            self.push_local_set(value_local);
            self.emit_runtime_typeof_tag_from_local(value_local)?;
        } else {
            self.emit_named_error_throw("ReferenceError")?;
        }
        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();
        Ok(true)
    }
}
