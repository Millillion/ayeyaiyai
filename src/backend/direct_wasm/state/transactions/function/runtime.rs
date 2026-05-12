use super::super::super::*;

#[derive(Clone)]
pub(in crate::backend::direct_wasm) struct FunctionRuntimeIsolatedIndirectEvalSnapshot {
    pub(in crate::backend::direct_wasm) runtime_locals: FunctionRuntimeLocalsState,
}

impl FunctionRuntimeState {
    pub(in crate::backend::direct_wasm) fn snapshot_isolated_indirect_eval(
        &self,
    ) -> FunctionRuntimeIsolatedIndirectEvalSnapshot {
        FunctionRuntimeIsolatedIndirectEvalSnapshot {
            runtime_locals: self.locals.clone(),
        }
    }

    pub(in crate::backend::direct_wasm) fn restore_isolated_indirect_eval(
        &mut self,
        snapshot: FunctionRuntimeIsolatedIndirectEvalSnapshot,
    ) {
        let next_local_index = self
            .locals
            .next_local_index
            .max(snapshot.runtime_locals.next_local_index);
        self.locals = snapshot.runtime_locals;
        self.locals.next_local_index = next_local_index;
    }
}
