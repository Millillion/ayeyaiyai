use super::*;

impl<'a> FunctionCompiler<'a> {
    pub(in crate::backend::direct_wasm) fn canonicalize_with_scope_expression(
        &self,
        expression: &Expression,
    ) -> Expression {
        match expression {
            Expression::Identifier(name) => {
                if let Some(scope_object) = self.resolve_with_scope_binding_for_specialization(name)
                {
                    return self.materialize_static_expression(&Expression::Member {
                        object: Box::new(scope_object),
                        property: Box::new(Expression::String(name.clone())),
                    });
                }
                expression.clone()
            }
            Expression::Assign { name, .. } => Expression::Identifier(name.clone()),
            Expression::Sequence(expressions) => expressions
                .last()
                .map(|expression| self.canonicalize_with_scope_expression(expression))
                .unwrap_or(Expression::Undefined),
            _ => self.materialize_static_expression(expression),
        }
    }

    pub(in crate::backend::direct_wasm) fn scope_object_has_binding_property(
        &self,
        scope_object: &Expression,
        name: &str,
    ) -> bool {
        let property = Expression::String(name.to_string());
        let has_property = |object: &Expression| {
            self.resolve_member_function_binding(object, &property)
                .is_some()
                || self
                    .resolve_member_getter_binding(object, &property)
                    .is_some()
                || self
                    .resolve_member_setter_binding(object, &property)
                    .is_some()
                || (self.is_direct_arguments_object(object)
                    && self.direct_arguments_has_property(name))
                || self
                    .resolve_arguments_binding_from_expression(object)
                    .is_some_and(|arguments_binding| match name {
                        "callee" => arguments_binding.callee_present,
                        "length" => arguments_binding.length_present,
                        _ => argument_index_from_expression(&property)
                            .is_some_and(|index| index < arguments_binding.values.len() as u32),
                    })
                || self
                    .resolve_object_binding_from_expression(object)
                    .is_some_and(|object_binding| {
                        object_binding_has_property(&object_binding, &property)
                    })
                || (name == "constructor"
                    && self
                        .resolve_constructed_object_constructor_binding(object)
                        .is_some())
        };
        let result = has_property(scope_object)
            || {
                let captured_scope_object = match scope_object {
                    Expression::Identifier(name) => self
                        .resolve_user_function_capture_hidden_name(name)
                        .map(Expression::Identifier)
                        .or_else(|| {
                            self.resolve_eval_local_function_hidden_name(name)
                                .map(Expression::Identifier)
                        }),
                    _ => None,
                };
                captured_scope_object
                    .as_ref()
                    .is_some_and(|object| has_property(object))
                    || matches!(
                        scope_object,
                        Expression::Identifier(name)
                            if self.global_object_binding(name).is_some_and(|object_binding| {
                                object_binding_has_property(object_binding, &property)
                            }) || self
                                .backend
                                .shared_global_semantics
                                .values
                                .object_binding(name)
                                .is_some_and(|object_binding| {
                                    object_binding_has_property(object_binding, &property)
                            })
                    )
                    || matches!(
                        scope_object,
                        Expression::Identifier(scope_name)
                            if !name.starts_with("__ayy")
                                && (self
                                    .resolve_user_function_capture_hidden_name(scope_name)
                                    .is_some()
                                    || self
                                        .resolve_eval_local_function_hidden_name(scope_name)
                                        .is_some())
                    )
            }
            || {
                let materialized_scope_object = self.materialize_static_expression(scope_object);
                !static_expression_matches(&materialized_scope_object, scope_object)
                    && has_property(&materialized_scope_object)
            };
        if std::env::var_os("AYY_TRACE_WITH_SCOPE").is_some() {
            let capture_hidden = match scope_object {
                Expression::Identifier(name) => {
                    self.resolve_user_function_capture_hidden_name(name)
                }
                _ => None,
            };
            let source_global_has = match scope_object {
                Expression::Identifier(name) => self.global_object_binding(name).is_some(),
                _ => false,
            };
            let source_shared_has = match scope_object {
                Expression::Identifier(name) => self
                    .backend
                    .shared_global_semantics
                    .values
                    .object_binding(name)
                    .is_some(),
                _ => false,
            };
            eprintln!(
                "with_scope has_property fn={:?} object={scope_object:?} name={name} result={result} materialized={:?} capture_hidden={capture_hidden:?} source_global_has={source_global_has} source_shared_has={source_shared_has}",
                self.current_function_name(),
                self.materialize_static_expression(scope_object),
            );
        }
        result
    }

