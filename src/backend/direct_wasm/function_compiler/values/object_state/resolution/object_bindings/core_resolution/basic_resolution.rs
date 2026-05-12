use super::*;

impl<'a> FunctionCompiler<'a> {
    fn object_binding_has_private_capture_marker(object_binding: &ObjectValueBinding) -> bool {
        object_binding
            .string_properties
            .iter()
            .any(|(property_name, value)| {
                (property_name.starts_with("__ayy$private$")
                    || is_private_brand_marker_property_name(property_name))
                    && matches!(
                        value,
                        Expression::Identifier(identifier)
                            if identifier.starts_with("__ayy_closure_slot_")
                    )
            })
    }

    fn object_binding_descriptor_position(
        &self,
        object_binding: &ObjectValueBinding,
        property: &Expression,
    ) -> Option<usize> {
        let canonical_property = self.canonical_object_property_expression(property);
        let requested_symbol = self
            .resolve_symbol_identity_expression(&canonical_property)
            .or_else(|| self.resolve_symbol_identity_expression(property));

        object_binding
            .property_descriptors
            .iter()
            .position(|(existing_property, _)| {
                if static_expression_matches(existing_property, property) {
                    return true;
                }

                let canonical_existing =
                    self.canonical_object_property_expression(existing_property);
                if static_expression_matches(&canonical_existing, &canonical_property) {
                    return true;
                }

                let Some(requested_symbol) = requested_symbol.as_ref() else {
                    return false;
                };
                self.resolve_symbol_identity_expression(&canonical_existing)
                    .or_else(|| self.resolve_symbol_identity_expression(existing_property))
                    .is_some_and(|existing_symbol| {
                        static_expression_matches(&existing_symbol, requested_symbol)
                    })
            })
    }

    fn descriptor_has_accessor(descriptor: &PropertyDescriptorBinding) -> bool {
        descriptor.has_get
            || descriptor.has_set
            || descriptor.getter.is_some()
            || descriptor.setter.is_some()
    }

    fn merge_runtime_shadow_static_symbol_metadata(
        &self,
        target: &mut ObjectValueBinding,
        source: Option<&ObjectValueBinding>,
    ) {
        let Some(source) = source else {
            return;
        };

        for (property, value) in &source.symbol_properties {
            if self
                .resolve_static_symbol_property_shadow_entry(target, property)
                .is_none()
            {
                target
                    .symbol_properties
                    .push((property.clone(), value.clone()));
            }
        }

        for (property, descriptor) in &source.property_descriptors {
            if let Some(position) = self.object_binding_descriptor_position(target, property) {
                if Self::descriptor_has_accessor(descriptor)
                    && !Self::descriptor_has_accessor(&target.property_descriptors[position].1)
                {
                    target.property_descriptors[position].1 = descriptor.clone();
                }
                continue;
            }
            target
                .property_descriptors
                .push((property.clone(), descriptor.clone()));
        }
        for property_name in &source.non_enumerable_string_properties {
            let property = Expression::String(property_name.clone());
            if !target
                .string_properties
                .iter()
                .any(|(target_name, _)| target_name == property_name)
                || target
                    .non_enumerable_string_properties
                    .iter()
                    .any(|target_name| target_name == property_name)
                || object_binding_lookup_descriptor(target, &property)
                    .is_some_and(|descriptor| descriptor.enumerable)
            {
                continue;
            }
            target
                .non_enumerable_string_properties
                .push(property_name.clone());
        }
        target.runtime_symbol_properties |= source.runtime_symbol_properties;
    }

    fn runtime_shadow_with_static_symbol_metadata(
        &self,
        mut runtime_shadow: ObjectValueBinding,
        sources: &[Option<&ObjectValueBinding>],
    ) -> ObjectValueBinding {
        for source in sources {
            self.merge_runtime_shadow_static_symbol_metadata(&mut runtime_shadow, *source);
        }
        runtime_shadow
    }

