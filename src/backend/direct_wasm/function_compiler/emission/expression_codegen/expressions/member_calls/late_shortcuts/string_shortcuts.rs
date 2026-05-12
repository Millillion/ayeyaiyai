use super::*;

impl<'a> FunctionCompiler<'a> {
    pub(super) fn emit_string_member_call_shortcuts(
        &mut self,
        object: &Expression,
        property: &Expression,
        arguments: &[CallArgument],
    ) -> DirectResult<bool> {
        if matches!(property, Expression::String(property_name) if property_name == "toString")
            && arguments.is_empty()
            && (self.infer_value_kind(object) == Some(StaticValueKind::String)
                || self.resolve_static_string_value(object).is_some()
                || !self.runtime_string_print_candidates(object).is_empty())
        {
            self.emit_numeric_expression(object)?;
            return Ok(true);
        }
        if matches!(property, Expression::String(property_name) if property_name == "toString")
            && arguments.is_empty()
        {
            let object_local = self.allocate_temp_local();
            self.emit_numeric_expression(object)?;
            self.push_local_set(object_local);
            self.emit_throw_if_member_base_nullish_local(object_local)?;
            self.push_local_get(object_local);
            return Ok(true);
        }
        if matches!(property, Expression::String(property_name) if property_name == "indexOf")
            && let Expression::String(text) = object
            && let [CallArgument::Expression(Expression::String(search))] = arguments
        {
            self.emit_numeric_expression(object)?;
            self.state.emission.output.instructions.push(0x1a);
            self.emit_numeric_expression(&Expression::String(search.clone()))?;
            self.state.emission.output.instructions.push(0x1a);
            self.push_i32_const(text.find(search).map(|index| index as i32).unwrap_or(-1));
            return Ok(true);
        }
        if matches!(property, Expression::String(property_name) if property_name == "indexOf")
            && inline_summary_side_effect_free_expression(object)
            && (self.infer_value_kind(object) == Some(StaticValueKind::String)
                || self.resolve_static_string_value(object).is_some()
                || !self.runtime_string_print_candidates(object).is_empty())
            && let [CallArgument::Expression(search_expression)] = arguments
            && let Some(search) = self.resolve_static_string_value(search_expression)
        {
            let value_local = self.allocate_temp_local();
            let result_local = self.allocate_temp_local();
            self.emit_numeric_expression(object)?;
            self.push_local_set(value_local);
            self.emit_numeric_expression(search_expression)?;
            self.state.emission.output.instructions.push(0x1a);
            self.push_i32_const(-1);
            self.push_local_set(result_local);

            for (string_pointer, bytes) in self.backend.module_artifacts.string_data.clone() {
                let Ok(text) = String::from_utf8(bytes) else {
                    continue;
                };
                let index = text
                    .find(search.as_str())
                    .map(|index| index as i32)
                    .unwrap_or(-1);
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
                self.push_i32_const(index);
                self.push_local_set(result_local);
                self.state.emission.output.instructions.push(0x0b);
                self.pop_control_frame();
            }

            self.push_local_get(result_local);
            return Ok(true);
        }
        if matches!(property, Expression::String(property_name) if property_name == "replace")
            && inline_summary_side_effect_free_expression(object)
            && let Some(source_text) = self.resolve_static_string_value(object)
            && let [
                CallArgument::Expression(search_expression),
                CallArgument::Expression(replacement_expression),
            ] = arguments
            && inline_summary_side_effect_free_expression(search_expression)
            && inline_summary_side_effect_free_expression(replacement_expression)
            && let Some(search_text) = self.resolve_static_string_value(search_expression)
        {
            self.emit_numeric_expression(object)?;
            self.state.emission.output.instructions.push(0x1a);
            self.emit_numeric_expression(search_expression)?;
            self.state.emission.output.instructions.push(0x1a);
            self.emit_numeric_expression(replacement_expression)?;
            self.state.emission.output.instructions.push(0x1a);

            let replacement_text = if let Some(replacement_text) =
                self.resolve_static_string_value(replacement_expression)
            {
                Some(replacement_text)
            } else if let Some(LocalFunctionBinding::User(function_name)) =
                self.resolve_function_binding_from_expression(replacement_expression)
            {
                let Some(user_function) = self.user_function(&function_name).cloned() else {
                    self.push_i32_const(JS_UNDEFINED_TAG);
                    return Ok(true);
                };
                let Some(match_index) = source_text.find(&search_text) else {
                    self.emit_static_string_literal(&source_text)?;
                    return Ok(true);
                };
                let callback_argument_expressions = vec![
                    Expression::String(search_text.clone()),
                    Expression::Number(match_index as f64),
                    Expression::String(source_text.clone()),
                ];
                let callback_arguments = callback_argument_expressions
                    .iter()
                    .cloned()
                    .map(CallArgument::Expression)
                    .collect::<Vec<_>>();
                self.emit_user_function_call(&user_function, &callback_arguments)?;
                self.state.emission.output.instructions.push(0x1a);
                let this_binding = if user_function.strict {
                    Expression::Undefined
                } else {
                    Expression::This
                };
                self.resolve_function_binding_static_return_expression_with_call_frame(
                    &LocalFunctionBinding::User(function_name),
                    &callback_argument_expressions,
                    &this_binding,
                )
                .and_then(|value| self.resolve_static_string_value(&value))
            } else {
                None
            };

            if let Some(replacement_text) = replacement_text {
                self.emit_static_string_literal(&source_text.replacen(
                    &search_text,
                    &replacement_text,
                    1,
                ))?;
                return Ok(true);
            }
        }
        Ok(false)
    }
}
