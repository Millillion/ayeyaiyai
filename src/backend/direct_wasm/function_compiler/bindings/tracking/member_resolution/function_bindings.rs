use super::*;

const IDENTIFIER_FUNCTION_VALUE_CAPTURE_PROPERTY: &str = "__ayy[[FunctionValueCaptureSlots]]";

impl<'a> FunctionCompiler<'a> {
    pub(in crate::backend::direct_wasm) fn identifier_function_value_capture_slots_key(
        name: &str,
    ) -> MemberFunctionBindingKey {
        MemberFunctionBindingKey {
            target: MemberFunctionBindingTarget::Identifier(name.to_string()),
            property: MemberFunctionBindingProperty::String(
                IDENTIFIER_FUNCTION_VALUE_CAPTURE_PROPERTY.to_string(),
            ),
        }
    }

    pub(in crate::backend::direct_wasm) fn resolve_identifier_function_value_capture_slots(
        &self,
        name: &str,
    ) -> Option<BTreeMap<String, String>> {
        let key = Self::identifier_function_value_capture_slots_key(name);
        let slots = self.member_function_capture_slots_entry(&key);
        if std::env::var_os("AYY_TRACE_CAPTURE_BINDINGS").is_some() {
            eprintln!("capture_slots identifier_resolve name={name} slots={slots:?}");
        }
        slots
    }

    pub(in crate::backend::direct_wasm) fn resolve_capture_hidden_source_binding_name(
        &self,
        hidden_name: &str,
    ) -> Option<String> {
        let current_function_name = self.current_function_name()?;
        self.user_function_capture_bindings(current_function_name)?
            .iter()
            .find_map(|(capture_name, capture_hidden_name)| {
                (capture_hidden_name == hidden_name).then_some(capture_name.clone())
            })
    }

    fn resolve_local_array_iterator_next_binding(
        &self,
        object: &Expression,
        property: &Expression,
    ) -> Option<LocalFunctionBinding> {
        let Expression::Identifier(name) = object else {
            return None;
        };
        let Expression::String(property_name) = self.materialize_static_expression(property) else {
            return None;
        };
        if property_name != "next" {
            return None;
        }
        let binding_name = self.resolve_local_array_iterator_binding_name(name)?;
        let binding = self
            .state
            .speculation
            .static_semantics
            .local_array_iterator_binding(&binding_name)?;
        match &binding.source {
            IteratorSourceKind::SimpleGenerator { is_async, .. } => {
                Some(LocalFunctionBinding::User(if *is_async {
                    "__ayy_simple_async_generator_next".to_string()
                } else {
                    "__ayy_simple_generator_next".to_string()
                }))
            }
            _ => None,
        }
    }

    pub(in crate::backend::direct_wasm) fn resolve_member_function_binding_shallow(
        &self,
        object: &Expression,
        property: &Expression,
    ) -> Option<LocalFunctionBinding> {
        self.resolve_member_function_binding_shallow_with_runtime_public_this_guard(
            object, property, true,
        )
    }

    pub(in crate::backend::direct_wasm) fn resolve_member_function_binding_shallow_without_runtime_public_this_guard(
        &self,
        object: &Expression,
        property: &Expression,
    ) -> Option<LocalFunctionBinding> {
        self.resolve_member_function_binding_shallow_with_runtime_public_this_guard(
            object, property, false,
        )
    }

    fn identifier_own_member_function_binding_key(
        &self,
        name: &str,
        property: &Expression,
    ) -> Option<MemberFunctionBindingKey> {
        let property = self.member_function_binding_property(property)?;
        Some(MemberFunctionBindingKey {
            target: MemberFunctionBindingTarget::Identifier(name.to_string()),
            property,
        })
    }

    fn resolve_static_module_global_object_member_function_binding(
        &self,
        property: &Expression,
    ) -> Option<LocalFunctionBinding> {
        let requested_property = self.canonical_object_property_expression(property);
        for user_function in self.user_functions() {
            if !user_function.name.starts_with("__ayy_module_init_") {
                continue;
            }
            let Some(declaration) =
                self.resolve_registered_function_declaration(&user_function.name)
            else {
                continue;
            };
            for statement in &declaration.body {
                let (object, assigned_property, value) = match statement {
                    Statement::AssignMember {
                        object,
                        property,
                        value,
                    } => (object, property, value),
                    Statement::Expression(Expression::AssignMember {
                        object,
                        property,
                        value,
                    }) => (object.as_ref(), property.as_ref(), value.as_ref()),
                    _ => continue,
                };
                if self
                    .resolve_static_global_object_alias_expression(object)
                    .is_none()
                {
                    continue;
                }
                let assigned_property =
                    self.canonical_object_property_expression(assigned_property);
                if !static_expression_matches(&assigned_property, &requested_property) {
                    continue;
                }
                if let Some(binding) = self.resolve_function_binding_from_expression(value) {
                    return Some(binding);
                }
            }
        }
        None
    }

    fn resolve_module_namespace_live_binding_member_initializer(
        &self,
        object: &Expression,
        property: &Expression,
    ) -> Option<(String, Expression)> {
        let Expression::Identifier(name) = object else {
            return None;
        };
        let module_index = Self::module_index_from_namespace_like_identifier(name)?;
        self.resolve_static_dynamic_import_namespace_live_binding_member_binding_initializer_value(
            module_index,
            property,
        )
    }

