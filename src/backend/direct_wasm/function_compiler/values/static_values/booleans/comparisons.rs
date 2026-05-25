use super::super::*;

fn generated_class_constructor_binding_name(name: &str) -> Option<&str> {
    if !name.starts_with("__ayy_class_ctor_") {
        return None;
    }
    name.rsplit_once("__name_")
        .map(|(_, binding_name)| binding_name)
        .filter(|binding_name| !binding_name.is_empty())
}

fn class_constructor_identity_aliases_match(left: &Expression, right: &Expression) -> bool {
    let (Expression::Identifier(left_name), Expression::Identifier(right_name)) = (left, right)
    else {
        return false;
    };
    generated_class_constructor_binding_name(left_name).is_some_and(|name| name == right_name)
        || generated_class_constructor_binding_name(right_name)
            .is_some_and(|name| name == left_name)
}

fn expression_is_fresh_object_allocation(expression: &Expression) -> bool {
    matches!(
        expression,
        Expression::Array(_) | Expression::Object(_) | Expression::New { .. }
    ) || matches!(
        expression,
        Expression::Call { callee, .. }
            if matches!(callee.as_ref(), Expression::Identifier(name) if name == "__ayyDynamicImport")
    )
}

fn reference_key_is_known_not_top_level_this(key: &str) -> bool {
    key != "this" && !key.contains("__ayy_scope$")
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

fn static_bigint_from_string_to_bigint(text: &str) -> Option<StaticBigInt> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        Some(StaticBigInt::from(0))
    } else {
        parse_static_bigint_literal(trimmed)
    }
}

impl<'a> FunctionCompiler<'a> {
    pub(in crate::backend::direct_wasm) fn resolve_static_binary_boolean_result(
        &self,
        op: &BinaryOp,
        left: &Expression,
        right: &Expression,
    ) -> Option<bool> {
        match op {
            BinaryOp::Equal
            | BinaryOp::LooseEqual
            | BinaryOp::NotEqual
            | BinaryOp::LooseNotEqual => {
                self.resolve_static_equality_boolean_result(op, left, right)
            }
            BinaryOp::LessThan
            | BinaryOp::LessThanOrEqual
            | BinaryOp::GreaterThan
            | BinaryOp::GreaterThanOrEqual => {
                self.resolve_static_relational_boolean_result(op, left, right)
            }
            _ => None,
        }
    }

    fn resolve_static_equality_boolean_result(
        &self,
        op: &BinaryOp,
        left: &Expression,
        right: &Expression,
    ) -> Option<bool> {
        self.resolve_static_symbol_equality_boolean(op, left, right)
            .or_else(|| self.resolve_static_object_identity_boolean(op, left, right))
            .or_else(|| self.resolve_static_primitive_equality_boolean(op, left, right))
    }

    fn resolve_static_symbol_equality_boolean(
        &self,
        op: &BinaryOp,
        left: &Expression,
        right: &Expression,
    ) -> Option<bool> {
        let left_symbol = self.resolve_symbol_identity_expression(left);
        let right_symbol = self.resolve_symbol_identity_expression(right);
        let is_not_equal = matches!(op, BinaryOp::NotEqual | BinaryOp::LooseNotEqual);
        if let (Some(left_symbol), Some(right_symbol)) =
            (left_symbol.as_ref(), right_symbol.as_ref())
        {
            return Some(static_expression_matches(left_symbol, right_symbol) ^ is_not_equal);
        }
        let symbol_vs_other = (left_symbol.is_some()
            && self.resolve_static_primitive_or_object_identity(right))
            || (right_symbol.is_some() && self.resolve_static_primitive_or_object_identity(left));
        symbol_vs_other.then_some(is_not_equal)
    }

