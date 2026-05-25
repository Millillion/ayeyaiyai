use super::*;

fn expression_is_dynamic_import_call(expression: &Expression) -> bool {
    matches!(
        expression,
        Expression::Call { callee, .. }
            if matches!(callee.as_ref(), Expression::Identifier(name) if name == "__ayyDynamicImport")
    )
}

fn import_meta_identifier_module_index(name: &str) -> Option<&str> {
    let suffix = name.strip_prefix("__ayy_import_meta_").or_else(|| {
        name.rsplit_once("__ayy_import_meta_")
            .map(|(_, suffix)| suffix)
    })?;
    let digit_count = suffix
        .bytes()
        .take_while(|byte| byte.is_ascii_digit())
        .count();
    (digit_count > 0 && digit_count == suffix.len()).then_some(suffix)
}

fn import_meta_reference_identity_key(expression: &Expression) -> Option<String> {
    match expression {
        Expression::Identifier(name) => import_meta_identifier_module_index(name)
            .map(|module_index| format!("import-meta:{module_index}")),
        Expression::Call { callee, arguments } if matches!(callee.as_ref(), Expression::Identifier(name) if name == "__ayyImportMeta") =>
        {
            let module_key = match arguments.as_slice() {
                [] => "global".to_string(),
                [CallArgument::Expression(Expression::Number(index))] if index.is_finite() => {
                    let integer = index.trunc();
                    if integer != *index || integer < 0.0 {
                        return None;
                    }
                    format!("{integer:.0}")
                }
                _ => return None,
            };
            Some(format!("import-meta:{module_key}"))
        }
        _ => None,
    }
}

fn template_object_reference_identity_key(expression: &Expression) -> Option<String> {
    let Expression::Call { callee, arguments } = expression else {
        return None;
    };
    if !matches!(callee.as_ref(), Expression::Identifier(name) if name == "__ayyTemplateObject") {
        return None;
    }
    let Some(
        CallArgument::Expression(Expression::String(site_key))
        | CallArgument::Spread(Expression::String(site_key)),
    ) = arguments.first()
    else {
        return None;
    };
    Some(format!("template-object:{site_key}"))
}

impl<'a> FunctionCompiler<'a> {
    fn constructed_object_reference_identity_key(
        &self,
        expression: &Expression,
        callee: &Expression,
        arguments: &[CallArgument],
    ) -> Option<String> {
        let LocalFunctionBinding::User(function_name) =
            self.resolve_function_binding_from_expression(callee)?
        else {
            return None;
        };
        let user_function = self.user_function(&function_name)?;
        if !user_function.is_constructible() {
            return None;
        }

        let capture_source_bindings =
            self.resolve_constructor_capture_source_bindings_from_expression(callee);
        if let Some((return_expression, explicit)) = self
            .resolve_user_constructor_return_expression_with_explicit_status_for_function(
                user_function,
                arguments,
                capture_source_bindings.as_ref(),
            )
            && explicit
            && !matches!(
                &return_expression,
                Expression::Identifier(name) if name == Self::STATIC_NEW_THIS_BINDING
            )
            && let Some(return_key) = self.resolve_static_reference_identity_key(&return_expression)
        {
            return Some(return_key);
        }

        Some(format!("new-object:{expression:p}"))
    }

    fn resolve_template_object_reference_identity_expression_inner(
        &self,
        expression: &Expression,
        visited: &mut HashSet<String>,
    ) -> Option<Expression> {
        if template_object_reference_identity_key(expression).is_some() {
            return Some(expression.clone());
        }

        if let Expression::Identifier(name) = expression {
            if self.with_scope_blocks_static_identifier_resolution(name)
                || self
                    .state
                    .runtime
                    .locals
                    .runtime_dynamic_bindings
                    .contains(name)
                || !visited.insert(name.clone())
            {
                return None;
            }

            if let Some((resolved_name, _)) = self.resolve_current_local_binding(name) {
                if self
                    .state
                    .runtime
                    .locals
                    .runtime_dynamic_bindings
                    .contains(&resolved_name)
                {
                    return None;
                }
                if let Some(value) = self
                    .state
                    .speculation
                    .static_semantics
                    .local_value_binding(&resolved_name)
                {
                    return self.resolve_template_object_reference_identity_expression_inner(
                        value, visited,
                    );
                }
                if resolved_name != *name {
                    return self.resolve_template_object_reference_identity_expression_inner(
                        &Expression::Identifier(resolved_name),
                        visited,
                    );
                }
            }

            if let Some(value) = self
                .state
                .speculation
                .static_semantics
                .local_value_binding(name)
            {
                return self
                    .resolve_template_object_reference_identity_expression_inner(value, visited);
            }

            if let Some(value) = self
                .global_value_binding(name)
                .filter(|value| !matches!(value, Expression::Identifier(alias) if alias == name))
            {
                return self
                    .resolve_template_object_reference_identity_expression_inner(value, visited);
            }
        }

        if let Some(resolved) = self.resolve_bound_alias_expression(expression)
            && !static_expression_matches(&resolved, expression)
        {
            return self
                .resolve_template_object_reference_identity_expression_inner(&resolved, visited);
        }

        None
    }

