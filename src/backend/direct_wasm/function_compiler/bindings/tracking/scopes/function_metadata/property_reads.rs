use super::*;

impl<'a> FunctionCompiler<'a> {
    pub(in crate::backend::direct_wasm) fn emit_runtime_user_function_property_read(
        &mut self,
        object: &Expression,
        property: &Expression,
    ) -> DirectResult<bool> {
        if let Expression::String(property_name) = property {
            return self.emit_runtime_user_function_static_property_read(object, property_name);
        }

        self.emit_runtime_user_function_dynamic_property_read(object, property)
    }

    fn emit_runtime_user_function_static_property_read(
        &mut self,
        object: &Expression,
        property_name: &str,
    ) -> DirectResult<bool> {
        if property_name == "caller" || property_name == "arguments" {
            let restricted_function_values = self
                .user_functions()
                .into_iter()
                .filter(|user_function| user_function.is_arrow() || user_function.strict)
                .map(|user_function| user_function_runtime_value(&user_function))
                .collect::<Vec<_>>();
            if restricted_function_values.is_empty() {
                return Ok(false);
            }

            let object_local = self.allocate_temp_local();
            self.emit_numeric_expression(object)?;
            self.push_local_set(object_local);
            for runtime_value in restricted_function_values {
                self.push_local_get(object_local);
                self.push_i32_const(runtime_value);
                self.push_binary_op(BinaryOp::Equal)?;
                self.state.emission.output.instructions.push(0x04);
                self.state
                    .emission
                    .output
                    .instructions
                    .push(EMPTY_BLOCK_TYPE);
                self.push_control_frame();
                self.emit_named_error_throw("TypeError")?;
                self.state.emission.output.instructions.push(0x0b);
                self.pop_control_frame();
            }
            self.push_i32_const(JS_UNDEFINED_TAG);
            return Ok(true);
        }

        let mut candidates = Vec::new();
        for user_function in self.user_functions() {
            let Some(value) =
                self.runtime_user_function_property_value(&user_function, property_name)
            else {
                continue;
            };
            candidates.push((user_function_runtime_value(&user_function), value));
        }
        if candidates.is_empty() {
            return Ok(false);
        }

        let object_local = self.allocate_temp_local();
        let result_local = self.allocate_temp_local();
        let matched_local = self.allocate_temp_local();
        self.emit_numeric_expression(object)?;
        self.push_local_set(object_local);
        self.push_i32_const(JS_UNDEFINED_TAG);
        self.push_local_set(result_local);
        self.push_i32_const(0);
        self.push_local_set(matched_local);

        for (runtime_value, value) in candidates {
            self.push_local_get(object_local);
            self.push_i32_const(runtime_value);
            self.push_binary_op(BinaryOp::Equal)?;
            self.state.emission.output.instructions.push(0x04);
            self.state
                .emission
                .output
                .instructions
                .push(EMPTY_BLOCK_TYPE);
            self.push_control_frame();
            self.emit_numeric_expression(&value)?;
            self.push_local_set(result_local);
            self.push_i32_const(1);
            self.push_local_set(matched_local);
            self.state.emission.output.instructions.push(0x0b);
            self.pop_control_frame();
        }

        self.push_local_get(matched_local);
        self.state.emission.output.instructions.push(0x04);
        self.state.emission.output.instructions.push(I32_TYPE);
        self.push_control_frame();
        self.push_local_get(result_local);
        self.state.emission.output.instructions.push(0x05);
        self.push_i32_const(JS_TYPEOF_OBJECT_TAG);
        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();
        Ok(true)
    }

    fn emit_runtime_user_function_dynamic_property_read(
        &mut self,
        object: &Expression,
        property: &Expression,
    ) -> DirectResult<bool> {
        let restricted_function_values = self
            .user_functions()
            .into_iter()
            .filter(|user_function| user_function.is_arrow() || user_function.strict)
            .map(|user_function| user_function_runtime_value(&user_function))
            .collect::<Vec<_>>();

        let mut candidates = Vec::new();
        for property_name in ["name", "length"] {
            for user_function in self.user_functions() {
                let Some(value) =
                    self.runtime_user_function_property_value(&user_function, property_name)
                else {
                    continue;
                };
                candidates.push((
                    Expression::String(property_name.to_string()),
                    user_function_runtime_value(&user_function),
                    value,
                ));
            }
        }

        if restricted_function_values.is_empty() && candidates.is_empty() {
            return Ok(false);
        }

        let object_local = self.allocate_temp_local();
        let property_local = self.allocate_temp_local();
        let result_local = self.allocate_temp_local();
        let matched_local = self.allocate_temp_local();

        self.emit_numeric_expression(object)?;
        self.push_local_set(object_local);
        self.emit_numeric_expression(property)?;
        self.push_local_set(property_local);
        self.push_i32_const(JS_UNDEFINED_TAG);
        self.push_local_set(result_local);
        self.push_i32_const(0);
        self.push_local_set(matched_local);

        for restricted_property in ["caller", "arguments"] {
            let property_key = Expression::String(restricted_property.to_string());
            self.push_local_get(property_local);
            self.emit_numeric_expression(&property_key)?;
            self.push_binary_op(BinaryOp::Equal)?;
            self.state.emission.output.instructions.push(0x04);
            self.state
                .emission
                .output
                .instructions
                .push(EMPTY_BLOCK_TYPE);
            self.push_control_frame();
            for runtime_value in &restricted_function_values {
                self.push_local_get(object_local);
                self.push_i32_const(*runtime_value);
                self.push_binary_op(BinaryOp::Equal)?;
                self.state.emission.output.instructions.push(0x04);
                self.state
                    .emission
                    .output
                    .instructions
                    .push(EMPTY_BLOCK_TYPE);
                self.push_control_frame();
                self.emit_named_error_throw("TypeError")?;
                self.state.emission.output.instructions.push(0x0b);
                self.pop_control_frame();
            }
            self.state.emission.output.instructions.push(0x0b);
            self.pop_control_frame();
        }

        for (property_key, runtime_value, value) in candidates {
            self.push_local_get(object_local);
            self.push_i32_const(runtime_value);
            self.push_binary_op(BinaryOp::Equal)?;
            self.state.emission.output.instructions.push(0x04);
            self.state
                .emission
                .output
                .instructions
                .push(EMPTY_BLOCK_TYPE);
            self.push_control_frame();
            self.push_local_get(property_local);
            self.emit_numeric_expression(&property_key)?;
            self.push_binary_op(BinaryOp::Equal)?;
            self.state.emission.output.instructions.push(0x04);
            self.state
                .emission
                .output
                .instructions
                .push(EMPTY_BLOCK_TYPE);
            self.push_control_frame();
            self.emit_numeric_expression(&value)?;
            self.push_local_set(result_local);
            self.push_i32_const(1);
            self.push_local_set(matched_local);
            self.state.emission.output.instructions.push(0x0b);
            self.pop_control_frame();
            self.state.emission.output.instructions.push(0x0b);
            self.pop_control_frame();
        }

        self.push_local_get(matched_local);
        self.state.emission.output.instructions.push(0x04);
        self.state.emission.output.instructions.push(I32_TYPE);
        self.push_control_frame();
        self.push_local_get(result_local);
        self.state.emission.output.instructions.push(0x05);
        self.push_i32_const(JS_TYPEOF_OBJECT_TAG);
        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();
        Ok(true)
    }
}
