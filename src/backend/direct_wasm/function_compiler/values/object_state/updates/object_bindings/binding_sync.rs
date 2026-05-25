use super::super::super::*;

impl<'a> FunctionCompiler<'a> {
    fn rewrite_static_new_this_call_argument(&self, argument: &CallArgument) -> CallArgument {
        match argument {
            CallArgument::Expression(expression) => {
                CallArgument::Expression(self.rewrite_static_new_this_expression(expression))
            }
            CallArgument::Spread(expression) => {
                CallArgument::Spread(self.rewrite_static_new_this_expression(expression))
            }
        }
    }

    fn rewrite_static_new_this_array_element(&self, element: &ArrayElement) -> ArrayElement {
        match element {
            ArrayElement::Expression(expression) => {
                ArrayElement::Expression(self.rewrite_static_new_this_expression(expression))
            }
            ArrayElement::Spread(expression) => {
                ArrayElement::Spread(self.rewrite_static_new_this_expression(expression))
            }
        }
    }

    fn rewrite_static_new_this_object_entry(&self, entry: &ObjectEntry) -> ObjectEntry {
        match entry {
            ObjectEntry::Data { key, value } => ObjectEntry::Data {
                key: self.rewrite_static_new_this_expression(key),
                value: self.rewrite_static_new_this_expression(value),
            },
            ObjectEntry::Getter { key, getter } => ObjectEntry::Getter {
                key: self.rewrite_static_new_this_expression(key),
                getter: self.rewrite_static_new_this_expression(getter),
            },
            ObjectEntry::Setter { key, setter } => ObjectEntry::Setter {
                key: self.rewrite_static_new_this_expression(key),
                setter: self.rewrite_static_new_this_expression(setter),
            },
            ObjectEntry::Spread(expression) => {
                ObjectEntry::Spread(self.rewrite_static_new_this_expression(expression))
            }
        }
    }

    pub(in crate::backend::direct_wasm) fn rewrite_static_new_this_expression(
        &self,
        expression: &Expression,
    ) -> Expression {
        self.rewrite_static_new_this_expression_with_replacement(expression, &Expression::This)
    }

