use super::*;

impl<'a> FunctionCompiler<'a> {
    fn expression_is_top_level_global_object_reference(&self, expression: &Expression) -> bool {
        matches!(expression, Expression::Identifier(name) if name == "globalThis" && self.is_unshadowed_builtin_identifier(name))
            || (self.state.speculation.execution_context.top_level_function
                && matches!(expression, Expression::This))
    }

    fn emit_top_level_global_object_member_delete(
        &mut self,
        object: &Expression,
        property: &Expression,
    ) -> DirectResult<bool> {
        if !self.expression_is_top_level_global_object_reference(object) {
            return Ok(false);
        }

        let resolved_property = self
            .resolve_property_key_expression(property)
            .unwrap_or_else(|| self.materialize_static_expression(property));
        let Expression::String(property_name) = resolved_property else {
            return Ok(false);
        };

        if self
            .backend
            .global_property_descriptor(&property_name)
            .is_some_and(|descriptor| !descriptor.configurable)
        {
            self.push_i32_const(0);
            return Ok(true);
        }

        if builtin_identifier_kind(&property_name).is_some()
            && !builtin_identifier_delete_returns_true(&property_name)
        {
            self.push_i32_const(0);
            return Ok(true);
        }

        Ok(false)
    }

    fn object_binding_property_removal_plan(
        &self,
        object_binding: &ObjectValueBinding,
        property: &Expression,
    ) -> (Option<String>, Vec<Expression>) {
        let resolved_property = self.canonical_object_property_expression(property);
        if let Some(property_name) = static_property_name_from_expression(&resolved_property) {
            return (Some(property_name), Vec::new());
        }

        let matching_keys = object_binding
            .symbol_properties
            .iter()
            .filter_map(|(existing_key, _)| {
                let resolved_existing = self.canonical_object_property_expression(existing_key);
                static_expression_matches(&resolved_existing, &resolved_property)
                    .then_some(existing_key.clone())
            })
            .collect();
        (None, matching_keys)
    }

    fn emit_dynamic_symbol_named_object_member_delete(
        &mut self,
        name: &str,
        property: &Expression,
    ) -> DirectResult<bool> {
        let object_expression = Expression::Identifier(name.to_string());
        let Some(object_binding) = self.resolve_object_binding_from_expression(&object_expression)
        else {
            return Ok(false);
        };
        let owner_name = if let Some(owner_name) =
            self.runtime_object_property_shadow_owner_name_for_identifier(name)
        {
            owner_name
        } else {
            if !self.binding_name_is_global(name)
                && !self
                    .state
                    .speculation
                    .static_semantics
                    .has_local_object_binding(name)
            {
                let local_object_binding = self
                    .state
                    .speculation
                    .static_semantics
                    .ensure_local_object_binding(name);
                *local_object_binding = object_binding.clone();
            }
            let Some(owner_name) =
                self.runtime_object_property_shadow_owner_name_for_identifier(name)
            else {
                return Ok(false);
            };
            owner_name
        };
        if object_binding.symbol_properties.is_empty() {
            return Ok(false);
        }
        if let Some(object_binding) = self
            .state
            .speculation
            .static_semantics
            .local_object_binding_mut(name)
        {
            object_binding.runtime_symbol_properties = true;
        }
        if let Some(object_binding) = self
            .backend
            .global_semantics
            .values
            .object_bindings
            .get_mut(name)
        {
            object_binding.runtime_symbol_properties = true;
        }

        let property_local = self.allocate_temp_local();
        self.emit_numeric_expression(property)?;
        self.push_local_set(property_local);

        if let Some((existing_key, _)) =
            self.resolve_static_symbol_property_shadow_entry(&object_binding, property)
        {
            let binding =
                self.runtime_object_property_shadow_binding_by_property(&owner_name, &existing_key);
            let deleted_binding = self.runtime_object_property_shadow_deleted_binding_by_property(
                &owner_name,
                &existing_key,
            );
            self.push_i32_const(JS_UNDEFINED_TAG);
            self.push_global_set(binding.value_index);
            self.push_i32_const(0);
            self.push_global_set(binding.present_index);
            self.push_i32_const(JS_UNDEFINED_TAG);
            self.push_global_set(deleted_binding.value_index);
            self.push_i32_const(1);
            self.push_global_set(deleted_binding.present_index);
            self.push_i32_const(1);
            return Ok(true);
        }

        let mut open_frames = 0;
        for (existing_key, _) in object_binding.symbol_properties {
            let comparison_key = self.canonical_object_property_expression(&existing_key);
            self.push_local_get(property_local);
            self.emit_numeric_expression(&comparison_key)?;
            self.push_binary_op(BinaryOp::Equal)?;
            self.state.emission.output.instructions.push(0x04);
            self.state.emission.output.instructions.push(I32_TYPE);
            self.push_control_frame();
            open_frames += 1;
            let binding =
                self.runtime_object_property_shadow_binding_by_property(&owner_name, &existing_key);
            let deleted_binding = self.runtime_object_property_shadow_deleted_binding_by_property(
                &owner_name,
                &existing_key,
            );
            self.push_i32_const(JS_UNDEFINED_TAG);
            self.push_global_set(binding.value_index);
            self.push_i32_const(0);
            self.push_global_set(binding.present_index);
            self.push_i32_const(JS_UNDEFINED_TAG);
            self.push_global_set(deleted_binding.value_index);
            self.push_i32_const(1);
            self.push_global_set(deleted_binding.present_index);
            self.push_i32_const(1);
            self.state.emission.output.instructions.push(0x05);
        }

        self.push_i32_const(1);
        for _ in 0..open_frames {
            self.state.emission.output.instructions.push(0x0b);
            self.pop_control_frame();
        }
        Ok(true)
    }

