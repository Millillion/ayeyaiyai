use super::*;

fn collect_static_block_scope_bindings(statements: &[Stmt]) -> Result<Vec<String>> {
    let mut bindings = collect_function_scope_binding_names(statements)?;
    for binding in collect_direct_statement_lexical_bindings(statements)? {
        if !bindings.contains(&binding) {
            bindings.push(binding);
        }
    }
    Ok(bindings)
}

impl Lowerer {
    fn nested_destructuring_assignment_expression(
        expression: &Expr,
    ) -> Option<&swc_ecma_ast::AssignExpr> {
        match expression {
            Expr::Assign(assignment)
                if assignment.op == AssignOp::Assign
                    && matches!(assignment.left, swc_ecma_ast::AssignTarget::Pat(_)) =>
            {
                Some(assignment)
            }
            Expr::Paren(parenthesized) => {
                Self::nested_destructuring_assignment_expression(&parenthesized.expr)
            }
            _ => None,
        }
    }

    fn statement_has_deterministic_terminal_throw(statement: &Statement) -> bool {
        match statement {
            Statement::Throw(_) => true,
            Statement::Block { body }
            | Statement::Declaration { body }
            | Statement::Labeled { body, .. } => {
                Self::statements_have_deterministic_terminal_throw(body)
            }
            Statement::If {
                then_branch,
                else_branch,
                ..
            } => {
                !then_branch.is_empty()
                    && !else_branch.is_empty()
                    && Self::statements_have_deterministic_terminal_throw(then_branch)
                    && Self::statements_have_deterministic_terminal_throw(else_branch)
            }
            _ => false,
        }
    }

    fn statements_have_deterministic_terminal_throw(statements: &[Statement]) -> bool {
        statements
            .last()
            .is_some_and(Self::statement_has_deterministic_terminal_throw)
    }

    fn neutralize_control_before_terminal_finalizer(statements: Vec<Statement>) -> Vec<Statement> {
        statements
            .into_iter()
            .map(|statement| match statement {
                Statement::Return(expression) => Statement::Expression(expression),
                Statement::Break { .. } | Statement::Continue { .. } => {
                    Statement::Expression(Expression::Undefined)
                }
                Statement::If {
                    condition,
                    then_branch,
                    else_branch,
                } => Statement::If {
                    condition,
                    then_branch: Self::neutralize_control_before_terminal_finalizer(then_branch),
                    else_branch: Self::neutralize_control_before_terminal_finalizer(else_branch),
                },
                Statement::Block { body } => Statement::Block {
                    body: Self::neutralize_control_before_terminal_finalizer(body),
                },
                Statement::Declaration { body } => Statement::Declaration {
                    body: Self::neutralize_control_before_terminal_finalizer(body),
                },
                Statement::Labeled { labels, body } => Statement::Labeled {
                    labels,
                    body: Self::neutralize_control_before_terminal_finalizer(body),
                },
                Statement::Try {
                    body,
                    catch_binding,
                    catch_setup,
                    catch_body,
                } => Statement::Try {
                    body: Self::neutralize_control_before_terminal_finalizer(body),
                    catch_binding,
                    catch_setup,
                    catch_body: Self::neutralize_control_before_terminal_finalizer(catch_body),
                },
                statement => statement,
            })
            .collect()
    }

    fn marked_finalizer_statements(finalizer: &[Statement], marker_name: &str) -> Vec<Statement> {
        let mut statements = Vec::with_capacity(finalizer.len() + 1);
        statements.push(Statement::Assign {
            name: marker_name.to_string(),
            value: Expression::Bool(true),
        });
        statements.extend(finalizer.iter().cloned());
        statements
    }

    fn prepend_finalizer_before_abrupt_completion(
        statements: Vec<Statement>,
        finalizer: &[Statement],
        marker_name: &str,
    ) -> Vec<Statement> {
        let mut rewritten = Vec::new();
        for statement in statements {
            match statement {
                Statement::Break { .. } | Statement::Continue { .. } | Statement::Return(_) => {
                    rewritten.extend(Self::marked_finalizer_statements(finalizer, marker_name));
                    rewritten.push(statement);
                }
                Statement::If {
                    condition,
                    then_branch,
                    else_branch,
                } => rewritten.push(Statement::If {
                    condition,
                    then_branch: Self::prepend_finalizer_before_abrupt_completion(
                        then_branch,
                        finalizer,
                        marker_name,
                    ),
                    else_branch: Self::prepend_finalizer_before_abrupt_completion(
                        else_branch,
                        finalizer,
                        marker_name,
                    ),
                }),
                Statement::Block { body } => rewritten.push(Statement::Block {
                    body: Self::prepend_finalizer_before_abrupt_completion(
                        body,
                        finalizer,
                        marker_name,
                    ),
                }),
                Statement::Declaration { body } => rewritten.push(Statement::Declaration {
                    body: Self::prepend_finalizer_before_abrupt_completion(
                        body,
                        finalizer,
                        marker_name,
                    ),
                }),
                Statement::Labeled { labels, body } => rewritten.push(Statement::Labeled {
                    labels,
                    body: Self::prepend_finalizer_before_abrupt_completion(
                        body,
                        finalizer,
                        marker_name,
                    ),
                }),
                Statement::Try {
                    body,
                    catch_binding,
                    catch_setup,
                    catch_body,
                } => rewritten.push(Statement::Try {
                    body: Self::prepend_finalizer_before_abrupt_completion(
                        body,
                        finalizer,
                        marker_name,
                    ),
                    catch_binding,
                    catch_setup,
                    catch_body: Self::prepend_finalizer_before_abrupt_completion(
                        catch_body,
                        finalizer,
                        marker_name,
                    ),
                }),
                _ => rewritten.push(statement),
            }
        }
        rewritten
    }

    fn symbol_dispose_expression() -> Expression {
        Expression::Member {
            object: Box::new(Expression::Identifier("Symbol".to_string())),
            property: Box::new(Expression::String("dispose".to_string())),
        }
    }

