use super::*;

impl DirectWasmCompiler {
    fn seeded_test262_realm_assignment_value(value: &Expression) -> Option<Expression> {
        let Expression::Identifier(name) = value else {
            return None;
        };
        (parse_test262_realm_identifier(name).is_some()
            || parse_test262_realm_global_identifier(name).is_some()
            || parse_test262_realm_eval_builtin(name).is_some())
        .then(|| value.clone())
    }

    fn expression_references_top_level_this_property(
        expression: &Expression,
        property_name: &str,
    ) -> bool {
        match expression {
            Expression::Array(elements) => elements.iter().any(|element| match element {
                ArrayElement::Expression(value) | ArrayElement::Spread(value) => {
                    Self::expression_references_top_level_this_property(value, property_name)
                }
            }),
            Expression::Object(entries) => entries.iter().any(|entry| match entry {
                ObjectEntry::Data { key, value } => {
                    Self::expression_references_top_level_this_property(key, property_name)
                        || Self::expression_references_top_level_this_property(value, property_name)
                }
                ObjectEntry::Getter { key, getter } => {
                    Self::expression_references_top_level_this_property(key, property_name)
                        || Self::expression_references_top_level_this_property(
                            getter,
                            property_name,
                        )
                }
                ObjectEntry::Setter { key, setter } => {
                    Self::expression_references_top_level_this_property(key, property_name)
                        || Self::expression_references_top_level_this_property(
                            setter,
                            property_name,
                        )
                }
                ObjectEntry::Spread(value) => {
                    Self::expression_references_top_level_this_property(value, property_name)
                }
            }),
            Expression::Member { object, property } => {
                let direct_match = matches!(object.as_ref(), Expression::This)
                    && matches!(property.as_ref(), Expression::String(name) if name == property_name);
                direct_match
                    || Self::expression_references_top_level_this_property(object, property_name)
                    || Self::expression_references_top_level_this_property(property, property_name)
            }
            Expression::SuperMember { property } => {
                Self::expression_references_top_level_this_property(property, property_name)
            }
            Expression::Assign { value, .. } => {
                Self::expression_references_top_level_this_property(value, property_name)
            }
            Expression::AssignMember {
                object,
                property,
                value,
            } => {
                Self::expression_references_top_level_this_property(object, property_name)
                    || Self::expression_references_top_level_this_property(property, property_name)
                    || Self::expression_references_top_level_this_property(value, property_name)
            }
            Expression::AssignSuperMember { property, value } => {
                Self::expression_references_top_level_this_property(property, property_name)
                    || Self::expression_references_top_level_this_property(value, property_name)
            }
            Expression::Await(value)
            | Expression::EnumerateKeys(value)
            | Expression::GetIterator(value)
            | Expression::IteratorClose(value) => {
                Self::expression_references_top_level_this_property(value, property_name)
            }
            Expression::Unary { expression, .. } => {
                Self::expression_references_top_level_this_property(expression, property_name)
            }
            Expression::Binary { left, right, .. } => {
                Self::expression_references_top_level_this_property(left, property_name)
                    || Self::expression_references_top_level_this_property(right, property_name)
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                Self::expression_references_top_level_this_property(condition, property_name)
                    || Self::expression_references_top_level_this_property(
                        then_expression,
                        property_name,
                    )
                    || Self::expression_references_top_level_this_property(
                        else_expression,
                        property_name,
                    )
            }
            Expression::Sequence(expressions) => expressions.iter().any(|expression| {
                Self::expression_references_top_level_this_property(expression, property_name)
            }),
            Expression::Call { callee, arguments }
            | Expression::SuperCall { callee, arguments }
            | Expression::New { callee, arguments } => {
                Self::expression_references_top_level_this_property(callee, property_name)
                    || arguments.iter().any(|argument| match argument {
                        CallArgument::Expression(value) | CallArgument::Spread(value) => {
                            Self::expression_references_top_level_this_property(
                                value,
                                property_name,
                            )
                        }
                    })
            }
            Expression::Number(_)
            | Expression::BigInt(_)
            | Expression::String(_)
            | Expression::Bool(_)
            | Expression::Null
            | Expression::Undefined
            | Expression::NewTarget
            | Expression::Identifier(_)
            | Expression::This
            | Expression::Sent
            | Expression::Update { .. } => false,
        }
    }

