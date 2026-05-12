use super::*;
use crate::ir::hir::SwitchCase;

impl<'a> FunctionCompiler<'a> {
    pub(in crate::backend::direct_wasm) fn substitute_simple_generator_statements_with_call_frame_bindings(
        &self,
        statements: &[Statement],
        user_function: &UserFunction,
        mapped_arguments: bool,
        call_argument_values: &mut Vec<Expression>,
        arguments_values: &mut Vec<Expression>,
        this_binding: &Expression,
    ) -> Option<Vec<Statement>> {
        let mut arguments_binding_override = None;
        self.substitute_simple_generator_statements_with_call_frame_bindings_in_scope(
            statements,
            user_function,
            mapped_arguments,
            call_argument_values,
            arguments_values,
            this_binding,
            &mut arguments_binding_override,
        )
    }

    fn simple_generator_arguments_rebinding_name(user_function: &UserFunction) -> String {
        format!("__ayy_simple_gen_arguments_{}", user_function.name)
    }

    fn simple_generator_assignment_targets_arguments(
        &self,
        user_function: &UserFunction,
        name: &str,
    ) -> bool {
        !self.simple_generator_arguments_are_shadowed(user_function)
            && (name == "arguments"
                || scoped_binding_source_name(name)
                    .is_some_and(|source_name| source_name == "arguments"))
    }

