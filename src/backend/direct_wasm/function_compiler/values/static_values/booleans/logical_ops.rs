use super::super::*;

impl<'a> FunctionCompiler<'a> {
    pub(in crate::backend::direct_wasm) fn resolve_static_if_condition_value(
        &self,
        expression: &Expression,
    ) -> Option<bool> {
        if Self::expression_contains_assignment_or_update(expression) {
            return None;
        }
        if let Expression::Binary { op, left, right } = expression {
            let compare = |lhs: bool, rhs: bool| match op {
                BinaryOp::Equal | BinaryOp::LooseEqual => Some(lhs == rhs),
                BinaryOp::NotEqual | BinaryOp::LooseNotEqual => Some(lhs != rhs),
                _ => None,
            };
            if let Some(result) =
                self.resolve_static_if_primitive_equality_condition(*op, left, right)
            {
                return Some(result);
            }
            if let Some(result) = self.resolve_static_binary_boolean_result(op, left, right) {
                return Some(result);
            }
            if let Some(result) = self.resolve_static_property_key_condition(*op, left, right) {
                return Some(result);
            }
            if let Some(lhs) = self.resolve_static_is_nan_call_result(left)
                && let Expression::Bool(rhs) = self.materialize_static_expression(right)
            {
                return compare(lhs, rhs);
            }
            if let Some(rhs) = self.resolve_static_is_nan_call_result(right)
                && let Expression::Bool(lhs) = self.materialize_static_expression(left)
            {
                return compare(lhs, rhs);
            }
        }
        self.resolve_static_boolean_expression(expression)
    }

    fn resolve_static_if_primitive_equality_condition(
        &self,
        op: BinaryOp,
        left: &Expression,
        right: &Expression,
    ) -> Option<bool> {
        if !matches!(op, BinaryOp::Equal | BinaryOp::NotEqual) {
            return None;
        }

        let (left, left_from_array_member) =
            self.resolve_static_if_primitive_condition_operand(left)?;
        let (right, right_from_array_member) =
            self.resolve_static_if_primitive_condition_operand(right)?;
        if !left_from_array_member && !right_from_array_member {
            return None;
        }
        let equal = match (&left, &right) {
            (Expression::Bool(left), Expression::Bool(right)) => Some(left == right),
            (Expression::Number(left), Expression::Number(right)) => Some(left == right),
            (Expression::String(left), Expression::String(right)) => Some(left == right),
            (Expression::BigInt(left), Expression::BigInt(right)) => {
                Some(parse_static_bigint_literal(left)? == parse_static_bigint_literal(right)?)
            }
            (Expression::Null, Expression::Null)
            | (Expression::Undefined, Expression::Undefined) => Some(true),
            (
                Expression::Number(_)
                | Expression::BigInt(_)
                | Expression::String(_)
                | Expression::Bool(_)
                | Expression::Null
                | Expression::Undefined,
                Expression::Number(_)
                | Expression::BigInt(_)
                | Expression::String(_)
                | Expression::Bool(_)
                | Expression::Null
                | Expression::Undefined,
            ) => Some(false),
            _ => None,
        }?;
        Some(equal ^ matches!(op, BinaryOp::NotEqual))
    }

    fn resolve_static_if_primitive_condition_operand(
        &self,
        expression: &Expression,
    ) -> Option<(Expression, bool)> {
        if let Some(value) = self.resolve_static_if_array_member_condition_operand(expression) {
            return Some((value, true));
        }
        self.resolve_static_primitive_expression_with_context(
            expression,
            self.current_function_name(),
        )
        .map(|value| (value, false))
    }

    fn resolve_static_if_array_member_condition_operand(
        &self,
        expression: &Expression,
    ) -> Option<Expression> {
        let Expression::Member { object, property } = expression else {
            return None;
        };
        if !matches!(object.as_ref(), Expression::Identifier(_)) {
            return None;
        }

        let property = self
            .resolve_property_key_expression(property)
            .unwrap_or_else(|| self.materialize_static_expression(property));
        let array_binding = self.resolve_array_binding_from_expression(object)?;
        let value = if matches!(&property, Expression::String(name) if name == "length") {
            Expression::Number(array_binding.values.len() as f64)
        } else {
            let index = argument_index_from_expression(&property)? as usize;
            array_binding
                .values
                .get(index)
                .cloned()
                .flatten()
                .unwrap_or(Expression::Undefined)
        };
        self.resolve_static_primitive_expression_with_context(&value, self.current_function_name())
    }

