use super::super::super::super::super::*;
use crate::backend::direct_wasm::GlobalValueService;

impl GlobalValueService {
    pub(in crate::backend::direct_wasm) fn clear_value_binding(&mut self, name: &str) {
        crate::backend::direct_wasm::memo::bump_static_state_generation();
        self.value_bindings.remove(name);
    }

    pub(in crate::backend::direct_wasm) fn set_value_binding(
        &mut self,
        name: String,
        value: Expression,
    ) {
        crate::backend::direct_wasm::memo::bump_static_state_generation();
        self.value_bindings.insert(name, value);
    }
}
