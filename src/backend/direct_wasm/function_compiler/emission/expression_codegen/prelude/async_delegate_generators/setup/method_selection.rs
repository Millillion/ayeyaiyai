use super::*;

impl<'a> FunctionCompiler<'a> {
    fn resolve_async_delegate_object_method_value(
        &self,
        expression: &Expression,
        property: &Expression,
    ) -> Option<Expression> {
        self.resolve_object_binding_from_expression(expression)
            .and_then(|object_binding| {
                object_binding_lookup_value(&object_binding, property).cloned()
            })
    }

    pub(in crate::backend::direct_wasm) fn async_yield_delegate_uses_async_iterator_method(
        &self,
        plan: &AsyncYieldDelegateGeneratorPlan,
        async_iterator_property: &Expression,
    ) -> bool {
        if let Some(getter_binding) =
            self.resolve_member_getter_binding(&plan.delegate_expression, async_iterator_property)
        {
            return self
                .resolve_function_binding_static_return_expression_with_call_frame(
                    &getter_binding,
                    &[],
                    &plan.delegate_expression,
                )
                .map(|value| !matches!(value, Expression::Null | Expression::Undefined))
                .unwrap_or(false);
        }
        self.resolve_member_function_binding(&plan.delegate_expression, async_iterator_property)
            .is_some()
            || self
                .resolve_object_binding_from_expression(&plan.delegate_expression)
                .and_then(|object_binding| {
                    object_binding_lookup_value(&object_binding, async_iterator_property).cloned()
                })
                .is_some_and(|method_value| {
                    !matches!(method_value, Expression::Undefined | Expression::Null)
                })
    }

    pub(in crate::backend::direct_wasm) fn emit_async_yield_delegate_setup(
        &mut self,
        plan: &AsyncYieldDelegateGeneratorPlan,
        uses_async_iterator_method: bool,
        async_iterator_member: &Expression,
        iterator_member: &Expression,
        delegate_iterator_method_name: &str,
        delegate_iterator_name: &str,
        delegate_next_name: &str,
        async_iterator_property: &Expression,
        iterator_property: &Expression,
    ) -> DirectResult<()> {
        self.with_current_user_function_name(Some(plan.function_name.clone()), |compiler| {
            for effect in &plan.prefix_effects {
                compiler.emit_statement(effect)?;
            }
            let selected_iterator_property = if uses_async_iterator_method {
                async_iterator_property
            } else {
                iterator_property
            };
            if compiler
                .resolve_async_delegate_object_method_value(
                    &plan.delegate_expression,
                    selected_iterator_property,
                )
                .is_some_and(|method_value| {
                    !matches!(method_value, Expression::Undefined | Expression::Null)
                        && compiler
                            .resolve_function_binding_from_expression(&method_value)
                            .is_none()
                })
            {
                compiler.emit_named_error_throw("TypeError")?;
                return Ok(());
            }
            if compiler
                .resolve_member_getter_binding(&plan.delegate_expression, async_iterator_property)
                .is_some()
                && !uses_async_iterator_method
            {
                compiler.emit_statement(&Statement::Expression(async_iterator_member.clone()))?;
            }
            let delegate_iterator_member = if uses_async_iterator_method {
                async_iterator_member.clone()
            } else {
                iterator_member.clone()
            };
            compiler.with_restored_function_static_binding_metadata(|compiler| {
                compiler.emit_statement(&Statement::Assign {
                    name: delegate_iterator_method_name.to_string(),
                    value: delegate_iterator_member.clone(),
                })
            })?;
            let delegate_iterator_call = Expression::Call {
                callee: Box::new(Expression::Member {
                    object: Box::new(Expression::Identifier(
                        delegate_iterator_method_name.to_string(),
                    )),
                    property: Box::new(Expression::String("call".to_string())),
                }),
                arguments: vec![CallArgument::Expression(plan.delegate_expression.clone())],
            };
            compiler.with_restored_function_static_binding_metadata(|compiler| {
                compiler.emit_statement(&Statement::Assign {
                    name: delegate_iterator_name.to_string(),
                    value: delegate_iterator_call.clone(),
                })
            })?;
            compiler.with_restored_function_static_binding_metadata(|compiler| {
                compiler.emit_statement(&Statement::Assign {
                    name: delegate_next_name.to_string(),
                    value: Expression::Member {
                        object: Box::new(Expression::Identifier(
                            delegate_iterator_name.to_string(),
                        )),
                        property: Box::new(Expression::String("next".to_string())),
                    },
                })
            })?;
            Ok(())
        })
    }
}
