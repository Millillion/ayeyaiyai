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
        let Some((receiver, descriptor_property)) =
            self.dynamic_property_descriptor_source_for_local(name)
        else {
            return Ok(false);
        };

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

        let trace_descriptor_reads = std::env::var_os("AYY_TRACE_DESCRIPTOR_READS").is_some();
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
