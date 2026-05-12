use super::*;

impl<'a> FunctionCompiler<'a> {
    fn prototype_member_reference_identity_key(
        &self,
        object: &Expression,
        property: &Expression,
    ) -> Option<String> {
        if !matches!(property, Expression::String(name) if name == "prototype") {
            return None;
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

        if let Expression::Member { object, property } = expression
            && let Some(key) = self.prototype_member_reference_identity_key(object, property)
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
        if let Some(binding) = self
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
            })
            .and_then(|value| self.resolve_static_reference_identity_key(value))
        {
            return Some(binding);
        }
        let should_prefer_global_value_alias = self.global_has_binding(name)
            && (self.state.speculation.execution_context.top_level_function
                || current_local_binding.is_none());
        if should_prefer_global_value_alias
            && let Some(binding) = self
                .global_value_binding(name)
                .filter(|value| !matches!(value, Expression::Identifier(alias) if alias == name))
                .and_then(|value| self.resolve_static_reference_identity_key(value))
        {
            return Some(binding);
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
        if let Some(binding) = self
            .global_value_binding(name)
            .filter(|value| !matches!(value, Expression::Identifier(alias) if alias == name))
            .and_then(|value| self.resolve_static_reference_identity_key(value))
        {
            return Some(binding);
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
