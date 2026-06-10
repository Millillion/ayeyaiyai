use super::*;

impl FunctionCompilerState {
    pub(in crate::backend::direct_wasm) fn clear_eval_local_function_binding_metadata(
        &mut self,
        name: &str,
    ) {
        crate::backend::direct_wasm::memo::bump_static_state_generation();
        self.speculation
            .static_semantics
            .clear_eval_local_function_binding_metadata(name);
    }

    pub(in crate::backend::direct_wasm) fn clear_local_static_binding_metadata(
        &mut self,
        name: &str,
    ) {
        crate::backend::direct_wasm::memo::bump_static_state_generation();
        self.speculation
            .static_semantics
            .clear_local_static_binding_metadata(name);
        self.parameters.clear_local_binding_metadata(name);
    }

    pub(in crate::backend::direct_wasm) fn clear_local_runtime_binding_metadata(
        &mut self,
        name: &str,
    ) {
        crate::backend::direct_wasm::memo::bump_static_state_generation();
        self.runtime.locals.remove(name);
        self.speculation
            .static_semantics
            .clear_local_runtime_binding_metadata(name);
        self.parameters.clear_local_binding_metadata(name);
    }

    pub(in crate::backend::direct_wasm) fn clear_member_bindings_for_name(
        &mut self,
        name: &str,
        include_prototype: bool,
    ) {
        crate::backend::direct_wasm::memo::bump_static_state_generation();
        self.speculation
            .static_semantics
            .clear_member_bindings_for_name(name, include_prototype);
    }
}
