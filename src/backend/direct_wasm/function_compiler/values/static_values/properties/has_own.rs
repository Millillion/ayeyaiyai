use super::*;

impl<'a> FunctionCompiler<'a> {
    fn expression_is_global_object_has_own_receiver(&self, expression: &Expression) -> bool {
        if self.state.speculation.execution_context.top_level_function
            && matches!(expression, Expression::This)
        {
            return true;
        }
        if matches!(expression, Expression::Identifier(name) if name == "globalThis" && self.is_unshadowed_builtin_identifier(name))
        {
            return true;
        }
        if self.expression_aliases_captured_top_level_this(expression) {
            return true;
        }
        if let Some(resolved) = self
            .resolve_bound_alias_expression(expression)
            .filter(|resolved| !static_expression_matches(resolved, expression))
            && (matches!(resolved, Expression::This)
                || matches!(resolved, Expression::Identifier(ref name) if name == "globalThis" && self.is_unshadowed_builtin_identifier(name))
                || self.expression_aliases_captured_top_level_this(&resolved))
        {
            return true;
        }
        let materialized = self.materialize_static_expression(expression);
        !static_expression_matches(&materialized, expression)
            && (matches!(materialized, Expression::This)
                || matches!(materialized, Expression::Identifier(ref name) if name == "globalThis" && self.is_unshadowed_builtin_identifier(name))
                || self.expression_aliases_captured_top_level_this(&materialized))
    }

    pub(in crate::backend::direct_wasm) fn resolve_top_level_global_object_has_own_property_result(
        &self,
        object: &Expression,
        property: &Expression,
    ) -> Option<bool> {
        if !self.expression_is_global_object_has_own_receiver(object) {
            return None;
        }
        let property = self
            .resolve_property_key_expression(property)
            .unwrap_or_else(|| self.materialize_static_expression(property));
        let property_name = static_property_name_from_expression(&property)?;
        Some(
            self.resolve_top_level_global_property_descriptor_binding(&property_name)
                .is_some(),
        )
    }

    fn has_own_receiver_is_import_meta_object(expression: &Expression) -> bool {
        matches!(
            expression,
            Expression::Call { callee, arguments }
                if matches!(
                    arguments.as_slice(),
                    []
                        | [CallArgument::Expression(Expression::Number(_))]
                        | [CallArgument::Spread(Expression::Number(_))]
                )
                    && matches!(callee.as_ref(), Expression::Identifier(name) if name == "__ayyImportMeta")
        )
    }

    fn has_own_receiver_is_dynamic_import_promise(expression: &Expression) -> bool {
        matches!(
            expression,
            Expression::Call { callee, .. }
                if matches!(callee.as_ref(), Expression::Identifier(name) if name == "__ayyDynamicImport")
        )
    }

    fn import_meta_known_property_presence(property: &Expression) -> Option<bool> {
        if matches!(property, Expression::String(name) if name == "toString") {
            return Some(true);
        }
        if matches!(property, Expression::String(name) if name == "valueOf")
            || static_expression_matches(property, &symbol_to_primitive_expression())
        {
            return Some(false);
        }
        None
    }

    fn dynamic_import_promise_known_own_property_presence(property: &Expression) -> Option<bool> {
        match property {
            Expression::String(property_name)
                if matches!(property_name.as_str(), "then" | "catch" | "finally") =>
            {
                Some(false)
            }
            _ => None,
        }
    }

    fn resolve_static_reflect_has_result_with_depth(
        &self,
        object: &Expression,
        property: &Expression,
        depth: usize,
    ) -> Option<bool> {
        if depth > 16 {
            return None;
        }

        if self.resolve_function_object_has_own_property(object, property) == Some(true)
            || self
                .resolve_bound_function_prototype_call_descriptor(object, property)
                .is_some()
        {
            return Some(true);
        }

        if let Some(has_own_property) =
            self.resolve_static_object_has_own_property_result(object, property)
        {
            match has_own_property {
                Some(true) => return Some(true),
                Some(false) => {}
                None => return None,
            }
        }

        let prototype = self.resolve_static_object_prototype_expression(object)?;
        if matches!(prototype, Expression::Null) {
            return Some(false);
        }
        if static_expression_matches(&prototype, object) {
            return None;
        }
        self.resolve_static_reflect_has_result_with_depth(&prototype, property, depth + 1)
    }

