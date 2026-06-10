use super::*;

impl<'a> FunctionCompiler<'a> {
    fn emit_property_descriptor_binding_member_value(
        &mut self,
        descriptor: &PropertyDescriptorBinding,
        property_name: &str,
    ) -> DirectResult<bool> {
        match property_name {
            "value" => {
                if let Some(value) = descriptor.value.clone() {
                    self.emit_numeric_expression(&value)?;
                } else {
                    self.push_i32_const(JS_UNDEFINED_TAG);
                }
                Ok(true)
            }
            "configurable" => {
                self.push_i32_const(if descriptor.configurable { 1 } else { 0 });
                Ok(true)
            }
            "enumerable" => {
                self.push_i32_const(if descriptor.enumerable { 1 } else { 0 });
                Ok(true)
            }
            "writable" => {
                if let Some(writable) = descriptor.writable {
                    self.push_i32_const(if writable { 1 } else { 0 });
                } else {
                    self.push_i32_const(JS_UNDEFINED_TAG);
                }
                Ok(true)
            }
            "get" => {
                if let Some(getter) = descriptor.getter.clone() {
                    self.emit_numeric_expression(&getter)?;
                } else {
                    self.push_i32_const(JS_UNDEFINED_TAG);
                }
                Ok(true)
            }
            "set" => {
                if let Some(setter) = descriptor.setter.clone() {
                    self.emit_numeric_expression(&setter)?;
                } else {
                    self.push_i32_const(JS_UNDEFINED_TAG);
                }
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    fn emit_runtime_dynamic_property_descriptor_result_member_read(
        &mut self,
        name: &str,
        property_name: &str,
    ) -> DirectResult<bool> {
        let trace = crate::ayy_env_flag!("AYY_TRACE_DESCRIPTOR_READS");
        let Some((receiver, descriptor_property)) =
            self.dynamic_property_descriptor_source_for_local(name)
        else {
            return Ok(false);
        };
        if trace {
            eprintln!(
                "descriptor_read:dynamic_source local={name} receiver={receiver:?} descriptor_property={descriptor_property:?} member={property_name}"
            );
        }

        if self.emit_module_namespace_dynamic_descriptor_member_read(
            &receiver,
            &descriptor_property,
            property_name,
        )? {
            return Ok(true);
        }

        let resolved_receiver = self
            .resolve_bound_alias_expression(&receiver)
            .filter(|resolved| !static_expression_matches(resolved, &receiver));
        let materialized_receiver = self.materialize_static_expression(&receiver);
        let receiver_candidates = [
            Some(&receiver),
            resolved_receiver.as_ref(),
            (!static_expression_matches(&materialized_receiver, &receiver))
                .then_some(&materialized_receiver),
        ];
        let Some(object_binding) =
            receiver_candidates
                .into_iter()
                .flatten()
                .find_map(|candidate| {
                    self.resolve_object_binding_from_expression(candidate)
                        .or_else(|| match candidate {
                            Expression::Identifier(name) => self
                                .resolve_identifier_object_binding_fallback(name)
                                .or_else(|| self.resolve_runtime_shadow_object_binding(name)),
                            Expression::This => self.resolve_runtime_shadow_object_binding("this"),
                            _ => None,
                        })
                })
        else {
            return Ok(false);
        };

        let descriptors = Self::dynamic_string_descriptor_property_names(&object_binding)
            .into_iter()
            .filter_map(|descriptor_name| {
                self.dynamic_string_property_descriptor_binding(
                    &receiver,
                    resolved_receiver.as_ref(),
                    &materialized_receiver,
                    &descriptor_name,
                )
                .map(|descriptor| (descriptor_name, descriptor))
            })
            .collect::<Vec<_>>();
        if descriptors.is_empty() {
            return Ok(false);
        }

        let descriptor_property_local = self.allocate_temp_local();
        let result_local = self.allocate_temp_local();
        self.emit_numeric_expression(&descriptor_property)?;
        self.push_local_set(descriptor_property_local);
        self.push_i32_const(JS_UNDEFINED_TAG);
        self.push_local_set(result_local);

        for (descriptor_name, descriptor) in descriptors {
            self.push_local_get(descriptor_property_local);
            self.emit_static_string_literal(&descriptor_name)?;
            self.push_binary_op(BinaryOp::Equal)?;
            self.state.emission.output.instructions.push(0x04);
            self.state
                .emission
                .output
                .instructions
                .push(EMPTY_BLOCK_TYPE);
            self.push_control_frame();
            if self.emit_property_descriptor_binding_member_value(&descriptor, property_name)? {
                self.push_local_set(result_local);
            }
            self.state.emission.output.instructions.push(0x0b);
            self.pop_control_frame();
        }

        self.push_local_get(result_local);
        Ok(true)
    }

    fn emit_module_namespace_dynamic_descriptor_member_read(
        &mut self,
        receiver: &Expression,
        descriptor_property: &Expression,
        property_name: &str,
    ) -> DirectResult<bool> {
        if !matches!(
            property_name,
            "value" | "configurable" | "enumerable" | "writable" | "get" | "set"
        ) {
            return Ok(false);
        }
        let Some(module_index) = self.module_namespace_index_from_expression(receiver) else {
            return Ok(false);
        };
        let materialized_property = self
            .resolve_property_key_expression(descriptor_property)
            .unwrap_or_else(|| self.materialize_static_expression(descriptor_property));
        if static_property_name_from_expression(&materialized_property).is_some()
            || is_symbol_to_string_tag_expression(&materialized_property)
        {
            if let Some(descriptor) = self.module_namespace_current_descriptor_from_module_index(
                receiver,
                module_index,
                &materialized_property,
            ) {
                return self
                    .emit_property_descriptor_binding_member_value(&descriptor, property_name);
            }
            self.push_i32_const(JS_UNDEFINED_TAG);
            return Ok(true);
        }
        if !inline_summary_side_effect_free_expression(descriptor_property) {
            return Ok(false);
        }

        let mut descriptors = self
            .resolve_static_dynamic_import_namespace_own_property_names_binding(module_index)
            .map(|binding| {
                binding
                    .values
                    .into_iter()
                    .filter_map(|value| {
                        let Some(Expression::String(property_name)) = value else {
                            return None;
                        };
                        if property_name.starts_with("__ayy$") || property_name == "then" {
                            return None;
                        }
                        let property = Expression::String(property_name);
                        self.module_namespace_current_descriptor_from_module_index(
                            receiver,
                            module_index,
                            &property,
                        )
                        .map(|descriptor| (property, descriptor))
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let to_string_tag_property = Expression::Member {
            object: Box::new(Expression::Identifier("Symbol".to_string())),
            property: Box::new(Expression::String("toStringTag".to_string())),
        };
        if let Some(descriptor) = self.module_namespace_current_descriptor_from_module_index(
            receiver,
            module_index,
            &to_string_tag_property,
        ) {
            descriptors.push((to_string_tag_property, descriptor));
        }
        if descriptors.is_empty() {
            self.push_i32_const(JS_UNDEFINED_TAG);
            return Ok(true);
        }

        let descriptor_property_local = self.allocate_temp_local();
        let result_local = self.allocate_temp_local();
        self.emit_numeric_expression(descriptor_property)?;
        self.push_local_set(descriptor_property_local);
        self.push_i32_const(JS_UNDEFINED_TAG);
        self.push_local_set(result_local);

        for (descriptor_name, descriptor) in descriptors {
            self.push_local_get(descriptor_property_local);
            if let Expression::String(property_name) = &descriptor_name {
                self.emit_static_string_literal(property_name)?;
            } else {
                self.emit_numeric_expression(&descriptor_name)?;
            }
            self.push_binary_op(BinaryOp::Equal)?;
            self.state.emission.output.instructions.push(0x04);
            self.state
                .emission
                .output
                .instructions
                .push(EMPTY_BLOCK_TYPE);
            self.push_control_frame();
            if self.emit_property_descriptor_binding_member_value(&descriptor, property_name)? {
                self.push_local_set(result_local);
            }
            self.state.emission.output.instructions.push(0x0b);
            self.pop_control_frame();
        }

        self.push_local_get(result_local);
        Ok(true)
    }

    pub(in crate::backend::direct_wasm) fn emit_runtime_descriptor_member_read(
        &mut self,
        object: &Expression,
        property: &Expression,
    ) -> DirectResult<bool> {
        let Expression::Identifier(name) = object else {
            return Ok(false);
        };
        let Expression::String(property_name) = property else {
            return Ok(false);
        };
        if matches!(
            property_name.as_str(),
            "value" | "configurable" | "enumerable" | "writable" | "get" | "set"
        ) && self
            .emit_runtime_dynamic_property_descriptor_result_member_read(name, property_name)?
        {
            return Ok(true);
        }

        let resolved_name = self
            .resolve_current_local_binding(name)
            .map(|(resolved_name, _)| resolved_name)
            .unwrap_or_else(|| name.clone());
        let Some(descriptor) = self
            .state
            .speculation
            .static_semantics
            .objects
            .local_descriptor_bindings
            .get(&resolved_name)
            .cloned()
        else {
            return Ok(false);
        };

        let trace_descriptor_reads = crate::ayy_env_flag!("AYY_TRACE_DESCRIPTOR_READS");
        if trace_descriptor_reads {
            eprintln!(
                "descriptor_read object={object:?} property={property:?} value={:?} configurable={} enumerable={} writable={:?} getter={:?} setter={:?}",
                descriptor.value,
                descriptor.configurable,
                descriptor.enumerable,
                descriptor.writable,
                descriptor.getter,
                descriptor.setter
            );
        }

        if property_name == "value"
            && let Some(value) = descriptor.value.as_ref()
            && matches!(
                value,
                Expression::Member {
                    object: value_object,
                    property: value_property,
                } if value_object.as_ref() == object && value_property.as_ref() == property
            )
        {
            if trace_descriptor_reads {
                eprintln!("descriptor_read:self_reference object={object:?} property={property:?}");
            }
            return Ok(false);
        }
        self.emit_property_descriptor_binding_member_value(&descriptor, property_name)
    }
}