    fn using_resource_is_disposable_expression(name: &str) -> Expression {
        let resource = Expression::Identifier(name.to_string());
        Expression::Binary {
            op: BinaryOp::LogicalAnd,
            left: Box::new(Expression::Binary {
                op: BinaryOp::NotEqual,
                left: Box::new(resource.clone()),
                right: Box::new(Expression::Null),
            }),
            right: Box::new(Expression::Binary {
                op: BinaryOp::NotEqual,
                left: Box::new(resource),
                right: Box::new(Expression::Undefined),
            }),
        }
    }

    fn using_dispose_statement(name: &str, dispose_object: Expression) -> Statement {
        Statement::If {
            condition: Self::using_resource_is_disposable_expression(name),
            then_branch: vec![Statement::Expression(Expression::Call {
                callee: Box::new(Expression::Member {
                    object: Box::new(dispose_object),
                    property: Box::new(Self::symbol_dispose_expression()),
                }),
                arguments: Vec::new(),
            })],
            else_branch: Vec::new(),
        }
    }

    fn suppressed_error_expression(error: Expression, suppressed: Expression) -> Expression {
        Expression::Object(vec![
            ObjectEntry::Data {
                key: Expression::String("__proto__".to_string()),
                value: Expression::Member {
                    object: Box::new(Expression::Identifier("SuppressedError".to_string())),
                    property: Box::new(Expression::String("prototype".to_string())),
                },
            },
            ObjectEntry::Data {
                key: Expression::String("constructor".to_string()),
                value: Expression::Identifier("SuppressedError".to_string()),
            },
            ObjectEntry::Data {
                key: Expression::String("name".to_string()),
                value: Expression::String("SuppressedError".to_string()),
            },
            ObjectEntry::Data {
                key: Expression::String("error".to_string()),
                value: error,
            },
            ObjectEntry::Data {
                key: Expression::String("suppressed".to_string()),
                value: suppressed,
            },
        ])
    }

    fn guarded_using_finalizer_statement(finalizer: &[Statement], marker_name: &str) -> Statement {
        Statement::If {
            condition: Expression::Unary {
                op: UnaryOp::Not,
                expression: Box::new(Expression::Identifier(marker_name.to_string())),
            },
            then_branch: Self::marked_finalizer_statements(finalizer, marker_name),
            else_branch: Vec::new(),
        }
    }

    fn guarded_using_finalizer_with_existing_completion(
        &mut self,
        finalizer: &[Statement],
        marker_name: &str,
        completion_name: &str,
    ) -> Statement {
        let mut then_branch = vec![Statement::Assign {
            name: marker_name.to_string(),
            value: Expression::Bool(true),
        }];
        for statement in finalizer {
            let disposal_error_name = self.fresh_temporary_name("using_dispose_error");
            then_branch.push(Statement::Try {
                body: vec![statement.clone()],
                catch_binding: Some(disposal_error_name.clone()),
                catch_setup: Vec::new(),
                catch_body: vec![Statement::Assign {
                    name: completion_name.to_string(),
                    value: Self::suppressed_error_expression(
                        Expression::Identifier(disposal_error_name),
                        Expression::Identifier(completion_name.to_string()),
                    ),
                }],
            });
        }

        Statement::If {
            condition: Expression::Unary {
                op: UnaryOp::Not,
                expression: Box::new(Expression::Identifier(marker_name.to_string())),
            },
            then_branch,
            else_branch: Vec::new(),
        }
    }

    fn wrap_using_scope(
        &mut self,
        body: Vec<Statement>,
        mut finalizer: Vec<Statement>,
    ) -> Vec<Statement> {
        if finalizer.is_empty() {
            return body;
        }

        finalizer.reverse();
        let disposed_name = self.fresh_temporary_name("using_disposed");
        let error_name = self.fresh_temporary_name("using_error");
        let body =
            Self::prepend_finalizer_before_abrupt_completion(body, &finalizer, &disposed_name);
        let guarded_finalizer = Self::guarded_using_finalizer_statement(&finalizer, &disposed_name);
        let guarded_throwing_finalizer = self.guarded_using_finalizer_with_existing_completion(
            &finalizer,
            &disposed_name,
            &error_name,
        );

        vec![
            Statement::Let {
                name: disposed_name.clone(),
                mutable: true,
                value: Expression::Bool(false),
            },
            Statement::Try {
                body,
                catch_binding: Some(error_name.clone()),
                catch_setup: Vec::new(),
                catch_body: vec![
                    guarded_throwing_finalizer,
                    Statement::Throw(Expression::Identifier(error_name)),
                ],
            },
            guarded_finalizer,
        ]
    }

    pub(super) fn lower_using_declaration(
        &mut self,
        using_declaration: &swc_ecma_ast::UsingDecl,
        protect_abrupt_completion: bool,
    ) -> Result<(Vec<Statement>, Vec<Statement>, Vec<Statement>)> {
        ensure!(
            !using_declaration.is_await,
            "`await using` is not supported in statement lowering yet"
        );

        let mut prelude = Vec::new();
        let mut lowered = Vec::new();
        let mut finalizer = Vec::new();

        for declarator in &using_declaration.decls {
            let initializer_is_class_expression =
                matches!(declarator.init.as_deref(), Some(Expr::Class(_)));
            if let Pat::Ident(identifier) = &declarator.name {
                let name = self.resolve_binding_name(identifier.id.sym.as_ref());
                let value = match declarator.init.as_deref() {
                    Some(initializer) => self.lower_expression_with_name_hint(
                        initializer,
                        Some(identifier.id.sym.as_ref()),
                    )?,
                    None => Expression::Undefined,
                };
                if protect_abrupt_completion {
                    prelude.push(Statement::Let {
                        name: name.clone(),
                        mutable: true,
                        value: Expression::Undefined,
                    });
                    lowered.push(Statement::Assign {
                        name: name.clone(),
                        value: value.clone(),
                    });
                } else {
                    lowered.push(Statement::Let {
                        name: name.clone(),
                        mutable: false,
                        value: value.clone(),
                    });
                }
                let dispose_object = match &value {
                    Expression::Identifier(_) => value.clone(),
                    _ => Expression::Identifier(name.clone()),
                };
                if !initializer_is_class_expression {
                    finalizer.push(Self::using_dispose_statement(&name, dispose_object));
                }
                continue;
            }

            let temporary_name = self.fresh_temporary_name("using");
            let value = match declarator.init.as_deref() {
                Some(initializer) => self.lower_expression_with_name_hint(
                    initializer,
                    pattern_name_hint(&declarator.name),
                )?,
                None => Expression::Undefined,
            };
            if protect_abrupt_completion {
                prelude.push(Statement::Let {
                    name: temporary_name.clone(),
                    mutable: true,
                    value: Expression::Undefined,
                });
                lowered.push(Statement::Assign {
                    name: temporary_name.clone(),
                    value,
                });
            } else {
                lowered.push(Statement::Let {
                    name: temporary_name.clone(),
                    mutable: false,
                    value,
                });
            }
            self.lower_for_of_pattern_binding(
                &declarator.name,
                Expression::Identifier(temporary_name.clone()),
                ForOfPatternBindingKind::Lexical { mutable: false },
                &mut lowered,
            )?;
            if !initializer_is_class_expression {
                finalizer.push(Self::using_dispose_statement(
                    &temporary_name,
                    Expression::Identifier(temporary_name.clone()),
                ));
            }
        }

        Ok((prelude, lowered, finalizer))
    }

