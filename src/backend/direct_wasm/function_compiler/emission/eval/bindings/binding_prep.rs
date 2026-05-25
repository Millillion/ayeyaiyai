use super::*;

impl<'a> FunctionCompiler<'a> {
    fn allocate_eval_lexical_hidden_local(&mut self, source_name: &str) -> String {
        let next_local_index = self.state.runtime.locals.next_local_index;
        let name = format!("__ayy_eval_lex${source_name}${next_local_index}");
        self.state
            .runtime
            .locals
            .insert(name.clone(), next_local_index);
        self.state
            .speculation
            .static_semantics
            .set_local_kind(&name, StaticValueKind::Unknown);
        self.state.runtime.locals.next_local_index += 1;
        name
    }

    pub(in crate::backend::direct_wasm) fn register_bindings_skipping_eval_local_function_declarations(
        &mut self,
        statements: &[Statement],
        eval_local_function_declarations: &HashMap<String, String>,
    ) -> DirectResult<()> {
        for statement in statements {
            if is_eval_local_function_declaration_statement(
                statement,
                eval_local_function_declarations,
            ) {
                continue;
            }

            match statement {
                Statement::Declaration { body }
                | Statement::Block { body }
                | Statement::Labeled { body, .. } => self
                    .register_bindings_skipping_eval_local_function_declarations(
                        body,
                        eval_local_function_declarations,
                    )?,
                Statement::Var { name, .. } | Statement::Let { name, .. } => {
                    if self.state.speculation.execution_context.top_level_function
                        && self.backend.global_has_binding(name)
                    {
                        continue;
                    }
                    if self.state.runtime.locals.bindings.contains_key(name) {
                        continue;
                    }
                    let next_local_index = self.state.runtime.locals.next_local_index;
                    self.state
                        .runtime
                        .locals
                        .insert(name.clone(), next_local_index);
                    self.state.runtime.locals.next_local_index += 1;
                }
                Statement::If {
                    then_branch,
                    else_branch,
                    ..
                } => {
                    self.register_bindings_skipping_eval_local_function_declarations(
                        then_branch,
                        eval_local_function_declarations,
                    )?;
                    self.register_bindings_skipping_eval_local_function_declarations(
                        else_branch,
                        eval_local_function_declarations,
                    )?;
                }
                Statement::With { body, .. } => {
                    self.register_bindings_skipping_eval_local_function_declarations(
                        body,
                        eval_local_function_declarations,
                    )?;
                }
                Statement::While { body, .. } | Statement::DoWhile { body, .. } => self
                    .register_bindings_skipping_eval_local_function_declarations(
                        body,
                        eval_local_function_declarations,
                    )?,
                Statement::Try {
                    body,
                    catch_binding,
                    catch_setup,
                    catch_body,
                    ..
                } => {
                    self.register_bindings_skipping_eval_local_function_declarations(
                        body,
                        eval_local_function_declarations,
                    )?;
                    if let Some(catch_binding) = catch_binding {
                        if !self
                            .state
                            .runtime
                            .locals
                            .bindings
                            .contains_key(catch_binding)
                        {
                            let next_local_index = self.state.runtime.locals.next_local_index;
                            self.state
                                .runtime
                                .locals
                                .insert(catch_binding.clone(), next_local_index);
                            self.state
                                .speculation
                                .static_semantics
                                .set_local_kind(catch_binding, StaticValueKind::Object);
                            self.state.runtime.locals.next_local_index += 1;
                        }
                    }
                    self.register_bindings_skipping_eval_local_function_declarations(
                        catch_setup,
                        eval_local_function_declarations,
                    )?;
                    self.register_bindings_skipping_eval_local_function_declarations(
                        catch_body,
                        eval_local_function_declarations,
                    )?;
                }
                Statement::For {
                    init,
                    per_iteration_bindings,
                    body,
                    ..
                } => {
                    self.register_bindings_skipping_eval_local_function_declarations(
                        init,
                        eval_local_function_declarations,
                    )?;
                    for binding in per_iteration_bindings {
                        if self.state.runtime.locals.bindings.contains_key(binding) {
                            continue;
                        }
                        let next_local_index = self.state.runtime.locals.next_local_index;
                        self.state
                            .runtime
                            .locals
                            .insert(binding.clone(), next_local_index);
                        self.state
                            .speculation
                            .static_semantics
                            .set_local_kind(binding, StaticValueKind::Unknown);
                        self.state.runtime.locals.next_local_index += 1;
                    }
                    self.register_bindings_skipping_eval_local_function_declarations(
                        body,
                        eval_local_function_declarations,
                    )?;
                }
                Statement::Switch {
                    bindings, cases, ..
                } => {
                    for binding in bindings {
                        if self.state.runtime.locals.bindings.contains_key(binding) {
                            continue;
                        }
                        let next_local_index = self.state.runtime.locals.next_local_index;
                        self.state
                            .runtime
                            .locals
                            .insert(binding.clone(), next_local_index);
                        self.state
                            .speculation
                            .static_semantics
                            .set_local_kind(binding, StaticValueKind::Unknown);
                        self.state.runtime.locals.next_local_index += 1;
                    }
                    for case in cases {
                        self.register_bindings_skipping_eval_local_function_declarations(
                            &case.body,
                            eval_local_function_declarations,
                        )?;
                    }
                }
                Statement::Assign { .. }
                | Statement::Break { .. }
                | Statement::Continue { .. }
                | Statement::Expression(_)
                | Statement::Print { .. }
                | Statement::Return(_) => {}
                _ => {}
            }
        }

        Ok(())
    }

