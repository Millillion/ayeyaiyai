use super::*;

impl<'a> FunctionCompiler<'a> {
    pub(in crate::backend::direct_wasm) fn global_has_binding(&self, name: &str) -> bool {
        self.backend.global_has_binding(name)
    }

    pub(in crate::backend::direct_wasm) fn global_has_implicit_binding(&self, name: &str) -> bool {
        self.backend.global_has_implicit_binding(name)
    }

    pub(in crate::backend::direct_wasm) fn global_binding_kind(
        &self,
        name: &str,
    ) -> Option<StaticValueKind> {
        self.backend.global_binding_kind(name)
    }

    pub(in crate::backend::direct_wasm) fn clear_global_binding_kind(&mut self, name: &str) {
        self.backend.clear_global_binding_kind(name);
    }

    pub(in crate::backend::direct_wasm) fn implicit_global_binding(
        &self,
        name: &str,
    ) -> Option<ImplicitGlobalBinding> {
        self.backend.implicit_global_binding(name)
    }

    pub(in crate::backend::direct_wasm) fn ensure_implicit_global_binding(
        &mut self,
        name: &str,
    ) -> ImplicitGlobalBinding {
        self.backend.ensure_implicit_global_binding(name)
    }

    pub(in crate::backend::direct_wasm) fn clear_global_binding_state(&mut self, name: &str) {
        self.backend.clear_global_binding_state(name);
    }

    pub(in crate::backend::direct_wasm) fn update_static_global_assignment_metadata(
        &mut self,
        name: &str,
        value: &Expression,
    ) {
        let snapshot_value = self
            .global_value_binding(name)
            .map(|snapshot| substitute_self_referential_binding_snapshot(value, name, snapshot))
            .unwrap_or_else(|| value.clone());
        if matches!(value, Expression::Identifier(source_name) if source_name == name)
            && (self.global_value_binding(name).is_some()
                || self.global_binding_kind(name).is_some()
                || self.backend.global_function_binding(name).is_some()
                || self.backend.global_property_descriptor(name).is_some()
                || self.backend.global_array_binding(name).is_some()
                || self.backend.global_object_binding(name).is_some())
        {
            return;
        }
        let mut referenced_names = HashSet::new();
        collect_referenced_binding_names_from_expression(&snapshot_value, &mut referenced_names);
        if referenced_names.contains(name) {
            self.clear_global_binding_state(name);
            return;
        }

        let template_object_identity_value =
            self.resolve_template_object_reference_identity_expression(&snapshot_value);
        if std::env::var_os("AYY_TRACE_REFERENCE_IDENTITY").is_some() {
            eprintln!(
                "reference_identity:global_update name={name} value={value:?} snapshot={snapshot_value:?} template={template_object_identity_value:?}"
            );
        }
        let metadata_source_value = template_object_identity_value
            .as_ref()
            .unwrap_or(&snapshot_value);
        let function_binding = self.resolve_function_binding_from_expression(metadata_source_value);
        let preserve_private_brand_identifier = matches!(&snapshot_value, Expression::Identifier(name) if name.contains("__ayy_class_brand_"));
        let preserve_symbol_call_binding = matches!(
            &snapshot_value,
            Expression::Call { callee, .. }
                if matches!(callee.as_ref(), Expression::Identifier(symbol_name)
                    if symbol_name == "Symbol" && self.is_unshadowed_builtin_identifier(symbol_name))
        );
        let preserve_reference_alias = preserve_private_brand_identifier
            || self
                .resolve_iterator_source_kind(metadata_source_value)
                .is_some()
            || function_binding.is_some()
            || matches!(&snapshot_value, Expression::Identifier(name) if self.lookup_identifier_kind(name) == Some(StaticValueKind::Symbol))
            || preserve_symbol_call_binding
            || (matches!(snapshot_value, Expression::Identifier(_) | Expression::This)
                && (self
                    .resolve_object_binding_from_expression(metadata_source_value)
                    .is_some()
                    || self
                        .resolve_array_binding_from_expression(metadata_source_value)
                        .is_some()));
        let materialized_value =
            if let Some(template_object_identity_value) = template_object_identity_value.clone() {
                template_object_identity_value
            } else if preserve_reference_alias {
                snapshot_value.clone()
            } else if let Some(bigint) = self.resolve_static_bigint_value(metadata_source_value) {
                Expression::BigInt(bigint.to_string())
            } else {
                self.resolve_static_string_value(metadata_source_value)
                    .map(Expression::String)
                    .unwrap_or_else(|| self.materialize_static_expression(metadata_source_value))
            };
        let mut kind = self
            .infer_value_kind(metadata_source_value)
            .unwrap_or(StaticValueKind::Unknown);
        if kind != StaticValueKind::String
            && !self
                .runtime_string_print_candidates(metadata_source_value)
                .is_empty()
        {
            kind = StaticValueKind::String;
        }

        let array_binding = self.resolve_array_binding_from_expression(metadata_source_value);
        let object_binding = self
            .resolve_object_binding_from_expression(metadata_source_value)
            .map(|binding| self.merge_global_assignment_object_metadata(name, binding));
        let arguments_binding =
            self.resolve_arguments_binding_from_expression(metadata_source_value);
        self.backend.set_global_binding_kind(name, kind);
        self.backend
            .shared_global_semantics
            .set_global_binding_kind(name, kind);
        self.backend
            .sync_global_expression_binding(name, Some(materialized_value.clone()));
        self.backend
            .shared_global_semantics
            .values
            .set_value_binding(name.to_string(), materialized_value);
        self.backend
            .sync_global_array_binding(name, array_binding.clone());
        self.backend
            .shared_global_semantics
            .values
            .sync_array_binding(name, array_binding);
        self.backend
            .sync_global_object_binding(name, object_binding.clone());
        self.backend
            .shared_global_semantics
            .values
            .sync_object_binding(name, object_binding);
        self.backend
            .sync_global_arguments_binding(name, arguments_binding.clone());
        self.backend
            .shared_global_semantics
            .values
            .sync_arguments_binding(name, arguments_binding);
        self.backend
            .sync_global_function_binding(name, function_binding.clone());
        if let Some(function_binding) = function_binding {
            self.backend
                .shared_global_semantics
                .set_global_function_binding(name, function_binding);
        } else {
            self.backend
                .shared_global_semantics
                .clear_global_function_binding(name);
        }
    }

