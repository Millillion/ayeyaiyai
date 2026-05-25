use super::*;

impl<'a> FunctionCompiler<'a> {
    pub(in crate::backend::direct_wasm) fn resolve_test262_realm_id_from_expression(
        &self,
        expression: &Expression,
    ) -> Option<u32> {
        match expression {
            Expression::Identifier(name) => {
                if let Some(realm_id) = parse_test262_realm_identifier(name) {
                    return Some(realm_id);
                }
                let resolved = self.resolve_bound_alias_expression(expression)?;
                let Expression::Identifier(name) = resolved else {
                    return None;
                };
                parse_test262_realm_identifier(&name)
            }
            _ => None,
        }
    }

    pub(in crate::backend::direct_wasm) fn resolve_test262_realm_global_id_from_expression(
        &self,
        expression: &Expression,
    ) -> Option<u32> {
        if let Expression::Identifier(name) = expression
            && let Some(value) = self
                .state
                .speculation
                .static_semantics
                .local_value_binding(name)
                .or_else(|| self.global_value_binding(name))
                .filter(|value| !static_expression_matches(value, expression))
            && let Some(realm_id) = self.resolve_test262_realm_global_id_from_expression(value)
        {
            return Some(realm_id);
        }
        if let Some(resolved) = self
            .resolve_bound_alias_expression(expression)
            .filter(|resolved| !static_expression_matches(resolved, expression))
            && let Some(realm_id) = self.resolve_test262_realm_global_id_from_expression(&resolved)
        {
            return Some(realm_id);
        }
        let materialized = self.materialize_static_expression(expression);
        match &materialized {
            Expression::Identifier(name) => parse_test262_realm_global_identifier(name),
            Expression::Member { object, property } if matches!(property.as_ref(), Expression::String(name) if name == "global") => {
                self.resolve_test262_realm_id_from_expression(object)
            }
            _ => None,
        }
    }
}
