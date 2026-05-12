use super::*;

impl<'a> FunctionCompiler<'a> {
    pub(in crate::backend::direct_wasm) fn emit_runtime_known_symbol_property_presence_check(
        &mut self,
        receiver: &Expression,
        property: &Expression,
    ) -> DirectResult<bool> {
        let resolved_receiver = self
            .resolve_bound_alias_expression(receiver)
            .filter(|resolved| !static_expression_matches(resolved, receiver));
        let materialized_receiver = self.materialize_static_expression(receiver);
        let receiver_candidates = [
            Some(receiver),
            resolved_receiver.as_ref(),
            (!static_expression_matches(&materialized_receiver, receiver))
                .then_some(&materialized_receiver),
        ];
        let Some((owner_name, object_binding)) = receiver_candidates
            .into_iter()
            .flatten()
            .find_map(|candidate| {
                let binding = self
                    .resolve_object_binding_from_expression(candidate)
                    .or_else(|| match candidate {
                        Expression::Identifier(name) => {
                            self.resolve_identifier_object_binding_fallback(name)
                        }
                        _ => None,
                    })?;
                let owner_name = match candidate {
                    Expression::Identifier(name) => {
                        self.runtime_object_property_shadow_owner_name_for_identifier(name)
                    }
                    _ => None,
                };
                (!binding.symbol_properties.is_empty()).then_some((owner_name, binding))
            })
        else {
            return Ok(false);
        };

        if let Some((existing_key, _)) =
            self.resolve_static_symbol_property_shadow_entry(&object_binding, property)
        {
            if let Some(owner_name) = owner_name.as_ref() {
                let deleted_binding = self
                    .runtime_object_property_shadow_deleted_binding_by_property(
                        owner_name,
                        &existing_key,
                    );
                self.push_global_get(deleted_binding.present_index);
                self.state.emission.output.instructions.push(0x04);
                self.state.emission.output.instructions.push(I32_TYPE);
                self.push_control_frame();
                self.push_i32_const(0);
                self.state.emission.output.instructions.push(0x05);
                self.push_i32_const(1);
                self.state.emission.output.instructions.push(0x0b);
                self.pop_control_frame();
            } else {
                self.push_i32_const(1);
            }
            return Ok(true);
        }

        let property_local = self.allocate_temp_local();
        self.emit_numeric_expression(property)?;
        self.push_local_set(property_local);

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
            if let Some(owner_name) = owner_name.as_ref() {
                let deleted_binding = self
                    .runtime_object_property_shadow_deleted_binding_by_property(
                        owner_name,
                        &existing_key,
                    );
                self.push_global_get(deleted_binding.present_index);
                self.state.emission.output.instructions.push(0x04);
                self.state.emission.output.instructions.push(I32_TYPE);
                self.push_control_frame();
                self.push_i32_const(0);
                self.state.emission.output.instructions.push(0x05);
                self.push_i32_const(1);
                self.state.emission.output.instructions.push(0x0b);
                self.pop_control_frame();
            } else {
                self.push_i32_const(1);
            }
            self.state.emission.output.instructions.push(0x05);
        }

