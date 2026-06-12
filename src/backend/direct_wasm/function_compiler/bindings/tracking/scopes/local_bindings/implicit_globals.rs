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
        if crate::ayy_env_flag!("AYY_TRACE_RUNTIME_SHADOWS") {
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

    /// A strict-mode implicit-global store throws when the binding is absent
    /// (PutValue against a deleted binding). Top-level presence queries for
    /// accessor-deleted global properties read the dedicated delete-sync
    /// flag, so record the observed absence there before the store's
    /// ReferenceError unwinds. No-op unless the property registered a sync
    /// flag when its deleting accessor was bound.
    pub(in crate::backend::direct_wasm) fn emit_strict_implicit_global_delete_sync(
        &mut self,
        name: &str,
        binding: ImplicitGlobalBinding,
    ) -> DirectResult<()> {
        if !self.state.speculation.execution_context.strict_mode {
            return Ok(());
        }
        let sync_name = Self::global_object_property_delete_sync_binding_name(name);
        let global_sync = self.backend.delete_shadow_was_emitted(&sync_name);
        // A with-scoped binding backed by a self-deleting accessor resolves
        // through the scope object; when the strict store observes the
        // binding absent, presence queries against the scope object must see
        // the deletion through the scope owner's shadow pair.
        let with_scope_objects = self.state.emission.lexical_scopes.with_scopes.clone();
        let scope_owner_candidates = with_scope_objects
            .iter()
            .rev()
            .filter_map(|scope_object| match scope_object {
                Expression::Identifier(scope_name) => Some((scope_name.clone(), name.to_string())),
                _ => None,
            })
            .chain(
                // Inside a deferred closure compile the with scope is no
                // longer on the stack, but the binding's capture source still
                // names the scope object property it resolved through.
                self.resolve_user_function_capture_hidden_name(name)
                    .and_then(|hidden_name| {
                        self.resolve_capture_slot_source_binding_name(&hidden_name)
                    })
                    .and_then(|source_name| {
                        Self::capture_slot_member_source_key_parts(&source_name)
                    }),
            )
            .collect::<Vec<_>>();
        let with_scope_owner =
            scope_owner_candidates
                .into_iter()
                .find(|(owner_name, property_name)| {
                    let scope_object = Expression::Identifier(owner_name.clone());
                    let Some(object_binding) =
                        self.resolve_object_binding_from_expression(&scope_object)
                    else {
                        return false;
                    };
                    let property = Expression::String(property_name.clone());
                    self.static_in_object_property_getter_may_delete_property(
                        &scope_object,
                        &object_binding,
                        &property,
                    )
                });
        if !global_sync && with_scope_owner.is_none() {
            return Ok(());
        }
        self.push_global_get(binding.present_index);
        self.state.emission.output.instructions.push(0x45);
        self.state.emission.output.instructions.push(0x04);
        self.state
            .emission
            .output
            .instructions
            .push(EMPTY_BLOCK_TYPE);
        self.push_control_frame();
        if global_sync {
            let sync_binding = self.ensure_implicit_global_binding(&sync_name);
            self.push_i32_const(JS_UNDEFINED_TAG);
            self.push_global_set(sync_binding.value_index);
            self.push_i32_const(1);
            self.push_global_set(sync_binding.present_index);
        }
        if let Some((scope_name, property_name)) = with_scope_owner {
            let property = Expression::String(property_name);
            let deleted_binding = self
                .runtime_object_property_shadow_deleted_binding_by_property(&scope_name, &property);
            self.push_i32_const(JS_UNDEFINED_TAG);
            self.push_global_set(deleted_binding.value_index);
            self.push_i32_const(1);
            self.push_global_set(deleted_binding.present_index);
            let shadow_binding =
                self.runtime_object_property_shadow_binding_by_property(&scope_name, &property);
            self.push_i32_const(JS_UNDEFINED_TAG);
            self.push_global_set(shadow_binding.value_index);
            self.push_i32_const(0);
            self.push_global_set(shadow_binding.present_index);
        }
        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();
        Ok(())
    }

    #[track_caller]
    pub(in crate::backend::direct_wasm) fn emit_store_implicit_global_from_local(
        &mut self,
        binding: ImplicitGlobalBinding,
        value_local: u32,
    ) -> DirectResult<()> {
        if self.state.speculation.execution_context.strict_mode {
            if crate::ayy_env_flag!("AYY_TRACE_IMPLICIT_STORE") {
                eprintln!(
                    "implicit_global_strict_store fn={:?} value_index={} present_index={} caller={}",
                    self.current_function_name(),
                    binding.value_index,
                    binding.present_index,
                    std::panic::Location::caller()
                );
            }
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