    fn emit_dynamic_string_named_object_member_delete(
        &mut self,
        name: &str,
        property: &Expression,
    ) -> DirectResult<bool> {
        let object_expression = Expression::Identifier(name.to_string());
        let Some(object_binding) = self.resolve_object_binding_from_expression(&object_expression)
        else {
            if std::env::var_os("AYY_TRACE_RUNTIME_SHADOWS").is_some() {
                eprintln!("dynamic_string_delete object={name} binding=<none>");
            }
            return Ok(false);
        };
        if std::env::var_os("AYY_TRACE_RUNTIME_SHADOWS").is_some() {
            eprintln!(
                "dynamic_string_delete object={name} keys={:?}",
                object_binding
                    .string_properties
                    .iter()
                    .map(|(property_name, _)| property_name.clone())
                    .collect::<Vec<_>>()
            );
        }
        if object_binding.string_properties.is_empty() {
            return Ok(false);
        }
        let owner_name = if let Some(owner_name) =
            self.runtime_object_property_shadow_owner_name_for_identifier(name)
        {
            owner_name
        } else {
            if !self.binding_name_is_global(name)
                && !self
                    .state
                    .speculation
                    .static_semantics
                    .has_local_object_binding(name)
            {
                let local_object_binding = self
                    .state
                    .speculation
                    .static_semantics
                    .ensure_local_object_binding(name);
                *local_object_binding = object_binding.clone();
            }
            let Some(owner_name) =
                self.runtime_object_property_shadow_owner_name_for_identifier(name)
            else {
                return Ok(false);
            };
            owner_name
        };

        if matches!(property, Expression::Identifier(property_name) if property_name == "name")
            && object_binding
                .string_properties
                .iter()
                .any(|(property_name, _)| property_name == "name")
        {
            let existing_key = Expression::String("name".to_string());
            let binding =
                self.runtime_object_property_shadow_binding_by_property(&owner_name, &existing_key);
            let deleted_binding = self.runtime_object_property_shadow_deleted_binding_by_property(
                &owner_name,
                &existing_key,
            );
            self.push_i32_const(JS_UNDEFINED_TAG);
            self.push_global_set(binding.value_index);
            self.push_i32_const(0);
            self.push_global_set(binding.present_index);
            self.push_i32_const(JS_UNDEFINED_TAG);
            self.push_global_set(deleted_binding.value_index);
            self.push_i32_const(1);
            self.push_global_set(deleted_binding.present_index);
            self.push_i32_const(1);
            return Ok(true);
        }

        let property_local = self.allocate_temp_local();
        self.emit_numeric_expression(property)?;
        self.push_local_set(property_local);

        let mut open_frames = 0;
        for (property_name, _) in object_binding.string_properties {
            let existing_key = Expression::String(property_name);
            self.push_local_get(property_local);
            self.emit_numeric_expression(&existing_key)?;
            self.push_binary_op(BinaryOp::Equal)?;
            self.state.emission.output.instructions.push(0x04);
            self.state.emission.output.instructions.push(I32_TYPE);
            self.push_control_frame();
            open_frames += 1;
            let binding =
                self.runtime_object_property_shadow_binding_by_property(&owner_name, &existing_key);
            let deleted_binding = self.runtime_object_property_shadow_deleted_binding_by_property(
                &owner_name,
                &existing_key,
            );
            self.push_i32_const(JS_UNDEFINED_TAG);
            self.push_global_set(binding.value_index);
            self.push_i32_const(0);
            self.push_global_set(binding.present_index);
            self.push_i32_const(JS_UNDEFINED_TAG);
            self.push_global_set(deleted_binding.value_index);
            self.push_i32_const(1);
            self.push_global_set(deleted_binding.present_index);
            self.push_i32_const(1);
            self.state.emission.output.instructions.push(0x05);
        }

        self.push_i32_const(1);
        for _ in 0..open_frames {
            self.state.emission.output.instructions.push(0x0b);
            self.pop_control_frame();
        }
        Ok(true)
    }