    fn rewrite_static_new_this_expression_with_replacement(
        &self,
        expression: &Expression,
        replacement: &Expression,
    ) -> Expression {
        match expression {
            Expression::Identifier(name) if name == Self::STATIC_NEW_THIS_BINDING => {
                replacement.clone()
            }
            Expression::Array(elements) => Expression::Array(
                elements
                    .iter()
                    .map(|element| match element {
                        ArrayElement::Expression(expression) => ArrayElement::Expression(
                            self.rewrite_static_new_this_expression_with_replacement(
                                expression,
                                replacement,
                            ),
                        ),
                        ArrayElement::Spread(expression) => ArrayElement::Spread(
                            self.rewrite_static_new_this_expression_with_replacement(
                                expression,
                                replacement,
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
                            key: self.rewrite_static_new_this_expression_with_replacement(
                                key,
                                replacement,
                            ),
                            value: self.rewrite_static_new_this_expression_with_replacement(
                                value,
                                replacement,
                            ),
                        },
                        ObjectEntry::Getter { key, getter } => ObjectEntry::Getter {
                            key: self.rewrite_static_new_this_expression_with_replacement(
                                key,
                                replacement,
                            ),
                            getter: self.rewrite_static_new_this_expression_with_replacement(
                                getter,
                                replacement,
                            ),
                        },
                        ObjectEntry::Setter { key, setter } => ObjectEntry::Setter {
                            key: self.rewrite_static_new_this_expression_with_replacement(
                                key,
                                replacement,
                            ),
                            setter: self.rewrite_static_new_this_expression_with_replacement(
                                setter,
                                replacement,
                            ),
                        },
                        ObjectEntry::Spread(expression) => ObjectEntry::Spread(
                            self.rewrite_static_new_this_expression_with_replacement(
                                expression,
                                replacement,
                            ),
                        ),
                    })
                    .collect(),
            ),
            Expression::Member { object, property } => Expression::Member {
                object: Box::new(
                    self.rewrite_static_new_this_expression_with_replacement(object, replacement),
                ),
                property: Box::new(
                    self.rewrite_static_new_this_expression_with_replacement(property, replacement),
                ),
            },
            Expression::SuperMember { property } => Expression::SuperMember {
                property: Box::new(
                    self.rewrite_static_new_this_expression_with_replacement(property, replacement),
                ),
            },
            Expression::Assign { name, value } => Expression::Assign {
                name: name.clone(),
                value: Box::new(
                    self.rewrite_static_new_this_expression_with_replacement(value, replacement),
                ),
            },
            Expression::AssignMember {
                object,
                property,
                value,
            } => Expression::AssignMember {
                object: Box::new(
                    self.rewrite_static_new_this_expression_with_replacement(object, replacement),
                ),
                property: Box::new(
                    self.rewrite_static_new_this_expression_with_replacement(property, replacement),
                ),
                value: Box::new(
                    self.rewrite_static_new_this_expression_with_replacement(value, replacement),
                ),
            },
            Expression::AssignSuperMember { property, value } => Expression::AssignSuperMember {
                property: Box::new(
                    self.rewrite_static_new_this_expression_with_replacement(property, replacement),
                ),
                value: Box::new(
                    self.rewrite_static_new_this_expression_with_replacement(value, replacement),
                ),
            },
            Expression::Await(expression) => Expression::Await(Box::new(
                self.rewrite_static_new_this_expression_with_replacement(expression, replacement),
            )),
            Expression::EnumerateKeys(expression) => Expression::EnumerateKeys(Box::new(
                self.rewrite_static_new_this_expression_with_replacement(expression, replacement),
            )),
            Expression::GetIterator(expression) => Expression::GetIterator(Box::new(
                self.rewrite_static_new_this_expression_with_replacement(expression, replacement),
            )),
            Expression::IteratorClose(expression) => Expression::IteratorClose(Box::new(
                self.rewrite_static_new_this_expression_with_replacement(expression, replacement),
            )),
            Expression::Unary { op, expression } => {
                Expression::Unary {
                    op: *op,
                    expression: Box::new(self.rewrite_static_new_this_expression_with_replacement(
                        expression,
                        replacement,
                    )),
                }
            }
            Expression::Binary { op, left, right } => Expression::Binary {
                op: *op,
                left: Box::new(
                    self.rewrite_static_new_this_expression_with_replacement(left, replacement),
                ),
                right: Box::new(
                    self.rewrite_static_new_this_expression_with_replacement(right, replacement),
                ),
            },
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                Expression::Conditional {
                    condition: Box::new(self.rewrite_static_new_this_expression_with_replacement(
                        condition,
                        replacement,
                    )),
                    then_expression: Box::new(
                        self.rewrite_static_new_this_expression_with_replacement(
                            then_expression,
                            replacement,
                        ),
                    ),
                    else_expression: Box::new(
                        self.rewrite_static_new_this_expression_with_replacement(
                            else_expression,
                            replacement,
                        ),
                    ),
                }
            }
            Expression::Sequence(expressions) => Expression::Sequence(
                expressions
                    .iter()
                    .map(|expression| {
                        self.rewrite_static_new_this_expression_with_replacement(
                            expression,
                            replacement,
                        )
                    })
                    .collect(),
            ),
            Expression::Call { callee, arguments } => Expression::Call {
                callee: Box::new(
                    self.rewrite_static_new_this_expression_with_replacement(callee, replacement),
                ),
                arguments: arguments
                    .iter()
                    .map(|argument| match argument {
                        CallArgument::Expression(expression) => CallArgument::Expression(
                            self.rewrite_static_new_this_expression_with_replacement(
                                expression,
                                replacement,
                            ),
                        ),
                        CallArgument::Spread(expression) => CallArgument::Spread(
                            self.rewrite_static_new_this_expression_with_replacement(
                                expression,
                                replacement,
                            ),
                        ),
                    })
                    .collect(),
            },
            Expression::SuperCall { callee, arguments } => Expression::SuperCall {
                callee: Box::new(
                    self.rewrite_static_new_this_expression_with_replacement(callee, replacement),
                ),
                arguments: arguments
                    .iter()
                    .map(|argument| match argument {
                        CallArgument::Expression(expression) => CallArgument::Expression(
                            self.rewrite_static_new_this_expression_with_replacement(
                                expression,
                                replacement,
                            ),
                        ),
                        CallArgument::Spread(expression) => CallArgument::Spread(
                            self.rewrite_static_new_this_expression_with_replacement(
                                expression,
                                replacement,
                            ),
                        ),
                    })
                    .collect(),
            },
            Expression::New { callee, arguments } => Expression::New {
                callee: Box::new(
                    self.rewrite_static_new_this_expression_with_replacement(callee, replacement),
                ),
                arguments: arguments
                    .iter()
                    .map(|argument| match argument {
                        CallArgument::Expression(expression) => CallArgument::Expression(
                            self.rewrite_static_new_this_expression_with_replacement(
                                expression,
                                replacement,
                            ),
                        ),
                        CallArgument::Spread(expression) => CallArgument::Spread(
                            self.rewrite_static_new_this_expression_with_replacement(
                                expression,
                                replacement,
                            ),
                        ),
                    })
                    .collect(),
            },
            _ => expression.clone(),
        }
    }

