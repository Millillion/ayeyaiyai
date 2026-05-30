use super::*;

pub(in crate::backend::direct_wasm) struct CapturedIteratorNextMethodPlan {
    pub(in crate::backend::direct_wasm) function_name: String,
    pub(in crate::backend::direct_wasm) current_slot: String,
    pub(in crate::backend::direct_wasm) next_value: Expression,
}

impl<'a> FunctionCompiler<'a> {
    fn emit_copy_iterator_next_result_shadow_property(
        &mut self,
        source_owner: &str,
        target_owner: &str,
        property: &Expression,
    ) -> DirectResult<()> {
        let source_binding =
            self.runtime_object_property_shadow_binding_by_property(source_owner, property);
        let target_binding =
            self.runtime_object_property_shadow_binding_by_property(target_owner, property);
        let source_deleted =
            self.runtime_object_property_shadow_deleted_binding_by_property(source_owner, property);
        let target_deleted =
            self.runtime_object_property_shadow_deleted_binding_by_property(target_owner, property);

        self.push_global_get(source_deleted.present_index);
        self.state.emission.output.instructions.push(0x04);
        self.state
            .emission
            .output
            .instructions
            .push(EMPTY_BLOCK_TYPE);
        self.push_control_frame();
        self.push_i32_const(JS_UNDEFINED_TAG);
        self.push_global_set(target_binding.value_index);
        self.push_i32_const(0);
        self.push_global_set(target_binding.present_index);
        self.push_i32_const(JS_UNDEFINED_TAG);
        self.push_global_set(target_deleted.value_index);
        self.push_i32_const(1);
        self.push_global_set(target_deleted.present_index);
        self.state.emission.output.instructions.push(0x05);
        self.push_global_get(source_binding.present_index);
        self.state.emission.output.instructions.push(0x04);
        self.state
            .emission
            .output
            .instructions
            .push(EMPTY_BLOCK_TYPE);
        self.push_control_frame();
        self.push_i32_const(JS_UNDEFINED_TAG);
        self.push_global_set(target_deleted.value_index);
        self.push_i32_const(0);
        self.push_global_set(target_deleted.present_index);
        if !self.emit_iterator_next_static_getter_shadow_property(
            source_owner,
            &target_binding,
            &target_deleted,
            property,
        )? {
            self.push_global_get(source_binding.value_index);
            self.push_global_set(target_binding.value_index);
            self.push_i32_const(1);
            self.push_global_set(target_binding.present_index);
        }
        self.state.emission.output.instructions.push(0x05);
        if self.emit_iterator_next_proxy_get_fallback_shadow_property(
            source_owner,
            &target_binding,
            &target_deleted,
            property,
        )? || self.emit_iterator_next_static_fallback_shadow_property(
            source_owner,
            &target_binding,
            &target_deleted,
            property,
        )? {
        } else {
            self.push_i32_const(JS_UNDEFINED_TAG);
            self.push_global_set(target_deleted.value_index);
            self.push_i32_const(0);
            self.push_global_set(target_deleted.present_index);
            self.push_i32_const(JS_UNDEFINED_TAG);
            self.push_global_set(target_binding.value_index);
            self.push_i32_const(0);
            self.push_global_set(target_binding.present_index);
        }
        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();
        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();
        Ok(())
    }

