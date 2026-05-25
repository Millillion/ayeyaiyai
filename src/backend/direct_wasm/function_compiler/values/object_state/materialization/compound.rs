use super::*;

impl<'a> FunctionCompiler<'a> {
    pub(in crate::backend::direct_wasm) fn materialize_conditional_expression(
        &self,
        condition: &Expression,
        then_expression: &Expression,
        else_expression: &Expression,
    ) -> Expression {
        let materialized_condition = self.materialize_static_expression(condition);
        let condition_value = if self.if_condition_depends_on_active_loop_assignment(condition)
            || self.expression_has_dynamic_member_property_access(condition)
        {
            None
        } else {
            self.resolve_static_if_condition_value(&materialized_condition)
        }
        .or_else(|| {
            (!self.expression_has_dynamic_member_property_access(condition)
                && self.if_condition_depends_on_active_iterator_loop_assignment(condition))
            .then(|| self.resolve_static_loop_dependent_if_condition_value(condition))
            .flatten()
        });
        if let Some(condition_value) = condition_value {
            let branch = if condition_value {
                then_expression
            } else {
                else_expression
            };
            return self.materialize_static_expression(branch);
        }
        Expression::Conditional {
            condition: Box::new(materialized_condition),
            then_expression: Box::new(self.materialize_static_expression(then_expression)),
            else_expression: Box::new(self.materialize_static_expression(else_expression)),
        }
    }

    pub(in crate::backend::direct_wasm) fn materialize_call_expression(
        &self,
        expression: &Expression,
        callee: &Expression,
        arguments: &[CallArgument],
    ) -> Expression {
        let static_has_own_result = match callee {
            Expression::Member { property, .. } if matches!(property.as_ref(), Expression::String(name) if name == "hasOwnProperty") => {
                self.resolve_static_has_own_property_call_result(expression)
            }
            _ => None,
        };
        if let Some(value) = static_has_own_result
            .map(Expression::Bool)
            .or_else(|| {
                self.resolve_static_reflect_has_call_result(expression)
                    .map(Expression::Bool)
            })
            .or_else(|| {
                self.resolve_static_is_nan_call_result(expression)
                    .map(Expression::Bool)
            })
            .or_else(|| {
                self.resolve_static_object_is_call_result(expression)
                    .map(Expression::Bool)
            })
            .or_else(|| {
                self.resolve_static_array_is_array_call_result(expression)
                    .map(Expression::Bool)
            })
        {
            return value;
        }
        if arguments.is_empty()
            && let Expression::Identifier(function_name) = callee
            && let Some(value) = self.infer_static_class_init_call_result_expression(function_name)
        {
            return self.materialize_static_expression(&value);
        }
        if let Expression::Member { object, property } = callee
            && matches!(object.as_ref(), Expression::Identifier(name) if name == "Object")
            && matches!(property.as_ref(), Expression::String(name) if name == "getPrototypeOf")
            && let Some(CallArgument::Expression(target) | CallArgument::Spread(target)) =
                arguments.first()
            && let Some(prototype) = self.resolve_static_object_prototype_expression(target)
        {
            return self.materialize_static_expression(&prototype);
        }
        if arguments.is_empty()
            && let Expression::Member { object, property } = callee
            && let Expression::String(property_name) = property.as_ref()
            && matches!(property_name.as_str(), "toString" | "valueOf")
            && let Some(StaticEvalOutcome::Value(value)) = self
                .resolve_static_member_call_outcome_with_context(
                    object,
                    property_name,
                    self.current_function_name(),
                )
        {
            return self.materialize_static_expression(&value);
        }
        if matches!(callee, Expression::Identifier(_))
            && !self
                .resolve_user_function_from_expression(callee)
                .is_some_and(|user_function| {
                    user_function.is_async() || user_function.is_generator()
                })
            && let Some(value) = self.resolve_static_call_result_expression(callee, arguments)
        {
            return self.materialize_static_expression(&value);
        }
        if let Expression::Member { object, property } = callee
            && matches!(property.as_ref(), Expression::String(name) if name == "bind")
        {
            return Expression::Call {
                callee: Box::new(Expression::Member {
                    object: object.clone(),
                    property: property.clone(),
                }),
                arguments: arguments
                    .iter()
                    .map(|argument| match argument {
                        CallArgument::Expression(expression) => {
                            CallArgument::Expression(self.materialize_static_expression(expression))
                        }
                        CallArgument::Spread(expression) => {
                            CallArgument::Spread(self.materialize_static_expression(expression))
                        }
                    })
                    .collect(),
            };
        }
        materialize_recursive_expression(expression, true, true, &|nested| {
            Some(self.materialize_static_expression(nested))
        })
        .expect("function-side recursive materialization supports generic call rebuild")
    }

    pub(in crate::backend::direct_wasm) fn materialize_recursive_expression_default(
        &self,
        expression: &Expression,
    ) -> Expression {
        materialize_recursive_expression(expression, true, true, &|nested| {
            Some(self.materialize_static_expression(nested))
        })
        .unwrap_or_else(|| expression.clone())
    }
}