    pub(in crate::backend::direct_wasm) fn emit_delete_expression(
        &mut self,
        expression: &Expression,
    ) -> DirectResult<()> {
        if let Expression::Identifier(name) = expression
            && let Some(scope_object) = self.resolve_with_scope_binding(name)?
        {
            let member_expression = Expression::Member {
                object: Box::new(scope_object),
                property: Box::new(Expression::String(name.clone())),
            };
            return self.emit_delete_expression(&member_expression);
        }

        match expression {
            Expression::Identifier(name)
                if self.resolve_current_local_binding(name).is_none()
                    && self.backend.global_binding_index(name).is_none()
                    && self.resolve_eval_local_function_hidden_name(name).is_some() =>
            {
                self.clear_eval_local_function_binding_metadata(name);
                self.emit_delete_eval_local_function_binding(name)?;
                return Ok(());
            }
            Expression::Identifier(name)
                if self.resolve_current_local_binding(name).is_none()
                    && self.backend.global_binding_index(name).is_none()
                    && self.backend.global_has_implicit_binding(name) =>
            {
                self.state
                    .runtime
                    .locals
                    .deleted_builtin_identifiers
                    .remove(name);
                self.emit_delete_implicit_global_binding(name)?;
                return Ok(());
            }
            Expression::Identifier(name)
                if self.resolve_current_local_binding(name).is_none()
                    && self.backend.global_binding_index(name).is_none()
                    && self.is_unshadowed_builtin_identifier(name)
                    && builtin_identifier_delete_returns_true(name) =>
            {
                self.clear_static_identifier_binding_metadata(name);
                self.state
                    .runtime
                    .locals
                    .deleted_builtin_identifiers
                    .insert(name.clone());
                self.push_i32_const(1);
                return Ok(());
            }
            Expression::Identifier(name)
                if self.is_current_arguments_binding_name(name)
                    && self
                        .current_user_function()
                        .is_some_and(|function| !function.lexical_this) =>
            {
                self.push_i32_const(0);
            }
            Expression::Identifier(name) if self.is_identifier_bound(name) => {
                self.push_i32_const(0);
            }
            Expression::Identifier(_) => {
                self.push_i32_const(1);
            }
            Expression::Member { object, property } if matches!(property.as_ref(), Expression::String(property_name) if property_name == "callee" || property_name == "length") =>
            {
                let Expression::String(property_name) = property.as_ref() else {
                    unreachable!("filtered above");
                };
                if self.is_direct_arguments_object(object) {
                    match property_name.as_str() {
                        "callee" => {
                            if self.state.speculation.execution_context.strict_mode {
                                self.push_i32_const(0);
                            } else {
                                self.apply_current_arguments_effect(
                                    "callee",
                                    ArgumentsPropertyEffect::Delete,
                                );
                                self.push_i32_const(1);
                            }
                        }
                        "length" => {
                            self.apply_current_arguments_effect(
                                "length",
                                ArgumentsPropertyEffect::Delete,
                            );
                            self.push_i32_const(1);
                        }
                        _ => unreachable!("filtered above"),
                    }
                    self.emit_delete_result_or_throw_if_strict()?;
                    return Ok(());
                }
                if let Some(arguments_binding) =
                    self.resolve_arguments_binding_from_expression(object)
                {
                    self.emit_numeric_expression(object)?;
                    self.state.emission.output.instructions.push(0x1a);
                    self.emit_numeric_expression(property)?;
                    self.state.emission.output.instructions.push(0x1a);
                    if property_name == "callee" && arguments_binding.strict {
                        self.push_i32_const(0);
                    } else {
                        self.update_named_arguments_binding_effect(
                            object,
                            property_name,
                            ArgumentsPropertyEffect::Delete,
                        );
                        self.push_i32_const(1);
                    }
                    return Ok(());
                }
                if property_name == "length"
                    && self.resolve_array_binding_from_expression(object).is_some()
                {
                    self.push_i32_const(0);
                    return Ok(());
                }
                self.emit_numeric_expression(expression)?;
                self.state.emission.output.instructions.push(0x1a);
                self.push_i32_const(1);
            }
            Expression::Member { object, property }
                if self.is_direct_arguments_object(object)
                    && argument_index_from_expression(property).is_some() =>
            {
                self.emit_arguments_slot_delete(
                    argument_index_from_expression(property).expect("checked above"),
                );
                self.emit_delete_result_or_throw_if_strict()?;
                return Ok(());
            }
            Expression::Member { object, property } if self.is_direct_arguments_object(object) => {
                self.emit_dynamic_direct_arguments_property_delete(property)?;
                self.emit_delete_result_or_throw_if_strict()?;
                return Ok(());
            }
            Expression::Member { object, property }
                if argument_index_from_expression(property).is_some() =>
            {
                let index = argument_index_from_expression(property).expect("checked above");
                if let Expression::Identifier(name) = object.as_ref() {
                    if let Some(array_binding) = self
                        .state
                        .speculation
                        .static_semantics
                        .local_array_binding_mut(name)
                    {
                        if let Some(value) = array_binding.values.get_mut(index as usize) {
                            *value = None;
                        }
                        self.clear_runtime_array_slot(name, index);
                        self.push_i32_const(1);
                        return Ok(());
                    }
                    if let Some(array_binding) = self
                        .backend
                        .global_semantics
                        .values
                        .array_bindings
                        .get_mut(name)
                    {
                        if let Some(value) = array_binding.values.get_mut(index as usize) {
                            *value = None;
                        }
                        self.clear_global_runtime_array_slot(name, index);
                        self.push_i32_const(1);
                        return Ok(());
                    }
                    if let Some(arguments_binding) =
                        self.state.parameters.local_arguments_bindings.get_mut(name)
                    {
                        if let Some(value) = arguments_binding.values.get_mut(index as usize) {
                            *value = Expression::Undefined;
                        }
                        self.push_i32_const(1);
                        return Ok(());
                    }
                    if let Some(arguments_binding) = self
                        .backend
                        .global_semantics
                        .values
                        .arguments_bindings
                        .get_mut(name)
                    {
                        if let Some(value) = arguments_binding.values.get_mut(index as usize) {
                            *value = Expression::Undefined;
                        }
                        self.push_i32_const(1);
                        return Ok(());
                    }
                }
                self.emit_numeric_expression(expression)?;
                self.state.emission.output.instructions.push(0x1a);
                self.push_i32_const(1);
            }
            Expression::Member { object, property }
                if self.is_array_prototype_symbol_iterator_member(object, property) =>
            {
                self.emit_array_prototype_symbol_iterator_deleted_marker(true)?;
                self.push_i32_const(1);
                return Ok(());
            }
            Expression::Member { object, property } => {
                if self.emit_top_level_global_object_member_delete(object, property)? {
                    return Ok(());
                }
                let resolved_property = self
                    .resolve_property_key_expression(property)
                    .or_else(|| {
                        self.resolve_static_string_value(property)
                            .map(Expression::String)
                    })
                    .unwrap_or_else(|| self.materialize_static_expression(property));
                if matches!(
                    resolved_property,
                    Expression::String(ref property_name) if property_name == "length"
                ) && self.resolve_array_binding_from_expression(object).is_some()
                {
                    self.push_i32_const(0);
                    return Ok(());
                }
                if let (Expression::Identifier(object_name), Expression::String(property_name)) = (
                    self.materialize_static_expression(object),
                    resolved_property.clone(),
                ) && self.is_unshadowed_builtin_identifier(&object_name)
                    && builtin_member_delete_returns_false(&object_name, &property_name)
                {
                    self.push_i32_const(0);
                    return Ok(());
                }
                if let Expression::Identifier(name) = object.as_ref() {
                    let materialized_property =
                        self.canonical_object_property_expression(&resolved_property);
                    let local_removal_plan = self
                        .state
                        .speculation
                        .static_semantics
                        .local_object_binding(name)
                        .map(|binding| {
                            self.object_binding_property_removal_plan(
                                binding,
                                &materialized_property,
                            )
                        });
                    let global_removal_plan = self
                        .backend
                        .global_semantics
                        .values
                        .object_bindings
                        .get(name)
                        .map(|binding| {
                            self.object_binding_property_removal_plan(
                                binding,
                                &materialized_property,
                            )
                        });
                    if static_property_name_from_expression(&materialized_property).is_none()
                        && self.emit_dynamic_string_named_object_member_delete(name, property)?
                    {
                        return Ok(());
                    }
                    if static_property_name_from_expression(&materialized_property).is_none()
                        && self.emit_dynamic_symbol_named_object_member_delete(name, property)?
                    {
                        return Ok(());
                    }
                    self.mark_runtime_object_property_shadow_deleted_binding(
                        object,
                        &materialized_property,
                    );
                    if let Some(object_binding) = self
                        .state
                        .speculation
                        .static_semantics
                        .local_object_binding_mut(name)
                    {
                        let (string_property, symbol_keys) =
                            local_removal_plan.unwrap_or((None, Vec::new()));
                        if let Some(property_name) = string_property {
                            object_binding
                                .string_properties
                                .retain(|(existing_name, _)| *existing_name != property_name);
                            object_binding
                                .non_enumerable_string_properties
                                .retain(|hidden_name| hidden_name != &property_name);
                        } else {
                            object_binding
                                .symbol_properties
                                .retain(|(existing_key, _)| {
                                    !symbol_keys.iter().any(|key| key == existing_key)
                                });
                        }
                        self.push_i32_const(1);
                        return Ok(());
                    }
                    if let Some(object_binding) = self
                        .backend
                        .global_semantics
                        .values
                        .object_bindings
                        .get_mut(name)
                    {
                        let (string_property, symbol_keys) =
                            global_removal_plan.unwrap_or((None, Vec::new()));
                        if let Some(property_name) = string_property {
                            object_binding
                                .string_properties
                                .retain(|(existing_name, _)| *existing_name != property_name);
                            object_binding
                                .non_enumerable_string_properties
                                .retain(|hidden_name| hidden_name != &property_name);
                        } else {
                            object_binding
                                .symbol_properties
                                .retain(|(existing_key, _)| {
                                    !symbol_keys.iter().any(|key| key == existing_key)
                                });
                        }
                        self.push_i32_const(1);
                        return Ok(());
                    }
                }
                self.emit_numeric_expression(expression)?;
                self.state.emission.output.instructions.push(0x1a);
                self.push_i32_const(1);
            }
            Expression::SuperMember { .. }
            | Expression::AssignMember { .. }
            | Expression::AssignSuperMember { .. }
            | Expression::This => {
                self.emit_numeric_expression(expression)?;
                self.state.emission.output.instructions.push(0x1a);
                self.push_i32_const(1);
            }
            _ => {
                self.emit_numeric_expression(expression)?;
                self.state.emission.output.instructions.push(0x1a);
                self.push_i32_const(1);
            }
        }
        Ok(())
    }
}
