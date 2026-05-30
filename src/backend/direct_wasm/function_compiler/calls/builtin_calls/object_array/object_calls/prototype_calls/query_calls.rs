use super::*;

impl<'a> FunctionCompiler<'a> {
    fn object_integrity_target_is_module_namespace(&self, target: &Expression) -> bool {
        if matches!(
            target,
            Expression::Identifier(name)
                if Self::module_index_from_namespace_like_identifier(name).is_some()
        ) {
            return true;
        }
        self.resolve_object_binding_from_expression(target)
            .or_else(|| match target {
                Expression::Identifier(name) => {
                    self.resolve_identifier_object_binding_fallback(name)
                }
                _ => None,
            })
            .is_some_and(|binding| Self::object_binding_has_module_namespace_marker(&binding))
    }

    fn reflect_get_static_module_namespace_result(
        &self,
        target: &Expression,
        property: &Expression,
    ) -> Option<(Option<usize>, Expression)> {
        let property_key = self
            .resolve_property_key_expression(property)
            .unwrap_or_else(|| self.materialize_static_expression(property));
        let property_key = static_property_name_from_expression(&property_key)
            .map(Expression::String)
            .unwrap_or(property_key);
        static_property_name_from_expression(&property_key)?;

        let target_binding = self
            .resolve_object_binding_from_expression(target)
            .or_else(|| match target {
                Expression::Identifier(name) => {
                    self.resolve_identifier_object_binding_fallback(name)
                }
                _ => None,
            });
        let module_index = match target {
            Expression::Identifier(name) => Self::module_index_from_namespace_like_identifier(name),
            _ => None,
        }
        .or_else(|| {
            target_binding
                .as_ref()
                .and_then(|binding| {
                    object_binding_lookup_value(
                        binding,
                        &Expression::String("__ayy$module$namespace$moduleIndex".to_string()),
                    )
                })
                .and_then(|value| match value {
                    Expression::Number(index)
                        if index.is_finite() && *index >= 0.0 && index.fract() == 0.0 =>
                    {
                        Some(*index as usize)
                    }
                    _ => None,
                })
        });

        let target_is_namespace = module_index.is_some()
            || target_binding
                .as_ref()
                .is_some_and(Self::object_binding_has_module_namespace_marker);
        if !target_is_namespace {
            return None;
        };

        let live_value =
            self.resolve_module_namespace_live_binding_member_value(target, &property_key);
        let initializer = module_index.and_then(|module_index| {
            self.resolve_static_dynamic_import_namespace_live_binding_member_initializer_value(
                module_index,
                &property_key,
            )
        });

        Some((
            module_index,
            Self::module_namespace_member_value_with_initializer_fallback(live_value, initializer)
                .unwrap_or(Expression::Undefined),
        ))
    }

    pub(in crate::backend::direct_wasm) fn emit_reflect_get_call(
        &mut self,
        callee_object: &Expression,
        callee_property: &Expression,
        arguments: &[CallArgument],
    ) -> DirectResult<bool> {
        if !matches!(callee_object, Expression::Identifier(name) if name == "Reflect") {
            return Ok(false);
        }
        if !matches!(callee_property, Expression::String(name) if name == "get") {
            return Ok(false);
        }
        let Some(CallArgument::Expression(target) | CallArgument::Spread(target)) =
            arguments.first()
        else {
            self.emit_named_error_throw("TypeError")?;
            return Ok(true);
        };
        let property = match arguments.get(1) {
            Some(CallArgument::Expression(property) | CallArgument::Spread(property)) => property,
            None => &Expression::Undefined,
        };
        let Some((module_index, result)) =
            self.reflect_get_static_module_namespace_result(target, property)
        else {
            return Ok(false);
        };

        self.emit_numeric_expression(target)?;
        self.state.emission.output.instructions.push(0x1a);
        self.emit_property_key_expression_effects(property)?;
        self.discard_call_arguments(&arguments[2..])?;
        if let Some(module_index) = module_index {
            self.emit_sync_module_init_if_needed(module_index, &mut HashSet::new())?;
        }
        self.emit_numeric_expression(&result)?;
        Ok(true)
    }

