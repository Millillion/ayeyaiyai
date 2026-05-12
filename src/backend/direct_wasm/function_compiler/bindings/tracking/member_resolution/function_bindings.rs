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
        for key in self.primitive_prototype_binding_keys(object, property) {
            if let Some(binding) = self.member_function_binding_entry(&key) {
                if trace_member_bindings {
                    eprintln!("member_binding:primitive binding={binding:?}");
                }
                return Some(binding);
            }
        }

        if let Expression::Object(entries) = object
            && let Some(binding) = self.resolve_object_literal_member_binding(entries, property, 0)
        {
            return Some(binding);
        }
        let materialized_object = self.materialize_static_expression(object);
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
        let resolved = match object {
            Expression::Identifier(name) => {
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
        if resolved.is_some() {
            if trace_member_bindings {
                eprintln!(
                    "member_binding:resolved object={object:?} property={property:?} resolved={resolved:?}"
                );
            }
            return resolved;
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

        if !static_expression_matches(&materialized_object, object)
            || !static_expression_matches(&materialized_property, property)
        {
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
            return Some(
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
                    .collect(),
            );
        }

        let materialized_property = self.materialize_static_expression(property);
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
        let capture_bindings = self.user_function_capture_bindings(&function_name)?;
        let existing_capture_slots = self.resolve_function_expression_capture_slots(function_value);
        let mut capture_slots = BTreeMap::new();
        for capture_name in capture_bindings.keys() {
            if capture_name == "this" {
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
