use super::super::*;
use super::FunctionStaticEvalContext;

impl StaticMaterializationPolicySource for FunctionStaticEvalContext<'_, '_> {
    fn static_materialize_member_expression(
        &self,
        expression: &Expression,
        object: &Expression,
        property: &Expression,
        environment: &mut Self::Environment,
    ) -> Option<Expression> {
        if self.expression_is_restricted_function_property_with_state(object, property, environment)
        {
            return Some(expression.clone());
        }
        if let Some(value) = self.resolve_static_number_value(expression) {
            return Some(Expression::Number(value));
        }
        let resolved_property = self
            .evaluate_expression_with_state(property, environment)
            .or_else(|| self.materialize_expression_with_state(property, environment))
            .unwrap_or_else(|| property.clone());
        let resolved_object = self
            .evaluate_expression_with_state(object, environment)
            .or_else(|| self.materialize_expression_with_state(object, environment))
            .unwrap_or_else(|| object.clone());
        if let Some(property_name) = static_property_name_from_expression(&resolved_property) {
            for candidate_object in [object, &resolved_object] {
                if let Expression::Identifier(object_name) = candidate_object
                    && self.is_unshadowed_builtin_identifier(object_name)
                    && let Some(value) = builtin_member_number_value(object_name, &property_name)
                {
                    return Some(Expression::Number(value));
                }
            }
        }
        if !self.is_private_member_read_property(&resolved_property) {
            return None;
        }
        let getter_object = match object {
            Expression::Identifier(name) if name == FunctionCompiler::STATIC_NEW_THIS_BINDING => {
                Expression::This
            }
            _ => object.clone(),
        };
        let value = self.resolve_static_member_getter_value_with_context(
            &getter_object,
            &resolved_property,
            self.current_function_name(),
        )?;
        let _ = expression;
        self.evaluate_expression_with_state(&value, environment)
            .or_else(|| self.materialize_expression_with_state(&value, environment))
            .or(Some(value))
    }

    fn static_preserve_new_expressions_in_materialization(&self) -> bool {
        true
    }

    fn static_preserve_call_expressions_in_materialization(&self) -> bool {
        true
    }

    fn static_materialize_post_structural_fallback_expression(
        &self,
        expression: &Expression,
        _environment: &Self::Environment,
    ) -> Option<Expression> {
        Some(self.materialize_expression(expression))
    }
}
