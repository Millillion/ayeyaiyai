use super::*;

impl Lowerer {
    pub(crate) fn lower_generator_statements(
        &mut self,
        statements: &[Stmt],
        allow_return: bool,
    ) -> Result<Vec<Statement>> {
        let mut lowered = Vec::new();
        let mut using_finalizer = Vec::new();

        for statement in statements {
            if let Stmt::Decl(Decl::Using(using_declaration)) = statement {
                let (mut prelude, mut using_bindings, mut finalizer) =
                    self.lower_using_declaration(using_declaration, false)?;
                lowered.append(&mut prelude);
                lowered.append(&mut using_bindings);
                using_finalizer.append(&mut finalizer);
            } else {
                lowered.extend(self.lower_generator_statement(statement, allow_return)?);
            }
        }

        using_finalizer.reverse();
        lowered.extend(using_finalizer);
        Ok(lowered)
    }

    pub(crate) fn lower_generator_statement(
        &mut self,
        statement: &Stmt,
        allow_return: bool,
    ) -> Result<Vec<Statement>> {
        match statement {
            Stmt::Expr(ExprStmt { expr, .. }) => {
                if let Some(lowered) = self.lower_generator_assignment_expression(expr)? {
                    return Ok(lowered);
                }

                if let Some(lowered) = self.lower_generator_effect_expression(expr)? {
                    return Ok(lowered);
                }

                self.lower_expression_statement(expr)
            }
            Stmt::Decl(Decl::Var(variable_declaration)) => {
                self.lower_generator_variable_declaration(variable_declaration)
            }
            Stmt::Decl(Decl::Class(class_declaration)) => {
                self.lower_generator_class_declaration(class_declaration)
            }
            Stmt::Block(BlockStmt { stmts, .. })
                if stmts.iter().all(|statement| {
                    matches!(
                        statement,
                        Stmt::Expr(_) | Stmt::Debugger(_) | Stmt::Empty(_)
                    )
                }) =>
            {
                self.lower_generator_statements(stmts, allow_return)
            }
            Stmt::For(for_statement) => Ok(vec![Statement::For {
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
                per_iteration_bindings: for_statement
                    .init
                    .as_ref()
                    .map(collect_for_per_iteration_bindings)
                    .transpose()?
                    .unwrap_or_default(),
                break_hook: None,
                body: self.lower_generator_loop_body(&for_statement.body, allow_return)?,
            }]),
            Stmt::ForOf(for_of_statement) => {
                self.lower_generator_for_of_statement(for_of_statement, allow_return)
            }
            Stmt::ForIn(for_in_statement) => {
                self.lower_for_in_statement(for_in_statement, allow_return)
            }
            Stmt::If(if_statement) => Ok(vec![Statement::If {
                condition: self.lower_expression(&if_statement.test)?,
                then_branch: self.lower_generator_branch(&if_statement.cons, allow_return)?,
                else_branch: if let Some(alternate) = &if_statement.alt {
                    self.lower_generator_branch(alternate, allow_return)?
                } else {
                    Vec::new()
                },
            }]),
            Stmt::DoWhile(do_while_statement) => Ok(vec![Statement::DoWhile {
                labels: Vec::new(),
                condition: self.lower_expression(&do_while_statement.test)?,
                break_hook: None,
                body: self.lower_generator_loop_body(&do_while_statement.body, allow_return)?,
            }]),
            Stmt::Labeled(labeled_statement) => {
                self.lower_labeled_statement(labeled_statement, allow_return, false)
            }
            Stmt::With(with_statement) => {
                self.lower_generator_with_statement(with_statement, allow_return)
            }
            Stmt::Try(try_statement) => {
                self.lower_generator_try_statement(try_statement, allow_return)
            }
            Stmt::Return(return_statement) => {
                ensure!(allow_return, "`return` is only supported inside functions");
                let Some(expression) = return_statement.arg.as_deref() else {
                    return Ok(vec![Statement::Return(Expression::Undefined)]);
                };
                if let Some((mut lowered, expression)) =
                    self.lower_generator_assignment_value(expression)?
                {
                    lowered.push(Statement::Return(expression));
                    return Ok(lowered);
                }
                Ok(vec![Statement::Return(self.lower_expression(expression)?)])
            }
            Stmt::Debugger(_) | Stmt::Empty(_) => Ok(Vec::new()),
            other => self.lower_statement(other, allow_return, false),
        }
    }

