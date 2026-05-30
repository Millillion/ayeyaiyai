use super::*;

impl<'a> FunctionCompiler<'a> {
    pub(super) fn update_runtime_iterator_step_static_array(
        &mut self,
        iterator_binding: &mut ArrayIteratorBinding,
        current_index_local: u32,
        done_local: u32,
        value_local: u32,
    ) {
        let no_runtime_name: Option<String> = None;
        let (values, keys_only, length_local, runtime_name, entries, map_entries) =
            match &iterator_binding.source {
                IteratorSourceKind::StaticArray {
                    values,
                    keys_only,
                    length_local,
                    runtime_name,
                } => (values, *keys_only, length_local, runtime_name, false, false),
                IteratorSourceKind::StaticArrayEntries {
                    values,
                    length_local,
                    runtime_name,
                } => (values, false, length_local, runtime_name, true, false),
                IteratorSourceKind::StaticMapEntries {
                    values,
                    length_local,
                    ..
                } => (values, false, length_local, &no_runtime_name, false, true),
                _ => unreachable!("filtered by caller"),
            };
        if let Some(current_index) = iterator_binding.static_index {
            iterator_binding.static_index = Some(current_index.saturating_add(1));
        } else {
            iterator_binding.static_index = None;
        }
        self.push_local_get(current_index_local);
        if let Some(length_local) = length_local {
            self.push_local_get(*length_local);
        } else if let Some(runtime_name) = runtime_name {
            if !self.emit_global_runtime_array_length_read(runtime_name) {
                self.push_i32_const(values.len() as i32);
            }
        } else {
            self.push_i32_const(values.len() as i32);
        }
        self.push_binary_op(BinaryOp::GreaterThanOrEqual)
            .expect("static iterator comparisons are supported");
        self.push_local_set(done_local);

        self.push_local_get(done_local);
        self.state.emission.output.instructions.push(0x04);
        self.state
            .emission
            .output
            .instructions
            .push(EMPTY_BLOCK_TYPE);
        self.push_control_frame();
        self.push_i32_const(JS_UNDEFINED_TAG);
        self.push_local_set(value_local);
        self.state.emission.output.instructions.push(0x05);
        if keys_only {
            self.push_local_get(current_index_local);
        } else if entries || map_entries {
            self.push_i32_const(JS_TYPEOF_OBJECT_TAG);
        } else if let Some(runtime_name) = runtime_name {
            if !self
                .emit_dynamic_runtime_array_slot_read_from_local(runtime_name, current_index_local)
                .expect("dynamic runtime array iterator reads are supported")
                && !self
                    .emit_dynamic_global_runtime_array_slot_read_from_local(
                        runtime_name,
                        current_index_local,
                    )
                    .expect("dynamic global runtime array iterator reads are supported")
            {
                self.emit_runtime_array_iterator_value_from_local(current_index_local, values)
                    .expect("static iterator values are supported");
            }
        } else {
            self.emit_runtime_array_iterator_value_from_local(current_index_local, values)
                .expect("static iterator values are supported");
        }
        self.push_local_set(value_local);
        self.push_local_get(current_index_local);
        self.push_i32_const(1);
        self.state.emission.output.instructions.push(0x6a);
        self.push_local_set(iterator_binding.index_local);
        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();
    }

