use super::*;

impl<'a> FunctionCompiler<'a> {
    fn dynamic_property_descriptor_source_from_expression(
        &self,
        expression: &Expression,
    ) -> Option<(Expression, Expression)> {
        let Expression::Call { callee, arguments } = expression else {
            return None;
        };
        if !matches!(
            callee.as_ref(),
            Expression::Member { object, property }
                if matches!(object.as_ref(), Expression::Identifier(name) if name == "Object" || name == "Reflect")
                    && matches!(
                        property.as_ref(),
                        Expression::String(name) if name == "getOwnPropertyDescriptor"
                    )
        ) {
            return None;
        }
        let [
            CallArgument::Expression(target),
            CallArgument::Expression(property_name),
            ..,
        ] = arguments.as_slice()
        else {
            return None;
        };
        if !matches!(
            property_name,
            Expression::Identifier(identifier)
                if self.resolve_current_local_binding(identifier).is_some()
                    && self
                        .resolve_bound_alias_expression(property_name)
                        .filter(|resolved| !static_expression_matches(resolved, property_name))
                        .is_none()
                    && self
                        .resolve_symbol_identity_expression(property_name)
                        .is_none()
        ) {
            return None;
        }
        Some((target.clone(), property_name.clone()))
    }

    fn binding_name_matches_source_name(binding: &str, source_name: &str) -> bool {
        binding == source_name || scoped_binding_source_name(binding) == Some(source_name)
    }

    fn collect_dynamic_property_descriptor_sources_for_local(
        &self,
        statements: &[Statement],
        source_name: &str,
        sources: &mut Vec<(Expression, Expression)>,
        invalidated: &mut bool,
    ) {
        for statement in statements {
            match statement {
                Statement::Var { name, value }
                | Statement::Let { name, value, .. }
                | Statement::Assign { name, value }
                    if Self::binding_name_matches_source_name(name, source_name) =>
                {
                    if let Some(source) =
                        self.dynamic_property_descriptor_source_from_expression(value)
                    {
                        sources.push(source);
                    } else if !matches!(value, Expression::Undefined) {
                        *invalidated = true;
                    }
                }
                Statement::Declaration { body }
                | Statement::Block { body }
                | Statement::Labeled { body, .. }
                | Statement::With { body, .. }
                | Statement::While { body, .. }
                | Statement::DoWhile { body, .. } => self
                    .collect_dynamic_property_descriptor_sources_for_local(
                        body,
                        source_name,
                        sources,
                        invalidated,
                    ),
                Statement::If {
                    then_branch,
                    else_branch,
                    ..
                } => {
                    self.collect_dynamic_property_descriptor_sources_for_local(
                        then_branch,
                        source_name,
                        sources,
                        invalidated,
                    );
                    self.collect_dynamic_property_descriptor_sources_for_local(
                        else_branch,
                        source_name,
                        sources,
                        invalidated,
                    );
                }
                Statement::Try {
                    body,
                    catch_setup,
                    catch_body,
                    ..
                } => {
                    self.collect_dynamic_property_descriptor_sources_for_local(
                        body,
                        source_name,
                        sources,
                        invalidated,
                    );
                    self.collect_dynamic_property_descriptor_sources_for_local(
                        catch_setup,
                        source_name,
                        sources,
                        invalidated,
                    );
                    self.collect_dynamic_property_descriptor_sources_for_local(
                        catch_body,
                        source_name,
                        sources,
                        invalidated,
                    );
                }
                Statement::Switch { cases, .. } => {
                    for case in cases {
                        self.collect_dynamic_property_descriptor_sources_for_local(
                            &case.body,
                            source_name,
                            sources,
                            invalidated,
                        );
                    }
                }
                Statement::For { init, body, .. } => {
                    self.collect_dynamic_property_descriptor_sources_for_local(
                        init,
                        source_name,
                        sources,
                        invalidated,
                    );
                    self.collect_dynamic_property_descriptor_sources_for_local(
                        body,
                        source_name,
                        sources,
                        invalidated,
                    );
                }
                _ => {}
            }
        }
    }

