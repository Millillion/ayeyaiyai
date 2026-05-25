use super::super::*;
use std::collections::HashMap;

impl<'a> FunctionCompilerBackend<'a> {
    pub(in crate::backend::direct_wasm) fn new(
        module_artifacts: &'a mut ModuleArtifactsState,
        function_registry: &'a mut FunctionRegistryState,
        shared_global_semantics: &'a mut GlobalSemanticState,
        test262: &'a mut Test262State,
        template_object_array_bindings: &'a HashMap<i32, ArrayValueBinding>,
        template_object_raw_array_bindings: &'a HashMap<i32, ArrayValueBinding>,
        global_semantics: GlobalStaticSemanticsSnapshot,
    ) -> FunctionCompilerBackend<'a> {
        Self {
            module_artifacts,
            function_registry,
            shared_global_semantics,
            test262,
            template_object_array_bindings,
            template_object_raw_array_bindings,
            global_semantics,
        }
    }

    pub(in crate::backend::direct_wasm) fn intern_string(&mut self, bytes: Vec<u8>) -> (u32, u32) {
        self.module_artifacts.intern_string(bytes)
    }
}
