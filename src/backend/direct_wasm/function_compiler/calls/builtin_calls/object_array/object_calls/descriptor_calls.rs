use super::*;

fn object_binding_is_module_namespace(object_binding: &ObjectValueBinding) -> bool {
    object_binding
        .string_properties
        .iter()
        .any(|(name, value)| {
            name == "__ayy$module$namespace" && matches!(value, Expression::Bool(true))
        })
}

fn target_is_module_init_exports_parameter(
    current_function_name: Option<&str>,
    target: &Expression,
) -> bool {
    current_function_name.is_some_and(|name| name.starts_with("__ayy_module_init_"))
        && matches!(target, Expression::Identifier(name) if name == "exports")
}

fn descriptor_definition_is_empty(descriptor: &PropertyDescriptorDefinition) -> bool {
    descriptor.value.is_none()
        && descriptor.writable.is_none()
        && descriptor.enumerable.is_none()
        && descriptor.configurable.is_none()
        && descriptor.getter.is_none()
        && descriptor.setter.is_none()
}

impl<'a> FunctionCompiler<'a> {
    fn static_define_property_target_binding(
        &self,
        target: &Expression,
    ) -> Option<ObjectValueBinding> {
        self.resolve_object_binding_from_expression(target)
            .or_else(|| match target {
                Expression::Identifier(name) => self
                    .resolve_identifier_object_binding_fallback(name)
                    .or_else(|| self.resolve_runtime_shadow_object_binding(name)),
                Expression::This => self.resolve_runtime_shadow_object_binding("this"),
                _ => None,
            })
    }

    fn static_property_current_descriptor(
        &self,
        object_binding: &ObjectValueBinding,
        property: &Expression,
    ) -> Option<PropertyDescriptorBinding> {
        if let Some(descriptor) = object_binding_lookup_descriptor(object_binding, property) {
            return Some(descriptor.clone());
        }

        let value = object_binding_lookup_value(object_binding, property)?.clone();
        let enumerable =
            static_property_name_from_expression(property).is_none_or(|property_name| {
                !object_binding
                    .non_enumerable_string_properties
                    .iter()
                    .any(|hidden_name| hidden_name == &property_name)
            });
        Some(PropertyDescriptorBinding {
            value: Some(value),
            configurable: true,
            enumerable,
            writable: Some(true),
            getter: None,
            setter: None,
            has_get: false,
            has_set: false,
        })
    }

    fn static_define_property_values_match(
        &self,
        left: &Expression,
        right: &Expression,
    ) -> Option<bool> {
        self.resolve_static_same_value_result_with_context(
            left,
            right,
            self.current_function_name(),
        )
        .or_else(|| {
            let left_materialized = self.materialize_static_expression(left);
            let right_materialized = self.materialize_static_expression(right);
            self.resolve_static_same_value_result_with_context(
                &left_materialized,
                &right_materialized,
                self.current_function_name(),
            )
        })
        .or_else(|| {
            (static_expression_matches(left, right)
                || static_expression_matches(
                    &self.materialize_static_expression(left),
                    &self.materialize_static_expression(right),
                ))
            .then_some(true)
        })
    }

    fn module_namespace_define_property_allowed(
        &self,
        object_binding: &ObjectValueBinding,
        property: &Expression,
        descriptor: &PropertyDescriptorDefinition,
    ) -> Option<bool> {
        let Some(_) = static_property_name_from_expression(property) else {
            return None;
        };
        let current = self.static_property_current_descriptor(object_binding, property);
        let Some(current) = current else {
            return Some(false);
        };
        if descriptor.configurable == Some(true)
            || descriptor.enumerable == Some(false)
            || descriptor.is_accessor()
            || descriptor.writable == Some(false)
        {
            return Some(false);
        }
        if let Some(value) = descriptor.value.as_ref() {
            let current_value = current.value.as_ref().unwrap_or(&Expression::Undefined);
            if !self.static_define_property_values_match(value, current_value)? {
                return Some(false);
            }
        }
        Some(true)
    }

