use super::*;

impl<'a> FunctionCompiler<'a> {
    pub(super) fn apply_object_define_property_update(&mut self, arguments: &[CallArgument]) {
        if std::env::var_os("AYY_TRACE_DEFINE_PROPERTY_UPDATE").is_some() {
            eprintln!("define_property_update args={arguments:?}");
        }
        let [
            CallArgument::Expression(target),
            CallArgument::Expression(property),
            CallArgument::Expression(descriptor_expression),
            ..,
        ] = arguments
        else {
            return;
        };
        let Some(descriptor) = resolve_property_descriptor_definition(descriptor_expression) else {
            return;
        };
        if let Some(accepted_without_mutation) = self
            .static_define_property_accepts_without_mutation(
                target,
                property,
                descriptor_expression,
            )
        {
            if accepted_without_mutation
                && matches!(target, Expression::This)
                && self.current_function_name().is_none()
            {
                self.update_global_property_descriptor(property, &descriptor);
            }
            return;
        }

        match target {
            Expression::This => {
                if self.current_function_name().is_some() {
                    self.define_object_property_from_descriptor("this", property, &descriptor);
                } else {
                    self.update_global_property_descriptor(property, &descriptor);
                }
            }
            Expression::Identifier(name) => {
                self.define_object_property_from_descriptor(name, property, &descriptor);
            }
            Expression::Member {
                object,
                property: target_property,
            } if matches!(target_property.as_ref(), Expression::String(name) if name == "prototype") =>
            {
                let Expression::Identifier(name) = object.as_ref() else {
                    return;
                };
                self.define_prototype_object_property_from_descriptor(name, property, &descriptor);
            }
            Expression::Member {
                object,
                property: target_property,
            } => {
                self.define_nested_object_property_from_descriptor(
                    object,
                    target_property,
                    property,
                    &descriptor,
                );
            }
            _ => {}
        }
    }

    pub(in crate::backend::direct_wasm) fn apply_object_define_properties_update(
        &mut self,
        arguments: &[CallArgument],
    ) {
        let [
            CallArgument::Expression(target),
            CallArgument::Expression(properties),
            ..,
        ] = arguments
        else {
            return;
        };
        let Expression::Object(entries) = properties else {
            return;
        };

        for entry in entries {
            let crate::ir::hir::ObjectEntry::Data {
                key,
                value: descriptor_expression,
            } = entry
            else {
                continue;
            };
            let Some(descriptor) = resolve_property_descriptor_definition(descriptor_expression)
            else {
                continue;
            };
            if let Some(accepted_without_mutation) = self
                .static_define_property_accepts_without_mutation(target, key, descriptor_expression)
            {
                if accepted_without_mutation
                    && matches!(target, Expression::This)
                    && self.current_function_name().is_none()
                {
                    self.update_global_property_descriptor(key, &descriptor);
                }
                continue;
            }

            match target {
                Expression::This => {
                    if self.current_function_name().is_some() {
                        self.define_object_property_from_descriptor("this", key, &descriptor);
                    } else {
                        self.update_global_property_descriptor(key, &descriptor);
                    }
                }
                Expression::Identifier(name) => {
                    self.define_object_property_from_descriptor(name, key, &descriptor);
                }
                Expression::Member {
                    object,
                    property: target_property,
                } if matches!(target_property.as_ref(), Expression::String(name) if name == "prototype") =>
                {
                    let Expression::Identifier(name) = object.as_ref() else {
                        continue;
                    };
                    self.define_prototype_object_property_from_descriptor(name, key, &descriptor);
                }
                Expression::Member {
                    object,
                    property: target_property,
                } => {
                    self.define_nested_object_property_from_descriptor(
                        object,
                        target_property,
                        key,
                        &descriptor,
                    );
                }
                _ => {}
            }
        }
    }

    fn object_binding_to_expression_with_descriptor_entries(
        object_binding: &ObjectValueBinding,
    ) -> Expression {
        let mut entries = Vec::new();
        for (name, value) in &object_binding.string_properties {
            entries.push(crate::ir::hir::ObjectEntry::Data {
                key: Expression::String(name.clone()),
                value: value.clone(),
            });
        }
        for (property, value) in &object_binding.symbol_properties {
            entries.push(crate::ir::hir::ObjectEntry::Data {
                key: property.clone(),
                value: value.clone(),
            });
        }
        for (property, descriptor) in &object_binding.property_descriptors {
            if let Some(getter) = descriptor.getter.as_ref() {
                entries.push(crate::ir::hir::ObjectEntry::Getter {
                    key: property.clone(),
                    getter: getter.clone(),
                });
            }
            if let Some(setter) = descriptor.setter.as_ref() {
                entries.push(crate::ir::hir::ObjectEntry::Setter {
                    key: property.clone(),
                    setter: setter.clone(),
                });
            }
            if !descriptor.has_get
                && !descriptor.has_set
                && let Some(value) = descriptor.value.as_ref()
            {
                entries.push(crate::ir::hir::ObjectEntry::Data {
                    key: property.clone(),
                    value: value.clone(),
                });
            }
        }
        Expression::Object(entries)
    }

    fn update_global_property_descriptor(
        &mut self,
        property: &Expression,
        descriptor: &PropertyDescriptorDefinition,
    ) {
        let property = self.canonical_object_property_expression(property);
        let property_name: String = match static_property_name_from_expression(&property) {
            Some(property_name) => property_name,
            None => return,
        };
        let existing = self
            .backend
            .global_property_descriptor(&property_name)
            .cloned();
        let value = if descriptor.is_accessor() {
            Expression::Undefined
        } else {
            descriptor
                .value
                .as_ref()
                .map(|expression| self.materialize_static_expression(expression))
                .or_else(|| existing.as_ref().map(|state| state.value.clone()))
                .unwrap_or(Expression::Undefined)
        };
        let writable = if descriptor.is_accessor() {
            None
        } else {
            Some(
                descriptor
                    .writable
                    .or_else(|| existing.as_ref().and_then(|state| state.writable))
                    .unwrap_or(false),
            )
        };
        let enumerable = descriptor.enumerable.unwrap_or_else(|| {
            existing
                .as_ref()
                .map(|state| state.enumerable)
                .unwrap_or(false)
        });
        let configurable = descriptor.configurable.unwrap_or_else(|| {
            existing
                .as_ref()
                .map(|state| state.configurable)
                .unwrap_or(false)
        });
        self.backend.upsert_global_property_descriptor(
            property_name,
            GlobalPropertyDescriptorState {
                value,
                writable,
                enumerable,
                configurable,
                getter: descriptor.getter.clone(),
                setter: descriptor.setter.clone(),
                has_get: descriptor.getter.is_some(),
                has_set: descriptor.setter.is_some(),
            },
        );
    }

