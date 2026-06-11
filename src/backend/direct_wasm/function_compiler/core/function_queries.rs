use super::*;

thread_local! {
    /// (active query depth, guard serial of the outermost active query).
    static RUNTIME_PUBLIC_THIS_RESOLUTION_QUERY: std::cell::Cell<(usize, u64)> =
        const { std::cell::Cell::new((0, 0)) };
    /// (static-state generation, function name -> requires-runtime-public-this).
    /// `current_function_requires_runtime_public_this_resolution` runs a
    /// transitive private-member reachability scan over the current
    /// function's body and callees; member materialization consults it for
    /// every member expression, so pathological inputs re-run the same scan
    /// millions of times at an unchanged generation.
    static RUNTIME_PUBLIC_THIS_RESOLUTION_CACHE: std::cell::RefCell<(u64, HashMap<String, bool>)> =
        std::cell::RefCell::new((0, HashMap::new()));
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

    /// Registration-ordered function names; unlike `user_functions` this does
    /// not deep-clone every function body, so name-only scans stay cheap.
    pub(in crate::backend::direct_wasm) fn user_function_names(&self) -> &[String] {
        self.prepared_program.ordered_user_function_names()
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

    pub(in crate::backend::direct_wasm) fn current_rest_parameter_binding(
        &self,
    ) -> Option<(usize, String)> {
        self.current_user_function_declaration()?
            .params
            .iter()
            .enumerate()
            .find_map(|(index, parameter)| parameter.rest.then(|| (index, parameter.name.clone())))
    }

    pub(in crate::backend::direct_wasm) fn is_current_rest_parameter_binding_name(
        &self,
        name: &str,
    ) -> bool {
        let source_name = scoped_binding_source_name(name).unwrap_or(name);
        self.current_user_function_declaration()
            .is_some_and(|declaration| {
                declaration.params.iter().any(|parameter| {
                    if !parameter.rest {
                        return false;
                    }
                    let parameter_source_name =
                        scoped_binding_source_name(&parameter.name).unwrap_or(&parameter.name);
                    parameter.name == name
                        || parameter.name == source_name
                        || parameter_source_name == name
                        || parameter_source_name == source_name
                })
            })
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
        use crate::backend::direct_wasm::memo;
        // Re-entry within an active query is a deterministic self-cycle of
        // the outermost query's own guard: note the conflict against that
        // guard's serial (not a blanket block) so memo windows opened by the
        // outermost query itself remain storable.
        let reentered_serial = RUNTIME_PUBLIC_THIS_RESOLUTION_QUERY.with(|query| {
            let (depth, serial) = query.get();
            (depth > 0).then_some(serial)
        });
        if let Some(serial) = reentered_serial {
            memo::note_resolution_guard_block_conflict(serial);
            return false;
        }
        let generation = memo::static_state_generation();
        let cached = self.current_function_name().and_then(|function_name| {
            RUNTIME_PUBLIC_THIS_RESOLUTION_CACHE.with(|cache| {
                let mut cache = cache.borrow_mut();
                if cache.0 != generation {
                    cache.0 = generation;
                    cache.1.clear();
                }
                cache.1.get(function_name).copied()
            })
        });
        if let Some(result) = cached {
            return result;
        }
        let token = memo::MemoStoreToken::capture();
        let serial = memo::next_guard_serial();
        RUNTIME_PUBLIC_THIS_RESOLUTION_QUERY.with(|query| query.set((1, serial)));
        let _memo_guard = memo::ResolutionGuardScope::enter_class(21);
        let result = self.current_user_function().is_some_and(|user_function| {
            self.user_function_mentions_private_member_access(user_function)
        });
        RUNTIME_PUBLIC_THIS_RESOLUTION_QUERY.with(|query| query.set((0, 0)));
        if token.is_clean()
            && let Some(function_name) = self.current_function_name()
        {
            RUNTIME_PUBLIC_THIS_RESOLUTION_CACHE.with(|cache| {
                let mut cache = cache.borrow_mut();
                if cache.0 == generation {
                    cache.1.insert(function_name.to_string(), result);
                }
            });
        }
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
