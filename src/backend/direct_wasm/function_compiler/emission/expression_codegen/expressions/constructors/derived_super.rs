use super::*;

impl<'a> FunctionCompiler<'a> {
    pub(in crate::backend::direct_wasm) fn current_lexical_function_captures_this(&self) -> bool {
        self.current_user_function()
            .is_some_and(|function| function.lexical_this)
            && self
                .resolve_user_function_capture_hidden_name("this")
                .is_some()
    }

    fn current_derived_super_this_capture_hidden_name(&self) -> Option<String> {
        self.current_lexical_function_captures_this()
            .then(|| self.resolve_user_function_capture_hidden_name("this"))
            .flatten()
    }

    fn current_derived_super_new_target_capture_hidden_name(&self) -> Option<String> {
        self.current_user_function()
            .is_some_and(|function| function.lexical_this)
            .then(|| self.resolve_user_function_capture_hidden_name("new.target"))
            .flatten()
    }

    fn emit_current_derived_super_this_value(&mut self, this_capture_hidden_name: Option<&str>) {
        if let Some(hidden_name) = this_capture_hidden_name {
            let binding = self
                .implicit_global_binding(hidden_name)
                .unwrap_or_else(|| self.ensure_implicit_global_binding(hidden_name));
            self.push_global_get(binding.value_index);
        } else {
            self.push_global_get(CURRENT_THIS_GLOBAL_INDEX);
        }
    }

    fn emit_store_derived_super_initialized_this(
        &mut self,
        initialized_this_local: u32,
        this_capture_hidden_name: Option<&str>,
    ) {
        if let Some(hidden_name) = this_capture_hidden_name {
            let binding = self
                .implicit_global_binding(hidden_name)
                .unwrap_or_else(|| self.ensure_implicit_global_binding(hidden_name));
            self.push_local_get(initialized_this_local);
            self.push_global_set(binding.value_index);
            self.push_i32_const(1);
            self.push_global_set(binding.present_index);
        }
        self.push_local_get(initialized_this_local);
        self.push_global_set(CURRENT_THIS_GLOBAL_INDEX);
    }

    pub(in crate::backend::direct_wasm) fn emit_derived_constructor_super_call(
        &mut self,
        user_function: &UserFunction,
        arguments: &[CallArgument],
    ) -> DirectResult<()> {
        let expanded_arguments = self.expand_call_arguments(arguments);
        let (runtime_arguments, argument_shadow_writebacks) =
            self.prepare_constructor_runtime_argument_bindings(arguments)?;

        let super_target_hidden_name = Self::STATIC_NEW_THIS_BINDING.to_string();
        if !self
            .state
            .runtime
            .locals
            .bindings
            .contains_key(&super_target_hidden_name)
        {
            let next_local_index = self.state.runtime.locals.next_local_index;
            self.state
                .runtime
                .locals
                .insert(super_target_hidden_name.clone(), next_local_index);
            self.state
                .speculation
                .static_semantics
                .set_local_kind(&super_target_hidden_name, StaticValueKind::Object);
            self.state.runtime.locals.next_local_index += 1;
        }
        let super_target_local = self
            .state
            .runtime
            .locals
            .get(&super_target_hidden_name)
            .copied()
            .expect("derived super target hidden local must exist");
        self.emit_numeric_expression(&Expression::Object(Vec::new()))?;
        self.push_local_set(super_target_local);
        self.update_local_value_binding(&super_target_hidden_name, &Expression::Object(Vec::new()));
        self.update_local_object_binding(
            &super_target_hidden_name,
            &Expression::Object(Vec::new()),
        );
        self.clear_runtime_object_property_shadow_prefix(&super_target_hidden_name);

        let saved_new_target_local = if let Some(hidden_name) =
            self.current_derived_super_new_target_capture_hidden_name()
        {
            let saved_local = self.allocate_temp_local();
            self.push_global_get(CURRENT_NEW_TARGET_GLOBAL_INDEX);
            self.push_local_set(saved_local);
            self.emit_identifier_expression_value(&hidden_name)?;
            self.push_global_set(CURRENT_NEW_TARGET_GLOBAL_INDEX);
            Some(saved_local)
        } else {
            None
        };
        let super_this_expression = if self.user_function_is_derived_constructor(user_function) {
            Expression::Undefined
        } else {
            Expression::Identifier(super_target_hidden_name.clone())
        };
        self.emit_user_function_call_with_current_new_target_and_this_expression(
            user_function,
            &runtime_arguments,
            &super_this_expression,
        )?;
        let return_value_local = self.allocate_temp_local();
        self.push_local_set(return_value_local);
        for (hidden_name, source_owner) in argument_shadow_writebacks {
            self.emit_runtime_object_property_shadow_copy(&hidden_name, &source_owner)?;
        }
        if let Some(saved_new_target_local) = saved_new_target_local {
            self.push_local_get(saved_new_target_local);
            self.push_global_set(CURRENT_NEW_TARGET_GLOBAL_INDEX);
        }
        self.push_local_get(return_value_local);
        self.sync_direct_arguments_assignments_from_static_user_call(
            user_function,
            &expanded_arguments,
        );
        self.finish_derived_super_call_return(&runtime_arguments, |compiler| {
            compiler.sync_derived_constructor_this_binding_after_super_call(
                user_function,
                arguments,
                Self::STATIC_NEW_THIS_BINDING,
            )
        })
    }