    fn emit_iterator_next_proxy_get_fallback_shadow_property(
        &mut self,
        source_owner: &str,
        target_binding: &ImplicitGlobalBinding,
        target_deleted: &ImplicitGlobalBinding,
        property: &Expression,
    ) -> DirectResult<bool> {
        let source = Expression::Identifier(source_owner.to_string());
        let Some(proxy_binding) = self.resolve_proxy_binding_from_expression(&source) else {
            return Ok(false);
        };
        let Some(get_binding) = proxy_binding.get_binding.clone() else {
            return Ok(false);
        };

        self.push_i32_const(JS_UNDEFINED_TAG);
        self.push_global_set(target_deleted.value_index);
        self.push_i32_const(0);
        self.push_global_set(target_deleted.present_index);

        let arguments = [
            CallArgument::Expression(proxy_binding.target.clone()),
            CallArgument::Expression(property.clone()),
            CallArgument::Expression(source),
        ];
        match get_binding {
            LocalFunctionBinding::User(function_name) => {
                let Some(user_function) = self.user_function(&function_name).cloned() else {
                    self.push_i32_const(JS_UNDEFINED_TAG);
                    self.push_global_set(target_binding.value_index);
                    self.push_i32_const(1);
                    self.push_global_set(target_binding.present_index);
                    return Ok(true);
                };
                self.emit_user_function_call_with_new_target_and_this_expression_without_static_snapshot(
                    &user_function,
                    &arguments,
                    JS_UNDEFINED_TAG,
                    &proxy_binding.handler,
                )?;
                self.push_global_set(target_binding.value_index);
            }
            LocalFunctionBinding::Builtin(_) => {
                self.push_i32_const(JS_UNDEFINED_TAG);
                self.push_global_set(target_binding.value_index);
            }
        }
        self.push_i32_const(1);
        self.push_global_set(target_binding.present_index);
        Ok(true)
    }

    fn emit_iterator_next_static_getter_shadow_property(
        &mut self,
        source_owner: &str,
        target_binding: &ImplicitGlobalBinding,
        target_deleted: &ImplicitGlobalBinding,
        property: &Expression,
    ) -> DirectResult<bool> {
        let source = Expression::Identifier(source_owner.to_string());
        let Some(getter_binding) = self.resolve_member_getter_binding(&source, property) else {
            return Ok(false);
        };

        self.push_i32_const(JS_UNDEFINED_TAG);
        self.push_global_set(target_deleted.value_index);
        self.push_i32_const(0);
        self.push_global_set(target_deleted.present_index);

        match getter_binding {
            LocalFunctionBinding::User(function_name) => {
                let capture_slots = self.resolve_member_function_capture_slots(&source, property);
                self.emit_member_getter_call_with_bound_this(
                    &function_name,
                    &source,
                    capture_slots.as_ref(),
                )?;
                self.push_global_set(target_binding.value_index);
            }
            LocalFunctionBinding::Builtin(function_name) => {
                let callee = Expression::Identifier(function_name);
                if !self.emit_arguments_slot_accessor_call(&callee, &[], 0, Some(&[]))? {
                    self.push_i32_const(JS_UNDEFINED_TAG);
                }
                self.push_global_set(target_binding.value_index);
            }
        }
        self.push_i32_const(1);
        self.push_global_set(target_binding.present_index);
        Ok(true)
    }

    fn emit_iterator_next_static_fallback_shadow_property(
        &mut self,
        source_owner: &str,
        target_binding: &ImplicitGlobalBinding,
        target_deleted: &ImplicitGlobalBinding,
        property: &Expression,
    ) -> DirectResult<bool> {
        if self.emit_iterator_next_static_getter_shadow_property(
            source_owner,
            target_binding,
            target_deleted,
            property,
        )? {
            return Ok(true);
        }

        let source = Expression::Identifier(source_owner.to_string());
        let Some(object_binding) = self.resolve_object_binding_from_expression(&source) else {
            return Ok(false);
        };
        let Some(value) = self.resolve_object_binding_property_value_with_inherited(
            &source,
            &object_binding,
            property,
        ) else {
            return Ok(false);
        };

        self.push_i32_const(JS_UNDEFINED_TAG);
        self.push_global_set(target_deleted.value_index);
        self.push_i32_const(0);
        self.push_global_set(target_deleted.present_index);
        self.emit_static_iterator_next_shadow_property_value(property, &value)?;
        self.push_global_set(target_binding.value_index);
        self.push_i32_const(1);
        self.push_global_set(target_binding.present_index);
        Ok(true)
    }

