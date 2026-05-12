use super::*;

impl<'a> FunctionCompiler<'a> {
    fn emit_runtime_object_binding_property_value(
        &mut self,
        owner_name: Option<&str>,
        existing_key: &Expression,
        fallback_value: &Expression,
    ) -> DirectResult<()> {
        if let Some(owner_name) = owner_name {
            let binding =
                self.runtime_object_property_shadow_binding_by_property(owner_name, existing_key);
            let deleted_binding = self.runtime_object_property_shadow_deleted_binding_by_property(
                owner_name,
                existing_key,
            );
            self.push_global_get(deleted_binding.present_index);
            self.state.emission.output.instructions.push(0x04);
            self.state.emission.output.instructions.push(I32_TYPE);
            self.push_control_frame();
            self.push_i32_const(JS_UNDEFINED_TAG);
            self.state.emission.output.instructions.push(0x05);
            self.push_global_get(binding.present_index);
            self.state.emission.output.instructions.push(0x04);
            self.state.emission.output.instructions.push(I32_TYPE);
            self.push_control_frame();
            self.push_global_get(binding.value_index);
            self.state.emission.output.instructions.push(0x05);
            self.emit_runtime_shadow_fallback_value(fallback_value)?;
            self.state.emission.output.instructions.push(0x0b);
            self.pop_control_frame();
            self.state.emission.output.instructions.push(0x0b);
            self.pop_control_frame();
        } else {
            self.emit_runtime_shadow_fallback_value(fallback_value)?;
        }
        Ok(())
    }

    fn emit_dynamic_runtime_string_object_binding_member_read(
        &mut self,
        object: &Expression,
        property: &Expression,
        object_binding: &ObjectValueBinding,
    ) -> DirectResult<bool> {
        if object_binding.string_properties.is_empty() {
            return Ok(false);
        }

        let property_local = self.allocate_temp_local();
        self.emit_numeric_expression(property)?;
        self.push_local_set(property_local);
        let owner_name = match object {
            Expression::Identifier(name) => {
                self.runtime_object_property_shadow_owner_name_for_identifier(name)
            }
            _ => None,
        };

        let mut open_frames = 0;
        for (property_name, fallback_value) in
            self.object_binding_string_property_values_with_inherited(object, object_binding)
        {
            let existing_key = Expression::String(property_name);
            self.push_local_get(property_local);
            self.emit_numeric_expression(&existing_key)?;
            self.push_binary_op(BinaryOp::Equal)?;
            self.state.emission.output.instructions.push(0x04);
            self.state.emission.output.instructions.push(I32_TYPE);
            self.push_control_frame();
            open_frames += 1;
            self.emit_runtime_object_binding_property_value(
                owner_name.as_deref(),
                &existing_key,
                &fallback_value,
            )?;
            self.state.emission.output.instructions.push(0x05);
        }

        self.push_i32_const(JS_UNDEFINED_TAG);
        for _ in 0..open_frames {
            self.state.emission.output.instructions.push(0x0b);
            self.pop_control_frame();
        }
        Ok(true)
    }

    fn emit_dynamic_runtime_symbol_object_binding_member_read(
        &mut self,
        object: &Expression,
        property: &Expression,
        object_binding: &ObjectValueBinding,
    ) -> DirectResult<bool> {
        if object_binding.symbol_properties.is_empty() {
            return Ok(false);
        }

        let property_local = self.allocate_temp_local();
        self.emit_numeric_expression(property)?;
        self.push_local_set(property_local);
        let owner_name = match object {
            Expression::Identifier(name) => {
                self.runtime_object_property_shadow_owner_name_for_identifier(name)
            }
            _ => None,
        };

        if let Some((existing_key, fallback_value)) =
            self.resolve_static_symbol_property_shadow_entry(object_binding, property)
        {
            self.emit_runtime_object_binding_property_value(
                owner_name.as_deref(),
                &existing_key,
                &fallback_value,
            )?;
            return Ok(true);
        }

        let mut open_frames = 0;
        for (existing_key, fallback_value) in object_binding.symbol_properties.clone() {
            let comparison_key = self.canonical_object_property_expression(&existing_key);
            self.push_local_get(property_local);
            self.emit_numeric_expression(&comparison_key)?;
            self.push_binary_op(BinaryOp::Equal)?;
            self.state.emission.output.instructions.push(0x04);
            self.state.emission.output.instructions.push(I32_TYPE);
            self.push_control_frame();
            open_frames += 1;
            self.emit_runtime_object_binding_property_value(
                owner_name.as_deref(),
                &existing_key,
                &fallback_value,
            )?;
            self.state.emission.output.instructions.push(0x05);
        }

        self.push_i32_const(JS_UNDEFINED_TAG);
        for _ in 0..open_frames {
            self.state.emission.output.instructions.push(0x0b);
            self.pop_control_frame();
        }
        Ok(true)
    }

