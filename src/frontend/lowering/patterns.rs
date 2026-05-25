use super::*;

impl Lowerer {
    pub(super) fn lower_for_of_binding(
        &mut self,
        head: &ForHead,
        value: Expression,
    ) -> Result<ForOfBinding> {
        self.lower_for_of_binding_with_generator_defaults(head, value, false)
    }

    pub(super) fn lower_generator_for_of_binding(
        &mut self,
        head: &ForHead,
        value: Expression,
    ) -> Result<ForOfBinding> {
        self.lower_for_of_binding_with_generator_defaults(head, value, true)
    }

    fn lower_for_of_binding_with_generator_defaults(
        &mut self,
        head: &ForHead,
        value: Expression,
        generator_body: bool,
    ) -> Result<ForOfBinding> {
        match head {
            ForHead::VarDecl(variable_declaration) => {
                ensure!(
                    variable_declaration.decls.len() == 1,
                    "for-of declarations with multiple bindings are not supported yet"
                );
                let pattern = &variable_declaration.decls[0].name;
                let mut binding = ForOfBinding {
                    before_loop: Vec::new(),
                    per_iteration: Vec::new(),
                };
                let binding_kind = match variable_declaration.kind {
                    VarDeclKind::Var => ForOfPatternBindingKind::Var,
                    VarDeclKind::Let => ForOfPatternBindingKind::Lexical { mutable: true },
                    VarDeclKind::Const => ForOfPatternBindingKind::Lexical { mutable: false },
                };

                if matches!(variable_declaration.kind, VarDeclKind::Var) {
                    let mut names = Vec::new();
                    collect_for_of_binding_names(pattern, &mut names)?;
                    binding.before_loop = names
                        .into_iter()
                        .map(|name| Statement::Var {
                            name,
                            value: Expression::Undefined,
                        })
                        .collect();
                }

                self.lower_for_of_pattern_binding_with_generator_defaults(
                    pattern,
                    value,
                    binding_kind,
                    &mut binding.per_iteration,
                    generator_body,
                )?;

                Ok(binding)
            }
            ForHead::Pat(pattern) => {
                let mut binding = ForOfBinding {
                    before_loop: Vec::new(),
                    per_iteration: Vec::new(),
                };
                self.lower_for_of_pattern_binding_with_generator_defaults(
                    pattern,
                    value,
                    ForOfPatternBindingKind::Assignment,
                    &mut binding.per_iteration,
                    generator_body,
                )?;
                Ok(binding)
            }
            ForHead::UsingDecl(using_declaration) => {
                ensure!(
                    using_declaration.decls.len() == 1,
                    "for-of using declarations with multiple bindings are not supported yet"
                );
                let pattern = &using_declaration.decls[0].name;
                let mut binding = ForOfBinding {
                    before_loop: Vec::new(),
                    per_iteration: Vec::new(),
                };
                self.lower_for_of_pattern_binding_with_generator_defaults(
                    pattern,
                    value,
                    ForOfPatternBindingKind::Lexical { mutable: false },
                    &mut binding.per_iteration,
                    generator_body,
                )?;
                Ok(binding)
            }
        }
    }

    pub(super) fn lower_for_of_pattern_binding(
        &mut self,
        pattern: &Pat,
        value: Expression,
        binding_kind: ForOfPatternBindingKind,
        statements: &mut Vec<Statement>,
    ) -> Result<()> {
        self.lower_for_of_pattern_binding_with_generator_defaults(
            pattern,
            value,
            binding_kind,
            statements,
            false,
        )
    }