    fn emit_static_iterator_next_shadow_property_value(
        &mut self,
        property: &Expression,
        value: &Expression,
    ) -> DirectResult<()> {
        if matches!(property, Expression::String(property_name) if property_name == "done")
            && let Some(text) = self.resolve_static_string_value(value)
        {
            self.push_i32_const(if text.is_empty() { 0 } else { 1 });
            return Ok(());
        }
        self.emit_numeric_expression(value)
    }

    pub(in crate::backend::direct_wasm) fn captured_iterator_next_method_plan(
        &self,
        object: &Expression,
        property: &Expression,
        arguments: &[CallArgument],
    ) -> Option<CapturedIteratorNextMethodPlan> {
        if !arguments.is_empty()
            || !matches!(
                self.materialize_static_expression(property),
                Expression::String(property_name) if property_name == "next"
            )
        {
            return None;
        }

        let LocalFunctionBinding::User(function_name) =
            self.resolve_member_function_binding(object, property)?
        else {
            return None;
        };
        let capture_slots = self.resolve_member_function_capture_slots(object, property)?;
        let function = self.resolve_registered_function_declaration(&function_name)?;
        let statements = function
            .body
            .iter()
            .filter(|statement| !matches!(statement, Statement::Block { body } if body.is_empty()))
            .collect::<Vec<_>>();
        let [first, second, third] = statements.as_slice() else {
            return None;
        };

        let (result_name, current_capture_name) = match first {
            Statement::Var {
                name,
                value: Expression::Identifier(current_capture_name),
            }
            | Statement::Let {
                name,
                value: Expression::Identifier(current_capture_name),
                ..
            } => (name, current_capture_name),
            _ => return None,
        };
        let next_value = match second {
            Statement::Assign { name, value } if name == current_capture_name => value,
            _ => return None,
        };
        match third {
            Statement::Return(Expression::Identifier(returned_name))
                if returned_name == result_name => {}
            _ => return None,
        }

        let current_slot = capture_slots.get(current_capture_name)?.clone();
        let next_value = self.substitute_capture_slot_bindings(next_value, &capture_slots);
        Some(CapturedIteratorNextMethodPlan {
            function_name,
            current_slot,
            next_value,
        })
    }

    pub(in crate::backend::direct_wasm) fn emit_store_iterator_next_capture_slot(
        &mut self,
        slot_name: &str,
        value: &Expression,
    ) -> DirectResult<()> {
        self.emit_numeric_expression(value)?;
        let value_local =
            if let Some(local_index) = self.state.runtime.locals.bindings.get(slot_name).copied() {
                self.push_local_set(local_index);
                local_index
            } else {
                let binding = self.ensure_implicit_global_binding(slot_name);
                self.push_global_set(binding.value_index);
                self.push_i32_const(1);
                self.push_global_set(binding.present_index);
                let value_local = self.allocate_temp_local();
                self.push_global_get(binding.value_index);
                self.push_local_set(value_local);
                value_local
            };
        self.update_member_function_bindings_for_value(slot_name, value, value_local)?;
        self.update_capture_slot_binding_from_expression(slot_name, value)?;
        self.sync_capture_slot_runtime_object_shadows_from_expression(slot_name, value)
    }

