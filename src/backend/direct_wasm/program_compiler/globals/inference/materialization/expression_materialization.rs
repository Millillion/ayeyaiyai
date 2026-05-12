use super::*;

impl DirectWasmCompiler {
    fn preserves_global_symbol_call_binding(&self, value: &Expression) -> bool {
        matches!(
            value,
            Expression::Call { callee, .. }
                if matches!(callee.as_ref(), Expression::Identifier(symbol_name)
                    if symbol_name == "Symbol"
                        && !self.global_has_binding(symbol_name)
                        && !self.global_has_lexical_binding(symbol_name))
        )
    }

    pub(in crate::backend::direct_wasm) fn global_expression_is_static_symbol_property_key(
        &self,
        expression: &Expression,
    ) -> bool {
        match expression {
            Expression::Identifier(name) => {
                self.global_binding_kind(name) == Some(StaticValueKind::Symbol)
                    || self
                        .global_value_binding(name)
                        .is_some_and(|value| self.preserves_global_symbol_call_binding(value))
            }
            Expression::Member { object, property }
                if matches!(object.as_ref(), Expression::Identifier(name)
                    if name == "Symbol"
                        && !self.global_has_binding(name)
                        && !self.global_has_lexical_binding(name))
                    && matches!(property.as_ref(), Expression::String(_)) =>
            {
                true
            }
            Expression::Call { callee, .. }
                if matches!(callee.as_ref(), Expression::Identifier(name)
                    if name == "Symbol"
                        && !self.global_has_binding(name)
                        && !self.global_has_lexical_binding(name)) =>
            {
                true
            }
            _ => false,
        }
    }

    pub(in crate::backend::direct_wasm) fn canonical_global_object_property_expression(
        &self,
        property: &Expression,
    ) -> Expression {
        let resolved = match property {
            Expression::Identifier(name) => self
                .resolve_static_class_init_local_alias_expression(name)
                .unwrap_or_else(|| property.clone()),
            _ => property.clone(),
        };
        let evaluated = self
            .evaluate_static_expression(&resolved)
            .unwrap_or_else(|| self.materialize_global_expression(&resolved));
        if self.global_expression_is_static_symbol_property_key(&resolved) {
            return resolved;
        }
        if self.global_expression_is_static_symbol_property_key(&evaluated) {
            return evaluated;
        }
        evaluated
    }

    pub(in crate::backend::direct_wasm) fn materialize_global_expression_with_state(
        &self,
        expression: &Expression,
        local_bindings: &HashMap<String, Expression>,
        value_bindings: &HashMap<String, Expression>,
        object_bindings: &HashMap<String, ObjectValueBinding>,
    ) -> Option<Expression> {
        let context = self.static_eval_context();
        materialize_expression_in_binding_maps(
            &context,
            expression,
            local_bindings,
            value_bindings,
            object_bindings,
            &|expression, local_bindings, value_bindings, object_bindings| {
                resolve_stateful_object_binding_in_binding_maps(
                    expression,
                    local_bindings,
                    value_bindings,
                    object_bindings,
                    &|expression, _local_bindings, value_bindings, object_bindings| {
                        self.infer_global_object_binding_with_state(
                            expression,
                            &mut value_bindings.clone(),
                            &mut object_bindings.clone(),
                        )
                    },
                )
            },
            &|object, property| {
                preserves_missing_member_function_capture(
                    object,
                    property,
                    |object, property| self.global_member_function_binding_key(object, property),
                    |key| self.has_global_member_function_capture_slots(key),
                )
            },
        )
    }

    pub(in crate::backend::direct_wasm) fn materialize_global_expression(
        &self,
        expression: &Expression,
    ) -> Expression {
        match expression {
            Expression::Identifier(name) => {
                if name == "undefined"
                    && !self.global_has_binding(name)
                    && !self.global_has_lexical_binding(name)
                {
                    return Expression::Undefined;
                }
                if self.global_binding_kind(name) == Some(StaticValueKind::Symbol) {
                    return expression.clone();
                }
                if self
                    .global_value_binding(name)
                    .is_some_and(|value| self.preserves_global_symbol_call_binding(value))
                {
                    return expression.clone();
                }
                if let Some(value) = self.global_value_binding(name) {
                    if self.global_object_binding(name).is_some()
                        && matches!(value, Expression::Object(_) | Expression::Identifier(_))
                    {
                        return Expression::Identifier(name.clone());
                    }
                    if !matches!(value, Expression::Identifier(alias) if alias == name) {
                        return self.materialize_global_expression(value);
                    }
                }
                expression.clone()
            }
            Expression::Member { object, property } => {
                if self
                    .global_member_function_binding_key(object, property)
                    .is_some_and(|key| self.has_global_member_function_capture_slots(&key))
                {
                    return expression.clone();
                }
                if let Some(array_binding) = self.infer_global_array_binding(object)
                    && let Some(index) = argument_index_from_expression(property)
                {
                    if let Some(Some(value)) = array_binding.values.get(index as usize) {
                        return self.materialize_global_expression(value);
                    }
                    return Expression::Undefined;
                }
                if let Some(object_binding) = self.infer_global_object_binding(object) {
                    let materialized_property = self.materialize_global_expression(property);
                    if let Some(value) =
                        object_binding_lookup_value(&object_binding, &materialized_property)
                    {
                        return self.materialize_global_expression(value);
                    }
                    if static_property_name_from_expression(&materialized_property).is_some()
                        || object_binding_has_property(&object_binding, &materialized_property)
                    {
                        return Expression::Undefined;
                    }
                }
                if let Expression::String(text) = object.as_ref()
                    && let Some(index) = argument_index_from_expression(property)
                {
                    return text
                        .chars()
                        .nth(index as usize)
                        .map(|character| Expression::String(character.to_string()))
                        .unwrap_or(Expression::Undefined);
                }
                let materialized_property = self.materialize_global_expression(property);
                let materialized = Expression::Member {
                    object: Box::new(self.materialize_global_expression(object)),
                    property: Box::new(materialized_property.clone()),
                };
                materialize_missing_member_expression_with_policy(
                    expression,
                    object,
                    materialized_property,
                    &(),
                    &|expression, _| Some(self.materialize_global_expression(expression)),
                    &|_full_expression, object, property, _environment| {
                        preserves_missing_member_function_capture(
                            object,
                            property,
                            |object, property| {
                                self.global_member_function_binding_key(object, property)
                            },
                            |key| self.has_global_member_function_capture_slots(key),
                        )
                    },
                )
                .unwrap_or(materialized)
            }
            Expression::Call { callee, arguments } => {
                if let Some(value) = self.infer_static_call_result_expression(callee, arguments) {
                    return self.materialize_global_expression(&value);
                }
                materialize_recursive_expression(expression, true, true, &|expression| {
                    Some(self.materialize_global_expression(expression))
                })
                .expect("program-side recursive materialization supports generic call rebuild")
            }
            _ => materialize_recursive_expression(expression, true, true, &|expression| {
                Some(self.materialize_global_expression(expression))
            })
            .unwrap_or_else(|| expression.clone()),
        }
    }
}
