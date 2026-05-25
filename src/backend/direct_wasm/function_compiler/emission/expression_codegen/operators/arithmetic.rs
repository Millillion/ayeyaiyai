use super::*;

impl<'a> FunctionCompiler<'a> {
    fn exponentiation_is_odd_integer(value: f64) -> bool {
        value.is_finite()
            && value.fract() == 0.0
            && value.abs() <= i64::MAX as f64
            && (value as i64).abs() % 2 == 1
    }

    fn js_number_exponentiate(base: f64, exponent: f64) -> f64 {
        if exponent.is_nan() {
            return f64::NAN;
        }
        if exponent == 0.0 {
            return 1.0;
        }
        if base.is_nan() {
            return f64::NAN;
        }

        let abs_base = base.abs();
        if exponent.is_infinite() {
            if abs_base > 1.0 {
                return if exponent.is_sign_positive() {
                    f64::INFINITY
                } else {
                    0.0
                };
            }
            if abs_base == 1.0 {
                return f64::NAN;
            }
            return if exponent.is_sign_positive() {
                0.0
            } else {
                f64::INFINITY
            };
        }

        if base == f64::INFINITY {
            return if exponent > 0.0 { f64::INFINITY } else { 0.0 };
        }
        if base == f64::NEG_INFINITY {
            let odd = Self::exponentiation_is_odd_integer(exponent);
            if exponent > 0.0 {
                return if odd {
                    f64::NEG_INFINITY
                } else {
                    f64::INFINITY
                };
            }
            return if odd { -0.0 } else { 0.0 };
        }

        if base == 0.0 {
            if base.is_sign_negative() {
                let odd = Self::exponentiation_is_odd_integer(exponent);
                if exponent > 0.0 {
                    return if odd { -0.0 } else { 0.0 };
                }
                return if odd {
                    f64::NEG_INFINITY
                } else {
                    f64::INFINITY
                };
            }
            return if exponent > 0.0 { 0.0 } else { f64::INFINITY };
        }

        if base < 0.0 && base.is_finite() && exponent.is_finite() && exponent.fract() != 0.0 {
            return f64::NAN;
        }

        base.powf(exponent)
    }

    fn exponentiation_builtin_number_alias(expression: &Expression) -> Option<f64> {
        match expression {
            Expression::Number(value) => Some(*value),
            Expression::Identifier(name) if name == "NaN" => Some(f64::NAN),
            Expression::Identifier(name) if name == "Infinity" => Some(f64::INFINITY),
            Expression::Identifier(name) if name == "undefined" => Some(f64::NAN),
            Expression::Unary {
                op: UnaryOp::Plus,
                expression,
            } => Self::exponentiation_builtin_number_alias(expression),
            Expression::Unary {
                op: UnaryOp::Negate,
                expression,
            } => Some(-Self::exponentiation_builtin_number_alias(expression)?),
            Expression::Member { object, property } => {
                let Expression::Identifier(object_name) = object.as_ref() else {
                    return None;
                };
                let Expression::String(property_name) = property.as_ref() else {
                    return None;
                };
                builtin_member_number_value(object_name, property_name)
            }
            _ => None,
        }
    }

    fn exponentiation_static_number_value(&self, expression: &Expression) -> Option<f64> {
        self.resolve_static_number_value(expression)
            .or_else(|| Self::exponentiation_builtin_number_alias(expression))
            .or_else(|| {
                self.resolve_bound_alias_expression(expression)
                    .filter(|resolved| !static_expression_matches(resolved, expression))
                    .and_then(|resolved| {
                        self.resolve_static_number_value(&resolved)
                            .or_else(|| Self::exponentiation_builtin_number_alias(&resolved))
                    })
            })
            .or_else(|| {
                let Expression::Identifier(name) = expression else {
                    return None;
                };
                self.global_value_binding(name).and_then(|value| {
                    self.resolve_static_number_value(value)
                        .or_else(|| Self::exponentiation_builtin_number_alias(value))
                })
            })
    }

