use super::*;

impl<'a> FunctionCompiler<'a> {
    fn identifier_callee_name_is_inline_exempt(name: &str) -> bool {
        matches!(
            name,
            "__assert"
                | "__assertSameValue"
                | "__assertNotSameValue"
                | "__ayyAssertCompareArray"
                | "assert"
                | "TypeError"
        )
    }

    fn expression_contains_identifier_callee_call(expression: &Expression) -> bool {
        match expression {
            Expression::Call { callee, arguments }
            | Expression::SuperCall { callee, arguments }
            | Expression::New { callee, arguments } => {
                matches!(
                    callee.as_ref(),
                    Expression::Identifier(name)
                        if !Self::identifier_callee_name_is_inline_exempt(name)
                ) || Self::expression_contains_identifier_callee_call(callee)
                    || arguments.iter().any(|argument| match argument {
                        CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                            Self::expression_contains_identifier_callee_call(expression)
                        }
                    })
            }
            Expression::Member { object, property } => {
                Self::expression_contains_identifier_callee_call(object)
                    || Self::expression_contains_identifier_callee_call(property)
            }
            Expression::SuperMember { property } => {
                Self::expression_contains_identifier_callee_call(property)
            }
            Expression::Assign { value, .. }
            | Expression::Await(value)
            | Expression::EnumerateKeys(value)
            | Expression::GetIterator(value)
            | Expression::IteratorClose(value)
            | Expression::Unary {
                expression: value, ..
            } => Self::expression_contains_identifier_callee_call(value),
            Expression::AssignMember {
                object,
                property,
                value,
            } => {
                Self::expression_contains_identifier_callee_call(object)
                    || Self::expression_contains_identifier_callee_call(property)
                    || Self::expression_contains_identifier_callee_call(value)
            }
            Expression::AssignSuperMember { property, value } => {
                Self::expression_contains_identifier_callee_call(property)
                    || Self::expression_contains_identifier_callee_call(value)
            }
            Expression::Binary { left, right, .. } => {
                Self::expression_contains_identifier_callee_call(left)
                    || Self::expression_contains_identifier_callee_call(right)
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                Self::expression_contains_identifier_callee_call(condition)
                    || Self::expression_contains_identifier_callee_call(then_expression)
                    || Self::expression_contains_identifier_callee_call(else_expression)
            }
            Expression::Sequence(expressions) => expressions
                .iter()
                .any(Self::expression_contains_identifier_callee_call),
            Expression::Array(elements) => elements.iter().any(|element| match element {
                ArrayElement::Expression(expression) | ArrayElement::Spread(expression) => {
                    Self::expression_contains_identifier_callee_call(expression)
                }
            }),
            Expression::Object(entries) => entries.iter().any(|entry| match entry {
                ObjectEntry::Data { key, value } => {
                    Self::expression_contains_identifier_callee_call(key)
                        || Self::expression_contains_identifier_callee_call(value)
                }
                ObjectEntry::Getter { key, getter }
                | ObjectEntry::Setter {
                    key,
                    setter: getter,
                } => {
                    Self::expression_contains_identifier_callee_call(key)
                        || Self::expression_contains_identifier_callee_call(getter)
                }
                ObjectEntry::Spread(expression) => {
                    Self::expression_contains_identifier_callee_call(expression)
                }
            }),
            _ => false,
        }
    }

    fn statement_contains_identifier_callee_call(statement: &Statement) -> bool {
        match statement {
            Statement::Var { value, .. }
            | Statement::Let { value, .. }
            | Statement::Assign { value, .. }
            | Statement::Expression(value)
            | Statement::Throw(value)
            | Statement::Return(value) => Self::expression_contains_identifier_callee_call(value),
            Statement::AssignMember {
                object,
                property,
                value,
            } => {
                Self::expression_contains_identifier_callee_call(object)
                    || Self::expression_contains_identifier_callee_call(property)
                    || Self::expression_contains_identifier_callee_call(value)
            }
            Statement::Print { values } => values
                .iter()
                .any(Self::expression_contains_identifier_callee_call),
            Statement::If {
                condition,
                then_branch,
                else_branch,
            } => {
                Self::expression_contains_identifier_callee_call(condition)
                    || then_branch
                        .iter()
                        .any(Self::statement_contains_identifier_callee_call)
                    || else_branch
                        .iter()
                        .any(Self::statement_contains_identifier_callee_call)
            }
            Statement::Block { body } => body
                .iter()
                .any(Self::statement_contains_identifier_callee_call),
            Statement::With { object, body } => {
                Self::expression_contains_identifier_callee_call(object)
                    || body
                        .iter()
                        .any(Self::statement_contains_identifier_callee_call)
            }
            _ => false,
        }
    }

    pub(in crate::backend::direct_wasm) fn user_function_contains_identifier_callee_call(
        &self,
        user_function: &UserFunction,
    ) -> bool {
        self.resolve_registered_function_declaration(&user_function.name)
            .is_some_and(|function| {
                function
                    .body
                    .iter()
                    .any(Self::statement_contains_identifier_callee_call)
            })
    }

    fn identifier_callee_call_is_direct_async_safe(
        &self,
        callee_name: &str,
        arguments: &[CallArgument],
        current_function_name: Option<&str>,
    ) -> bool {
        let mut call_arguments = Vec::with_capacity(arguments.len());
        for argument in arguments {
            let CallArgument::Expression(expression) = argument else {
                return false;
            };
            call_arguments.push(expression.clone());
        }
        let callee = Expression::Identifier(callee_name.to_string());
        let Some(LocalFunctionBinding::User(function_name)) = self
            .resolve_function_binding_from_expression_with_context(&callee, current_function_name)
        else {
            return false;
        };
        let Some(user_function) = self.user_function(&function_name) else {
            return false;
        };
        let no_try = !self.current_function_contains_try_statement()
            && self.state.emission.control_flow.try_stack.is_empty();
        let arguments_inline_safe = call_arguments
            .iter()
            .all(|argument| self.inline_safe_argument_expression(argument));
        let arguments_no_shadowed_implicit_global = !call_arguments
            .iter()
            .any(|argument| self.inline_argument_mentions_shadowed_implicit_global(argument));
        let no_async_generator = !user_function.is_async() && !user_function.is_generator();
        let parameters_supported = !user_function.has_parameter_defaults()
            && !user_function.has_lowered_pattern_parameters()
            && user_function.extra_argument_indices.is_empty();
        let no_private = !self.user_function_mentions_private_member_access(user_function);
        let no_direct_eval = !self.user_function_mentions_direct_eval(user_function);
        let no_identifier_callee_calls =
            !self.user_function_contains_identifier_callee_call(user_function);
        let no_capture_bindings = !self
            .backend
            .function_registry
            .analysis
            .user_function_capture_bindings
            .contains_key(&user_function.name);
        let capture_bindings_supported = no_capture_bindings || call_arguments.is_empty();
        let no_captured_user_function_refs =
            !self.user_function_references_captured_user_function(user_function);
        let terminal_body =
            self.user_function_has_explicit_call_frame_inlineable_terminal_body(user_function);
        let safe = no_try
            && arguments_inline_safe
            && arguments_no_shadowed_implicit_global
            && no_async_generator
            && parameters_supported
            && no_private
            && no_direct_eval
            && no_identifier_callee_calls
            && capture_bindings_supported
            && no_captured_user_function_refs
            && terminal_body;
        safe
    }

    fn expression_identifier_callee_calls_are_direct_async_safe(
        &self,
        expression: &Expression,
        current_function_name: Option<&str>,
    ) -> bool {
        match expression {
            Expression::Call { callee, arguments } => {
                let direct_identifier_safe = match callee.as_ref() {
                    Expression::Identifier(name)
                        if Self::identifier_callee_name_is_inline_exempt(name) =>
                    {
                        true
                    }
                    Expression::Identifier(name) => self
                        .identifier_callee_call_is_direct_async_safe(
                            name,
                            arguments,
                            current_function_name,
                        ),
                    _ => true,
                };
                direct_identifier_safe
                    && self.expression_identifier_callee_calls_are_direct_async_safe(
                        callee,
                        current_function_name,
                    )
                    && arguments.iter().all(|argument| match argument {
                        CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                            self.expression_identifier_callee_calls_are_direct_async_safe(
                                expression,
                                current_function_name,
                            )
                        }
                    })
            }
            Expression::SuperCall { callee, arguments } | Expression::New { callee, arguments } => {
                let direct_identifier_safe = match callee.as_ref() {
                    Expression::Identifier(name)
                        if Self::identifier_callee_name_is_inline_exempt(name) =>
                    {
                        true
                    }
                    Expression::Identifier(_) => false,
                    _ => true,
                };
                direct_identifier_safe
                    && self.expression_identifier_callee_calls_are_direct_async_safe(
                        callee,
                        current_function_name,
                    )
                    && arguments.iter().all(|argument| match argument {
                        CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                            self.expression_identifier_callee_calls_are_direct_async_safe(
                                expression,
                                current_function_name,
                            )
                        }
                    })
            }
            Expression::Member { object, property } => {
                self.expression_identifier_callee_calls_are_direct_async_safe(
                    object,
                    current_function_name,
                ) && self.expression_identifier_callee_calls_are_direct_async_safe(
                    property,
                    current_function_name,
                )
            }
            Expression::SuperMember { property } => self
                .expression_identifier_callee_calls_are_direct_async_safe(
                    property,
                    current_function_name,
                ),
            Expression::Assign { value, .. }
            | Expression::Await(value)
            | Expression::EnumerateKeys(value)
            | Expression::GetIterator(value)
            | Expression::IteratorClose(value)
            | Expression::Unary {
                expression: value, ..
            } => self.expression_identifier_callee_calls_are_direct_async_safe(
                value,
                current_function_name,
            ),
            Expression::AssignMember {
                object,
                property,
                value,
            } => {
                self.expression_identifier_callee_calls_are_direct_async_safe(
                    object,
                    current_function_name,
                ) && self.expression_identifier_callee_calls_are_direct_async_safe(
                    property,
                    current_function_name,
                ) && self.expression_identifier_callee_calls_are_direct_async_safe(
                    value,
                    current_function_name,
                )
            }
            Expression::AssignSuperMember { property, value } => {
                self.expression_identifier_callee_calls_are_direct_async_safe(
                    property,
                    current_function_name,
                ) && self.expression_identifier_callee_calls_are_direct_async_safe(
                    value,
                    current_function_name,
                )
            }
            Expression::Binary { left, right, .. } => {
                self.expression_identifier_callee_calls_are_direct_async_safe(
                    left,
                    current_function_name,
                ) && self.expression_identifier_callee_calls_are_direct_async_safe(
                    right,
                    current_function_name,
                )
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                self.expression_identifier_callee_calls_are_direct_async_safe(
                    condition,
                    current_function_name,
                ) && self.expression_identifier_callee_calls_are_direct_async_safe(
                    then_expression,
                    current_function_name,
                ) && self.expression_identifier_callee_calls_are_direct_async_safe(
                    else_expression,
                    current_function_name,
                )
            }
            Expression::Sequence(expressions) => expressions.iter().all(|expression| {
                self.expression_identifier_callee_calls_are_direct_async_safe(
                    expression,
                    current_function_name,
                )
            }),
            Expression::Array(elements) => elements.iter().all(|element| match element {
                ArrayElement::Expression(expression) | ArrayElement::Spread(expression) => self
                    .expression_identifier_callee_calls_are_direct_async_safe(
                        expression,
                        current_function_name,
                    ),
            }),
            Expression::Object(entries) => entries.iter().all(|entry| match entry {
                ObjectEntry::Data { key, value } => {
                    self.expression_identifier_callee_calls_are_direct_async_safe(
                        key,
                        current_function_name,
                    ) && self.expression_identifier_callee_calls_are_direct_async_safe(
                        value,
                        current_function_name,
                    )
                }
                ObjectEntry::Getter { key, getter } => {
                    self.expression_identifier_callee_calls_are_direct_async_safe(
                        key,
                        current_function_name,
                    ) && self.expression_identifier_callee_calls_are_direct_async_safe(
                        getter,
                        current_function_name,
                    )
                }
                ObjectEntry::Setter { key, setter } => {
                    self.expression_identifier_callee_calls_are_direct_async_safe(
                        key,
                        current_function_name,
                    ) && self.expression_identifier_callee_calls_are_direct_async_safe(
                        setter,
                        current_function_name,
                    )
                }
                ObjectEntry::Spread(expression) => self
                    .expression_identifier_callee_calls_are_direct_async_safe(
                        expression,
                        current_function_name,
                    ),
            }),
            _ => true,
        }
    }

    fn statement_identifier_callee_calls_are_direct_async_safe(
        &self,
        statement: &Statement,
        current_function_name: Option<&str>,
    ) -> bool {
        match statement {
            Statement::Var { value, .. }
            | Statement::Let { value, .. }
            | Statement::Assign { value, .. }
            | Statement::Expression(value)
            | Statement::Throw(value)
            | Statement::Return(value) => self
                .expression_identifier_callee_calls_are_direct_async_safe(
                    value,
                    current_function_name,
                ),
            Statement::AssignMember {
                object,
                property,
                value,
            } => {
                self.expression_identifier_callee_calls_are_direct_async_safe(
                    object,
                    current_function_name,
                ) && self.expression_identifier_callee_calls_are_direct_async_safe(
                    property,
                    current_function_name,
                ) && self.expression_identifier_callee_calls_are_direct_async_safe(
                    value,
                    current_function_name,
                )
            }
            Statement::Print { values } => values.iter().all(|value| {
                self.expression_identifier_callee_calls_are_direct_async_safe(
                    value,
                    current_function_name,
                )
            }),
            Statement::If {
                condition,
                then_branch,
                else_branch,
            } => {
                self.expression_identifier_callee_calls_are_direct_async_safe(
                    condition,
                    current_function_name,
                ) && then_branch.iter().all(|statement| {
                    self.statement_identifier_callee_calls_are_direct_async_safe(
                        statement,
                        current_function_name,
                    )
                }) && else_branch.iter().all(|statement| {
                    self.statement_identifier_callee_calls_are_direct_async_safe(
                        statement,
                        current_function_name,
                    )
                })
            }
            Statement::Block { body } => body.iter().all(|statement| {
                self.statement_identifier_callee_calls_are_direct_async_safe(
                    statement,
                    current_function_name,
                )
            }),
            Statement::With { object, body } => {
                self.expression_identifier_callee_calls_are_direct_async_safe(
                    object,
                    current_function_name,
                ) && body.iter().all(|statement| {
                    self.statement_identifier_callee_calls_are_direct_async_safe(
                        statement,
                        current_function_name,
                    )
                })
            }
            _ => false,
        }
    }

    pub(in crate::backend::direct_wasm) fn user_function_identifier_callee_calls_are_direct_async_safe(
        &self,
        user_function: &UserFunction,
    ) -> bool {
        self.resolve_registered_function_declaration(&user_function.name)
            .is_some_and(|function| {
                function.body.iter().all(|statement| {
                    self.statement_identifier_callee_calls_are_direct_async_safe(
                        statement,
                        Some(user_function.name.as_str()),
                    )
                })
            })
    }

    fn expression_references_only_direct_async_safe_captured_user_function_calls(
        &self,
        expression: &Expression,
        captured_user_function_names: &HashSet<String>,
        current_function_name: Option<&str>,
    ) -> bool {
        match expression {
            Expression::Identifier(name) => !captured_user_function_names.contains(name),
            Expression::Call { callee, arguments } => {
                if let Expression::Identifier(name) = callee.as_ref()
                    && captured_user_function_names.contains(name)
                {
                    return self.identifier_callee_call_is_direct_async_safe(
                        name,
                        arguments,
                        current_function_name,
                    ) && arguments.iter().all(|argument| match argument {
                        CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                            self.expression_references_only_direct_async_safe_captured_user_function_calls(
                                expression,
                                captured_user_function_names,
                                current_function_name,
                            )
                        }
                    });
                }
                self.expression_references_only_direct_async_safe_captured_user_function_calls(
                    callee,
                    captured_user_function_names,
                    current_function_name,
                ) && arguments.iter().all(|argument| match argument {
                    CallArgument::Expression(expression) | CallArgument::Spread(expression) => self
                        .expression_references_only_direct_async_safe_captured_user_function_calls(
                            expression,
                            captured_user_function_names,
                            current_function_name,
                        ),
                })
            }
            Expression::SuperCall { callee, arguments } | Expression::New { callee, arguments } => {
                self.expression_references_only_direct_async_safe_captured_user_function_calls(
                    callee,
                    captured_user_function_names,
                    current_function_name,
                ) && arguments.iter().all(|argument| match argument {
                    CallArgument::Expression(expression) | CallArgument::Spread(expression) => self
                        .expression_references_only_direct_async_safe_captured_user_function_calls(
                            expression,
                            captured_user_function_names,
                            current_function_name,
                        ),
                })
            }
            Expression::Array(elements) => elements.iter().all(|element| match element {
                ArrayElement::Expression(expression) | ArrayElement::Spread(expression) => self
                    .expression_references_only_direct_async_safe_captured_user_function_calls(
                        expression,
                        captured_user_function_names,
                        current_function_name,
                    ),
            }),
            Expression::Object(entries) => entries.iter().all(|entry| match entry {
                ObjectEntry::Data { key, value } => {
                    self.expression_references_only_direct_async_safe_captured_user_function_calls(
                        key,
                        captured_user_function_names,
                        current_function_name,
                    ) && self
                        .expression_references_only_direct_async_safe_captured_user_function_calls(
                            value,
                            captured_user_function_names,
                            current_function_name,
                        )
                }
                ObjectEntry::Getter { key, getter } => {
                    self.expression_references_only_direct_async_safe_captured_user_function_calls(
                        key,
                        captured_user_function_names,
                        current_function_name,
                    ) && self
                        .expression_references_only_direct_async_safe_captured_user_function_calls(
                            getter,
                            captured_user_function_names,
                            current_function_name,
                        )
                }
                ObjectEntry::Setter { key, setter } => {
                    self.expression_references_only_direct_async_safe_captured_user_function_calls(
                        key,
                        captured_user_function_names,
                        current_function_name,
                    ) && self
                        .expression_references_only_direct_async_safe_captured_user_function_calls(
                            setter,
                            captured_user_function_names,
                            current_function_name,
                        )
                }
                ObjectEntry::Spread(expression) => self
                    .expression_references_only_direct_async_safe_captured_user_function_calls(
                        expression,
                        captured_user_function_names,
                        current_function_name,
                    ),
            }),
            Expression::Member { object, property } => {
                self.expression_references_only_direct_async_safe_captured_user_function_calls(
                    object,
                    captured_user_function_names,
                    current_function_name,
                ) && self.expression_references_only_direct_async_safe_captured_user_function_calls(
                    property,
                    captured_user_function_names,
                    current_function_name,
                )
            }
            Expression::SuperMember { property } => self
                .expression_references_only_direct_async_safe_captured_user_function_calls(
                    property,
                    captured_user_function_names,
                    current_function_name,
                ),
            Expression::Assign { value, .. }
            | Expression::Await(value)
            | Expression::EnumerateKeys(value)
            | Expression::GetIterator(value)
            | Expression::IteratorClose(value)
            | Expression::Unary {
                expression: value, ..
            } => self.expression_references_only_direct_async_safe_captured_user_function_calls(
                value,
                captured_user_function_names,
                current_function_name,
            ),
            Expression::AssignMember {
                object,
                property,
                value,
            } => {
                self.expression_references_only_direct_async_safe_captured_user_function_calls(
                    object,
                    captured_user_function_names,
                    current_function_name,
                ) && self.expression_references_only_direct_async_safe_captured_user_function_calls(
                    property,
                    captured_user_function_names,
                    current_function_name,
                ) && self.expression_references_only_direct_async_safe_captured_user_function_calls(
                    value,
                    captured_user_function_names,
                    current_function_name,
                )
            }
            Expression::AssignSuperMember { property, value } => {
                self.expression_references_only_direct_async_safe_captured_user_function_calls(
                    property,
                    captured_user_function_names,
                    current_function_name,
                ) && self.expression_references_only_direct_async_safe_captured_user_function_calls(
                    value,
                    captured_user_function_names,
                    current_function_name,
                )
            }
            Expression::Binary { left, right, .. } => {
                self.expression_references_only_direct_async_safe_captured_user_function_calls(
                    left,
                    captured_user_function_names,
                    current_function_name,
                ) && self.expression_references_only_direct_async_safe_captured_user_function_calls(
                    right,
                    captured_user_function_names,
                    current_function_name,
                )
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                self.expression_references_only_direct_async_safe_captured_user_function_calls(
                    condition,
                    captured_user_function_names,
                    current_function_name,
                ) && self.expression_references_only_direct_async_safe_captured_user_function_calls(
                    then_expression,
                    captured_user_function_names,
                    current_function_name,
                ) && self.expression_references_only_direct_async_safe_captured_user_function_calls(
                    else_expression,
                    captured_user_function_names,
                    current_function_name,
                )
            }
            Expression::Sequence(expressions) => expressions.iter().all(|expression| {
                self.expression_references_only_direct_async_safe_captured_user_function_calls(
                    expression,
                    captured_user_function_names,
                    current_function_name,
                )
            }),
            Expression::Number(_)
            | Expression::BigInt(_)
            | Expression::String(_)
            | Expression::Bool(_)
            | Expression::Null
            | Expression::Undefined
            | Expression::NewTarget
            | Expression::This
            | Expression::Sent
            | Expression::Update { .. } => true,
        }
    }

    fn statement_references_only_direct_async_safe_captured_user_function_calls(
        &self,
        statement: &Statement,
        captured_user_function_names: &HashSet<String>,
        current_function_name: Option<&str>,
    ) -> bool {
        match statement {
            Statement::Declaration { body }
            | Statement::Block { body }
            | Statement::Labeled { body, .. } => body.iter().all(|statement| {
                self.statement_references_only_direct_async_safe_captured_user_function_calls(
                    statement,
                    captured_user_function_names,
                    current_function_name,
                )
            }),
            Statement::Var { value, .. }
            | Statement::Let { value, .. }
            | Statement::Assign { value, .. }
            | Statement::Expression(value)
            | Statement::Throw(value)
            | Statement::Return(value)
            | Statement::Yield { value }
            | Statement::YieldDelegate { value } => self
                .expression_references_only_direct_async_safe_captured_user_function_calls(
                    value,
                    captured_user_function_names,
                    current_function_name,
                ),
            Statement::AssignMember {
                object,
                property,
                value,
            } => {
                self.expression_references_only_direct_async_safe_captured_user_function_calls(
                    object,
                    captured_user_function_names,
                    current_function_name,
                ) && self.expression_references_only_direct_async_safe_captured_user_function_calls(
                    property,
                    captured_user_function_names,
                    current_function_name,
                ) && self.expression_references_only_direct_async_safe_captured_user_function_calls(
                    value,
                    captured_user_function_names,
                    current_function_name,
                )
            }
            Statement::Print { values } => values.iter().all(|value| {
                self.expression_references_only_direct_async_safe_captured_user_function_calls(
                    value,
                    captured_user_function_names,
                    current_function_name,
                )
            }),
            Statement::With { object, body } => {
                self.expression_references_only_direct_async_safe_captured_user_function_calls(
                    object,
                    captured_user_function_names,
                    current_function_name,
                ) && body.iter().all(|statement| {
                    self.statement_references_only_direct_async_safe_captured_user_function_calls(
                        statement,
                        captured_user_function_names,
                        current_function_name,
                    )
                })
            }
            Statement::If {
                condition,
                then_branch,
                else_branch,
            } => {
                self.expression_references_only_direct_async_safe_captured_user_function_calls(
                    condition,
                    captured_user_function_names,
                    current_function_name,
                ) && then_branch.iter().all(|statement| {
                    self.statement_references_only_direct_async_safe_captured_user_function_calls(
                        statement,
                        captured_user_function_names,
                        current_function_name,
                    )
                }) && else_branch.iter().all(|statement| {
                    self.statement_references_only_direct_async_safe_captured_user_function_calls(
                        statement,
                        captured_user_function_names,
                        current_function_name,
                    )
                })
            }
            Statement::Try {
                body,
                catch_setup,
                catch_body,
                ..
            } => body
                .iter()
                .chain(catch_setup.iter())
                .chain(catch_body.iter())
                .all(|statement| {
                    self.statement_references_only_direct_async_safe_captured_user_function_calls(
                        statement,
                        captured_user_function_names,
                        current_function_name,
                    )
                }),
            Statement::Switch {
                discriminant,
                cases,
                ..
            } => self.expression_references_only_direct_async_safe_captured_user_function_calls(
                discriminant,
                captured_user_function_names,
                current_function_name,
            ) && cases.iter().all(|case| {
                case.test.as_ref().is_none_or(|test| {
                    self.expression_references_only_direct_async_safe_captured_user_function_calls(
                        test,
                        captured_user_function_names,
                        current_function_name,
                    )
                }) && case.body.iter().all(|statement| {
                    self.statement_references_only_direct_async_safe_captured_user_function_calls(
                        statement,
                        captured_user_function_names,
                        current_function_name,
                    )
                })
            }),
            Statement::For {
                init,
                condition,
                update,
                break_hook,
                body,
                ..
            } => {
                init.iter().all(|statement| {
                    self.statement_references_only_direct_async_safe_captured_user_function_calls(
                        statement,
                        captured_user_function_names,
                        current_function_name,
                    )
                }) && condition.as_ref().is_none_or(|condition| {
                    self.expression_references_only_direct_async_safe_captured_user_function_calls(
                        condition,
                        captured_user_function_names,
                        current_function_name,
                    )
                }) && update.as_ref().is_none_or(|update| {
                    self.expression_references_only_direct_async_safe_captured_user_function_calls(
                        update,
                        captured_user_function_names,
                        current_function_name,
                    )
                }) && break_hook.as_ref().is_none_or(|break_hook| {
                    self.expression_references_only_direct_async_safe_captured_user_function_calls(
                        break_hook,
                        captured_user_function_names,
                        current_function_name,
                    )
                }) && body.iter().all(|statement| {
                    self.statement_references_only_direct_async_safe_captured_user_function_calls(
                        statement,
                        captured_user_function_names,
                        current_function_name,
                    )
                })
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
                self.expression_references_only_direct_async_safe_captured_user_function_calls(
                    condition,
                    captured_user_function_names,
                    current_function_name,
                ) && break_hook.as_ref().is_none_or(|break_hook| {
                    self.expression_references_only_direct_async_safe_captured_user_function_calls(
                        break_hook,
                        captured_user_function_names,
                        current_function_name,
                    )
                }) && body.iter().all(|statement| {
                    self.statement_references_only_direct_async_safe_captured_user_function_calls(
                        statement,
                        captured_user_function_names,
                        current_function_name,
                    )
                })
            }
            Statement::Break { .. } | Statement::Continue { .. } => true,
        }
    }

    pub(in crate::backend::direct_wasm) fn user_function_references_only_direct_async_safe_captured_user_function_calls(
        &self,
        user_function: &UserFunction,
    ) -> bool {
        if self
            .backend
            .function_registry
            .analysis
            .user_function_capture_bindings
            .is_empty()
        {
            return true;
        }
        let captured_user_function_names = self
            .backend
            .function_registry
            .analysis
            .user_function_capture_bindings
            .keys()
            .cloned()
            .collect::<HashSet<_>>();
        self.resolve_registered_function_declaration(&user_function.name)
            .is_some_and(|function| {
                function.body.iter().all(|statement| {
                    self.statement_references_only_direct_async_safe_captured_user_function_calls(
                        statement,
                        &captured_user_function_names,
                        Some(user_function.name.as_str()),
                    )
                })
            })
    }

    fn statement_declares_local_binding(statement: &Statement) -> bool {
        match statement {
            Statement::Var { .. } | Statement::Let { .. } => true,
            Statement::Block { body } => body.iter().any(Self::statement_declares_local_binding),
            Statement::If {
                then_branch,
                else_branch,
                ..
            } => then_branch
                .iter()
                .chain(else_branch.iter())
                .any(Self::statement_declares_local_binding),
            _ => false,
        }
    }

    pub(in crate::backend::direct_wasm) fn user_function_contains_local_declaration(
        &self,
        user_function: &UserFunction,
    ) -> bool {
        self.resolve_registered_function_declaration(&user_function.name)
            .is_some_and(|function| {
                function
                    .body
                    .iter()
                    .any(Self::statement_declares_local_binding)
            })
    }
}
