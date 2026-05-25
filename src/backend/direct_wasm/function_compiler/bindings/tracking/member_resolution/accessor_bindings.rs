use super::*;

impl<'a> FunctionCompiler<'a> {
    fn expression_is_static_undefined_value(value: &Expression) -> bool {
        matches!(value, Expression::Undefined)
            || matches!(value, Expression::Number(number) if *number == JS_UNDEFINED_TAG as f64)
    }

    fn runtime_shadow_has_own_non_accessor_data_value(
        &self,
        object: &Expression,
        property: &Expression,
    ) -> bool {
        let Some(shadow_binding_name) =
            self.runtime_object_property_shadow_binding_name_for_expression(object, property)
        else {
            return false;
        };
        self.global_value_binding(&shadow_binding_name)
            .or_else(|| {
                self.backend
                    .shared_global_semantics
                    .values
                    .value_bindings
                    .get(&shadow_binding_name)
            })
            .is_some_and(|value| !Self::expression_is_static_undefined_value(value))
    }

    fn runtime_shadow_has_accessor_descriptor(
        &self,
        object: &Expression,
        property: &Expression,
    ) -> bool {
        let Some(shadow_binding_name) =
            self.runtime_object_property_shadow_binding_name_for_expression(object, property)
        else {
            return false;
        };
        self.backend
            .global_property_descriptor(&shadow_binding_name)
            .or_else(|| {
                self.backend
                    .shared_global_semantics
                    .values
                    .property_descriptor(&shadow_binding_name)
            })
            .is_some_and(|descriptor| {
                descriptor.has_get
                    || descriptor.has_set
                    || descriptor.getter.is_some()
                    || descriptor.setter.is_some()
            })
    }

    fn object_has_own_non_getter_property_binding(
        &self,
        object: &Expression,
        property: &Expression,
    ) -> bool {
        if self.runtime_shadow_has_own_non_accessor_data_value(object, property) {
            return true;
        }
        if self.runtime_shadow_has_accessor_descriptor(object, property) {
            return false;
        }

        let resolved_object = self
            .resolve_bound_alias_expression(object)
            .filter(|resolved| !static_expression_matches(resolved, object));
        let materialized_object = self.materialize_static_expression(object);
        let resolved_property = self.resolve_property_key_expression(property).or_else(|| {
            self.resolve_bound_alias_expression(property)
                .filter(|resolved| !static_expression_matches(resolved, property))
        });
        let materialized_property = self.materialize_static_expression(property);

        let object_candidates = [
            Some(object),
            resolved_object.as_ref(),
            (!static_expression_matches(&materialized_object, object))
                .then_some(&materialized_object),
        ];
        let property_candidates = [
            Some(property),
            resolved_property.as_ref(),
            (!static_expression_matches(&materialized_property, property))
                .then_some(&materialized_property),
        ];

        for object_candidate in object_candidates.into_iter().flatten() {
            let object_binding = self
                .resolve_object_binding_from_expression(object_candidate)
                .or_else(|| match object_candidate {
                    Expression::Identifier(name) => self
                        .resolve_identifier_object_binding_fallback(name)
                        .or_else(|| self.resolve_runtime_shadow_object_binding(name)),
                    _ => None,
                });

            for property_candidate in property_candidates.into_iter().flatten() {
                let canonical_property =
                    self.canonical_object_property_expression(property_candidate);
                if let Some(property_name) =
                    static_property_name_from_expression(&canonical_property)
                    && self.function_binding_has_synthetic_own_data_property(
                        object_candidate,
                        &property_name,
                    )
                {
                    return true;
                }
                if let Some(object_binding) = object_binding.as_ref() {
                    if let Some(descriptor) =
                        object_binding_lookup_descriptor(object_binding, property_candidate)
                            .or_else(|| {
                                object_binding_lookup_descriptor(
                                    object_binding,
                                    &canonical_property,
                                )
                            })
                    {
                        if descriptor.getter.is_none() {
                            return true;
                        }
                        let data_value =
                            object_binding_lookup_value(object_binding, property_candidate)
                                .or_else(|| {
                                    object_binding_lookup_value(object_binding, &canonical_property)
                                });
                        if data_value.is_some_and(|value| !matches!(value, Expression::Undefined)) {
                            return true;
                        }
                        continue;
                    }
                    if object_binding_lookup_value(object_binding, property_candidate).is_some()
                        || object_binding_lookup_value(object_binding, &canonical_property)
                            .is_some()
                    {
                        return true;
                    }
                }
            }
        }

        false
    }

