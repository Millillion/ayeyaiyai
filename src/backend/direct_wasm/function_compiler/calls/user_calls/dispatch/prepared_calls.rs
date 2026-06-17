use super::*;

impl<'a> FunctionCompiler<'a> {
    fn module_index_from_init_function_name(name: &str) -> Option<usize> {
        name.strip_prefix("__ayy_module_init_")?.parse().ok()
    }

    fn prepared_call_static_snapshot_result_is_direct_literal(result: &Expression) -> bool {
        matches!(
            result,
            Expression::Number(_)
                | Expression::BigInt(_)
                | Expression::String(_)
                | Expression::Bool(_)
                | Expression::Null
                | Expression::Undefined
        )
    }

    fn prepared_call_static_snapshot_result_is_function_identity(
        &self,
        result: &Expression,
    ) -> bool {
        matches!(
            result,
            Expression::Identifier(name)
                if is_internal_user_function_identifier(name)
                    && self.user_function_runtime_value(name).is_some()
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn prepared_call_static_snapshot_can_replace_runtime_call(
        &self,
        user_function: &UserFunction,
        expanded_arguments: &[Expression],
        prepared_capture_bindings: &[PreparedCaptureBinding],
        result: &Expression,
        module_init_call: bool,
        runtime_only_promise_chain_call: bool,
        assigned_nonlocal_bindings: &HashSet<String>,
        call_effect_nonlocal_bindings: &HashSet<String>,
        additional_call_effect_nonlocal_bindings: &HashSet<String>,
        updated_nonlocal_bindings: &HashSet<String>,
    ) -> bool {
        (Self::prepared_call_static_snapshot_result_is_direct_literal(result)
            && prepared_capture_bindings.is_empty()
            || self.prepared_call_static_snapshot_result_is_function_identity(result))
            && !module_init_call
            && !runtime_only_promise_chain_call
            && !user_function.is_async()
            && !user_function.is_generator()
            && !user_function.has_parameter_defaults()
            && expanded_arguments
                .iter()
                .all(inline_summary_side_effect_free_expression)
            && !self.user_function_contains_print(user_function)
            && !self.user_function_contains_identifier_callee_call(user_function)
            && assigned_nonlocal_bindings.is_empty()
            && call_effect_nonlocal_bindings.is_empty()
            && additional_call_effect_nonlocal_bindings.is_empty()
            && updated_nonlocal_bindings.is_empty()
    }

    fn emit_cache_sync_module_init_throw_if_pending(
        &mut self,
        user_function: &UserFunction,
    ) -> DirectResult<()> {
        let Some(module_index) = Self::module_index_from_init_function_name(&user_function.name)
        else {
            return Ok(());
        };
        self.push_global_get(THROW_TAG_GLOBAL_INDEX);
        self.push_i32_const(0);
        self.push_binary_op(BinaryOp::NotEqual)?;
        self.state.emission.output.instructions.push(0x04);
        self.state
            .emission
            .output
            .instructions
            .push(EMPTY_BLOCK_TYPE);
        self.push_control_frame();

        let error_local = self.allocate_temp_local();
        self.push_global_get(THROW_VALUE_GLOBAL_INDEX);
        self.push_local_set(error_local);
        self.emit_store_identifier_from_local(
            &format!("__ayy_module_error_{module_index}"),
            error_local,
        )?;

        let status_local = self.allocate_temp_local();
        self.push_i32_const(3);
        self.push_local_set(status_local);
        self.emit_store_identifier_from_local(
            &format!("__ayy_module_status_{module_index}"),
            status_local,
        )?;

        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();
        Ok(())
    }

    fn user_function_call_effects_are_assigned_bindings_only(
        &self,
        user_function: &UserFunction,
    ) -> bool {
        if user_function.has_parameter_defaults()
            || !self
                .user_function_parameter_iterator_consumption_indices(user_function)
                .is_empty()
        {
            return false;
        }
        let Some(function) = self.resolve_registered_function_declaration(&user_function.name)
        else {
            return false;
        };

        struct CallOrConstructFinder {
            found: bool,
        }
        impl crate::ir::visit::Visitor for CallOrConstructFinder {
            fn visit_expression(&mut self, expression: &Expression) {
                if self.found {
                    return;
                }
                if matches!(
                    expression,
                    Expression::Call { .. } | Expression::New { .. } | Expression::SuperCall { .. }
                ) {
                    self.found = true;
                    return;
                }
                crate::ir::visit::walk_expression(self, expression);
            }
        }

        let mut finder = CallOrConstructFinder { found: false };
        for statement in &function.body {
            crate::ir::visit::Visitor::visit_statement(&mut finder, statement);
            if finder.found {
                return false;
            }
        }
        true
    }

    pub(in crate::backend::direct_wasm) fn emit_prepared_user_function_call_with_new_target_and_this(
        &mut self,
        user_function: &UserFunction,
        expanded_arguments: &[Expression],
        new_target_value: i32,
        this_value: i32,
        prepared_capture_bindings: Vec<PreparedCaptureBinding>,
    ) -> DirectResult<()> {
        self.emit_prepared_user_function_call_with_new_target_and_this_impl(
            user_function,
            expanded_arguments,
            new_target_value,
            this_value,
            prepared_capture_bindings,
            true,
        )
    }

    pub(in crate::backend::direct_wasm) fn emit_prepared_user_function_call_with_new_target_and_this_without_static_snapshot(
        &mut self,
        user_function: &UserFunction,
        expanded_arguments: &[Expression],
        new_target_value: i32,
        this_value: i32,
        prepared_capture_bindings: Vec<PreparedCaptureBinding>,
    ) -> DirectResult<()> {
        self.emit_prepared_user_function_call_with_new_target_and_this_impl(
            user_function,
            expanded_arguments,
            new_target_value,
            this_value,
            prepared_capture_bindings,
            false,
        )
    }

    fn emit_prepared_user_function_call_with_new_target_and_this_impl(
        &mut self,
        user_function: &UserFunction,
        expanded_arguments: &[Expression],
        new_target_value: i32,
        this_value: i32,
        prepared_capture_bindings: Vec<PreparedCaptureBinding>,
        enable_static_snapshot: bool,
    ) -> DirectResult<()> {
        let trace_user_calls = crate::ayy_env_flag!("AYY_TRACE_USER_CALLS");
        if trace_user_calls {
            eprintln!(
                "prepared_user_call:start current_fn={:?} target={} enable_static_snapshot={} args={expanded_arguments:?}",
                self.current_function_name(),
                user_function.name,
                enable_static_snapshot
            );
        }
        let module_init_call = user_function.name.starts_with("__ayy_module_init_");
        let large_static_call_analysis =
            self.user_function_exceeds_static_call_analysis_budget(user_function);
        self.sync_static_with_scope_member_assignment_effects(user_function);
        let runtime_only_parameter_iterator_call = user_function.has_lowered_pattern_parameters()
            || !self
                .user_function_parameter_iterator_consumption_indices(user_function)
                .is_empty();
        let arguments_contain_await = expanded_arguments
            .iter()
            .any(Self::expression_contains_await_for_user_call_runtime);
        let runtime_only_promise_chain_call = !enable_static_snapshot
            && self.registered_function_body_mentions_promise_like_chain(&user_function.name);
        let skip_large_static_call_effect_analysis =
            large_static_call_analysis && prepared_capture_bindings.is_empty();
        let runtime_only_without_static_snapshot = !enable_static_snapshot;
        let skip_static_call_effect_analysis = runtime_only_parameter_iterator_call
            || module_init_call
            || runtime_only_without_static_snapshot
            || arguments_contain_await
            || skip_large_static_call_effect_analysis;
        if trace_user_calls {
            eprintln!(
                "prepared_user_call:after_runtime_only target={} runtime_only={} promise_chain_runtime_only={} await_args={} skip_static_effects={}",
                user_function.name,
                runtime_only_parameter_iterator_call,
                runtime_only_promise_chain_call,
                arguments_contain_await,
                skip_static_call_effect_analysis
            );
        }
        let allow_static_snapshot = enable_static_snapshot
            && !large_static_call_analysis
            && !self.user_function_mentions_private_member_access(user_function)
            && !self.user_function_contains_self_callee_reference(&user_function.name);
        if trace_user_calls {
            eprintln!(
                "prepared_user_call:after_allow_static target={} allow_static={}",
                user_function.name, allow_static_snapshot
            );
        }
        let synced_capture_source_bindings =
            self.synced_prepared_user_function_capture_source_bindings(&prepared_capture_bindings);
        if trace_user_calls {
            eprintln!(
                "prepared_user_call:after_synced_captures target={} count={}",
                user_function.name,
                synced_capture_source_bindings.len()
            );
        }
        let capture_snapshot =
            self.snapshot_user_function_capture_source_bindings(&prepared_capture_bindings);
        if trace_user_calls {
            eprintln!(
                "prepared_user_call:after_capture_snapshot target={} count={}",
                user_function.name,
                capture_snapshot.len()
            );
        }
        let this_expression = if this_value == JS_UNDEFINED_TAG {
            Expression::Undefined
        } else {
            Expression::This
        };
        let static_this_expression = self.resolve_static_snapshot_this_expression(&this_expression);
        if trace_user_calls {
            eprintln!(
                "prepared_user_call:after_static_this target={} this={static_this_expression:?}",
                user_function.name
            );
        }
        let static_result = if enable_static_snapshot
            && !runtime_only_parameter_iterator_call
            && !arguments_contain_await
            && allow_static_snapshot
            && new_target_value == JS_UNDEFINED_TAG
        {
            self.resolve_bound_snapshot_user_function_result_with_arguments_and_this(
                &user_function.name,
                &capture_snapshot,
                expanded_arguments,
                &static_this_expression,
            )
        } else {
            None
        };
        if trace_user_calls {
            eprintln!(
                "prepared_user_call:after_static_result target={} has_static={} static_result={static_result:?}",
                user_function.name,
                static_result.is_some()
            );
        }
        let updated_bindings = static_result
            .as_ref()
            .map(|(_, updated_bindings)| updated_bindings.clone())
            .or_else(|| {
                (allow_static_snapshot
                    && !runtime_only_parameter_iterator_call
                    && !arguments_contain_await)
                    .then(|| self.infer_static_class_init_nonlocal_updated_bindings(user_function))
                    .flatten()
            });
        self.state
            .speculation
            .static_semantics
            .last_bound_user_function_call = (enable_static_snapshot
            && !runtime_only_parameter_iterator_call
            && !arguments_contain_await
            && allow_static_snapshot)
            .then(|| BoundUserFunctionCallSnapshot {
                function_name: user_function.name.clone(),
                source_expression: None,
                result_expression: static_result.as_ref().map(|(result, _)| result.clone()),
                prototype_source_expression: None,
                updated_bindings: updated_bindings
                    .clone()
                    .unwrap_or_else(|| capture_snapshot.clone()),
            });
        crate::backend::direct_wasm::memo::bump_static_state_generation();
        let assigned_nonlocal_bindings = if skip_static_call_effect_analysis {
            self.prepared_user_function_assigned_nonlocal_bindings(user_function)
        } else {
            self.collect_user_function_assigned_nonlocal_bindings(user_function)
        };
        if trace_user_calls {
            eprintln!(
                "prepared_user_call:after_assigned target={} count={}",
                user_function.name,
                assigned_nonlocal_bindings.len()
            );
        }
        let mut call_effect_nonlocal_bindings = if skip_large_static_call_effect_analysis {
            assigned_nonlocal_bindings.clone()
        } else if runtime_only_without_static_snapshot
            && self.user_function_call_effects_are_assigned_bindings_only(user_function)
        {
            assigned_nonlocal_bindings.clone()
        } else if runtime_only_without_static_snapshot {
            self.collect_user_function_call_effect_nonlocal_bindings(user_function)
        } else if skip_static_call_effect_analysis {
            HashSet::new()
        } else {
            self.collect_user_function_call_effect_nonlocal_bindings(user_function)
        };
        if trace_user_calls {
            eprintln!(
                "prepared_user_call:after_call_effect target={} count={}",
                user_function.name,
                call_effect_nonlocal_bindings.len()
            );
        }
        if !skip_static_call_effect_analysis {
            call_effect_nonlocal_bindings.extend(
                self.collect_user_function_argument_call_effect_nonlocal_bindings(
                    user_function,
                    expanded_arguments,
                ),
            );
        }
        if trace_user_calls {
            eprintln!(
                "prepared_user_call:after_arg_call_effect target={} count={}",
                user_function.name,
                call_effect_nonlocal_bindings.len()
            );
        }
        let assigned_nonlocal_binding_results = if skip_static_call_effect_analysis {
            None
        } else {
            self.assigned_nonlocal_binding_results(&user_function.name)
                .cloned()
        };
        let additional_call_effect_nonlocal_bindings =
            if skip_large_static_call_effect_analysis || runtime_only_without_static_snapshot {
                call_effect_nonlocal_bindings
                    .iter()
                    .filter(|name| !synced_capture_source_bindings.contains(*name))
                    .cloned()
                    .collect::<HashSet<_>>()
            } else if skip_static_call_effect_analysis {
                HashSet::new()
            } else {
                let mut names = call_effect_nonlocal_bindings
                    .iter()
                    .filter(|name| !synced_capture_source_bindings.contains(*name))
                    .cloned()
                    .collect::<HashSet<_>>();
                names.extend(self.collect_snapshot_updated_nonlocal_bindings(
                    user_function,
                    updated_bindings.as_ref(),
                ));
                names
            };
        let updated_nonlocal_bindings = if skip_static_call_effect_analysis {
            assigned_nonlocal_bindings.clone()
        } else {
            self.collect_user_function_updated_nonlocal_bindings(user_function)
        };
        if trace_user_calls {
            eprintln!(
                "prepared_user_call:after_updated target={} count={}",
                user_function.name,
                updated_nonlocal_bindings.len()
            );
        }
        if let Some((static_result, _)) = &static_result
            && self.prepared_call_static_snapshot_can_replace_runtime_call(
                user_function,
                expanded_arguments,
                &prepared_capture_bindings,
                static_result,
                module_init_call,
                runtime_only_promise_chain_call,
                &assigned_nonlocal_bindings,
                &call_effect_nonlocal_bindings,
                &additional_call_effect_nonlocal_bindings,
                &updated_nonlocal_bindings,
            )
        {
            if trace_user_calls {
                eprintln!(
                    "prepared_user_call:emit_static_snapshot_result target={}",
                    user_function.name
                );
            }
            self.emit_numeric_expression(static_result)?;
            return Ok(());
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
            self.push_global_get(CURRENT_THIS_GLOBAL_INDEX);
            self.push_local_set(saved_local);
            self.push_i32_const(this_value);
            self.push_global_set(CURRENT_THIS_GLOBAL_INDEX);
            Some(saved_local)
        };
        let saved_this_shadow_owner = if user_function.lexical_this || module_init_call {
            None
        } else {
            self.prepare_user_function_runtime_this_shadow_state(&this_expression)?
        };

        self.emit_prepare_user_function_capture_globals(&user_function.name)?;
        if trace_user_calls {
            eprintln!(
                "prepared_user_call:after_prepare_captures target={}",
                user_function.name
            );
        }
        let (
            return_value_local,
            parameter_object_shadow_writebacks,
            static_argument_member_writebacks,
        ) = self.emit_user_function_runtime_call_from_expanded_arguments(
            user_function,
            expanded_arguments,
            updated_bindings.as_ref(),
            runtime_only_promise_chain_call,
        )?;
        if trace_user_calls {
            eprintln!(
                "prepared_user_call:after_runtime_call target={} return_local={}",
                user_function.name, return_value_local
            );
        }
        if module_init_call {
            self.restore_user_function_capture_bindings(&prepared_capture_bindings);
            if let Some(saved_new_target_local) = saved_new_target_local {
                self.push_local_get(saved_new_target_local);
                self.push_global_set(CURRENT_NEW_TARGET_GLOBAL_INDEX);
            }
            if let Some(saved_this_local) = saved_this_local {
                self.push_local_get(saved_this_local);
                self.push_global_set(CURRENT_THIS_GLOBAL_INDEX);
            }
            if user_function.is_async() {
                self.emit_async_call_pending_rejection_capture()?;
                self.push_i32_const(JS_TYPEOF_OBJECT_TAG);
            } else {
                self.emit_cache_sync_module_init_throw_if_pending(user_function)?;
                self.emit_check_global_throw_for_user_call()?;
                self.push_local_get(return_value_local);
            }
            return Ok(());
        }
        let receiver_updated_via_parameter_writeback = self
            .receiver_shadow_updated_via_parameter_writebacks(
                &this_expression,
                &parameter_object_shadow_writebacks,
            );
        self.finalize_user_function_call(
            user_function,
            &this_expression,
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
            expanded_arguments,
        )?;
        self.sync_static_argument_object_member_writeback_values(
            &static_argument_member_writebacks,
        );
        Ok(())
    }
}
