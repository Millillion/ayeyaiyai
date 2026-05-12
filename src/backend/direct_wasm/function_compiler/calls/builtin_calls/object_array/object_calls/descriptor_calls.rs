use super::*;

impl<'a> FunctionCompiler<'a> {
    fn emit_implicit_global_property_descriptor_result(
        &mut self,
        property_name: &str,
        binding: ImplicitGlobalBinding,
    ) -> DirectResult<()> {
        let descriptor = PropertyDescriptorBinding {
            value: Some(Expression::Identifier(property_name.to_string())),
            configurable: true,
            enumerable: true,
            writable: Some(true),
            getter: None,
            setter: None,
            has_get: false,
            has_set: false,
        };
        let descriptor_expression = object_binding_to_expression(
            &self.object_binding_from_property_descriptor(&descriptor),
        );

        self.push_global_get(binding.present_index);
        self.state.emission.output.instructions.push(0x04);
        self.state.emission.output.instructions.push(I32_TYPE);
        self.push_control_frame();
        self.emit_numeric_expression(&descriptor_expression)?;
        self.state.emission.output.instructions.push(0x05);
        self.push_i32_const(JS_UNDEFINED_TAG);
        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();
        Ok(())
    }

    fn emit_descriptor_or_deleted_undefined(
        &mut self,
        receiver: &Expression,
        property: &Expression,
        descriptor: &PropertyDescriptorBinding,
    ) -> DirectResult<()> {
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
        let deleted_binding = receiver_candidates
            .into_iter()
            .flatten()
            .find_map(|candidate| {
                self.resolve_runtime_object_property_shadow_deleted_binding(candidate, property)
            });
        let descriptor_expression =
            object_binding_to_expression(&self.object_binding_from_property_descriptor(descriptor));

        if let Some(deleted_binding) = deleted_binding {
            self.push_global_get(deleted_binding.present_index);
            self.state.emission.output.instructions.push(0x04);
            self.state.emission.output.instructions.push(I32_TYPE);
            self.push_control_frame();
            self.push_i32_const(JS_UNDEFINED_TAG);
            self.state.emission.output.instructions.push(0x05);
            self.emit_numeric_expression(&descriptor_expression)?;
            self.state.emission.output.instructions.push(0x0b);
            self.pop_control_frame();
        } else {
            self.emit_numeric_expression(&descriptor_expression)?;
        }

        Ok(())
    }

