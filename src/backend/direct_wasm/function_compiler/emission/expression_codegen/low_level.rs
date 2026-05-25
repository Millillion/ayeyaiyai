use super::*;

impl<'a> FunctionCompiler<'a> {
    pub(in crate::backend::direct_wasm) fn emit_truthy_expression(
        &mut self,
        expression: &Expression,
    ) -> DirectResult<()> {
        match expression {
            Expression::Bool(value) => {
                self.push_i32_const(if *value { 1 } else { 0 });
                return Ok(());
            }
            Expression::Null | Expression::Undefined => {
                self.push_i32_const(0);
                return Ok(());
            }
            Expression::Number(value) => {
                self.push_i32_const(if *value != 0.0 && !value.is_nan() {
                    1
                } else {
                    0
                });
                return Ok(());
            }
            Expression::String(text) => {
                self.push_i32_const(if text.is_empty() { 0 } else { 1 });
                return Ok(());
            }
            _ => {}
        }
        if inline_summary_side_effect_free_expression(expression)
            && !Self::expression_contains_assignment_or_update(expression)
            && !Self::expression_references_internal_assignment_temp(expression)
            && let Some(value) = self.resolve_static_boolean_expression(expression)
        {
            self.push_i32_const(value as i32);
            return Ok(());
        }
        let value_local = self.allocate_temp_local();
        self.emit_numeric_expression(expression)?;
        self.push_local_set(value_local);

        self.push_local_get(value_local);
        self.push_i32_const(0);
        self.push_binary_op(BinaryOp::NotEqual)?;

        self.push_local_get(value_local);
        self.push_i32_const(JS_NULL_TAG);
        self.push_binary_op(BinaryOp::NotEqual)?;
        self.state.emission.output.instructions.push(0x71);

        self.push_local_get(value_local);
        self.push_i32_const(JS_UNDEFINED_TAG);
        self.push_binary_op(BinaryOp::NotEqual)?;
        self.state.emission.output.instructions.push(0x71);

        self.push_local_get(value_local);
        self.push_i32_const(JS_NAN_TAG);
        self.push_binary_op(BinaryOp::NotEqual)?;
        self.state.emission.output.instructions.push(0x71);
        Ok(())
    }

    pub(in crate::backend::direct_wasm) fn emit_loose_number(
        &mut self,
        expression: &Expression,
    ) -> DirectResult<()> {
        if let Some(primitive) = self.resolve_static_boxed_primitive_value(expression) {
            return self.emit_loose_number(&primitive);
        }
        if let Some(StaticEvalOutcome::Value(primitive)) = self
            .resolve_static_to_primitive_outcome_with_context(
                expression,
                PrimitiveHint::Default,
                self.current_function_name(),
            )
            && !static_expression_matches(&primitive, expression)
        {
            if !inline_summary_side_effect_free_expression(expression) {
                self.emit_numeric_expression(expression)?;
                self.state.emission.output.instructions.push(0x1a);
            }
            return self.emit_loose_number(&primitive);
        }
        match expression {
            Expression::Null => {
                self.push_i32_const(0);
                Ok(())
            }
            Expression::Undefined => {
                self.push_i32_const(0);
                Ok(())
            }
            Expression::String(text) => {
                match parse_string_to_loose_i32(text) {
                    Ok(parsed) => self.push_i32_const(parsed),
                    Err(Unsupported("string literal collides with reserved JS tag")) => {
                        return Err(Unsupported("string literal collides with reserved JS tag"));
                    }
                    Err(_) => {
                        self.emit_static_string_literal(text)?;
                    }
                }
                Ok(())
            }
            _ => self.emit_numeric_expression(expression),
        }
    }

    pub(in crate::backend::direct_wasm) fn find_labeled_loop_index(
        &self,
        label: &str,
    ) -> DirectResult<Option<usize>> {
        Ok(self
            .state
            .emission
            .control_flow
            .loop_stack
            .iter()
            .rposition(|loop_context| loop_context.labels.iter().any(|name| name == label)))
    }

    pub(in crate::backend::direct_wasm) fn break_hook_for_target(
        &self,
        break_target: usize,
    ) -> DirectResult<Option<Expression>> {
        for break_context in self.state.emission.control_flow.break_stack.iter().rev() {
            if break_context.break_target == break_target {
                return Ok(break_context.break_hook.clone());
            }
        }
        Ok(None)
    }