    pub(in crate::backend::direct_wasm) fn emit_reflect_set_call(
        &mut self,
        callee_object: &Expression,
        callee_property: &Expression,
        arguments: &[CallArgument],
    ) -> DirectResult<bool> {
        if !matches!(callee_object, Expression::Identifier(name) if name == "Reflect") {
            return Ok(false);
        }
        if !matches!(callee_property, Expression::String(name) if name == "set") {
            return Ok(false);
        }
        let Some(CallArgument::Expression(target) | CallArgument::Spread(target)) =
            arguments.first()
        else {
            self.emit_named_error_throw("TypeError")?;
            return Ok(true);
        };
        let property = match arguments.get(1) {
            Some(CallArgument::Expression(property) | CallArgument::Spread(property)) => property,
            None => &Expression::Undefined,
        };
        let value = match arguments.get(2) {
            Some(CallArgument::Expression(value) | CallArgument::Spread(value)) => value,
            None => &Expression::Undefined,
        };

        self.emit_numeric_expression(target)?;
        self.state.emission.output.instructions.push(0x1a);
        self.emit_numeric_expression(property)?;
        self.state.emission.output.instructions.push(0x1a);
        self.emit_numeric_expression(value)?;
        self.state.emission.output.instructions.push(0x1a);
        if arguments.len() > 3 {
            self.discard_call_arguments(&arguments[3..])?;
        }

        let target_binding = self
            .resolve_object_binding_from_expression(target)
            .or_else(|| match target {
                Expression::Identifier(name) => {
                    self.resolve_identifier_object_binding_fallback(name)
                }
                _ => None,
            });
        if target_binding
            .as_ref()
            .is_some_and(Self::object_binding_has_module_namespace_marker)
            || self
                .module_namespace_index_from_expression(target)
                .is_some()
        {
            self.push_i32_const(0);
            return Ok(true);
        }

        Ok(false)
    }

    pub(in crate::backend::direct_wasm) fn emit_reflect_has_call(
        &mut self,
        callee_object: &Expression,
        callee_property: &Expression,
        arguments: &[CallArgument],
    ) -> DirectResult<bool> {
        if !matches!(callee_object, Expression::Identifier(name) if name == "Reflect") {
            return Ok(false);
        }
        if !matches!(callee_property, Expression::String(name) if name == "has") {
            return Ok(false);
        }
        let Some(CallArgument::Expression(target) | CallArgument::Spread(target)) =
            arguments.first()
        else {
            self.emit_named_error_throw("TypeError")?;
            return Ok(true);
        };
        let property = match arguments.get(1) {
            Some(CallArgument::Expression(property) | CallArgument::Spread(property)) => {
                property.clone()
            }
            None => Expression::Undefined,
        };

        self.emit_numeric_expression(target)?;
        self.state.emission.output.instructions.push(0x1a);
        self.emit_numeric_expression(&property)?;
        self.state.emission.output.instructions.push(0x1a);
        self.discard_call_arguments(&arguments[2..])?;

        if let Some(has_property) = self.resolve_static_reflect_has_result(target, &property) {
            self.push_i32_const(has_property as i32);
            return Ok(true);
        }

        self.emit_object_get_own_property_descriptor_result(target, &property)?;
        self.push_i32_const(JS_UNDEFINED_TAG);
        self.push_binary_op(BinaryOp::NotEqual)?;
        Ok(true)
    }