    fn function_binding_has_synthetic_own_data_property(
        &self,
        object: &Expression,
        property_name: &str,
    ) -> bool {
        if !matches!(object, Expression::Identifier(_)) {
            return false;
        }
        if self.function_binding_resolution_is_active() {
            return false;
        }
        let Some(function_binding) = self.resolve_function_binding_from_expression(object) else {
            return false;
        };
        match property_name {
            "prototype" => self
                .default_function_prototype_object_binding(&function_binding)
                .is_some(),
            "name" | "length" => true,
            _ => false,
        }
    }

    fn member_binding_key_targets_current_local_identifier(
        &self,
        key: &MemberFunctionBindingKey,
    ) -> bool {
        let MemberFunctionBindingTarget::Identifier(name) = &key.target else {
            return false;
        };
        self.resolve_current_local_binding(name).is_some()
            || self
                .state
                .speculation
                .static_semantics
                .has_local_object_binding(name)
    }

    fn member_getter_binding_entry_for_scoped_resolution(
        &self,
        key: &MemberFunctionBindingKey,
    ) -> Option<LocalFunctionBinding> {
        if self.member_binding_key_targets_current_local_identifier(key) {
            return self
                .state
                .speculation
                .static_semantics
                .objects
                .member_getter_bindings
                .get(key)
                .cloned();
        }
        self.member_getter_binding_entry(key)
    }

    fn member_setter_binding_entry_for_scoped_resolution(
        &self,
        key: &MemberFunctionBindingKey,
    ) -> Option<LocalFunctionBinding> {
        if self.member_binding_key_targets_current_local_identifier(key) {
            return self
                .state
                .speculation
                .static_semantics
                .objects
                .member_setter_bindings
                .get(key)
                .cloned();
        }
        self.member_setter_binding_entry(key)
    }

    pub(in crate::backend::direct_wasm) fn resolve_member_getter_binding(
        &self,
        object: &Expression,
        property: &Expression,
    ) -> Option<LocalFunctionBinding> {
        let trace_member_bindings = std::env::var_os("AYY_TRACE_MEMBER_BINDINGS").is_some();
        let Some(_shape_guard) =
            MemberBindingResolutionShapeGuard::enter("getter", object, property)
        else {
            if trace_member_bindings {
                eprintln!("member_getter_binding:cycle object={object:?} property={property:?}");
            }
            return None;
        };
        let trace_private = std::env::var_os("AYY_TRACE_PRIVATE_MEMBER_LOOKUP").is_some()
            && matches!(property, Expression::String(name) if name.starts_with("__ayy$private$"));
        let format_key = |key: &MemberFunctionBindingKey| {
            let target = match &key.target {
                MemberFunctionBindingTarget::Identifier(name) => format!("id:{name}"),
                MemberFunctionBindingTarget::Prototype(name) => format!("proto:{name}"),
            };
            let property = match &key.property {
                MemberFunctionBindingProperty::String(name) => format!("str:{name}"),
                MemberFunctionBindingProperty::Symbol(name) => format!("sym:{name}"),
                MemberFunctionBindingProperty::SymbolExpression(name) => {
                    format!("symexpr:{name}")
                }
            };
            format!("{target}/{property}")
        };
        let key = self.member_function_binding_key(object, property);
        if trace_private {
            let alias = self.resolve_bound_alias_expression(object);
            let home_object = self
                .current_function_name()
                .and_then(|name| self.resolve_home_object_name_for_function(name));
            eprintln!(
                "private_lookup getter current_fn={:?} home_object={:?} object={:?} alias={:?} property={:?} key={:?}",
                self.current_function_name(),
                home_object,
                object,
                alias,
                property,
                key.as_ref().map(&format_key),
            );
        }
        let resolved = key
            .as_ref()
            .and_then(|key| self.member_getter_binding_entry_for_scoped_resolution(key));
        if is_private_property_name_expression(property) && resolved.is_some() {
            return resolved;
        }
        if self.object_has_own_non_getter_property_binding(object, property) {
            if trace_member_bindings {
                eprintln!(
                    "member_getter_binding:own_non_getter_blocks_direct object={object:?} property={property:?}"
                );
            }
            return None;
        }
        if trace_member_bindings {
            eprintln!(
                "member_getter_binding:direct object={object:?} property={property:?} key={key:?} resolved={resolved:?}"
            );
        }
        if trace_private {
            eprintln!(
                "private_lookup getter current_fn={:?} resolved={:?}",
                self.current_function_name(),
                resolved,
            );
        }
        if resolved.is_some() {
            return resolved;
        }
        for candidate in self.iterator_step_member_static_value_binding_candidates(object) {
            if let Some(binding) = self.resolve_member_getter_binding(&candidate, property) {
                return Some(binding);
            }
        }
        if let Expression::Identifier(name) = object {
            for key in self.identifier_member_function_binding_fallback_keys(name, property) {
                if trace_member_bindings {
                    eprintln!("member_getter_binding:identifier_fallback_try key={key:?}");
                }
                if let Some(binding) = self.member_getter_binding_entry_for_scoped_resolution(&key)
                {
                    if trace_member_bindings {
                        eprintln!("member_getter_binding:identifier_fallback binding={binding:?}");
                    }
                    return Some(binding);
                }
            }
        }
        if let Some(binding) = self.resolve_descriptor_getter_binding(object, property) {
            if trace_member_bindings {
                eprintln!("member_getter_binding:descriptor binding={binding:?}");
            }
            return Some(binding);
        }
        if let Expression::Object(entries) = object
            && let Some(binding) = self.resolve_object_literal_member_binding(entries, property, 1)
        {
            return Some(binding);
        }
        for key in self.primitive_prototype_binding_keys(object, property) {
            if let Some(binding) = self.member_getter_binding_entry(&key) {
                return Some(binding);
            }
        }

        let materialized_object = self.materialize_static_expression(object);
        let materialized_property = self.materialize_static_expression(property);
        if let Some(prototype) = self.resolve_static_object_prototype_expression(object)
            && !static_expression_matches(&prototype, object)
            && let Some(binding) =
                self.resolve_member_getter_binding(&prototype, &materialized_property)
        {
            return Some(binding);
        }
        if !static_expression_matches(&materialized_object, object)
            || !static_expression_matches(&materialized_property, property)
        {
            return self
                .resolve_member_getter_binding(&materialized_object, &materialized_property);
        }
        None
    }

