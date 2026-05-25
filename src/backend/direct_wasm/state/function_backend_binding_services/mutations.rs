use super::super::*;

impl<'a> FunctionCompilerBackend<'a> {
    fn next_available_global_index(&self) -> u32 {
        let local_names_next_index = self.global_semantics.names.next_allocated_global_index();
        let shared_names_next_index = self
            .shared_global_semantics
            .names
            .next_allocated_global_index();
        let local_values_next_index = self
            .global_semantics
            .values
            .max_runtime_prototype_global_index()
            .map(|index| index + 1)
            .unwrap_or(local_names_next_index);
        let shared_values_next_index = self
            .shared_global_semantics
            .values
            .max_runtime_prototype_global_index()
            .map(|index| index + 1)
            .unwrap_or(shared_names_next_index);

        local_names_next_index
            .max(shared_names_next_index)
            .max(local_values_next_index)
            .max(shared_values_next_index)
    }

    pub(in crate::backend::direct_wasm) fn ensure_global_lexical_binding(
        &mut self,
        name: &str,
        mutable: bool,
    ) {
        let mut next_global_index = self.next_available_global_index();
        self.shared_global_semantics
            .ensure_global_binding_index(name, &mut next_global_index);
        if let Some(global_index) = self
            .shared_global_semantics
            .global_names()
            .binding_index(name)
        {
            self.global_semantics
                .names
                .bindings
                .insert(name.to_string(), global_index);
        }

        let mut next_global_index = self.next_available_global_index();
        self.shared_global_semantics.mark_global_lexical_binding(
            name,
            mutable,
            &mut next_global_index,
        );
        if let Some(binding) = self
            .shared_global_semantics
            .global_names()
            .lexical_binding(name)
        {
            self.global_semantics
                .names
                .lexical_bindings
                .insert(name.to_string(), binding);
        }
    }

    pub(in crate::backend::direct_wasm) fn set_global_string_binding(
        &mut self,
        name: &str,
        text: String,
    ) {
        self.set_global_expression_binding(name, Expression::String(text));
        self.set_global_binding_kind(name, StaticValueKind::String);
    }

    pub(in crate::backend::direct_wasm) fn set_global_number_binding(
        &mut self,
        name: &str,
        number: f64,
    ) {
        self.set_global_expression_binding(name, Expression::Number(number));
        self.set_global_binding_kind(name, StaticValueKind::Number);
    }

    pub(in crate::backend::direct_wasm) fn set_global_binding_kind(
        &mut self,
        name: &str,
        kind: StaticValueKind,
    ) {
        self.global_semantics.set_global_binding_kind(name, kind);
    }

    pub(in crate::backend::direct_wasm) fn set_global_function_binding(
        &mut self,
        name: &str,
        binding: LocalFunctionBinding,
    ) {
        self.global_semantics
            .set_global_function_binding(name, binding);
        self.set_global_binding_kind(name, StaticValueKind::Function);
    }

    pub(in crate::backend::direct_wasm) fn set_global_user_function_reference(
        &mut self,
        name: &str,
    ) {
        self.set_global_binding_kind(name, StaticValueKind::Function);
        self.set_global_expression_binding(name, Expression::Identifier(name.to_string()));
        self.set_global_function_binding(name, LocalFunctionBinding::User(name.to_string()));
    }

    pub(in crate::backend::direct_wasm) fn sync_global_function_binding(
        &mut self,
        name: &str,
        binding: Option<LocalFunctionBinding>,
    ) {
        if let Some(binding) = binding {
            self.set_global_function_binding(name, binding);
        } else {
            self.global_semantics.clear_global_function_binding(name);
        }
    }
}
