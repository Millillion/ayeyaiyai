use super::*;

impl<'a> FunctionCompiler<'a> {
    pub(in crate::backend::direct_wasm) fn statement_unconditionally_transfers_control(
        statement: &Statement,
    ) -> bool {
        match statement {
            Statement::Break { .. }
            | Statement::Continue { .. }
            | Statement::Return(_)
            | Statement::Throw(_) => true,
            Statement::Block { body } | Statement::Declaration { body } => {
                Self::statement_list_unconditionally_transfers_control(body)
            }
            Statement::Labeled { body, .. } => {
                Self::statement_list_unconditionally_transfers_control(body)
            }
            Statement::If {
                then_branch,
                else_branch,
                ..
            } if !else_branch.is_empty() => {
                Self::statement_list_unconditionally_transfers_control(then_branch)
                    && Self::statement_list_unconditionally_transfers_control(else_branch)
            }
            _ => false,
        }
    }

    pub(in crate::backend::direct_wasm) fn statement_list_unconditionally_transfers_control(
        statements: &[Statement],
    ) -> bool {
        statements
            .iter()
            .any(Self::statement_unconditionally_transfers_control)
    }

    pub(in crate::backend::direct_wasm) fn emit_statements(
        &mut self,
        statements: &[Statement],
    ) -> DirectResult<()> {
        let trace = std::env::var_os("AYY_TRACE_FUNCTION_COMPILE").is_some();
        let mut index = 0;
        while let Some(statement) = statements.get(index) {
            if trace {
                eprintln!("function_compile=statement:{statement:?}");
            }
            let next_statement = statements.get(index + 1);
            if !self.try_emit_destructuring_default_iterator_close_statement(
                statement,
                next_statement,
            )? {
                self.emit_statement(statement)?;
            }
            if trace {
                eprintln!("function_compile=statement_done:{statement:?}");
            }
            if Self::statement_unconditionally_transfers_control(statement) {
                break;
            }
            index += 1;
        }
        Ok(())
    }

    pub(in crate::backend::direct_wasm) fn emit_statements_in_direct_lexical_scope(
        &mut self,
        statements: &[Statement],
    ) -> DirectResult<()> {
        self.with_active_eval_lexical_scope(
            collect_direct_eval_lexical_binding_names(statements),
            |compiler| compiler.emit_statements(statements),
        )
    }

    pub(in crate::backend::direct_wasm) fn with_active_eval_lexical_scope<T>(
        &mut self,
        names: Vec<String>,
        body: impl FnOnce(&mut Self) -> DirectResult<T>,
    ) -> DirectResult<T> {
        let source_names = names
            .iter()
            .map(|name| {
                scoped_binding_source_name(name)
                    .unwrap_or(name.as_str())
                    .to_string()
            })
            .collect::<HashSet<_>>();
        self.push_active_eval_lexical_scope(names);
        let result = body(self);
        self.pop_active_eval_lexical_scope();
        for source_name in source_names {
            if self
                .state
                .emission
                .lexical_scopes
                .active_eval_lexical_binding_counts
                .contains_key(&source_name)
                || self.resolve_current_local_binding(&source_name).is_some()
                || self.global_has_binding(&source_name)
                || self.backend.global_has_lexical_binding(&source_name)
                || self.global_has_implicit_binding(&source_name)
            {
                continue;
            }
            self.state.clear_local_static_binding_metadata(&source_name);
            self.backend
                .clear_global_static_binding_metadata(&source_name);
            self.backend
                .clear_shared_global_static_binding_metadata(&source_name);
        }
        result
    }

    pub(in crate::backend::direct_wasm) fn push_active_eval_lexical_scope(
        &mut self,
        names: Vec<String>,
    ) {
        self.state.push_active_eval_lexical_scope(names);
    }

    pub(in crate::backend::direct_wasm) fn pop_active_eval_lexical_scope(&mut self) {
        self.state.pop_active_eval_lexical_scope();
    }

    pub(in crate::backend::direct_wasm) fn emit_labeled_block(
        &mut self,
        labels: &[String],
        body: &[Statement],
    ) -> DirectResult<()> {
        self.state.emission.output.instructions.push(0x02);
        self.state
            .emission
            .output
            .instructions
            .push(EMPTY_BLOCK_TYPE);
        let break_target = self.push_control_frame();
        self.state
            .emission
            .control_flow
            .break_stack
            .push(BreakContext {
                break_target,
                labels: labels.to_vec(),
                break_hook: None,
            });
        self.emit_statements(body)?;
        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();
        self.state.emission.control_flow.break_stack.pop();
        Ok(())
    }
}
