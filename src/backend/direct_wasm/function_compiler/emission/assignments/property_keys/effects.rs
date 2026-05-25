use super::*;

impl<'a> FunctionCompiler<'a> {
    fn property_key_expression_requires_to_property_key_type_error(
        &self,
        expression: &Expression,
    ) -> bool {
        if self
            .resolve_primitive_property_key_expression(expression)
            .is_some()
        {
            return false;
        }

        let materialized = self.materialize_static_expression(expression);
        let object_binding = self
            .resolve_object_binding_from_expression(expression)
            .or_else(|| {
                (!static_expression_matches(&materialized, expression))
                    .then(|| self.resolve_object_binding_from_expression(&materialized))
                    .flatten()
            });
        let Some(object_binding) = object_binding else {
            return false;
        };
        if self
            .resolve_property_key_coercion_binding_from_object_binding(&object_binding)
            .is_some()
        {
            return false;
        }

        let prototype = self
            .resolve_static_object_prototype_expression(expression)
            .or_else(|| {
                (!static_expression_matches(&materialized, expression))
                    .then(|| self.resolve_static_object_prototype_expression(&materialized))
                    .flatten()
            });
        matches!(prototype, Some(Expression::Null))
    }

    pub(in crate::backend::direct_wasm) fn emit_property_key_expression_effects(
        &mut self,
        expression: &Expression,
    ) -> DirectResult<Option<Expression>> {
        let resolved = self.resolve_property_key_expression_with_coercion(expression);
        self.emit_numeric_expression(expression)?;
        self.state.emission.output.instructions.push(0x1a);

        if let Some(binding) = resolved
            .as_ref()
            .and_then(|resolved| resolved.coercion.clone())
            .or_else(|| self.resolve_property_key_coercion_binding(expression))
        {
            match binding {
                LocalFunctionBinding::User(function_name) => {
                    if let Some(user_function) = self.user_function(&function_name).cloned() {
                        self.with_suspended_with_scopes(|compiler| {
                            if compiler.emit_inline_user_function_summary_with_arguments(
                                &user_function,
                                &[],
                            )? {
                                compiler.state.emission.output.instructions.push(0x1a);
                            } else {
                                compiler.emit_user_function_call(&user_function, &[])?;
                                compiler.state.emission.output.instructions.push(0x1a);
                            }
                            Ok(())
                        })?;
                    }
                }
                LocalFunctionBinding::Builtin(function_name) => {
                    self.with_suspended_with_scopes(|compiler| {
                        if compiler.emit_builtin_call(&function_name, &[])? {
                            compiler.state.emission.output.instructions.push(0x1a);
                        }
                        Ok(())
                    })?;
                }
            }
        }

        if resolved.is_none()
            && self.property_key_expression_requires_to_property_key_type_error(expression)
        {
            self.emit_named_error_throw("TypeError")?;
        }

        Ok(resolved.map(|resolved| resolved.key))
    }
}
