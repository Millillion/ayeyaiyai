use super::*;

impl<'a> FunctionCompiler<'a> {
    fn emit_super_property_key_expression_effects(
        &mut self,
        property: &Expression,
    ) -> DirectResult<Option<ResolvedPropertyKey>> {
        let resolved = self.resolve_property_key_expression_with_coercion(property);
        self.emit_numeric_expression(property)?;
        self.state.emission.output.instructions.push(0x1a);
        Ok(resolved)
    }

    fn emit_super_property_key_coercion_effect(
        &mut self,
        binding: &LocalFunctionBinding,
    ) -> DirectResult<()> {
        match binding {
            LocalFunctionBinding::User(function_name) => {
                if let Some(user_function) = self.user_function(function_name).cloned() {
                    self.with_suspended_with_scopes(|compiler| {
                        if compiler
                            .emit_inline_user_function_summary_with_arguments(&user_function, &[])?
                        {
                            compiler.state.emission.output.instructions.push(0x1a);
                        } else {
                            compiler.emit_user_function_call(&user_function, &[])?;
                            compiler.state.emission.output.instructions.push(0x1a);
                        }
                        Ok(())
                    })?;
                }
            }
            LocalFunctionBinding::Builtin(function_name) => {
                self.with_suspended_with_scopes(|compiler| {
                    if compiler.emit_builtin_call(function_name, &[])? {
                        compiler.state.emission.output.instructions.push(0x1a);
                    }
                    Ok(())
                })?;
            }
        }
        Ok(())
    }

    fn super_base_is_statically_nullish(&self, super_base: Option<&Expression>) -> bool {
        super_base.is_some_and(|base| {
            matches!(
                self.infer_value_kind(base),
                Some(StaticValueKind::Null | StaticValueKind::Undefined)
            )
        })
    }

    fn emit_throw_if_runtime_super_base_nullish_after_value(
        &mut self,
        binding: &GlobalObjectRuntimePrototypeBinding,
        state_local: u32,
        value: &Expression,
    ) -> DirectResult<()> {
        for (variant_index, prototype) in binding.variants.iter().enumerate() {
            if prototype.is_some() {
                continue;
            }
            self.push_local_get(state_local);
            self.push_i32_const(variant_index as i32);
            self.push_binary_op(BinaryOp::Equal)?;
            self.state.emission.output.instructions.push(0x04);
            self.state
                .emission
                .output
                .instructions
                .push(EMPTY_BLOCK_TYPE);
            self.push_control_frame();
            self.emit_numeric_expression(value)?;
            self.state.emission.output.instructions.push(0x1a);
            self.emit_named_error_throw("TypeError")?;
            self.state.emission.output.instructions.push(0x0b);
            self.pop_control_frame();
        }
        Ok(())
    }

    pub(in crate::backend::direct_wasm) fn emit_assign_super_member_expression(
        &mut self,
        property: &Expression,
        value: &Expression,
    ) -> DirectResult<()> {
        let runtime_prototype_binding =
            self.resolve_super_runtime_prototype_binding_with_context(self.current_function_name());
        let runtime_state_local = runtime_prototype_binding
            .as_ref()
            .and_then(|(_, binding)| binding.global_index)
            .map(|global_index| {
                let local = self.allocate_temp_local();
                self.push_global_get(global_index);
                self.push_local_set(local);
                local
            });

        let resolved_property = self.emit_super_property_key_expression_effects(property)?;
        let super_base =
            self.resolve_super_base_expression_with_context(self.current_function_name());
        let Some(resolved_property) = resolved_property else {
            self.emit_numeric_expression(value)?;
            self.state.emission.output.instructions.push(0x1a);
            if let Some((_, binding)) = runtime_prototype_binding.as_ref()
                && let Some(state_local) = runtime_state_local
            {
                self.emit_throw_if_runtime_super_base_nullish_after_value(
                    binding,
                    state_local,
                    &Expression::Undefined,
                )?;
            }
            if self.super_base_is_statically_nullish(super_base.as_ref()) {
                self.emit_named_error_throw("TypeError")?;
                return Ok(());
            }
            self.push_i32_const(JS_TYPEOF_OBJECT_TAG);
            return Ok(());
        };
        let effective_property = resolved_property.key;
        let property_coercion = resolved_property.coercion;

        if self.super_base_is_statically_nullish(super_base.as_ref()) {
            self.emit_numeric_expression(value)?;
            self.state.emission.output.instructions.push(0x1a);
            self.emit_named_error_throw("TypeError")?;
            return Ok(());
        }

        if let Some((_, binding)) = runtime_prototype_binding.as_ref()
            && let Some(state_local) = runtime_state_local
        {
            self.emit_throw_if_runtime_super_base_nullish_after_value(binding, state_local, value)?;
        }

        if let Some((_, binding)) = runtime_prototype_binding.as_ref()
            && let Some(state_local) = runtime_state_local
            && let Some(variants) =
                self.resolve_user_super_setter_variants(binding, &effective_property)
        {
            let value_local = self.allocate_temp_local();
            self.emit_numeric_expression(value)?;
            self.push_local_set(value_local);
            if let Some(coercion) = property_coercion.as_ref() {
                self.emit_super_property_key_coercion_effect(coercion)?;
            }
            self.emit_super_member_user_setter_call_via_runtime_prototype_state(
                &variants,
                state_local,
                value_local,
            )?;
            self.push_local_get(value_local);
            return Ok(());
        }

        if runtime_prototype_binding.is_none()
            && let Some(super_base) = super_base.as_ref()
            && let Some((user_function, capture_slots)) =
                self.resolve_user_super_setter_call(super_base, &effective_property)
        {
            let value_local = self.allocate_temp_local();
            self.emit_numeric_expression(value)?;
            self.push_local_set(value_local);
            if let Some(coercion) = property_coercion.as_ref() {
                self.emit_super_property_key_coercion_effect(coercion)?;
            }
            self.emit_super_member_user_setter_call(
                &user_function,
                capture_slots.as_ref(),
                value_local,
            )?;
            self.push_local_get(value_local);
            return Ok(());
        }

        self.emit_numeric_expression(&Expression::AssignMember {
            object: Box::new(Expression::This),
            property: Box::new(effective_property),
            value: Box::new(value.clone()),
        })
    }
}