    pub(in crate::backend::direct_wasm) fn find_labeled_break(
        &self,
        label: &str,
    ) -> DirectResult<Option<usize>> {
        Ok(self
            .state
            .emission
            .control_flow
            .break_stack
            .iter()
            .rposition(|break_context| break_context.labels.iter().any(|name| name == label)))
    }

    pub(in crate::backend::direct_wasm) fn allocate_temp_local(&mut self) -> u32 {
        let local_index = self.state.runtime.locals.next_local_index;
        self.state.runtime.locals.next_local_index += 1;
        local_index
    }

    pub(in crate::backend::direct_wasm) fn push_control_frame(&mut self) -> usize {
        self.state.emission.control_flow.control_stack.push(());
        self.state.emission.control_flow.control_stack.len() - 1
    }

    pub(in crate::backend::direct_wasm) fn pop_control_frame(&mut self) {
        self.state.emission.control_flow.control_stack.pop();
    }

    pub(in crate::backend::direct_wasm) fn relative_depth(&self, target: usize) -> u32 {
        (self.state.emission.control_flow.control_stack.len() - 1 - target) as u32
    }

    pub(in crate::backend::direct_wasm) fn push_i32_const(&mut self, value: i32) {
        self.state.emission.output.instructions.push(0x41);
        push_i32(&mut self.state.emission.output.instructions, value);
    }

    pub(in crate::backend::direct_wasm) fn push_local_get(&mut self, local_index: u32) {
        self.state.emission.output.instructions.push(0x20);
        push_u32(&mut self.state.emission.output.instructions, local_index);
    }

    pub(in crate::backend::direct_wasm) fn push_local_set(&mut self, local_index: u32) {
        self.state.emission.output.instructions.push(0x21);
        push_u32(&mut self.state.emission.output.instructions, local_index);
    }

    pub(in crate::backend::direct_wasm) fn push_global_get(&mut self, global_index: u32) {
        self.state.emission.output.instructions.push(0x23);
        push_u32(&mut self.state.emission.output.instructions, global_index);
    }

    #[track_caller]
    pub(in crate::backend::direct_wasm) fn push_global_set(&mut self, global_index: u32) {
        if let Some(targets) = std::env::var_os("AYY_TRACE_GLOBAL_SET")
            && targets
                .to_string_lossy()
                .split(',')
                .filter_map(|target| target.trim().parse::<u32>().ok())
                .any(|target| target == global_index)
        {
            let caller = std::panic::Location::caller();
            eprintln!(
                "global_set_trace fn={:?} global={global_index} instruction={} caller={}:{}",
                self.current_function_name(),
                self.state.emission.output.instructions.len(),
                caller.file(),
                caller.line(),
            );
        }
        self.state.emission.output.instructions.push(0x24);
        push_u32(&mut self.state.emission.output.instructions, global_index);
    }

    pub(in crate::backend::direct_wasm) fn push_local_tee(&mut self, local_index: u32) {
        self.state.emission.output.instructions.push(0x22);
        push_u32(&mut self.state.emission.output.instructions, local_index);
    }

    pub(in crate::backend::direct_wasm) fn push_call(&mut self, function_index: u32) {
        self.state.emission.output.instructions.push(0x10);
        push_u32(&mut self.state.emission.output.instructions, function_index);
    }

    pub(in crate::backend::direct_wasm) fn push_user_function_call(
        &mut self,
        user_function: &UserFunction,
    ) {
        self.backend
            .function_registry
            .record_runtime_called_user_function(&user_function.name);
        self.push_call(user_function.function_index);
    }

    pub(in crate::backend::direct_wasm) fn push_br(&mut self, relative_depth: u32) {
        self.state.emission.output.instructions.push(0x0c);
        push_u32(&mut self.state.emission.output.instructions, relative_depth);
    }

    pub(in crate::backend::direct_wasm) fn push_br_if(&mut self, relative_depth: u32) {
        self.state.emission.output.instructions.push(0x0d);
        push_u32(&mut self.state.emission.output.instructions, relative_depth);
    }
}
