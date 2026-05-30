use super::*;

impl<'a> FunctionCompiler<'a> {
    fn resolve_static_non_finite_number_value(&self, expression: &Expression) -> Option<f64> {
        if let Some(value) = self
            .resolve_static_number_value(expression)
            .filter(|value| value.is_nan() || !value.is_finite())
        {
            return Some(value);
        }

        match expression {
            Expression::Number(value) if value.is_nan() || !value.is_finite() => Some(*value),
            Expression::Identifier(name)
                if name == "NaN" && self.is_unshadowed_builtin_identifier(name) =>
            {
                Some(f64::NAN)
            }
            Expression::Identifier(name)
                if name == "Infinity" && self.is_unshadowed_builtin_identifier(name) =>
            {
                Some(f64::INFINITY)
            }
            Expression::Unary {
                op: UnaryOp::Plus,
                expression,
            } => self.resolve_static_non_finite_number_value(expression),
            Expression::Unary {
                op: UnaryOp::Negate,
                expression,
            } => self
                .resolve_static_non_finite_number_value(expression)
                .map(|value| -value),
            Expression::Member { object, property }
                if matches!(object.as_ref(), Expression::Identifier(name) if name == "Number" && self.is_unshadowed_builtin_identifier(name))
                    && matches!(property.as_ref(), Expression::String(name) if matches!(name.as_str(), "NaN" | "POSITIVE_INFINITY" | "NEGATIVE_INFINITY")) =>
            {
                match property.as_ref() {
                    Expression::String(name) if name == "NaN" => Some(f64::NAN),
                    Expression::String(name) if name == "NEGATIVE_INFINITY" => {
                        Some(f64::NEG_INFINITY)
                    }
                    Expression::String(_) => Some(f64::INFINITY),
                    _ => None,
                }
            }
            _ => None,
        }
    }

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
        if let Expression::Sequence(expressions) = property
            && let Some(last) = expressions.last()
        {
            return self.static_property_name_for_in_query(last);
        }
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

    fn static_builtin_object_name_for_in_query(&self, object: &Expression) -> Option<String> {
        if let Expression::Sequence(expressions) = object
            && let Some(last) = expressions.last()
        {
            return self.static_builtin_object_name_for_in_query(last);
        }
        if let Expression::Identifier(name) = object
            && name == "Number"
            && self.is_unshadowed_builtin_identifier(name)
        {
            return Some(name.clone());
        }
        if let Expression::Identifier(name) = object
            && let Some(Expression::Identifier(alias)) = self
                .state
                .speculation
                .static_semantics
                .local_value_binding(name)
                .or_else(|| self.global_value_binding(name))
            && alias == "Number"
            && self.is_unshadowed_builtin_identifier(alias)
        {
            return Some(alias.clone());
        }

        let resolved = self
            .resolve_bound_alias_expression(object)
            .filter(|resolved| !static_expression_matches(resolved, object));
        if let Some(Expression::Identifier(name)) = resolved.as_ref()
            && name == "Number"
            && self.is_unshadowed_builtin_identifier(name)
        {
            return Some(name.clone());
        }

        let materialized = self.materialize_static_expression(object);
        if let Expression::Identifier(name) = materialized
            && name == "Number"
            && self.is_unshadowed_builtin_identifier(&name)
        {
            return Some(name);
        }

        None
    }