    fn define_nested_object_property_from_descriptor(
        &mut self,
        object: &Expression,
        target_property: &Expression,
        property: &Expression,
        descriptor: &PropertyDescriptorDefinition,
    ) {
        let parent_name = match object {
            Expression::Identifier(name) => Some(name.clone()),
            Expression::This => Some("this".to_string()),
            _ => self
                .resolve_bound_alias_expression(object)
                .and_then(|resolved| {
                    if let Expression::Identifier(name) = resolved {
                        Some(name)
                    } else {
                        None
                    }
                }),
        };
        let Some(parent_name) = parent_name else {
            return;
        };

        let target_property = self.canonical_object_property_expression(target_property);
        let property = self.canonical_object_property_expression(property);
        let parent_binding = self
            .state
            .speculation
            .static_semantics
            .local_object_binding(&parent_name)
            .cloned()
            .or_else(|| self.backend.global_object_binding(&parent_name).cloned());
        let Some(parent_binding) = parent_binding else {
            return;
        };

        let existing_nested_value = self
            .resolve_object_binding_property_value(&parent_binding, &target_property)
            .or_else(|| object_binding_lookup_value(&parent_binding, &target_property).cloned());
        let mut nested_binding = existing_nested_value
            .as_ref()
            .and_then(|value| self.resolve_object_binding_from_expression(value))
            .unwrap_or_else(empty_object_value_binding);
        if !object_binding_can_define_property(&nested_binding, &property) {
            return;
        }

        let property_name = static_property_name_from_expression(&property);
        let existing_value = object_binding_lookup_value(&nested_binding, &property).cloned();
        let existing_descriptor =
            object_binding_lookup_descriptor(&nested_binding, &property).cloned();
        let current_enumerable = property_name.as_ref().is_some_and(|property_name| {
            !nested_binding
                .non_enumerable_string_properties
                .iter()
                .any(|hidden_name| hidden_name == property_name)
        });
        let enumerable = descriptor.enumerable.unwrap_or_else(|| {
            existing_descriptor
                .as_ref()
                .map(|descriptor| descriptor.enumerable)
                .unwrap_or(current_enumerable)
        });
        let configurable = descriptor.configurable.unwrap_or_else(|| {
            existing_descriptor
                .as_ref()
                .map(|descriptor| descriptor.configurable)
                .unwrap_or(false)
        });
        let (value, writable, getter, setter, has_get, has_set) = if descriptor.is_accessor() {
            (
                None,
                None,
                descriptor
                    .getter
                    .as_ref()
                    .map(|expression| {
                        self.materialize_emitted_define_property_value_expression(expression)
                    })
                    .or_else(|| {
                        existing_descriptor
                            .as_ref()
                            .and_then(|descriptor| descriptor.getter.clone())
                    }),
                descriptor
                    .setter
                    .as_ref()
                    .map(|expression| {
                        self.materialize_emitted_define_property_value_expression(expression)
                    })
                    .or_else(|| {
                        existing_descriptor
                            .as_ref()
                            .and_then(|descriptor| descriptor.setter.clone())
                    }),
                descriptor.getter.is_some()
                    || existing_descriptor
                        .as_ref()
                        .is_some_and(|descriptor| descriptor.has_get),
                descriptor.setter.is_some()
                    || existing_descriptor
                        .as_ref()
                        .is_some_and(|descriptor| descriptor.has_set),
            )
        } else {
            let value = descriptor
                .value
                .as_ref()
                .map(|expression| {
                    self.materialize_emitted_define_property_value_expression(expression)
                })
                .or_else(|| {
                    existing_value
                        .as_ref()
                        .map(|expression| self.materialize_static_expression(expression))
                })
                .or_else(|| {
                    existing_descriptor
                        .as_ref()
                        .and_then(|descriptor| descriptor.value.clone())
                })
                .unwrap_or(Expression::Undefined);
            let writable = descriptor.writable.or_else(|| {
                existing_descriptor
                    .as_ref()
                    .and_then(|descriptor| descriptor.writable)
            });
            (
                Some(value),
                Some(writable.unwrap_or(false)),
                None,
                None,
                false,
                false,
            )
        };
        object_binding_define_property_descriptor(
            &mut nested_binding,
            property,
            PropertyDescriptorBinding {
                value,
                configurable,
                enumerable,
                writable,
                getter,
                setter,
                has_get,
                has_set,
            },
        );

        let nested_expression =
            Self::object_binding_to_expression_with_descriptor_entries(&nested_binding);
        if let Some(parent_object_binding) = self
            .state
            .speculation
            .static_semantics
            .local_object_binding_mut(&parent_name)
        {
            object_binding_set_property(
                parent_object_binding,
                target_property.clone(),
                nested_expression.clone(),
            );
        }
        if self.binding_name_is_global(&parent_name) {
            let mut parent_binding = self
                .backend
                .global_object_binding(&parent_name)
                .cloned()
                .unwrap_or(parent_binding);
            object_binding_set_property(
                &mut parent_binding,
                target_property,
                nested_expression.clone(),
            );
            self.backend
                .sync_global_object_binding(&parent_name, Some(parent_binding));
        }

        let shadow_owner = self
            .runtime_object_property_shadow_owner_name_for_identifier(&parent_name)
            .unwrap_or_else(|| parent_name.clone());
        let updated_binding = self
            .state
            .speculation
            .static_semantics
            .local_object_binding(&parent_name)
            .cloned()
            .or_else(|| self.backend.global_object_binding(&parent_name).cloned());
        if let Some(object_binding) = updated_binding {
            self.sync_runtime_object_property_shadow_static_metadata_from_binding(
                &shadow_owner,
                &object_binding,
            );
        }
    }