    fn expression_is_static_script_global_object_reference(&self, object: &Expression) -> bool {
        matches!(object, Expression::Identifier(name) if name == "globalThis" && self.is_unshadowed_builtin_identifier(name))
            || (self.state.speculation.execution_context.top_level_function
                && matches!(object, Expression::This))
            || self
                .resolve_static_global_object_alias_expression(object)
                .is_some()
    }

    fn resolve_script_global_object_member_function_binding(
        &self,
        object: &Expression,
        property: &Expression,
    ) -> Option<LocalFunctionBinding> {
        if !self.expression_is_static_script_global_object_reference(object) {
            return None;
        }
        let Expression::String(property_name) = property else {
            return None;
        };
        if self.backend.lexical_global_binding(property_name).is_some() {
            return None;
        }
        self.global_value_binding(property_name)
            .and_then(|value| self.resolve_function_binding_from_expression(value))
            .or_else(|| self.backend.global_function_binding(property_name).cloned())
            .or_else(|| {
                builtin_function_runtime_value(property_name)
                    .map(|_| LocalFunctionBinding::Builtin(property_name.clone()))
            })
    }

    fn resolve_static_module_export_capture_slots_for_function(
        &self,
        declaration: &FunctionDeclaration,
        function_name: &str,
    ) -> Option<BTreeMap<String, String>> {
        let target_captures = self.user_function_capture_bindings(function_name)?;
        let mut slots = BTreeMap::new();
        for statement in &declaration.body {
            let Statement::Expression(Expression::Call { callee, arguments }) = statement else {
                continue;
            };
            let Expression::Member { object, property } = callee.as_ref() else {
                continue;
            };
            if !matches!(object.as_ref(), Expression::Identifier(name) if name == "Object")
                || !matches!(property.as_ref(), Expression::String(name) if name == "defineProperty")
            {
                continue;
            }
            let [
                CallArgument::Expression(_target),
                CallArgument::Expression(_property),
                CallArgument::Expression(descriptor),
                ..,
            ] = arguments.as_slice()
            else {
                continue;
            };
            let Some(descriptor) = resolve_property_descriptor_definition(descriptor) else {
                continue;
            };
            let Some(getter) = descriptor.getter.as_ref() else {
                continue;
            };
            let Some(LocalFunctionBinding::User(getter_name)) =
                self.resolve_function_binding_from_expression(getter)
            else {
                continue;
            };
            let Some(getter_captures) = self.user_function_capture_bindings(&getter_name) else {
                continue;
            };
            for capture_name in target_captures.keys() {
                if let Some(getter_hidden_name) = getter_captures.get(capture_name)
                    && !slots.contains_key(capture_name)
                {
                    slots.insert(capture_name.clone(), getter_hidden_name.clone());
                }
            }
        }
        (!slots.is_empty()).then_some(slots)
    }

    fn resolve_static_module_global_object_member_function_capture_slots(
        &self,
        object: &Expression,
        property: &Expression,
    ) -> Option<BTreeMap<String, String>> {
        self.resolve_static_global_object_alias_expression(object)?;
        let requested_property = self.canonical_object_property_expression(property);
        for user_function in self.user_functions() {
            if !user_function.name.starts_with("__ayy_module_init_") {
                continue;
            }
            let Some(declaration) =
                self.resolve_registered_function_declaration(&user_function.name)
            else {
                continue;
            };
            for statement in &declaration.body {
                let (assigned_object, assigned_property, value) = match statement {
                    Statement::AssignMember {
                        object,
                        property,
                        value,
                    } => (object, property, value),
                    Statement::Expression(Expression::AssignMember {
                        object,
                        property,
                        value,
                    }) => (object.as_ref(), property.as_ref(), value.as_ref()),
                    _ => continue,
                };
                if self
                    .resolve_static_global_object_alias_expression(assigned_object)
                    .is_none()
                {
                    continue;
                }
                let assigned_property =
                    self.canonical_object_property_expression(assigned_property);
                if !static_expression_matches(&assigned_property, &requested_property) {
                    continue;
                }
                let Some(LocalFunctionBinding::User(function_name)) =
                    self.resolve_function_binding_from_expression(value)
                else {
                    continue;
                };
                if let Some(slots) = self.resolve_static_module_export_capture_slots_for_function(
                    declaration,
                    &function_name,
                ) {
                    return Some(slots);
                }
            }
        }
        None
    }

