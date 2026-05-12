use super::*;

impl<'a> FunctionCompiler<'a> {
    pub(in crate::backend::direct_wasm) fn resolve_static_function_outcome_from_binding_with_call_frame_and_context(
        &self,
        binding: &LocalFunctionBinding,
        arguments: &[CallArgument],
        this_binding: &Expression,
        current_function_name: Option<&str>,
    ) -> Option<StaticEvalOutcome> {
        let LocalFunctionBinding::User(function_name) = binding else {
            return self.resolve_static_function_outcome_from_binding_with_context(
                binding,
                arguments,
                current_function_name,
            );
        };
        let user_function = self.user_function(function_name)?;
        if self.user_function_mentions_private_member_access(user_function)
            && self
                .resolve_object_binding_from_expression(this_binding)
                .is_none()
        {
            return None;
        }
        let function = self.resolve_registered_function_declaration(function_name)?;
        if self.user_function_mentions_direct_eval(user_function) {
            return self.resolve_static_direct_eval_return_outcome_from_user_function(
                user_function,
                function,
                arguments,
                this_binding,
            );
        }
        if function.body.is_empty() {
            return Some(StaticEvalOutcome::Value(Expression::Undefined));
        }
        let [statement] = function.body.as_slice() else {
            return None;
        };
        let arguments_binding = Expression::Array(
            arguments
                .iter()
                .map(|argument| match argument {
                    CallArgument::Expression(expression) => {
                        ArrayElement::Expression(expression.clone())
                    }
                    CallArgument::Spread(expression) => ArrayElement::Spread(expression.clone()),
                })
                .collect(),
        );
        match statement {
            Statement::Return(expression) => Some(StaticEvalOutcome::Value(
                self.substitute_user_function_call_frame_bindings(
                    expression,
                    user_function,
                    arguments,
                    this_binding,
                    &arguments_binding,
                ),
            )),
            Statement::Throw(expression) => Some(StaticEvalOutcome::Throw(
                StaticThrowValue::Value(self.substitute_user_function_call_frame_bindings(
                    expression,
                    user_function,
                    arguments,
                    this_binding,
                    &arguments_binding,
                )),
            )),
            _ => None,
        }
    }

    pub(in crate::backend::direct_wasm) fn resolve_static_function_outcome_from_binding_with_context(
        &self,
        binding: &LocalFunctionBinding,
        arguments: &[CallArgument],
        current_function_name: Option<&str>,
    ) -> Option<StaticEvalOutcome> {
        let LocalFunctionBinding::User(function_name) = binding else {
            let LocalFunctionBinding::Builtin(function_name) = binding else {
                return None;
            };
            return self.resolve_static_builtin_function_outcome(
                function_name,
                arguments,
                current_function_name,
            );
        };
        let user_function = self.user_function(function_name)?;
        if self.user_function_mentions_private_member_access(user_function) {
            return None;
        }

        let function = self.resolve_registered_function_declaration(function_name)?;
        if self.user_function_mentions_direct_eval(user_function) {
            let this_binding =
                if self.should_box_sloppy_function_this(user_function, &Expression::Undefined) {
                    Expression::This
                } else {
                    Expression::Undefined
                };
            return self.resolve_static_direct_eval_return_outcome_from_user_function(
                user_function,
                function,
                arguments,
                &this_binding,
            );
        }
        if function.body.is_empty() {
            return Some(StaticEvalOutcome::Value(Expression::Undefined));
        }
        let [statement] = function.body.as_slice() else {
            return None;
        };
        let arguments_binding = Expression::Array(
            arguments
                .iter()
                .map(|argument| match argument {
                    CallArgument::Expression(expression) => {
                        ArrayElement::Expression(expression.clone())
                    }
                    CallArgument::Spread(expression) => ArrayElement::Spread(expression.clone()),
                })
                .collect(),
        );
        let this_binding =
            if self.should_box_sloppy_function_this(user_function, &Expression::Undefined) {
                Expression::This
            } else {
                Expression::Undefined
            };
        match statement {
            Statement::Return(expression) => Some(StaticEvalOutcome::Value(
                self.substitute_user_function_call_frame_bindings(
                    expression,
                    user_function,
                    arguments,
                    &this_binding,
                    &arguments_binding,
                ),
            )),
            Statement::Throw(expression) => Some(StaticEvalOutcome::Throw(
                StaticThrowValue::Value(self.substitute_user_function_call_frame_bindings(
                    expression,
                    user_function,
                    arguments,
                    &this_binding,
                    &arguments_binding,
                )),
            )),
            _ => None,
        }
    }
}
