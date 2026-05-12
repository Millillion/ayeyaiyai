use super::*;

impl<'a> FunctionCompiler<'a> {
    pub(in crate::backend::direct_wasm) fn resolve_user_function_length(
        &self,
        object: &Expression,
        property: &Expression,
    ) -> Option<u32> {
        let trace_function_length = std::env::var_os("AYY_TRACE_FUNCTION_LENGTH").is_some();
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

        for object_candidate in object_candidates.into_iter().flatten() {
            for property_candidate in property_candidates.into_iter().flatten() {
                if !matches!(property_candidate, Expression::String(property_name) if property_name == "length")
                {
                    if trace_function_length {
                        eprintln!(
                            "function_length:skip_property object={object_candidate:?} property={property_candidate:?}"
                        );
                    }
                    continue;
                }
                let Some(function_binding) =
                    self.resolve_function_binding_from_expression(object_candidate)
                else {
                    if trace_function_length {
                        eprintln!(
                            "function_length:no_binding object={object_candidate:?} property={property_candidate:?}"
                        );
                    }
                    continue;
                };
                if trace_function_length {
                    eprintln!(
                        "function_length:binding object={object_candidate:?} property={property_candidate:?} binding={function_binding:?}"
                    );
                }
                if let LocalFunctionBinding::User(function_name) = &function_binding
                    && let Some(value) = self.explicit_function_self_binding_property_value(
                        function_name,
                        property_candidate,
                    )
                {
                    match value {
                        Expression::Number(length) => return Some(length as u32),
                        _ => continue,
                    }
                }
                if self
                    .function_object_has_explicit_own_property(object_candidate, property_candidate)
                {
                    if trace_function_length {
                        eprintln!(
                            "function_length:masked object={object_candidate:?} property={property_candidate:?}"
                        );
                    }
                    continue;
                }
                match function_binding {
                    LocalFunctionBinding::User(function_name) => {
                        let length = self
                            .user_function(&function_name)
                            .map(|user_function| user_function.length);
                        if trace_function_length {
                            eprintln!(
                                "function_length:user function={function_name} length={length:?}"
                            );
                        }
                        return length;
                    }
                    LocalFunctionBinding::Builtin(function_name) => {
                        let length = builtin_function_length(&function_name);
                        if trace_function_length {
                            eprintln!(
                                "function_length:builtin function={function_name} length={length:?}"
                            );
                        }
                        return length;
                    }
                }
            }
        }
        if trace_function_length {
            eprintln!("function_length:none object={object:?} property={property:?}");
        }
        None
    }

    pub(in crate::backend::direct_wasm) fn runtime_user_function_property_value(
        &self,
        user_function: &UserFunction,
        property_name: &str,
    ) -> Option<Expression> {
        let property = Expression::String(property_name.to_string());
        let function_expression = Expression::Identifier(user_function.name.clone());
        if let Some(object_binding) = self.backend.global_object_binding(&user_function.name)
            && let Some(value) = object_binding_lookup_value(object_binding, &property)
        {
            match value {
                Expression::Identifier(name)
                    if property_name == "name" && name == &user_function.name => {}
                Expression::String(_) | Expression::Number(_) | Expression::Identifier(_) => {
                    return Some(value.clone());
                }
                _ => return None,
            }
        }
        if let Some(value) =
            self.explicit_function_self_binding_property_value(&user_function.name, &property)
        {
            return match (property_name, value) {
                ("name", Expression::String(text)) => Some(Expression::String(text)),
                ("length", Expression::Number(length)) => Some(Expression::Number(length)),
                _ => None,
            };
        }
        if self.function_object_has_explicit_own_property(&function_expression, &property) {
            return None;
        }
        match property_name {
            "name" => Some(Expression::String(
                self.resolve_user_function_display_name(&user_function.name)
                    .unwrap_or_default(),
            )),
            "length" => Some(Expression::Number(user_function.length as f64)),
            _ => None,
        }
    }

    pub(in crate::backend::direct_wasm) fn resolve_function_name_value(
        &self,
        object: &Expression,
        property: &Expression,
    ) -> Option<String> {
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

        for object_candidate in object_candidates.into_iter().flatten() {
            for property_candidate in property_candidates.into_iter().flatten() {
                if !matches!(property_candidate, Expression::String(property_name) if property_name == "name")
                {
                    continue;
                }
                let Some(function_binding) =
                    self.resolve_function_binding_from_expression(object_candidate)
                else {
                    continue;
                };
                if let LocalFunctionBinding::User(function_name) = &function_binding
                    && let Some(value) = self.explicit_function_self_binding_property_value(
                        function_name,
                        property_candidate,
                    )
                {
                    match value {
                        Expression::String(name) => return Some(name),
                        _ => continue,
                    }
                }
                if self
                    .function_object_has_explicit_own_property(object_candidate, property_candidate)
                {
                    continue;
                }
                match function_binding {
                    LocalFunctionBinding::User(function_name) => {
                        return Some(
                            self.resolve_user_function_display_name(&function_name)
                                .unwrap_or_default(),
                        );
                    }
                    LocalFunctionBinding::Builtin(function_name) => {
                        return Some(builtin_function_display_name(&function_name).to_string());
                    }
                }
            }
        }
        None
    }
}