    fn resolve_member_function_binding_shallow_with_runtime_public_this_guard(
        &self,
        object: &Expression,
        property: &Expression,
        guard_runtime_public_this_resolution: bool,
    ) -> Option<LocalFunctionBinding> {
        if let Some(binding) = self.resolve_local_array_iterator_next_binding(object, property) {
            return Some(binding);
        }
        if let Some(source_expression) = self.direct_iterator_binding_source_expression(object)
            && !std::ptr::eq(source_expression, object)
            && let Some(binding) = self
                .resolve_member_function_binding_shallow_with_runtime_public_this_guard(
                    source_expression,
                    property,
                    guard_runtime_public_this_resolution,
                )
        {
            return Some(binding);
        }
        if let Expression::Identifier(name) = object
            && let Some(key) = self.identifier_own_member_function_binding_key(name, property)
            && let Some(binding) = self.member_function_binding_entry(&key)
        {
            return Some(binding);
        }
        let key = if guard_runtime_public_this_resolution {
            self.member_function_binding_key(object, property)
        } else {
            self.member_function_binding_key_without_runtime_public_this_guard(object, property)
        };
        let resolved = key
            .as_ref()
            .and_then(|key| self.member_function_binding_entry(key));
        if resolved.is_some() {
            return resolved;
        }
        if let Expression::Object(entries) = object {
            return self.resolve_object_literal_member_binding(entries, property, 0);
        }
        None
    }