    fn statement_has_explicit_abrupt_completion(statement: &Stmt) -> bool {
        match statement {
            Stmt::Throw(_) | Stmt::Return(_) | Stmt::Break(_) | Stmt::Continue(_) => true,
            Stmt::Block(block) => block
                .stmts
                .iter()
                .any(Self::statement_has_explicit_abrupt_completion),
            Stmt::Labeled(labeled) => Self::statement_has_explicit_abrupt_completion(&labeled.body),
            Stmt::If(if_statement) => {
                Self::statement_has_explicit_abrupt_completion(&if_statement.cons)
                    || if_statement
                        .alt
                        .as_deref()
                        .is_some_and(Self::statement_has_explicit_abrupt_completion)
            }
            Stmt::Try(try_statement) => {
                try_statement
                    .block
                    .stmts
                    .iter()
                    .any(Self::statement_has_explicit_abrupt_completion)
                    || try_statement.handler.as_ref().is_some_and(|handler| {
                        handler
                            .body
                            .stmts
                            .iter()
                            .any(Self::statement_has_explicit_abrupt_completion)
                    })
                    || try_statement.finalizer.as_ref().is_some_and(|finalizer| {
                        finalizer
                            .stmts
                            .iter()
                            .any(Self::statement_has_explicit_abrupt_completion)
                    })
            }
            _ => false,
        }
    }

    fn statement_has_multi_declarator_using(statement: &Stmt) -> bool {
        matches!(
            statement,
            Stmt::Decl(Decl::Using(using_declaration)) if using_declaration.decls.len() > 1
        )
    }

    pub(crate) fn lower_statements(
        &mut self,
        statements: &[Stmt],
        allow_return: bool,
        allow_loop_control: bool,
    ) -> Result<Vec<Statement>> {
        let scope_bindings = collect_direct_statement_lexical_bindings(statements)?;
        self.push_renaming_binding_scope(scope_bindings);
        let lowered = self.lower_statement_list(statements, allow_return, allow_loop_control);
        self.pop_binding_scope();
        lowered
    }

    pub(crate) fn lower_static_block_statements(
        &mut self,
        statements: &[Stmt],
    ) -> Result<Vec<Statement>> {
        let scope_bindings = collect_static_block_scope_bindings(statements)?;
        self.push_renaming_binding_scope(scope_bindings);
        let lowered = self.lower_statement_list(statements, false, false);
        self.pop_binding_scope();
        lowered
    }

    pub(crate) fn lower_block_statements(
        &mut self,
        statements: &[Stmt],
        allow_return: bool,
        allow_loop_control: bool,
    ) -> Result<Vec<Statement>> {
        let scope_bindings = collect_direct_statement_lexical_bindings(statements)?;
        self.push_renaming_binding_scope(scope_bindings);
        let lowered = self.lower_statement_list(statements, allow_return, allow_loop_control);
        self.pop_binding_scope();
        lowered
    }

    fn lower_statement_list(
        &mut self,
        statements: &[Stmt],
        allow_return: bool,
        allow_loop_control: bool,
    ) -> Result<Vec<Statement>> {
        let lowered = (|| -> Result<Vec<Statement>> {
            let mut lowered = Vec::new();
            let mut using_prelude = Vec::new();
            let mut using_finalizer = Vec::new();
            let protect_using = statements
                .iter()
                .any(Self::statement_has_explicit_abrupt_completion)
                || statements
                    .iter()
                    .any(Self::statement_has_multi_declarator_using);

            for statement in statements {
                if let Stmt::Decl(Decl::Using(using_declaration)) = statement {
                    let (mut prelude, mut using_bindings, mut finalizer) =
                        self.lower_using_declaration(using_declaration, protect_using)?;
                    using_prelude.append(&mut prelude);
                    lowered.append(&mut using_bindings);
                    using_finalizer.append(&mut finalizer);
                } else {
                    lowered.extend(self.lower_statement(
                        statement,
                        allow_return,
                        allow_loop_control,
                    )?);
                }
            }

            if protect_using {
                using_prelude.extend(self.wrap_using_scope(lowered, using_finalizer));
            } else {
                using_finalizer.reverse();
                lowered.extend(using_finalizer);
                using_prelude.extend(lowered);
            }
            Ok(using_prelude)
        })();
        lowered
    }

