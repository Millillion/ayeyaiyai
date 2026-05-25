use super::*;

impl<'a> FunctionCompiler<'a> {
    pub(in crate::backend::direct_wasm) fn symbol_to_primitive_preempts_ordinary_to_primitive(
        &self,
        expression: &Expression,
        current_function_name: Option<&str>,
    ) -> bool {
        let symbol_property = symbol_to_primitive_expression();
        if let Some(getter_binding) =
            self.resolve_member_getter_binding(expression, &symbol_property)
        {
            let Some(getter_outcome) = self
                .resolve_static_function_outcome_from_binding_with_context(
                    &getter_binding,
                    &[],
                    current_function_name,
                )
            else {
                return true;
            };
            let StaticEvalOutcome::Value(method_value) = getter_outcome else {
                return true;
            };
            if let Some(primitive) = self.resolve_static_primitive_expression_with_context(
                &method_value,
                current_function_name,
            ) {
                return !matches!(primitive, Expression::Null | Expression::Undefined);
            }
            return true;
        }

        if self
            .resolve_member_function_binding(expression, &symbol_property)
            .is_some()
        {
            return true;
        }

        let Some(object_binding) = self.resolve_object_binding_from_expression(expression) else {
            return false;
        };
        let Some(method_value) =
            self.resolve_object_binding_property_value(&object_binding, &symbol_property)
        else {
            return false;
        };
        if let Some(primitive) = self
            .resolve_static_primitive_expression_with_context(&method_value, current_function_name)
        {
            return !matches!(primitive, Expression::Null | Expression::Undefined);
        }
        true
    }

    pub(in crate::backend::direct_wasm) fn symbol_to_primitive_non_callable_type_error(
        &self,
        expression: &Expression,
    ) -> bool {
        let symbol_property = symbol_to_primitive_expression();
        let Some(object_binding) = self.resolve_object_binding_from_expression(expression) else {
            return false;
        };
        let Some(method_value) =
            self.resolve_object_binding_property_value(&object_binding, &symbol_property)
        else {
            return false;
        };
        if self
            .resolve_function_binding_from_expression(&method_value)
            .is_some()
        {
            return false;
        }
        if let Some(primitive) = self.resolve_static_primitive_expression_with_context(
            &method_value,
            self.current_function_name(),
        ) && matches!(primitive, Expression::Null | Expression::Undefined)
        {
            return false;
        }
        true
    }

    pub(in crate::backend::direct_wasm) fn symbol_to_primitive_callable_terminal_effect(
        &self,
        expression: &Expression,
        default_argument: &Expression,
    ) -> bool {
        let symbol_property = symbol_to_primitive_expression();
        let Some(function_binding) = self
            .resolve_member_function_binding(expression, &symbol_property)
            .or_else(|| {
                self.resolve_object_binding_from_expression(expression)
                    .and_then(|object_binding| {
                        self.resolve_object_binding_property_value(
                            &object_binding,
                            &symbol_property,
                        )
                        .and_then(|value| self.resolve_function_binding_from_expression(&value))
                    })
            })
        else {
            return false;
        };
        if self.function_binding_always_throws(&function_binding) {
            return true;
        }
        self.resolve_function_binding_static_return_expression_with_call_frame(
            &function_binding,
            std::slice::from_ref(default_argument),
            expression,
        )
        .or_else(|| {
            self.resolve_function_binding_static_return_expression(
                &function_binding,
                std::slice::from_ref(default_argument),
            )
        })
        .is_some_and(|return_expression| {
            self.static_expression_is_non_object_primitive(&return_expression) == Some(false)
        })
    }