    pub(in crate::backend::direct_wasm) fn resolve_home_object_this_binding(
        &self,
    ) -> Option<ObjectValueBinding> {
        let current_function_name = self.current_function_name()?;
        let home_object_name = self.resolve_home_object_name_for_function(current_function_name)?;
        if let Some(class_name) = home_object_name.strip_suffix(".prototype") {
            return self.resolve_user_constructor_object_binding_from_new(
                &Expression::Identifier(class_name.to_string()),
                &[],
            );
        }
        self.resolve_object_binding_from_expression(&Expression::Identifier(home_object_name))
    }

    pub(super) fn resolve_basic_object_binding(
        &self,
        expression: &Expression,
    ) -> Option<ObjectValueBinding> {
        match expression {
            Expression::Identifier(name) => {
                self.resolve_identifier_object_binding(name, expression)
            }
            Expression::This => self.resolve_this_object_binding(),
            _ => None,
        }
    }

    fn resolve_identifier_object_binding(
        &self,
        name: &str,
        expression: &Expression,
    ) -> Option<ObjectValueBinding> {
        let resolved_name = self
            .resolve_current_local_binding(name)
            .map(|(resolved_name, _)| resolved_name)
            .unwrap_or_else(|| name.to_string());
        if name == "$262" {
            let mut host_object_binding = empty_object_value_binding();
            object_binding_set_property(
                &mut host_object_binding,
                Expression::String("createRealm".to_string()),
                Expression::Identifier(TEST262_CREATE_REALM_BUILTIN.to_string()),
            );
            return Some(host_object_binding);
        }
        if let Some(realm_id) = parse_test262_realm_identifier(name) {
            return self.backend.test262_realm_object_binding(realm_id);
        }
        if let Some(realm_id) = parse_test262_realm_global_identifier(name) {
            return self.test262_realm_global_object_binding(realm_id);
        }
        let local_object_binding = self
            .state
            .speculation
            .static_semantics
            .local_object_binding(&resolved_name)
            .cloned();
        let local_value_object_binding = self
            .state
            .speculation
            .static_semantics
            .local_value_binding(&resolved_name)
            .filter(|value| !static_expression_matches(value, expression))
            .and_then(|value| self.resolve_object_binding_from_expression(value));
        let hidden_object_binding = self
            .resolve_user_function_capture_hidden_name(name)
            .and_then(|hidden_name| self.global_object_binding(&hidden_name).cloned());
        let runtime_shadow_object_binding = self
            .runtime_object_property_shadow_owner_name_for_identifier(name)
            .and_then(|owner_name| self.resolve_runtime_shadow_object_binding(&owner_name));
        let global_object_binding = self.global_object_binding(name).cloned();
        let global_value_object_binding = self
            .global_value_binding(name)
            .filter(|value| !static_expression_matches(value, expression))
            .and_then(|value| self.resolve_object_binding_from_expression(value));
        let function_object_binding = self
            .identifier_resolves_to_function_object(&resolved_name, name)
            .then(empty_object_value_binding);
        let has_active_local_binding = local_object_binding.is_some()
            || self.resolve_current_local_binding(name).is_some()
            || self
                .state
                .speculation
                .static_semantics
                .local_value_binding(&resolved_name)
                .is_some()
            || self
                .state
                .speculation
                .static_semantics
                .local_function_binding(&resolved_name)
                .is_some()
            || self
                .state
                .speculation
                .static_semantics
                .local_proxy_binding(&resolved_name)
                .is_some();
        if local_object_binding
            .as_ref()
            .is_some_and(Self::object_binding_has_private_capture_marker)
        {
            return local_object_binding;
        }
        let runtime_shadow_object_binding = runtime_shadow_object_binding.map(|binding| {
            self.runtime_shadow_with_static_symbol_metadata(
                binding,
                &[
                    local_object_binding.as_ref(),
                    local_value_object_binding.as_ref(),
                    hidden_object_binding.as_ref(),
                    global_object_binding.as_ref(),
                    global_value_object_binding.as_ref(),
                ],
            )
        });
        if local_object_binding
            .as_ref()
            .is_some_and(|binding| self.object_binding_is_static_map(binding))
        {
            return local_object_binding;
        }
        if global_object_binding
            .as_ref()
            .is_some_and(|binding| self.object_binding_is_static_map(binding))
        {
            return global_object_binding;
        }
        let candidate_binding = if has_active_local_binding {
            runtime_shadow_object_binding
                .clone()
                .or(local_object_binding.clone())
                .or(local_value_object_binding.clone())
                .or(hidden_object_binding.clone())
                .or(global_object_binding.clone())
                .or(global_value_object_binding.clone())
                .or(function_object_binding.clone())
        } else if self.binding_name_is_global(name)
            || self.global_has_binding(name)
            || self.backend.global_has_lexical_binding(name)
            || self.global_has_implicit_binding(name)
        {
            runtime_shadow_object_binding
                .clone()
                .or(global_object_binding.clone())
                .or(global_value_object_binding.clone())
                .or(local_object_binding.clone())
                .or(local_value_object_binding.clone())
                .or(hidden_object_binding.clone())
                .or(function_object_binding.clone())
        } else {
            runtime_shadow_object_binding
                .clone()
                .or(local_object_binding.clone())
                .or(local_value_object_binding.clone())
                .or(hidden_object_binding.clone())
                .or(global_object_binding.clone())
                .or(global_value_object_binding.clone())
                .or(function_object_binding.clone())
        };
        let binding = candidate_binding
            .or_else(|| {
                let proxy = self
                    .state
                    .speculation
                    .static_semantics
                    .local_proxy_binding(&resolved_name)
                    .cloned()
                    .or_else(|| self.global_proxy_binding(name).cloned())?;
                self.resolve_object_binding_from_expression(&proxy.target)
            })
            .or_else(|| {
                let resolved = self.resolve_bound_alias_expression(expression)?;
                (!static_expression_matches(&resolved, expression))
                    .then(|| self.resolve_object_binding_from_expression(&resolved))
                    .flatten()
            });
        binding
    }