    pub(crate) fn lower_statement(
        &mut self,
        statement: &Stmt,
        allow_return: bool,
        allow_loop_control: bool,
    ) -> Result<Vec<Statement>> {
        match statement {
            Stmt::Decl(Decl::Var(variable_declaration)) => {
                self.lower_variable_declaration(variable_declaration)
            }
            Stmt::Decl(Decl::Fn(function_declaration)) => {
                self.lower_nested_function_declaration(function_declaration)
            }
            Stmt::Decl(Decl::Class(class_declaration)) => {
                self.lower_class_declaration(class_declaration)
            }
            Stmt::Decl(Decl::Using(using_declaration)) => {
                let (mut prelude, lowered, finalizer) =
                    self.lower_using_declaration(using_declaration, false)?;
                let mut finalizer = finalizer;
                finalizer.reverse();
                prelude.extend(lowered);
                prelude.extend(finalizer);
                Ok(prelude)
            }
            Stmt::Expr(ExprStmt { expr, .. }) => self.lower_expression_statement(expr),
            Stmt::Block(block) => Ok(vec![Statement::Block {
                body: self.lower_block_statements(
                    &block.stmts,
                    allow_return,
                    allow_loop_control,
                )?,
            }]),
            Stmt::If(if_statement) => Ok(vec![Statement::If {
                condition: self.lower_expression(&if_statement.test)?,
                then_branch: self.lower_block_or_statement(
                    &if_statement.cons,
                    allow_return,
                    allow_loop_control,
                )?,
                else_branch: self.lower_optional_else(
                    if_statement.alt.as_deref(),
                    allow_return,
                    allow_loop_control,
                )?,
            }]),
            Stmt::Switch(switch_statement) => {
                self.lower_switch_statement(switch_statement, allow_return, allow_loop_control)
            }
            Stmt::For(for_statement) => {
                let for_lexical_bindings = for_statement
                    .init
                    .as_ref()
                    .map(collect_for_per_iteration_bindings)
                    .transpose()?
                    .unwrap_or_default();
                if !for_lexical_bindings.is_empty() {
                    self.push_renaming_binding_scope(for_lexical_bindings.clone());
                }
                let lowered = (|| -> Result<Statement> {
                    Ok(Statement::For {
                        labels: Vec::new(),
                        init: match &for_statement.init {
                            Some(VarDeclOrExpr::VarDecl(variable_declaration)) => {
                                self.lower_variable_declaration(variable_declaration)?
                            }
                            Some(VarDeclOrExpr::Expr(expression)) => {
                                self.lower_expression_statement(expression)?
                            }
                            None => Vec::new(),
                        },
                        condition: for_statement
                            .test
                            .as_deref()
                            .map(|expression| self.lower_expression(expression))
                            .transpose()?,
                        update: for_statement
                            .update
                            .as_deref()
                            .map(|expression| self.lower_expression(expression))
                            .transpose()?,
                        per_iteration_bindings: for_lexical_bindings
                            .iter()
                            .map(|name| self.resolve_binding_name(name))
                            .collect(),
                        break_hook: None,
                        body: self.lower_block_or_statement(
                            &for_statement.body,
                            allow_return,
                            true,
                        )?,
                    })
                })();
                if !for_lexical_bindings.is_empty() {
                    self.pop_binding_scope();
                }
                Ok(vec![lowered?])
            }
            Stmt::ForOf(for_of_statement) => {
                self.lower_for_of_statement(for_of_statement, allow_return)
            }
            Stmt::ForIn(for_in_statement) => {
                self.lower_for_in_statement(for_in_statement, allow_return)
            }
            Stmt::DoWhile(do_while_statement) => Ok(vec![Statement::DoWhile {
                labels: Vec::new(),
                condition: self.lower_expression(&do_while_statement.test)?,
                break_hook: None,
                body: self.lower_block_or_statement(
                    &do_while_statement.body,
                    allow_return,
                    true,
                )?,
            }]),
            Stmt::With(with_statement) => {
                let object = self.lower_expression(&with_statement.obj)?;
                let body = self.lower_inside_with_scope(|lowerer| {
                    lowerer.lower_block_or_statement(
                        &with_statement.body,
                        allow_return,
                        allow_loop_control,
                    )
                })?;
                Ok(vec![Statement::With { object, body }])
            }
            Stmt::While(while_statement) => Ok(vec![Statement::While {
                labels: Vec::new(),
                condition: self.lower_expression(&while_statement.test)?,
                break_hook: None,
                body: self.lower_block_or_statement(&while_statement.body, allow_return, true)?,
            }]),
            Stmt::Throw(throw_statement) => Ok(vec![Statement::Throw(
                self.lower_expression(&throw_statement.arg)?,
            )]),
            Stmt::Try(try_statement) => {
                self.lower_try_statement(try_statement, allow_return, allow_loop_control)
            }
            Stmt::Return(return_statement) => {
                ensure!(allow_return, "`return` is only supported inside functions");
                Ok(vec![Statement::Return(
                    match return_statement.arg.as_deref() {
                        Some(expression) => self.lower_expression(expression)?,
                        None => Expression::Undefined,
                    },
                )])
            }
            Stmt::Break(break_statement) => {
                self.lower_break_statement(break_statement, allow_loop_control)
            }
            Stmt::Continue(continue_statement) => {
                self.lower_continue_statement(continue_statement, allow_loop_control)
            }
            Stmt::Labeled(labeled_statement) => {
                self.lower_labeled_statement(labeled_statement, allow_return, allow_loop_control)
            }
            Stmt::Debugger(_) => Ok(Vec::new()),
            Stmt::Empty(_) => Ok(Vec::new()),
            _ => bail!("unsupported statement: {statement:?}"),
        }
    }

