use super::*;

impl<'a> FunctionCompiler<'a> {
    pub(in crate::backend::direct_wasm) fn resolve_static_getter_value_from_binding_with_context(
        &self,
        binding: &LocalFunctionBinding,
        this_binding: &Expression,
        current_function_name: Option<&str>,
    ) -> Option<Expression> {
        let getter_function_name = match binding {
            LocalFunctionBinding::User(function_name) => Some(function_name.as_str()),
            LocalFunctionBinding::Builtin(_) => None,
        };
        let getter_context = getter_function_name.or(current_function_name);
        if let LocalFunctionBinding::User(function_name) = binding
            && self
                .user_function(function_name)
                .is_some_and(|user_function| {
                    self.user_function_mentions_private_member_access(user_function)
                })
        {
            return None;
        }

        let adjusted_this_binding;
        let effective_this_binding = if let LocalFunctionBinding::User(function_name) = binding
            && let Some(user_function) = self.user_function(function_name)
            && let Some(boxed_this) =
                self.static_sloppy_function_this_binding(user_function, this_binding)
        {
            adjusted_this_binding = boxed_this;
            &adjusted_this_binding
        } else {
            this_binding
        };

        let value = self
            .resolve_function_binding_static_return_expression_with_call_frame(
                binding,
                &[],
                effective_this_binding,
            )
            .or_else(|| {
                match self.resolve_static_function_outcome_from_binding_with_call_frame_and_context(
                    binding,
                    &[],
                    effective_this_binding,
                    getter_context,
                ) {
                    Some(StaticEvalOutcome::Value(value)) => Some(value),
                    _ => None,
                }
            })?;
        Some(self.resolve_static_getter_super_members_with_context(
            &value,
            effective_this_binding,
            getter_context,
        ))
    }

    pub(in crate::backend::direct_wasm) fn resolve_static_member_getter_value_with_context(
        &self,
        object: &Expression,
        property: &Expression,
        current_function_name: Option<&str>,
    ) -> Option<Expression> {
        let getter_binding = self.resolve_member_getter_binding(object, property)?;
        self.resolve_static_getter_value_from_binding_with_context(
            &getter_binding,
            object,
            current_function_name,
        )
    }

