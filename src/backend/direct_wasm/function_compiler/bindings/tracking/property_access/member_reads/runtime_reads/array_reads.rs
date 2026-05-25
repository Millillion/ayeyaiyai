use super::*;

fn static_array_property_is_known_non_index(property: &Expression) -> bool {
    match property {
        Expression::String(text) => {
            text != "length" && argument_index_from_expression(property).is_none()
        }
        Expression::Number(_) => argument_index_from_expression(property).is_none(),
        _ => false,
    }
}

impl<'a> FunctionCompiler<'a> {
    fn emit_dynamic_static_array_member_read(
        &mut self,
        object: &Expression,
        property: &Expression,
        array_binding: &ArrayValueBinding,
    ) -> DirectResult<bool> {
        if static_array_property_is_known_non_index(property) {
            return Ok(false);
        }

        let property_local = self.allocate_temp_local();
        self.emit_numeric_expression(property)?;
        self.push_local_set(property_local);

        if let Some(binding_name) = self.runtime_array_binding_name_for_expression(object) {
            if self.emit_dynamic_global_runtime_array_slot_read_from_local(
                &binding_name,
                property_local,
            )? {
                return Ok(true);
            }
            if self
                .emit_dynamic_runtime_array_slot_read_from_local(&binding_name, property_local)?
            {
                return Ok(true);
            }
        }

        let mut open_frames = 0;
        for (index, value) in array_binding.values.iter().enumerate() {
            self.push_local_get(property_local);
            self.push_i32_const(index as i32);
            self.push_binary_op(BinaryOp::Equal)?;
            self.state.emission.output.instructions.push(0x04);
            self.state.emission.output.instructions.push(I32_TYPE);
            self.push_control_frame();
            open_frames += 1;
            if let Some(value) = value {
                self.emit_numeric_expression(value)?;
            } else {
                self.push_i32_const(JS_UNDEFINED_TAG);
            }
            self.state.emission.output.instructions.push(0x05);
        }

        self.push_i32_const(JS_UNDEFINED_TAG);
        for _ in 0..open_frames {
            self.state.emission.output.instructions.push(0x0b);
            self.pop_control_frame();
        }
        Ok(true)
    }

    pub(in crate::backend::direct_wasm) fn emit_runtime_array_member_read(
        &mut self,
        object: &Expression,
        static_array_property: &Expression,
    ) -> DirectResult<bool> {
        if static_array_property_is_known_non_index(static_array_property) {
            return Ok(false);
        }

        if let Some(binding_name) = self.runtime_array_binding_name_for_expression(object)
            && let Some(index) = argument_index_from_expression(static_array_property)
        {
            if self.emit_global_runtime_array_slot_read(&binding_name, index)? {
                return Ok(true);
            }
            if self.emit_runtime_array_slot_read(&binding_name, index)? {
                return Ok(true);
            }
            if let Some(array_binding) = self.resolve_array_binding_from_expression(object)
                && let Some(Some(value)) = array_binding.values.get(index as usize)
            {
                self.emit_numeric_expression(value)?;
                return Ok(true);
            }
        }

        if matches!(static_array_property, Expression::String(text) if text == "length") {
            if let Some(binding_name) = self.runtime_array_binding_name_for_expression(object)
                && self.emit_global_runtime_array_length_read(&binding_name)
            {
                if std::env::var_os("AYY_TRACE_MEMBER_READS").is_some() {
                    eprintln!(
                        "runtime_array_read:length_global object={object:?} binding={binding_name}"
                    );
                }
                return Ok(true);
            }
        }

        if let Some(binding_name) = self.runtime_array_binding_name_for_expression(object)
            && argument_index_from_expression(static_array_property).is_none()
            && !static_array_property_is_known_non_index(static_array_property)
            && !matches!(static_array_property, Expression::String(text) if text == "length")
            && !binding_name.starts_with("__ayy_for_in_keys_")
        {
            let property_local = self.allocate_temp_local();
            self.emit_numeric_expression(static_array_property)?;
            self.push_local_set(property_local);
            if self.emit_dynamic_global_runtime_array_slot_read_from_local(
                &binding_name,
                property_local,
            )? {
                return Ok(true);
            }
            if self
                .emit_dynamic_runtime_array_slot_read_from_local(&binding_name, property_local)?
            {
                return Ok(true);
            }
        }

        let array_binding = self.resolve_array_binding_from_expression(object);
        if matches!(static_array_property, Expression::String(text) if text == "length")
            && let Some(binding_name) = self.runtime_array_binding_name_for_expression(object)
            && self.emit_global_runtime_array_length_read(&binding_name)
        {
            if std::env::var_os("AYY_TRACE_MEMBER_READS").is_some() {
                eprintln!(
                    "runtime_array_read:length_global object={object:?} binding={binding_name}"
                );
            }
            return Ok(true);
        }
        let Some(array_binding) = array_binding else {
            if std::env::var_os("AYY_TRACE_MEMBER_READS").is_some() {
                eprintln!("runtime_array_read:no_static_binding object={object:?}");
            }
            return Ok(false);
        };
        if matches!(static_array_property, Expression::String(text) if text == "length") {
            if let Some(length_local) = self.runtime_array_length_local_for_expression(object) {
                if std::env::var_os("AYY_TRACE_MEMBER_READS").is_some() {
                    let binding_name = self.runtime_array_binding_name_for_expression(object);
                    eprintln!(
                        "runtime_array_read:length_local object={object:?} binding={binding_name:?} local={length_local}"
                    );
                }
                self.push_local_get(length_local);
            } else {
                if std::env::var_os("AYY_TRACE_MEMBER_READS").is_some() {
                    eprintln!(
                        "runtime_array_read:length_static object={object:?} len={}",
                        array_binding.values.len()
                    );
                }
                self.push_i32_const(array_binding.values.len() as i32);
            }
            return Ok(true);
        }
        if let Some(index) = argument_index_from_expression(static_array_property) {
            if let Some(binding_name) = self.runtime_array_binding_name_for_expression(object)
                && self.emit_global_runtime_array_slot_read(&binding_name, index)?
            {
                return Ok(true);
            }
            if let Some(binding_name) = self.runtime_array_binding_name_for_expression(object)
                && self.emit_runtime_array_slot_read(&binding_name, index)?
            {
                return Ok(true);
            }
            if let Some(Some(value)) = array_binding.values.get(index as usize) {
                self.emit_numeric_expression(value)?;
            } else {
                self.push_i32_const(JS_UNDEFINED_TAG);
            }
            return Ok(true);
        }

        return self.emit_dynamic_static_array_member_read(
            object,
            static_array_property,
            &array_binding,
        );
    }
}