    fn emit_runtime_known_object_dynamic_has_property_check(
        &mut self,
        receiver: &Expression,
        property: &Expression,
    ) -> DirectResult<bool> {
        let canonical_property = self.canonical_object_property_expression(property);
        if static_property_name_from_expression(&canonical_property).is_some() {
            return Ok(false);
        }

        let object_binding = self
            .resolve_object_binding_from_expression(receiver)
            .or_else(|| match receiver {
                Expression::Identifier(name) => self
                    .resolve_identifier_object_binding_fallback(name)
                    .or_else(|| self.resolve_runtime_shadow_object_binding(name)),
                Expression::This => self.resolve_runtime_shadow_object_binding("this"),
                _ => None,
            });
        let Some(object_binding) = object_binding else {
            return Ok(false);
        };

        let property_names = ordered_object_property_names(&object_binding);
        let symbol_properties = object_binding
            .symbol_properties
            .iter()
            .map(|(property, _)| property.clone())
            .collect::<Vec<_>>();
        if property_names.is_empty() && symbol_properties.is_empty() {
            return Ok(false);
        }

        let property_local = self.allocate_temp_local();
        self.emit_numeric_expression(property)?;
        self.push_local_set(property_local);

        let mut open_frames = 0;
        for property_name in property_names {
            self.push_local_get(property_local);
            self.emit_static_string_literal(&property_name)?;
            self.push_binary_op(BinaryOp::Equal)?;
            self.state.emission.output.instructions.push(0x04);
            self.state.emission.output.instructions.push(I32_TYPE);
            self.push_control_frame();
            open_frames += 1;
            if !self.emit_runtime_known_object_has_property_check(
                receiver,
                &Expression::String(property_name),
            )? {
                self.push_i32_const(1);
            }
            self.state.emission.output.instructions.push(0x05);
        }
        for symbol_property in symbol_properties {
            self.push_local_get(property_local);
            let comparison_key = self.canonical_object_property_expression(&symbol_property);
            self.emit_numeric_expression(&comparison_key)?;
            self.push_binary_op(BinaryOp::Equal)?;
            self.state.emission.output.instructions.push(0x04);
            self.state.emission.output.instructions.push(I32_TYPE);
            self.push_control_frame();
            open_frames += 1;
            if !self
                .emit_runtime_known_symbol_property_presence_check(receiver, &symbol_property)?
            {
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

    pub(in crate::backend::direct_wasm) fn emit_class_prototype_init_call(
        &mut self,
        arguments: &[CallArgument],
    ) -> DirectResult<bool> {
        let [
            CallArgument::Expression(target),
            CallArgument::Expression(prototype_parent),
            rest @ ..,
        ] = arguments
        else {
            return Ok(false);
        };

        self.discard_call_arguments(rest)?;

        let prototype_object = Expression::Call {
            callee: Box::new(Expression::Member {
                object: Box::new(Expression::Identifier("Object".to_string())),
                property: Box::new(Expression::String("create".to_string())),
            }),
            arguments: vec![CallArgument::Expression(prototype_parent.clone())],
        };
        if let Expression::Identifier(name) = target {
            self.update_prototype_object_binding(name, &prototype_object);
        }

        self.emit_numeric_expression(&prototype_object)?;
        self.state.emission.output.instructions.push(0x1a);
        self.emit_numeric_expression(target)?;
        Ok(true)
    }

    pub(in crate::backend::direct_wasm) fn emit_runtime_known_object_has_property_check(
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
        if self.emit_runtime_known_object_dynamic_has_property_check(receiver, property)? {
            return Ok(true);
        }
        let Some((binding, deleted_binding, fallback_present)) = receiver_candidates
            .into_iter()
            .flatten()
            .find_map(|candidate| {
                let binding =
                    self.resolve_runtime_object_property_shadow_binding(candidate, property);
                let deleted_binding = self
                    .resolve_runtime_object_property_shadow_deleted_binding(candidate, property);
                let fallback_present = self
                    .resolve_object_binding_from_expression(candidate)
                    .or_else(|| match candidate {
                        Expression::Identifier(name) => self
                            .resolve_identifier_object_binding_fallback(name)
                            .or_else(|| self.resolve_runtime_shadow_object_binding(name)),
                        Expression::This => self.resolve_runtime_shadow_object_binding("this"),
                        _ => None,
                    })
                    .and_then(|object_binding| {
                        self.resolve_object_binding_property_value(&object_binding, property)
                    })
                    .is_some();
                (binding.is_some() || fallback_present).then_some((
                    binding,
                    deleted_binding,
                    fallback_present,
                ))
            })
        else {
            return Ok(false);
        };

        let fallback_value = i32::from(fallback_present);
        if let Some(deleted_binding) = deleted_binding {
            self.push_global_get(deleted_binding.present_index);
            self.state.emission.output.instructions.push(0x04);
            self.state.emission.output.instructions.push(I32_TYPE);
            self.push_control_frame();
            self.push_i32_const(0);
            self.state.emission.output.instructions.push(0x05);
            if let Some(binding) = binding {
                self.push_global_get(binding.present_index);
                self.state.emission.output.instructions.push(0x04);
                self.state.emission.output.instructions.push(I32_TYPE);
                self.push_control_frame();
                self.push_i32_const(1);
                self.state.emission.output.instructions.push(0x05);
                self.push_i32_const(fallback_value);
                self.state.emission.output.instructions.push(0x0b);
                self.pop_control_frame();
            } else {
                self.push_i32_const(fallback_value);
            }
            self.state.emission.output.instructions.push(0x0b);
            self.pop_control_frame();
            return Ok(true);
        }

        if let Some(binding) = binding {
            self.push_global_get(binding.present_index);
            self.state.emission.output.instructions.push(0x04);
            self.state.emission.output.instructions.push(I32_TYPE);
            self.push_control_frame();
            self.push_i32_const(1);
            self.state.emission.output.instructions.push(0x05);
            self.push_i32_const(fallback_value);
            self.state.emission.output.instructions.push(0x0b);
            self.pop_control_frame();
            return Ok(true);
        }

        self.push_i32_const(fallback_value);
        Ok(true)
    }

    pub(in crate::backend::direct_wasm) fn emit_object_get_own_property_descriptor_result(
        &mut self,
        receiver: &Expression,
        property: &Expression,
    ) -> DirectResult<()> {
        let materialized_property = self
            .resolve_property_key_expression(property)
            .unwrap_or_else(|| self.materialize_static_expression(property));
        if self.state.speculation.execution_context.top_level_function
            && matches!(receiver, Expression::This)
            && let Some(property_name) =
                static_property_name_from_expression(&materialized_property)
            && let Some(binding) = self.implicit_global_binding(&property_name)
        {
            self.emit_implicit_global_property_descriptor_result(&property_name, binding)?;
            return Ok(());
        }

        let synthesized_call = Expression::Call {
            callee: Box::new(Expression::Member {
                object: Box::new(Expression::Identifier("Object".to_string())),
                property: Box::new(Expression::String("getOwnPropertyDescriptor".to_string())),
            }),
            arguments: vec![
                CallArgument::Expression(receiver.clone()),
                CallArgument::Expression(property.clone()),
            ],
        };

        if let Some(descriptor) = self.resolve_descriptor_binding_from_expression(&synthesized_call)
        {
            self.emit_descriptor_or_deleted_undefined(receiver, property, &descriptor)?;
        } else if self.emit_runtime_known_symbol_property_descriptor_call(receiver, property)? {
        } else {
            self.push_i32_const(JS_UNDEFINED_TAG);
        }

        Ok(())
    }

    pub(in crate::backend::direct_wasm) fn emit_runtime_known_symbol_property_descriptor_call(
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
        let Some((owner_name, existing_key, fallback_value)) = receiver_candidates
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
                let (existing_key, fallback_value) =
                    self.resolve_static_symbol_property_shadow_entry(&binding, property)?;
                Some((owner_name, existing_key, fallback_value))
            })
        else {
            return Ok(false);
        };

        let descriptor = PropertyDescriptorBinding {
            value: Some(fallback_value),
            configurable: true,
            enumerable: true,
            writable: Some(true),
            getter: None,
            setter: None,
            has_get: false,
            has_set: false,
        };
        let descriptor_expression = object_binding_to_expression(
            &self.object_binding_from_property_descriptor(&descriptor),
        );

        if let Some(owner_name) = owner_name.as_ref() {
            let deleted_binding = self.runtime_object_property_shadow_deleted_binding_by_property(
                owner_name,
                &existing_key,
            );
            self.push_global_get(deleted_binding.present_index);
            self.state.emission.output.instructions.push(0x04);
            self.state.emission.output.instructions.push(I32_TYPE);
            self.push_control_frame();
            self.push_i32_const(JS_UNDEFINED_TAG);
            self.state.emission.output.instructions.push(0x05);
            self.emit_numeric_expression(&descriptor_expression)?;
            self.state.emission.output.instructions.push(0x0b);
            self.pop_control_frame();
        } else {
            self.emit_numeric_expression(&descriptor_expression)?;
        }
        Ok(true)
    }

    fn reflect_define_property_data_value(
        &self,
        descriptor_expression: &Expression,
    ) -> Option<Expression> {
        if let Some(descriptor) =
            self.resolve_descriptor_binding_from_expression(descriptor_expression)
            && !descriptor.has_get
            && !descriptor.has_set
        {
            return Some(descriptor.value.unwrap_or(Expression::Undefined));
        }

        let descriptor = resolve_property_descriptor_definition(descriptor_expression)?;
        if descriptor.is_accessor() {
            return None;
        }

        Some(descriptor.value.unwrap_or(Expression::Undefined))
    }

    fn define_property_can_update_without_assignment(
        &self,
        target: &Expression,
        property: &Expression,
    ) -> bool {
        let materialized_property = self.canonical_object_property_expression(property);
        let Some(property_name) = static_property_name_from_expression(&materialized_property)
        else {
            return false;
        };
        let resolved_target = self
            .resolve_bound_alias_expression(target)
            .filter(|resolved| !static_expression_matches(resolved, target));
        let materialized_target = self.materialize_static_expression(target);
        let descriptor = self
            .resolve_function_property_descriptor_binding(
                target,
                resolved_target.as_ref(),
                &materialized_target,
                &property_name,
            )
            .or_else(|| {
                self.resolve_object_property_descriptor_binding(
                    target,
                    resolved_target.as_ref(),
                    &materialized_target,
                    &materialized_property,
                    Some(&property_name),
                )
            });

        descriptor
            .is_some_and(|descriptor| descriptor.configurable && descriptor.writable == Some(false))
    }

    fn define_property_data_value_can_emit_without_assignment(
        &self,
        target: &Expression,
        property: &Expression,
        value: &Expression,
    ) -> bool {
        if self.define_property_can_update_without_assignment(target, property) {
            return true;
        }

        let materialized_property = self.canonical_object_property_expression(property);
        self.member_function_binding_key(target, &materialized_property)
            .is_some()
            && self
                .resolve_function_binding_from_expression(value)
                .is_some()
    }

    fn emit_define_property_effects_without_assignment(
        &mut self,
        target: &Expression,
        property: &Expression,
        value: &Expression,
    ) -> DirectResult<()> {
        self.emit_numeric_expression(target)?;
        self.state.emission.output.instructions.push(0x1a);
        self.emit_property_key_expression_effects(property)?;
        self.emit_numeric_expression(value)?;
        self.state.emission.output.instructions.push(0x1a);
        Ok(())
    }

    pub(in crate::backend::direct_wasm) fn emit_object_get_own_property_descriptor_call(
        &mut self,
        callee_object: &Expression,
        callee_property: &Expression,
        arguments: &[CallArgument],
    ) -> DirectResult<bool> {
        if !matches!(callee_object, Expression::Identifier(name) if name == "Object") {
            return Ok(false);
        }
        if !matches!(callee_property, Expression::String(name) if name == "getOwnPropertyDescriptor")
        {
            return Ok(false);
        }
        for argument in arguments {
            match argument {
                CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                    self.emit_numeric_expression(expression)?;
                    self.state.emission.output.instructions.push(0x1a);
                }
            }
        }
        if let [
            CallArgument::Expression(receiver) | CallArgument::Spread(receiver),
            CallArgument::Expression(property) | CallArgument::Spread(property),
            ..,
        ] = arguments
        {
            self.emit_object_get_own_property_descriptor_result(receiver, property)?;
        } else {
            self.push_i32_const(JS_UNDEFINED_TAG);
        }
        Ok(true)
    }

