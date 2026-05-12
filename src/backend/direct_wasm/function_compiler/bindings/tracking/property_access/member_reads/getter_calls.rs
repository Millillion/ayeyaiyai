use super::*;

impl<'a> FunctionCompiler<'a> {
    pub(in crate::backend::direct_wasm) fn emit_member_getter_call_with_bound_this(
        &mut self,
        function_name: &str,
        this_expression: &Expression,
        capture_slots: Option<&BTreeMap<String, String>>,
    ) -> DirectResult<()> {
        let Some(user_function) = self.user_function(function_name).cloned() else {
            self.push_i32_const(JS_UNDEFINED_TAG);
            return Ok(());
        };
        if let Some(capture_slots) = capture_slots {
            return self
                .emit_user_function_call_with_new_target_and_this_expression_and_bound_captures(
                    &user_function,
                    &[],
                    JS_UNDEFINED_TAG,
                    this_expression,
                    capture_slots,
                );
        }
        self.emit_user_function_call_with_new_target_and_this_expression(
            &user_function,
            &[],
            JS_UNDEFINED_TAG,
            this_expression,
        )
    }

    pub(super) fn emit_member_binding_read_without_prelude(
        &mut self,
        object: &Expression,
        property: &Expression,
    ) -> DirectResult<bool> {
        if !matches!(property, Expression::String(_) | Expression::Number(_))
            && self.resolve_property_key_expression(property).is_none()
        {
            return Ok(false);
        }
        if std::env::var_os("AYY_TRACE_PRIVATE_MEMBER_LOOKUP").is_some()
            && matches!(property, Expression::String(name) if name.starts_with("__ayy$private$"))
        {
            eprintln!(
                "private_member_binding_read current_fn={:?} object={object:?} property={property:?}",
                self.current_function_name(),
            );
        }
        let private_receiver_requires_runtime_brand_check = self
            .is_private_member_read_property(property)
            && (matches!(object, Expression::This)
                || self
                    .resolve_bound_alias_expression(object)
                    .is_some_and(|resolved| {
                        !static_expression_matches(&resolved, object)
                            && matches!(resolved, Expression::This)
                    })
                || self.expression_uses_runtime_dynamic_binding(object));
        if private_receiver_requires_runtime_brand_check {
            return Ok(false);
        }
        let can_resolve_private_binding_from_receiver = !self
            .is_private_member_read_property(property)
            || matches!(object, Expression::This)
            || self
                .resolve_bound_alias_expression(object)
                .is_some_and(|resolved| matches!(resolved, Expression::This));
        if !can_resolve_private_binding_from_receiver {
            return Ok(false);
        }
        if let Some(function_binding) = self.resolve_member_getter_binding(object, property) {
            if std::env::var_os("AYY_TRACE_RESTRICTED_PROPERTIES").is_some()
                && matches!(property, Expression::String(property_name) if property_name == "caller" || property_name == "arguments")
            {
                eprintln!(
                    "restricted_property_read getter current_fn={:?} object={object:?} property={property:?} binding={function_binding:?}",
                    self.current_function_name(),
                );
            }
            let capture_slots = self.resolve_member_function_capture_slots(object, property);
            match function_binding {
                LocalFunctionBinding::User(function_name) => {
                    let static_this_expression =
                        self.resolve_static_snapshot_this_expression(object);
                    let static_getter_binding = LocalFunctionBinding::User(function_name.clone());
                    let can_use_static_getter_return = self
                        .user_function(&function_name)
                        .and_then(|user_function| user_function.inline_summary.as_ref())
                        .is_some_and(|summary| summary.effects.is_empty());
                    if can_use_static_getter_return {
                        if let Some(return_value) = self
                            .resolve_function_binding_static_return_expression_with_call_frame(
                                &static_getter_binding,
                                &[],
                                &static_this_expression,
                            )
                        {
                            self.emit_numeric_expression(&return_value)?;
                            return Ok(true);
                        }
                    }
                    self.emit_member_getter_call_with_bound_this(
                        &function_name,
                        object,
                        capture_slots.as_ref(),
                    )?;
                }
                LocalFunctionBinding::Builtin(function_name) => {
                    let callee = Expression::Identifier(function_name);
                    if !self.emit_arguments_slot_accessor_call(&callee, &[], 0, Some(&[]))? {
                        self.push_i32_const(JS_UNDEFINED_TAG);
                    }
                }
            }
            return Ok(true);
        }
        if let Some(function_binding) = self.resolve_member_function_binding(object, property) {
            if std::env::var_os("AYY_TRACE_RESTRICTED_PROPERTIES").is_some()
                && matches!(property, Expression::String(property_name) if property_name == "caller" || property_name == "arguments")
            {
                eprintln!(
                    "restricted_property_read method current_fn={:?} object={object:?} property={property:?} binding={function_binding:?}",
                    self.current_function_name(),
                );
            }
            match function_binding {
                LocalFunctionBinding::User(function_name) => {
                    if let Some(user_function) = self.user_function(&function_name) {
                        self.push_i32_const(user_function_runtime_value(user_function));
                    } else {
                        self.push_i32_const(JS_UNDEFINED_TAG);
                    }
                }
                LocalFunctionBinding::Builtin(function_name) => {
                    self.push_i32_const(
                        builtin_function_runtime_value(&function_name)
                            .unwrap_or(JS_TYPEOF_FUNCTION_TAG),
                    );
                }
            }
            return Ok(true);
        }
        if matches!(property, Expression::String(property_name) if property_name == "caller") {
            if let Some(strict) = self.resolve_arguments_callee_strictness(object) {
                if strict {
                    return self.emit_error_throw().map(|()| true);
                }
                self.push_i32_const(JS_UNDEFINED_TAG);
                return Ok(true);
            }
        }
        if self.is_restricted_function_property(object, property) {
            if std::env::var_os("AYY_TRACE_RESTRICTED_PROPERTIES").is_some() {
                eprintln!(
                    "restricted_property_read throw current_fn={:?} object={object:?} property={property:?}",
                    self.current_function_name(),
                );
            }
            self.emit_numeric_expression(object)?;
            self.state.emission.output.instructions.push(0x1a);
            return self.emit_named_error_throw("TypeError").map(|()| true);
        }
        Ok(false)
    }
}
