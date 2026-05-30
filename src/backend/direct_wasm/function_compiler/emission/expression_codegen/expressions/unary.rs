use super::*;

#[path = "unary/call_dispatch.rs"]
mod call_dispatch;
#[path = "unary/delete_ops.rs"]
mod delete_ops;
#[path = "unary/typeof_ops.rs"]
mod typeof_ops;

impl<'a> FunctionCompiler<'a> {
    fn unary_to_primitive_target_expression<'b>(expression: &'b Expression) -> &'b Expression {
        match expression {
            Expression::Sequence(expressions) => expressions
                .last()
                .map(Self::unary_to_primitive_target_expression)
                .unwrap_or(expression),
            _ => expression,
        }
    }

    fn unary_plus_number_from_ordinary_plan(&self, plan: &OrdinaryToPrimitivePlan) -> Option<f64> {
        for step in &plan.steps {
            let StaticEvalOutcome::Value(value) = &step.outcome else {
                return None;
            };
            match self.static_expression_is_non_object_primitive(value) {
                Some(true) => return self.resolve_static_number_value(value),
                Some(false) => continue,
                None => return None,
            }
        }
        None
    }

    fn unary_minus_bigint_from_ordinary_plan(
        &self,
        plan: &OrdinaryToPrimitivePlan,
    ) -> Option<StaticBigInt> {
        for step in &plan.steps {
            let StaticEvalOutcome::Value(value) = &step.outcome else {
                return None;
            };
            match self.static_expression_is_non_object_primitive(value) {
                Some(true) if self.infer_value_kind(value) == Some(StaticValueKind::BigInt) => {
                    return self.resolve_static_bigint_value(value);
                }
                Some(true) => return None,
                Some(false) => continue,
                None => return None,
            }
        }
        None
    }

    fn emit_unary_plus_ordinary_to_number(
        &mut self,
        expression: &Expression,
    ) -> DirectResult<bool> {
        let current_function_name = self.current_function_name();
        if self
            .symbol_to_primitive_preempts_ordinary_to_primitive(expression, current_function_name)
        {
            return Ok(false);
        }
        let Some(plan) = self.resolve_ordinary_to_primitive_plan(expression) else {
            return Ok(false);
        };
        let analysis = self.analyze_ordinary_to_primitive_plan(&plan);
        let static_number = match analysis {
            OrdinaryToPrimitiveAnalysis::Primitive(
                StaticValueKind::Symbol | StaticValueKind::BigInt,
            ) => None,
            OrdinaryToPrimitiveAnalysis::Primitive(_) => Some(
                self.unary_plus_number_from_ordinary_plan(&plan)
                    .ok_or_else(|| {
                        Unsupported(
                            "unary plus ordinary ToPrimitive result is not statically numeric",
                        )
                    })?,
            ),
            OrdinaryToPrimitiveAnalysis::Throw | OrdinaryToPrimitiveAnalysis::TypeError => None,
            OrdinaryToPrimitiveAnalysis::Unknown => return Ok(false),
        };

        let result_local = self.allocate_temp_local();
        self.emit_numeric_expression(expression)?;
        self.push_local_set(result_local);

        let target = Self::unary_to_primitive_target_expression(expression);
        match self.emit_ordinary_to_primitive_from_plan(target, &plan, result_local)? {
            SymbolToPrimitiveHandling::AlwaysThrows => {
                self.push_i32_const(JS_UNDEFINED_TAG);
                return Ok(true);
            }
            SymbolToPrimitiveHandling::Handled => {}
            SymbolToPrimitiveHandling::NotHandled => return Ok(false),
        }

        match analysis {
            OrdinaryToPrimitiveAnalysis::Primitive(StaticValueKind::Symbol)
            | OrdinaryToPrimitiveAnalysis::Primitive(StaticValueKind::BigInt)
            | OrdinaryToPrimitiveAnalysis::TypeError => {
                self.emit_named_error_throw("TypeError")?;
                self.push_i32_const(JS_UNDEFINED_TAG);
            }
            OrdinaryToPrimitiveAnalysis::Primitive(_) => {
                self.emit_numeric_expression(&Expression::Number(
                    static_number.expect("primitive analysis computed a number above"),
                ))?;
            }
            OrdinaryToPrimitiveAnalysis::Throw => {
                self.push_i32_const(JS_UNDEFINED_TAG);
            }
            OrdinaryToPrimitiveAnalysis::Unknown => return Ok(false),
        }
        Ok(true)
    }

    fn emit_unary_minus_ordinary_to_number(
        &mut self,
        expression: &Expression,
    ) -> DirectResult<bool> {
        let current_function_name = self.current_function_name();
        if self
            .symbol_to_primitive_preempts_ordinary_to_primitive(expression, current_function_name)
        {
            return Ok(false);
        }
        let Some(plan) = self.resolve_ordinary_to_primitive_plan(expression) else {
            return Ok(false);
        };
        let analysis = self.analyze_ordinary_to_primitive_plan(&plan);
        let static_bigint = match analysis {
            OrdinaryToPrimitiveAnalysis::Primitive(StaticValueKind::BigInt) => Some(
                self.unary_minus_bigint_from_ordinary_plan(&plan)
                    .ok_or_else(|| {
                        Unsupported(
                            "unary minus ordinary ToPrimitive result is not statically bigint",
                        )
                    })?,
            ),
            _ => None,
        };
        let static_number = match analysis {
            OrdinaryToPrimitiveAnalysis::Primitive(StaticValueKind::Symbol) => None,
            OrdinaryToPrimitiveAnalysis::Primitive(StaticValueKind::BigInt) => None,
            OrdinaryToPrimitiveAnalysis::Primitive(_) => Some(
                self.unary_plus_number_from_ordinary_plan(&plan)
                    .ok_or_else(|| {
                        Unsupported(
                            "unary minus ordinary ToPrimitive result is not statically numeric",
                        )
                    })?,
            ),
            OrdinaryToPrimitiveAnalysis::Throw | OrdinaryToPrimitiveAnalysis::TypeError => None,
            OrdinaryToPrimitiveAnalysis::Unknown => return Ok(false),
        };

        let result_local = self.allocate_temp_local();
        self.emit_numeric_expression(expression)?;
        self.push_local_set(result_local);

        let target = Self::unary_to_primitive_target_expression(expression);
        match self.emit_ordinary_to_primitive_from_plan(target, &plan, result_local)? {
            SymbolToPrimitiveHandling::AlwaysThrows => {
                self.push_i32_const(JS_UNDEFINED_TAG);
                return Ok(true);
            }
            SymbolToPrimitiveHandling::Handled => {}
            SymbolToPrimitiveHandling::NotHandled => return Ok(false),
        }

        match analysis {
            OrdinaryToPrimitiveAnalysis::Primitive(StaticValueKind::Symbol)
            | OrdinaryToPrimitiveAnalysis::TypeError => {
                self.emit_named_error_throw("TypeError")?;
                self.push_i32_const(JS_UNDEFINED_TAG);
            }
            OrdinaryToPrimitiveAnalysis::Primitive(StaticValueKind::BigInt) => {
                self.emit_numeric_expression(&Expression::BigInt(
                    (-static_bigint.expect("primitive analysis computed a bigint above"))
                        .to_string(),
                ))?;
            }
            OrdinaryToPrimitiveAnalysis::Primitive(_) => {
                self.emit_numeric_expression(&Expression::Number(
                    -static_number.expect("primitive analysis computed a number above"),
                ))?;
            }
            OrdinaryToPrimitiveAnalysis::Throw => {
                self.push_i32_const(JS_UNDEFINED_TAG);
            }
            OrdinaryToPrimitiveAnalysis::Unknown => return Ok(false),
        }
        Ok(true)
    }

    fn emit_static_unary_minus_nan(&mut self, expression: &Expression) -> DirectResult<bool> {
        if !inline_summary_side_effect_free_expression(expression)
            || Self::expression_contains_assignment_or_update(expression)
        {
            return Ok(false);
        }
        let primitive = self
            .resolve_static_primitive_expression_with_context(
                expression,
                self.current_function_name(),
            )
            .or_else(|| {
                self.resolve_static_string_value_with_context(
                    expression,
                    self.current_function_name(),
                )
                .map(Expression::String)
            });
        if Self::expression_references_internal_assignment_temp(expression) && primitive.is_none() {
            return Ok(false);
        }
        let primitive = primitive.unwrap_or_else(|| expression.clone());
        if matches!(
            primitive,
            Expression::Number(_)
                | Expression::String(_)
                | Expression::Bool(_)
                | Expression::Null
                | Expression::Undefined
        ) && self
            .resolve_static_number_value(&primitive)
            .is_some_and(f64::is_nan)
        {
            self.push_i32_const(JS_NAN_TAG);
            return Ok(true);
        }
        Ok(false)
    }

    fn emit_static_unary_plus_number(&mut self, expression: &Expression) -> DirectResult<bool> {
        if !inline_summary_side_effect_free_expression(expression)
            || Self::expression_contains_assignment_or_update(expression)
        {
            return Ok(false);
        }
        let primitive = self
            .resolve_static_primitive_expression_with_context(
                expression,
                self.current_function_name(),
            )
            .or_else(|| {
                self.resolve_static_string_value_with_context(
                    expression,
                    self.current_function_name(),
                )
                .map(Expression::String)
            });
        if Self::expression_references_internal_assignment_temp(expression) && primitive.is_none() {
            return Ok(false);
        }
        let primitive = primitive.unwrap_or_else(|| expression.clone());
        if !matches!(
            primitive,
            Expression::Number(_)
                | Expression::String(_)
                | Expression::Bool(_)
                | Expression::Null
                | Expression::Undefined
        ) {
            return Ok(false);
        }
        let Some(number) = self.resolve_static_number_value(&primitive) else {
            return Ok(false);
        };
        if number.is_nan() {
            self.push_i32_const(JS_NAN_TAG);
            return Ok(true);
        }
        if number.is_finite() && number.fract() == 0.0 {
            self.emit_numeric_expression(&Expression::Number(number))?;
            return Ok(true);
        }
        Ok(false)
    }

    fn emit_unary_plus_runtime_undefined_to_nan(
        &mut self,
        expression: &Expression,
    ) -> DirectResult<()> {
        let value_local = self.allocate_temp_local();
        self.emit_numeric_expression(expression)?;
        self.push_local_set(value_local);

        self.push_local_get(value_local);
        self.push_i32_const(JS_UNDEFINED_TAG);
        self.push_binary_op(BinaryOp::Equal)?;
        self.state.emission.output.instructions.push(0x04);
        self.state.emission.output.instructions.push(I32_TYPE);
        self.push_control_frame();
        self.push_i32_const(JS_NAN_TAG);
        self.state.emission.output.instructions.push(0x05);
        self.push_local_get(value_local);
        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();
        Ok(())
    }

    fn emit_unary_plus_bigint_type_error(&mut self, expression: &Expression) -> DirectResult<bool> {
        if self.infer_value_kind(expression) != Some(StaticValueKind::BigInt)
            && !matches!(
                self.resolve_static_primitive_expression_with_context(
                    expression,
                    self.current_function_name(),
                ),
                Some(Expression::BigInt(_))
            )
        {
            return Ok(false);
        }

        self.emit_numeric_expression(expression)?;
        self.state.emission.output.instructions.push(0x1a);
        self.emit_named_error_throw("TypeError")?;
        self.push_i32_const(JS_UNDEFINED_TAG);
        Ok(true)
    }

    fn emit_static_unary_minus_bigint(&mut self, expression: &Expression) -> DirectResult<bool> {
        if let Some(value) = self.resolve_static_boxed_primitive_value(expression)
            && self.infer_value_kind(&value) == Some(StaticValueKind::BigInt)
            && let Some(bigint) = self.resolve_static_bigint_value(&value)
        {
            self.emit_numeric_expression(&Expression::BigInt((-bigint).to_string()))?;
            return Ok(true);
        }

        let Some(StaticEvalOutcome::Value(value)) = self
            .resolve_static_symbol_to_primitive_outcome_with_context(
                expression,
                self.current_function_name(),
            )
        else {
            return Ok(false);
        };
        if self.infer_value_kind(&value) != Some(StaticValueKind::BigInt) {
            return Ok(false);
        }
        let Some(bigint) = self.resolve_static_bigint_value(&value) else {
            return Ok(false);
        };
        let numeric_hint_argument = Expression::String("number".to_string());
        match self
            .emit_effectful_symbol_to_primitive_for_operand(expression, &numeric_hint_argument)?
        {
            SymbolToPrimitiveHandling::AlwaysThrows => {
                self.push_i32_const(JS_UNDEFINED_TAG);
                Ok(true)
            }
            SymbolToPrimitiveHandling::Handled => {
                self.emit_numeric_expression(&Expression::BigInt((-bigint).to_string()))?;
                Ok(true)
            }
            SymbolToPrimitiveHandling::NotHandled => Ok(false),
        }
    }

    fn emit_new_target_to_number(&mut self) -> DirectResult<()> {
        let value_local = self.allocate_temp_local();
        let result_local = self.allocate_temp_local();

        self.emit_numeric_expression(&Expression::NewTarget)?;
        self.push_local_set(value_local);
        self.push_local_get(value_local);
        self.push_local_set(result_local);

        self.emit_runtime_typeof_exact_match(
            value_local,
            result_local,
            JS_UNDEFINED_TAG,
            JS_NAN_TAG,
        )?;
        self.emit_runtime_typeof_exact_match(
            value_local,
            result_local,
            JS_TYPEOF_FUNCTION_TAG,
            JS_NAN_TAG,
        )?;
        self.emit_runtime_typeof_exact_match(
            value_local,
            result_local,
            JS_BUILTIN_EVAL_VALUE,
            JS_NAN_TAG,
        )?;
        self.emit_runtime_typeof_range_match(
            value_local,
            result_local,
            JS_BUILTIN_FUNCTION_VALUE_BASE,
            JS_BUILTIN_FUNCTION_VALUE_BASE + JS_BUILTIN_FUNCTION_VALUE_LIMIT,
            JS_NAN_TAG,
        )?;
        self.emit_runtime_typeof_range_match(
            value_local,
            result_local,
            JS_USER_FUNCTION_VALUE_BASE,
            JS_USER_FUNCTION_VALUE_BASE + JS_USER_FUNCTION_VALUE_LIMIT,
            JS_NAN_TAG,
        )?;

        self.push_local_get(result_local);
        Ok(())
    }

    fn emit_new_target_to_int32(&mut self) -> DirectResult<()> {
        let value_local = self.allocate_temp_local();
        let result_local = self.allocate_temp_local();

        self.emit_new_target_to_number()?;
        self.push_local_set(value_local);
        self.push_local_get(value_local);
        self.push_local_set(result_local);

        self.emit_runtime_typeof_exact_match(value_local, result_local, JS_NAN_TAG, 0)?;

        self.push_local_get(result_local);
        Ok(())
    }

    fn emit_negated_new_target_number(&mut self) -> DirectResult<()> {
        let value_local = self.allocate_temp_local();
        let result_local = self.allocate_temp_local();

        self.emit_new_target_to_number()?;
        self.push_local_set(value_local);
        self.push_i32_const(0);
        self.push_local_get(value_local);
        self.state.emission.output.instructions.push(0x6b);
        self.push_local_set(result_local);

        self.emit_runtime_typeof_exact_match(value_local, result_local, JS_NAN_TAG, JS_NAN_TAG)?;

        self.push_local_get(result_local);
        Ok(())
    }

    pub(in crate::backend::direct_wasm) fn emit_unary_expression(
        &mut self,
        op: UnaryOp,
        expression: &Expression,
    ) -> DirectResult<()> {
        match op {
            UnaryOp::TypeOf => self.emit_typeof_value_expression(expression),
            UnaryOp::Not => {
                self.emit_truthy_expression(expression)?;
                self.state.emission.output.instructions.push(0x45);
                Ok(())
            }
            UnaryOp::BitwiseNot => {
                if matches!(expression, Expression::NewTarget) {
                    self.emit_new_target_to_int32()?;
                } else {
                    self.emit_numeric_expression(expression)?;
                }
                self.push_i32_const(-1);
                self.state.emission.output.instructions.push(0x73);
                Ok(())
            }
            UnaryOp::Negate => {
                match expression {
                    Expression::Number(value) if value.is_finite() && value.fract() == 0.0 => {
                        let integer = -(*value as i64);
                        if is_reserved_js_runtime_value(integer) {
                            return Err(Unsupported(
                                "number literal collides with reserved JS tag",
                            ));
                        }
                    }
                    Expression::BigInt(value) => {
                        let integer = format!("-{}", value.strip_suffix('n').unwrap_or(value));
                        if let Ok(parsed) = integer.parse::<i64>()
                            && is_reserved_js_runtime_value(parsed)
                        {
                            return Err(Unsupported(
                                "bigint literal collides with reserved JS tag",
                            ));
                        }
                    }
                    _ => {}
                }
                if matches!(expression, Expression::NewTarget) {
                    self.emit_negated_new_target_number()?;
                } else if self.emit_static_unary_minus_nan(expression)? {
                    return Ok(());
                } else if self.emit_static_unary_minus_bigint(expression)? {
                    return Ok(());
                } else if self.emit_unary_minus_ordinary_to_number(expression)? {
                    return Ok(());
                } else {
                    self.push_i32_const(0);
                    self.emit_numeric_expression(expression)?;
                    self.state.emission.output.instructions.push(0x6b);
                }
                Ok(())
            }
            UnaryOp::Plus => {
                if self.emit_unary_plus_ordinary_to_number(expression)? {
                    return Ok(());
                }
                if self.emit_unary_plus_bigint_type_error(expression)? {
                    return Ok(());
                }
                if self.emit_static_unary_plus_number(expression)? {
                    return Ok(());
                }
                if matches!(expression, Expression::NewTarget) {
                    self.emit_new_target_to_number()
                } else {
                    self.emit_unary_plus_runtime_undefined_to_nan(expression)
                }
            }
            UnaryOp::Void => {
                let temp_local = self.allocate_temp_local();
                self.emit_numeric_expression(expression)?;
                self.push_local_set(temp_local);
                self.push_i32_const(JS_UNDEFINED_TAG);
                Ok(())
            }
            UnaryOp::Delete => self.emit_delete_expression(expression),
        }
    }
}