    pub(in crate::backend::direct_wasm) fn resolve_static_reflect_has_result(
        &self,
        object: &Expression,
        property: &Expression,
    ) -> Option<bool> {
        self.resolve_static_reflect_has_result_with_depth(object, property, 0)
    }

    pub(in crate::backend::direct_wasm) fn resolve_static_object_has_own_property_result(
        &self,
        object: &Expression,
        property: &Expression,
    ) -> Option<Option<bool>> {
        let resolved_object = self
            .resolve_bound_alias_expression(object)
            .filter(|resolved| !static_expression_matches(resolved, object));
        let materialized_object = self.materialize_static_expression(object);
        let resolved_property = self.resolve_property_key_expression(property).or_else(|| {
            self.resolve_bound_alias_expression(property)
                .filter(|resolved| !static_expression_matches(resolved, property))
        });
        let materialized_property = self.materialize_static_expression(property);

        if self.current_function_requires_runtime_public_this_resolution()
            && self.expression_is_current_this_reference(object)
            && !is_private_property_name_expression(&materialized_property)
        {
            return Some(None);
        }

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

        if object_candidates
            .iter()
            .flatten()
            .any(|candidate| Self::has_own_receiver_is_dynamic_import_promise(candidate))
            && let Some(has_own_property) =
                property_candidates.iter().flatten().find_map(|candidate| {
                    Self::dynamic_import_promise_known_own_property_presence(candidate)
                })
        {
            return Some(Some(has_own_property));
        }
        for object_candidate in object_candidates.iter().flatten() {
            for property_candidate in property_candidates.iter().flatten() {
                if let Some(has_own_property) = self
                    .resolve_top_level_global_object_has_own_property_result(
                        object_candidate,
                        property_candidate,
                    )
                {
                    return Some(Some(has_own_property));
                }
            }
        }
        for object_candidate in object_candidates.iter().flatten() {
            for property_candidate in property_candidates.iter().flatten() {
                if self
                    .static_builtin_prototype_has_own_property(object_candidate, property_candidate)
                {
                    return Some(Some(true));
                }
            }
        }

        let mut saw_object_binding = false;
        let mut saw_dynamic_property_lookup = false;
        let mut saw_symbol_property_lookup = false;
        let mut saw_parameter_object_binding = false;
        let boxed_string_length_lookup =
            property_candidates
                .iter()
                .flatten()
                .any(|property_candidate| {
                    matches!(
                        self.resolve_property_key_expression(property_candidate)
                            .unwrap_or_else(|| (*property_candidate).clone()),
                        Expression::String(ref property_name) if property_name == "length"
                    )
                });
        let regexp_last_index_lookup =
            property_candidates
                .iter()
                .flatten()
                .any(|property_candidate| {
                    matches!(
                        self.resolve_property_key_expression(property_candidate)
                            .unwrap_or_else(|| (*property_candidate).clone()),
                        Expression::String(ref property_name) if property_name == "lastIndex"
                    )
                });

        for object_candidate in object_candidates.into_iter().flatten() {
            if boxed_string_length_lookup
                && matches!(
                    self.resolve_static_boxed_primitive_value(object_candidate),
                    Some(Expression::String(_))
                )
            {
                return Some(Some(true));
            }
            if regexp_last_index_lookup
                && self.expression_is_static_regexp_instance(object_candidate)
            {
                return Some(Some(true));
            }
            let object_binding = self
                .resolve_object_binding_from_expression(object_candidate)
                .or_else(|| match object_candidate {
                    Expression::Identifier(name) => {
                        self.resolve_identifier_object_binding_fallback(name)
                    }
                    _ => None,
                });
            let Some(object_binding) = object_binding else {
                continue;
            };
            saw_object_binding = true;
            if let Expression::Identifier(name) = object_candidate {
                let resolved_name = scoped_binding_source_name(name).unwrap_or(name);
                saw_parameter_object_binding |= self
                    .state
                    .parameters
                    .parameter_names
                    .iter()
                    .any(|parameter_name| parameter_name == resolved_name);
            }

            for property_candidate in property_candidates.into_iter().flatten() {
                let canonical_property =
                    self.canonical_object_property_expression(property_candidate);
                let requested_well_known_symbol = self
                    .well_known_symbol_name(&canonical_property)
                    .or_else(|| self.well_known_symbol_name(property_candidate));
                let requested_symbol = self
                    .resolve_symbol_identity_expression(&canonical_property)
                    .or_else(|| self.resolve_symbol_identity_expression(property_candidate));
                if Self::object_binding_has_module_namespace_marker(&object_binding)
                    && (requested_well_known_symbol.is_some() || requested_symbol.is_some())
                {
                    return Some(Some(
                        is_symbol_to_string_tag_expression(&canonical_property)
                            || is_symbol_to_string_tag_expression(property_candidate),
                    ));
                }
                if static_property_name_from_expression(&canonical_property).is_none()
                    && requested_symbol.is_none()
                    && requested_well_known_symbol.is_none()
                    && (!object_binding.string_properties.is_empty()
                        || !object_binding.symbol_properties.is_empty())
                {
                    saw_dynamic_property_lookup = true;
                }
                if !object_binding.symbol_properties.is_empty()
                    && (!matches!(canonical_property, Expression::String(_))
                        || requested_symbol.is_some())
                {
                    saw_symbol_property_lookup = true;
                }
                if object_binding.runtime_symbol_properties && requested_symbol.is_some() {
                    return Some(None);
                }

                if self.runtime_object_property_shadow_deletion_may_affect_property(
                    object_candidate,
                    &canonical_property,
                ) {
                    return Some(None);
                }

                if self
                    .resolve_object_binding_property_value(&object_binding, property_candidate)
                    .is_some()
                {
                    return Some(Some(true));
                }
            }
        }

        if saw_symbol_property_lookup {
            return Some(None);
        }

        if saw_parameter_object_binding {
            return Some(None);
        }

        if saw_dynamic_property_lookup {
            return Some(None);
        }

        saw_object_binding.then_some(Some(false))
    }