    pub(in crate::backend::direct_wasm) fn emit_reflect_define_property_call(
        &mut self,
        callee_object: &Expression,
        callee_property: &Expression,
        arguments: &[CallArgument],
    ) -> DirectResult<bool> {
        if !matches!(callee_object, Expression::Identifier(name) if name == "Reflect") {
            return Ok(false);
        }
        if !matches!(callee_property, Expression::String(name) if name == "defineProperty") {
            return Ok(false);
        }

        let [
            CallArgument::Expression(target),
            CallArgument::Expression(property),
            CallArgument::Expression(descriptor_expression),
            rest @ ..,
        ] = arguments
        else {
            self.discard_call_arguments(arguments)?;
            self.push_i32_const(0);
            return Ok(true);
        };

        self.discard_call_arguments(rest)?;

        if let Some(value) = self.reflect_define_property_data_value(descriptor_expression) {
            if self.define_property_data_value_can_emit_without_assignment(target, property, &value)
            {
                self.emit_define_property_effects_without_assignment(target, property, &value)?;
            } else {
                self.emit_numeric_expression(&Expression::AssignMember {
                    object: Box::new(target.clone()),
                    property: Box::new(property.clone()),
                    value: Box::new(value),
                })?;
                self.state.emission.output.instructions.push(0x1a);
            }
            self.push_i32_const(1);
            return Ok(true);
        }

        self.emit_numeric_expression(target)?;
        self.state.emission.output.instructions.push(0x1a);
        self.emit_property_key_expression_effects(property)?;
        self.emit_numeric_expression(descriptor_expression)?;
        self.state.emission.output.instructions.push(0x1a);
        self.push_i32_const(1);
        Ok(true)
    }

