use super::*;

impl<'a> FunctionCompiler<'a> {
    fn resolve_static_replace_callback_terminal_return_text(
        &self,
        function_name: &str,
        user_function: &UserFunction,
        callback_argument_expressions: &[Expression],
        this_binding: &Expression,
    ) -> Option<String> {
        let function = self.resolve_registered_function_declaration(function_name)?;
        let crate::ir::hir::Statement::Return(return_value) = function.body.last()? else {
            return None;
        };
        let call_arguments = callback_argument_expressions
            .iter()
            .cloned()
            .map(CallArgument::Expression)
            .collect::<Vec<_>>();
        let arguments_binding = Expression::Array(
            callback_argument_expressions
                .iter()
                .cloned()
                .map(crate::ir::hir::ArrayElement::Expression)
                .collect(),
        );
        let substituted = self.substitute_user_function_call_frame_bindings(
            return_value,
            user_function,
            &call_arguments,
            this_binding,
            &arguments_binding,
        );
        let resolved = self.resolve_static_super_members_in_call_frame_return(
            &substituted,
            function_name,
            this_binding,
        );
        self.resolve_static_string_value_with_context(&resolved, Some(function_name))
    }

    fn static_string_replace_replacement_is_shortcut_safe(
        &self,
        replacement_expression: &Expression,
    ) -> bool {
        inline_summary_side_effect_free_expression(replacement_expression)
            || self
                .resolve_function_binding_from_expression(replacement_expression)
                .is_some()
    }

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
            && let Some(StaticEvalOutcome::Value(Expression::String(value))) = self
                .resolve_static_member_call_outcome_with_context(
                    object,
                    "toString",
                    self.current_function_name(),
                )
        {
            self.emit_numeric_expression(object)?;
            self.state.emission.output.instructions.push(0x1a);
            self.emit_static_string_literal(&value)?;
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
            && self.static_string_replace_replacement_is_shortcut_safe(replacement_expression)
            && let Some(search_text) = self.resolve_static_string_value(search_expression)
        {
            let mut callback_to_emit = None;
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
                let this_binding = if user_function.strict {
                    Expression::Undefined
                } else {
                    Expression::This
                };
                let function_binding = LocalFunctionBinding::User(function_name.clone());
                let replacement_text = self
                    .resolve_function_binding_static_return_expression_with_call_frame(
                        &function_binding,
                        &callback_argument_expressions,
                        &this_binding,
                    )
                    .and_then(|value| self.resolve_static_string_value(&value))
                    .or_else(|| {
                        self.resolve_static_replace_callback_terminal_return_text(
                            &function_name,
                            &user_function,
                            &callback_argument_expressions,
                            &this_binding,
                        )
                    });
                callback_to_emit = replacement_text
                    .as_ref()
                    .map(|_| (user_function, callback_arguments));
                replacement_text
            } else {
                None
            };

            if let Some(replacement_text) = replacement_text {
                self.emit_numeric_expression(object)?;
                self.state.emission.output.instructions.push(0x1a);
                self.emit_numeric_expression(search_expression)?;
                self.state.emission.output.instructions.push(0x1a);
                if let Some((user_function, callback_arguments)) = callback_to_emit {
                    if !inline_summary_side_effect_free_expression(replacement_expression) {
                        self.emit_numeric_expression(replacement_expression)?;
                        self.state.emission.output.instructions.push(0x1a);
                    }
                    if user_function.strict {
                        self.emit_user_function_call_without_inline_with_new_target_and_this(
                            &user_function,
                            &callback_arguments,
                            JS_UNDEFINED_TAG,
                            JS_UNDEFINED_TAG,
                        )?;
                    } else {
                        self.emit_user_function_call_without_inline_with_new_target_and_this_expression(
                            &user_function,
                            &callback_arguments,
                            JS_UNDEFINED_TAG,
                            &Expression::Identifier("globalThis".to_string()),
                        )?;
                    }
                    self.state.emission.output.instructions.push(0x1a);
                } else {
                    self.emit_numeric_expression(replacement_expression)?;
                    self.state.emission.output.instructions.push(0x1a);
                }
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
