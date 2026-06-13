use super::*;

fn logical_expression_references_internal_iterator_temp(expression: &Expression) -> bool {
    match expression {
        Expression::Identifier(name) => {
            name.starts_with("__ayy_array_step_")
                || name.starts_with("__ayy_array_iter_value_")
                || name.starts_with("__ayy_array_iter_done_")
                || name.starts_with("__ayy_for_of_step_")
                || name.starts_with("__ayy_for_of_iter_value_")
                || name.starts_with("__ayy_for_of_iter_done_")
                || name.starts_with("__ayy_binding_value_")
        }
        Expression::Array(elements) => elements.iter().any(|element| match element {
            ArrayElement::Expression(value) | ArrayElement::Spread(value) => {
                logical_expression_references_internal_iterator_temp(value)
            }
        }),
        Expression::Object(entries) => entries.iter().any(|entry| match entry {
            ObjectEntry::Data { key, value } => {
                logical_expression_references_internal_iterator_temp(key)
                    || logical_expression_references_internal_iterator_temp(value)
            }
            ObjectEntry::Getter { key, getter } => {
                logical_expression_references_internal_iterator_temp(key)
                    || logical_expression_references_internal_iterator_temp(getter)
            }
            ObjectEntry::Setter { key, setter } => {
                logical_expression_references_internal_iterator_temp(key)
                    || logical_expression_references_internal_iterator_temp(setter)
            }
            ObjectEntry::Spread(value) => {
                logical_expression_references_internal_iterator_temp(value)
            }
        }),
        Expression::Binary { left, right, .. } => {
            logical_expression_references_internal_iterator_temp(left)
                || logical_expression_references_internal_iterator_temp(right)
        }
        Expression::Conditional {
            condition,
            then_expression,
            else_expression,
        } => {
            logical_expression_references_internal_iterator_temp(condition)
                || logical_expression_references_internal_iterator_temp(then_expression)
                || logical_expression_references_internal_iterator_temp(else_expression)
        }
        Expression::Member { object, property } => {
            logical_expression_references_internal_iterator_temp(object)
                || logical_expression_references_internal_iterator_temp(property)
        }
        Expression::Unary { expression, .. }
        | Expression::Await(expression)
        | Expression::EnumerateKeys(expression)
        | Expression::GetIterator(expression)
        | Expression::IteratorClose(expression) => {
            logical_expression_references_internal_iterator_temp(expression)
        }
        Expression::Assign { value, .. } => {
            logical_expression_references_internal_iterator_temp(value)
        }
        Expression::AssignMember {
            object,
            property,
            value,
        } => {
            logical_expression_references_internal_iterator_temp(object)
                || logical_expression_references_internal_iterator_temp(property)
                || logical_expression_references_internal_iterator_temp(value)
        }
        Expression::AssignSuperMember { property, value } => {
            logical_expression_references_internal_iterator_temp(property)
                || logical_expression_references_internal_iterator_temp(value)
        }
        Expression::Call { callee, arguments }
        | Expression::New { callee, arguments }
        | Expression::SuperCall { callee, arguments } => {
            logical_expression_references_internal_iterator_temp(callee)
                || arguments.iter().any(|argument| match argument {
                    CallArgument::Expression(value) | CallArgument::Spread(value) => {
                        logical_expression_references_internal_iterator_temp(value)
                    }
                })
        }
        Expression::SuperMember { property } => {
            logical_expression_references_internal_iterator_temp(property)
        }
        Expression::Sequence(expressions) => expressions
            .iter()
            .any(logical_expression_references_internal_iterator_temp),
        _ => false,
    }
}

impl<'a> FunctionCompiler<'a> {
    pub(in crate::backend::direct_wasm) fn static_logical_assignment_left_value(
        &self,
        left: &Expression,
    ) -> Expression {
        let materialized = self.materialize_static_expression(left);
        if !static_expression_matches(&materialized, left) {
            return materialized;
        }

        let Expression::Identifier(name) = left else {
            return materialized;
        };
        if let Some((resolved_name, _)) = self.resolve_current_local_binding(name)
            && let Some(value) = self
                .state
                .speculation
                .static_semantics
                .local_value_binding(&resolved_name)
            && !static_expression_matches(value, left)
        {
            return value.clone();
        }
        self.state
            .speculation
            .static_semantics
            .local_value_binding(name)
            .or_else(|| self.global_value_binding(name))
            .or_else(|| self.backend.global_value_binding(name))
            .filter(|value| !static_expression_matches(value, left))
            .cloned()
            .unwrap_or(materialized)
    }