    pub(in crate::backend::direct_wasm) fn emit_with_scope_unscopables_block_check(
        &mut self,
        scope_object: &Expression,
        name: &str,
    ) -> DirectResult<bool> {
        let unscopables_key = Expression::Member {
            object: Box::new(Expression::Identifier("Symbol".to_string())),
            property: Box::new(Expression::String("unscopables".to_string())),
        };
        let property = Expression::String(name.to_string());

        if let Some(getter_binding) =
            self.resolve_member_getter_binding(scope_object, &unscopables_key)
        {
            let return_value =
                self.resolve_function_binding_static_return_expression(&getter_binding, &[]);
            let blocked = if let Some(return_value) = return_value {
                self.resolve_object_binding_from_expression(&return_value)
                    .and_then(|unscopables_object| {
                        object_binding_lookup_value(&unscopables_object, &property)
                            .and_then(|value| self.resolve_static_boolean_expression(value))
                    })
                    .unwrap_or(false)
            } else if let Some(unscopables_object) =
                self.resolve_function_binding_static_return_object_binding(&getter_binding, &[])
            {
                object_binding_lookup_value(&unscopables_object, &property)
                    .and_then(|value| self.resolve_static_boolean_expression(value))
                    .unwrap_or(false)
            } else {
                self.emit_function_binding_side_effects_with_arguments(&getter_binding, &[])?;
                return Ok(false);
            };
            self.emit_function_binding_side_effects_with_arguments(&getter_binding, &[])?;
            return Ok(blocked);
        }

        let Some(scope_binding) = self.resolve_object_binding_from_expression(scope_object) else {
            return Ok(false);
        };
        let Some(unscopables_value) =
            object_binding_lookup_value(&scope_binding, &unscopables_key).cloned()
        else {
            return Ok(false);
        };
        let unscopables_member = Expression::Member {
            object: Box::new(scope_object.clone()),
            property: Box::new(unscopables_key.clone()),
        };
        if let Some(getter_binding) =
            self.resolve_member_getter_binding(&unscopables_member, &property)
        {
            let return_value =
                self.resolve_function_binding_static_return_expression(&getter_binding, &[]);
            let blocked = return_value
                .as_ref()
                .and_then(|value| self.resolve_static_boolean_expression(value))
                .unwrap_or(false);
            self.emit_function_binding_side_effects_with_arguments(&getter_binding, &[])?;
            return Ok(blocked);
        }
        let Some(unscopables_object) =
            self.resolve_object_binding_from_expression(&unscopables_value)
        else {
            return Ok(false);
        };
        if let Some(getter_binding) =
            self.resolve_member_getter_binding(&unscopables_value, &property)
        {
            let return_value =
                self.resolve_function_binding_static_return_expression(&getter_binding, &[]);
            let blocked = return_value
                .as_ref()
                .and_then(|value| self.resolve_static_boolean_expression(value))
                .unwrap_or(false);
            self.emit_function_binding_side_effects_with_arguments(&getter_binding, &[])?;
            return Ok(blocked);
        }
        Ok(object_binding_lookup_value(&unscopables_object, &property)
            .and_then(|value| self.resolve_static_boolean_expression(value))
            .unwrap_or(false))
    }

    fn emit_proxy_with_scope_unscopables_block_check(
        &mut self,
        scope_object: &Expression,
        proxy_binding: &ProxyValueBinding,
        name: &str,
    ) -> DirectResult<bool> {
        let unscopables_key = Expression::Member {
            object: Box::new(Expression::Identifier("Symbol".to_string())),
            property: Box::new(Expression::String("unscopables".to_string())),
        };

        if let Some(get_binding) = proxy_binding.get_binding.clone() {
            let arguments = [
                proxy_binding.target.clone(),
                unscopables_key.clone(),
                scope_object.clone(),
            ];
            self.emit_function_binding_effect_statements_with_arguments(&get_binding, &arguments)?;
        }

        self.with_suspended_with_scopes(|compiler| {
            compiler.emit_with_scope_unscopables_block_check(&proxy_binding.target, name)
        })
    }

