use super::*;

thread_local! {
    static RUNTIME_STRING_PRINT_CANDIDATE_STACK: std::cell::RefCell<Vec<Expression>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

impl<'a> FunctionCompiler<'a> {
    fn add_runtime_string_print_candidate(
        &mut self,
        candidates: &mut Vec<(i32, String)>,
        text: &str,
    ) {
        let (ptr, _) = self.intern_string(text.as_bytes().to_vec());
        if !candidates
            .iter()
            .any(|(candidate_ptr, _)| *candidate_ptr == ptr as i32)
        {
            candidates.push((ptr as i32, text.to_string()));
        }
    }

    fn collect_runtime_ambient_string_print_candidates(
        &mut self,
        candidates: &mut Vec<(i32, String)>,
    ) {
        for name in NATIVE_ERROR_NAMES {
            if native_error_runtime_value(name).is_some() {
                self.add_runtime_string_print_candidate(candidates, name);
            }
        }

        let mut user_function_names = Vec::new();
        for user_function in self.user_functions() {
            if let Some(Expression::String(text)) =
                self.runtime_user_function_property_value(&user_function, "name")
            {
                user_function_names.push(text);
            }
        }
        for text in user_function_names {
            self.add_runtime_string_print_candidate(candidates, &text);
        }
    }

    pub(in crate::backend::direct_wasm) fn runtime_string_print_candidates(
        &mut self,
        value: &Expression,
    ) -> Vec<(i32, String)> {
        let mut candidates = Vec::new();
        self.collect_runtime_string_print_candidates(value, &mut candidates);
        candidates
    }

    fn collect_runtime_string_print_candidates(
        &mut self,
        value: &Expression,
        candidates: &mut Vec<(i32, String)>,
    ) {
        let reentered = RUNTIME_STRING_PRINT_CANDIDATE_STACK.with(|stack| {
            stack
                .borrow()
                .iter()
                .any(|visited| static_expression_matches(visited, value))
        });
        if reentered {
            return;
        }

        RUNTIME_STRING_PRINT_CANDIDATE_STACK.with(|stack| {
            stack.borrow_mut().push(value.clone());
        });
        self.collect_runtime_string_print_candidates_inner(value, candidates);
        RUNTIME_STRING_PRINT_CANDIDATE_STACK.with(|stack| {
            stack.borrow_mut().pop();
        });
    }

    fn collect_runtime_string_print_candidates_inner(
        &mut self,
        value: &Expression,
        candidates: &mut Vec<(i32, String)>,
    ) {
        let materialized_value = self.materialize_static_expression(value);
        if !static_expression_matches(&materialized_value, value) {
            self.collect_runtime_string_print_candidates(&materialized_value, candidates);
            if !candidates.is_empty() {
                return;
            }
        }

        if let Some(resolved_value) = self
            .resolve_bound_alias_expression(value)
            .filter(|resolved| !static_expression_matches(resolved, value))
        {
            self.collect_runtime_string_print_candidates(&resolved_value, candidates);
            if !candidates.is_empty() {
                return;
            }
        }

        if let Some(global_value) = self
            .resolve_global_value_expression(value)
            .filter(|resolved| !static_expression_matches(resolved, value))
        {
            self.collect_runtime_string_print_candidates(&global_value, candidates);
            if !candidates.is_empty() {
                return;
            }
        }

        let Expression::Member { object, property } = value else {
            if self.infer_value_kind(value) == Some(StaticValueKind::String) {
                self.collect_runtime_ambient_string_print_candidates(candidates);
            }
            return;
        };
        if let Some(object_binding) = self.resolve_object_binding_from_expression(object) {
            let values = object_binding
                .string_properties
                .iter()
                .filter_map(|(_, value)| self.resolve_static_string_value(value))
                .collect::<Vec<_>>();
            for text in values {
                self.add_runtime_string_print_candidate(candidates, &text);
            }
            if !candidates.is_empty() {
                return;
            }
        }
        let Expression::String(property_name) = self.materialize_static_expression(property) else {
            if self.infer_value_kind(value) == Some(StaticValueKind::String) {
                self.collect_runtime_ambient_string_print_candidates(candidates);
            }
            return;
        };

        if property_name == "name" {
            for name in NATIVE_ERROR_NAMES {
                let Some(_) = native_error_runtime_value(name) else {
                    continue;
                };
                self.add_runtime_string_print_candidate(candidates, name);
            }
        }

        let mut user_function_values = Vec::new();
        for user_function in self.user_functions() {
            let Some(Expression::String(text)) =
                self.runtime_user_function_property_value(&user_function, &property_name)
            else {
                continue;
            };
            user_function_values.push(text);
        }
        for text in user_function_values {
            self.add_runtime_string_print_candidate(candidates, &text);
        }

        if candidates.is_empty() && self.infer_value_kind(value) == Some(StaticValueKind::String) {
            self.collect_runtime_ambient_string_print_candidates(candidates);
        }
    }

    pub(in crate::backend::direct_wasm) fn emit_runtime_print_known_string_value(
        &mut self,
        value: &Expression,
    ) -> DirectResult<bool> {
        let candidates = self.runtime_string_print_candidates(value);
        let should_handle =
            self.infer_value_kind(value) == Some(StaticValueKind::String) || !candidates.is_empty();
        if !should_handle {
            return Ok(false);
        }

        let string_data = self.backend.module_artifacts.string_data.clone();
        let value_local = self.allocate_temp_local();
        let handled_local = self.allocate_temp_local();
        self.emit_numeric_expression(value)?;
        self.push_local_set(value_local);
        self.push_i32_const(0);
        self.push_local_set(handled_local);

        for (string_pointer, bytes) in string_data {
            self.push_local_get(value_local);
            self.push_i32_const(string_pointer as i32);
            self.push_binary_op(BinaryOp::Equal)?;
            self.state.emission.output.instructions.push(0x04);
            self.state
                .emission
                .output
                .instructions
                .push(EMPTY_BLOCK_TYPE);
            self.push_control_frame();
            self.push_i32_const(string_pointer as i32);
            self.push_i32_const(bytes.len() as i32);
            self.push_call(WRITE_BYTES_FUNCTION_INDEX);
            self.push_i32_const(1);
            self.push_local_set(handled_local);
            self.state.emission.output.instructions.push(0x0b);
            self.pop_control_frame();
        }

        self.push_local_get(handled_local);
        self.state.emission.output.instructions.push(0x45);
        self.state.emission.output.instructions.push(0x04);
        self.state
            .emission
            .output
            .instructions
            .push(EMPTY_BLOCK_TYPE);
        self.push_control_frame();
        self.emit_runtime_print_numeric_value(value)?;
        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();
        Ok(true)
    }

    pub(in crate::backend::direct_wasm) fn emit_runtime_print_numeric_value(
        &mut self,
        value: &Expression,
    ) -> DirectResult<()> {
        let value_local = self.allocate_temp_local();
        let handled_local = self.allocate_temp_local();
        self.emit_numeric_expression(value)?;
        self.push_local_set(value_local);
        self.push_i32_const(0);
        self.push_local_set(handled_local);

        for (tag, text) in [(JS_NULL_TAG, "null"), (JS_UNDEFINED_TAG, "undefined")] {
            self.push_local_get(value_local);
            self.push_i32_const(tag);
            self.push_binary_op(BinaryOp::Equal)?;
            self.state.emission.output.instructions.push(0x04);
            self.state
                .emission
                .output
                .instructions
                .push(EMPTY_BLOCK_TYPE);
            self.push_control_frame();
            self.emit_print_string(text)?;
            self.push_i32_const(1);
            self.push_local_set(handled_local);
            self.state.emission.output.instructions.push(0x0b);
            self.pop_control_frame();
        }

        self.push_local_get(handled_local);
        self.state.emission.output.instructions.push(0x45);
        self.state.emission.output.instructions.push(0x04);
        self.state
            .emission
            .output
            .instructions
            .push(EMPTY_BLOCK_TYPE);
        self.push_control_frame();
        self.push_local_get(value_local);
        self.push_i32_const(JS_TYPEOF_NUMBER_TAG);
        self.push_binary_op(BinaryOp::GreaterThanOrEqual)?;
        self.push_local_get(value_local);
        self.push_i32_const(JS_TYPEOF_BIGINT_TAG);
        self.push_binary_op(BinaryOp::LessThanOrEqual)?;
        self.state.emission.output.instructions.push(0x71);
        self.state.emission.output.instructions.push(0x04);
        self.state
            .emission
            .output
            .instructions
            .push(EMPTY_BLOCK_TYPE);
        self.push_control_frame();
        self.emit_typeof_print_from_local(value_local)?;
        self.state.emission.output.instructions.push(0x05);
        self.push_local_get(value_local);
        self.push_i32_const(JS_NAN_TAG);
        self.push_binary_op(BinaryOp::Equal)?;
        self.state.emission.output.instructions.push(0x04);
        self.state
            .emission
            .output
            .instructions
            .push(EMPTY_BLOCK_TYPE);
        self.push_control_frame();
        self.emit_print_string("NaN")?;
        self.state.emission.output.instructions.push(0x05);
        self.push_local_get(value_local);
        self.push_call(PRINT_I32_FUNCTION_INDEX);
        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();
        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();
        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();
        Ok(())
    }
}
