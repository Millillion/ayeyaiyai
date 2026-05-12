use super::*;

impl<'a> FunctionCompiler<'a> {
    pub(in crate::backend::direct_wasm) fn emit_has_own_property_call(
        &mut self,
        object: &Expression,
        arguments: &[CallArgument],
    ) -> DirectResult<bool> {
        let Expression::Member {
            object: _target_object,
            property: target_property,
        } = object
        else {
            return Ok(false);
        };
        if !matches!(target_property.as_ref(), Expression::String(name) if name == "hasOwnProperty")
        {
            return Ok(false);
        }
        let [
            CallArgument::Expression(receiver),
            CallArgument::Expression(argument_property),
            rest @ ..,
        ] = arguments
        else {
            return Ok(false);
        };

        let mut canonical_argument_property = self
            .resolve_property_key_expression(argument_property)
            .or_else(|| {
                self.resolve_static_string_value(argument_property)
                    .map(Expression::String)
            })
            .unwrap_or_else(|| self.materialize_static_expression(argument_property));
        if matches!(&canonical_argument_property, Expression::Identifier(property_name) if property_name == "name")
            && let Some(owner_name) = match receiver {
                Expression::Identifier(name) => {
                    self.runtime_object_property_shadow_owner_name_for_identifier(name)
                }
                Expression::This => {
                    self.runtime_object_property_shadow_owner_name_for_identifier("this")
                }
                _ => None,
            }
            && self
                .resolve_runtime_shadow_object_binding(&owner_name)
                .as_ref()
                .is_some_and(|binding| {
                    object_binding_has_property(binding, &Expression::String("name".to_string()))
                })
        {
            canonical_argument_property = Expression::String("name".to_string());
        }
        let function_synthetic_property = match &canonical_argument_property {
            Expression::String(property_name) if property_name == "name" => {
                self.resolve_function_name_value(receiver, &canonical_argument_property)
                    .is_some()
                    || self.infer_value_kind(receiver) == Some(StaticValueKind::Function)
            }
            Expression::String(property_name) if property_name == "length" => {
                self.resolve_user_function_length(receiver, &canonical_argument_property)
                    .is_some()
                    || self.infer_value_kind(receiver) == Some(StaticValueKind::Function)
            }
            _ => false,
        };
        if std::env::var_os("AYY_TRACE_HAS_OWN").is_some() {
            eprintln!(
                "has_own:receiver={receiver:?} property={argument_property:?} function_synthetic={function_synthetic_property} kind={:?}",
                self.infer_value_kind(receiver)
            );
        }

        let result = if function_synthetic_property {
            Some(true)
        } else if let Some(array_binding) = self.resolve_array_binding_from_expression(receiver) {
            Some(
                matches!(argument_property, Expression::String(property_name) if property_name == "length")
                    || argument_index_from_expression(argument_property).is_some_and(|index| {
                        array_binding
                            .values
                            .get(index as usize)
                            .is_some_and(|value| value.is_some())
                    }),
            )
        } else if self.is_direct_arguments_object(receiver) {
            match argument_property {
                Expression::String(property_name) => match property_name.as_str() {
                    "callee" | "length" => Some(self.direct_arguments_has_property(property_name)),
                    _ => canonical_array_index_from_property_name(property_name)
                        .map(|index| self.state.parameters.arguments_slots.contains_key(&index)),
                },
                _ => None,
            }
        } else if let Some(arguments_binding) =
            self.resolve_arguments_binding_from_expression(receiver)
        {
            match argument_property {
                Expression::String(property_name) => Some(match property_name.as_str() {
                    "callee" => arguments_binding.callee_present,
                    "length" => arguments_binding.length_present,
                    _ => property_name
                        .parse::<usize>()
                        .ok()
                        .is_some_and(|index| index < arguments_binding.values.len()),
                }),
                _ => None,
            }
        } else if self
            .resolve_function_binding_from_expression(receiver)
            .is_some()
        {
            self.resolve_function_object_has_own_property(receiver, argument_property)
        } else if let Some(has_property) =
            self.resolve_static_object_has_own_property_result(receiver, argument_property)
        {
            has_property
        } else if self
            .resolve_bound_function_prototype_call_descriptor(receiver, argument_property)
            .is_some()
        {
            Some(true)
        } else if self
            .resolve_object_binding_from_expression(receiver)
            .is_some()
        {
            None
        } else {
            self.resolve_static_object_has_own_property_result(receiver, argument_property)
                .flatten()
        };
        if std::env::var_os("AYY_TRACE_HAS_OWN").is_some() {
            eprintln!("has_own:result={result:?}");
        }
        if result.is_none()
            && matches!(
                argument_property,
                Expression::String(property_name)
                    if property_name == "name" || property_name == "length"
            )
        {
            self.emit_numeric_expression(object)?;
            self.state.emission.output.instructions.push(0x1a);
            let receiver_local = self.allocate_temp_local();
            self.emit_numeric_expression(receiver)?;
            self.push_local_set(receiver_local);
            self.emit_numeric_expression(argument_property)?;
            self.state.emission.output.instructions.push(0x1a);
            for argument in rest {
                match argument {
                    CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                        self.emit_numeric_expression(expression)?;
                        self.state.emission.output.instructions.push(0x1a);
                    }
                }
            }
            if self.runtime_object_property_shadow_deletion_may_affect_property(
                receiver,
                &canonical_argument_property,
            ) {
                self.emit_object_get_own_property_descriptor_result(
                    receiver,
                    &canonical_argument_property,
                )?;
                self.push_i32_const(JS_UNDEFINED_TAG);
                self.push_binary_op(BinaryOp::NotEqual)?;
                return Ok(true);
            }
            let result_local = self.allocate_temp_local();
            self.push_i32_const(0);
            self.push_local_set(result_local);
            self.emit_runtime_typeof_exact_match(
                receiver_local,
                result_local,
                JS_TYPEOF_FUNCTION_TAG,
                1,
            )?;
            self.emit_runtime_typeof_range_match(
                receiver_local,
                result_local,
                JS_BUILTIN_FUNCTION_VALUE_BASE,
                JS_BUILTIN_FUNCTION_VALUE_BASE + JS_BUILTIN_FUNCTION_VALUE_LIMIT,
                1,
            )?;
            self.emit_runtime_typeof_range_match(
                receiver_local,
                result_local,
                JS_USER_FUNCTION_VALUE_BASE,
                JS_USER_FUNCTION_VALUE_BASE + JS_USER_FUNCTION_VALUE_LIMIT,
                1,
            )?;
            self.push_local_get(result_local);
            return Ok(true);
        }
        let Some(has_property) = result else {
            self.emit_numeric_expression(object)?;
            self.state.emission.output.instructions.push(0x1a);
            let receiver_local = self.allocate_temp_local();
            self.emit_numeric_expression(receiver)?;
            self.push_local_set(receiver_local);
            let property_local = self.allocate_temp_local();
            self.emit_numeric_expression(argument_property)?;
            self.push_local_set(property_local);
            for argument in rest {
                match argument {
                    CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                        self.emit_numeric_expression(expression)?;
                        self.state.emission.output.instructions.push(0x1a);
                    }
                }
            }
            if self.runtime_object_property_shadow_deletion_may_affect_property(
                receiver,
                &canonical_argument_property,
            ) {
                self.emit_object_get_own_property_descriptor_result(
                    receiver,
                    &canonical_argument_property,
                )?;
                self.push_i32_const(JS_UNDEFINED_TAG);
                self.push_binary_op(BinaryOp::NotEqual)?;
                return Ok(true);
            }
            let function_match_local = self.allocate_temp_local();
            self.push_i32_const(0);
            self.push_local_set(function_match_local);
            self.emit_runtime_typeof_exact_match(
                receiver_local,
                function_match_local,
                JS_TYPEOF_FUNCTION_TAG,
                1,
            )?;
            self.emit_runtime_typeof_range_match(
                receiver_local,
                function_match_local,
                JS_BUILTIN_FUNCTION_VALUE_BASE,
                JS_BUILTIN_FUNCTION_VALUE_BASE + JS_BUILTIN_FUNCTION_VALUE_LIMIT,
                1,
            )?;
            self.emit_runtime_typeof_range_match(
                receiver_local,
                function_match_local,
                JS_USER_FUNCTION_VALUE_BASE,
                JS_USER_FUNCTION_VALUE_BASE + JS_USER_FUNCTION_VALUE_LIMIT,
                1,
            )?;
            let property_match_local = self.allocate_temp_local();
            self.push_i32_const(0);
            self.push_local_set(property_match_local);
            self.push_local_get(property_local);
            self.emit_static_string_literal("name")?;
            self.push_binary_op(BinaryOp::Equal)?;
            self.state.emission.output.instructions.push(0x04);
            self.state
                .emission
                .output
                .instructions
                .push(EMPTY_BLOCK_TYPE);
            self.push_control_frame();
            self.push_i32_const(1);
            self.push_local_set(property_match_local);
            self.state.emission.output.instructions.push(0x0b);
            self.pop_control_frame();
            self.push_local_get(property_local);
            self.emit_static_string_literal("length")?;
            self.push_binary_op(BinaryOp::Equal)?;
            self.state.emission.output.instructions.push(0x04);
            self.state
                .emission
                .output
                .instructions
                .push(EMPTY_BLOCK_TYPE);
            self.push_control_frame();
            self.push_i32_const(1);
            self.push_local_set(property_match_local);
            self.state.emission.output.instructions.push(0x0b);
            self.pop_control_frame();
            self.push_local_get(function_match_local);
            self.push_local_get(property_match_local);
            self.state.emission.output.instructions.push(0x71);
            self.state.emission.output.instructions.push(0x04);
            self.state.emission.output.instructions.push(I32_TYPE);
            self.push_control_frame();
            self.push_i32_const(1);
            self.state.emission.output.instructions.push(0x05);
            if !self.emit_runtime_known_object_has_property_check(receiver, argument_property)? {
                self.emit_object_get_own_property_descriptor_result(receiver, argument_property)?;
                self.push_i32_const(JS_UNDEFINED_TAG);
                self.push_binary_op(BinaryOp::NotEqual)?;
            }
            self.state.emission.output.instructions.push(0x0b);
            self.pop_control_frame();
            return Ok(true);
        };

        self.emit_numeric_expression(object)?;
        self.state.emission.output.instructions.push(0x1a);
        self.emit_numeric_expression(receiver)?;
        self.state.emission.output.instructions.push(0x1a);
        self.emit_numeric_expression(argument_property)?;
        self.state.emission.output.instructions.push(0x1a);
        for argument in rest {
            match argument {
                CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                    self.emit_numeric_expression(expression)?;
                    self.state.emission.output.instructions.push(0x1a);
                }
            }
        }
        if self.runtime_object_property_shadow_deletion_may_affect_property(
            receiver,
            &canonical_argument_property,
        ) {
            self.emit_object_get_own_property_descriptor_result(
                receiver,
                &canonical_argument_property,
            )?;
            self.push_i32_const(JS_UNDEFINED_TAG);
            self.push_binary_op(BinaryOp::NotEqual)?;
            return Ok(true);
        }
        if !has_property
            && self.emit_runtime_known_object_has_property_check(receiver, argument_property)?
        {
            return Ok(true);
        }
        self.push_i32_const(if has_property { 1 } else { 0 });
        Ok(true)
    }

    pub(in crate::backend::direct_wasm) fn emit_property_is_enumerable_call(
        &mut self,
        object: &Expression,
        arguments: &[CallArgument],
    ) -> DirectResult<bool> {
        let Expression::Member {
            object: _target_object,
            property: target_property,
        } = object
        else {
            return Ok(false);
        };
        if !matches!(target_property.as_ref(), Expression::String(name) if name == "propertyIsEnumerable")
        {
            return Ok(false);
        }

        self.emit_bound_function_prototype_call_builtin(
            "Object.prototype.propertyIsEnumerable",
            arguments,
        )
    }
}
