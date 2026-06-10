use super::*;

impl<'a> FunctionCompiler<'a> {
    pub(super) fn resolve_updated_specialized_function_value(
        &mut self,
        value: &Expression,
    ) -> DirectResult<Option<SpecializedFunctionValue>> {
        if self.expression_depends_on_active_loop_assignment(value) {
            return Ok(None);
        }
        if let Some(specialized) = self.resolve_specialized_function_value_from_expression(value) {
            return Ok(Some(specialized));
        }
        let Some(template) = self.resolve_function_value_template_from_expression(value) else {
            return Ok(None);
        };
        self.instantiate_specialized_function_value(&template)
    }

    pub(in crate::backend::direct_wasm) fn update_local_specialized_function_value(
        &mut self,
        name: &str,
        value: &Expression,
    ) -> DirectResult<()> {
        crate::backend::direct_wasm::memo::bump_static_state_generation();
        self.state
            .speculation
            .static_semantics
            .values
            .local_specialized_function_values
            .remove(name);
        let Some(specialized) = self.resolve_updated_specialized_function_value(value)? else {
            return Ok(());
        };
        crate::backend::direct_wasm::memo::bump_static_state_generation();
        self.state
            .speculation
            .static_semantics
            .values
            .local_specialized_function_values
            .insert(name.to_string(), specialized);
        Ok(())
    }

    pub(in crate::backend::direct_wasm) fn update_global_specialized_function_value(
        &mut self,
        name: &str,
        value: &Expression,
    ) -> DirectResult<()> {
        let trace_capture_bindings = crate::ayy_env_flag!("AYY_TRACE_CAPTURE_BINDINGS");
        if trace_capture_bindings {
            eprintln!(
                "capture_bindings update_global_specialized:start name={name} value={value:?}"
            );
        }
        crate::backend::direct_wasm::memo::bump_static_state_generation();
        self.backend
            .global_semantics
            .functions
            .specialized_function_values
            .remove(name);
        let Some(specialized) = self.resolve_updated_specialized_function_value(value)? else {
            if trace_capture_bindings {
                eprintln!("capture_bindings update_global_specialized:none name={name}");
            }
            return Ok(());
        };
        if trace_capture_bindings {
            eprintln!(
                "capture_bindings update_global_specialized:set name={name} binding={:?} return={:?} effects={}",
                specialized.binding,
                specialized.summary.return_value,
                specialized.summary.effects.len()
            );
        }
        crate::backend::direct_wasm::memo::bump_static_state_generation();
        self.backend
            .global_semantics
            .functions
            .specialized_function_values
            .insert(name.to_string(), specialized);
        Ok(())
    }
}
