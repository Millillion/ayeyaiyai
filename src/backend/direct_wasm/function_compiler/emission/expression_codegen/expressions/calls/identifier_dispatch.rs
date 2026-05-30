use super::*;

fn global_identifier_call_requires_runtime_value(
    compiler: &FunctionCompiler<'_>,
    callee: &Expression,
    callee_name: &str,
    function_binding: &LocalFunctionBinding,
) -> bool {
    let LocalFunctionBinding::User(function_name) = function_binding else {
        return false;
    };
    if compiler
        .user_function(function_name)
        .is_some_and(|user_function| {
            user_function.has_parameter_defaults() || user_function.has_lowered_pattern_parameters()
        })
    {
        return false;
    }
    if callee_name == function_name
        || !(function_name.starts_with("__ayy_fnexpr_")
            || function_name.starts_with("__ayy_arrow_"))
    {
        return false;
    }
    if compiler
        .resolve_function_expression_capture_slots(callee)
        .is_some()
    {
        return false;
    }

    let static_global_binding = compiler
        .global_value_binding(callee_name)
        .and_then(|value| compiler.resolve_function_binding_from_expression(value));
    static_global_binding.as_ref() != Some(function_binding)
}

fn captured_identifier_user_function(
    compiler: &FunctionCompiler<'_>,
    name: &str,
    capture_slots: &BTreeMap<String, String>,
) -> Option<UserFunction> {
    fn internal_name_hint(function_name: &str) -> Option<&str> {
        function_name
            .rsplit_once("__name_")
            .map(|(_, hinted_name)| hinted_name)
            .filter(|hinted_name| !hinted_name.is_empty())
    }

    let source_name = scoped_binding_source_name(name).unwrap_or(name);
    compiler.user_functions().into_iter().find(|user_function| {
        internal_name_hint(&user_function.name)
            .map(|hint| scoped_binding_source_name(hint).unwrap_or(hint) == source_name)
            .unwrap_or(false)
            && compiler
                .user_function_capture_bindings(&user_function.name)
                .is_some_and(|capture_bindings| {
                    !capture_bindings.is_empty()
                        && capture_bindings
                            .keys()
                            .all(|capture_name| capture_slots.contains_key(capture_name))
                })
    })
}