    pub(in crate::backend::direct_wasm) fn update_runtime_iterator_step_static_array_entry_slots(
        &mut self,
        source: &IteratorSourceKind,
        current_index_local: u32,
        done_local: u32,
        entry_array: &IteratorStepEntryArrayBinding,
    ) {
        let no_runtime_name: Option<String> = None;
        let (values, runtime_name, key_runtime_name, value_runtime_name, map_entries) = match source
        {
            IteratorSourceKind::StaticArrayEntries {
                values,
                runtime_name,
                ..
            } => (
                values,
                runtime_name,
                &no_runtime_name,
                &no_runtime_name,
                false,
            ),
            IteratorSourceKind::StaticMapEntries {
                values,
                key_runtime_name,
                value_runtime_name,
                ..
            } => (
                values,
                &no_runtime_name,
                key_runtime_name,
                value_runtime_name,
                true,
            ),
            _ => return,
        };

        self.push_local_get(done_local);
        self.state.emission.output.instructions.push(0x04);
        self.state
            .emission
            .output
            .instructions
            .push(EMPTY_BLOCK_TYPE);
        self.push_control_frame();
        self.push_i32_const(JS_UNDEFINED_TAG);
        self.push_local_set(entry_array.index_local);
        self.push_i32_const(JS_UNDEFINED_TAG);
        self.push_local_set(entry_array.value_local);
        self.state.emission.output.instructions.push(0x05);
        if map_entries {
            let keys = values
                .iter()
                .map(|entry| match entry {
                    Some(Expression::Array(elements)) => {
                        elements.first().and_then(|element| match element {
                            ArrayElement::Expression(expression) => Some(expression.clone()),
                            ArrayElement::Spread(_) => None,
                        })
                    }
                    Some(value) => Some(value.clone()),
                    None => None,
                })
                .collect::<Vec<_>>();
            let map_values = values
                .iter()
                .map(|entry| match entry {
                    Some(Expression::Array(elements)) => elements
                        .get(1)
                        .and_then(|element| match element {
                            ArrayElement::Expression(expression) => Some(expression.clone()),
                            ArrayElement::Spread(_) => None,
                        })
                        .or(Some(Expression::Undefined)),
                    Some(value) => Some(value.clone()),
                    None => None,
                })
                .collect::<Vec<_>>();
            if let Some(runtime_name) = key_runtime_name {
                if !self
                    .emit_dynamic_runtime_array_slot_read_from_local(
                        runtime_name,
                        current_index_local,
                    )
                    .expect("dynamic map iterator key reads are supported")
                {
                    self.emit_runtime_array_iterator_value_from_local(current_index_local, &keys)
                        .expect("static map iterator keys are supported");
                }
            } else {
                self.emit_runtime_array_iterator_value_from_local(current_index_local, &keys)
                    .expect("static map iterator keys are supported");
            }
            self.push_local_set(entry_array.index_local);
            if let Some(runtime_name) = value_runtime_name {
                if !self
                    .emit_dynamic_runtime_array_slot_read_from_local(
                        runtime_name,
                        current_index_local,
                    )
                    .expect("dynamic map iterator value reads are supported")
                {
                    self.emit_runtime_array_iterator_value_from_local(
                        current_index_local,
                        &map_values,
                    )
                    .expect("static map iterator values are supported");
                }
            } else {
                self.emit_runtime_array_iterator_value_from_local(current_index_local, &map_values)
                    .expect("static map iterator values are supported");
            }
            self.push_local_set(entry_array.value_local);
        } else {
            self.push_local_get(current_index_local);
            self.push_local_set(entry_array.index_local);
            if let Some(runtime_name) = runtime_name {
                if !self
                    .emit_dynamic_runtime_array_slot_read_from_local(
                        runtime_name,
                        current_index_local,
                    )
                    .expect("dynamic runtime array entries iterator reads are supported")
                    && !self
                        .emit_dynamic_global_runtime_array_slot_read_from_local(
                            runtime_name,
                            current_index_local,
                        )
                        .expect("dynamic global runtime array entries iterator reads are supported")
                {
                    self.emit_runtime_array_iterator_value_from_local(current_index_local, values)
                        .expect("static entries iterator values are supported");
                }
            } else {
                self.emit_runtime_array_iterator_value_from_local(current_index_local, values)
                    .expect("static entries iterator values are supported");
            }
            self.push_local_set(entry_array.value_local);
        }
        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();
    }

