use super::*;

impl<'a> FunctionCompiler<'a> {
    fn collect_effectful_iterator_assigned_binding_names_from_statement(
        &self,
        statement: &Statement,
        names: &mut HashSet<String>,
    ) {
        match statement {
            Statement::Declaration { body }
            | Statement::Block { body }
            | Statement::Labeled { body, .. } => {
                for statement in body {
                    self.collect_effectful_iterator_assigned_binding_names_from_statement(
                        statement, names,
                    );
                }
            }
            Statement::Var { value, .. }
            | Statement::Let { value, .. }
            | Statement::Assign { value, .. }
            | Statement::Expression(value)
            | Statement::Throw(value)
            | Statement::Return(value)
            | Statement::Yield { value }
            | Statement::YieldDelegate { value } => {
                self.collect_effectful_iterator_assigned_binding_names_from_expression(
                    value, names,
                );
            }
            Statement::AssignMember {
                object,
                property,
                value,
            } => {
                self.collect_effectful_iterator_assigned_binding_names_from_expression(
                    object, names,
                );
                self.collect_effectful_iterator_assigned_binding_names_from_expression(
                    property, names,
                );
                self.collect_effectful_iterator_assigned_binding_names_from_expression(
                    value, names,
                );
            }
            Statement::Print { values } => {
                for value in values {
                    self.collect_effectful_iterator_assigned_binding_names_from_expression(
                        value, names,
                    );
                }
            }
            Statement::With { object, body } => {
                self.collect_effectful_iterator_assigned_binding_names_from_expression(
                    object, names,
                );
                for statement in body {
                    self.collect_effectful_iterator_assigned_binding_names_from_statement(
                        statement, names,
                    );
                }
            }
            Statement::If {
                condition,
                then_branch,
                else_branch,
            } => {
                self.collect_effectful_iterator_assigned_binding_names_from_expression(
                    condition, names,
                );
                for statement in then_branch.iter().chain(else_branch) {
                    self.collect_effectful_iterator_assigned_binding_names_from_statement(
                        statement, names,
                    );
                }
            }
            Statement::Try {
                body,
                catch_setup,
                catch_body,
                ..
            } => {
                for statement in body.iter().chain(catch_setup).chain(catch_body) {
                    self.collect_effectful_iterator_assigned_binding_names_from_statement(
                        statement, names,
                    );
                }
            }
            Statement::Switch {
                discriminant,
                cases,
                ..
            } => {
                self.collect_effectful_iterator_assigned_binding_names_from_expression(
                    discriminant,
                    names,
                );
                for case in cases {
                    if let Some(test) = &case.test {
                        self.collect_effectful_iterator_assigned_binding_names_from_expression(
                            test, names,
                        );
                    }
                    for statement in &case.body {
                        self.collect_effectful_iterator_assigned_binding_names_from_statement(
                            statement, names,
                        );
                    }
                }
            }
            Statement::For {
                init,
                condition,
                update,
                break_hook,
                body,
                ..
            } => {
                for statement in init {
                    self.collect_effectful_iterator_assigned_binding_names_from_statement(
                        statement, names,
                    );
                }
                if let Some(condition) = condition {
                    self.collect_effectful_iterator_assigned_binding_names_from_expression(
                        condition, names,
                    );
                }
                if let Some(update) = update {
                    self.collect_effectful_iterator_assigned_binding_names_from_expression(
                        update, names,
                    );
                }
                if let Some(break_hook) = break_hook {
                    self.collect_effectful_iterator_assigned_binding_names_from_expression(
                        break_hook, names,
                    );
                }
                for statement in body {
                    self.collect_effectful_iterator_assigned_binding_names_from_statement(
                        statement, names,
                    );
                }
            }
            Statement::While {
                condition,
                break_hook,
                body,
                ..
            }
            | Statement::DoWhile {
                condition,
                break_hook,
                body,
                ..
            } => {
                self.collect_effectful_iterator_assigned_binding_names_from_expression(
                    condition, names,
                );
                if let Some(break_hook) = break_hook {
                    self.collect_effectful_iterator_assigned_binding_names_from_expression(
                        break_hook, names,
                    );
                }
                for statement in body {
                    self.collect_effectful_iterator_assigned_binding_names_from_statement(
                        statement, names,
                    );
                }
            }
            Statement::Break { .. } | Statement::Continue { .. } => {}
        }
    }