    pub(in crate::backend::direct_wasm) fn resolve_member_getter_binding_shallow(
        &self,
        object: &Expression,
        property: &Expression,
    ) -> Option<LocalFunctionBinding> {
        let key = self.member_function_binding_key(object, property);
        let resolved = key
            .as_ref()
            .and_then(|key| self.member_getter_binding_entry(key));
        if resolved.is_some() {
            return resolved;
        }
        if let Expression::Object(entries) = object {
            return self.resolve_object_literal_member_binding(entries, property, 1);
        }
        None
    }

    fn resolve_descriptor_getter_binding(
        &self,
        object: &Expression,
        property: &Expression,
    ) -> Option<LocalFunctionBinding> {
        let resolved_object = self
            .resolve_bound_alias_expression(object)
            .filter(|resolved| !static_expression_matches(resolved, object));
        let materialized_object = self.materialize_static_expression(object);
        let resolved_property = self.resolve_property_key_expression(property).or_else(|| {
            self.resolve_bound_alias_expression(property)
                .filter(|resolved| !static_expression_matches(resolved, property))
        });
        let materialized_property = self.materialize_static_expression(property);

        let object_candidates = [
            Some(object),
            resolved_object.as_ref(),
            (!static_expression_matches(&materialized_object, object))
                .then_some(&materialized_object),
        ];
        let property_candidates = [
            Some(property),
            resolved_property.as_ref(),
            (!static_expression_matches(&materialized_property, property))
                .then_some(&materialized_property),
        ];

        for object_candidate in object_candidates.into_iter().flatten() {
            let object_binding = self
                .resolve_object_binding_from_expression(object_candidate)
                .or_else(|| match object_candidate {
                    Expression::Identifier(name) => self
                        .resolve_identifier_object_binding_fallback(name)
                        .or_else(|| self.resolve_runtime_shadow_object_binding(name)),
                    _ => None,
                });
            let Some(object_binding) = object_binding else {
                continue;
            };

            for property_candidate in property_candidates.into_iter().flatten() {
                let canonical_property =
                    self.canonical_object_property_expression(property_candidate);
                let descriptor =
                    object_binding_lookup_descriptor(&object_binding, property_candidate).or_else(
                        || object_binding_lookup_descriptor(&object_binding, &canonical_property),
                    );
                let Some(descriptor) = descriptor else {
                    continue;
                };
                let Some(getter) = descriptor.getter.as_ref() else {
                    continue;
                };
                let binding = self.resolve_function_binding_from_expression(getter);
                if binding.is_some() {
                    return binding;
                }
            }
        }
        None
    }

    pub(in crate::backend::direct_wasm) fn resolve_member_setter_binding(
        &self,
        object: &Expression,
        property: &Expression,
    ) -> Option<LocalFunctionBinding> {
        self.resolve_member_setter_binding_with_context(
            object,
            property,
            self.current_function_name(),
        )
    }