    pub(in crate::backend::direct_wasm) fn resolve_member_function_binding(
        &self,
        object: &Expression,
        property: &Expression,
    ) -> Option<LocalFunctionBinding> {
        let trace_member_bindings = std::env::var_os("AYY_TRACE_MEMBER_BINDINGS").is_some();
        if trace_member_bindings {
            eprintln!("member_binding:start object={object:?} property={property:?}");
        }
        let _guard = MemberFunctionBindingResolutionGuard::enter(object, property);
        let Some(_shape_guard) =
            MemberBindingResolutionShapeGuard::enter("function", object, property)
        else {
            if trace_member_bindings {
                eprintln!("member_binding:cycle object={object:?} property={property:?}");
            }
            return None;
        };
        if let Some(binding) = self.resolve_local_array_iterator_next_binding(object, property) {
            if trace_member_bindings {
                eprintln!("member_binding:local_array_iterator binding={binding:?}");
            }
            return Some(binding);
        }
        if let Some(source_expression) = self.direct_iterator_binding_source_expression(object)
            && !std::ptr::eq(source_expression, object)
            && let Some(binding) = self.resolve_member_function_binding(source_expression, property)
        {
            return Some(binding);
        }
        let materialized_property = self.materialize_static_expression(property);
        if self.runtime_object_property_shadow_deletion_is_statically_present(
            object,
            &materialized_property,
        ) {
            return None;
        }
        if let Some(live_value) =
            self.resolve_module_namespace_live_binding_member_value(object, &materialized_property)
        {
            let binding = self
                .resolve_function_binding_from_expression(&live_value)
                .or_else(|| {
                    self.resolve_module_namespace_live_binding_member_initializer(
                        object,
                        &materialized_property,
                    )
                    .and_then(|(_, initializer)| {
                        self.resolve_function_binding_from_expression(&initializer)
                    })
                });
            if trace_member_bindings {
                eprintln!(
                    "member_binding:module_namespace_live object={object:?} property={materialized_property:?} value={live_value:?} binding={binding:?}"
                );
            }
            if let Some(binding) = binding {
                return Some(binding);
            }
        }
        if let Some(binding) = self
            .resolve_script_global_object_member_function_binding(object, &materialized_property)
        {
            if trace_member_bindings {
                eprintln!(
                    "member_binding:script_global_object object={object:?} property={materialized_property:?} binding={binding:?}"
                );
            }
            return Some(binding);
        }
        if let Expression::Identifier(name) = object
            && let Some(key) = self.identifier_own_member_function_binding_key(name, property)
        {
            if trace_member_bindings {
                eprintln!("member_binding:identifier_own_try key={key:?}");
            }
            if let Some(binding) = self.member_function_binding_entry(&key) {
                if trace_member_bindings {
                    eprintln!("member_binding:identifier_own binding={binding:?}");
                }
                return Some(binding);
            }
        }
        let key = self.member_function_binding_key(object, property);
        let resolved = key
            .as_ref()
            .and_then(|key| self.member_function_binding_entry(key));
        if resolved.is_some() {
            if trace_member_bindings {
                eprintln!(
                    "member_binding:direct key_present={} resolved={resolved:?}",
                    key.is_some()
                );
            }
            return resolved;
        }
        if let Expression::Identifier(name) = object {
            for key in self.identifier_member_function_binding_fallback_keys(name, property) {
                if trace_member_bindings {
                    eprintln!("member_binding:identifier_fallback_try key={key:?}");
                }
                if let Some(binding) = self.member_function_binding_entry(&key) {
                    if trace_member_bindings {
                        eprintln!("member_binding:identifier_fallback binding={binding:?}");
                    }
                    return Some(binding);
                }
            }
        }
        if trace_member_bindings {
            eprintln!("member_binding:before_primitive object={object:?} property={property:?}");
        }
        for key in self.primitive_prototype_binding_keys(object, property) {
            if trace_member_bindings {
                eprintln!("member_binding:primitive_try key={key:?}");
            }
            if let Some(binding) = self.member_function_binding_entry(&key) {
                if trace_member_bindings {
                    eprintln!("member_binding:primitive binding={binding:?}");
                }
                return Some(binding);
            }
        }
        if trace_member_bindings {
            eprintln!("member_binding:after_primitive object={object:?} property={property:?}");
        }

        if let Expression::Object(entries) = object
            && let Some(binding) = self.resolve_object_literal_member_binding(entries, property, 0)
        {
            return Some(binding);
        }
        if let Expression::Identifier(name) = object {
            let resolved_name = self
                .resolve_current_local_binding(name)
                .map(|(resolved_name, _)| resolved_name)
                .unwrap_or_else(|| name.clone());
            let direct_object_value = self
                .state
                .speculation
                .static_semantics
                .local_object_binding(&resolved_name)
                .or_else(|| self.global_object_binding(name))
                .and_then(|object_binding| {
                    object_binding_lookup_value(object_binding, &materialized_property)
                })
                .cloned();
            if let Some(binding) = direct_object_value
                .as_ref()
                .and_then(|value| self.resolve_function_binding_from_expression(value))
            {
                return Some(binding);
            }
            let constructed_instance_value = self
                .state
                .speculation
                .static_semantics
                .local_value_binding(&resolved_name)
                .or_else(|| {
                    self.state
                        .speculation
                        .static_semantics
                        .local_value_binding(name)
                })
                .or_else(|| self.global_value_binding(name))
                .filter(|value| matches!(value, Expression::Call { .. } | Expression::New { .. }))
                .cloned();
            if trace_member_bindings {
                eprintln!(
                    "member_binding:identifier_constructed_value name={name} resolved_name={resolved_name} value={constructed_instance_value:?}"
                );
            }
            if let Some(constructed_instance_value) = constructed_instance_value.as_ref() {
                let prototype = self
                    .resolve_static_object_prototype_expression(constructed_instance_value)
                    .or_else(|| self.resolve_static_object_prototype_expression(object));
                if trace_member_bindings {
                    eprintln!(
                        "member_binding:identifier_constructed_prototype object={object:?} value={constructed_instance_value:?} prototype={prototype:?}"
                    );
                }
                if let Some(prototype) = prototype
                    && !static_expression_matches(&prototype, object)
                    && let Some(binding) =
                        self.resolve_member_function_binding(&prototype, &materialized_property)
                {
                    if trace_member_bindings {
                        eprintln!(
                            "member_binding:identifier_instance_prototype object={object:?} prototype={prototype:?} property={materialized_property:?} binding={binding:?}"
                        );
                    }
                    return Some(binding);
                }
            }
        }
        if trace_member_bindings {
            eprintln!("member_binding:before_materialize_object object={object:?}");
        }
        if let Expression::String(property_name) = &materialized_property
            && let Some(identity_key) = self.resolve_static_reference_identity_key(object)
            && let Some(prototype_owner) = identity_key.strip_prefix("function-prototype:")
            && let Some(function_name) =
                builtin_prototype_function_name(prototype_owner, property_name)
        {
            return Some(LocalFunctionBinding::Builtin(function_name.to_string()));
        }
        let materialized_object = self.materialize_static_expression(object);
        if trace_member_bindings {
            eprintln!(
                "member_binding:after_materialize_object object={object:?} materialized={materialized_object:?}"
            );
        }
        if matches!(&materialized_object, Expression::Identifier(name) if name == "globalThis")
            && let Some(binding) = self
                .resolve_static_module_global_object_member_function_binding(&materialized_property)
        {
            if trace_member_bindings {
                eprintln!(
                    "member_binding:module_global_object object={object:?} property={materialized_property:?} binding={binding:?}"
                );
            }
            return Some(binding);
        }
        if let (Expression::Identifier(object_name), Expression::String(property_name)) =
            (&materialized_object, &materialized_property)
            && let Some(realm_id) = parse_test262_realm_global_identifier(object_name)
            && builtin_function_runtime_value(property_name).is_some()
        {
            let function_name = if property_name == "eval" {
                test262_realm_eval_builtin_name(realm_id)
            } else {
                property_name.clone()
            };
            return Some(LocalFunctionBinding::Builtin(function_name));
        }
        if let (
            Expression::Member {
                object: prototype_owner,
                property: prototype_property,
            },
            Expression::String(property_name),
        ) = (&materialized_object, &materialized_property)
            && matches!(prototype_property.as_ref(), Expression::String(name) if name == "prototype")
            && let Expression::Identifier(object_name) = prototype_owner.as_ref()
            && self.is_unshadowed_builtin_identifier(object_name)
            && let Some(function_name) = builtin_prototype_function_name(object_name, property_name)
        {
            return Some(LocalFunctionBinding::Builtin(function_name.to_string()));
        }
        if let (Expression::Identifier(object_name), Expression::String(property_name)) =
            (&materialized_object, &materialized_property)
            && self.is_unshadowed_builtin_identifier(object_name)
            && let Some(function_name) = builtin_member_function_name(object_name, property_name)
        {
            return Some(LocalFunctionBinding::Builtin(function_name.to_string()));
        }
        if trace_member_bindings {
            eprintln!(
                "member_binding:before_resolved_match object={object:?} materialized={materialized_object:?} property={materialized_property:?}"
            );
        }
        let resolved = match object {
            Expression::Identifier(name) => {
                if trace_member_bindings {
                    eprintln!(
                        "member_binding:resolved_match_identifier name={name} property={materialized_property:?}"
                    );
                }
                if let Some(index) = argument_index_from_expression(&materialized_property) {
                    if let Some(binding) = self
                        .state
                        .speculation
                        .static_semantics
                        .tracked_array_specialized_function_value(name, index)
                        .map(|value| value.binding.clone())
                    {
                        return Some(binding);
                    }
                    if let Some(value) = self
                        .state
                        .speculation
                        .static_semantics
                        .local_array_binding(name)
                        .or_else(|| self.global_array_binding(name))
                        .and_then(|array_binding| array_binding.values.get(index as usize))
                        .cloned()
                        .flatten()
                    {
                        return self.resolve_function_binding_from_expression(&value);
                    }
                }
                self.state
                    .speculation
                    .static_semantics
                    .local_object_binding(name)
                    .or_else(|| self.global_object_binding(name))
                    .and_then(|object_binding| {
                        object_binding_lookup_value(object_binding, &materialized_property)
                    })
                    .and_then(|value| self.resolve_function_binding_from_expression(value))
                    .or_else(|| {
                        self.resolve_object_binding_from_expression(object)
                            .and_then(|object_binding| {
                                object_binding_lookup_value(&object_binding, &materialized_property)
                                    .cloned()
                            })
                            .and_then(|value| self.resolve_function_binding_from_expression(&value))
                    })
            }
            Expression::Member { object, property } if matches!(property.as_ref(), Expression::String(name) if name == "prototype") =>
            {
                let Expression::Identifier(name) = object.as_ref() else {
                    return None;
                };
                if trace_member_bindings {
                    eprintln!(
                        "member_binding:prototype_object_lookup owner={name} property={materialized_property:?}"
                    );
                }
                self.resolve_function_prototype_object_binding(name)
                    .as_ref()
                    .and_then(|object_binding| {
                        object_binding_lookup_value(object_binding, &materialized_property)
                    })
                    .and_then(|value| self.resolve_function_binding_from_expression(value))
            }
            Expression::New { callee, .. } => {
                let Expression::Identifier(name) = callee.as_ref() else {
                    return None;
                };
                self.resolve_function_prototype_object_binding(name)
                    .as_ref()
                    .and_then(|object_binding| {
                        object_binding_lookup_value(object_binding, &materialized_property)
                    })
                    .and_then(|value| self.resolve_function_binding_from_expression(value))
            }
            _ => self
                .resolve_object_binding_from_expression(object)
                .and_then(|object_binding| {
                    object_binding_lookup_value(&object_binding, &materialized_property).cloned()
                })
                .and_then(|value| self.resolve_function_binding_from_expression(&value)),
        };
        if trace_member_bindings {
            eprintln!(
                "member_binding:after_resolved_match object={object:?} resolved={resolved:?}"
            );
        }
        if resolved.is_some() {
            if trace_member_bindings {
                eprintln!(
                    "member_binding:resolved object={object:?} property={property:?} resolved={resolved:?}"
                );
            }
            return resolved;
        }

        if trace_member_bindings {
            eprintln!("member_binding:before_prototype_lookup object={object:?}");
        }
        if let Some(prototype) = self.resolve_static_object_prototype_expression(object)
            && !static_expression_matches(&prototype, object)
            && let Some(binding) =
                self.resolve_member_function_binding(&prototype, &materialized_property)
        {
            if trace_member_bindings {
                eprintln!(
                    "member_binding:prototype object={object:?} prototype={prototype:?} property={materialized_property:?} binding={binding:?}"
                );
            }
            return Some(binding);
        }
        if trace_member_bindings {
            eprintln!("member_binding:after_prototype_lookup object={object:?}");
        }

        if !static_expression_matches(&materialized_object, object)
            || !static_expression_matches(&materialized_property, property)
        {
            if trace_member_bindings {
                eprintln!(
                    "member_binding:before_materialized_retry object={object:?} materialized={materialized_object:?} property={materialized_property:?}"
                );
            }
            return self
                .resolve_member_function_binding(&materialized_object, &materialized_property);
        }
        if trace_member_bindings {
            eprintln!(
                "member_binding:none object={object:?} property={property:?} materialized_object={materialized_object:?} materialized_property={materialized_property:?}"
            );
        }
        None
    }

