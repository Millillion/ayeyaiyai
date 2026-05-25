use super::*;

impl<'a> FunctionCompiler<'a> {
    pub(in crate::backend::direct_wasm) fn canonical_object_property_expression(
        &self,
        property: &Expression,
    ) -> Expression {
        if let Expression::Sequence(expressions) = property {
            return expressions
                .last()
                .map(|expression| self.canonical_object_property_expression(expression))
                .unwrap_or(Expression::Undefined);
        }

        let identifier_property = match property {
            Expression::Identifier(name) => self
                .resolve_current_local_binding(name)
                .map(|(resolved_name, _)| Expression::Identifier(resolved_name))
                .or_else(|| {
                    (self
                        .state
                        .speculation
                        .static_semantics
                        .local_value_binding(name)
                        .is_some()
                        || self.backend.global_value_binding(name).is_some())
                    .then(|| Expression::Identifier(name.clone()))
                }),
            _ => None,
        };
        let materialized = self.materialize_static_expression(property);
        let coerced = self
            .resolve_property_key_expression(property)
            .or(identifier_property)
            .unwrap_or(materialized);
        self.resolve_symbol_identity_expression(&coerced)
            .or_else(|| self.resolve_symbol_identity_expression(property))
            .unwrap_or(coerced)
    }
}
