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
}