    pub(in crate::backend::direct_wasm) fn resolve_member_function_capture_slots(
        &self,
        object: &Expression,
        property: &Expression,
    ) -> Option<BTreeMap<String, String>> {
        let trace_capture_bindings = std::env::var_os("AYY_TRACE_CAPTURE_BINDINGS").is_some();
        if let Some(source_expression) = self.identifier_iterator_binding_source_expression(object)
            && let Some(capture_slots) =
                self.resolve_member_function_capture_slots(&source_expression, property)
        {
            return Some(capture_slots);
        }
        if let Some(source_expression) = self.direct_iterator_binding_source_expression(object)
            && !std::ptr::eq(source_expression, object)
            && let Some(capture_slots) =
                self.resolve_member_function_capture_slots(source_expression, property)
        {
            return Some(capture_slots);
        }
        let materialized_property = self.materialize_static_expression(property);
        if let Some(live_value) =
            self.resolve_module_namespace_live_binding_member_value(object, &materialized_property)
        {
            let capture_slots = self
                .resolve_function_expression_capture_slots(&live_value)
                .or_else(|| {
                    self.resolve_module_namespace_live_binding_member_initializer(
                        object,
                        &materialized_property,
                    )
                    .and_then(|(binding_name, initializer)| {
                        self.resolve_module_namespace_live_function_capture_slots(
                            &binding_name,
                            &initializer,
                            &live_value,
                        )
                    })
                });
            if trace_capture_bindings {
                eprintln!(
                    "capture_slots module_namespace_live object={object:?} property={materialized_property:?} value={live_value:?} slots={capture_slots:?}"
                );
            }
            if let Some(capture_slots) = capture_slots {
                return Some(capture_slots);
            }
        }
        if let Expression::Identifier(name) = object
            && let Some(key) = self.identifier_own_member_function_binding_key(name, property)
        {
            if trace_capture_bindings {
                eprintln!("capture_slots member_own_try key={key:?}");
            }
            if let Some(capture_slots) = self.member_function_capture_slots_entry(&key) {
                if trace_capture_bindings {
                    eprintln!("capture_slots member_own_hit key={key:?} slots={capture_slots:?}");
                }
                let capture_slots =
                    self.complete_member_function_capture_slots_from_binding(&key, capture_slots);
                let capture_slots = self.merge_member_function_capture_slots_from_alias_keys(
                    object,
                    &key,
                    capture_slots,
                );
                return Some(self.resolve_member_function_capture_slot_names(capture_slots));
            }
        }
        if let Some(capture_slots) =
            self.resolve_static_module_global_object_member_function_capture_slots(object, property)
        {
            if trace_capture_bindings {
                eprintln!(
                    "capture_slots module_global_object object={object:?} property={property:?} slots={capture_slots:?}"
                );
            }
            return Some(capture_slots);
        }
        let key = self.member_function_binding_key(object, property)?;
        if trace_capture_bindings {
            eprintln!(
                "capture_slots member_resolve object={object:?} property={property:?} key={key:?}"
            );
        }
        if let Some(capture_slots) = self.member_function_capture_slots_entry(&key) {
            if trace_capture_bindings {
                eprintln!("capture_slots member_hit key={key:?} slots={capture_slots:?}");
            }
            let capture_slots =
                self.complete_member_function_capture_slots_from_binding(&key, capture_slots);
            let capture_slots = self.merge_member_function_capture_slots_from_alias_keys(
                object,
                &key,
                capture_slots,
            );
            return Some(self.resolve_member_function_capture_slot_names(capture_slots));
        }
        if let Expression::Member {
            object: prototype_owner,
            property: prototype_property,
        } = object
            && matches!(prototype_property.as_ref(), Expression::String(name) if name == "prototype")
            && let Expression::Identifier(owner_name) = prototype_owner.as_ref()
        {
            let alias_key = MemberFunctionBindingKey {
                target: MemberFunctionBindingTarget::Prototype(owner_name.clone()),
                property: key.property.clone(),
            };
            if alias_key != key
                && let Some(capture_slots) = self.member_function_capture_slots_entry(&alias_key)
            {
                if trace_capture_bindings {
                    eprintln!(
                        "capture_slots member_alias_hit key={alias_key:?} slots={capture_slots:?}"
                    );
                }
                let capture_slots = self
                    .complete_member_function_capture_slots_from_binding(&alias_key, capture_slots);
                let capture_slots = self.merge_member_function_capture_slots_from_alias_keys(
                    object,
                    &alias_key,
                    capture_slots,
                );
                return Some(self.resolve_member_function_capture_slot_names(capture_slots));
            }
        }

        if let Some(capture_slots) = self.resolve_static_object_member_function_value_capture_slots(
            object,
            &materialized_property,
        ) {
            if trace_capture_bindings {
                eprintln!(
                    "capture_slots member_static_object object={object:?} property={materialized_property:?} slots={capture_slots:?}"
                );
            }
            return Some(capture_slots);
        }

        if let Some(prototype) = self.resolve_static_object_prototype_expression(object)
            && !static_expression_matches(&prototype, object)
            && let Some(capture_slots) =
                self.resolve_member_function_capture_slots(&prototype, property)
        {
            return Some(capture_slots);
        }

        let materialized_object = self.materialize_static_expression(object);
        if !static_expression_matches(&materialized_object, object)
            || !static_expression_matches(&materialized_property, property)
        {
            if trace_capture_bindings {
                eprintln!(
                    "capture_slots member_materialized object={object:?}->{materialized_object:?} property={property:?}->{materialized_property:?}"
                );
            }
            return self.resolve_member_function_capture_slots(
                &materialized_object,
                &materialized_property,
            );
        }

        if trace_capture_bindings {
            eprintln!("capture_slots member_miss key={key:?}");
        }
        None
    }

