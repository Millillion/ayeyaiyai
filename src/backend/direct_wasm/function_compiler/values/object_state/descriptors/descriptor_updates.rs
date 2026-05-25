use super::*;

impl<'a> FunctionCompiler<'a> {
    pub(in crate::backend::direct_wasm) fn update_local_descriptor_binding(
        &mut self,
        name: &str,
        value: &Expression,
    ) {
        let Some(descriptor_binding) = self.resolve_descriptor_binding_from_expression(value)
        else {
            self.state
                .speculation
                .static_semantics
                .objects
                .local_descriptor_bindings
                .remove(name);
            return;
        };
        self.state
            .speculation
            .static_semantics
            .objects
            .local_descriptor_bindings
            .insert(name.to_string(), descriptor_binding);
        self.state
            .speculation
            .static_semantics
            .set_local_kind(name, StaticValueKind::Object);
    }

    pub(in crate::backend::direct_wasm) fn update_global_property_descriptor_value(
        &mut self,
        name: &str,
        value_expression: &Expression,
    ) {
        let materialized = self
            .backend
            .global_value_binding(name)
            .cloned()
            .unwrap_or_else(|| self.materialize_static_expression(value_expression));
        if let Some(mut state) = self.backend.global_property_descriptor(name).cloned() {
            state.value = materialized;
            self.backend
                .upsert_global_property_descriptor(name.to_string(), state);
        }
    }

    pub(in crate::backend::direct_wasm) fn ensure_global_property_descriptor_value(
        &mut self,
        name: &str,
        value_expression: &Expression,
        configurable: bool,
    ) {
        let materialized = self
            .backend
            .global_value_binding(name)
            .cloned()
            .unwrap_or_else(|| self.materialize_static_expression(value_expression));
        let next_state = self
            .backend
            .global_property_descriptor(name)
            .cloned()
            .map(|mut state| {
                state.value = materialized.clone();
                state
            })
            .unwrap_or(GlobalPropertyDescriptorState {
                value: materialized,
                writable: Some(true),
                enumerable: true,
                configurable,
                getter: None,
                setter: None,
                has_get: false,
                has_set: false,
            });
        self.backend
            .upsert_global_property_descriptor(name.to_string(), next_state);
    }

    pub(in crate::backend::direct_wasm) fn instantiate_eval_global_function_property_descriptor(
        &mut self,
        name: &str,
        configurable: bool,
    ) {
        let value = Expression::Identifier(name.to_string());
        let next_state = match self.backend.global_property_descriptor(name).cloned() {
            Some(mut state) if !state.configurable => {
                state.value = value;
                state
            }
            Some(_) | None => GlobalPropertyDescriptorState {
                value,
                writable: Some(true),
                enumerable: true,
                configurable,
                getter: None,
                setter: None,
                has_get: false,
                has_set: false,
            },
        };
        self.backend
            .upsert_global_property_descriptor(name.to_string(), next_state);
    }

    pub(in crate::backend::direct_wasm) fn update_local_value_binding(
        &mut self,
        name: &str,
        value: &Expression,
    ) {
        let snapshot_value = self
            .state
            .speculation
            .static_semantics
            .local_value_binding(name)
            .or_else(|| self.global_value_binding(name))
            .map(|snapshot| substitute_self_referential_binding_snapshot(value, name, snapshot))
            .unwrap_or_else(|| value.clone());
        let mut referenced_names = HashSet::new();
        collect_referenced_binding_names_from_expression(&snapshot_value, &mut referenced_names);
        if referenced_names.contains(name) {
            self.state
                .speculation
                .static_semantics
                .clear_local_value_binding(name);
            return;
        }
        let template_object_identity_value =
            self.resolve_template_object_reference_identity_expression(&snapshot_value);
        let metadata_source_value = template_object_identity_value
            .as_ref()
            .unwrap_or(&snapshot_value);
        let preserve_reference_alias =
            matches!(snapshot_value, Expression::Identifier(_) | Expression::This)
                && (self
                    .resolve_object_binding_from_expression(metadata_source_value)
                    .is_some()
                    || self
                        .resolve_array_binding_from_expression(metadata_source_value)
                        .is_some()
                    || self
                        .resolve_function_binding_from_expression(metadata_source_value)
                        .is_some());
        let preserve_object_literal_member_function_alias = self
            .object_literal_member_function_display_name(metadata_source_value, 0)
            .is_some()
            && self
                .resolve_function_binding_from_expression(metadata_source_value)
                .is_some();
        let materialized_value =
            if let Some(template_object_identity_value) = template_object_identity_value {
                template_object_identity_value
            } else if preserve_reference_alias || preserve_object_literal_member_function_alias {
                snapshot_value.clone()
            } else if matches!(
                metadata_source_value,
                Expression::Call { callee, .. }
                    if matches!(callee.as_ref(), Expression::Identifier(name)
                        if name == "Symbol" && self.is_unshadowed_builtin_identifier(name))
            ) {
                snapshot_value.clone()
            } else if let Some(bigint) = self.resolve_static_bigint_value(metadata_source_value) {
                Expression::BigInt(bigint.to_string())
            } else {
                self.resolve_static_string_value(metadata_source_value)
                    .map(Expression::String)
                    .unwrap_or_else(|| self.materialize_static_expression(metadata_source_value))
            };
        self.state
            .speculation
            .static_semantics
            .set_local_value_binding(name, materialized_value);
    }
}
