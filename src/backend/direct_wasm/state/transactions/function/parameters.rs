use super::super::super::*;

#[derive(Clone)]
pub(in crate::backend::direct_wasm) struct FunctionParameterIsolatedIndirectEvalSnapshot {
    pub(in crate::backend::direct_wasm) in_parameter_default_initialization: bool,
    pub(in crate::backend::direct_wasm) actual_argument_count_local: Option<u32>,
    pub(in crate::backend::direct_wasm) arguments_slots: HashMap<u32, ArgumentsSlot>,
    pub(in crate::backend::direct_wasm) local_arguments_bindings:
        HashMap<String, ArgumentsValueBinding>,
    pub(in crate::backend::direct_wasm) direct_arguments_aliases: HashSet<String>,
}

impl FunctionParameterState {
    pub(in crate::backend::direct_wasm) fn capture_isolated_indirect_eval(
        &self,
    ) -> FunctionParameterIsolatedIndirectEvalSnapshot {
        FunctionParameterIsolatedIndirectEvalSnapshot {
            in_parameter_default_initialization: self.in_parameter_default_initialization,
            actual_argument_count_local: self.actual_argument_count_local,
            arguments_slots: self.arguments_slots.clone(),
            local_arguments_bindings: self.local_arguments_bindings.clone(),
            direct_arguments_aliases: self.direct_arguments_aliases.clone(),
        }
    }

    pub(in crate::backend::direct_wasm) fn restore_isolated_indirect_eval(
        &mut self,
        snapshot: FunctionParameterIsolatedIndirectEvalSnapshot,
    ) {
        self.in_parameter_default_initialization = snapshot.in_parameter_default_initialization;
        self.actual_argument_count_local = snapshot.actual_argument_count_local;
        self.arguments_slots = snapshot.arguments_slots;
        self.local_arguments_bindings = snapshot.local_arguments_bindings;
        self.direct_arguments_aliases = snapshot.direct_arguments_aliases;
    }
}