    pub(in crate::backend::direct_wasm) fn rewrite_static_new_this_expression_for_owner(
        &self,
        expression: &Expression,
        owner_name: &str,
    ) -> Expression {
        let replacement = if owner_name == "this" {
            Expression::This
        } else {
            Expression::Identifier(owner_name.to_string())
        };
        self.rewrite_static_new_this_expression_with_replacement(expression, &replacement)
    }

    pub(in crate::backend::direct_wasm) fn rewrite_static_new_this_object_binding(
        &self,
        object_binding: &ObjectValueBinding,
    ) -> ObjectValueBinding {
        self.rewrite_static_new_this_object_binding_with_replacement(
            object_binding,
            &Expression::This,
        )
    }

    fn rewrite_static_new_this_object_binding_with_replacement(
        &self,
        object_binding: &ObjectValueBinding,
        replacement: &Expression,
    ) -> ObjectValueBinding {
        ObjectValueBinding {
            string_properties: object_binding
                .string_properties
                .iter()
                .map(|(name, value)| {
                    (
                        name.clone(),
                        self.rewrite_static_new_this_expression_with_replacement(
                            value,
                            replacement,
                        ),
                    )
                })
                .collect(),
            symbol_properties: object_binding
                .symbol_properties
                .iter()
                .map(|(key, value)| {
                    (
                        self.rewrite_static_new_this_expression_with_replacement(key, replacement),
                        self.rewrite_static_new_this_expression_with_replacement(
                            value,
                            replacement,
                        ),
                    )
                })
                .collect(),
            property_descriptors: object_binding
                .property_descriptors
                .iter()
                .map(|(property, descriptor)| {
                    let mut descriptor = descriptor.clone();
                    descriptor.value = descriptor.value.map(|value| {
                        self.rewrite_static_new_this_expression_with_replacement(
                            &value,
                            replacement,
                        )
                    });
                    descriptor.getter = descriptor.getter.map(|value| {
                        self.rewrite_static_new_this_expression_with_replacement(
                            &value,
                            replacement,
                        )
                    });
                    descriptor.setter = descriptor.setter.map(|value| {
                        self.rewrite_static_new_this_expression_with_replacement(
                            &value,
                            replacement,
                        )
                    });
                    (
                        self.rewrite_static_new_this_expression_with_replacement(
                            property,
                            replacement,
                        ),
                        descriptor,
                    )
                })
                .collect(),
            non_enumerable_string_properties: object_binding
                .non_enumerable_string_properties
                .clone(),
            runtime_symbol_properties: object_binding.runtime_symbol_properties,
            extensible: object_binding.extensible,
        }
    }

