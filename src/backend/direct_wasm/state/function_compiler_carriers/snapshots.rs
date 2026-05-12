use super::super::{
    FunctionCompiler, GlobalBindingEnvironment, SharedGlobalBindingEnvironment,
    StaticResolutionEnvironment,
};
use crate::ir::hir::Expression;
use std::collections::HashMap;

impl FunctionCompiler<'_> {
    pub(in crate::backend::direct_wasm) fn assigned_nonlocal_binding_results(
        &self,
        function_name: &str,
    ) -> Option<&HashMap<String, Expression>> {
        self.assigned_nonlocal_binding_results.get(function_name)
    }
}

impl<'a> FunctionCompiler<'a> {
    fn live_shared_global_binding_environment(&self) -> SharedGlobalBindingEnvironment {
        let mut value_bindings = self
            .backend
            .global_semantics
            .values
            .snapshot_value_bindings();
        value_bindings.extend(
            self.backend
                .shared_global_semantics
                .values
                .snapshot_value_bindings(),
        );
        let mut object_bindings = self
            .backend
            .global_semantics
            .values
            .snapshot_object_bindings();
        object_bindings.extend(
            self.backend
                .shared_global_semantics
                .values
                .snapshot_object_bindings(),
        );
        SharedGlobalBindingEnvironment::from_binding_environment(&GlobalBindingEnvironment {
            value_bindings,
            object_bindings,
        })
    }

    pub(in crate::backend::direct_wasm) fn snapshot_static_resolution_environment(
        &self,
    ) -> StaticResolutionEnvironment {
        let shared_global_bindings = self.live_shared_global_binding_environment();
        self.state
            .snapshot_static_resolution_environment(&shared_global_bindings)
    }

    pub(in crate::backend::direct_wasm) fn snapshot_static_resolution_environment_with_local_bindings(
        &self,
        local_bindings: HashMap<String, Expression>,
    ) -> StaticResolutionEnvironment {
        let shared_global_bindings = self.live_shared_global_binding_environment();
        self.state
            .snapshot_static_resolution_environment_with_local_bindings(
                &shared_global_bindings,
                local_bindings,
            )
    }

    pub(in crate::backend::direct_wasm) fn snapshot_static_resolution_environment_without_locals(
        &self,
    ) -> StaticResolutionEnvironment {
        self.snapshot_static_resolution_environment_with_local_bindings(HashMap::new())
    }
}
