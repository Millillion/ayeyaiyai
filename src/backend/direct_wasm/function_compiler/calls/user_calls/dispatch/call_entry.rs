use super::*;

impl<'a> FunctionCompiler<'a> {
    pub(in crate::backend::direct_wasm) fn emit_user_function_call_with_new_target(
        &mut self,
        user_function: &UserFunction,
        arguments: &[CallArgument],
        new_target_value: i32,
    ) -> DirectResult<()> {
        self.emit_user_function_call_with_new_target_and_this(
            user_function,
            arguments,
            new_target_value,
            JS_TYPEOF_OBJECT_TAG,
        )
    }

    pub(in crate::backend::direct_wasm) fn emit_user_function_call_with_new_target_and_this(
        &mut self,
        user_function: &UserFunction,
        arguments: &[CallArgument],
        new_target_value: i32,
        this_value: i32,
    ) -> DirectResult<()> {
        let trace_user_calls = std::env::var_os("AYY_TRACE_USER_CALLS").is_some();
        if trace_user_calls {
            eprintln!(
                "user_call_entry:start current_fn={:?} target={} args={arguments:?}",
                self.current_function_name(),
                user_function.name
            );
        }
        let expanded_arguments = self.expand_call_arguments(arguments);
        let arguments_read_descriptor_member = expanded_arguments
            .iter()
            .any(|argument| self.expression_reads_local_descriptor_binding_member(argument));
        if trace_user_calls {
            eprintln!(
                "user_call_entry:expanded target={} descriptor_args={} expanded={expanded_arguments:?}",
                user_function.name, arguments_read_descriptor_member
            );
        }
        let materialized_inline_arguments = if arguments_read_descriptor_member {
            Vec::new()
        } else {
            expanded_arguments
                .iter()
                .map(|argument| self.materialize_static_expression(argument))
                .collect::<Vec<_>>()
        };
        if trace_user_calls {
            eprintln!(
                "user_call_entry:materialized target={} count={}",
                user_function.name,
                materialized_inline_arguments.len()
            );
        }
        let inline_this_expression = if this_value == JS_UNDEFINED_TAG {
            Expression::Undefined
        } else {
            Expression::This
        };
        let static_this_expression =
            self.resolve_static_snapshot_this_expression(&inline_this_expression);
        if trace_user_calls {
            eprintln!(
                "user_call_entry:before_deferred target={}",
                user_function.name
            );
        }
        if self.emit_deferred_generator_call_result(user_function, &expanded_arguments)? {
            return Ok(());
        }
        if trace_user_calls {
            eprintln!(
                "user_call_entry:after_deferred target={}",
                user_function.name
            );
        }
        if new_target_value == JS_UNDEFINED_TAG
            && !arguments_read_descriptor_member
            && self.emit_inline_lowered_pattern_user_function_with_arguments(
                user_function,
                &expanded_arguments,
                &inline_this_expression,
            )?
        {
            return Ok(());
        }
        if new_target_value == JS_UNDEFINED_TAG
            && self
                .can_direct_call_use_explicit_frame_without_rebinding_lexical_state(user_function)
            && !arguments_read_descriptor_member
            && self.can_inline_user_function_call_with_explicit_call_frame(
                user_function,
                &materialized_inline_arguments,
                &static_this_expression,
            )
        {
            let result_local = self.allocate_temp_local();
            if self.emit_inline_user_function_summary_with_explicit_call_frame(
                user_function,
                &expanded_arguments,
                &static_this_expression,
                result_local,
            )? {
                self.push_local_get(result_local);
                return Ok(());
            }
        }
        if new_target_value == JS_UNDEFINED_TAG
            && !arguments_read_descriptor_member
            && self.can_inline_user_function_call(user_function, &expanded_arguments)
        {
            for argument in &expanded_arguments {
                self.emit_numeric_expression(argument)?;
                self.state.emission.output.instructions.push(0x1a);
            }
            if self.emit_inline_user_function_summary_with_arguments(
                user_function,
                &expanded_arguments,
            )? {
                return Ok(());
            }
        }

        let prepared_capture_bindings =
            self.prepare_user_function_capture_bindings(user_function)?;
        if trace_user_calls {
            eprintln!(
                "user_call_entry:prepared_captures target={} count={}",
                user_function.name,
                prepared_capture_bindings.len()
            );
        }

        if arguments_read_descriptor_member {
            if trace_user_calls {
                eprintln!(
                    "user_call_entry:without_static_snapshot target={}",
                    user_function.name
                );
            }
            return self
                .emit_prepared_user_function_call_with_new_target_and_this_without_static_snapshot(
                    user_function,
                    &expanded_arguments,
                    new_target_value,
                    this_value,
                    prepared_capture_bindings,
                );
        }

        self.emit_prepared_user_function_call_with_new_target_and_this(
            user_function,
            &expanded_arguments,
            new_target_value,
            this_value,
            prepared_capture_bindings,
        )
    }

    #[allow(dead_code)]
    pub(in crate::backend::direct_wasm) fn emit_user_function_call_without_inline_with_new_target_and_this(
        &mut self,
        user_function: &UserFunction,
        arguments: &[CallArgument],
        new_target_value: i32,
        this_value: i32,
    ) -> DirectResult<()> {
        let expanded_arguments = self.expand_call_arguments(arguments);
        let prepared_capture_bindings =
            self.prepare_user_function_capture_bindings(user_function)?;
        self.emit_prepared_user_function_call_with_new_target_and_this(
            user_function,
            &expanded_arguments,
            new_target_value,
            this_value,
            prepared_capture_bindings,
        )
    }

    pub(in crate::backend::direct_wasm) fn emit_user_function_call_without_inline_or_static_snapshot_with_new_target_and_this(
        &mut self,
        user_function: &UserFunction,
        arguments: &[CallArgument],
        new_target_value: i32,
        this_value: i32,
    ) -> DirectResult<()> {
        let expanded_arguments = self.expand_call_arguments(arguments);
        let prepared_capture_bindings =
            self.prepare_user_function_capture_bindings(user_function)?;
        self.emit_prepared_user_function_call_with_new_target_and_this_without_static_snapshot(
            user_function,
            &expanded_arguments,
            new_target_value,
            this_value,
            prepared_capture_bindings,
        )
    }
}