    pub(in crate::backend::direct_wasm) fn rewrite_static_new_this_object_binding_for_owner(
        &self,
        object_binding: &ObjectValueBinding,
        owner_name: &str,
    ) -> ObjectValueBinding {
        let replacement = if owner_name == "this" {
            Expression::This
        } else {
            Expression::Identifier(owner_name.to_string())
        };
        self.rewrite_static_new_this_object_binding_with_replacement(object_binding, &replacement)
    }

    pub(in crate::backend::direct_wasm) fn rewrite_static_new_this_object_binding_for_expression(
        &self,
        object_binding: &ObjectValueBinding,
        replacement: &Expression,
    ) -> ObjectValueBinding {
        self.rewrite_static_new_this_object_binding_with_replacement(object_binding, replacement)
    }

    pub(in crate::backend::direct_wasm) fn seed_local_this_object_binding(&mut self) {
        if self
            .state
            .speculation
            .static_semantics
            .has_local_object_binding("this")
        {
            self.state
                .speculation
                .static_semantics
                .set_local_kind("this", StaticValueKind::Object);
            return;
        }

        if let Some(this_binding) = self.resolve_object_binding_from_expression(&Expression::This) {
            self.state
                .speculation
                .static_semantics
                .set_local_object_binding(
                    "this",
                    self.rewrite_static_new_this_object_binding(&this_binding),
                );
            self.state
                .speculation
                .static_semantics
                .set_local_kind("this", StaticValueKind::Object);
            return;
        }

        self.state
            .speculation
            .static_semantics
            .ensure_local_object_binding("this");
        self.state
            .speculation
            .static_semantics
            .set_local_kind("this", StaticValueKind::Object);
    }

    pub(in crate::backend::direct_wasm) fn update_object_prototype_binding_from_value(
        &mut self,
        name: &str,
        value: &Expression,
    ) {
        let prototype_from_value = |compiler: &Self, expression: &Expression| {
            object_literal_prototype_expression(expression)
                .or_else(|| {
                    let Expression::New { callee, .. } = expression else {
                        return None;
                    };
                    let constructor_expression = compiler
                        .resolve_function_prototype_bind_call(
                            callee,
                            compiler.current_function_name(),
                        )
                        .map(|(target, _, _)| target)
                        .unwrap_or_else(|| callee.as_ref().clone());
                    let constructor_name = match &constructor_expression {
                        Expression::Identifier(constructor_name) => Some(constructor_name.clone()),
                        _ => compiler
                            .resolve_function_binding_from_expression(&constructor_expression)
                            .and_then(|binding| {
                                compiler.function_prototype_binding_owner_name(&binding)
                            }),
                    }?;
                    Some(Expression::Member {
                        object: Box::new(Expression::Identifier(constructor_name)),
                        property: Box::new(Expression::String("prototype".to_string())),
                    })
                })
                .or_else(|| {
                    let Expression::Call { .. } = expression else {
                        return None;
                    };
                    compiler.resolve_static_object_prototype_expression(expression)
                })
        };
        let prototype = prototype_from_value(self, value).or_else(|| {
            let materialized = self.materialize_static_expression(value);
            if static_expression_matches(&materialized, value) {
                return None;
            }
            prototype_from_value(self, &materialized)
        });
        if std::env::var_os("AYY_TRACE_OBJECT_PROTOTYPES").is_some() {
            eprintln!(
                "object_prototype_update name={name} value={value:?} prototype={prototype:?}"
            );
        }

        self.backend
            .sync_global_object_prototype_expression(name, prototype);
    }

