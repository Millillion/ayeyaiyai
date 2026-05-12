use super::*;

impl<'a> FunctionCompiler<'a> {
    pub(in crate::backend::direct_wasm) fn resolve_object_binding_property_value(
        &self,
        object_binding: &ObjectValueBinding,
        property: &Expression,
    ) -> Option<Expression> {
        let canonical_property = self.canonical_object_property_expression(property);
        let requested_symbol = self
            .resolve_symbol_identity_expression(&canonical_property)
            .or_else(|| self.resolve_symbol_identity_expression(property));
        if object_binding.runtime_symbol_properties && requested_symbol.is_some() {
            return None;
        }
        if let Some(value) = object_binding_lookup_value(object_binding, &canonical_property) {
            return Some(value.clone());
        }

        let requested_symbol = requested_symbol?;
        object_binding
            .symbol_properties
            .iter()
            .find_map(|(existing_key, value)| {
                let canonical_existing = self
                    .resolve_symbol_identity_expression(existing_key)
                    .unwrap_or_else(|| existing_key.clone());
                (static_expression_matches(&canonical_existing, &requested_symbol)
                    || static_expression_matches(existing_key, &requested_symbol))
                .then(|| value.clone())
            })
    }

    pub(in crate::backend::direct_wasm) fn resolve_inherited_object_property_value(
        &self,
        object: &Expression,
        property: &Expression,
    ) -> Option<Expression> {
        let mut prototype = self.resolve_static_object_prototype_expression(object)?;
        for _ in 0..32 {
            let materialized_prototype = self.materialize_static_expression(&prototype);
            if matches!(materialized_prototype, Expression::Null) {
                return None;
            }

            for candidate in [&prototype, &materialized_prototype] {
                if let Some(object_binding) = self.resolve_object_binding_from_expression(candidate)
                    && let Some(value) =
                        self.resolve_object_binding_property_value(&object_binding, property)
                {
                    return Some(value);
                }
            }

            let next_prototype = self
                .resolve_static_object_prototype_expression(&materialized_prototype)
                .or_else(|| self.resolve_static_object_prototype_expression(&prototype))?;
            if static_expression_matches(&next_prototype, &prototype)
                || static_expression_matches(&next_prototype, &materialized_prototype)
            {
                return None;
            }
            prototype = next_prototype;
        }
        None
    }

    pub(in crate::backend::direct_wasm) fn resolve_object_binding_property_value_with_inherited(
        &self,
        object: &Expression,
        object_binding: &ObjectValueBinding,
        property: &Expression,
    ) -> Option<Expression> {
        self.resolve_object_binding_property_value(object_binding, property)
            .or_else(|| self.resolve_inherited_object_property_value(object, property))
    }

    pub(in crate::backend::direct_wasm) fn object_binding_string_property_values_with_inherited(
        &self,
        object: &Expression,
        object_binding: &ObjectValueBinding,
    ) -> Vec<(String, Expression)> {
        let mut values = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for (name, value) in &object_binding.string_properties {
            if seen.insert(name.clone()) {
                values.push((name.clone(), value.clone()));
            }
        }

        let Some(mut prototype) = self.resolve_static_object_prototype_expression(object) else {
            return values;
        };
        for _ in 0..32 {
            let materialized_prototype = self.materialize_static_expression(&prototype);
            if matches!(materialized_prototype, Expression::Null) {
                break;
            }

            for candidate in [&prototype, &materialized_prototype] {
                let Some(prototype_binding) =
                    self.resolve_object_binding_from_expression(candidate)
                else {
                    continue;
                };
                for (name, value) in &prototype_binding.string_properties {
                    if seen.insert(name.clone()) {
                        values.push((name.clone(), value.clone()));
                    }
                }
                break;
            }

            let Some(next_prototype) = self
                .resolve_static_object_prototype_expression(&materialized_prototype)
                .or_else(|| self.resolve_static_object_prototype_expression(&prototype))
            else {
                break;
            };
            if static_expression_matches(&next_prototype, &prototype)
                || static_expression_matches(&next_prototype, &materialized_prototype)
            {
                break;
            }
            prototype = next_prototype;
        }
        values
    }
}
