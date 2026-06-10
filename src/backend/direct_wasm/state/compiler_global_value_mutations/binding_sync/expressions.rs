use super::*;

impl CompilerState {
    pub(in crate::backend::direct_wasm) fn set_global_expression_binding(
        &mut self,
        name: &str,
        value: Expression,
    ) {
        crate::backend::direct_wasm::memo::bump_static_state_generation();
        self.global_semantics
            .values
            .set_value_binding(name.to_string(), value);
    }
}
