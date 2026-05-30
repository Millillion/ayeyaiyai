use super::*;

impl<'a> FunctionCompiler<'a> {
    pub(in crate::backend::direct_wasm) fn simple_generator_has_eager_call_time_prefix(
        &self,
        expression: &Expression,
    ) -> bool {
        matches!(expression, Expression::Call { .. })
            && self
                .simple_generator_call_time_prefix_effects(expression)
                .is_some_and(|effects| !effects.is_empty())
    }

    pub(in crate::backend::direct_wasm) fn is_async_generator_iterator_expression(
        &self,
        expression: &Expression,
    ) -> bool {
        self.is_async_generator_iterator_expression_with_seen(
            expression,
            &mut std::collections::HashSet::new(),
        )
    }

    fn is_async_generator_iterator_expression_with_seen(
        &self,
        expression: &Expression,
        seen: &mut std::collections::HashSet<String>,
    ) -> bool {
        if !seen.insert(format!("{expression:?}")) {
            return false;
        }
        if let Some(resolved) = self
            .resolve_bound_alias_expression(expression)
            .filter(|resolved| !static_expression_matches(resolved, expression))
        {
            return self.is_async_generator_iterator_expression_with_seen(&resolved, seen);
        }
        if let Expression::Identifier(name) = expression
            && let Some(value) = self
                .state
                .speculation
                .static_semantics
                .local_value_binding(name)
                .or_else(|| self.global_value_binding(name))
            && !static_expression_matches(value, expression)
        {
            return self.is_async_generator_iterator_expression_with_seen(value, seen);
        }
        let materialized = self.materialize_static_expression(expression);
        if !static_expression_matches(&materialized, expression) {
            return self.is_async_generator_iterator_expression_with_seen(&materialized, seen);
        }

        let Expression::Call { callee, .. } = expression else {
            return false;
        };
        if self.simple_generator_has_eager_call_time_prefix(expression) {
            return false;
        }
        let Some(LocalFunctionBinding::User(function_name)) =
            self.resolve_function_binding_from_expression(callee)
        else {
            return false;
        };
        self.user_function(&function_name)
            .is_some_and(|function| matches!(function.kind, FunctionKind::AsyncGenerator))
    }

    pub(in crate::backend::direct_wasm) fn simple_generator_source_metadata(
        &self,
        object: &Expression,
    ) -> Option<(bool, Vec<SimpleGeneratorStep>, Vec<Statement>, Expression)> {
        self.simple_generator_source_metadata_with_seen(
            object,
            &mut std::collections::HashSet::new(),
        )
    }

    fn simple_generator_source_metadata_with_seen(
        &self,
        object: &Expression,
        seen: &mut std::collections::HashSet<String>,
    ) -> Option<(bool, Vec<SimpleGeneratorStep>, Vec<Statement>, Expression)> {
        if !seen.insert(format!("{object:?}")) {
            return None;
        }
        if self.simple_generator_has_eager_call_time_prefix(object) {
            return None;
        }
        if let Expression::Identifier(name) = object
            && let Some(binding_name) = self.resolve_local_array_iterator_binding_name(name)
            && let Some(ArrayIteratorBinding {
                source:
                    IteratorSourceKind::SimpleGenerator {
                        is_async,
                        steps,
                        completion_effects,
                        completion_value,
                    },
                ..
            }) = self
                .state
                .speculation
                .static_semantics
                .local_array_iterator_binding(&binding_name)
        {
            return Some((
                *is_async,
                steps.clone(),
                completion_effects.clone(),
                completion_value.clone(),
            ));
        }
        if let Expression::Identifier(name) = object
            && let Some(value) = self
                .state
                .speculation
                .static_semantics
                .local_value_binding(name)
                .or_else(|| self.global_value_binding(name))
            && !static_expression_matches(value, object)
        {
            return self.simple_generator_source_metadata_with_seen(value, seen);
        }
        if let Expression::Call { callee, .. } = object
            && let Some(LocalFunctionBinding::User(function_name)) =
                self.resolve_function_binding_from_expression(callee)
            && let Some(user_function) = self.user_function(&function_name)
        {
            let (steps, completion_effects, completion_value) =
                self.resolve_simple_generator_source(object)?;
            return Some((
                matches!(user_function.kind, FunctionKind::AsyncGenerator),
                steps,
                completion_effects,
                completion_value,
            ));
        }
        let materialized = self.materialize_static_expression(object);
        if !static_expression_matches(&materialized, object) {
            return self.simple_generator_source_metadata_with_seen(&materialized, seen);
        }

        let Expression::Call { callee, .. } = object else {
            return None;
        };
        let LocalFunctionBinding::User(function_name) =
            self.resolve_function_binding_from_expression(callee)?
        else {
            return None;
        };
        let user_function = self.user_function(&function_name)?;
        let (steps, completion_effects, completion_value) =
            self.resolve_simple_generator_source(object)?;
        Some((
            matches!(user_function.kind, FunctionKind::AsyncGenerator),
            steps,
            completion_effects,
            completion_value,
        ))
    }

    pub(in crate::backend::direct_wasm) fn simple_generator_source_function_name(
        &self,
        object: &Expression,
    ) -> Option<String> {
        self.simple_generator_source_function_name_with_seen(
            object,
            &mut std::collections::HashSet::new(),
        )
    }

    fn simple_generator_source_function_name_with_seen(
        &self,
        object: &Expression,
        seen: &mut std::collections::HashSet<String>,
    ) -> Option<String> {
        if !seen.insert(format!("{object:?}")) {
            return None;
        }
        if let Some(resolved) = self
            .resolve_bound_alias_expression(object)
            .filter(|resolved| !static_expression_matches(resolved, object))
        {
            return self.simple_generator_source_function_name_with_seen(&resolved, seen);
        }
        if let Expression::Identifier(name) = object
            && let Some(value) = self
                .state
                .speculation
                .static_semantics
                .local_value_binding(name)
                .or_else(|| self.global_value_binding(name))
            && !static_expression_matches(value, object)
        {
            return self.simple_generator_source_function_name_with_seen(value, seen);
        }

        if let Expression::Call { callee, .. } = object {
            let binding = self.resolve_function_binding_from_expression(callee);
            return match binding? {
                LocalFunctionBinding::User(function_name) => Some(function_name),
                LocalFunctionBinding::Builtin(_) => None,
            };
        }

        let materialized = self.materialize_static_expression(object);
        if !static_expression_matches(&materialized, object) {
            return self.simple_generator_source_function_name_with_seen(&materialized, seen);
        }

        let Expression::Call { callee, .. } = object else {
            return None;
        };
        match self.resolve_function_binding_from_expression(callee)? {
            LocalFunctionBinding::User(function_name) => Some(function_name),
            LocalFunctionBinding::Builtin(_) => None,
        }
    }
}
