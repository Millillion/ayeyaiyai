use super::FunctionObjectSemanticsState;

impl FunctionObjectSemanticsState {
    pub(in crate::backend::direct_wasm) fn clear_isolated_indirect_eval_state(&mut self) {
        crate::backend::direct_wasm::memo::bump_static_state_generation();
        self.member_function_bindings.clear();
        self.member_function_capture_slots.clear();
        self.member_getter_bindings.clear();
        self.member_setter_bindings.clear();
        self.local_object_bindings.clear();
        self.local_prototype_object_bindings.clear();
        self.local_descriptor_bindings.clear();
    }

    pub(in crate::backend::direct_wasm) fn clear_local_binding_metadata(&mut self, name: &str) {
        crate::backend::direct_wasm::memo::bump_static_state_generation();
        self.local_object_bindings.remove(name);
        self.local_prototype_object_bindings.remove(name);
        self.local_descriptor_bindings.remove(name);
    }
}