    pub(in crate::backend::direct_wasm) fn resolve_with_scope_binding(
        &mut self,
        name: &str,
    ) -> DirectResult<Option<Expression>> {
        if name.starts_with("__ayy_") {
            return Ok(None);
        }
        let scopes = self.state.emission.lexical_scopes.with_scopes.clone();
        let property = Expression::String(name.to_string());
        if std::env::var_os("AYY_TRACE_WITH_SCOPE").is_some() {
            eprintln!(
                "with_scope resolve fn={:?} name={name} scopes={scopes:?}",
                self.current_function_name(),
            );
        }
        for scope_object in scopes.into_iter().rev() {
            if let Some(proxy_binding) = self.resolve_proxy_binding_from_expression(&scope_object) {
                let has_binding_property = if let Some(has_binding) =
                    proxy_binding.has_binding.clone()
                {
                    let arguments = [proxy_binding.target.clone(), property.clone()];
                    let has_binding_result = self
                        .resolve_function_binding_static_return_bool(&has_binding, &arguments)
                        .unwrap_or_else(|| {
                            self.with_suspended_with_scopes(|compiler| {
                                Ok(compiler
                                    .scope_object_has_binding_property(&proxy_binding.target, name))
                            })
                            .unwrap_or(false)
                        });
                    self.emit_function_binding_effect_statements_with_arguments(
                        &has_binding,
                        &arguments,
                    )?;
                    has_binding_result
                } else {
                    self.with_suspended_with_scopes(|compiler| {
                        Ok(compiler.scope_object_has_binding_property(&proxy_binding.target, name))
                    })?
                };
                let blocked = self.emit_proxy_with_scope_unscopables_block_check(
                    &scope_object,
                    &proxy_binding,
                    name,
                )?;
                if has_binding_property && !blocked {
                    return Ok(Some(scope_object));
                }
                continue;
            }

            let has_binding_property = self.with_suspended_with_scopes(|compiler| {
                Ok(compiler.scope_object_has_binding_property(&scope_object, name))
            })?;
            if std::env::var_os("AYY_TRACE_WITH_SCOPE").is_some() {
                eprintln!(
                    "with_scope candidate fn={:?} name={name} object={scope_object:?} has={has_binding_property}",
                    self.current_function_name(),
                );
            }
            if !has_binding_property {
                continue;
            }
            let blocked = self.with_suspended_with_scopes(|compiler| {
                compiler.emit_with_scope_unscopables_block_check(&scope_object, name)
            })?;
            if std::env::var_os("AYY_TRACE_WITH_SCOPE").is_some() {
                eprintln!(
                    "with_scope blocked fn={:?} name={name} object={scope_object:?} blocked={blocked}",
                    self.current_function_name(),
                );
            }
            if blocked {
                continue;
            }
            return Ok(Some(scope_object));
        }

        Ok(None)
    }