    fn exponentiation_numeric_kind(&self, expression: &Expression) -> Option<StaticValueKind> {
        if matches!(
            self.resolve_static_boxed_primitive_value(expression),
            Some(Expression::BigInt(_))
        ) {
            return Some(StaticValueKind::BigInt);
        }
        if matches!(expression, Expression::BigInt(_)) {
            return Some(StaticValueKind::BigInt);
        }
        if self
            .exponentiation_static_number_value(expression)
            .is_some()
        {
            return Some(StaticValueKind::Number);
        }
        match self.infer_value_kind(expression) {
            Some(StaticValueKind::BigInt) => Some(StaticValueKind::BigInt),
            Some(
                StaticValueKind::Number
                | StaticValueKind::String
                | StaticValueKind::Bool
                | StaticValueKind::Null
                | StaticValueKind::Undefined,
            ) => Some(StaticValueKind::Number),
            _ => None,
        }
    }

    fn emit_exponentiation_operand_effects(
        &mut self,
        base: &Expression,
        exponent: &Expression,
    ) -> DirectResult<()> {
        self.emit_numeric_expression(base)?;
        self.state.emission.output.instructions.push(0x1a);
        self.emit_numeric_expression(exponent)?;
        self.state.emission.output.instructions.push(0x1a);
        Ok(())
    }

    fn emit_static_exponentiation_result_after_effects(
        &mut self,
        base: &Expression,
        exponent: &Expression,
        result: f64,
    ) -> DirectResult<()> {
        self.emit_exponentiation_operand_effects(base, exponent)?;
        self.emit_numeric_expression(&Expression::Number(result))
    }

    fn static_number_value_in_loop_environment(
        &mut self,
        expression: &Expression,
        environment: &HashMap<String, i64>,
    ) -> Option<f64> {
        match expression {
            Expression::Identifier(name) => environment
                .get(name)
                .copied()
                .or_else(|| {
                    scoped_binding_source_name(name)
                        .and_then(|source_name| environment.get(source_name).copied())
                })
                .map(|value| value as f64)
                .or_else(|| self.exponentiation_static_number_value(expression)),
            Expression::Member { object, property } => {
                if !self.expression_depends_on_active_loop_assignment(expression)
                    && let Some(value) = self.exponentiation_static_number_value(expression)
                {
                    return Some(value);
                }
                let index = self
                    .active_loop_integer_value(property, environment)
                    .or_else(|| {
                        self.exponentiation_static_number_value(property)
                            .and_then(|value| {
                                (value.is_finite() && value.fract() == 0.0).then_some(value as i64)
                            })
                    })?;
                if index < 0 {
                    return Some(f64::NAN);
                }
                if let Some(array_binding) = self.resolve_array_binding_from_expression(object)
                    && let Some(Some(value)) = array_binding.values.get(index as usize)
                {
                    return self.static_number_value_in_loop_environment(value, environment);
                }
                if !self.expression_depends_on_active_loop_assignment(expression) {
                    self.exponentiation_static_number_value(expression)
                } else {
                    None
                }
            }
            Expression::Unary {
                op: UnaryOp::Plus,
                expression,
            } => self.static_number_value_in_loop_environment(expression, environment),
            Expression::Unary {
                op: UnaryOp::Negate,
                expression,
            } => Some(-self.static_number_value_in_loop_environment(expression, environment)?),
            Expression::Binary {
                op: BinaryOp::Add,
                left,
                right,
            } => Some(
                self.static_number_value_in_loop_environment(left, environment)?
                    + self.static_number_value_in_loop_environment(right, environment)?,
            ),
            Expression::Binary {
                op: BinaryOp::Subtract,
                left,
                right,
            } => Some(
                self.static_number_value_in_loop_environment(left, environment)?
                    - self.static_number_value_in_loop_environment(right, environment)?,
            ),
            Expression::Binary {
                op: BinaryOp::Multiply,
                left,
                right,
            } => Some(
                self.static_number_value_in_loop_environment(left, environment)?
                    * self.static_number_value_in_loop_environment(right, environment)?,
            ),
            Expression::Binary {
                op: BinaryOp::Divide,
                left,
                right,
            } => Some(
                self.static_number_value_in_loop_environment(left, environment)?
                    / self.static_number_value_in_loop_environment(right, environment)?,
            ),
            Expression::Binary {
                op: BinaryOp::Modulo,
                left,
                right,
            } => Some(
                self.static_number_value_in_loop_environment(left, environment)?
                    % self.static_number_value_in_loop_environment(right, environment)?,
            ),
            Expression::Binary {
                op: BinaryOp::Exponentiate,
                left,
                right,
            } => Some(Self::js_number_exponentiate(
                self.static_number_value_in_loop_environment(left, environment)?,
                self.static_number_value_in_loop_environment(right, environment)?,
            )),
            _ if !self.expression_depends_on_active_loop_assignment(expression) => {
                self.exponentiation_static_number_value(expression)
            }
            _ => None,
        }
    }