    pub(super) fn lower_for_of_pattern_binding_with_generator_defaults(
        &mut self,
        pattern: &Pat,
        value: Expression,
        binding_kind: ForOfPatternBindingKind,
        statements: &mut Vec<Statement>,
        generator_body: bool,
    ) -> Result<()> {
        match pattern {
            Pat::Ident(identifier) => {
                let name = self.resolve_binding_name(identifier.id.sym.as_ref());
                statements.push(match binding_kind {
                    ForOfPatternBindingKind::Var => Statement::Var { name, value },
                    ForOfPatternBindingKind::Assignment => Statement::Assign { name, value },
                    ForOfPatternBindingKind::Lexical { mutable } => Statement::Let {
                        name,
                        mutable,
                        value,
                    },
                })
            }
            Pat::Expr(expression) => {
                ensure!(
                    matches!(binding_kind, ForOfPatternBindingKind::Assignment),
                    "unsupported declaration binding pattern"
                );
                let target = self.lower_for_of_expression_target_with_generator_defaults(
                    expression,
                    generator_body,
                    statements,
                )?;
                statements.push(target.into_statement(value));
            }
            Pat::Assign(assign) => {
                let temporary_name = self.fresh_temporary_name("binding_value");
                let generator_default = if generator_body {
                    self.lower_generator_assignment_value_with_name_hint(
                        &assign.right,
                        pattern_name_hint(&assign.left),
                    )?
                } else {
                    None
                };
                let default_value = if let Some((_, value)) = &generator_default {
                    value.clone()
                } else {
                    self.lower_expression_with_name_hint(
                        &assign.right,
                        pattern_name_hint(&assign.left),
                    )?
                };
                if let Pat::Ident(ident) = assign.left.as_ref() {
                    let name = self.resolve_binding_name(ident.id.sym.as_ref());
                    if let Some((mut generator_prefix, default_value)) = generator_default {
                        statements.push(Statement::Let {
                            name: temporary_name.clone(),
                            mutable: true,
                            value,
                        });
                        let then_value = Expression::Identifier(temporary_name.clone());
                        let then_branch = vec![match binding_kind {
                            ForOfPatternBindingKind::Var => Statement::Var {
                                name: name.clone(),
                                value: then_value,
                            },
                            ForOfPatternBindingKind::Assignment => Statement::Assign {
                                name: name.clone(),
                                value: then_value,
                            },
                            ForOfPatternBindingKind::Lexical { mutable } => Statement::Let {
                                name: name.clone(),
                                mutable,
                                value: then_value,
                            },
                        }];
                        let mut else_branch = Vec::new();
                        else_branch.append(&mut generator_prefix);
                        else_branch.push(match binding_kind {
                            ForOfPatternBindingKind::Var => Statement::Var {
                                name,
                                value: default_value,
                            },
                            ForOfPatternBindingKind::Assignment => Statement::Assign {
                                name,
                                value: default_value,
                            },
                            ForOfPatternBindingKind::Lexical { mutable } => Statement::Let {
                                name,
                                mutable,
                                value: default_value,
                            },
                        });
                        statements.push(Statement::If {
                            condition: Expression::Binary {
                                op: BinaryOp::NotEqual,
                                left: Box::new(Expression::Identifier(temporary_name)),
                                right: Box::new(Expression::Undefined),
                            },
                            then_branch,
                            else_branch,
                        });
                        return Ok(());
                    }

                    if matches!(value, Expression::Member { .. })
                        && matches!(
                            binding_kind,
                            ForOfPatternBindingKind::Var | ForOfPatternBindingKind::Assignment
                        )
                    {
                        if matches!(binding_kind, ForOfPatternBindingKind::Var) {
                            statements.push(Statement::Var {
                                name: name.clone(),
                                value: Expression::Undefined,
                            });
                        }
                        statements.push(Statement::Let {
                            name: temporary_name.clone(),
                            mutable: true,
                            value: Expression::Undefined,
                        });
                        statements.push(Statement::Assign {
                            name,
                            value: Expression::Conditional {
                                condition: Box::new(Expression::Binary {
                                    op: BinaryOp::NotEqual,
                                    left: Box::new(Expression::Assign {
                                        name: temporary_name.clone(),
                                        value: Box::new(value),
                                    }),
                                    right: Box::new(Expression::Undefined),
                                }),
                                then_expression: Box::new(Expression::Identifier(temporary_name)),
                                else_expression: Box::new(default_value),
                            },
                        });
                        return Ok(());
                    }

                    statements.push(Statement::Let {
                        name: temporary_name.clone(),
                        mutable: true,
                        value,
                    });
                    statements.push(match binding_kind {
                        ForOfPatternBindingKind::Var => Statement::Var {
                            name: name.clone(),
                            value: Expression::Undefined,
                        },
                        ForOfPatternBindingKind::Assignment => Statement::Assign {
                            name: name.clone(),
                            value: Expression::Undefined,
                        },
                        ForOfPatternBindingKind::Lexical { mutable } => Statement::Let {
                            name: name.clone(),
                            mutable,
                            value: Expression::Undefined,
                        },
                    });
                    statements.push(Statement::If {
                        condition: Expression::Binary {
                            op: BinaryOp::NotEqual,
                            left: Box::new(Expression::Identifier(temporary_name.clone())),
                            right: Box::new(Expression::Undefined),
                        },
                        then_branch: vec![Statement::Assign {
                            name: name.clone(),
                            value: Expression::Identifier(temporary_name),
                        }],
                        else_branch: vec![Statement::Assign {
                            name,
                            value: default_value,
                        }],
                    });
                    return Ok(());
                }

                if matches!(binding_kind, ForOfPatternBindingKind::Assignment)
                    && let Expression::Member { object, property } = value.clone()
                    && let Pat::Expr(expression) = assign.left.as_ref()
                    && Self::is_cached_member_assignment_target(expression)
                {
                    let property_name = self.fresh_temporary_name("source_property");
                    statements.push(Statement::Let {
                        name: property_name.clone(),
                        mutable: true,
                        value: Self::property_key_value_expression(*property),
                    });
                    if let Some(target) =
                        self.lower_cached_member_assignment_target(expression, statements)?
                    {
                        statements.push(Statement::Let {
                            name: temporary_name.clone(),
                            mutable: true,
                            value: Expression::Undefined,
                        });
                        let source_value = Expression::Member {
                            object,
                            property: Box::new(Expression::Identifier(property_name)),
                        };
                        statements.push(target.into_statement(Expression::Conditional {
                            condition: Box::new(Expression::Binary {
                                op: BinaryOp::NotEqual,
                                left: Box::new(Expression::Assign {
                                    name: temporary_name.clone(),
                                    value: Box::new(source_value),
                                }),
                                right: Box::new(Expression::Undefined),
                            }),
                            then_expression: Box::new(Expression::Identifier(temporary_name)),
                            else_expression: Box::new(default_value),
                        }));
                        return Ok(());
                    }
                }

                statements.push(Statement::Let {
                    name: temporary_name.clone(),
                    mutable: true,
                    value,
                });
                let mut then_branch = Vec::new();
                self.lower_for_of_pattern_binding_with_generator_defaults(
                    &assign.left,
                    Expression::Identifier(temporary_name.clone()),
                    binding_kind,
                    &mut then_branch,
                    generator_body,
                )?;
                let mut else_branch = Vec::new();
                if let Some((mut generator_prefix, _)) = generator_default {
                    else_branch.append(&mut generator_prefix);
                }
                self.lower_for_of_pattern_binding_with_generator_defaults(
                    &assign.left,
                    default_value,
                    binding_kind,
                    &mut else_branch,
                    generator_body,
                )?;
                statements.push(Statement::If {
                    condition: Expression::Binary {
                        op: BinaryOp::NotEqual,
                        left: Box::new(Expression::Identifier(temporary_name)),
                        right: Box::new(Expression::Undefined),
                    },
                    then_branch,
                    else_branch,
                });
            }
            Pat::Array(array) => {
                let has_rest = array
                    .elems
                    .iter()
                    .flatten()
                    .any(|element| matches!(element, Pat::Rest(_)));
                if !has_rest {
                    let pure_elision_count = self.pure_array_pattern_elision_count(array);
                    let iterator_value_name = self.fresh_temporary_name("array_iter_value");
                    let iterator_name = self.fresh_temporary_name("array_iter");
                    let iterator_done_name = self.fresh_temporary_name("array_iter_done");
                    statements.push(Statement::Let {
                        name: iterator_value_name.clone(),
                        mutable: true,
                        value: value.clone(),
                    });
                    self.emit_require_object_coercible_check(
                        &Expression::Identifier(iterator_value_name.clone()),
                        statements,
                    );
                    statements.push(Statement::Let {
                        name: iterator_name.clone(),
                        mutable: true,
                        value: Expression::GetIterator(Box::new(Expression::Identifier(
                            iterator_value_name,
                        ))),
                    });
                    statements.push(Statement::Let {
                        name: iterator_done_name.clone(),
                        mutable: true,
                        value: Expression::Bool(false),
                    });

                    if array.elems.is_empty() && pure_elision_count > 0 {
                        for _ in 0..pure_elision_count {
                            let step_name = self.fresh_temporary_name("array_step");
                            statements.push(Statement::Let {
                                name: step_name.clone(),
                                mutable: true,
                                value: Expression::Call {
                                    callee: Box::new(Expression::Member {
                                        object: Box::new(Expression::Identifier(
                                            iterator_name.clone(),
                                        )),
                                        property: Box::new(Expression::String("next".to_string())),
                                    }),
                                    arguments: Vec::new(),
                                },
                            });
                            statements.push(Statement::Assign {
                                name: iterator_done_name.clone(),
                                value: Expression::Member {
                                    object: Box::new(Expression::Identifier(step_name)),
                                    property: Box::new(Expression::String("done".to_string())),
                                },
                            });
                        }
                        statements.push(Statement::If {
                            condition: Expression::Binary {
                                op: BinaryOp::Equal,
                                left: Box::new(Expression::Identifier(iterator_done_name)),
                                right: Box::new(Expression::Bool(false)),
                            },
                            then_branch: vec![Statement::Expression(Expression::IteratorClose(
                                Box::new(Expression::Identifier(iterator_name)),
                            ))],
                            else_branch: Vec::new(),
                        });
                        return Ok(());
                    }

                    let trailing_comma_index = self
                        .array_pattern_has_non_elision_trailing_comma(array)
                        .then(|| array.elems.len().saturating_sub(1));

                    for (element_index, element) in array.elems.iter().enumerate() {
                        if trailing_comma_index == Some(element_index) && element.is_none() {
                            continue;
                        }
                        let precomputed_target = if let Some(Pat::Expr(expression)) = element {
                            if generator_body {
                                Some(self.lower_for_of_expression_target_with_generator_defaults(
                                    expression, true, statements,
                                )?)
                            } else if matches!(binding_kind, ForOfPatternBindingKind::Assignment) {
                                Some(self.lower_for_of_expression_target_with_iterator_close(
                                    expression,
                                    &iterator_name,
                                    statements,
                                )?)
                            } else {
                                None
                            }
                        } else {
                            None
                        };
                        let step_name = self.fresh_temporary_name("array_step");
                        statements.push(Statement::Let {
                            name: step_name.clone(),
                            mutable: true,
                            value: Expression::Call {
                                callee: Box::new(Expression::Member {
                                    object: Box::new(Expression::Identifier(iterator_name.clone())),
                                    property: Box::new(Expression::String("next".to_string())),
                                }),
                                arguments: Vec::new(),
                            },
                        });
                        let step_done = Expression::Member {
                            object: Box::new(Expression::Identifier(step_name.clone())),
                            property: Box::new(Expression::String("done".to_string())),
                        };
                        statements.push(Statement::Assign {
                            name: iterator_done_name.clone(),
                            value: step_done.clone(),
                        });
                        let step_value = Expression::Conditional {
                            condition: Box::new(Expression::Binary {
                                op: BinaryOp::Equal,
                                left: Box::new(step_done),
                                right: Box::new(Expression::Bool(false)),
                            }),
                            then_expression: Box::new(Expression::Member {
                                object: Box::new(Expression::Identifier(step_name)),
                                property: Box::new(Expression::String("value".to_string())),
                            }),
                            else_expression: Box::new(Expression::Undefined),
                        };

                        if let Some(target) = precomputed_target {
                            statements.push(target.into_statement(step_value));
                        } else if let Some(element) = element {
                            self.lower_for_of_pattern_binding_with_generator_defaults(
                                element,
                                step_value,
                                binding_kind,
                                statements,
                                generator_body,
                            )?;
                        }
                    }
                    for _ in 0..self.array_pattern_trailing_elision_count(array) {
                        let step_name = self.fresh_temporary_name("array_step");
                        statements.push(Statement::Let {
                            name: step_name.clone(),
                            mutable: true,
                            value: Expression::Call {
                                callee: Box::new(Expression::Member {
                                    object: Box::new(Expression::Identifier(iterator_name.clone())),
                                    property: Box::new(Expression::String("next".to_string())),
                                }),
                                arguments: Vec::new(),
                            },
                        });
                        statements.push(Statement::Assign {
                            name: iterator_done_name.clone(),
                            value: Expression::Member {
                                object: Box::new(Expression::Identifier(step_name)),
                                property: Box::new(Expression::String("done".to_string())),
                            },
                        });
                    }

                    statements.push(Statement::If {
                        condition: Expression::Binary {
                            op: BinaryOp::Equal,
                            left: Box::new(Expression::Identifier(iterator_done_name)),
                            right: Box::new(Expression::Bool(false)),
                        },
                        then_branch: vec![Statement::Expression(Expression::IteratorClose(
                            Box::new(Expression::Identifier(iterator_name)),
                        ))],
                        else_branch: Vec::new(),
                    });
                    return Ok(());
                }

                let iterator_value_name = self.fresh_temporary_name("array_iter_value");
                let iterator_name = self.fresh_temporary_name("array_iter");
                let iterator_done_name = self.fresh_temporary_name("array_iter_done");
                statements.push(Statement::Let {
                    name: iterator_value_name.clone(),
                    mutable: true,
                    value: value.clone(),
                });
                self.emit_require_object_coercible_check(
                    &Expression::Identifier(iterator_value_name.clone()),
                    statements,
                );
                statements.push(Statement::Let {
                    name: iterator_name.clone(),
                    mutable: true,
                    value: Expression::GetIterator(Box::new(Expression::Identifier(
                        iterator_value_name,
                    ))),
                });
                statements.push(Statement::Let {
                    name: iterator_done_name.clone(),
                    mutable: true,
                    value: Expression::Bool(false),
                });

                let trailing_comma_index = self
                    .array_pattern_has_non_elision_trailing_comma(array)
                    .then(|| array.elems.len().saturating_sub(1));

                let mut rest_seen = false;
                for (element_index, element) in array.elems.iter().enumerate() {
                    if trailing_comma_index == Some(element_index) && element.is_none() {
                        continue;
                    }
                    let precomputed_target = if let Some(Pat::Expr(expression)) = element {
                        if generator_body {
                            Some(self.lower_for_of_expression_target_with_generator_defaults(
                                expression, true, statements,
                            )?)
                        } else if matches!(binding_kind, ForOfPatternBindingKind::Assignment) {
                            Some(self.lower_for_of_expression_target_with_iterator_close(
                                expression,
                                &iterator_name,
                                statements,
                            )?)
                        } else {
                            None
                        }
                    } else {
                        None
                    };
                    if let Some(Pat::Rest(rest)) = element {
                        rest_seen = true;
                        let precomputed_rest_target = if let Pat::Expr(expression) =
                            rest.arg.as_ref()
                        {
                            if generator_body {
                                Some(self.lower_for_of_expression_target_with_generator_defaults(
                                    expression, true, statements,
                                )?)
                            } else if matches!(binding_kind, ForOfPatternBindingKind::Assignment) {
                                Some(self.lower_for_of_expression_target_with_iterator_close(
                                    expression,
                                    &iterator_name,
                                    statements,
                                )?)
                            } else {
                                None
                            }
                        } else {
                            None
                        };
                        let rest_array_name = self.fresh_temporary_name("array_rest");
                        let rest_step_name = self.fresh_temporary_name("array_step");
                        statements.push(Statement::Let {
                            name: rest_array_name.clone(),
                            mutable: true,
                            value: Expression::Array(Vec::new()),
                        });
                        statements.push(Statement::While {
                            labels: Vec::new(),
                            condition: Expression::Binary {
                                op: BinaryOp::Equal,
                                left: Box::new(Expression::Identifier(iterator_done_name.clone())),
                                right: Box::new(Expression::Bool(false)),
                            },
                            break_hook: None,
                            body: vec![
                                Statement::Let {
                                    name: rest_step_name.clone(),
                                    mutable: true,
                                    value: Expression::Call {
                                        callee: Box::new(Expression::Member {
                                            object: Box::new(Expression::Identifier(
                                                iterator_name.clone(),
                                            )),
                                            property: Box::new(Expression::String(
                                                "next".to_string(),
                                            )),
                                        }),
                                        arguments: Vec::new(),
                                    },
                                },
                                Statement::Assign {
                                    name: iterator_done_name.clone(),
                                    value: Expression::Member {
                                        object: Box::new(Expression::Identifier(
                                            rest_step_name.clone(),
                                        )),
                                        property: Box::new(Expression::String("done".to_string())),
                                    },
                                },
                                Statement::If {
                                    condition: Expression::Binary {
                                        op: BinaryOp::Equal,
                                        left: Box::new(Expression::Identifier(
                                            iterator_done_name.clone(),
                                        )),
                                        right: Box::new(Expression::Bool(false)),
                                    },
                                    then_branch: vec![Statement::Expression(Expression::Call {
                                        callee: Box::new(Expression::Member {
                                            object: Box::new(Expression::Identifier(
                                                rest_array_name.clone(),
                                            )),
                                            property: Box::new(Expression::String(
                                                "push".to_string(),
                                            )),
                                        }),
                                        arguments: vec![CallArgument::Expression(
                                            Expression::Member {
                                                object: Box::new(Expression::Identifier(
                                                    rest_step_name.clone(),
                                                )),
                                                property: Box::new(Expression::String(
                                                    "value".to_string(),
                                                )),
                                            },
                                        )],
                                    })],
                                    else_branch: Vec::new(),
                                },
                            ],
                        });
                        if let Some(target) = precomputed_rest_target {
                            statements.push(
                                target.into_statement(Expression::Identifier(rest_array_name)),
                            );
                        } else {
                            self.lower_for_of_pattern_binding_with_generator_defaults(
                                &rest.arg,
                                Expression::Identifier(rest_array_name),
                                binding_kind,
                                statements,
                                generator_body,
                            )?;
                        }
                        break;
                    }

                    let step_name = self.fresh_temporary_name("array_step");
                    statements.push(Statement::Let {
                        name: step_name.clone(),
                        mutable: true,
                        value: Expression::Call {
                            callee: Box::new(Expression::Member {
                                object: Box::new(Expression::Identifier(iterator_name.clone())),
                                property: Box::new(Expression::String("next".to_string())),
                            }),
                            arguments: Vec::new(),
                        },
                    });
                    let step_done = Expression::Member {
                        object: Box::new(Expression::Identifier(step_name.clone())),
                        property: Box::new(Expression::String("done".to_string())),
                    };
                    statements.push(Statement::Assign {
                        name: iterator_done_name.clone(),
                        value: step_done.clone(),
                    });
                    let step_value = Expression::Conditional {
                        condition: Box::new(Expression::Binary {
                            op: BinaryOp::Equal,
                            left: Box::new(step_done),
                            right: Box::new(Expression::Bool(false)),
                        }),
                        then_expression: Box::new(Expression::Member {
                            object: Box::new(Expression::Identifier(step_name)),
                            property: Box::new(Expression::String("value".to_string())),
                        }),
                        else_expression: Box::new(Expression::Undefined),
                    };

                    if let Some(target) = precomputed_target {
                        statements.push(target.into_statement(step_value));
                    } else if let Some(element) = element {
                        self.lower_for_of_pattern_binding_with_generator_defaults(
                            element,
                            step_value,
                            binding_kind,
                            statements,
                            generator_body,
                        )?;
                    }
                }
                if !rest_seen {
                    for _ in 0..self.array_pattern_trailing_elision_count(array) {
                        let step_name = self.fresh_temporary_name("array_step");
                        statements.push(Statement::Let {
                            name: step_name.clone(),
                            mutable: true,
                            value: Expression::Call {
                                callee: Box::new(Expression::Member {
                                    object: Box::new(Expression::Identifier(iterator_name.clone())),
                                    property: Box::new(Expression::String("next".to_string())),
                                }),
                                arguments: Vec::new(),
                            },
                        });
                        statements.push(Statement::Assign {
                            name: iterator_done_name.clone(),
                            value: Expression::Member {
                                object: Box::new(Expression::Identifier(step_name)),
                                property: Box::new(Expression::String("done".to_string())),
                            },
                        });
                    }
                }

                statements.push(Statement::If {
                    condition: Expression::Binary {
                        op: BinaryOp::Equal,
                        left: Box::new(Expression::Identifier(iterator_done_name)),
                        right: Box::new(Expression::Bool(false)),
                    },
                    then_branch: vec![Statement::Expression(Expression::IteratorClose(Box::new(
                        Expression::Identifier(iterator_name),
                    )))],
                    else_branch: Vec::new(),
                });
            }
            Pat::Object(object) => {
                self.emit_require_object_coercible_check(&value, statements);
                let mut rest_pattern = None;
                let mut excluded_properties = Vec::new();
                for property in &object.props {
                    match property {
                        ObjectPatProp::KeyValue(property) => {
                            let property_key = self.lower_prop_name(&property.key)?;
                            let property_value = Expression::Member {
                                object: Box::new(value.clone()),
                                property: Box::new(property_key.clone()),
                            };
                            excluded_properties.push(property_key);
                            self.lower_for_of_pattern_binding_with_generator_defaults(
                                &property.value,
                                property_value,
                                binding_kind,
                                statements,
                                generator_body,
                            )?;
                        }
                        ObjectPatProp::Assign(property) => {
                            let binding_name_hint = property.key.id.sym.to_string();
                            let binding_name =
                                self.resolve_binding_name(property.key.id.sym.as_ref());
                            let property_value = Expression::Member {
                                object: Box::new(value.clone()),
                                property: Box::new(Expression::String(
                                    property.key.id.sym.to_string(),
                                )),
                            };
                            excluded_properties
                                .push(Expression::String(property.key.id.sym.to_string()));
                            let property_value = if let Some(default) = &property.value {
                                let generator_default = if generator_body {
                                    self.lower_generator_assignment_value_with_name_hint(
                                        default,
                                        Some(binding_name_hint.as_str()),
                                    )?
                                } else {
                                    None
                                };
                                if let Some((mut generator_prefix, default_value)) =
                                    generator_default
                                {
                                    let then_branch = vec![match binding_kind {
                                        ForOfPatternBindingKind::Var => Statement::Var {
                                            name: binding_name.clone(),
                                            value: property_value.clone(),
                                        },
                                        ForOfPatternBindingKind::Assignment => Statement::Assign {
                                            name: binding_name.clone(),
                                            value: property_value.clone(),
                                        },
                                        ForOfPatternBindingKind::Lexical { mutable } => {
                                            Statement::Let {
                                                name: binding_name.clone(),
                                                mutable,
                                                value: property_value.clone(),
                                            }
                                        }
                                    }];
                                    let mut else_branch = Vec::new();
                                    else_branch.append(&mut generator_prefix);
                                    else_branch.push(match binding_kind {
                                        ForOfPatternBindingKind::Var => Statement::Var {
                                            name: binding_name,
                                            value: default_value,
                                        },
                                        ForOfPatternBindingKind::Assignment => Statement::Assign {
                                            name: binding_name,
                                            value: default_value,
                                        },
                                        ForOfPatternBindingKind::Lexical { mutable } => {
                                            Statement::Let {
                                                name: binding_name,
                                                mutable,
                                                value: default_value,
                                            }
                                        }
                                    });
                                    statements.push(Statement::If {
                                        condition: Expression::Binary {
                                            op: BinaryOp::NotEqual,
                                            left: Box::new(property_value),
                                            right: Box::new(Expression::Undefined),
                                        },
                                        then_branch,
                                        else_branch,
                                    });
                                    continue;
                                }
                                let default_value = self.lower_expression_with_name_hint(
                                    default,
                                    Some(binding_name_hint.as_str()),
                                )?;
                                Expression::Conditional {
                                    condition: Box::new(Expression::Binary {
                                        op: BinaryOp::NotEqual,
                                        left: Box::new(property_value.clone()),
                                        right: Box::new(Expression::Undefined),
                                    }),
                                    then_expression: Box::new(property_value),
                                    else_expression: Box::new(default_value),
                                }
                            } else {
                                property_value
                            };
                            statements.push(match binding_kind {
                                ForOfPatternBindingKind::Var => Statement::Var {
                                    name: binding_name,
                                    value: property_value,
                                },
                                ForOfPatternBindingKind::Assignment => Statement::Assign {
                                    name: binding_name,
                                    value: property_value,
                                },
                                ForOfPatternBindingKind::Lexical { mutable } => Statement::Let {
                                    name: binding_name,
                                    mutable,
                                    value: property_value,
                                },
                            });
                        }
                        ObjectPatProp::Rest(rest) => {
                            rest_pattern = Some(rest);
                        }
                    }
                }

                if let Some(rest) = rest_pattern {
                    let rest_name = self.fresh_temporary_name("object_rest");
                    statements.push(Statement::Let {
                        name: rest_name.clone(),
                        mutable: true,
                        value: Expression::Object(vec![ObjectEntry::Spread(value)]),
                    });
                    for property in excluded_properties {
                        statements.push(Statement::Expression(Expression::Unary {
                            op: UnaryOp::Delete,
                            expression: Box::new(Expression::Member {
                                object: Box::new(Expression::Identifier(rest_name.clone())),
                                property: Box::new(property),
                            }),
                        }));
                    }
                    self.lower_for_of_pattern_binding_with_generator_defaults(
                        &rest.arg,
                        Expression::Identifier(rest_name),
                        binding_kind,
                        statements,
                        generator_body,
                    )?;
                }
            }
            _ => bail!("unsupported for-of binding pattern"),
        }

        Ok(())
    }

