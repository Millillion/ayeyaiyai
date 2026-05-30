use super::*;

impl<'a> FunctionCompiler<'a> {
    fn module_namespace_to_string_tag_for_target(
        &self,
        target: &Expression,
        resolved_target: Option<&Expression>,
        materialized_target: &Expression,
        target_is_module_namespace: bool,
    ) -> Option<&'static str> {
        let target_candidates = [
            Some(target),
            resolved_target,
            (!static_expression_matches(materialized_target, target))
                .then_some(materialized_target),
        ];
        for target_candidate in target_candidates.into_iter().flatten() {
            if let Expression::Identifier(name) = target_candidate
                && Self::module_index_from_namespace_like_identifier(name).is_some()
            {
                return Some(
                    if name.starts_with("__ayy_module_deferred_namespace_")
                        || name.contains("__ayy_module_deferred_namespace_")
                    {
                        "Deferred Module"
                    } else {
                        "Module"
                    },
                );
            }
        }
        target_is_module_namespace.then_some("Module")
    }

    fn class_member_binding_expression(binding: &LocalFunctionBinding) -> Option<Expression> {
        match binding {
            LocalFunctionBinding::User(function_name)
                if function_name.starts_with("__ayy_class_method_") =>
            {
                Some(Expression::Identifier(function_name.clone()))
            }
            _ => None,
        }
    }

    fn lowered_class_member_descriptor_binding(
        &self,
        target: &Expression,
        property: &Expression,
    ) -> Option<PropertyDescriptorBinding> {
        if let Some(value) = self
            .resolve_member_function_binding(target, property)
            .as_ref()
            .and_then(Self::class_member_binding_expression)
        {
            return Some(PropertyDescriptorBinding {
                value: Some(value),
                configurable: true,
                enumerable: false,
                writable: Some(true),
                getter: None,
                setter: None,
                has_get: false,
                has_set: false,
            });
        }

        let getter = self
            .resolve_member_getter_binding(target, property)
            .as_ref()
            .and_then(Self::class_member_binding_expression);
        let setter = self
            .resolve_member_setter_binding(target, property)
            .as_ref()
            .and_then(Self::class_member_binding_expression);
        let has_get = getter.is_some();
        let has_set = setter.is_some();
        if !has_get && !has_set {
            return None;
        }

        Some(PropertyDescriptorBinding {
            value: None,
            configurable: true,
            enumerable: false,
            writable: None,
            getter,
            setter,
            has_get,
            has_set,
        })
    }

    pub(in crate::backend::direct_wasm) fn resolve_inherited_object_property_descriptor_binding(
        &self,
        object: &Expression,
        property: &Expression,
    ) -> Option<PropertyDescriptorBinding> {
        let canonical_property = self.canonical_object_property_expression(property);
        let requested_symbol = self
            .resolve_symbol_identity_expression(&canonical_property)
            .or_else(|| self.resolve_symbol_identity_expression(property));
        let mut prototype = self.resolve_static_object_prototype_expression(object)?;
        for _ in 0..32 {
            let materialized_prototype = self.materialize_static_expression(&prototype);
            if matches!(materialized_prototype, Expression::Null) {
                return None;
            }

            for candidate in [&prototype, &materialized_prototype] {
                let Some(object_binding) = self.resolve_object_binding_from_expression(candidate)
                else {
                    continue;
                };
                if object_binding.runtime_symbol_properties && requested_symbol.is_some() {
                    continue;
                }
                if let Some(descriptor) =
                    object_binding_lookup_descriptor(&object_binding, &canonical_property)
                        .or_else(|| object_binding_lookup_descriptor(&object_binding, property))
                {
                    return Some(descriptor.clone());
                }
                if let Some(value) = self
                    .resolve_object_binding_property_value(&object_binding, &canonical_property)
                    .or_else(|| {
                        self.resolve_object_binding_property_value(&object_binding, property)
                    })
                {
                    return Some(PropertyDescriptorBinding {
                        value: Some(value),
                        configurable: true,
                        enumerable: true,
                        writable: Some(true),
                        getter: None,
                        setter: None,
                        has_get: false,
                        has_set: false,
                    });
                }
            }

            let next_prototype = self
                .resolve_static_object_prototype_expression(&materialized_prototype)
                .or_else(|| self.resolve_static_object_prototype_expression(&prototype))?;
            if static_expression_matches(&next_prototype, &prototype)
                || static_expression_matches(&next_prototype, &materialized_prototype)
            {
                return None;
            }
            prototype = next_prototype;
        }
        None
    }

    fn runtime_shadow_descriptor_value(
        &self,
        target: &Expression,
        resolved_target: Option<&Expression>,
        materialized_target: &Expression,
        property: &Expression,
    ) -> Option<Expression> {
        let candidates = [
            Some(target),
            resolved_target,
            (!static_expression_matches(materialized_target, target))
                .then_some(materialized_target),
        ];

        let candidate = candidates.into_iter().flatten().find(|candidate| {
            self.runtime_object_property_shadow_binding_name_for_expression(candidate, property)
                .is_some_and(|shadow_binding_name| {
                    self.runtime_object_property_shadow_binding_should_defer_static_resolution(
                        &shadow_binding_name,
                    )
                })
        })?;

        Some(Expression::Member {
            object: Box::new(candidate.clone()),
            property: Box::new(property.clone()),
        })
    }

    pub(in crate::backend::direct_wasm) fn expression_is_static_regexp_instance(
        &self,
        target: &Expression,
    ) -> bool {
        let resolved = self
            .resolve_bound_alias_expression(target)
            .filter(|resolved| !static_expression_matches(resolved, target))
            .unwrap_or_else(|| self.materialize_static_expression(target));
        matches!(
            resolved,
            Expression::New { ref callee, .. } | Expression::Call { ref callee, .. }
                if matches!(callee.as_ref(), Expression::Identifier(name) if name == "RegExp" && self.is_unshadowed_builtin_identifier(name))
        )
    }

    pub(in crate::backend::direct_wasm) fn resolve_object_property_descriptor_binding(
        &self,
        target: &Expression,
        resolved_target: Option<&Expression>,
        materialized_target: &Expression,
        property: &Expression,
        string_property_name: Option<&str>,
    ) -> Option<PropertyDescriptorBinding> {
        let object_binding = self
            .resolve_object_binding_from_expression(target)
            .or_else(|| match target {
                Expression::Identifier(name) => self
                    .resolve_identifier_object_binding_fallback(name)
                    .or_else(|| self.resolve_runtime_shadow_object_binding(name)),
                _ => None,
            })
            .or_else(|| {
                resolved_target.and_then(|resolved| {
                    self.resolve_object_binding_from_expression(resolved)
                        .or_else(|| match resolved {
                            Expression::Identifier(name) => self
                                .resolve_identifier_object_binding_fallback(name)
                                .or_else(|| self.resolve_runtime_shadow_object_binding(name)),
                            _ => None,
                        })
                })
            })
            .or_else(|| {
                (!static_expression_matches(materialized_target, target)).then(|| {
                    self.resolve_object_binding_from_expression(materialized_target)
                        .or_else(|| match materialized_target {
                            Expression::Identifier(name) => self
                                .resolve_identifier_object_binding_fallback(name)
                                .or_else(|| self.resolve_runtime_shadow_object_binding(name)),
                            _ => None,
                        })
                })?
            });
        let canonical_property = self.canonical_object_property_expression(property);
        let requested_symbol = self
            .resolve_symbol_identity_expression(&canonical_property)
            .or_else(|| self.resolve_symbol_identity_expression(property));
        if object_binding
            .as_ref()
            .is_some_and(|binding| binding.runtime_symbol_properties && requested_symbol.is_some())
        {
            return None;
        }
        if string_property_name == Some("lastIndex")
            && (self.expression_is_static_regexp_instance(target)
                || resolved_target
                    .is_some_and(|resolved| self.expression_is_static_regexp_instance(resolved))
                || (!static_expression_matches(materialized_target, target)
                    && self.expression_is_static_regexp_instance(materialized_target)))
        {
            return Some(PropertyDescriptorBinding {
                value: Some(Expression::Number(0.0)),
                configurable: false,
                enumerable: false,
                writable: Some(true),
                getter: None,
                setter: None,
                has_get: false,
                has_set: false,
            });
        }
        if string_property_name == Some("length") {
            let target_candidates = [
                Some(target),
                resolved_target,
                (!static_expression_matches(materialized_target, target))
                    .then_some(materialized_target),
            ];
            for target_candidate in target_candidates.into_iter().flatten() {
                if let Some(Expression::String(text)) =
                    self.resolve_static_boxed_primitive_value(target_candidate)
                {
                    return Some(PropertyDescriptorBinding {
                        value: Some(Expression::Number(text.chars().count() as f64)),
                        configurable: false,
                        enumerable: false,
                        writable: Some(false),
                        getter: None,
                        setter: None,
                        has_get: false,
                        has_set: false,
                    });
                }
            }
        }
        if self.state.speculation.execution_context.top_level_function
            && let Some(property_name) = string_property_name
        {
            let target_candidates = [
                Some(target),
                resolved_target,
                (!static_expression_matches(materialized_target, target))
                    .then_some(materialized_target),
            ];
            if target_candidates
                .into_iter()
                .flatten()
                .any(|candidate| matches!(candidate, Expression::This))
            {
                return self.resolve_top_level_global_property_descriptor_binding(property_name);
            }
        }
        if self.expression_aliases_captured_top_level_this(target)
            && let Some(property_name) = string_property_name
        {
            return self.resolve_top_level_global_property_descriptor_binding(property_name);
        }
        if let Some(property_name) = string_property_name {
            let target_candidates = [
                Some(target),
                resolved_target,
                (!static_expression_matches(materialized_target, target))
                    .then_some(materialized_target),
            ];
            for target_candidate in target_candidates.into_iter().flatten() {
                if let Expression::Identifier(object_name) = target_candidate
                    && self.is_unshadowed_builtin_identifier(object_name)
                    && let Some(value) = builtin_member_number_value(object_name, property_name)
                {
                    return Some(PropertyDescriptorBinding {
                        value: Some(Expression::Number(value)),
                        configurable: false,
                        enumerable: false,
                        writable: Some(false),
                        getter: None,
                        setter: None,
                        has_get: false,
                        has_set: false,
                    });
                }
            }
        }
        let target_is_module_namespace = object_binding
            .as_ref()
            .is_some_and(Self::object_binding_has_module_namespace_marker)
            || matches!(
                target,
                Expression::Identifier(name)
                    if FunctionCompiler::module_index_from_namespace_like_identifier(name).is_some()
            );
        let property_is_symbol_to_string_tag = self
            .well_known_symbol_name(&canonical_property)
            .or_else(|| self.well_known_symbol_name(property))
            .as_deref()
            == Some("Symbol.toStringTag");
        if target_is_module_namespace
            && property_is_symbol_to_string_tag
            && let Some(tag) = self.module_namespace_to_string_tag_for_target(
                target,
                resolved_target,
                materialized_target,
                target_is_module_namespace,
            )
        {
            return Some(PropertyDescriptorBinding {
                value: Some(Expression::String(tag.to_string())),
                configurable: false,
                enumerable: false,
                writable: Some(false),
                getter: None,
                setter: None,
                has_get: false,
                has_set: false,
            });
        }
        if string_property_name.is_some()
            && target_is_module_namespace
            && let Some(value) = if let Expression::Identifier(name) = target {
                FunctionCompiler::module_index_from_namespace_like_identifier(name)
                    .and_then(|module_index| {
                        self.resolve_static_dynamic_import_namespace_live_binding_member_initializer_value(
                            module_index,
                            &canonical_property,
                        )
                    })
                    .or_else(|| {
                        self.resolve_module_namespace_live_binding_member_value(
                            target,
                            &canonical_property,
                        )
                    })
            } else {
                self.resolve_module_namespace_live_binding_member_value(target, &canonical_property)
            }
        {
            return Some(PropertyDescriptorBinding {
                value: Some(value),
                configurable: false,
                enumerable: true,
                writable: Some(true),
                getter: None,
                setter: None,
                has_get: false,
                has_set: false,
            });
        }
        if let Some(descriptor) = object_binding
            .as_ref()
            .and_then(|binding| object_binding_lookup_descriptor(binding, property))
        {
            return Some(descriptor.clone());
        }
        if let Some(descriptor) = self.lowered_class_member_descriptor_binding(target, property) {
            return Some(descriptor);
        }
        let property_present_in_binding = object_binding.as_ref().is_some_and(|binding| {
            self.resolve_object_binding_property_value(binding, property)
                .is_some()
        });
        let value = object_binding
            .as_ref()
            .and_then(|binding| self.resolve_object_binding_property_value(binding, property));
        let target_candidates = [
            Some(target),
            resolved_target,
            (!static_expression_matches(materialized_target, target))
                .then_some(materialized_target),
        ];
        let target_is_function_like = target_candidates
            .into_iter()
            .flatten()
            .any(|candidate| self.infer_value_kind(candidate) == Some(StaticValueKind::Function));
        if target_is_function_like
            && matches!(string_property_name, Some("name"))
            && matches!(value, Some(Expression::String(_)))
            && let Some(value) = value.clone()
        {
            return Some(PropertyDescriptorBinding {
                value: Some(value),
                configurable: true,
                enumerable: false,
                writable: Some(false),
                getter: None,
                setter: None,
                has_get: false,
                has_set: false,
            });
        }
        if target_is_function_like
            && matches!(string_property_name, Some("length"))
            && matches!(value, Some(Expression::Number(_)))
            && let Some(value) = value.clone()
        {
            return Some(PropertyDescriptorBinding {
                value: Some(value),
                configurable: true,
                enumerable: false,
                writable: Some(false),
                getter: None,
                setter: None,
                has_get: false,
                has_set: false,
            });
        }
        if matches!(string_property_name, Some("length") | Some("name"))
            && let Some(binding) = object_binding.as_ref()
            && object_binding_lookup_value(
                binding,
                &function_constructor_source_property_expression(),
            )
            .is_some()
            && let Some(value) = value.clone()
        {
            return Some(PropertyDescriptorBinding {
                value: Some(value),
                configurable: true,
                enumerable: false,
                writable: Some(false),
                getter: None,
                setter: None,
                has_get: false,
                has_set: false,
            });
        }
        if matches!(string_property_name, Some("prototype"))
            && let Some(binding) = object_binding.as_ref()
            && object_binding_lookup_value(
                binding,
                &function_constructor_source_property_expression(),
            )
            .is_some()
            && let Some(value) = value.clone()
        {
            return Some(PropertyDescriptorBinding {
                value: Some(value),
                configurable: false,
                enumerable: false,
                writable: Some(true),
                getter: None,
                setter: None,
                has_get: false,
                has_set: false,
            });
        }
        let getter = self
            .resolve_member_getter_binding(target, property)
            .or_else(|| {
                resolved_target
                    .and_then(|resolved| self.resolve_member_getter_binding(resolved, property))
            })
            .or_else(|| {
                (!static_expression_matches(materialized_target, target))
                    .then(|| self.resolve_member_getter_binding(materialized_target, property))?
            })
            .map(|binding| Self::function_binding_to_expression(&binding));
        let setter = self
            .resolve_member_setter_binding(target, property)
            .or_else(|| {
                resolved_target
                    .and_then(|resolved| self.resolve_member_setter_binding(resolved, property))
            })
            .or_else(|| {
                (!static_expression_matches(materialized_target, target))
                    .then(|| self.resolve_member_setter_binding(materialized_target, property))?
            })
            .map(|binding| Self::function_binding_to_expression(&binding));
        let enumerable = object_binding.as_ref().is_some_and(|binding| {
            property_present_in_binding
                && string_property_name.is_none_or(|property_name| {
                    !binding
                        .non_enumerable_string_properties
                        .iter()
                        .any(|name| name == property_name)
                })
        });
        if value.is_some() || getter.is_some() || setter.is_some() {
            let runtime_value = if getter.is_none() && setter.is_none() {
                self.runtime_shadow_descriptor_value(
                    target,
                    resolved_target,
                    materialized_target,
                    property,
                )
            } else {
                None
            };
            return Some(PropertyDescriptorBinding {
                value: if getter.is_some() || setter.is_some() {
                    None
                } else {
                    runtime_value.or(value)
                },
                configurable: true,
                enumerable,
                writable: if getter.is_some() || setter.is_some() {
                    None
                } else {
                    Some(true)
                },
                getter: getter.clone(),
                setter: setter.clone(),
                has_get: getter.is_some(),
                has_set: setter.is_some(),
            });
        }
        None
    }
}
