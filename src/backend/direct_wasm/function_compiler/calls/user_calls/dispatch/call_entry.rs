use super::*;

impl<'a> FunctionCompiler<'a> {
    pub(in crate::backend::direct_wasm) fn expression_contains_await_for_user_call_runtime(
        expression: &Expression,
    ) -> bool {
        match expression {
            Expression::Await(_) => true,
            Expression::Array(elements) => elements.iter().any(|element| match element {
                ArrayElement::Expression(value) | ArrayElement::Spread(value) => {
                    Self::expression_contains_await_for_user_call_runtime(value)
                }
            }),
            Expression::Object(entries) => entries.iter().any(|entry| match entry {
                ObjectEntry::Data { key, value } => {
                    Self::expression_contains_await_for_user_call_runtime(key)
                        || Self::expression_contains_await_for_user_call_runtime(value)
                }
                ObjectEntry::Getter { key, getter } => {
                    Self::expression_contains_await_for_user_call_runtime(key)
                        || Self::expression_contains_await_for_user_call_runtime(getter)
                }
                ObjectEntry::Setter { key, setter } => {
                    Self::expression_contains_await_for_user_call_runtime(key)
                        || Self::expression_contains_await_for_user_call_runtime(setter)
                }
                ObjectEntry::Spread(value) => {
                    Self::expression_contains_await_for_user_call_runtime(value)
                }
            }),
            Expression::Member { object, property } => {
                Self::expression_contains_await_for_user_call_runtime(object)
                    || Self::expression_contains_await_for_user_call_runtime(property)
            }
            Expression::SuperMember { property } => {
                Self::expression_contains_await_for_user_call_runtime(property)
            }
            Expression::Call { callee, arguments }
            | Expression::SuperCall { callee, arguments }
            | Expression::New { callee, arguments } => {
                Self::expression_contains_await_for_user_call_runtime(callee)
                    || arguments.iter().any(|argument| {
                        Self::expression_contains_await_for_user_call_runtime(argument.expression())
                    })
            }
            Expression::Assign { value, .. }
            | Expression::EnumerateKeys(value)
            | Expression::GetIterator(value)
            | Expression::IteratorClose(value)
            | Expression::Unary {
                expression: value, ..
            } => Self::expression_contains_await_for_user_call_runtime(value),
            Expression::AssignMember {
                object,
                property,
                value,
            } => {
                Self::expression_contains_await_for_user_call_runtime(object)
                    || Self::expression_contains_await_for_user_call_runtime(property)
                    || Self::expression_contains_await_for_user_call_runtime(value)
            }
            Expression::AssignSuperMember { property, value } => {
                Self::expression_contains_await_for_user_call_runtime(property)
                    || Self::expression_contains_await_for_user_call_runtime(value)
            }
            Expression::Binary { left, right, .. } => {
                Self::expression_contains_await_for_user_call_runtime(left)
                    || Self::expression_contains_await_for_user_call_runtime(right)
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                Self::expression_contains_await_for_user_call_runtime(condition)
                    || Self::expression_contains_await_for_user_call_runtime(then_expression)
                    || Self::expression_contains_await_for_user_call_runtime(else_expression)
            }
            Expression::Sequence(expressions) => expressions
                .iter()
                .any(Self::expression_contains_await_for_user_call_runtime),
            Expression::Number(_)
            | Expression::BigInt(_)
            | Expression::String(_)
            | Expression::Bool(_)
            | Expression::Null
            | Expression::Undefined
            | Expression::Identifier(_)
            | Expression::This
            | Expression::NewTarget
            | Expression::Sent
            | Expression::Update { .. } => false,
        }
    }

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
        let trace_user_calls = crate::ayy_env_flag!("AYY_TRACE_USER_CALLS");
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
        let arguments_contain_await = expanded_arguments
            .iter()
            .any(Self::expression_contains_await_for_user_call_runtime);
        let arguments_require_runtime_only =
            arguments_read_descriptor_member || arguments_contain_await;
        if trace_user_calls {
            eprintln!(
                "user_call_entry:expanded target={} descriptor_args={} await_args={} expanded={expanded_arguments:?}",
                user_function.name, arguments_read_descriptor_member, arguments_contain_await
            );
        }
        let materialized_inline_arguments = if arguments_require_runtime_only {
            Vec::new()
        } else {
            expanded_arguments
                .iter()
                .map(|argument| {
                    let materialized = self.materialize_static_expression(argument);
                    (self.infer_value_kind(&materialized) == Some(StaticValueKind::Number))
                        .then(|| self.resolve_static_number_value(&materialized))
                        .flatten()
                        .map(Expression::Number)
                        .unwrap_or(materialized)
                })
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
            && !arguments_require_runtime_only
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
            && !arguments_require_runtime_only
        {
            let explicit_inline_arguments = if expanded_arguments
                .iter()
                .all(inline_summary_side_effect_free_expression)
            {
                &materialized_inline_arguments
            } else {
                &expanded_arguments
            };
            if self.can_inline_user_function_call_with_explicit_call_frame(
                user_function,
                explicit_inline_arguments,
                &static_this_expression,
            ) {
                let result_local = self.allocate_temp_local();
                if self.emit_inline_user_function_summary_with_explicit_call_frame(
                    user_function,
                    explicit_inline_arguments,
                    &static_this_expression,
                    result_local,
                )? {
                    self.push_local_get(result_local);
                    return Ok(());
                }
            }
        }
        let can_inline_materialized = new_target_value == JS_UNDEFINED_TAG
            && !arguments_require_runtime_only
            && self.can_inline_user_function_call_with_materialized_arguments(
                user_function,
                &expanded_arguments,
                &materialized_inline_arguments,
            );
        if trace_user_calls {
            eprintln!(
                "user_call_entry:direct_inline_check target={} can={} materialized={materialized_inline_arguments:?}",
                user_function.name, can_inline_materialized
            );
        }
        if can_inline_materialized {
            for argument in &expanded_arguments {
                self.emit_numeric_expression(argument)?;
                self.state.emission.output.instructions.push(0x1a);
            }
            if self.emit_inline_user_function_summary_with_arguments(
                user_function,
                &materialized_inline_arguments,
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

        if arguments_require_runtime_only {
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
