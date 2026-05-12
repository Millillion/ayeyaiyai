use super::*;

impl<'a> FunctionCompiler<'a> {
    fn update_active_object_literal_with_scope_property(
        &mut self,
        scope_object: &Expression,
        name: &str,
        value: &Expression,
    ) {
        let Expression::Object(mut entries) = scope_object.clone() else {
            return;
        };

        if let Some(ObjectEntry::Data {
            value: existing_value,
            ..
        }) = entries.iter_mut().rev().find(|entry| {
            matches!(
                entry,
                ObjectEntry::Data { key, .. }
                    if matches!(key, Expression::String(property_name) if property_name == name)
            )
        }) {
            *existing_value = value.clone();
        } else {
            entries.push(ObjectEntry::Data {
                key: Expression::String(name.to_string()),
                value: value.clone(),
            });
        }

        let updated_scope = Expression::Object(entries);
        if let Some(active_scope) = self
            .state
            .emission
            .lexical_scopes
            .with_scopes
            .iter_mut()
            .rev()
            .find(|active_scope| *active_scope == scope_object)
        {
            *active_scope = updated_scope;
        }
    }

    fn emit_strict_scoped_deleted_binding_store_check(
        &mut self,
        scope_object: &Expression,
        property: &Expression,
    ) -> DirectResult<()> {
        if !self.state.speculation.execution_context.strict_mode {
            return Ok(());
        }
        let Some(deleted_binding) =
            self.resolve_runtime_object_property_shadow_deleted_binding(scope_object, property)
        else {
            return Ok(());
        };
        self.push_global_get(deleted_binding.present_index);
        self.state.emission.output.instructions.push(0x04);
        self.state
            .emission
            .output
            .instructions
            .push(EMPTY_BLOCK_TYPE);
        self.push_control_frame();
        self.emit_named_error_throw("ReferenceError")?;
        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();
        Ok(())
    }

    fn scoped_store_is_rejected_by_typed_array_prototype(
        &self,
        scope_object: &Expression,
        property: &Expression,
    ) -> bool {
        if !matches!(property, Expression::String(name) if name == "NaN") {
            return false;
        }
        let Some(prototype) = self.resolve_static_object_prototype_expression(scope_object) else {
            return false;
        };
        self.resolve_static_typed_array_object_binding_from_expression(&prototype)
            .is_some()
            || self
                .resolve_typed_array_view_binding_from_expression(&prototype)
                .is_some()
    }

    fn user_function_is_direct_reflect_set_forwarder(&self, user_function: &UserFunction) -> bool {
        let Some(function) = self.resolve_registered_function_declaration(&user_function.name)
        else {
            return false;
        };
        let [target_param, key_param, value_param, receiver_param, ..] =
            user_function.params.as_slice()
        else {
            return false;
        };

        function.body.iter().any(|statement| {
            let Statement::Return(Expression::Call { callee, arguments }) = statement else {
                return false;
            };
            matches!(
                callee.as_ref(),
                Expression::Member { object, property }
                    if matches!(object.as_ref(), Expression::Identifier(name) if name == "Reflect")
                        && matches!(property.as_ref(), Expression::String(name) if name == "set")
            ) && matches!(
                arguments.as_slice(),
                [
                    CallArgument::Expression(Expression::Identifier(target)),
                    CallArgument::Expression(Expression::Identifier(key)),
                    CallArgument::Expression(Expression::Identifier(value)),
                    CallArgument::Expression(Expression::Identifier(receiver)),
                ] if target == target_param
                    && key == key_param
                    && value == value_param
                    && receiver == receiver_param
            )
        })
    }

    fn proxy_set_binding_is_direct_reflect_set_forwarder(
        &self,
        set_binding: &LocalFunctionBinding,
    ) -> bool {
        let LocalFunctionBinding::User(function_name) = set_binding else {
            return false;
        };
        self.user_function(function_name)
            .is_some_and(|user_function| {
                self.user_function_is_direct_reflect_set_forwarder(user_function)
            })
    }

