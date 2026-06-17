use super::*;

impl<'a> FunctionCompiler<'a> {
    pub(in crate::backend::direct_wasm) fn resolve_fast_static_number_expression(
        &self,
        expression: &Expression,
        depth: usize,
    ) -> Option<f64> {
        if depth > 12 {
            return None;
        }
        match expression {
            Expression::Number(value) => Some(*value),
            Expression::Identifier(name) => {
                let resolved_name = self
                    .resolve_current_local_binding(name)
                    .map(|(resolved_name, _)| resolved_name);
                let value = resolved_name
                    .as_deref()
                    .and_then(|resolved_name| {
                        self.state
                            .speculation
                            .static_semantics
                            .local_value_binding(resolved_name)
                    })
                    .or_else(|| {
                        self.state
                            .speculation
                            .static_semantics
                            .local_value_binding(name)
                    })
                    .or_else(|| self.global_value_binding(name))?;
                if static_expression_matches(value, expression) {
                    return None;
                }
                self.resolve_fast_static_number_expression(value, depth + 1)
            }
            Expression::Binary { op, left, right } => {
                let left = self.resolve_fast_static_number_expression(left, depth + 1)?;
                let right = self.resolve_fast_static_number_expression(right, depth + 1)?;
                match op {
                    BinaryOp::Add => Some(left + right),
                    BinaryOp::Subtract => Some(left - right),
                    BinaryOp::Multiply => Some(left * right),
                    BinaryOp::Divide => Some(left / right),
                    BinaryOp::Modulo => Some(left % right),
                    BinaryOp::Exponentiate => Some(left.powf(right)),
                    _ => None,
                }
            }
            Expression::Unary {
                op: UnaryOp::Negate,
                expression,
            } => self
                .resolve_fast_static_number_expression(expression, depth + 1)
                .map(|value| -value),
            Expression::Unary {
                op: UnaryOp::Plus,
                expression,
            } => self.resolve_fast_static_number_expression(expression, depth + 1),
            Expression::Member { object, property } => {
                let object_binding = self.resolve_object_binding_from_expression(object)?;
                let canonical_property = self.canonical_object_property_expression(property);
                let value = object_binding_lookup_value(&object_binding, &canonical_property)
                    .or_else(|| object_binding_lookup_value(&object_binding, property))?;
                self.resolve_fast_static_number_expression(value, depth + 1)
            }
            Expression::Call { callee, arguments } => {
                let return_value =
                    self.resolve_effectful_call_return_metadata_value(callee, arguments)?;
                if static_expression_matches(&return_value, expression) {
                    return None;
                }
                self.resolve_fast_static_number_expression(&return_value, depth + 1)
            }
            Expression::Sequence(expressions) => {
                self.resolve_fast_static_number_expression(expressions.last()?, depth + 1)
            }
            _ => None,
        }
    }

    pub(in crate::backend::direct_wasm) fn resolve_fast_static_user_function_call_number(
        &self,
        user_function: &UserFunction,
        arguments: &[CallArgument],
        this_binding: &Expression,
    ) -> Option<f64> {
        let trace = crate::ayy_env_flag!("AYY_TRACE_FAST_METHOD_NUMBER");
        if user_function.is_async()
            || user_function.is_generator()
            || user_function.has_parameter_defaults()
            || user_function.has_lowered_pattern_parameters()
            || self.user_function_mentions_private_member_access(user_function)
            || self.user_function_mentions_direct_eval(user_function)
            || arguments
                .iter()
                .any(|argument| matches!(argument, CallArgument::Spread(_)))
        {
            if trace {
                eprintln!(
                    "fast_method_number:reject_shape function={} async={} generator={} defaults={} lowered={} private={} eval={} spread={}",
                    user_function.name,
                    user_function.is_async(),
                    user_function.is_generator(),
                    user_function.has_parameter_defaults(),
                    user_function.has_lowered_pattern_parameters(),
                    self.user_function_mentions_private_member_access(user_function),
                    self.user_function_mentions_direct_eval(user_function),
                    arguments
                        .iter()
                        .any(|argument| matches!(argument, CallArgument::Spread(_)))
                );
            }
            return None;
        }
        let return_value =
            self.fast_static_user_function_return_expression(user_function, trace)?;
        let expanded_arguments = self.expand_call_arguments(arguments);
        let arguments_binding = Expression::Array(
            expanded_arguments
                .iter()
                .cloned()
                .map(ArrayElement::Expression)
                .collect(),
        );
        let substituted = self.substitute_user_function_call_frame_bindings(
            &return_value,
            user_function,
            arguments,
            this_binding,
            &arguments_binding,
        );
        let number = self.resolve_fast_static_number_expression(&substituted, 0);
        if trace {
            eprintln!(
                "fast_method_number:result function={} this={:?} return={:?} substituted={:?} number={:?}",
                user_function.name, this_binding, return_value, substituted, number
            );
        }
        number
    }

