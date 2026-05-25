use super::*;

impl<'a> FunctionCompiler<'a> {
    fn rebound_member_target(
        &self,
        target: &MemberFunctionBindingTarget,
        name: &str,
        source_name: &str,
    ) -> Option<MemberFunctionBindingTarget> {
        match target {
            MemberFunctionBindingTarget::Identifier(target) if target == source_name => {
                Some(MemberFunctionBindingTarget::Identifier(name.to_string()))
            }
            MemberFunctionBindingTarget::Prototype(target) if target == source_name => {
                Some(MemberFunctionBindingTarget::Prototype(name.to_string()))
            }
            _ => None,
        }
    }

    pub(in crate::backend::direct_wasm) fn copy_member_bindings_for_alias(
        &mut self,
        name: &str,
        source_name: &str,
    ) {
        let normalized_source_name = self
            .resolve_registered_function_declaration(source_name)
            .and_then(|function| {
                function
                    .self_binding
                    .as_ref()
                    .or(function.top_level_binding.as_ref())
            })
            .cloned()
            .or_else(|| scoped_binding_source_name(source_name).map(str::to_string))
            .unwrap_or_else(|| source_name.to_string());
        let source_name = normalized_source_name.as_str();
        let mut function_bindings = Vec::new();
        let mut function_capture_slots = Vec::new();
        let mut getter_bindings = Vec::new();
        let mut setter_bindings = Vec::new();

        let mut function_binding_entries = self
            .state
            .speculation
            .static_semantics
            .objects
            .member_function_bindings
            .iter()
            .map(|(key, binding)| (key.clone(), binding.clone()))
            .collect::<Vec<_>>();
        function_binding_entries.extend(self.backend.global_member_function_binding_entries());
        function_binding_entries.extend(
            self.backend
                .shared_global_semantics
                .global_members()
                .function_bindings()
                .iter()
                .map(|(key, binding)| (key.clone(), binding.clone())),
        );
        for (key, binding) in function_binding_entries {
            let Some(target) = self.rebound_member_target(&key.target, name, source_name) else {
                continue;
            };
            let rebound_key = MemberFunctionBindingKey {
                target,
                property: key.property.clone(),
            };
            function_bindings.push((rebound_key.clone(), binding));
            if let Some(capture_slots) = self
                .state
                .speculation
                .static_semantics
                .objects
                .member_function_capture_slots
                .get(&key)
                .cloned()
                .or_else(|| {
                    self.backend
                        .global_member_function_capture_slots(&key)
                        .cloned()
                })
            {
                let mut capture_slots = capture_slots;
                if matches!(
                    key.target,
                    MemberFunctionBindingTarget::Identifier(ref target)
                        if target == source_name
                ) && capture_slots
                    .get("this")
                    .is_some_and(|slot_name| slot_name == source_name)
                {
                    capture_slots.insert("this".to_string(), name.to_string());
                }
                function_capture_slots.push((rebound_key, capture_slots));
            }
        }

        let mut getter_binding_entries = self
            .state
            .speculation
            .static_semantics
            .objects
            .member_getter_bindings
            .iter()
            .map(|(key, binding)| (key.clone(), binding.clone()))
            .collect::<Vec<_>>();
        getter_binding_entries.extend(self.backend.global_member_getter_binding_entries());
        getter_binding_entries.extend(
            self.backend
                .shared_global_semantics
                .global_members()
                .getter_bindings()
                .iter()
                .map(|(key, binding)| (key.clone(), binding.clone())),
        );
        for (key, binding) in getter_binding_entries {
            let Some(target) = self.rebound_member_target(&key.target, name, source_name) else {
                continue;
            };
            getter_bindings.push((
                MemberFunctionBindingKey {
                    target,
                    property: key.property.clone(),
                },
                binding,
            ));
        }

        let mut setter_binding_entries = self
            .state
            .speculation
            .static_semantics
            .objects
            .member_setter_bindings
            .iter()
            .map(|(key, binding)| (key.clone(), binding.clone()))
            .collect::<Vec<_>>();
        setter_binding_entries.extend(self.backend.global_member_setter_binding_entries());
        setter_binding_entries.extend(
            self.backend
                .shared_global_semantics
                .global_members()
                .setter_bindings()
                .iter()
                .map(|(key, binding)| (key.clone(), binding.clone())),
        );
        for (key, binding) in setter_binding_entries {
            let Some(target) = self.rebound_member_target(&key.target, name, source_name) else {
                continue;
            };
            setter_bindings.push((
                MemberFunctionBindingKey {
                    target,
                    property: key.property.clone(),
                },
                binding,
            ));
        }

        for (key, binding) in function_bindings {
            self.state
                .speculation
                .static_semantics
                .objects
                .member_function_bindings
                .insert(key.clone(), binding.clone());
            if self.binding_name_is_global(name) {
                self.backend
                    .set_global_member_function_binding(key, binding);
            }
        }
        for (key, capture_slots) in function_capture_slots {
            self.state
                .speculation
                .static_semantics
                .objects
                .member_function_capture_slots
                .insert(key.clone(), capture_slots.clone());
            if self.binding_name_is_global(name) {
                self.backend
                    .set_global_member_function_capture_slots(key, capture_slots);
            }
        }
        for (key, binding) in getter_bindings {
            self.state
                .speculation
                .static_semantics
                .objects
                .member_getter_bindings
                .insert(key.clone(), binding.clone());
            if self.binding_name_is_global(name) {
                self.backend.set_global_member_getter_binding(key, binding);
            }
        }
        for (key, binding) in setter_bindings {
            self.state
                .speculation
                .static_semantics
                .objects
                .member_setter_bindings
                .insert(key.clone(), binding.clone());
            if self.binding_name_is_global(name) {
                self.backend.set_global_member_setter_binding(key, binding);
            }
        }
    }
}
