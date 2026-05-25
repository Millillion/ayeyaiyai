use super::*;

impl<'a> FunctionCompiler<'a> {
    pub(in crate::backend::direct_wasm) fn substitute_statement_bindings(
        &self,
        statement: &Statement,
        bindings: &HashMap<String, Expression>,
    ) -> Statement {
        let substitute_name = |name: &str| {
            bindings
                .get(name)
                .and_then(|value| match value {
                    Expression::Identifier(replacement) => Some(replacement.clone()),
                    _ => None,
                })
                .unwrap_or_else(|| name.to_string())
        };

        match statement {
            Statement::Declaration { body } => Statement::Declaration {
                body: body
                    .iter()
                    .map(|statement| self.substitute_statement_bindings(statement, bindings))
                    .collect(),
            },
            Statement::Block { body } => Statement::Block {
                body: body
                    .iter()
                    .map(|statement| self.substitute_statement_bindings(statement, bindings))
                    .collect(),
            },
            Statement::Labeled { labels, body } => Statement::Labeled {
                labels: labels.clone(),
                body: body
                    .iter()
                    .map(|statement| self.substitute_statement_bindings(statement, bindings))
                    .collect(),
            },
            Statement::Var { name, value } => Statement::Var {
                name: substitute_name(name),
                value: self.substitute_expression_bindings(value, bindings),
            },
            Statement::Let {
                name,
                mutable,
                value,
            } => Statement::Let {
                name: substitute_name(name),
                mutable: *mutable,
                value: self.substitute_expression_bindings(value, bindings),
            },
            Statement::Assign { name, value } => Statement::Assign {
                name: substitute_name(name),
                value: self.substitute_expression_bindings(value, bindings),
            },
            Statement::AssignMember {
                object,
                property,
                value,
            } => Statement::AssignMember {
                object: self.substitute_expression_bindings(object, bindings),
                property: self.substitute_expression_bindings(property, bindings),
                value: self.substitute_expression_bindings(value, bindings),
            },
            Statement::Print { values } => Statement::Print {
                values: values
                    .iter()
                    .map(|value| self.substitute_expression_bindings(value, bindings))
                    .collect(),
            },
            Statement::Expression(expression) => {
                Statement::Expression(self.substitute_expression_bindings(expression, bindings))
            }
            Statement::Throw(value) => {
                Statement::Throw(self.substitute_expression_bindings(value, bindings))
            }
            Statement::Return(value) => {
                Statement::Return(self.substitute_expression_bindings(value, bindings))
            }
            Statement::Yield { value } => Statement::Yield {
                value: self.substitute_expression_bindings(value, bindings),
            },
            Statement::YieldDelegate { value } => Statement::YieldDelegate {
                value: self.substitute_expression_bindings(value, bindings),
            },
            Statement::If {
                condition,
                then_branch,
                else_branch,
            } => Statement::If {
                condition: self.substitute_expression_bindings(condition, bindings),
                then_branch: then_branch
                    .iter()
                    .map(|statement| self.substitute_statement_bindings(statement, bindings))
                    .collect(),
                else_branch: else_branch
                    .iter()
                    .map(|statement| self.substitute_statement_bindings(statement, bindings))
                    .collect(),
            },
            Statement::Try {
                body,
                catch_binding,
                catch_setup,
                catch_body,
            } => Statement::Try {
                body: body
                    .iter()
                    .map(|statement| self.substitute_statement_bindings(statement, bindings))
                    .collect(),
                catch_binding: catch_binding.clone(),
                catch_setup: catch_setup
                    .iter()
                    .map(|statement| self.substitute_statement_bindings(statement, bindings))
                    .collect(),
                catch_body: catch_body
                    .iter()
                    .map(|statement| self.substitute_statement_bindings(statement, bindings))
                    .collect(),
            },
            Statement::Switch {
                labels,
                bindings: switch_bindings,
                discriminant,
                cases,
            } => Statement::Switch {
                labels: labels.clone(),
                bindings: switch_bindings.clone(),
                discriminant: self.substitute_expression_bindings(discriminant, bindings),
                cases: cases
                    .iter()
                    .map(|case| crate::ir::hir::SwitchCase {
                        test: case
                            .test
                            .as_ref()
                            .map(|test| self.substitute_expression_bindings(test, bindings)),
                        body: case
                            .body
                            .iter()
                            .map(|statement| {
                                self.substitute_statement_bindings(statement, bindings)
                            })
                            .collect(),
                    })
                    .collect(),
            },
            Statement::For {
                labels,
                init,
                per_iteration_bindings,
                condition,
                update,
                break_hook,
                body,
            } => Statement::For {
                labels: labels.clone(),
                init: init
                    .iter()
                    .map(|statement| self.substitute_statement_bindings(statement, bindings))
                    .collect(),
                per_iteration_bindings: per_iteration_bindings.clone(),
                condition: condition
                    .as_ref()
                    .map(|condition| self.substitute_expression_bindings(condition, bindings)),
                update: update
                    .as_ref()
                    .map(|update| self.substitute_expression_bindings(update, bindings)),
                break_hook: break_hook
                    .as_ref()
                    .map(|break_hook| self.substitute_expression_bindings(break_hook, bindings)),
                body: body
                    .iter()
                    .map(|statement| self.substitute_statement_bindings(statement, bindings))
                    .collect(),
            },
            Statement::While {
                labels,
                condition,
                break_hook,
                body,
            } => Statement::While {
                labels: labels.clone(),
                condition: self.substitute_expression_bindings(condition, bindings),
                break_hook: break_hook
                    .as_ref()
                    .map(|break_hook| self.substitute_expression_bindings(break_hook, bindings)),
                body: body
                    .iter()
                    .map(|statement| self.substitute_statement_bindings(statement, bindings))
                    .collect(),
            },
            Statement::DoWhile {
                labels,
                condition,
                break_hook,
                body,
            } => Statement::DoWhile {
                labels: labels.clone(),
                condition: self.substitute_expression_bindings(condition, bindings),
                break_hook: break_hook
                    .as_ref()
                    .map(|break_hook| self.substitute_expression_bindings(break_hook, bindings)),
                body: body
                    .iter()
                    .map(|statement| self.substitute_statement_bindings(statement, bindings))
                    .collect(),
            },
            Statement::With { object, body } => Statement::With {
                object: self.substitute_expression_bindings(object, bindings),
                body: body
                    .iter()
                    .map(|statement| self.substitute_statement_bindings(statement, bindings))
                    .collect(),
            },
            _ => statement.clone(),
        }
    }

    pub(in crate::backend::direct_wasm) fn substitute_async_yield_delegate_generator_plan_scope_bindings(
        &self,
        plan: &AsyncYieldDelegateGeneratorPlan,
        bindings: &HashMap<String, Expression>,
    ) -> AsyncYieldDelegateGeneratorPlan {
        AsyncYieldDelegateGeneratorPlan {
            function_name: plan.function_name.clone(),
            prefix_effects: plan
                .prefix_effects
                .iter()
                .map(|statement| self.substitute_statement_bindings(statement, bindings))
                .collect(),
            delegate_expression: self
                .substitute_expression_bindings(&plan.delegate_expression, bindings),
            completion_effects: plan
                .completion_effects
                .iter()
                .map(|statement| self.substitute_statement_bindings(statement, bindings))
                .collect(),
            completion_value: self.substitute_expression_bindings(&plan.completion_value, bindings),
            completion_throw_value: plan
                .completion_throw_value
                .as_ref()
                .map(|value| self.substitute_expression_bindings(value, bindings)),
            scope_bindings: Vec::new(),
        }
    }
}