    fn fast_static_user_function_return_expression(
        &self,
        user_function: &UserFunction,
        trace: bool,
    ) -> Option<Expression> {
        if let Some(summary) = user_function.inline_summary.as_ref()
            && summary.effects.is_empty()
            && let Some(return_value) = summary.return_value.as_ref()
        {
            return Some(return_value.clone());
        }

        let Some(function) = self.resolve_registered_function_declaration(&user_function.name)
        else {
            if trace {
                eprintln!(
                    "fast_method_number:reject_missing_declaration function={}",
                    user_function.name
                );
            }
            return None;
        };
        let [Statement::Return(return_value)] = function.body.as_slice() else {
            if trace {
                eprintln!(
                    "fast_method_number:reject_body function={} statement_count={}",
                    user_function.name,
                    function.body.len()
                );
            }
            return None;
        };
        if !inline_summary_side_effect_free_expression(return_value) {
            if trace {
                eprintln!(
                    "fast_method_number:reject_return_effects function={} return={:?}",
                    user_function.name, return_value
                );
            }
            return None;
        }
        Some(return_value.clone())
    }

    fn home_object_expression_for_user_function(
        user_function: &UserFunction,
    ) -> Option<Expression> {
        let home_object = user_function.home_object_binding.as_ref()?;
        if let Some(class_name) = home_object.strip_suffix(".prototype") {
            return Some(Expression::Member {
                object: Box::new(Expression::Identifier(class_name.to_string())),
                property: Box::new(Expression::String("prototype".to_string())),
            });
        }
        Some(Expression::Identifier(home_object.clone()))
    }

    fn resolve_member_call_capture_slots_for_user_function(
        &self,
        user_function: &UserFunction,
        object: &Expression,
        property: &Expression,
    ) -> Option<BTreeMap<String, String>> {
        self.resolve_member_function_capture_slots(object, property)
            .or_else(|| {
                let home_object = Self::home_object_expression_for_user_function(user_function)?;
                self.resolve_member_function_capture_slots(&home_object, property)
            })
    }

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
                    if let Some(capture_slots) = self
                        .resolve_member_call_capture_slots_for_user_function(
                            &user_function,
                            object,
                            property,
                        )
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

    pub(in crate::backend::direct_wasm) fn emit_returned_function_value_call_expression(
        &mut self,
        callee: &Expression,
        arguments: &[CallArgument],
    ) -> DirectResult<bool> {
        let (returned_callee, returned_arguments) = match callee {
            Expression::Call { callee, arguments } | Expression::New { callee, arguments } => {
                (callee.as_ref(), arguments.as_slice())
            }
            _ => return Ok(false),
        };
        let Some(LocalFunctionBinding::User(function_name)) =
            self.resolve_returned_function_binding_from_call(returned_callee, returned_arguments)
        else {
            return Ok(false);
        };
        let Some(user_function) = self.user_function(&function_name).cloned() else {
            return Ok(false);
        };

        if !self.emit_returned_function_value_call_side_effects(callee)? {
            self.emit_numeric_expression(callee)?;
            self.state.emission.output.instructions.push(0x1a);
        }
        self.emit_user_function_call(&user_function, arguments)?;
        Ok(true)
    }

