use super::*;

fn collect_for_head_lexical_binding_names(head: &ForHead) -> Result<Vec<String>> {
    let mut names = Vec::new();
    match head {
        ForHead::VarDecl(variable_declaration)
            if !matches!(variable_declaration.kind, VarDeclKind::Var) =>
        {
            for declarator in &variable_declaration.decls {
                collect_for_of_binding_names(&declarator.name, &mut names)?;
            }
        }
        ForHead::UsingDecl(using_declaration) => {
            for declarator in &using_declaration.decls {
                collect_for_of_binding_names(&declarator.name, &mut names)?;
            }
        }
        _ => {}
    }
    Ok(names)
}

impl Lowerer {
    pub(crate) fn lower_for_of_statement(
        &mut self,
        for_of_statement: &ForOfStmt,
        allow_return: bool,
    ) -> Result<Vec<Statement>> {
        self.lower_for_of_statement_with_body_mode(for_of_statement, allow_return, false)
    }

    pub(crate) fn lower_generator_for_of_statement(
        &mut self,
        for_of_statement: &ForOfStmt,
        allow_return: bool,
    ) -> Result<Vec<Statement>> {
        if let Some(lowered) =
            self.lower_generator_for_await_yield_delegate_statement(for_of_statement)?
        {
            return Ok(lowered);
        }

        self.lower_for_of_statement_with_body_mode(for_of_statement, allow_return, true)
    }

    fn lower_generator_for_await_yield_delegate_statement(
        &mut self,
        for_of_statement: &ForOfStmt,
    ) -> Result<Option<Vec<Statement>>> {
        if !for_of_statement.is_await {
            return Ok(None);
        }

        let Some(binding_name) = for_of_binding_identifier_name(&for_of_statement.left) else {
            return Ok(None);
        };

        if !for_await_body_yields_binding(&for_of_statement.body, binding_name) {
            return Ok(None);
        }

        Ok(Some(vec![Statement::YieldDelegate {
            value: self.lower_expression(&for_of_statement.right)?,
        }]))
    }

    fn lower_for_of_statement_with_body_mode(
        &mut self,
        for_of_statement: &ForOfStmt,
        allow_return: bool,
        generator_body: bool,
    ) -> Result<Vec<Statement>> {
        let iterator_name = self.fresh_temporary_name("for_of_iter");
        let step_name = self.fresh_temporary_name("for_of_step");
        let value_name = self.fresh_temporary_name("for_of_value");
        let done_name = self.fresh_temporary_name("for_of_done");
        let lexical_binding_names = collect_for_head_lexical_binding_names(&for_of_statement.left)?;
        let pushed_loop_head_scope = !lexical_binding_names.is_empty();
        if pushed_loop_head_scope {
            self.push_renaming_binding_scope(lexical_binding_names.clone());
        }
        let iterator_value =
            Expression::GetIterator(Box::new(self.lower_expression(&for_of_statement.right)?));
        let step_value = Expression::Call {
            callee: Box::new(Expression::Member {
                object: Box::new(Expression::Identifier(iterator_name.clone())),
                property: Box::new(Expression::String("next".to_string())),
            }),
            arguments: Vec::new(),
        };
        let step_done = Expression::Member {
            object: Box::new(Expression::Identifier(step_name.clone())),
            property: Box::new(Expression::String("done".to_string())),
        };
        let mut iterated_value = Expression::Member {
            object: Box::new(Expression::Identifier(step_name.clone())),
            property: Box::new(Expression::String("value".to_string())),
        };
        if for_of_statement.is_await {
            iterated_value = Expression::Await(Box::new(iterated_value));
        }
        let break_hook = Expression::Conditional {
            condition: Box::new(Expression::Identifier(done_name.clone())),
            then_expression: Box::new(Expression::Undefined),
            else_expression: Box::new(Expression::IteratorClose(Box::new(Expression::Identifier(
                iterator_name.clone(),
            )))),
        };
        let lowered_binding_and_body =
            (|| -> Result<(ForOfBinding, Vec<Statement>, Vec<String>)> {
                let binding_value = Expression::Identifier(value_name.clone());
                let binding = if generator_body {
                    self.lower_generator_for_of_binding(&for_of_statement.left, binding_value)?
                } else {
                    self.lower_for_of_binding(&for_of_statement.left, binding_value)?
                };
                let per_iteration_bindings = lexical_binding_names
                    .iter()
                    .map(|name| self.resolve_binding_name(name))
                    .collect();
                let body = if generator_body {
                    self.lower_generator_loop_body(&for_of_statement.body, allow_return)?
                } else {
                    self.lower_block_or_statement(&for_of_statement.body, allow_return, true)?
                };
                Ok((binding, body, per_iteration_bindings))
            })();
        if pushed_loop_head_scope {
            self.pop_binding_scope();
        }
        let (binding, lowered_loop_body, per_iteration_bindings) = lowered_binding_and_body?;

        let mut body = vec![
            Statement::Let {
                name: step_name,
                mutable: true,
                value: step_value,
            },
            Statement::If {
                condition: step_done,
                then_branch: vec![
                    Statement::Assign {
                        name: done_name.clone(),
                        value: Expression::Bool(true),
                    },
                    Statement::Break { label: None },
                ],
                else_branch: Vec::new(),
            },
            Statement::Let {
                name: value_name,
                mutable: true,
                value: iterated_value,
            },
        ];
        let catch_name = self.fresh_temporary_name("for_of_catch");
        let mut protected_iteration = binding.per_iteration;
        protected_iteration.extend(lowered_loop_body);
        body.push(Statement::Try {
            body: protected_iteration,
            catch_binding: Some(catch_name.clone()),
            catch_setup: Vec::new(),
            catch_body: vec![
                Statement::Expression(Expression::IteratorClose(Box::new(Expression::Identifier(
                    iterator_name.clone(),
                )))),
                Statement::Throw(Expression::Identifier(catch_name)),
            ],
        });

        let mut init = vec![Statement::Let {
            name: iterator_name,
            mutable: true,
            value: iterator_value,
        }];
        init.extend(binding.before_loop);
        init.push(Statement::Let {
            name: done_name,
            mutable: true,
            value: Expression::Bool(false),
        });
        Ok(vec![Statement::For {
            labels: Vec::new(),
            init,
            per_iteration_bindings,
            condition: Some(Expression::Bool(true)),
            update: None,
            break_hook: Some(break_hook),
            body,
        }])
    }

