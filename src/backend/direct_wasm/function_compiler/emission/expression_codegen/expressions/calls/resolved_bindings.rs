use super::*;

impl<'a> FunctionCompiler<'a> {
    pub(in crate::backend::direct_wasm) fn private_member_call_requires_runtime_brand_check(
        &self,
        object: &Expression,
        property: &Expression,
    ) -> bool {
        let property = self
            .resolve_property_key_expression(property)
            .unwrap_or_else(|| self.materialize_static_expression(property));
        is_private_property_name_expression(&property)
            && (matches!(object, Expression::This | Expression::Identifier(_))
                || self
                    .resolve_bound_alias_expression(object)
                    .is_some_and(|resolved| {
                        !static_expression_matches(&resolved, object)
                            && matches!(resolved, Expression::This)
                    })
                || self.expression_uses_runtime_dynamic_binding(object))
    }

    fn emit_private_member_call_brand_check(
        &mut self,
        callee: &Expression,
        object: &Expression,
        property: &Expression,
    ) -> DirectResult<()> {
        if !self.private_member_call_requires_runtime_brand_check(object, property) {
            return Ok(());
        }
        self.emit_numeric_expression(callee)?;
        self.state.emission.output.instructions.push(0x1a);
        Ok(())
    }

    pub(in crate::backend::direct_wasm) fn emit_resolved_function_binding_call_expression(
        &mut self,
        source_expression: &Expression,
        callee: &Expression,
        arguments: &[CallArgument],
    ) -> DirectResult<bool> {
        let Some(function_binding) = self.resolve_function_binding_from_expression(callee) else {
            return Ok(false);
        };
        if let Expression::Member { object, property } = callee
            && !inline_summary_side_effect_free_expression(property)
        {
            if !inline_summary_side_effect_free_expression(object) {
                return Ok(false);
            }
            self.emit_property_key_expression_effects(property)?;
        }
        match function_binding {
            LocalFunctionBinding::User(function_name) => {
                let Some(user_function) = self.user_function(&function_name).cloned() else {
                    return Ok(false);
                };
                if let Expression::Member { object, property } = callee {
                    self.emit_private_member_call_brand_check(callee, object, property)?;
                    let runtime_fallback = self
                        .promise_member_call_requires_runtime_fallback(object, property, arguments);
                    let materialized_this_expression = self.materialize_static_expression(object);
                    let materialized_call_arguments = arguments
                        .iter()
                        .map(|argument| match argument {
                            CallArgument::Expression(expression)
                            | CallArgument::Spread(expression) => {
                                self.materialize_static_expression(expression)
                            }
                        })
                        .collect::<Vec<_>>();
                    if let Some(capture_slots) =
                        self.resolve_member_function_capture_slots(object, property)
                    {
                        if runtime_fallback {
                            self.emit_user_function_call_with_new_target_and_this_expression_and_bound_captures_without_static_snapshot(
                                &user_function,
                                arguments,
                                JS_UNDEFINED_TAG,
                                object,
                                &capture_slots,
                            )?;
                        } else {
                            self.emit_user_function_call_with_new_target_and_this_expression_and_bound_captures(
                                &user_function,
                                arguments,
                                JS_UNDEFINED_TAG,
                                object,
                                &capture_slots,
                            )?;
                        }
                    } else {
                        let expression_capture_slots =
                            self.resolve_function_expression_capture_slots(callee);
                        if !runtime_fallback
                            && self.can_inline_user_function_call_with_explicit_call_frame(
                                &user_function,
                                &materialized_call_arguments,
                                &materialized_this_expression,
                            )
                        {
                            let result_local = self.allocate_temp_local();
                            if self.emit_inline_user_function_summary_with_explicit_call_frame(
                                &user_function,
                                &materialized_call_arguments,
                                &materialized_this_expression,
                                result_local,
                            )? {
                                self.push_local_get(result_local);
                                return Ok(true);
                            }
                        }
                        if let Some(capture_slots) = expression_capture_slots.as_ref() {
                            if runtime_fallback {
                                self.emit_user_function_call_with_new_target_and_this_expression_and_bound_captures_without_static_snapshot(
                                    &user_function,
                                    arguments,
                                    JS_UNDEFINED_TAG,
                                    object,
                                    capture_slots,
                                )?;
                            } else {
                                self.emit_user_function_call_with_function_this_binding(
                                    &user_function,
                                    arguments,
                                    object,
                                    Some(capture_slots),
                                )?;
                            }
                        } else if runtime_fallback {
                            self.emit_user_function_call_with_new_target_and_this_expression_without_static_snapshot(
                                &user_function,
                                arguments,
                                JS_UNDEFINED_TAG,
                                object,
                            )?;
                        } else {
                            self.emit_user_function_call_with_function_this_binding(
                                &user_function,
                                arguments,
                                object,
                                None,
                            )?;
                        }
                    }
                    self.note_last_bound_user_function_source_expression(source_expression);
                } else if matches!(callee, Expression::SuperMember { .. }) {
                    self.emit_user_function_call_with_new_target_and_this_expression(
                        &user_function,
                        arguments,
                        JS_UNDEFINED_TAG,
                        &Expression::This,
                    )?;
                    self.note_last_bound_user_function_source_expression(source_expression);
                } else {
                    let callee_is_returning_call =
                        matches!(callee, Expression::Call { .. } | Expression::New { .. });
                    let initialized_capture_slots = if callee_is_returning_call {
                        self.initialize_user_function_capture_slots_from_expression(
                            callee,
                            &user_function,
                        )?
                    } else {
                        self.resolve_function_expression_capture_slots(callee)
                    };
                    if callee_is_returning_call
                        && initialized_capture_slots.is_none()
                        && self
                            .user_function_capture_bindings(&user_function.name)
                            .is_some_and(|captures| !captures.is_empty())
                    {
                        return Ok(false);
                    }
                    if let Some(capture_slots) = initialized_capture_slots.as_ref() {
                        self.emit_user_function_call_with_function_this_binding(
                            &user_function,
                            arguments,
                            &Expression::Undefined,
                            Some(capture_slots),
                        )?;
                    } else {
                        self.emit_user_function_call(&user_function, arguments)?;
                    }
                    self.note_last_bound_user_function_source_expression(source_expression);
                }
                Ok(true)
            }
            LocalFunctionBinding::Builtin(function_name) => {
                if self.emit_builtin_call_for_callee(callee, &function_name, arguments, false)? {
                    return Ok(true);
                }
                Ok(false)
            }
        }
    }

