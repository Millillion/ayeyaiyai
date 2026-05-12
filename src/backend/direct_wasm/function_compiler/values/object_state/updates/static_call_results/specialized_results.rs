use super::*;

impl<'a> FunctionCompiler<'a> {
    pub(super) fn resolve_specialized_static_call_result_with_context(
        &self,
        callee: &Expression,
        arguments: &[CallArgument],
    ) -> Option<(Expression, Option<String>)> {
        if let Some(specialized) = self.resolve_specialized_function_value_from_expression(callee) {
            return self.resolve_specialized_static_result_from_binding(
                &specialized.binding,
                &specialized.summary,
                arguments,
            );
        }

        if let Expression::Member { object, property } = callee
            && let Some(specialized) =
                self.resolve_tracked_array_specialized_function_value(object, property)
        {
            return self.resolve_specialized_static_result_from_binding(
                &specialized.binding,
                &specialized.summary,
                arguments,
            );
        }

        None
    }

    fn resolve_specialized_static_result_from_binding(
        &self,
        binding: &LocalFunctionBinding,
        summary: &InlineFunctionSummary,
        arguments: &[CallArgument],
    ) -> Option<(Expression, Option<String>)> {
        let LocalFunctionBinding::User(function_name) = binding else {
            return None;
        };
        let user_function = self.user_function(function_name)?;
        if self
            .backend
            .function_registry
            .analysis
            .user_function_capture_bindings
            .contains_key(&user_function.name)
            || self.user_function_references_captured_user_function(user_function)
        {
            return None;
        }
        if user_function.has_lowered_pattern_parameters()
            || !self
                .user_function_parameter_iterator_consumption_indices(user_function)
                .is_empty()
        {
            return None;
        }
        if !summary.effects.is_empty() {
            return None;
        }
        if !self.user_function_has_explicit_call_frame_inlineable_terminal_body(user_function) {
            return None;
        }
        let return_value = summary.return_value.as_ref()?;
        let expanded_arguments = self.expand_call_arguments(arguments);
        let arguments_binding = Expression::Array(
            expanded_arguments
                .iter()
                .cloned()
                .map(ArrayElement::Expression)
                .collect(),
        );
        Some((
            self.substitute_user_function_call_frame_bindings(
                return_value,
                user_function,
                arguments,
                &Expression::Undefined,
                &arguments_binding,
            ),
            Some(function_name.clone()),
        ))
    }
}
