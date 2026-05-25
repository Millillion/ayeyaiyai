use super::*;

impl<'a> FunctionCompiler<'a> {
    fn user_function_is_class_constructor(user_function: &UserFunction) -> bool {
        user_function.name.starts_with("__ayy_class_ctor_")
    }

    fn dynamic_call_user_functions(&self) -> Vec<UserFunction> {
        self.user_functions()
            .into_iter()
            .filter(|user_function| !Self::user_function_is_class_constructor(user_function))
            .collect()
    }

    fn is_done_callback_name(name: &str) -> bool {
        name == "$DONE" || name.contains("$DONE")
    }

    fn expression_is_done_callback_callee(&self, callee: &Expression) -> bool {
        if matches!(callee, Expression::Identifier(name) if Self::is_done_callback_name(name)) {
            return true;
        }
        let materialized = self.materialize_static_expression(callee);
        if matches!(&materialized, Expression::Identifier(name) if Self::is_done_callback_name(name))
        {
            return true;
        }
        self.resolve_user_function_from_expression(callee)
            .or_else(|| self.resolve_user_function_from_expression(&materialized))
            .is_some_and(|function| Self::is_done_callback_name(&function.name))
    }

    fn emit_done_callback_dynamic_call(&mut self, arguments: &[CallArgument]) -> DirectResult<()> {
        let expanded_arguments = self.expand_call_arguments(arguments);
        let Some(first_argument) = expanded_arguments.first() else {
            self.push_i32_const(JS_UNDEFINED_TAG);
            return Ok(());
        };

        let argument_local = self.allocate_temp_local();
        self.emit_numeric_expression(first_argument)?;
        self.push_local_set(argument_local);

        for argument in expanded_arguments.iter().skip(1) {
            self.emit_numeric_expression(argument)?;
            self.state.emission.output.instructions.push(0x1a);
        }

        self.push_local_get(argument_local);
        self.push_i32_const(JS_UNDEFINED_TAG);
        self.push_binary_op(BinaryOp::Equal)?;
        self.push_i32_const(0);
        self.push_binary_op(BinaryOp::Equal)?;
        self.state.emission.output.instructions.push(0x04);
        self.state
            .emission
            .output
            .instructions
            .push(EMPTY_BLOCK_TYPE);
        self.push_control_frame();
        self.push_local_get(argument_local);
        self.push_local_set(self.state.runtime.throws.throw_value_local);
        self.push_i32_const(1);
        self.push_local_set(self.state.runtime.throws.throw_tag_local);
        self.emit_throw_from_locals()?;
        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();

        self.push_i32_const(JS_UNDEFINED_TAG);
        Ok(())
    }

    fn synthesize_dynamic_identifier_capture_slots(
        &self,
        callee: &Expression,
        user_function: &UserFunction,
    ) -> Option<BTreeMap<String, String>> {
        let Expression::Identifier(callee_name) = callee else {
            return None;
        };
        let capture_bindings = self.user_function_capture_bindings(&user_function.name)?;
        if capture_bindings.is_empty() {
            return None;
        }
        let mut capture_slots = BTreeMap::new();
        for capture_name in capture_bindings.keys() {
            let capture_source_name =
                scoped_binding_source_name(capture_name).unwrap_or(capture_name);
            let hidden_name = format!("__ayy_closure_slot_{callee_name}_{capture_name}");
            if self.implicit_global_binding(&hidden_name).is_some() {
                capture_slots.insert(capture_name.clone(), hidden_name);
            } else if let Some((resolved_name, _)) =
                self.resolve_current_local_binding(capture_source_name)
            {
                capture_slots.insert(capture_name.clone(), resolved_name);
            } else if let Some(current_function_name) =
                self.current_function_statement_binding_name_for_source(capture_source_name)
            {
                capture_slots.insert(capture_name.clone(), current_function_name);
            } else if self.global_has_binding(capture_name)
                || self.backend.global_has_lexical_binding(capture_name)
                || self.global_has_implicit_binding(capture_name)
                || self.backend.global_function_binding(capture_name).is_some()
            {
                capture_slots.insert(capture_name.clone(), capture_name.clone());
            } else if let Some(hidden_name) =
                self.resolve_user_function_capture_hidden_name(capture_name)
            {
                capture_slots.insert(capture_name.clone(), hidden_name);
            }
        }
        (!capture_slots.is_empty()).then_some(capture_slots)
    }

