use super::*;

thread_local! {
    static PROMISE_INSTANCE_CLASSIFICATION_STACK: std::cell::RefCell<Vec<Expression>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

impl<'a> FunctionCompiler<'a> {
    fn with_promise_instance_classification_guard(
        &self,
        expression: &Expression,
        f: impl FnOnce(&Self) -> bool,
    ) -> bool {
        let reentered = PROMISE_INSTANCE_CLASSIFICATION_STACK.with(|stack| {
            stack
                .borrow()
                .iter()
                .any(|visited| static_expression_matches(visited, expression))
        });
        if reentered {
            return false;
        }

        PROMISE_INSTANCE_CLASSIFICATION_STACK.with(|stack| {
            stack.borrow_mut().push(expression.clone());
        });
        let result = f(self);
        PROMISE_INSTANCE_CLASSIFICATION_STACK.with(|stack| {
            stack.borrow_mut().pop();
        });
        result
    }

    pub(in crate::backend::direct_wasm) fn expression_is_builtin_array_constructor(
        &self,
        expression: &Expression,
    ) -> bool {
        matches!(
            self.materialize_static_expression(expression),
            Expression::Identifier(name) if name == "Array"
        )
    }

    pub(in crate::backend::direct_wasm) fn expression_is_known_array_value(
        &self,
        expression: &Expression,
    ) -> bool {
        if self
            .resolve_array_binding_from_expression(expression)
            .is_some()
        {
            return true;
        }

        let materialized = self.materialize_static_expression(expression);
        if !static_expression_matches(&materialized, expression)
            && self
                .resolve_array_binding_from_expression(&materialized)
                .is_some()
        {
            return true;
        }

        self.resolve_bound_alias_expression(expression)
            .filter(|resolved| !static_expression_matches(resolved, expression))
            .is_some_and(|resolved| self.expression_is_known_array_value(&resolved))
    }

    pub(in crate::backend::direct_wasm) fn expression_is_known_non_object_value_for_instanceof(
        &self,
        expression: &Expression,
    ) -> bool {
        if self.expression_is_known_array_value(expression)
            || self.expression_is_known_function_value_for_instanceof(expression)
            || self.expression_is_known_generator_instance_for_instanceof(expression)
            || self.expression_is_known_promise_instance_for_instanceof(expression)
            || self.expression_is_known_constructor_instance_for_instanceof(expression, "WeakMap")
            || self.expression_is_known_constructor_instance_for_instanceof(expression, "WeakRef")
            || self.expression_is_known_constructor_instance_for_instanceof(expression, "WeakSet")
            || self.expression_is_known_native_error_instance_for_instanceof(expression, "Error")
        {
            return false;
        }
        if let Some(resolved) = self
            .resolve_bound_alias_expression(expression)
            .filter(|resolved| !static_expression_matches(resolved, expression))
        {
            return self.expression_is_known_non_object_value_for_instanceof(&resolved);
        }
        let materialized = self.materialize_static_expression(expression);
        if !static_expression_matches(&materialized, expression) {
            return self.expression_is_known_non_object_value_for_instanceof(&materialized);
        }
        if self
            .resolve_object_binding_from_expression(&materialized)
            .is_some()
        {
            return false;
        }
        matches!(
            self.infer_value_kind(&materialized),
            Some(
                StaticValueKind::Number
                    | StaticValueKind::Bool
                    | StaticValueKind::String
                    | StaticValueKind::BigInt
                    | StaticValueKind::Symbol
                    | StaticValueKind::Null
                    | StaticValueKind::Undefined
            )
        )
    }

    pub(in crate::backend::direct_wasm) fn expression_is_known_function_value_for_instanceof(
        &self,
        expression: &Expression,
    ) -> bool {
        if self
            .resolve_function_binding_from_expression(expression)
            .is_some()
        {
            return true;
        }
        if matches!(
            expression,
            Expression::Member { object, property }
                if matches!(object.as_ref(), Expression::Identifier(name) if name == "Function")
                    && matches!(property.as_ref(), Expression::String(name) if name == "prototype")
        ) {
            return true;
        }
        if let Some(resolved) = self
            .resolve_bound_alias_expression(expression)
            .filter(|resolved| !static_expression_matches(resolved, expression))
        {
            return self.expression_is_known_function_value_for_instanceof(&resolved);
        }
        if matches!(
            expression,
            Expression::Call { callee, .. }
                if matches!(callee.as_ref(), Expression::Identifier(name)
                    if is_function_constructor_builtin(name))
        ) {
            return true;
        }
        let materialized = self.materialize_static_expression(expression);
        if !static_expression_matches(&materialized, expression) {
            return self.expression_is_known_function_value_for_instanceof(&materialized);
        }
        matches!(
            self.infer_value_kind(&materialized),
            Some(StaticValueKind::Function)
        ) || matches!(
            materialized,
            Expression::Call { ref callee, .. }
                if matches!(callee.as_ref(), Expression::Identifier(name)
                    if is_function_constructor_builtin(name))
        )
    }

    pub(in crate::backend::direct_wasm) fn expression_is_known_generator_instance_for_instanceof(
        &self,
        expression: &Expression,
    ) -> bool {
        if let Some(resolved) = self
            .resolve_bound_alias_expression(expression)
            .filter(|resolved| !static_expression_matches(resolved, expression))
        {
            return self.expression_is_known_generator_instance_for_instanceof(&resolved);
        }
        if let Expression::Call { callee, .. } = expression
            && self
                .resolve_user_function_from_expression(callee)
                .is_some_and(|user_function| user_function.is_generator())
        {
            return true;
        }
        let materialized = self.materialize_static_expression(expression);
        if !static_expression_matches(&materialized, expression) {
            return self.expression_is_known_generator_instance_for_instanceof(&materialized);
        }
        false
    }

    pub(in crate::backend::direct_wasm) fn expression_is_known_promise_instance_for_instanceof(
        &self,
        expression: &Expression,
    ) -> bool {
        self.with_promise_instance_classification_guard(expression, |this| {
            this.expression_is_known_promise_instance_for_instanceof_inner(expression)
        })
    }

    fn expression_is_known_promise_instance_for_instanceof_inner(
        &self,
        expression: &Expression,
    ) -> bool {
        if let Expression::Identifier(name) = expression {
            let resolved_local_name = self
                .resolve_current_local_binding(name)
                .map(|(resolved_name, _)| resolved_name);
            let bound_value = resolved_local_name
                .as_deref()
                .and_then(|resolved_name| {
                    self.state
                        .speculation
                        .static_semantics
                        .local_value_binding(resolved_name)
                })
                .or_else(|| {
                    resolved_local_name
                        .is_none()
                        .then(|| self.global_value_binding(name))
                        .flatten()
                });
            if let Some(bound_value) = bound_value
                && !static_expression_matches(bound_value, expression)
                && self.expression_is_known_promise_instance_for_instanceof(bound_value)
            {
                return true;
            }
            if self.global_object_prototype_expression(name).is_some_and(|prototype| {
                matches!(
                    prototype,
                    Expression::Member { object, property }
                        if matches!(object.as_ref(), Expression::Identifier(owner) if owner == "Promise")
                            && matches!(property.as_ref(), Expression::String(property_name) if property_name == "prototype")
                )
            }) {
                return true;
            }
        }
        if let Some(resolved) = self
            .resolve_bound_alias_expression(expression)
            .filter(|resolved| !static_expression_matches(resolved, expression))
        {
            return self.expression_is_known_promise_instance_for_instanceof(&resolved);
        }
        match expression {
            Expression::New { callee, .. } => {
                return matches!(callee.as_ref(), Expression::Identifier(name) if name == "Promise");
            }
            Expression::Call { callee, .. } => {
                if matches!(
                    callee.as_ref(),
                    Expression::Identifier(name)
                        if matches!(name.as_str(), "Promise" | "__ayyDynamicImport")
                ) {
                    return true;
                }
                if matches!(
                    callee.as_ref(),
                    Expression::Member { object, property }
                        if matches!(object.as_ref(), Expression::Identifier(name) if name == "Promise")
                            && matches!(property.as_ref(), Expression::String(name) if name == "resolve")
                ) {
                    return true;
                }
                if self
                    .resolve_user_function_from_expression(callee.as_ref())
                    .is_some_and(|user_function| user_function.is_async())
                {
                    return true;
                }
            }
            _ => {}
        }
        let materialized = self.materialize_static_expression(expression);
        if !static_expression_matches(&materialized, expression) {
            return self.expression_is_known_promise_instance_for_instanceof(&materialized);
        }
        match materialized {
            Expression::New { callee, .. } => {
                matches!(callee.as_ref(), Expression::Identifier(name) if name == "Promise")
            }
            Expression::Call { callee, .. } => {
                if matches!(
                    callee.as_ref(),
                    Expression::Identifier(name)
                        if matches!(name.as_str(), "Promise" | "__ayyDynamicImport")
                ) {
                    return true;
                }
                if matches!(
                    callee.as_ref(),
                    Expression::Member { object, property }
                        if matches!(object.as_ref(), Expression::Identifier(name) if name == "Promise")
                            && matches!(property.as_ref(), Expression::String(name) if name == "resolve")
                ) {
                    return true;
                }
                self.resolve_user_function_from_expression(callee.as_ref())
                    .is_some_and(|user_function| user_function.is_async())
            }
            _ => false,
        }
    }

    pub(in crate::backend::direct_wasm) fn expression_is_known_constructor_instance_for_instanceof(
        &self,
        expression: &Expression,
        constructor_name: &str,
    ) -> bool {
        if let Some(resolved) = self
            .resolve_bound_alias_expression(expression)
            .filter(|resolved| !static_expression_matches(resolved, expression))
        {
            return self.expression_is_known_constructor_instance_for_instanceof(
                &resolved,
                constructor_name,
            );
        }
        match expression {
            Expression::New { callee, .. } => {
                return matches!(callee.as_ref(), Expression::Identifier(name) if name == constructor_name);
            }
            Expression::Call { callee, .. } => {
                return matches!(callee.as_ref(), Expression::Identifier(name) if name == constructor_name)
                    && (constructor_name == "AggregateError"
                        || native_error_runtime_value(constructor_name).is_some());
            }
            _ => {}
        }
        let materialized = self.materialize_static_expression(expression);
        if !static_expression_matches(&materialized, expression) {
            return self.expression_is_known_constructor_instance_for_instanceof(
                &materialized,
                constructor_name,
            );
        }
        match materialized {
            Expression::New { callee, .. } => {
                matches!(callee.as_ref(), Expression::Identifier(name) if name == constructor_name)
            }
            Expression::Call { callee, .. } => {
                matches!(callee.as_ref(), Expression::Identifier(name) if name == constructor_name)
                    && (constructor_name == "AggregateError"
                        || native_error_runtime_value(constructor_name).is_some())
            }
            _ => false,
        }
    }

    pub(in crate::backend::direct_wasm) fn expression_is_known_native_error_instance_for_instanceof(
        &self,
        expression: &Expression,
        constructor_name: &str,
    ) -> bool {
        if constructor_name == "Error" {
            return NATIVE_ERROR_NAMES.iter().any(|candidate| {
                self.expression_is_known_constructor_instance_for_instanceof(expression, candidate)
            });
        }
        self.expression_is_known_constructor_instance_for_instanceof(expression, constructor_name)
    }

    pub(in crate::backend::direct_wasm) fn expression_is_known_object_like_value_for_instanceof(
        &self,
        expression: &Expression,
    ) -> bool {
        if self.expression_is_known_array_value(expression)
            || self.expression_is_known_function_value_for_instanceof(expression)
            || self.expression_is_known_generator_instance_for_instanceof(expression)
            || self.expression_is_known_promise_instance_for_instanceof(expression)
            || self.expression_is_known_constructor_instance_for_instanceof(expression, "WeakMap")
            || self.expression_is_known_constructor_instance_for_instanceof(expression, "WeakRef")
            || self.expression_is_known_constructor_instance_for_instanceof(expression, "WeakSet")
            || self.expression_is_known_native_error_instance_for_instanceof(expression, "Error")
        {
            return true;
        }
        if let Some(resolved) = self
            .resolve_bound_alias_expression(expression)
            .filter(|resolved| !static_expression_matches(resolved, expression))
        {
            return self.expression_is_known_object_like_value_for_instanceof(&resolved);
        }
        let materialized = self.materialize_static_expression(expression);
        if !static_expression_matches(&materialized, expression) {
            return self.expression_is_known_object_like_value_for_instanceof(&materialized);
        }
        let object_binding = self.resolve_object_binding_from_expression(&materialized);
        if std::env::var_os("AYY_TRACE_INSTANCEOF").is_some() {
            eprintln!(
                "instanceof:object_like expression={expression:?} materialized={materialized:?} object_binding={} kind={:?}",
                object_binding.is_some(),
                self.infer_value_kind(&materialized)
            );
        }
        if object_binding
            .as_ref()
            .is_some_and(Self::object_binding_has_module_namespace_marker)
        {
            return false;
        }
        if object_binding.is_some() {
            return true;
        }
        matches!(
            self.infer_value_kind(&materialized),
            Some(StaticValueKind::Object)
        )
    }
}
