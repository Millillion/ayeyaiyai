use super::*;

thread_local! {
    static RUNTIME_PUBLIC_THIS_RESOLUTION_QUERY_DEPTH: std::cell::Cell<usize> =
        const { std::cell::Cell::new(0) };
}

impl<'a> FunctionCompiler<'a> {
    pub(in crate::backend::direct_wasm) fn user_function(
        &self,
        function_name: &str,
    ) -> Option<&UserFunction> {
        self.prepared_program.user_function(function_name)
    }

    pub(in crate::backend::direct_wasm) fn contains_user_function(&self, name: &str) -> bool {
        self.prepared_program.contains_user_function(name)
    }

    pub(in crate::backend::direct_wasm) fn user_functions(&self) -> Vec<UserFunction> {
        self.prepared_program.ordered_user_functions()
    }

    pub(in crate::backend::direct_wasm) fn resolve_user_function_by_binding_name(
        &self,
        name: &str,
    ) -> Option<&UserFunction> {
        self.prepared_program
            .resolve_user_function_by_binding_name(name)
    }

    pub(in crate::backend::direct_wasm) fn current_user_function(&self) -> Option<&UserFunction> {
        self.state
            .speculation
            .execution_context
            .current_user_function
            .as_ref()
    }

    pub(in crate::backend::direct_wasm) fn current_function_name(&self) -> Option<&str> {
        self.state
            .speculation
            .execution_context
            .current_user_function_name
            .as_deref()
    }

    pub(in crate::backend::direct_wasm) fn has_current_user_function(&self) -> bool {
        self.state
            .speculation
            .execution_context
            .current_user_function_name
            .is_some()
    }

    pub(in crate::backend::direct_wasm) fn current_user_function_declaration(
        &self,
    ) -> Option<&FunctionDeclaration> {
        self.state
            .speculation
            .execution_context
            .current_function_declaration
            .as_ref()
    }

    pub(in crate::backend::direct_wasm) fn assignment_targets_immutable_class_binding(
        &self,
        name: &str,
    ) -> bool {
        let Some(declaration) = self.current_user_function_declaration() else {
            return false;
        };
        let source_name = scoped_binding_source_name(name).unwrap_or(name);
        declaration.immutable_class_bindings.iter().any(|binding| {
            let binding_source_name = scoped_binding_source_name(binding).unwrap_or(binding);
            binding == name
                || binding == source_name
                || binding_source_name == name
                || binding_source_name == source_name
        })
    }

    pub(in crate::backend::direct_wasm) fn user_function_runtime_value(
        &self,
        function_name: &str,
    ) -> Option<i32> {
        self.user_function(function_name)
            .map(user_function_runtime_value)
    }

    pub(in crate::backend::direct_wasm) fn prepared_function_declaration(
        &self,
        function_name: &str,
    ) -> Option<&FunctionDeclaration> {
        self.prepared_program
            .user_function_declaration(function_name)
    }

    pub(in crate::backend::direct_wasm) fn user_function_capture_bindings(
        &self,
        function_name: &str,
    ) -> Option<HashMap<String, String>> {
        let mut bindings = self
            .prepared_program
            .user_function_capture_bindings(function_name)
            .cloned()
            .unwrap_or_default();
        if let Some(live_bindings) = self
            .backend
            .function_registry
            .analysis
            .user_function_capture_bindings(function_name)
        {
            bindings.extend(live_bindings.clone());
        }
        (!bindings.is_empty()).then_some(bindings)
    }

    pub(in crate::backend::direct_wasm) fn eval_local_function_bindings(
        &self,
        function_name: &str,
    ) -> Option<HashMap<String, String>> {
        self.prepared_program
            .eval_local_function_bindings(function_name)
            .cloned()
    }

    pub(in crate::backend::direct_wasm) fn current_function_is_derived_constructor(&self) -> bool {
        self.state.speculation.execution_context.derived_constructor
    }

    pub(in crate::backend::direct_wasm) fn user_function_is_derived_constructor(
        &self,
        user_function: &UserFunction,
    ) -> bool {
        self.resolve_registered_function_declaration(&user_function.name)
            .is_some_and(|function| function.derived_constructor)
    }

    pub(in crate::backend::direct_wasm) fn current_function_requires_runtime_public_this_resolution(
        &self,
    ) -> bool {
        let reentered = RUNTIME_PUBLIC_THIS_RESOLUTION_QUERY_DEPTH.with(|depth| {
            let current = depth.get();
            depth.set(current + 1);
            current > 0
        });
        if reentered {
            RUNTIME_PUBLIC_THIS_RESOLUTION_QUERY_DEPTH
                .with(|depth| depth.set(depth.get().saturating_sub(1)));
            return false;
        }
        let result = self.current_user_function().is_some_and(|user_function| {
            self.user_function_mentions_private_member_access(user_function)
        });
        RUNTIME_PUBLIC_THIS_RESOLUTION_QUERY_DEPTH
            .with(|depth| depth.set(depth.get().saturating_sub(1)));
        result
    }

    pub(in crate::backend::direct_wasm) fn expression_is_current_this_reference(
        &self,
        expression: &Expression,
    ) -> bool {
        matches!(expression, Expression::This)
            || self
                .resolve_bound_alias_expression(expression)
                .is_some_and(|resolved| {
                    !static_expression_matches(&resolved, expression)
                        && matches!(resolved, Expression::This)
                })
    }
}