    fn resolve_static_getter_super_members_with_context(
        &self,
        value: &Expression,
        this_binding: &Expression,
        current_function_name: Option<&str>,
    ) -> Expression {
        match value {
            Expression::SuperMember { property } => self
                .resolve_static_super_member_value_with_context(
                    property,
                    current_function_name,
                    this_binding,
                )
                .unwrap_or_else(|| Expression::SuperMember {
                    property: Box::new(self.resolve_static_getter_super_members_with_context(
                        property,
                        this_binding,
                        current_function_name,
                    )),
                }),
            Expression::Member { object, property } => Expression::Member {
                object: Box::new(self.resolve_static_getter_super_members_with_context(
                    object,
                    this_binding,
                    current_function_name,
                )),
                property: Box::new(self.resolve_static_getter_super_members_with_context(
                    property,
                    this_binding,
                    current_function_name,
                )),
            },
            Expression::Assign { name, value } => Expression::Assign {
                name: name.clone(),
                value: Box::new(self.resolve_static_getter_super_members_with_context(
                    value,
                    this_binding,
                    current_function_name,
                )),
            },
            Expression::AssignMember {
                object,
                property,
                value,
            } => Expression::AssignMember {
                object: Box::new(self.resolve_static_getter_super_members_with_context(
                    object,
                    this_binding,
                    current_function_name,
                )),
                property: Box::new(self.resolve_static_getter_super_members_with_context(
                    property,
                    this_binding,
                    current_function_name,
                )),
                value: Box::new(self.resolve_static_getter_super_members_with_context(
                    value,
                    this_binding,
                    current_function_name,
                )),
            },
            Expression::AssignSuperMember { property, value } => Expression::AssignSuperMember {
                property: Box::new(self.resolve_static_getter_super_members_with_context(
                    property,
                    this_binding,
                    current_function_name,
                )),
                value: Box::new(self.resolve_static_getter_super_members_with_context(
                    value,
                    this_binding,
                    current_function_name,
                )),
            },
            Expression::Await(value) => Expression::Await(Box::new(
                self.resolve_static_getter_super_members_with_context(
                    value,
                    this_binding,
                    current_function_name,
                ),
            )),
            Expression::EnumerateKeys(value) => Expression::EnumerateKeys(Box::new(
                self.resolve_static_getter_super_members_with_context(
                    value,
                    this_binding,
                    current_function_name,
                ),
            )),
            Expression::GetIterator(value) => Expression::GetIterator(Box::new(
                self.resolve_static_getter_super_members_with_context(
                    value,
                    this_binding,
                    current_function_name,
                ),
            )),
            Expression::IteratorClose(value) => Expression::IteratorClose(Box::new(
                self.resolve_static_getter_super_members_with_context(
                    value,
                    this_binding,
                    current_function_name,
                ),
            )),
            Expression::Unary { op, expression } => Expression::Unary {
                op: *op,
                expression: Box::new(self.resolve_static_getter_super_members_with_context(
                    expression,
                    this_binding,
                    current_function_name,
                )),
            },
            Expression::Binary { op, left, right } => Expression::Binary {
                op: *op,
                left: Box::new(self.resolve_static_getter_super_members_with_context(
                    left,
                    this_binding,
                    current_function_name,
                )),
                right: Box::new(self.resolve_static_getter_super_members_with_context(
                    right,
                    this_binding,
                    current_function_name,
                )),
            },
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => Expression::Conditional {
                condition: Box::new(self.resolve_static_getter_super_members_with_context(
                    condition,
                    this_binding,
                    current_function_name,
                )),
                then_expression: Box::new(self.resolve_static_getter_super_members_with_context(
                    then_expression,
                    this_binding,
                    current_function_name,
                )),
                else_expression: Box::new(self.resolve_static_getter_super_members_with_context(
                    else_expression,
                    this_binding,
                    current_function_name,
                )),
            },
            Expression::Sequence(expressions) => Expression::Sequence(
                expressions
                    .iter()
                    .map(|expression| {
                        self.resolve_static_getter_super_members_with_context(
                            expression,
                            this_binding,
                            current_function_name,
                        )
                    })
                    .collect(),
            ),
            Expression::Call { callee, arguments } => Expression::Call {
                callee: Box::new(self.resolve_static_getter_super_members_with_context(
                    callee,
                    this_binding,
                    current_function_name,
                )),
                arguments: arguments
                    .iter()
                    .map(|argument| match argument {
                        CallArgument::Expression(expression) => CallArgument::Expression(
                            self.resolve_static_getter_super_members_with_context(
                                expression,
                                this_binding,
                                current_function_name,
                            ),
                        ),
                        CallArgument::Spread(expression) => CallArgument::Spread(
                            self.resolve_static_getter_super_members_with_context(
                                expression,
                                this_binding,
                                current_function_name,
                            ),
                        ),
                    })
                    .collect(),
            },
            Expression::SuperCall { callee, arguments } => Expression::SuperCall {
                callee: Box::new(self.resolve_static_getter_super_members_with_context(
                    callee,
                    this_binding,
                    current_function_name,
                )),
                arguments: arguments
                    .iter()
                    .map(|argument| match argument {
                        CallArgument::Expression(expression) => CallArgument::Expression(
                            self.resolve_static_getter_super_members_with_context(
                                expression,
                                this_binding,
                                current_function_name,
                            ),
                        ),
                        CallArgument::Spread(expression) => CallArgument::Spread(
                            self.resolve_static_getter_super_members_with_context(
                                expression,
                                this_binding,
                                current_function_name,
                            ),
                        ),
                    })
                    .collect(),
            },
            Expression::New { callee, arguments } => Expression::New {
                callee: Box::new(self.resolve_static_getter_super_members_with_context(
                    callee,
                    this_binding,
                    current_function_name,
                )),
                arguments: arguments
                    .iter()
                    .map(|argument| match argument {
                        CallArgument::Expression(expression) => CallArgument::Expression(
                            self.resolve_static_getter_super_members_with_context(
                                expression,
                                this_binding,
                                current_function_name,
                            ),
                        ),
                        CallArgument::Spread(expression) => CallArgument::Spread(
                            self.resolve_static_getter_super_members_with_context(
                                expression,
                                this_binding,
                                current_function_name,
                            ),
                        ),
                    })
                    .collect(),
            },
            Expression::Array(elements) => Expression::Array(
                elements
                    .iter()
                    .map(|element| match element {
                        ArrayElement::Expression(expression) => ArrayElement::Expression(
                            self.resolve_static_getter_super_members_with_context(
                                expression,
                                this_binding,
                                current_function_name,
                            ),
                        ),
                        ArrayElement::Spread(expression) => ArrayElement::Spread(
                            self.resolve_static_getter_super_members_with_context(
                                expression,
                                this_binding,
                                current_function_name,
                            ),
                        ),
                    })
                    .collect(),
            ),
            Expression::Object(entries) => Expression::Object(
                entries
                    .iter()
                    .map(|entry| match entry {
                        ObjectEntry::Data { key, value } => ObjectEntry::Data {
                            key: self.resolve_static_getter_super_members_with_context(
                                key,
                                this_binding,
                                current_function_name,
                            ),
                            value: self.resolve_static_getter_super_members_with_context(
                                value,
                                this_binding,
                                current_function_name,
                            ),
                        },
                        ObjectEntry::Getter { key, getter } => ObjectEntry::Getter {
                            key: self.resolve_static_getter_super_members_with_context(
                                key,
                                this_binding,
                                current_function_name,
                            ),
                            getter: self.resolve_static_getter_super_members_with_context(
                                getter,
                                this_binding,
                                current_function_name,
                            ),
                        },
                        ObjectEntry::Setter { key, setter } => ObjectEntry::Setter {
                            key: self.resolve_static_getter_super_members_with_context(
                                key,
                                this_binding,
                                current_function_name,
                            ),
                            setter: self.resolve_static_getter_super_members_with_context(
                                setter,
                                this_binding,
                                current_function_name,
                            ),
                        },
                        ObjectEntry::Spread(expression) => ObjectEntry::Spread(
                            self.resolve_static_getter_super_members_with_context(
                                expression,
                                this_binding,
                                current_function_name,
                            ),
                        ),
                    })
                    .collect(),
            ),
            _ => value.clone(),
        }
    }
}