    fn member_function_capture_slot_alias_keys(
        &self,
        object: &Expression,
        key: &MemberFunctionBindingKey,
    ) -> Vec<MemberFunctionBindingKey> {
        let mut keys = Vec::new();
        let mut push_key = |target: MemberFunctionBindingTarget| {
            let alias_key = MemberFunctionBindingKey {
                target,
                property: key.property.clone(),
            };
            if alias_key != *key && !keys.iter().any(|existing| existing == &alias_key) {
                keys.push(alias_key);
            }
        };

        match object {
            Expression::New { callee, .. } => {
                if let Expression::Identifier(name) = callee.as_ref() {
                    push_key(MemberFunctionBindingTarget::Prototype(name.clone()));
                    push_key(MemberFunctionBindingTarget::Identifier(name.clone()));
                }
            }
            Expression::Member { object, property } if matches!(property.as_ref(), Expression::String(name) if name == "prototype") => {
                if let Expression::Identifier(name) = object.as_ref() {
                    push_key(MemberFunctionBindingTarget::Prototype(name.clone()));
                    push_key(MemberFunctionBindingTarget::Identifier(name.clone()));
                }
            }
            Expression::Identifier(name) => {
                push_key(MemberFunctionBindingTarget::Identifier(name.clone()));
            }
            _ => {}
        }

        keys
    }