    pub(crate) fn emit_require_object_coercible_check(
        &mut self,
        value: &Expression,
        statements: &mut Vec<Statement>,
    ) {
        let is_nullish = Expression::Binary {
            op: BinaryOp::LogicalOr,
            left: Box::new(Expression::Binary {
                op: BinaryOp::Equal,
                left: Box::new(value.clone()),
                right: Box::new(Expression::Null),
            }),
            right: Box::new(Expression::Binary {
                op: BinaryOp::Equal,
                left: Box::new(value.clone()),
                right: Box::new(Expression::Undefined),
            }),
        };

        statements.push(Statement::If {
            condition: is_nullish,
            then_branch: vec![Statement::Throw(Expression::New {
                callee: Box::new(Expression::Identifier("TypeError".to_string())),
                arguments: Vec::new(),
            })],
            else_branch: Vec::new(),
        });
    }

    fn lower_for_of_expression_target_with_generator_defaults(
        &mut self,
        expression: &Expr,
        generator_body: bool,
        statements: &mut Vec<Statement>,
    ) -> Result<AssignmentTarget> {
        if !generator_body {
            return self.lower_for_of_expression_target_without_generator_defaults(expression);
        }

        match expression {
            Expr::Ident(identifier) => Ok(AssignmentTarget::Identifier(
                self.resolve_binding_name(identifier.sym.as_ref()),
            )),
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
                if handled {
                    statements.append(&mut lowered);
                }
                Ok(AssignmentTarget::Member { object, property })
            }
            Expr::Paren(parenthesized) => self
                .lower_for_of_expression_target_with_generator_defaults(
                    &parenthesized.expr,
                    generator_body,
                    statements,
                ),
            _ => bail!("unsupported for-of assignment target"),
        }
    }

    fn lower_for_of_expression_target_without_generator_defaults(
        &mut self,
        expression: &Expr,
    ) -> Result<AssignmentTarget> {
        match expression {
            Expr::Ident(identifier) => Ok(AssignmentTarget::Identifier(
                self.resolve_binding_name(identifier.sym.as_ref()),
            )),
            Expr::Member(member) => Ok(AssignmentTarget::Member {
                object: self.lower_expression(&member.obj)?,
                property: self.lower_member_property(&member.prop)?,
            }),
            Expr::Paren(parenthesized) => {
                self.lower_for_of_expression_target_without_generator_defaults(&parenthesized.expr)
            }
            _ => bail!("unsupported for-of assignment target"),
        }
    }

    fn lower_cached_member_assignment_target(
        &mut self,
        expression: &Expr,
        statements: &mut Vec<Statement>,
    ) -> Result<Option<AssignmentTarget>> {
        match expression {
            Expr::Member(member) => {
                let object = self.lower_expression(&member.obj)?;
                let property = self.lower_member_property(&member.prop)?;
                let object_name = self.fresh_temporary_name("target_object");
                let property_name = self.fresh_temporary_name("target_property");
                statements.push(Statement::Let {
                    name: object_name.clone(),
                    mutable: true,
                    value: object,
                });
                statements.push(Statement::Let {
                    name: property_name.clone(),
                    mutable: true,
                    value: property,
                });
                Ok(Some(AssignmentTarget::Member {
                    object: Expression::Identifier(object_name),
                    property: Expression::Identifier(property_name),
                }))
            }
            Expr::Paren(parenthesized) => {
                self.lower_cached_member_assignment_target(&parenthesized.expr, statements)
            }
            _ => Ok(None),
        }
    }

    fn lower_cached_member_assignment_target_to_expressions(
        &mut self,
        expression: &Expr,
        expressions: &mut Vec<Expression>,
    ) -> Result<Option<AssignmentTarget>> {
        match expression {
            Expr::Member(member) => {
                let object = self.lower_expression(&member.obj)?;
                let property = self.lower_member_property(&member.prop)?;
                let object_name = self.fresh_temporary_name("target_object");
                let property_name = self.fresh_temporary_name("target_property");
                expressions.push(Expression::Assign {
                    name: object_name.clone(),
                    value: Box::new(object),
                });
                expressions.push(Expression::Assign {
                    name: property_name.clone(),
                    value: Box::new(property),
                });
                Ok(Some(AssignmentTarget::Member {
                    object: Expression::Identifier(object_name),
                    property: Expression::Identifier(property_name),
                }))
            }
            Expr::Paren(parenthesized) => self
                .lower_cached_member_assignment_target_to_expressions(
                    &parenthesized.expr,
                    expressions,
                ),
            _ => Ok(None),
        }
    }

    fn is_cached_member_assignment_target(expression: &Expr) -> bool {
        match expression {
            Expr::Member(_) => true,
            Expr::Paren(parenthesized) => {
                Self::is_cached_member_assignment_target(&parenthesized.expr)
            }
            _ => false,
        }
    }

    fn property_key_value_expression(expression: Expression) -> Expression {
        expression
    }

    fn lower_for_of_expression_target_with_iterator_close(
        &mut self,
        expression: &Expr,
        iterator_name: &str,
        statements: &mut Vec<Statement>,
    ) -> Result<AssignmentTarget> {
        match expression {
            Expr::Ident(identifier) => Ok(AssignmentTarget::Identifier(
                self.resolve_binding_name(identifier.sym.as_ref()),
            )),
            Expr::Member(member) => {
                let object = self.lower_expression(&member.obj)?;
                let property = self.lower_member_property(&member.prop)?;
                let object_name = self.fresh_temporary_name("target_object");
                let property_name = self.fresh_temporary_name("target_property");
                statements.push(Statement::Let {
                    name: object_name.clone(),
                    mutable: true,
                    value: Expression::Undefined,
                });
                statements.push(Statement::Let {
                    name: property_name.clone(),
                    mutable: true,
                    value: Expression::Undefined,
                });
                let catch_name = self.fresh_temporary_name("target_catch");
                statements.push(Statement::Try {
                    body: vec![
                        Statement::Assign {
                            name: object_name.clone(),
                            value: object,
                        },
                        Statement::Assign {
                            name: property_name.clone(),
                            value: property,
                        },
                    ],
                    catch_binding: Some(catch_name.clone()),
                    catch_setup: Vec::new(),
                    catch_body: vec![
                        Statement::Try {
                            body: vec![Statement::Expression(Expression::IteratorClose(Box::new(
                                Expression::Identifier(iterator_name.to_string()),
                            )))],
                            catch_binding: None,
                            catch_setup: Vec::new(),
                            catch_body: Vec::new(),
                        },
                        Statement::Throw(Expression::Identifier(catch_name)),
                    ],
                });
                Ok(AssignmentTarget::Member {
                    object: Expression::Identifier(object_name),
                    property: Expression::Identifier(property_name),
                })
            }
            Expr::Paren(parenthesized) => self.lower_for_of_expression_target_with_iterator_close(
                &parenthesized.expr,
                iterator_name,
                statements,
            ),
            _ => self.lower_for_of_expression_target_without_generator_defaults(expression),
        }
    }

    pub(super) fn lower_assignment_target(
        &mut self,
        target: &AssignTarget,
    ) -> Result<AssignmentTarget> {
        match target {
            AssignTarget::Simple(SimpleAssignTarget::Ident(identifier)) => Ok(
                AssignmentTarget::Identifier(self.resolve_binding_name(identifier.id.sym.as_ref())),
            ),
            AssignTarget::Simple(SimpleAssignTarget::Member(member)) => {
                Ok(AssignmentTarget::Member {
                    object: self.lower_expression(&member.obj)?,
                    property: self.lower_member_property(&member.prop)?,
                })
            }
            AssignTarget::Simple(SimpleAssignTarget::Paren(parenthesized)) => {
                let Ok(target) = AssignTarget::try_from(parenthesized.expr.clone()) else {
                    bail!("unsupported assignment target")
                };
                self.lower_assignment_target(&target)
            }
            AssignTarget::Simple(SimpleAssignTarget::SuperProp(super_property)) => {
                Ok(AssignmentTarget::SuperMember {
                    property: self.lower_super_property(super_property)?,
                })
            }
            _ => bail!("unsupported assignment target"),
        }
    }

    pub(super) fn assignment_target_name_hint(&self, target: &AssignTarget) -> Option<String> {
        match target {
            AssignTarget::Simple(SimpleAssignTarget::Ident(identifier)) => {
                Some(self.resolve_binding_name(identifier.id.sym.as_ref()))
            }
            AssignTarget::Simple(
                SimpleAssignTarget::Member(_)
                | SimpleAssignTarget::Paren(_)
                | SimpleAssignTarget::SuperProp(_),
            ) => Some(String::new()),
            _ => None,
        }
    }

    pub(super) fn lower_assignment_pattern_expression(
        &mut self,
        pattern: &Pat,
        value: Expression,
    ) -> Result<Expression> {
        if Self::assignment_pattern_expression_can_reuse_value(&value) {
            let mut expressions = Vec::new();
            self.lower_assignment_pattern_to_expressions(pattern, value.clone(), &mut expressions)?;
            expressions.push(value);
            return Ok(Expression::Sequence(expressions));
        }

        let value_name = self.fresh_temporary_name("destructure_value");
        let mut expressions = vec![Expression::Assign {
            name: value_name.clone(),
            value: Box::new(value),
        }];
        self.lower_assignment_pattern_to_expressions(
            pattern,
            Expression::Identifier(value_name.clone()),
            &mut expressions,
        )?;
        expressions.push(Expression::Identifier(value_name));
        Ok(Expression::Sequence(expressions))
    }

    fn assignment_pattern_expression_can_reuse_value(value: &Expression) -> bool {
        matches!(value, Expression::Identifier(_) | Expression::This)
    }

    fn lower_assignment_pattern_to_expressions(
        &mut self,
        pattern: &Pat,
        value: Expression,
        expressions: &mut Vec<Expression>,
    ) -> Result<()> {
        match pattern {
            Pat::Ident(identifier) => {
                expressions.push(Expression::Assign {
                    name: self.resolve_binding_name(identifier.id.sym.as_ref()),
                    value: Box::new(value),
                });
            }
            Pat::Expr(expression) => {
                let target =
                    self.lower_for_of_expression_target_without_generator_defaults(expression)?;
                expressions.push(target.into_expression(value));
            }
            Pat::Assign(assign) => {
                let default_value = self.lower_expression_with_name_hint(
                    &assign.right,
                    pattern_name_hint(&assign.left),
                )?;
                if let Expression::Member { object, property } = value.clone()
                    && let Pat::Expr(expression) = assign.left.as_ref()
                    && Self::is_cached_member_assignment_target(expression)
                {
                    let property_name = self.fresh_temporary_name("source_property");
                    expressions.push(Expression::Assign {
                        name: property_name.clone(),
                        value: Box::new(Self::property_key_value_expression(*property)),
                    });
                    if let Some(target) = self
                        .lower_cached_member_assignment_target_to_expressions(
                            expression,
                            expressions,
                        )?
                    {
                        let binding_value_name = self.fresh_temporary_name("binding_value");
                        expressions.push(Expression::Assign {
                            name: binding_value_name.clone(),
                            value: Box::new(Expression::Undefined),
                        });
                        let source_value = Expression::Member {
                            object,
                            property: Box::new(Expression::Identifier(property_name)),
                        };
                        expressions.push(target.into_expression(Expression::Conditional {
                            condition: Box::new(Expression::Binary {
                                op: BinaryOp::NotEqual,
                                left: Box::new(Expression::Assign {
                                    name: binding_value_name.clone(),
                                    value: Box::new(source_value),
                                }),
                                right: Box::new(Expression::Undefined),
                            }),
                            then_expression: Box::new(Expression::Identifier(binding_value_name)),
                            else_expression: Box::new(default_value),
                        }));
                        return Ok(());
                    }
                }
                let assigned_value = Expression::Conditional {
                    condition: Box::new(Expression::Binary {
                        op: BinaryOp::NotEqual,
                        left: Box::new(value.clone()),
                        right: Box::new(Expression::Undefined),
                    }),
                    then_expression: Box::new(value),
                    else_expression: Box::new(default_value),
                };
                self.lower_assignment_pattern_to_expressions(
                    &assign.left,
                    assigned_value,
                    expressions,
                )?;
            }
            Pat::Object(object) => {
                for property in &object.props {
                    match property {
                        ObjectPatProp::KeyValue(property) => {
                            let property_key = self.lower_prop_name(&property.key)?;
                            let property_value = Expression::Member {
                                object: Box::new(value.clone()),
                                property: Box::new(property_key),
                            };
                            self.lower_assignment_pattern_to_expressions(
                                &property.value,
                                property_value,
                                expressions,
                            )?;
                        }
                        ObjectPatProp::Assign(property) => {
                            let binding_name_hint = property.key.id.sym.to_string();
                            let binding_name =
                                self.resolve_binding_name(property.key.id.sym.as_ref());
                            let property_value = Expression::Member {
                                object: Box::new(value.clone()),
                                property: Box::new(Expression::String(
                                    property.key.id.sym.to_string(),
                                )),
                            };
                            let assigned_value = if let Some(default) = &property.value {
                                let default_value = self.lower_expression_with_name_hint(
                                    default,
                                    Some(binding_name_hint.as_str()),
                                )?;
                                Expression::Conditional {
                                    condition: Box::new(Expression::Binary {
                                        op: BinaryOp::NotEqual,
                                        left: Box::new(property_value.clone()),
                                        right: Box::new(Expression::Undefined),
                                    }),
                                    then_expression: Box::new(property_value),
                                    else_expression: Box::new(default_value),
                                }
                            } else {
                                property_value
                            };
                            expressions.push(Expression::Assign {
                                name: binding_name,
                                value: Box::new(assigned_value),
                            });
                        }
                        ObjectPatProp::Rest(_) => {
                            bail!("unsupported object rest in assignment expression")
                        }
                    }
                }
            }
            _ => bail!("unsupported assignment expression pattern"),
        }
        Ok(())
    }
}