    fn resolve_private_brand_capture_slot_for_binding_name(&self, name: &str) -> Option<String> {
        let local_capture_slots = self
            .state
            .speculation
            .static_semantics
            .objects
            .member_function_capture_slots
            .iter()
            .map(|(key, capture_slots)| (key.clone(), capture_slots.clone()));
        let global_capture_slots = self.backend.global_member_function_capture_slot_entries();
        local_capture_slots
            .chain(global_capture_slots)
            .find_map(|(key, capture_slots)| match key.target {
                MemberFunctionBindingTarget::Identifier(target) if target == name => capture_slots
                    .into_iter()
                    .find_map(|(capture_name, slot_name)| {
                        capture_name
                            .starts_with("__ayy_class_brand_")
                            .then_some(slot_name)
                    }),
                _ => None,
            })
    }

    fn rewrite_private_member_markers_for_binding_name(
        &self,
        name: &str,
        object_binding: &mut ObjectValueBinding,
    ) {
        let Some(slot_name) = self.resolve_private_brand_capture_slot_for_binding_name(name) else {
            return;
        };
        for property_name in ordered_object_property_names(object_binding) {
            if !property_name.starts_with("__ayy$private$") {
                continue;
            }
            let Some(marker_property) =
                private_brand_marker_property_expression(&Expression::String(property_name))
            else {
                continue;
            };
            if matches!(
                object_binding_lookup_value(object_binding, &marker_property),
                Some(Expression::Bool(true))
            ) {
                continue;
            }
            object_binding_define_property(
                object_binding,
                marker_property,
                Expression::Identifier(slot_name.clone()),
                false,
            );
        }
    }

    pub(in crate::backend::direct_wasm) fn update_local_object_binding(
        &mut self,
        name: &str,
        value: &Expression,
    ) {
        if matches!(value, Expression::Identifier(source_name) if source_name == name)
            && self.resolve_current_local_binding(name).is_none()
            && (self.global_has_binding(name)
                || self.backend.global_has_lexical_binding(name)
                || self.global_has_implicit_binding(name))
        {
            return;
        }

        let Some(object_binding) = self.resolve_object_binding_from_expression(value) else {
            self.state
                .speculation
                .static_semantics
                .clear_local_object_binding(name);
            if self.binding_name_is_global(name) {
                self.backend.sync_global_object_binding(name, None);
            }
            return;
        };
        self.update_local_object_binding_from_resolved(name, value, object_binding);
    }

    pub(in crate::backend::direct_wasm) fn update_local_object_binding_from_resolved(
        &mut self,
        name: &str,
        value: &Expression,
        mut object_binding: ObjectValueBinding,
    ) {
        let resolved_class_instance = matches!(
            value,
            Expression::New { callee, .. }
                if matches!(
                    callee.as_ref(),
                    Expression::Identifier(function_name)
                        if function_name.starts_with("__ayy_class_ctor_")
                )
        );
        if !resolved_class_instance {
            self.seed_boxed_primitive_value_property(value, &mut object_binding);
            self.seed_date_value_property(value, &mut object_binding);
            self.seed_native_error_object_binding(value, &mut object_binding);
            self.seed_constructed_function_object_binding(value, &mut object_binding);
        }
        self.rewrite_private_member_markers_for_binding_name(name, &mut object_binding);
        self.state
            .speculation
            .static_semantics
            .set_local_object_binding(name, object_binding.clone());
        if self.binding_name_is_global(name) {
            self.backend
                .sync_global_object_binding(name, Some(object_binding));
            let kind = if resolved_class_instance {
                StaticValueKind::Object
            } else if self
                .resolve_function_binding_from_expression(&Expression::Identifier(name.to_string()))
                .is_some()
            {
                StaticValueKind::Function
            } else {
                StaticValueKind::Object
            };
            self.backend.set_global_binding_kind(name, kind);
        }
        self.state
            .speculation
            .static_semantics
            .set_local_kind(name, StaticValueKind::Object);
    }