    pub(super) fn emit_runtime_object_binding_member_read(
        &mut self,
        object: &Expression,
        property: &Expression,
    ) -> DirectResult<bool> {
        if !matches!(property, Expression::String(_) | Expression::Number(_))
            && self.resolve_property_key_expression(property).is_none()
        {
            return Ok(false);
        }
        let Some(object_binding) = self.resolve_object_binding_from_expression(object) else {
            return Ok(false);
        };
        let is_private_property = self.is_private_member_read_property(property);
        let resolved_object = self
            .resolve_bound_alias_expression(object)
            .filter(|resolved| !static_expression_matches(resolved, object));
        let materialized_object = self.materialize_static_expression(object);
        let resolved_property = self.resolve_property_key_expression(property).or_else(|| {
            self.resolve_bound_alias_expression(property)
                .filter(|resolved| !static_expression_matches(resolved, property))
        });
        let materialized_property = self.materialize_static_expression(property);

        let getter_binding = self
            .resolve_member_getter_binding(object, property)
            .or_else(|| {
                resolved_object
                    .as_ref()
                    .and_then(|resolved| self.resolve_member_getter_binding(resolved, property))
            })
            .or_else(|| {
                resolved_property
                    .as_ref()
                    .and_then(|resolved| self.resolve_member_getter_binding(object, resolved))
            })
            .or_else(|| {
                resolved_object.as_ref().and_then(|resolved_object| {
                    resolved_property.as_ref().and_then(|resolved_property| {
                        self.resolve_member_getter_binding(resolved_object, resolved_property)
                    })
                })
            })
            .or_else(|| {
                (!static_expression_matches(&materialized_object, object))
                    .then(|| self.resolve_member_getter_binding(&materialized_object, property))?
            })
            .or_else(|| {
                (!static_expression_matches(&materialized_property, property))
                    .then(|| self.resolve_member_getter_binding(object, &materialized_property))?
            })
            .or_else(|| {
                (!static_expression_matches(&materialized_object, object)
                    || !static_expression_matches(&materialized_property, property))
                .then(|| {
                    self.resolve_member_getter_binding(&materialized_object, &materialized_property)
                })?
            });
        if is_private_property && std::env::var_os("AYY_TRACE_PRIVATE_MEMBER_LOOKUP").is_some() {
            eprintln!(
                "private_object_binding_read object={object:?} property={property:?} getter_binding={getter_binding:?}",
            );
        }

        if !is_private_property && let Some(function_binding) = getter_binding {
            let capture_slots = self.resolve_member_function_capture_slots(object, property);
            match function_binding {
                LocalFunctionBinding::User(function_name) => {
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
            return Ok(true);
        }

        if let Some(value) = self.resolve_object_binding_property_value(&object_binding, property) {
            if is_private_property {
                let value_local = self.allocate_temp_local();
                if !self.emit_private_brand_marker_runtime_value(object, property, &value)? {
                    self.emit_numeric_expression(&value)?;
                }
                self.push_local_set(value_local);
                self.emit_private_member_binding_value_from_local(object, property, value_local)?;
                return Ok(true);
            }
            self.emit_numeric_expression(&value)?;
        } else if self.emit_dynamic_runtime_string_object_binding_member_read(
            object,
            property,
            &object_binding,
        )? {
            return Ok(true);
        } else if self.emit_dynamic_runtime_symbol_object_binding_member_read(
            object,
            property,
            &object_binding,
        )? {
            return Ok(true);
        } else if !is_private_property
            && let Some(value) = self.resolve_inherited_object_property_value(object, property)
        {
            self.emit_numeric_expression(&value)?;
        } else if matches!(property, Expression::String(text) if text == "constructor") {
            if let Some(binding) = self.resolve_constructed_object_constructor_binding(object) {
                match binding {
                    LocalFunctionBinding::User(function_name) => {
                        if let Some(user_function) = self.user_function(&function_name) {
                            self.push_i32_const(user_function_runtime_value(user_function));
                        } else {
                            self.push_i32_const(JS_TYPEOF_FUNCTION_TAG);
                        }
                    }
                    LocalFunctionBinding::Builtin(function_name) => {
                        self.push_i32_const(
                            builtin_function_runtime_value(&function_name)
                                .unwrap_or(JS_TYPEOF_FUNCTION_TAG),
                        );
                    }
                }
            } else {
                self.push_i32_const(JS_UNDEFINED_TAG);
            }
        } else if self.emit_runtime_user_function_property_read(object, property)? {
            return Ok(true);
        } else {
            if is_private_property {
                return self.emit_named_error_throw("TypeError").map(|()| true);
            }
            self.push_i32_const(JS_UNDEFINED_TAG);
        }
        Ok(true)
    }
}
