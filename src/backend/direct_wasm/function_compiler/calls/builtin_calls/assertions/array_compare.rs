use super::*;

impl<'a> FunctionCompiler<'a> {
    fn compare_array_static_operand_safe(&self, expression: &Expression) -> bool {
        if inline_summary_side_effect_free_expression(expression) {
            return true;
        }
        if let Expression::Call { callee, arguments } = expression
            && self
                .static_builtin_object_array_call_binding(callee, arguments)
                .is_some()
        {
            return true;
        }
        if matches!(expression, Expression::Call { callee, .. } if matches!(
            callee.as_ref(),
            Expression::Identifier(name) if name == "ToNumbers"
        )) && self
            .resolve_array_binding_from_expression(expression)
            .is_some()
        {
            return true;
        }
        matches!(
            expression,
            Expression::Call { callee, arguments }
                if matches!(
                    callee.as_ref(),
                    Expression::Member { object, property }
                        if matches!(object.as_ref(), Expression::Identifier(name) if name == "Array")
                            && matches!(property.as_ref(), Expression::String(name) if name == "from")
                ) && matches!(
                    arguments.as_slice(),
                    [CallArgument::Expression(target) | CallArgument::Spread(target), ..]
                        if self.static_typed_array_values_from_expression(target).is_some()
                )
        )
    }

    pub(in crate::backend::direct_wasm) fn emit_assert_compare_array_call(
        &mut self,
        arguments: &[CallArgument],
    ) -> DirectResult<bool> {
        let [
            CallArgument::Expression(actual),
            CallArgument::Expression(expected),
            rest @ ..,
        ] = arguments
        else {
            return Ok(false);
        };
        let Some(expected_binding) = self.resolve_array_binding_from_expression(expected) else {
            return Ok(false);
        };

        if self.compare_array_static_operand_safe(actual)
            && self.compare_array_static_operand_safe(expected)
            && !self.expression_uses_runtime_array_state(actual)
            && !self.expression_uses_runtime_array_state(expected)
            && rest.iter().all(|argument| match argument {
                CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                    self.compare_array_static_operand_safe(expression)
                }
            })
            && let Some(actual_binding) = self.resolve_array_binding_from_expression(actual)
        {
            if std::env::var_os("AYY_TRACE_COMPARE_ARRAY").is_some() {
                eprintln!(
                    "assert_compare_array:static actual={:?} expected={:?}",
                    actual_binding.values, expected_binding.values
                );
            }
            if !self.array_bindings_equal(&actual_binding, &expected_binding) {
                self.emit_error_throw()?;
            } else {
                self.push_i32_const(JS_UNDEFINED_TAG);
            }
            return Ok(true);
        }