    pub(in crate::backend::direct_wasm) fn emit_effectful_symbol_to_primitive_for_operand(
        &mut self,
        expression: &Expression,
        default_argument: &Expression,
    ) -> DirectResult<SymbolToPrimitiveHandling> {
        if self.expression_depends_on_active_loop_assignment(expression) {
            return Ok(SymbolToPrimitiveHandling::NotHandled);
        }
        let symbol_property = symbol_to_primitive_expression();
        if let Some(getter_binding) =
            self.resolve_member_getter_binding(expression, &symbol_property)
        {
            let getter_result_local = self.allocate_temp_local();
            if !self.emit_binding_call_result_to_local_with_explicit_this(
                &getter_binding,
                &[],
                expression,
                JS_TYPEOF_OBJECT_TAG,
                getter_result_local,
            )? {
                return Ok(SymbolToPrimitiveHandling::NotHandled);
            }
            if self.function_binding_always_throws(&getter_binding) {
                return Ok(SymbolToPrimitiveHandling::AlwaysThrows);
            }
            if let Some(return_expression) = self
                .resolve_function_binding_static_return_expression_with_call_frame(
                    &getter_binding,
                    &[],
                    expression,
                )
                .or_else(|| {
                    self.resolve_function_binding_static_return_expression(&getter_binding, &[])
                })
            {
                if let Some(primitive) = self.resolve_static_primitive_expression_with_context(
                    &return_expression,
                    self.current_function_name(),
                ) {
                    if matches!(primitive, Expression::Null | Expression::Undefined) {
                        return Ok(SymbolToPrimitiveHandling::Handled);
                    }
                    self.emit_named_error_throw("TypeError")?;
                    return Ok(SymbolToPrimitiveHandling::AlwaysThrows);
                }
                if let Some(return_binding) =
                    self.resolve_function_binding_from_expression(&return_expression)
                {
                    let return_result_local = self.allocate_temp_local();
                    if !self.emit_binding_call_result_to_local_with_explicit_this(
                        &return_binding,
                        std::slice::from_ref(default_argument),
                        expression,
                        JS_TYPEOF_OBJECT_TAG,
                        return_result_local,
                    )? {
                        return Ok(SymbolToPrimitiveHandling::NotHandled);
                    }
                    if self.function_binding_always_throws(&return_binding) {
                        return Ok(SymbolToPrimitiveHandling::AlwaysThrows);
                    }
                    return Ok(SymbolToPrimitiveHandling::Handled);
                }
                self.emit_named_error_throw("TypeError")?;
                return Ok(SymbolToPrimitiveHandling::AlwaysThrows);
            }
            if self.function_binding_defaults_to_undefined(&getter_binding) {
                return Ok(SymbolToPrimitiveHandling::Handled);
            }
        }

        if let Some(function_binding) = self
            .resolve_member_function_binding(expression, &symbol_property)
            .or_else(|| {
                self.resolve_object_binding_from_expression(expression)
                    .and_then(|object_binding| {
                        self.resolve_object_binding_property_value(
                            &object_binding,
                            &symbol_property,
                        )
                        .and_then(|value| self.resolve_function_binding_from_expression(&value))
                    })
            })
        {
            let result_local = self.allocate_temp_local();
            if !self.emit_binding_call_result_to_local_with_explicit_this(
                &function_binding,
                std::slice::from_ref(default_argument),
                expression,
                JS_TYPEOF_OBJECT_TAG,
                result_local,
            )? {
                return Ok(SymbolToPrimitiveHandling::NotHandled);
            }
            if self.function_binding_always_throws(&function_binding) {
                return Ok(SymbolToPrimitiveHandling::AlwaysThrows);
            }
            if let Some(return_expression) = self
                .resolve_function_binding_static_return_expression_with_call_frame(
                    &function_binding,
                    std::slice::from_ref(default_argument),
                    expression,
                )
                .or_else(|| {
                    self.resolve_function_binding_static_return_expression(
                        &function_binding,
                        std::slice::from_ref(default_argument),
                    )
                })
                && self.static_expression_is_non_object_primitive(&return_expression) == Some(false)
            {
                self.emit_named_error_throw("TypeError")?;
                return Ok(SymbolToPrimitiveHandling::AlwaysThrows);
            }
            return Ok(SymbolToPrimitiveHandling::Handled);
        }

        if let Some(object_binding) = self.resolve_object_binding_from_expression(expression)
            && let Some(method_value) =
                self.resolve_object_binding_property_value(&object_binding, &symbol_property)
            && let Some(primitive) = self.resolve_static_primitive_expression_with_context(
                &method_value,
                self.current_function_name(),
            )
        {
            if matches!(primitive, Expression::Null | Expression::Undefined) {
                return Ok(SymbolToPrimitiveHandling::NotHandled);
            }
            self.emit_named_error_throw("TypeError")?;
            return Ok(SymbolToPrimitiveHandling::AlwaysThrows);
        }

        Ok(SymbolToPrimitiveHandling::NotHandled)
    }

