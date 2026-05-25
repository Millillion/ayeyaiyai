use super::*;

impl<'a> FunctionCompiler<'a> {
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
        if !matches!(callee_object, Expression::Identifier(name) if name == "Object") {
            return Ok(false);
        }
        if !matches!(callee_property, Expression::String(name) if name == "getPrototypeOf") {
            return Ok(false);
        }
        let [CallArgument::Expression(target), rest @ ..] = arguments else {
            self.push_i32_const(JS_UNDEFINED_TAG);
            return Ok(true);
        };
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

    pub(in crate::backend::direct_wasm) fn emit_object_is_extensible_call(
        &mut self,
        callee_object: &Expression,
        callee_property: &Expression,
        arguments: &[CallArgument],
    ) -> DirectResult<bool> {
        if !matches!(callee_object, Expression::Identifier(name) if name == "Object") {
            return Ok(false);
        }
        if !matches!(callee_property, Expression::String(name) if name == "isExtensible") {
            return Ok(false);
        }
        let target = match arguments.first() {
            Some(CallArgument::Expression(target)) | Some(CallArgument::Spread(target)) => target,
            None => {
                self.push_i32_const(0);
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
