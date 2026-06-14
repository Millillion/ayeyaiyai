use super::{
    ArrayValueBinding, FunctionRegistryState, GlobalSemanticState, ModuleArtifactsState,
    Test262State,
};
use std::collections::HashMap;

#[derive(Clone, Copy)]
pub(in crate::backend::direct_wasm) struct ProcessArgvRuntimeLayout {
    pub(in crate::backend::direct_wasm) argv_pointers_offset: u32,
    pub(in crate::backend::direct_wasm) raw_buffer_offset: u32,
    pub(in crate::backend::direct_wasm) string_slots_offset: u32,
    pub(in crate::backend::direct_wasm) display_name_ptr: u32,
}

#[derive(Default)]
pub(in crate::backend::direct_wasm) struct DirectWasmCompiler {
    pub(in crate::backend::direct_wasm) state: CompilerState,
}

#[derive(Default)]
pub(in crate::backend::direct_wasm) struct CompilerState {
    pub(in crate::backend::direct_wasm) module_artifacts: ModuleArtifactsState,
    pub(in crate::backend::direct_wasm) function_registry: FunctionRegistryState,
    pub(in crate::backend::direct_wasm) global_semantics: GlobalSemanticState,
    pub(in crate::backend::direct_wasm) test262: Test262State,
    pub(in crate::backend::direct_wasm) template_object_array_bindings:
        HashMap<i32, ArrayValueBinding>,
    pub(in crate::backend::direct_wasm) template_object_raw_array_bindings:
        HashMap<i32, ArrayValueBinding>,
    pub(in crate::backend::direct_wasm) process_argv_layout: Option<ProcessArgvRuntimeLayout>,
}

#[derive(Clone, Copy)]
pub(in crate::backend::direct_wasm) struct ImplicitGlobalBinding {
    pub(in crate::backend::direct_wasm) value_index: u32,
    pub(in crate::backend::direct_wasm) present_index: u32,
}

#[derive(Clone, Copy)]
pub(in crate::backend::direct_wasm) struct LexicalGlobalBinding {
    pub(in crate::backend::direct_wasm) initialized_index: u32,
    pub(in crate::backend::direct_wasm) mutable: bool,
}
