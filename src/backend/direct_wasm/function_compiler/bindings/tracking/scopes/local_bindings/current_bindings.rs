use super::*;

impl<'a> FunctionCompiler<'a> {
    fn capture_binding_function_references_nested_function(
        &self,
        function: &FunctionDeclaration,
        nested_function_name: &str,
    ) -> bool {
        if collect_referenced_binding_names_from_statements(&function.body)
            .contains(nested_function_name)
        {
            return true;
        }

        function.params.iter().any(|parameter| {
            parameter.default.as_ref().is_some_and(|default| {
                let mut referenced = HashSet::new();
                collect_referenced_binding_names_from_expression(default, &mut referenced);
                referenced.contains(nested_function_name)
            })
        })
    }

    fn enclosing_function_name_for_capture_binding(&self, function_name: &str) -> Option<String> {
        self.user_functions()
            .into_iter()
            .filter(|candidate| candidate.name != function_name)
            .find(|candidate| {
                self.resolve_registered_function_declaration(&candidate.name)
                    .is_some_and(|function| {
                        self.capture_binding_function_references_nested_function(
                            function,
                            function_name,
                        )
                    })
            })
            .map(|candidate| candidate.name.clone())
    }

    fn function_declares_immutable_local_binding_in_statements(
        statements: &[Statement],
        target_name: &str,
    ) -> bool {
        statements.iter().any(|statement| match statement {
            Statement::Declaration { body }
            | Statement::Block { body }
            | Statement::Labeled { body, .. }
            | Statement::With { body, .. } => {
                Self::function_declares_immutable_local_binding_in_statements(body, target_name)
            }
            Statement::If {
                then_branch,
                else_branch,
                ..
            } => {
                Self::function_declares_immutable_local_binding_in_statements(
                    then_branch,
                    target_name,
                ) || Self::function_declares_immutable_local_binding_in_statements(
                    else_branch,
                    target_name,
                )
            }
            Statement::Try {
                body,
                catch_setup,
                catch_body,
                ..
            } => {
                Self::function_declares_immutable_local_binding_in_statements(body, target_name)
                    || Self::function_declares_immutable_local_binding_in_statements(
                        catch_setup,
                        target_name,
                    )
                    || Self::function_declares_immutable_local_binding_in_statements(
                        catch_body,
                        target_name,
                    )
            }
            Statement::Switch { cases, .. } => cases.iter().any(|case| {
                Self::function_declares_immutable_local_binding_in_statements(
                    &case.body,
                    target_name,
                )
            }),
            Statement::For { init, body, .. } => {
                Self::function_declares_immutable_local_binding_in_statements(init, target_name)
                    || Self::function_declares_immutable_local_binding_in_statements(
                        body,
                        target_name,
                    )
            }
            Statement::While { body, .. } | Statement::DoWhile { body, .. } => {
                Self::function_declares_immutable_local_binding_in_statements(body, target_name)
            }
            Statement::Let { name, mutable, .. } => {
                !*mutable && scoped_binding_source_name(name).unwrap_or(name) == target_name
            }
            _ => false,
        })
    }

    fn user_function_capture_binding_is_immutable_in_function(
        &self,
        function_name: &str,
        source_name: &str,
        visited: &mut HashSet<(String, String)>,
    ) -> bool {
        if !visited.insert((function_name.to_string(), source_name.to_string())) {
            return false;
        }

        let Some(enclosing_name) = self.enclosing_function_name_for_capture_binding(function_name)
        else {
            return false;
        };
        let Some(enclosing_function) =
            self.resolve_registered_function_declaration(&enclosing_name)
        else {
            return false;
        };

        if Self::function_declares_immutable_local_binding_in_statements(
            &enclosing_function.body,
            source_name,
        ) {
            return true;
        }

        self.user_function_capture_bindings(&enclosing_name)
            .is_some_and(|bindings| bindings.contains_key(source_name))
            && self.user_function_capture_binding_is_immutable_in_function(
                &enclosing_name,
                source_name,
                visited,
            )
    }

    pub(in crate::backend::direct_wasm) fn hidden_implicit_global_binding(
        &self,
        hidden_name: &str,
    ) -> Option<ImplicitGlobalBinding> {
        self.backend.implicit_global_binding(hidden_name)
    }

    pub(in crate::backend::direct_wasm) fn resolve_global_binding_index(
        &self,
        name: &str,
    ) -> Option<u32> {
        self.backend.resolve_global_binding_index(name)
    }

    pub(in crate::backend::direct_wasm) fn resolve_eval_local_function_hidden_name(
        &self,
        name: &str,
    ) -> Option<String> {
        let current_function_name = self.current_function_name()?;
        let bindings = self.eval_local_function_bindings(current_function_name)?;
        if let Some(hidden_name) = bindings.get(name) {
            return Some(hidden_name.clone());
        }

        let renamed_prefix = format!("__ayy_scope${name}$");
        let mut resolved: Option<(u32, String)> = None;
        for (candidate_name, hidden_name) in bindings {
            if !candidate_name.starts_with(&renamed_prefix) {
                continue;
            }
            let Some((_, scope_id)) = candidate_name.rsplit_once('$') else {
                continue;
            };
            let Ok(scope_id) = scope_id.parse::<u32>() else {
                continue;
            };
            if resolved
                .as_ref()
                .is_none_or(|(best_scope_id, _)| scope_id > *best_scope_id)
            {
                resolved = Some((scope_id, hidden_name.clone()));
            }
        }

        resolved.map(|(_, hidden_name)| hidden_name)
    }