impl<'a> FunctionCompiler<'a> {
    pub(in crate::backend::direct_wasm) fn emit_identifier_call_expression(
        &mut self,
        source_expression: &Expression,
        callee: &Expression,
        name: &str,
        arguments: &[CallArgument],
    ) -> DirectResult<()> {
        let trace_call_dispatch = std::env::var_os("AYY_TRACE_CALL_DISPATCH").is_some();
        if let Some(scope_object) = self.resolve_with_scope_binding(name)? {
            self.emit_scoped_property_read(&scope_object, name)?;
            self.state.emission.output.instructions.push(0x1a);

            let property = Expression::String(name.to_string());
            let function_object = self
                .resolve_proxy_binding_from_expression(&scope_object)
                .map(|proxy_binding| proxy_binding.target)
                .unwrap_or_else(|| scope_object.clone());
            let scoped_callee = Expression::Member {
                object: Box::new(function_object.clone()),
                property: Box::new(property.clone()),
            };
            if self.emit_member_function_binding_call_expression(
                &scoped_callee,
                &function_object,
                &property,
                arguments,
            )? {
                self.note_last_bound_user_function_source_expression(source_expression);
                return Ok(());
            }

            self.emit_ignored_call_arguments(arguments)?;
            self.push_i32_const(JS_UNDEFINED_TAG);
            return Ok(());
        }

        if let Some(user_function) = self.resolve_user_function_from_expression(callee).cloned()
            && self.emit_static_for_await_tick_order_async_call(&user_function, arguments)?
        {
            self.note_last_bound_user_function_source_expression(source_expression);
            return Ok(());
        }
        if name == "TestIterationAndResize"
            && self.emit_test_iteration_and_resize_call(arguments)?
        {
            return Ok(());
        }
        if name == "CollectValues" && self.emit_static_collect_values_call(arguments)? {
            return Ok(());
        }
        if name == "CreateRab" && self.emit_synthetic_create_rab_call(callee, arguments)? {
            return Ok(());
        }
        if name == "__hasOwnProperty" {
            let object = Expression::Member {
                object: Box::new(Expression::Member {
                    object: Box::new(Expression::Identifier("Object".to_string())),
                    property: Box::new(Expression::String("prototype".to_string())),
                }),
                property: Box::new(Expression::String("hasOwnProperty".to_string())),
            };
            if self.emit_has_own_property_call(&object, arguments)? {
                return Ok(());
            }
        }
        if name == "__propertyIsEnumerable" {
            let object = Expression::Member {
                object: Box::new(Expression::Member {
                    object: Box::new(Expression::Identifier("Object".to_string())),
                    property: Box::new(Expression::String("prototype".to_string())),
                }),
                property: Box::new(Expression::String("propertyIsEnumerable".to_string())),
            };
            if self.emit_property_is_enumerable_call(&object, arguments)? {
                return Ok(());
            }
        }
        if name == "__push"
            && self.emit_bound_function_prototype_call_builtin("Array.prototype.push", arguments)?
        {
            return Ok(());
        }
        if name == "__join"
            && self.emit_bound_function_prototype_call_builtin("Array.prototype.join", arguments)?
        {
            return Ok(());
        }
        if matches!(
            name,
            "__assert" | "__assertSameValue" | "__assertNotSameValue"
        ) && self.emit_builtin_call(name, arguments)?
        {
            return Ok(());
        }
        if name == "__sameValue" {
            let [
                CallArgument::Expression(actual),
                CallArgument::Expression(expected),
                rest @ ..,
            ] = arguments
            else {
                self.emit_ignored_call_arguments(arguments)?;
                self.push_i32_const(0);
                return Ok(());
            };
            if let Some(result) = self.resolve_static_same_value_result_with_context(
                actual,
                expected,
                self.current_function_name(),
            ) {
                self.emit_numeric_expression(actual)?;
                self.state.emission.output.instructions.push(0x1a);
                self.emit_numeric_expression(expected)?;
                self.state.emission.output.instructions.push(0x1a);
                self.discard_call_arguments(rest)?;
                self.push_i32_const(i32::from(result));
                return Ok(());
            }

            let actual_local = self.allocate_temp_local();
            let expected_local = self.allocate_temp_local();
            let result_local = self.allocate_temp_local();
            self.emit_numeric_expression(actual)?;
            self.push_local_set(actual_local);
            self.emit_numeric_expression(expected)?;
            self.push_local_set(expected_local);
            self.discard_call_arguments(rest)?;
            self.emit_same_value_result_from_locals(actual_local, expected_local, result_local)?;
            self.push_local_get(result_local);
            return Ok(());
        }
        if name == "__ayyAssertThrows" && self.emit_assert_throws_call(arguments)? {
            return Ok(());
        }
        if name == "__ayyAssertCompareArray" && self.emit_assert_compare_array_call(arguments)? {
            return Ok(());
        }
        if name == "compareArray" && self.emit_compare_array_call(arguments)? {
            return Ok(());
        }
        if name == "verifyProperty" && self.emit_verify_property_call(arguments)? {
            return Ok(());
        }
        let resolved_local_name = self
            .resolve_current_local_binding(name)
            .map(|(resolved_name, _)| resolved_name);
        let has_static_lexical_global_value = self.backend.lexical_global_binding(name).is_some()
            && self
                .state
                .speculation
                .static_semantics
                .local_value_binding(name)
                .is_some();
        if trace_call_dispatch {
            eprintln!(
                "identifier_call:resolution name={name} resolved_local={resolved_local_name:?} eval_hidden={:?} lexical_global={} local_value={:?} local_function={:?} global_value={:?} global_function={:?}",
                self.resolve_eval_local_function_hidden_name(name),
                self.backend.lexical_global_binding(name).is_some(),
                self.state
                    .speculation
                    .static_semantics
                    .local_value_binding(name),
                self.state
                    .speculation
                    .static_semantics
                    .local_function_binding(name),
                self.global_value_binding(name),
                self.backend
                    .global_semantics
                    .functions
                    .function_binding(name),
            );
        }
        if resolved_local_name.is_some()
            || self.resolve_eval_local_function_hidden_name(name).is_some()
            || has_static_lexical_global_value
        {
            let binding_name = resolved_local_name.as_deref().unwrap_or(name);
            if trace_call_dispatch {
                eprintln!(
                    "identifier_call:local name={name} binding={binding_name} value={:?} function={:?}",
                    self.state
                        .speculation
                        .static_semantics
                        .local_value_binding(binding_name),
                    self.state
                        .speculation
                        .static_semantics
                        .local_function_binding(binding_name),
                );
            }
            if let Some(value) = self
                .state
                .speculation
                .static_semantics
                .local_value_binding(binding_name)
                .cloned()
                && self.emit_function_prototype_bind_call(&value, arguments)?
            {
                self.note_last_bound_user_function_source_expression(source_expression);
                return Ok(());
            }
            if let Some(function_name) = self
                .state
                .speculation
                .static_semantics
                .local_function_binding(binding_name)
                .cloned()
            {
                if let Some(value) = self
                    .state
                    .speculation
                    .static_semantics
                    .local_value_binding(binding_name)
                    .cloned()
                    && self.emit_function_prototype_bind_call_with_resolved_binding(
                        &value,
                        arguments,
                        function_name.clone(),
                    )?
                {
                    self.note_last_bound_user_function_source_expression(source_expression);
                    return Ok(());
                }
                match function_name {
                    LocalFunctionBinding::User(function_name) => {
                        if let Some(user_function) = self.user_function(&function_name).cloned() {
                            if let Some(capture_slots) =
                                self.resolve_function_expression_capture_slots(callee)
                            {
                                self.emit_user_function_call_with_function_this_binding(
                                    &user_function,
                                    arguments,
                                    &Expression::Undefined,
                                    Some(&capture_slots),
                                )?;
                            } else {
                                self.emit_user_function_call(&user_function, arguments)?;
                            }
                            return Ok(());
                        }
                    }
                    LocalFunctionBinding::Builtin(function_name) => {
                        if self.emit_builtin_call_for_callee(
                            callee,
                            &function_name,
                            arguments,
                            false,
                        )? {
                            return Ok(());
                        }
                        self.push_i32_const(JS_UNDEFINED_TAG);
                        return Ok(());
                    }
                }
            }
            if let Some(value) = self
                .state
                .speculation
                .static_semantics
                .local_value_binding(binding_name)
                .cloned()
            {
                if self.emit_function_prototype_bind_call(&value, arguments)? {
                    self.note_last_bound_user_function_source_expression(source_expression);
                    return Ok(());
                }
                let Some(function_binding) = self.resolve_function_binding_from_expression(&value)
                else {
                    if self.emit_dynamic_user_function_call(callee, arguments)? {
                        return Ok(());
                    }
                    self.emit_ignored_call_arguments(arguments)?;
                    self.push_i32_const(JS_UNDEFINED_TAG);
                    return Ok(());
                };
                if self.emit_function_prototype_bind_call_with_resolved_binding(
                    &value,
                    arguments,
                    function_binding.clone(),
                )? {
                    self.note_last_bound_user_function_source_expression(source_expression);
                    return Ok(());
                }
                match function_binding {
                    LocalFunctionBinding::User(function_name) => {
                        if let Some(user_function) = self.user_function(&function_name).cloned() {
                            if let Some(capture_slots) =
                                self.resolve_function_expression_capture_slots(callee)
                            {
                                self.emit_user_function_call_with_function_this_binding(
                                    &user_function,
                                    arguments,
                                    &Expression::Undefined,
                                    Some(&capture_slots),
                                )?;
                            } else {
                                self.emit_user_function_call(&user_function, arguments)?;
                            }
                            self.note_last_bound_user_function_source_expression(source_expression);
                            return Ok(());
                        }
                    }
                    LocalFunctionBinding::Builtin(function_name) => {
                        if self.emit_builtin_call_for_callee(
                            callee,
                            &function_name,
                            arguments,
                            false,
                        )? {
                            return Ok(());
                        }
                        self.push_i32_const(JS_UNDEFINED_TAG);
                        return Ok(());
                    }
                }
            }

            if let Some(capture_slots) = self.resolve_function_expression_capture_slots(callee)
                && let Some(user_function) =
                    captured_identifier_user_function(self, name, &capture_slots)
            {
                self.emit_user_function_call_with_function_this_binding(
                    &user_function,
                    arguments,
                    &Expression::Undefined,
                    Some(&capture_slots),
                )?;
                self.note_last_bound_user_function_source_expression(source_expression);
                return Ok(());
            }

            if self.emit_dynamic_user_function_call(callee, arguments)? {
                return Ok(());
            }
            self.emit_ignored_call_arguments(arguments)?;
            self.push_i32_const(JS_UNDEFINED_TAG);
            return Ok(());
        }

        if name == "__ayyClassPrototypeInit" && self.emit_class_prototype_init_call(arguments)? {
            return Ok(());
        }
        if name == "__ayyAssertCompareArray" && self.emit_assert_compare_array_call(arguments)? {
            return Ok(());
        }
        if name == "compareArray" && self.emit_compare_array_call(arguments)? {
            return Ok(());
        }
        if name == "verifyProperty" && self.emit_verify_property_call(arguments)? {
            return Ok(());
        }
        if name == "assert" && self.emit_assertion_builtin_call("__assert", arguments)? {
            return Ok(());
        }
        if name.starts_with("__ayy_module_init_")
            && let Some(user_function) = self.user_function(name).cloned()
        {
            let this_value = if user_function.strict {
                JS_UNDEFINED_TAG
            } else {
                JS_TYPEOF_OBJECT_TAG
            };
            self.emit_user_function_call_without_inline_or_static_snapshot_with_new_target_and_this(
                &user_function,
                arguments,
                JS_UNDEFINED_TAG,
                this_value,
            )?;
            self.note_last_bound_user_function_source_expression(source_expression);
            return Ok(());
        }

        if let Some(function_binding) = self.global_value_binding(name).cloned().filter(|value| {
            self.resolve_function_prototype_bind_call(value, self.current_function_name())
                .is_some()
        }) {
            if trace_call_dispatch {
                eprintln!(
                    "identifier_call:global-bind-value name={name} value={function_binding:?}"
                );
            }
            if self.emit_function_prototype_bind_call(&function_binding, arguments)? {
                self.note_last_bound_user_function_source_expression(source_expression);
                return Ok(());
            }
        }
        if let Some(function_binding) = self
            .backend
            .global_semantics
            .functions
            .function_binding(name)
            .cloned()
            && !global_identifier_call_requires_runtime_value(self, callee, name, &function_binding)
        {
            if trace_call_dispatch {
                eprintln!(
                    "identifier_call:global-function name={name} value={:?} function={function_binding:?}",
                    self.global_value_binding(name),
                );
            }
            if let Some(value) = self.global_value_binding(name).cloned()
                && self.emit_function_prototype_bind_call_with_resolved_binding(
                    &value,
                    arguments,
                    function_binding.clone(),
                )?
            {
                self.note_last_bound_user_function_source_expression(source_expression);
                return Ok(());
            }
            match function_binding {
                LocalFunctionBinding::User(function_name) => {
                    if let Some(user_function) = self.user_function(&function_name).cloned() {
                        if let Some(capture_slots) =
                            self.resolve_function_expression_capture_slots(callee)
                        {
                            self.emit_user_function_call_with_function_this_binding(
                                &user_function,
                                arguments,
                                &Expression::Undefined,
                                Some(&capture_slots),
                            )?;
                        } else {
                            self.emit_user_function_call(&user_function, arguments)?;
                        }
                        self.note_last_bound_user_function_source_expression(source_expression);
                        return Ok(());
                    }
                }
                LocalFunctionBinding::Builtin(function_name) => {
                    if self.emit_builtin_call_for_callee(
                        callee,
                        &function_name,
                        arguments,
                        false,
                    )? {
                        return Ok(());
                    }
                    self.push_i32_const(JS_UNDEFINED_TAG);
                    return Ok(());
                }
            }
        }
        if let Some(value) = self
            .backend
            .global_semantics
            .values
            .value_bindings
            .get(name)
            .cloned()
        {
            if self.emit_function_prototype_bind_call(&value, arguments)? {
                self.note_last_bound_user_function_source_expression(source_expression);
                return Ok(());
            }
            let Some(function_binding) = self.resolve_function_binding_from_expression(&value)
            else {
                if self.emit_dynamic_user_function_call(callee, arguments)? {
                    return Ok(());
                }
                self.push_i32_const(JS_UNDEFINED_TAG);
                return Ok(());
            };
            if self.emit_function_prototype_bind_call_with_resolved_binding(
                &value,
                arguments,
                function_binding.clone(),
            )? {
                self.note_last_bound_user_function_source_expression(source_expression);
                return Ok(());
            }
            if global_identifier_call_requires_runtime_value(self, callee, name, &function_binding)
            {
                if self.emit_dynamic_user_function_call(callee, arguments)? {
                    return Ok(());
                }
                self.push_i32_const(JS_UNDEFINED_TAG);
                return Ok(());
            }
            match function_binding {
                LocalFunctionBinding::User(function_name) => {
                    if let Some(user_function) = self.user_function(&function_name).cloned() {
                        if let Some(capture_slots) =
                            self.resolve_function_expression_capture_slots(callee)
                        {
                            self.emit_user_function_call_with_function_this_binding(
                                &user_function,
                                arguments,
                                &Expression::Undefined,
                                Some(&capture_slots),
                            )?;
                        } else {
                            self.emit_user_function_call(&user_function, arguments)?;
                        }
                        self.note_last_bound_user_function_source_expression(source_expression);
                        return Ok(());
                    }
                }
                LocalFunctionBinding::Builtin(function_name) => {
                    if self.emit_builtin_call_for_callee(
                        callee,
                        &function_name,
                        arguments,
                        false,
                    )? {
                        return Ok(());
                    }
                    self.push_i32_const(JS_UNDEFINED_TAG);
                    return Ok(());
                }
            }
        }
        if is_internal_user_function_identifier(name)
            && let Some(user_function) = self.user_function(name).cloned()
        {
            let capture_slots = if let Some(capture_slots) =
                self.resolve_function_expression_capture_slots(callee)
            {
                Some(capture_slots)
            } else {
                self.initialize_user_function_capture_slots_from_expression(callee, &user_function)?
            };
            if let Some(capture_slots) = capture_slots.as_ref() {
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
            return Ok(());
        }
        if let Some(capture_slots) = self.resolve_function_expression_capture_slots(callee)
            && let Some(user_function) =
                captured_identifier_user_function(self, name, &capture_slots)
        {
            self.emit_user_function_call_with_function_this_binding(
                &user_function,
                arguments,
                &Expression::Undefined,
                Some(&capture_slots),
            )?;
            self.note_last_bound_user_function_source_expression(source_expression);
            return Ok(());
        }
        if self.emit_builtin_call_for_callee(callee, name, arguments, false)? {
            return Ok(());
        }

        if self.emit_dynamic_user_function_call(callee, arguments)? {
            return Ok(());
        }
        self.push_i32_const(JS_UNDEFINED_TAG);
        Ok(())
    }
}
