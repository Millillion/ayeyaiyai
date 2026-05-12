use super::*;

impl<'a> FunctionCompiler<'a> {
    pub(in crate::backend::direct_wasm) fn emit_force_global_runtime_array_state_from_internal_rest_source(
        &mut self,
        name: &str,
        source: &Expression,
    ) -> DirectResult<bool> {
        let trace_identifier_store = std::env::var_os("AYY_TRACE_IDENTIFIER_STORE").is_some();
        let Expression::Identifier(source_name) = source else {
            if trace_identifier_store {
                eprintln!(
                    "identifier_store:{name}:global_array_force_rest source_not_identifier source={source:?}"
                );
            }
            return Ok(false);
        };
        let Some(source_name) = self.resolve_runtime_array_binding_name(source_name) else {
            if trace_identifier_store {
                eprintln!(
                    "identifier_store:{name}:global_array_force_rest source_not_runtime source={source_name}"
                );
            }
            return Ok(false);
        };
        let target_is_declared_or_tracked_global = self.is_named_global_array_binding(name)
            || self.backend.global_binding_index(name).is_some()
            || self.backend.global_has_implicit_binding(name);
        if !source_name.starts_with("__ayy_array_rest_") || !target_is_declared_or_tracked_global {
            if trace_identifier_store {
                eprintln!(
                    "identifier_store:{name}:global_array_force_rest rejected resolved_source={source_name} target_global={target_is_declared_or_tracked_global}"
                );
            }
            return Ok(false);
        }
        if trace_identifier_store {
            eprintln!(
                "identifier_store:{name}:global_array_force_rest accepted resolved_source={source_name}"
            );
        }

        self.backend.mark_global_array_with_runtime_state(name);

        let length_binding = self.global_runtime_array_length_binding(name);
        if let Some(source_length_local) = self
            .state
            .speculation
            .static_semantics
            .runtime_array_length_local(&source_name)
        {
            self.push_local_get(source_length_local);
        } else if let Some(binding) = self.resolve_array_binding_from_expression(source) {
            self.push_i32_const(binding.values.len() as i32);
        } else {
            self.push_i32_const(0);
        }
        self.push_global_set(length_binding.value_index);
        self.push_i32_const(1);
        self.push_global_set(length_binding.present_index);

        let source_binding = self.resolve_array_binding_from_expression(source);
        for index in 0..TRACKED_ARRAY_SLOT_LIMIT {
            let slot_binding = self.global_runtime_array_slot_binding(name, index);
            if let Some(source_slot) = self.runtime_array_slot(&source_name, index) {
                self.push_local_get(source_slot.value_local);
                self.push_global_set(slot_binding.value_index);
                self.push_local_get(source_slot.present_local);
                self.push_global_set(slot_binding.present_index);
            } else if let Some(Some(value)) = source_binding
                .as_ref()
                .and_then(|binding| binding.values.get(index as usize))
            {
                self.emit_numeric_expression(value)?;
                self.push_global_set(slot_binding.value_index);
                self.push_i32_const(1);
                self.push_global_set(slot_binding.present_index);
            } else {
                self.push_i32_const(JS_UNDEFINED_TAG);
                self.push_global_set(slot_binding.value_index);
                self.push_i32_const(0);
                self.push_global_set(slot_binding.present_index);
            }
        }

        Ok(true)
    }

