use super::*;
use crate::ir::hir::js_string_utf16_code_units;

fn static_js_string_code_unit_ordering(left: &str, right: &str) -> std::cmp::Ordering {
    let left_units = js_string_utf16_code_units(left);
    let right_units = js_string_utf16_code_units(right);
    let mut left_units = left_units.iter();
    let mut right_units = right_units.iter();
    loop {
        match (left_units.next(), right_units.next()) {
            (Some(left_unit), Some(right_unit)) => match left_unit.cmp(&right_unit) {
                std::cmp::Ordering::Equal => continue,
                ordering => return ordering,
            },
            (None, None) => return std::cmp::Ordering::Equal,
            (None, Some(_)) => return std::cmp::Ordering::Less,
            (Some(_), None) => return std::cmp::Ordering::Greater,
        }
    }
}

fn static_relational_bool_from_ordering(
    op: BinaryOp,
    ordering: std::cmp::Ordering,
) -> Option<bool> {
    Some(match op {
        BinaryOp::LessThan => ordering == std::cmp::Ordering::Less,
        BinaryOp::LessThanOrEqual => ordering != std::cmp::Ordering::Greater,
        BinaryOp::GreaterThan => ordering == std::cmp::Ordering::Greater,
        BinaryOp::GreaterThanOrEqual => ordering != std::cmp::Ordering::Less,
        _ => return None,
    })
}

fn static_bigint_from_integral_f64(value: f64) -> Option<StaticBigInt> {
    if !value.is_finite() || value.fract() != 0.0 {
        return None;
    }
    if value == 0.0 {
        return Some(StaticBigInt::from(0));
    }

    let bits = value.to_bits();
    let negative = (bits >> 63) != 0;
    let exponent_bits = ((bits >> 52) & 0x7ff) as i32;
    if exponent_bits == 0 {
        return None;
    }

    let exponent = exponent_bits - 1023;
    let significand = (1u64 << 52) | (bits & ((1u64 << 52) - 1));
    let shift = exponent - 52;
    let mut integer = if shift >= 0 {
        StaticBigInt::from(significand) << (shift as usize)
    } else {
        let discarded_bits = (-shift) as u32;
        if discarded_bits >= 64 {
            return None;
        }
        let mask = (1u64 << discarded_bits) - 1;
        if significand & mask != 0 {
            return None;
        }
        StaticBigInt::from(significand >> discarded_bits)
    };
    if negative {
        integer = -integer;
    }
    Some(integer)
}

fn static_string_to_bigint(text: &str) -> Option<StaticBigInt> {
    let trimmed = text.trim();
    if trimmed.ends_with('n') || trimmed.ends_with('N') {
        return None;
    }
    let unsigned = trimmed
        .strip_prefix('+')
        .or_else(|| trimmed.strip_prefix('-'))
        .unwrap_or(trimmed);
    if unsigned.starts_with('+') || unsigned.starts_with('-') {
        return None;
    }
    if trimmed.is_empty() {
        Some(StaticBigInt::from(0))
    } else {
        parse_static_bigint_literal(trimmed)
    }
}

fn static_bigint_number_ordering(bigint: &StaticBigInt, number: f64) -> Option<std::cmp::Ordering> {
    if number.is_nan() {
        return None;
    }
    if number == f64::INFINITY {
        return Some(std::cmp::Ordering::Less);
    }
    if number == f64::NEG_INFINITY {
        return Some(std::cmp::Ordering::Greater);
    }

    let comparison_integer = if number.fract() == 0.0 {
        static_bigint_from_integral_f64(number)?
    } else {
        static_bigint_from_integral_f64(number.floor())?
    };
    let ordering = bigint.cmp(&comparison_integer);
    if number.fract() == 0.0 {
        Some(ordering)
    } else if ordering == std::cmp::Ordering::Greater {
        Some(std::cmp::Ordering::Greater)
    } else {
        Some(std::cmp::Ordering::Less)
    }
}

fn static_js_to_uint32(value: f64) -> u32 {
    if !value.is_finite() || value == 0.0 {
        return 0;
    }
    let truncated = value.trunc();
    let modulo = truncated.rem_euclid(4_294_967_296.0);
    modulo as u32
}

fn static_js_to_int32(value: f64) -> i32 {
    static_js_to_uint32(value) as i32
}