    pub(in crate::backend::direct_wasm) fn emit_object_define_property_call(
        &mut self,
        callee_object: &Expression,
        callee_property: &Expression,
        arguments: &[CallArgument],
    ) -> DirectResult<bool> {
        if !matches!(callee_object, Expression::Identifier(name) if name == "Object") {
            return Ok(false);
        }
        if !matches!(callee_property, Expression::String(name) if name == "defineProperty") {
            return Ok(false);
        }

        let [
            CallArgument::Expression(target),
            CallArgument::Expression(property),
            CallArgument::Expression(descriptor_expression),
            rest @ ..,
        ] = arguments
        else {
            self.discard_call_arguments(arguments)?;
            self.push_i32_const(JS_TYPEOF_OBJECT_TAG);
            return Ok(true);
        };

        self.discard_call_arguments(rest)?;

        if let Some(value) = self.reflect_define_property_data_value(descriptor_expression) {
            let this_binding = self
                .state
                .speculation
                .execution_context
                .direct_eval_in_class_field_initializer
                .then(|| match target {
                    Expression::Identifier(name) => Expression::Identifier(name.clone()),
                    Expression::This => Expression::This,
                    _ => target.clone(),
                });
            let value = if inline_summary_side_effect_free_expression(&value) {
                self.materialize_define_property_value_expression_with_this_binding(
                    &value,
                    this_binding.as_ref(),
                )
            } else {
                value
            };
            let materialized_property = self.canonical_object_property_expression(property);
            let private_initializer_definition =
                is_private_property_name_expression(&materialized_property)
                    && match target {
                        Expression::Identifier(name) => self.emit_private_field_initializer_add(
                            name,
                            target,
                            &materialized_property,
                            &value,
                        )?,
                        Expression::This => self.emit_private_field_initializer_add(
                            "this",
                            target,
                            &materialized_property,
                            &value,
                        )?,
                        _ => false,
                    };
            if private_initializer_definition {
                self.state.emission.output.instructions.push(0x1a);
            } else if self
                .define_property_data_value_can_emit_without_assignment(target, property, &value)
            {
                self.emit_define_property_effects_without_assignment(target, property, &value)?;
            } else {
                self.emit_numeric_expression(&Expression::AssignMember {
                    object: Box::new(target.clone()),
                    property: Box::new(property.clone()),
                    value: Box::new(value),
                })?;
                self.state.emission.output.instructions.push(0x1a);
            }
        } else {
            self.emit_numeric_expression(target)?;
            self.state.emission.output.instructions.push(0x1a);
            self.emit_property_key_expression_effects(property)?;
            self.emit_numeric_expression(descriptor_expression)?;
            self.state.emission.output.instructions.push(0x1a);
        }

        self.emit_numeric_expression(target)?;
        Ok(true)
    }
}