    pub(in crate::backend::direct_wasm) fn resolve_template_object_reference_identity_expression(
        &self,
        expression: &Expression,
    ) -> Option<Expression> {
        self.resolve_template_object_reference_identity_expression_inner(
            expression,
            &mut HashSet::new(),
        )
    }

    fn prototype_member_reference_identity_key(
        &self,
        object: &Expression,
        property: &Expression,
    ) -> Option<String> {
        if !matches!(property, Expression::String(name) if name == "prototype") {
            return None;
        }
        if let Expression::Member {
            object: constructor_object,
            property: constructor_property,
        } = object
            && let Expression::String(constructor_name) = constructor_property.as_ref()
            && builtin_identifier_kind(constructor_name) == Some(StaticValueKind::Function)
            && let Some(realm_id) =
                self.resolve_test262_realm_global_id_from_expression(constructor_object)
        {
            return Some(format!(
                "test262-realm:{realm_id}:function-prototype:{constructor_name}"
            ));
        }
        if let Some(owner) = self
            .resolve_function_binding_from_expression(object)
            .and_then(|binding| self.function_prototype_binding_owner_name(&binding))
        {
            return Some(format!("function-prototype:{owner}"));
        }
        if let Expression::Identifier(name) = object
            && (builtin_identifier_kind(name) == Some(StaticValueKind::Function)
                || infer_call_result_kind(name).is_some()
                || self.backend.global_has_prototype_object_binding(name)
                || self.global_object_prototype_expression(name).is_some())
        {
            return Some(format!("function-prototype:{name}"));
        }
        self.resolve_static_reference_identity_key(object)
            .map(|object_key| format!("{object_key}.prototype"))
    }

