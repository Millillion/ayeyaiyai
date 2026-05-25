use super::*;

impl<'a> FunctionCompiler<'a> {
    fn resolve_property_key_sequence_with_internal_assignments(
        &self,
        expressions: &[Expression],
    ) -> Option<ResolvedPropertyKey> {
        let (last, preceding) = expressions.split_last()?;
        if preceding.is_empty() {
            return self.resolve_property_key_expression_with_coercion(last);
        }

        let mut bindings = HashMap::new();
        for expression in preceding {
            let Expression::Assign { name, value } = expression else {
                return None;
            };
            if !name.starts_with("__ayy_optional_base_") {
                return None;
            }

            let substituted = substitute_inline_summary_bindings(value, &bindings);
            let materialized = self.materialize_static_expression(&substituted);
            let binding_value = if static_expression_matches(&materialized, &substituted) {
                substituted
            } else {
                materialized
            };
            bindings.insert(name.clone(), binding_value);
        }

        let substituted_last = substitute_inline_summary_bindings(last, &bindings);
        self.resolve_property_key_expression_with_coercion(&substituted_last)
            .or_else(|| {
                let materialized = self.materialize_static_expression(&substituted_last);
                (!static_expression_matches(&materialized, &substituted_last))
                    .then(|| self.resolve_property_key_expression_with_coercion(&materialized))
                    .flatten()
            })
    }

    pub(in crate::backend::direct_wasm) fn resolve_property_key_from_function_binding(
        &self,
        binding: &LocalFunctionBinding,
    ) -> Option<Expression> {
        if let LocalFunctionBinding::User(function_name) = binding
            && let Some(user_function) = self.user_function(function_name)
            && let Some(summary) = user_function.inline_summary.as_ref()
            && let Some(return_value) = summary.return_value.as_ref()
        {
            let substituted =
                self.substitute_user_function_argument_bindings(return_value, user_function, &[]);
            if let Some(key) = self.resolve_primitive_property_key_expression(&substituted) {
                return Some(key);
            }
        }

        match self.resolve_terminal_function_outcome_from_binding(binding, &[])? {
            StaticEvalOutcome::Value(expression) => {
                self.resolve_primitive_property_key_expression(&expression)
            }
            StaticEvalOutcome::Throw(_) => None,
        }
    }

    pub(in crate::backend::direct_wasm) fn resolve_property_key_coercion_binding_from_object_binding(
        &self,
        object_binding: &ObjectValueBinding,
    ) -> Option<LocalFunctionBinding> {
        let symbol_property = symbol_to_primitive_expression();
        if let Some(method_value) = object_binding_lookup_value(object_binding, &symbol_property) {
            if matches!(
                self.resolve_static_primitive_expression_with_context(
                    method_value,
                    self.current_function_name(),
                ),
                Some(Expression::Null | Expression::Undefined)
            ) {
                // Fall through to ordinary coercion when @@toPrimitive is absent.
            } else {
                return self.resolve_function_binding_from_expression(method_value);
            }
        }

        for method_name in ["toString", "valueOf"] {
            let method_value = object_binding_lookup_value(
                object_binding,
                &Expression::String(method_name.to_string()),
            );
            match method_value {
                None | Some(Expression::Null) | Some(Expression::Undefined) => continue,
                Some(value) => {
                    if let Some(binding) = self.resolve_function_binding_from_expression(value) {
                        return Some(binding);
                    }
                }
            }
        }
        None
    }

    pub(in crate::backend::direct_wasm) fn resolve_property_key_coercion_from_object_binding(
        &self,
        object_binding: &ObjectValueBinding,
    ) -> Option<(LocalFunctionBinding, Expression)> {
        let binding =
            self.resolve_property_key_coercion_binding_from_object_binding(object_binding)?;
        let key = self.resolve_property_key_from_function_binding(&binding)?;
        Some((binding, key))
    }

    pub(in crate::backend::direct_wasm) fn resolve_property_key_coercion_binding(
        &self,
        expression: &Expression,
    ) -> Option<LocalFunctionBinding> {
        if self
            .resolve_primitive_property_key_expression(expression)
            .is_some()
        {
            return None;
        }

        let object_binding = match expression {
            Expression::Object(_) => None,
            _ => self.resolve_object_binding_from_expression(expression),
        }
        .or_else(|| {
            let materialized = self.materialize_static_expression(expression);
            match materialized {
                Expression::Object(_) => None,
                _ => self.resolve_object_binding_from_expression(&materialized),
            }
        })?;

        self.resolve_property_key_coercion_binding_from_object_binding(&object_binding)
    }

    pub(in crate::backend::direct_wasm) fn resolve_property_key_expression_with_coercion(
        &self,
        expression: &Expression,
    ) -> Option<ResolvedPropertyKey> {
        if let Expression::Sequence(expressions) = expression {
            return self.resolve_property_key_sequence_with_internal_assignments(expressions);
        }
        if let Some(key) = self.resolve_primitive_property_key_expression(expression) {
            return Some(ResolvedPropertyKey {
                key,
                coercion: None,
            });
        }

        let object_binding = match expression {
            Expression::Object(_) => None,
            _ => self.resolve_object_binding_from_expression(expression),
        }
        .or_else(|| {
            let materialized = self.materialize_static_expression(expression);
            match materialized {
                Expression::Object(_) => None,
                _ => self.resolve_object_binding_from_expression(&materialized),
            }
        })?;
        let (coercion, key) =
            self.resolve_property_key_coercion_from_object_binding(&object_binding)?;
        Some(ResolvedPropertyKey {
            key,
            coercion: Some(coercion),
        })
    }

    pub(in crate::backend::direct_wasm) fn resolve_property_key_expression(
        &self,
        expression: &Expression,
    ) -> Option<Expression> {
        self.resolve_property_key_expression_with_coercion(expression)
            .map(|resolved| resolved.key)
    }
}