    pub(in crate::backend::direct_wasm) fn emit_member_function_binding_call_expression(
        &mut self,
        callee: &Expression,
        object: &Expression,
        property: &Expression,
        arguments: &[CallArgument],
    ) -> DirectResult<bool> {
        let Some(function_binding) = self.resolve_member_function_binding(object, property) else {
            return Ok(false);
        };
        if !inline_summary_side_effect_free_expression(property) {
            if !inline_summary_side_effect_free_expression(object) {
                return Ok(false);
            }
            self.emit_property_key_expression_effects(property)?;
        }
        match function_binding {
            LocalFunctionBinding::User(function_name) => {
                let Some(user_function) = self.user_function(&function_name).cloned() else {
                    return Ok(false);
                };
                let runtime_fallback =
                    self.promise_member_call_requires_runtime_fallback(object, property, arguments);
                self.emit_private_member_call_brand_check(callee, object, property)?;
                let materialized_this_expression = self.materialize_static_expression(object);
                let materialized_call_arguments = arguments
                    .iter()
                    .map(|argument| match argument {
                        CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                            self.materialize_static_expression(expression)
                        }
                    })
                    .collect::<Vec<_>>();
                if let Some(capture_slots) =
                    self.resolve_member_function_capture_slots(object, property)
                {
                    if runtime_fallback {
                        self.emit_user_function_call_with_new_target_and_this_expression_and_bound_captures_without_static_snapshot(
                            &user_function,
                            arguments,
                            JS_UNDEFINED_TAG,
                            object,
                            &capture_slots,
                        )?;
                    } else {
                        self.emit_user_function_call_with_new_target_and_this_expression_and_bound_captures(
                            &user_function,
                            arguments,
                            JS_UNDEFINED_TAG,
                            object,
                            &capture_slots,
                        )?;
                    }
                } else {
                    let expression_capture_slots =
                        self.resolve_function_expression_capture_slots(callee);
                    if !runtime_fallback
                        && self.can_inline_user_function_call_with_explicit_call_frame(
                            &user_function,
                            &materialized_call_arguments,
                            &materialized_this_expression,
                        )
                    {
                        let result_local = self.allocate_temp_local();
                        if self.emit_inline_user_function_summary_with_explicit_call_frame(
                            &user_function,
                            &materialized_call_arguments,
                            &materialized_this_expression,
                            result_local,
                        )? {
                            self.push_local_get(result_local);
                            return Ok(true);
                        }
                    }
                    if let Some(capture_slots) = expression_capture_slots.as_ref() {
                        if runtime_fallback {
                            self.emit_user_function_call_with_new_target_and_this_expression_and_bound_captures_without_static_snapshot(
                                &user_function,
                                arguments,
                                JS_UNDEFINED_TAG,
                                object,
                                capture_slots,
                            )?;
                        } else {
                            self.emit_user_function_call_with_function_this_binding(
                                &user_function,
                                arguments,
                                object,
                                Some(capture_slots),
                            )?;
                        }
                    } else if runtime_fallback {
                        self.emit_user_function_call_with_new_target_and_this_expression_without_static_snapshot(
                            &user_function,
                            arguments,
                            JS_UNDEFINED_TAG,
                            object,
                        )?;
                    } else {
                        self.emit_user_function_call_with_function_this_binding(
                            &user_function,
                            arguments,
                            object,
                            None,
                        )?;
                    }
                }
                self.note_last_bound_user_function_source_expression(&Expression::Call {
                    callee: Box::new(callee.clone()),
                    arguments: arguments.to_vec(),
                });
                Ok(true)
            }
            LocalFunctionBinding::Builtin(function_name) => {
                if self.emit_builtin_call_for_callee(callee, &function_name, arguments, false)? {
                    return Ok(true);
                }
                Ok(false)
            }
        }
    }
}
