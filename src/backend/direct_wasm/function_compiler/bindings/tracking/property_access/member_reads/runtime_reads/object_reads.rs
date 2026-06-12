use super::*;

thread_local! {
    static ACTIVE_OBJECT_BINDING_MEMBER_READ_VALUES: RefCell<HashSet<String>> =
        RefCell::new(HashSet::new());
}

/// Guards against emitting a tracked object-binding property value that
/// (directly or transitively) reads the same member again. Self-referential
/// member updates such as `--object.prop` can record a property value that
/// still contains a read of `object.prop`, which would otherwise recurse
/// forever during emission.
struct ObjectBindingMemberReadValueGuard {
    key: String,
}

impl ObjectBindingMemberReadValueGuard {
    fn enter(object: &Expression, property: &Expression, value: &Expression) -> Option<Self> {
        let key = format!("{object:?}:{property:?}:{value:?}");
        let inserted = ACTIVE_OBJECT_BINDING_MEMBER_READ_VALUES
            .with(|active| active.borrow_mut().insert(key.clone()));
        inserted.then_some(Self { key })
    }
}

impl Drop for ObjectBindingMemberReadValueGuard {
    fn drop(&mut self) {
        ACTIVE_OBJECT_BINDING_MEMBER_READ_VALUES.with(|active| {
            active.borrow_mut().remove(&self.key);
        });
    }
}

impl<'a> FunctionCompiler<'a> {
    pub(in crate::backend::direct_wasm) fn emit_runtime_property_key_match_from_local(
        &mut self,
        property_local: u32,
        existing_key: &Expression,
    ) -> DirectResult<()> {
        if let Expression::String(property_name) = existing_key {
            self.emit_runtime_string_literal_memory_comparison(property_local, property_name)?;
            return Ok(());
        }

        self.push_local_get(property_local);
        self.emit_numeric_expression(existing_key)?;
        self.push_binary_op(BinaryOp::Equal)?;

        Ok(())
    }

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

