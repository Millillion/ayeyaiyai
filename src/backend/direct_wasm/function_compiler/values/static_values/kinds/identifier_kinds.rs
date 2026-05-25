use super::*;

impl<'a> FunctionCompiler<'a> {
    pub(in crate::backend::direct_wasm) fn infer_proxy_target_typeof_kind(
        &self,
        target: &Expression,
    ) -> StaticValueKind {
        if let Some(proxy_binding) = self.resolve_proxy_binding_from_expression(target) {
            return self.infer_proxy_target_typeof_kind(&proxy_binding.target);
        }
        if self
            .resolve_function_binding_from_expression(target)
            .is_some()
            || self.infer_value_kind(target) == Some(StaticValueKind::Function)
        {
            StaticValueKind::Function
        } else {
            StaticValueKind::Object
        }
    }

    pub(in crate::backend::direct_wasm) fn proxy_revocable_result_target_expression(
        &self,
        expression: &Expression,
    ) -> Option<Expression> {
        let static_call_result = |callee: &Expression, arguments: &[CallArgument]| {
            self.resolve_static_call_result_expression_with_context(
                callee,
                arguments,
                self.current_function_name(),
            )
            .map(|(value, _)| value)
        };
        let candidate = match expression {
            Expression::Call { callee, arguments } => {
                static_call_result(callee, arguments).unwrap_or_else(|| expression.clone())
            }
            Expression::Identifier(name) => self
                .state
                .speculation
                .static_semantics
                .local_value_binding(name)
                .or_else(|| self.global_value_binding(name))
                .cloned()
                .unwrap_or_else(|| expression.clone()),
            Expression::Member { object, property } => {
                let materialized = self.materialize_static_expression(expression);
                if static_expression_matches(&materialized, expression)
                    && let Expression::Call { callee, arguments } = object.as_ref()
                    && let Some(result) = static_call_result(callee, arguments)
                {
                    Expression::Member {
                        object: Box::new(result),
                        property: property.clone(),
                    }
                } else {
                    materialized
                }
            }
            _ => {
                let materialized = self.materialize_static_expression(expression);
                if static_expression_matches(&materialized, expression)
                    && let Expression::Call { callee, arguments } = expression
                    && let Some(result) = static_call_result(callee, arguments)
                {
                    result
                } else {
                    materialized
                }
            }
        };
        match candidate {
            Expression::Call { callee, arguments } => {
                if let Expression::Member { object, property } = callee.as_ref()
                    && matches!(object.as_ref(), Expression::Identifier(name) if name == "Proxy" && self.is_unshadowed_builtin_identifier(name))
                    && matches!(property.as_ref(), Expression::String(name) if name == "revocable")
                    && let Some(CallArgument::Expression(target) | CallArgument::Spread(target)) =
                        arguments.first()
                {
                    return Some(self.materialize_static_expression(target));
                }
            }
            Expression::Object(entries) => {
                for entry in entries {
                    let ObjectEntry::Data { key, value } = entry else {
                        continue;
                    };
                    if !matches!(key, Expression::String(name) if name == "proxy") {
                        continue;
                    }
                    if let Some(proxy_binding) = self.resolve_proxy_binding_from_expression(&value)
                    {
                        return Some(proxy_binding.target);
                    }
                }
            }
            _ => {}
        }
        None
    }

    pub(in crate::backend::direct_wasm) fn infer_typeof_operand_kind(
        &self,
        expression: &Expression,
    ) -> Option<StaticValueKind> {
        if let Some(proxy_binding) = self.resolve_proxy_binding_from_expression(expression) {
            return Some(self.infer_proxy_target_typeof_kind(&proxy_binding.target));
        }
        if let Expression::Member { object, property } = expression
            && matches!(property.as_ref(), Expression::String(name) if name == "proxy")
            && let Some(target) = self.proxy_revocable_result_target_expression(object)
        {
            return Some(self.infer_proxy_target_typeof_kind(&target));
        }
        let materialized = self.materialize_static_expression(expression);
        if !static_expression_matches(&materialized, expression) {
            if self.expression_is_static_boxed_primitive_object(&materialized) {
                return Some(StaticValueKind::Object);
            }
            return self.infer_typeof_operand_kind(&materialized);
        }
        match expression {
            Expression::Member { object, property } => {
                if let Some(value) = self.resolve_static_member_getter_value_with_context(
                    object,
                    property,
                    self.current_function_name(),
                ) {
                    if self.expression_is_static_boxed_primitive_object(&value) {
                        return Some(StaticValueKind::Object);
                    }
                    return self.infer_typeof_operand_kind(&value);
                }
                if let Some(getter_binding) =
                    self.resolve_member_getter_binding_shallow(object, property)
                {
                    if let Some(StaticEvalOutcome::Value(value)) = self
                        .resolve_static_function_outcome_from_binding_with_context(
                            &getter_binding,
                            &[],
                            self.current_function_name(),
                        )
                    {
                        return self.infer_typeof_operand_kind(&value);
                    }
                    return None;
                }
                self.infer_value_kind(expression)
            }
            Expression::This => self
                .state
                .speculation
                .execution_context
                .top_level_function
                .then_some(StaticValueKind::Object),
            Expression::Identifier(name)
                if name == "NaN" && self.is_unshadowed_builtin_identifier(name) =>
            {
                Some(StaticValueKind::Number)
            }
            Expression::Identifier(name) => self
                .lookup_identifier_kind(name)
                .or(Some(StaticValueKind::Undefined)),
            _ => self.infer_value_kind(expression),
        }
    }

