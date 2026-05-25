use super::*;

impl<'a> FunctionCompiler<'a> {
    pub(in crate::backend::direct_wasm) fn runtime_array_binding_has_state(
        &self,
        name: &str,
    ) -> bool {
        self.state
            .speculation
            .static_semantics
            .runtime_array_length_local(name)
            .is_some()
            || self
                .state
                .speculation
                .static_semantics
                .has_runtime_array_slots(name)
            || (self.is_named_global_array_binding(name)
                && self.uses_global_runtime_array_state(name))
    }

    pub(in crate::backend::direct_wasm) fn ensure_runtime_array_length_local(
        &mut self,
        name: &str,
    ) -> u32 {
        if let Some(local) = self
            .state
            .speculation
            .static_semantics
            .runtime_array_length_local(name)
        {
            return local;
        }
        let local = self.allocate_temp_local();
        self.state
            .speculation
            .static_semantics
            .set_runtime_array_length_local(name, local);
        local
    }

    pub(in crate::backend::direct_wasm) fn resolve_runtime_array_binding_name(
        &self,
        name: &str,
    ) -> Option<String> {
        if self
            .state
            .speculation
            .static_semantics
            .has_local_array_binding(name)
            || self
                .state
                .speculation
                .static_semantics
                .runtime_array_length_local(name)
                .is_some()
            || self
                .state
                .speculation
                .static_semantics
                .has_runtime_array_slots(name)
        {
            return Some(name.to_string());
        }
        let (resolved_name, _) = self.resolve_current_local_binding(name)?;
        if self
            .state
            .speculation
            .static_semantics
            .has_local_array_binding(&resolved_name)
            || self
                .state
                .speculation
                .static_semantics
                .runtime_array_length_local(&resolved_name)
                .is_some()
            || self
                .state
                .speculation
                .static_semantics
                .has_runtime_array_slots(&resolved_name)
        {
            return Some(resolved_name);
        }
        None
    }

    pub(in crate::backend::direct_wasm) fn resolve_local_array_iterator_binding_name(
        &self,
        name: &str,
    ) -> Option<String> {
        if self
            .state
            .speculation
            .static_semantics
            .has_local_array_iterator_binding(name)
        {
            return Some(name.to_string());
        }
        let (resolved_name, _) = self.resolve_current_local_binding(name)?;
        self.state
            .speculation
            .static_semantics
            .has_local_array_iterator_binding(&resolved_name)
            .then_some(resolved_name)
    }

    pub(in crate::backend::direct_wasm) fn runtime_array_length_local_for_expression(
        &self,
        expression: &Expression,
    ) -> Option<u32> {
        let binding_name = self.runtime_array_binding_name_for_expression(expression)?;
        self.state
            .speculation
            .static_semantics
            .runtime_array_length_local(&binding_name)
    }

    pub(in crate::backend::direct_wasm) fn runtime_array_binding_name_for_expression(
        &self,
        expression: &Expression,
    ) -> Option<String> {
        if let Expression::Identifier(name) = expression {
            if let Some(alias_expression) = self.direct_identifier_value_binding(name)
                && !static_expression_matches(alias_expression, expression)
                && let Some(alias_binding_name) =
                    self.runtime_array_binding_name_for_expression(alias_expression)
                && self.runtime_array_binding_has_state(&alias_binding_name)
            {
                return Some(alias_binding_name);
            }
            if let Some(binding_name) = self
                .resolve_runtime_array_binding_name(name)
                .or_else(|| {
                    self.state
                        .speculation
                        .static_semantics
                        .runtime_array_length_local(name)
                        .is_some()
                        .then(|| name.clone())
                })
                .or_else(|| {
                    self.state
                        .speculation
                        .static_semantics
                        .has_runtime_array_slots(name)
                        .then(|| name.clone())
                })
                .or_else(|| {
                    (self.is_named_global_array_binding(name)
                        && (self.uses_global_runtime_array_state(name)
                            || self.backend.global_array_binding(name).is_some()))
                    .then(|| name.clone())
                })
            {
                return Some(binding_name);
            }
        }

        if let Some(resolved) = self
            .resolve_bound_alias_expression(expression)
            .filter(|resolved| !static_expression_matches(resolved, expression))
            && let Some(binding_name) = self.runtime_array_binding_name_for_expression(&resolved)
        {
            return Some(binding_name);
        }

        if let Expression::Member { object, property } = expression {
            let canonical_property = self.canonical_object_property_expression(property);
            if let Some(shadow_binding_name) = self
                .runtime_object_property_shadow_binding_name_for_expression(
                    object,
                    &canonical_property,
                )
                && let Some(value) = self.global_value_binding(&shadow_binding_name).or_else(|| {
                    self.backend
                        .shared_global_semantics
                        .values
                        .value_bindings
                        .get(&shadow_binding_name)
                })
                && let Some(binding_name) = self.runtime_array_binding_name_for_expression(value)
            {
                return Some(binding_name);
            }

            if let Some(object_binding) = self.resolve_object_binding_from_expression(object)
                && let Some(value) = self
                    .resolve_object_binding_property_value(&object_binding, &canonical_property)
                    .or_else(|| {
                        self.resolve_object_binding_property_value(&object_binding, property)
                    })
                && !static_expression_matches(&value, expression)
                && let Some(binding_name) = self.runtime_array_binding_name_for_expression(&value)
            {
                return Some(binding_name);
            }
        }

        let materialized = self.materialize_static_expression(expression);
        if !static_expression_matches(&materialized, expression) {
            return self.runtime_array_binding_name_for_expression(&materialized);
        }
        None
    }

    fn direct_identifier_value_binding(&self, name: &str) -> Option<&Expression> {
        if let Some((resolved_name, _)) = self.resolve_current_local_binding(name)
            && let Some(value) = self
                .state
                .speculation
                .static_semantics
                .local_value_binding(&resolved_name)
        {
            return Some(value);
        }
        self.state
            .speculation
            .static_semantics
            .local_value_binding(name)
            .or_else(|| self.backend.global_value_binding(name))
    }

    pub(in crate::backend::direct_wasm) fn ensure_runtime_array_slots_for_binding(
        &mut self,
        name: &str,
        binding: &ArrayValueBinding,
    ) {
        for index in 0..TRACKED_ARRAY_SLOT_LIMIT {
            let slot = if let Some(slot) = self.runtime_array_slot(name, index) {
                slot
            } else {
                let slot = RuntimeArraySlot {
                    value_local: self.allocate_temp_local(),
                    present_local: self.allocate_temp_local(),
                };
                self.state
                    .speculation
                    .static_semantics
                    .set_runtime_array_slot(name, index, slot.clone());
                slot
            };
            match binding.values.get(index as usize).cloned().flatten() {
                Some(value) => {
                    self.emit_numeric_expression(&value)
                        .expect("runtime array slot initialization is supported");
                    self.push_local_set(slot.value_local);
                    self.push_i32_const(1);
                    self.push_local_set(slot.present_local);
                }
                None => {
                    self.push_i32_const(JS_UNDEFINED_TAG);
                    self.push_local_set(slot.value_local);
                    self.push_i32_const(0);
                    self.push_local_set(slot.present_local);
                }
            }
        }
    }

    pub(in crate::backend::direct_wasm) fn runtime_array_slot(
        &self,
        name: &str,
        index: u32,
    ) -> Option<RuntimeArraySlot> {
        self.state
            .speculation
            .static_semantics
            .runtime_array_slot(name, index)
    }
}