    fn emit_proxy_reflect_set_forwarded_effects(
        &mut self,
        proxy_binding: &ProxyValueBinding,
        property: &Expression,
        value_expression: &Expression,
    ) -> DirectResult<()> {
        if let Some(get_own_property_descriptor_binding) =
            proxy_binding.get_own_property_descriptor_binding.clone()
        {
            let arguments = [proxy_binding.target.clone(), property.clone()];
            self.emit_function_binding_effect_statements_with_arguments(
                &get_own_property_descriptor_binding,
                &arguments,
            )?;
        }

        let descriptor = Expression::Object(vec![ObjectEntry::Data {
            key: Expression::String("value".to_string()),
            value: value_expression.clone(),
        }]);
        if let Some(define_property_binding) = proxy_binding.define_property_binding.clone() {
            let arguments = [
                proxy_binding.target.clone(),
                property.clone(),
                descriptor.clone(),
            ];
            self.emit_function_binding_side_effects_with_arguments(
                &define_property_binding,
                &arguments,
            )?;
        } else {
            let reflect_arguments = [
                CallArgument::Expression(proxy_binding.target.clone()),
                CallArgument::Expression(property.clone()),
                CallArgument::Expression(descriptor),
            ];
            if self.emit_reflect_define_property_call(
                &Expression::Identifier("Reflect".to_string()),
                &Expression::String("defineProperty".to_string()),
                &reflect_arguments,
            )? {
                self.state.emission.output.instructions.push(0x1a);
            }
        }
        Ok(())
    }

    fn emit_proxy_scoped_property_store_from_local(
        &mut self,
        scope_object: &Expression,
        property: &Expression,
        value_local: u32,
        value_expression: &Expression,
        proxy_binding: &ProxyValueBinding,
    ) -> DirectResult<()> {
        if let Some(has_binding) = proxy_binding.has_binding.clone() {
            let arguments = [proxy_binding.target.clone(), property.clone()];
            self.emit_function_binding_effect_statements_with_arguments(&has_binding, &arguments)?;
        }

        let value_arg_name = self.allocate_named_hidden_local(
            "proxy_set_value",
            self.infer_value_kind(value_expression)
                .unwrap_or(StaticValueKind::Unknown),
        );
        let value_arg_local = self
            .state
            .runtime
            .locals
            .get(&value_arg_name)
            .copied()
            .expect("fresh proxy set value hidden local must exist");
        self.push_local_get(value_local);
        self.push_local_set(value_arg_local);
        let value_argument = Expression::Identifier(value_arg_name);

        if let Some(set_binding) = proxy_binding.set_binding.clone() {
            let arguments = [
                proxy_binding.target.clone(),
                property.clone(),
                value_argument.clone(),
                scope_object.clone(),
            ];
            self.emit_function_binding_effect_statements_with_arguments(&set_binding, &arguments)?;
            if self.proxy_set_binding_is_direct_reflect_set_forwarder(&set_binding) {
                self.emit_proxy_reflect_set_forwarded_effects(
                    proxy_binding,
                    property,
                    &value_argument,
                )?;
            }
        } else {
            self.emit_proxy_reflect_set_forwarded_effects(
                proxy_binding,
                property,
                &value_argument,
            )?;
        }

        self.push_local_get(value_local);
        Ok(())
    }

