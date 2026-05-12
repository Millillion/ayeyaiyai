use super::*;

#[derive(Clone)]
pub(in crate::backend::direct_wasm) enum IteratorSourceKind {
    StaticArray {
        values: Vec<Option<Expression>>,
        keys_only: bool,
        length_local: Option<u32>,
        runtime_name: Option<String>,
    },
    StaticArrayEntries {
        values: Vec<Option<Expression>>,
        length_local: Option<u32>,
        runtime_name: Option<String>,
    },
    StaticMapEntries {
        values: Vec<Option<Expression>>,
        length_local: Option<u32>,
        key_runtime_name: Option<String>,
        value_runtime_name: Option<String>,
    },
    SimpleGenerator {
        is_async: bool,
        steps: Vec<SimpleGeneratorStep>,
        completion_effects: Vec<Statement>,
        completion_value: Expression,
    },
    AsyncYieldDelegateGenerator {
        plan: AsyncYieldDelegateGeneratorPlan,
        delegate_iterator_name: String,
        delegate_next_name: String,
        delegate_completion_name: String,
        uses_async_iterator_method: Option<bool>,
        snapshot_bindings: Option<HashMap<String, Expression>>,
    },
    TypedArrayView {
        name: String,
    },
    DirectArguments {
        tracked_prefix_len: u32,
    },
}

#[derive(Clone)]
pub(in crate::backend::direct_wasm) struct ArrayIteratorBinding {
    pub(in crate::backend::direct_wasm) source: IteratorSourceKind,
    pub(in crate::backend::direct_wasm) index_local: u32,
    pub(in crate::backend::direct_wasm) static_index: Option<usize>,
}

#[derive(Clone)]
pub(in crate::backend::direct_wasm) struct CachedIteratorNextMethodBinding {
    pub(in crate::backend::direct_wasm) function_binding: LocalFunctionBinding,
    pub(in crate::backend::direct_wasm) this_expression: Expression,
    pub(in crate::backend::direct_wasm) capture_slots:
        Option<std::collections::BTreeMap<String, String>>,
}

#[derive(Clone)]
pub(in crate::backend::direct_wasm) struct IteratorStepEntryArrayBinding {
    pub(in crate::backend::direct_wasm) index_local: u32,
    pub(in crate::backend::direct_wasm) value_local: u32,
}

#[derive(Clone)]
pub(in crate::backend::direct_wasm) enum IteratorStepBinding {
    Runtime {
        done_local: u32,
        value_local: u32,
        function_binding: Option<LocalFunctionBinding>,
        static_done: Option<bool>,
        static_value: Option<Expression>,
        value_candidates: Vec<Expression>,
        entry_array: Option<IteratorStepEntryArrayBinding>,
    },
}
