use super::super::{FunctionCompilerBackend, ImplicitGlobalBinding};

impl<'a> FunctionCompilerBackend<'a> {
    pub(in crate::backend::direct_wasm) fn ensure_implicit_global_binding(
        &mut self,
        name: &str,
    ) -> ImplicitGlobalBinding {
        if let Some(binding) = self.global_semantics.global_names().implicit_binding(name) {
            return binding;
        }

        let binding = self.shared_global_semantics.ensure_implicit_binding(name);
        self.global_semantics.sync_implicit_binding(name, binding);
        binding
    }

    pub(in crate::backend::direct_wasm) fn record_emitted_delete_shadow(&mut self, name: &str) {
        self.shared_global_semantics
            .names
            .emitted_delete_shadow_names
            .insert(name.to_string());
        self.global_semantics
            .names
            .emitted_delete_shadow_names
            .insert(name.to_string());
    }

    pub(in crate::backend::direct_wasm) fn delete_shadow_was_emitted(&self, name: &str) -> bool {
        self.global_semantics
            .names
            .emitted_delete_shadow_names
            .contains(name)
            || self
                .shared_global_semantics
                .names
                .emitted_delete_shadow_names
                .contains(name)
    }
}