        self.emit_numeric_expression(actual)?;
        self.state.emission.output.instructions.push(0x1a);
        self.emit_numeric_expression(expected)?;
        self.state.emission.output.instructions.push(0x1a);
        for argument in rest {
            match argument {
                CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                    self.emit_numeric_expression(expression)?;
                    self.state.emission.output.instructions.push(0x1a);
                }
            }
        }

        if self.has_current_user_function()
            && matches!(
                actual,
                Expression::Identifier(_) | Expression::Member { .. }
            )
            && self
                .runtime_array_binding_name_for_expression(actual)
                .is_some()
            && self
                .runtime_array_binding_name_for_expression(expected)
                .is_some()
        {
            return self.emit_runtime_assert_compare_arrays(actual, expected);
        }

        if matches!(
            actual,
            Expression::Identifier(_) | Expression::Member { .. }
        ) && self
            .runtime_array_binding_name_for_expression(actual)
            .is_some()
        {
            return self
                .emit_runtime_assert_compare_array_against_expected(actual, &expected_binding);
        }

        if self.has_current_user_function()
            && matches!(
                actual,
                Expression::Identifier(_) | Expression::Member { .. }
            )
        {
            return Ok(false);
        }

        if matches!(
            actual,
            Expression::Identifier(_) | Expression::Member { .. }
        ) {
            return self
                .emit_runtime_assert_compare_array_against_expected(actual, &expected_binding);
        }

        let Some(actual_binding) = self.resolve_array_binding_from_expression(actual) else {
            return Ok(false);
        };
        if std::env::var_os("AYY_TRACE_COMPARE_ARRAY").is_some() {
            eprintln!(
                "assert_compare_array:actual={:?} expected={:?}",
                actual_binding.values, expected_binding.values
            );
        }
        if !self.array_bindings_equal(&actual_binding, &expected_binding) {
            self.emit_error_throw()?;
        } else {
            self.push_i32_const(JS_UNDEFINED_TAG);
        }
        Ok(true)
    }

    pub(in crate::backend::direct_wasm) fn emit_runtime_assert_compare_array_against_expected(
        &mut self,
        actual: &Expression,
        expected_binding: &ArrayValueBinding,
    ) -> DirectResult<bool> {
        let mismatch_local = self.allocate_temp_local();
        self.push_i32_const(0);
        self.push_local_set(mismatch_local);

        self.emit_numeric_expression(&Expression::Member {
            object: Box::new(actual.clone()),
            property: Box::new(Expression::String("length".to_string())),
        })?;
        self.push_i32_const(expected_binding.values.len() as i32);
        self.push_binary_op(BinaryOp::NotEqual)?;
        self.state.emission.output.instructions.push(0x04);
        self.state
            .emission
            .output
            .instructions
            .push(EMPTY_BLOCK_TYPE);
        self.push_control_frame();
        self.push_i32_const(1);
        self.push_local_set(mismatch_local);
        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();

        for (index, expected_value) in expected_binding.values.iter().enumerate() {
            self.emit_numeric_expression(&Expression::Member {
                object: Box::new(actual.clone()),
                property: Box::new(Expression::Number(index as f64)),
            })?;
            self.emit_numeric_expression(&expected_value.clone().unwrap_or(Expression::Undefined))?;
            self.push_binary_op(BinaryOp::NotEqual)?;
            self.state.emission.output.instructions.push(0x04);
            self.state
                .emission
                .output
                .instructions
                .push(EMPTY_BLOCK_TYPE);
            self.push_control_frame();
            self.push_i32_const(1);
            self.push_local_set(mismatch_local);
            self.state.emission.output.instructions.push(0x0b);
            self.pop_control_frame();
        }

        self.push_local_get(mismatch_local);
        self.state.emission.output.instructions.push(0x04);
        self.state
            .emission
            .output
            .instructions
            .push(EMPTY_BLOCK_TYPE);
        self.push_control_frame();
        self.emit_error_throw()?;
        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();
        self.push_i32_const(JS_UNDEFINED_TAG);
        Ok(true)
    }

    fn emit_runtime_assert_compare_arrays(
        &mut self,
        actual: &Expression,
        expected: &Expression,
    ) -> DirectResult<bool> {
        let mismatch_local = self.emit_runtime_compare_arrays_mismatch_local(actual, expected)?;
        self.push_local_get(mismatch_local);
        self.state.emission.output.instructions.push(0x04);
        self.state
            .emission
            .output
            .instructions
            .push(EMPTY_BLOCK_TYPE);
        self.push_control_frame();
        self.emit_error_throw()?;
        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();
        self.push_i32_const(JS_UNDEFINED_TAG);
        Ok(true)
    }

    pub(in crate::backend::direct_wasm) fn emit_compare_array_call(
        &mut self,
        arguments: &[CallArgument],
    ) -> DirectResult<bool> {
        let [
            CallArgument::Expression(actual),
            CallArgument::Expression(expected),
            rest @ ..,
        ] = arguments
        else {
            return Ok(false);
        };
        let Some(expected_binding) = self.resolve_array_binding_from_expression(expected) else {
            return Ok(false);
        };

        if self.compare_array_static_operand_safe(actual)
            && self.compare_array_static_operand_safe(expected)
            && !self.expression_uses_runtime_array_state(actual)
            && !self.expression_uses_runtime_array_state(expected)
            && rest.iter().all(|argument| match argument {
                CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                    self.compare_array_static_operand_safe(expression)
                }
            })
            && let Some(actual_binding) = self.resolve_array_binding_from_expression(actual)
        {
            self.push_i32_const(
                self.array_bindings_equal(&actual_binding, &expected_binding) as i32,
            );
            return Ok(true);
        }

        self.emit_numeric_expression(actual)?;
        self.state.emission.output.instructions.push(0x1a);
        self.emit_numeric_expression(expected)?;
        self.state.emission.output.instructions.push(0x1a);
        for argument in rest {
            match argument {
                CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                    self.emit_numeric_expression(expression)?;
                    self.state.emission.output.instructions.push(0x1a);
                }
            }
        }

        if self.has_current_user_function()
            && matches!(
                actual,
                Expression::Identifier(_) | Expression::Member { .. }
            )
            && self
                .runtime_array_binding_name_for_expression(actual)
                .is_some()
            && self
                .runtime_array_binding_name_for_expression(expected)
                .is_some()
        {
            let mismatch_local =
                self.emit_runtime_compare_arrays_mismatch_local(actual, expected)?;
            self.push_local_get(mismatch_local);
            self.push_i32_const(0);
            self.push_binary_op(BinaryOp::Equal)?;
            return Ok(true);
        }

        if matches!(
            actual,
            Expression::Identifier(_) | Expression::Member { .. }
        ) && self
            .runtime_array_binding_name_for_expression(actual)
            .is_some()
        {
            self.push_i32_const(1);
            let result_local = self.allocate_temp_local();
            self.push_local_set(result_local);

            self.emit_numeric_expression(&Expression::Member {
                object: Box::new(actual.clone()),
                property: Box::new(Expression::String("length".to_string())),
            })?;
            self.push_i32_const(expected_binding.values.len() as i32);
            self.push_binary_op(BinaryOp::NotEqual)?;
            self.state.emission.output.instructions.push(0x04);
            self.state
                .emission
                .output
                .instructions
                .push(EMPTY_BLOCK_TYPE);
            self.push_control_frame();
            self.push_i32_const(0);
            self.push_local_set(result_local);
            self.state.emission.output.instructions.push(0x0b);
            self.pop_control_frame();

            for (index, expected_value) in expected_binding.values.iter().enumerate() {
                self.emit_numeric_expression(&Expression::Member {
                    object: Box::new(actual.clone()),
                    property: Box::new(Expression::Number(index as f64)),
                })?;
                self.emit_numeric_expression(
                    &expected_value.clone().unwrap_or(Expression::Undefined),
                )?;
                self.push_binary_op(BinaryOp::NotEqual)?;
                self.state.emission.output.instructions.push(0x04);
                self.state
                    .emission
                    .output
                    .instructions
                    .push(EMPTY_BLOCK_TYPE);
                self.push_control_frame();
                self.push_i32_const(0);
                self.push_local_set(result_local);
                self.state.emission.output.instructions.push(0x0b);
                self.pop_control_frame();
            }

            self.push_local_get(result_local);
            return Ok(true);
        }

        if self.has_current_user_function()
            && matches!(
                actual,
                Expression::Identifier(_) | Expression::Member { .. }
            )
        {
            return Ok(false);
        }

        if matches!(
            actual,
            Expression::Identifier(_) | Expression::Member { .. }
        ) {
            self.push_i32_const(1);
            let result_local = self.allocate_temp_local();
            self.push_local_set(result_local);

            self.emit_numeric_expression(&Expression::Member {
                object: Box::new(actual.clone()),
                property: Box::new(Expression::String("length".to_string())),
            })?;
            self.push_i32_const(expected_binding.values.len() as i32);
            self.push_binary_op(BinaryOp::NotEqual)?;
            self.state.emission.output.instructions.push(0x04);
            self.state
                .emission
                .output
                .instructions
                .push(EMPTY_BLOCK_TYPE);
            self.push_control_frame();
            self.push_i32_const(0);
            self.push_local_set(result_local);
            self.state.emission.output.instructions.push(0x0b);
            self.pop_control_frame();

            for (index, expected_value) in expected_binding.values.iter().enumerate() {
                self.emit_numeric_expression(&Expression::Member {
                    object: Box::new(actual.clone()),
                    property: Box::new(Expression::Number(index as f64)),
                })?;
                self.emit_numeric_expression(
                    &expected_value.clone().unwrap_or(Expression::Undefined),
                )?;
                self.push_binary_op(BinaryOp::NotEqual)?;
                self.state.emission.output.instructions.push(0x04);
                self.state
                    .emission
                    .output
                    .instructions
                    .push(EMPTY_BLOCK_TYPE);
                self.push_control_frame();
                self.push_i32_const(0);
                self.push_local_set(result_local);
                self.state.emission.output.instructions.push(0x0b);
                self.pop_control_frame();
            }

            self.push_local_get(result_local);
            return Ok(true);
        }

        let Some(actual_binding) = self.resolve_array_binding_from_expression(actual) else {
            return Ok(false);
        };
        if std::env::var_os("AYY_TRACE_COMPARE_ARRAY").is_some() {
            eprintln!(
                "compare_array:actual={:?} expected={:?}",
                actual_binding.values, expected_binding.values
            );
        }
        self.push_i32_const(
            if self.array_bindings_equal(&actual_binding, &expected_binding) {
                1
            } else {
                0
            },
        );
        Ok(true)
    }

    fn emit_runtime_compare_arrays_mismatch_local(
        &mut self,
        actual: &Expression,
        expected: &Expression,
    ) -> DirectResult<u32> {
        let mismatch_local = self.allocate_temp_local();
        let actual_length_local = self.allocate_temp_local();
        let expected_length_local = self.allocate_temp_local();

        self.push_i32_const(0);
        self.push_local_set(mismatch_local);

        self.emit_numeric_expression(&Expression::Member {
            object: Box::new(actual.clone()),
            property: Box::new(Expression::String("length".to_string())),
        })?;
        self.push_local_set(actual_length_local);
        self.emit_numeric_expression(&Expression::Member {
            object: Box::new(expected.clone()),
            property: Box::new(Expression::String("length".to_string())),
        })?;
        self.push_local_set(expected_length_local);

        self.push_local_get(actual_length_local);
        self.push_local_get(expected_length_local);
        self.push_binary_op(BinaryOp::NotEqual)?;
        self.state.emission.output.instructions.push(0x04);
        self.state
            .emission
            .output
            .instructions
            .push(EMPTY_BLOCK_TYPE);
        self.push_control_frame();
        self.push_i32_const(1);
        self.push_local_set(mismatch_local);
        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();

        for index in 0..TRACKED_ARRAY_SLOT_LIMIT {
            self.push_local_get(actual_length_local);
            self.push_i32_const(index as i32);
            self.push_binary_op(BinaryOp::GreaterThan)?;
            self.state.emission.output.instructions.push(0x04);
            self.state
                .emission
                .output
                .instructions
                .push(EMPTY_BLOCK_TYPE);
            self.push_control_frame();
            self.emit_numeric_expression(&Expression::Member {
                object: Box::new(actual.clone()),
                property: Box::new(Expression::Number(index as f64)),
            })?;
            self.emit_numeric_expression(&Expression::Member {
                object: Box::new(expected.clone()),
                property: Box::new(Expression::Number(index as f64)),
            })?;
            self.push_binary_op(BinaryOp::NotEqual)?;
            self.state.emission.output.instructions.push(0x04);
            self.state
                .emission
                .output
                .instructions
                .push(EMPTY_BLOCK_TYPE);
            self.push_control_frame();
            self.push_i32_const(1);
            self.push_local_set(mismatch_local);
            self.state.emission.output.instructions.push(0x0b);
            self.pop_control_frame();
            self.state.emission.output.instructions.push(0x0b);
            self.pop_control_frame();
        }

        Ok(mismatch_local)
    }
}