    fn same_value_number_result(left: f64, right: f64) -> bool {
        if left.is_nan() && right.is_nan() {
            true
        } else if left == 0.0 && right == 0.0 {
            left.is_sign_negative() == right.is_sign_negative()
        } else {
            left == right
        }
    }

    fn active_loop_static_exponentiation_result(
        &mut self,
        base: &Expression,
        exponent: &Expression,
    ) -> Option<f64> {
        if !self.expression_depends_on_active_loop_assignment(base)
            && !self.expression_depends_on_active_loop_assignment(exponent)
        {
            return None;
        }
        let environments = self.active_numeric_loop_environments()?;
        if environments.is_empty() {
            return None;
        }

        let mut result = None;
        for environment in environments {
            let base_value = self.static_number_value_in_loop_environment(base, &environment)?;
            let exponent_value =
                self.static_number_value_in_loop_environment(exponent, &environment)?;
            let next_result = Self::js_number_exponentiate(base_value, exponent_value);
            if let Some(previous) = result {
                if !Self::same_value_number_result(previous, next_result) {
                    return None;
                }
            } else {
                result = Some(next_result);
            }
        }
        result
    }

    fn exponentiation_coercion_target_expression<'b>(expression: &'b Expression) -> &'b Expression {
        match expression {
            Expression::Sequence(expressions) => expressions
                .last()
                .map(Self::exponentiation_coercion_target_expression)
                .unwrap_or(expression),
            _ => expression,
        }
    }

    fn ordinary_to_primitive_plan_has_observable_effect(
        &self,
        plan: &OrdinaryToPrimitivePlan,
    ) -> bool {
        plan.steps.iter().any(|step| match &step.binding {
            LocalFunctionBinding::User(function_name) => self
                .resolve_registered_function_declaration(function_name)
                .map(|function| {
                    !matches!(
                        function.body.as_slice(),
                        [Statement::Return(expression)]
                            if inline_summary_side_effect_free_expression(expression)
                    )
                })
                .unwrap_or(true),
            LocalFunctionBinding::Builtin(_) => false,
        })
    }

    fn exponentiation_effectful_ordinary_plan(
        &self,
        expression: &Expression,
    ) -> Option<OrdinaryToPrimitivePlan> {
        let current_function_name = self.current_function_name();
        if self
            .symbol_to_primitive_preempts_ordinary_to_primitive(expression, current_function_name)
        {
            return None;
        }
        let plan = self.resolve_ordinary_to_primitive_plan(expression)?;
        match self.analyze_ordinary_to_primitive_plan(&plan) {
            OrdinaryToPrimitiveAnalysis::Primitive(kind) if kind != StaticValueKind::Symbol => {
                let observable = self.ordinary_to_primitive_plan_has_observable_effect(&plan);
                observable.then_some(plan)
            }
            OrdinaryToPrimitiveAnalysis::Primitive(_)
            | OrdinaryToPrimitiveAnalysis::Throw
            | OrdinaryToPrimitiveAnalysis::TypeError
            | OrdinaryToPrimitiveAnalysis::Unknown => None,
        }
    }

    fn exponentiation_primitive_from_ordinary_plan(
        &self,
        plan: &OrdinaryToPrimitivePlan,
    ) -> Option<Expression> {
        for step in &plan.steps {
            let StaticEvalOutcome::Value(value) = &step.outcome else {
                return None;
            };
            match self.static_expression_is_non_object_primitive(value) {
                Some(true) => {
                    return self
                        .resolve_static_primitive_expression_with_context(
                            value,
                            self.current_function_name(),
                        )
                        .or_else(|| Some(value.clone()));
                }
                Some(false) => continue,
                None => return None,
            }
        }
        None
    }

    fn exponentiation_static_unary_number_after_effects(
        &self,
        expression: &Expression,
    ) -> Option<Expression> {
        let Expression::Unary { op, expression } = expression else {
            return None;
        };
        if !matches!(op, UnaryOp::Plus | UnaryOp::Negate) {
            return None;
        }
        let current_function_name = self.current_function_name();
        let target = Self::exponentiation_coercion_target_expression(expression);
        let primitive = if self
            .symbol_to_primitive_preempts_ordinary_to_primitive(target, current_function_name)
        {
            match self.resolve_static_to_primitive_outcome_with_context(
                target,
                PrimitiveHint::Number,
                current_function_name,
            )? {
                StaticEvalOutcome::Value(value) => value,
                StaticEvalOutcome::Throw(_) => return None,
            }
        } else if let Some(plan) = self.resolve_ordinary_to_primitive_plan(target) {
            self.exponentiation_primitive_from_ordinary_plan(&plan)?
        } else {
            match self.resolve_static_to_primitive_outcome_with_context(
                target,
                PrimitiveHint::Number,
                current_function_name,
            )? {
                StaticEvalOutcome::Value(value) => value,
                StaticEvalOutcome::Throw(_) => return None,
            }
        };
        let number = self.exponentiation_static_number_value(&primitive)?;
        Some(Expression::Number(if matches!(op, UnaryOp::Negate) {
            -number
        } else {
            number
        }))
    }

    fn exponentiation_static_primitive_after_effects(
        &self,
        expression: &Expression,
        plan: Option<&OrdinaryToPrimitivePlan>,
    ) -> Option<Expression> {
        if let Some(plan) = plan {
            return self.exponentiation_primitive_from_ordinary_plan(plan);
        }
        if let Some(number) = self.exponentiation_static_unary_number_after_effects(expression) {
            return Some(number);
        }
        let target = Self::exponentiation_coercion_target_expression(expression);
        match self.resolve_static_to_primitive_outcome_with_context(
            target,
            PrimitiveHint::Number,
            self.current_function_name(),
        )? {
            StaticEvalOutcome::Value(value) => Some(value),
            StaticEvalOutcome::Throw(_) => None,
        }
    }

    fn emit_effectful_static_exponentiation(
        &mut self,
        base: &Expression,
        exponent: &Expression,
    ) -> DirectResult<bool> {
        let base_plan = self.exponentiation_effectful_ordinary_plan(base);
        let exponent_plan = self.exponentiation_effectful_ordinary_plan(exponent);
        if base_plan.is_none() && exponent_plan.is_none() {
            return Ok(false);
        }
        let Some(base_primitive) =
            self.exponentiation_static_primitive_after_effects(base, base_plan.as_ref())
        else {
            return Ok(false);
        };
        let Some(exponent_primitive) =
            self.exponentiation_static_primitive_after_effects(exponent, exponent_plan.as_ref())
        else {
            return Ok(false);
        };
        let Some(outcome) = self.resolve_static_numeric_binary_outcome_with_context(
            BinaryOp::Exponentiate,
            &base_primitive,
            &exponent_primitive,
            self.current_function_name(),
        ) else {
            return Ok(false);
        };

        let base_local = self.allocate_temp_local();
        let exponent_local = self.allocate_temp_local();
        self.emit_numeric_expression(base)?;
        self.push_local_set(base_local);
        self.emit_numeric_expression(exponent)?;
        self.push_local_set(exponent_local);

        if let Some(plan) = base_plan.as_ref() {
            let target = Self::exponentiation_coercion_target_expression(base);
            match self.emit_ordinary_to_primitive_from_plan(target, plan, base_local)? {
                SymbolToPrimitiveHandling::AlwaysThrows => {
                    self.push_i32_const(JS_UNDEFINED_TAG);
                    return Ok(true);
                }
                SymbolToPrimitiveHandling::Handled => {}
                SymbolToPrimitiveHandling::NotHandled => return Ok(false),
            }
        }
        if let Some(plan) = exponent_plan.as_ref() {
            let target = Self::exponentiation_coercion_target_expression(exponent);
            match self.emit_ordinary_to_primitive_from_plan(target, plan, exponent_local)? {
                SymbolToPrimitiveHandling::AlwaysThrows => {
                    self.push_i32_const(JS_UNDEFINED_TAG);
                    return Ok(true);
                }
                SymbolToPrimitiveHandling::Handled => {}
                SymbolToPrimitiveHandling::NotHandled => return Ok(false),
            }
        }

        self.emit_static_eval_outcome(&outcome)?;
        Ok(true)
    }

    pub(in crate::backend::direct_wasm) fn emit_exponentiate(
        &mut self,
        base: &Expression,
        exponent: &Expression,
    ) -> DirectResult<()> {
        let base_kind = self.exponentiation_numeric_kind(base);
        let exponent_kind = self.exponentiation_numeric_kind(exponent);
        if matches!(
            (base_kind, exponent_kind),
            (Some(StaticValueKind::BigInt), Some(StaticValueKind::Number))
                | (Some(StaticValueKind::Number), Some(StaticValueKind::BigInt))
        ) {
            self.emit_exponentiation_operand_effects(base, exponent)?;
            return self.emit_named_error_throw("TypeError");
        }
        if matches!(
            (base_kind, exponent_kind),
            (Some(StaticValueKind::BigInt), Some(StaticValueKind::BigInt))
        ) && let Some(exponent_value) = self.resolve_static_bigint_value(exponent)
            && exponent_value < StaticBigInt::from(0)
        {
            if !inline_summary_side_effect_free_expression(base) {
                self.emit_numeric_expression(base)?;
                self.state.emission.output.instructions.push(0x1a);
            }
            if !inline_summary_side_effect_free_expression(exponent) {
                self.emit_numeric_expression(exponent)?;
                self.state.emission.output.instructions.push(0x1a);
            }
            return self.emit_named_error_throw("RangeError");
        }
        if self.emit_effectful_static_exponentiation(base, exponent)? {
            return Ok(());
        }
        if let Some(outcome) = self.resolve_static_numeric_binary_outcome_with_context(
            BinaryOp::Exponentiate,
            base,
            exponent,
            self.current_function_name(),
        ) {
            let can_emit_outcome = matches!(&outcome, StaticEvalOutcome::Throw(_))
                || (inline_summary_side_effect_free_expression(base)
                    && inline_summary_side_effect_free_expression(exponent));
            if can_emit_outcome {
                return self.emit_static_eval_outcome(&outcome);
            }
        }

        if let Some(result) = self.active_loop_static_exponentiation_result(base, exponent) {
            return self.emit_static_exponentiation_result_after_effects(base, exponent, result);
        }
        if let (Some(base_value), Some(exponent_value)) = (
            self.exponentiation_static_number_value(base),
            self.exponentiation_static_number_value(exponent),
        ) {
            let result = Self::js_number_exponentiate(base_value, exponent_value);
            return self.emit_static_exponentiation_result_after_effects(base, exponent, result);
        }
        if let Some(base_value) = self.exponentiation_static_number_value(base)
            && base_value.is_nan()
        {
            return self.emit_static_exponentiation_result_after_effects(base, exponent, f64::NAN);
        }
        if let Some(exponent_value) = self.exponentiation_static_number_value(exponent)
            && exponent_value.is_nan()
        {
            return self.emit_static_exponentiation_result_after_effects(base, exponent, f64::NAN);
        }

        let base_local = self.allocate_temp_local();
        let result_local = self.allocate_temp_local();
        let exponent_local = self.allocate_temp_local();

        self.emit_numeric_expression(base)?;
        self.push_local_set(base_local);

        if let Expression::Number(power) = exponent {
            let power = f64_to_i32(*power)?;
            if power < 0 {
                self.push_i32_const(0);
            } else {
                self.push_i32_const(power);
            }
        } else {
            self.emit_numeric_expression(exponent)?;
        }
        self.push_local_set(exponent_local);

        self.push_i32_const(1);
        self.push_local_set(result_local);

        self.state.emission.output.instructions.push(0x02);
        self.state
            .emission
            .output
            .instructions
            .push(EMPTY_BLOCK_TYPE);
        let break_target = self.push_control_frame();

        self.state.emission.output.instructions.push(0x03);
        self.state
            .emission
            .output
            .instructions
            .push(EMPTY_BLOCK_TYPE);
        let loop_target = self.push_control_frame();

        self.push_local_get(exponent_local);
        self.push_i32_const(0);
        self.push_binary_op(BinaryOp::LessThanOrEqual)?;
        self.push_br_if(self.relative_depth(break_target));

        self.push_local_get(result_local);
        self.push_local_get(base_local);
        self.state.emission.output.instructions.push(0x6c);
        self.push_local_set(result_local);

        self.push_local_get(exponent_local);
        self.push_i32_const(1);
        self.state.emission.output.instructions.push(0x6b);
        self.push_local_set(exponent_local);

        self.push_br(self.relative_depth(loop_target));
        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();
        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();

        self.push_local_get(result_local);
        Ok(())
    }
}