    fn ordinary_define_property_allowed(
        &self,
        object_binding: &ObjectValueBinding,
        property: &Expression,
        descriptor: &PropertyDescriptorDefinition,
    ) -> Option<bool> {
        let Some(current) = self.static_property_current_descriptor(object_binding, property)
        else {
            return Some(object_binding.extensible);
        };
        if descriptor_definition_is_empty(descriptor) {
            return Some(true);
        }
        if current.configurable {
            return Some(true);
        }
        if descriptor.configurable == Some(true) {
            return Some(false);
        }
        if descriptor
            .enumerable
            .is_some_and(|enumerable| enumerable != current.enumerable)
        {
            return Some(false);
        }

        let current_is_accessor = current.has_get || current.has_set || current.writable.is_none();
        if descriptor.is_accessor() {
            if !current_is_accessor {
                return Some(false);
            }
            if let Some(getter) = descriptor.getter.as_ref()
                && current.getter.as_ref().is_none_or(|current_getter| {
                    !self
                        .static_define_property_values_match(getter, current_getter)
                        .unwrap_or(false)
                })
            {
                return Some(false);
            }
            if let Some(setter) = descriptor.setter.as_ref()
                && current.setter.as_ref().is_none_or(|current_setter| {
                    !self
                        .static_define_property_values_match(setter, current_setter)
                        .unwrap_or(false)
                })
            {
                return Some(false);
            }
            return Some(true);
        }

        if current_is_accessor {
            if descriptor.value.is_some() || descriptor.writable.is_some() {
                return Some(false);
            }
            return Some(true);
        }

        if current.writable == Some(false) {
            if descriptor.writable == Some(true) {
                return Some(false);
            }
            if let Some(value) = descriptor.value.as_ref() {
                let current_value = current.value.as_ref().unwrap_or(&Expression::Undefined);
                if !self.static_define_property_values_match(value, current_value)? {
                    return Some(false);
                }
            }
        }
        Some(true)
    }

    fn define_property_descriptor_matches_without_change(
        &self,
        current: &PropertyDescriptorBinding,
        descriptor: &PropertyDescriptorDefinition,
    ) -> Option<bool> {
        if descriptor
            .configurable
            .is_some_and(|configurable| configurable != current.configurable)
            || descriptor
                .enumerable
                .is_some_and(|enumerable| enumerable != current.enumerable)
        {
            return Some(false);
        }
        let current_is_accessor = current.has_get || current.has_set || current.writable.is_none();
        if descriptor.is_accessor() {
            if !current_is_accessor {
                return Some(false);
            }
            if let Some(getter) = descriptor.getter.as_ref() {
                let current_getter = current.getter.as_ref()?;
                if !self.static_define_property_values_match(getter, current_getter)? {
                    return Some(false);
                }
            }
            if let Some(setter) = descriptor.setter.as_ref() {
                let current_setter = current.setter.as_ref()?;
                if !self.static_define_property_values_match(setter, current_setter)? {
                    return Some(false);
                }
            }
            return Some(true);
        }
        if current_is_accessor {
            return Some(descriptor.value.is_none() && descriptor.writable.is_none());
        }
        if descriptor
            .writable
            .is_some_and(|writable| Some(writable) != current.writable)
        {
            return Some(false);
        }
        if let Some(value) = descriptor.value.as_ref() {
            let current_value = current.value.as_ref().unwrap_or(&Expression::Undefined);
            if !self.static_define_property_values_match(value, current_value)? {
                return Some(false);
            }
        }
        Some(true)
    }

    pub(in crate::backend::direct_wasm) fn static_define_property_accepts_without_mutation(
        &self,
        target: &Expression,
        property: &Expression,
        descriptor_expression: &Expression,
    ) -> Option<bool> {
        let trace = std::env::var_os("AYY_TRACE_DEFINE_PROPERTY_DECISION").is_some();
        let descriptor = resolve_property_descriptor_definition(descriptor_expression)?;
        let materialized_property = self.canonical_object_property_expression(property);
        let object_binding = self.static_define_property_target_binding(target)?;
        let target_is_module_init_exports =
            target_is_module_init_exports_parameter(self.current_function_name(), target);
        let object_is_module_namespace = object_binding_is_module_namespace(&object_binding);
        let allowed = if object_is_module_namespace
            && static_property_name_from_expression(&materialized_property).is_some()
            && !target_is_module_init_exports
        {
            self.module_namespace_define_property_allowed(
                &object_binding,
                &materialized_property,
                &descriptor,
            )?
        } else {
            self.ordinary_define_property_allowed(
                &object_binding,
                &materialized_property,
                &descriptor,
            )?
        };
        if !allowed {
            if trace {
                eprintln!(
                    "define_property_decision reject target={target:?} property={materialized_property:?} descriptor={descriptor_expression:?} namespace={} module_init_exports={} extensible={}",
                    object_is_module_namespace,
                    target_is_module_init_exports,
                    object_binding.extensible
                );
            }
            return Some(false);
        }

        let current =
            self.static_property_current_descriptor(&object_binding, &materialized_property)?;
        let no_change =
            self.define_property_descriptor_matches_without_change(&current, &descriptor)?;
        if trace {
            eprintln!(
                "define_property_decision allowed target={target:?} property={materialized_property:?} descriptor={descriptor_expression:?} current={:?} no_change={no_change} namespace={} module_init_exports={} extensible={}",
                current.value,
                object_is_module_namespace,
                target_is_module_init_exports,
                object_binding.extensible
            );
        }
        no_change.then_some(true)
    }

