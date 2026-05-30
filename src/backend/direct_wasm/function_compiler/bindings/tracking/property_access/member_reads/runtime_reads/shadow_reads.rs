use super::*;

impl<'a> FunctionCompiler<'a> {
    fn dynamic_object_shadow_read_has_static_fallback(
        &self,
        object: &Expression,
        property: &Expression,
    ) -> bool {
        let property = self.canonical_object_property_expression(property);
        self.resolve_member_getter_binding(object, &property)
            .is_some()
            || self
                .resolve_object_binding_from_expression(object)
                .and_then(|object_binding| {
                    self.resolve_object_binding_property_value_with_inherited(
                        object,
                        &object_binding,
                        &property,
                    )
                })
                .is_some()
            || self
                .resolve_inherited_object_property_value(object, &property)
                .is_some()
    }

    pub(in crate::backend::direct_wasm) fn emit_runtime_object_dynamic_shadow_member_read_without_static_fallback(
        &mut self,
        object: &Expression,
        property: &Expression,
    ) -> DirectResult<bool> {
        let owner_name = match object {
            Expression::Identifier(name) => {
                self.runtime_object_property_shadow_owner_name_for_identifier(name)
            }
            Expression::This => {
                self.runtime_object_property_shadow_owner_name_for_identifier("this")
            }
            _ => None,
        };
        let Some(owner_name) = owner_name else {
            return Ok(false);
        };
        if self.is_private_member_read_property(property)
            || self.dynamic_object_shadow_read_has_static_fallback(object, property)
        {
            return Ok(false);
        }
        if !self.runtime_object_dynamic_property_shadow_has_binding(&owner_name) {
            return Ok(false);
        }

        let key_binding = self.runtime_object_dynamic_property_key_shadow_binding(&owner_name);
        let value_binding = self.runtime_object_dynamic_property_value_shadow_binding(&owner_name);
        let dynamic_key_local = self.allocate_temp_local();
        self.push_global_get(key_binding.present_index);
        self.state.emission.output.instructions.push(0x04);
        self.state.emission.output.instructions.push(I32_TYPE);
        self.push_control_frame();
        self.push_global_get(key_binding.value_index);
        self.push_local_set(dynamic_key_local);
        if self.resolve_property_key_expression(property).is_some()
            || matches!(property, Expression::String(_) | Expression::Number(_))
        {
            let canonical_property = self.canonical_object_property_expression(property);
            self.emit_runtime_property_key_match_from_local(
                dynamic_key_local,
                &canonical_property,
            )?;
        } else {
            let property_local = self.allocate_temp_local();
            self.emit_numeric_expression(property)?;
            self.push_local_set(property_local);
            self.push_local_get(dynamic_key_local);
            self.push_local_get(property_local);
            self.push_binary_op(BinaryOp::Equal)?;
        }
        self.state.emission.output.instructions.push(0x04);
        self.state.emission.output.instructions.push(I32_TYPE);
        self.push_control_frame();
        self.push_global_get(value_binding.value_index);
        self.state.emission.output.instructions.push(0x05);
        self.push_i32_const(JS_UNDEFINED_TAG);
        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();
        self.state.emission.output.instructions.push(0x05);
        self.push_i32_const(JS_UNDEFINED_TAG);
        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();
        Ok(true)
    }

    fn emit_runtime_shadow_getter_fallback_read(
        &mut self,
        object: &Expression,
        property: &Expression,
    ) -> DirectResult<bool> {
        let Some(function_binding) = self.resolve_member_getter_binding(object, property) else {
            return Ok(false);
        };
        let capture_slots = self.resolve_member_function_capture_slots(object, property);
        match function_binding {
            LocalFunctionBinding::User(function_name) => {
                let static_getter_binding = LocalFunctionBinding::User(function_name.clone());
                let static_this_expression = self.resolve_static_snapshot_this_expression(object);
                if let Some(return_value) = self
                    .resolve_static_getter_value_from_binding_with_context(
                        &static_getter_binding,
                        &static_this_expression,
                        self.current_function_name(),
                    )
                {
                    let return_value = if self
                        .resolve_static_boxed_primitive_value(&return_value)
                        .is_some()
                    {
                        return_value
                    } else {
                        self.resolve_static_primitive_expression_with_context(
                            &return_value,
                            self.current_function_name(),
                        )
                        .unwrap_or(return_value)
                    };
                    self.emit_numeric_expression(&return_value)?;
                    return Ok(true);
                }
                self.emit_member_getter_call_with_bound_this(
                    &function_name,
                    object,
                    capture_slots.as_ref(),
                )?;
            }
            LocalFunctionBinding::Builtin(function_name) => {
                let callee = Expression::Identifier(function_name);
                if !self.emit_arguments_slot_accessor_call(&callee, &[], 0, Some(&[]))? {
                    self.push_i32_const(JS_UNDEFINED_TAG);
                }
            }
        }
        Ok(true)
    }

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
                if is_private_property {
                    self.resolve_object_binding_property_value(&object_binding, property)
                } else {
                    self.resolve_object_binding_property_value_with_inherited(
                        object,
                        &object_binding,
                        property,
                    )
                }
            })
            .or_else(|| {
                (!is_private_property)
                    .then(|| self.resolve_inherited_object_property_value(object, property))
                    .flatten()
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
            if !is_private_property
                && self.emit_runtime_shadow_getter_fallback_read(object, property)?
            {
            } else if !is_private_property
                && self.emit_runtime_array_member_read(object, property)?
            {
            } else if let Some(fallback_value) = fallback_value {
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
            } else if is_private_property
                && self.emit_private_member_missing_shadow_read_fallback(object, property)?
            {
            } else if is_private_property {
                self.emit_named_error_throw("TypeError")?;
            } else if self.emit_runtime_native_error_member_read(object, property)? {
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
        if !is_private_property
            && self.emit_runtime_shadow_getter_fallback_read(object, property)?
        {
        } else if !is_private_property && self.emit_runtime_array_member_read(object, property)? {
        } else if let Some(fallback_value) = fallback_value {
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
        } else if is_private_property
            && self.emit_private_member_missing_shadow_read_fallback(object, property)?
        {
        } else if is_private_property {
            self.emit_named_error_throw("TypeError")?;
        } else if self.emit_runtime_native_error_member_read(object, property)? {
        } else if !self.emit_runtime_user_function_property_read(object, property)? {
            self.push_i32_const(JS_UNDEFINED_TAG);
        }
        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();
        Ok(true)
    }
}