    pub(in crate::backend::direct_wasm) fn resolve_function_object_has_own_property(
        &self,
        object: &Expression,
        property: &Expression,
    ) -> Option<bool> {
        self.resolve_function_binding_from_expression(object)?;

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
                if self
                    .function_object_has_explicit_own_property(object_candidate, property_candidate)
                {
                    return Some(true);
                }
                let Expression::String(property_name) = property_candidate else {
                    continue;
                };
                if property_name == "caller" || property_name == "arguments" {
                    return Some(false);
                }
                if self.runtime_object_property_shadow_deletion_may_hide_static_property(
                    object_candidate,
                    property_candidate,
                ) {
                    return None;
                }
                if self
                    .resolve_function_property_descriptor_binding(
                        object_candidate,
                        resolved_object.as_ref(),
                        &materialized_object,
                        property_name,
                    )
                    .is_some()
                {
                    return Some(true);
                }
            }
        }

        Some(false)
    }

    pub(in crate::backend::direct_wasm) fn resolve_static_has_own_property_call_result(
        &self,
        expression: &Expression,
    ) -> Option<bool> {
        let Expression::Call { callee, arguments } = expression else {
            return None;
        };
        let (object, argument_property) = match callee.as_ref() {
            Expression::Member { object, property } if matches!(property.as_ref(), Expression::String(name) if name == "hasOwnProperty") =>
            {
                let [CallArgument::Expression(argument_property)] = arguments.as_slice() else {
                    return None;
                };
                (object.as_ref(), argument_property)
            }
            Expression::Member { object, property } if matches!(property.as_ref(), Expression::String(name) if name == "call") =>
            {
                let Expression::Member {
                    object: _target_object,
                    property: target_property,
                } = object.as_ref()
                else {
                    return None;
                };
                if !matches!(target_property.as_ref(), Expression::String(name) if name == "hasOwnProperty")
                {
                    return None;
                }
                let [
                    CallArgument::Expression(receiver),
                    CallArgument::Expression(argument_property),
                    ..,
                ] = arguments.as_slice()
                else {
                    return None;
                };
                (receiver, argument_property)
            }
            _ => return None,
        };

        if Self::has_own_receiver_is_import_meta_object(object)
            && let Some(has_own_property) =
                Self::import_meta_known_property_presence(argument_property)
        {
            return Some(has_own_property);
        }
        if Self::has_own_receiver_is_dynamic_import_promise(object)
            && let Some(has_own_property) =
                Self::dynamic_import_promise_known_own_property_presence(argument_property)
        {
            return Some(has_own_property);
        }

        if let Some(array_binding) = self.resolve_array_binding_from_expression(object) {
            return Some(
                matches!(argument_property, Expression::String(property_name) if property_name == "length")
                    || argument_index_from_expression(argument_property).is_some_and(|index| {
                        array_binding
                            .values
                            .get(index as usize)
                            .is_some_and(|value| value.is_some())
                    }),
            );
        }

        if self.is_direct_arguments_object(object) {
            return match argument_property {
                Expression::String(property_name) => match property_name.as_str() {
                    "callee" | "length" => Some(self.direct_arguments_has_property(property_name)),
                    _ => canonical_array_index_from_property_name(property_name)
                        .map(|index| self.state.parameters.arguments_slots.contains_key(&index)),
                },
                _ => None,
            };
        }

        if let Some(arguments_binding) = self.resolve_arguments_binding_from_expression(object) {
            return match argument_property {
                Expression::String(property_name) => Some(match property_name.as_str() {
                    "callee" => arguments_binding.callee_present,
                    "length" => arguments_binding.length_present,
                    _ => property_name
                        .parse::<usize>()
                        .ok()
                        .is_some_and(|index| index < arguments_binding.values.len()),
                }),
                _ => None,
            };
        }

        if self
            .resolve_function_binding_from_expression(object)
            .is_some()
        {
            return self.resolve_function_object_has_own_property(object, argument_property);
        }

        if let Some(has_property) =
            self.resolve_static_object_has_own_property_result(object, argument_property)
        {
            return has_property;
        }

        if self
            .resolve_bound_function_prototype_call_descriptor(object, argument_property)
            .is_some()
        {
            return Some(true);
        }

        None
    }

    pub(in crate::backend::direct_wasm) fn resolve_static_reflect_has_call_result(
        &self,
        expression: &Expression,
    ) -> Option<bool> {
        let Expression::Call { callee, arguments } = expression else {
            return None;
        };
        let Expression::Member { object, property } = callee.as_ref() else {
            return None;
        };
        if !matches!(object.as_ref(), Expression::Identifier(name) if name == "Reflect" && self.is_unshadowed_builtin_identifier(name))
            || !matches!(property.as_ref(), Expression::String(name) if name == "has")
        {
            return None;
        }
        let target = match arguments.first() {
            Some(CallArgument::Expression(expression) | CallArgument::Spread(expression)) => {
                expression
            }
            None => return None,
        };
        let property = match arguments.get(1) {
            Some(CallArgument::Expression(expression) | CallArgument::Spread(expression)) => {
                expression.clone()
            }
            None => Expression::Undefined,
        };
        self.resolve_static_reflect_has_result(target, &property)
    }
}
