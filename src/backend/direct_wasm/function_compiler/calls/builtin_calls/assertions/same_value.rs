use super::*;

impl<'a> FunctionCompiler<'a> {
    fn emit_same_value_operand(&mut self, expression: &Expression) -> DirectResult<()> {
        self.emit_numeric_expression(expression)
    }

    fn same_value_assertion_needs_runtime_identifier_check(
        &self,
        actual: &Expression,
        expected: &Expression,
    ) -> bool {
        fn is_syntactic_primitive(
            compiler: &FunctionCompiler<'_>,
            expression: &Expression,
        ) -> bool {
            matches!(
                expression,
                Expression::Number(_)
                    | Expression::BigInt(_)
                    | Expression::String(_)
                    | Expression::Bool(_)
                    | Expression::Null
                    | Expression::Undefined
            ) || matches!(
                expression,
                Expression::Identifier(name)
                    if matches!(name.as_str(), "undefined" | "NaN")
                        && compiler.is_unshadowed_builtin_identifier(name)
            )
        }

        matches!(actual, Expression::Identifier(_)) && is_syntactic_primitive(self, expected)
            || matches!(expected, Expression::Identifier(_)) && is_syntactic_primitive(self, actual)
    }

    pub(in crate::backend::direct_wasm) fn emit_same_value_assertion(
        &mut self,
        name: &str,
        arguments: &[CallArgument],
    ) -> DirectResult<bool> {
        let [
            CallArgument::Expression(actual),
            CallArgument::Expression(expected),
            ..,
        ] = arguments
        else {
            return Ok(false);
        };
        let assertion_failure = match name {
            "__assertSameValue" => BinaryOp::NotEqual,
            "__assertNotSameValue" => BinaryOp::Equal,
            _ => return Ok(false),
        };
        let actual_local = self.allocate_temp_local();
        let expected_local = self.allocate_temp_local();
        let actual_reference_identity = self.resolve_static_reference_identity_key(actual);
        let expected_reference_identity = self.resolve_static_reference_identity_key(expected);
        let has_static_reference_identity =
            actual_reference_identity.is_some() && expected_reference_identity.is_some();
        let needs_runtime_identifier_check =
            self.same_value_assertion_needs_runtime_identifier_check(actual, expected);
        let operands_side_effect_free = inline_summary_side_effect_free_expression(actual)
            && inline_summary_side_effect_free_expression(expected);
        if operands_side_effect_free
            && !needs_runtime_identifier_check
            && let (Some(actual_text), Some(expected_text)) = (
                self.resolve_static_string_value(actual),
                self.resolve_static_string_value(expected),
            )
        {
            self.push_i32_const((actual_text == expected_text) as i32);
            self.push_local_set(actual_local);
            if assertion_failure == BinaryOp::NotEqual {
                self.push_local_get(actual_local);
                self.state.emission.output.instructions.push(0x45);
                self.push_local_set(actual_local);
            }
        } else {
            let handled_as_typeof = matches!(
                (actual, expected),
                (
                    Expression::Unary {
                        op: UnaryOp::TypeOf,
                        ..
                    },
                    Expression::String(_)
                ) | (
                    Expression::String(_),
                    Expression::Unary {
                        op: UnaryOp::TypeOf,
                        ..
                    }
                )
            ) || matches!(
                (actual, expected),
                (Expression::String(text), _) | (_, Expression::String(text))
                    if parse_typeof_tag_optional(text).is_some()
            );
            if handled_as_typeof {
                if self.emit_typeof_string_comparison(actual, expected, assertion_failure)?
                    || self.emit_runtime_typeof_tag_string_comparison(
                        actual,
                        expected,
                        assertion_failure,
                    )?
                {
                    self.push_local_set(actual_local);
                } else {
                    self.push_i32_const(0);
                    self.push_local_set(actual_local);
                }
            } else if operands_side_effect_free
                && !needs_runtime_identifier_check
                && (matches!(actual, Expression::String(_))
                    || matches!(expected, Expression::String(_)))
            {
                self.emit_numeric_expression(&Expression::Binary {
                    op: BinaryOp::Equal,
                    left: Box::new(actual.clone()),
                    right: Box::new(expected.clone()),
                })?;
                self.push_local_set(actual_local);
                if assertion_failure == BinaryOp::NotEqual {
                    self.push_local_get(actual_local);
                    self.state.emission.output.instructions.push(0x45);
                    self.push_local_set(actual_local);
                }
            } else if operands_side_effect_free
                && !needs_runtime_identifier_check
                && self.emit_runtime_static_string_equality_comparison(
                    actual,
                    expected,
                    BinaryOp::Equal,
                )?
            {
                self.push_local_set(actual_local);
                if assertion_failure == BinaryOp::NotEqual {
                    self.push_local_get(actual_local);
                    self.state.emission.output.instructions.push(0x45);
                    self.push_local_set(actual_local);
                }
            } else {
                let static_same_value_result = self.resolve_static_same_value_result_with_context(
                    actual,
                    expected,
                    self.current_function_name(),
                );
                if !needs_runtime_identifier_check
                    && (!self.assertion_requires_runtime_same_value_fallback()
                        || has_static_reference_identity)
                    && operands_side_effect_free
                    && (matches!(actual, Expression::This)
                        || matches!(expected, Expression::This)
                        || self.resolve_array_binding_from_expression(actual).is_some()
                        || self
                            .resolve_array_binding_from_expression(expected)
                            .is_some()
                        || self
                            .resolve_object_binding_from_expression(actual)
                            .is_some()
                        || self
                            .resolve_object_binding_from_expression(expected)
                            .is_some()
                        || self.resolve_user_function_from_expression(actual).is_some()
                        || self
                            .resolve_user_function_from_expression(expected)
                            .is_some()
                        || has_static_reference_identity
                        || (!matches!(actual, Expression::Identifier(_))
                            && !matches!(expected, Expression::Identifier(_))))
                    && let Some(result) = static_same_value_result
                {
                    self.push_i32_const(result as i32);
                    self.push_local_set(actual_local);
                    if assertion_failure == BinaryOp::NotEqual {
                        self.push_local_get(actual_local);
                        self.state.emission.output.instructions.push(0x45);
                        self.push_local_set(actual_local);
                    }
                } else {
                    self.emit_same_value_operand(actual)?;
                    self.push_local_set(actual_local);
                    self.emit_same_value_operand(expected)?;
                    self.push_local_set(expected_local);
                    self.emit_same_value_result_from_locals(
                        actual_local,
                        expected_local,
                        actual_local,
                    )?;
                    if assertion_failure == BinaryOp::NotEqual {
                        self.push_local_get(actual_local);
                        self.state.emission.output.instructions.push(0x45);
                        self.push_local_set(actual_local);
                    }
                }
            }
        }
        for argument in arguments.iter().skip(2) {
            match argument {
                CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                    self.emit_numeric_expression(expression)?;
                    self.state.emission.output.instructions.push(0x1a);
                }
            }
        }
        self.push_local_get(actual_local);
        self.state.emission.output.instructions.push(0x04);
        self.state
            .emission
            .output
            .instructions
            .push(EMPTY_BLOCK_TYPE);
        self.push_control_frame();
        if std::env::var_os("AYY_TRACE_ASSERTIONS").is_some() {
            self.emit_print(&[
                Expression::String(format!(
                    "same_value_assertion_fail name={name} actual={actual:?} expected={expected:?} fn={:?}",
                    self.current_function_name()
                )),
                actual.clone(),
                expected.clone(),
            ])?;
        }
        self.emit_error_throw()?;
        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();
        self.push_i32_const(JS_UNDEFINED_TAG);
        Ok(true)
    }
}