    pub(in crate::backend::direct_wasm) fn resolve_static_reference_identity_key(
        &self,
        expression: &Expression,
    ) -> Option<String> {
        if matches!(expression, Expression::This) {
            return Some("this".to_string());
        }
        if matches!(expression, Expression::Identifier(name) if name == "globalThis" && self.is_unshadowed_builtin_identifier(name))
        {
            return Some("this".to_string());
        }

        if let Some(key) = import_meta_reference_identity_key(expression) {
            return Some(key);
        }

        if let Some(key) = template_object_reference_identity_key(expression) {
            return Some(key);
        }

        if let Expression::New { callee, arguments } = expression
            && let Some(key) =
                self.constructed_object_reference_identity_key(expression, callee, arguments)
        {
            return Some(key);
        }

        if let Expression::GetIterator(iterated) = expression {
            if let Some(object_binding) = self.resolve_object_binding_from_expression(iterated) {
                if object_binding_lookup_value(
                    &object_binding,
                    &Expression::String("next".to_string()),
                )
                .and_then(|value| self.resolve_function_binding_from_expression(value))
                .is_some()
                {
                    return self.resolve_static_reference_identity_key(iterated);
                }
                let iterator_property =
                    self.materialize_static_expression(&symbol_iterator_expression());
                if let Some(iterator_method) =
                    object_binding_lookup_value(&object_binding, &iterator_property)
                    && let Some(iterator_function) =
                        self.resolve_function_binding_from_expression(iterator_method)
                    && let Some(return_value) = self
                        .resolve_function_binding_static_return_expression_with_call_frame(
                            &iterator_function,
                            &[],
                            iterated,
                        )
                    && let Some(key) = self.resolve_static_reference_identity_key(&return_value)
                {
                    return Some(key);
                }
            }

            let iterator_call = Expression::Call {
                callee: Box::new(Expression::Member {
                    object: Box::new((**iterated).clone()),
                    property: Box::new(
                        self.materialize_static_expression(&symbol_iterator_expression()),
                    ),
                }),
                arguments: Vec::new(),
            };
            if let Some(key) = self.resolve_static_reference_identity_key(&iterator_call) {
                return Some(key);
            }
        }

        if let Some((resolved, callee_function_name)) = match expression {
            Expression::Call { callee, arguments } => self
                .resolve_static_call_result_expression_with_context(
                    callee,
                    arguments,
                    self.current_function_name(),
                ),
            _ => None,
        } && !static_expression_matches(&resolved, expression)
            && let Some(key) =
                self.resolve_static_reference_identity_key(&if let Expression::Call {
                    callee, ..
                } = expression
                    && let Expression::Member { object, property } = callee.as_ref()
                    && let Some(capture_slots) =
                        self.resolve_member_function_capture_slots(object, property)
                {
                    self.substitute_capture_slot_bindings(&resolved, &capture_slots)
                } else {
                    resolved
                })
        {
            let _ = callee_function_name;
            return Some(key);
        }

        if let Some(resolved) = self.resolve_bound_alias_expression(expression)
            && !static_expression_matches(&resolved, expression)
            && let Some(key) = self.resolve_static_reference_identity_key(&resolved)
        {
            return Some(key);
        }

        if let Expression::Call { callee, arguments } = expression
            && let Expression::Member { object, property } = callee.as_ref()
            && matches!(object.as_ref(), Expression::Identifier(name) if name == "Object")
            && matches!(property.as_ref(), Expression::String(name) if name == "getPrototypeOf")
            && let Some(CallArgument::Expression(target) | CallArgument::Spread(target)) =
                arguments.first()
            && let Some(prototype) = self.resolve_static_object_prototype_expression(target)
            && let Some(key) = self.resolve_static_reference_identity_key(&prototype)
        {
            return Some(key);
        }

        if let Expression::Member { object, property } = expression
            && let Some(key) = self.prototype_member_reference_identity_key(object, property)
        {
            return Some(key);
        }

        if let Expression::Member { object, property } = expression
            && let Some(binding) = self.resolve_member_function_binding(object, property)
        {
            return Some(match binding {
                LocalFunctionBinding::User(function_name) => {
                    format!("user-function:{function_name}")
                }
                LocalFunctionBinding::Builtin(function_name) => {
                    format!("builtin-function:{function_name}")
                }
            });
        }

        if let Expression::Member { object, property } = expression
            && matches!(property.as_ref(), Expression::String(name) if name == "constructor")
            && let Some(binding) = self.resolve_constructed_object_constructor_binding(object)
        {
            return Some(match binding {
                LocalFunctionBinding::User(function_name) => {
                    format!("user-function:{function_name}")
                }
                LocalFunctionBinding::Builtin(function_name) => {
                    format!("builtin-function:{function_name}")
                }
            });
        }

        if let Expression::Member { object, property } = expression
            && let Some(object_binding) = self.resolve_object_binding_from_expression(object)
            && let Some(value) =
                self.resolve_object_binding_property_value(&object_binding, property)
            && let Some(key) = self.resolve_static_reference_identity_key(&value)
        {
            return Some(key);
        }

        if let Expression::Identifier(name) = expression
            && let Some(key) = self.reference_identity_key_for_identifier(name)
        {
            return Some(key);
        }

        if let Some(function) = self.resolve_user_function_from_expression(expression) {
            return Some(format!("user-function:{}", function.name));
        }

        match expression {
            Expression::This => Some("this".to_string()),
            _ => self
                .resolve_user_function_from_expression(expression)
                .map(|function| format!("user-function:{}", function.name)),
        }
    }

