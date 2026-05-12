use super::*;

impl<'a> FunctionCompiler<'a> {
    fn static_typed_array_values_from_expression(
        &self,
        source_expression: &Expression,
    ) -> Option<ArrayValueBinding> {
        let mut object_binding =
            self.resolve_static_typed_array_object_binding_from_expression(source_expression)?;
        let length = self.static_typed_array_length_from_binding(&object_binding)?;
        let source_values = match source_expression {
            Expression::New { arguments, .. } => {
                let expanded_arguments = self.expand_call_arguments(arguments);
                expanded_arguments
                    .first()
                    .and_then(|source| self.resolve_array_binding_from_expression(source))
            }
            _ => None,
        };
        if let Some(source_values) = source_values {
            for (index, value) in source_values.values.into_iter().enumerate().take(length) {
                let Some(value) = value else {
                    continue;
                };
                object_binding_define_property(
                    &mut object_binding,
                    Expression::Number(index as f64),
                    value,
                    true,
                );
            }
        }
        let values = (0..length)
            .map(|index| {
                self.static_typed_array_member_value_from_binding(
                    &object_binding,
                    &Expression::Number(index as f64),
                )
            })
            .collect::<Vec<_>>();
        Some(ArrayValueBinding { values })
    }

    pub(in crate::backend::direct_wasm) fn seed_local_viewed_array_buffer_object_binding(
        &mut self,
        name: &str,
        source_expression: &Expression,
    ) {
        let Some(buffer_expression) =
            self.resolve_static_constructed_viewed_array_buffer_expression(source_expression)
        else {
            return;
        };
        let mut object_binding = self
            .state
            .speculation
            .static_semantics
            .local_object_binding(name)
            .cloned()
            .or_else(|| self.backend.global_object_binding(name).cloned())
            .unwrap_or_else(empty_object_value_binding);
        object_binding_define_property(
            &mut object_binding,
            viewed_array_buffer_property_expression(),
            buffer_expression,
            false,
        );
        self.state
            .speculation
            .static_semantics
            .set_local_object_binding(name, object_binding.clone());
        if self.binding_name_is_global(name) {
            self.backend
                .sync_global_object_binding(name, Some(object_binding));
        }
        self.state
            .speculation
            .static_semantics
            .set_local_kind(name, StaticValueKind::Object);
    }

    pub(in crate::backend::direct_wasm) fn seed_global_viewed_array_buffer_object_binding(
        &mut self,
        name: &str,
        source_expression: &Expression,
    ) {
        let Some(buffer_expression) =
            self.resolve_static_constructed_viewed_array_buffer_expression(source_expression)
        else {
            return;
        };
        let mut object_binding = self
            .backend
            .global_object_binding(name)
            .cloned()
            .unwrap_or_else(empty_object_value_binding);
        object_binding_define_property(
            &mut object_binding,
            viewed_array_buffer_property_expression(),
            buffer_expression,
            false,
        );
        self.backend
            .sync_global_object_binding(name, Some(object_binding));
    }

    pub(in crate::backend::direct_wasm) fn seed_local_typed_array_object_binding(
        &mut self,
        name: &str,
        source_expression: &Expression,
    ) {
        let Some(typed_array_binding) =
            self.resolve_static_typed_array_object_binding_from_expression(source_expression)
        else {
            return;
        };
        let mut object_binding = self
            .state
            .speculation
            .static_semantics
            .local_object_binding(name)
            .cloned()
            .or_else(|| self.backend.global_object_binding(name).cloned())
            .unwrap_or_else(empty_object_value_binding);
        self.merge_static_typed_array_object_binding(&mut object_binding, &typed_array_binding);
        self.state
            .speculation
            .static_semantics
            .set_local_object_binding(name, object_binding.clone());
        if let Some(array_binding) =
            self.static_typed_array_values_from_expression(source_expression)
        {
            let length_local = self.ensure_runtime_array_length_local(name);
            self.push_i32_const(array_binding.values.len() as i32);
            self.push_local_set(length_local);
            self.ensure_runtime_array_slots_for_binding(name, &array_binding);
            self.state
                .speculation
                .static_semantics
                .set_local_array_binding(name, array_binding.clone());
            if self.binding_name_is_global(name) {
                self.backend
                    .sync_global_array_binding(name, Some(array_binding));
            }
        }
        if self.binding_name_is_global(name) {
            self.backend
                .sync_global_object_binding(name, Some(object_binding));
        }
        self.state
            .speculation
            .static_semantics
            .set_local_kind(name, StaticValueKind::Object);
    }

