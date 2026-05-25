use super::*;

#[path = "parameter_consumption/expression_traversal.rs"]
mod expression_traversal;
#[path = "parameter_consumption/statement_traversal.rs"]
mod statement_traversal;

impl<'a> FunctionCompiler<'a> {
    pub(in crate::backend::direct_wasm) fn user_function_parameter_iterator_consumption_indices(
        &self,
        user_function: &UserFunction,
    ) -> HashSet<usize> {
        let Some(function) = self.resolve_registered_function_declaration(&user_function.name)
        else {
            return HashSet::new();
        };
        let param_names = user_function.params.iter().cloned().collect::<HashSet<_>>();
        let mut consumed_names = HashSet::new();
        Self::collect_parameter_get_iterator_names_from_statements(
            &function.body,
            &param_names,
            &mut consumed_names,
        );
        user_function
            .params
            .iter()
            .enumerate()
            .filter_map(|(index, param_name)| {
                if consumed_names.contains(param_name) {
                    return Some(index);
                }
                let mut aliases = HashSet::from([param_name.clone()]);
                Self::parameter_iterator_alias_consumed_by_statements(&function.body, &mut aliases)
                    .then_some(index)
            })
            .collect()
    }

    fn parameter_iterator_alias_consumed_by_statements(
        statements: &[Statement],
        aliases: &mut HashSet<String>,
    ) -> bool {
        for statement in statements {
            if Self::parameter_iterator_alias_consumed_by_statement(statement, aliases) {
                return true;
            }
        }
        false
    }

    fn parameter_iterator_alias_consumed_by_statement(
        statement: &Statement,
        aliases: &mut HashSet<String>,
    ) -> bool {
        match statement {
            Statement::Let { name, value, .. }
            | Statement::Var { name, value }
            | Statement::Assign { name, value } => {
                if Self::parameter_iterator_alias_consumed_by_expression(value, aliases) {
                    return true;
                }
                if let Expression::Identifier(source_name) = value
                    && aliases.contains(source_name)
                {
                    aliases.insert(name.clone());
                }
                false
            }
            Statement::Expression(expression)
            | Statement::Return(expression)
            | Statement::Throw(expression)
            | Statement::Yield { value: expression }
            | Statement::YieldDelegate { value: expression } => {
                Self::parameter_iterator_alias_consumed_by_expression(expression, aliases)
            }
            Statement::AssignMember {
                object,
                property,
                value,
            } => {
                Self::parameter_iterator_alias_consumed_by_expression(object, aliases)
                    || Self::parameter_iterator_alias_consumed_by_expression(property, aliases)
                    || Self::parameter_iterator_alias_consumed_by_expression(value, aliases)
            }
            Statement::Print { values } => values
                .iter()
                .any(|value| Self::parameter_iterator_alias_consumed_by_expression(value, aliases)),
            Statement::Block { body }
            | Statement::Declaration { body }
            | Statement::Labeled { body, .. }
            | Statement::With { body, .. } => {
                let mut scoped_aliases = aliases.clone();
                Self::parameter_iterator_alias_consumed_by_statements(body, &mut scoped_aliases)
            }
            Statement::If {
                condition,
                then_branch,
                else_branch,
            } => {
                if Self::parameter_iterator_alias_consumed_by_expression(condition, aliases) {
                    return true;
                }
                let mut then_aliases = aliases.clone();
                let mut else_aliases = aliases.clone();
                Self::parameter_iterator_alias_consumed_by_statements(
                    then_branch,
                    &mut then_aliases,
                ) || Self::parameter_iterator_alias_consumed_by_statements(
                    else_branch,
                    &mut else_aliases,
                )
            }
            Statement::Try {
                body,
                catch_setup,
                catch_body,
                ..
            } => {
                let mut body_aliases = aliases.clone();
                let mut setup_aliases = aliases.clone();
                let mut catch_aliases = aliases.clone();
                Self::parameter_iterator_alias_consumed_by_statements(body, &mut body_aliases)
                    || Self::parameter_iterator_alias_consumed_by_statements(
                        catch_setup,
                        &mut setup_aliases,
                    )
                    || Self::parameter_iterator_alias_consumed_by_statements(
                        catch_body,
                        &mut catch_aliases,
                    )
            }
            Statement::For {
                init,
                condition,
                update,
                break_hook,
                body,
                ..
            } => {
                let mut scoped_aliases = aliases.clone();
                Self::parameter_iterator_alias_consumed_by_statements(init, &mut scoped_aliases)
                    || condition.as_ref().is_some_and(|condition| {
                        Self::parameter_iterator_alias_consumed_by_expression(
                            condition,
                            &mut scoped_aliases,
                        )
                    })
                    || update.as_ref().is_some_and(|update| {
                        Self::parameter_iterator_alias_consumed_by_expression(
                            update,
                            &mut scoped_aliases,
                        )
                    })
                    || break_hook.as_ref().is_some_and(|break_hook| {
                        Self::parameter_iterator_alias_consumed_by_expression(
                            break_hook,
                            &mut scoped_aliases,
                        )
                    })
                    || Self::parameter_iterator_alias_consumed_by_statements(
                        body,
                        &mut scoped_aliases,
                    )
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
                Self::parameter_iterator_alias_consumed_by_expression(condition, aliases)
                    || break_hook.as_ref().is_some_and(|break_hook| {
                        Self::parameter_iterator_alias_consumed_by_expression(break_hook, aliases)
                    })
                    || Self::parameter_iterator_alias_consumed_by_statements(body, aliases)
            }
            Statement::Switch {
                discriminant,
                cases,
                ..
            } => {
                if Self::parameter_iterator_alias_consumed_by_expression(discriminant, aliases) {
                    return true;
                }
                cases.iter().any(|case| {
                    let mut case_aliases = aliases.clone();
                    case.test.as_ref().is_some_and(|test| {
                        Self::parameter_iterator_alias_consumed_by_expression(
                            test,
                            &mut case_aliases,
                        )
                    }) || Self::parameter_iterator_alias_consumed_by_statements(
                        &case.body,
                        &mut case_aliases,
                    )
                })
            }
            Statement::Break { .. } | Statement::Continue { .. } => false,
        }
    }

