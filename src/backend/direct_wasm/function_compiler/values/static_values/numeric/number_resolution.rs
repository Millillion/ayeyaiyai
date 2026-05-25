use super::*;

fn js_to_uint32(value: f64) -> u32 {
    if !value.is_finite() || value == 0.0 {
        return 0;
    }
    let truncated = value.trunc();
    let modulo = truncated.rem_euclid(4_294_967_296.0);
    modulo as u32
}

fn js_to_int32(value: f64) -> i32 {
    js_to_uint32(value) as i32
}

const STATIC_NUMBER_VALUE_RECURSION_LIMIT: usize = 128;

thread_local! {
    static STATIC_NUMBER_VALUE_DEPTH: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

struct StaticNumberValueDepthGuard;

impl StaticNumberValueDepthGuard {
    fn enter() -> Option<Self> {
        STATIC_NUMBER_VALUE_DEPTH.with(|depth| {
            let current = depth.get();
            if current >= STATIC_NUMBER_VALUE_RECURSION_LIMIT {
                return None;
            }
            depth.set(current + 1);
            Some(Self)
        })
    }
}

impl Drop for StaticNumberValueDepthGuard {
    fn drop(&mut self) {
        STATIC_NUMBER_VALUE_DEPTH.with(|depth| depth.set(depth.get().saturating_sub(1)));
    }
}

fn js_string_to_number(value: &str) -> f64 {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return 0.0;
    }
    match trimmed {
        "Infinity" | "+Infinity" => return f64::INFINITY,
        "-Infinity" => return f64::NEG_INFINITY,
        _ => {}
    }
    if trimmed.eq_ignore_ascii_case("infinity")
        || trimmed.eq_ignore_ascii_case("+infinity")
        || trimmed.eq_ignore_ascii_case("-infinity")
        || trimmed.eq_ignore_ascii_case("inf")
        || trimmed.eq_ignore_ascii_case("+inf")
        || trimmed.eq_ignore_ascii_case("-inf")
    {
        return f64::NAN;
    }
    if let Some(hex) = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
    {
        return u64::from_str_radix(hex, 16)
            .map(|value| value as f64)
            .unwrap_or(f64::NAN);
    }
    if let Some(binary) = trimmed
        .strip_prefix("0b")
        .or_else(|| trimmed.strip_prefix("0B"))
    {
        return u64::from_str_radix(binary, 2)
            .map(|value| value as f64)
            .unwrap_or(f64::NAN);
    }
    if let Some(octal) = trimmed
        .strip_prefix("0o")
        .or_else(|| trimmed.strip_prefix("0O"))
    {
        return u64::from_str_radix(octal, 8)
            .map(|value| value as f64)
            .unwrap_or(f64::NAN);
    }
    trimmed.parse::<f64>().unwrap_or(f64::NAN)
}

fn expression_is_builtin_object_reference(expression: &Expression) -> bool {
    match expression {
        Expression::Identifier(name) => {
            builtin_identifier_kind(name).is_some() || infer_call_result_kind(name).is_some()
        }
        Expression::Member { object, property } if matches!(property.as_ref(), Expression::String(name) if name == "prototype") =>
        {
            matches!(
                object.as_ref(),
                Expression::Identifier(name)
                    if builtin_identifier_kind(name).is_some()
                        || infer_call_result_kind(name).is_some()
            )
        }
        _ => false,
    }
}

