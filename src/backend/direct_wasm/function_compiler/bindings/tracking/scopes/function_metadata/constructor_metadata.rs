use super::*;

impl<'a> FunctionCompiler<'a> {
    fn resolve_iterator_step_typed_array_builtin_bytes_per_element(
        &self,
        expression: &Expression,
        depth: usize,
    ) -> Option<u32> {
        if depth > 6 {
            return None;
        }
        match expression {
            Expression::Identifier(name) => {
                if let Some(bytes_per_element) = typed_array_builtin_bytes_per_element(name) {
                    return Some(bytes_per_element);
                }
                if let Some(LocalFunctionBinding::Builtin(function_name)) = self
                    .state
                    .speculation
                    .static_semantics
                    .local_function_binding(name)
                    .cloned()
                    .or_else(|| self.backend.global_function_binding(name).cloned())
                    && let Some(bytes_per_element) =
                        typed_array_builtin_bytes_per_element(&function_name)
                {
                    return Some(bytes_per_element);
                }
                if let Some(value) = self
                    .state
                    .speculation
                    .static_semantics
                    .local_value_binding(name)
                    .or_else(|| self.global_value_binding(name))
                    && !static_expression_matches(value, expression)
                {
                    return self.resolve_iterator_step_typed_array_builtin_bytes_per_element(
                        value,
                        depth + 1,
                    );
                }
                None
            }
            Expression::Member { object, property } if matches!(property.as_ref(), Expression::String(name) if name == "value") =>
            {
                let IteratorStepBinding::Runtime {
                    function_binding,
                    static_value,
                    value_candidates,
                    ..
                } = self.resolve_iterator_step_binding_from_expression(object)?;
                if let Some(LocalFunctionBinding::Builtin(function_name)) = function_binding
                    && let Some(bytes_per_element) =
                        typed_array_builtin_bytes_per_element(&function_name)
                {
                    return Some(bytes_per_element);
                }
                let mut candidates = Vec::new();
                if let Some(value) = static_value {
                    candidates.push(value);
                }
                candidates.extend(value_candidates);

                let mut resolved = None;
                for candidate in candidates {
                    let bytes = self.resolve_iterator_step_typed_array_builtin_bytes_per_element(
                        &candidate,
                        depth + 1,
                    )?;
                    if resolved.is_some_and(|existing| existing != bytes) {
                        return None;
                    }
                    resolved = Some(bytes);
                }
                resolved
            }
            _ => None,
        }
    }

    pub(in crate::backend::direct_wasm) fn resolve_constructed_object_constructor_binding(
        &self,
        object: &Expression,
    ) -> Option<LocalFunctionBinding> {
        if let Some(binding) = self
            .resolve_member_function_binding(object, &Expression::String("constructor".to_string()))
        {
            return Some(binding);
        }
        if self.expression_is_known_promise_instance_for_instanceof(object) {
            return Some(LocalFunctionBinding::Builtin("Promise".to_string()));
        }
        let materialized_object = self.materialize_static_expression(object);
        match &materialized_object {
            Expression::New { callee, .. } => self.resolve_function_binding_from_expression(callee),
            _ if !static_expression_matches(&materialized_object, object) => {
                self.resolve_constructed_object_constructor_binding(&materialized_object)
            }
            _ => None,
        }
    }

    pub(in crate::backend::direct_wasm) fn resolve_typed_array_builtin_bytes_per_element(
        &self,
        object: &Expression,
        property: &Expression,
    ) -> Option<u32> {
        if !matches!(property, Expression::String(property_name) if property_name == "BYTES_PER_ELEMENT")
        {
            return None;
        }
        if let Some(bytes_per_element) =
            self.resolve_iterator_step_typed_array_builtin_bytes_per_element(object, 0)
        {
            return Some(bytes_per_element);
        }
        if let Some(LocalFunctionBinding::Builtin(function_name)) =
            self.resolve_function_binding_from_expression(object)
            && let Some(bytes_per_element) = typed_array_builtin_bytes_per_element(&function_name)
        {
            return Some(bytes_per_element);
        }
        if let Expression::Identifier(name) = object {
            let mut candidates = vec![name.clone()];
            if let Some((resolved_name, _)) = self.resolve_current_local_binding(name) {
                candidates.push(resolved_name);
            }
            for candidate in candidates {
                if let Some(LocalFunctionBinding::Builtin(function_name)) = self
                    .state
                    .speculation
                    .static_semantics
                    .local_function_binding(&candidate)
                    .cloned()
                    .or_else(|| self.backend.global_function_binding(&candidate).cloned())
                    && let Some(bytes_per_element) =
                        typed_array_builtin_bytes_per_element(&function_name)
                {
                    return Some(bytes_per_element);
                }
            }
        }
        let Expression::Identifier(name) = self.materialize_static_expression(object) else {
            return None;
        };
        typed_array_builtin_bytes_per_element(&name)
    }
}