    fn resolve_static_object_identity_boolean(
        &self,
        op: &BinaryOp,
        left: &Expression,
        right: &Expression,
    ) -> Option<bool> {
        if !matches!(op, BinaryOp::Equal | BinaryOp::NotEqual) {
            return None;
        }
        let is_not_equal = matches!(op, BinaryOp::NotEqual);
        let left_reference_key = self.resolve_static_reference_identity_key(left);
        let right_reference_key = self.resolve_static_reference_identity_key(right);
        if self.current_function_name().is_none()
            && ((left_reference_key
                .as_deref()
                .is_some_and(|key| key.starts_with("new-object:"))
                && right_reference_key.as_deref() == Some("this"))
                || (right_reference_key
                    .as_deref()
                    .is_some_and(|key| key.starts_with("new-object:"))
                    && left_reference_key.as_deref() == Some("this")))
        {
            return Some(false ^ is_not_equal);
        }

        let materializes_to_top_level_this = |expression: &Expression| {
            self.current_function_name().is_none()
                && (matches!(expression, Expression::This)
                    || matches!(
                        self.materialize_static_expression(expression),
                        Expression::This
                    ))
        };
        let left_materializes_to_top_level_this = materializes_to_top_level_this(left);
        let right_materializes_to_top_level_this = materializes_to_top_level_this(right);
        if left_materializes_to_top_level_this ^ right_materializes_to_top_level_this {
            let non_this_reference_key = if left_materializes_to_top_level_this {
                right_reference_key.as_deref()
            } else {
                left_reference_key.as_deref()
            };
            if non_this_reference_key.is_some_and(reference_key_is_known_not_top_level_this) {
                return Some(false ^ is_not_equal);
            }
            return None;
        }
        if self.current_function_name().is_none()
            && ((left_reference_key.as_deref() == Some("this"))
                ^ (right_reference_key.as_deref() == Some("this")))
        {
            let non_this_reference_key = if left_reference_key.as_deref() == Some("this") {
                right_reference_key.as_deref()
            } else {
                left_reference_key.as_deref()
            };
            if non_this_reference_key.is_some_and(reference_key_is_known_not_top_level_this) {
                return Some(false ^ is_not_equal);
            }
            return None;
        }
        if let (Some(left_key), Some(right_key)) = (left_reference_key, right_reference_key) {
            if left_key != right_key
                && (left_key.contains("__ayy_scope$") || right_key.contains("__ayy_scope$"))
            {
                return None;
            }
            return Some((left_key == right_key) ^ is_not_equal);
        }
        if let (Some(left_identity), Some(right_identity)) = (
            self.resolve_static_object_identity_expression(left),
            self.resolve_static_object_identity_expression(right),
        ) {
            let involves_uncertain_capture_identity = |expression: &Expression| {
                matches!(
                    expression,
                    Expression::Identifier(name)
                        if name.starts_with("__ayy_capture_binding__")
                            || name.starts_with("__ayy_closure_slot_")
                )
            };
            if involves_uncertain_capture_identity(&left_identity)
                || involves_uncertain_capture_identity(&right_identity)
            {
                return None;
            }
            if expression_is_fresh_object_allocation(&left_identity)
                || expression_is_fresh_object_allocation(&right_identity)
            {
                return Some(false ^ is_not_equal);
            }
            let same_identity = left_identity == right_identity
                || class_constructor_identity_aliases_match(&left_identity, &right_identity);
            return Some(same_identity ^ is_not_equal);
        }
        let object_vs_primitive = (self
            .resolve_static_object_identity_expression(left)
            .is_some()
            && self
                .resolve_static_primitive_expression_with_context(
                    right,
                    self.current_function_name(),
                )
                .is_some())
            || (self
                .resolve_static_object_identity_expression(right)
                .is_some()
                && self
                    .resolve_static_primitive_expression_with_context(
                        left,
                        self.current_function_name(),
                    )
                    .is_some());
        object_vs_primitive.then_some(is_not_equal)
    }

    fn resolve_static_primitive_equality_boolean(
        &self,
        op: &BinaryOp,
        left: &Expression,
        right: &Expression,
    ) -> Option<bool> {
        if matches!(op, BinaryOp::LooseEqual | BinaryOp::LooseNotEqual) {
            let equal = self.resolve_static_loose_equality_boolean(left, right, 0)?;
            return Some(equal ^ matches!(op, BinaryOp::LooseNotEqual));
        }

        let left_primitive = self
            .resolve_static_primitive_expression_with_context(left, self.current_function_name())?;
        let right_primitive = self.resolve_static_primitive_expression_with_context(
            right,
            self.current_function_name(),
        )?;
        let is_not_equal = matches!(op, BinaryOp::NotEqual);
        let equal = match (left_primitive, right_primitive) {
            (Expression::Bool(left), Expression::Bool(right)) => Some(left == right),
            (Expression::Number(left), Expression::Number(right)) => Some(left == right),
            (Expression::String(left), Expression::String(right)) => Some(left == right),
            (Expression::BigInt(left), Expression::BigInt(right)) => {
                Some(parse_static_bigint_literal(&left)? == parse_static_bigint_literal(&right)?)
            }
            (Expression::Null, Expression::Null)
            | (Expression::Undefined, Expression::Undefined) => Some(true),
            (_, _) => Some(false),
        }?;
        Some(equal ^ is_not_equal)
    }

