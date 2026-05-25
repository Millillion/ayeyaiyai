use super::*;

impl<'a> FunctionCompiler<'a> {
    pub(in crate::backend::direct_wasm) fn emit_user_function_call_with_new_target_and_this_expression_and_bound_captures_from_argument_locals(
        &mut self,
        user_function: &UserFunction,
        argument_locals: &[u32],
        argument_count: usize,
        new_target_value: i32,
        this_expression: &Expression,
        capture_slots: &BTreeMap<String, String>,
    ) -> DirectResult<()> {
        let runtime_only_parameter_iterator_call = user_function.has_lowered_pattern_parameters()
            || !self
                .user_function_parameter_iterator_consumption_indices(user_function)
                .is_empty();
        let has_member_source_capture =
            self.bound_capture_slots_include_member_source(capture_slots);
        let allow_static_snapshot = !self
            .user_function_mentions_private_member_access(user_function)
            && !has_member_source_capture
            && !self.user_function_mentions_direct_eval(user_function);
        let (
            prepared_capture_bindings,
            synced_capture_source_bindings,
            saved_new_target_local,
            saved_this_local,
            saved_this_shadow_owner,
        ) = self.prepare_bound_user_function_call_context(
            user_function,
            capture_slots,
            new_target_value,
            this_expression,
        )?;

        let capture_snapshot =
            self.snapshot_prepared_bound_user_function_capture_bindings(&prepared_capture_bindings);
        let bound_argument_expressions = argument_locals
            .iter()
            .take(argument_count)
            .map(|argument_local| {
                self.state
                    .runtime
                    .locals
                    .iter()
                    .find_map(|(name, local)| {
                        (*local == *argument_local).then_some(
                            self.state
                                .speculation
                                .static_semantics
                                .local_value_binding(name)
                                .cloned()
                                .or_else(|| {
                                    self.resolve_object_binding_from_expression(
                                        &Expression::Identifier(name.clone()),
                                    )
                                    .map(|binding| object_binding_to_expression(&binding))
                                })
                                .or_else(|| {
                                    self.resolve_array_binding_from_expression(
                                        &Expression::Identifier(name.clone()),
                                    )
                                    .map(|binding| {
                                        Expression::Array(
                                            binding
                                                .values
                                                .into_iter()
                                                .map(|value| {
                                                    ArrayElement::Expression(
                                                        value.unwrap_or(Expression::Undefined),
                                                    )
                                                })
                                                .collect(),
                                        )
                                    })
                                })
                                .unwrap_or(Expression::Identifier(name.clone())),
                        )
                    })
                    .unwrap_or(Expression::Undefined)
            })
            .collect::<Vec<_>>();
        let static_result = if runtime_only_parameter_iterator_call || !allow_static_snapshot {
            None
        } else {
            self.resolve_bound_snapshot_user_function_result_with_arguments_and_this(
                &user_function.name,
                &capture_snapshot,
                &bound_argument_expressions,
                this_expression,
            )
        };
        let reliable_updated_bindings = static_result
            .as_ref()
            .map(|(_, updated_bindings)| updated_bindings.clone());
        let existing_snapshot = self
            .state
            .speculation
            .static_semantics
            .last_bound_user_function_call
            .clone()
            .filter(|snapshot| snapshot.function_name == user_function.name);
        self.state
            .speculation
            .static_semantics
            .last_bound_user_function_call = (!runtime_only_parameter_iterator_call
            && allow_static_snapshot)
            .then(|| BoundUserFunctionCallSnapshot {
                function_name: user_function.name.clone(),
                source_expression: None,
                result_expression: static_result.as_ref().map(|(result, _)| result.clone()),
                prototype_source_expression: None,
                updated_bindings: reliable_updated_bindings.clone().unwrap_or_default(),
            })
            .or(existing_snapshot);
        let mut call_effect_nonlocal_bindings = if runtime_only_parameter_iterator_call {
            HashSet::new()
        } else {
            self.collect_user_function_call_effect_nonlocal_bindings(user_function)
        };
        if !runtime_only_parameter_iterator_call {
            call_effect_nonlocal_bindings.extend(
                self.collect_user_function_argument_call_effect_nonlocal_bindings(
                    user_function,
                    &bound_argument_expressions,
                ),
            );
        }
        let assigned_nonlocal_binding_results = if runtime_only_parameter_iterator_call {
            None
        } else {
            self.assigned_nonlocal_binding_results(&user_function.name)
                .cloned()
        };
        let closure_slot_capture_names = prepared_capture_bindings
            .iter()
            .filter_map(|binding| {
                binding
                    .source_binding_name
                    .as_ref()
                    .is_some_and(|source_name| source_name.starts_with("__ayy_closure_slot_"))
                    .then(|| binding.capture_name.clone())
            })
            .collect::<HashSet<_>>();
        let additional_call_effect_nonlocal_bindings = if runtime_only_parameter_iterator_call {
            HashSet::new()
        } else {
            let mut names = call_effect_nonlocal_bindings
                .iter()
                .filter(|name| {
                    if closure_slot_capture_names.contains(*name) {
                        return false;
                    }
                    !synced_capture_source_bindings.contains(*name)
                })
                .cloned()
                .collect::<HashSet<_>>();
            names.extend(
                self.collect_snapshot_updated_nonlocal_bindings(
                    user_function,
                    static_result
                        .as_ref()
                        .map(|(_, updated_bindings)| updated_bindings),
                ),
            );
            names.retain(|name| !synced_capture_source_bindings.contains(name));
            names
        };

        self.emit_prepare_bound_user_function_capture_globals(&prepared_capture_bindings)?;
        let static_argument_member_writebacks = self
            .user_function_static_argument_object_member_writeback_values(
                user_function,
                &bound_argument_expressions,
            );
        self.predeclare_static_argument_object_member_writeback_properties(
            &static_argument_member_writebacks,
        );
        let parameter_object_shadow_writebacks = self
            .emit_user_function_parameter_object_shadow_setup(
                user_function,
                &bound_argument_expressions,
            )?;

        let visible_param_count = user_function.visible_param_count() as usize;
        for argument_index in 0..visible_param_count {
            if let Some(argument_local) = argument_locals.get(argument_index).copied() {
                self.push_local_get(argument_local);
            } else {
                self.push_i32_const(JS_UNDEFINED_TAG);
            }
        }

        self.push_i32_const(argument_count as i32);

        for index in &user_function.extra_argument_indices {
            if let Some(argument_local) = argument_locals.get(*index as usize).copied() {
                self.push_local_get(argument_local);
            } else {
                self.push_i32_const(JS_UNDEFINED_TAG);
            }
        }
        self.push_user_function_call(user_function);
        let return_value_local = self.allocate_temp_local();
        self.push_local_set(return_value_local);
        self.emit_user_function_parameter_object_shadow_writeback(
            &parameter_object_shadow_writebacks,
        )?;
        let receiver_updated_via_parameter_writeback = self
            .receiver_shadow_updated_via_parameter_writebacks(
                this_expression,
                &parameter_object_shadow_writebacks,
            );
        let updated_bindings = reliable_updated_bindings;
        self.sync_user_function_parameter_object_shadow_writeback_static_metadata(
            &parameter_object_shadow_writebacks,
            updated_bindings.as_ref(),
        );
        self.sync_static_argument_object_member_writeback_values(
            &static_argument_member_writebacks,
        );

        self.finalize_bound_user_function_call(
            user_function,
            this_expression,
            receiver_updated_via_parameter_writeback,
            &prepared_capture_bindings,
            updated_bindings,
            additional_call_effect_nonlocal_bindings,
            assigned_nonlocal_binding_results,
            saved_new_target_local,
            saved_this_local,
            saved_this_shadow_owner.as_deref(),
            return_value_local,
            &bound_argument_expressions,
        )?;
        self.sync_static_argument_object_member_writeback_values(
            &static_argument_member_writebacks,
        );
        Ok(())
    }
}
