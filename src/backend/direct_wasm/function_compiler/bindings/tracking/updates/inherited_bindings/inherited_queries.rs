use super::*;

impl<'a> FunctionCompiler<'a> {
    pub(super) fn resolve_static_class_init_new_constructor_name(
        &self,
        callee: &Expression,
    ) -> Option<String> {
        match callee {
            Expression::Identifier(constructor_name) => Some(constructor_name.clone()),
            Expression::Call {
                callee: init_callee,
                arguments: init_arguments,
            } if init_arguments.is_empty() => {
                let Expression::Identifier(function_name) = init_callee.as_ref() else {
                    return None;
                };
                self.infer_static_class_init_call_result_expression(function_name)
                    .and_then(|result| match result {
                        Expression::Identifier(name) => Some(name),
                        _ => None,
                    })
            }
            _ => None,
        }
    }

    fn resolve_static_member_call_return_expression(
        &self,
        callee: &Expression,
        arguments: &[CallArgument],
    ) -> Option<Expression> {
        let LocalFunctionBinding::User(function_name) =
            self.resolve_function_binding_from_expression(callee)?
        else {
            return None;
        };
        let capture_source_bindings =
            self.resolve_function_expression_capture_slots(callee)
                .map(|capture_slots| {
                    capture_slots
                        .into_iter()
                        .map(|(capture_name, slot_name)| {
                            (
                                capture_name,
                                self.snapshot_bound_capture_slot_expression(&slot_name),
                            )
                        })
                        .collect::<HashMap<_, _>>()
                });
        self.resolve_static_return_expression_from_user_function_call(
            &function_name,
            arguments,
            capture_source_bindings.as_ref(),
        )
    }

    fn object_literal_member_function_bindings(
        &self,
        entries: &[crate::ir::hir::ObjectEntry],
    ) -> Vec<ReturnedMemberFunctionBinding> {
        entries
            .iter()
            .filter_map(|entry| {
                let crate::ir::hir::ObjectEntry::Data { key, value } = entry else {
                    return None;
                };
                let Expression::String(property) = key else {
                    return None;
                };
                if matches!(value, Expression::Sequence(_)) {
                    return None;
                }
                let binding = self.resolve_function_binding_from_expression(value)?;
                Some(ReturnedMemberFunctionBinding {
                    target: ReturnedMemberFunctionBindingTarget::Value,
                    property: property.clone(),
                    binding,
                })
            })
            .collect()
    }

    fn object_literal_member_getter_bindings(
        &self,
        entries: &[crate::ir::hir::ObjectEntry],
    ) -> Vec<ReturnedMemberFunctionBinding> {
        entries
            .iter()
            .filter_map(|entry| {
                let crate::ir::hir::ObjectEntry::Getter { key, getter } = entry else {
                    return None;
                };
                let Expression::String(property) = key else {
                    return None;
                };
                let binding = self.resolve_function_binding_from_expression(getter)?;
                Some(ReturnedMemberFunctionBinding {
                    target: ReturnedMemberFunctionBindingTarget::Value,
                    property: property.clone(),
                    binding,
                })
            })
            .collect()
    }

    fn object_literal_member_setter_bindings(
        &self,
        entries: &[crate::ir::hir::ObjectEntry],
    ) -> Vec<ReturnedMemberFunctionBinding> {
        entries
            .iter()
            .filter_map(|entry| {
                let crate::ir::hir::ObjectEntry::Setter { key, setter } = entry else {
                    return None;
                };
                let Expression::String(property) = key else {
                    return None;
                };
                let binding = self.resolve_function_binding_from_expression(setter)?;
                Some(ReturnedMemberFunctionBinding {
                    target: ReturnedMemberFunctionBindingTarget::Value,
                    property: property.clone(),
                    binding,
                })
            })
            .collect()
    }