    pub(in crate::backend::direct_wasm) fn emit_scoped_property_store_from_local(
        &mut self,
        scope_object: &Expression,
        name: &str,
        value_local: u32,
        value_expression: &Expression,
    ) -> DirectResult<()> {
        let property = Expression::String(name.to_string());
        let materialized_value =
            self.reference_preserving_static_value_expression(value_expression);
        self.emit_strict_scoped_deleted_binding_store_check(scope_object, &property)?;
        if self.scoped_store_is_rejected_by_typed_array_prototype(scope_object, &property) {
            self.push_local_get(value_local);
            return Ok(());
        }
        if let Some(proxy_binding) = self.resolve_proxy_binding_from_expression(scope_object) {
            return self.emit_proxy_scoped_property_store_from_local(
                scope_object,
                &property,
                value_local,
                value_expression,
                &proxy_binding,
            );
        }
        if matches!(name, "callee" | "length") {
            if self.is_direct_arguments_object(scope_object) {
                if name == "callee" && self.state.speculation.execution_context.strict_mode {
                    self.push_local_get(value_local);
                    self.state.emission.output.instructions.push(0x1a);
                    self.emit_error_throw()?;
                    return Ok(());
                }
                self.apply_current_arguments_effect(
                    name,
                    ArgumentsPropertyEffect::Assign(value_expression.clone()),
                );
                self.push_local_get(value_local);
                return Ok(());
            }
            if let Some(arguments_binding) =
                self.resolve_arguments_binding_from_expression(scope_object)
            {
                if name == "callee" && arguments_binding.strict {
                    self.push_local_get(value_local);
                    self.state.emission.output.instructions.push(0x1a);
                    self.emit_error_throw()?;
                    return Ok(());
                }
                self.update_named_arguments_binding_effect(
                    scope_object,
                    name,
                    ArgumentsPropertyEffect::Assign(value_expression.clone()),
                );
                self.push_local_get(value_local);
                return Ok(());
            }
        }
        self.update_member_function_assignment_binding(scope_object, &property, value_expression);
        if let Expression::Identifier(scope_name) = scope_object
            && let Some(source_name) = self.resolve_capture_slot_source_binding_name(scope_name)
        {
            let source_object = Expression::Identifier(source_name.clone());
            self.update_member_function_assignment_binding(
                &source_object,
                &property,
                value_expression,
            );
            if let Some(object_binding) = self
                .backend
                .global_semantics
                .values
                .object_bindings
                .get_mut(&source_name)
            {
                object_binding_set_property(
                    object_binding,
                    property.clone(),
                    materialized_value.clone(),
                );
            }
        }
        if matches!(scope_object, Expression::This)
            && self.current_function_name().is_none()
            && !name.starts_with("__ayy")
        {
            let binding = self.ensure_implicit_global_binding(name);
            self.emit_store_implicit_global_from_local(binding, value_local)?;
            self.update_static_global_assignment_metadata(name, &materialized_value);
        }
        if let Some(binding) =
            self.resolve_runtime_object_property_shadow_binding(scope_object, &property)
        {
            self.push_local_get(value_local);
            self.push_global_set(binding.value_index);
            self.clear_runtime_object_property_shadow_deleted_binding(scope_object, &property);
            self.push_i32_const(1);
            self.push_global_set(binding.present_index);
            if let Some(shadow_binding_name) = self
                .runtime_object_property_shadow_binding_name_for_expression(scope_object, &property)
            {
                if std::env::var_os("AYY_TRACE_THIS_FLOW").is_some() {
                    eprintln!(
                        "this_flow shadow_metadata_write fn={:?} name={} value={materialized_value:?}",
                        self.current_function_name(),
                        shadow_binding_name,
                    );
                }
                self.update_static_global_assignment_metadata(
                    &shadow_binding_name,
                    &materialized_value,
                );
                self.backend
                    .shared_global_semantics
                    .values
                    .set_value_binding(shadow_binding_name.clone(), materialized_value.clone());
                if let Some(kind) = self.infer_value_kind(&materialized_value) {
                    self.backend
                        .shared_global_semantics
                        .set_global_binding_kind(&shadow_binding_name, kind);
                }
            }
        }
        if let Expression::Identifier(scope_name) = scope_object
            && !scope_name.starts_with("__ayy")
        {
            let source_binding =
                self.runtime_object_property_shadow_binding_by_property(scope_name, &property);
            let source_deleted = self
                .runtime_object_property_shadow_deleted_binding_by_property(scope_name, &property);
            self.push_local_get(value_local);
            self.push_global_set(source_binding.value_index);
            self.push_i32_const(1);
            self.push_global_set(source_binding.present_index);
            self.push_i32_const(JS_UNDEFINED_TAG);
            self.push_global_set(source_deleted.value_index);
            self.push_i32_const(0);
            self.push_global_set(source_deleted.present_index);
            let source_expression = Expression::Identifier(scope_name.clone());
            if let Some(shadow_binding_name) = self
                .runtime_object_property_shadow_binding_name_for_expression(
                    &source_expression,
                    &property,
                )
            {
                self.update_static_global_assignment_metadata(
                    &shadow_binding_name,
                    &materialized_value,
                );
                self.backend
                    .shared_global_semantics
                    .values
                    .set_value_binding(shadow_binding_name.clone(), materialized_value.clone());
                if let Some(kind) = self.infer_value_kind(&materialized_value) {
                    self.backend
                        .shared_global_semantics
                        .set_global_binding_kind(&shadow_binding_name, kind);
                }
            }
        }
        if let Some(setter_binding) = self.resolve_member_setter_binding(scope_object, &property) {
            let receiver_hidden_name = self.allocate_named_hidden_local(
                "scoped_setter_receiver",
                self.infer_value_kind(scope_object)
                    .unwrap_or(StaticValueKind::Unknown),
            );
            let receiver_local = self
                .state
                .runtime
                .locals
                .get(&receiver_hidden_name)
                .copied()
                .expect("fresh scoped setter receiver hidden local must exist");
            let value_hidden_name = self.allocate_named_hidden_local(
                "scoped_setter_value",
                self.infer_value_kind(value_expression)
                    .unwrap_or(StaticValueKind::Unknown),
            );
            let value_hidden_local = self
                .state
                .runtime
                .locals
                .get(&value_hidden_name)
                .copied()
                .expect("fresh scoped setter value hidden local must exist");
            self.emit_numeric_expression(scope_object)?;
            self.push_local_set(receiver_local);
            self.push_local_get(value_local);
            self.push_local_set(value_hidden_local);
            self.update_local_value_binding(&receiver_hidden_name, scope_object);
            self.update_local_object_binding(&receiver_hidden_name, scope_object);
            self.update_capture_slot_binding_from_expression(&value_hidden_name, value_expression)?;
            let receiver_expression = Expression::Identifier(receiver_hidden_name.clone());
            if self.emit_function_binding_call_with_function_this_binding_from_argument_locals(
                &setter_binding,
                &[value_hidden_local],
                1,
                &receiver_expression,
            )? {
                self.state.emission.output.instructions.push(0x1a);
            }
            self.sync_simple_setter_nonlocal_assignment_metadata(
                &setter_binding,
                value_expression,
            )?;
            match scope_object {
                Expression::Identifier(name) => {
                    self.emit_runtime_object_property_shadow_copy(&receiver_hidden_name, name)?;
                }
                Expression::This => {
                    self.emit_runtime_object_property_shadow_copy(&receiver_hidden_name, "this")?;
                }
                _ => {}
            }
            self.push_local_get(value_local);
            return Ok(());
        }

        let scope_name = match scope_object {
            Expression::Identifier(name) => Some(name.as_str()),
            Expression::This => Some("this"),
            _ => None,
        };
        if let Some(scope_name) = scope_name {
            let resolved_scope_object_binding =
                self.resolve_object_binding_from_expression(scope_object);
            if std::env::var_os("AYY_TRACE_RUNTIME_SHADOWS").is_some() {
                eprintln!(
                    "scoped_property_store scope_object={scope_object:?} scope_name={scope_name} property={name} hidden_this={:?}",
                    self.resolve_user_function_capture_hidden_name("this")
                );
            }
            let lexical_this_capture_hidden_name = (scope_name == "this")
                .then(|| self.resolve_user_function_capture_hidden_name("this"))
                .flatten();
            if let Some(object_binding) = self
                .state
                .speculation
                .static_semantics
                .local_object_binding_mut(scope_name)
            {
                object_binding_set_property(
                    object_binding,
                    property.clone(),
                    materialized_value.clone(),
                );
                if let Some(global_object_binding) = self
                    .backend
                    .global_semantics
                    .values
                    .object_binding_mut(scope_name)
                {
                    object_binding_set_property(
                        global_object_binding,
                        property.clone(),
                        materialized_value.clone(),
                    );
                }
                if let Some(shared_object_binding) = self
                    .backend
                    .shared_global_semantics
                    .values
                    .object_bindings
                    .get_mut(scope_name)
                {
                    object_binding_set_property(
                        shared_object_binding,
                        property.clone(),
                        materialized_value.clone(),
                    );
                }
                if let Some(hidden_name) = lexical_this_capture_hidden_name.as_deref() {
                    let hidden_object_binding = self
                        .backend
                        .global_semantics
                        .values
                        .object_bindings
                        .entry(hidden_name.to_string())
                        .or_insert_with(empty_object_value_binding);
                    object_binding_set_property(
                        hidden_object_binding,
                        property.clone(),
                        materialized_value.clone(),
                    );
                    let shared_hidden_object_binding = self
                        .backend
                        .shared_global_semantics
                        .values
                        .object_bindings
                        .entry(hidden_name.to_string())
                        .or_insert_with(empty_object_value_binding);
                    object_binding_set_property(
                        shared_hidden_object_binding,
                        property.clone(),
                        materialized_value.clone(),
                    );
                }
                self.sync_closure_capture_slots_from_member_store(
                    scope_object,
                    &property,
                    value_local,
                    value_expression,
                )?;
                self.push_local_get(value_local);
                return Ok(());
            }
            let mut updated_global_object = false;
            if let Some(object_binding) = self
                .backend
                .global_semantics
                .values
                .object_binding_mut(scope_name)
            {
                object_binding_set_property(
                    object_binding,
                    property.clone(),
                    materialized_value.clone(),
                );
                updated_global_object = true;
            }
            if let Some(object_binding) = self
                .backend
                .shared_global_semantics
                .values
                .object_bindings
                .get_mut(scope_name)
            {
                object_binding_set_property(
                    object_binding,
                    property.clone(),
                    materialized_value.clone(),
                );
                updated_global_object = true;
            }
            if updated_global_object {
                if let Some(hidden_name) = lexical_this_capture_hidden_name.as_deref() {
                    let hidden_object_binding = self
                        .backend
                        .global_semantics
                        .values
                        .object_bindings
                        .entry(hidden_name.to_string())
                        .or_insert_with(empty_object_value_binding);
                    object_binding_set_property(
                        hidden_object_binding,
                        property.clone(),
                        materialized_value.clone(),
                    );
                    let shared_hidden_object_binding = self
                        .backend
                        .shared_global_semantics
                        .values
                        .object_bindings
                        .entry(hidden_name.to_string())
                        .or_insert_with(empty_object_value_binding);
                    object_binding_set_property(
                        shared_hidden_object_binding,
                        property.clone(),
                        materialized_value.clone(),
                    );
                }
                self.sync_closure_capture_slots_from_member_store(
                    scope_object,
                    &property,
                    value_local,
                    value_expression,
                )?;
                self.push_local_get(value_local);
                return Ok(());
            }
            if let Some(hidden_name) = self.resolve_user_function_capture_hidden_name(scope_name) {
                let hidden_object = Expression::Identifier(hidden_name.clone());
                self.update_member_function_assignment_binding(
                    &hidden_object,
                    &property,
                    value_expression,
                );
                let object_binding = self
                    .backend
                    .global_semantics
                    .values
                    .object_bindings
                    .entry(hidden_name.clone())
                    .or_insert_with(empty_object_value_binding);
                object_binding_set_property(
                    object_binding,
                    property.clone(),
                    materialized_value.clone(),
                );
                let shared_object_binding = self
                    .backend
                    .shared_global_semantics
                    .values
                    .object_bindings
                    .entry(hidden_name)
                    .or_insert_with(empty_object_value_binding);
                object_binding_set_property(
                    shared_object_binding,
                    property.clone(),
                    materialized_value.clone(),
                );
                self.sync_closure_capture_slots_from_member_store(
                    scope_object,
                    &property,
                    value_local,
                    value_expression,
                )?;
                self.push_local_get(value_local);
                return Ok(());
            }
            if let Some(mut object_binding) = resolved_scope_object_binding {
                object_binding_set_property(
                    &mut object_binding,
                    property.clone(),
                    materialized_value.clone(),
                );
                self.state
                    .speculation
                    .static_semantics
                    .set_local_object_binding(scope_name, object_binding);
                self.sync_closure_capture_slots_from_member_store(
                    scope_object,
                    &property,
                    value_local,
                    value_expression,
                )?;
                self.push_local_get(value_local);
                return Ok(());
            }
        }

        self.sync_closure_capture_slots_from_member_store(
            scope_object,
            &property,
            value_local,
            value_expression,
        )?;
        self.update_active_object_literal_with_scope_property(
            scope_object,
            name,
            &materialized_value,
        );
        self.push_local_get(value_local);
        Ok(())
    }