    fn merge_member_function_capture_slots_from_alias_keys(
        &self,
        object: &Expression,
        key: &MemberFunctionBindingKey,
        mut capture_slots: BTreeMap<String, String>,
    ) -> BTreeMap<String, String> {
        let trace_capture_bindings = std::env::var_os("AYY_TRACE_CAPTURE_BINDINGS").is_some();
        let key_binding = self.member_function_binding_entry(key);
        for alias_key in self.member_function_capture_slot_alias_keys(object, key) {
            let alias_binding = self.member_function_binding_entry(&alias_key);
            if let (Some(key_binding), Some(alias_binding)) =
                (key_binding.as_ref(), alias_binding.as_ref())
                && key_binding != alias_binding
            {
                if trace_capture_bindings {
                    eprintln!(
                        "capture_slots member_alias_skip key={alias_key:?} binding={alias_binding:?} target_binding={key_binding:?}"
                    );
                }
                continue;
            }
            let Some(alias_slots) = self.member_function_capture_slots_entry(&alias_key) else {
                if trace_capture_bindings {
                    eprintln!("capture_slots member_alias_miss key={alias_key:?}");
                }
                continue;
            };
            if trace_capture_bindings {
                eprintln!(
                    "capture_slots member_alias_merge key={alias_key:?} slots={alias_slots:?}"
                );
            }
            for (capture_name, slot_name) in alias_slots {
                capture_slots.entry(capture_name).or_insert(slot_name);
            }
        }
        capture_slots
    }

    fn resolve_module_namespace_live_function_capture_slots(
        &self,
        binding_name: &str,
        initializer: &Expression,
        live_value: &Expression,
    ) -> Option<BTreeMap<String, String>> {
        let Expression::Identifier(live_slot_name) = live_value else {
            return self.resolve_function_expression_capture_slots(initializer);
        };
        let Some(LocalFunctionBinding::User(function_name)) =
            self.resolve_function_binding_from_expression(initializer)
        else {
            return self.resolve_function_expression_capture_slots(initializer);
        };
        let capture_bindings = self.user_function_capture_bindings(&function_name)?;
        let mut capture_slots = self
            .resolve_function_expression_capture_slots(initializer)
            .unwrap_or_default();
        for capture_name in capture_bindings.keys() {
            if capture_name == binding_name
                || scoped_binding_source_name(capture_name)
                    .is_some_and(|source_name| source_name == binding_name)
                || scoped_binding_source_name(binding_name)
                    .is_some_and(|source_name| source_name == capture_name)
            {
                capture_slots.insert(capture_name.clone(), live_slot_name.clone());
            }
        }
        (!capture_slots.is_empty()).then_some(capture_slots)
    }

    fn complete_member_function_capture_slots_from_binding(
        &self,
        key: &MemberFunctionBindingKey,
        mut capture_slots: BTreeMap<String, String>,
    ) -> BTreeMap<String, String> {
        let Some(LocalFunctionBinding::User(function_name)) =
            self.member_function_binding_entry(key)
        else {
            return capture_slots;
        };
        let Some(capture_bindings) = self.user_function_capture_bindings(&function_name) else {
            return capture_slots;
        };

        let mut capture_names = capture_bindings.keys().cloned().collect::<Vec<_>>();
        capture_names.sort();
        for capture_name in capture_names {
            if capture_slots.contains_key(&capture_name) {
                continue;
            }
            if let Some(slot_name) =
                self.resolve_user_function_capture_slot_binding_name(&capture_name)
            {
                capture_slots.insert(capture_name, slot_name);
            }
        }

        capture_slots
    }

    fn resolve_member_function_capture_slot_names(
        &self,
        capture_slots: BTreeMap<String, String>,
    ) -> BTreeMap<String, String> {
        capture_slots
            .into_iter()
            .map(|(capture_name, slot_name)| {
                let resolved_slot_name = self
                    .resolve_current_local_binding(&slot_name)
                    .map(|(resolved_name, _)| resolved_name)
                    .or_else(|| self.resolve_user_function_capture_hidden_name(&slot_name))
                    .or_else(|| self.resolve_eval_local_function_hidden_name(&slot_name))
                    .unwrap_or(slot_name);
                (capture_name, resolved_slot_name)
            })
            .collect()
    }