    pub(in crate::backend::direct_wasm) fn emit_scoped_property_read(
        &mut self,
        scope_object: &Expression,
        name: &str,
    ) -> DirectResult<()> {
        let property = Expression::String(name.to_string());
        if let Some(proxy_binding) = self.resolve_proxy_binding_from_expression(scope_object) {
            if let Some(has_binding) = proxy_binding.has_binding.clone() {
                let arguments = [proxy_binding.target.clone(), property.clone()];
                self.emit_function_binding_effect_statements_with_arguments(
                    &has_binding,
                    &arguments,
                )?;
            }
            if let Some(get_binding) = proxy_binding.get_binding.clone() {
                let arguments = [
                    proxy_binding.target.clone(),
                    property.clone(),
                    scope_object.clone(),
                ];
                self.emit_function_binding_effect_statements_with_arguments(
                    &get_binding,
                    &arguments,
                )?;
            }
            return self.with_suspended_with_scopes(|compiler| {
                compiler.emit_scoped_property_read(&proxy_binding.target, name)
            });
        }
        if let Some(getter_binding) = self.resolve_member_getter_binding(scope_object, &property) {
            match getter_binding {
                LocalFunctionBinding::User(function_name) => {
                    if let Some(user_function) = self.user_function(&function_name).cloned() {
                        self.with_suspended_with_scopes_if_active_scope_object(
                            scope_object,
                            |compiler| {
                                compiler.emit_user_function_call_with_function_this_binding(
                                    &user_function,
                                    &[],
                                    scope_object,
                                    None,
                                )
                            },
                        )?;
                    } else {
                        self.push_i32_const(JS_UNDEFINED_TAG);
                    }
                }
                LocalFunctionBinding::Builtin(function_name) => {
                    let callee = Expression::Identifier(function_name);
                    if !self.emit_arguments_slot_accessor_call(&callee, &[], 0, Some(&[]))? {
                        self.push_i32_const(JS_UNDEFINED_TAG);
                    }
                }
            }
            return Ok(());
        }
        if let Some(object_binding) = self.resolve_object_binding_from_expression(scope_object)
            && let Some(value) = object_binding_lookup_value(&object_binding, &property)
        {
            if self
                .resolve_runtime_object_property_shadow_binding(scope_object, &property)
                .is_some()
                || self.runtime_object_property_shadow_deletion_may_affect_property(
                    scope_object,
                    &property,
                )
            {
                if self.state.speculation.execution_context.strict_mode
                    && let Some(deleted_binding) = self
                        .resolve_runtime_object_property_shadow_deleted_binding(
                            scope_object,
                            &property,
                        )
                {
                    self.push_global_get(deleted_binding.present_index);
                    self.state.emission.output.instructions.push(0x04);
                    self.state.emission.output.instructions.push(I32_TYPE);
                    self.push_control_frame();
                    self.emit_named_error_throw("ReferenceError")?;
                    self.state.emission.output.instructions.push(0x0b);
                    self.pop_control_frame();
                }
                if self.emit_runtime_object_shadow_member_read(scope_object, &property)? {
                    return Ok(());
                }
            }
            self.with_suspended_with_scopes(|compiler| compiler.emit_numeric_expression(value))?;
            return Ok(());
        }
        if let Some(function_binding) =
            self.resolve_member_function_binding(scope_object, &property)
        {
            match function_binding {
                LocalFunctionBinding::User(function_name) => {
                    if let Some(user_function) = self.user_function(&function_name) {
                        self.push_i32_const(user_function_runtime_value(user_function));
                    } else {
                        self.push_i32_const(JS_UNDEFINED_TAG);
                    }
                }
                LocalFunctionBinding::Builtin(_) => {
                    self.push_i32_const(JS_TYPEOF_FUNCTION_TAG);
                }
            }
            return Ok(());
        }
        if name == "constructor"
            && let Some(function_binding) =
                self.resolve_constructed_object_constructor_binding(scope_object)
        {
            match function_binding {
                LocalFunctionBinding::User(function_name) => {
                    if let Some(user_function) = self.user_function(&function_name) {
                        self.push_i32_const(user_function_runtime_value(user_function));
                    } else {
                        self.push_i32_const(JS_TYPEOF_FUNCTION_TAG);
                    }
                }
                LocalFunctionBinding::Builtin(function_name) => {
                    self.push_i32_const(
                        builtin_function_runtime_value(&function_name)
                            .unwrap_or(JS_TYPEOF_FUNCTION_TAG),
                    );
                }
            }
            return Ok(());
        }
        if self.emit_runtime_object_shadow_member_read(scope_object, &property)? {
            return Ok(());
        }
        if let Some(object_binding) = self.resolve_object_binding_from_expression(scope_object) {
            if let Some(value) = object_binding_lookup_value(&object_binding, &property) {
                self.with_suspended_with_scopes(|compiler| {
                    compiler.emit_numeric_expression(value)
                })?;
            } else {
                self.push_i32_const(JS_UNDEFINED_TAG);
            }
            return Ok(());
        }
        self.push_i32_const(JS_UNDEFINED_TAG);
        Ok(())
    }
}