    pub(crate) fn lower_try_statement(
        &mut self,
        try_statement: &swc_ecma_ast::TryStmt,
        allow_return: bool,
        allow_loop_control: bool,
    ) -> Result<Vec<Statement>> {
        if try_statement.finalizer.is_none()
            && try_statement.block.stmts.is_empty()
            && try_statement
                .handler
                .as_ref()
                .is_some_and(|handler| handler.body.stmts.is_empty())
        {
            return Ok(Vec::new());
        }

        let lowered_body =
            self.lower_statements(&try_statement.block.stmts, allow_return, allow_loop_control)?;
        let lowered_handler = try_statement
            .handler
            .as_ref()
            .map(|handler| self.lower_catch_clause(handler, allow_return, allow_loop_control))
            .transpose()?;

        if let Some(finalizer) = &try_statement.finalizer {
            let threw_name = self.fresh_temporary_name("finally_threw");
            let error_name = self.fresh_temporary_name("finally_error");
            let ran_name = self.fresh_temporary_name("finally_ran");
            let outer_catch_name = self.fresh_temporary_name("finally_catch");
            let lowered_finalizer =
                self.lower_statements(&finalizer.stmts, allow_return, allow_loop_control)?;
            let finalizer_terminal_throw =
                Self::statements_have_deterministic_terminal_throw(&lowered_finalizer);
            let mut statements = vec![
                Statement::Let {
                    name: threw_name.clone(),
                    mutable: true,
                    value: Expression::Bool(false),
                },
                Statement::Let {
                    name: error_name.clone(),
                    mutable: true,
                    value: Expression::Undefined,
                },
                Statement::Let {
                    name: ran_name.clone(),
                    mutable: true,
                    value: Expression::Bool(false),
                },
            ];
            let marked_finalizer = Self::marked_finalizer_statements(&lowered_finalizer, &ran_name);

            let protected_body =
                if let Some((catch_binding, catch_setup, catch_body)) = lowered_handler {
                    let body = if finalizer_terminal_throw {
                        Self::neutralize_control_before_terminal_finalizer(lowered_body)
                    } else {
                        Self::prepend_finalizer_before_abrupt_completion(
                            lowered_body,
                            &lowered_finalizer,
                            &ran_name,
                        )
                    };
                    let catch_body = if finalizer_terminal_throw {
                        Self::neutralize_control_before_terminal_finalizer(catch_body)
                    } else {
                        Self::prepend_finalizer_before_abrupt_completion(
                            catch_body,
                            &lowered_finalizer,
                            &ran_name,
                        )
                    };
                    vec![Statement::Try {
                        body,
                        catch_binding,
                        catch_setup,
                        catch_body,
                    }]
                } else if finalizer_terminal_throw {
                    Self::neutralize_control_before_terminal_finalizer(lowered_body)
                } else {
                    Self::prepend_finalizer_before_abrupt_completion(
                        lowered_body,
                        &lowered_finalizer,
                        &ran_name,
                    )
                };

            statements.push(Statement::Try {
                body: protected_body,
                catch_binding: Some(outer_catch_name.clone()),
                catch_setup: Vec::new(),
                catch_body: vec![
                    Statement::Assign {
                        name: threw_name.clone(),
                        value: Expression::Bool(true),
                    },
                    Statement::Assign {
                        name: error_name.clone(),
                        value: Expression::Identifier(outer_catch_name),
                    },
                ],
            });
            statements.push(Statement::If {
                condition: Expression::Unary {
                    op: UnaryOp::Not,
                    expression: Box::new(Expression::Identifier(ran_name)),
                },
                then_branch: marked_finalizer,
                else_branch: Vec::new(),
            });
            statements.push(Statement::If {
                condition: Expression::Identifier(threw_name),
                then_branch: vec![Statement::Throw(Expression::Identifier(error_name))],
                else_branch: Vec::new(),
            });
            return Ok(statements);
        }

        let (catch_binding, catch_setup, catch_body) =
            lowered_handler.context("`try` without `catch` is not supported yet")?;
        Ok(vec![Statement::Try {
            body: lowered_body,
            catch_binding,
            catch_setup,
            catch_body,
        }])
    }

    pub(crate) fn lower_catch_clause(
        &mut self,
        handler: &swc_ecma_ast::CatchClause,
        allow_return: bool,
        allow_loop_control: bool,
    ) -> Result<(Option<String>, Vec<Statement>, Vec<Statement>)> {
        let mut scope_bindings = Vec::new();
        if let Some(pattern) = handler.param.as_ref() {
            collect_pattern_binding_names(pattern, &mut scope_bindings)?;
        }

        self.push_renaming_binding_scope(scope_bindings);
        let lowered = (|| -> Result<(Option<String>, Vec<Statement>, Vec<Statement>)> {
            let (catch_binding, catch_setup) = match handler.param.as_ref() {
                Some(Pat::Ident(binding)) => (
                    Some(self.resolve_binding_name(binding.id.sym.as_ref())),
                    Vec::new(),
                ),
                None => (None, Vec::new()),
                Some(pattern) => {
                    let temporary_name = self.fresh_temporary_name("catch");
                    let mut setup = Vec::new();
                    self.lower_for_of_pattern_binding(
                        pattern,
                        Expression::Identifier(temporary_name.clone()),
                        ForOfPatternBindingKind::Lexical { mutable: true },
                        &mut setup,
                    )?;
                    (Some(temporary_name), setup)
                }
            };

            Ok((
                catch_binding,
                catch_setup,
                self.lower_statements(&handler.body.stmts, allow_return, allow_loop_control)?,
            ))
        })();
        self.pop_binding_scope();
        lowered
    }

