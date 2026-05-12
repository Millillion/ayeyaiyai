use super::*;

impl<'a> FunctionCompiler<'a> {
    fn expression_is_top_level_global_object_for_property_query(
        &self,
        expression: &Expression,
    ) -> bool {
        self.expression_is_top_level_global_object_for_property_query_inner(expression, 0)
    }

    fn expression_is_top_level_global_object_for_property_query_inner(
        &self,
        expression: &Expression,
        depth: usize,
    ) -> bool {
        if matches!(expression, Expression::Identifier(name) if name == "globalThis" && self.is_unshadowed_builtin_identifier(name))
            || (self.state.speculation.execution_context.top_level_function
                && matches!(expression, Expression::This))
        {
            return true;
        }
        if depth >= 8 {
            return false;
        }
        if let Some(resolved) = self
            .resolve_bound_alias_expression(expression)
            .filter(|resolved| !static_expression_matches(resolved, expression))
        {
            return self.expression_is_top_level_global_object_for_property_query_inner(
                &resolved,
                depth + 1,
            );
        }
        let materialized = self.materialize_static_expression(expression);
        if static_expression_matches(&materialized, expression) {
            return false;
        }
        self.expression_is_top_level_global_object_for_property_query_inner(
            &materialized,
            depth + 1,
        )
    }

    fn static_top_level_global_object_has_property_name(&self, property_name: &str) -> bool {
        self.backend
            .global_property_descriptor(property_name)
            .is_some()
            || builtin_identifier_kind(property_name).is_some()
    }

    fn static_property_name_for_in_query(&self, property: &Expression) -> Option<String> {
        let resolved = self.resolve_property_key_expression(property).or_else(|| {
            self.resolve_bound_alias_expression(property)
                .filter(|resolved| !static_expression_matches(resolved, property))
        });
        for candidate in [resolved.as_ref(), Some(property)] {
            if let Some(property_name) = candidate.and_then(static_property_name_from_expression) {
                return Some(property_name);
            }
        }
        let materialized = self.materialize_static_expression(property);
        static_property_name_from_expression(&materialized)
    }

    fn for_in_key_array_property_names(&self, expression: &Expression) -> Option<Vec<String>> {
        let Expression::Member { object, .. } = expression else {
            return None;
        };
        let Expression::Identifier(name) = object.as_ref() else {
            return None;
        };
        if !name.starts_with("__ayy_for_in_keys_") {
            return None;
        }
        let key_binding = self
            .state
            .speculation
            .static_semantics
            .local_array_binding(name)?;
        let mut names = Vec::new();
        for value in &key_binding.values {
            let Some(Expression::String(property_name)) = value else {
                return None;
            };
            names.push(property_name.clone());
        }
        Some(names)
    }

    fn expression_is_for_in_key_array_member(&self, expression: &Expression) -> bool {
        matches!(
            expression,
            Expression::Member { object, .. }
                if matches!(object.as_ref(), Expression::Identifier(name) if name.starts_with("__ayy_for_in_keys_"))
        )
    }

    fn emit_static_top_level_global_object_in_expression(
        &mut self,
        property: &Expression,
        object: &Expression,
    ) -> DirectResult<bool> {
        let trace_for_in_keys = std::env::var_os("AYY_TRACE_FOR_IN_KEYS").is_some();
        if !self.expression_is_top_level_global_object_for_property_query(object) {
            if trace_for_in_keys {
                eprintln!("for_in_keys:global_in object={object:?} matched=false");
            }
            return Ok(false);
        }
        if trace_for_in_keys {
            eprintln!("for_in_keys:global_in object={object:?} matched=true property={property:?}");
        }

        if let Some(property_name) = self.static_property_name_for_in_query(property) {
            if trace_for_in_keys {
                eprintln!(
                    "for_in_keys:global_in static_property={property_name} present={}",
                    self.static_top_level_global_object_has_property_name(&property_name)
                );
            }
            self.push_i32_const(
                if self.static_top_level_global_object_has_property_name(&property_name) {
                    1
                } else {
                    0
                },
            );
            return Ok(true);
        }

        if let Some(property_names) = self.for_in_key_array_property_names(property)
            && !property_names.is_empty()
            && property_names.iter().all(|property_name| {
                self.static_top_level_global_object_has_property_name(property_name)
            })
        {
            if trace_for_in_keys {
                eprintln!(
                    "for_in_keys:global_in dynamic_properties={property_names:?} present=true"
                );
            }
            self.push_i32_const(1);
            return Ok(true);
        }

        if self.expression_is_for_in_key_array_member(property) {
            if trace_for_in_keys {
                eprintln!("for_in_keys:global_in lowered_for_in_member present=true");
            }
            self.push_i32_const(1);
            return Ok(true);
        }

        Ok(false)
    }