    fn identifier_iterator_binding_source_expression(
        &self,
        object: &Expression,
    ) -> Option<Expression> {
        let Expression::Identifier(name) = object else {
            return None;
        };
        let source = self
            .state
            .speculation
            .static_semantics
            .local_value_binding(name)?;
        let iterated = match source {
            Expression::GetIterator(iterated) => Some((**iterated).clone()),
            Expression::Call { callee, arguments }
                if arguments.is_empty()
                    && matches!(
                        callee.as_ref(),
                        Expression::Member { property, .. }
                            if is_symbol_iterator_expression(property)
                    ) =>
            {
                let Expression::Member {
                    object: iterator_object,
                    ..
                } = callee.as_ref()
                else {
                    unreachable!("filtered above");
                };
                Some((**iterator_object).clone())
            }
            _ => None,
        }?;
        let materialized = self.materialize_static_expression(&iterated);
        if static_expression_matches(&materialized, object) {
            Some(iterated)
        } else {
            Some(materialized)
        }
    }

    fn resolve_static_object_member_function_value_capture_slots(
        &self,
        object: &Expression,
        materialized_property: &Expression,
    ) -> Option<BTreeMap<String, String>> {
        let Expression::Identifier(receiver_name) = object else {
            return None;
        };
        let value = self
            .state
            .speculation
            .static_semantics
            .local_object_binding(receiver_name)
            .or_else(|| self.global_object_binding(receiver_name))
            .and_then(|object_binding| {
                object_binding_lookup_value(object_binding, materialized_property).cloned()
            })
            .or_else(|| {
                self.resolve_object_binding_from_expression(object)
                    .and_then(|object_binding| {
                        object_binding_lookup_value(&object_binding, materialized_property).cloned()
                    })
            })?;
        self.synthesize_function_value_capture_slots_for_receiver(&value, receiver_name)
    }

    fn synthesize_function_value_capture_slots_for_receiver(
        &self,
        function_value: &Expression,
        receiver_name: &str,
    ) -> Option<BTreeMap<String, String>> {
        let LocalFunctionBinding::User(function_name) =
            self.resolve_function_binding_from_expression(function_value)?
        else {
            return None;
        };
        let home_object_binding = self
            .user_function(&function_name)
            .and_then(|function| function.home_object_binding.as_deref());
        let capture_bindings = self.user_function_capture_bindings(&function_name)?;
        let existing_capture_slots = self.resolve_function_expression_capture_slots(function_value);
        let mut capture_slots = BTreeMap::new();
        for capture_name in capture_bindings.keys() {
            if capture_name == "this" {
                capture_slots.insert(capture_name.clone(), receiver_name.to_string());
                continue;
            }
            if home_object_binding.is_some_and(|home_object| {
                !home_object.ends_with(".prototype")
                    && (home_object == capture_name
                        || scoped_binding_source_name(home_object)
                            .is_some_and(|source_name| source_name == capture_name)
                        || scoped_binding_source_name(capture_name)
                            .is_some_and(|source_name| source_name == home_object))
            }) {
                capture_slots.insert(capture_name.clone(), receiver_name.to_string());
                continue;
            }
            let slot_name = existing_capture_slots
                .as_ref()
                .and_then(|slots| slots.get(capture_name))
                .cloned()?;
            capture_slots.insert(capture_name.clone(), slot_name);
        }
        (!capture_slots.is_empty()).then_some(capture_slots)
    }

    pub(in crate::backend::direct_wasm) fn resolve_capture_slot_source_binding_name(
        &self,
        slot_name: &str,
    ) -> Option<String> {
        self.state
            .speculation
            .static_semantics
            .capture_slot_source_bindings
            .get(slot_name)
            .cloned()
            .or_else(|| {
                self.state
                    .speculation
                    .static_semantics
                    .local_value_binding(slot_name)
                    .and_then(|value| {
                        let Expression::Identifier(name) =
                            self.materialize_static_expression(value)
                        else {
                            return None;
                        };
                        self.resolve_capture_hidden_source_binding_name(&name)
                    })
            })
    }

    pub(in crate::backend::direct_wasm) fn resolve_capture_slot_static_source_expression(
        &self,
        slot_name: &str,
    ) -> Option<Expression> {
        if let Some(source_name) = self
            .state
            .speculation
            .static_semantics
            .capture_slot_initial_source_bindings
            .get(slot_name)
            .cloned()
        {
            return Some(Expression::Identifier(source_name));
        }
        if let Some(source_name) = self.resolve_capture_slot_source_binding_name(slot_name) {
            return Some(Expression::Identifier(source_name));
        }
        let value = self
            .state
            .speculation
            .static_semantics
            .local_value_binding(slot_name)?;
        let Expression::Identifier(source_name) = self.materialize_static_expression(value) else {
            return None;
        };
        (source_name != slot_name).then_some(Expression::Identifier(source_name))
    }

    pub(in crate::backend::direct_wasm) fn resolve_member_function_capture_source_bindings(
        &self,
        object: &Expression,
        property: &Expression,
    ) -> HashSet<String> {
        let mut names = HashSet::new();
        if let Some(capture_slots) = self.resolve_member_function_capture_slots(object, property) {
            for slot_name in capture_slots.values() {
                if let Some(name) = self.resolve_capture_slot_source_binding_name(slot_name) {
                    names.insert(name);
                }
            }
        }
        names
    }
}