    pub(crate) fn lower_expression_statement(
        &mut self,
        expression: &Expr,
    ) -> Result<Vec<Statement>> {
        if let Some(arguments) = console_log_arguments(expression) {
            return Ok(vec![Statement::Print {
                values: arguments
                    .iter()
                    .map(|argument| self.lower_expression(&argument.expr))
                    .collect::<Result<Vec<_>>>()?,
            }]);
        }

        if let Some(call) = assert_throws_call(expression) {
            return self.lower_assert_throws_statement(call);
        }

        if let Expr::Paren(parenthesized) = expression {
            return self.lower_expression_statement(&parenthesized.expr);
        }

        if let Expr::Seq(sequence) = expression {
            let mut statements = Vec::new();
            for expression in &sequence.exprs {
                statements.extend(self.lower_expression_statement(expression)?);
            }
            return Ok(statements);
        }

        if let Expr::Update(update) = expression {
            let op = lower_update_operator(update.op);
            match update.arg.as_ref() {
                Expr::Ident(identifier) => {
                    let name = self.resolve_binding_name(identifier.sym.as_ref());
                    return Ok(vec![Statement::Expression(Expression::Update {
                        name,
                        op,
                        prefix: update.prefix,
                    })]);
                }
                other => {
                    if let Some(name) = self.try_lower_top_level_this_member_update(other)? {
                        let target = AssignmentTarget::Member {
                            object: Expression::This,
                            property: Expression::String(name),
                        };
                        let value = Self::update_assignment_value(&target, op);
                        return Ok(vec![Statement::Expression(target.into_expression(value))]);
                    }

                    let target = self.lower_update_assignment_target(other)?;
                    let expression = if update.prefix {
                        self.lower_prefix_update_assignment_expression(target, op)?
                    } else {
                        self.lower_postfix_update_assignment_expression(target, op)?
                    };
                    return Ok(vec![Statement::Expression(expression)]);
                }
            }
        }

        if let Expr::Assign(assignment) = expression {
            if assignment.op == AssignOp::Assign
                && let swc_ecma_ast::AssignTarget::Pat(pattern) = &assignment.left
            {
                let value = self.lower_expression(&assignment.right)?;
                let value_name = self.fresh_temporary_name("destructure_value");
                let mut statements = vec![Statement::Let {
                    name: value_name.clone(),
                    mutable: true,
                    value,
                }];
                let pattern: Pat = pattern.clone().into();
                self.lower_for_of_pattern_binding(
                    &pattern,
                    Expression::Identifier(value_name),
                    ForOfPatternBindingKind::Assignment,
                    &mut statements,
                )?;
                return Ok(statements);
            }

            if assignment.op == AssignOp::Assign
                && let Some(nested_assignment) =
                    Self::nested_destructuring_assignment_expression(&assignment.right)
                && let swc_ecma_ast::AssignTarget::Pat(pattern) = &nested_assignment.left
            {
                let target = self.lower_assignment_target(&assignment.left)?;
                let value = self.lower_expression(&nested_assignment.right)?;
                let value_name = self.fresh_temporary_name("destructure_value");
                let mut statements = vec![Statement::Let {
                    name: value_name.clone(),
                    mutable: true,
                    value,
                }];
                let pattern: Pat = pattern.clone().into();
                self.lower_for_of_pattern_binding(
                    &pattern,
                    Expression::Identifier(value_name.clone()),
                    ForOfPatternBindingKind::Assignment,
                    &mut statements,
                )?;
                statements.push(target.into_statement(Expression::Identifier(value_name)));
                return Ok(statements);
            }

            let target_name_hint = self.assignment_target_name_hint(&assignment.left);
            let target = self.lower_assignment_target(&assignment.left)?;

            if assignment.op == AssignOp::Assign {
                let value = match target_name_hint.as_deref() {
                    Some(name_hint) => {
                        self.lower_expression_with_name_hint(&assignment.right, Some(name_hint))?
                    }
                    None => self.lower_expression(&assignment.right)?,
                };
                return Ok(vec![target.into_statement(value)]);
            }

            if matches!(
                assignment.op,
                AssignOp::AndAssign | AssignOp::OrAssign | AssignOp::NullishAssign
            ) {
                let right = match target_name_hint.as_deref() {
                    Some(name_hint) => {
                        self.lower_expression_with_name_hint(&assignment.right, Some(name_hint))?
                    }
                    None => self.lower_expression(&assignment.right)?,
                };
                let kind = match assignment.op {
                    AssignOp::AndAssign => LogicalAssignmentKind::And,
                    AssignOp::OrAssign => LogicalAssignmentKind::Or,
                    AssignOp::NullishAssign => LogicalAssignmentKind::Nullish,
                    _ => unreachable!("filtered above"),
                };
                return Ok(vec![Statement::Expression(
                    self.lower_logical_assignment_expression(target, right, kind)?,
                )]);
            }

            let operator = assignment
                .op
                .to_update()
                .context("unsupported assignment operator")?;

            let right = match target_name_hint.as_deref() {
                Some(name_hint) => {
                    self.lower_expression_with_name_hint(&assignment.right, Some(name_hint))?
                }
                None => self.lower_expression(&assignment.right)?,
            };
            let binary = match &target {
                AssignmentTarget::Identifier(name) => Expression::Binary {
                    op: lower_binary_operator(operator)?,
                    left: Box::new(Expression::Identifier(name.clone())),
                    right: Box::new(right),
                },
                AssignmentTarget::Member { object, property } => Expression::Binary {
                    op: lower_binary_operator(operator)?,
                    left: Box::new(Expression::Member {
                        object: Box::new(object.clone()),
                        property: Box::new(property.clone()),
                    }),
                    right: Box::new(right),
                },
                AssignmentTarget::SuperMember { property } => Expression::Binary {
                    op: lower_binary_operator(operator)?,
                    left: Box::new(Expression::SuperMember {
                        property: Box::new(property.clone()),
                    }),
                    right: Box::new(right),
                },
            };

            return Ok(vec![target.into_statement(binary)]);
        }

        Ok(vec![Statement::Expression(
            self.lower_expression(expression)?,
        )])
    }

    fn lower_assert_throws_inline_callback_statement(
        &mut self,
        statement: &Stmt,
    ) -> Result<Option<Vec<Statement>>> {
        match statement {
            Stmt::Expr(ExprStmt { expr, .. }) => Ok(Some(self.lower_expression_statement(expr)?)),
            Stmt::Return(return_statement) => Ok(Some(vec![Statement::Expression(
                match return_statement.arg.as_deref() {
                    Some(expression) => self.lower_expression(expression)?,
                    None => Expression::Undefined,
                },
            )])),
            Stmt::Throw(throw_statement) => Ok(Some(vec![Statement::Throw(
                self.lower_expression(&throw_statement.arg)?,
            )])),
            Stmt::Block(BlockStmt { stmts, .. }) if stmts.len() == 1 => {
                self.lower_assert_throws_inline_callback_statement(&stmts[0])
            }
            _ => Ok(None),
        }
    }

    fn lower_assert_throws_inline_function_body(
        &mut self,
        function: &Function,
    ) -> Result<Option<Vec<Statement>>> {
        if function.is_async || function.is_generator || !function.params.is_empty() {
            return Ok(None);
        }
        if !self.current_strict_mode() && function_has_use_strict_directive(function) {
            return Ok(None);
        }
        let Some(body) = function.body.as_ref() else {
            return Ok(None);
        };
        let scope_bindings = collect_static_block_scope_bindings(&body.stmts)?;
        self.push_renaming_binding_scope(scope_bindings);
        let lowered = if body.stmts.len() != 1 {
            self.lower_assert_throws_inline_callback_statement_list(&body.stmts)
        } else {
            self.lower_assert_throws_inline_callback_statement(&body.stmts[0])
        };
        self.pop_binding_scope();
        lowered
    }

