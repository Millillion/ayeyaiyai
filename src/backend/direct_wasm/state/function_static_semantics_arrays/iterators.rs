use super::FunctionArraySemanticsState;
use crate::backend::direct_wasm::{
    ArrayIteratorBinding, CachedIteratorNextMethodBinding, IteratorStepBinding,
};

impl FunctionArraySemanticsState {
    pub(in crate::backend::direct_wasm) fn local_array_iterator_binding(
        &self,
        name: &str,
    ) -> Option<&ArrayIteratorBinding> {
        self.local_array_iterator_bindings.get(name)
    }

    pub(in crate::backend::direct_wasm) fn local_array_iterator_binding_mut(
        &mut self,
        name: &str,
    ) -> Option<&mut ArrayIteratorBinding> {
        crate::backend::direct_wasm::memo::bump_static_state_generation();
        self.local_array_iterator_bindings.get_mut(name)
    }

    pub(in crate::backend::direct_wasm) fn has_local_array_iterator_binding(
        &self,
        name: &str,
    ) -> bool {
        self.local_array_iterator_bindings.contains_key(name)
    }

    pub(in crate::backend::direct_wasm) fn set_local_array_iterator_binding(
        &mut self,
        name: &str,
        binding: ArrayIteratorBinding,
    ) {
        crate::backend::direct_wasm::memo::bump_static_state_generation();
        self.local_array_iterator_bindings
            .insert(name.to_string(), binding);
    }

    pub(in crate::backend::direct_wasm) fn clear_local_array_iterator_binding(
        &mut self,
        name: &str,
    ) {
        crate::backend::direct_wasm::memo::bump_static_state_generation();
        self.local_array_iterator_bindings.remove(name);
    }

    pub(in crate::backend::direct_wasm) fn cached_iterator_next_method_binding(
        &self,
        name: &str,
    ) -> Option<&CachedIteratorNextMethodBinding> {
        self.cached_iterator_next_method_bindings.get(name)
    }

    pub(in crate::backend::direct_wasm) fn set_cached_iterator_next_method_binding(
        &mut self,
        name: &str,
        binding: CachedIteratorNextMethodBinding,
    ) {
        crate::backend::direct_wasm::memo::bump_static_state_generation();
        self.cached_iterator_next_method_bindings
            .insert(name.to_string(), binding);
    }

    pub(in crate::backend::direct_wasm) fn clear_cached_iterator_next_method_binding(
        &mut self,
        name: &str,
    ) {
        crate::backend::direct_wasm::memo::bump_static_state_generation();
        self.cached_iterator_next_method_bindings.remove(name);
    }

    pub(in crate::backend::direct_wasm) fn local_iterator_step_binding(
        &self,
        name: &str,
    ) -> Option<&IteratorStepBinding> {
        self.local_iterator_step_bindings.get(name)
    }

    pub(in crate::backend::direct_wasm) fn set_local_iterator_step_binding(
        &mut self,
        name: &str,
        binding: IteratorStepBinding,
    ) {
        crate::backend::direct_wasm::memo::bump_static_state_generation();
        self.local_iterator_step_bindings
            .insert(name.to_string(), binding);
    }

    pub(in crate::backend::direct_wasm) fn clear_local_iterator_step_binding(
        &mut self,
        name: &str,
    ) {
        crate::backend::direct_wasm::memo::bump_static_state_generation();
        self.local_iterator_step_bindings.remove(name);
    }
}
