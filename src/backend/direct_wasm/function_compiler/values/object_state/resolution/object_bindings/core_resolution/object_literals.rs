use super::*;

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

        let mut object_binding = empty_object_value_binding();
        for entry in entries {
            match entry {
                ObjectEntry::Data { key, value } => {
                    let descriptor_value =
                        if self.object_literal_value_reads_runtime_nonlocal_binding(value) {
                            value.clone()
                        } else {
                            self.materialize_static_expression(value)
                        };
                    object_binding_define_property_descriptor(
                        &mut object_binding,
                        self.materialize_static_expression(key),
                        PropertyDescriptorBinding {
                            value: Some(descriptor_value),
                            configurable: true,
                            enumerable: true,
                            writable: Some(true),
                            getter: None,
                            setter: None,
                            has_get: false,
                            has_set: false,
                        },
                    );
                }
                ObjectEntry::Getter { key, getter } => {
                    let key = self.materialize_static_expression(key);
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
                    let key = self.materialize_static_expression(key);
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