    fn lower_assert_throws_inline_callback_statement_list(
        &mut self,
        statements: &[Stmt],
    ) -> Result<Option<Vec<Statement>>> {
        let mut lowered = Vec::new();
        for statement in statements {
            match statement {
                Stmt::Return(_) | Stmt::Break(_) | Stmt::Continue(_) | Stmt::Labeled(_) => {
                    return Ok(None);
                }
                _ => lowered.extend(self.lower_statement(statement, false, false)?),
            }
        }
        Ok(Some(lowered))
    }

    fn lower_assert_throws_inline_callback_body(
        &mut self,
        callback: &Expr,
    ) -> Result<Option<Vec<Statement>>> {
        match callback {
            Expr::Paren(parenthesized) => {
                self.lower_assert_throws_inline_callback_body(&parenthesized.expr)
            }
            Expr::Fn(function_expression) => {
                self.lower_assert_throws_inline_function_body(&function_expression.function)
            }
            Expr::Arrow(arrow_expression) => {
                if arrow_expression.is_async || !arrow_expression.params.is_empty() {
                    return Ok(None);
                }
                match arrow_expression.body.as_ref() {
                    BlockStmtOrExpr::BlockStmt(block)
                        if !self.current_strict_mode()
                            && script_has_use_strict_directive(&block.stmts) =>
                    {
                        Ok(None)
                    }
                    BlockStmtOrExpr::BlockStmt(block) if block.stmts.len() == 1 => {
                        self.lower_assert_throws_inline_callback_statement(&block.stmts[0])
                    }
                    BlockStmtOrExpr::BlockStmt(block) => {
                        self.lower_assert_throws_inline_callback_statement_list(&block.stmts)
                    }
                    BlockStmtOrExpr::Expr(expression) => {
                        Ok(Some(self.lower_expression_statement(expression)?))
                    }
                }
            }
            _ => Ok(None),
        }
    }

    fn assert_throws_callback_needs_backend_strict_inline(&self, callback: &Expr) -> bool {
        if self.current_strict_mode() {
            return false;
        }
        match callback {
            Expr::Paren(parenthesized) => {
                self.assert_throws_callback_needs_backend_strict_inline(&parenthesized.expr)
            }
            Expr::Fn(function_expression) => {
                let function = &function_expression.function;
                !function.is_async
                    && !function.is_generator
                    && function.params.is_empty()
                    && function_has_use_strict_directive(function)
            }
            Expr::Arrow(arrow_expression) => {
                !arrow_expression.is_async
                    && arrow_expression.params.is_empty()
                    && matches!(
                        arrow_expression.body.as_ref(),
                        BlockStmtOrExpr::BlockStmt(block)
                            if script_has_use_strict_directive(&block.stmts)
                    )
            }
            _ => false,
        }
    }

    fn assert_throws_callback_can_use_direct_try_call(callback: &Expr) -> bool {
        match callback {
            Expr::Paren(parenthesized) => {
                Self::assert_throws_callback_can_use_direct_try_call(&parenthesized.expr)
            }
            Expr::Ident(_) => true,
            _ => false,
        }
    }

    fn assert_throws_statement_contains_for_of(statement: &Stmt) -> bool {
        match statement {
            Stmt::ForOf(_) => true,
            Stmt::Block(block) => block
                .stmts
                .iter()
                .any(Self::assert_throws_statement_contains_for_of),
            Stmt::Labeled(labeled) => Self::assert_throws_statement_contains_for_of(&labeled.body),
            Stmt::If(if_statement) => {
                Self::assert_throws_statement_contains_for_of(&if_statement.cons)
                    || if_statement.alt.as_ref().is_some_and(|alternate| {
                        Self::assert_throws_statement_contains_for_of(alternate)
                    })
            }
            Stmt::Try(try_statement) => {
                try_statement
                    .block
                    .stmts
                    .iter()
                    .any(Self::assert_throws_statement_contains_for_of)
                    || try_statement.handler.as_ref().is_some_and(|handler| {
                        handler
                            .body
                            .stmts
                            .iter()
                            .any(Self::assert_throws_statement_contains_for_of)
                    })
                    || try_statement.finalizer.as_ref().is_some_and(|finalizer| {
                        finalizer
                            .stmts
                            .iter()
                            .any(Self::assert_throws_statement_contains_for_of)
                    })
            }
            _ => false,
        }
    }

    fn assert_throws_callback_contains_for_of(callback: &Expr) -> bool {
        match callback {
            Expr::Paren(parenthesized) => {
                Self::assert_throws_callback_contains_for_of(&parenthesized.expr)
            }
            Expr::Fn(function_expression) => function_expression
                .function
                .body
                .as_ref()
                .is_some_and(|body| {
                    body.stmts
                        .iter()
                        .any(Self::assert_throws_statement_contains_for_of)
                }),
            Expr::Arrow(arrow_expression) => match arrow_expression.body.as_ref() {
                BlockStmtOrExpr::BlockStmt(block) => block
                    .stmts
                    .iter()
                    .any(Self::assert_throws_statement_contains_for_of),
                BlockStmtOrExpr::Expr(_) => false,
            },
            _ => false,
        }
    }

    fn assert_throws_expected_builtin_error_argument(argument: &Expr, expected_name: &str) -> bool {
        match argument {
            Expr::Paren(parenthesized) => Self::assert_throws_expected_builtin_error_argument(
                &parenthesized.expr,
                expected_name,
            ),
            Expr::Ident(identifier) => identifier.sym.as_ref() == expected_name,
            _ => false,
        }
    }

    fn assert_throws_expected_type_error_argument(argument: &Expr) -> bool {
        Self::assert_throws_expected_builtin_error_argument(argument, "TypeError")
    }

    fn assert_throws_expected_syntax_error_argument(argument: &Expr) -> bool {
        Self::assert_throws_expected_builtin_error_argument(argument, "SyntaxError")
    }