    fn define_object_property_from_descriptor(
        &mut self,
        name: &str,
        property: &Expression,
        descriptor: &PropertyDescriptorDefinition,
    ) {
        if name == "this" {
            self.seed_local_this_object_binding();
        }
        let property = self.canonical_object_property_expression(property);
        if std::env::var_os("AYY_TRACE_DEFINE_PROPERTY_UPDATE").is_some() {
            eprintln!(
                "define_property_update target={name} canonical_property={property:?} global={} local_object={}",
                self.binding_name_is_global(name),
                self.state
                    .speculation
                    .static_semantics
                    .local_object_binding(name)
                    .is_some()
            );
        }
        let property_name = static_property_name_from_expression(&property);
        let existing_binding = self
            .state
            .speculation
            .static_semantics
            .local_object_binding(name)
            .or_else(|| self.backend.global_object_binding(name));
        if existing_binding.is_some_and(|object_binding| {
            !object_binding_can_define_property(object_binding, &property)
        }) {
            return;
        }
        let existing_binding = self
            .state
            .speculation
            .static_semantics
            .local_object_binding(name)
            .or_else(|| self.backend.global_object_binding(name));
        let existing_value = existing_binding
            .and_then(|object_binding| object_binding_lookup_value(object_binding, &property))
            .cloned();
        let existing_descriptor = existing_binding
            .and_then(|object_binding| object_binding_lookup_descriptor(object_binding, &property))
            .cloned();
        let current_enumerable = property_name.as_ref().is_some_and(|property_name| {
            self.state
                .speculation
                .static_semantics
                .local_object_binding(name)
                .or_else(|| self.backend.global_object_binding(name))
                .map(|object_binding| {
                    !object_binding
                        .non_enumerable_string_properties
                        .iter()
                        .any(|hidden_name| hidden_name == property_name)
                })
                .unwrap_or(false)
        });
        let enumerable = descriptor.enumerable.unwrap_or_else(|| {
            existing_descriptor
                .as_ref()
                .map(|descriptor| descriptor.enumerable)
                .unwrap_or(current_enumerable)
        });
        let configurable = descriptor.configurable.unwrap_or_else(|| {
            existing_descriptor
                .as_ref()
                .map(|descriptor| descriptor.configurable)
                .unwrap_or(false)
        });
        let (value, writable, getter, setter, has_get, has_set) = if descriptor.is_accessor() {
            (
                None,
                None,
                descriptor
                    .getter
                    .as_ref()
                    .map(|expression| {
                        self.materialize_emitted_define_property_value_expression(expression)
                    })
                    .or_else(|| {
                        existing_descriptor
                            .as_ref()
                            .and_then(|descriptor| descriptor.getter.clone())
                    }),
                descriptor
                    .setter
                    .as_ref()
                    .map(|expression| {
                        self.materialize_emitted_define_property_value_expression(expression)
                    })
                    .or_else(|| {
                        existing_descriptor
                            .as_ref()
                            .and_then(|descriptor| descriptor.setter.clone())
                    }),
                descriptor.getter.is_some()
                    || existing_descriptor
                        .as_ref()
                        .is_some_and(|descriptor| descriptor.has_get),
                descriptor.setter.is_some()
                    || existing_descriptor
                        .as_ref()
                        .is_some_and(|descriptor| descriptor.has_set),
            )
        } else {
            let value = descriptor
                .value
                .as_ref()
                .map(|expression| {
                    let this_binding = self
                        .state
                        .speculation
                        .execution_context
                        .direct_eval_in_class_field_initializer
                        .then(|| {
                            if name == "this" {
                                Expression::This
                            } else {
                                Expression::Identifier(name.to_string())
                            }
                        });
                    self.materialize_emitted_define_property_value_expression_with_this_binding(
                        expression,
                        this_binding.as_ref(),
                    )
                })
                .or_else(|| {
                    existing_value
                        .as_ref()
                        .map(|expression| self.materialize_static_expression(expression))
                })
                .or_else(|| {
                    existing_descriptor
                        .as_ref()
                        .and_then(|descriptor| descriptor.value.clone())
                })
                .unwrap_or(Expression::Undefined);
            let writable = descriptor.writable.or_else(|| {
                existing_descriptor
                    .as_ref()
                    .and_then(|descriptor| descriptor.writable)
            });
            (
                Some(value),
                Some(writable.unwrap_or(false)),
                None,
                None,
                false,
                false,
            )
        };
        let descriptor_binding = PropertyDescriptorBinding {
            value: value.clone(),
            configurable,
            enumerable,
            writable,
            getter,
            setter,
            has_get,
            has_set,
        };
        let updated_existing_local_binding = if let Some(object_binding) = self
            .state
            .speculation
            .static_semantics
            .local_object_binding_mut(name)
        {
            object_binding_define_property_descriptor(
                object_binding,
                property.clone(),
                descriptor_binding.clone(),
            );
            true
        } else {
            false
        };
        if self.binding_name_is_global(name) {
            let mut object_binding = self
                .backend
                .global_object_binding(name)
                .cloned()
                .unwrap_or_else(empty_object_value_binding);
            object_binding_define_property_descriptor(
                &mut object_binding,
                property,
                descriptor_binding,
            );
            self.backend
                .sync_global_object_binding(name, Some(object_binding));
        } else if !updated_existing_local_binding
            && (name == "this"
                || self
                    .resolve_function_binding_from_expression(&Expression::Identifier(
                        name.to_string(),
                    ))
                    .is_some())
        {
            if name == "this" {
                if let Some(object_binding) = self
                    .state
                    .speculation
                    .static_semantics
                    .local_object_binding_mut(name)
                {
                    object_binding_define_property_descriptor(
                        object_binding,
                        property,
                        descriptor_binding,
                    );
                    self.state
                        .speculation
                        .static_semantics
                        .set_local_kind(name, StaticValueKind::Object);
                }
            } else {
                let object_binding = self
                    .state
                    .speculation
                    .static_semantics
                    .ensure_local_object_binding(name);
                object_binding_define_property_descriptor(
                    object_binding,
                    property,
                    descriptor_binding,
                );
                self.state
                    .speculation
                    .static_semantics
                    .set_local_kind(name, StaticValueKind::Object);
            }
        }

        let shadow_owner = self
            .runtime_object_property_shadow_owner_name_for_identifier(name)
            .unwrap_or_else(|| name.to_string());
        let updated_binding = self
            .state
            .speculation
            .static_semantics
            .local_object_binding(name)
            .cloned()
            .or_else(|| self.backend.global_object_binding(name).cloned());
        if let Some(object_binding) = updated_binding {
            self.sync_runtime_object_property_shadow_static_metadata_from_binding(
                &shadow_owner,
                &object_binding,
            );
        }
    }

    fn define_prototype_object_property_from_descriptor(
        &mut self,
        name: &str,
        property: &Expression,
        descriptor: &PropertyDescriptorDefinition,
    ) {
        let property = self.canonical_object_property_expression(property);
        if std::env::var_os("AYY_TRACE_DEFINE_PROPERTY_UPDATE").is_some() {
            eprintln!(
                "define_property_update prototype target={name} canonical_property={property:?}"
            );
        }
        let property_name = static_property_name_from_expression(&property);
        let existing_binding = self
            .state
            .speculation
            .static_semantics
            .objects
            .local_prototype_object_bindings
            .get(name)
            .or_else(|| self.backend.global_prototype_object_binding(name));
        if existing_binding.is_some_and(|object_binding| {
            !object_binding_can_define_property(object_binding, &property)
        }) {
            return;
        }
        let existing_binding = self
            .state
            .speculation
            .static_semantics
            .objects
            .local_prototype_object_bindings
            .get(name)
            .or_else(|| self.backend.global_prototype_object_binding(name));
        let existing_value = existing_binding
            .and_then(|object_binding| object_binding_lookup_value(object_binding, &property))
            .cloned();
        let existing_descriptor = existing_binding
            .and_then(|object_binding| object_binding_lookup_descriptor(object_binding, &property))
            .cloned();
        let current_enumerable = property_name.as_ref().is_some_and(|property_name| {
            self.state
                .speculation
                .static_semantics
                .objects
                .local_prototype_object_bindings
                .get(name)
                .or_else(|| self.backend.global_prototype_object_binding(name))
                .map(|object_binding| {
                    !object_binding
                        .non_enumerable_string_properties
                        .iter()
                        .any(|hidden_name| hidden_name == property_name)
                })
                .unwrap_or(false)
        });
        let enumerable = descriptor.enumerable.unwrap_or_else(|| {
            existing_descriptor
                .as_ref()
                .map(|descriptor| descriptor.enumerable)
                .unwrap_or(current_enumerable)
        });
        let configurable = descriptor.configurable.unwrap_or_else(|| {
            existing_descriptor
                .as_ref()
                .map(|descriptor| descriptor.configurable)
                .unwrap_or(false)
        });
        let (value, writable, getter, setter, has_get, has_set) = if descriptor.is_accessor() {
            (
                None,
                None,
                descriptor
                    .getter
                    .as_ref()
                    .map(|expression| {
                        self.materialize_emitted_define_property_value_expression(expression)
                    })
                    .or_else(|| {
                        existing_descriptor
                            .as_ref()
                            .and_then(|descriptor| descriptor.getter.clone())
                    }),
                descriptor
                    .setter
                    .as_ref()
                    .map(|expression| {
                        self.materialize_emitted_define_property_value_expression(expression)
                    })
                    .or_else(|| {
                        existing_descriptor
                            .as_ref()
                            .and_then(|descriptor| descriptor.setter.clone())
                    }),
                descriptor.getter.is_some()
                    || existing_descriptor
                        .as_ref()
                        .is_some_and(|descriptor| descriptor.has_get),
                descriptor.setter.is_some()
                    || existing_descriptor
                        .as_ref()
                        .is_some_and(|descriptor| descriptor.has_set),
            )
        } else {
            let value = descriptor
                .value
                .as_ref()
                .map(|expression| {
                    self.materialize_emitted_define_property_value_expression(expression)
                })
                .or_else(|| {
                    existing_value
                        .as_ref()
                        .map(|expression| self.materialize_static_expression(expression))
                })
                .or_else(|| {
                    existing_descriptor
                        .as_ref()
                        .and_then(|descriptor| descriptor.value.clone())
                })
                .unwrap_or(Expression::Undefined);
            let writable = descriptor.writable.or_else(|| {
                existing_descriptor
                    .as_ref()
                    .and_then(|descriptor| descriptor.writable)
            });
            (
                Some(value),
                Some(writable.unwrap_or(false)),
                None,
                None,
                false,
                false,
            )
        };
        let descriptor_binding = PropertyDescriptorBinding {
            value,
            configurable,
            enumerable,
            writable,
            getter,
            setter,
            has_get,
            has_set,
        };
        let updated_existing_local_binding = if let Some(object_binding) = self
            .state
            .speculation
            .static_semantics
            .objects
            .local_prototype_object_bindings
            .get_mut(name)
        {
            object_binding_define_property_descriptor(
                object_binding,
                property.clone(),
                descriptor_binding.clone(),
            );
            true
        } else {
            false
        };
        if self.binding_name_is_global(name) {
            let mut object_binding = self
                .backend
                .global_prototype_object_binding(name)
                .cloned()
                .unwrap_or_else(empty_object_value_binding);
            object_binding_define_property_descriptor(
                &mut object_binding,
                property,
                descriptor_binding,
            );
            if std::env::var_os("AYY_TRACE_DEFINE_PROPERTY_UPDATE").is_some() {
                eprintln!(
                    "define_property_update prototype_sync target={name} symbols={:?}",
                    object_binding
                        .symbol_properties
                        .iter()
                        .map(|(key, _)| key)
                        .collect::<Vec<_>>()
                );
            }
            self.backend
                .sync_global_prototype_object_binding(name, Some(object_binding));
        } else if !updated_existing_local_binding {
            let object_binding = self
                .state
                .speculation
                .static_semantics
                .objects
                .local_prototype_object_bindings
                .entry(name.to_string())
                .or_insert_with(empty_object_value_binding);
            object_binding_define_property_descriptor(object_binding, property, descriptor_binding);
            if std::env::var_os("AYY_TRACE_DEFINE_PROPERTY_UPDATE").is_some() {
                eprintln!(
                    "define_property_update local_prototype target={name} symbols={:?}",
                    object_binding
                        .symbol_properties
                        .iter()
                        .map(|(key, _)| key)
                        .collect::<Vec<_>>()
                );
            }
        }
    }