    pub(in crate::backend::direct_wasm) fn inherited_member_function_bindings(
        &self,
        value: &Expression,
    ) -> Vec<ReturnedMemberFunctionBinding> {
        let trace_inherited_bindings = std::env::var_os("AYY_TRACE_INHERITED_BINDINGS").is_some();
        if trace_inherited_bindings {
            eprintln!("inherited_member_function_bindings:start value={value:?}");
        }
        match value {
            Expression::Identifier(source_name) => {
                if trace_inherited_bindings {
                    eprintln!("inherited_member_function_bindings:identifier source={source_name}");
                }
                let mut source_names = vec![source_name.clone()];
                if let Some(function) = self.resolve_registered_function_declaration(source_name) {
                    if let Some(self_binding) = function.self_binding.as_ref() {
                        source_names.push(self_binding.clone());
                    }
                    if let Some(top_level_binding) = function.top_level_binding.as_ref() {
                        source_names.push(top_level_binding.clone());
                    }
                }
                if let Some(scoped_source_name) = scoped_binding_source_name(source_name) {
                    source_names.push(scoped_source_name.to_string());
                }
                let local_bindings = self
                    .state
                    .speculation
                    .static_semantics
                    .objects
                    .member_function_bindings
                    .iter()
                    .map(|(key, binding)| (key.clone(), binding.clone()));
                let global_bindings = self.backend.global_member_function_binding_entries();
                local_bindings
                    .chain(global_bindings)
                    .filter_map(|(key, binding)| match &key.target {
                        MemberFunctionBindingTarget::Identifier(target)
                            if source_names
                                .iter()
                                .any(|source_name| target.as_str() == source_name.as_str()) =>
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
                            if source_names
                                .iter()
                                .any(|source_name| target.as_str() == source_name.as_str()) =>
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
            Expression::New { callee, .. } => {
                let Some(constructor_name) =
                    self.resolve_static_class_init_new_constructor_name(callee)
                else {
                    return Vec::new();
                };
                let constructor_is_runtime_alias = matches!(
                    callee.as_ref(),
                    Expression::Identifier(alias)
                        if alias == &constructor_name && !alias.starts_with("__ayy_class_ctor_")
                );
                let normalized_target = (!constructor_is_runtime_alias)
                    .then(|| {
                        self.resolve_function_binding_from_expression(&Expression::Identifier(
                            constructor_name.clone(),
                        ))
                        .and_then(|binding| self.function_prototype_binding_owner_name(&binding))
                    })
                    .flatten();
                let local_bindings = self
                    .state
                    .speculation
                    .static_semantics
                    .objects
                    .member_function_bindings
                    .iter()
                    .map(|(key, binding)| (key.clone(), binding.clone()));
                let global_bindings = self.backend.global_member_function_binding_entries();
                let prototype_bindings = local_bindings
                    .chain(global_bindings)
                    .filter_map(|(key, binding)| match (&key.target, &key.property) {
                        (
                            MemberFunctionBindingTarget::Prototype(target),
                            MemberFunctionBindingProperty::String(property),
                        ) if target == &constructor_name
                            || normalized_target
                                .as_ref()
                                .is_some_and(|normalized| target == normalized) =>
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
                if constructor_is_runtime_alias {
                    return prototype_bindings;
                }
                let mut bindings = self
                    .resolve_object_binding_from_expression(value)
                    .map(|object_binding| {
                        object_binding
                            .string_properties
                            .iter()
                            .filter_map(|(property, value)| {
                                let binding =
                                    self.resolve_function_binding_from_expression(value)?;
                                Some(ReturnedMemberFunctionBinding {
                                    target: ReturnedMemberFunctionBindingTarget::Value,
                                    property: property.clone(),
                                    binding,
                                })
                            })
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                bindings.extend(prototype_bindings);
                if !bindings.is_empty() {
                    return bindings;
                }
                let Some(prototype_binding) = self
                    .resolve_function_prototype_object_binding(&constructor_name)
                    .or_else(|| {
                        normalized_target.as_ref().and_then(|normalized_target| {
                            self.resolve_function_prototype_object_binding(normalized_target)
                        })
                    })
                else {
                    return Vec::new();
                };
                prototype_binding
                    .string_properties
                    .iter()
                    .filter_map(|(property, value)| {
                        let binding = self.resolve_function_binding_from_expression(value)?;
                        Some(ReturnedMemberFunctionBinding {
                            target: ReturnedMemberFunctionBindingTarget::Value,
                            property: property.clone(),
                            binding,
                        })
                    })
                    .collect()
            }
            Expression::Call { callee, arguments } => {
                if let Expression::Member { object, property } = callee.as_ref()
                    && matches!(property.as_ref(), Expression::String(name) if name == "next")
                    && self.simple_generator_source_metadata(object).is_some()
                {
                    if trace_inherited_bindings {
                        eprintln!("inherited_member_function_bindings:call:simple_generator_next");
                    }
                    return Vec::new();
                }
                if trace_inherited_bindings {
                    eprintln!("inherited_member_function_bindings:call:static_result:start");
                }
                if let Some((resolved, _)) = self
                    .resolve_static_call_result_expression_with_context(
                        callee,
                        arguments,
                        self.current_function_name(),
                    )
                    && !static_expression_matches(&resolved, value)
                {
                    if trace_inherited_bindings {
                        eprintln!(
                            "inherited_member_function_bindings:call:static_result value={resolved:?}"
                        );
                    }
                    let bindings = self.inherited_member_function_bindings(&resolved);
                    if !bindings.is_empty() {
                        if trace_inherited_bindings {
                            eprintln!(
                                "inherited_member_function_bindings:call:static_result:done count={}",
                                bindings.len()
                            );
                        }
                        return bindings;
                    }
                }
                if let Some(resolved) = self
                    .resolve_static_member_call_return_expression(callee, arguments)
                    .filter(|resolved| !static_expression_matches(resolved, value))
                {
                    let bindings = self.inherited_member_function_bindings(&resolved);
                    if !bindings.is_empty() {
                        return bindings;
                    }
                }
                if trace_inherited_bindings {
                    eprintln!("inherited_member_function_bindings:call:object_binding:start");
                }
                if let Some(object_binding) =
                    self.resolve_returned_object_binding_from_call(callee, arguments)
                {
                    let bindings = object_binding
                        .string_properties
                        .iter()
                        .filter_map(|(property, value)| {
                            let binding = self.resolve_function_binding_from_expression(value)?;
                            Some(ReturnedMemberFunctionBinding {
                                target: ReturnedMemberFunctionBindingTarget::Value,
                                property: property.clone(),
                                binding,
                            })
                        })
                        .collect::<Vec<_>>();
                    if !bindings.is_empty() {
                        if trace_inherited_bindings {
                            eprintln!(
                                "inherited_member_function_bindings:call:object_binding:done count={}",
                                bindings.len()
                            );
                        }
                        return bindings;
                    }
                }
                if trace_inherited_bindings {
                    eprintln!("inherited_member_function_bindings:call:user_function:start");
                }
                let Some(user_function) = self.resolve_user_function_from_expression(callee) else {
                    if trace_inherited_bindings {
                        eprintln!("inherited_member_function_bindings:call:user_function:none");
                    }
                    return Vec::new();
                };
                if trace_inherited_bindings {
                    eprintln!(
                        "inherited_member_function_bindings:call:user_function:done count={}",
                        user_function.returned_member_function_bindings.len()
                    );
                }
                user_function.returned_member_function_bindings.clone()
            }
            Expression::Object(entries) => self.object_literal_member_function_bindings(entries),
            _ => Vec::new(),
        }
    }

    pub(in crate::backend::direct_wasm) fn inherited_member_getter_bindings(
        &self,
        value: &Expression,
    ) -> Vec<ReturnedMemberFunctionBinding> {
        match value {
            Expression::Identifier(source_name) => {
                let local_bindings = self
                    .state
                    .speculation
                    .static_semantics
                    .objects
                    .member_getter_bindings
                    .iter()
                    .map(|(key, binding)| (key.clone(), binding.clone()));
                let global_bindings = self.backend.global_member_getter_binding_entries();
                local_bindings
                    .chain(global_bindings)
                    .filter_map(|(key, binding)| match &key.target {
                        MemberFunctionBindingTarget::Identifier(target)
                            if target.as_str() == source_name.as_str() =>
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
                            if target.as_str() == source_name.as_str() =>
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
            Expression::New { callee, .. } => {
                let Some(constructor_name) =
                    self.resolve_static_class_init_new_constructor_name(callee)
                else {
                    return Vec::new();
                };
                let constructor_is_runtime_alias = matches!(
                    callee.as_ref(),
                    Expression::Identifier(alias)
                        if alias == &constructor_name && !alias.starts_with("__ayy_class_ctor_")
                );
                let normalized_target = (!constructor_is_runtime_alias)
                    .then(|| {
                        self.resolve_function_binding_from_expression(&Expression::Identifier(
                            constructor_name.clone(),
                        ))
                        .and_then(|binding| self.function_prototype_binding_owner_name(&binding))
                    })
                    .flatten();
                let local_bindings = self
                    .state
                    .speculation
                    .static_semantics
                    .objects
                    .member_getter_bindings
                    .iter()
                    .map(|(key, binding)| (key.clone(), binding.clone()));
                let global_bindings = self.backend.global_member_getter_binding_entries();
                local_bindings
                    .chain(global_bindings)
                    .filter_map(|(key, binding)| match (&key.target, &key.property) {
                        (
                            MemberFunctionBindingTarget::Prototype(target),
                            MemberFunctionBindingProperty::String(property),
                        ) if target == &constructor_name
                            || normalized_target
                                .as_ref()
                                .is_some_and(|normalized| target == normalized) =>
                        {
                            Some(ReturnedMemberFunctionBinding {
                                target: ReturnedMemberFunctionBindingTarget::Value,
                                property: property.clone(),
                                binding,
                            })
                        }
                        _ => None,
                    })
                    .collect()
            }
            Expression::Call { callee, arguments } => {
                if let Some((value, _)) = self.resolve_static_call_result_expression_with_context(
                    callee,
                    arguments,
                    self.current_function_name(),
                ) {
                    let bindings = self.inherited_member_getter_bindings(&value);
                    if !bindings.is_empty() {
                        return bindings;
                    }
                }
                self.resolve_static_member_call_return_expression(callee, arguments)
                    .map(|value| self.inherited_member_getter_bindings(&value))
                    .filter(|bindings| !bindings.is_empty())
                    .unwrap_or_default()
            }
            Expression::Object(entries) => self.object_literal_member_getter_bindings(entries),
            _ => Vec::new(),
        }
    }

    pub(in crate::backend::direct_wasm) fn inherited_member_setter_bindings(
        &self,
        value: &Expression,
    ) -> Vec<ReturnedMemberFunctionBinding> {
        match value {
            Expression::Identifier(source_name) => {
                let local_bindings = self
                    .state
                    .speculation
                    .static_semantics
                    .objects
                    .member_setter_bindings
                    .iter()
                    .map(|(key, binding)| (key.clone(), binding.clone()));
                let global_bindings = self.backend.global_member_setter_binding_entries();
                local_bindings
                    .chain(global_bindings)
                    .filter_map(|(key, binding)| match &key.target {
                        MemberFunctionBindingTarget::Identifier(target)
                            if target.as_str() == source_name.as_str() =>
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
                            if target.as_str() == source_name.as_str() =>
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
            Expression::New { callee, .. } => {
                let Some(constructor_name) =
                    self.resolve_static_class_init_new_constructor_name(callee)
                else {
                    return Vec::new();
                };
                let constructor_is_runtime_alias = matches!(
                    callee.as_ref(),
                    Expression::Identifier(alias)
                        if alias == &constructor_name && !alias.starts_with("__ayy_class_ctor_")
                );
                let normalized_target = (!constructor_is_runtime_alias)
                    .then(|| {
                        self.resolve_function_binding_from_expression(&Expression::Identifier(
                            constructor_name.clone(),
                        ))
                        .and_then(|binding| self.function_prototype_binding_owner_name(&binding))
                    })
                    .flatten();
                let local_bindings = self
                    .state
                    .speculation
                    .static_semantics
                    .objects
                    .member_setter_bindings
                    .iter()
                    .map(|(key, binding)| (key.clone(), binding.clone()));
                let global_bindings = self.backend.global_member_setter_binding_entries();
                local_bindings
                    .chain(global_bindings)
                    .filter_map(|(key, binding)| match (&key.target, &key.property) {
                        (
                            MemberFunctionBindingTarget::Prototype(target),
                            MemberFunctionBindingProperty::String(property),
                        ) if target == &constructor_name
                            || normalized_target
                                .as_ref()
                                .is_some_and(|normalized| target == normalized) =>
                        {
                            Some(ReturnedMemberFunctionBinding {
                                target: ReturnedMemberFunctionBindingTarget::Value,
                                property: property.clone(),
                                binding,
                            })
                        }
                        _ => None,
                    })
                    .collect()
            }
            Expression::Call { callee, arguments } => {
                if let Some((value, _)) = self.resolve_static_call_result_expression_with_context(
                    callee,
                    arguments,
                    self.current_function_name(),
                ) {
                    let bindings = self.inherited_member_setter_bindings(&value);
                    if !bindings.is_empty() {
                        return bindings;
                    }
                }
                self.resolve_static_member_call_return_expression(callee, arguments)
                    .map(|value| self.inherited_member_setter_bindings(&value))
                    .filter(|bindings| !bindings.is_empty())
                    .unwrap_or_default()
            }
            Expression::Object(entries) => self.object_literal_member_setter_bindings(entries),
            _ => Vec::new(),
        }
    }
}
