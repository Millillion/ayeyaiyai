use super::*;

impl<'a> FunctionCompiler<'a> {
    fn expression_is_internal_update_target_temp(expression: &Expression, prefix: &str) -> bool {
        matches!(expression, Expression::Identifier(name) if name.starts_with(prefix))
    }

    pub(in crate::backend::direct_wasm) fn lowered_member_update_operand<'b>(
        &self,
        object: &Expression,
        property: &Expression,
        value: &'b Expression,
    ) -> Option<(BinaryOp, &'b Expression)> {
        let Expression::Binary { op, left, right } = value else {
            return None;
        };
        if !matches!(op, BinaryOp::Add | BinaryOp::Subtract) {
            return None;
        }
        if !matches!(right.as_ref(), Expression::Number(number) if *number == 1.0) {
            return None;
        }

        if matches!(left.as_ref(), Expression::Identifier(name) if name.starts_with("__ayy_postfix_previous_"))
        {
            return Some((*op, left.as_ref()));
        }

        let Expression::Member {
            object: left_object,
            property: left_property,
        } = left.as_ref()
        else {
            return None;
        };

        let uses_lowered_target_cache =
            Self::expression_is_internal_update_target_temp(object, "__ayy_target_object_")
                || Self::expression_is_internal_update_target_temp(
                    property,
                    "__ayy_target_property_",
                )
                || Self::expression_is_internal_update_target_temp(
                    left_object,
                    "__ayy_target_object_",
                )
                || Self::expression_is_internal_update_target_temp(
                    left_property,
                    "__ayy_target_property_",
                );
        if !uses_lowered_target_cache {
            return None;
        }

        let materialized_object = self.materialize_static_expression(object);
        let materialized_left_object = self.materialize_static_expression(left_object);
        let object_matches = static_expression_matches(left_object, object)
            || static_expression_matches(&materialized_left_object, object)
            || static_expression_matches(left_object, &materialized_object)
            || static_expression_matches(&materialized_left_object, &materialized_object);
        if !object_matches {
            return None;
        }

        let canonical_property = self.canonical_object_property_expression(property);
        let canonical_left_property = self.canonical_object_property_expression(left_property);
        let property_matches = static_expression_matches(left_property, property)
            || static_expression_matches(&canonical_left_property, property)
            || static_expression_matches(left_property, &canonical_property)
            || static_expression_matches(&canonical_left_property, &canonical_property);
        if !property_matches {
            return None;
        }

        Some((*op, left.as_ref()))
    }

    fn emit_arguments_slot_update_from_lowered_member_update(
        &mut self,
        index: u32,
        object: &Expression,
        property: &Expression,
        value: &Expression,
    ) -> DirectResult<bool> {
        let Some((op, previous_value)) =
            self.lowered_member_update_operand(object, property, value)
        else {
            return Ok(false);
        };
        let value_local = self.allocate_temp_local();
        self.emit_numeric_expression(previous_value)?;
        self.push_i32_const(1);
        self.push_binary_op(op)?;
        self.push_local_set(value_local);
        self.emit_arguments_slot_write_from_local(index, value_local)?;
        Ok(true)
    }

    pub(in crate::backend::direct_wasm) fn emit_arguments_or_restricted_member_assignment(
        &mut self,
        object: &Expression,
        property: &Expression,
        value: &Expression,
    ) -> DirectResult<bool> {
        if matches!(property, Expression::String(property_name) if property_name == "callee" || property_name == "length")
        {
            let Expression::String(property_name) = property else {
                unreachable!("filtered above");
            };
            if self.is_direct_arguments_object(object) {
                let temp_local = self.allocate_temp_local();
                self.emit_numeric_expression(value)?;
                self.push_local_set(temp_local);
                if property_name == "callee" && self.state.speculation.execution_context.strict_mode
                {
                    self.push_local_get(temp_local);
                    self.state.emission.output.instructions.push(0x1a);
                    return self.emit_error_throw().map(|_| true);
                }
                self.apply_current_arguments_effect(
                    property_name,
                    ArgumentsPropertyEffect::Assign(value.clone()),
                );
                self.push_local_get(temp_local);
                return Ok(true);
            }
            if let Some(arguments_binding) = self.resolve_arguments_binding_from_expression(object)
            {
                self.emit_numeric_expression(object)?;
                self.state.emission.output.instructions.push(0x1a);
                self.emit_numeric_expression(property)?;
                self.state.emission.output.instructions.push(0x1a);
                let temp_local = self.allocate_temp_local();
                self.emit_numeric_expression(value)?;
                self.push_local_set(temp_local);
                if property_name == "callee" && arguments_binding.strict {
                    self.push_local_get(temp_local);
                    self.state.emission.output.instructions.push(0x1a);
                    return self.emit_error_throw().map(|_| true);
                }
                self.update_named_arguments_binding_effect(
                    object,
                    property_name,
                    ArgumentsPropertyEffect::Assign(value.clone()),
                );
                self.push_local_get(temp_local);
                return Ok(true);
            }
        }

        if self.is_direct_arguments_object(object) {
            if let Some(index) = argument_index_from_expression(property) {
                if self.emit_arguments_slot_update_from_lowered_member_update(
                    index, object, property, value,
                )? {
                    return Ok(true);
                }
                self.emit_arguments_slot_write(index, value)?;
                return Ok(true);
            }
            self.emit_dynamic_direct_arguments_property_write(property, value)?;
            return Ok(true);
        }

        if self.is_restricted_function_property(object, property) {
            self.emit_numeric_expression(object)?;
            self.state.emission.output.instructions.push(0x1a);
            self.emit_numeric_expression(property)?;
            self.state.emission.output.instructions.push(0x1a);
            self.emit_numeric_expression(value)?;
            self.state.emission.output.instructions.push(0x1a);
            self.emit_named_error_throw("TypeError")?;
            return Ok(true);
        }

        Ok(false)
    }
}