    fn resolve_static_loose_equality_boolean(
        &self,
        left: &Expression,
        right: &Expression,
        depth: usize,
    ) -> Option<bool> {
        if depth > 8 {
            return None;
        }

        let current_function_name = self.current_function_name();
        let left_primitive =
            self.resolve_static_primitive_expression_with_context(left, current_function_name);
        let right_primitive =
            self.resolve_static_primitive_expression_with_context(right, current_function_name);

        match (left_primitive, right_primitive) {
            (Some(left_value), Some(right_value)) => {
                self.resolve_static_loose_primitive_equality_boolean(&left_value, &right_value)
            }
            (Some(left_value), None) => {
                let right_value = self.resolve_static_loose_equality_object_primitive(right)?;
                self.resolve_static_loose_equality_boolean(&left_value, &right_value, depth + 1)
            }
            (None, Some(right_value)) => {
                let left_value = self.resolve_static_loose_equality_object_primitive(left)?;
                self.resolve_static_loose_equality_boolean(&left_value, &right_value, depth + 1)
            }
            (None, None) => {
                self.resolve_static_object_identity_boolean(&BinaryOp::Equal, left, right)
            }
        }
    }

    fn resolve_static_loose_equality_object_primitive(
        &self,
        expression: &Expression,
    ) -> Option<Expression> {
        let outcome = self.resolve_static_to_primitive_outcome_with_context(
            expression,
            PrimitiveHint::Default,
            self.current_function_name(),
        )?;
        match outcome {
            StaticEvalOutcome::Value(value) => Some(value),
            StaticEvalOutcome::Throw(_) => None,
        }
    }

    fn resolve_static_loose_primitive_equality_boolean(
        &self,
        left: &Expression,
        right: &Expression,
    ) -> Option<bool> {
        match (left, right) {
            (Expression::Undefined, Expression::Undefined)
            | (Expression::Null, Expression::Null)
            | (Expression::Undefined, Expression::Null)
            | (Expression::Null, Expression::Undefined) => Some(true),
            (Expression::Bool(left), Expression::Bool(right)) => Some(left == right),
            (Expression::Number(left), Expression::Number(right)) => Some(left == right),
            (Expression::String(left), Expression::String(right)) => Some(left == right),
            (Expression::BigInt(left), Expression::BigInt(right)) => {
                Some(parse_static_bigint_literal(left)? == parse_static_bigint_literal(right)?)
            }
            (Expression::Bool(left), other) => self
                .resolve_static_loose_primitive_equality_boolean(
                    &Expression::Number(if *left { 1.0 } else { 0.0 }),
                    other,
                ),
            (other, Expression::Bool(right)) => self
                .resolve_static_loose_primitive_equality_boolean(
                    other,
                    &Expression::Number(if *right { 1.0 } else { 0.0 }),
                ),
            (Expression::Number(left), Expression::String(_)) => {
                Some(*left == self.resolve_static_number_value(right)?)
            }
            (Expression::String(_), Expression::Number(right)) => {
                Some(self.resolve_static_number_value(left)? == *right)
            }
            (Expression::Number(number), Expression::BigInt(bigint))
            | (Expression::BigInt(bigint), Expression::Number(number)) => {
                let Some(integer) = static_bigint_from_integral_f64(*number) else {
                    return Some(false);
                };
                Some(integer == parse_static_bigint_literal(bigint)?)
            }
            (Expression::String(text), Expression::BigInt(bigint))
            | (Expression::BigInt(bigint), Expression::String(text)) => {
                let Some(parsed) = static_bigint_from_string_to_bigint(text) else {
                    return Some(false);
                };
                Some(parsed == parse_static_bigint_literal(bigint)?)
            }
            _ => Some(false),
        }
    }

    fn resolve_static_relational_boolean_result(
        &self,
        op: &BinaryOp,
        left: &Expression,
        right: &Expression,
    ) -> Option<bool> {
        if Self::expression_contains_assignment_or_update(left)
            || Self::expression_contains_assignment_or_update(right)
        {
            return None;
        }

        if let Some(outcome) = self.resolve_static_relational_outcome_with_context(
            *op,
            left,
            right,
            self.current_function_name(),
        ) {
            return match outcome {
                StaticEvalOutcome::Value(Expression::Bool(value)) => Some(value),
                StaticEvalOutcome::Value(_) | StaticEvalOutcome::Throw(_) => None,
            };
        }

        let (Some(left_number), Some(right_number)) = (
            self.resolve_static_number_value(left),
            self.resolve_static_number_value(right),
        ) else {
            return None;
        };
        Some(match op {
            BinaryOp::LessThan => left_number < right_number,
            BinaryOp::LessThanOrEqual => left_number <= right_number,
            BinaryOp::GreaterThan => left_number > right_number,
            BinaryOp::GreaterThanOrEqual => left_number >= right_number,
            _ => unreachable!("filtered above"),
        })
    }

    fn resolve_static_primitive_or_object_identity(&self, expression: &Expression) -> bool {
        self.resolve_static_primitive_expression_with_context(
            expression,
            self.current_function_name(),
        )
        .is_some()
            || self
                .resolve_static_object_identity_expression(expression)
                .is_some()
    }
}