    pub(crate) fn lower_for_in_statement(
        &mut self,
        for_in_statement: &ForInStmt,
        allow_return: bool,
    ) -> Result<Vec<Statement>> {
        let target_name = self.fresh_temporary_name("for_in_target");
        let keys_name = self.fresh_temporary_name("for_in_keys");
        let index_name = self.fresh_temporary_name("for_in_index");
        let target_value = self.lower_expression(&for_in_statement.right)?;
        let target_expression = Expression::Identifier(target_name.clone());
        let enumerated_keys = Expression::EnumerateKeys(Box::new(target_expression.clone()));
        let current_key = Expression::Member {
            object: Box::new(Expression::Identifier(keys_name.clone())),
            property: Box::new(Expression::Identifier(index_name.clone())),
        };
        let lexical_binding_names = collect_for_head_lexical_binding_names(&for_in_statement.left)?;
        if !lexical_binding_names.is_empty() {
            self.push_renaming_binding_scope(lexical_binding_names.clone());
        }
        let lowered_binding_and_body =
            (|| -> Result<(ForOfBinding, Vec<Statement>, Vec<String>)> {
                let binding =
                    self.lower_for_of_binding(&for_in_statement.left, current_key.clone())?;
                let per_iteration_bindings = lexical_binding_names
                    .iter()
                    .map(|name| self.resolve_binding_name(name))
                    .collect();
                let body =
                    self.lower_block_or_statement(&for_in_statement.body, allow_return, true)?;
                Ok((binding, body, per_iteration_bindings))
            })();
        if !lexical_binding_names.is_empty() {
            self.pop_binding_scope();
        }
        let (binding, lowered_loop_body, per_iteration_bindings) = lowered_binding_and_body?;

        let mut init = binding.before_loop;
        init.push(Statement::Let {
            name: target_name,
            mutable: false,
            value: target_value,
        });
        init.push(Statement::Let {
            name: keys_name.clone(),
            mutable: false,
            value: enumerated_keys,
        });
        init.push(Statement::Let {
            name: index_name.clone(),
            mutable: true,
            value: Expression::Number(0.0),
        });

        let mut guarded_body = binding.per_iteration;
        guarded_body.extend(lowered_loop_body);
        let body = vec![Statement::If {
            condition: Expression::Binary {
                op: BinaryOp::In,
                left: Box::new(current_key),
                right: Box::new(target_expression),
            },
            then_branch: guarded_body,
            else_branch: Vec::new(),
        }];

        Ok(vec![Statement::For {
            labels: Vec::new(),
            init,
            per_iteration_bindings,
            condition: Some(Expression::Binary {
                op: BinaryOp::LessThan,
                left: Box::new(Expression::Identifier(index_name.clone())),
                right: Box::new(Expression::Member {
                    object: Box::new(Expression::Identifier(keys_name)),
                    property: Box::new(Expression::String("length".to_string())),
                }),
            }),
            update: Some(Expression::Update {
                name: index_name,
                op: UpdateOp::Increment,
                prefix: false,
            }),
            break_hook: None,
            body,
        }])
    }