    pub(in crate::backend::direct_wasm) fn materialize_define_property_value_expression(
        &self,
        expression: &Expression,
    ) -> Expression {
        self.materialize_define_property_value_expression_with_this_binding(expression, None)
    }

    pub(in crate::backend::direct_wasm) fn materialize_define_property_value_expression_with_this_binding(
        &self,
        expression: &Expression,
        this_binding: Option<&Expression>,
    ) -> Expression {
        let direct_eval_in_class_field_initializer = self
            .state
            .speculation
            .execution_context
            .direct_eval_in_class_field_initializer;
        let rewritten = Self::rewrite_static_define_property_eval_expression(
            expression,
            this_binding,
            direct_eval_in_class_field_initializer,
        );
        if !inline_summary_side_effect_free_expression(&rewritten)
            && !Self::define_property_expression_mentions_direct_eval(&rewritten)
        {
            return rewritten;
        }
        let mut environment = self.snapshot_static_resolution_environment_without_locals();
        self.resolve_static_define_property_value_expression_with_eval_environment(
            &rewritten,
            self.current_function_name(),
            direct_eval_in_class_field_initializer,
            self.state.speculation.execution_context.strict_mode,
            &mut environment,
            this_binding,
        )
        .or_else(|| {
            self.resolve_static_define_property_nested_eval_expression(
                &rewritten,
                self.current_function_name(),
                direct_eval_in_class_field_initializer,
                self.state.speculation.execution_context.strict_mode,
                &mut environment,
                this_binding,
            )
        })
        .or_else(|| self.evaluate_static_expression_with_state(&rewritten, &mut environment))
        .or_else(|| self.materialize_static_expression_with_state(&rewritten, &environment))
        .unwrap_or_else(|| self.materialize_static_expression(&rewritten))
    }

    fn materialize_emitted_define_property_value_expression(
        &self,
        expression: &Expression,
    ) -> Expression {
        self.materialize_emitted_define_property_value_expression_with_this_binding(
            expression, None,
        )
    }

    fn materialize_emitted_define_property_value_expression_with_this_binding(
        &self,
        expression: &Expression,
        this_binding: Option<&Expression>,
    ) -> Expression {
        if let Expression::Update {
            name,
            op,
            prefix: false,
        } = expression
            && let Expression::Number(current) =
                self.materialize_static_expression(&Expression::Identifier(name.clone()))
        {
            return Expression::Number(match op {
                UpdateOp::Increment => current - 1.0,
                UpdateOp::Decrement => current + 1.0,
            });
        }

        self.materialize_define_property_value_expression_with_this_binding(
            expression,
            this_binding,
        )
    }