        self.push_i32_const(0);
        for _ in 0..open_frames {
            self.state.emission.output.instructions.push(0x0b);
            self.pop_control_frame();
        }
        Ok(true)
    }

    pub(in crate::backend::direct_wasm) fn resolve_bound_function_prototype_call_descriptor(
        &self,
        receiver: &Expression,
        property: &Expression,
    ) -> Option<PropertyDescriptorBinding> {
        let resolved_receiver = self
            .resolve_bound_alias_expression(receiver)
            .filter(|resolved| !static_expression_matches(resolved, receiver));
        let materialized_receiver = self.materialize_static_expression(receiver);
        let resolved_property = self.resolve_property_key_expression(property).or_else(|| {
            self.resolve_bound_alias_expression(property)
                .filter(|resolved| !static_expression_matches(resolved, property))
        });
        let materialized_property = self.materialize_static_expression(property);
        let receiver_candidates = [
            Some(receiver),
            resolved_receiver.as_ref(),
            (!static_expression_matches(&materialized_receiver, receiver))
                .then_some(&materialized_receiver),
        ];
        let property_candidates = [
            Some(property),
            resolved_property.as_ref(),
            (!static_expression_matches(&materialized_property, property))
                .then_some(&materialized_property),
        ];

        for receiver_candidate in receiver_candidates.into_iter().flatten() {
            for property_candidate in property_candidates.into_iter().flatten() {
                if let Some(descriptor) =
                    self.resolve_descriptor_binding_from_expression(&Expression::Call {
                        callee: Box::new(Expression::Member {
                            object: Box::new(Expression::Identifier("Object".to_string())),
                            property: Box::new(Expression::String(
                                "getOwnPropertyDescriptor".to_string(),
                            )),
                        }),
                        arguments: vec![
                            CallArgument::Expression(receiver_candidate.clone()),
                            CallArgument::Expression(property_candidate.clone()),
                        ],
                    })
                {
                    return Some(descriptor);
                }
            }
        }

        None
    }

    fn resolve_bound_array_join_value(
        &self,
        receiver: &Expression,
        arguments: &[CallArgument],
    ) -> Option<Expression> {
        let array_binding = self.resolve_array_binding_from_expression(receiver)?;
        let separator = match arguments.first() {
            Some(CallArgument::Expression(expression) | CallArgument::Spread(expression)) => {
                self.resolve_static_string_concat_value(expression, self.current_function_name())?
            }
            None => ",".to_string(),
        };
        let mut parts = Vec::with_capacity(array_binding.values.len());
        for value in &array_binding.values {
            let Some(value) = value else {
                parts.push(String::new());
                continue;
            };
            let materialized = self
                .resolve_static_primitive_expression_with_context(
                    value,
                    self.current_function_name(),
                )
                .unwrap_or_else(|| self.materialize_static_expression(value));
            let text = match materialized {
                Expression::Undefined | Expression::Null => String::new(),
                _ => self.resolve_static_string_concat_value(
                    &materialized,
                    self.current_function_name(),
                )?,
            };
            parts.push(text);
        }
        Some(Expression::String(parts.join(&separator)))
    }

    pub(in crate::backend::direct_wasm) fn emit_bound_function_prototype_call_builtin(
        &mut self,
        target_name: &str,
        arguments: &[CallArgument],
    ) -> DirectResult<bool> {
        let Some(CallArgument::Expression(receiver) | CallArgument::Spread(receiver)) =
            arguments.first()
        else {
            self.push_i32_const(JS_UNDEFINED_TAG);
            return Ok(true);
        };
        let target_arguments = &arguments[1..];

        match target_name {
            "Array.prototype.push" => {
                return self.emit_tracked_array_push_call(receiver, target_arguments);
            }
            "Array.prototype.join" => {
                for argument in arguments {
                    match argument {
                        CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                            self.emit_numeric_expression(expression)?;
                            self.state.emission.output.instructions.push(0x1a);
                        }
                    }
                }
                if let Some(value) = self.resolve_bound_array_join_value(receiver, target_arguments)
                {
                    self.emit_numeric_expression(&value)?;
                    return Ok(true);
                }
                return Ok(false);
            }
            "Object.prototype.hasOwnProperty" => {
                let Some(CallArgument::Expression(property) | CallArgument::Spread(property)) =
                    target_arguments.first()
                else {
                    self.push_i32_const(0);
                    return Ok(true);
                };
                for argument in arguments {
                    match argument {
                        CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                            self.emit_numeric_expression(expression)?;
                            self.state.emission.output.instructions.push(0x1a);
                        }
                    }
                }
                if self
                    .runtime_object_property_shadow_deletion_may_affect_property(receiver, property)
                {
                    self.emit_object_get_own_property_descriptor_result(receiver, property)?;
                    self.push_i32_const(JS_UNDEFINED_TAG);
                    self.push_binary_op(BinaryOp::NotEqual)?;
                    return Ok(true);
                }
                if let Some(has_own) =
                    self.resolve_static_bound_has_own_property_result(receiver, property)
                {
                    self.push_i32_const(has_own as i32);
                } else if self.emit_runtime_known_object_has_property_check(receiver, property)? {
                } else {
                    self.emit_object_get_own_property_descriptor_result(receiver, property)?;
                    self.push_i32_const(JS_UNDEFINED_TAG);
                    self.push_binary_op(BinaryOp::NotEqual)?;
                    return Ok(true);
                }
                return Ok(true);
            }
            "Object.prototype.propertyIsEnumerable" => {
                let Some(CallArgument::Expression(property) | CallArgument::Spread(property)) =
                    target_arguments.first()
                else {
                    self.push_i32_const(0);
                    return Ok(true);
                };
                for argument in arguments {
                    match argument {
                        CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                            self.emit_numeric_expression(expression)?;
                            self.state.emission.output.instructions.push(0x1a);
                        }
                    }
                }
                if let Some(descriptor) =
                    self.resolve_bound_function_prototype_call_descriptor(receiver, property)
                {
                    self.push_i32_const(descriptor.enumerable as i32);
                } else if self
                    .emit_runtime_known_symbol_property_presence_check(receiver, property)?
                {
                    return Ok(true);
                } else {
                    self.push_i32_const(0);
                }
                return Ok(true);
            }
            _ => {}
        }

        Ok(false)
    }
}