    pub(crate) fn lower_break_statement(
        &mut self,
        break_statement: &BreakStmt,
        allow_loop_control: bool,
    ) -> Result<Vec<Statement>> {
        if break_statement.label.is_none() {
            ensure!(allow_loop_control, "`break` is only supported inside loops");
        }

        Ok(vec![Statement::Break {
            label: break_statement
                .label
                .as_ref()
                .map(|label| label.sym.to_string()),
        }])
    }

    pub(crate) fn lower_continue_statement(
        &mut self,
        continue_statement: &ContinueStmt,
        allow_loop_control: bool,
    ) -> Result<Vec<Statement>> {
        if continue_statement.label.is_none() {
            ensure!(
                allow_loop_control,
                "`continue` is only supported inside loops"
            );
        }

        Ok(vec![Statement::Continue {
            label: continue_statement
                .label
                .as_ref()
                .map(|label| label.sym.to_string()),
        }])
    }

    pub(crate) fn lower_switch_statement(
        &mut self,
        switch_statement: &SwitchStmt,
        allow_return: bool,
        _allow_loop_control: bool,
    ) -> Result<Vec<Statement>> {
        let bindings = collect_switch_bindings(switch_statement)?;
        let binding_names = bindings.iter().cloned().collect::<HashSet<_>>();
        let discriminant = self.lower_expression(&switch_statement.discriminant)?;

        self.push_renaming_binding_scope(bindings.clone());
        let lowered = (|| -> Result<Statement> {
            let scoped_bindings = bindings
                .iter()
                .map(|name| self.resolve_binding_name(name))
                .collect::<Vec<_>>();
            let cases = switch_statement
                .cases
                .iter()
                .map(|case| {
                    Ok(SwitchCase {
                        test: case
                            .test
                            .as_deref()
                            .map(|expression| self.lower_expression(expression))
                            .transpose()?,
                        body: self.lower_switch_case_statements(
                            &case.cons,
                            allow_return,
                            true,
                            &binding_names,
                        )?,
                    })
                })
                .collect::<Result<Vec<_>>>()?;

            Ok(Statement::Switch {
                labels: Vec::new(),
                bindings: scoped_bindings,
                discriminant,
                cases,
            })
        })();
        self.pop_binding_scope();

        Ok(vec![lowered?])
    }

    pub(crate) fn lower_switch_case_statements(
        &mut self,
        statements: &[Stmt],
        allow_return: bool,
        allow_loop_control: bool,
        bindings: &HashSet<String>,
    ) -> Result<Vec<Statement>> {
        let mut lowered = Vec::new();

        for statement in statements {
            if let Stmt::Decl(Decl::Var(variable_declaration)) = statement
                && !matches!(variable_declaration.kind, VarDeclKind::Var)
            {
                lowered.extend(
                    self.lower_switch_case_lexical_declaration(variable_declaration, bindings)?,
                );
                continue;
            }

            lowered.extend(self.lower_statement(statement, allow_return, allow_loop_control)?);
        }

        Ok(lowered)
    }

