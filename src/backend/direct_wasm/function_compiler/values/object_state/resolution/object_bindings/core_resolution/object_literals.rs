use super::*;

fn static_property_key_is_symbol_to_string_tag(key: &Expression) -> bool {
    matches!(
        key,
        Expression::Member { object, property }
            if matches!(object.as_ref(), Expression::Identifier(name) if name == "Symbol")
                && matches!(property.as_ref(), Expression::String(name) if name == "toStringTag")
    )
}

impl<'a> FunctionCompiler<'a> {
    fn object_literal_value_reads_runtime_nonlocal_binding(&self, expression: &Expression) -> bool {
        if self.current_function_name().is_none() {
            return false;
        }

        let mut referenced_names = HashSet::new();
        collect_referenced_binding_names_from_expression(expression, &mut referenced_names);
        referenced_names.iter().any(|name| {
            let source_name = scoped_binding_source_name(name).unwrap_or(name);
            self.resolve_current_local_binding(source_name).is_none()
                && (self.global_has_binding(source_name)
                    || self.global_has_implicit_binding(source_name)
                    || self
                        .resolve_user_function_capture_hidden_name(source_name)
                        .is_some())
        })
    }

    fn object_literal_value_should_preserve_reference(&self, expression: &Expression) -> bool {
        matches!(
            expression,
            Expression::Identifier(_) | Expression::This | Expression::Member { .. }
        ) && matches!(
            self.infer_value_kind(expression),
            Some(StaticValueKind::Object | StaticValueKind::Function)
        )
    }

    fn materialized_object_literal_property_key(&self, key: &Expression) -> Expression {
        self.resolve_property_key_expression(key)
            .unwrap_or_else(|| {
                let materialized = self.materialize_static_expression(key);
                static_property_name_from_expression(&materialized)
                    .map(Expression::String)
                    .unwrap_or(materialized)
            })
    }

    pub(in crate::backend::direct_wasm) fn canonicalize_contextual_object_binding_property_keys(
        &self,
        object_binding: ObjectValueBinding,
    ) -> ObjectValueBinding {
        let mut canonical_binding = empty_object_value_binding();
        canonical_binding.runtime_symbol_properties = object_binding.runtime_symbol_properties;
        canonical_binding.extensible = object_binding.extensible;

        for (property, descriptor) in object_binding.property_descriptors {
            let property = self.materialized_object_literal_property_key(&property);
            object_binding_define_property_descriptor(&mut canonical_binding, property, descriptor);
        }

        for (property_name, value) in object_binding.string_properties {
            let property = Expression::String(property_name);
            if object_binding_lookup_value(&canonical_binding, &property).is_none() {
                object_binding_define_property(&mut canonical_binding, property, value, true);
            }
        }

        for (property, value) in object_binding.symbol_properties {
            let property = self.materialized_object_literal_property_key(&property);
            if object_binding_lookup_value(&canonical_binding, &property).is_none() {
                object_binding_define_property(&mut canonical_binding, property, value, true);
            }
        }

        for property_name in object_binding.non_enumerable_string_properties {
            object_binding_set_string_property_enumerable(
                &mut canonical_binding,
                &property_name,
                false,
            );
        }

        canonical_binding
    }

    fn resolve_materialized_object_literal_binding(
        &self,
        entries: &[ObjectEntry],
    ) -> Option<ObjectValueBinding> {
        if entries
            .iter()
            .any(|entry| matches!(entry, ObjectEntry::Spread(_)))
        {
            return None;
        }

        let module_namespace_literal = entries.iter().any(|entry| {
            matches!(
                entry,
                ObjectEntry::Data {
                    key: Expression::String(name),
                    value: Expression::Bool(true),
                } if name == "__ayy$module$namespace"
            )
        });
        let mut object_binding = empty_object_value_binding();
        if module_namespace_literal {
            object_binding.extensible = false;
        }
        for entry in entries {
            match entry {
                ObjectEntry::Data { key, value } => {
                    if object_entry_is_literal_proto_setter(entry) {
                        continue;
                    }
                    let key = self.materialized_object_literal_property_key(key);
                    let descriptor_value = if self
                        .object_literal_value_reads_runtime_nonlocal_binding(value)
                        || self.object_literal_value_should_preserve_reference(value)
                    {
                        value.clone()
                    } else {
                        self.materialize_static_expression(value)
                    };
                    let (configurable, enumerable, writable) = if module_namespace_literal {
                        if static_property_key_is_symbol_to_string_tag(&key) {
                            (false, false, Some(false))
                        } else if matches!(&key, Expression::String(name) if name == "__ayy$module$namespace")
                        {
                            (false, false, Some(false))
                        } else {
                            (false, true, Some(true))
                        }
                    } else {
                        (true, true, Some(true))
                    };
                    object_binding_define_property_descriptor(
                        &mut object_binding,
                        key,
                        PropertyDescriptorBinding {
                            value: Some(descriptor_value),
                            configurable,
                            enumerable,
                            writable,
                            getter: None,
                            setter: None,
                            has_get: false,
                            has_set: false,
                        },
                    );
                }
                ObjectEntry::Getter { key, getter } => {
                    let key = self.materialized_object_literal_property_key(key);
                    let existing = object_binding_lookup_descriptor(&object_binding, &key).cloned();
                    object_binding_define_property_descriptor(
                        &mut object_binding,
                        key,
                        PropertyDescriptorBinding {
                            value: None,
                            configurable: true,
                            enumerable: true,
                            writable: None,
                            getter: Some(self.materialize_static_expression(getter)),
                            setter: existing
                                .as_ref()
                                .and_then(|descriptor| descriptor.setter.clone()),
                            has_get: true,
                            has_set: existing
                                .as_ref()
                                .is_some_and(|descriptor| descriptor.has_set),
                        },
                    );
                }
                ObjectEntry::Setter { key, setter } => {
                    let key = self.materialized_object_literal_property_key(key);
                    let existing = object_binding_lookup_descriptor(&object_binding, &key).cloned();
                    object_binding_define_property_descriptor(
                        &mut object_binding,
                        key,
                        PropertyDescriptorBinding {
                            value: None,
                            configurable: true,
                            enumerable: true,
                            writable: None,
                            getter: existing
                                .as_ref()
                                .and_then(|descriptor| descriptor.getter.clone()),
                            setter: Some(self.materialize_static_expression(setter)),
                            has_get: existing
                                .as_ref()
                                .is_some_and(|descriptor| descriptor.has_get),
                            has_set: true,
                        },
                    );
                }
                ObjectEntry::Spread(_) => unreachable!(),
            }
        }

        Some(object_binding)
    }

    pub(super) fn resolve_object_literal_expression_binding(
        &self,
        expression: &Expression,
    ) -> Option<ObjectValueBinding> {
        let Expression::Object(entries) = expression else {
            return None;
        };
        let mut environment = self.snapshot_static_resolution_environment();
        self.resolve_object_binding_entries_with_state(entries, &mut environment)
            .or_else(|| self.resolve_materialized_object_literal_binding(entries))
    }
}