    fn assert_throws_import_meta_value_expression(expression: &Expression) -> bool {
        matches!(
            expression,
            Expression::Call { callee, arguments }
                if arguments.is_empty()
                    && matches!(callee.as_ref(), Expression::Identifier(name) if name == "__ayyImportMeta")
        )
    }

    fn assert_throws_inline_body_statically_throws_type_error(body: &[Statement]) -> bool {
        let [Statement::Expression(expression)] = body else {
            return false;
        };
        match expression {
            Expression::Call { callee, .. } | Expression::New { callee, .. } => {
                Self::assert_throws_import_meta_value_expression(callee)
            }
            _ => false,
        }
    }

    fn assert_throws_function_constructor_import_meta_expression(expression: &Expression) -> bool {
        let (callee, arguments) = match expression {
            Expression::Call { callee, arguments } | Expression::New { callee, arguments } => {
                (callee.as_ref(), arguments.as_slice())
            }
            _ => return false,
        };
        if !matches!(
            callee,
            Expression::Identifier(name)
                if matches!(
                    name.as_str(),
                    "Function" | "AsyncFunction" | "GeneratorFunction" | "AsyncGeneratorFunction"
                )
        ) {
            return false;
        }
        arguments.iter().any(|argument| {
            matches!(
                argument,
                CallArgument::Expression(Expression::String(source))
                    if source.contains("import.meta")
            )
        })
    }

    fn assert_throws_inline_body_statically_throws_syntax_error(body: &[Statement]) -> bool {
        let [Statement::Expression(expression)] = body else {
            return false;
        };
        Self::assert_throws_function_constructor_import_meta_expression(expression)
    }

    pub(crate) fn lower_assert_throws_statement(
        &mut self,
        call: &swc_ecma_ast::CallExpr,
    ) -> Result<Vec<Statement>> {
        ensure!(
            call.args.len() >= 2,
            "__ayyAssertThrows expects at least two arguments"
        );
        ensure!(
            call.args.iter().all(|argument| argument.spread.is_none()),
            "__ayyAssertThrows does not support spread arguments"
        );

        if self.assert_throws_callback_needs_backend_strict_inline(&call.args[1].expr) {
            return Ok(vec![Statement::Expression(
                self.lower_expression(&Expr::Call(call.clone()))?,
            )]);
        }
        if Self::assert_throws_expected_type_error_argument(&call.args[0].expr)
            && Self::assert_throws_callback_contains_for_of(&call.args[1].expr)
        {
            return Ok(vec![Statement::Expression(
                self.lower_expression(&Expr::Call(call.clone()))?,
            )]);
        }

        let inline_body = self.lower_assert_throws_inline_callback_body(&call.args[1].expr)?;
        if Self::assert_throws_expected_type_error_argument(&call.args[0].expr)
            && inline_body.as_ref().is_some_and(|body| {
                Self::assert_throws_inline_body_statically_throws_type_error(body)
            })
        {
            let mut lowered = Vec::new();
            lowered.extend(self.lower_expression_statement(&call.args[0].expr)?);
            for argument in call.args.iter().skip(2) {
                lowered.extend(self.lower_expression_statement(&argument.expr)?);
            }
            return Ok(lowered);
        }
        if Self::assert_throws_expected_syntax_error_argument(&call.args[0].expr)
            && inline_body.as_ref().is_some_and(|body| {
                Self::assert_throws_inline_body_statically_throws_syntax_error(body)
            })
        {
            let mut lowered = Vec::new();
            lowered.extend(self.lower_expression_statement(&call.args[0].expr)?);
            for argument in call.args.iter().skip(2) {
                lowered.extend(self.lower_expression_statement(&argument.expr)?);
            }
            return Ok(lowered);
        }
        let caught_name = self.fresh_temporary_name("assert_throws_caught");

        let mut lowered = Vec::new();
        lowered.push(Statement::Let {
            name: caught_name.clone(),
            mutable: true,
            value: Expression::Bool(false),
        });
        let try_body = if let Some(inline_body) = inline_body {
            inline_body
        } else if Self::assert_throws_callback_can_use_direct_try_call(&call.args[1].expr) {
            vec![Statement::Expression(Expression::Call {
                callee: Box::new(self.lower_expression(&call.args[1].expr)?),
                arguments: Vec::new(),
            })]
        } else {
            let callback_name = self.fresh_temporary_name("assert_throws_callback");
            let callback_value =
                self.lower_expression_with_name_hint(&call.args[1].expr, Some(&callback_name))?;
            lowered.insert(
                0,
                Statement::Let {
                    name: callback_name.clone(),
                    mutable: false,
                    value: callback_value,
                },
            );
            vec![Statement::Expression(Expression::Call {
                callee: Box::new(Expression::Identifier(callback_name)),
                arguments: Vec::new(),
            })]
        };
        lowered.push(Statement::Try {
            body: try_body,
            catch_binding: None,
            catch_setup: Vec::new(),
            catch_body: vec![Statement::Assign {
                name: caught_name.clone(),
                value: Expression::Bool(true),
            }],
        });
        lowered.push(Statement::If {
            condition: Expression::Binary {
                op: BinaryOp::Equal,
                left: Box::new(Expression::Identifier(caught_name)),
                right: Box::new(Expression::Bool(false)),
            },
            then_branch: vec![Statement::Throw(Expression::Undefined)],
            else_branch: Vec::new(),
        });

        Ok(lowered)
    }

    pub(crate) fn lower_block_or_statement(
        &mut self,
        statement: &Stmt,
        allow_return: bool,
        allow_loop_control: bool,
    ) -> Result<Vec<Statement>> {
        match statement {
            Stmt::Block(BlockStmt { stmts, .. }) => Ok(vec![Statement::Block {
                body: self.lower_block_statements(stmts, allow_return, allow_loop_control)?,
            }]),
            other => self.lower_statement(other, allow_return, allow_loop_control),
        }
    }

    pub(crate) fn lower_optional_else(
        &mut self,
        statement: Option<&Stmt>,
        allow_return: bool,
        allow_loop_control: bool,
    ) -> Result<Vec<Statement>> {
        match statement {
            Some(statement) => {
                self.lower_block_or_statement(statement, allow_return, allow_loop_control)
            }
            None => Ok(Vec::new()),
        }
    }
}
