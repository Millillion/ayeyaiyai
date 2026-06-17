use super::*;
mod finalization;
mod planning;
mod runtime_call;
use self::planning::GeneralUserFunctionCallPlan;

impl<'a> FunctionCompiler<'a> {
    fn nonlocal_set_is_empty_or_only_this(names: &HashSet<String>) -> bool {
        names.is_empty() || (names.len() == 1 && names.contains("this"))
    }

    fn static_result_is_internal_function_identity(&self, result: &Expression) -> bool {
        matches!(
            result,
            Expression::Identifier(name)
                if is_internal_user_function_identifier(name)
                    && self.user_function_runtime_value(name).is_some()
        )
    }

    pub(in crate::backend::direct_wasm) fn simple_this_member_write_return_function_identity(
        &self,
        user_function: &UserFunction,
    ) -> Option<(Expression, Vec<(String, Expression)>)> {
        if user_function.lexical_this
            || user_function.is_async()
            || user_function.is_generator()
            || user_function.has_parameter_defaults()
            || user_function.has_lowered_pattern_parameters()
            || self.user_function_mentions_private_member_access(user_function)
            || self.user_function_mentions_direct_eval(user_function)
        {
            return None;
        }
        let function = self.resolve_registered_function_declaration(&user_function.name)?;
        let (last, prefix) = function.body.split_last()?;
        let Statement::Return(result) = last else {
            return None;
        };
        if prefix.is_empty() || !self.static_result_is_internal_function_identity(result) {
            return None;
        }
        let mut writes = Vec::new();
        for statement in prefix {
            let Statement::AssignMember {
                object,
                property,
                value,
            } = statement
            else {
                return None;
            };
            if !matches!(object, Expression::This)
                || !inline_summary_side_effect_free_expression(property)
                || !inline_summary_side_effect_free_expression(value)
            {
                return None;
            }
            writes.push((
                static_property_name_from_expression(property)?,
                value.clone(),
            ));
        }
        Some((result.clone(), writes))
    }

    fn can_emit_static_this_only_function_identity_call(
        &self,
        user_function: &UserFunction,
        static_result: &Expression,
        prepared_capture_bindings: &[PreparedCaptureBinding],
        assigned_nonlocal_bindings: &HashSet<String>,
        call_effect_nonlocal_bindings: &HashSet<String>,
        updated_nonlocal_bindings: &HashSet<String>,
        additional_call_effect_nonlocal_bindings: &HashSet<String>,
        updated_bindings: Option<&HashMap<String, Expression>>,
    ) -> bool {
        !user_function.lexical_this
            && !user_function.is_async()
            && !user_function.is_generator()
            && prepared_capture_bindings.is_empty()
            && self.static_result_is_internal_function_identity(static_result)
            && assigned_nonlocal_bindings.is_empty()
            && call_effect_nonlocal_bindings.contains("this")
            && Self::nonlocal_set_is_empty_or_only_this(call_effect_nonlocal_bindings)
            && updated_nonlocal_bindings.contains("this")
            && Self::nonlocal_set_is_empty_or_only_this(updated_nonlocal_bindings)
            && Self::nonlocal_set_is_empty_or_only_this(additional_call_effect_nonlocal_bindings)
            && updated_bindings.is_some_and(|bindings| {
                bindings.contains_key("this") && bindings.keys().all(|name| name == "this")
            })
    }