    pub(in crate::backend::direct_wasm) fn emit_sync_global_runtime_array_state_from_runtime_source(
        &mut self,
        name: &str,
        source: &Expression,
    ) -> DirectResult<bool> {
        let Expression::Identifier(source_name) = source else {
            return Ok(false);
        };
        let Some(source_name) = self.resolve_runtime_array_binding_name(source_name) else {
            return Ok(false);
        };
        let target_is_declared_or_tracked_global = self.is_named_global_array_binding(name)
            || self.backend.global_binding_index(name).is_some()
            || self.backend.global_has_implicit_binding(name);
        if !target_is_declared_or_tracked_global {
            return Ok(false);
        }

        let source_binding = self.resolve_array_binding_from_expression(source);
        if self.state.speculation.execution_context.top_level_function
            && !self.uses_global_runtime_array_state(name)
        {
            let length_local = self.ensure_runtime_array_length_local(name);
            if let Some(source_length_local) = self
                .state
                .speculation
                .static_semantics
                .runtime_array_length_local(&source_name)
            {
                self.push_local_get(source_length_local);
            } else if self.emit_global_runtime_array_length_read(&source_name) {
            } else if let Some(binding) = source_binding.as_ref() {
                self.push_i32_const(binding.values.len() as i32);
            } else {
                self.push_i32_const(0);
            }
            self.push_local_set(length_local);

            for index in 0..TRACKED_ARRAY_SLOT_LIMIT {
                let target_slot = self.ensure_runtime_array_slot_entry(name, index);
                if let Some(source_slot) = self.runtime_array_slot(&source_name, index) {
                    self.push_local_get(source_slot.value_local);
                    self.push_local_set(target_slot.value_local);
                    self.push_local_get(source_slot.present_local);
                    self.push_local_set(target_slot.present_local);
                } else if self.is_named_global_array_binding(&source_name)
                    && self.uses_global_runtime_array_state(&source_name)
                {
                    let source_slot = self.global_runtime_array_slot_binding(&source_name, index);
                    self.push_global_get(source_slot.value_index);
                    self.push_local_set(target_slot.value_local);
                    self.push_global_get(source_slot.present_index);
                    self.push_local_set(target_slot.present_local);
                } else if let Some(Some(value)) = source_binding
                    .as_ref()
                    .and_then(|binding| binding.values.get(index as usize))
                {
                    self.emit_numeric_expression(value)?;
                    self.push_local_set(target_slot.value_local);
                    self.push_i32_const(1);
                    self.push_local_set(target_slot.present_local);
                } else {
                    self.push_i32_const(JS_UNDEFINED_TAG);
                    self.push_local_set(target_slot.value_local);
                    self.push_i32_const(0);
                    self.push_local_set(target_slot.present_local);
                }
            }
            return Ok(true);
        }

        self.backend.mark_global_array_with_runtime_state(name);
        let length_binding = self.global_runtime_array_length_binding(name);
        if let Some(source_length_local) = self
            .state
            .speculation
            .static_semantics
            .runtime_array_length_local(&source_name)
        {
            self.push_local_get(source_length_local);
        } else if self.emit_global_runtime_array_length_read(&source_name) {
        } else if let Some(binding) = source_binding.as_ref() {
            self.push_i32_const(binding.values.len() as i32);
        } else {
            self.push_i32_const(0);
        }
        self.push_global_set(length_binding.value_index);
        self.push_i32_const(1);
        self.push_global_set(length_binding.present_index);

        for index in 0..TRACKED_ARRAY_SLOT_LIMIT {
            let target_slot = self.global_runtime_array_slot_binding(name, index);
            if let Some(source_slot) = self.runtime_array_slot(&source_name, index) {
                self.push_local_get(source_slot.value_local);
                self.push_global_set(target_slot.value_index);
                self.push_local_get(source_slot.present_local);
                self.push_global_set(target_slot.present_index);
            } else if self.is_named_global_array_binding(&source_name)
                && self.uses_global_runtime_array_state(&source_name)
            {
                let source_slot = self.global_runtime_array_slot_binding(&source_name, index);
                self.push_global_get(source_slot.value_index);
                self.push_global_set(target_slot.value_index);
                self.push_global_get(source_slot.present_index);
                self.push_global_set(target_slot.present_index);
            } else if let Some(Some(value)) = source_binding
                .as_ref()
                .and_then(|binding| binding.values.get(index as usize))
            {
                self.emit_numeric_expression(value)?;
                self.push_global_set(target_slot.value_index);
                self.push_i32_const(1);
                self.push_global_set(target_slot.present_index);
            } else {
                self.push_i32_const(JS_UNDEFINED_TAG);
                self.push_global_set(target_slot.value_index);
                self.push_i32_const(0);
                self.push_global_set(target_slot.present_index);
            }
        }

        Ok(true)
    }

    pub(in crate::backend::direct_wasm) fn emit_sync_global_runtime_array_state_from_binding(
        &mut self,
        name: &str,
        binding: &ArrayValueBinding,
    ) -> DirectResult<bool> {
        if !self.is_named_global_array_binding(name) {
            return Ok(false);
        }
        if self.state.speculation.execution_context.top_level_function
            && !self.uses_global_runtime_array_state(name)
        {
            let length_local = self.ensure_runtime_array_length_local(name);
            self.push_i32_const(binding.values.len() as i32);
            self.push_local_set(length_local);
            self.ensure_runtime_array_slots_for_binding(name, binding);
            return Ok(true);
        }

        self.emit_global_runtime_array_length_write(name, binding.values.len() as i32);
        for index in 0..TRACKED_ARRAY_SLOT_LIMIT {
            let slot_binding = self.global_runtime_array_slot_binding(name, index);
            match binding.values.get(index as usize).cloned().flatten() {
                Some(value) => {
                    self.emit_numeric_expression(&value)?;
                    self.push_global_set(slot_binding.value_index);
                    self.push_i32_const(1);
                    self.push_global_set(slot_binding.present_index);
                }
                None => {
                    self.push_i32_const(JS_UNDEFINED_TAG);
                    self.push_global_set(slot_binding.value_index);
                    self.push_i32_const(0);
                    self.push_global_set(slot_binding.present_index);
                }
            }
        }
        Ok(true)
    }

    pub(in crate::backend::direct_wasm) fn emit_force_global_runtime_array_state_from_binding(
        &mut self,
        name: &str,
        binding: &ArrayValueBinding,
    ) -> DirectResult<bool> {
        if !self.is_named_global_array_binding(name) {
            return Ok(false);
        }
        self.backend.mark_global_array_with_runtime_state(name);
        self.emit_global_runtime_array_length_write(name, binding.values.len() as i32);
        for index in 0..TRACKED_ARRAY_SLOT_LIMIT {
            let slot_binding = self.global_runtime_array_slot_binding(name, index);
            match binding.values.get(index as usize).cloned().flatten() {
                Some(value) => {
                    self.emit_numeric_expression(&value)?;
                    self.push_global_set(slot_binding.value_index);
                    self.push_i32_const(1);
                    self.push_global_set(slot_binding.present_index);
                }
                None => {
                    self.push_i32_const(JS_UNDEFINED_TAG);
                    self.push_global_set(slot_binding.value_index);
                    self.push_i32_const(0);
                    self.push_global_set(slot_binding.present_index);
                }
            }
        }
        Ok(true)
    }
}
