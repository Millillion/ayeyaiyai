use super::*;

impl<'a> FunctionCompiler<'a> {
    pub(in crate::backend::direct_wasm) fn emit_runtime_object_shadow_member_read(
        &mut self,
        object: &Expression,
        property: &Expression,
    ) -> DirectResult<bool> {
        let Some(binding) = self.resolve_runtime_object_property_shadow_binding(object, property)
        else {
            return Ok(false);
        };
        let deleted_binding =
            self.resolve_runtime_object_property_shadow_deleted_binding(object, property);
        let is_private_property = self.is_private_member_read_property(property);
        let fallback_value = self
            .resolve_object_binding_from_expression(object)
            .and_then(|object_binding| {
                self.resolve_object_binding_property_value(&object_binding, property)
            })
            .or_else(|| {
                if matches!(object, Expression::This)
                    && let Expression::String(property_name) = property
                    && self.global_has_binding(property_name)
                {
                    return Some(Expression::Identifier(property_name.clone()));
                }
                None
            });
        let fallback_value =
            if is_private_property && self.expression_is_current_this_reference(object) {
                None
            } else {
                fallback_value
            };
        if is_private_property && std::env::var_os("AYY_TRACE_PRIVATE_MEMBER_LOOKUP").is_some() {
            eprintln!(
                "private_shadow_read object={object:?} property={property:?} fallback={fallback_value:?} deleted_present={} binding_present={}",
                deleted_binding.is_some(),
                true
            );
        }
        if let Some(deleted_binding) = deleted_binding {
            self.push_global_get(deleted_binding.present_index);
            self.state.emission.output.instructions.push(0x04);
            self.state.emission.output.instructions.push(I32_TYPE);
            self.push_control_frame();
            if is_private_property {
                self.emit_named_error_throw("TypeError")?;
            } else {
                self.push_i32_const(JS_UNDEFINED_TAG);
            }
            self.state.emission.output.instructions.push(0x05);
            self.push_global_get(binding.present_index);
            self.state.emission.output.instructions.push(0x04);
            self.state.emission.output.instructions.push(I32_TYPE);
            self.push_control_frame();
            if is_private_property {
                let value_local = self.allocate_temp_local();
                self.push_global_get(binding.value_index);
                self.push_local_set(value_local);
                self.emit_private_member_binding_value_from_local(object, property, value_local)?;
            } else {
                self.push_global_get(binding.value_index);
            }
            self.state.emission.output.instructions.push(0x05);
            if let Some(fallback_value) = fallback_value {
                if is_private_property
                    && let Some(function_binding) =
                        self.resolve_function_binding_from_expression(&fallback_value)
                {
                    let capture_slots =
                        self.resolve_function_expression_capture_slots(&fallback_value);
                    self.emit_private_member_fallback_function_binding_read(
                        object,
                        property,
                        &function_binding,
                        capture_slots.as_ref(),
                    )?;
                } else {
                    self.emit_runtime_shadow_fallback_value(&fallback_value)?;
                }
            } else if is_private_property {
                self.emit_named_error_throw("TypeError")?;
            } else if !self.emit_runtime_user_function_property_read(object, property)? {
                self.push_i32_const(JS_UNDEFINED_TAG);
            }
            self.state.emission.output.instructions.push(0x0b);
            self.pop_control_frame();
            self.state.emission.output.instructions.push(0x0b);
            self.pop_control_frame();
            return Ok(true);
        }
        self.push_global_get(binding.present_index);
        self.state.emission.output.instructions.push(0x04);
        self.state.emission.output.instructions.push(I32_TYPE);
        self.push_control_frame();
        if is_private_property {
            let value_local = self.allocate_temp_local();
            self.push_global_get(binding.value_index);
            self.push_local_set(value_local);
            self.emit_private_member_binding_value_from_local(object, property, value_local)?;
        } else {
            self.push_global_get(binding.value_index);
        }
        self.state.emission.output.instructions.push(0x05);
        if let Some(fallback_value) = fallback_value {
            if is_private_property
                && let Some(function_binding) =
                    self.resolve_function_binding_from_expression(&fallback_value)
            {
                let capture_slots = self.resolve_function_expression_capture_slots(&fallback_value);
                self.emit_private_member_fallback_function_binding_read(
                    object,
                    property,
                    &function_binding,
                    capture_slots.as_ref(),
                )?;
            } else {
                self.emit_runtime_shadow_fallback_value(&fallback_value)?;
            }
        } else if is_private_property {
            self.emit_named_error_throw("TypeError")?;
        } else if !self.emit_runtime_user_function_property_read(object, property)? {
            self.push_i32_const(JS_UNDEFINED_TAG);
        }
        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();
        Ok(true)
    }
}
