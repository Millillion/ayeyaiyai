use super::*;

impl<'a> FunctionCompiler<'a> {
    pub(in crate::backend::direct_wasm) fn substitute_statement_call_frame_bindings(
        &self,
        statement: &Statement,
        user_function: &UserFunction,
        call_arguments: &[CallArgument],
        this_binding: &Expression,
        arguments_binding: &Expression,
    ) -> Statement {
        let substitute_name = |name: &str| match self.substitute_user_function_call_frame_bindings(
            &Expression::Identifier(name.to_string()),
            user_function,
            call_arguments,
            this_binding,
            arguments_binding,
        ) {
            Expression::Identifier(name) => name,
            _ => name.to_string(),
        };

        match statement {
            Statement::Block { body } => Statement::Block {
                body: body
                    .iter()
                    .map(|statement| {
                        self.substitute_statement_call_frame_bindings(
                            statement,
                            user_function,
                            call_arguments,
                            this_binding,
                            arguments_binding,
                        )
                    })
                    .collect(),
            },
            Statement::Var { name, value } => Statement::Var {
                name: substitute_name(name),
                value: self.substitute_user_function_call_frame_bindings(
                    value,
                    user_function,
                    call_arguments,
                    this_binding,
                    arguments_binding,
                ),
            },
            Statement::Let {
                name,
                mutable,
                value,
            } => Statement::Let {
                name: substitute_name(name),
                mutable: *mutable,
                value: self.substitute_user_function_call_frame_bindings(
                    value,
                    user_function,
                    call_arguments,
                    this_binding,
                    arguments_binding,
                ),
            },
            Statement::Assign { name, value } => Statement::Assign {
                name: substitute_name(name),
                value: self.substitute_user_function_call_frame_bindings(
                    value,
                    user_function,
                    call_arguments,
                    this_binding,
                    arguments_binding,
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
                    call_arguments,
                    this_binding,
                    arguments_binding,
                ),
                property: self.substitute_user_function_call_frame_bindings(
                    property,
                    user_function,
                    call_arguments,
                    this_binding,
                    arguments_binding,
                ),
                value: self.substitute_user_function_call_frame_bindings(
                    value,
                    user_function,
                    call_arguments,
                    this_binding,
                    arguments_binding,
                ),
            },
            Statement::Print { values } => Statement::Print {
                values: values
                    .iter()
                    .map(|value| {
                        self.substitute_user_function_call_frame_bindings(
                            value,
                            user_function,
                            call_arguments,
                            this_binding,
                            arguments_binding,
                        )
                    })
                    .collect(),
            },
            Statement::Expression(expression) => {
                Statement::Expression(self.substitute_user_function_call_frame_bindings(
                    expression,
                    user_function,
                    call_arguments,
                    this_binding,
                    arguments_binding,
                ))
            }
            Statement::Throw(expression) => {
                Statement::Throw(self.substitute_user_function_call_frame_bindings(
                    expression,
                    user_function,
                    call_arguments,
                    this_binding,
                    arguments_binding,
                ))
            }
            Statement::Return(expression) => {
                Statement::Return(self.substitute_user_function_call_frame_bindings(
                    expression,
                    user_function,
                    call_arguments,
                    this_binding,
                    arguments_binding,
                ))
            }
            Statement::If {
                condition,
                then_branch,
                else_branch,
            } => Statement::If {
                condition: self.substitute_user_function_call_frame_bindings(
                    condition,
                    user_function,
                    call_arguments,
                    this_binding,
                    arguments_binding,
                ),
                then_branch: then_branch
                    .iter()
                    .map(|statement| {
                        self.substitute_statement_call_frame_bindings(
                            statement,
                            user_function,
                            call_arguments,
                            this_binding,
                            arguments_binding,
                        )
                    })
                    .collect(),
                else_branch: else_branch
                    .iter()
                    .map(|statement| {
                        self.substitute_statement_call_frame_bindings(
                            statement,
                            user_function,
                            call_arguments,
                            this_binding,
                            arguments_binding,
                        )
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
                    .map(|statement| {
                        self.substitute_statement_call_frame_bindings(
                            statement,
                            user_function,
                            call_arguments,
                            this_binding,
                            arguments_binding,
                        )
                    })
                    .collect(),
                per_iteration_bindings: per_iteration_bindings.clone(),
                condition: condition.as_ref().map(|condition| {
                    self.substitute_user_function_call_frame_bindings(
                        condition,
                        user_function,
                        call_arguments,
                        this_binding,
                        arguments_binding,
                    )
                }),
                update: update.as_ref().map(|update| {
                    self.substitute_user_function_call_frame_bindings(
                        update,
                        user_function,
                        call_arguments,
                        this_binding,
                        arguments_binding,
                    )
                }),
                break_hook: break_hook.as_ref().map(|break_hook| {
                    self.substitute_user_function_call_frame_bindings(
                        break_hook,
                        user_function,
                        call_arguments,
                        this_binding,
                        arguments_binding,
                    )
                }),
                body: body
                    .iter()
                    .map(|statement| {
                        self.substitute_statement_call_frame_bindings(
                            statement,
                            user_function,
                            call_arguments,
                            this_binding,
                            arguments_binding,
                        )
                    })
                    .collect(),
            },
            Statement::While {
                labels,
                condition,
                break_hook,
                body,
            } => Statement::While {
                labels: labels.clone(),
                condition: self.substitute_user_function_call_frame_bindings(
                    condition,
                    user_function,
                    call_arguments,
                    this_binding,
                    arguments_binding,
                ),
                break_hook: break_hook.as_ref().map(|break_hook| {
                    self.substitute_user_function_call_frame_bindings(
                        break_hook,
                        user_function,
                        call_arguments,
                        this_binding,
                        arguments_binding,
                    )
                }),
                body: body
                    .iter()
                    .map(|statement| {
                        self.substitute_statement_call_frame_bindings(
                            statement,
                            user_function,
                            call_arguments,
                            this_binding,
                            arguments_binding,
                        )
                    })
                    .collect(),
            },
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
                    call_arguments,
                    this_binding,
                    arguments_binding,
                ),
                break_hook: break_hook.as_ref().map(|break_hook| {
                    self.substitute_user_function_call_frame_bindings(
                        break_hook,
                        user_function,
                        call_arguments,
                        this_binding,
                        arguments_binding,
                    )
                }),
                body: body
                    .iter()
                    .map(|statement| {
                        self.substitute_statement_call_frame_bindings(
                            statement,
                            user_function,
                            call_arguments,
                            this_binding,
                            arguments_binding,
                        )
                    })
                    .collect(),
            },
            _ => statement.clone(),
        }
    }
}