    fn identifier_resolves_to_function_object(&self, resolved_name: &str, name: &str) -> bool {
        self.state
            .speculation
            .static_semantics
            .local_function_binding(resolved_name)
            .is_some()
            || self
                .state
                .speculation
                .static_semantics
                .local_function_binding(name)
                .is_some()
            || self
                .backend
                .global_function_binding(resolved_name)
                .is_some()
            || self.backend.global_function_binding(name).is_some()
            || (is_internal_user_function_identifier(resolved_name)
                && self.contains_user_function(resolved_name))
            || (is_internal_user_function_identifier(name) && self.contains_user_function(name))
            || builtin_function_runtime_value(name).is_some()
            || builtin_function_runtime_value(resolved_name).is_some()
    }

    fn resolve_this_object_binding(&self) -> Option<ObjectValueBinding> {
        let runtime_dynamic_this = self
            .state
            .runtime
            .locals
            .runtime_dynamic_bindings
            .contains("this");
        let hidden_capture_binding = self
            .resolve_user_function_capture_hidden_name("this")
            .and_then(|hidden_name| self.global_object_binding(&hidden_name).cloned());
        let local_object_binding = self
            .state
            .speculation
            .static_semantics
            .local_object_binding("this")
            .cloned();
        let local_value_binding = self
            .state
            .speculation
            .static_semantics
            .local_value_binding("this")
            .cloned()
            .and_then(|value| {
                (!matches!(value, Expression::Undefined))
                    .then(|| self.resolve_object_binding_from_expression(&value))
                    .flatten()
            });
        let binding = if runtime_dynamic_this {
            hidden_capture_binding
                .clone()
                .or(local_object_binding.clone())
                .or(local_value_binding.clone())
        } else {
            let home_object_fallback = self
                .current_user_function()
                .filter(|user_function| user_function.lexical_this)
                .and_then(|_| self.resolve_home_object_this_binding());
            local_object_binding
                .clone()
                .or(local_value_binding.clone())
                .or(hidden_capture_binding.clone())
                .or(home_object_fallback)
        }
        .map(|binding| self.rewrite_static_new_this_object_binding(&binding));
        if std::env::var_os("AYY_TRACE_THIS_FLOW").is_some() {
            eprintln!(
                "this_flow resolve_this_object_binding fn={:?} runtime_dynamic_this={} resolved={}",
                self.current_function_name(),
                runtime_dynamic_this,
                binding.is_some()
            );
        }
        binding
    }
}