    pub(in crate::backend::direct_wasm) fn emit_object_get_prototype_of_call(
        &mut self,
        callee_object: &Expression,
        callee_property: &Expression,
        arguments: &[CallArgument],
    ) -> DirectResult<bool> {
        let reflect_call =
            matches!(callee_object, Expression::Identifier(name) if name == "Reflect");
        if !matches!(callee_object, Expression::Identifier(name) if name == "Object" || name == "Reflect")
        {
            return Ok(false);
        }
        if !matches!(callee_property, Expression::String(name) if name == "getPrototypeOf") {
            return Ok(false);
        }
        let [CallArgument::Expression(target), rest @ ..] = arguments else {
            if reflect_call {
                self.emit_named_error_throw("TypeError")?;
            } else {
                self.push_i32_const(JS_UNDEFINED_TAG);
            }
            return Ok(true);
        };
        if Self::expression_is_module_namespace_member_read(target)
            && let Some(prototype) = self.resolve_static_object_prototype_expression(target)
        {
            self.discard_call_arguments(rest)?;
            let prototype = self.resolve_static_class_init_local_aliases_in_expression(&prototype);
            self.emit_numeric_expression(&prototype)?;
            return Ok(true);
        }
        self.emit_numeric_expression(target)?;
        self.state.emission.output.instructions.push(0x1a);
        self.discard_call_arguments(rest)?;
        if let Some(prototype) = self.resolve_static_object_prototype_expression(target) {
            let prototype = self.resolve_static_class_init_local_aliases_in_expression(&prototype);
            self.emit_numeric_expression(&prototype)?;
        } else {
            self.push_i32_const(JS_UNDEFINED_TAG);
        }
        Ok(true)
    }

    fn expression_is_module_namespace_member_read(expression: &Expression) -> bool {
        matches!(
            expression,
            Expression::Member { object, property }
                if matches!(object.as_ref(), Expression::Identifier(name) if Self::module_index_from_namespace_like_identifier(name).is_some())
                    && matches!(property.as_ref(), Expression::String(_))
        )
    }

    pub(in crate::backend::direct_wasm) fn emit_object_is_extensible_call(
        &mut self,
        callee_object: &Expression,
        callee_property: &Expression,
        arguments: &[CallArgument],
    ) -> DirectResult<bool> {
        let reflect_call =
            matches!(callee_object, Expression::Identifier(name) if name == "Reflect");
        if !matches!(callee_object, Expression::Identifier(name) if name == "Object" || name == "Reflect")
        {
            return Ok(false);
        }
        if !matches!(callee_property, Expression::String(name) if name == "isExtensible") {
            return Ok(false);
        }
        let target = match arguments.first() {
            Some(CallArgument::Expression(target)) | Some(CallArgument::Spread(target)) => target,
            None => {
                if reflect_call {
                    self.emit_named_error_throw("TypeError")?;
                } else {
                    self.push_i32_const(0);
                }
                return Ok(true);
            }
        };
        self.emit_numeric_expression(target)?;
        self.state.emission.output.instructions.push(0x1a);
        self.discard_call_arguments(&arguments[1..])?;
        self.push_i32_const(
            if self.resolve_static_object_extensibility(target) == Some(true) {
                1
            } else {
                0
            },
        );
        Ok(true)
    }

    fn emit_object_integrity_query_call(
        &mut self,
        callee_object: &Expression,
        callee_property: &Expression,
        expected_property: &str,
        arguments: &[CallArgument],
    ) -> DirectResult<bool> {
        if !matches!(callee_object, Expression::Identifier(name) if name == "Object") {
            return Ok(false);
        }
        if !matches!(callee_property, Expression::String(name) if name == expected_property) {
            return Ok(false);
        }
        let target = match arguments.first() {
            Some(CallArgument::Expression(target)) | Some(CallArgument::Spread(target)) => target,
            None => {
                self.push_i32_const(1);
                return Ok(true);
            }
        };
        self.emit_numeric_expression(target)?;
        self.state.emission.output.instructions.push(0x1a);
        self.discard_call_arguments(&arguments[1..])?;
        if expected_property == "isFrozen"
            && self.object_integrity_target_is_module_namespace(target)
        {
            self.push_i32_const(0);
            return Ok(true);
        }
        self.push_i32_const(
            if self.resolve_static_object_extensibility(target) == Some(false) {
                1
            } else {
                0
            },
        );
        Ok(true)
    }

