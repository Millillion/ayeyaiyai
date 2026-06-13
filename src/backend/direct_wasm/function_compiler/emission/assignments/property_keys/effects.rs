use super::*;

impl<'a> FunctionCompiler<'a> {
    fn resolve_property_key_logical_assignment_binding_result(
        &self,
        expression: &Expression,
    ) -> Option<(String, Expression)> {
        match expression {
            Expression::Sequence(expressions) => expressions.iter().rev().find_map(|expression| {
                self.resolve_property_key_logical_assignment_binding_result(expression)
            }),
            Expression::Call { callee, arguments }
                if matches!(callee.as_ref(), Expression::Identifier(name)
                    if name == "String" && self.is_unshadowed_builtin_identifier(name)) =>
            {
                let argument = arguments.first()?.expression();
                self.resolve_property_key_logical_assignment_binding_result(argument)
            }
            Expression::Binary {
                op: op @ (BinaryOp::LogicalAnd | BinaryOp::LogicalOr | BinaryOp::NullishCoalescing),
                left,
                right,
            } => self.resolve_static_logical_assignment_binding_result(*op, left, right),
            _ => None,
        }
    }

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
        if self.symbol_to_primitive_non_callable_type_error(expression)
            || (!static_expression_matches(&materialized, expression)
                && self.symbol_to_primitive_non_callable_type_error(&materialized))
        {
            return true;
        }

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
        let logical_assignment_result =
            self.resolve_property_key_logical_assignment_binding_result(expression);
        let resolved = self.resolve_property_key_expression_with_coercion(expression);
        self.emit_numeric_expression(expression)?;
        self.state.emission.output.instructions.push(0x1a);
        if let Some((name, value)) = logical_assignment_result {
            self.restore_logical_assignment_binding_metadata(&name, &value)?;
        }

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