    pub(in crate::backend::direct_wasm) fn resolve_member_setter_binding_with_context(
        &self,
        object: &Expression,
        property: &Expression,
        current_function_name: Option<&str>,
    ) -> Option<LocalFunctionBinding> {
        let trace_member_bindings = std::env::var_os("AYY_TRACE_MEMBER_BINDINGS").is_some();
        let Some(_shape_guard) =
            MemberBindingResolutionShapeGuard::enter("setter", object, property)
        else {
            if trace_member_bindings {
                eprintln!("member_setter_binding:cycle object={object:?} property={property:?}");
            }
            return None;
        };
        let key =
            self.member_function_binding_key_with_context(object, property, current_function_name);
        let resolved = key
            .as_ref()
            .and_then(|key| self.member_setter_binding_entry_for_scoped_resolution(key));
        if trace_member_bindings {
            eprintln!(
                "member_setter_binding:direct object={object:?} property={property:?} key={key:?} resolved={resolved:?}"
            );
        }
        if resolved.is_some() {
            return resolved;
        }
        for candidate in self.iterator_step_member_static_value_binding_candidates(object) {
            if let Some(binding) = self.resolve_member_setter_binding_with_context(
                &candidate,
                property,
                current_function_name,
            ) {
                return Some(binding);
            }
        }
        if let Expression::Identifier(name) = object {
            for key in self.identifier_member_function_binding_fallback_keys(name, property) {
                if trace_member_bindings {
                    eprintln!("member_setter_binding:identifier_fallback_try key={key:?}");
                }
                if let Some(binding) = self.member_setter_binding_entry_for_scoped_resolution(&key)
                {
                    if trace_member_bindings {
                        eprintln!("member_setter_binding:identifier_fallback binding={binding:?}");
                    }
                    return Some(binding);
                }
            }
        }
        if let Some(binding) =
            self.resolve_descriptor_setter_binding(object, property, current_function_name)
        {
            if trace_member_bindings {
                eprintln!("member_setter_binding:descriptor binding={binding:?}");
            }
            return Some(binding);
        }
        if self.object_has_own_non_getter_property_binding(object, property) {
            if trace_member_bindings {
                eprintln!(
                    "member_setter_binding:own_non_setter_blocks_inherited object={object:?} property={property:?}"
                );
            }
            return None;
        }
        for key in self.primitive_prototype_binding_keys(object, property) {
            if let Some(binding) = self.member_setter_binding_entry(&key) {
                return Some(binding);
            }
        }

        if let Expression::Object(entries) = object
            && let Some(binding) = self.resolve_object_literal_member_binding(entries, property, 2)
        {
            return Some(binding);
        }

        let materialized_object = self.materialize_static_expression(object);
        let materialized_property = self.materialize_static_expression(property);
        if let Some(prototype) = self.resolve_static_object_prototype_expression(object)
            && !static_expression_matches(&prototype, object)
            && let Some(binding) = self.resolve_member_setter_binding_with_context(
                &prototype,
                &materialized_property,
                current_function_name,
            )
        {
            return Some(binding);
        }

        if !static_expression_matches(&materialized_object, object)
            || !static_expression_matches(&materialized_property, property)
        {
            return self.resolve_member_setter_binding_with_context(
                &materialized_object,
                &materialized_property,
                current_function_name,
            );
        }
        None
    }

    fn resolve_descriptor_setter_binding(
        &self,
        object: &Expression,
        property: &Expression,
        current_function_name: Option<&str>,
    ) -> Option<LocalFunctionBinding> {
        let object_binding = self
            .resolve_object_binding_from_expression(object)
            .or_else(|| match object {
                Expression::Identifier(name) => self
                    .resolve_identifier_object_binding_fallback(name)
                    .or_else(|| self.resolve_runtime_shadow_object_binding(name)),
                _ => None,
            })?;
        let canonical_property = self.canonical_object_property_expression(property);
        let descriptor = object_binding_lookup_descriptor(&object_binding, property)
            .or_else(|| object_binding_lookup_descriptor(&object_binding, &canonical_property))?;
        let setter = descriptor.setter.as_ref()?;
        self.resolve_function_binding_from_expression_with_context(setter, current_function_name)
    }

    pub(in crate::backend::direct_wasm) fn resolve_member_setter_binding_shallow(
        &self,
        object: &Expression,
        property: &Expression,
    ) -> Option<LocalFunctionBinding> {
        let key = self.member_function_binding_key(object, property);
        let resolved = key
            .as_ref()
            .and_then(|key| self.member_setter_binding_entry(key));
        if resolved.is_some() {
            return resolved;
        }
        if let Expression::Object(entries) = object {
            return self.resolve_object_literal_member_binding(entries, property, 2);
        }
        None
    }
}