    pub(in crate::backend::direct_wasm) fn emit_object_is_sealed_call(
        &mut self,
        callee_object: &Expression,
        callee_property: &Expression,
        arguments: &[CallArgument],
    ) -> DirectResult<bool> {
        self.emit_object_integrity_query_call(callee_object, callee_property, "isSealed", arguments)
    }

    pub(in crate::backend::direct_wasm) fn emit_object_is_frozen_call(
        &mut self,
        callee_object: &Expression,
        callee_property: &Expression,
        arguments: &[CallArgument],
    ) -> DirectResult<bool> {
        self.emit_object_integrity_query_call(callee_object, callee_property, "isFrozen", arguments)
    }

    pub(in crate::backend::direct_wasm) fn emit_object_prevent_extensions_call(
        &mut self,
        callee_object: &Expression,
        callee_property: &Expression,
        arguments: &[CallArgument],
    ) -> DirectResult<bool> {
        let reflect_call =
            matches!(callee_object, Expression::Identifier(name) if name == "Reflect");
        if !matches!(callee_object, Expression::Identifier(name) if name == "Object" || name == "Reflect")
        {
            return Ok(false);
        }
        if !matches!(callee_property, Expression::String(name) if name == "preventExtensions") {
            return Ok(false);
        }
        let Some(CallArgument::Expression(target) | CallArgument::Spread(target)) =
            arguments.first()
        else {
            if reflect_call {
                self.emit_named_error_throw("TypeError")?;
            } else {
                self.push_i32_const(JS_UNDEFINED_TAG);
            }
            return Ok(true);
        };
        let target_local = self.allocate_temp_local();
        self.emit_numeric_expression(target)?;
        self.push_local_set(target_local);
        self.discard_call_arguments(&arguments[1..])?;
        self.apply_object_prevent_extensions_update(callee_object, arguments);
        if reflect_call {
            self.push_i32_const(1);
        } else {
            self.push_local_get(target_local);
        }
        Ok(true)
    }

    pub(in crate::backend::direct_wasm) fn emit_object_freeze_call(
        &mut self,
        callee_object: &Expression,
        callee_property: &Expression,
        arguments: &[CallArgument],
    ) -> DirectResult<bool> {
        if !matches!(callee_object, Expression::Identifier(name) if name == "Object") {
            return Ok(false);
        }
        if !matches!(callee_property, Expression::String(name) if name == "freeze") {
            return Ok(false);
        }
        let Some(CallArgument::Expression(target) | CallArgument::Spread(target)) =
            arguments.first()
        else {
            self.push_i32_const(JS_UNDEFINED_TAG);
            return Ok(true);
        };
        let target_local = self.allocate_temp_local();
        self.emit_numeric_expression(target)?;
        self.push_local_set(target_local);
        self.discard_call_arguments(&arguments[1..])?;
        if self.object_integrity_target_is_module_namespace(target) {
            self.emit_named_error_throw("TypeError")?;
            return Ok(true);
        }
        self.apply_object_freeze_update(arguments);
        self.push_local_get(target_local);
        Ok(true)
    }

    pub(in crate::backend::direct_wasm) fn emit_object_seal_call(
        &mut self,
        callee_object: &Expression,
        callee_property: &Expression,
        arguments: &[CallArgument],
    ) -> DirectResult<bool> {
        if !matches!(callee_object, Expression::Identifier(name) if name == "Object") {
            return Ok(false);
        }
        if !matches!(callee_property, Expression::String(name) if name == "seal") {
            return Ok(false);
        }
        let Some(CallArgument::Expression(target) | CallArgument::Spread(target)) =
            arguments.first()
        else {
            self.push_i32_const(JS_UNDEFINED_TAG);
            return Ok(true);
        };
        let target_local = self.allocate_temp_local();
        self.emit_numeric_expression(target)?;
        self.push_local_set(target_local);
        self.discard_call_arguments(&arguments[1..])?;
        self.apply_object_freeze_update(arguments);
        self.push_local_get(target_local);
        Ok(true)
    }
}