fn static_exponentiation_is_odd_integer(value: f64) -> bool {
    value.is_finite()
        && value.fract() == 0.0
        && value.abs() <= i64::MAX as f64
        && (value as i64).abs() % 2 == 1
}

fn static_js_number_exponentiate(base: f64, exponent: f64) -> f64 {
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
        let odd = static_exponentiation_is_odd_integer(exponent);
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
            let odd = static_exponentiation_is_odd_integer(exponent);
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

impl<'a> FunctionCompiler<'a> {
    fn resolve_static_native_error_to_string_from_binding(
        &self,
        object_binding: &ObjectValueBinding,
        current_function_name: Option<&str>,
    ) -> Option<String> {
        let name_property = Expression::String("name".to_string());
        let message_property = Expression::String("message".to_string());
        let name = object_binding_lookup_value(object_binding, &name_property)
            .and_then(|value| self.resolve_static_string_concat_value(value, current_function_name))
            .unwrap_or_else(|| "Error".to_string());
        let message = object_binding_lookup_value(object_binding, &message_property)
            .and_then(|value| self.resolve_static_string_concat_value(value, current_function_name))
            .unwrap_or_default();

        if name.is_empty() {
            Some(message)
        } else if message.is_empty() {
            Some(name)
        } else {
            Some(format!("{name}: {message}"))
        }
    }

    pub(in crate::backend::direct_wasm) fn resolve_static_member_call_outcome_with_context(
        &self,
        object: &Expression,
        property_name: &str,
        current_function_name: Option<&str>,
    ) -> Option<StaticEvalOutcome> {
        let property = Expression::String(property_name.to_string());
        if is_private_property_name_expression(&property) {
            return None;
        }
        if let Some(binding) = self.resolve_member_function_binding(object, &property) {
            if let LocalFunctionBinding::User(function_name) = &binding
                && let Some(user_function) = self.user_function(function_name)
                && (self.user_function_mentions_private_member_access(user_function)
                    || self.user_function_mentions_direct_eval(user_function)
                    || user_function.has_parameter_defaults()
                    || user_function.has_lowered_pattern_parameters()
                    || !self
                        .user_function_parameter_iterator_consumption_indices(user_function)
                        .is_empty())
            {
                return None;
            }
            let capture_slots = self.resolve_member_function_capture_slots(object, &property);
            if capture_slots
                .as_ref()
                .is_some_and(|capture_slots| capture_slots.contains_key("new.target"))
            {
                return None;
            }
            if let Some(outcome) = self
                .resolve_static_function_outcome_from_binding_with_call_frame_and_context(
                    &binding,
                    &[],
                    object,
                    current_function_name,
                )
            {
                return Some(match outcome {
                    StaticEvalOutcome::Value(expression) => {
                        let expression = capture_slots
                            .as_ref()
                            .map(|capture_slots| {
                                self.substitute_capture_slot_bindings(&expression, capture_slots)
                            })
                            .unwrap_or(expression);
                        StaticEvalOutcome::Value(self.materialize_static_expression(&expression))
                    }
                    StaticEvalOutcome::Throw(throw_value) => StaticEvalOutcome::Throw(throw_value),
                });
            }
            if let Some(result) = self
                .resolve_function_binding_static_return_expression_with_call_frame(
                    &binding,
                    &[],
                    object,
                )
            {
                let result = capture_slots
                    .as_ref()
                    .map(|capture_slots| {
                        self.substitute_capture_slot_bindings(&result, capture_slots)
                    })
                    .unwrap_or(result);
                return Some(StaticEvalOutcome::Value(
                    self.materialize_static_expression(&result),
                ));
            }
            if property_name == "valueOf"
                && matches!(
                    &binding,
                    LocalFunctionBinding::Builtin(function_name)
                        if function_name == "Object.prototype.valueOf"
                )
                && self.resolve_array_binding_from_expression(object).is_some()
            {
                return Some(StaticEvalOutcome::Value(
                    self.materialize_static_expression(object),
                ));
            }
        }
        if let Some(object_binding) = self.resolve_object_binding_from_expression(object)
            && let Some(method_value) = object_binding_lookup_value(&object_binding, &property)
        {
            let binding = self.resolve_function_binding_from_expression_with_context(
                method_value,
                current_function_name,
            )?;
            if let LocalFunctionBinding::User(function_name) = &binding
                && let Some(user_function) = self.user_function(function_name)
                && (self.user_function_mentions_direct_eval(user_function)
                    || user_function.has_parameter_defaults()
                    || user_function.has_lowered_pattern_parameters()
                    || !self
                        .user_function_parameter_iterator_consumption_indices(user_function)
                        .is_empty())
            {
                return None;
            }
            return self.resolve_static_function_outcome_from_binding_with_call_frame_and_context(
                &binding,
                &[],
                object,
                current_function_name,
            );
        }

        if property_name == "toString"
            && let Some(array_binding) = self.resolve_array_binding_from_expression(object)
        {
            let mut parts = Vec::with_capacity(array_binding.values.len());
            for value in &array_binding.values {
                let Some(value) = value else {
                    parts.push(String::new());
                    continue;
                };
                let materialized = self
                    .resolve_static_primitive_expression_with_context(value, current_function_name)
                    .unwrap_or_else(|| self.materialize_static_expression(value));
                let text = match materialized {
                    Expression::Undefined | Expression::Null => String::new(),
                    _ => self
                        .resolve_static_string_concat_value(&materialized, current_function_name)?,
                };
                parts.push(text);
            }
            return Some(StaticEvalOutcome::Value(Expression::String(
                parts.join(","),
            )));
        }

        if let Expression::Object(entries) = object
            && Self::raw_object_literal_ordinary_to_primitive_method(entries, property_name)
                .is_none()
        {
            return match property_name {
                "valueOf" => Some(StaticEvalOutcome::Value(object.clone())),
                "toString" => Some(StaticEvalOutcome::Value(Expression::String(
                    "[object Object]".to_string(),
                ))),
                _ => None,
            };
        }

        if matches!(property_name, "toString" | "valueOf")
            && let Some(value) =
                self.resolve_static_primitive_expression_with_context(object, current_function_name)
            && matches!(
                value,
                Expression::Bool(_)
                    | Expression::Number(_)
                    | Expression::String(_)
                    | Expression::BigInt(_)
            )
        {
            return match property_name {
                "valueOf" => Some(StaticEvalOutcome::Value(value)),
                "toString" => Some(StaticEvalOutcome::Value(Expression::String(
                    self.resolve_static_string_concat_value(&value, current_function_name)?,
                ))),
                _ => None,
            };
        }

        if matches!(property_name, "toLowerCase" | "toUpperCase")
            && let Some(text) =
                self.resolve_static_string_value_with_context(object, current_function_name)
        {
            return Some(StaticEvalOutcome::Value(Expression::String(
                match property_name {
                    "toLowerCase" => text.to_lowercase(),
                    "toUpperCase" => text.to_uppercase(),
                    _ => return None,
                },
            )));
        }

        if let Some(value) = self.resolve_static_boxed_primitive_value(object) {
            return match property_name {
                "valueOf" => Some(StaticEvalOutcome::Value(value)),
                "toString" => Some(StaticEvalOutcome::Value(Expression::String(
                    self.resolve_static_string_concat_value(&value, current_function_name)?,
                ))),
                _ => None,
            };
        }

        if let Some(symbol_text) =
            self.resolve_static_symbol_to_string_value_with_context(object, current_function_name)
        {
            return match property_name {
                "toString" => Some(StaticEvalOutcome::Value(Expression::String(symbol_text))),
                "valueOf" => Some(StaticEvalOutcome::Value(
                    self.resolve_bound_alias_expression(object)
                        .unwrap_or_else(|| self.materialize_static_expression(object)),
                )),
                _ => None,
            };
        }

        if let Some(timestamp) = self.resolve_static_date_timestamp(object) {
            return match property_name {
                "toString" => Some(StaticEvalOutcome::Value(Expression::String(
                    self.synthesize_static_date_string(timestamp),
                ))),
                "valueOf" => Some(StaticEvalOutcome::Value(Expression::Number(timestamp))),
                "getFullYear" | "getUTCFullYear" | "getMonth" | "getUTCMonth" | "getDate"
                | "getUTCDate" => Some(StaticEvalOutcome::Value(Expression::Number(
                    self.resolve_static_date_component(timestamp, property_name)?,
                ))),
                _ => None,
            };
        }

        if let Some(binding) = self
            .resolve_function_binding_from_expression_with_context(object, current_function_name)
        {
            return match property_name {
                "valueOf" => Some(StaticEvalOutcome::Value(
                    self.materialize_static_expression(object),
                )),
                "toString" => {
                    let text = match binding {
                        LocalFunctionBinding::User(function_name) => {
                            self.synthesize_static_function_to_string(&function_name)
                        }
                        LocalFunctionBinding::Builtin(function_name) => format!(
                            "function {}() {{ [native code] }}",
                            builtin_function_display_name(&function_name)
                        ),
                    };
                    Some(StaticEvalOutcome::Value(Expression::String(text)))
                }
                _ => None,
            };
        }

        if self
            .resolve_object_binding_from_expression(object)
            .is_some()
        {
            if property_name == "toString"
                && let Some(object_binding) = self.resolve_object_binding_from_expression(object)
                && object_binding_lookup_value(
                    &object_binding,
                    &Expression::String("constructor".to_string()),
                )
                .is_some_and(|constructor| {
                    matches!(constructor, Expression::Identifier(name) if native_error_runtime_value(name).is_some())
                })
            {
                return Some(StaticEvalOutcome::Value(Expression::String(
                    self.resolve_static_native_error_to_string_from_binding(
                        &object_binding,
                        current_function_name,
                    )?,
                )));
            }
            return match property_name {
                "valueOf" => Some(StaticEvalOutcome::Value(
                    self.materialize_static_expression(object),
                )),
                "toString" => Some(StaticEvalOutcome::Value(Expression::String(
                    "[object Object]".to_string(),
                ))),
                _ => None,
            };
        }

        None
    }

    pub(in crate::backend::direct_wasm) fn resolve_static_to_primitive_outcome_with_context(
        &self,
        expression: &Expression,
        hint: PrimitiveHint,
        current_function_name: Option<&str>,
    ) -> Option<StaticEvalOutcome> {
        let resolved = self
            .resolve_bound_alias_expression(expression)
            .unwrap_or_else(|| expression.clone());
        if let Some(primitive) =
            self.resolve_static_primitive_expression_with_context(&resolved, current_function_name)
        {
            return Some(StaticEvalOutcome::Value(primitive));
        }
        if let Expression::Member { object, property } = &resolved
            && let Some(value) = self.resolve_static_member_getter_value_with_context(
                object,
                property,
                current_function_name,
            )
            && !static_expression_matches(&value, &resolved)
        {
            if let Some(primitive) = self.resolve_static_boxed_primitive_value(&value) {
                return Some(StaticEvalOutcome::Value(primitive));
            }
            if let Some(outcome) = self.resolve_static_to_primitive_outcome_with_context(
                &value,
                hint,
                current_function_name,
            ) {
                return Some(outcome);
            }
        }
        let materialized = self.materialize_static_expression(&resolved);
        if !static_expression_matches(&materialized, &resolved)
            && self.expression_is_static_boxed_primitive_object(&materialized)
            && let Some(primitive) = self.resolve_static_boxed_primitive_value(&materialized)
        {
            return Some(StaticEvalOutcome::Value(primitive));
        }

        if let Some(outcome) = self.resolve_static_symbol_to_primitive_outcome_with_context(
            expression,
            current_function_name,
        ) {
            return Some(outcome);
        }
        if !static_expression_matches(&resolved, expression)
            && let Some(outcome) = self.resolve_static_symbol_to_primitive_outcome_with_context(
                &resolved,
                current_function_name,
            )
        {
            return Some(outcome);
        }
        if self.symbol_to_primitive_requires_runtime_with_context(expression, current_function_name)
            || (!static_expression_matches(&resolved, expression)
                && self.symbol_to_primitive_requires_runtime_with_context(
                    &resolved,
                    current_function_name,
                ))
        {
            return None;
        }

        let coercion_target = if matches!(expression, Expression::Identifier(_)) {
            expression
        } else {
            &resolved
        };

        if let Expression::Object(entries) = coercion_target
            && let Some(plan) =
                self.resolve_raw_object_literal_ordinary_to_primitive_plan(coercion_target, entries)
        {
            for step in plan.steps {
                match step.outcome {
                    StaticEvalOutcome::Throw(throw_value) => {
                        return Some(StaticEvalOutcome::Throw(throw_value));
                    }
                    StaticEvalOutcome::Value(value) => {
                        if let Some(primitive) = self
                            .resolve_static_primitive_expression_with_context(
                                &value,
                                current_function_name,
                            )
                        {
                            return Some(StaticEvalOutcome::Value(primitive));
                        }
                    }
                }
            }
            return Some(StaticEvalOutcome::Throw(StaticThrowValue::NamedError(
                "TypeError",
            )));
        }

        if self.ordinary_to_primitive_requires_runtime_with_context(
            coercion_target,
            current_function_name,
        ) {
            return None;
        }

        let prefers_string = matches!(hint, PrimitiveHint::Default)
            && self
                .resolve_static_date_timestamp(coercion_target)
                .is_some();
        let method_order = if prefers_string {
            ["toString", "valueOf"]
        } else {
            ["valueOf", "toString"]
        };

        for method_name in method_order {
            let outcome = self.resolve_static_member_call_outcome_with_context(
                coercion_target,
                method_name,
                current_function_name,
            );
            match outcome {
                Some(StaticEvalOutcome::Value(value)) => {
                    if let Some(primitive) = self.resolve_static_primitive_expression_with_context(
                        &value,
                        current_function_name,
                    ) {
                        return Some(StaticEvalOutcome::Value(primitive));
                    }
                }
                Some(StaticEvalOutcome::Throw(throw_value)) => {
                    return Some(StaticEvalOutcome::Throw(throw_value));
                }
                None => continue,
            }
        }

        if self
            .resolve_object_binding_from_expression(coercion_target)
            .is_some()
            || self
                .resolve_static_date_timestamp(coercion_target)
                .is_some()
            || self
                .resolve_function_binding_from_expression_with_context(
                    coercion_target,
                    current_function_name,
                )
                .is_some()
        {
            return Some(StaticEvalOutcome::Throw(StaticThrowValue::NamedError(
                "TypeError",
            )));
        }

        None
    }

    pub(in crate::backend::direct_wasm) fn resolve_static_addition_outcome_with_context(
        &self,
        left: &Expression,
        right: &Expression,
        current_function_name: Option<&str>,
    ) -> Option<StaticEvalOutcome> {
        if self.expression_depends_on_active_loop_assignment(left)
            || self.expression_depends_on_active_loop_assignment(right)
        {
            return None;
        }
        if current_function_name.is_some()
            && (self.addition_operand_requires_runtime_value(left)
                || self.addition_operand_requires_runtime_value(right))
        {
            return None;
        }
        let left_primitive = self.resolve_static_to_primitive_outcome_with_context(
            left,
            PrimitiveHint::Default,
            current_function_name,
        )?;
        let right_primitive = self.resolve_static_to_primitive_outcome_with_context(
            right,
            PrimitiveHint::Default,
            current_function_name,
        )?;
        let (left_value, right_value) = match (left_primitive, right_primitive) {
            (StaticEvalOutcome::Throw(throw_value), _)
            | (_, StaticEvalOutcome::Throw(throw_value)) => {
                return Some(StaticEvalOutcome::Throw(throw_value));
            }
            (StaticEvalOutcome::Value(left_value), StaticEvalOutcome::Value(right_value)) => {
                (left_value, right_value)
            }
        };

        if self
            .resolve_static_symbol_to_string_value_with_context(&left_value, current_function_name)
            .is_some()
            || self
                .resolve_static_symbol_to_string_value_with_context(
                    &right_value,
                    current_function_name,
                )
                .is_some()
        {
            return Some(StaticEvalOutcome::Throw(StaticThrowValue::NamedError(
                "TypeError",
            )));
        }

        if self.infer_value_kind(&left_value) == Some(StaticValueKind::String)
            || self.infer_value_kind(&right_value) == Some(StaticValueKind::String)
        {
            return Some(StaticEvalOutcome::Value(Expression::String(format!(
                "{}{}",
                self.resolve_static_string_concat_value(&left_value, current_function_name)?,
                self.resolve_static_string_concat_value(&right_value, current_function_name)?,
            ))));
        }

        let left_kind = self.infer_value_kind(&left_value);
        let right_kind = self.infer_value_kind(&right_value);
        if left_kind == Some(StaticValueKind::BigInt) && right_kind == Some(StaticValueKind::BigInt)
        {
            return Some(StaticEvalOutcome::Value(Expression::BigInt(
                (self.resolve_static_bigint_value(&left_value)?
                    + self.resolve_static_bigint_value(&right_value)?)
                .to_string(),
            )));
        }
        if left_kind == Some(StaticValueKind::BigInt) || right_kind == Some(StaticValueKind::BigInt)
        {
            return Some(StaticEvalOutcome::Throw(StaticThrowValue::NamedError(
                "TypeError",
            )));
        }

        Some(StaticEvalOutcome::Value(Expression::Number(
            self.resolve_static_number_value(&left_value)?
                + self.resolve_static_number_value(&right_value)?,
        )))
    }

    pub(in crate::backend::direct_wasm) fn resolve_static_numeric_binary_outcome_with_context(
        &self,
        op: BinaryOp,
        left: &Expression,
        right: &Expression,
        current_function_name: Option<&str>,
    ) -> Option<StaticEvalOutcome> {
        if self.expression_depends_on_active_loop_assignment(left)
            || self.expression_depends_on_active_loop_assignment(right)
        {
            return None;
        }
        if !matches!(
            op,
            BinaryOp::Subtract
                | BinaryOp::Multiply
                | BinaryOp::Divide
                | BinaryOp::Modulo
                | BinaryOp::Exponentiate
                | BinaryOp::BitwiseAnd
                | BinaryOp::BitwiseOr
                | BinaryOp::BitwiseXor
                | BinaryOp::LeftShift
                | BinaryOp::RightShift
                | BinaryOp::UnsignedRightShift
        ) {
            return None;
        }

        let left_primitive = self.resolve_static_to_primitive_outcome_with_context(
            left,
            PrimitiveHint::Number,
            current_function_name,
        )?;
        let right_primitive = self.resolve_static_to_primitive_outcome_with_context(
            right,
            PrimitiveHint::Number,
            current_function_name,
        )?;
        let (left_value, right_value) = match (left_primitive, right_primitive) {
            (StaticEvalOutcome::Throw(throw_value), _)
            | (_, StaticEvalOutcome::Throw(throw_value)) => {
                return Some(StaticEvalOutcome::Throw(throw_value));
            }
            (StaticEvalOutcome::Value(left_value), StaticEvalOutcome::Value(right_value)) => {
                (left_value, right_value)
            }
        };

        if self
            .resolve_static_symbol_to_string_value_with_context(&left_value, current_function_name)
            .is_some()
            || self
                .resolve_static_symbol_to_string_value_with_context(
                    &right_value,
                    current_function_name,
                )
                .is_some()
        {
            return Some(StaticEvalOutcome::Throw(StaticThrowValue::NamedError(
                "TypeError",
            )));
        }

        let left_kind = self.infer_value_kind(&left_value);
        let right_kind = self.infer_value_kind(&right_value);
        if left_kind == Some(StaticValueKind::BigInt) && right_kind == Some(StaticValueKind::BigInt)
        {
            if op == BinaryOp::UnsignedRightShift {
                return Some(StaticEvalOutcome::Throw(StaticThrowValue::NamedError(
                    "TypeError",
                )));
            }
            let left_bigint = self.resolve_static_bigint_value(&left_value)?;
            let right_bigint = self.resolve_static_bigint_value(&right_value)?;
            let zero = StaticBigInt::from(0);
            return Some(StaticEvalOutcome::Value(Expression::BigInt(
                match op {
                    BinaryOp::Subtract => &left_bigint - &right_bigint,
                    BinaryOp::Multiply => &left_bigint * &right_bigint,
                    BinaryOp::Divide => {
                        if right_bigint == zero {
                            return Some(StaticEvalOutcome::Throw(StaticThrowValue::NamedError(
                                "RangeError",
                            )));
                        }
                        &left_bigint / &right_bigint
                    }
                    BinaryOp::Modulo => {
                        if right_bigint == zero {
                            return Some(StaticEvalOutcome::Throw(StaticThrowValue::NamedError(
                                "RangeError",
                            )));
                        }
                        &left_bigint % &right_bigint
                    }
                    BinaryOp::Exponentiate => {
                        if right_bigint < zero {
                            return Some(StaticEvalOutcome::Throw(StaticThrowValue::NamedError(
                                "RangeError",
                            )));
                        }
                        left_bigint.pow(u32::try_from(right_bigint).ok()?)
                    }
                    BinaryOp::BitwiseAnd => left_bigint & right_bigint,
                    BinaryOp::BitwiseOr => left_bigint | right_bigint,
                    BinaryOp::BitwiseXor => left_bigint ^ right_bigint,
                    BinaryOp::LeftShift => {
                        let shift = i64::try_from(right_bigint).ok()?;
                        if shift >= 0 {
                            left_bigint << usize::try_from(shift).ok()?
                        } else {
                            left_bigint >> usize::try_from(-shift).ok()?
                        }
                    }
                    BinaryOp::RightShift => {
                        let shift = i64::try_from(right_bigint).ok()?;
                        if shift >= 0 {
                            left_bigint >> usize::try_from(shift).ok()?
                        } else {
                            left_bigint << usize::try_from(-shift).ok()?
                        }
                    }
                    _ => unreachable!("filtered above"),
                }
                .to_string(),
            )));
        }
        if left_kind == Some(StaticValueKind::BigInt) || right_kind == Some(StaticValueKind::BigInt)
        {
            return Some(StaticEvalOutcome::Throw(StaticThrowValue::NamedError(
                "TypeError",
            )));
        }

        let left_number = self.resolve_static_number_value(&left_value)?;
        let right_number = self.resolve_static_number_value(&right_value)?;
        Some(StaticEvalOutcome::Value(Expression::Number(match op {
            BinaryOp::Subtract => left_number - right_number,
            BinaryOp::Multiply => left_number * right_number,
            BinaryOp::Divide => left_number / right_number,
            BinaryOp::Modulo => left_number % right_number,
            BinaryOp::Exponentiate => static_js_number_exponentiate(left_number, right_number),
            BinaryOp::BitwiseAnd => {
                (static_js_to_int32(left_number) & static_js_to_int32(right_number)) as f64
            }
            BinaryOp::BitwiseOr => {
                (static_js_to_int32(left_number) | static_js_to_int32(right_number)) as f64
            }
            BinaryOp::BitwiseXor => {
                (static_js_to_int32(left_number) ^ static_js_to_int32(right_number)) as f64
            }
            BinaryOp::LeftShift => {
                (static_js_to_int32(left_number) << (static_js_to_uint32(right_number) & 0x1f))
                    as f64
            }
            BinaryOp::RightShift => {
                (static_js_to_int32(left_number) >> (static_js_to_uint32(right_number) & 0x1f))
                    as f64
            }
            BinaryOp::UnsignedRightShift => {
                (static_js_to_uint32(left_number) >> (static_js_to_uint32(right_number) & 0x1f))
                    as f64
            }
            _ => unreachable!("filtered above"),
        })))
    }

    pub(in crate::backend::direct_wasm) fn resolve_static_relational_outcome_with_context(
        &self,
        op: BinaryOp,
        left: &Expression,
        right: &Expression,
        current_function_name: Option<&str>,
    ) -> Option<StaticEvalOutcome> {
        if self.expression_depends_on_active_loop_assignment(left)
            || self.expression_depends_on_active_loop_assignment(right)
        {
            return None;
        }
        if !matches!(
            op,
            BinaryOp::LessThan
                | BinaryOp::LessThanOrEqual
                | BinaryOp::GreaterThan
                | BinaryOp::GreaterThanOrEqual
        ) {
            return None;
        }

        let left_primitive = self.resolve_static_to_primitive_outcome_with_context(
            left,
            PrimitiveHint::Number,
            current_function_name,
        )?;
        let right_primitive = self.resolve_static_to_primitive_outcome_with_context(
            right,
            PrimitiveHint::Number,
            current_function_name,
        )?;
        let (left_value, right_value) = match (left_primitive, right_primitive) {
            (StaticEvalOutcome::Throw(throw_value), _)
            | (_, StaticEvalOutcome::Throw(throw_value)) => {
                return Some(StaticEvalOutcome::Throw(throw_value));
            }
            (StaticEvalOutcome::Value(left_value), StaticEvalOutcome::Value(right_value)) => {
                (left_value, right_value)
            }
        };

        if self
            .resolve_static_symbol_to_string_value_with_context(&left_value, current_function_name)
            .is_some()
            || self
                .resolve_static_symbol_to_string_value_with_context(
                    &right_value,
                    current_function_name,
                )
                .is_some()
        {
            return Some(StaticEvalOutcome::Throw(StaticThrowValue::NamedError(
                "TypeError",
            )));
        }

        let left_kind = self.infer_value_kind(&left_value);
        let right_kind = self.infer_value_kind(&right_value);
        let result = if left_kind == Some(StaticValueKind::String)
            && right_kind == Some(StaticValueKind::String)
            && let (Some(left_string), Some(right_string)) = (
                self.resolve_static_string_value_with_context(&left_value, current_function_name),
                self.resolve_static_string_value_with_context(&right_value, current_function_name),
            ) {
            let ordering = static_js_string_code_unit_ordering(&left_string, &right_string);
            static_relational_bool_from_ordering(op, ordering)?
        } else if left_kind == Some(StaticValueKind::BigInt)
            && right_kind == Some(StaticValueKind::BigInt)
        {
            let left_bigint = self.resolve_static_bigint_value(&left_value)?;
            let right_bigint = self.resolve_static_bigint_value(&right_value)?;
            static_relational_bool_from_ordering(op, left_bigint.cmp(&right_bigint))?
        } else if left_kind == Some(StaticValueKind::BigInt)
            && right_kind == Some(StaticValueKind::String)
        {
            let left_bigint = self.resolve_static_bigint_value(&left_value)?;
            let Some(right_bigint) = self
                .resolve_static_string_value_with_context(&right_value, current_function_name)
                .and_then(|text| static_string_to_bigint(&text))
            else {
                return Some(StaticEvalOutcome::Value(Expression::Bool(false)));
            };
            static_relational_bool_from_ordering(op, left_bigint.cmp(&right_bigint))?
        } else if left_kind == Some(StaticValueKind::String)
            && right_kind == Some(StaticValueKind::BigInt)
        {
            let Some(left_bigint) = self
                .resolve_static_string_value_with_context(&left_value, current_function_name)
                .and_then(|text| static_string_to_bigint(&text))
            else {
                return Some(StaticEvalOutcome::Value(Expression::Bool(false)));
            };
            let right_bigint = self.resolve_static_bigint_value(&right_value)?;
            static_relational_bool_from_ordering(op, left_bigint.cmp(&right_bigint))?
        } else if left_kind == Some(StaticValueKind::BigInt) {
            let left_bigint = self.resolve_static_bigint_value(&left_value)?;
            let Some(ordering) = static_bigint_number_ordering(
                &left_bigint,
                self.resolve_static_number_value(&right_value)?,
            ) else {
                return Some(StaticEvalOutcome::Value(Expression::Bool(false)));
            };
            static_relational_bool_from_ordering(op, ordering)?
        } else if right_kind == Some(StaticValueKind::BigInt) {
            let right_bigint = self.resolve_static_bigint_value(&right_value)?;
            let Some(ordering) = static_bigint_number_ordering(
                &right_bigint,
                self.resolve_static_number_value(&left_value)?,
            ) else {
                return Some(StaticEvalOutcome::Value(Expression::Bool(false)));
            };
            static_relational_bool_from_ordering(op, ordering.reverse())?
        } else {
            let left_number = self.resolve_static_number_value(&left_value)?;
            let right_number = self.resolve_static_number_value(&right_value)?;
            match op {
                BinaryOp::LessThan => left_number < right_number,
                BinaryOp::LessThanOrEqual => left_number <= right_number,
                BinaryOp::GreaterThan => left_number > right_number,
                BinaryOp::GreaterThanOrEqual => left_number >= right_number,
                _ => unreachable!("filtered above"),
            }
        };

        Some(StaticEvalOutcome::Value(Expression::Bool(result)))
    }

    pub(in crate::backend::direct_wasm) fn resolve_static_string_addition_value_with_context(
        &self,
        left: &Expression,
        right: &Expression,
        current_function_name: Option<&str>,
    ) -> Option<String> {
        if self.expression_depends_on_active_loop_assignment(left)
            || self.expression_depends_on_active_loop_assignment(right)
        {
            return None;
        }
        let left_primitive = match self.resolve_static_to_primitive_outcome_with_context(
            left,
            PrimitiveHint::Default,
            current_function_name,
        )? {
            StaticEvalOutcome::Value(value) => value,
            StaticEvalOutcome::Throw(_) => return None,
        };
        let right_primitive = match self.resolve_static_to_primitive_outcome_with_context(
            right,
            PrimitiveHint::Default,
            current_function_name,
        )? {
            StaticEvalOutcome::Value(value) => value,
            StaticEvalOutcome::Throw(_) => return None,
        };

        if self.infer_value_kind(&left_primitive) != Some(StaticValueKind::String)
            && self.infer_value_kind(&right_primitive) != Some(StaticValueKind::String)
        {
            return None;
        }

        Some(format!(
            "{}{}",
            self.resolve_static_string_concat_value(&left_primitive, current_function_name)?,
            self.resolve_static_string_concat_value(&right_primitive, current_function_name)?,
        ))
    }
}