    fn merge_global_assignment_object_metadata(
        &self,
        name: &str,
        mut object_binding: ObjectValueBinding,
    ) -> ObjectValueBinding {
        let Some(existing_binding) = self.backend.global_object_binding(name) else {
            return object_binding;
        };

        let existing_ordered_names = ordered_object_property_names(existing_binding);
        let new_ordered_names = ordered_object_property_names(&object_binding);
        if existing_ordered_names.is_empty()
            || new_ordered_names.is_empty()
            || existing_ordered_names == new_ordered_names
        {
            return object_binding;
        }

        let existing_set = existing_ordered_names
            .iter()
            .cloned()
            .collect::<HashSet<_>>();
        let new_set = new_ordered_names.iter().cloned().collect::<HashSet<_>>();
        if existing_set != new_set {
            return object_binding;
        }

        let previous_binding = object_binding.clone();
        object_binding.string_properties = existing_ordered_names
            .into_iter()
            .filter_map(|property_name| {
                if let Some((_, value)) = previous_binding
                    .string_properties
                    .iter()
                    .find(|(name, _)| name == &property_name)
                {
                    return Some((property_name, value.clone()));
                }
                existing_binding
                    .string_properties
                    .iter()
                    .find(|(name, _)| name == &property_name)
                    .map(|(_, value)| (property_name, value.clone()))
            })
            .collect();
        object_binding
    }

    pub(in crate::backend::direct_wasm) fn allocate_test262_realm(&mut self) -> u32 {
        self.backend.allocate_test262_realm()
    }

    pub(in crate::backend::direct_wasm) fn global_value_binding(
        &self,
        name: &str,
    ) -> Option<&Expression> {
        self.backend.global_value_binding(name)
    }

    pub(in crate::backend::direct_wasm) fn global_object_binding(
        &self,
        name: &str,
    ) -> Option<&ObjectValueBinding> {
        self.backend.global_object_binding(name)
    }

    pub(in crate::backend::direct_wasm) fn global_array_binding(
        &self,
        name: &str,
    ) -> Option<&ArrayValueBinding> {
        self.backend.global_array_binding(name)
    }

    pub(in crate::backend::direct_wasm) fn global_prototype_object_binding(
        &self,
        name: &str,
    ) -> Option<&ObjectValueBinding> {
        self.backend.global_prototype_object_binding(name)
    }

    pub(in crate::backend::direct_wasm) fn global_proxy_binding(
        &self,
        name: &str,
    ) -> Option<&ProxyValueBinding> {
        self.backend.global_proxy_binding(name)
    }

    pub(in crate::backend::direct_wasm) fn global_object_prototype_expression(
        &self,
        name: &str,
    ) -> Option<&Expression> {
        self.backend.global_object_prototype_expression(name)
    }

    pub(in crate::backend::direct_wasm) fn find_global_home_object_binding_name(
        &self,
        function_name: &str,
    ) -> Option<String> {
        self.backend
            .find_global_home_object_binding_name(function_name)
    }

    pub(in crate::backend::direct_wasm) fn global_runtime_prototype_binding(
        &self,
        name: &str,
    ) -> Option<&GlobalObjectRuntimePrototypeBinding> {
        self.backend.global_runtime_prototype_binding(name)
    }

    pub(in crate::backend::direct_wasm) fn test262_realm_global_object_binding(
        &self,
        realm_id: u32,
    ) -> Option<ObjectValueBinding> {
        self.backend.test262_realm_global_object_binding(realm_id)
    }

    pub(in crate::backend::direct_wasm) fn test262_realm_mut(
        &mut self,
        realm_id: u32,
    ) -> Option<&mut Test262Realm> {
        self.backend.test262_realm_mut(realm_id)
    }
}
