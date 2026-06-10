use super::FunctionValueSemanticsState;
use crate::backend::direct_wasm::LocalFunctionBinding;

impl FunctionValueSemanticsState {
    pub(in crate::backend::direct_wasm) fn local_function_binding(
        &self,
        name: &str,
    ) -> Option<&LocalFunctionBinding> {
        self.local_function_bindings.get(name)
    }

    pub(in crate::backend::direct_wasm) fn set_local_function_binding(
        &mut self,
        name: &str,
        binding: LocalFunctionBinding,
    ) {
        crate::backend::direct_wasm::memo::bump_static_state_generation();
        self.local_function_bindings
            .insert(name.to_string(), binding);
    }

    pub(in crate::backend::direct_wasm) fn clear_local_function_binding(&mut self, name: &str) {
        crate::backend::direct_wasm::memo::bump_static_state_generation();
        self.local_function_bindings.remove(name);
    }
}