    pub(crate) fn lower_switch_case_lexical_declaration(
        &mut self,
        variable_declaration: &swc_ecma_ast::VarDecl,
        bindings: &HashSet<String>,
    ) -> Result<Vec<Statement>> {
        let mut lowered = Vec::new();

        for declarator in &variable_declaration.decls {
            let mut names = Vec::new();
            collect_pattern_binding_names(&declarator.name, &mut names)?;
            if names.iter().any(|name| !bindings.contains(name)) {
                bail!("unsupported switch lexical binding");
            }

            let value = match declarator.init.as_deref() {
                Some(initializer) => self.lower_expression_with_name_hint(
                    initializer,
                    pattern_name_hint(&declarator.name),
                )?,
                None => Expression::Undefined,
            };

            if let Pat::Ident(identifier) = &declarator.name {
                lowered.push(Statement::Let {
                    name: self.resolve_binding_name(identifier.id.sym.as_ref()),
                    mutable: !matches!(variable_declaration.kind, VarDeclKind::Const),
                    value,
                });
                continue;
            }

            let temporary_name = self.fresh_temporary_name("switch_decl");
            lowered.push(Statement::Let {
                name: temporary_name.clone(),
                mutable: true,
                value,
            });
            self.lower_for_of_pattern_binding(
                &declarator.name,
                Expression::Identifier(temporary_name),
                ForOfPatternBindingKind::Lexical {
                    mutable: !matches!(variable_declaration.kind, VarDeclKind::Const),
                },
                &mut lowered,
            )?;
        }

        Ok(lowered)
    }

    pub(crate) fn lower_labeled_statement(
        &mut self,
        labeled_statement: &LabeledStmt,
        allow_return: bool,
        allow_loop_control: bool,
    ) -> Result<Vec<Statement>> {
        let label = labeled_statement.label.sym.to_string();
        let mut lowered = match &*labeled_statement.body {
            Stmt::Block(block) => vec![Statement::Labeled {
                labels: Vec::new(),
                body: self.lower_statements(&block.stmts, allow_return, allow_loop_control)?,
            }],
            statement => self.lower_statement(statement, allow_return, allow_loop_control)?,
        };

        self.attach_label_to_lowered(&mut lowered, label)?;
        Ok(lowered)
    }

    pub(crate) fn attach_label_to_lowered(
        &mut self,
        lowered: &mut Vec<Statement>,
        label: String,
    ) -> Result<()> {
        let single_statement = lowered.len() == 1;
        if let Some(last) = lowered.last_mut() {
            match last {
                Statement::For { labels, .. }
                | Statement::While { labels, .. }
                | Statement::DoWhile { labels, .. }
                | Statement::Switch { labels, .. } => {
                    labels.insert(0, label);
                    return Ok(());
                }
                Statement::Labeled { labels, .. } if single_statement => {
                    labels.insert(0, label);
                    return Ok(());
                }
                _ => {}
            }
        }

        if lowered.is_empty() {
            bail!("unsupported labeled statement")
        }

        let body = std::mem::take(lowered);
        lowered.push(Statement::Labeled {
            labels: vec![label],
            body,
        });
        Ok(())
    }
}

fn for_of_binding_identifier_name(left: &ForHead) -> Option<&str> {
    match left {
        ForHead::Pat(pattern) => {
            let Pat::Ident(identifier) = &**pattern else {
                return None;
            };
            Some(identifier.id.sym.as_ref())
        }
        ForHead::VarDecl(variable_declaration) => {
            let [declarator] = &variable_declaration.decls[..] else {
                return None;
            };
            if declarator.init.is_some() {
                return None;
            }
            let Pat::Ident(identifier) = &declarator.name else {
                return None;
            };
            Some(identifier.id.sym.as_ref())
        }
        ForHead::UsingDecl(using_declaration) => {
            let [declarator] = &using_declaration.decls[..] else {
                return None;
            };
            if declarator.init.is_some() {
                return None;
            }
            let Pat::Ident(identifier) = &declarator.name else {
                return None;
            };
            Some(identifier.id.sym.as_ref())
        }
    }
}

fn for_await_body_yields_binding(statement: &Stmt, binding_name: &str) -> bool {
    let statement = match statement {
        Stmt::Block(BlockStmt { stmts, .. }) => {
            let [statement] = &stmts[..] else {
                return false;
            };
            statement
        }
        other => other,
    };

    let Stmt::Expr(ExprStmt { expr, .. }) = statement else {
        return false;
    };
    let Expr::Yield(yield_expression) = &**expr else {
        return false;
    };
    if yield_expression.delegate {
        return false;
    }
    let Some(argument) = yield_expression.arg.as_deref() else {
        return false;
    };
    let Expr::Ident(identifier) = argument else {
        return false;
    };

    identifier.sym == binding_name
}
