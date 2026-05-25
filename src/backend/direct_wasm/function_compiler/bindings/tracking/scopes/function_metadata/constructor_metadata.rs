use super::*;

impl<'a> FunctionCompiler<'a> {
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
        if let Some(LocalFunctionBinding::Builtin(function_name)) =
            self.resolve_function_binding_from_expression(object)
            && let Some(bytes_per_element) = typed_array_builtin_bytes_per_element(&function_name)
        {
            return Some(bytes_per_element);
        }
        let Expression::Identifier(name) = self.materialize_static_expression(object) else {
            return None;
        };
        typed_array_builtin_bytes_per_element(&name)
    }
}