    pub(in crate::backend::direct_wasm) fn resolve_static_logical_assignment_binding_result(
        &self,
        op: BinaryOp,
        left: &Expression,
        right: &Expression,
    ) -> Option<(String, Expression)> {
        let (
            Expression::Identifier(left_name),
            Expression::Assign {
                name: assigned_name,
                value,
            },
        ) = (left, right)
        else {
            return None;
        };
        if left_name != assigned_name {
            return None;
        }

        let materialized_left = self.static_logical_assignment_left_value(left);
        match op {
            BinaryOp::LogicalAnd => {
                let left_truthy = self
                    .resolve_static_boolean_expression(&materialized_left)
                    .or_else(|| self.resolve_static_boolean_expression(left))?;
                let result = if left_truthy {
                    self.materialize_static_expression(value)
                } else {
                    materialized_left
                };
                Some((assigned_name.clone(), result))
            }
            BinaryOp::LogicalOr => {
                let left_truthy = self
                    .resolve_static_boolean_expression(&materialized_left)
                    .or_else(|| self.resolve_static_boolean_expression(left))?;
                let result = if left_truthy {
                    materialized_left
                } else {
                    self.materialize_static_expression(value)
                };
                Some((assigned_name.clone(), result))
            }
            BinaryOp::NullishCoalescing => {
                let primitive_left = self
                    .resolve_static_primitive_expression_with_context(
                        &materialized_left,
                        self.current_function_name(),
                    )
                    .or_else(|| {
                        self.resolve_static_primitive_expression_with_context(
                            left,
                            self.current_function_name(),
                        )
                    })?;
                let result = if matches!(primitive_left, Expression::Null | Expression::Undefined) {
                    self.materialize_static_expression(value)
                } else {
                    primitive_left
                };
                Some((assigned_name.clone(), result))
            }
            _ => None,
        }
    }

    pub(in crate::backend::direct_wasm) fn restore_logical_assignment_binding_metadata(
        &mut self,
        name: &str,
        value: &Expression,
    ) -> DirectResult<()> {
        let has_current_local_binding = self.resolve_current_local_binding(name).is_some();
        if !has_current_local_binding || self.current_function_name().is_none() {
            self.sync_bound_capture_source_binding_metadata(name, value)?;
        }

        if has_current_local_binding {
            self.update_local_value_binding(name, value);
        }

        Ok(())
    }

    fn logical_operand_static_truthiness_after_evaluation(
        &self,
        expression: &Expression,
    ) -> Option<bool> {
        if Self::expression_references_internal_assignment_temp(expression) {
            return None;
        }
        match expression {
            Expression::Array(_)
            | Expression::Object(_)
            | Expression::New { .. }
            | Expression::This => Some(true),
            Expression::Identifier(_) => {
                if matches!(
                    self.infer_value_kind(expression),
                    Some(
                        StaticValueKind::Object
                            | StaticValueKind::Function
                            | StaticValueKind::Symbol
                    )
                ) || self
                    .resolve_object_binding_from_expression(expression)
                    .is_some()
                    || self
                        .resolve_array_binding_from_expression(expression)
                        .is_some()
                    || self
                        .resolve_arguments_binding_from_expression(expression)
                        .is_some()
                    || self
                        .resolve_proxy_binding_from_expression(expression)
                        .is_some()
                    || self
                        .resolve_function_binding_from_expression(expression)
                        .is_some()
                {
                    Some(true)
                } else {
                    self.resolve_static_boolean_expression(expression)
                }
            }
            _ => self.resolve_static_boolean_expression(expression),
        }
    }

    fn emit_logical_operand_truthiness_from_local(
        &mut self,
        expression: &Expression,
        value_local: u32,
    ) -> DirectResult<()> {
        if let Some(truthy) = self.logical_operand_static_truthiness_after_evaluation(expression) {
            self.push_i32_const(truthy as i32);
            return Ok(());
        }

        self.push_local_get(value_local);
        self.push_i32_const(0);
        self.push_binary_op(BinaryOp::NotEqual)?;

        self.push_local_get(value_local);
        self.push_i32_const(JS_NULL_TAG);
        self.push_binary_op(BinaryOp::NotEqual)?;
        self.state.emission.output.instructions.push(0x71);

        self.push_local_get(value_local);
        self.push_i32_const(JS_UNDEFINED_TAG);
        self.push_binary_op(BinaryOp::NotEqual)?;
        self.state.emission.output.instructions.push(0x71);

        self.push_local_get(value_local);
        self.push_i32_const(JS_NAN_TAG);
        self.push_binary_op(BinaryOp::NotEqual)?;
        self.state.emission.output.instructions.push(0x71);
        Ok(())
    }