impl<'a> FunctionCompiler<'a> {
    pub(in crate::backend::direct_wasm) fn resolve_static_number_value(
        &self,
        expression: &Expression,
    ) -> Option<f64> {
        let Some(_depth_guard) = StaticNumberValueDepthGuard::enter() else {
            return None;
        };
        if expression_is_builtin_object_reference(expression) {
            return None;
        }
        if self.expression_depends_on_active_loop_assignment(expression) {
            return None;
        }
        if Self::call_is_promise_like_chain(expression) {
            return None;
        }
        if let Expression::Call { callee, .. }
        | Expression::SuperCall { callee, .. }
        | Expression::New { callee, .. } = expression
            && let Some(LocalFunctionBinding::User(function_name)) =
                self.resolve_function_binding_from_expression(callee)
            && self
                .user_function(&function_name)
                .is_some_and(|user_function| {
                    self.user_function_may_read_restricted_function_property(&user_function)
                })
        {
            return None;
        }
        if let Expression::Identifier(name) = expression
            && let Some(resolved) = self
                .resolve_bound_alias_expression(expression)
                .filter(|resolved| !static_expression_matches(resolved, expression))
                .or_else(|| {
                    self.resolve_global_value_expression(expression)
                        .filter(|resolved| !static_expression_matches(resolved, expression))
                })
        {
            let mut referenced_names = HashSet::new();
            collect_referenced_binding_names_from_expression(&resolved, &mut referenced_names);
            if referenced_names.contains(name) {
                return None;
            }
            return self.resolve_static_number_value(&resolved);
        }
        if let Some(value) = self.resolve_static_boxed_primitive_value(expression)
            && !static_expression_matches(&value, expression)
        {
            return self.resolve_static_number_value(&value);
        }
        if !matches!(
            expression,
            Expression::Number(_)
                | Expression::BigInt(_)
                | Expression::String(_)
                | Expression::Bool(_)
                | Expression::Null
                | Expression::Undefined
        ) && matches!(
            self.infer_value_kind(expression),
            Some(StaticValueKind::Object | StaticValueKind::Function)
        ) && let Some(StaticEvalOutcome::Value(value)) = self
            .resolve_static_to_primitive_outcome_with_context(
                expression,
                PrimitiveHint::Number,
                self.current_function_name(),
            )
            && !static_expression_matches(&value, expression)
        {
            return self.resolve_static_number_value(&value);
        }
        if let Expression::Identifier(name) = expression
            && name == "Infinity"
            && self.is_unshadowed_builtin_identifier(name)
        {
            return Some(f64::INFINITY);
        }
        if let Expression::Member { object, property } = expression
            && matches!(object.as_ref(), Expression::Identifier(name) if name == "Number" && self.is_unshadowed_builtin_identifier(name))
            && let Expression::String(property_name) = property.as_ref()
        {
            return match property_name.as_str() {
                "NaN" => Some(f64::NAN),
                "POSITIVE_INFINITY" => Some(f64::INFINITY),
                "NEGATIVE_INFINITY" => Some(f64::NEG_INFINITY),
                "MAX_VALUE" => Some(f64::MAX),
                "MIN_VALUE" => Some(f64::from_bits(1)),
                _ => None,
            };
        }
        if let Expression::Member { object, property } = expression {
            if let Expression::Identifier(object_name) = self.materialize_static_expression(object)
                && self.is_unshadowed_builtin_identifier(&object_name)
                && let Expression::String(property_name) =
                    self.materialize_static_expression(property)
                && let Some(value) = builtin_member_number_value(&object_name, &property_name)
            {
                return Some(value);
            }
            if let Expression::Member {
                object: prototype_owner,
                property: prototype_property,
            } = self.materialize_static_expression(object)
                && matches!(prototype_property.as_ref(), Expression::String(name) if name == "prototype")
                && let Expression::Identifier(object_name) = prototype_owner.as_ref()
                && self.is_unshadowed_builtin_identifier(object_name)
                && let Expression::String(property_name) =
                    self.materialize_static_expression(property)
                && let Some(value) = builtin_prototype_number_value(object_name, &property_name)
            {
                return Some(value);
            }
            if self
                .resolve_user_function_length(object, property)
                .is_some()
            {
                return self
                    .resolve_user_function_length(object, property)
                    .map(f64::from);
            }
            if let Some(bytes_per_element) =
                self.resolve_typed_array_builtin_bytes_per_element(object, property)
            {
                return Some(bytes_per_element as f64);
            }
            if matches!(property.as_ref(), Expression::String(property_name) if property_name == "length")
                && let Some(array_binding) = self.resolve_array_binding_from_expression(object)
            {
                let has_runtime_array_state = self
                    .runtime_array_length_local_for_expression(object)
                    .is_some()
                    || matches!(
                        object.as_ref(),
                        Expression::Identifier(name)
                            if self.is_named_global_array_binding(name)
                                && self.uses_global_runtime_array_state(name)
                    );
                if !has_runtime_array_state {
                    return Some(array_binding.values.len() as f64);
                }
            }
            if matches!(property.as_ref(), Expression::String(property_name) if property_name == "length")
                && self
                    .resolve_function_binding_from_expression(object)
                    .is_none()
                && self
                    .resolve_member_getter_binding(object, property)
                    .is_none()
                && self
                    .resolve_member_function_binding(object, property)
                    .is_none()
                && self
                    .resolve_member_setter_binding(object, property)
                    .is_none()
                && let Expression::String(text) = self.materialize_static_expression(object)
            {
                return Some(text.encode_utf16().count() as f64);
            }
        }
        let materialized = self.materialize_static_expression(expression);
        match materialized {
            Expression::Number(value) => Some(value),
            Expression::Bool(value) => Some(if value { 1.0 } else { 0.0 }),
            Expression::String(value) => Some(js_string_to_number(&value)),
            Expression::Null => Some(0.0),
            Expression::Undefined => Some(f64::NAN),
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                let branch = if self.resolve_static_if_condition_value(&condition)? {
                    &then_expression
                } else {
                    &else_expression
                };
                self.resolve_static_number_value(branch)
            }
            Expression::Identifier(name)
                if name == "undefined" && self.is_unshadowed_builtin_identifier(&name) =>
            {
                Some(f64::NAN)
            }
            Expression::Identifier(name)
                if name == "NaN" && self.is_unshadowed_builtin_identifier(&name) =>
            {
                Some(f64::NAN)
            }
            Expression::Identifier(name)
                if name == "Infinity" && self.is_unshadowed_builtin_identifier(&name) =>
            {
                Some(f64::INFINITY)
            }
            Expression::Unary {
                op: UnaryOp::Plus,
                expression,
            } => self.resolve_static_number_value(&expression),
            Expression::Unary {
                op: UnaryOp::Not,
                expression,
            } => Some(if self.resolve_static_boolean_expression(&expression)? {
                0.0
            } else {
                1.0
            }),
            Expression::Unary {
                op: UnaryOp::BitwiseNot,
                expression,
            } => Some((!js_to_int32(self.resolve_static_number_value(&expression)?)) as f64),
            Expression::Unary {
                op: UnaryOp::TypeOf,
                expression,
            } => Some(js_string_to_number(
                self.infer_typeof_operand_kind(&expression)?
                    .as_typeof_str()?,
            )),
            Expression::Unary {
                op: UnaryOp::Void, ..
            } => Some(f64::NAN),
            Expression::Unary {
                op: UnaryOp::Delete,
                expression,
            } => Some(
                if self.resolve_static_boolean_expression(&Expression::Unary {
                    op: UnaryOp::Delete,
                    expression,
                })? {
                    1.0
                } else {
                    0.0
                },
            ),
            Expression::Unary {
                op: UnaryOp::Negate,
                expression,
            } => Some(-self.resolve_static_number_value(&expression)?),
            Expression::Binary {
                op: BinaryOp::Add,
                left,
                right,
            } => Some(
                self.resolve_static_number_value(&left)?
                    + self.resolve_static_number_value(&right)?,
            ),
            Expression::Binary {
                op: BinaryOp::Subtract,
                left,
                right,
            } => Some(
                self.resolve_static_number_value(&left)?
                    - self.resolve_static_number_value(&right)?,
            ),
            Expression::Binary {
                op: BinaryOp::Multiply,
                left,
                right,
            } => Some(
                self.resolve_static_number_value(&left)?
                    * self.resolve_static_number_value(&right)?,
            ),
            Expression::Binary {
                op: BinaryOp::Divide,
                left,
                right,
            } => Some(
                self.resolve_static_number_value(&left)?
                    / self.resolve_static_number_value(&right)?,
            ),
            Expression::Binary {
                op: BinaryOp::Modulo,
                left,
                right,
            } => Some(
                self.resolve_static_number_value(&left)?
                    % self.resolve_static_number_value(&right)?,
            ),
            Expression::Binary {
                op: BinaryOp::Exponentiate,
                left,
                right,
            } => Some(
                self.resolve_static_number_value(&left)?
                    .powf(self.resolve_static_number_value(&right)?),
            ),
            Expression::Binary {
                op: BinaryOp::BitwiseAnd,
                left,
                right,
            } => Some(
                (js_to_int32(self.resolve_static_number_value(&left)?)
                    & js_to_int32(self.resolve_static_number_value(&right)?))
                    as f64,
            ),
            Expression::Binary {
                op: BinaryOp::BitwiseOr,
                left,
                right,
            } => Some(
                (js_to_int32(self.resolve_static_number_value(&left)?)
                    | js_to_int32(self.resolve_static_number_value(&right)?))
                    as f64,
            ),
            Expression::Binary {
                op: BinaryOp::BitwiseXor,
                left,
                right,
            } => Some(
                (js_to_int32(self.resolve_static_number_value(&left)?)
                    ^ js_to_int32(self.resolve_static_number_value(&right)?))
                    as f64,
            ),
            Expression::Binary {
                op: BinaryOp::LeftShift,
                left,
                right,
            } => Some(
                (js_to_int32(self.resolve_static_number_value(&left)?)
                    << (js_to_uint32(self.resolve_static_number_value(&right)?) & 0x1f))
                    as f64,
            ),
            Expression::Binary {
                op: BinaryOp::RightShift,
                left,
                right,
            } => Some(
                (js_to_int32(self.resolve_static_number_value(&left)?)
                    >> (js_to_uint32(self.resolve_static_number_value(&right)?) & 0x1f))
                    as f64,
            ),
            Expression::Binary {
                op: BinaryOp::UnsignedRightShift,
                left,
                right,
            } => Some(
                (js_to_uint32(self.resolve_static_number_value(&left)?)
                    >> (js_to_uint32(self.resolve_static_number_value(&right)?) & 0x1f))
                    as f64,
            ),
            Expression::Call { callee, arguments } => {
                let (value, callee_function_name) = self
                    .resolve_static_call_result_expression_with_context(
                        &callee,
                        &arguments,
                        self.current_function_name(),
                    )?;
                self.resolve_static_primitive_expression_with_context(
                    &value,
                    callee_function_name
                        .as_deref()
                        .or(self.current_function_name()),
                )
                .and_then(|value| match value {
                    Expression::Number(number) => Some(number),
                    _ => None,
                })
            }
            _ => None,
        }
    }
}