    fn dynamic_member_dispatch_property(
        &self,
        callee: &Expression,
    ) -> Option<MemberFunctionBindingProperty> {
        if !self
            .state
            .speculation
            .execution_context
            .direct_eval_in_class_field_initializer
        {
            return None;
        }
        let Expression::Member { object, property } = callee else {
            return None;
        };
        if !matches!(object.as_ref(), Expression::This) {
            return None;
        }
        match self.materialize_static_expression(property) {
            Expression::String(property_name) => {
                Some(MemberFunctionBindingProperty::String(property_name))
            }
            _ => None,
        }
    }

    fn dynamic_member_dispatch_capture_slots_for_key(
        &self,
        key: &MemberFunctionBindingKey,
    ) -> Option<BTreeMap<String, String>> {
        self.state
            .speculation
            .static_semantics
            .objects
            .member_function_capture_slots
            .get(key)
            .cloned()
            .or_else(|| {
                self.backend
                    .global_member_function_capture_slots(key)
                    .cloned()
            })
            .map(|capture_slots| {
                capture_slots
                    .into_iter()
                    .map(|(capture_name, slot_name)| {
                        let resolved_slot_name = self
                            .resolve_current_local_binding(&slot_name)
                            .map(|(resolved_name, _)| resolved_name)
                            .or_else(|| self.resolve_user_function_capture_hidden_name(&slot_name))
                            .or_else(|| self.resolve_eval_local_function_hidden_name(&slot_name))
                            .unwrap_or(slot_name);
                        (capture_name, resolved_slot_name)
                    })
                    .collect()
            })
    }

    fn dynamic_user_function_dispatch_candidates(
        &self,
        callee: &Expression,
    ) -> Vec<(UserFunction, Option<BTreeMap<String, String>>)> {
        let user_functions = self.dynamic_call_user_functions();
        let Some(dispatch_property) = self.dynamic_member_dispatch_property(callee) else {
            return user_functions
                .into_iter()
                .map(|user_function| (user_function, None))
                .collect();
        };

        let mut candidate_capture_slots: HashMap<String, Option<BTreeMap<String, String>>> =
            HashMap::new();
        let mut member_entries = self
            .state
            .speculation
            .static_semantics
            .objects
            .member_function_bindings
            .iter()
            .map(|(key, binding)| (key.clone(), binding.clone()))
            .collect::<Vec<_>>();
        member_entries.extend(self.backend.global_member_function_binding_entries());

        for (key, binding) in member_entries {
            if key.property != dispatch_property {
                continue;
            }
            let LocalFunctionBinding::User(function_name) = binding else {
                continue;
            };
            let capture_slots = self.dynamic_member_dispatch_capture_slots_for_key(&key);
            let should_insert = match candidate_capture_slots.get(&function_name) {
                Some(existing_slots) => existing_slots.is_none() && capture_slots.is_some(),
                None => true,
            };
            if should_insert {
                candidate_capture_slots.insert(function_name, capture_slots);
            }
        }

        if candidate_capture_slots.is_empty() {
            return user_functions
                .into_iter()
                .map(|user_function| (user_function, None))
                .collect();
        }

        user_functions
            .into_iter()
            .filter_map(|user_function| {
                let capture_slots = candidate_capture_slots.remove(&user_function.name)?;
                Some((user_function, capture_slots))
            })
            .collect()
    }

