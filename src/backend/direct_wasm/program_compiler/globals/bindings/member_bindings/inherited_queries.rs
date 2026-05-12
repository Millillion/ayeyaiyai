use super::*;

impl DirectWasmCompiler {
    pub(in crate::backend::direct_wasm) fn direct_user_function_return_expression(
        &self,
        function_name: &str,
        depth: usize,
    ) -> Option<Expression> {
        if depth > 8 {
            return None;
        }
        let function = self.registered_function(function_name)?;
        for statement in &function.body {
            let Statement::Return(value) = statement else {
                continue;
            };
            if let Expression::Call { callee, .. } = value
                && let Expression::Identifier(callee_name) = callee.as_ref()
            {
                if let Some(result) =
                    self.infer_static_class_init_call_result_expression(callee_name)
                {
                    return Some(result);
                }
                if self.contains_user_function(callee_name)
                    && let Some(result) =
                        self.direct_user_function_return_expression(callee_name, depth + 1)
                {
                    return Some(result);
                }
            }
            return Some(value.clone());
        }
        None
    }

    pub(in crate::backend::direct_wasm) fn global_inherited_member_function_bindings(
        &self,
        value: &Expression,
    ) -> Vec<ReturnedMemberFunctionBinding> {
        match value {
            Expression::Identifier(source_name) => {
                let mut source_names = vec![source_name.clone()];
                if let Some(function) = self.registered_function(source_name) {
                    if let Some(self_binding) = function.self_binding.as_ref() {
                        source_names.push(self_binding.clone());
                    }
                    if let Some(top_level_binding) = function.top_level_binding.as_ref() {
                        source_names.push(top_level_binding.clone());
                    }
                }
                self.global_member_function_binding_entries()
                    .into_iter()
                    .filter_map(|(key, binding)| match &key.target {
                        MemberFunctionBindingTarget::Identifier(target)
                            if source_names.iter().any(|source_name| target == source_name) =>
                        {
                            let MemberFunctionBindingProperty::String(property) = &key.property
                            else {
                                return None;
                            };
                            Some(ReturnedMemberFunctionBinding {
                                target: ReturnedMemberFunctionBindingTarget::Value,
                                property: property.clone(),
                                binding,
                            })
                        }
                        MemberFunctionBindingTarget::Prototype(target)
                            if source_names.iter().any(|source_name| target == source_name) =>
                        {
                            let MemberFunctionBindingProperty::String(property) = &key.property
                            else {
                                return None;
                            };
                            Some(ReturnedMemberFunctionBinding {
                                target: ReturnedMemberFunctionBindingTarget::Prototype,
                                property: property.clone(),
                                binding,
                            })
                        }
                        _ => None,
                    })
                    .collect()
            }
            Expression::Call { callee, arguments } => {
                if let Some(resolved) = self
                    .infer_static_call_result_expression(callee, arguments)
                    .filter(|resolved| !static_expression_matches(resolved, value))
                {
                    let bindings = self.global_inherited_member_function_bindings(&resolved);
                    if !bindings.is_empty() {
                        return bindings;
                    }
                }
                let Some(LocalFunctionBinding::User(function_name)) =
                    self.infer_global_function_binding(callee)
                else {
                    return Vec::new();
                };
                if let Some(returned) = self
                    .direct_user_function_return_expression(&function_name, 0)
                    .filter(|returned| !static_expression_matches(returned, value))
                {
                    let bindings = self.global_inherited_member_function_bindings(&returned);
                    if !bindings.is_empty() {
                        return bindings;
                    }
                }
                self.user_function_returned_member_function_bindings(&function_name)
            }
            Expression::New { callee, .. } => {
                if let Expression::Identifier(constructor_name) = callee.as_ref() {
                    let mut constructor_names = vec![constructor_name.clone()];
                    if let Some(LocalFunctionBinding::User(function_name)) =
                        self.infer_global_function_binding(callee)
                        && let Some(function) = self.registered_function(&function_name)
                    {
                        if let Some(self_binding) = function.self_binding.as_ref() {
                            constructor_names.push(self_binding.clone());
                        }
                        if let Some(top_level_binding) = function.top_level_binding.as_ref() {
                            constructor_names.push(top_level_binding.clone());
                        }
                    }
                    let bindings = self
                        .global_member_function_binding_entries()
                        .into_iter()
                        .filter_map(|(key, binding)| match (&key.target, &key.property) {
                            (
                                MemberFunctionBindingTarget::Prototype(target),
                                MemberFunctionBindingProperty::String(property),
                            ) if constructor_names
                                .iter()
                                .any(|constructor_name| target == constructor_name) =>
                            {
                                Some(ReturnedMemberFunctionBinding {
                                    target: ReturnedMemberFunctionBindingTarget::Value,
                                    property: property.clone(),
                                    binding,
                                })
                            }
                            _ => None,
                        })
                        .collect::<Vec<_>>();
                    if !bindings.is_empty() {
                        return bindings;
                    }
                }
                let Some(LocalFunctionBinding::User(function_name)) =
                    self.infer_global_function_binding(callee)
                else {
                    return Vec::new();
                };
                self.user_function_returned_member_function_bindings(&function_name)
            }
            _ => Vec::new(),
        }
    }