    pub(in crate::backend::direct_wasm) fn emit_scoped_property_update(
        &mut self,
        scope_object: &Expression,
        name: &str,
        op: UpdateOp,
        prefix: bool,
    ) -> DirectResult<()> {
        let opcode = match op {
            UpdateOp::Increment => 0x6a,
            UpdateOp::Decrement => 0x6b,
        };
        let property = Expression::String(name.to_string());
        let member_expression = Expression::Member {
            object: Box::new(scope_object.clone()),
            property: Box::new(property.clone()),
        };
        let previous_kind = self
            .infer_value_kind(&member_expression)
            .unwrap_or(StaticValueKind::Unknown);
        let current_value = self
            .resolve_object_binding_from_expression(scope_object)
            .and_then(|object_binding| {
                object_binding_lookup_value(&object_binding, &property).cloned()
            })
            .unwrap_or(Expression::Undefined);
        let increment = match op {
            UpdateOp::Increment => 1.0,
            UpdateOp::Decrement => -1.0,
        };

        match previous_kind {
            StaticValueKind::Undefined
            | StaticValueKind::String
            | StaticValueKind::Object
            | StaticValueKind::Function
            | StaticValueKind::Symbol
            | StaticValueKind::BigInt => {
                let nan_local = self.allocate_temp_local();
                self.push_i32_const(JS_NAN_TAG);
                self.push_local_set(nan_local);
                self.emit_scoped_property_store_from_local(
                    scope_object,
                    name,
                    nan_local,
                    &Expression::Number(f64::NAN),
                )?;
                self.state.emission.output.instructions.push(0x1a);
                self.push_local_get(nan_local);
                return Ok(());
            }
            StaticValueKind::Null => {
                let previous_local = self.allocate_temp_local();
                let next_local = self.allocate_temp_local();
                self.push_i32_const(0);
                self.push_local_set(previous_local);
                self.push_i32_const(increment as i32);
                self.push_local_set(next_local);
                self.emit_scoped_property_store_from_local(
                    scope_object,
                    name,
                    next_local,
                    &Expression::Number(increment),
                )?;
                self.state.emission.output.instructions.push(0x1a);
                if prefix {
                    self.push_local_get(next_local);
                } else {
                    self.push_local_get(previous_local);
                }
                return Ok(());
            }
            _ => {}
        }

        let previous_local = self.allocate_temp_local();
        let next_local = self.allocate_temp_local();
        self.emit_scoped_property_read(scope_object, name)?;
        self.push_local_set(previous_local);
        self.push_local_get(previous_local);
        self.push_i32_const(1);
        self.state.emission.output.instructions.push(opcode);
        self.push_local_set(next_local);
        let next_expression = match previous_kind {
            StaticValueKind::Bool => {
                let previous = match self.materialize_static_expression(&current_value) {
                    Expression::Bool(value) => {
                        if value {
                            1.0
                        } else {
                            0.0
                        }
                    }
                    _ => 0.0,
                };
                Expression::Number(previous + increment)
            }
            _ => self
                .resolve_static_number_value(&current_value)
                .map(|value| Expression::Number(value + increment))
                .unwrap_or(Expression::Number(f64::NAN)),
        };
        self.emit_scoped_property_store_from_local(
            scope_object,
            name,
            next_local,
            &next_expression,
        )?;
        self.state.emission.output.instructions.push(0x1a);
        if prefix {
            self.push_local_get(next_local);
        } else {
            self.push_local_get(previous_local);
        }
        Ok(())
    }
}