    fn collect_effectful_iterator_assigned_binding_names_from_expression(
        &self,
        expression: &Expression,
        names: &mut HashSet<String>,
    ) {
        match expression {
            Expression::Call { callee, arguments } => {
                if arguments.is_empty()
                    && let Expression::Member { object, property } = callee.as_ref()
                    && matches!(property.as_ref(), Expression::String(name) if name == "next")
                    && let Some(IteratorSourceKind::SimpleGenerator {
                        steps,
                        completion_effects,
                        ..
                    }) = self.resolve_iterator_source_kind(object)
                {
                    for step in &steps {
                        for effect in &step.effects {
                            collect_assigned_binding_names_from_statement(effect, names);
                        }
                    }
                    for effect in &completion_effects {
                        collect_assigned_binding_names_from_statement(effect, names);
                    }
                }
                collect_assigned_binding_names_from_expression(callee, names);
                for argument in arguments {
                    match argument {
                        CallArgument::Expression(argument) | CallArgument::Spread(argument) => {
                            self.collect_effectful_iterator_assigned_binding_names_from_expression(
                                argument, names,
                            );
                        }
                    }
                }
            }
            Expression::Member { object, property } => {
                self.collect_effectful_iterator_assigned_binding_names_from_expression(
                    object, names,
                );
                self.collect_effectful_iterator_assigned_binding_names_from_expression(
                    property, names,
                );
            }
            Expression::SuperMember { property } => {
                self.collect_effectful_iterator_assigned_binding_names_from_expression(
                    property, names,
                );
            }
            Expression::Assign { value, .. }
            | Expression::AssignSuperMember { value, .. }
            | Expression::Await(value)
            | Expression::EnumerateKeys(value)
            | Expression::GetIterator(value)
            | Expression::IteratorClose(value)
            | Expression::Unary {
                expression: value, ..
            } => {
                self.collect_effectful_iterator_assigned_binding_names_from_expression(value, names)
            }
            Expression::AssignMember {
                object,
                property,
                value,
            } => {
                self.collect_effectful_iterator_assigned_binding_names_from_expression(
                    object, names,
                );
                self.collect_effectful_iterator_assigned_binding_names_from_expression(
                    property, names,
                );
                self.collect_effectful_iterator_assigned_binding_names_from_expression(
                    value, names,
                );
            }
            Expression::Binary { left, right, .. } => {
                self.collect_effectful_iterator_assigned_binding_names_from_expression(left, names);
                self.collect_effectful_iterator_assigned_binding_names_from_expression(
                    right, names,
                );
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                self.collect_effectful_iterator_assigned_binding_names_from_expression(
                    condition, names,
                );
                self.collect_effectful_iterator_assigned_binding_names_from_expression(
                    then_expression,
                    names,
                );
                self.collect_effectful_iterator_assigned_binding_names_from_expression(
                    else_expression,
                    names,
                );
            }
            Expression::Sequence(expressions) => {
                for expression in expressions {
                    self.collect_effectful_iterator_assigned_binding_names_from_expression(
                        expression, names,
                    );
                }
            }
            Expression::New { callee, arguments } | Expression::SuperCall { callee, arguments } => {
                self.collect_effectful_iterator_assigned_binding_names_from_expression(
                    callee, names,
                );
                for argument in arguments {
                    match argument {
                        CallArgument::Expression(argument) | CallArgument::Spread(argument) => {
                            self.collect_effectful_iterator_assigned_binding_names_from_expression(
                                argument, names,
                            );
                        }
                    }
                }
            }
            Expression::Array(elements) => {
                for element in elements {
                    match element {
                        ArrayElement::Expression(expression) | ArrayElement::Spread(expression) => {
                            self.collect_effectful_iterator_assigned_binding_names_from_expression(
                                expression, names,
                            );
                        }
                    }
                }
            }
            Expression::Object(entries) => {
                for entry in entries {
                    match entry {
                        ObjectEntry::Data { key, value } => {
                            self.collect_effectful_iterator_assigned_binding_names_from_expression(
                                key, names,
                            );
                            self.collect_effectful_iterator_assigned_binding_names_from_expression(
                                value, names,
                            );
                        }
                        ObjectEntry::Getter { key, getter } => {
                            self.collect_effectful_iterator_assigned_binding_names_from_expression(
                                key, names,
                            );
                            self.collect_effectful_iterator_assigned_binding_names_from_expression(
                                getter, names,
                            );
                        }
                        ObjectEntry::Setter { key, setter } => {
                            self.collect_effectful_iterator_assigned_binding_names_from_expression(
                                key, names,
                            );
                            self.collect_effectful_iterator_assigned_binding_names_from_expression(
                                setter, names,
                            );
                        }
                        ObjectEntry::Spread(expression) => {
                            self.collect_effectful_iterator_assigned_binding_names_from_expression(
                                expression, names,
                            );
                        }
                    }
                }
            }
            Expression::Identifier(_)
            | Expression::Number(_)
            | Expression::BigInt(_)
            | Expression::String(_)
            | Expression::Bool(_)
            | Expression::Null
            | Expression::Undefined
            | Expression::This
            | Expression::NewTarget
            | Expression::Sent
            | Expression::Update { .. } => {}
        }
    }

    pub(in crate::backend::direct_wasm) fn collect_loop_assigned_binding_names_with_effectful_iterators(
        &self,
        condition: &Expression,
        break_hook: Option<&Expression>,
        body: &[Statement],
        init: Option<&[Statement]>,
        update: Option<&Expression>,
    ) -> HashSet<String> {
        let mut names = if let Some(init) = init {
            collect_loop_assigned_binding_names_from_for(
                init,
                Some(condition),
                update,
                break_hook,
                body,
            )
        } else {
            collect_loop_assigned_binding_names(condition, break_hook, body, None, update)
        };
        if let Some(init) = init {
            for statement in init {
                self.collect_effectful_iterator_assigned_binding_names_from_statement(
                    statement, &mut names,
                );
            }
        }
        self.collect_effectful_iterator_assigned_binding_names_from_expression(
            condition, &mut names,
        );
        if let Some(update) = update {
            self.collect_effectful_iterator_assigned_binding_names_from_expression(
                update, &mut names,
            );
        }
        if let Some(break_hook) = break_hook {
            self.collect_effectful_iterator_assigned_binding_names_from_expression(
                break_hook, &mut names,
            );
        }
        for statement in body {
            self.collect_effectful_iterator_assigned_binding_names_from_statement(
                statement, &mut names,
            );
        }
        names
    }
}