    pub(in crate::backend::direct_wasm) fn emit_derived_constructor_builtin_super_call(
        &mut self,
        function_name: &str,
        arguments: &[CallArgument],
    ) -> DirectResult<bool> {
        if !self.emit_builtin_call(function_name, arguments)? {
            return Ok(false);
        }
        self.finish_derived_super_call_return(arguments, |compiler| {
            compiler.sync_derived_constructor_this_binding_after_builtin_super_call();
            Ok(())
        })?;
        Ok(true)
    }

    fn finish_derived_super_call_return(
        &mut self,
        _arguments: &[CallArgument],
        sync_this: impl FnOnce(&mut Self) -> DirectResult<()>,
    ) -> DirectResult<()> {
        let return_value_local = self.allocate_temp_local();
        self.push_local_set(return_value_local);
        self.push_local_get(return_value_local);
        let return_value_visible_local = self.allocate_temp_local();
        self.push_local_set(return_value_visible_local);
        let this_capture_hidden_name = self.current_derived_super_this_capture_hidden_name();

        self.emit_current_derived_super_this_value(this_capture_hidden_name.as_deref());
        self.push_i32_const(JS_UNDEFINED_TAG);
        self.push_binary_op(BinaryOp::NotEqual)?;
        self.state.emission.output.instructions.push(0x04);
        self.state.emission.output.instructions.push(I32_TYPE);
        self.push_control_frame();
        self.emit_named_error_throw("ReferenceError")?;
        self.push_i32_const(JS_UNDEFINED_TAG);
        self.state.emission.output.instructions.push(0x05);

        sync_this(self)?;

        let initialized_this_local = self.allocate_temp_local();
        self.push_local_get(return_value_visible_local);
        self.push_i32_const(JS_TYPEOF_OBJECT_TAG);
        self.push_binary_op(BinaryOp::Equal)?;
        self.state.emission.output.instructions.push(0x04);
        self.state.emission.output.instructions.push(I32_TYPE);
        self.push_control_frame();
        self.push_local_get(return_value_visible_local);
        self.state.emission.output.instructions.push(0x05);
        self.push_local_get(return_value_visible_local);
        self.push_i32_const(JS_TYPEOF_FUNCTION_TAG);
        self.push_binary_op(BinaryOp::Equal)?;
        self.state.emission.output.instructions.push(0x04);
        self.state.emission.output.instructions.push(I32_TYPE);
        self.push_control_frame();
        self.push_local_get(return_value_visible_local);
        self.state.emission.output.instructions.push(0x05);
        self.push_i32_const(JS_TYPEOF_OBJECT_TAG);
        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();
        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();
        self.push_local_set(initialized_this_local);
        self.emit_store_derived_super_initialized_this(
            initialized_this_local,
            this_capture_hidden_name.as_deref(),
        );
        self.sync_derived_constructor_this_capture_slots_after_super(initialized_this_local)?;
        self.push_local_get(initialized_this_local);
        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();
        Ok(())
    }

    fn sync_derived_constructor_this_capture_slots_after_super(
        &mut self,
        initialized_this_local: u32,
    ) -> DirectResult<()> {
        let slot_names = self
            .state
            .speculation
            .static_semantics
            .capture_slot_source_bindings
            .iter()
            .filter_map(|(slot_name, source_name)| {
                (source_name == "this").then(|| slot_name.clone())
            })
            .collect::<Vec<_>>();

        for slot_name in slot_names {
            if let Some(slot_local) = self.state.runtime.locals.get(&slot_name).copied() {
                self.push_local_get(initialized_this_local);
                self.push_local_set(slot_local);
                self.update_capture_slot_binding_from_expression(&slot_name, &Expression::This)?;
                self.sync_capture_slot_runtime_object_shadows_from_expression(
                    &slot_name,
                    &Expression::This,
                )?;
                self.state
                    .speculation
                    .static_semantics
                    .capture_slot_source_bindings
                    .insert(slot_name, "this".to_string());
                continue;
            }

            if let Some(hidden_binding) = self.hidden_implicit_global_binding(&slot_name) {
                self.push_local_get(initialized_this_local);
                self.push_global_set(hidden_binding.value_index);
                self.push_i32_const(1);
                self.push_global_set(hidden_binding.present_index);
                self.update_static_global_assignment_metadata(&slot_name, &Expression::This);
                self.update_global_property_descriptor_value(&slot_name, &Expression::This);
                self.sync_capture_slot_runtime_object_shadows_from_expression(
                    &slot_name,
                    &Expression::This,
                )?;
                self.state
                    .speculation
                    .static_semantics
                    .capture_slot_source_bindings
                    .insert(slot_name, "this".to_string());
            }
        }

        Ok(())
    }
}