    pub(in crate::backend::direct_wasm) fn emit_effectful_symbol_to_primitive_addition(
        &mut self,
        left: &Expression,
        right: &Expression,
    ) -> DirectResult<bool> {
        if self.expression_depends_on_active_loop_assignment(left)
            || self.expression_depends_on_active_loop_assignment(right)
        {
            return Ok(false);
        }
        let default_argument = Expression::String("default".to_string());
        let left_handling =
            self.emit_effectful_symbol_to_primitive_for_operand(left, &default_argument)?;
        if left_handling == SymbolToPrimitiveHandling::AlwaysThrows {
            self.push_i32_const(JS_UNDEFINED_TAG);
            return Ok(true);
        }
        let right_handling =
            self.emit_effectful_symbol_to_primitive_for_operand(right, &default_argument)?;

        if left_handling == SymbolToPrimitiveHandling::NotHandled
            && right_handling == SymbolToPrimitiveHandling::NotHandled
        {
            return Ok(false);
        }

        if left_handling == SymbolToPrimitiveHandling::NotHandled {
            self.emit_numeric_expression(left)?;
            self.state.emission.output.instructions.push(0x1a);
        }
        if right_handling == SymbolToPrimitiveHandling::NotHandled {
            self.emit_numeric_expression(right)?;
            self.state.emission.output.instructions.push(0x1a);
        }

        self.push_i32_const(JS_NAN_TAG);
        Ok(true)
    }

    fn emit_symbol_to_primitive_result_for_loose_equality_operand(
        &mut self,
        expression: &Expression,
        default_argument: &Expression,
        result_local: u32,
    ) -> DirectResult<SymbolToPrimitiveHandling> {
        if self.expression_depends_on_active_loop_assignment(expression) {
            return Ok(SymbolToPrimitiveHandling::NotHandled);
        }

        let symbol_property = symbol_to_primitive_expression();
        if let Some(getter_binding) =
            self.resolve_member_getter_binding(expression, &symbol_property)
        {
            let getter_result_local = self.allocate_temp_local();
            if !self.emit_binding_call_result_to_local_with_explicit_this(
                &getter_binding,
                &[],
                expression,
                JS_TYPEOF_OBJECT_TAG,
                getter_result_local,
            )? {
                return Ok(SymbolToPrimitiveHandling::NotHandled);
            }
            if self.function_binding_always_throws(&getter_binding) {
                return Ok(SymbolToPrimitiveHandling::AlwaysThrows);
            }
            if let Some(return_expression) = self
                .resolve_function_binding_static_return_expression_with_call_frame(
                    &getter_binding,
                    &[],
                    expression,
                )
                .or_else(|| {
                    self.resolve_function_binding_static_return_expression(&getter_binding, &[])
                })
            {
                if let Some(primitive) = self.resolve_static_primitive_expression_with_context(
                    &return_expression,
                    self.current_function_name(),
                ) {
                    if matches!(primitive, Expression::Null | Expression::Undefined) {
                        return Ok(SymbolToPrimitiveHandling::NotHandled);
                    }
                    self.emit_named_error_throw("TypeError")?;
                    return Ok(SymbolToPrimitiveHandling::AlwaysThrows);
                }
                if let Some(return_binding) =
                    self.resolve_function_binding_from_expression(&return_expression)
                {
                    if !self.emit_binding_call_result_to_local_with_explicit_this(
                        &return_binding,
                        std::slice::from_ref(default_argument),
                        expression,
                        JS_TYPEOF_OBJECT_TAG,
                        result_local,
                    )? {
                        return Ok(SymbolToPrimitiveHandling::NotHandled);
                    }
                    if self.function_binding_always_throws(&return_binding) {
                        return Ok(SymbolToPrimitiveHandling::AlwaysThrows);
                    }
                    return Ok(SymbolToPrimitiveHandling::Handled);
                }
                self.emit_named_error_throw("TypeError")?;
                return Ok(SymbolToPrimitiveHandling::AlwaysThrows);
            }
        }

        if let Some(function_binding) = self
            .resolve_member_function_binding(expression, &symbol_property)
            .or_else(|| {
                self.resolve_object_binding_from_expression(expression)
                    .and_then(|object_binding| {
                        self.resolve_object_binding_property_value(
                            &object_binding,
                            &symbol_property,
                        )
                        .and_then(|value| self.resolve_function_binding_from_expression(&value))
                    })
            })
        {
            if !self.emit_binding_call_result_to_local_with_explicit_this(
                &function_binding,
                std::slice::from_ref(default_argument),
                expression,
                JS_TYPEOF_OBJECT_TAG,
                result_local,
            )? {
                return Ok(SymbolToPrimitiveHandling::NotHandled);
            }
            if self.function_binding_always_throws(&function_binding) {
                return Ok(SymbolToPrimitiveHandling::AlwaysThrows);
            }
            if let Some(return_expression) = self
                .resolve_function_binding_static_return_expression_with_call_frame(
                    &function_binding,
                    std::slice::from_ref(default_argument),
                    expression,
                )
                .or_else(|| {
                    self.resolve_function_binding_static_return_expression(
                        &function_binding,
                        std::slice::from_ref(default_argument),
                    )
                })
                && self.static_expression_is_non_object_primitive(&return_expression) == Some(false)
            {
                self.emit_named_error_throw("TypeError")?;
                return Ok(SymbolToPrimitiveHandling::AlwaysThrows);
            }
            return Ok(SymbolToPrimitiveHandling::Handled);
        }

        if let Some(object_binding) = self.resolve_object_binding_from_expression(expression)
            && let Some(method_value) =
                self.resolve_object_binding_property_value(&object_binding, &symbol_property)
            && let Some(primitive) = self.resolve_static_primitive_expression_with_context(
                &method_value,
                self.current_function_name(),
            )
        {
            if matches!(primitive, Expression::Null | Expression::Undefined) {
                return Ok(SymbolToPrimitiveHandling::NotHandled);
            }
            self.emit_named_error_throw("TypeError")?;
            return Ok(SymbolToPrimitiveHandling::AlwaysThrows);
        }

        Ok(SymbolToPrimitiveHandling::NotHandled)
    }