    fn define_property_expression_mentions_direct_eval(expression: &Expression) -> bool {
        match expression {
            Expression::Call { callee, arguments } => {
                matches!(callee.as_ref(), Expression::Identifier(name) if name == "eval")
                    || Self::define_property_expression_mentions_direct_eval(callee)
                    || arguments.iter().any(|argument| match argument {
                        CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                            Self::define_property_expression_mentions_direct_eval(expression)
                        }
                    })
            }
            Expression::SuperCall { callee, arguments } | Expression::New { callee, arguments } => {
                Self::define_property_expression_mentions_direct_eval(callee)
                    || arguments.iter().any(|argument| match argument {
                        CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                            Self::define_property_expression_mentions_direct_eval(expression)
                        }
                    })
            }
            Expression::Array(elements) => elements.iter().any(|element| match element {
                ArrayElement::Expression(expression) | ArrayElement::Spread(expression) => {
                    Self::define_property_expression_mentions_direct_eval(expression)
                }
            }),
            Expression::Object(entries) => entries.iter().any(|entry| match entry {
                ObjectEntry::Data { key, value } => {
                    Self::define_property_expression_mentions_direct_eval(key)
                        || Self::define_property_expression_mentions_direct_eval(value)
                }
                ObjectEntry::Getter { key, getter } => {
                    Self::define_property_expression_mentions_direct_eval(key)
                        || Self::define_property_expression_mentions_direct_eval(getter)
                }
                ObjectEntry::Setter { key, setter } => {
                    Self::define_property_expression_mentions_direct_eval(key)
                        || Self::define_property_expression_mentions_direct_eval(setter)
                }
                ObjectEntry::Spread(expression) => {
                    Self::define_property_expression_mentions_direct_eval(expression)
                }
            }),
            Expression::Member { object, property } => {
                Self::define_property_expression_mentions_direct_eval(object)
                    || Self::define_property_expression_mentions_direct_eval(property)
            }
            Expression::AssignMember {
                object,
                property,
                value,
            } => {
                Self::define_property_expression_mentions_direct_eval(object)
                    || Self::define_property_expression_mentions_direct_eval(property)
                    || Self::define_property_expression_mentions_direct_eval(value)
            }
            Expression::Assign { value, .. }
            | Expression::Await(value)
            | Expression::EnumerateKeys(value)
            | Expression::GetIterator(value)
            | Expression::IteratorClose(value)
            | Expression::Unary {
                expression: value, ..
            } => Self::define_property_expression_mentions_direct_eval(value),
            Expression::AssignSuperMember { property, value } => {
                Self::define_property_expression_mentions_direct_eval(property)
                    || Self::define_property_expression_mentions_direct_eval(value)
            }
            Expression::SuperMember { property } => {
                Self::define_property_expression_mentions_direct_eval(property)
            }
            Expression::Binary { left, right, .. } => {
                Self::define_property_expression_mentions_direct_eval(left)
                    || Self::define_property_expression_mentions_direct_eval(right)
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                Self::define_property_expression_mentions_direct_eval(condition)
                    || Self::define_property_expression_mentions_direct_eval(then_expression)
                    || Self::define_property_expression_mentions_direct_eval(else_expression)
            }
            Expression::Sequence(expressions) => expressions
                .iter()
                .any(Self::define_property_expression_mentions_direct_eval),
            Expression::Number(_)
            | Expression::BigInt(_)
            | Expression::String(_)
            | Expression::Bool(_)
            | Expression::Null
            | Expression::Undefined
            | Expression::Identifier(_)
            | Expression::This
            | Expression::NewTarget
            | Expression::Sent
            | Expression::Update { .. } => false,
        }
    }

    fn resolve_static_define_property_nested_eval_expression(
        &self,
        expression: &Expression,
        current_function_name: Option<&str>,
        direct_eval_in_class_field_initializer: bool,
        strict_mode: bool,
        environment: &mut StaticResolutionEnvironment,
        this_binding: Option<&Expression>,
    ) -> Option<Expression> {
        if let Some(value) = self
            .resolve_static_define_property_value_expression_with_eval_environment(
                expression,
                current_function_name,
                direct_eval_in_class_field_initializer,
                strict_mode,
                environment,
                this_binding,
            )
        {
            return Some(value);
        }

        let base_environment = environment.clone();
        let rewritten = materialize_recursive_expression(expression, true, true, &|nested| {
            let mut nested_environment = base_environment.clone();
            self.resolve_static_define_property_nested_eval_expression(
                nested,
                current_function_name,
                direct_eval_in_class_field_initializer,
                strict_mode,
                &mut nested_environment,
                this_binding,
            )
            .or_else(|| self.evaluate_static_expression_with_state(nested, &mut nested_environment))
            .or_else(|| self.materialize_static_expression_with_state(nested, &nested_environment))
            .or_else(|| Some(self.materialize_static_expression(nested)))
        })?;

        self.evaluate_static_expression_with_state(&rewritten, environment)
            .or_else(|| self.materialize_static_expression_with_state(&rewritten, environment))
            .or(Some(rewritten))
    }

    pub(in crate::backend::direct_wasm) fn resolve_static_define_property_value_expression(
        &self,
        expression: &Expression,
    ) -> Option<Expression> {
        self.resolve_static_define_property_value_expression_with_eval_context(
            expression,
            self.current_function_name(),
            self.state
                .speculation
                .execution_context
                .direct_eval_in_class_field_initializer,
            self.state.speculation.execution_context.strict_mode,
        )
    }

    pub(in crate::backend::direct_wasm) fn resolve_static_define_property_value_expression_with_eval_context(
        &self,
        expression: &Expression,
        current_function_name: Option<&str>,
        direct_eval_in_class_field_initializer: bool,
        strict_mode: bool,
    ) -> Option<Expression> {
        let mut environment = self.snapshot_static_resolution_environment_without_locals();
        self.resolve_static_define_property_value_expression_with_eval_environment(
            expression,
            current_function_name,
            direct_eval_in_class_field_initializer,
            strict_mode,
            &mut environment,
            None,
        )
    }

    pub(in crate::backend::direct_wasm) fn resolve_static_define_property_value_expression_with_eval_environment(
        &self,
        expression: &Expression,
        current_function_name: Option<&str>,
        direct_eval_in_class_field_initializer: bool,
        strict_mode: bool,
        environment: &mut StaticResolutionEnvironment,
        this_binding: Option<&Expression>,
    ) -> Option<Expression> {
        if let Expression::Sequence(expressions) = expression {
            let mut completion = Expression::Undefined;
            for expression in expressions {
                completion = self
                    .resolve_static_define_property_value_expression_with_eval_environment(
                        expression,
                        current_function_name,
                        direct_eval_in_class_field_initializer,
                        strict_mode,
                        environment,
                        this_binding,
                    )
                    .or_else(|| self.evaluate_static_expression_with_state(expression, environment))
                    .or_else(|| {
                        self.materialize_static_expression_with_state(expression, environment)
                    })?;
            }
            return Some(completion);
        }

        let Expression::Call { callee, arguments } = expression else {
            return None;
        };
        if !matches!(callee.as_ref(), Expression::Identifier(name) if name == "eval") {
            return None;
        }
        if !matches!(
            self.resolve_function_binding_from_expression(callee),
            Some(LocalFunctionBinding::Builtin(function_name)) if function_name == "eval"
        ) {
            return None;
        }

        self.resolve_static_define_property_eval_completion_with_context_in_environment_mut(
            arguments,
            current_function_name,
            direct_eval_in_class_field_initializer,
            strict_mode,
            environment,
            this_binding,
        )
    }

    pub(in crate::backend::direct_wasm) fn resolve_static_define_property_eval_completion(
        &self,
        arguments: &[CallArgument],
    ) -> Option<Expression> {
        self.resolve_static_define_property_eval_completion_with_context(
            arguments,
            self.current_function_name(),
            self.state
                .speculation
                .execution_context
                .direct_eval_in_class_field_initializer,
            self.state.speculation.execution_context.strict_mode,
        )
    }

    pub(in crate::backend::direct_wasm) fn resolve_static_define_property_eval_completion_with_context(
        &self,
        arguments: &[CallArgument],
        current_function_name: Option<&str>,
        direct_eval_in_class_field_initializer: bool,
        strict_mode: bool,
    ) -> Option<Expression> {
        self.resolve_static_define_property_eval_completion_with_context_in_environment(
            arguments,
            current_function_name,
            direct_eval_in_class_field_initializer,
            strict_mode,
            &mut self.snapshot_static_resolution_environment_without_locals(),
            None,
        )
    }

    fn resolve_static_define_property_eval_completion_with_context_in_environment(
        &self,
        arguments: &[CallArgument],
        current_function_name: Option<&str>,
        direct_eval_in_class_field_initializer: bool,
        strict_mode: bool,
        environment: &StaticResolutionEnvironment,
        this_binding: Option<&Expression>,
    ) -> Option<Expression> {
        let mut environment = environment.clone();
        self.resolve_static_define_property_eval_completion_with_context_in_environment_mut(
            arguments,
            current_function_name,
            direct_eval_in_class_field_initializer,
            strict_mode,
            &mut environment,
            this_binding,
        )
    }

    fn resolve_static_define_property_eval_completion_with_context_in_environment_mut(
        &self,
        arguments: &[CallArgument],
        current_function_name: Option<&str>,
        direct_eval_in_class_field_initializer: bool,
        strict_mode: bool,
        environment: &mut StaticResolutionEnvironment,
        this_binding: Option<&Expression>,
    ) -> Option<Expression> {
        let CallArgument::Expression(Expression::String(argument_source)) = arguments.first()?
        else {
            return None;
        };

        let raw_source = argument_source.clone();
        let argument_source = if strict_mode {
            let mut strict_argument_source = String::from("\"use strict\";");
            strict_argument_source.push_str(argument_source);
            std::borrow::Cow::Owned(strict_argument_source)
        } else {
            std::borrow::Cow::Borrowed(argument_source.as_str())
        };

        let mut program = if let Some(program) = self.parse_define_property_eval_program_in_context(
            &argument_source,
            current_function_name,
            direct_eval_in_class_field_initializer,
        ) {
            program
        } else if let Ok(program) = frontend::parse_script_goal(&argument_source) {
            program
        } else {
            return None;
        };

        namespace_eval_program_internal_function_names(
            &mut program,
            current_function_name,
            &raw_source,
        );
        self.normalize_eval_scoped_bindings_to_source_names(&mut program);

        if ((direct_eval_in_class_field_initializer
            && self
                .state
                .speculation
                .execution_context
                .direct_eval_in_class_field_initializer)
            && self.eval_arguments_initializer_conflict(&program))
            || self.eval_arguments_declaration_conflicts(&program)
            || self.eval_program_declares_var_collision_with_global_lexical(&program)
            || self.eval_program_declares_var_collision_with_active_lexical(&program)
            || self.eval_program_declares_non_definable_global_function(&program)
            || self.eval_program_declares_non_declarable_global_var(&program, false)
        {
            return None;
        }

        let rewritten_statements = program
            .statements
            .iter()
            .map(|statement| {
                Self::rewrite_static_define_property_eval_statement(
                    statement,
                    this_binding,
                    direct_eval_in_class_field_initializer,
                )
            })
            .collect::<Vec<_>>();
        let completion = self
            .evaluate_static_define_property_eval_statements(&rewritten_statements, environment)?;
        let materialized = self
            .materialize_static_expression_with_state(&completion, environment)
            .unwrap_or_else(|| self.materialize_static_expression(&completion));

        Some(match materialized {
            Expression::NewTarget if direct_eval_in_class_field_initializer => {
                Expression::Undefined
            }
            other => other,
        })
    }

    fn evaluate_static_define_property_eval_statements(
        &self,
        statements: &[Statement],
        environment: &mut StaticResolutionEnvironment,
    ) -> Option<Expression> {
        let mut completion = Expression::Undefined;
        for statement in statements {
            match statement {
                Statement::Declaration { body }
                | Statement::Block { body }
                | Statement::Labeled { body, .. } => {
                    completion =
                        self.evaluate_static_define_property_eval_statements(body, environment)?;
                }
                Statement::If {
                    condition,
                    then_branch,
                    else_branch,
                } => {
                    let condition = self
                        .evaluate_static_expression_with_state(condition, environment)
                        .or_else(|| {
                            self.materialize_static_expression_with_state(condition, environment)
                        })?;
                    completion = match condition {
                        Expression::Bool(true) => self
                            .evaluate_static_define_property_eval_statements(
                                then_branch,
                                environment,
                            )?,
                        Expression::Bool(false) => self
                            .evaluate_static_define_property_eval_statements(
                                else_branch,
                                environment,
                            )?,
                        _ => return None,
                    };
                }
                Statement::Assign { name, value } => {
                    completion = self
                        .evaluate_static_expression_with_state(
                            &Expression::Assign {
                                name: name.clone(),
                                value: Box::new(value.clone()),
                            },
                            environment,
                        )
                        .or_else(|| {
                            self.materialize_static_expression_with_state(value, environment)
                        })?;
                }
                Statement::AssignMember {
                    object,
                    property,
                    value,
                } => {
                    completion = self
                        .evaluate_static_expression_with_state(
                            &Expression::AssignMember {
                                object: Box::new(object.clone()),
                                property: Box::new(property.clone()),
                                value: Box::new(value.clone()),
                            },
                            environment,
                        )
                        .or_else(|| {
                            self.materialize_static_expression_with_state(value, environment)
                        })?;
                }
                Statement::Expression(expression) => {
                    completion = self
                        .evaluate_static_expression_with_state(expression, environment)
                        .or_else(|| {
                            self.materialize_static_expression_with_state(expression, environment)
                        })?;
                }
                Statement::Return(expression) => {
                    return self
                        .evaluate_static_expression_with_state(expression, environment)
                        .or_else(|| {
                            self.materialize_static_expression_with_state(expression, environment)
                        });
                }
                Statement::Var { .. } | Statement::Let { .. } | Statement::Print { .. } => {
                    self.execute_static_statements_with_state(
                        std::slice::from_ref(statement),
                        environment,
                    )?;
                }
                _ => return None,
            }
        }
        Some(completion)
    }

    fn rewrite_static_define_property_eval_statement(
        statement: &Statement,
        this_binding: Option<&Expression>,
        direct_eval_in_class_field_initializer: bool,
    ) -> Statement {
        match statement {
            Statement::Declaration { body } => Statement::Declaration {
                body: body
                    .iter()
                    .map(|statement| {
                        Self::rewrite_static_define_property_eval_statement(
                            statement,
                            this_binding,
                            direct_eval_in_class_field_initializer,
                        )
                    })
                    .collect(),
            },
            Statement::Block { body } => Statement::Block {
                body: body
                    .iter()
                    .map(|statement| {
                        Self::rewrite_static_define_property_eval_statement(
                            statement,
                            this_binding,
                            direct_eval_in_class_field_initializer,
                        )
                    })
                    .collect(),
            },
            Statement::Labeled { labels, body } => Statement::Labeled {
                labels: labels.clone(),
                body: body
                    .iter()
                    .map(|statement| {
                        Self::rewrite_static_define_property_eval_statement(
                            statement,
                            this_binding,
                            direct_eval_in_class_field_initializer,
                        )
                    })
                    .collect(),
            },
            Statement::Var { name, value } => Statement::Var {
                name: name.clone(),
                value: Self::rewrite_static_define_property_eval_expression(
                    value,
                    this_binding,
                    direct_eval_in_class_field_initializer,
                ),
            },
            Statement::Let {
                name,
                mutable,
                value,
            } => Statement::Let {
                name: name.clone(),
                mutable: *mutable,
                value: Self::rewrite_static_define_property_eval_expression(
                    value,
                    this_binding,
                    direct_eval_in_class_field_initializer,
                ),
            },
            Statement::Assign { name, value } => Statement::Assign {
                name: name.clone(),
                value: Self::rewrite_static_define_property_eval_expression(
                    value,
                    this_binding,
                    direct_eval_in_class_field_initializer,
                ),
            },
            Statement::AssignMember {
                object,
                property,
                value,
            } => Statement::AssignMember {
                object: Self::rewrite_static_define_property_eval_expression(
                    object,
                    this_binding,
                    direct_eval_in_class_field_initializer,
                ),
                property: Self::rewrite_static_define_property_eval_expression(
                    property,
                    this_binding,
                    direct_eval_in_class_field_initializer,
                ),
                value: Self::rewrite_static_define_property_eval_expression(
                    value,
                    this_binding,
                    direct_eval_in_class_field_initializer,
                ),
            },
            Statement::Print { values } => Statement::Print {
                values: values
                    .iter()
                    .map(|value| {
                        Self::rewrite_static_define_property_eval_expression(
                            value,
                            this_binding,
                            direct_eval_in_class_field_initializer,
                        )
                    })
                    .collect(),
            },
            Statement::Expression(expression) => {
                Statement::Expression(Self::rewrite_static_define_property_eval_expression(
                    expression,
                    this_binding,
                    direct_eval_in_class_field_initializer,
                ))
            }
            Statement::Throw(expression) => {
                Statement::Throw(Self::rewrite_static_define_property_eval_expression(
                    expression,
                    this_binding,
                    direct_eval_in_class_field_initializer,
                ))
            }
            Statement::Return(expression) => {
                Statement::Return(Self::rewrite_static_define_property_eval_expression(
                    expression,
                    this_binding,
                    direct_eval_in_class_field_initializer,
                ))
            }
            Statement::With { object, body } => Statement::With {
                object: Self::rewrite_static_define_property_eval_expression(
                    object,
                    this_binding,
                    direct_eval_in_class_field_initializer,
                ),
                body: body
                    .iter()
                    .map(|statement| {
                        Self::rewrite_static_define_property_eval_statement(
                            statement,
                            this_binding,
                            direct_eval_in_class_field_initializer,
                        )
                    })
                    .collect(),
            },
            Statement::If {
                condition,
                then_branch,
                else_branch,
            } => Statement::If {
                condition: Self::rewrite_static_define_property_eval_expression(
                    condition,
                    this_binding,
                    direct_eval_in_class_field_initializer,
                ),
                then_branch: then_branch
                    .iter()
                    .map(|statement| {
                        Self::rewrite_static_define_property_eval_statement(
                            statement,
                            this_binding,
                            direct_eval_in_class_field_initializer,
                        )
                    })
                    .collect(),
                else_branch: else_branch
                    .iter()
                    .map(|statement| {
                        Self::rewrite_static_define_property_eval_statement(
                            statement,
                            this_binding,
                            direct_eval_in_class_field_initializer,
                        )
                    })
                    .collect(),
            },
            Statement::Try {
                body,
                catch_binding,
                catch_setup,
                catch_body,
            } => Statement::Try {
                body: body
                    .iter()
                    .map(|statement| {
                        Self::rewrite_static_define_property_eval_statement(
                            statement,
                            this_binding,
                            direct_eval_in_class_field_initializer,
                        )
                    })
                    .collect(),
                catch_binding: catch_binding.clone(),
                catch_setup: catch_setup
                    .iter()
                    .map(|statement| {
                        Self::rewrite_static_define_property_eval_statement(
                            statement,
                            this_binding,
                            direct_eval_in_class_field_initializer,
                        )
                    })
                    .collect(),
                catch_body: catch_body
                    .iter()
                    .map(|statement| {
                        Self::rewrite_static_define_property_eval_statement(
                            statement,
                            this_binding,
                            direct_eval_in_class_field_initializer,
                        )
                    })
                    .collect(),
            },
            Statement::Switch {
                labels,
                bindings,
                discriminant,
                cases,
            } => Statement::Switch {
                labels: labels.clone(),
                bindings: bindings.clone(),
                discriminant: Self::rewrite_static_define_property_eval_expression(
                    discriminant,
                    this_binding,
                    direct_eval_in_class_field_initializer,
                ),
                cases: cases
                    .iter()
                    .map(|case| crate::ir::hir::SwitchCase {
                        test: case.test.as_ref().map(|test| {
                            Self::rewrite_static_define_property_eval_expression(
                                test,
                                this_binding,
                                direct_eval_in_class_field_initializer,
                            )
                        }),
                        body: case
                            .body
                            .iter()
                            .map(|statement| {
                                Self::rewrite_static_define_property_eval_statement(
                                    statement,
                                    this_binding,
                                    direct_eval_in_class_field_initializer,
                                )
                            })
                            .collect(),
                    })
                    .collect(),
            },
            Statement::For {
                labels,
                init,
                per_iteration_bindings,
                condition,
                update,
                break_hook,
                body,
            } => Statement::For {
                labels: labels.clone(),
                init: init
                    .iter()
                    .map(|statement| {
                        Self::rewrite_static_define_property_eval_statement(
                            statement,
                            this_binding,
                            direct_eval_in_class_field_initializer,
                        )
                    })
                    .collect(),
                per_iteration_bindings: per_iteration_bindings.clone(),
                condition: condition.as_ref().map(|condition| {
                    Self::rewrite_static_define_property_eval_expression(
                        condition,
                        this_binding,
                        direct_eval_in_class_field_initializer,
                    )
                }),
                update: update.as_ref().map(|update| {
                    Self::rewrite_static_define_property_eval_expression(
                        update,
                        this_binding,
                        direct_eval_in_class_field_initializer,
                    )
                }),
                break_hook: break_hook.as_ref().map(|break_hook| {
                    Self::rewrite_static_define_property_eval_expression(
                        break_hook,
                        this_binding,
                        direct_eval_in_class_field_initializer,
                    )
                }),
                body: body
                    .iter()
                    .map(|statement| {
                        Self::rewrite_static_define_property_eval_statement(
                            statement,
                            this_binding,
                            direct_eval_in_class_field_initializer,
                        )
                    })
                    .collect(),
            },
            Statement::While {
                labels,
                condition,
                break_hook,
                body,
            } => Statement::While {
                labels: labels.clone(),
                condition: Self::rewrite_static_define_property_eval_expression(
                    condition,
                    this_binding,
                    direct_eval_in_class_field_initializer,
                ),
                break_hook: break_hook.as_ref().map(|break_hook| {
                    Self::rewrite_static_define_property_eval_expression(
                        break_hook,
                        this_binding,
                        direct_eval_in_class_field_initializer,
                    )
                }),
                body: body
                    .iter()
                    .map(|statement| {
                        Self::rewrite_static_define_property_eval_statement(
                            statement,
                            this_binding,
                            direct_eval_in_class_field_initializer,
                        )
                    })
                    .collect(),
            },
            Statement::DoWhile {
                labels,
                condition,
                break_hook,
                body,
            } => Statement::DoWhile {
                labels: labels.clone(),
                condition: Self::rewrite_static_define_property_eval_expression(
                    condition,
                    this_binding,
                    direct_eval_in_class_field_initializer,
                ),
                break_hook: break_hook.as_ref().map(|break_hook| {
                    Self::rewrite_static_define_property_eval_expression(
                        break_hook,
                        this_binding,
                        direct_eval_in_class_field_initializer,
                    )
                }),
                body: body
                    .iter()
                    .map(|statement| {
                        Self::rewrite_static_define_property_eval_statement(
                            statement,
                            this_binding,
                            direct_eval_in_class_field_initializer,
                        )
                    })
                    .collect(),
            },
            Statement::Break { label } => Statement::Break {
                label: label.clone(),
            },
            Statement::Continue { label } => Statement::Continue {
                label: label.clone(),
            },
            Statement::Yield { value } => Statement::Yield {
                value: Self::rewrite_static_define_property_eval_expression(
                    value,
                    this_binding,
                    direct_eval_in_class_field_initializer,
                ),
            },
            Statement::YieldDelegate { value } => Statement::YieldDelegate {
                value: Self::rewrite_static_define_property_eval_expression(
                    value,
                    this_binding,
                    direct_eval_in_class_field_initializer,
                ),
            },
        }
    }

    fn rewrite_static_define_property_eval_expression(
        expression: &Expression,
        this_binding: Option<&Expression>,
        direct_eval_in_class_field_initializer: bool,
    ) -> Expression {
        match expression {
            Expression::This => this_binding.cloned().unwrap_or(Expression::This),
            Expression::NewTarget if direct_eval_in_class_field_initializer => {
                Expression::Undefined
            }
            Expression::Member { object, property } => Expression::Member {
                object: Box::new(Self::rewrite_static_define_property_eval_expression(
                    object,
                    this_binding,
                    direct_eval_in_class_field_initializer,
                )),
                property: Box::new(Self::rewrite_static_define_property_eval_expression(
                    property,
                    this_binding,
                    direct_eval_in_class_field_initializer,
                )),
            },
            Expression::SuperMember { property } => Expression::SuperMember {
                property: Box::new(Self::rewrite_static_define_property_eval_expression(
                    property,
                    this_binding,
                    direct_eval_in_class_field_initializer,
                )),
            },
            Expression::Assign { name, value } => Expression::Assign {
                name: name.clone(),
                value: Box::new(Self::rewrite_static_define_property_eval_expression(
                    value,
                    this_binding,
                    direct_eval_in_class_field_initializer,
                )),
            },
            Expression::AssignMember {
                object,
                property,
                value,
            } => Expression::AssignMember {
                object: Box::new(Self::rewrite_static_define_property_eval_expression(
                    object,
                    this_binding,
                    direct_eval_in_class_field_initializer,
                )),
                property: Box::new(Self::rewrite_static_define_property_eval_expression(
                    property,
                    this_binding,
                    direct_eval_in_class_field_initializer,
                )),
                value: Box::new(Self::rewrite_static_define_property_eval_expression(
                    value,
                    this_binding,
                    direct_eval_in_class_field_initializer,
                )),
            },
            Expression::AssignSuperMember { property, value } => Expression::AssignSuperMember {
                property: Box::new(Self::rewrite_static_define_property_eval_expression(
                    property,
                    this_binding,
                    direct_eval_in_class_field_initializer,
                )),
                value: Box::new(Self::rewrite_static_define_property_eval_expression(
                    value,
                    this_binding,
                    direct_eval_in_class_field_initializer,
                )),
            },
            Expression::Await(value) => Expression::Await(Box::new(
                Self::rewrite_static_define_property_eval_expression(
                    value,
                    this_binding,
                    direct_eval_in_class_field_initializer,
                ),
            )),
            Expression::EnumerateKeys(value) => Expression::EnumerateKeys(Box::new(
                Self::rewrite_static_define_property_eval_expression(
                    value,
                    this_binding,
                    direct_eval_in_class_field_initializer,
                ),
            )),
            Expression::GetIterator(value) => Expression::GetIterator(Box::new(
                Self::rewrite_static_define_property_eval_expression(
                    value,
                    this_binding,
                    direct_eval_in_class_field_initializer,
                ),
            )),
            Expression::IteratorClose(value) => Expression::IteratorClose(Box::new(
                Self::rewrite_static_define_property_eval_expression(
                    value,
                    this_binding,
                    direct_eval_in_class_field_initializer,
                ),
            )),
            Expression::Unary { op, expression } => Expression::Unary {
                op: *op,
                expression: Box::new(Self::rewrite_static_define_property_eval_expression(
                    expression,
                    this_binding,
                    direct_eval_in_class_field_initializer,
                )),
            },
            Expression::Binary { op, left, right } => Expression::Binary {
                op: *op,
                left: Box::new(Self::rewrite_static_define_property_eval_expression(
                    left,
                    this_binding,
                    direct_eval_in_class_field_initializer,
                )),
                right: Box::new(Self::rewrite_static_define_property_eval_expression(
                    right,
                    this_binding,
                    direct_eval_in_class_field_initializer,
                )),
            },
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => Expression::Conditional {
                condition: Box::new(Self::rewrite_static_define_property_eval_expression(
                    condition,
                    this_binding,
                    direct_eval_in_class_field_initializer,
                )),
                then_expression: Box::new(Self::rewrite_static_define_property_eval_expression(
                    then_expression,
                    this_binding,
                    direct_eval_in_class_field_initializer,
                )),
                else_expression: Box::new(Self::rewrite_static_define_property_eval_expression(
                    else_expression,
                    this_binding,
                    direct_eval_in_class_field_initializer,
                )),
            },
            Expression::Sequence(expressions) => Expression::Sequence(
                expressions
                    .iter()
                    .map(|expression| {
                        Self::rewrite_static_define_property_eval_expression(
                            expression,
                            this_binding,
                            direct_eval_in_class_field_initializer,
                        )
                    })
                    .collect(),
            ),
            Expression::Call { callee, arguments } => Expression::Call {
                callee: Box::new(Self::rewrite_static_define_property_eval_expression(
                    callee,
                    this_binding,
                    direct_eval_in_class_field_initializer,
                )),
                arguments: arguments
                    .iter()
                    .map(|argument| match argument {
                        CallArgument::Expression(expression) => CallArgument::Expression(
                            Self::rewrite_static_define_property_eval_expression(
                                expression,
                                this_binding,
                                direct_eval_in_class_field_initializer,
                            ),
                        ),
                        CallArgument::Spread(expression) => CallArgument::Spread(
                            Self::rewrite_static_define_property_eval_expression(
                                expression,
                                this_binding,
                                direct_eval_in_class_field_initializer,
                            ),
                        ),
                    })
                    .collect(),
            },
            Expression::SuperCall { callee, arguments } => Expression::SuperCall {
                callee: Box::new(Self::rewrite_static_define_property_eval_expression(
                    callee,
                    this_binding,
                    direct_eval_in_class_field_initializer,
                )),
                arguments: arguments
                    .iter()
                    .map(|argument| match argument {
                        CallArgument::Expression(expression) => CallArgument::Expression(
                            Self::rewrite_static_define_property_eval_expression(
                                expression,
                                this_binding,
                                direct_eval_in_class_field_initializer,
                            ),
                        ),
                        CallArgument::Spread(expression) => CallArgument::Spread(
                            Self::rewrite_static_define_property_eval_expression(
                                expression,
                                this_binding,
                                direct_eval_in_class_field_initializer,
                            ),
                        ),
                    })
                    .collect(),
            },
            Expression::New { callee, arguments } => Expression::New {
                callee: Box::new(Self::rewrite_static_define_property_eval_expression(
                    callee,
                    this_binding,
                    direct_eval_in_class_field_initializer,
                )),
                arguments: arguments
                    .iter()
                    .map(|argument| match argument {
                        CallArgument::Expression(expression) => CallArgument::Expression(
                            Self::rewrite_static_define_property_eval_expression(
                                expression,
                                this_binding,
                                direct_eval_in_class_field_initializer,
                            ),
                        ),
                        CallArgument::Spread(expression) => CallArgument::Spread(
                            Self::rewrite_static_define_property_eval_expression(
                                expression,
                                this_binding,
                                direct_eval_in_class_field_initializer,
                            ),
                        ),
                    })
                    .collect(),
            },
            Expression::Array(elements) => Expression::Array(
                elements
                    .iter()
                    .map(|element| match element {
                        ArrayElement::Expression(expression) => ArrayElement::Expression(
                            Self::rewrite_static_define_property_eval_expression(
                                expression,
                                this_binding,
                                direct_eval_in_class_field_initializer,
                            ),
                        ),
                        ArrayElement::Spread(expression) => ArrayElement::Spread(
                            Self::rewrite_static_define_property_eval_expression(
                                expression,
                                this_binding,
                                direct_eval_in_class_field_initializer,
                            ),
                        ),
                    })
                    .collect(),
            ),
            Expression::Object(entries) => Expression::Object(
                entries
                    .iter()
                    .map(|entry| match entry {
                        ObjectEntry::Data { key, value } => ObjectEntry::Data {
                            key: Self::rewrite_static_define_property_eval_expression(
                                key,
                                this_binding,
                                direct_eval_in_class_field_initializer,
                            ),
                            value: Self::rewrite_static_define_property_eval_expression(
                                value,
                                this_binding,
                                direct_eval_in_class_field_initializer,
                            ),
                        },
                        ObjectEntry::Getter { key, getter } => ObjectEntry::Getter {
                            key: Self::rewrite_static_define_property_eval_expression(
                                key,
                                this_binding,
                                direct_eval_in_class_field_initializer,
                            ),
                            getter: Self::rewrite_static_define_property_eval_expression(
                                getter,
                                this_binding,
                                direct_eval_in_class_field_initializer,
                            ),
                        },
                        ObjectEntry::Setter { key, setter } => ObjectEntry::Setter {
                            key: Self::rewrite_static_define_property_eval_expression(
                                key,
                                this_binding,
                                direct_eval_in_class_field_initializer,
                            ),
                            setter: Self::rewrite_static_define_property_eval_expression(
                                setter,
                                this_binding,
                                direct_eval_in_class_field_initializer,
                            ),
                        },
                        ObjectEntry::Spread(expression) => ObjectEntry::Spread(
                            Self::rewrite_static_define_property_eval_expression(
                                expression,
                                this_binding,
                                direct_eval_in_class_field_initializer,
                            ),
                        ),
                    })
                    .collect(),
            ),
            _ => expression.clone(),
        }
    }

    fn parse_define_property_eval_program_in_context(
        &self,
        source: &str,
        current_function_name: Option<&str>,
        direct_eval_in_class_field_initializer: bool,
    ) -> Option<Program> {
        if direct_eval_in_class_field_initializer
            && let Some(function_name) = current_function_name
            && let Some(program) = self
                .parse_eval_program_in_class_field_initializer_context_for_function(
                    function_name,
                    source,
                )
        {
            return Some(program);
        }

        if let Some(current_function_name) = current_function_name {
            if self
                .resolve_home_object_name_for_function(current_function_name)
                .is_some()
                && source.contains("super")
                && let Some(program) = self.parse_eval_program_in_method_context(source)
            {
                return Some(program);
            }
        }

        if current_function_name.is_some() || direct_eval_in_class_field_initializer {
            return self.parse_eval_program_in_ordinary_function_context(source);
        }

        None
    }
}
