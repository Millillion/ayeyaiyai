use super::*;

impl<'a> FunctionCompiler<'a> {
    pub(in crate::backend::direct_wasm) fn resolve_static_super_members_in_call_frame_return(
        &self,
        expression: &Expression,
        function_name: &str,
        this_binding: &Expression,
    ) -> Expression {
        match expression {
            Expression::Call { callee, arguments }
                if matches!(callee.as_ref(), Expression::SuperMember { .. }) =>
            {
                let Expression::SuperMember { property } = callee.as_ref() else {
                    unreachable!("guarded by matches above");
                };
                let property = self.resolve_static_super_members_in_call_frame_return(
                    property,
                    function_name,
                    this_binding,
                );
                let rewritten_arguments = arguments
                    .iter()
                    .map(|argument| match argument {
                        CallArgument::Expression(value) => CallArgument::Expression(
                            self.resolve_static_super_members_in_call_frame_return(
                                value,
                                function_name,
                                this_binding,
                            ),
                        ),
                        CallArgument::Spread(value) => CallArgument::Spread(
                            self.resolve_static_super_members_in_call_frame_return(
                                value,
                                function_name,
                                this_binding,
                            ),
                        ),
                    })
                    .collect::<Vec<_>>();
                let resolved_value = self
                    .resolve_super_base_expression_with_context(Some(function_name))
                    .and_then(|base| {
                        self.resolve_member_function_binding(&base, &property)
                            .or_else(|| {
                                self.resolve_object_binding_from_expression(&base)
                                    .and_then(|object_binding| {
                                        object_binding_lookup_value(&object_binding, &property)
                                            .cloned()
                                    })
                                    .and_then(|value| {
                                        self.resolve_function_binding_from_expression(&value)
                                    })
                            })
                    })
                    .and_then(|binding| {
                        match self
                            .resolve_static_function_outcome_from_binding_with_call_frame_and_context(
                                &binding,
                                &rewritten_arguments,
                                this_binding,
                                Some(function_name),
                            ) {
                            Some(StaticEvalOutcome::Value(value)) => Some(value),
                            _ => {
                                let expanded_arguments =
                                    self.expand_call_arguments(&rewritten_arguments);
                                self.resolve_function_binding_static_return_expression_with_call_frame(
                                    &binding,
                                    &expanded_arguments,
                                    this_binding,
                                )
                            }
                        }
                    });
                resolved_value.unwrap_or_else(|| Expression::Call {
                    callee: Box::new(Expression::SuperMember {
                        property: Box::new(property),
                    }),
                    arguments: rewritten_arguments,
                })
            }
            Expression::SuperMember { property } => {
                let property = self.resolve_static_super_members_in_call_frame_return(
                    property,
                    function_name,
                    this_binding,
                );
                self.resolve_static_super_member_value_with_context(
                    &property,
                    Some(function_name),
                    this_binding,
                )
                .unwrap_or_else(|| Expression::SuperMember {
                    property: Box::new(property),
                })
            }
            Expression::Member { object, property } => Expression::Member {
                object: Box::new(self.resolve_static_super_members_in_call_frame_return(
                    object,
                    function_name,
                    this_binding,
                )),
                property: Box::new(self.resolve_static_super_members_in_call_frame_return(
                    property,
                    function_name,
                    this_binding,
                )),
            },
            Expression::Assign { name, value } => Expression::Assign {
                name: name.clone(),
                value: Box::new(self.resolve_static_super_members_in_call_frame_return(
                    value,
                    function_name,
                    this_binding,
                )),
            },
            Expression::AssignMember {
                object,
                property,
                value,
            } => Expression::AssignMember {
                object: Box::new(self.resolve_static_super_members_in_call_frame_return(
                    object,
                    function_name,
                    this_binding,
                )),
                property: Box::new(self.resolve_static_super_members_in_call_frame_return(
                    property,
                    function_name,
                    this_binding,
                )),
                value: Box::new(self.resolve_static_super_members_in_call_frame_return(
                    value,
                    function_name,
                    this_binding,
                )),
            },
            Expression::AssignSuperMember { property, value } => Expression::AssignSuperMember {
                property: Box::new(self.resolve_static_super_members_in_call_frame_return(
                    property,
                    function_name,
                    this_binding,
                )),
                value: Box::new(self.resolve_static_super_members_in_call_frame_return(
                    value,
                    function_name,
                    this_binding,
                )),
            },
            Expression::Array(elements) => Expression::Array(
                elements
                    .iter()
                    .map(|element| match element {
                        ArrayElement::Expression(value) => ArrayElement::Expression(
                            self.resolve_static_super_members_in_call_frame_return(
                                value,
                                function_name,
                                this_binding,
                            ),
                        ),
                        ArrayElement::Spread(value) => ArrayElement::Spread(
                            self.resolve_static_super_members_in_call_frame_return(
                                value,
                                function_name,
                                this_binding,
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
                            key: self.resolve_static_super_members_in_call_frame_return(
                                key,
                                function_name,
                                this_binding,
                            ),
                            value: self.resolve_static_super_members_in_call_frame_return(
                                value,
                                function_name,
                                this_binding,
                            ),
                        },
                        ObjectEntry::Getter { key, getter } => ObjectEntry::Getter {
                            key: self.resolve_static_super_members_in_call_frame_return(
                                key,
                                function_name,
                                this_binding,
                            ),
                            getter: self.resolve_static_super_members_in_call_frame_return(
                                getter,
                                function_name,
                                this_binding,
                            ),
                        },
                        ObjectEntry::Setter { key, setter } => ObjectEntry::Setter {
                            key: self.resolve_static_super_members_in_call_frame_return(
                                key,
                                function_name,
                                this_binding,
                            ),
                            setter: self.resolve_static_super_members_in_call_frame_return(
                                setter,
                                function_name,
                                this_binding,
                            ),
                        },
                        ObjectEntry::Spread(value) => ObjectEntry::Spread(
                            self.resolve_static_super_members_in_call_frame_return(
                                value,
                                function_name,
                                this_binding,
                            ),
                        ),
                    })
                    .collect(),
            ),
            Expression::Unary { op, expression } => Expression::Unary {
                op: *op,
                expression: Box::new(self.resolve_static_super_members_in_call_frame_return(
                    expression,
                    function_name,
                    this_binding,
                )),
            },
            Expression::Binary { op, left, right } => Expression::Binary {
                op: *op,
                left: Box::new(self.resolve_static_super_members_in_call_frame_return(
                    left,
                    function_name,
                    this_binding,
                )),
                right: Box::new(self.resolve_static_super_members_in_call_frame_return(
                    right,
                    function_name,
                    this_binding,
                )),
            },
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => Expression::Conditional {
                condition: Box::new(self.resolve_static_super_members_in_call_frame_return(
                    condition,
                    function_name,
                    this_binding,
                )),
                then_expression: Box::new(self.resolve_static_super_members_in_call_frame_return(
                    then_expression,
                    function_name,
                    this_binding,
                )),
                else_expression: Box::new(self.resolve_static_super_members_in_call_frame_return(
                    else_expression,
                    function_name,
                    this_binding,
                )),
            },
            Expression::Sequence(expressions) => Expression::Sequence(
                expressions
                    .iter()
                    .map(|expression| {
                        self.resolve_static_super_members_in_call_frame_return(
                            expression,
                            function_name,
                            this_binding,
                        )
                    })
                    .collect(),
            ),
            Expression::Await(expression) => Expression::Await(Box::new(
                self.resolve_static_super_members_in_call_frame_return(
                    expression,
                    function_name,
                    this_binding,
                ),
            )),
            Expression::EnumerateKeys(expression) => Expression::EnumerateKeys(Box::new(
                self.resolve_static_super_members_in_call_frame_return(
                    expression,
                    function_name,
                    this_binding,
                ),
            )),
            Expression::GetIterator(expression) => Expression::GetIterator(Box::new(
                self.resolve_static_super_members_in_call_frame_return(
                    expression,
                    function_name,
                    this_binding,
                ),
            )),
            Expression::IteratorClose(expression) => Expression::IteratorClose(Box::new(
                self.resolve_static_super_members_in_call_frame_return(
                    expression,
                    function_name,
                    this_binding,
                ),
            )),
            Expression::Call { callee, arguments } => Expression::Call {
                callee: Box::new(self.resolve_static_super_members_in_call_frame_return(
                    callee,
                    function_name,
                    this_binding,
                )),
                arguments: arguments
                    .iter()
                    .map(|argument| match argument {
                        CallArgument::Expression(value) => CallArgument::Expression(
                            self.resolve_static_super_members_in_call_frame_return(
                                value,
                                function_name,
                                this_binding,
                            ),
                        ),
                        CallArgument::Spread(value) => CallArgument::Spread(
                            self.resolve_static_super_members_in_call_frame_return(
                                value,
                                function_name,
                                this_binding,
                            ),
                        ),
                    })
                    .collect(),
            },
            Expression::SuperCall { callee, arguments } => Expression::SuperCall {
                callee: Box::new(self.resolve_static_super_members_in_call_frame_return(
                    callee,
                    function_name,
                    this_binding,
                )),
                arguments: arguments
                    .iter()
                    .map(|argument| match argument {
                        CallArgument::Expression(value) => CallArgument::Expression(
                            self.resolve_static_super_members_in_call_frame_return(
                                value,
                                function_name,
                                this_binding,
                            ),
                        ),
                        CallArgument::Spread(value) => CallArgument::Spread(
                            self.resolve_static_super_members_in_call_frame_return(
                                value,
                                function_name,
                                this_binding,
                            ),
                        ),
                    })
                    .collect(),
            },
            Expression::New { callee, arguments } => Expression::New {
                callee: Box::new(self.resolve_static_super_members_in_call_frame_return(
                    callee,
                    function_name,
                    this_binding,
                )),
                arguments: arguments
                    .iter()
                    .map(|argument| match argument {
                        CallArgument::Expression(value) => CallArgument::Expression(
                            self.resolve_static_super_members_in_call_frame_return(
                                value,
                                function_name,
                                this_binding,
                            ),
                        ),
                        CallArgument::Spread(value) => CallArgument::Spread(
                            self.resolve_static_super_members_in_call_frame_return(
                                value,
                                function_name,
                                this_binding,
                            ),
                        ),
                    })
                    .collect(),
            },
            Expression::Update { .. }
            | Expression::Number(_)
            | Expression::BigInt(_)
            | Expression::String(_)
            | Expression::Bool(_)
            | Expression::Null
            | Expression::Undefined
            | Expression::NewTarget
            | Expression::Identifier(_)
            | Expression::This
            | Expression::Sent => expression.clone(),
        }
    }

    pub(in crate::backend::direct_wasm) fn resolve_function_binding_static_return_expression_with_call_frame(
        &self,
        binding: &LocalFunctionBinding,
        arguments: &[Expression],
        this_binding: &Expression,
    ) -> Option<Expression> {
        let LocalFunctionBinding::User(function_name) = binding else {
            return None;
        };
        let user_function = self.user_function(function_name)?;
        if self.user_function_mentions_direct_eval(user_function) {
            return None;
        }
        if self.user_function_deletes_call_frame_arguments_member(user_function) {
            return None;
        }
        if user_function
            .inline_summary
            .as_ref()
            .is_some_and(inline_summary_mentions_unsupported_explicit_call_frame_state)
        {
            return None;
        }
        let user_function_mentions_private_member_access =
            self.user_function_mentions_private_member_access(user_function);
        if user_function_mentions_private_member_access
            && self
                .resolve_object_binding_from_expression(this_binding)
                .is_none()
        {
            return None;
        }
        if self
            .backend
            .function_registry
            .analysis
            .user_function_capture_bindings
            .contains_key(&user_function.name)
            || self.user_function_references_captured_user_function(user_function)
        {
            return None;
        }
        if user_function.has_lowered_pattern_parameters()
            || !self
                .user_function_parameter_iterator_consumption_indices(user_function)
                .is_empty()
        {
            return None;
        }
        let call_arguments = arguments
            .iter()
            .cloned()
            .map(CallArgument::Expression)
            .collect::<Vec<_>>();
        let arguments_binding = Expression::Array(
            arguments
                .iter()
                .cloned()
                .map(crate::ir::hir::ArrayElement::Expression)
                .collect(),
        );
        if !user_function_mentions_private_member_access
            && let Some(summary) = user_function.inline_summary.as_ref()
            && self.user_function_has_explicit_call_frame_inlineable_terminal_body(user_function)
            && !user_function.has_parameter_defaults()
            && summary.effects.is_empty()
            && let Some(return_value) = summary.return_value.as_ref()
        {
            let result = self.substitute_user_function_call_frame_bindings(
                return_value,
                user_function,
                &call_arguments,
                this_binding,
                &arguments_binding,
            );
            return Some(self.resolve_static_super_members_in_call_frame_return(
                &result,
                function_name,
                this_binding,
            ));
        }

        if self
            .collect_user_function_assigned_nonlocal_bindings(user_function)
            .is_empty()
            && self
                .collect_user_function_call_effect_nonlocal_bindings(user_function)
                .is_empty()
            && let Some((result, updated_bindings)) = self
                .resolve_bound_snapshot_user_function_result_with_arguments_and_this(
                    function_name,
                    &HashMap::new(),
                    arguments,
                    this_binding,
                )
        {
            let materialized_this_binding = self.materialize_static_expression(this_binding);
            let this_binding_changed = updated_bindings.get("this").is_some_and(|updated_this| {
                let materialized_updated_this = self.materialize_static_expression(updated_this);
                !static_expression_matches(updated_this, this_binding)
                    && !static_expression_matches(
                        &materialized_updated_this,
                        &materialized_this_binding,
                    )
            });
            if !this_binding_changed {
                return Some(self.resolve_static_super_members_in_call_frame_return(
                    &result,
                    function_name,
                    this_binding,
                ));
            }
        }

        if user_function.has_parameter_defaults() {
            return None;
        }

        let function = self.resolve_registered_function_declaration(function_name)?;
        let (terminal_statement, effect_statements) = function.body.split_last()?;
        if !effect_statements
            .iter()
            .all(|statement| matches!(statement, Statement::Block { body } if body.is_empty()))
        {
            return None;
        }
        let Statement::Return(return_value) = terminal_statement else {
            return None;
        };
        let result = self.substitute_user_function_call_frame_bindings(
            return_value,
            user_function,
            &call_arguments,
            this_binding,
            &arguments_binding,
        );
        Some(self.resolve_static_super_members_in_call_frame_return(
            &result,
            function_name,
            this_binding,
        ))
    }

    pub(in crate::backend::direct_wasm) fn resolve_inline_call_from_returned_member(
        &self,
        object: &Expression,
        property: &Expression,
        arguments: &[CallArgument],
    ) -> Option<Expression> {
        let Expression::String(property_name) = property else {
            return None;
        };

        let (outer_callee, outer_arguments) = match object {
            Expression::Call { callee, arguments } | Expression::New { callee, arguments } => {
                (callee.as_ref(), arguments.as_slice())
            }
            _ => return None,
        };

        let Expression::Identifier(outer_name) = outer_callee else {
            return None;
        };
        let outer_user_function = self.resolve_user_function_from_callee_name(outer_name)?;
        let returned_value = outer_user_function
            .returned_member_value_bindings
            .iter()
            .find(|binding| binding.property == *property_name)?
            .value
            .clone();
        let substituted_value = self.substitute_user_function_argument_bindings(
            &returned_value,
            outer_user_function,
            outer_arguments,
        );
        let Expression::Identifier(inner_name) = substituted_value else {
            return None;
        };
        let inner_user_function = self.user_function(&inner_name)?;
        let summary = inner_user_function.inline_summary.as_ref()?;
        if !summary.effects.is_empty() {
            return None;
        }
        let return_value = summary.return_value.as_ref()?;
        let outer_substituted_return = self.substitute_user_function_argument_bindings(
            return_value,
            outer_user_function,
            outer_arguments,
        );

        Some(self.substitute_user_function_argument_bindings(
            &outer_substituted_return,
            inner_user_function,
            arguments,
        ))
    }
}