    fn prototype_member_references_owner(expression: &Expression, owner_name: &str) -> bool {
        matches!(
            expression,
            Expression::Member { object, property }
                if matches!(object.as_ref(), Expression::Identifier(name) if name == owner_name)
                    && matches!(property.as_ref(), Expression::String(name) if name == "prototype")
        )
    }

    fn snapshot_existing_object_prototype_references(
        &mut self,
        owner_name: &str,
        old_binding: &ObjectValueBinding,
    ) {
        let referencing_object_names = self
            .backend
            .global_semantics
            .values
            .object_prototype_bindings
            .iter()
            .filter_map(|(object_name, prototype)| {
                Self::prototype_member_references_owner(prototype, owner_name)
                    .then(|| object_name.clone())
            })
            .collect::<Vec<_>>();

        for object_name in referencing_object_names {
            let snapshot_owner = format!("__ayy_prototype_snapshot_{owner_name}_{object_name}");
            let snapshot_prototype = Expression::Member {
                object: Box::new(Expression::Identifier(snapshot_owner.clone())),
                property: Box::new(Expression::String("prototype".to_string())),
            };
            self.state
                .speculation
                .static_semantics
                .objects
                .local_prototype_object_bindings
                .insert(snapshot_owner.clone(), old_binding.clone());
            self.backend
                .sync_global_prototype_object_binding(&snapshot_owner, Some(old_binding.clone()));
            self.backend
                .sync_global_object_prototype_expression(&object_name, Some(snapshot_prototype));
        }
    }

    pub(in crate::backend::direct_wasm) fn update_prototype_object_binding(
        &mut self,
        name: &str,
        value: &Expression,
    ) {
        let old_binding = self
            .state
            .speculation
            .static_semantics
            .objects
            .local_prototype_object_bindings
            .get(name)
            .cloned()
            .or_else(|| {
                self.backend
                    .global_semantics
                    .values
                    .prototype_object_bindings
                    .get(name)
                    .cloned()
            })
            .or_else(|| {
                self.resolve_function_binding_from_expression(&Expression::Identifier(
                    name.to_string(),
                ))
                .and_then(|binding| self.default_function_prototype_object_binding(&binding))
            });
        if let Some(old_binding) = old_binding.as_ref() {
            self.snapshot_existing_object_prototype_references(name, old_binding);
        }

        let object_binding = match value {
            Expression::Identifier(value_name) => {
                let resolved_name = self
                    .resolve_current_local_binding(value_name)
                    .map(|(resolved_name, _)| resolved_name)
                    .unwrap_or_else(|| value_name.clone());
                self.state
                    .speculation
                    .static_semantics
                    .local_object_binding(&resolved_name)
                    .cloned()
                    .or_else(|| self.global_object_binding(value_name).cloned())
                    .or_else(|| {
                        (resolved_name != *value_name)
                            .then(|| self.global_object_binding(&resolved_name).cloned())
                            .flatten()
                    })
                    .or_else(|| self.resolve_runtime_shadow_object_binding(value_name))
            }
            _ => self.resolve_object_binding_from_expression(value),
        };
        let Some(object_binding) = object_binding else {
            self.state
                .speculation
                .static_semantics
                .objects
                .local_prototype_object_bindings
                .remove(name);
            if self.binding_name_is_global(name) {
                self.backend
                    .sync_global_prototype_object_binding(name, None);
            }
            return;
        };
        self.state
            .speculation
            .static_semantics
            .objects
            .local_prototype_object_bindings
            .insert(name.to_string(), object_binding.clone());
        if self.binding_name_is_global(name) {
            self.backend
                .sync_global_prototype_object_binding(name, Some(object_binding));
        }
    }
}
