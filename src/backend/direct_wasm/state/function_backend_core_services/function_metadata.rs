use super::super::*;

fn user_function_declares_source_binding(user_function: &UserFunction, source_name: &str) -> bool {
    user_function.scope_bindings.contains(source_name)
        || user_function.params.iter().any(|param| {
            let param_source_name = scoped_binding_source_name(param).unwrap_or(param);
            param == source_name || param_source_name == source_name
        })
}

impl<'a> FunctionCompilerBackend<'a> {
    pub(in crate::backend::direct_wasm) fn collect_user_function_assigned_nonlocal_bindings(
        &self,
        user_function: &UserFunction,
    ) -> HashSet<String> {
        let Some(function) = self
            .function_registry
            .registered_function(&user_function.name)
        else {
            return HashSet::new();
        };
        let mut names = HashSet::new();
        for statement in &function.body {
            collect_assigned_binding_names_from_statement(statement, &mut names);
        }
        names.retain(|name| {
            let source_name = scoped_binding_source_name(name).unwrap_or(name);
            let targets_immutable_class_binding =
                function.immutable_class_bindings.iter().any(|binding| {
                    let binding_source_name =
                        scoped_binding_source_name(binding).unwrap_or(binding);
                    binding == name
                        || binding == source_name
                        || binding_source_name == name
                        || binding_source_name == source_name
                });
            let targets_immutable_global_lexical_binding = self
                .global_semantics
                .global_names()
                .lexical_binding(name)
                .is_some_and(|binding| !binding.mutable);
            !user_function_declares_source_binding(user_function, source_name)
                && !targets_immutable_class_binding
                && !targets_immutable_global_lexical_binding
        });
        names
    }
}