    pub(in crate::backend::direct_wasm) fn emit_simple_this_member_write_return_function_identity_call(
        &mut self,
        user_function: &UserFunction,
        expanded_arguments: &[Expression],
        new_target_value: i32,
        this_expression: &Expression,
        static_result: &Expression,
        writes: &[(String, Expression)],
    ) -> DirectResult<bool> {
        let trace_user_calls = crate::ayy_env_flag!("AYY_TRACE_USER_CALLS");
        let trace_started_at = trace_user_calls.then(std::time::Instant::now);
        if new_target_value != JS_UNDEFINED_TAG
            || self.should_box_sloppy_function_this(user_function, this_expression)
            || !inline_summary_side_effect_free_expression(this_expression)
            || !expanded_arguments
                .iter()
                .all(inline_summary_side_effect_free_expression)
        {
            return Ok(false);
        }
        if trace_user_calls {
            eprintln!(
                "simple_this_identity:checked target={} elapsed_ms={}",
                user_function.name,
                trace_started_at.unwrap().elapsed().as_millis()
            );
        }
        let Some(target_owner) =
            self.resolve_user_function_call_receiver_shadow_owner(this_expression)
        else {
            return Ok(false);
        };
        if trace_user_calls {
            eprintln!(
                "simple_this_identity:receiver target={} owner={target_owner} elapsed_ms={}",
                user_function.name,
                trace_started_at.unwrap().elapsed().as_millis()
            );
        }

        let mut updated_bindings = HashMap::new();
        updated_bindings.insert("this".to_string(), this_expression.clone());
        self.state
            .speculation
            .static_semantics
            .last_bound_user_function_call = Some(BoundUserFunctionCallSnapshot {
            function_name: user_function.name.clone(),
            source_expression: None,
            result_expression: Some(static_result.clone()),
            prototype_source_expression: None,
            updated_bindings: updated_bindings.clone(),
        });
        crate::backend::direct_wasm::memo::bump_static_state_generation();
        if trace_user_calls {
            eprintln!(
                "simple_this_identity:snapshot target={} elapsed_ms={}",
                user_function.name,
                trace_started_at.unwrap().elapsed().as_millis()
            );
        }

        self.with_suspended_with_scopes_if_active_scope_object(this_expression, |compiler| {
            compiler.emit_numeric_expression(this_expression)
        })?;
        self.state.emission.output.instructions.push(0x1a);
        for argument in expanded_arguments {
            self.emit_numeric_expression(argument)?;
            self.state.emission.output.instructions.push(0x1a);
        }
        if trace_user_calls {
            eprintln!(
                "simple_this_identity:emitted_inputs target={} elapsed_ms={}",
                user_function.name,
                trace_started_at.unwrap().elapsed().as_millis()
            );
        }

        let call_arguments = expanded_arguments
            .iter()
            .cloned()
            .map(CallArgument::Expression)
            .collect::<Vec<_>>();
        let mut receiver_binding = self
            .resolve_runtime_shadow_object_binding(&target_owner)
            .or_else(|| self.resolve_object_binding_from_expression(this_expression))
            .unwrap_or_else(empty_object_value_binding);
        if trace_user_calls {
            eprintln!(
                "simple_this_identity:resolved_receiver_binding target={} elapsed_ms={}",
                user_function.name,
                trace_started_at.unwrap().elapsed().as_millis()
            );
        }
        for (property_name, value) in writes {
            let property = Expression::String(property_name.clone());
            let substituted = self.substitute_user_function_argument_bindings(
                value,
                user_function,
                &call_arguments,
            );
            let materialized_value = self.materialize_static_expression(&substituted);
            object_binding_set_property(
                &mut receiver_binding,
                property.clone(),
                materialized_value.clone(),
            );
            let shadow_binding =
                self.runtime_object_property_shadow_binding_by_names(&target_owner, property_name);
            self.emit_numeric_expression(&materialized_value)?;
            self.push_global_set(shadow_binding.value_index);
            self.push_i32_const(1);
            self.push_global_set(shadow_binding.present_index);
            if trace_user_calls {
                eprintln!(
                    "simple_this_identity:write target={} property={property_name} elapsed_ms={}",
                    user_function.name,
                    trace_started_at.unwrap().elapsed().as_millis()
                );
            }
        }
        let updated_receiver_expression = object_binding_to_expression(&receiver_binding);
        self.update_local_value_binding(&target_owner, &updated_receiver_expression);
        self.update_local_object_binding_from_resolved(
            &target_owner,
            &updated_receiver_expression,
            receiver_binding,
        );
        if trace_user_calls {
            eprintln!(
                "simple_this_identity:updated_receiver target={} elapsed_ms={}",
                user_function.name,
                trace_started_at.unwrap().elapsed().as_millis()
            );
        }
        self.emit_numeric_expression(static_result)?;
        if trace_user_calls {
            eprintln!(
                "simple_this_identity:done target={} elapsed_ms={}",
                user_function.name,
                trace_started_at.unwrap().elapsed().as_millis()
            );
        }
        Ok(true)
    }