    fn last_assignment_value_to_identifier_in_expression<'b>(
        expression: &'b Expression,
        target_name: &str,
    ) -> Option<&'b Expression> {
        match expression {
            Expression::Assign { name, value } if name == target_name => Some(value),
            Expression::Sequence(expressions) => {
                expressions.iter().fold(None, |last, expression| {
                    Self::last_assignment_value_to_identifier_in_expression(expression, target_name)
                        .or(last)
                })
            }
            _ => None,
        }
    }

    fn static_builtin_object_name_for_in_query_after_left(
        &self,
        left: &Expression,
        right: &Expression,
    ) -> Option<String> {
        self.static_builtin_object_name_for_in_query(right)
            .or_else(|| {
                let Expression::Identifier(name) = right else {
                    return None;
                };
                let assigned_value =
                    Self::last_assignment_value_to_identifier_in_expression(left, name)?;
                self.static_builtin_object_name_for_in_query(assigned_value)
            })
    }

    fn static_builtin_object_has_in_property(object_name: &str, property_name: &str) -> bool {
        match object_name {
            "Number" => matches!(
                property_name,
                "MAX_VALUE" | "MIN_VALUE" | "NaN" | "POSITIVE_INFINITY" | "NEGATIVE_INFINITY"
            ),
            _ => false,
        }
    }

    fn static_in_rhs_is_primitive(&self, right: &Expression) -> bool {
        matches!(
            self.infer_value_kind(right),
            Some(
                StaticValueKind::Number
                    | StaticValueKind::Bool
                    | StaticValueKind::String
                    | StaticValueKind::BigInt
                    | StaticValueKind::Symbol
                    | StaticValueKind::Null
                    | StaticValueKind::Undefined
            )
        )
    }

    fn emit_in_expression_static_type_error(
        &mut self,
        left: &Expression,
        right: &Expression,
    ) -> DirectResult<()> {
        self.emit_numeric_expression(left)?;
        self.state.emission.output.instructions.push(0x1a);
        self.emit_numeric_expression(right)?;
        self.state.emission.output.instructions.push(0x1a);
        self.emit_named_error_throw("TypeError")
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
            self.emit_static_in_operand_effects(property, object)?;
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
            self.emit_static_in_operand_effects(property, object)?;
            self.push_i32_const(1);
            return Ok(true);
        }

        if self.expression_is_for_in_key_array_member(property) {
            if trace_for_in_keys {
                eprintln!("for_in_keys:global_in lowered_for_in_member present=true");
            }
            self.emit_static_in_operand_effects(property, object)?;
            self.push_i32_const(1);
            return Ok(true);
        }

        Ok(false)
    }

    fn emit_static_in_operand_effects(
        &mut self,
        left: &Expression,
        right: &Expression,
    ) -> DirectResult<()> {
        if !inline_summary_side_effect_free_expression(left) {
            self.emit_numeric_expression(left)?;
            self.state.emission.output.instructions.push(0x1a);
        }
        if !inline_summary_side_effect_free_expression(right) {
            self.emit_numeric_expression(right)?;
            self.state.emission.output.instructions.push(0x1a);
        }
        Ok(())
    }

    fn object_has_static_own_in_property(
        &self,
        object: &Expression,
        property: &Expression,
    ) -> bool {
        let canonical_property = self.canonical_object_property_expression(property);
        self.resolve_object_binding_from_expression(object)
            .is_some_and(|binding| {
                object_binding_has_property(&binding, &canonical_property)
                    || object_binding_lookup_descriptor(&binding, &canonical_property).is_some()
                    || object_binding_lookup_value(&binding, &canonical_property).is_some()
                    || object_binding_lookup_value(&binding, property).is_some()
            })
    }

    fn deferred_module_namespace_in_property_key(
        &self,
        property: &Expression,
    ) -> Option<Expression> {
        let property_name = self.static_property_name_for_in_query(property)?;
        if property_name == "then" || property_name.starts_with("__ayy$") {
            return None;
        }
        Some(Expression::String(property_name))
    }

    fn deferred_module_namespace_has_property_module_index(
        &self,
        object: &Expression,
        property: &Expression,
    ) -> Option<usize> {
        let property_key = self.deferred_module_namespace_in_property_key(property)?;
        if let Expression::Identifier(name) = object
            && name.starts_with("__ayy_module_deferred_namespace_")
        {
            let module_index = Self::module_index_from_namespace_like_identifier(name)?;
            if self.current_function_name().is_some_and(|function_name| {
                function_name == format!("__ayy_module_init_{module_index}")
            }) {
                return None;
            }
            return Some(module_index);
        }

        if self.object_has_static_own_in_property(object, &property_key) {
            return None;
        }

        let mut prototype = self.resolve_static_object_prototype_expression(object)?;
        for _ in 0..32 {
            let materialized_prototype = self.materialize_static_expression(&prototype);
            for candidate in [&prototype, &materialized_prototype] {
                if let Expression::Identifier(name) = candidate
                    && name.starts_with("__ayy_module_deferred_namespace_")
                {
                    let module_index = Self::module_index_from_namespace_like_identifier(name)?;
                    if self.current_function_name().is_some_and(|function_name| {
                        function_name == format!("__ayy_module_init_{module_index}")
                    }) {
                        return None;
                    }
                    return Some(module_index);
                }
                if self.object_has_static_own_in_property(candidate, &property_key) {
                    return None;
                }
            }
            if matches!(materialized_prototype, Expression::Null) {
                return None;
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

    fn emit_deferred_module_namespace_has_property(
        &mut self,
        property: &Expression,
        object: &Expression,
    ) -> DirectResult<bool> {
        let Some(property_key) = self.deferred_module_namespace_in_property_key(property) else {
            return Ok(false);
        };
        let Some(module_index) =
            self.deferred_module_namespace_has_property_module_index(object, &property_key)
        else {
            return Ok(false);
        };

        self.emit_static_in_operand_effects(property, object)?;
        self.emit_sync_module_init_if_needed(module_index, &mut std::collections::HashSet::new())?;
        let has_property = self
            .resolve_static_dynamic_import_namespace_live_binding_member_value(
                module_index,
                &property_key,
            )
            .or_else(|| {
                self.resolve_static_dynamic_import_namespace_live_binding_member_initializer_value(
                    module_index,
                    &property_key,
                )
            })
            .is_some();
        self.push_i32_const(i32::from(has_property));
        Ok(true)
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

    pub(in crate::backend::direct_wasm) fn emit_static_bigint_non_finite_loose_equality(
        &mut self,
        op: BinaryOp,
        left: &Expression,
        right: &Expression,
    ) -> DirectResult<bool> {
        if !matches!(op, BinaryOp::LooseEqual | BinaryOp::LooseNotEqual)
            || !inline_summary_side_effect_free_expression(left)
            || !inline_summary_side_effect_free_expression(right)
        {
            return Ok(false);
        }

        let left_is_bigint = self.resolve_static_bigint_value(left).is_some();
        let right_is_bigint = self.resolve_static_bigint_value(right).is_some();
        let left_is_non_finite_number = self.resolve_static_non_finite_number_value(left).is_some();
        let right_is_non_finite_number =
            self.resolve_static_non_finite_number_value(right).is_some();

        if (left_is_bigint && right_is_non_finite_number)
            || (right_is_bigint && left_is_non_finite_number)
        {
            self.push_i32_const(matches!(op, BinaryOp::LooseNotEqual) as i32);
            return Ok(true);
        }

        Ok(false)
    }

    pub(in crate::backend::direct_wasm) fn emit_in_expression(
        &mut self,
        left: &Expression,
        right: &Expression,
    ) -> DirectResult<()> {
        if matches!(
            static_property_name_from_expression(left).as_deref(),
            Some("prototype")
        ) && matches!(
            right,
            Expression::Member { object, property }
                if matches!(
                    (object.as_ref(), property.as_ref()),
                    (Expression::Identifier(name), Expression::String(property_name))
                        if matches!(property_name.as_str(), "get" | "set")
                            && self.local_binding_is_dynamic_property_descriptor_result(name)
                )
        ) {
            if !inline_summary_side_effect_free_expression(left) {
                self.emit_numeric_expression(left)?;
                self.state.emission.output.instructions.push(0x1a);
            }
            self.push_i32_const(0);
            return Ok(());
        }

        if self.emit_deferred_module_namespace_has_property(left, right)? {
            return Ok(());
        }

        if self.emit_static_top_level_global_object_in_expression(left, right)? {
            return Ok(());
        }

        if let Some(result) = self.resolve_static_in_expression_result(left, right) {
            self.emit_static_in_operand_effects(left, right)?;
            self.push_i32_const(i32::from(result));
            return Ok(());
        }

        if let Some(array_binding) = self.resolve_array_binding_from_expression(right) {
            if matches!(left, Expression::String(property_name) if property_name == "length") {
                self.emit_static_in_operand_effects(left, right)?;
                self.push_i32_const(1);
                return Ok(());
            }
            let materialized_left = self.materialize_static_expression(left);
            if let Some(index) = argument_index_from_expression(left)
                .or_else(|| argument_index_from_expression(&materialized_left))
            {
                self.emit_static_in_operand_effects(left, right)?;
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
                self.emit_static_in_operand_effects(left, right)?;
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
        if let Some(object_name) =
            self.static_builtin_object_name_for_in_query_after_left(left, right)
            && let Some(property_name) = self.static_property_name_for_in_query(left)
        {
            let has_property =
                Self::static_builtin_object_has_in_property(&object_name, &property_name);
            if has_property {
                self.emit_static_in_operand_effects(left, right)?;
                self.push_i32_const(1);
                return Ok(());
            }
        }
        if self.static_in_rhs_is_primitive(right) {
            return self.emit_in_expression_static_type_error(left, right);
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
                                .resolve_object_binding_has_property_with_inherited(
                                    right,
                                    &object_binding,
                                    &Expression::String(property_name.clone()),
                                )
                    )
                })
            {
                self.emit_static_in_operand_effects(left, right)?;
                self.push_i32_const(1);
                return Ok(());
            }
            let materialized_left = self.materialize_static_expression(left);
            self.emit_static_in_operand_effects(left, right)?;
            self.push_i32_const(
                if self.resolve_object_binding_has_property_with_inherited(
                    right,
                    &object_binding,
                    &materialized_left,
                ) {
                    1
                } else {
                    0
                },
            );
            return Ok(());
        }
        self.emit_numeric_expression(left)?;
        self.state.emission.output.instructions.push(0x1a);
        self.emit_numeric_expression(right)?;
        self.state.emission.output.instructions.push(0x1a);
        self.push_i32_const(0);
        Ok(())
    }
}