    pub(in crate::backend::direct_wasm) fn lookup_identifier_kind(
        &self,
        name: &str,
    ) -> Option<StaticValueKind> {
        if self.with_scope_blocks_static_identifier_resolution(name) {
            return None;
        }
        self.lookup_identifier_kind_ignoring_with_scopes(name)
    }

    pub(in crate::backend::direct_wasm) fn lookup_identifier_kind_ignoring_with_scopes(
        &self,
        name: &str,
    ) -> Option<StaticValueKind> {
        if parse_test262_realm_identifier(name).is_some()
            || parse_test262_realm_global_identifier(name).is_some()
        {
            return Some(StaticValueKind::Object);
        }
        let identifier = Expression::Identifier(name.to_string());
        if let Some(resolved) = self.resolve_bound_alias_expression(&identifier)
            && !static_expression_matches(&resolved, &identifier)
            && let Some(kind) = self.infer_value_kind(&resolved)
            && kind != StaticValueKind::Unknown
        {
            return Some(kind);
        }
        if let Some((resolved_name, _)) = self.resolve_current_local_binding(name) {
            return Some(
                self.state
                    .speculation
                    .static_semantics
                    .local_kind(&resolved_name)
                    .unwrap_or(StaticValueKind::Unknown),
            );
        }
        if self.is_current_arguments_binding_name(name) && self.has_arguments_object() {
            return Some(StaticValueKind::Object);
        }
        if let Some(hidden_name) = self.resolve_user_function_capture_hidden_name(name)
            && let Some(kind) = self.global_binding_kind(&hidden_name)
        {
            return Some(kind);
        }
        if matches!(
            self.state
                .speculation
                .static_semantics
                .local_function_binding(name),
            Some(LocalFunctionBinding::User(_) | LocalFunctionBinding::Builtin(_))
        ) {
            return Some(StaticValueKind::Function);
        }
        if let Some(state) = self.backend.global_property_descriptor(name).or_else(|| {
            self.backend
                .shared_global_semantics
                .values
                .property_descriptor(name)
        }) {
            if state.has_get || state.getter.is_some() {
                return Some(StaticValueKind::Unknown);
            }
            return self
                .infer_value_kind(&state.value)
                .filter(|kind| *kind != StaticValueKind::Unknown)
                .or(Some(StaticValueKind::Unknown));
        }
        if let Some(kind) = self.global_binding_kind(name) {
            return Some(kind);
        }
        if self.implicit_global_binding(name).is_some() {
            return self
                .global_value_binding(name)
                .and_then(|value| self.infer_value_kind(value))
                .filter(|kind| *kind != StaticValueKind::Unknown)
                .or(Some(StaticValueKind::Unknown));
        }
        if self.resolve_eval_local_function_hidden_name(name).is_some() {
            return Some(
                self.state
                    .speculation
                    .static_semantics
                    .local_kind(name)
                    .unwrap_or(StaticValueKind::Unknown),
            );
        }
        if self.global_has_binding(name) {
            return Some(StaticValueKind::Unknown);
        }
        if self
            .state
            .runtime
            .locals
            .deleted_builtin_identifiers
            .contains(name)
        {
            return None;
        }
        if is_internal_user_function_identifier(name)
            && self
                .backend
                .function_registry
                .catalog
                .user_function(name)
                .is_some()
        {
            return Some(StaticValueKind::Function);
        }
        builtin_identifier_kind(name)
    }
}