    pub(in crate::backend::direct_wasm) fn prepare_eval_lexical_bindings(
        &mut self,
        statements: &mut [Statement],
        eval_local_function_declarations: &HashMap<String, String>,
    ) -> DirectResult<()> {
        fn collect_eval_lexical_names(
            statements: &[Statement],
            eval_local_function_declarations: &HashMap<String, String>,
        ) -> Vec<String> {
            let mut names = Vec::new();
            for statement in statements {
                match statement {
                    Statement::Let { name, .. }
                        if !name.starts_with("__ayy_")
                            && !is_eval_local_function_declaration_statement(
                                statement,
                                eval_local_function_declarations,
                            ) =>
                    {
                        names.push(name.clone());
                    }
                    Statement::Declaration { body } => {
                        for statement in body {
                            if let Statement::Let { name, .. } = statement
                                && !name.starts_with("__ayy_")
                                && !is_eval_local_function_declaration_statement(
                                    statement,
                                    eval_local_function_declarations,
                                )
                            {
                                names.push(name.clone());
                            }
                        }
                    }
                    _ => {}
                }
            }
            names
        }

        let lexical_names =
            collect_eval_lexical_names(statements, eval_local_function_declarations);
        if lexical_names.is_empty() {
            return Ok(());
        }

        let mut renamed_bindings = HashMap::new();
        for name in lexical_names {
            if renamed_bindings.contains_key(&name) {
                continue;
            }
            let hidden_name = self.allocate_eval_lexical_hidden_local(&name);
            let initialized_local = self.allocate_temp_local();
            let hidden_local = self
                .state
                .runtime
                .locals
                .get(&hidden_name)
                .copied()
                .expect("fresh hidden eval lexical local must exist");
            self.push_i32_const(JS_UNDEFINED_TAG);
            self.push_local_set(hidden_local);
            self.push_i32_const(0);
            self.push_local_set(initialized_local);
            self.state
                .speculation
                .static_semantics
                .eval_lexical_initialized_locals
                .insert(hidden_name.clone(), initialized_local);
            renamed_bindings.insert(name, hidden_name);
        }

        for statement in statements {
            self.rewrite_eval_lexical_statement(statement, &renamed_bindings);
        }

        Ok(())
    }

    pub(in crate::backend::direct_wasm) fn prepare_eval_script_global_lexical_bindings(
        &mut self,
        statements: &[Statement],
        eval_local_function_declarations: &HashMap<String, String>,
    ) {
        let mut seen = HashSet::new();
        self.prepare_eval_script_global_lexical_bindings_in_statements(
            statements,
            eval_local_function_declarations,
            &mut seen,
        );
    }

    pub(in crate::backend::direct_wasm) fn eval_script_global_lexical_declaration_collides_with_existing_global(
        &self,
        statements: &[Statement],
        eval_local_function_declarations: &HashMap<String, String>,
    ) -> bool {
        let mut seen = HashSet::new();
        self.eval_script_global_lexical_declaration_collides_with_existing_global_in_statements(
            statements,
            eval_local_function_declarations,
            &mut seen,
        )
    }

    fn eval_script_global_lexical_declaration_collides_with_existing_global_in_statements(
        &self,
        statements: &[Statement],
        eval_local_function_declarations: &HashMap<String, String>,
        seen: &mut HashSet<String>,
    ) -> bool {
        for statement in statements {
            if let Statement::Declaration { body } | Statement::Labeled { body, .. } = statement {
                if self
                    .eval_script_global_lexical_declaration_collides_with_existing_global_in_statements(
                        body,
                        eval_local_function_declarations,
                        seen,
                    )
                {
                    return true;
                }
                continue;
            }

            let Statement::Let { name, .. } = statement else {
                continue;
            };
            if name.starts_with("__ayy_")
                || !seen.insert(name.clone())
                || is_eval_local_function_declaration_statement(
                    statement,
                    eval_local_function_declarations,
                )
            {
                continue;
            }
            if self.backend.global_has_lexical_binding(name) {
                return true;
            }
            if self
                .backend
                .global_property_descriptor(name)
                .is_some_and(|descriptor| !descriptor.configurable)
            {
                return true;
            }
        }

        false
    }

    fn prepare_eval_script_global_lexical_bindings_in_statements(
        &mut self,
        statements: &[Statement],
        eval_local_function_declarations: &HashMap<String, String>,
        seen: &mut HashSet<String>,
    ) {
        for statement in statements {
            if let Statement::Declaration { body } | Statement::Labeled { body, .. } = statement {
                self.prepare_eval_script_global_lexical_bindings_in_statements(
                    body,
                    eval_local_function_declarations,
                    seen,
                );
                continue;
            }

            let Statement::Let {
                name,
                mutable,
                value,
            } = statement
            else {
                continue;
            };
            if name.starts_with("__ayy_")
                || !seen.insert(name.clone())
                || is_eval_local_function_declaration_statement(
                    statement,
                    eval_local_function_declarations,
                )
            {
                continue;
            }
            self.backend.ensure_global_lexical_binding(name, *mutable);
            self.state
                .speculation
                .static_semantics
                .clear_local_function_binding(name);
            self.backend.sync_global_function_binding(name, None);
            self.backend
                .shared_global_semantics
                .clear_global_function_binding(name);
            self.backend.set_global_binding_kind(
                name,
                self.infer_value_kind(value)
                    .unwrap_or(StaticValueKind::Unknown),
            );
        }
    }
}