    pub(in crate::backend::direct_wasm) fn emit_define_property_argument_effects(
        &mut self,
        target: &Expression,
        property: &Expression,
        descriptor_expression: &Expression,
    ) -> DirectResult<()> {
        self.emit_numeric_expression(target)?;
        self.state.emission.output.instructions.push(0x1a);
        self.emit_property_key_expression_effects(property)?;
        self.emit_numeric_expression(descriptor_expression)?;
        self.state.emission.output.instructions.push(0x1a);
        Ok(())
    }

    pub(in crate::backend::direct_wasm) fn emit_define_property_target_result_with_argument_effects(
        &mut self,
        target: &Expression,
        property: &Expression,
        descriptor_expression: &Expression,
    ) -> DirectResult<()> {
        let target_local = self.allocate_temp_local();
        self.emit_numeric_expression(target)?;
        self.push_local_set(target_local);
        self.emit_property_key_expression_effects(property)?;
        self.emit_numeric_expression(descriptor_expression)?;
        self.state.emission.output.instructions.push(0x1a);
        self.push_local_get(target_local);
        Ok(())
    }

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

        let requested_well_known_symbol = self
            .well_known_symbol_name(&canonical_property)
            .or_else(|| self.well_known_symbol_name(property));
        if requested_well_known_symbol.is_some()
            && Self::object_binding_has_module_namespace_marker(&object_binding)
            && self
                .resolve_object_binding_property_value(&object_binding, property)
                .is_none()
        {
            self.push_i32_const(0);
            return Ok(true);
        }

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
        let materialized_receiver_is_current_value =
            !static_expression_matches(&materialized_receiver, receiver);
        let receiver_candidates = [
            Some(receiver),
            resolved_receiver.as_ref(),
            materialized_receiver_is_current_value.then_some(&materialized_receiver),
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
                let explicit_static_value = match candidate {
                    Expression::Identifier(name) => {
                        let resolved_local_name = self
                            .resolve_current_local_binding(name)
                            .map(|(resolved_name, _)| resolved_name);
                        let has_current_local_binding = resolved_local_name.is_some();
                        resolved_local_name
                            .as_deref()
                            .and_then(|resolved_name| {
                                self.state
                                    .speculation
                                    .static_semantics
                                    .local_value_binding(resolved_name)
                            })
                            .or_else(|| {
                                self.state
                                    .speculation
                                    .static_semantics
                                    .local_value_binding(name)
                            })
                            .or_else(|| {
                                (!has_current_local_binding)
                                    .then(|| self.global_value_binding(name))
                                    .flatten()
                            })
                            .cloned()
                    }
                    _ => None,
                };
                let fallback_binding = if let Some(explicit_static_value) = explicit_static_value
                    .as_ref()
                    .filter(|value| !static_expression_matches(value, candidate))
                {
                    self.resolve_object_binding_from_expression(explicit_static_value)
                } else if materialized_receiver_is_current_value {
                    self.resolve_object_binding_from_expression(&materialized_receiver)
                } else {
                    self.resolve_object_binding_from_expression(candidate)
                        .or_else(|| match candidate {
                            Expression::Identifier(name) => self
                                .resolve_identifier_object_binding_fallback(name)
                                .or_else(|| self.resolve_runtime_shadow_object_binding(name)),
                            Expression::This => self.resolve_runtime_shadow_object_binding("this"),
                            _ => None,
                        })
                };
                let fallback_present = fallback_binding
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
        if let Some(descriptor) = resolve_property_descriptor_definition(descriptor_expression) {
            if descriptor.is_accessor() {
                return None;
            }

            return Some(descriptor.value.unwrap_or(Expression::Undefined));
        }

        if let Some(descriptor) =
            self.resolve_descriptor_binding_from_expression(descriptor_expression)
            && !descriptor.has_get
            && !descriptor.has_set
        {
            return Some(descriptor.value.unwrap_or(Expression::Undefined));
        }
        None
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

        if matches!(
            value,
            Expression::Call { .. } | Expression::SuperCall { .. } | Expression::New { .. }
        ) {
            return false;
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

    fn emit_synthetic_class_runtime_data_property_definition(
        &mut self,
        target: &Expression,
        property: &Expression,
        value: &Expression,
    ) -> DirectResult<bool> {
        if !matches!(
            value,
            Expression::Call { .. } | Expression::SuperCall { .. } | Expression::New { .. }
        ) {
            return Ok(false);
        }

        let Expression::Identifier(owner_name) = target else {
            return Ok(false);
        };
        if !owner_name.starts_with("__ayy_class_expr_")
            && !owner_name.starts_with("__ayy_class_ctor_")
        {
            return Ok(false);
        }

        let materialized_property = self.canonical_object_property_expression(property);
        if static_property_name_from_expression(&materialized_property).is_none() {
            return Ok(false);
        }

        let binding = self
            .runtime_object_property_shadow_binding_by_property(owner_name, &materialized_property);
        let deleted_binding = self.runtime_object_property_shadow_deleted_binding_by_property(
            owner_name,
            &materialized_property,
        );
        let value_local = self.allocate_temp_local();
        self.emit_numeric_expression(value)?;
        self.push_local_set(value_local);
        self.push_local_get(value_local);
        self.push_global_set(binding.value_index);
        self.push_i32_const(JS_UNDEFINED_TAG);
        self.push_global_set(deleted_binding.value_index);
        self.push_i32_const(0);
        self.push_global_set(deleted_binding.present_index);
        self.push_i32_const(1);
        self.push_global_set(binding.present_index);
        self.push_local_get(value_local);
        Ok(true)
    }

    fn emit_define_property_function_capture_initializers(
        &mut self,
        descriptor_expression: &Expression,
    ) -> DirectResult<()> {
        let mut function_expressions = Vec::new();
        if let Some(descriptor) = resolve_property_descriptor_definition(descriptor_expression) {
            if let Some(value) = descriptor.value {
                function_expressions.push(value);
            }
            if let Some(getter) = descriptor.getter {
                function_expressions.push(getter);
            }
            if let Some(setter) = descriptor.setter {
                function_expressions.push(setter);
            }
        } else if let Some(descriptor) =
            self.resolve_descriptor_binding_from_expression(descriptor_expression)
        {
            if let Some(value) = descriptor.value {
                function_expressions.push(value);
            }
            if let Some(getter) = descriptor.getter {
                function_expressions.push(getter);
            }
            if let Some(setter) = descriptor.setter {
                function_expressions.push(setter);
            }
        }

        for function_expression in function_expressions {
            let Some(LocalFunctionBinding::User(function_name)) =
                self.resolve_function_binding_from_expression(&function_expression)
            else {
                continue;
            };
            let Some(function) = self.resolve_registered_function_declaration(&function_name)
            else {
                continue;
            };
            let capture_names = function.synthetic_capture_bindings.clone();
            let Some(capture_bindings) = self.user_function_capture_bindings(&function_name) else {
                continue;
            };

            for capture_name in capture_names {
                if !capture_name.starts_with("__ayy_class_brand_") {
                    continue;
                }
                let Some((resolved_name, local_index)) =
                    self.resolve_current_local_binding(&capture_name)
                else {
                    continue;
                };
                let Some(hidden_name) = capture_bindings.get(&capture_name) else {
                    continue;
                };
                let hidden_name = hidden_name.clone();
                let hidden_binding = self
                    .implicit_global_binding(&hidden_name)
                    .unwrap_or_else(|| self.ensure_implicit_global_binding(&hidden_name));
                self.push_local_get(local_index);
                self.push_global_set(hidden_binding.value_index);
                self.push_i32_const(1);
                self.push_global_set(hidden_binding.present_index);
                self.state
                    .speculation
                    .static_semantics
                    .capture_slot_source_bindings
                    .insert(hidden_name, resolved_name);
            }
        }

        Ok(())
    }

    fn property_descriptor_binding_from_expression(
        &self,
        descriptor_expression: &Expression,
    ) -> Option<PropertyDescriptorBinding> {
        if let Some(descriptor) =
            self.resolve_descriptor_binding_from_expression(descriptor_expression)
        {
            return Some(descriptor);
        }

        if let Some(object_binding) =
            self.resolve_object_binding_from_expression(descriptor_expression)
        {
            let descriptor_property = |name: &str| {
                self.resolve_object_binding_property_value(
                    &object_binding,
                    &Expression::String(name.to_string()),
                )
            };
            let descriptor_bool_property = |name: &str| match descriptor_property(name)? {
                Expression::Bool(value) => Some(value),
                other => self.resolve_static_boolean_expression(&other),
            };
            let value = descriptor_property("value");
            let getter = descriptor_property("get");
            let setter = descriptor_property("set");
            let has_get = getter.is_some();
            let has_set = setter.is_some();
            return Some(PropertyDescriptorBinding {
                value,
                configurable: descriptor_bool_property("configurable").unwrap_or(false),
                enumerable: descriptor_bool_property("enumerable").unwrap_or(false),
                writable: if has_get || has_set {
                    None
                } else {
                    Some(descriptor_bool_property("writable").unwrap_or(false))
                },
                getter,
                setter,
                has_get,
                has_set,
            });
        }

        let descriptor = resolve_property_descriptor_definition(descriptor_expression)?;
        let value = descriptor
            .value
            .as_ref()
            .map(|value| self.materialize_static_expression(value));
        let getter = descriptor
            .getter
            .as_ref()
            .map(|getter| self.materialize_static_expression(getter));
        let setter = descriptor
            .setter
            .as_ref()
            .map(|setter| self.materialize_static_expression(setter));

        Some(PropertyDescriptorBinding {
            value,
            configurable: descriptor.configurable.unwrap_or(false),
            enumerable: descriptor.enumerable.unwrap_or(false),
            writable: if descriptor.is_accessor() {
                None
            } else {
                Some(descriptor.writable.unwrap_or(false))
            },
            getter,
            setter,
            has_get: descriptor.getter.is_some(),
            has_set: descriptor.setter.is_some(),
        })
    }

    pub(in crate::backend::direct_wasm) fn sync_static_define_property_descriptor_metadata_from_expression(
        &mut self,
        target: &Expression,
        property: &Expression,
        descriptor_expression: &Expression,
    ) {
        let Some(descriptor) =
            self.property_descriptor_binding_from_expression(descriptor_expression)
        else {
            return;
        };
        let materialized_property = self.canonical_object_property_expression(property);
        let mut object_binding = self
            .resolve_object_binding_from_expression(target)
            .or_else(|| match target {
                Expression::Identifier(name) => {
                    self.resolve_identifier_object_binding_fallback(name)
                }
                Expression::This => self.resolve_runtime_shadow_object_binding("this"),
                _ => None,
            })
            .unwrap_or_else(empty_object_value_binding);
        object_binding_define_property_descriptor(
            &mut object_binding,
            materialized_property,
            descriptor,
        );

        match target {
            Expression::Identifier(name) => {
                if self
                    .state
                    .speculation
                    .static_semantics
                    .has_local_object_binding(name)
                {
                    self.state
                        .speculation
                        .static_semantics
                        .set_local_object_binding(name, object_binding.clone());
                }
                if self.binding_name_is_global(name)
                    || self
                        .backend
                        .global_semantics
                        .values
                        .object_bindings
                        .contains_key(name)
                {
                    self.backend
                        .sync_global_object_binding(name, Some(object_binding.clone()));
                }
                if let Some(owner_name) =
                    self.runtime_object_property_shadow_owner_name_for_identifier(name)
                {
                    self.sync_runtime_object_shadow_owner_static_metadata_from_binding(
                        &owner_name,
                        &object_binding,
                    );
                }
            }
            Expression::This => {
                self.sync_runtime_object_shadow_owner_static_metadata_from_binding(
                    "this",
                    &object_binding,
                );
            }
            _ => {}
        }
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
        self.emit_define_property_function_capture_initializers(descriptor_expression)?;

        if let Some(accepted_without_mutation) = self
            .static_define_property_accepts_without_mutation(
                target,
                property,
                descriptor_expression,
            )
        {
            self.emit_define_property_argument_effects(target, property, descriptor_expression)?;
            self.push_i32_const(if accepted_without_mutation { 1 } else { 0 });
            return Ok(true);
        }

        if let Some(value) = self.reflect_define_property_data_value(descriptor_expression) {
            if self.define_property_data_value_can_emit_without_assignment(target, property, &value)
            {
                self.emit_define_property_effects_without_assignment(target, property, &value)?;
            } else if self
                .emit_synthetic_class_runtime_data_property_definition(target, property, &value)?
            {
                self.state.emission.output.instructions.push(0x1a);
            } else {
                self.emit_numeric_expression(&Expression::AssignMember {
                    object: Box::new(target.clone()),
                    property: Box::new(property.clone()),
                    value: Box::new(value),
                })?;
                self.state.emission.output.instructions.push(0x1a);
            }
            self.sync_static_define_property_descriptor_metadata_from_expression(
                target,
                property,
                descriptor_expression,
            );
            self.push_i32_const(1);
            return Ok(true);
        }

        self.sync_static_define_property_descriptor_metadata_from_expression(
            target,
            property,
            descriptor_expression,
        );
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
        self.emit_define_property_function_capture_initializers(descriptor_expression)?;

        if let Some(accepted_without_mutation) = self
            .static_define_property_accepts_without_mutation(
                target,
                property,
                descriptor_expression,
            )
        {
            if accepted_without_mutation {
                self.emit_define_property_target_result_with_argument_effects(
                    target,
                    property,
                    descriptor_expression,
                )?;
                return Ok(true);
            }
            self.emit_define_property_argument_effects(target, property, descriptor_expression)?;
            return self.emit_named_error_throw("TypeError").map(|_| true);
        }

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
            let materialized_property = self.canonical_object_property_expression(property);
            let is_private_initializer_property =
                is_private_property_name_expression(&materialized_property);
            let value = if !is_private_initializer_property
                && inline_summary_side_effect_free_expression(&value)
            {
                self.materialize_define_property_value_expression_with_this_binding(
                    &value,
                    this_binding.as_ref(),
                )
            } else {
                value
            };
            let private_initializer_definition = is_private_initializer_property
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
                .emit_synthetic_class_runtime_data_property_definition(target, property, &value)?
            {
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
            self.sync_static_define_property_descriptor_metadata_from_expression(
                target,
                property,
                descriptor_expression,
            );
        } else {
            self.sync_static_define_property_descriptor_metadata_from_expression(
                target,
                property,
                descriptor_expression,
            );
            self.emit_numeric_expression(target)?;
            self.state.emission.output.instructions.push(0x1a);
            self.emit_property_key_expression_effects(property)?;
            self.emit_numeric_expression(descriptor_expression)?;
            self.state.emission.output.instructions.push(0x1a);
        }

        self.emit_numeric_expression(target)?;
        Ok(true)
    }

    pub(in crate::backend::direct_wasm) fn emit_object_define_properties_call(
        &mut self,
        callee_object: &Expression,
        callee_property: &Expression,
        arguments: &[CallArgument],
    ) -> DirectResult<bool> {
        if !matches!(callee_object, Expression::Identifier(name) if name == "Object") {
            return Ok(false);
        }
        if !matches!(callee_property, Expression::String(name) if name == "defineProperties") {
            return Ok(false);
        }

        let [
            CallArgument::Expression(target),
            CallArgument::Expression(properties),
            rest @ ..,
        ] = arguments
        else {
            self.discard_call_arguments(arguments)?;
            self.push_i32_const(JS_TYPEOF_OBJECT_TAG);
            return Ok(true);
        };

        self.discard_call_arguments(rest)?;
        if let Expression::Object(entries) = properties {
            for entry in entries {
                let crate::ir::hir::ObjectEntry::Data {
                    key: _,
                    value: descriptor_expression,
                } = entry
                else {
                    continue;
                };
                self.emit_define_property_function_capture_initializers(descriptor_expression)?;
            }
        }

        self.apply_object_define_properties_update(arguments);
        if let Expression::Object(entries) = properties {
            for entry in entries {
                let crate::ir::hir::ObjectEntry::Data {
                    key,
                    value: descriptor_expression,
                } = entry
                else {
                    continue;
                };
                self.sync_static_define_property_descriptor_metadata_from_expression(
                    target,
                    key,
                    descriptor_expression,
                );
            }
        }
        self.emit_numeric_expression(target)?;
        Ok(true)
    }
}