    #[allow(clippy::too_many_arguments)]
    fn emit_static_this_only_function_identity_call(
        &mut self,
        user_function: &UserFunction,
        expanded_arguments: &[Expression],
        new_target_value: i32,
        this_expression: &Expression,
        static_result: &Expression,
        prepared_capture_bindings: &[PreparedCaptureBinding],
        assigned_nonlocal_bindings: &HashSet<String>,
        call_effect_nonlocal_bindings: &HashSet<String>,
        updated_nonlocal_bindings: &HashSet<String>,
        updated_bindings: Option<&HashMap<String, Expression>>,
        additional_call_effect_nonlocal_bindings: HashSet<String>,
        assigned_nonlocal_binding_results: Option<HashMap<String, Expression>>,
    ) -> DirectResult<()> {
        let saved_new_target_local = if user_function.lexical_this {
            None
        } else {
            let saved_local = self.allocate_temp_local();
            self.push_global_get(CURRENT_NEW_TARGET_GLOBAL_INDEX);
            self.push_local_set(saved_local);
            self.push_i32_const(new_target_value);
            self.push_global_set(CURRENT_NEW_TARGET_GLOBAL_INDEX);
            Some(saved_local)
        };
        let saved_this_local = if user_function.lexical_this {
            None
        } else {
            let saved_local = self.allocate_temp_local();
            let this_local = self.allocate_temp_local();
            self.push_global_get(CURRENT_THIS_GLOBAL_INDEX);
            self.push_local_set(saved_local);
            self.with_suspended_with_scopes_if_active_scope_object(this_expression, |compiler| {
                compiler.emit_numeric_expression(this_expression)
            })?;
            self.push_local_set(this_local);
            self.push_local_get(this_local);
            self.push_global_set(CURRENT_THIS_GLOBAL_INDEX);
            Some(saved_local)
        };
        let saved_this_shadow_owner = if user_function.lexical_this {
            None
        } else {
            self.predeclare_user_function_this_private_initializer_shadow_properties(user_function);
            self.with_suspended_with_scopes_if_active_scope_object(this_expression, |compiler| {
                compiler.prepare_user_function_runtime_this_shadow_state(this_expression)
            })?
        };

        for argument in expanded_arguments {
            self.emit_numeric_expression(argument)?;
            self.state.emission.output.instructions.push(0x1a);
        }

        let return_value_local = self.allocate_temp_local();
        self.emit_numeric_expression(static_result)?;
        self.push_local_set(return_value_local);
        self.finalize_user_function_call(
            user_function,
            this_expression,
            false,
            prepared_capture_bindings,
            assigned_nonlocal_bindings,
            call_effect_nonlocal_bindings,
            updated_nonlocal_bindings,
            updated_bindings,
            additional_call_effect_nonlocal_bindings,
            assigned_nonlocal_binding_results,
            saved_new_target_local,
            saved_this_local,
            saved_this_shadow_owner.as_deref(),
            return_value_local,
            expanded_arguments,
        )
    }

    pub(in crate::backend::direct_wasm) fn emit_user_function_call_with_new_target_and_this_expression(
        &mut self,
        user_function: &UserFunction,
        arguments: &[CallArgument],
        new_target_value: i32,
        this_expression: &Expression,
    ) -> DirectResult<()> {
        self.emit_user_function_call_with_new_target_and_this_expression_impl(
            user_function,
            arguments,
            new_target_value,
            this_expression,
            true,
            true,
        )
    }

    pub(in crate::backend::direct_wasm) fn emit_user_function_call_with_new_target_and_this_expression_without_static_snapshot(
        &mut self,
        user_function: &UserFunction,
        arguments: &[CallArgument],
        new_target_value: i32,
        this_expression: &Expression,
    ) -> DirectResult<()> {
        self.emit_user_function_call_with_new_target_and_this_expression_impl(
            user_function,
            arguments,
            new_target_value,
            this_expression,
            false,
            false,
        )
    }

    pub(in crate::backend::direct_wasm) fn emit_user_function_call_without_inline_with_new_target_and_this_expression(
        &mut self,
        user_function: &UserFunction,
        arguments: &[CallArgument],
        new_target_value: i32,
        this_expression: &Expression,
    ) -> DirectResult<()> {
        self.emit_user_function_call_with_new_target_and_this_expression_impl(
            user_function,
            arguments,
            new_target_value,
            this_expression,
            true,
            false,
        )
    }

