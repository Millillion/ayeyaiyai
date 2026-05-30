use super::*;

#[derive(Clone, Default)]
pub(in crate::backend::direct_wasm) struct GlobalNameService {
    pub(in crate::backend::direct_wasm) bindings: HashMap<String, u32>,
    pub(in crate::backend::direct_wasm) lexical_bindings: HashMap<String, LexicalGlobalBinding>,
    pub(in crate::backend::direct_wasm) kinds: HashMap<String, StaticValueKind>,
    pub(in crate::backend::direct_wasm) implicit_bindings: HashMap<String, ImplicitGlobalBinding>,
}

impl GlobalNameService {
    pub(in crate::backend::direct_wasm) fn reset_for_program(&mut self) {
        self.bindings.clear();
        self.lexical_bindings.clear();
        self.kinds.clear();
        self.implicit_bindings.clear();
    }

    pub(in crate::backend::direct_wasm) fn ensure_binding_index(
        &mut self,
        name: &str,
        next_global_index: &mut u32,
    ) {
        if self.has_binding(name) {
            return;
        }
        self.bindings.insert(name.to_string(), *next_global_index);
        *next_global_index += 1;
    }

    pub(in crate::backend::direct_wasm) fn mark_lexical_binding(
        &mut self,
        name: &str,
        mutable: bool,
        next_global_index: &mut u32,
    ) {
        if let Some(existing) = self.lexical_bindings.get(name).copied() {
            debug_assert_eq!(existing.mutable, mutable);
            return;
        }

        let binding = LexicalGlobalBinding {
            initialized_index: *next_global_index,
            mutable,
        };
        *next_global_index += 1;
        self.lexical_bindings.insert(name.to_string(), binding);
    }

    pub(in crate::backend::direct_wasm) fn has_binding(&self, name: &str) -> bool {
        self.bindings.contains_key(name)
    }

    pub(in crate::backend::direct_wasm) fn binding_index(&self, name: &str) -> Option<u32> {
        self.bindings.get(name).copied()
    }

    pub(in crate::backend::direct_wasm) fn resolve_binding_index(&self, name: &str) -> Option<u32> {
        if let Some(global_index) = self.binding_index(name) {
            return Some(global_index);
        }
        if let Some(source_name) = scoped_binding_source_name(name)
            && let Some(global_index) = self.binding_index(source_name)
        {
            return Some(global_index);
        }
        let mut scoped_matches =
            self.bindings
                .iter()
                .filter_map(|(binding_name, &global_index)| {
                    (scoped_binding_source_name(binding_name) == Some(name)).then_some(global_index)
                });
        let scoped_match = scoped_matches.next()?;
        scoped_matches.next().is_none().then_some(scoped_match)
    }

    pub(in crate::backend::direct_wasm) fn lexical_binding(
        &self,
        name: &str,
    ) -> Option<LexicalGlobalBinding> {
        if let Some(binding) = self.lexical_bindings.get(name).copied() {
            return Some(binding);
        }
        if let Some(source_name) = scoped_binding_source_name(name)
            && let Some(binding) = self.lexical_bindings.get(source_name).copied()
        {
            return Some(binding);
        }
        let mut scoped_matches =
            self.lexical_bindings
                .iter()
                .filter_map(|(binding_name, binding)| {
                    (scoped_binding_source_name(binding_name) == Some(name)).then_some(*binding)
                });
        let scoped_match = scoped_matches.next()?;
        scoped_matches.next().is_none().then_some(scoped_match)
    }

    pub(in crate::backend::direct_wasm) fn has_lexical_binding(&self, name: &str) -> bool {
        self.lexical_binding(name).is_some()
    }

    pub(in crate::backend::direct_wasm) fn has_exact_lexical_binding(&self, name: &str) -> bool {
        self.lexical_bindings.contains_key(name)
    }

    pub(in crate::backend::direct_wasm) fn has_implicit_binding(&self, name: &str) -> bool {
        self.implicit_bindings.contains_key(name)
    }

    pub(in crate::backend::direct_wasm) fn implicit_binding(
        &self,
        name: &str,
    ) -> Option<ImplicitGlobalBinding> {
        self.implicit_bindings.get(name).copied()
    }

    pub(in crate::backend::direct_wasm) fn kind(&self, name: &str) -> Option<StaticValueKind> {
        self.kinds.get(name).copied()
    }

    pub(in crate::backend::direct_wasm) fn set_kind(&mut self, name: &str, kind: StaticValueKind) {
        self.kinds.insert(name.to_string(), kind);
    }

    pub(in crate::backend::direct_wasm) fn clear_kind(&mut self, name: &str) {
        self.kinds.remove(name);
    }

    pub(in crate::backend::direct_wasm) fn binding_count(&self) -> u32 {
        self.bindings.len() as u32
    }

    pub(in crate::backend::direct_wasm) fn implicit_binding_count(&self) -> u32 {
        self.implicit_bindings.len() as u32
    }

    pub(in crate::backend::direct_wasm) fn lexical_binding_runtime_slot_count(&self) -> u32 {
        self.lexical_bindings.len() as u32
    }

    pub(in crate::backend::direct_wasm) fn next_allocated_global_index(&self) -> u32 {
        self.bindings
            .values()
            .copied()
            .chain(
                self.implicit_bindings
                    .values()
                    .flat_map(|binding| [binding.value_index, binding.present_index]),
            )
            .chain(
                self.lexical_bindings
                    .values()
                    .map(|binding| binding.initialized_index),
            )
            .max()
            .map(|index| index + 1)
            .unwrap_or(NEXT_PRIVATE_BRAND_GLOBAL_INDEX + 1)
    }

    pub(in crate::backend::direct_wasm) fn ensure_implicit_binding(
        &mut self,
        name: &str,
    ) -> ImplicitGlobalBinding {
        if let Some(binding) = self.implicit_binding(name) {
            return binding;
        }

        let next_global_index = self.next_allocated_global_index();

        let binding = ImplicitGlobalBinding {
            value_index: next_global_index,
            present_index: next_global_index + 1,
        };
        self.implicit_bindings.insert(name.to_string(), binding);
        binding
    }

    pub(in crate::backend::direct_wasm) fn sync_implicit_binding(
        &mut self,
        name: &str,
        binding: ImplicitGlobalBinding,
    ) {
        if let Some(existing) = self.implicit_binding(name) {
            debug_assert!(
                existing.value_index == binding.value_index
                    && existing.present_index == binding.present_index
            );
            return;
        }
        self.implicit_bindings.insert(name.to_string(), binding);
    }

    pub(in crate::backend::direct_wasm) fn sync_implicit_bindings_from(
        &mut self,
        other: &GlobalNameService,
    ) {
        for (name, binding) in &other.implicit_bindings {
            self.sync_implicit_binding(name, *binding);
        }
    }
}