    pub(in crate::backend::direct_wasm) fn update_static_global_assignment_metadata(
        &mut self,
        name: &str,
        value: &Expression,
    ) {
        let seeded_realm_value = self
            .global_value_binding(name)
            .and_then(Self::seeded_test262_realm_assignment_value);
        let snapshot_value = seeded_realm_value.unwrap_or_else(|| {
            self.global_value_binding(name)
                .map(|snapshot| substitute_self_referential_binding_snapshot(value, name, snapshot))
                .unwrap_or_else(|| value.clone())
        });
        let mut referenced_names = HashSet::new();
        collect_referenced_binding_names_from_expression(&snapshot_value, &mut referenced_names);
        if referenced_names.contains(name) {
            self.clear_global_binding_state(name);
            return;
        }
        self.set_global_binding_kind(name, infer_global_expression_kind(&snapshot_value));
        let inferred_array_binding = self.infer_global_array_binding(&snapshot_value);
        let inferred_object_binding = self.infer_global_object_binding(&snapshot_value);
        let inferred_arguments_binding = self.infer_global_arguments_binding(&snapshot_value);
        let inferred_function_binding = self.infer_global_function_binding(&snapshot_value);
        let preserve_private_brand_identifier = matches!(&snapshot_value, Expression::Identifier(name) if name.contains("__ayy_class_brand_"));
        let preserve_reference_alias = preserve_private_brand_identifier
            || (matches!(snapshot_value, Expression::Identifier(_))
                && (inferred_array_binding.is_some()
                    || inferred_object_binding.is_some()
                    || inferred_function_binding.is_some()));
        let materialized_value = if preserve_reference_alias {
            snapshot_value.clone()
        } else {
            self.materialize_global_expression(&snapshot_value)
        };
        self.set_global_expression_binding(name, materialized_value);
        self.sync_global_array_binding(name, inferred_array_binding);
        self.sync_global_object_binding(name, inferred_object_binding);
        self.sync_global_arguments_binding(name, inferred_arguments_binding);
        self.sync_global_function_binding(name, inferred_function_binding);
        let materialized_snapshot = self.materialize_global_expression(&snapshot_value);
        let member_binding_alias_source = match &snapshot_value {
            Expression::Identifier(source_name)
                if self.has_global_member_bindings_for_name(source_name)
                    || self
                        .global_object_prototype_expression(source_name)
                        .is_some()
                    || self
                        .global_object_prototype_expression(&format!("{source_name}.prototype"))
                        .is_some() =>
            {
                Some(source_name)
            }
            _ => match &materialized_snapshot {
                Expression::Identifier(source_name) => Some(source_name),
                _ => None,
            },
        };
        if let Some(source_name) = member_binding_alias_source {
            self.copy_global_member_bindings_for_alias(name, source_name);
        } else {
            let preserved_capture_slots =
                self.global_member_capture_slots_by_property_for_name(name);
            let inherited_member_bindings =
                self.global_inherited_member_function_bindings(&snapshot_value);
            let inherited_getter_bindings =
                self.global_inherited_member_getter_bindings(&snapshot_value);
            if inherited_member_bindings.is_empty() && inherited_getter_bindings.is_empty() {
                if !self.has_global_member_bindings_for_name(name) {
                    self.update_global_object_literal_member_bindings_for_value(
                        name,
                        &snapshot_value,
                    );
                }
            } else {
                self.clear_global_member_bindings_for_name(name);
                for binding in inherited_member_bindings {
                    self.insert_global_inherited_member_function_binding_for_name(
                        name,
                        binding,
                        &preserved_capture_slots,
                    );
                }
                for binding in inherited_getter_bindings {
                    self.insert_global_inherited_member_getter_binding_for_name(
                        name,
                        binding,
                        &preserved_capture_slots,
                    );
                }
            }
        }
        self.update_global_object_literal_home_bindings(name, &snapshot_value);
        self.update_global_object_prototype_binding_from_value(name, &snapshot_value);
    }