    pub(crate) fn lower_generator_variable_declaration(
        &mut self,
        variable_declaration: &swc_ecma_ast::VarDecl,
    ) -> Result<Vec<Statement>> {
        let mut lowered = Vec::new();

        for declarator in &variable_declaration.decls {
            let name_hint = pattern_name_hint(&declarator.name);
            let generator_value = declarator
                .init
                .as_deref()
                .map(|initializer| {
                    self.lower_generator_assignment_value_with_name_hint(initializer, name_hint)
                })
                .transpose()?
                .flatten();

            if let Pat::Ident(identifier) = &declarator.name {
                let name = self.resolve_binding_name(identifier.id.sym.as_ref());
                let value = if let Some((mut generator_prefix, value)) = generator_value {
                    lowered.append(&mut generator_prefix);
                    value
                } else {
                    match declarator.init.as_deref() {
                        Some(initializer) => self.lower_expression_with_name_hint(
                            initializer,
                            Some(identifier.id.sym.as_ref()),
                        )?,
                        None => Expression::Undefined,
                    }
                };

                if matches!(variable_declaration.kind, VarDeclKind::Var) {
                    lowered.push(Statement::Var { name, value });
                } else {
                    lowered.push(Statement::Let {
                        name,
                        mutable: !matches!(variable_declaration.kind, VarDeclKind::Const),
                        value,
                    });
                }
                continue;
            }

            if matches!(variable_declaration.kind, VarDeclKind::Var) {
                let mut names = Vec::new();
                collect_pattern_binding_names(&declarator.name, &mut names)?;
                for name in names {
                    lowered.push(Statement::Var {
                        name,
                        value: Expression::Undefined,
                    });
                }
            }

            let temporary_name = self.fresh_temporary_name("decl");
            let value = if let Some((mut generator_prefix, value)) = generator_value {
                lowered.append(&mut generator_prefix);
                value
            } else {
                match declarator.init.as_deref() {
                    Some(initializer) => self.lower_expression_with_name_hint(
                        initializer,
                        pattern_name_hint(&declarator.name),
                    )?,
                    None => Expression::Undefined,
                }
            };
            lowered.push(Statement::Let {
                name: temporary_name.clone(),
                mutable: true,
                value,
            });
            self.lower_for_of_pattern_binding(
                &declarator.name,
                Expression::Identifier(temporary_name),
                if matches!(variable_declaration.kind, VarDeclKind::Var) {
                    ForOfPatternBindingKind::Assignment
                } else {
                    ForOfPatternBindingKind::Lexical {
                        mutable: !matches!(variable_declaration.kind, VarDeclKind::Const),
                    }
                },
                &mut lowered,
            )?;
        }

        Ok(lowered)
    }