    pub(in crate::backend::direct_wasm) fn reference_identity_key_for_identifier(
        &self,
        name: &str,
    ) -> Option<String> {
        let current_local_binding = self.resolve_current_local_binding(name);
        let resolved_name = current_local_binding
            .as_ref()
            .map(|(resolved_name, _)| resolved_name.clone())
            .unwrap_or_else(|| name.to_string());
        let local_value_binding = self
            .state
            .speculation
            .static_semantics
            .local_value_binding(&resolved_name)
            .filter(|value| {
                !matches!(
                    value,
                    Expression::Identifier(alias)
                        if alias == name || alias == &resolved_name
                )
            });
        if let Some(value) = local_value_binding {
            if expression_is_dynamic_import_call(value) {
                return Some(format!("local:{resolved_name}"));
            }
            if let Some(binding) = self.resolve_static_reference_identity_key(value) {
                return Some(binding);
            }
        }
        let should_prefer_global_value_alias = self.global_has_binding(name)
            && (self.state.speculation.execution_context.top_level_function
                || current_local_binding.is_none());
        if should_prefer_global_value_alias
            && let Some(value) = self
                .global_value_binding(name)
                .filter(|value| !matches!(value, Expression::Identifier(alias) if alias == name))
        {
            if std::env::var_os("AYY_TRACE_REFERENCE_IDENTITY").is_some() {
                eprintln!("reference_identity:identifier {name}:prefer_global value={value:?}");
            }
            if expression_is_dynamic_import_call(value) {
                return Some(format!("global:{name}"));
            }
            if let Some(binding) = self.resolve_static_reference_identity_key(value) {
                if std::env::var_os("AYY_TRACE_REFERENCE_IDENTITY").is_some() {
                    eprintln!("reference_identity:identifier {name}:prefer_global key={binding}");
                }
                return Some(binding);
            }
        }
        if let Some((resolved_name, _)) = self.resolve_current_local_binding(name)
            && (self
                .state
                .speculation
                .static_semantics
                .has_local_array_binding(&resolved_name)
                || self
                    .state
                    .speculation
                    .static_semantics
                    .has_local_object_binding(&resolved_name)
                || self
                    .state
                    .speculation
                    .static_semantics
                    .has_local_function_binding(&resolved_name))
        {
            return Some(format!("local:{resolved_name}"));
        }
        if self
            .state
            .speculation
            .static_semantics
            .has_local_array_binding(name)
            || self
                .state
                .speculation
                .static_semantics
                .has_local_object_binding(name)
            || self
                .state
                .speculation
                .static_semantics
                .has_local_function_binding(name)
        {
            return Some(format!("local:{name}"));
        }
        if let Some(value) = self
            .global_value_binding(name)
            .filter(|value| !matches!(value, Expression::Identifier(alias) if alias == name))
        {
            if std::env::var_os("AYY_TRACE_REFERENCE_IDENTITY").is_some() {
                eprintln!("reference_identity:identifier {name}:global value={value:?}");
            }
            if expression_is_dynamic_import_call(value) {
                return Some(format!("global:{name}"));
            }
            if let Some(binding) = self.resolve_static_reference_identity_key(value) {
                if std::env::var_os("AYY_TRACE_REFERENCE_IDENTITY").is_some() {
                    eprintln!("reference_identity:identifier {name}:global key={binding}");
                }
                return Some(binding);
            }
        }
        if let Some(binding) = self.backend.global_function_binding(name) {
            return Some(match binding {
                LocalFunctionBinding::User(function_name) => {
                    format!("user-function:{function_name}")
                }
                LocalFunctionBinding::Builtin(function_name) => {
                    format!("builtin-function:{function_name}")
                }
            });
        }
        if self.is_unshadowed_builtin_identifier(name)
            && builtin_identifier_kind(name) == Some(StaticValueKind::Function)
        {
            return Some(format!("builtin-function:{name}"));
        }
        if self.backend.global_array_binding(name).is_some()
            || self.backend.global_object_binding(name).is_some()
        {
            return Some(format!("global:{name}"));
        }
        None
    }

    pub(in crate::backend::direct_wasm) fn resolve_static_object_identity_expression(
        &self,
        expression: &Expression,
    ) -> Option<Expression> {
        if self
            .resolve_static_object_prototype_expression(expression)
            .is_none()
        {
            return None;
        }
        if let Some(resolved) = self
            .resolve_bound_alias_expression(expression)
            .filter(|resolved| !static_expression_matches(resolved, expression))
        {
            return self.resolve_static_object_identity_expression(&resolved);
        }
        match expression {
            Expression::Array(_)
            | Expression::Object(_)
            | Expression::New { .. }
            | Expression::Member { .. }
            | Expression::This => Some(expression.clone()),
            Expression::Call { .. }
                if self
                    .resolve_static_weakref_target_expression(expression)
                    .is_some()
                    || self.expression_is_known_promise_instance_for_instanceof(expression) =>
            {
                Some(expression.clone())
            }
            Expression::Identifier(_) => Some(expression.clone()),
            _ => {
                let materialized = self.materialize_static_expression(expression);
                if !static_expression_matches(&materialized, expression) {
                    self.resolve_static_object_identity_expression(&materialized)
                } else {
                    None
                }
            }
        }
    }
}