    pub(in crate::backend::direct_wasm) fn dynamic_property_descriptor_source_for_local(
        &self,
        name: &str,
    ) -> Option<(Expression, Expression)> {
        let value = self
            .resolve_current_local_binding(name)
            .and_then(|(resolved_name, _)| {
                self.state
                    .speculation
                    .static_semantics
                    .local_value_binding(&resolved_name)
            })
            .or_else(|| {
                self.state
                    .speculation
                    .static_semantics
                    .local_value_binding(name)
            });
        if let Some(value) = value
            && let Some(source) = self.dynamic_property_descriptor_source_from_expression(value)
        {
            return Some(source);
        }

        let source_name = scoped_binding_source_name(name).unwrap_or(name);
        let mut sources = Vec::new();
        let mut invalidated = false;
        if let Some(function) = self.current_user_function_declaration() {
            self.collect_dynamic_property_descriptor_sources_for_local(
                &function.body,
                source_name,
                &mut sources,
                &mut invalidated,
            );
        }
        (!invalidated && sources.len() == 1).then(|| sources.remove(0))
    }

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

    pub(in crate::backend::direct_wasm) fn local_binding_is_dynamic_property_descriptor_result(
        &self,
        name: &str,
    ) -> bool {
        self.dynamic_property_descriptor_source_for_local(name)
            .is_some()
    }

    pub(in crate::backend::direct_wasm) fn infer_typeof_operand_kind(
        &self,
        expression: &Expression,
    ) -> Option<StaticValueKind> {
        if let Expression::Identifier(name) = expression {
            if let Some((resolved_name, _)) = self.resolve_current_local_binding(name)
                && self
                    .local_lexical_initialized_local(&resolved_name)
                    .is_some()
            {
                return None;
            }
            if self.user_function_capture_typeof_needs_runtime_check(name) {
                return None;
            }
        }
        if let Some(proxy_binding) = self.resolve_proxy_binding_from_expression(expression) {
            return Some(self.infer_proxy_target_typeof_kind(&proxy_binding.target));
        }
        if let Expression::Member { object, property } = expression
            && matches!(
                property.as_ref(),
                Expression::String(name)
                    if matches!(
                        name.as_str(),
                        "value" | "configurable" | "enumerable" | "writable" | "get" | "set"
                    )
            )
            && let Expression::Identifier(name) = object.as_ref()
            && self.local_binding_is_dynamic_property_descriptor_result(name)
        {
            return None;
        }
        if let Expression::Member { object, property } = expression
            && matches!(property.as_ref(), Expression::String(name) if name == "proxy")
            && let Some(target) = self.proxy_revocable_result_target_expression(object)
        {
            return Some(self.infer_proxy_target_typeof_kind(&target));
        }
        if let Expression::Member { object, property } = expression
            && self
                .resolve_module_namespace_live_binding_member_raw_value(object, property)
                .as_ref()
                .is_some_and(|value| {
                    self.module_namespace_live_binding_value_is_capture_slot(value)
                })
        {
            return None;
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
                let resolved_property = self
                    .resolve_property_key_expression(property)
                    .unwrap_or_else(|| self.materialize_static_expression(property));
                if let Some(value) = self
                    .resolve_module_namespace_live_binding_member_value(object, &resolved_property)
                {
                    if self.expression_is_static_boxed_primitive_object(&value) {
                        return Some(StaticValueKind::Object);
                    }
                    if let Some(kind) = self.infer_typeof_operand_kind(&value)
                        && kind != StaticValueKind::Unknown
                    {
                        return Some(kind);
                    }
                }
                if let Expression::Identifier(name) = object.as_ref()
                    && let Some(module_index) = Self::module_index_from_namespace_like_identifier(name)
                    && let Some(initializer) = self
                        .resolve_static_dynamic_import_namespace_live_binding_member_initializer_value(
                            module_index,
                            &resolved_property,
                        )
                {
                    if self.expression_is_static_boxed_primitive_object(&initializer) {
                        return Some(StaticValueKind::Object);
                    }
                    if let Some(kind) = self.infer_typeof_operand_kind(&initializer)
                        && kind != StaticValueKind::Unknown
                    {
                        return Some(kind);
                    }
                }
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
            Expression::Identifier(name)
                if self.with_scope_blocks_static_identifier_resolution(name) =>
            {
                None
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