    fn parameter_iterator_alias_consumed_by_expression(
        expression: &Expression,
        aliases: &mut HashSet<String>,
    ) -> bool {
        if let Expression::GetIterator(value) = expression
            && let Expression::Identifier(name) = value.as_ref()
            && aliases.contains(name)
        {
            return true;
        }
        match expression {
            Expression::Member { object, property } => {
                Self::parameter_iterator_alias_consumed_by_expression(object, aliases)
                    || Self::parameter_iterator_alias_consumed_by_expression(property, aliases)
            }
            Expression::SuperMember { property } => {
                Self::parameter_iterator_alias_consumed_by_expression(property, aliases)
            }
            Expression::Assign { value, .. }
            | Expression::Await(value)
            | Expression::EnumerateKeys(value)
            | Expression::GetIterator(value)
            | Expression::IteratorClose(value)
            | Expression::Unary {
                expression: value, ..
            } => Self::parameter_iterator_alias_consumed_by_expression(value, aliases),
            Expression::AssignMember {
                object,
                property,
                value,
            } => {
                Self::parameter_iterator_alias_consumed_by_expression(object, aliases)
                    || Self::parameter_iterator_alias_consumed_by_expression(property, aliases)
                    || Self::parameter_iterator_alias_consumed_by_expression(value, aliases)
            }
            Expression::AssignSuperMember { property, value } => {
                Self::parameter_iterator_alias_consumed_by_expression(property, aliases)
                    || Self::parameter_iterator_alias_consumed_by_expression(value, aliases)
            }
            Expression::Binary { left, right, .. } => {
                Self::parameter_iterator_alias_consumed_by_expression(left, aliases)
                    || Self::parameter_iterator_alias_consumed_by_expression(right, aliases)
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                Self::parameter_iterator_alias_consumed_by_expression(condition, aliases)
                    || Self::parameter_iterator_alias_consumed_by_expression(
                        then_expression,
                        aliases,
                    )
                    || Self::parameter_iterator_alias_consumed_by_expression(
                        else_expression,
                        aliases,
                    )
            }
            Expression::Sequence(expressions) => expressions.iter().any(|expression| {
                Self::parameter_iterator_alias_consumed_by_expression(expression, aliases)
            }),
            Expression::Call { callee, arguments }
            | Expression::SuperCall { callee, arguments }
            | Expression::New { callee, arguments } => {
                Self::parameter_iterator_alias_consumed_by_expression(callee, aliases)
                    || arguments.iter().any(|argument| match argument {
                        CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                            Self::parameter_iterator_alias_consumed_by_expression(
                                expression, aliases,
                            )
                        }
                    })
            }
            Expression::Array(elements) => elements.iter().any(|element| match element {
                ArrayElement::Expression(expression) | ArrayElement::Spread(expression) => {
                    Self::parameter_iterator_alias_consumed_by_expression(expression, aliases)
                }
            }),
            Expression::Object(entries) => entries.iter().any(|entry| match entry {
                ObjectEntry::Data { key, value } => {
                    Self::parameter_iterator_alias_consumed_by_expression(key, aliases)
                        || Self::parameter_iterator_alias_consumed_by_expression(value, aliases)
                }
                ObjectEntry::Getter { key, getter } => {
                    Self::parameter_iterator_alias_consumed_by_expression(key, aliases)
                        || Self::parameter_iterator_alias_consumed_by_expression(getter, aliases)
                }
                ObjectEntry::Setter { key, setter } => {
                    Self::parameter_iterator_alias_consumed_by_expression(key, aliases)
                        || Self::parameter_iterator_alias_consumed_by_expression(setter, aliases)
                }
                ObjectEntry::Spread(expression) => {
                    Self::parameter_iterator_alias_consumed_by_expression(expression, aliases)
                }
            }),
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
            | Expression::Update { .. } => false,
        }
    }
}
