use super::*;

impl<'a> FunctionCompiler<'a> {
    fn push_i32_load8_u(&mut self, offset: u32) {
        self.state.emission.output.instructions.push(0x2d);
        self.state.emission.output.instructions.push(0x00);
        push_u32(&mut self.state.emission.output.instructions, offset);
    }

    pub(in crate::backend::direct_wasm) fn emit_runtime_string_literal_memory_comparison(
        &mut self,
        value_local: u32,
        literal: &str,
    ) -> DirectResult<()> {
        let (literal_ptr, literal_len) = self.intern_string(literal.as_bytes().to_vec());
        let result_local = self.allocate_temp_local();
        let index_local = self.allocate_temp_local();

        self.push_i32_const(0);
        self.push_local_set(result_local);

        if let Ok(parsed) = parse_string_to_i32(literal) {
            self.push_local_get(value_local);
            self.push_i32_const(parsed);
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
        }

        if literal_len == 0 {
            self.push_local_get(value_local);
            self.push_i32_const(literal_ptr as i32);
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
            self.state.emission.output.instructions.push(0x0b);
            self.pop_control_frame();
            if parse_string_to_i32(literal).is_ok() {
                self.state.emission.output.instructions.push(0x0b);
                self.pop_control_frame();
            }
            self.push_local_get(result_local);
            return Ok(());
        }

        self.push_local_get(value_local);
        self.push_i32_const(DATA_START_OFFSET as i32);
        self.push_binary_op(BinaryOp::GreaterThanOrEqual)?;
        self.push_local_get(value_local);
        self.state.emission.output.instructions.push(0x3f);
        self.state.emission.output.instructions.push(0x00);
        self.push_i32_const(WASM_MEMORY_PAGE_SIZE as i32);
        self.push_binary_op(BinaryOp::Multiply)?;
        self.push_i32_const(literal_len as i32);
        self.push_binary_op(BinaryOp::Subtract)?;
        self.push_binary_op(BinaryOp::LessThanOrEqual)?;
        self.state.emission.output.instructions.push(0x71);
        self.state.emission.output.instructions.push(0x04);
        self.state
            .emission
            .output
            .instructions
            .push(EMPTY_BLOCK_TYPE);
        self.push_control_frame();
        self.push_i32_const(1);
        self.push_local_set(result_local);
        self.push_i32_const(0);
        self.push_local_set(index_local);
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

        self.push_local_get(index_local);
        self.push_i32_const(literal_len as i32);
        self.push_binary_op(BinaryOp::GreaterThanOrEqual)?;
        self.push_br_if(self.relative_depth(break_target));

        self.push_local_get(value_local);
        self.push_local_get(index_local);
        self.push_binary_op(BinaryOp::Add)?;
        self.push_i32_load8_u(0);
        self.push_i32_const(literal_ptr as i32);
        self.push_local_get(index_local);
        self.push_binary_op(BinaryOp::Add)?;
        self.push_i32_load8_u(0);
        self.push_binary_op(BinaryOp::NotEqual)?;
        self.state.emission.output.instructions.push(0x04);
        self.state
            .emission
            .output
            .instructions
            .push(EMPTY_BLOCK_TYPE);
        self.push_control_frame();
        self.push_i32_const(0);
        self.push_local_set(result_local);
        self.push_br(self.relative_depth(break_target));
        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();

        self.push_local_get(index_local);
        self.push_i32_const(1);
        self.push_binary_op(BinaryOp::Add)?;
        self.push_local_set(index_local);
        self.push_br(self.relative_depth(loop_target));
        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();
        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();
        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();
        if parse_string_to_i32(literal).is_ok() {
            self.state.emission.output.instructions.push(0x0b);
            self.pop_control_frame();
        }
        self.push_local_get(result_local);
        Ok(())
    }

    pub(in crate::backend::direct_wasm) fn emit_hex_quad_string_comparison(
        &mut self,
        left: &Expression,
        right: &Expression,
        op: BinaryOp,
    ) -> DirectResult<bool> {
        let (hex_expression, literal_text) = match (left, right) {
            (expression, Expression::String(text)) => (expression, text.as_str()),
            (Expression::String(text), expression) => (expression, text.as_str()),
            _ => return Ok(false),
        };

        let Some(expected) = parse_fixed_hex_quad(literal_text) else {
            return Ok(false);
        };
        let Some(actual_expression) = self.resolve_hex_quad_numeric_expression(hex_expression)
        else {
            return Ok(false);
        };

        self.emit_numeric_expression(&actual_expression)?;
        self.push_i32_const(expected as i32);
        let comparison = match op {
            BinaryOp::Equal | BinaryOp::LooseEqual => BinaryOp::Equal,
            BinaryOp::NotEqual | BinaryOp::LooseNotEqual => BinaryOp::NotEqual,
            _ => return Ok(false),
        };
        self.push_binary_op(comparison)?;
        Ok(true)
    }
}
