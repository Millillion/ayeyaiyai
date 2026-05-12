use super::*;

impl<'a> FunctionCompiler<'a> {
    pub(in crate::backend::direct_wasm) fn explicit_function_self_binding_property_value(
        &self,
        function_name: &str,
        property: &Expression,
    ) -> Option<Expression> {
        let self_binding = self
            .resolve_registered_function_declaration(function_name)?
            .self_binding
            .as_ref()?;
        self.state
            .speculation
            .static_semantics
            .local_object_binding(self_binding)
            .or_else(|| self.backend.global_object_binding(self_binding))
            .and_then(|object_binding| {
                self.resolve_object_binding_property_value(object_binding, property)
            })
    }

    fn function_self_binding_has_explicit_own_property(
        &self,
        binding: &LocalFunctionBinding,
        property: &Expression,
    ) -> bool {
        let LocalFunctionBinding::User(function_name) = binding else {
            return false;
        };
        let Some(self_binding) = self
            .resolve_registered_function_declaration(function_name)
            .and_then(|function| function.self_binding.as_ref())
        else {
            return false;
        };

        let self_expression = Expression::Identifier(self_binding.clone());
        self.resolve_member_function_binding_shallow(&self_expression, property)
            .is_some()
            || self
                .resolve_member_getter_binding_shallow(&self_expression, property)
                .is_some()
            || self
                .resolve_member_setter_binding_shallow(&self_expression, property)
                .is_some()
            || self
                .state
                .speculation
                .static_semantics
                .local_object_binding(self_binding)
                .or_else(|| self.backend.global_object_binding(self_binding))
                .is_some_and(|object_binding| {
                    self.resolve_object_binding_property_value(object_binding, property)
                        .is_some()
                })
    }

    pub(in crate::backend::direct_wasm) fn function_object_has_explicit_own_property(
        &self,
        object: &Expression,
        property: &Expression,
    ) -> bool {
        let resolved_object = self
            .resolve_bound_alias_expression(object)
            .filter(|resolved| !static_expression_matches(resolved, object));
        let materialized_object = self.materialize_static_expression(object);
        let resolved_property = self.resolve_property_key_expression(property).or_else(|| {
            self.resolve_bound_alias_expression(property)
                .filter(|resolved| !static_expression_matches(resolved, property))
        });
        let materialized_property = self.materialize_static_expression(property);

        let object_candidates = [
            Some(object),
            resolved_object.as_ref(),
            (!static_expression_matches(&materialized_object, object))
                .then_some(&materialized_object),
        ];
        let property_candidates = [
            Some(property),
            resolved_property.as_ref(),
            (!static_expression_matches(&materialized_property, property))
                .then_some(&materialized_property),
        ];

        object_candidates
            .into_iter()
            .flatten()
            .any(|object_candidate| {
                property_candidates
                    .into_iter()
                    .flatten()
                    .any(|property_candidate| {
                        self.resolve_member_function_binding_shallow(
                            object_candidate,
                            property_candidate,
                        )
                        .is_some()
                            || self
                                .resolve_member_getter_binding_shallow(
                                    object_candidate,
                                    property_candidate,
                                )
                                .is_some()
                            || self
                                .resolve_member_setter_binding_shallow(
                                    object_candidate,
                                    property_candidate,
                                )
                                .is_some()
                            || self
                                .resolve_object_binding_from_expression(object_candidate)
                                .is_some_and(|object_binding| {
                                    self.resolve_object_binding_property_value(
                                        &object_binding,
                                        property_candidate,
                                    )
                                    .is_some()
                                })
                            || self
                                .resolve_function_binding_from_expression(object_candidate)
                                .as_ref()
                                .is_some_and(|binding| {
                                    self.function_self_binding_has_explicit_own_property(
                                        binding,
                                        property_candidate,
                                    )
                                })
                    })
            })
    }
}