    pub(crate) fn lower_generator_try_statement(
        &mut self,
        try_statement: &swc_ecma_ast::TryStmt,
        allow_return: bool,
    ) -> Result<Vec<Statement>> {
        let lowered_body =
            self.lower_generator_scoped_statements(&try_statement.block.stmts, allow_return)?;
        let lowered_handler = try_statement
            .handler
            .as_ref()
            .map(|handler| self.lower_generator_catch_clause(handler, allow_return))
            .transpose()?;

        if let Some(finalizer) = &try_statement.finalizer {
            let threw_name = self.fresh_temporary_name("finally_threw");
            let error_name = self.fresh_temporary_name("finally_error");
            let outer_catch_name = self.fresh_temporary_name("finally_catch");
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
            ];

            let protected_body =
                if let Some((catch_binding, catch_setup, catch_body)) = lowered_handler {
                    vec![Statement::Try {
                        body: lowered_body,
                        catch_binding,
                        catch_setup,
                        catch_body,
                    }]
                } else {
                    lowered_body
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
            statements
                .extend(self.lower_generator_scoped_statements(&finalizer.stmts, allow_return)?);
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

    fn lower_generator_scoped_statements(
        &mut self,
        statements: &[Stmt],
        allow_return: bool,
    ) -> Result<Vec<Statement>> {
        let scope_bindings = collect_direct_statement_lexical_bindings(statements)?;
        self.push_renaming_binding_scope(scope_bindings);
        let lowered = self.lower_generator_statements(statements, allow_return);
        self.pop_binding_scope();
        lowered
    }

    pub(crate) fn lower_generator_catch_clause(
        &mut self,
        handler: &swc_ecma_ast::CatchClause,
        allow_return: bool,
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
                self.lower_generator_scoped_statements(&handler.body.stmts, allow_return)?,
            ))
        })();
        self.pop_binding_scope();
        lowered
    }

    pub(crate) fn lower_generator_loop_body(
        &mut self,
        statement: &Stmt,
        allow_return: bool,
    ) -> Result<Vec<Statement>> {
        match statement {
            Stmt::Block(BlockStmt { stmts, .. }) => {
                self.lower_generator_statements(stmts, allow_return)
            }
            other => self.lower_generator_statement(other, allow_return),
        }
    }

    pub(crate) fn lower_generator_branch(
        &mut self,
        statement: &Stmt,
        allow_return: bool,
    ) -> Result<Vec<Statement>> {
        match statement {
            Stmt::Block(BlockStmt { stmts, .. }) => {
                self.lower_generator_statements(stmts, allow_return)
            }
            other => self.lower_generator_statement(other, allow_return),
        }
    }

    pub(crate) fn lower_generator_with_statement(
        &mut self,
        with_statement: &WithStmt,
        allow_return: bool,
    ) -> Result<Vec<Statement>> {
        let Expr::Object(object) = &*with_statement.obj else {
            return Ok(vec![Statement::With {
                object: self.lower_expression(&with_statement.obj)?,
                body: self.lower_generator_loop_body(&with_statement.body, allow_return)?,
            }]);
        };

        let mut bindings = HashMap::new();
        for property in &object.props {
            match property {
                PropOrSpread::Prop(property) => match &**property {
                    Prop::KeyValue(property) => {
                        let key = match &property.key {
                            PropName::Ident(identifier) => identifier.sym.to_string(),
                            PropName::Str(string) => string.value.to_string_lossy().into_owned(),
                            _ => bail!("unsupported `with` property key"),
                        };
                        bindings.insert(key, self.lower_expression(&property.value)?);
                    }
                    _ => bail!("unsupported `with` object property"),
                },
                PropOrSpread::Spread(_) => bail!("unsupported `with` object spread"),
            }
        }

        self.lower_generator_with_body(&with_statement.body, allow_return, &bindings)
    }

    pub(crate) fn lower_generator_with_body(
        &mut self,
        statement: &Stmt,
        allow_return: bool,
        bindings: &HashMap<String, Expression>,
    ) -> Result<Vec<Statement>> {
        match statement {
            Stmt::Block(BlockStmt { stmts, .. }) => {
                let mut lowered = Vec::new();
                for statement in stmts {
                    lowered.extend(self.lower_generator_with_body(
                        statement,
                        allow_return,
                        bindings,
                    )?);
                }
                Ok(lowered)
            }
            Stmt::Expr(ExprStmt { expr, .. }) => {
                self.lower_generator_with_expression(expr, bindings)
            }
            Stmt::Empty(_) => Ok(Vec::new()),
            Stmt::Return(return_statement) => {
                ensure!(allow_return, "`return` is only supported inside functions");
                Ok(vec![Statement::Return(
                    match return_statement.arg.as_deref() {
                        Some(expression) => {
                            self.lower_expression_with_generator_bindings(expression, bindings)?
                        }
                        None => Expression::Undefined,
                    },
                )])
            }
            _ => bail!("unsupported statement inside generator `with`"),
        }
    }

    pub(crate) fn lower_generator_with_expression(
        &mut self,
        expression: &Expr,
        bindings: &HashMap<String, Expression>,
    ) -> Result<Vec<Statement>> {
        let Expr::Yield(yield_expression) = expression else {
            bail!("unsupported expression inside generator `with`")
        };

        if yield_expression.delegate {
            let value = yield_expression
                .arg
                .as_deref()
                .context("`yield*` requires an operand")?;
            return Ok(vec![Statement::YieldDelegate {
                value: self.lower_expression_with_generator_bindings(value, bindings)?,
            }]);
        }

        match yield_expression.arg.as_deref() {
            Some(Expr::Yield(inner_yield)) => {
                ensure!(
                    !inner_yield.delegate,
                    "`yield*` as the operand of another `yield` is not supported yet"
                );
                Ok(vec![
                    Statement::Yield {
                        value: match inner_yield.arg.as_deref() {
                            Some(value) => {
                                self.lower_expression_with_generator_bindings(value, bindings)?
                            }
                            None => Expression::Undefined,
                        },
                    },
                    Statement::Yield {
                        value: Expression::Sent,
                    },
                ])
            }
            Some(value) => Ok(vec![Statement::Yield {
                value: self.lower_expression_with_generator_bindings(value, bindings)?,
            }]),
            None => Ok(vec![Statement::Yield {
                value: Expression::Undefined,
            }]),
        }
    }

    pub(crate) fn lower_expression_with_generator_bindings(
        &mut self,
        expression: &Expr,
        bindings: &HashMap<String, Expression>,
    ) -> Result<Expression> {
        match expression {
            Expr::Ident(identifier) => Ok(bindings
                .get(identifier.sym.as_ref())
                .cloned()
                .unwrap_or(Expression::Identifier(identifier.sym.to_string()))),
            _ => self.lower_expression(expression),
        }
    }

    pub(crate) fn lower_generator_assignment_expression(
        &mut self,
        expression: &Expr,
    ) -> Result<Option<Vec<Statement>>> {
        let Expr::Assign(assignment) = expression else {
            return Ok(None);
        };

        if assignment.op != AssignOp::Assign {
            return Ok(None);
        }

        if let AssignTarget::Pat(pattern) = &assignment.left {
            let mut lowered = Vec::new();
            let value = if let Some((mut prefix, value)) =
                self.lower_generator_assignment_value(&assignment.right)?
            {
                lowered.append(&mut prefix);
                value
            } else {
                self.lower_expression(&assignment.right)?
            };
            let value_name = self.fresh_temporary_name("destructure_value");
            lowered.push(Statement::Let {
                name: value_name.clone(),
                mutable: true,
                value,
            });
            let pattern: Pat = pattern.clone().into();
            self.lower_for_of_pattern_binding_with_generator_defaults(
                &pattern,
                Expression::Identifier(value_name),
                ForOfPatternBindingKind::Assignment,
                &mut lowered,
                true,
            )?;
            return Ok(Some(lowered));
        }

        let target_name_hint = self.assignment_target_name_hint(&assignment.left);
        let target = self.lower_generator_assignment_target(&assignment.left)?;
        let value = self.lower_generator_assignment_value_with_name_hint(
            &assignment.right,
            target_name_hint.as_deref(),
        )?;

        if target.is_none() && value.is_none() {
            return Ok(None);
        }

        let mut lowered = Vec::new();
        let target = if let Some((mut prefix, target)) = target {
            lowered.append(&mut prefix);
            target
        } else {
            self.lower_assignment_target(&assignment.left)?
        };
        let value = if let Some((mut prefix, value)) = value {
            lowered.append(&mut prefix);
            value
        } else {
            self.lower_expression_with_name_hint(&assignment.right, target_name_hint.as_deref())?
        };
        lowered.push(target.into_statement(value));
        Ok(Some(lowered))
    }

    pub(crate) fn lower_generator_assignment_value(
        &mut self,
        expression: &Expr,
    ) -> Result<Option<(Vec<Statement>, Expression)>> {
        self.lower_generator_assignment_value_with_name_hint(expression, None)
    }

    pub(crate) fn lower_generator_assignment_value_with_name_hint(
        &mut self,
        expression: &Expr,
        name_hint: Option<&str>,
    ) -> Result<Option<(Vec<Statement>, Expression)>> {
        match expression {
            Expr::Yield(yield_expression) => {
                Ok(Some(self.lower_generator_yield_value(yield_expression)?))
            }
            Expr::Paren(parenthesized) => {
                self.lower_generator_assignment_value_with_name_hint(&parenthesized.expr, name_hint)
            }
            Expr::Tpl(template) => self.lower_generator_template_value(template),
            _ => self.lower_generator_nested_yield_value_with_name_hint(expression, name_hint),
        }
    }

    fn lower_generator_yield_value(
        &mut self,
        yield_expression: &swc_ecma_ast::YieldExpr,
    ) -> Result<(Vec<Statement>, Expression)> {
        let mut lowered = self.lower_generator_yield_statement(yield_expression)?;
        let temporary_name = self.fresh_temporary_name("generator_sent");
        lowered.push(Statement::Let {
            name: temporary_name.clone(),
            mutable: false,
            value: Expression::Sent,
        });
        Ok((lowered, Expression::Identifier(temporary_name)))
    }

    pub(crate) fn lower_generator_effect_expression(
        &mut self,
        expression: &Expr,
    ) -> Result<Option<Vec<Statement>>> {
        match expression {
            Expr::Yield(yield_expression) => Ok(Some(
                self.lower_generator_yield_statement(yield_expression)?,
            )),
            Expr::Paren(parenthesized) => {
                self.lower_generator_effect_expression(&parenthesized.expr)
            }
            Expr::Seq(sequence) => {
                let mut lowered = Vec::new();
                let mut handled = false;
                for expression in &sequence.exprs {
                    if let Some(mut expression_lowered) =
                        self.lower_generator_effect_expression(expression)?
                    {
                        lowered.append(&mut expression_lowered);
                        handled = true;
                    } else {
                        lowered.extend(self.lower_expression_statement(expression)?);
                    }
                }
                Ok(handled.then_some(lowered))
            }
            Expr::Array(array) => {
                let mut lowered = Vec::new();
                let mut handled = false;
                for element in array.elems.iter().flatten() {
                    if let Some(mut expression_lowered) =
                        self.lower_generator_effect_expression(&element.expr)?
                    {
                        lowered.append(&mut expression_lowered);
                        handled = true;
                    } else {
                        lowered.extend(self.lower_expression_statement(&element.expr)?);
                    }
                }
                Ok(handled.then_some(lowered))
            }
            Expr::Cond(conditional) => {
                let Some((mut lowered, condition)) =
                    self.lower_generator_assignment_value(&conditional.test)?
                else {
                    return Ok(None);
                };
                let then_expression = self.lower_generator_effect_yield_value(&conditional.cons)?;
                let else_expression = self.lower_generator_effect_yield_value(&conditional.alt)?;
                lowered.push(Statement::Yield {
                    value: Expression::Conditional {
                        condition: Box::new(condition),
                        then_expression: Box::new(then_expression),
                        else_expression: Box::new(else_expression),
                    },
                });
                Ok(Some(lowered))
            }
            Expr::Bin(binary) => {
                let left_lowered = self.lower_generator_assignment_value(&binary.left)?;
                let right_lowered = self.lower_generator_assignment_value(&binary.right)?;

                if left_lowered.is_none() && right_lowered.is_none() {
                    return Ok(None);
                }

                let mut lowered = Vec::new();
                let mut left = match left_lowered {
                    Some((mut statements, expression)) => {
                        lowered.append(&mut statements);
                        expression
                    }
                    None => self.lower_expression(&binary.left)?,
                };

                if right_lowered.is_some() {
                    let temporary = self.fresh_temporary_name("generator_bin_left");
                    lowered.push(Statement::Let {
                        name: temporary.clone(),
                        mutable: false,
                        value: left,
                    });
                    left = Expression::Identifier(temporary);
                }

                let right = match right_lowered {
                    Some((mut statements, expression)) => {
                        lowered.append(&mut statements);
                        expression
                    }
                    None => self.lower_expression(&binary.right)?,
                };

                lowered.push(Statement::Expression(Expression::Binary {
                    op: lower_binary_operator(binary.op)?,
                    left: Box::new(left),
                    right: Box::new(right),
                }));
                Ok(Some(lowered))
            }
            _ => {
                if let Some((mut lowered, expression)) =
                    self.lower_generator_nested_yield_value(expression)?
                {
                    lowered.push(Statement::Expression(expression));
                    Ok(Some(lowered))
                } else {
                    Ok(None)
                }
            }
        }
    }

    pub(crate) fn lower_generator_effect_yield_value(
        &mut self,
        expression: &Expr,
    ) -> Result<Expression> {
        let Expr::Yield(yield_expression) = expression else {
            bail!("unsupported generator effect expression")
        };
        ensure!(
            !yield_expression.delegate,
            "`yield*` is not supported in generator effect branches yet"
        );
        match yield_expression.arg.as_deref() {
            Some(value) => self.lower_expression(value),
            None => Ok(Expression::Undefined),
        }
    }

    pub(crate) fn lower_generator_template_value(
        &mut self,
        template: &swc_ecma_ast::Tpl,
    ) -> Result<Option<(Vec<Statement>, Expression)>> {
        let mut yield_index = None;
        let mut yield_expression = None;

        for (index, expression) in template.exprs.iter().enumerate() {
            if let Expr::Yield(candidate) = &**expression {
                ensure!(
                    yield_index.is_none(),
                    "multiple yield expressions in template literals are not supported yet"
                );
                yield_index = Some(index);
                yield_expression = Some(candidate);
            }
        }

        let Some(yield_index) = yield_index else {
            return Ok(None);
        };

        let (lowered, substitution) = self
            .lower_generator_yield_value(yield_expression.expect("yield expression must exist"))?;
        let expression =
            self.lower_template_expression_with_substitution(template, yield_index, substitution)?;
        Ok(Some((lowered, expression)))
    }

    pub(super) fn lower_generator_nested_yield_value(
        &mut self,
        expression: &Expr,
    ) -> Result<Option<(Vec<Statement>, Expression)>> {
        self.lower_generator_nested_yield_value_with_name_hint(expression, None)
    }

    fn lower_generator_nested_yield_value_with_name_hint(
        &mut self,
        expression: &Expr,
        name_hint: Option<&str>,
    ) -> Result<Option<(Vec<Statement>, Expression)>> {
        match expression {
            Expr::Yield(yield_expression) => {
                Ok(Some(self.lower_generator_yield_value(yield_expression)?))
            }
            Expr::Paren(parenthesized) => self
                .lower_generator_nested_yield_value_with_name_hint(&parenthesized.expr, name_hint),
            Expr::Array(array) => {
                let mut lowered = Vec::new();
                let mut handled = false;
                let mut elements = Vec::with_capacity(array.elems.len());
                for element in &array.elems {
                    let Some(element) = element else {
                        elements.push(ArrayElement::Expression(Expression::Undefined));
                        continue;
                    };
                    let expression = if let Some((mut nested, expression)) =
                        self.lower_generator_nested_yield_value(&element.expr)?
                    {
                        handled = true;
                        lowered.append(&mut nested);
                        expression
                    } else {
                        self.lower_expression(&element.expr)?
                    };
                    elements.push(if element.spread.is_some() {
                        ArrayElement::Spread(expression)
                    } else {
                        ArrayElement::Expression(expression)
                    });
                }
                Ok(handled.then_some((lowered, Expression::Array(elements))))
            }
            Expr::Object(object) => {
                let mut lowered = Vec::new();
                let mut handled = false;
                let mut entries = Vec::with_capacity(object.props.len());
                for property in &object.props {
                    match property {
                        PropOrSpread::Spread(spread) => {
                            let expression = if let Some((mut nested, expression)) =
                                self.lower_generator_nested_yield_value(&spread.expr)?
                            {
                                handled = true;
                                lowered.append(&mut nested);
                                expression
                            } else {
                                self.lower_expression(&spread.expr)?
                            };
                            entries.push(ObjectEntry::Spread(expression));
                        }
                        _ => entries.push(self.lower_object_entry(property)?),
                    }
                }
                Ok(handled.then_some((lowered, Expression::Object(entries))))
            }
            Expr::Member(member) => {
                let mut lowered = Vec::new();
                let mut handled = false;
                let object = if let Some((mut nested, object)) =
                    self.lower_generator_nested_yield_value(&member.obj)?
                {
                    handled = true;
                    lowered.append(&mut nested);
                    object
                } else {
                    self.lower_expression(&member.obj)?
                };
                let property = match &member.prop {
                    MemberProp::Ident(identifier) => Expression::String(identifier.sym.to_string()),
                    MemberProp::PrivateName(private_name) => {
                        self.lower_private_name(private_name)?
                    }
                    MemberProp::Computed(computed) => {
                        if let Some((mut nested, property)) =
                            self.lower_generator_nested_yield_value(&computed.expr)?
                        {
                            handled = true;
                            lowered.append(&mut nested);
                            property
                        } else {
                            self.lower_expression(&computed.expr)?
                        }
                    }
                };
                Ok(handled.then_some((
                    lowered,
                    Expression::Member {
                        object: Box::new(object),
                        property: Box::new(property),
                    },
                )))
            }
            Expr::Call(call) => {
                let mut lowered = Vec::new();
                let mut handled = false;
                let callee = match &call.callee {
                    Callee::Expr(callee) => {
                        let callee = if let Some((mut nested, callee)) =
                            self.lower_generator_nested_yield_value(callee)?
                        {
                            handled = true;
                            lowered.append(&mut nested);
                            callee
                        } else {
                            self.lower_expression(callee)?
                        };
                        Expression::Call {
                            callee: Box::new(callee),
                            arguments: call
                                .args
                                .iter()
                                .map(|argument| {
                                    let expression = if let Some((mut nested, expression)) =
                                        self.lower_generator_nested_yield_value(&argument.expr)?
                                    {
                                        handled = true;
                                        lowered.append(&mut nested);
                                        expression
                                    } else {
                                        self.lower_expression(&argument.expr)?
                                    };
                                    Ok(if argument.spread.is_some() {
                                        CallArgument::Spread(expression)
                                    } else {
                                        CallArgument::Expression(expression)
                                    })
                                })
                                .collect::<Result<Vec<_>>>()?,
                        }
                    }
                    Callee::Super(_) | Callee::Import(_) => return Ok(None),
                };
                Ok(handled.then_some((lowered, callee)))
            }
            Expr::Assign(assignment) if assignment.op == AssignOp::Assign => {
                if let AssignTarget::Pat(pattern) = &assignment.left {
                    let mut lowered = Vec::new();
                    let value = if let Some((mut nested, value)) =
                        self.lower_generator_nested_yield_value(&assignment.right)?
                    {
                        lowered.append(&mut nested);
                        value
                    } else {
                        self.lower_expression(&assignment.right)?
                    };
                    let value_name = self.fresh_temporary_name("destructure_value");
                    lowered.push(Statement::Let {
                        name: value_name.clone(),
                        mutable: true,
                        value,
                    });
                    let pattern: Pat = pattern.clone().into();
                    self.lower_for_of_pattern_binding_with_generator_defaults(
                        &pattern,
                        Expression::Identifier(value_name.clone()),
                        ForOfPatternBindingKind::Assignment,
                        &mut lowered,
                        true,
                    )?;
                    return Ok(Some((lowered, Expression::Identifier(value_name))));
                }

                let mut lowered = Vec::new();
                let mut handled = false;
                let target = if let Some((mut nested, target)) =
                    self.lower_generator_assignment_target(&assignment.left)?
                {
                    handled = true;
                    lowered.append(&mut nested);
                    target
                } else {
                    self.lower_assignment_target(&assignment.left)?
                };
                let target_name_hint = self.assignment_target_name_hint(&assignment.left);
                let value = if let Some((mut nested, value)) = self
                    .lower_generator_nested_yield_value_with_name_hint(
                        &assignment.right,
                        target_name_hint.as_deref(),
                    )? {
                    handled = true;
                    lowered.append(&mut nested);
                    value
                } else {
                    self.lower_expression_with_name_hint(
                        &assignment.right,
                        target_name_hint.as_deref(),
                    )?
                };
                Ok(handled.then_some((lowered, target.into_expression(value))))
            }
            Expr::Class(class_expression) => {
                let (lowered, expression) =
                    self.lower_generator_class_expression(class_expression, name_hint)?;
                Ok(Some((lowered, expression)))
            }
            _ => Ok(None),
        }
    }

    fn lower_generator_assignment_target(
        &mut self,
        target: &AssignTarget,
    ) -> Result<Option<(Vec<Statement>, AssignmentTarget)>> {
        match target {
            AssignTarget::Simple(SimpleAssignTarget::Ident(_))
            | AssignTarget::Simple(SimpleAssignTarget::SuperProp(_)) => Ok(None),
            AssignTarget::Simple(SimpleAssignTarget::Member(member)) => {
                let mut lowered = Vec::new();
                let mut handled = false;
                let object = if let Some((mut nested, object)) =
                    self.lower_generator_nested_yield_value(&member.obj)?
                {
                    handled = true;
                    lowered.append(&mut nested);
                    object
                } else {
                    self.lower_expression(&member.obj)?
                };
                let property = match &member.prop {
                    MemberProp::Ident(identifier) => Expression::String(identifier.sym.to_string()),
                    MemberProp::PrivateName(private_name) => {
                        self.lower_private_name(private_name)?
                    }
                    MemberProp::Computed(computed) => {
                        if let Some((mut nested, property)) =
                            self.lower_generator_nested_yield_value(&computed.expr)?
                        {
                            handled = true;
                            lowered.append(&mut nested);
                            property
                        } else {
                            self.lower_expression(&computed.expr)?
                        }
                    }
                };
                Ok(handled.then_some((lowered, AssignmentTarget::Member { object, property })))
            }
            _ => Ok(None),
        }
    }

    pub(crate) fn lower_generator_yield_statement(
        &mut self,
        yield_expression: &swc_ecma_ast::YieldExpr,
    ) -> Result<Vec<Statement>> {
        if yield_expression.delegate {
            let value = yield_expression
                .arg
                .as_deref()
                .context("`yield*` requires an operand")?;
            return Ok(vec![Statement::YieldDelegate {
                value: self.lower_expression(value)?,
            }]);
        }

        match yield_expression.arg.as_deref() {
            None => Ok(vec![Statement::Yield {
                value: Expression::Undefined,
            }]),
            Some(value) => {
                if let Some((mut lowered, expression)) =
                    self.lower_generator_nested_yield_value(value)?
                {
                    lowered.push(Statement::Yield { value: expression });
                    return Ok(lowered);
                }
                Ok(vec![Statement::Yield {
                    value: self.lower_expression(value)?,
                }])
            }
        }
    }
}