    pub(in crate::backend::direct_wasm) fn emit_returned_function_value_call_side_effects(
        &mut self,
        call_expression: &Expression,
    ) -> DirectResult<bool> {
        let (callee, arguments) = match call_expression {
            Expression::Call { callee, arguments } | Expression::New { callee, arguments } => {
                (callee.as_ref(), arguments.as_slice())
            }
            _ => return Ok(false),
        };

        if !matches!(callee, Expression::Call { .. } | Expression::New { .. }) {
            self.emit_numeric_expression(call_expression)?;
            self.state.emission.output.instructions.push(0x1a);
            return Ok(true);
        }

        let Some(LocalFunctionBinding::User(function_name)) =
            self.resolve_returned_function_binding_from_call(callee, arguments)
        else {
            return Ok(false);
        };
        let Some(user_function) = self.user_function(&function_name).cloned() else {
            return Ok(false);
        };

        if !self.emit_returned_function_value_call_side_effects(callee)? {
            self.emit_numeric_expression(callee)?;
            self.state.emission.output.instructions.push(0x1a);
        }
        self.emit_user_function_call(&user_function, arguments)?;
        self.state.emission.output.instructions.push(0x1a);
        Ok(true)
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
                let materialized_this_expression = self.materialize_static_expression(object);
                let materialized_call_arguments = arguments
                    .iter()
                    .map(|argument| match argument {
                        CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                            self.materialize_static_expression(expression)
                        }
                    })
                    .collect::<Vec<_>>();
                if self
                    .resolve_object_binding_from_expression(object)
                    .is_some()
                    && inline_summary_side_effect_free_expression(object)
                    && inline_summary_side_effect_free_expression(property)
                    && arguments.iter().all(|argument| {
                        inline_summary_side_effect_free_expression(argument.expression())
                    })
                    && !self.user_function_mentions_private_member_access(&user_function)
                {
                    if let Some(number) = self.resolve_fast_static_user_function_call_number(
                        &user_function,
                        arguments,
                        &materialized_this_expression,
                    ) {
                        self.emit_numeric_expression(&Expression::Number(number))?;
                        self.note_last_bound_user_function_source_expression(&Expression::Call {
                            callee: Box::new(callee.clone()),
                            arguments: arguments.to_vec(),
                        });
                        return Ok(true);
                    }
                    let static_function_binding = LocalFunctionBinding::User(function_name.clone());
                    if let Some(return_value) = self
                        .resolve_function_binding_static_return_expression_with_call_frame(
                            &static_function_binding,
                            &materialized_call_arguments,
                            &materialized_this_expression,
                        )
                        && let Some(number) =
                            self.resolve_fast_static_number_expression(&return_value, 0)
                    {
                        self.emit_numeric_expression(&Expression::Number(number))?;
                        self.note_last_bound_user_function_source_expression(&Expression::Call {
                            callee: Box::new(callee.clone()),
                            arguments: arguments.to_vec(),
                        });
                        return Ok(true);
                    }
                }
                let runtime_fallback =
                    self.promise_member_call_requires_runtime_fallback(object, property, arguments);
                self.emit_private_member_call_brand_check(callee, object, property)?;
                if let Some(capture_slots) = self
                    .resolve_member_call_capture_slots_for_user_function(
                        &user_function,
                        object,
                        property,
                    )
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
                if matches!(
                    function_name.as_str(),
                    "Object.prototype.hasOwnProperty" | "Object.prototype.propertyIsEnumerable"
                ) {
                    let mut bound_arguments = Vec::with_capacity(arguments.len().saturating_add(1));
                    bound_arguments.push(CallArgument::Expression(object.clone()));
                    bound_arguments.extend(arguments.iter().cloned());
                    if self.emit_bound_function_prototype_call_builtin(
                        &function_name,
                        &bound_arguments,
                    )? {
                        return Ok(true);
                    }
                }
                if self.emit_builtin_call_for_callee(callee, &function_name, arguments, false)? {
                    return Ok(true);
                }
                Ok(false)
            }
        }
    }
}