    fn emit_user_function_call_with_new_target_and_this_expression_impl(
        &mut self,
        user_function: &UserFunction,
        arguments: &[CallArgument],
        new_target_value: i32,
        this_expression: &Expression,
        enable_static_snapshot: bool,
        allow_inline: bool,
    ) -> DirectResult<()> {
        let expanded_arguments = self.expand_call_arguments(arguments);
        let arguments_contain_await = expanded_arguments
            .iter()
            .any(Self::expression_contains_await_for_user_call_runtime);
        if allow_inline
            && !arguments_contain_await
            && let Some((static_result, writes)) =
                self.simple_this_member_write_return_function_identity(user_function)
            && self.emit_simple_this_member_write_return_function_identity_call(
                user_function,
                &expanded_arguments,
                new_target_value,
                this_expression,
                &static_result,
                &writes,
            )?
        {
            return Ok(());
        }
        let materialized_inline_arguments = if arguments_contain_await {
            Vec::new()
        } else {
            expanded_arguments
                .iter()
                .map(|argument| self.materialize_static_expression(argument))
                .collect::<Vec<_>>()
        };
        let static_this_expression = self
            .with_suspended_with_scopes_if_active_scope_object(this_expression, |compiler| {
                Ok(compiler.resolve_static_snapshot_this_expression(this_expression))
            })?;
        if self.emit_deferred_generator_call_result(user_function, &expanded_arguments)? {
            return Ok(());
        }
        if allow_inline && !arguments_contain_await {
            if self.emit_inline_lowered_pattern_user_function_with_arguments(
                user_function,
                &expanded_arguments,
                this_expression,
            )? {
                return Ok(());
            }
        }
        if allow_inline
            && !arguments_contain_await
            && self.can_inline_user_function_call(user_function, &expanded_arguments)
        {
            self.with_suspended_with_scopes_if_active_scope_object(this_expression, |compiler| {
                compiler.emit_numeric_expression(this_expression)
            })?;
            self.state.emission.output.instructions.push(0x1a);
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

        let GeneralUserFunctionCallPlan {
            expanded_arguments,
            prepared_capture_bindings,
            assigned_nonlocal_bindings,
            call_effect_nonlocal_bindings,
            updated_nonlocal_bindings,
            additional_call_effect_nonlocal_bindings,
            assigned_nonlocal_binding_results,
            updated_bindings,
            static_result,
            skip_static_argument_member_writebacks,
        } = self.prepare_general_user_function_call_plan(
            user_function,
            expanded_arguments,
            new_target_value,
            &static_this_expression,
            enable_static_snapshot && !arguments_contain_await,
        )?;

        if new_target_value == JS_UNDEFINED_TAG
            && let Some(static_result) = static_result.as_ref()
            && self.can_emit_static_this_only_function_identity_call(
                user_function,
                static_result,
                &prepared_capture_bindings,
                &assigned_nonlocal_bindings,
                &call_effect_nonlocal_bindings,
                &updated_nonlocal_bindings,
                &additional_call_effect_nonlocal_bindings,
                updated_bindings.as_ref(),
            )
        {
            return self.emit_static_this_only_function_identity_call(
                user_function,
                &expanded_arguments,
                new_target_value,
                this_expression,
                static_result,
                &prepared_capture_bindings,
                &assigned_nonlocal_bindings,
                &call_effect_nonlocal_bindings,
                &updated_nonlocal_bindings,
                updated_bindings.as_ref(),
                additional_call_effect_nonlocal_bindings,
                assigned_nonlocal_binding_results,
            );
        }

        let saved_new_target_local = if user_function.lexical_this {
            None
        } else {
            let saved_local = self.allocate_temp_local();
            self.push_global_get(CURRENT_NEW_TARGET_GLOBAL_INDEX);
            self.push_local_set(saved_local);
            self.push_i32_const(new_target_value);
            self.push_global_set(CURRENT_NEW_TARGET_GLOBAL_INDEX);
            Some(saved_local)
        };
        let saved_this_local = if user_function.lexical_this {
            None
        } else {
            let saved_local = self.allocate_temp_local();
            let this_local = self.allocate_temp_local();
            self.push_global_get(CURRENT_THIS_GLOBAL_INDEX);
            self.push_local_set(saved_local);
            self.with_suspended_with_scopes_if_active_scope_object(this_expression, |compiler| {
                compiler.emit_numeric_expression(this_expression)
            })?;
            self.push_local_set(this_local);
            self.push_local_get(this_local);
            self.push_global_set(CURRENT_THIS_GLOBAL_INDEX);
            Some(saved_local)
        };
        let saved_this_shadow_owner = if user_function.lexical_this {
            None
        } else {
            self.predeclare_user_function_this_private_initializer_shadow_properties(user_function);
            self.with_suspended_with_scopes_if_active_scope_object(this_expression, |compiler| {
                compiler.prepare_user_function_runtime_this_shadow_state(this_expression)
            })?
        };

        self.emit_prepare_user_function_capture_globals(&user_function.name)?;

        let (
            return_value_local,
            parameter_object_shadow_writebacks,
            static_argument_member_writebacks,
        ) = self.emit_user_function_runtime_call_from_expanded_arguments(
            user_function,
            &expanded_arguments,
            updated_bindings.as_ref(),
            skip_static_argument_member_writebacks,
        )?;
        let receiver_updated_via_parameter_writeback = self
            .receiver_shadow_updated_via_parameter_writebacks(
                this_expression,
                &parameter_object_shadow_writebacks,
            );
        self.finalize_user_function_call(
            user_function,
            this_expression,
            receiver_updated_via_parameter_writeback,
            &prepared_capture_bindings,
            &assigned_nonlocal_bindings,
            &call_effect_nonlocal_bindings,
            &updated_nonlocal_bindings,
            updated_bindings.as_ref(),
            additional_call_effect_nonlocal_bindings,
            assigned_nonlocal_binding_results,
            saved_new_target_local,
            saved_this_local,
            saved_this_shadow_owner.as_deref(),
            return_value_local,
            &expanded_arguments,
        )?;
        self.sync_static_argument_object_member_writeback_values(
            &static_argument_member_writebacks,
        );
        Ok(())
    }
}