    fn substitute_simple_generator_statements_with_call_frame_bindings_in_scope(
        &self,
        statements: &[Statement],
        user_function: &UserFunction,
        mapped_arguments: bool,
        call_argument_values: &mut Vec<Expression>,
        arguments_values: &mut Vec<Expression>,
        this_binding: &Expression,
        arguments_binding_override: &mut Option<Expression>,
    ) -> Option<Vec<Statement>> {
        let mut transformed = Vec::with_capacity(statements.len());
        for statement in statements {
            let call_arguments = self.simple_generator_call_arguments(call_argument_values);
            let arguments_binding = arguments_binding_override.clone().unwrap_or_else(|| {
                self.simple_generator_arguments_binding_expression(arguments_values)
            });
            let substituted = match statement {
                Statement::Block { body } => Statement::Block {
                    body: self.substitute_simple_generator_statements_with_call_frame_bindings_in_scope(
                        body,
                        user_function,
                        mapped_arguments,
                        call_argument_values,
                        arguments_values,
                        this_binding,
                        arguments_binding_override,
                    )?,
                },
                Statement::Declaration { body } => Statement::Declaration {
                    body: self.substitute_simple_generator_statements_with_call_frame_bindings_in_scope(
                        body,
                        user_function,
                        mapped_arguments,
                        call_argument_values,
                        arguments_values,
                        this_binding,
                        arguments_binding_override,
                    )?,
                },
                Statement::Labeled { labels, body } => Statement::Labeled {
                    labels: labels.clone(),
                    body: self.substitute_simple_generator_statements_with_call_frame_bindings_in_scope(
                        body,
                        user_function,
                        mapped_arguments,
                        call_argument_values,
                        arguments_values,
                        this_binding,
                        arguments_binding_override,
                    )?,
                },
                Statement::Assign { name, value }
                    if self.simple_generator_assignment_targets_arguments(user_function, name) =>
                {
                    let rebound_name = Self::simple_generator_arguments_rebinding_name(user_function);
                    let value = self.substitute_user_function_call_frame_bindings(
                        value,
                        user_function,
                        &call_arguments,
                        this_binding,
                        &arguments_binding,
                    );
                    *arguments_binding_override = Some(Expression::Identifier(rebound_name.clone()));
                    Statement::Assign {
                        name: rebound_name,
                        value,
                    }
                }
                Statement::Assign { name, value } => Statement::Assign {
                    name: name.clone(),
                    value: self.substitute_user_function_call_frame_bindings(
                        value,
                        user_function,
                        &call_arguments,
                        this_binding,
                        &arguments_binding,
                    ),
                },
                Statement::Var { name, value } => Statement::Var {
                    name: name.clone(),
                    value: self.substitute_user_function_call_frame_bindings(
                        value,
                        user_function,
                        &call_arguments,
                        this_binding,
                        &arguments_binding,
                    ),
                },
                Statement::Let {
                    name,
                    mutable,
                    value,
                } => Statement::Let {
                    name: name.clone(),
                    mutable: *mutable,
                    value: self.substitute_user_function_call_frame_bindings(
                        value,
                        user_function,
                        &call_arguments,
                        this_binding,
                        &arguments_binding,
                    ),
                },
                Statement::AssignMember {
                    object,
                    property,
                    value,
                } => Statement::AssignMember {
                    object: self.substitute_user_function_call_frame_bindings(
                        object,
                        user_function,
                        &call_arguments,
                        this_binding,
                        &arguments_binding,
                    ),
                    property: self.substitute_user_function_call_frame_bindings(
                        property,
                        user_function,
                        &call_arguments,
                        this_binding,
                        &arguments_binding,
                    ),
                    value: self.substitute_user_function_call_frame_bindings(
                        value,
                        user_function,
                        &call_arguments,
                        this_binding,
                        &arguments_binding,
                    ),
                },
                Statement::Print { values } => Statement::Print {
                    values: values
                        .iter()
                        .map(|value| {
                            self.substitute_user_function_call_frame_bindings(
                                value,
                                user_function,
                                &call_arguments,
                                this_binding,
                                &arguments_binding,
                            )
                        })
                        .collect(),
                },
                Statement::Expression(expression) => {
                    Statement::Expression(self.substitute_user_function_call_frame_bindings(
                        expression,
                        user_function,
                        &call_arguments,
                        this_binding,
                        &arguments_binding,
                    ))
                }
                Statement::Throw(value) => {
                    Statement::Throw(self.substitute_user_function_call_frame_bindings(
                        value,
                        user_function,
                        &call_arguments,
                        this_binding,
                        &arguments_binding,
                    ))
                }
                Statement::Return(value) => {
                    Statement::Return(self.substitute_user_function_call_frame_bindings(
                        value,
                        user_function,
                        &call_arguments,
                        this_binding,
                        &arguments_binding,
                    ))
                }
                Statement::Yield { value } => Statement::Yield {
                    value: self.substitute_user_function_call_frame_bindings(
                        value,
                        user_function,
                        &call_arguments,
                        this_binding,
                        &arguments_binding,
                    ),
                },
                Statement::YieldDelegate { value } => Statement::YieldDelegate {
                    value: self.substitute_user_function_call_frame_bindings(
                        value,
                        user_function,
                        &call_arguments,
                        this_binding,
                        &arguments_binding,
                    ),
                },
                Statement::Break { label } => Statement::Break {
                    label: label.clone(),
                },
                Statement::Continue { label } => Statement::Continue {
                    label: label.clone(),
                },
                Statement::With { object, body } => Statement::With {
                    object: self.substitute_user_function_call_frame_bindings(
                        object,
                        user_function,
                        &call_arguments,
                        this_binding,
                        &arguments_binding,
                    ),
                    body: self.substitute_simple_generator_statements_with_call_frame_bindings_in_scope(
                        body,
                        user_function,
                        mapped_arguments,
                        call_argument_values,
                        arguments_values,
                        this_binding,
                        arguments_binding_override,
                    )?,
                },
                Statement::If {
                    condition,
                    then_branch,
                    else_branch,
                } => {
                    let substituted_condition = self.substitute_user_function_call_frame_bindings(
                        condition,
                        user_function,
                        &call_arguments,
                        this_binding,
                        &arguments_binding,
                    );
                    if let Some(condition_value) =
                        self.resolve_static_if_condition_value(&substituted_condition)
                    {
                        let branch = if condition_value {
                            then_branch
                        } else {
                            else_branch
                        };
                        let Some(body) = self.substitute_simple_generator_statements_with_call_frame_bindings_in_scope(
                            branch,
                            user_function,
                            mapped_arguments,
                            call_argument_values,
                            arguments_values,
                            this_binding,
                            arguments_binding_override,
                        ) else {
                            return None;
                        };
                        Statement::Block {
                            body,
                        }
                    } else {
                        let Some(then_branch) = self.substitute_simple_generator_statements_with_call_frame_bindings_in_scope(
                            then_branch,
                            user_function,
                            mapped_arguments,
                            call_argument_values,
                            arguments_values,
                            this_binding,
                            &mut arguments_binding_override.clone(),
                        ) else {
                            return None;
                        };
                        let Some(else_branch) = self.substitute_simple_generator_statements_with_call_frame_bindings_in_scope(
                            else_branch,
                            user_function,
                            mapped_arguments,
                            call_argument_values,
                            arguments_values,
                            this_binding,
                            &mut arguments_binding_override.clone(),
                        ) else {
                            return None;
                        };
                        Statement::If {
                            condition: substituted_condition,
                            then_branch,
                            else_branch,
                        }
                    }
                }
                Statement::Try {
                    body,
                    catch_binding,
                    catch_setup,
                    catch_body,
                } => Statement::Try {
                    body: self.substitute_simple_generator_statements_with_call_frame_bindings_in_scope(
                        body,
                        user_function,
                        mapped_arguments,
                        call_argument_values,
                        arguments_values,
                        this_binding,
                        arguments_binding_override,
                    )?,
                    catch_binding: catch_binding.clone(),
                    catch_setup: self
                        .substitute_simple_generator_statements_with_call_frame_bindings_in_scope(
                            catch_setup,
                            user_function,
                            mapped_arguments,
                            call_argument_values,
                            arguments_values,
                            this_binding,
                            arguments_binding_override,
                        )?,
                    catch_body: self
                        .substitute_simple_generator_statements_with_call_frame_bindings_in_scope(
                            catch_body,
                            user_function,
                            mapped_arguments,
                            call_argument_values,
                            arguments_values,
                            this_binding,
                            arguments_binding_override,
                        )?,
                },
                Statement::Switch {
                    labels,
                    bindings,
                    discriminant,
                    cases,
                } => Statement::Switch {
                    labels: labels.clone(),
                    bindings: bindings.clone(),
                    discriminant: self.substitute_user_function_call_frame_bindings(
                        discriminant,
                        user_function,
                        &call_arguments,
                        this_binding,
                        &arguments_binding,
                    ),
                    cases: cases
                        .iter()
                        .map(|case| {
                            Some(SwitchCase {
                                test: case.test.as_ref().map(|test| {
                                    self.substitute_user_function_call_frame_bindings(
                                        test,
                                        user_function,
                                        &call_arguments,
                                        this_binding,
                                        &arguments_binding,
                                    )
                                }),
                                body: self
                                    .substitute_simple_generator_statements_with_call_frame_bindings_in_scope(
                                        &case.body,
                                        user_function,
                                        mapped_arguments,
                                        call_argument_values,
                                        arguments_values,
                                        this_binding,
                                        &mut arguments_binding_override.clone(),
                                    )?,
                            })
                        })
                        .collect::<Option<Vec<_>>>()?,
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
                    init: self.substitute_simple_generator_statements_with_call_frame_bindings_in_scope(
                        init,
                        user_function,
                        mapped_arguments,
                        call_argument_values,
                        arguments_values,
                        this_binding,
                        arguments_binding_override,
                    )?,
                    per_iteration_bindings: per_iteration_bindings.clone(),
                    condition: condition.as_ref().map(|condition| {
                        self.substitute_user_function_call_frame_bindings(
                            condition,
                            user_function,
                            &call_arguments,
                            this_binding,
                            &arguments_binding,
                        )
                    }),
                    update: update.as_ref().map(|update| {
                        self.substitute_user_function_call_frame_bindings(
                            update,
                            user_function,
                            &call_arguments,
                            this_binding,
                            &arguments_binding,
                        )
                    }),
                    break_hook: break_hook.as_ref().map(|break_hook| {
                        self.substitute_user_function_call_frame_bindings(
                            break_hook,
                            user_function,
                            &call_arguments,
                            this_binding,
                            &arguments_binding,
                        )
                    }),
                    body: self.substitute_simple_generator_statements_with_call_frame_bindings_in_scope(
                        body,
                        user_function,
                        mapped_arguments,
                        call_argument_values,
                        arguments_values,
                        this_binding,
                        arguments_binding_override,
                    )?,
                },
                Statement::While {
                    labels,
                    condition,
                    break_hook,
                    body,
                } => {
                    let substituted_condition = self.substitute_user_function_call_frame_bindings(
                        condition,
                        user_function,
                        &call_arguments,
                        this_binding,
                        &arguments_binding,
                    );
                    let substituted_break_hook = break_hook.as_ref().map(|break_hook| {
                        self.substitute_user_function_call_frame_bindings(
                            break_hook,
                            user_function,
                            &call_arguments,
                            this_binding,
                            &arguments_binding,
                        )
                    });
                    let Some(body) = self.substitute_simple_generator_statements_with_call_frame_bindings_in_scope(
                        body,
                        user_function,
                        mapped_arguments,
                        call_argument_values,
                        arguments_values,
                        this_binding,
                        arguments_binding_override,
                    ) else {
                        return None;
                    };
                    Statement::While {
                        labels: labels.clone(),
                        condition: substituted_condition,
                        break_hook: substituted_break_hook,
                        body,
                    }
                }
                Statement::DoWhile {
                    labels,
                    condition,
                    break_hook,
                    body,
                } => Statement::DoWhile {
                    labels: labels.clone(),
                    condition: self.substitute_user_function_call_frame_bindings(
                        condition,
                        user_function,
                        &call_arguments,
                        this_binding,
                        &arguments_binding,
                    ),
                    break_hook: break_hook.as_ref().map(|break_hook| {
                        self.substitute_user_function_call_frame_bindings(
                            break_hook,
                            user_function,
                            &call_arguments,
                            this_binding,
                            &arguments_binding,
                        )
                    }),
                    body: self.substitute_simple_generator_statements_with_call_frame_bindings_in_scope(
                        body,
                        user_function,
                        mapped_arguments,
                        call_argument_values,
                        arguments_values,
                        this_binding,
                        arguments_binding_override,
                    )?,
                },
            };
            self.update_simple_generator_call_frame_state(
                statement,
                &substituted,
                user_function,
                mapped_arguments,
                call_argument_values,
                arguments_values,
            );
            transformed.push(substituted);
        }
        Some(transformed)
    }

    pub(super) fn split_simple_generator_completion(
        &self,
        mut statements: Vec<Statement>,
    ) -> Option<(Vec<Statement>, Expression)> {
        let completion_value = if let Some(Statement::Return(value)) = statements.last() {
            let value = value.clone();
            statements.pop();
            value
        } else {
            Expression::Undefined
        };
        if statements
            .iter()
            .any(|statement| matches!(statement, Statement::Return(_)))
        {
            return None;
        }
        Some((statements, completion_value))
    }
}
