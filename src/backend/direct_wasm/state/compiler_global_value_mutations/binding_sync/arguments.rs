use super::*;

impl CompilerState {
    pub(in crate::backend::direct_wasm) fn sync_global_arguments_binding(
        &mut self,
        name: &str,
        binding: Option<ArgumentsValueBinding>,
    ) {
        crate::backend::direct_wasm::memo::bump_static_state_generation();
        self.global_semantics
            .values
            .sync_arguments_binding(name, binding);
    }
}
