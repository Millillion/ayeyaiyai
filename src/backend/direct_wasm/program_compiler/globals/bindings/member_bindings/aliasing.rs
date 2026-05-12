use super::*;

impl DirectWasmCompiler {
    pub(in crate::backend::direct_wasm) fn clear_global_object_literal_member_bindings_for_name(
        &mut self,
        name: &str,
    ) {
        self.state
            .global_semantics
            .clear_global_object_literal_member_bindings_for_name(name);
    }

    pub(in crate::backend::direct_wasm) fn clear_global_member_bindings_for_name(
        &mut self,
        name: &str,
    ) {
        self.clear_owned_global_member_bindings_for_name(name);
    }

    pub(in crate::backend::direct_wasm) fn copy_global_member_bindings_for_alias(
        &mut self,
        name: &str,
        source_name: &str,
    ) {
        self.clear_global_member_bindings_for_name(name);
        let source_prototype_object_binding =
            self.global_prototype_object_binding(source_name).cloned();
        let source_object_prototype_expression = self
            .global_object_prototype_expression(source_name)
            .cloned();
        let source_prototype_parent_expression = self
            .global_object_prototype_expression(&format!("{source_name}.prototype"))
            .cloned();
        self.state
            .sync_global_prototype_object_binding(name, source_prototype_object_binding);
        self.state
            .global_semantics
            .values
            .sync_object_prototype_expression(name, source_object_prototype_expression);
        self.state
            .global_semantics
            .values
            .sync_object_prototype_expression(
                &format!("{name}.prototype"),
                source_prototype_parent_expression,
            );

        let mut function_bindings = Vec::new();
        let mut function_capture_slots = Vec::new();
        let mut getter_bindings = Vec::new();
        let mut setter_bindings = Vec::new();

        for (key, binding) in self.global_member_function_binding_entries() {
            let target = match &key.target {
                MemberFunctionBindingTarget::Identifier(target) if target == source_name => {
                    Some(MemberFunctionBindingTarget::Identifier(name.to_string()))
                }
                MemberFunctionBindingTarget::Prototype(target) if target == source_name => {
                    Some(MemberFunctionBindingTarget::Prototype(name.to_string()))
                }
                _ => None,
            };
            if let Some(target) = target {
                let rebound_key = MemberFunctionBindingKey {
                    target,
                    property: key.property.clone(),
                };
                function_bindings.push((rebound_key.clone(), binding));
                if let Some(capture_slots) =
                    self.global_member_function_capture_slots(&key).cloned()
                {
                    function_capture_slots.push((rebound_key, capture_slots));
                }
            }
        }

        for (key, binding) in self.global_member_getter_binding_entries() {
            let target = match &key.target {
                MemberFunctionBindingTarget::Identifier(target) if target == source_name => {
                    Some(MemberFunctionBindingTarget::Identifier(name.to_string()))
                }
                MemberFunctionBindingTarget::Prototype(target) if target == source_name => {
                    Some(MemberFunctionBindingTarget::Prototype(name.to_string()))
                }
                _ => None,
            };
            if let Some(target) = target {
                getter_bindings.push((
                    MemberFunctionBindingKey {
                        target,
                        property: key.property.clone(),
                    },
                    binding,
                ));
            }
        }

        for (key, binding) in self.global_member_setter_binding_entries() {
            let target = match &key.target {
                MemberFunctionBindingTarget::Identifier(target) if target == source_name => {
                    Some(MemberFunctionBindingTarget::Identifier(name.to_string()))
                }
                MemberFunctionBindingTarget::Prototype(target) if target == source_name => {
                    Some(MemberFunctionBindingTarget::Prototype(name.to_string()))
                }
                _ => None,
            };
            if let Some(target) = target {
                setter_bindings.push((
                    MemberFunctionBindingKey {
                        target,
                        property: key.property.clone(),
                    },
                    binding,
                ));
            }
        }

        for (key, binding) in function_bindings {
            self.set_global_member_function_binding(key, binding);
        }
        for (key, capture_slots) in function_capture_slots {
            self.set_global_member_function_capture_slots(key, capture_slots);
        }
        for (key, binding) in getter_bindings {
            self.set_global_member_getter_binding(key, binding);
        }
        for (key, binding) in setter_bindings {
            self.set_global_member_setter_binding(key, binding);
        }
    }
}