    pub(in crate::backend::direct_wasm) fn seed_global_typed_array_object_binding(
        &mut self,
        name: &str,
        source_expression: &Expression,
    ) {
        let Some(typed_array_binding) =
            self.resolve_static_typed_array_object_binding_from_expression(source_expression)
        else {
            return;
        };
        let mut object_binding = self
            .backend
            .global_object_binding(name)
            .cloned()
            .unwrap_or_else(empty_object_value_binding);
        self.merge_static_typed_array_object_binding(&mut object_binding, &typed_array_binding);
        self.backend
            .sync_global_object_binding(name, Some(object_binding));
    }

    pub(in crate::backend::direct_wasm) fn apply_resizable_array_buffer_resize(
        &mut self,
        name: &str,
        new_length: usize,
    ) -> DirectResult<bool> {
        let Some(name) = self.resolve_local_resizable_array_buffer_binding_name(name) else {
            return Ok(false);
        };
        let Some(binding) = self
            .state
            .speculation
            .static_semantics
            .local_resizable_array_buffer_binding_mut(&name)
        else {
            return Ok(false);
        };
        if new_length > binding.max_length {
            return self.emit_named_error_throw("RangeError").map(|_| true);
        }
        let bytes_per_element = binding.bytes_per_element.max(1);
        if new_length % bytes_per_element != 0 {
            return self.emit_named_error_throw("RangeError").map(|_| true);
        }
        let new_element_length = new_length / bytes_per_element;
        let old_length = binding.values.len();
        if new_element_length < old_length {
            binding.values.truncate(new_element_length);
        } else if new_element_length > old_length {
            binding
                .values
                .extend((old_length..new_element_length).map(|_| Some(Expression::Number(0.0))));
        }

        let length_local = self.ensure_runtime_array_length_local(&name);
        self.push_i32_const(new_element_length as i32);
        self.push_local_set(length_local);
        for index in 0..TRACKED_ARRAY_SLOT_LIMIT {
            let slot = self.ensure_runtime_array_slot_entry(&name, index);
            if index < new_element_length as u32 {
                if index >= old_length as u32 {
                    self.push_i32_const(0);
                    self.push_local_set(slot.value_local);
                    self.push_i32_const(1);
                    self.push_local_set(slot.present_local);
                }
            } else {
                self.push_i32_const(JS_UNDEFINED_TAG);
                self.push_local_set(slot.value_local);
                self.push_i32_const(0);
                self.push_local_set(slot.present_local);
            }
        }
        self.sync_typed_array_views_for_buffer(&name)?;
        Ok(true)
    }

    pub(in crate::backend::direct_wasm) fn emit_typed_array_view_write(
        &mut self,
        view_name: &str,
        property: &Expression,
        value: &Expression,
    ) -> DirectResult<bool> {
        let Some(view) = self
            .state
            .speculation
            .static_semantics
            .local_typed_array_view_binding(view_name)
            .cloned()
        else {
            return Ok(false);
        };
        let value_local = self.allocate_temp_local();
        self.emit_numeric_expression(value)?;
        self.push_local_set(value_local);

        let handled = if let Some(index) = argument_index_from_expression(property) {
            let scale = if let Some(buffer) = self
                .state
                .speculation
                .static_semantics
                .local_resizable_array_buffer_binding(&view.buffer_name)
            {
                if buffer.bytes_per_element == view.bytes_per_element {
                    1
                } else if buffer.bytes_per_element == 1 {
                    view.bytes_per_element
                } else {
                    return Ok(false);
                }
            } else {
                1
            };
            let buffer_index = view.offset * scale + index as usize * scale;
            let materialized = self.materialize_static_expression(value);
            if let Some(buffer) = self
                .state
                .speculation
                .static_semantics
                .local_resizable_array_buffer_binding_mut(&view.buffer_name)
            {
                if buffer_index < buffer.values.len() {
                    buffer.values[buffer_index] = Some(materialized);
                }
            }
            self.emit_runtime_array_slot_write_from_local(
                &view.buffer_name,
                buffer_index as u32,
                value_local,
            )?
        } else if view.offset == 0 {
            self.emit_dynamic_runtime_array_slot_write(&view.buffer_name, property, value)?
        } else {
            false
        };

        if handled {
            self.state.emission.output.instructions.push(0x1a);
            self.sync_typed_array_views_for_buffer(&view.buffer_name)?;
            self.push_local_get(value_local);
            return Ok(true);
        }

        self.push_local_get(value_local);
        Ok(true)
    }