    pub(in crate::backend::direct_wasm) fn resolve_user_function_capture_hidden_name(
        &self,
        name: &str,
    ) -> Option<String> {
        let current_function_name = self.current_function_name()?;
        let bindings = self.user_function_capture_bindings(current_function_name)?;
        if let Some(hidden_name) = bindings.get(name) {
            return Some(hidden_name.clone());
        }

        let source_name = scoped_binding_source_name(name);
        if let Some(source_name) = source_name
            && let Some(hidden_name) = bindings.get(source_name)
        {
            return Some(hidden_name.clone());
        }

        bindings.iter().find_map(|(capture_name, hidden_name)| {
            self.resolve_registered_function_declaration(capture_name)
                .and_then(|function| function.self_binding.as_deref())
                .filter(|self_binding| {
                    *self_binding == name
                        || source_name.is_some_and(|source_name| *self_binding == source_name)
                })
                .map(|_| hidden_name.clone())
        })
    }

    pub(in crate::backend::direct_wasm) fn resolve_current_local_binding(
        &self,
        name: &str,
    ) -> Option<(String, u32)> {
        fn resolve_current_local_binding_exact(
            locals: &HashMap<String, u32>,
            active_scoped_lexical_bindings: &HashMap<String, Vec<String>>,
            name: &str,
        ) -> Option<(String, u32)> {
            if let Some(active_name) = active_scoped_lexical_bindings
                .get(name)
                .and_then(|bindings| bindings.last())
                .cloned()
            {
                if let Some(local_index) = locals.get(&active_name).copied() {
                    return Some((active_name, local_index));
                }
            }

            if let Some(local_index) = locals.get(name).copied() {
                return Some((name.to_string(), local_index));
            }
            None
        }

        if let Some(resolved) = resolve_current_local_binding_exact(
            &self.state.runtime.locals.bindings,
            &self
                .state
                .emission
                .lexical_scopes
                .active_scoped_lexical_bindings,
            name,
        ) {
            return Some(resolved);
        }
        if let Some(source_name) = scoped_binding_source_name(name) {
            return resolve_current_local_binding_exact(
                &self.state.runtime.locals.bindings,
                &self
                    .state
                    .emission
                    .lexical_scopes
                    .active_scoped_lexical_bindings,
                source_name,
            );
        }

        None
    }

    pub(in crate::backend::direct_wasm) fn emit_eval_lexical_binding_read(
        &mut self,
        name: &str,
    ) -> DirectResult<bool> {
        let Some(initialized_local) = self
            .state
            .speculation
            .static_semantics
            .eval_lexical_initialized_locals
            .get(name)
            .copied()
        else {
            return Ok(false);
        };
        let local_index = self
            .state
            .runtime
            .locals
            .get(name)
            .copied()
            .expect("tracked eval lexical binding must have a local slot");
        self.push_local_get(initialized_local);
        self.state.emission.output.instructions.push(0x04);
        self.state.emission.output.instructions.push(I32_TYPE);
        self.push_control_frame();
        self.push_local_get(local_index);
        self.state.emission.output.instructions.push(0x05);
        self.emit_named_error_throw("ReferenceError")?;
        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();
        Ok(true)
    }

    pub(in crate::backend::direct_wasm) fn local_lexical_initialized_local(
        &self,
        name: &str,
    ) -> Option<u32> {
        self.state
            .speculation
            .static_semantics
            .local_lexical_initialized_locals
            .get(name)
            .copied()
            .or_else(|| {
                scoped_binding_source_name(name).and_then(|source_name| {
                    self.state
                        .speculation
                        .static_semantics
                        .local_lexical_initialized_locals
                        .get(source_name)
                        .copied()
                })
            })
    }

    pub(in crate::backend::direct_wasm) fn local_binding_is_immutable(&self, name: &str) -> bool {
        self.state
            .speculation
            .static_semantics
            .immutable_local_bindings
            .contains(name)
            || scoped_binding_source_name(name).is_some_and(|source_name| {
                self.state
                    .speculation
                    .static_semantics
                    .immutable_local_bindings
                    .contains(source_name)
            })
    }

    pub(in crate::backend::direct_wasm) fn user_function_capture_binding_is_immutable(
        &self,
        name: &str,
    ) -> bool {
        let Some(current_function_name) = self.current_function_name() else {
            return false;
        };
        let source_name = scoped_binding_source_name(name).unwrap_or(name);
        let mut visited = HashSet::new();
        self.user_function_capture_binding_is_immutable_in_function(
            current_function_name,
            source_name,
            &mut visited,
        )
    }
}
