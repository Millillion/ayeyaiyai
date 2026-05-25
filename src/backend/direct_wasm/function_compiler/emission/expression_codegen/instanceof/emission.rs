use super::*;

impl<'a> FunctionCompiler<'a> {
    fn emit_instanceof_prototype_getter_call(
        &mut self,
        getter_binding: &LocalFunctionBinding,
        materialized_right: &Expression,
    ) -> DirectResult<()> {
        match getter_binding {
            LocalFunctionBinding::User(function_name) => {
                if let Some(user_function) = self.user_function(function_name).cloned() {
                    self.emit_user_function_call_with_new_target_and_this_expression_from_argument_locals(
                        &user_function,
                        &[],
                        0,
                        JS_UNDEFINED_TAG,
                        materialized_right,
                    )?;
                } else {
                    self.emit_numeric_expression(materialized_right)?;
                }
            }
            LocalFunctionBinding::Builtin(function_name) => {
                let getter_callee = Expression::Identifier(function_name.clone());
                if !self.emit_arguments_slot_accessor_call(&getter_callee, &[], 0, Some(&[]))? {
                    self.push_i32_const(JS_UNDEFINED_TAG);
                }
            }
        }
        self.state.emission.output.instructions.push(0x1a);
        Ok(())
    }

    fn last_instanceof_left_assignment_to_identifier<'b>(
        expression: &'b Expression,
        target_name: &str,
    ) -> Option<&'b Expression> {
        match expression {
            Expression::Assign { name, value } if name == target_name => Some(value),
            Expression::Sequence(expressions) => {
                expressions.iter().fold(None, |last, expression| {
                    Self::last_instanceof_left_assignment_to_identifier(expression, target_name)
                        .or(last)
                })
            }
            _ => None,
        }
    }

    pub(in crate::backend::direct_wasm) fn emit_instanceof_expression(
        &mut self,
        left: &Expression,
        right: &Expression,
    ) -> DirectResult<()> {
        let right_after_left_assignment = match right {
            Expression::Identifier(name) => {
                Self::last_instanceof_left_assignment_to_identifier(left, name).cloned()
            }
            _ => None,
        };
        let right_for_static_resolution = right_after_left_assignment.as_ref().unwrap_or(right);
        let has_instance_property = Expression::Member {
            object: Box::new(Expression::Identifier("Symbol".to_string())),
            property: Box::new(Expression::String("hasInstance".to_string())),
        };
        if let Some(function_binding) = self
            .resolve_member_function_binding(right_for_static_resolution, &has_instance_property)
        {
            let left_local = self.allocate_temp_local();
            self.emit_numeric_expression(left)?;
            self.push_local_set(left_local);
            let result_local = self.allocate_temp_local();
            match function_binding {
                LocalFunctionBinding::User(function_name) => {
                    let Some(user_function) = self.user_function(&function_name).cloned() else {
                        self.push_i32_const(0);
                        return Ok(());
                    };
                    let argument_locals = [left_local];
                    let static_argument_expressions = [self.materialize_static_expression(left)];
                    let static_return_truthy = self
                        .resolve_function_binding_static_return_expression_with_call_frame(
                            &LocalFunctionBinding::User(function_name.clone()),
                            &static_argument_expressions,
                            right_for_static_resolution,
                        )
                        .and_then(|return_value| {
                            self.resolve_static_boolean_expression(&return_value)
                        });
                    if let Some(capture_slots) =
                        self.resolve_member_function_capture_slots(right, &has_instance_property)
                    {
                        self.emit_user_function_call_with_new_target_and_this_expression_and_bound_captures_from_argument_locals(
                            &user_function,
                            &argument_locals,
                            1,
                            JS_UNDEFINED_TAG,
                            right_for_static_resolution,
                            &capture_slots,
                        )?;
                    } else {
                        self.emit_user_function_call_with_new_target_and_this_expression_from_argument_locals(
                            &user_function,
                            &argument_locals,
                            1,
                            JS_UNDEFINED_TAG,
                            right_for_static_resolution,
                        )?;
                    }
                    self.push_local_set(result_local);
                    self.sync_direct_arguments_assignments_from_static_user_call(
                        &user_function,
                        &static_argument_expressions,
                    );
                    if let Some(static_return_truthy) = static_return_truthy {
                        self.push_i32_const(static_return_truthy as i32);
                    } else {
                        self.emit_instanceof_truthy_from_local(result_local)?;
                    }
                    return Ok(());
                }
                LocalFunctionBinding::Builtin(_) => {
                    self.emit_numeric_expression(right)?;
                    self.state.emission.output.instructions.push(0x1a);
                    self.push_i32_const(0);
                    return Ok(());
                }
            }
        }

        let materialized_right = self.materialize_static_expression(right_for_static_resolution);
        if self.expression_is_known_non_object_value_for_instanceof(&materialized_right) {
            self.emit_numeric_expression(left)?;
            self.state.emission.output.instructions.push(0x1a);
            self.emit_numeric_expression(right)?;
            self.state.emission.output.instructions.push(0x1a);
            return self.emit_named_error_throw("TypeError");
        }
        if self.expression_is_known_object_like_value_for_instanceof(&materialized_right)
            && !self.expression_is_known_function_value_for_instanceof(&materialized_right)
        {
            self.emit_numeric_expression(left)?;
            self.state.emission.output.instructions.push(0x1a);
            self.emit_numeric_expression(right)?;
            self.state.emission.output.instructions.push(0x1a);
            return self.emit_named_error_throw("TypeError");
        }
        if self.expression_is_builtin_array_constructor(&materialized_right) {
            self.emit_numeric_expression(left)?;
            self.state.emission.output.instructions.push(0x1a);
            self.emit_numeric_expression(right)?;
            self.state.emission.output.instructions.push(0x1a);
            self.push_i32_const(if self.expression_is_known_array_value(left) {
                1
            } else {
                0
            });
            return Ok(());
        }

        if let Expression::Identifier(name) = &materialized_right {
            if let Some(expected_values) = native_error_instanceof_values(name) {
                let static_result = !self.expression_is_known_non_object_value_for_instanceof(left)
                    && self.expression_inherits_from_prototype_for_instanceof(
                        left,
                        &Self::prototype_member_expression(name),
                    );
                if std::env::var_os("AYY_TRACE_INSTANCEOF").is_some() {
                    eprintln!(
                        "instanceof:native_error left={left:?} right={right:?} materialized_right={materialized_right:?} static_result={static_result}"
                    );
                }
                let left_local = self.allocate_temp_local();
                self.emit_numeric_expression(left)?;
                self.push_local_set(left_local);
                self.emit_numeric_expression(right)?;
                self.state.emission.output.instructions.push(0x1a);
                if static_result {
                    self.push_i32_const(1);
                    return Ok(());
                }
                if expected_values.len() == 1 {
                    let expected_value = expected_values[0];
                    self.push_local_get(left_local);
                    self.push_i32_const(expected_value);
                    self.push_binary_op(BinaryOp::Equal)?;
                    return Ok(());
                }

                let matched_local = self.allocate_temp_local();
                self.push_i32_const(0);
                self.push_local_set(matched_local);
                for expected_value in expected_values {
                    self.push_local_get(left_local);
                    self.push_i32_const(expected_value);
                    self.push_binary_op(BinaryOp::Equal)?;
                    self.state.emission.output.instructions.push(0x04);
                    self.state
                        .emission
                        .output
                        .instructions
                        .push(EMPTY_BLOCK_TYPE);
                    self.push_control_frame();
                    self.push_i32_const(1);
                    self.push_local_set(matched_local);
                    self.state.emission.output.instructions.push(0x0b);
                    self.pop_control_frame();
                }
                self.push_local_get(matched_local);
                return Ok(());
            }
        }

        let prototype_property = Expression::String("prototype".to_string());
        if let Some(getter_binding) =
            self.resolve_member_getter_binding(&materialized_right, &prototype_property)
            && self
                .resolve_instanceof_getter_static_prototype_expression(
                    &getter_binding,
                    &materialized_right,
                )
                .is_none()
        {
            if self.expression_is_known_non_object_value_for_instanceof(left) {
                self.emit_numeric_expression(left)?;
                self.state.emission.output.instructions.push(0x1a);
                self.emit_numeric_expression(right)?;
                self.state.emission.output.instructions.push(0x1a);
                self.push_i32_const(0);
                return Ok(());
            }
            let left_local = self.allocate_temp_local();
            self.emit_numeric_expression(left)?;
            self.push_local_set(left_local);
            self.emit_instanceof_prototype_getter_call(&getter_binding, &materialized_right)?;
            self.emit_named_error_throw("TypeError")?;
            return Ok(());
        }

        if let Some(prototype_expression) =
            self.resolve_instanceof_prototype_expression(&materialized_right)
        {
            let materialized_prototype_expression =
                self.materialize_static_expression(&prototype_expression);
            if self.expression_is_known_non_object_value_for_instanceof(left) {
                self.emit_numeric_expression(left)?;
                self.state.emission.output.instructions.push(0x1a);
                self.emit_numeric_expression(right)?;
                self.state.emission.output.instructions.push(0x1a);
                self.push_i32_const(0);
                return Ok(());
            }
            if self.expression_is_known_non_object_value_for_instanceof(
                &materialized_prototype_expression,
            ) {
                self.emit_numeric_expression(left)?;
                self.state.emission.output.instructions.push(0x1a);
                self.emit_numeric_expression(right)?;
                self.state.emission.output.instructions.push(0x1a);
                return self.emit_named_error_throw("TypeError");
            }
            let left_local = self.allocate_temp_local();
            self.emit_numeric_expression(left)?;
            self.push_local_set(left_local);
            let static_result = if self.expression_is_known_non_object_value_for_instanceof(left) {
                false
            } else {
                self.expression_inherits_from_prototype_for_instanceof(left, &prototype_expression)
            };
            if std::env::var_os("AYY_TRACE_INSTANCEOF").is_some() {
                eprintln!(
                    "instanceof:prototype left={left:?} right={right:?} materialized_right={materialized_right:?} prototype={prototype_expression:?} static_result={static_result}"
                );
            }
            if let Some(getter_binding) = self.resolve_member_getter_binding(
                &materialized_right,
                &Expression::String("prototype".to_string()),
            ) {
                self.emit_instanceof_prototype_getter_call(&getter_binding, &materialized_right)?;
            } else {
                self.emit_numeric_expression(right)?;
                self.state.emission.output.instructions.push(0x1a);
            }
            self.push_i32_const(if static_result { 1 } else { 0 });
            return Ok(());
        }

        self.emit_numeric_expression(left)?;
        self.state.emission.output.instructions.push(0x1a);
        self.emit_numeric_expression(right)?;
        self.state.emission.output.instructions.push(0x1a);
        self.push_i32_const(0);
        Ok(())
    }
}