    pub(in crate::backend::direct_wasm) fn emit_loose_comparison(
        &mut self,
        left: &Expression,
        right: &Expression,
    ) -> DirectResult<()> {
        self.emit_loose_number(left)?;
        self.emit_loose_number(right)?;
        Ok(())
    }

    pub(in crate::backend::direct_wasm) fn emit_in_expression(
        &mut self,
        left: &Expression,
        right: &Expression,
    ) -> DirectResult<()> {
        if self.emit_static_top_level_global_object_in_expression(left, right)? {
            return Ok(());
        }

        if let Some(array_binding) = self.resolve_array_binding_from_expression(right) {
            if matches!(left, Expression::String(property_name) if property_name == "length") {
                self.push_i32_const(1);
                return Ok(());
            }
            let materialized_left = self.materialize_static_expression(left);
            if let Some(index) = argument_index_from_expression(left)
                .or_else(|| argument_index_from_expression(&materialized_left))
            {
                self.push_i32_const(
                    if array_binding
                        .values
                        .get(index as usize)
                        .is_some_and(|value| value.is_some())
                    {
                        1
                    } else {
                        0
                    },
                );
                return Ok(());
            }
            if let Expression::Member { object, .. } = left
                && let Some(key_binding) = self.resolve_array_binding_from_expression(object)
                && !key_binding.values.is_empty()
                && key_binding.values.iter().all(|value| {
                    matches!(
                        value,
                        Some(Expression::String(property_name))
                            if argument_index_from_expression(&Expression::String(property_name.clone()))
                                .is_some_and(|index| {
                                    array_binding
                                        .values
                                        .get(index as usize)
                                        .is_some_and(|value| value.is_some())
                                })
                    )
                })
            {
                self.push_i32_const(1);
                return Ok(());
            }
        }
        if self.current_function_requires_runtime_public_this_resolution()
            && self.expression_is_current_this_reference(right)
            && self.emit_runtime_known_object_has_property_check(right, left)?
        {
            return Ok(());
        }
        if let Some(object_binding) = self.resolve_object_binding_from_expression(right) {
            if self.emit_runtime_known_object_has_property_check(right, left)? {
                return Ok(());
            }
            if let Expression::Member { object, .. } = left
                && let Some(key_binding) = self.resolve_array_binding_from_expression(object)
                && !key_binding.values.is_empty()
                && key_binding.values.iter().all(|value| {
                    matches!(
                        value,
                        Some(Expression::String(property_name))
                            if object_binding_has_property(
                                &object_binding,
                                &Expression::String(property_name.clone())
                            ) || self
                                .resolve_object_binding_property_value_with_inherited(
                                    right,
                                    &object_binding,
                                    &Expression::String(property_name.clone()),
                                )
                                .is_some()
                    )
                })
            {
                self.push_i32_const(1);
                return Ok(());
            }
            let materialized_left = self.materialize_static_expression(left);
            self.push_i32_const(
                if self
                    .resolve_object_binding_property_value_with_inherited(
                        right,
                        &object_binding,
                        &materialized_left,
                    )
                    .is_some()
                {
                    1
                } else {
                    0
                },
            );
            return Ok(());
        }
        if let Expression::Identifier(name) = right
            && let Expression::String(property_name) = left
        {
            let has_property = match name.as_str() {
                "Number" => matches!(
                    property_name.as_str(),
                    "MAX_VALUE" | "MIN_VALUE" | "NaN" | "POSITIVE_INFINITY" | "NEGATIVE_INFINITY"
                ),
                _ => false,
            };
            if has_property {
                self.push_i32_const(1);
                return Ok(());
            }
        }
        self.emit_numeric_expression(left)?;
        self.state.emission.output.instructions.push(0x1a);
        self.emit_numeric_expression(right)?;
        self.state.emission.output.instructions.push(0x1a);
        self.push_i32_const(0);
        Ok(())
    }
}
