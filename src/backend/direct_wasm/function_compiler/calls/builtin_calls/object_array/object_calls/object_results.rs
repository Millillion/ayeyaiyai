use super::*;

impl<'a> FunctionCompiler<'a> {
    fn module_namespace_own_property_keys_trigger_module_index(
        &self,
        target: &Expression,
    ) -> Option<usize> {
        let module_index = self.module_namespace_index_from_expression(target)?;
        if self.current_function_name().is_some_and(|function_name| {
            function_name == format!("__ayy_module_init_{module_index}")
        }) {
            return None;
        }
        Some(module_index)
    }

    fn emit_module_namespace_object_keys_descriptor_reads(
        &mut self,
        module_index: usize,
    ) -> DirectResult<()> {
        let Some(names) =
            self.resolve_static_dynamic_import_namespace_own_property_names_binding(module_index)
        else {
            return Ok(());
        };

        for name in names.values.into_iter().flatten() {
            let Expression::String(name) = name else {
                continue;
            };
            let property = Expression::String(name);
            let live_value = self
                .resolve_static_dynamic_import_namespace_live_binding_member_value(
                    module_index,
                    &property,
                );
            if let Some(live_value) = live_value.as_ref()
                && self.module_namespace_live_value_is_readable_in_current_context(live_value)
            {
                self.emit_numeric_expression(live_value)?;
                self.state.emission.output.instructions.push(0x1a);
                continue;
            }
            if let Some((binding_name, _)) = self
                .resolve_static_dynamic_import_namespace_live_binding_member_binding_initializer_value(
                    module_index,
                    &property,
                )
            {
                let binding = Expression::Identifier(binding_name);
                if self.module_namespace_live_value_is_readable_in_current_context(&binding) {
                    self.emit_numeric_expression(&binding)?;
                    self.state.emission.output.instructions.push(0x1a);
                }
            }
        }

        Ok(())
    }

    pub(in crate::backend::direct_wasm) fn emit_object_create_call(
        &mut self,
        callee_object: &Expression,
        callee_property: &Expression,
        arguments: &[CallArgument],
    ) -> DirectResult<bool> {
        if !matches!(callee_object, Expression::Identifier(name) if name == "Object") {
            return Ok(false);
        }
        if !matches!(callee_property, Expression::String(name) if name == "create") {
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
        self.push_i32_const(JS_TYPEOF_OBJECT_TAG);
        Ok(true)
    }

    pub(in crate::backend::direct_wasm) fn emit_object_array_builtin_call(
        &mut self,
        callee_object: &Expression,
        callee_property: &Expression,
        arguments: &[CallArgument],
    ) -> DirectResult<bool> {
        let supported = matches!(
            (callee_object, callee_property),
            (
                Expression::Identifier(object_name),
                Expression::String(property_name),
            ) if object_name == "Object"
                && matches!(
                    property_name.as_str(),
                    "keys" | "getOwnPropertyNames" | "getOwnPropertySymbols"
                )
        ) || matches!(
            (callee_object, callee_property),
            (
                Expression::Identifier(object_name),
                Expression::String(property_name),
            ) if object_name == "Reflect" && property_name == "ownKeys"
        );
        if !supported {
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
            CallArgument::Expression(target) | CallArgument::Spread(target),
            ..,
        ] = arguments
        {
            if let Some(module_index) =
                self.module_namespace_own_property_keys_trigger_module_index(target)
            {
                self.emit_sync_module_init_if_needed(
                    module_index,
                    &mut std::collections::HashSet::new(),
                )?;
            }
            if matches!(callee_property, Expression::String(property_name) if property_name == "keys")
                && let Some(module_index) = self.module_namespace_index_from_expression(target)
            {
                self.emit_module_namespace_object_keys_descriptor_reads(module_index)?;
            }
        }
        self.push_i32_const(JS_TYPEOF_OBJECT_TAG);
        Ok(true)
    }
}
