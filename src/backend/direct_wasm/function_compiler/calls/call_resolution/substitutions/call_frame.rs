use super::*;

#[path = "call_frame/aggregate_traversal.rs"]
mod aggregate_traversal;
#[path = "call_frame/direct_bindings.rs"]
mod direct_bindings;
#[path = "call_frame/simple_traversal.rs"]
mod simple_traversal;

impl<'a> FunctionCompiler<'a> {
    pub(in crate::backend::direct_wasm) fn call_frame_arguments_shadowed(
        user_function: &UserFunction,
    ) -> bool {
        user_function.body_declares_arguments_binding
            || user_function.params.iter().any(|param| {
                param == "arguments"
                    || scoped_binding_source_name(param)
                        .is_some_and(|source_name| source_name == "arguments")
            })
    }

    pub(in crate::backend::direct_wasm) fn call_frame_arguments_identifier(
        expression: &Expression,
    ) -> bool {
        let Expression::Identifier(name) = expression else {
            return false;
        };
        name == "arguments"
            || scoped_binding_source_name(name)
                .is_some_and(|source_name| source_name == "arguments")
    }

    fn call_frame_arguments_property_index(property: &Expression) -> Option<usize> {
        match property {
            Expression::Number(value)
                if value.is_finite() && *value >= 0.0 && value.fract() == 0.0 =>
            {
                Some(*value as usize)
            }
            Expression::String(value) => value
                .parse::<usize>()
                .ok()
                .filter(|index| index.to_string() == *value),
            _ => None,
        }
    }

    pub(in crate::backend::direct_wasm) fn user_function_has_mapped_arguments(
        &self,
        user_function: &UserFunction,
    ) -> bool {
        self.resolve_registered_function_declaration(&user_function.name)
            .is_some_and(|function| function.mapped_arguments)
    }

    pub(in crate::backend::direct_wasm) fn call_frame_mapped_arguments_parameter_name<'b>(
        &self,
        user_function: &'b UserFunction,
        object: &Expression,
        property: &Expression,
    ) -> Option<&'b str> {
        if !self.user_function_has_mapped_arguments(user_function)
            || user_function.lexical_this
            || Self::call_frame_arguments_shadowed(user_function)
            || !Self::call_frame_arguments_identifier(object)
        {
            return None;
        }
        let index = Self::call_frame_arguments_property_index(property)?;
        if index >= user_function.visible_param_count() as usize {
            return None;
        }
        user_function.params.get(index).map(String::as_str)
    }

    pub(in crate::backend::direct_wasm) fn substitute_user_function_call_frame_bindings(
        &self,
        expression: &Expression,
        user_function: &UserFunction,
        arguments: &[CallArgument],
        this_binding: &Expression,
        arguments_binding: &Expression,
    ) -> Expression {
        let substituted =
            self.substitute_user_function_argument_bindings(expression, user_function, arguments);
        self.substitute_call_frame_special_bindings(
            &substituted,
            user_function,
            this_binding,
            arguments_binding,
        )
    }

    pub(in crate::backend::direct_wasm) fn substitute_call_frame_special_bindings(
        &self,
        expression: &Expression,
        user_function: &UserFunction,
        this_binding: &Expression,
        arguments_binding: &Expression,
    ) -> Expression {
        self.resolve_call_frame_direct_binding_substitution(
            expression,
            user_function,
            this_binding,
            arguments_binding,
        )
        .or_else(|| {
            self.substitute_call_frame_simple_expression(
                expression,
                user_function,
                this_binding,
                arguments_binding,
            )
        })
        .or_else(|| {
            self.substitute_call_frame_aggregate_expression(
                expression,
                user_function,
                this_binding,
                arguments_binding,
            )
        })
        .unwrap_or_else(|| expression.clone())
    }
}
