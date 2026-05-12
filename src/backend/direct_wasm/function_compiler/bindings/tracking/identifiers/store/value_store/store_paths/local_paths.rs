use super::*;

impl<'a> FunctionCompiler<'a> {
    pub(super) fn initialize_identifier_value_to_resolved_local(
        &mut self,
        name: &str,
        value_local: u32,
        resolved_name: &str,
        local_index: u32,
        state: &PreparedIdentifierStoreState,
    ) -> DirectResult<()> {
        if !state.is_internal_iterator_temp {
            self.update_local_value_binding(resolved_name, &state.module_assignment_expression);
            self.update_object_prototype_binding_from_value(
                resolved_name,
                state.prototype_binding_expression(),
            );
            self.state.speculation.static_semantics.set_local_kind(
                resolved_name,
                state.kind.unwrap_or(StaticValueKind::Unknown),
            );
        }
        self.push_local_get(value_local);
        self.push_local_set(local_index);
        self.sync_static_direct_eval_closure_capture_slot_from_local(
            resolved_name,
            value_local,
            state,
        )?;
        self.sync_identifier_store_runtime_object_shadows(name, resolved_name, state)?;
        self.sync_closure_capture_slots_from_local_store(
            resolved_name,
            value_local,
            &state.module_assignment_expression,
        )?;
        if !state.is_internal_iterator_temp
            && let Some(source_name) = scoped_binding_source_name(name)
            && self
                .resolve_eval_local_function_hidden_name(source_name)
                .is_some()
        {
            self.update_local_value_binding(source_name, &state.module_assignment_expression);
            if let Some(function_binding) = state.function_binding.clone() {
                self.state
                    .speculation
                    .static_semantics
                    .set_local_function_binding(source_name, function_binding);
            } else {
                self.state
                    .speculation
                    .static_semantics
                    .clear_local_function_binding(source_name);
            }
            self.state
                .speculation
                .static_semantics
                .set_local_kind(source_name, state.kind.unwrap_or(StaticValueKind::Unknown));
            self.emit_store_eval_local_function_binding_from_local(source_name, value_local)?;
            self.sync_identifier_store_runtime_object_shadows(source_name, source_name, state)?;
        }
        Ok(())
    }

    fn emit_store_resolved_local_from_local(
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

    pub(super) fn store_identifier_value_to_resolved_local(
        &mut self,
        name: &str,
        value_local: u32,
        resolved_name: &str,
        local_index: u32,
        state: &PreparedIdentifierStoreState,
    ) -> DirectResult<()> {
        if self
            .local_lexical_initialized_local(resolved_name)
            .is_some()
            && self.local_binding_is_immutable(resolved_name)
        {
            self.state
                .clear_local_static_binding_metadata(resolved_name);
        } else if !state.is_internal_iterator_temp {
            self.update_local_value_binding(resolved_name, &state.module_assignment_expression);
            self.update_object_prototype_binding_from_value(
                resolved_name,
                state.prototype_binding_expression(),
            );
            self.state.speculation.static_semantics.set_local_kind(
                resolved_name,
                state.kind.unwrap_or(StaticValueKind::Unknown),
            );
        }
        self.emit_store_resolved_local_from_local(resolved_name, local_index, value_local)?;
        self.sync_static_direct_eval_closure_capture_slot_from_local(
            resolved_name,
            value_local,
            state,
        )?;
        self.sync_identifier_store_runtime_object_shadows(name, resolved_name, state)?;
        self.sync_closure_capture_slots_from_local_store(
            resolved_name,
            value_local,
            &state.module_assignment_expression,
        )?;
        if !state.is_internal_iterator_temp
            && let Some(source_name) = scoped_binding_source_name(name)
            && self
                .resolve_eval_local_function_hidden_name(source_name)
                .is_some()
        {
            self.update_local_value_binding(source_name, &state.module_assignment_expression);
            if let Some(function_binding) = state.function_binding.clone() {
                self.state
                    .speculation
                    .static_semantics
                    .set_local_function_binding(source_name, function_binding);
            } else {
                self.state
                    .speculation
                    .static_semantics
                    .clear_local_function_binding(source_name);
            }
            self.state
                .speculation
                .static_semantics
                .set_local_kind(source_name, state.kind.unwrap_or(StaticValueKind::Unknown));
            self.emit_store_eval_local_function_binding_from_local(source_name, value_local)?;
            self.sync_identifier_store_runtime_object_shadows(source_name, source_name, state)?;
        }
        Ok(())
    }

    pub(super) fn store_identifier_value_to_eval_local_hidden(
        &mut self,
        name: &str,
        value_local: u32,
        state: &PreparedIdentifierStoreState,
    ) -> DirectResult<()> {
        self.update_local_value_binding(name, &state.module_assignment_expression);
        self.update_object_prototype_binding_from_value(name, state.prototype_binding_expression());
        if let Some(function_binding) = state.function_binding.clone() {
            self.state
                .speculation
                .static_semantics
                .set_local_function_binding(name, function_binding);
        } else {
            self.state
                .speculation
                .static_semantics
                .clear_local_function_binding(name);
        }
        self.state
            .speculation
            .static_semantics
            .set_local_kind(name, state.kind.unwrap_or(StaticValueKind::Unknown));
        if let Some(source_name) = scoped_binding_source_name(name) {
            self.update_local_value_binding(source_name, &state.module_assignment_expression);
            self.update_object_prototype_binding_from_value(
                source_name,
                state.prototype_binding_expression(),
            );
            if let Some(function_binding) = self
                .state
                .speculation
                .static_semantics
                .local_function_binding(name)
                .cloned()
            {
                self.state
                    .speculation
                    .static_semantics
                    .set_local_function_binding(source_name, function_binding);
            } else {
                self.state
                    .speculation
                    .static_semantics
                    .clear_local_function_binding(source_name);
            }
            self.state
                .speculation
                .static_semantics
                .set_local_kind(source_name, state.kind.unwrap_or(StaticValueKind::Unknown));
        }
        self.emit_store_eval_local_function_binding_from_local(name, value_local)?;
        self.sync_identifier_store_runtime_object_shadows(name, name, state)?;
        Ok(())
    }
}
