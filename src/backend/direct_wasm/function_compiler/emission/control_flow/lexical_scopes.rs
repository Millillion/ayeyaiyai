use super::*;

impl<'a> FunctionCompiler<'a> {
    fn expression_timing_label(expression: &Expression) -> String {
        match expression {
            Expression::Call { callee, .. } => {
                format!("Call({})", Self::expression_timing_label(callee))
            }
            Expression::Member { object, property } => {
                format!(
                    "Member({}.{})",
                    Self::expression_timing_label(object),
                    Self::expression_timing_label(property)
                )
            }
            Expression::Identifier(name) => format!("Identifier({name})"),
            Expression::String(value) => format!("String({value})"),
            Expression::Number(value) => format!("Number({value})"),
            Expression::Bool(value) => format!("Bool({value})"),
            Expression::Null => "Null".to_string(),
            Expression::Undefined => "Undefined".to_string(),
            _ => format!("{:?}", std::mem::discriminant(expression)),
        }
    }

    fn statement_timing_label(statement: &Statement) -> String {
        match statement {
            Statement::Let { name, value, .. } => {
                format!("Let {name} = {}", Self::expression_timing_label(value))
            }
            Statement::Var { name, value } => {
                format!("Var {name} = {}", Self::expression_timing_label(value))
            }
            Statement::Assign { name, value } => {
                format!("Assign {name} = {}", Self::expression_timing_label(value))
            }
            Statement::Expression(expression) => {
                format!("Expression {}", Self::expression_timing_label(expression))
            }
            Statement::Declaration { body } => format!("Declaration len={}", body.len()),
            Statement::Block { body } => format!("Block len={}", body.len()),
            Statement::If { .. } => "If".to_string(),
            Statement::Try { .. } => "Try".to_string(),
            Statement::Switch { cases, .. } => format!("Switch cases={}", cases.len()),
            Statement::For { .. } => "For".to_string(),
            Statement::While { .. } => "While".to_string(),
            Statement::DoWhile { .. } => "DoWhile".to_string(),
            Statement::Return(value) => format!("Return {}", Self::expression_timing_label(value)),
            Statement::Throw(value) => format!("Throw {}", Self::expression_timing_label(value)),
            Statement::Break { .. } => "Break".to_string(),
            Statement::Continue { .. } => "Continue".to_string(),
            Statement::Yield { value } => format!("Yield {}", Self::expression_timing_label(value)),
            Statement::YieldDelegate { value } => {
                format!("YieldDelegate {}", Self::expression_timing_label(value))
            }
            Statement::With { .. } => "With".to_string(),
            Statement::Labeled { labels, .. } => format!("Labeled labels={}", labels.len()),
            Statement::AssignMember { .. } => "AssignMember".to_string(),
            Statement::Print { values } => format!("Print values={}", values.len()),
        }
    }

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
        let trace_timing = std::env::var_os("AYY_TRACE_FUNCTION_STATEMENT_TIMING").is_some();
        let mut index = 0;
        while let Some(statement) = statements.get(index) {
            let timing_start = trace_timing.then(std::time::Instant::now);
            if trace_timing {
                eprintln!(
                    "function_statement_timing=start index={index} label={}",
                    Self::statement_timing_label(statement)
                );
            }
            if trace {
                eprintln!("function_compile=statement:{statement:?}");
            }
            let next_statement = statements.get(index + 1);
            if self.try_emit_static_lowered_await_resume_statement(statement, next_statement)? {
                index += 2;
                continue;
            } else if !self.try_emit_destructuring_default_iterator_close_statement(
                statement,
                next_statement,
            )? {
                self.emit_statement(statement)?;
            }
            if trace {
                eprintln!("function_compile=statement_done:{statement:?}");
            }
            if let Some(timing_start) = timing_start {
                eprintln!(
                    "function_statement_timing=done index={index} elapsed_ms={} label={}",
                    timing_start.elapsed().as_millis(),
                    Self::statement_timing_label(statement)
                );
            }
            if Self::statement_unconditionally_transfers_control(statement) {
                break;
            }
            index += 1;
        }
        Ok(())
    }

    fn expression_is_await_resume_sent(expression: &Expression) -> bool {
        matches!(
            expression,
            Expression::Call { callee, arguments }
                if matches!(callee.as_ref(), Expression::Identifier(name) if name == "__ayyAwaitResume")
                    && matches!(
                        arguments.as_slice(),
                        [CallArgument::Expression(Expression::Sent)]
                    )
        )
    }

    fn lowered_await_resume_statement(
        next_statement: &Statement,
        value: Expression,
    ) -> Option<Statement> {
        match next_statement {
            Statement::Expression(expression)
                if Self::expression_is_await_resume_sent(expression) =>
            {
                Some(Statement::Expression(value))
            }
            Statement::Var {
                name,
                value: expression,
            } if Self::expression_is_await_resume_sent(expression) => Some(Statement::Var {
                name: name.clone(),
                value,
            }),
            Statement::Let {
                name,
                mutable,
                value: expression,
            } if Self::expression_is_await_resume_sent(expression) => Some(Statement::Let {
                name: name.clone(),
                mutable: *mutable,
                value,
            }),
            Statement::Assign {
                name,
                value: expression,
            } if Self::expression_is_await_resume_sent(expression) => Some(Statement::Assign {
                name: name.clone(),
                value,
            }),
            Statement::Return(expression) if Self::expression_is_await_resume_sent(expression) => {
                Some(Statement::Return(value))
            }
            Statement::If {
                condition,
                then_branch,
                else_branch,
            } if Self::expression_is_await_resume_sent(condition) => Some(Statement::If {
                condition: value,
                then_branch: then_branch.clone(),
                else_branch: else_branch.clone(),
            }),
            _ => None,
        }
    }

    fn try_emit_static_lowered_await_resume_statement(
        &mut self,
        statement: &Statement,
        next_statement: Option<&Statement>,
    ) -> DirectResult<bool> {
        let Statement::Yield { value } = statement else {
            return Ok(false);
        };
        let Some(next_statement) = next_statement else {
            return Ok(false);
        };
        if Self::lowered_await_resume_statement(next_statement, Expression::Undefined).is_none() {
            return Ok(false);
        }
        self.emit_pending_static_promise_reactions()?;
        let Some(outcome) = self.resolve_static_await_resolution_outcome(value) else {
            return Ok(false);
        };
        self.emit_static_await_resolution_effects(value)?;
        self.emit_pending_static_promise_reactions()?;
        let resumed_value = match outcome {
            StaticEvalOutcome::Value(value) => value,
            StaticEvalOutcome::Throw(throw_value) => {
                self.emit_static_throw_value(&throw_value)?;
                return Ok(true);
            }
        };
        let resumed_statement = Self::lowered_await_resume_statement(next_statement, resumed_value)
            .expect("lowered await resume target checked before await resolution");
        self.emit_statement(&resumed_statement)?;
        Ok(true)
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