    pub(in crate::backend::direct_wasm) fn global_inherited_member_getter_bindings(
        &self,
        value: &Expression,
    ) -> Vec<ReturnedMemberFunctionBinding> {
        match value {
            Expression::Identifier(source_name) => self
                .global_member_getter_binding_entries()
                .into_iter()
                .filter_map(|(key, binding)| match &key.target {
                    MemberFunctionBindingTarget::Identifier(target) if target == source_name => {
                        let MemberFunctionBindingProperty::String(property) = &key.property else {
                            return None;
                        };
                        Some(ReturnedMemberFunctionBinding {
                            target: ReturnedMemberFunctionBindingTarget::Value,
                            property: property.clone(),
                            binding,
                        })
                    }
                    MemberFunctionBindingTarget::Prototype(target) if target == source_name => {
                        let MemberFunctionBindingProperty::String(property) = &key.property else {
                            return None;
                        };
                        Some(ReturnedMemberFunctionBinding {
                            target: ReturnedMemberFunctionBindingTarget::Prototype,
                            property: property.clone(),
                            binding,
                        })
                    }
                    _ => None,
                })
                .collect(),
            Expression::Call { callee, arguments } | Expression::New { callee, arguments } => self
                .infer_static_call_result_expression(callee, arguments)
                .and_then(|value| match value {
                    Expression::Object(entries) => Some(
                        entries
                            .into_iter()
                            .filter_map(|entry| match entry {
                                ObjectEntry::Getter { key, getter } => {
                                    let Expression::String(property) = key else {
                                        return None;
                                    };
                                    let binding = self.infer_global_function_binding(&getter)?;
                                    Some(ReturnedMemberFunctionBinding {
                                        target: ReturnedMemberFunctionBindingTarget::Value,
                                        property,
                                        binding,
                                    })
                                }
                                _ => None,
                            })
                            .collect(),
                    ),
                    _ => None,
                })
                .unwrap_or_default(),
            _ => Vec::new(),
        }
    }

    pub(in crate::backend::direct_wasm) fn global_member_capture_slots_by_property_for_name(
        &self,
        name: &str,
    ) -> HashMap<String, BTreeMap<String, String>> {
        self.global_member_function_capture_slot_entries()
            .into_iter()
            .filter_map(|(key, capture_slots)| {
                let property_name = match (&key.target, &key.property) {
                    (
                        MemberFunctionBindingTarget::Identifier(target)
                        | MemberFunctionBindingTarget::Prototype(target),
                        MemberFunctionBindingProperty::String(property),
                    ) if target == name => Some(property.clone()),
                    _ => None,
                }?;
                Some((property_name, capture_slots))
            })
            .collect()
    }

    pub(in crate::backend::direct_wasm) fn has_global_member_bindings_for_name(
        &self,
        name: &str,
    ) -> bool {
        self.global_member_function_binding_entries()
            .into_iter()
            .map(|(key, _)| key)
            .any(|key| {
                matches!(
                    key.target,
                    MemberFunctionBindingTarget::Identifier(target)
                        | MemberFunctionBindingTarget::Prototype(target)
                        if target == name
                )
            })
            || self
                .global_member_getter_binding_entries()
                .into_iter()
                .map(|(key, _)| key)
                .any(|key| {
                    matches!(
                        key.target,
                        MemberFunctionBindingTarget::Identifier(target)
                            | MemberFunctionBindingTarget::Prototype(target)
                            if target == name
                    )
                })
            || self
                .global_member_setter_binding_entries()
                .into_iter()
                .map(|(key, _)| key)
                .any(|key| {
                    matches!(
                        key.target,
                        MemberFunctionBindingTarget::Identifier(target)
                            | MemberFunctionBindingTarget::Prototype(target)
                            if target == name
                    )
                })
            || self
                .global_member_function_capture_slot_entries()
                .into_iter()
                .map(|(key, _)| key)
                .any(|key| {
                    matches!(
                        key.target,
                        MemberFunctionBindingTarget::Identifier(target)
                            | MemberFunctionBindingTarget::Prototype(target)
                            if target == name
                    )
                })
    }
}