    pub(in crate::backend::direct_wasm) fn update_global_member_assignment_metadata(
        &mut self,
        object: &Expression,
        property: &Expression,
        value: &Expression,
    ) {
        if self.global_property_key_requires_runtime_coercion(property) {
            self.invalidate_dynamic_global_member_assignment_target(object);
            return;
        }

        let materialized_property = self.materialize_global_expression(property);
        let materialized_value = self.materialize_global_expression(value);
        if let Expression::Identifier(name) = object
            && let Some(index) =
                argument_index_from_expression(&materialized_property).map(|index| index as usize)
            && self.set_global_array_element_binding(name, index, materialized_value.clone())
        {
        }
        if let Expression::Identifier(name) = object
            && matches!(&materialized_property, Expression::String(property_name) if property_name == "prototype")
            && let Some(prototype) = self.prototype_assignment_parent_expression(value)
        {
            self.update_global_object_prototype_binding(&format!("{name}.prototype"), &prototype);
        }
        match object {
            Expression::Identifier(name) if self.global_has_binding(name) => {
                self.define_global_object_property(
                    name,
                    materialized_property.clone(),
                    materialized_value.clone(),
                    true,
                );
            }
            Expression::This => {
                self.define_global_object_property(
                    "this",
                    materialized_property.clone(),
                    materialized_value.clone(),
                    true,
                );
                if let Expression::String(name) = &materialized_property
                    && !name.starts_with("__ayy")
                {
                    self.ensure_implicit_global_binding(name);
                    if Self::expression_references_top_level_this_property(
                        &materialized_value,
                        name,
                    ) || Self::expression_references_top_level_this_property(value, name)
                    {
                        self.clear_global_binding_state(name);
                    } else {
                        self.update_static_global_assignment_metadata(name, &materialized_value);
                    }
                }
            }
            Expression::Member {
                object: prototype_object,
                property: target_property,
            } if matches!(target_property.as_ref(), Expression::String(name) if name == "prototype") =>
            {
                let Expression::Identifier(name) = prototype_object.as_ref() else {
                    return;
                };
                self.define_global_prototype_object_property(
                    name,
                    materialized_property.clone(),
                    materialized_value.clone(),
                    true,
                );
            }
            _ => {}
        }

        let Some(key) = self.global_member_function_binding_key(object, property) else {
            return;
        };
        if let Some(binding) = self.infer_global_function_binding(value) {
            self.set_global_member_function_binding(key.clone(), binding);
        } else {
            self.clear_global_member_function_binding(&key);
        }
        self.clear_global_member_getter_binding(&key);
        self.clear_global_member_setter_binding(&key);
    }

    fn invalidate_dynamic_global_member_assignment_target(&mut self, object: &Expression) {
        match object {
            Expression::Identifier(name) if self.global_has_binding(name) => {
                self.clear_global_binding_state(name);
                self.clear_global_member_bindings_for_name(name);
            }
            Expression::This => {
                self.clear_global_binding_state("this");
                self.clear_global_member_bindings_for_name("this");
            }
            Expression::Member {
                object: prototype_object,
                property: target_property,
            } if matches!(target_property.as_ref(), Expression::String(name) if name == "prototype") => {
                if let Expression::Identifier(name) = prototype_object.as_ref() {
                    self.state.sync_global_prototype_object_binding(name, None);
                    self.clear_global_member_bindings_for_name(name);
                }
            }
            _ => {}
        }
    }
}