    pub(in crate::backend::direct_wasm) fn emit_logical_and(
        &mut self,
        left: &Expression,
        right: &Expression,
    ) -> DirectResult<()> {
        if !logical_expression_references_internal_iterator_temp(left)
            && !logical_expression_references_internal_iterator_temp(right)
            && !Self::expression_references_internal_assignment_temp(left)
            && !Self::expression_references_internal_assignment_temp(right)
            && inline_summary_side_effect_free_expression(left)
            && let Some(result) =
                self.resolve_static_logical_result_expression(BinaryOp::LogicalAnd, left, right)
        {
            return self.emit_numeric_expression(&result);
        }
        let logical_assignment_result = self.resolve_static_logical_assignment_binding_result(
            BinaryOp::LogicalAnd,
            left,
            right,
        );
        let temp_local = self.allocate_temp_local();
        self.emit_numeric_expression(left)?;
        self.push_local_set(temp_local);
        self.emit_logical_operand_truthiness_from_local(left, temp_local)?;
        self.state.emission.output.instructions.push(0x04);
        self.state.emission.output.instructions.push(I32_TYPE);
        self.push_control_frame();
        self.emit_numeric_expression(right)?;
        self.state.emission.output.instructions.push(0x05);
        self.push_local_get(temp_local);
        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();
        self.invalidate_operator_rhs_binding_metadata(right);
        if let Some((name, value)) = logical_assignment_result {
            self.restore_logical_assignment_binding_metadata(&name, &value)?;
        }
        Ok(())
    }

    pub(in crate::backend::direct_wasm) fn emit_logical_or(
        &mut self,
        left: &Expression,
        right: &Expression,
    ) -> DirectResult<()> {
        if !logical_expression_references_internal_iterator_temp(left)
            && !logical_expression_references_internal_iterator_temp(right)
            && !Self::expression_references_internal_assignment_temp(left)
            && !Self::expression_references_internal_assignment_temp(right)
            && inline_summary_side_effect_free_expression(left)
            && let Some(result) =
                self.resolve_static_logical_result_expression(BinaryOp::LogicalOr, left, right)
        {
            return self.emit_numeric_expression(&result);
        }
        let logical_assignment_result =
            self.resolve_static_logical_assignment_binding_result(BinaryOp::LogicalOr, left, right);
        let temp_local = self.allocate_temp_local();
        self.emit_numeric_expression(left)?;
        self.push_local_set(temp_local);
        self.emit_logical_operand_truthiness_from_local(left, temp_local)?;
        self.state.emission.output.instructions.push(0x04);
        self.state.emission.output.instructions.push(I32_TYPE);
        self.push_control_frame();
        self.push_local_get(temp_local);
        self.state.emission.output.instructions.push(0x05);
        self.emit_numeric_expression(right)?;
        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();
        self.invalidate_operator_rhs_binding_metadata(right);
        if let Some((name, value)) = logical_assignment_result {
            self.restore_logical_assignment_binding_metadata(&name, &value)?;
        }
        Ok(())
    }

    pub(in crate::backend::direct_wasm) fn emit_nullish_coalescing(
        &mut self,
        left: &Expression,
        right: &Expression,
    ) -> DirectResult<()> {
        if !logical_expression_references_internal_iterator_temp(left)
            && !logical_expression_references_internal_iterator_temp(right)
            && !Self::expression_references_internal_assignment_temp(left)
            && !Self::expression_references_internal_assignment_temp(right)
            && inline_summary_side_effect_free_expression(left)
            && let Some(result) = self.resolve_static_logical_result_expression(
                BinaryOp::NullishCoalescing,
                left,
                right,
            )
        {
            return self.emit_numeric_expression(&result);
        }
        let logical_assignment_result = self.resolve_static_logical_assignment_binding_result(
            BinaryOp::NullishCoalescing,
            left,
            right,
        );
        let temp_local = self.allocate_temp_local();

        self.emit_numeric_expression(left)?;
        self.push_local_set(temp_local);

        self.push_local_get(temp_local);
        self.push_i32_const(JS_NULL_TAG);
        self.push_binary_op(BinaryOp::NotEqual)?;

        self.push_local_get(temp_local);
        self.push_i32_const(JS_UNDEFINED_TAG);
        self.push_binary_op(BinaryOp::NotEqual)?;
        self.state.emission.output.instructions.push(0x71);

        self.state.emission.output.instructions.push(0x04);
        self.state.emission.output.instructions.push(I32_TYPE);
        self.push_control_frame();

        self.push_local_get(temp_local);

        self.state.emission.output.instructions.push(0x05);
        self.emit_numeric_expression(right)?;

        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();
        self.invalidate_operator_rhs_binding_metadata(right);
        if let Some((name, value)) = logical_assignment_result {
            self.restore_logical_assignment_binding_metadata(&name, &value)?;
        }
        Ok(())
    }

    fn invalidate_operator_rhs_binding_metadata(&mut self, expression: &Expression) {
        let mut invalidated_bindings = HashSet::new();
        collect_assigned_binding_names_from_expression(expression, &mut invalidated_bindings);
        let preserved_kinds = invalidated_bindings
            .iter()
            .filter_map(|name| {
                self.lookup_identifier_kind(name)
                    .map(|kind| (name.clone(), kind))
            })
            .collect::<HashMap<_, _>>();
        self.invalidate_static_binding_metadata_for_names_with_preserved_kinds(
            &invalidated_bindings,
            &preserved_kinds,
        );
    }
}