    fn symbol_to_primitive_loose_equality_candidate(&self, expression: &Expression) -> bool {
        if self.expression_depends_on_active_loop_assignment(expression) {
            return false;
        }

        let symbol_property = symbol_to_primitive_expression();
        if self
            .resolve_member_getter_binding(expression, &symbol_property)
            .is_some()
        {
            return true;
        }
        if self
            .resolve_member_function_binding(expression, &symbol_property)
            .is_some()
        {
            return true;
        }

        self.resolve_object_binding_from_expression(expression)
            .and_then(|object_binding| {
                self.resolve_object_binding_property_value(&object_binding, &symbol_property)
            })
            .is_some_and(|method_value| {
                self.resolve_function_binding_from_expression(&method_value)
                    .is_some()
                    || self
                        .resolve_static_primitive_expression_with_context(
                            &method_value,
                            self.current_function_name(),
                        )
                        .is_some_and(|primitive| {
                            !matches!(primitive, Expression::Null | Expression::Undefined)
                        })
            })
    }

    pub(in crate::backend::direct_wasm) fn emit_effectful_symbol_to_primitive_loose_equality(
        &mut self,
        op: BinaryOp,
        left: &Expression,
        right: &Expression,
    ) -> DirectResult<bool> {
        if !matches!(op, BinaryOp::LooseEqual | BinaryOp::LooseNotEqual)
            || self.expression_depends_on_active_loop_assignment(left)
            || self.expression_depends_on_active_loop_assignment(right)
        {
            return Ok(false);
        }

        let default_argument = Expression::String("default".to_string());
        if !self.symbol_to_primitive_loose_equality_candidate(left)
            && !self.symbol_to_primitive_loose_equality_candidate(right)
        {
            return Ok(false);
        }

        let left_local = self.allocate_temp_local();
        let right_local = self.allocate_temp_local();

        let left_handling = self.emit_symbol_to_primitive_result_for_loose_equality_operand(
            left,
            &default_argument,
            left_local,
        )?;
        if left_handling == SymbolToPrimitiveHandling::AlwaysThrows {
            self.push_i32_const(JS_UNDEFINED_TAG);
            return Ok(true);
        }
        if left_handling == SymbolToPrimitiveHandling::NotHandled {
            self.emit_numeric_expression(left)?;
            self.push_local_set(left_local);
        }

        let right_handling = self.emit_symbol_to_primitive_result_for_loose_equality_operand(
            right,
            &default_argument,
            right_local,
        )?;
        if right_handling == SymbolToPrimitiveHandling::AlwaysThrows {
            self.push_i32_const(JS_UNDEFINED_TAG);
            return Ok(true);
        }
        if right_handling == SymbolToPrimitiveHandling::NotHandled {
            self.emit_numeric_expression(right)?;
            self.push_local_set(right_local);
        }

        if left_handling == SymbolToPrimitiveHandling::NotHandled
            && right_handling == SymbolToPrimitiveHandling::NotHandled
        {
            return Ok(false);
        }

        self.push_local_get(left_local);
        self.push_local_get(right_local);
        self.push_binary_op(match op {
            BinaryOp::LooseEqual => BinaryOp::Equal,
            BinaryOp::LooseNotEqual => BinaryOp::NotEqual,
            _ => unreachable!("filtered above"),
        })?;
        Ok(true)
    }
}