    fn resolve_static_property_key_condition(
        &self,
        op: BinaryOp,
        left: &Expression,
        right: &Expression,
    ) -> Option<bool> {
        if !matches!(
            op,
            BinaryOp::Equal | BinaryOp::LooseEqual | BinaryOp::NotEqual | BinaryOp::LooseNotEqual
        ) {
            return None;
        }

        let left_name = self.static_condition_property_name(left)?;
        let right_name = self.static_condition_property_name(right)?;
        if crate::ayy_env_flag!("AYY_TRACE_STATIC_IF") {
            eprintln!(
                "static_if:property_key_condition left={left:?} right={right:?} left_name={left_name} right_name={right_name}"
            );
        }
        let not_equal = matches!(op, BinaryOp::NotEqual | BinaryOp::LooseNotEqual);
        Some((left_name == right_name) ^ not_equal)
    }

    fn static_condition_property_name(&self, expression: &Expression) -> Option<String> {
        // Function-valued operands compare by identity, not by their
        // stringified source text; two distinct functions with identical
        // source (for example a with-scope `parseInt` shadow versus the
        // builtin `parseInt`) must not fold equal through their coerced
        // property-key strings.
        if self
            .resolve_function_binding_from_expression(expression)
            .is_some()
        {
            return None;
        }
        let canonical = self.canonical_object_property_expression(expression);
        if crate::ayy_env_flag!("AYY_TRACE_LIVE_MUTABLE") {
            eprintln!("condition_property_name expr={expression:?} canonical={canonical:?}");
        }
        static_property_name_from_expression(&canonical)
            .or_else(|| static_property_name_from_expression(expression))
    }

    pub(in crate::backend::direct_wasm) fn resolve_static_logical_result_expression(
        &self,
        op: BinaryOp,
        left: &Expression,
        right: &Expression,
    ) -> Option<Expression> {
        if let Expression::Identifier(name) = left
            && self
                .resolve_bound_alias_expression(left)
                .filter(|resolved| !static_expression_matches(resolved, left))
                .is_none()
            && !(name == "undefined" && self.is_unshadowed_builtin_identifier(name))
            && !(name == "NaN" && self.is_unshadowed_builtin_identifier(name))
            && !matches!(
                self.lookup_identifier_kind(name),
                Some(
                    StaticValueKind::Object
                        | StaticValueKind::Function
                        | StaticValueKind::Symbol
                        | StaticValueKind::Null
                        | StaticValueKind::Undefined
                )
            )
        {
            return None;
        }
        match op {
            BinaryOp::LogicalAnd => {
                let left_truthy = self.resolve_static_boolean_expression(left)?;
                if left_truthy {
                    Some(self.materialize_static_expression(right))
                } else {
                    Some(self.materialize_static_expression(left))
                }
            }
            BinaryOp::LogicalOr => {
                let left_truthy = self.resolve_static_boolean_expression(left)?;
                if left_truthy {
                    Some(self.materialize_static_expression(left))
                } else {
                    Some(self.materialize_static_expression(right))
                }
            }
            BinaryOp::NullishCoalescing => {
                let materialized_left = self.materialize_static_expression(left);
                if let Some(primitive_left) = self.resolve_static_primitive_expression_with_context(
                    &materialized_left,
                    self.current_function_name(),
                ) {
                    return if matches!(primitive_left, Expression::Null | Expression::Undefined) {
                        Some(self.materialize_static_expression(right))
                    } else {
                        Some(primitive_left)
                    };
                }
                matches!(
                    self.infer_value_kind(&materialized_left),
                    Some(kind) if kind != StaticValueKind::Unknown
                )
                .then_some(materialized_left)
            }
            _ => None,
        }
    }
}
