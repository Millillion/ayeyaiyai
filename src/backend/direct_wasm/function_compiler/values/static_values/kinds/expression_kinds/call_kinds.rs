use super::*;

impl<'a> FunctionCompiler<'a> {
    fn call_expression_is_object_own_property_boolean_call(
        &self,
        callee: &Expression,
        arguments: &[CallArgument],
    ) -> bool {
        match callee {
            Expression::Member { property, .. }
                if matches!(
                    property.as_ref(),
                    Expression::String(name)
                        if matches!(name.as_str(), "hasOwnProperty" | "propertyIsEnumerable")
                ) =>
            {
                matches!(arguments, [CallArgument::Expression(_), ..])
            }
            Expression::Member { object, property } if matches!(property.as_ref(), Expression::String(name) if name == "call") =>
            {
                let Expression::Member {
                    property: target_property,
                    ..
                } = object.as_ref()
                else {
                    return false;
                };
                matches!(
                    target_property.as_ref(),
                    Expression::String(name)
                        if matches!(name.as_str(), "hasOwnProperty" | "propertyIsEnumerable")
                ) && matches!(
                    arguments,
                    [CallArgument::Expression(_), CallArgument::Expression(_), ..]
                )
            }
            _ => false,
        }
    }

    pub(super) fn infer_call_expression_kind(
        &self,
        expression: &Expression,
        callee: &Expression,
        arguments: &[CallArgument],
    ) -> Option<StaticValueKind> {
        if arguments.is_empty()
            && let Expression::Member { object, property } = callee
            && matches!(property.as_ref(), Expression::String(name) if name == "next")
            && let Expression::Identifier(iterator_name) = object.as_ref()
            && self
                .resolve_local_array_iterator_binding_name(iterator_name)
                .is_some()
        {
            return Some(StaticValueKind::Object);
        }
        if self.call_expression_is_object_own_property_boolean_call(callee, arguments) {
            return Some(StaticValueKind::Bool);
        }
        if self
            .resolve_static_has_own_property_call_result(expression)
            .is_some()
            || self
                .resolve_static_object_is_call_result(expression)
                .is_some()
            || self
                .resolve_static_array_is_array_call_result(expression)
                .is_some()
        {
            return Some(StaticValueKind::Bool);
        }
        if matches!(callee, Expression::Identifier(name) if name == "eval")
            && matches!(
                self.resolve_function_binding_from_expression(callee),
                Some(LocalFunctionBinding::Builtin(function_name)) if function_name == "eval"
            )
            && let Some(kind) = self.infer_static_direct_eval_completion_kind(arguments)
        {
            return Some(kind);
        }
        if matches!(callee, Expression::Identifier(name) if name == "__ayyDynamicImport") {
            return Some(StaticValueKind::Object);
        }
        if let Expression::Identifier(_) = callee
            && let Some(LocalFunctionBinding::User(function_name)) =
                self.resolve_function_binding_from_expression(callee)
            && self
                .user_function(&function_name)
                .is_some_and(|function| function.is_async())
        {
            return Some(StaticValueKind::Object);
        }
        if let Expression::Member { object, property } = callee
            && matches!(property.as_ref(), Expression::String(name) if name == "then" || name == "catch")
        {
            let object_is_async_user_call = if let Expression::Call {
                callee: object_callee,
                ..
            } = object.as_ref()
                && matches!(object_callee.as_ref(), Expression::Identifier(_))
            {
                self.resolve_function_binding_from_expression(object_callee)
                    .is_some_and(|binding| {
                        let LocalFunctionBinding::User(function_name) = binding else {
                            return false;
                        };
                        self.user_function(&function_name)
                            .is_some_and(|function| function.is_async())
                    })
            } else {
                false
            };
            if Self::call_is_promise_like_chain(object)
                || matches!(
                    object.as_ref(),
                    Expression::Call { callee, .. }
                        if matches!(callee.as_ref(), Expression::Identifier(name) if name == "__ayyDynamicImport")
                )
                || object_is_async_user_call
            {
                return Some(StaticValueKind::Object);
            }
        }
        if arguments.is_empty()
            && let Expression::Member { object, property } = callee
            && let Expression::String(property_name) = property.as_ref()
            && let Some(StaticEvalOutcome::Value(value)) = self
                .resolve_static_member_call_outcome_with_context(
                    object,
                    property_name,
                    self.current_function_name(),
                )
        {
            return self.infer_value_kind(&value);
        }
        if let Some((value, _)) = self.resolve_static_call_result_expression_with_context(
            callee,
            arguments,
            self.current_function_name(),
        ) {
            return self.infer_value_kind(&value);
        }
        match callee {
            Expression::Identifier(name) => {
                if let Some(LocalFunctionBinding::Builtin(function_name)) =
                    self.resolve_function_binding_from_expression(callee)
                {
                    self.infer_call_result_kind(&function_name)
                        .or(Some(StaticValueKind::Unknown))
                } else {
                    self.infer_call_result_kind(name)
                        .or(Some(StaticValueKind::Unknown))
                }
            }
            _ => Some(StaticValueKind::Unknown),
        }
    }
}