    pub(in crate::backend::direct_wasm) fn emit_captured_iterator_next_method_call(
        &mut self,
        source_expression: &Expression,
        object: &Expression,
        property: &Expression,
        arguments: &[CallArgument],
    ) -> DirectResult<bool> {
        let Some(plan) = self.captured_iterator_next_method_plan(object, property, arguments)
        else {
            return Ok(false);
        };

        let current_expression = Expression::Identifier(plan.current_slot.clone());
        let result_slot = self.allocate_named_hidden_local(
            "iterator_next_result",
            self.infer_value_kind(&current_expression)
                .unwrap_or(StaticValueKind::Object),
        );
        let result_local = self
            .state
            .runtime
            .locals
            .bindings
            .get(&result_slot)
            .copied()
            .expect("fresh iterator next result slot local must exist");

        self.emit_numeric_expression(&current_expression)?;
        self.push_local_set(result_local);
        self.update_capture_slot_binding_from_expression(&result_slot, &current_expression)?;
        self.sync_capture_slot_runtime_object_shadows_from_expression(
            &result_slot,
            &current_expression,
        )?;
        for property in [
            Expression::String("done".to_string()),
            Expression::String("value".to_string()),
        ] {
            self.emit_copy_iterator_next_result_shadow_property(
                &plan.current_slot,
                &result_slot,
                &property,
            )?;
        }
        self.state
            .speculation
            .static_semantics
            .clear_local_value_binding(&result_slot);

        self.emit_store_iterator_next_capture_slot(&plan.current_slot, &plan.next_value)?;

        self.state
            .speculation
            .static_semantics
            .last_bound_user_function_call = Some(BoundUserFunctionCallSnapshot {
            function_name: plan.function_name,
            source_expression: Some(source_expression.clone()),
            result_expression: Some(Expression::Identifier(result_slot.clone())),
            prototype_source_expression: None,
            updated_bindings: HashMap::new(),
        });
        self.push_local_get(result_local);
        Ok(true)
    }

    pub(in crate::backend::direct_wasm) fn cached_iterator_next_method_binding_for_object(
        &self,
        object: &Expression,
        property: &Expression,
    ) -> Option<CachedIteratorNextMethodBinding> {
        let Expression::String(property_name) = self.materialize_static_expression(property) else {
            return None;
        };
        if property_name != "next" {
            return None;
        }
        let Expression::Identifier(name) = object else {
            return None;
        };

        let mut candidates = Vec::new();
        let mut push_candidate = |candidate: String| {
            if !candidates.iter().any(|existing| existing == &candidate) {
                candidates.push(candidate);
            }
        };
        push_candidate(name.clone());
        if let Some((resolved_name, _)) = self.resolve_current_local_binding(name) {
            push_candidate(resolved_name);
        }
        if let Some(binding_name) = self.resolve_local_array_iterator_binding_name(name) {
            push_candidate(binding_name);
        }
        if let Some(Expression::Identifier(alias_name)) = self
            .state
            .speculation
            .static_semantics
            .local_value_binding(name)
            .or_else(|| self.backend.global_value_binding(name))
        {
            push_candidate(alias_name.clone());
        }

        candidates.into_iter().find_map(|candidate| {
            self.state
                .speculation
                .static_semantics
                .arrays
                .cached_iterator_next_method_binding(&candidate)
                .cloned()
        })
    }

    pub(in crate::backend::direct_wasm) fn emit_cached_iterator_next_method_call(
        &mut self,
        object: &Expression,
        property: &Expression,
        arguments: &[CallArgument],
    ) -> DirectResult<bool> {
        let trace = std::env::var_os("AYY_TRACE_ITERATOR_NEXT_CACHE").is_some();
        let Some(binding) = self.cached_iterator_next_method_binding_for_object(object, property)
        else {
            if trace
                && matches!(
                    self.materialize_static_expression(property),
                    Expression::String(property_name) if property_name == "next"
                )
            {
                eprintln!("iterator_next_cache:dispatch miss object={object:?}");
            }
            return Ok(false);
        };
        if trace {
            eprintln!(
                "iterator_next_cache:dispatch hit object={object:?} this={:?} binding={:?}",
                binding.this_expression, binding.function_binding
            );
        }

        match binding.function_binding {
            LocalFunctionBinding::User(function_name) => {
                let Some(user_function) = self.user_function(&function_name).cloned() else {
                    return Ok(false);
                };
                self.emit_user_function_call_with_function_this_binding(
                    &user_function,
                    arguments,
                    &binding.this_expression,
                    binding.capture_slots.as_ref(),
                )?;
                Ok(true)
            }
            LocalFunctionBinding::Builtin(function_name) => {
                let callee = Expression::Member {
                    object: Box::new(binding.this_expression),
                    property: Box::new(Expression::String("next".to_string())),
                };
                self.emit_builtin_call_for_callee(&callee, &function_name, arguments, false)
            }
        }
    }