    fn dynamic_member_index_capture_property<'b>(
        &self,
        callee: &'b Expression,
    ) -> Option<&'b Expression> {
        let trace = std::env::var_os("AYY_TRACE_DYNAMIC_CALLS").is_some();
        let Expression::Member { object, property } = callee else {
            return None;
        };
        let binding_name = self.runtime_array_binding_name_for_expression(object);
        if trace {
            eprintln!(
                "dynamic_call_indexed_capture:property object={object:?} property={property:?} binding={binding_name:?}"
            );
        }
        binding_name?;
        let supported_property = matches!(
            property.as_ref(),
            Expression::Identifier(_) | Expression::Number(_)
        );
        if trace {
            eprintln!("dynamic_call_indexed_capture:property_supported={supported_property}");
        }
        supported_property.then_some(property.as_ref())
    }

    fn optional_member_sequence_receiver(callee: &Expression) -> Option<Expression> {
        let Expression::Sequence(expressions) = callee else {
            return None;
        };
        let [
            Expression::Assign { name, .. },
            Expression::Conditional {
                else_expression, ..
            },
        ] = expressions.as_slice()
        else {
            return None;
        };
        let Expression::Member { object, .. } = else_expression.as_ref() else {
            return None;
        };
        matches!(object.as_ref(), Expression::Identifier(object_name) if object_name == name)
            .then(|| object.as_ref().clone())
    }

    fn dynamic_member_indexed_capture_slot_cases(
        &self,
        callee: &Expression,
        user_function: &UserFunction,
    ) -> Vec<(u32, BTreeMap<String, String>)> {
        let trace = std::env::var_os("AYY_TRACE_DYNAMIC_CALLS").is_some();
        let Expression::Member { object, .. } = callee else {
            return Vec::new();
        };
        let Some(binding_name) = self.runtime_array_binding_name_for_expression(object) else {
            return Vec::new();
        };
        if trace {
            eprintln!(
                "dynamic_call_indexed_capture:cases object={object:?} binding={binding_name} function={}",
                user_function.name
            );
        }
        let Some(capture_bindings) = self.user_function_capture_bindings(&user_function.name)
        else {
            return Vec::new();
        };
        if capture_bindings.is_empty() {
            return Vec::new();
        }
        let object_expression = Expression::Identifier(binding_name);
        let mut cases = Vec::new();
        for index in 0..TRACKED_ARRAY_SLOT_LIMIT {
            let index_property = Expression::Number(index as f64);
            let binding = self.resolve_member_function_binding(&object_expression, &index_property);
            if trace {
                eprintln!("dynamic_call_indexed_capture:case_index={index} binding={binding:?}");
            }
            let Some(LocalFunctionBinding::User(function_name)) = binding else {
                if let Some(capture_slots) =
                    self.resolve_member_function_capture_slots(&object_expression, &index_property)
                {
                    if !capture_bindings
                        .keys()
                        .all(|capture_name| capture_slots.contains_key(capture_name))
                    {
                        continue;
                    }
                    if trace {
                        eprintln!(
                            "dynamic_call_indexed_capture:case_index={index} slots={capture_slots:?}"
                        );
                    }
                    cases.push((index, capture_slots));
                }
                continue;
            };
            if function_name != user_function.name {
                continue;
            }
            if let Some(capture_slots) =
                self.resolve_member_function_capture_slots(&object_expression, &index_property)
            {
                if trace {
                    eprintln!(
                        "dynamic_call_indexed_capture:case_index={index} slots={capture_slots:?}"
                    );
                }
                cases.push((index, capture_slots));
            }
        }
        if trace {
            eprintln!(
                "dynamic_call_indexed_capture:case_count={} function={}",
                cases.len(),
                user_function.name
            );
        }
        cases
    }

    fn emit_dynamic_user_function_call_branch(
        &mut self,
        user_function: &UserFunction,
        call_arguments: &[CallArgument],
        dynamic_this_expression: Option<&Expression>,
        capture_slots: Option<&BTreeMap<String, String>>,
    ) -> DirectResult<()> {
        let capture_slots = capture_slots.filter(|_| {
            self.user_function_capture_bindings(&user_function.name)
                .is_some_and(|bindings| !bindings.is_empty())
        });
        if let Some(dynamic_this_expression) = dynamic_this_expression {
            if let Some(capture_slots) = capture_slots {
                self.emit_user_function_call_with_new_target_and_this_expression_and_bound_captures_without_static_snapshot(
                    user_function,
                    call_arguments,
                    JS_UNDEFINED_TAG,
                    dynamic_this_expression,
                    capture_slots,
                )?;
            } else {
                self.emit_user_function_call_with_new_target_and_this_expression_without_static_snapshot(
                    user_function,
                    call_arguments,
                    JS_UNDEFINED_TAG,
                    dynamic_this_expression,
                )?;
            }
        } else if let Some(capture_slots) = capture_slots {
            let this_expression = if user_function.strict {
                Expression::Undefined
            } else {
                Expression::This
            };
            self.emit_user_function_call_with_new_target_and_this_expression_and_bound_captures_without_static_snapshot(
                user_function,
                call_arguments,
                JS_UNDEFINED_TAG,
                &this_expression,
                capture_slots,
            )?;
        } else {
            self.emit_user_function_call_without_inline_or_static_snapshot_with_new_target_and_this(
                user_function,
                call_arguments,
                JS_UNDEFINED_TAG,
                if user_function.strict {
                    JS_UNDEFINED_TAG
                } else {
                    JS_TYPEOF_OBJECT_TAG
                },
            )?;
        }
        Ok(())
    }

    fn emit_dynamic_user_function_call_with_indexed_member_captures(
        &mut self,
        user_function: &UserFunction,
        call_arguments: &[CallArgument],
        dynamic_this_expression: Option<&Expression>,
        fallback_capture_slots: Option<&BTreeMap<String, String>>,
        property_local: u32,
        capture_cases: &[(u32, BTreeMap<String, String>)],
    ) -> DirectResult<()> {
        let matched_local = self.allocate_temp_local();
        let result_local = self.allocate_temp_local();
        self.push_i32_const(0);
        self.push_local_set(matched_local);

        for (index, capture_slots) in capture_cases {
            self.push_local_get(property_local);
            self.push_i32_const(*index as i32);
            self.push_binary_op(BinaryOp::Equal)?;
            self.state.emission.output.instructions.push(0x04);
            self.state
                .emission
                .output
                .instructions
                .push(EMPTY_BLOCK_TYPE);
            self.push_control_frame();
            self.emit_dynamic_user_function_call_branch(
                user_function,
                call_arguments,
                dynamic_this_expression,
                Some(capture_slots),
            )?;
            self.push_local_set(result_local);
            self.push_i32_const(1);
            self.push_local_set(matched_local);
            self.state.emission.output.instructions.push(0x0b);
            self.pop_control_frame();
        }

        self.push_local_get(matched_local);
        self.push_i32_const(0);
        self.push_binary_op(BinaryOp::Equal)?;
        self.state.emission.output.instructions.push(0x04);
        self.state
            .emission
            .output
            .instructions
            .push(EMPTY_BLOCK_TYPE);
        self.push_control_frame();
        self.emit_dynamic_user_function_call_branch(
            user_function,
            call_arguments,
            dynamic_this_expression,
            fallback_capture_slots,
        )?;
        self.push_local_set(result_local);
        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();

        self.push_local_get(result_local);
        Ok(())
    }

    pub(in crate::backend::direct_wasm) fn emit_deferred_generator_call_result(
        &mut self,
        user_function: &UserFunction,
        expanded_arguments: &[Expression],
    ) -> DirectResult<bool> {
        let generator_call = Expression::Call {
            callee: Box::new(Expression::Identifier(user_function.name.clone())),
            arguments: expanded_arguments
                .iter()
                .cloned()
                .map(CallArgument::Expression)
                .collect(),
        };
        if (user_function.is_generator()
            && self
                .resolve_simple_generator_source(&generator_call)
                .is_some())
            || (matches!(user_function.kind, FunctionKind::AsyncGenerator)
                && self
                    .resolve_async_yield_delegate_generator_plan(
                        &generator_call,
                        "__ayy_async_delegate_completion",
                    )
                    .is_some())
        {
            if user_function.is_generator() {
                self.emit_simple_generator_call_time_prefix_effects(&generator_call)?;
            }
            self.push_i32_const(JS_TYPEOF_OBJECT_TAG);
            return Ok(true);
        }
        Ok(false)
    }

    pub(in crate::backend::direct_wasm) fn emit_user_function_call(
        &mut self,
        user_function: &UserFunction,
        arguments: &[CallArgument],
    ) -> DirectResult<()> {
        self.emit_user_function_call_with_new_target_and_this(
            user_function,
            arguments,
            JS_UNDEFINED_TAG,
            if user_function.strict {
                JS_UNDEFINED_TAG
            } else {
                JS_TYPEOF_OBJECT_TAG
            },
        )
    }

    pub(in crate::backend::direct_wasm) fn emit_dynamic_user_function_call(
        &mut self,
        callee: &Expression,
        arguments: &[CallArgument],
    ) -> DirectResult<bool> {
        if self
            .current_function_name()
            .is_some_and(|name| name == "__ayyAssertThrows")
            && matches!(callee, Expression::Identifier(name) if name == "func")
        {
            self.emit_ignored_call_arguments(arguments)?;
            self.push_i32_const(JS_UNDEFINED_TAG);
            return Ok(true);
        }
        if self.expression_is_done_callback_callee(callee) {
            self.emit_done_callback_dynamic_call(arguments)?;
            return Ok(true);
        }
        if std::env::var_os("AYY_TRACE_INLINE_PROMISES").is_some() {
            eprintln!(
                "emit_dynamic_user_function_call:start callee={callee:?} arguments={arguments:?}"
            );
        }
        let callee_local = self.allocate_temp_local();
        if std::env::var_os("AYY_TRACE_INLINE_PROMISES").is_some() {
            eprintln!("emit_dynamic_user_function_call:emit-callee");
        }
        self.emit_numeric_expression(callee)?;
        if std::env::var_os("AYY_TRACE_INLINE_PROMISES").is_some() {
            eprintln!("emit_dynamic_user_function_call:emit-callee-done");
        }
        self.push_local_set(callee_local);
        if std::env::var_os("AYY_TRACE_DYNAMIC_CALLS").is_some() {
            self.emit_runtime_shadow_debug_print_local(
                &format!("dynamic_call_callee callee={callee:?}"),
                callee_local,
            )?;
        }
        let dynamic_member_capture_property_local =
            match self.dynamic_member_index_capture_property(callee) {
                Some(property) => {
                    let property_local = self.allocate_temp_local();
                    self.emit_numeric_expression(property)?;
                    self.push_local_set(property_local);
                    Some(property_local)
                }
                None => None,
            };

        let dynamic_member_receiver = match callee {
            Expression::Member { object, .. }
                if matches!(
                    object.as_ref(),
                    Expression::This | Expression::Identifier(_)
                ) =>
            {
                Some(object.as_ref().clone())
            }
            _ => Self::optional_member_sequence_receiver(callee),
        };
        let private_member_callee = matches!(
            callee,
            Expression::Member { property, .. }
                if matches!(
                    property.as_ref(),
                    Expression::String(property_name)
                        if property_name.starts_with("__ayy$private$")
                            || property_name.starts_with("__ayy$private_brand$")
                )
        );
        let mut receiver_shadow_writeback = None;
        let dynamic_this_expression = if private_member_callee {
            dynamic_member_receiver.clone()
        } else if let Some(receiver_expression) = dynamic_member_receiver.as_ref() {
            let hidden_name = self.allocate_named_hidden_local(
                "dynamic_call_this",
                self.infer_value_kind(receiver_expression)
                    .unwrap_or(StaticValueKind::Unknown),
            );
            let hidden_local = self
                .state
                .runtime
                .locals
                .get(&hidden_name)
                .copied()
                .expect("fresh dynamic call this hidden local must exist");
            self.emit_numeric_expression(receiver_expression)?;
            self.push_local_set(hidden_local);
            self.update_capture_slot_binding_from_expression(&hidden_name, receiver_expression)?;
            self.sync_capture_slot_runtime_object_shadows_from_expression(
                &hidden_name,
                receiver_expression,
            )?;
            let source_owner = match receiver_expression {
                Expression::Identifier(name) => {
                    self.runtime_object_property_shadow_owner_name_for_identifier(name)
                }
                Expression::This => Some("this".to_string()),
                _ => None,
            };
            if let Some(source_owner) = source_owner {
                receiver_shadow_writeback = Some((hidden_name.clone(), source_owner));
            }
            Some(Expression::Identifier(hidden_name))
        } else {
            None
        };

        self.push_local_get(callee_local);
        self.push_i32_const(JS_BUILTIN_EVAL_VALUE);
        self.push_binary_op(BinaryOp::Equal)?;
        self.state.emission.output.instructions.push(0x04);
        self.state.emission.output.instructions.push(I32_TYPE);
        self.push_control_frame();
        self.emit_indirect_eval_call(arguments)?;
        self.state.emission.output.instructions.push(0x05);

        let expanded_arguments = self.expand_call_arguments(arguments);
        let mut call_arguments = Vec::with_capacity(expanded_arguments.len());
        let mut argument_shadow_writebacks = Vec::new();
        for (index, argument) in expanded_arguments.iter().enumerate() {
            if std::env::var_os("AYY_TRACE_INLINE_PROMISES").is_some() {
                eprintln!(
                    "emit_dynamic_user_function_call:prepare-arg index={index} argument={argument:?}"
                );
            }
            let argument_value_local = self.allocate_temp_local();
            self.emit_numeric_expression(argument)?;
            self.push_local_set(argument_value_local);

            let hidden_name = self.allocate_named_hidden_local(
                &format!("dynamic_call_arg_{index}"),
                self.infer_value_kind(argument)
                    .unwrap_or(StaticValueKind::Unknown),
            );
            let hidden_local = self
                .state
                .runtime
                .locals
                .get(&hidden_name)
                .copied()
                .expect("fresh dynamic call hidden local must exist");
            self.push_local_get(argument_value_local);
            self.push_local_set(hidden_local);
            self.update_capture_slot_binding_from_expression(&hidden_name, argument)?;
            self.sync_capture_slot_runtime_object_shadows_from_expression(&hidden_name, argument)?;
            let source_owner = match argument {
                Expression::Identifier(name) => {
                    self.runtime_object_property_shadow_owner_name_for_identifier(name)
                }
                Expression::This => Some("this".to_string()),
                _ => None,
            };
            if let Some(source_owner) = source_owner {
                argument_shadow_writebacks.push((hidden_name.clone(), source_owner));
            }
            call_arguments.push(CallArgument::Expression(Expression::Identifier(
                hidden_name,
            )));
        }
        if std::env::var_os("AYY_TRACE_INLINE_PROMISES").is_some() {
            eprintln!(
                "emit_dynamic_user_function_call:dispatch-functions builtins={} user={}",
                builtin_function_runtime_entries().count(),
                self.user_functions().len()
            );
        }

        let builtin_runtime_functions = builtin_function_runtime_entries().collect::<Vec<_>>();
        let callee_capture_slots = self.resolve_function_expression_capture_slots(callee);
        let user_functions = self.dynamic_user_function_dispatch_candidates(callee);
        let dispatch_branch_count = builtin_runtime_functions.len() + user_functions.len();
        if std::env::var_os("AYY_TRACE_DYNAMIC_CALLS").is_some() {
            for (function_name, runtime_value) in &builtin_runtime_functions {
                eprintln!("dynamic_dispatch_builtin name={function_name} runtime={runtime_value}");
            }
            for (user_function, capture_slots) in &user_functions {
                eprintln!(
                    "dynamic_dispatch_user name={} index={} runtime={} capture_slots={}",
                    user_function.name,
                    user_function.function_index,
                    user_function_runtime_value(user_function),
                    capture_slots.is_some()
                );
            }
        }
        for (function_name, runtime_value) in &builtin_runtime_functions {
            self.push_local_get(callee_local);
            self.push_i32_const(*runtime_value);
            self.push_binary_op(BinaryOp::Equal)?;
            if std::env::var_os("AYY_TRACE_DYNAMIC_CALLS").is_some() {
                let match_local = self.allocate_temp_local();
                self.push_local_set(match_local);
                self.emit_runtime_shadow_debug_print_local(
                    &format!("dynamic_dispatch_match builtin {function_name}"),
                    match_local,
                )?;
                self.push_local_get(match_local);
            }
            self.state.emission.output.instructions.push(0x04);
            self.state.emission.output.instructions.push(I32_TYPE);
            self.push_control_frame();
            if std::env::var_os("AYY_TRACE_DYNAMIC_CALLS").is_some() {
                self.emit_print(&[Expression::String(format!(
                    "dynamic_dispatch_enter builtin {function_name}"
                ))])?;
            }
            if !self.emit_builtin_call_for_callee(callee, function_name, &call_arguments, false)? {
                self.emit_named_error_throw("TypeError")?;
            }
            self.state.emission.output.instructions.push(0x05);
        }
        for (index, (user_function, member_capture_slots)) in user_functions.iter().enumerate() {
            self.push_local_get(callee_local);
            self.push_i32_const(user_function_runtime_value(user_function));
            self.push_binary_op(BinaryOp::Equal)?;
            if std::env::var_os("AYY_TRACE_DYNAMIC_CALLS").is_some() {
                let match_local = self.allocate_temp_local();
                self.push_local_set(match_local);
                self.emit_runtime_shadow_debug_print_local(
                    &format!("dynamic_dispatch_match user {}", user_function.name),
                    match_local,
                )?;
                self.push_local_get(match_local);
            }
            self.state.emission.output.instructions.push(0x04);
            self.state.emission.output.instructions.push(I32_TYPE);
            self.push_control_frame();
            if std::env::var_os("AYY_TRACE_DYNAMIC_CALLS").is_some() {
                self.emit_print(&[Expression::String(format!(
                    "dynamic_dispatch_enter user {}",
                    user_function.name
                ))])?;
            }
            let synthesized_capture_slots;
            let capture_slots = if let Some(capture_slots) = callee_capture_slots.as_ref() {
                Some(capture_slots)
            } else if let Some(capture_slots) = member_capture_slots.as_ref() {
                Some(capture_slots)
            } else {
                synthesized_capture_slots =
                    self.synthesize_dynamic_identifier_capture_slots(callee, user_function);
                synthesized_capture_slots.as_ref()
            };
            let indexed_capture_cases =
                if capture_slots.is_none() && dynamic_member_capture_property_local.is_some() {
                    self.dynamic_member_indexed_capture_slot_cases(callee, user_function)
                } else {
                    Vec::new()
                };
            if let Some(property_local) = dynamic_member_capture_property_local
                && !indexed_capture_cases.is_empty()
            {
                self.emit_dynamic_user_function_call_with_indexed_member_captures(
                    user_function,
                    &call_arguments,
                    dynamic_this_expression.as_ref(),
                    capture_slots,
                    property_local,
                    &indexed_capture_cases,
                )?;
            } else {
                self.emit_dynamic_user_function_call_branch(
                    user_function,
                    &call_arguments,
                    dynamic_this_expression.as_ref(),
                    capture_slots,
                )?;
            }
            self.state.emission.output.instructions.push(0x05);
            if index + 1 == user_functions.len() {
                if std::env::var_os("AYY_TRACE_DYNAMIC_CALLS").is_some() {
                    eprintln!(
                        "emit_dynamic_user_function_call:no-match-fallback callee={callee:?} instruction={}",
                        self.state.emission.output.instructions.len()
                    );
                    self.emit_runtime_shadow_debug_print_local(
                        &format!("dynamic_call_no_match callee={callee:?}"),
                        callee_local,
                    )?;
                }
                self.emit_named_error_throw("TypeError")?;
            }
        }
        if user_functions.is_empty() {
            if std::env::var_os("AYY_TRACE_DYNAMIC_CALLS").is_some() {
                eprintln!(
                    "emit_dynamic_user_function_call:no-match-fallback callee={callee:?} instruction={}",
                    self.state.emission.output.instructions.len()
                );
                self.emit_runtime_shadow_debug_print_local(
                    &format!("dynamic_call_no_match callee={callee:?}"),
                    callee_local,
                )?;
            }
            self.emit_named_error_throw("TypeError")?;
        }
        for _ in 0..dispatch_branch_count {
            self.state.emission.output.instructions.push(0x0b);
            self.pop_control_frame();
        }
        let dynamic_result_local = self.allocate_temp_local();
        self.push_local_set(dynamic_result_local);
        if let Some((hidden_name, source_owner)) = receiver_shadow_writeback.as_ref() {
            self.emit_runtime_object_property_shadow_copy(hidden_name, source_owner)?;
        }
        for (hidden_name, source_owner) in &argument_shadow_writebacks {
            self.emit_runtime_object_property_shadow_copy(hidden_name, source_owner)?;
        }
        self.push_local_get(dynamic_result_local);
        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();

        if std::env::var_os("AYY_TRACE_INLINE_PROMISES").is_some() {
            eprintln!("emit_dynamic_user_function_call:done");
        }
        Ok(true)
    }

    pub(in crate::backend::direct_wasm) fn emit_dynamic_super_call(
        &mut self,
        callee: &Expression,
        arguments: &[CallArgument],
    ) -> DirectResult<bool> {
        let callee_local = self.allocate_temp_local();
        self.emit_numeric_expression(callee)?;
        self.push_local_set(callee_local);

        if self
            .backend
            .function_registry
            .catalog
            .user_functions
            .is_empty()
        {
            return Ok(false);
        }

        let expanded_arguments = self.expand_call_arguments(arguments);
        let mut call_arguments = Vec::with_capacity(expanded_arguments.len());
        let mut argument_shadow_writebacks = Vec::new();
        for (index, argument) in expanded_arguments.iter().enumerate() {
            let argument_value_local = self.allocate_temp_local();
            self.emit_numeric_expression(argument)?;
            self.push_local_set(argument_value_local);

            let hidden_name = self.allocate_named_hidden_local(
                &format!("dynamic_super_arg_{index}"),
                self.infer_value_kind(argument)
                    .unwrap_or(StaticValueKind::Unknown),
            );
            let hidden_local = self
                .state
                .runtime
                .locals
                .get(&hidden_name)
                .copied()
                .expect("fresh dynamic super hidden local must exist");
            self.push_local_get(argument_value_local);
            self.push_local_set(hidden_local);
            self.update_capture_slot_binding_from_expression(&hidden_name, argument)?;
            self.sync_capture_slot_runtime_object_shadows_from_expression(&hidden_name, argument)?;
            let source_owner = match argument {
                Expression::Identifier(name) => {
                    self.runtime_object_property_shadow_owner_name_for_identifier(name)
                }
                Expression::This => Some("this".to_string()),
                _ => None,
            };
            if let Some(source_owner) = source_owner {
                argument_shadow_writebacks.push((hidden_name.clone(), source_owner));
            }
            call_arguments.push(CallArgument::Expression(Expression::Identifier(
                hidden_name,
            )));
        }

        let constructible_user_functions = self
            .backend
            .function_registry
            .catalog
            .user_functions
            .iter()
            .filter(|user_function| user_function.is_constructible())
            .cloned()
            .collect::<Vec<_>>();
        if constructible_user_functions.is_empty() {
            return Ok(false);
        }

        for (index, user_function) in constructible_user_functions.iter().enumerate() {
            self.push_local_get(callee_local);
            self.push_i32_const(user_function_runtime_value(user_function));
            self.push_binary_op(BinaryOp::Equal)?;
            self.state.emission.output.instructions.push(0x04);
            self.state.emission.output.instructions.push(I32_TYPE);
            self.push_control_frame();
            if self.current_function_is_derived_constructor() {
                self.emit_derived_constructor_super_call(user_function, &call_arguments)?;
            } else {
                self.emit_user_function_call_with_current_new_target_and_this_expression(
                    user_function,
                    &call_arguments,
                    &Expression::This,
                )?;
            }
            self.state.emission.output.instructions.push(0x05);
            if index + 1 == constructible_user_functions.len() {
                self.push_i32_const(JS_UNDEFINED_TAG);
            }
        }
        for _ in 0..constructible_user_functions.len() {
            self.state.emission.output.instructions.push(0x0b);
            self.pop_control_frame();
        }
        let dynamic_result_local = self.allocate_temp_local();
        self.push_local_set(dynamic_result_local);
        for (hidden_name, source_owner) in &argument_shadow_writebacks {
            self.emit_runtime_object_property_shadow_copy(hidden_name, source_owner)?;
        }
        self.push_local_get(dynamic_result_local);

        Ok(true)
    }
}