    fn emit_runtime_object_descriptor_member_value(
        &mut self,
        object: &Expression,
        existing_key: &Expression,
        descriptor: &PropertyDescriptorBinding,
    ) -> DirectResult<()> {
        if let Some(getter) = descriptor.getter.as_ref()
            && let Some(function_binding) = self.resolve_function_binding_from_expression(getter)
        {
            let capture_slots = self.resolve_member_function_capture_slots(object, existing_key);
            match function_binding {
                LocalFunctionBinding::User(function_name) => {
                    let static_getter_binding = LocalFunctionBinding::User(function_name.clone());
                    let static_this_expression =
                        self.resolve_static_snapshot_this_expression(object);
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
                        return Ok(());
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
            return Ok(());
        }

        if let Some(value) = descriptor.value.as_ref() {
            let owner_name = match object {
                Expression::Identifier(name) => {
                    self.runtime_object_property_shadow_owner_name_for_identifier(name)
                }
                _ => None,
            };
            self.emit_runtime_object_binding_property_value(
                owner_name.as_deref(),
                existing_key,
                value,
            )?;
            return Ok(());
        }

        self.push_i32_const(JS_UNDEFINED_TAG);
        Ok(())
    }

    fn dynamic_string_descriptor_property_key_getter_names(
        &self,
        object: &Expression,
        object_binding: &ObjectValueBinding,
    ) -> Option<Vec<String>> {
        let static_this_expression = self.resolve_static_snapshot_this_expression(object);
        let mut property_names = Vec::new();
        for (property, descriptor) in &object_binding.property_descriptors {
            let property_name = static_property_name_from_expression(property)?;
            let Some(getter) = descriptor.getter.as_ref() else {
                if descriptor.value.is_some() {
                    return None;
                }
                continue;
            };
            let getter_binding = self.resolve_function_binding_from_expression(getter)?;
            let return_value = self.resolve_static_getter_value_from_binding_with_context(
                &getter_binding,
                &static_this_expression,
                self.current_function_name(),
            )?;
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
            if !matches!(return_value, Expression::String(value) if value == property_name) {
                return None;
            }
            if !property_names
                .iter()
                .any(|existing_name| existing_name == &property_name)
            {
                property_names.push(property_name);
            }
        }
        (!property_names.is_empty()).then_some(property_names)
    }

    fn emit_property_key_membership_from_local(
        &mut self,
        property_local: u32,
        property_names: &[String],
    ) -> DirectResult<bool> {
        let mut emitted = false;
        for property_name in property_names {
            let existing_key = Expression::String(property_name.clone());
            self.emit_runtime_property_key_match_from_local(property_local, &existing_key)?;
            if emitted {
                self.state.emission.output.instructions.push(0x72);
            }
            emitted = true;
        }
        Ok(emitted)
    }

    fn emit_dynamic_runtime_string_property_key_descriptor_read(
        &mut self,
        object: &Expression,
        property: &Expression,
        object_binding: &ObjectValueBinding,
    ) -> DirectResult<bool> {
        let Some(property_names) =
            self.dynamic_string_descriptor_property_key_getter_names(object, object_binding)
        else {
            return Ok(false);
        };

        let property_local = self.allocate_temp_local();
        self.emit_numeric_expression(property)?;
        self.push_local_set(property_local);
        if !self.emit_property_key_membership_from_local(property_local, &property_names)? {
            return Ok(false);
        }
        self.state.emission.output.instructions.push(0x04);
        self.state.emission.output.instructions.push(I32_TYPE);
        self.push_control_frame();
        self.push_local_get(property_local);
        self.state.emission.output.instructions.push(0x05);
        self.push_i32_const(JS_UNDEFINED_TAG);
        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();
        Ok(true)
    }

    fn emit_dynamic_runtime_string_descriptor_member_read(
        &mut self,
        object: &Expression,
        property: &Expression,
        object_binding: &ObjectValueBinding,
    ) -> DirectResult<bool> {
        let canonical_property = self.canonical_object_property_expression(property);
        if static_property_name_from_expression(&canonical_property).is_some() {
            return Ok(false);
        }

        if self.emit_dynamic_runtime_string_property_key_descriptor_read(
            object,
            property,
            object_binding,
        )? {
            return Ok(true);
        }

        let descriptor_entries = object_binding
            .property_descriptors
            .iter()
            .filter_map(|(property, descriptor)| {
                static_property_name_from_expression(property)
                    .map(|property_name| (property_name, descriptor.clone()))
            })
            .collect::<Vec<_>>();
        if descriptor_entries.is_empty() {
            return Ok(false);
        }

        let property_local = self.allocate_temp_local();
        self.emit_numeric_expression(property)?;
        self.push_local_set(property_local);

        let mut open_frames = 0;
        for (property_name, descriptor) in descriptor_entries {
            let existing_key = Expression::String(property_name);
            self.emit_runtime_property_key_match_from_local(property_local, &existing_key)?;
            self.state.emission.output.instructions.push(0x04);
            self.state.emission.output.instructions.push(I32_TYPE);
            self.push_control_frame();
            open_frames += 1;
            self.emit_runtime_object_descriptor_member_value(object, &existing_key, &descriptor)?;
            self.state.emission.output.instructions.push(0x05);
        }

        self.push_i32_const(JS_UNDEFINED_TAG);
        for _ in 0..open_frames {
            self.state.emission.output.instructions.push(0x0b);
            self.pop_control_frame();
        }
        Ok(true)
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
        // Properties stored through runtime shadow channels (e.g. constructor
        // `this.x = ...` writes) may be missing from the static binding; merge
        // the shadow channel so the dispatch covers every live property name.
        let shadow_aware_binding = owner_name
            .as_deref()
            .filter(|owner| self.runtime_object_property_shadow_owner_has_bindings(owner))
            .and_then(|owner| self.resolve_runtime_shadow_object_binding(owner));
        let dispatch_binding = shadow_aware_binding.as_ref().unwrap_or(object_binding);

        let mut open_frames = 0;
        for (property_name, fallback_value) in
            self.object_binding_string_property_values_with_inherited(object, dispatch_binding)
        {
            let existing_key = Expression::String(property_name);
            self.emit_runtime_property_key_match_from_local(property_local, &existing_key)?;
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
        let Some(object_binding) = self.resolve_object_binding_from_expression(object) else {
            return Ok(false);
        };
        if !matches!(property, Expression::String(_) | Expression::Number(_))
            && self.resolve_property_key_expression(property).is_none()
            && object_binding.string_properties.is_empty()
            && object_binding.symbol_properties.is_empty()
        {
            return Ok(false);
        }
        let is_private_property = self.is_private_member_read_property(property);
        if !is_private_property && static_property_name_from_expression(property).is_none() {
            if self.emit_dynamic_runtime_string_descriptor_member_read(
                object,
                property,
                &object_binding,
            )? {
                return Ok(true);
            }
            if self.emit_dynamic_runtime_string_object_binding_member_read(
                object,
                property,
                &object_binding,
            )? {
                return Ok(true);
            }
            if self.emit_dynamic_runtime_symbol_object_binding_member_read(
                object,
                property,
                &object_binding,
            )? {
                return Ok(true);
            }
        }
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
        if is_private_property && crate::ayy_env_flag!("AYY_TRACE_PRIVATE_MEMBER_LOOKUP") {
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
            let Some(_value_guard) =
                ObjectBindingMemberReadValueGuard::enter(object, property, &value)
            else {
                return Ok(false);
            };
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
        } else if !is_private_property
            && self.emit_dynamic_runtime_string_descriptor_member_read(
                object,
                property,
                &object_binding,
            )?
        {
            return Ok(true);
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