    pub(in crate::backend::direct_wasm) fn resolve_member_getter_returned_user_function(
        &self,
        object: &Expression,
        property: &Expression,
    ) -> Option<(UserFunction, BTreeMap<String, String>)> {
        let getter_binding = self.resolve_member_getter_binding(object, property)?;
        let returned_expression = self
            .resolve_function_binding_static_return_expression_with_call_frame(
                &getter_binding,
                &[],
                object,
            )?;
        let LocalFunctionBinding::User(function_name) =
            self.resolve_function_binding_from_expression(&returned_expression)?
        else {
            return None;
        };
        let user_function = self.user_function(&function_name)?.clone();
        let Some(captures) = self
            .backend
            .function_registry
            .analysis
            .user_function_capture_bindings
            .get(&user_function.name)
        else {
            return Some((user_function, BTreeMap::new()));
        };
        let getter_capture_slots = self
            .resolve_member_function_capture_slots(object, property)
            .or_else(|| self.resolve_function_expression_capture_slots(&returned_expression));
        let Some(getter_capture_slots) = getter_capture_slots else {
            return if captures.is_empty() {
                Some((user_function, BTreeMap::new()))
            } else {
                None
            };
        };
        let mut bound_capture_slots = BTreeMap::new();
        for capture_name in captures.keys() {
            let slot_name = getter_capture_slots.get(capture_name)?;
            bound_capture_slots.insert(capture_name.clone(), slot_name.clone());
        }
        Some((user_function, bound_capture_slots))
    }

    pub(in crate::backend::direct_wasm) fn emit_member_getter_returned_user_function_call(
        &mut self,
        object: &Expression,
        property: &Expression,
        arguments: &[CallArgument],
    ) -> DirectResult<bool> {
        if matches!(
            property,
            Expression::String(name) if (name == "then" || name == "catch")
                && Self::call_is_promise_like_chain(object)
        ) {
            return Ok(false);
        }
        if matches!(
            property,
            Expression::String(name) if matches!(name.as_str(), "then" | "catch" | "finally")
        ) && self.expression_is_direct_async_function_call(object)
        {
            return Ok(false);
        }
        if self.promise_member_call_requires_runtime_fallback(object, property, arguments) {
            return Ok(false);
        }
        let Some(LocalFunctionBinding::User(getter_function_name)) =
            self.resolve_member_getter_binding(object, property)
        else {
            return Ok(false);
        };
        let Some(getter_user_function) = self.user_function(&getter_function_name).cloned() else {
            return Ok(false);
        };
        let Some((returned_user_function, returned_capture_slots)) =
            self.resolve_member_getter_returned_user_function(object, property)
        else {
            return Ok(false);
        };

        let getter_capture_slots = self.resolve_member_function_capture_slots(object, property);
        self.emit_user_function_call_with_function_this_binding(
            &getter_user_function,
            &[],
            object,
            getter_capture_slots.as_ref(),
        )?;
        self.state.emission.output.instructions.push(0x1a);

        if returned_capture_slots.is_empty() {
            self.emit_user_function_call_with_function_this_binding(
                &returned_user_function,
                arguments,
                object,
                None,
            )?;
        } else {
            self.emit_user_function_call_with_new_target_and_this_expression_and_bound_captures(
                &returned_user_function,
                arguments,
                JS_UNDEFINED_TAG,
                object,
                &returned_capture_slots,
            )?;
        }
        Ok(true)
    }
}
