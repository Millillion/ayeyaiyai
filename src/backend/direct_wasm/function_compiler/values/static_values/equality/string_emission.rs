use super::*;

fn static_single_utf16_code_unit(text: &str) -> Option<u16> {
    let mut units = text.encode_utf16();
    let unit = units.next()?;
    if units.next().is_some() {
        return None;
    }
    Some(unit)
}

fn js_to_uint16(value: f64) -> u16 {
    if !value.is_finite() || value == 0.0 {
        return 0;
    }
    value.trunc().rem_euclid(65_536.0) as u16
}

impl<'a> FunctionCompiler<'a> {
    fn string_from_char_code_argument<'b>(expression: &'b Expression) -> Option<&'b Expression> {
        let Expression::Call { callee, arguments } = expression else {
            return None;
        };
        let Expression::Member { object, property } = callee.as_ref() else {
            return None;
        };
        if !matches!(object.as_ref(), Expression::Identifier(name) if name == "String") {
            return None;
        }
        if !matches!(property.as_ref(), Expression::String(name) if name == "fromCharCode") {
            return None;
        }
        let [CallArgument::Expression(argument)] = arguments.as_slice() else {
            return None;
        };
        Some(argument)
    }

    fn resolve_static_char_code_unit_argument(&self, argument: &Expression) -> Option<u16> {
        if let Some(Expression::Number(value)) = self.resolve_char_code_argument(argument) {
            return Some(value as u16);
        }
        self.resolve_static_number_value(argument).map(js_to_uint16)
    }

    fn resolve_string_from_char_code_table_equality(
        &self,
        code_argument: &Expression,
        string_expression: &Expression,
    ) -> Option<bool> {
        let Expression::Member {
            object: code_object,
            property: code_property,
        } = code_argument
        else {
            return None;
        };
        let Expression::Member {
            object: string_object,
            property: string_property,
        } = string_expression
        else {
            return None;
        };

        let code_property = self.materialize_static_expression(code_property);
        let string_property = self.materialize_static_expression(string_property);
        if !static_expression_matches(&code_property, &string_property) {
            return None;
        }

        let code_array = self.resolve_array_binding_from_expression(code_object)?;
        let string_array = self.resolve_array_binding_from_expression(string_object)?;
        if code_array.values.is_empty() || code_array.values.len() != string_array.values.len() {
            return None;
        }

        let mut all_equal = true;
        for (code_value, string_value) in code_array.values.iter().zip(&string_array.values) {
            let code_value = code_value.as_ref()?;
            let string_value = string_value.as_ref()?;
            let code_unit = self.resolve_static_char_code_unit_argument(code_value)?;
            let string_unit =
                static_single_utf16_code_unit(&self.resolve_static_string_value(string_value)?)?;
            all_equal &= code_unit == string_unit;
        }
        Some(all_equal)
    }

    pub(in crate::backend::direct_wasm) fn emit_string_from_char_code_equality_comparison(
        &mut self,
        left: &Expression,
        right: &Expression,
        op: BinaryOp,
    ) -> DirectResult<bool> {
        let Some((code_argument, other)) =
            Self::string_from_char_code_argument(left).map(|argument| (argument, right)).or_else(
                || Self::string_from_char_code_argument(right).map(|argument| (argument, left)),
            )
        else {
            return Ok(false);
        };
        if !matches!(
            op,
            BinaryOp::Equal | BinaryOp::NotEqual | BinaryOp::LooseEqual | BinaryOp::LooseNotEqual
        ) {
            return Ok(false);
        }

        if let Some(equal) = self
            .resolve_static_string_value(other)
            .and_then(|text| static_single_utf16_code_unit(&text))
            .and_then(|expected| {
                self.resolve_static_char_code_unit_argument(code_argument)
                    .map(|actual| actual == expected)
            })
            .or_else(|| self.resolve_string_from_char_code_table_equality(code_argument, other))
        {
            let result = match op {
                BinaryOp::Equal | BinaryOp::LooseEqual => equal,
                BinaryOp::NotEqual | BinaryOp::LooseNotEqual => !equal,
                _ => unreachable!("equality operator checked above"),
            };
            self.push_i32_const(if result { 1 } else { 0 });
            return Ok(true);
        }

        let Some(expected) = self
            .resolve_static_string_value(other)
            .and_then(|text| static_single_utf16_code_unit(&text))
        else {
            return Ok(false);
        };
        if self.infer_value_kind(code_argument) != Some(StaticValueKind::Number) {
            return Ok(false);
        }

        self.emit_numeric_expression(code_argument)?;
        self.push_i32_const(expected as i32);
        let comparison = match op {
            BinaryOp::Equal | BinaryOp::LooseEqual => BinaryOp::Equal,
            BinaryOp::NotEqual | BinaryOp::LooseNotEqual => BinaryOp::NotEqual,
            _ => unreachable!("equality operator checked above"),
        };
        self.push_binary_op(comparison)?;
        Ok(true)
    }

    pub(in crate::backend::direct_wasm) fn emit_same_value_result_from_locals(
        &mut self,
        actual_local: u32,
        expected_local: u32,
        result_local: u32,
    ) -> DirectResult<()> {
        self.push_local_get(actual_local);
        self.push_local_get(expected_local);
        self.push_binary_op(BinaryOp::Equal)?;
        self.state.emission.output.instructions.push(0x04);
        self.state
            .emission
            .output
            .instructions
            .push(EMPTY_BLOCK_TYPE);
        self.push_control_frame();
        self.push_i32_const(1);
        self.push_local_set(result_local);
        self.state.emission.output.instructions.push(0x05);
        self.push_local_get(actual_local);
        self.push_i32_const(JS_NAN_TAG);
        self.push_binary_op(BinaryOp::Equal)?;
        self.push_local_get(expected_local);
        self.push_i32_const(JS_NAN_TAG);
        self.push_binary_op(BinaryOp::Equal)?;
        self.state.emission.output.instructions.push(0x71);
        self.push_local_set(result_local);
        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();
        Ok(())
    }

    pub(in crate::backend::direct_wasm) fn emit_static_string_equality_comparison(
        &mut self,
        left: &Expression,
        right: &Expression,
        op: BinaryOp,
    ) -> DirectResult<bool> {
        if !matches!(left, Expression::String(_)) && !matches!(right, Expression::String(_)) {
            return Ok(false);
        }
        let Some(left_text) = self.resolve_static_string_value(left) else {
            return Ok(false);
        };
        let Some(right_text) = self.resolve_static_string_value(right) else {
            return Ok(false);
        };
        let equal = left_text == right_text;
        let result = match op {
            BinaryOp::Equal | BinaryOp::LooseEqual => equal,
            BinaryOp::NotEqual | BinaryOp::LooseNotEqual => !equal,
            _ => return Ok(false),
        };
        self.push_i32_const(if result { 1 } else { 0 });
        Ok(true)
    }

    pub(in crate::backend::direct_wasm) fn emit_runtime_static_string_equality_comparison(
        &mut self,
        left: &Expression,
        right: &Expression,
        op: BinaryOp,
    ) -> DirectResult<bool> {
        let (dynamic, literal) = match (left, right) {
            (dynamic, Expression::String(text))
                if self.infer_value_kind(dynamic) == Some(StaticValueKind::String) =>
            {
                (dynamic, text)
            }
            (Expression::String(text), dynamic)
                if self.infer_value_kind(dynamic) == Some(StaticValueKind::String) =>
            {
                (dynamic, text)
            }
            _ => return Ok(false),
        };
        if !matches!(
            op,
            BinaryOp::Equal | BinaryOp::NotEqual | BinaryOp::LooseEqual | BinaryOp::LooseNotEqual
        ) {
            return Ok(false);
        }

        let dynamic_local = self.allocate_temp_local();
        self.emit_numeric_expression(dynamic)?;
        self.push_local_set(dynamic_local);
        self.emit_runtime_string_literal_memory_comparison(dynamic_local, literal)?;
        if matches!(op, BinaryOp::NotEqual | BinaryOp::LooseNotEqual) {
            self.state.emission.output.instructions.push(0x45);
        }
        Ok(true)
    }
}
