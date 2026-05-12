use super::*;

impl<'a> FunctionCompiler<'a> {
    pub(in crate::backend::direct_wasm) fn materialize_identifier_expression(
        &self,
        name: &str,
        expression: &Expression,
    ) -> Expression {
        let resolved_local_name = self
            .resolve_current_local_binding(name)
            .map(|(resolved_name, _)| resolved_name);
        let active_name = resolved_local_name.as_deref().unwrap_or(name);
        let has_current_local_binding = resolved_local_name.is_some();

        if self.with_scope_blocks_static_identifier_resolution(name) {
            return Expression::Identifier(name.to_string());
        }
        if name == "undefined" && self.is_unshadowed_builtin_identifier(name) {
            return Expression::Undefined;
        }
        if name.starts_with("__ayy_target_object_") {
            let immediate_target_alias = resolved_local_name
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
                .or_else(|| {
                    (!has_current_local_binding)
                        .then(|| self.global_value_binding(name))
                        .flatten()
                })
                .cloned();
            if let Some(alias @ (Expression::Identifier(_) | Expression::This)) =
                immediate_target_alias
            {
                return alias;
            }
        }
        if name.starts_with("__ayy_target_object_") || name.starts_with("__ayy_target_property_") {
            let resolved_target_alias = self.resolve_bound_alias_expression(expression);
            if std::env::var_os("AYY_TRACE_MEMBER_ASSIGNMENT").is_some() {
                eprintln!("materialize_target_temp name={name} alias={resolved_target_alias:?}");
            }
            if let Some(resolved) = resolved_target_alias
                && !static_expression_matches(&resolved, expression)
                && inline_summary_side_effect_free_expression(&resolved)
            {
                return self.materialize_static_expression(&resolved);
            }
        }
        if self
            .state
            .speculation
            .static_semantics
            .has_local_object_binding(active_name)
            || self
                .state
                .speculation
                .static_semantics
                .objects
                .local_prototype_object_bindings
                .contains_key(active_name)
            || (!has_current_local_binding
                && self
                    .backend
                    .global_semantics
                    .values
                    .object_bindings
                    .contains_key(name))
            || (!has_current_local_binding
                && self
                    .backend
                    .global_semantics
                    .values
                    .prototype_object_bindings
                    .contains_key(name))
        {
            return expression.clone();
        }
        if self
            .runtime_object_property_shadow_owner_name_for_identifier(name)
            .is_some()
        {
            return expression.clone();
        }
        if self
            .state
            .speculation
            .static_semantics
            .has_local_array_binding(active_name)
            || self
                .state
                .speculation
                .static_semantics
                .has_local_array_iterator_binding(active_name)
            || self
                .state
                .speculation
                .static_semantics
                .has_local_typed_array_view_binding(active_name)
            || (!has_current_local_binding
                && self
                    .backend
                    .global_semantics
                    .values
                    .array_bindings
                    .contains_key(name))
        {
            return expression.clone();
        }
        if let Some(symbol_identity) = self.resolve_symbol_identity_expression(expression) {
            return symbol_identity;
        }
        if resolved_local_name
            .as_deref()
            .and_then(|resolved_name| {
                self.state
                    .speculation
                    .static_semantics
                    .local_value_binding(resolved_name)
            })
            .or_else(|| {
                (!has_current_local_binding)
                    .then(|| self.global_value_binding(name))
                    .flatten()
            })
            .is_some_and(|value| {
                matches!(
                    value,
                    Expression::Call { callee, .. }
                        if matches!(callee.as_ref(), Expression::Identifier(symbol_name)
                            if symbol_name == "Symbol"
                                && self.is_unshadowed_builtin_identifier(symbol_name))
                )
            })
        {
            return Expression::Identifier(name.to_string());
        }
        if is_function_constructor_builtin(name) {
            let aliased_value = resolved_local_name
                .as_deref()
                .and_then(|resolved_name| {
                    self.state
                        .speculation
                        .static_semantics
                        .local_value_binding(resolved_name)
                })
                .or_else(|| {
                    (!has_current_local_binding)
                        .then(|| self.global_value_binding(name))
                        .flatten()
                });
            if let Some(Expression::Member { object, property }) = aliased_value
                && matches!(
                    self.materialize_get_prototype_of_constructor_member(object, property),
                    Some(Expression::Identifier(ref constructor_name)) if constructor_name == name
                )
            {
                return Expression::Identifier(name.to_string());
            }
        }
        if let Some(resolved) = self.resolve_bound_alias_expression(expression) {
            if !static_expression_matches(&resolved, expression) {
                if let Expression::Call { callee, .. } = &resolved
                    && let Some(LocalFunctionBinding::User(function_name)) =
                        self.resolve_function_binding_from_expression(callee)
                    && self
                        .user_function(&function_name)
                        .is_some_and(|user_function| user_function.is_generator())
                {
                    return Expression::Identifier(name.to_string());
                }
                if !inline_summary_side_effect_free_expression(&resolved) {
                    return Expression::Identifier(name.to_string());
                }
                if self.resolve_iterator_source_kind(&resolved).is_some() {
                    return Expression::Identifier(name.to_string());
                }
                let mut referenced_names = HashSet::new();
                collect_referenced_binding_names_from_expression(&resolved, &mut referenced_names);
                if referenced_names.contains(name) {
                    return Expression::Identifier(name.to_string());
                }
                return self.materialize_static_expression(&resolved);
            }
        }
        expression.clone()
    }
}