    pub(in crate::backend::direct_wasm) fn update_local_resizable_array_buffer_binding(
        &mut self,
        name: &str,
        value: &Expression,
    ) -> DirectResult<()> {
        let Some(binding) = self.resolve_resizable_array_buffer_binding_from_expression(value)
        else {
            self.state
                .speculation
                .static_semantics
                .clear_local_resizable_array_buffer_binding(name);
            return Ok(());
        };
        let length = binding.values.len();
        let runtime_binding = ArrayValueBinding {
            values: binding.values.clone(),
        };
        self.state
            .speculation
            .static_semantics
            .set_local_resizable_array_buffer_binding(name, binding);
        let length_local = self.ensure_runtime_array_length_local(name);
        self.push_i32_const(length as i32);
        self.push_local_set(length_local);
        self.ensure_runtime_array_slots_for_binding(name, &runtime_binding);
        self.state
            .speculation
            .static_semantics
            .set_local_kind(name, StaticValueKind::Object);
        Ok(())
    }

    pub(in crate::backend::direct_wasm) fn update_local_typed_array_view_binding(
        &mut self,
        name: &str,
        value: &Expression,
    ) -> DirectResult<()> {
        let Some(binding) = self.resolve_typed_array_view_binding_from_expression(value) else {
            self.state
                .speculation
                .static_semantics
                .clear_local_typed_array_view_binding(name);
            self.state
                .speculation
                .static_semantics
                .clear_runtime_typed_array_oob_local(name);
            return Ok(());
        };
        self.state
            .speculation
            .static_semantics
            .set_local_typed_array_view_binding(name, binding);
        self.sync_typed_array_view_runtime_state(name)
    }

    pub(in crate::backend::direct_wasm) fn emit_test_iteration_and_resize_call(
        &mut self,
        arguments: &[CallArgument],
    ) -> DirectResult<bool> {
        let (iterable, rab, resize_after, new_byte_length) = match arguments {
            [
                CallArgument::Expression(iterable),
                CallArgument::Expression(_expected),
                CallArgument::Expression(rab),
                CallArgument::Expression(resize_after),
                CallArgument::Expression(new_byte_length),
                ..,
            ] => (iterable, rab, resize_after, new_byte_length),
            [
                CallArgument::Expression(iterable),
                CallArgument::Expression(rab),
                CallArgument::Expression(resize_after),
                CallArgument::Expression(new_byte_length),
                ..,
            ] => (iterable, rab, resize_after, new_byte_length),
            _ => return Ok(false),
        };
        let Some(view) = self.resolve_typed_array_view_binding_from_expression(iterable) else {
            return Ok(false);
        };
        let Some(values) = self.typed_array_view_static_values(&view) else {
            return Ok(false);
        };
        let Some(resize_after) = self.resolve_typed_array_element_count(resize_after) else {
            return Ok(false);
        };
        if resize_after > values.values.len() {
            return Ok(false);
        }
        let Some(new_byte_length) = self.resolve_typed_array_element_count(new_byte_length) else {
            return Ok(false);
        };
        let rab_name = match self.materialize_static_expression(rab) {
            Expression::Identifier(name) => name,
            _ => return Ok(false),
        };
        if !self.apply_resizable_array_buffer_resize(&rab_name, new_byte_length)? {
            return Ok(false);
        }
        self.push_i32_const(JS_UNDEFINED_TAG);
        Ok(true)
    }

    pub(in crate::backend::direct_wasm) fn resolve_static_collect_values_call_binding(
        &self,
        arguments: &[CallArgument],
    ) -> Option<ArrayValueBinding> {
        let CallArgument::Expression(iterable) = arguments.first()? else {
            return None;
        };
        let view = self.resolve_typed_array_view_binding_from_expression(iterable)?;
        self.typed_array_view_static_values(&view)
    }

    pub(in crate::backend::direct_wasm) fn emit_static_collect_values_call(
        &mut self,
        arguments: &[CallArgument],
    ) -> DirectResult<bool> {
        if self
            .resolve_static_collect_values_call_binding(arguments)
            .is_none()
        {
            return Ok(false);
        }
        self.emit_ignored_call_arguments(arguments)?;
        self.push_i32_const(JS_TYPEOF_OBJECT_TAG);
        Ok(true)
    }

    pub(in crate::backend::direct_wasm) fn emit_synthetic_create_rab_call(
        &mut self,
        callee: &Expression,
        arguments: &[CallArgument],
    ) -> DirectResult<bool> {
        let expression = Expression::Call {
            callee: Box::new(callee.clone()),
            arguments: arguments.to_vec(),
        };
        if self
            .resolve_resizable_array_buffer_binding_from_expression(&expression)
            .is_none()
        {
            return Ok(false);
        }
        self.emit_ignored_call_arguments(arguments)?;
        self.push_i32_const(JS_TYPEOF_OBJECT_TAG);
        Ok(true)
    }
}