    pub(super) fn update_runtime_iterator_step_typed_array_view(
        &mut self,
        iterator_binding: &mut ArrayIteratorBinding,
        current_index_local: u32,
        done_local: u32,
        value_local: u32,
    ) {
        let IteratorSourceKind::TypedArrayView { name: view_name } = &iterator_binding.source
        else {
            unreachable!("filtered by caller")
        };
        if let Some(current_index) = iterator_binding.static_index {
            iterator_binding.static_index = Some(current_index.saturating_add(1));
        }
        self.push_local_get(current_index_local);
        if let Some(view_length_local) = self
            .state
            .speculation
            .static_semantics
            .runtime_array_length_local(view_name)
        {
            self.push_local_get(view_length_local);
        } else {
            let static_length = self
                .typed_array_view_binding_for_name(view_name)
                .and_then(|view| self.typed_array_view_static_length(&view))
                .unwrap_or(0);
            self.push_i32_const(static_length as i32);
        }
        self.push_binary_op(BinaryOp::GreaterThanOrEqual)
            .expect("typed array iterator comparisons are supported");
        self.push_local_set(done_local);

        self.push_local_get(done_local);
        self.state.emission.output.instructions.push(0x04);
        self.state
            .emission
            .output
            .instructions
            .push(EMPTY_BLOCK_TYPE);
        self.push_control_frame();
        self.push_i32_const(JS_UNDEFINED_TAG);
        self.push_local_set(value_local);
        self.state.emission.output.instructions.push(0x05);
        if !self
            .emit_dynamic_runtime_array_slot_read_from_local(view_name, current_index_local)
            .expect("typed array iterator reads are supported")
            && !self
                .emit_dynamic_global_runtime_array_slot_read_from_local(
                    view_name,
                    current_index_local,
                )
                .expect("typed array iterator global reads are supported")
        {
            self.push_i32_const(JS_UNDEFINED_TAG);
        }
        self.push_local_set(value_local);
        self.push_local_get(current_index_local);
        self.push_i32_const(1);
        self.state.emission.output.instructions.push(0x6a);
        self.push_local_set(iterator_binding.index_local);
        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();
    }

    pub(super) fn update_runtime_iterator_step_direct_arguments(
        &mut self,
        iterator_binding: &mut ArrayIteratorBinding,
        current_index_local: u32,
        done_local: u32,
        value_local: u32,
    ) {
        let IteratorSourceKind::DirectArguments { tracked_prefix_len } = &iterator_binding.source
        else {
            unreachable!("filtered by caller")
        };
        iterator_binding.static_index = None;
        let effective_length_local = self.allocate_temp_local();
        if let Some(actual_argument_count_local) = self.state.parameters.actual_argument_count_local
        {
            self.push_local_get(actual_argument_count_local);
            self.push_i32_const(*tracked_prefix_len as i32);
            self.push_binary_op(BinaryOp::LessThanOrEqual)
                .expect("argument count comparisons are supported");
            self.state.emission.output.instructions.push(0x04);
            self.state
                .emission
                .output
                .instructions
                .push(EMPTY_BLOCK_TYPE);
            self.push_control_frame();
            self.push_local_get(actual_argument_count_local);
            self.push_local_set(effective_length_local);
            self.state.emission.output.instructions.push(0x05);
            self.push_i32_const(*tracked_prefix_len as i32);
            self.push_local_set(effective_length_local);
            self.state.emission.output.instructions.push(0x0b);
            self.pop_control_frame();
        } else {
            self.push_i32_const(*tracked_prefix_len as i32);
            self.push_local_set(effective_length_local);
        }

        self.push_local_get(current_index_local);
        self.push_local_get(effective_length_local);
        self.push_binary_op(BinaryOp::GreaterThanOrEqual)
            .expect("argument iterator comparisons are supported");
        self.push_local_set(done_local);

        self.push_local_get(done_local);
        self.state.emission.output.instructions.push(0x04);
        self.state
            .emission
            .output
            .instructions
            .push(EMPTY_BLOCK_TYPE);
        self.push_control_frame();
        self.push_i32_const(JS_UNDEFINED_TAG);
        self.push_local_set(value_local);
        self.state.emission.output.instructions.push(0x05);
        self.emit_dynamic_direct_arguments_property_read_from_local(current_index_local)
            .expect("direct arguments iteration reads are supported");
        self.push_local_set(value_local);
        self.push_local_get(current_index_local);
        self.push_i32_const(1);
        self.state.emission.output.instructions.push(0x6a);
        self.push_local_set(iterator_binding.index_local);
        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();
    }
}
