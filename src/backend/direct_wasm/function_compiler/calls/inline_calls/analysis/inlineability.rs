use super::*;

impl<'a> FunctionCompiler<'a> {
    fn expression_may_read_restricted_function_property(&self, expression: &Expression) -> bool {
        match expression {
            Expression::Member { object, property } => {
                let property = self.materialize_static_expression(property);
                if matches!(
                    property,
                    Expression::String(ref property_name)
                        if property_name == "caller" || property_name == "arguments"
                ) {
                    return true;
                }
                self.expression_may_read_restricted_function_property(object)
                    || self.expression_may_read_restricted_function_property(&property)
            }
            Expression::SuperMember { property } => {
                self.expression_may_read_restricted_function_property(property)
            }
            Expression::Assign { value, .. }
            | Expression::Await(value)
            | Expression::EnumerateKeys(value)
            | Expression::GetIterator(value)
            | Expression::IteratorClose(value)
            | Expression::Unary {
                expression: value, ..
            } => self.expression_may_read_restricted_function_property(value),
            Expression::AssignMember {
                object,
                property,
                value,
            } => {
                self.expression_may_read_restricted_function_property(object)
                    || self.expression_may_read_restricted_function_property(property)
                    || self.expression_may_read_restricted_function_property(value)
            }
            Expression::AssignSuperMember { property, value } => {
                self.expression_may_read_restricted_function_property(property)
                    || self.expression_may_read_restricted_function_property(value)
            }
            Expression::Binary { left, right, .. } => {
                self.expression_may_read_restricted_function_property(left)
                    || self.expression_may_read_restricted_function_property(right)
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                self.expression_may_read_restricted_function_property(condition)
                    || self.expression_may_read_restricted_function_property(then_expression)
                    || self.expression_may_read_restricted_function_property(else_expression)
            }
            Expression::Sequence(expressions) => expressions.iter().any(|expression| {
                self.expression_may_read_restricted_function_property(expression)
            }),
            Expression::Call { callee, arguments }
            | Expression::SuperCall { callee, arguments }
            | Expression::New { callee, arguments } => {
                self.expression_may_read_restricted_function_property(callee)
                    || arguments.iter().any(|argument| {
                        self.expression_may_read_restricted_function_property(argument.expression())
                    })
            }
            Expression::Array(elements) => elements.iter().any(|element| match element {
                ArrayElement::Expression(expression) | ArrayElement::Spread(expression) => {
                    self.expression_may_read_restricted_function_property(expression)
                }
            }),
            Expression::Object(entries) => entries.iter().any(|entry| match entry {
                crate::ir::hir::ObjectEntry::Data { key, value } => {
                    self.expression_may_read_restricted_function_property(key)
                        || self.expression_may_read_restricted_function_property(value)
                }
                crate::ir::hir::ObjectEntry::Getter { key, getter } => {
                    self.expression_may_read_restricted_function_property(key)
                        || self.expression_may_read_restricted_function_property(getter)
                }
                crate::ir::hir::ObjectEntry::Setter { key, setter } => {
                    self.expression_may_read_restricted_function_property(key)
                        || self.expression_may_read_restricted_function_property(setter)
                }
                crate::ir::hir::ObjectEntry::Spread(expression) => {
                    self.expression_may_read_restricted_function_property(expression)
                }
            }),
            Expression::Number(_)
            | Expression::BigInt(_)
            | Expression::String(_)
            | Expression::Bool(_)
            | Expression::Null
            | Expression::Undefined
            | Expression::Identifier(_)
            | Expression::This
            | Expression::NewTarget
            | Expression::Sent
            | Expression::Update { .. } => false,
        }
    }

    fn statement_may_read_restricted_function_property(&self, statement: &Statement) -> bool {
        match statement {
            Statement::Declaration { body }
            | Statement::Block { body }
            | Statement::Labeled { body, .. }
            | Statement::With { body, .. } => body
                .iter()
                .any(|statement| self.statement_may_read_restricted_function_property(statement)),
            Statement::Var { value, .. }
            | Statement::Let { value, .. }
            | Statement::Assign { value, .. }
            | Statement::Expression(value)
            | Statement::Throw(value)
            | Statement::Return(value)
            | Statement::Yield { value }
            | Statement::YieldDelegate { value } => {
                self.expression_may_read_restricted_function_property(value)
            }
            Statement::AssignMember {
                object,
                property,
                value,
            } => {
                self.expression_may_read_restricted_function_property(object)
                    || self.expression_may_read_restricted_function_property(property)
                    || self.expression_may_read_restricted_function_property(value)
            }
            Statement::Print { values } => values
                .iter()
                .any(|value| self.expression_may_read_restricted_function_property(value)),
            Statement::If {
                condition,
                then_branch,
                else_branch,
            } => {
                self.expression_may_read_restricted_function_property(condition)
                    || then_branch.iter().any(|statement| {
                        self.statement_may_read_restricted_function_property(statement)
                    })
                    || else_branch.iter().any(|statement| {
                        self.statement_may_read_restricted_function_property(statement)
                    })
            }
            Statement::Try {
                body,
                catch_setup,
                catch_body,
                ..
            } => {
                body.iter().any(|statement| {
                    self.statement_may_read_restricted_function_property(statement)
                }) || catch_setup.iter().any(|statement| {
                    self.statement_may_read_restricted_function_property(statement)
                }) || catch_body.iter().any(|statement| {
                    self.statement_may_read_restricted_function_property(statement)
                })
            }
            Statement::Switch {
                discriminant,
                cases,
                ..
            } => {
                self.expression_may_read_restricted_function_property(discriminant)
                    || cases.iter().any(|case| {
                        case.test.as_ref().is_some_and(|test| {
                            self.expression_may_read_restricted_function_property(test)
                        }) || case.body.iter().any(|statement| {
                            self.statement_may_read_restricted_function_property(statement)
                        })
                    })
            }
            Statement::For {
                init,
                condition,
                update,
                break_hook,
                body,
                ..
            } => {
                init.iter().any(|statement| {
                    self.statement_may_read_restricted_function_property(statement)
                }) || condition.as_ref().is_some_and(|condition| {
                    self.expression_may_read_restricted_function_property(condition)
                }) || update.as_ref().is_some_and(|update| {
                    self.expression_may_read_restricted_function_property(update)
                }) || break_hook.as_ref().is_some_and(|break_hook| {
                    self.expression_may_read_restricted_function_property(break_hook)
                }) || body.iter().any(|statement| {
                    self.statement_may_read_restricted_function_property(statement)
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
                self.expression_may_read_restricted_function_property(condition)
                    || break_hook.as_ref().is_some_and(|break_hook| {
                        self.expression_may_read_restricted_function_property(break_hook)
                    })
                    || body.iter().any(|statement| {
                        self.statement_may_read_restricted_function_property(statement)
                    })
            }
            Statement::Break { .. } | Statement::Continue { .. } => false,
        }
    }

    pub(in crate::backend::direct_wasm) fn user_function_may_read_restricted_function_property(
        &self,
        user_function: &UserFunction,
    ) -> bool {
        self.resolve_registered_function_declaration(&user_function.name)
            .is_some_and(|function| {
                function.body.iter().any(|statement| {
                    self.statement_may_read_restricted_function_property(statement)
                })
            })
    }

    fn statement_mentions_call_frame_state(statement: &Statement) -> bool {
        match statement {
            Statement::Declaration { body }
            | Statement::Block { body }
            | Statement::Labeled { body, .. }
            | Statement::With { body, .. } => {
                body.iter().any(Self::statement_mentions_call_frame_state)
            }
            Statement::Var { value, .. }
            | Statement::Let { value, .. }
            | Statement::Assign { value, .. }
            | Statement::Expression(value)
            | Statement::Throw(value)
            | Statement::Return(value)
            | Statement::Yield { value }
            | Statement::YieldDelegate { value } => expression_mentions_call_frame_state(value),
            Statement::AssignMember {
                object,
                property,
                value,
            } => {
                expression_mentions_call_frame_state(object)
                    || expression_mentions_call_frame_state(property)
                    || expression_mentions_call_frame_state(value)
            }
            Statement::Print { values } => values.iter().any(expression_mentions_call_frame_state),
            Statement::If {
                condition,
                then_branch,
                else_branch,
            } => {
                expression_mentions_call_frame_state(condition)
                    || then_branch
                        .iter()
                        .any(Self::statement_mentions_call_frame_state)
                    || else_branch
                        .iter()
                        .any(Self::statement_mentions_call_frame_state)
            }
            Statement::Try {
                body,
                catch_setup,
                catch_body,
                ..
            } => {
                body.iter().any(Self::statement_mentions_call_frame_state)
                    || catch_setup
                        .iter()
                        .any(Self::statement_mentions_call_frame_state)
                    || catch_body
                        .iter()
                        .any(Self::statement_mentions_call_frame_state)
            }
            Statement::Switch {
                discriminant,
                cases,
                ..
            } => {
                expression_mentions_call_frame_state(discriminant)
                    || cases.iter().any(|case| {
                        case.test
                            .as_ref()
                            .is_some_and(expression_mentions_call_frame_state)
                            || case
                                .body
                                .iter()
                                .any(Self::statement_mentions_call_frame_state)
                    })
            }
            Statement::For {
                init,
                condition,
                update,
                break_hook,
                body,
                ..
            } => {
                init.iter().any(Self::statement_mentions_call_frame_state)
                    || condition
                        .as_ref()
                        .is_some_and(expression_mentions_call_frame_state)
                    || update
                        .as_ref()
                        .is_some_and(expression_mentions_call_frame_state)
                    || break_hook
                        .as_ref()
                        .is_some_and(expression_mentions_call_frame_state)
                    || body.iter().any(Self::statement_mentions_call_frame_state)
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
                expression_mentions_call_frame_state(condition)
                    || break_hook
                        .as_ref()
                        .is_some_and(expression_mentions_call_frame_state)
                    || body.iter().any(Self::statement_mentions_call_frame_state)
            }
            Statement::Break { .. } | Statement::Continue { .. } => false,
        }
    }

    pub(in crate::backend::direct_wasm) fn can_direct_call_use_explicit_frame_without_rebinding_lexical_state(
        &self,
        user_function: &UserFunction,
    ) -> bool {
        if !user_function.lexical_this {
            return true;
        }
        if user_function
            .inline_summary
            .as_ref()
            .is_some_and(inline_summary_mentions_call_frame_state)
        {
            return false;
        }
        self.resolve_registered_function_declaration(&user_function.name)
            .is_some_and(|function| {
                function
                    .body
                    .iter()
                    .all(|statement| !Self::statement_mentions_call_frame_state(statement))
            })
    }

    fn explicit_call_frame_inlineable_effect_statement(statement: &Statement) -> bool {
        match statement {
            Statement::Var { value, .. } | Statement::Let { value, .. } => {
                !expression_mentions_unsupported_explicit_call_frame_state(value)
            }
            Statement::Assign { value, .. } => {
                !expression_mentions_unsupported_explicit_call_frame_state(value)
            }
            Statement::Expression(Expression::Update { .. }) => true,
            Statement::Print { values } => values
                .iter()
                .all(|value| !expression_mentions_unsupported_explicit_call_frame_state(value)),
            Statement::Expression(expression) | Statement::Throw(expression) => {
                !expression_mentions_unsupported_explicit_call_frame_state(expression)
            }
            Statement::If {
                condition,
                then_branch,
                else_branch,
            } => {
                !expression_mentions_unsupported_explicit_call_frame_state(condition)
                    && then_branch
                        .iter()
                        .all(Self::explicit_call_frame_inlineable_effect_statement)
                    && else_branch
                        .iter()
                        .all(Self::explicit_call_frame_inlineable_effect_statement)
            }
            Statement::Block { body } => body
                .iter()
                .all(Self::explicit_call_frame_inlineable_effect_statement),
            _ => false,
        }
    }

    pub(in crate::backend::direct_wasm) fn user_function_has_inlineable_terminal_body(
        &self,
        user_function: &UserFunction,
    ) -> bool {
        let Some(function) = self.resolve_registered_function_declaration(&user_function.name)
        else {
            return false;
        };
        let Some((terminal_statement, effect_statements)) = function.body.split_last() else {
            return false;
        };
        for statement in effect_statements {
            match statement {
                Statement::Assign { value, .. } => {
                    if !user_function.lexical_this && expression_mentions_call_frame_state(value) {
                        return false;
                    }
                }
                Statement::Expression(Expression::Update { .. }) => {}
                Statement::Print { .. } => {}
                Statement::Expression(expression) => {
                    if !user_function.lexical_this
                        && expression_mentions_call_frame_state(expression)
                    {
                        return false;
                    }
                }
                Statement::Block { body } if body.is_empty() => {}
                _ => return false,
            }
        }
        match terminal_statement {
            Statement::Return(expression) | Statement::Throw(expression) => {
                user_function.lexical_this || !expression_mentions_call_frame_state(expression)
            }
            Statement::Assign { value, .. } => {
                user_function.lexical_this || !expression_mentions_call_frame_state(value)
            }
            Statement::Expression(Expression::Update { .. }) => true,
            Statement::Print { values } => values.iter().all(|value| {
                user_function.lexical_this || !expression_mentions_call_frame_state(value)
            }),
            Statement::Block { body } if body.is_empty() => true,
            Statement::Expression(expression) => {
                user_function.lexical_this || !expression_mentions_call_frame_state(expression)
            }
            _ => false,
        }
    }

    pub(in crate::backend::direct_wasm) fn user_function_has_explicit_call_frame_inlineable_terminal_body(
        &self,
        user_function: &UserFunction,
    ) -> bool {
        if !self
            .user_function_parameter_iterator_consumption_indices(user_function)
            .is_empty()
        {
            return false;
        }
        let Some(function) = self.resolve_registered_function_declaration(&user_function.name)
        else {
            return false;
        };
        let Some((terminal_statement, effect_statements)) = function.body.split_last() else {
            return false;
        };
        for statement in effect_statements {
            if !Self::explicit_call_frame_inlineable_effect_statement(statement) {
                return false;
            }
        }
        match terminal_statement {
            Statement::Return(expression) | Statement::Throw(expression) => {
                !expression_mentions_unsupported_explicit_call_frame_state(expression)
            }
            Statement::Var { value, .. } | Statement::Let { value, .. } => {
                !expression_mentions_unsupported_explicit_call_frame_state(value)
            }
            Statement::Assign { value, .. } => {
                !expression_mentions_unsupported_explicit_call_frame_state(value)
            }
            Statement::Expression(Expression::Update { .. }) => true,
            Statement::Print { values } => values
                .iter()
                .all(|value| !expression_mentions_unsupported_explicit_call_frame_state(value)),
            Statement::Block { body } if body.is_empty() => true,
            Statement::Expression(expression) => {
                !expression_mentions_unsupported_explicit_call_frame_state(expression)
            }
            _ => false,
        }
    }

    pub(in crate::backend::direct_wasm) fn inline_argument_mentions_shadowed_implicit_global(
        &self,
        expression: &Expression,
    ) -> bool {
        match expression {
            Expression::Identifier(name) => {
                self.resolve_current_local_binding(name).is_some()
                    && self.backend.global_has_implicit_binding(name)
            }
            Expression::Member { object, property } => {
                self.inline_argument_mentions_shadowed_implicit_global(object)
                    || self.inline_argument_mentions_shadowed_implicit_global(property)
            }
            Expression::SuperMember { property } => {
                self.inline_argument_mentions_shadowed_implicit_global(property)
            }
            Expression::Assign { value, .. } => {
                self.inline_argument_mentions_shadowed_implicit_global(value)
            }
            Expression::AssignMember {
                object,
                property,
                value,
            } => {
                self.inline_argument_mentions_shadowed_implicit_global(object)
                    || self.inline_argument_mentions_shadowed_implicit_global(property)
                    || self.inline_argument_mentions_shadowed_implicit_global(value)
            }
            Expression::AssignSuperMember { property, value } => {
                self.inline_argument_mentions_shadowed_implicit_global(property)
                    || self.inline_argument_mentions_shadowed_implicit_global(value)
            }
            Expression::Await(value)
            | Expression::EnumerateKeys(value)
            | Expression::GetIterator(value)
            | Expression::IteratorClose(value)
            | Expression::Unary {
                expression: value, ..
            } => self.inline_argument_mentions_shadowed_implicit_global(value),
            Expression::Binary { left, right, .. } => {
                self.inline_argument_mentions_shadowed_implicit_global(left)
                    || self.inline_argument_mentions_shadowed_implicit_global(right)
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                self.inline_argument_mentions_shadowed_implicit_global(condition)
                    || self.inline_argument_mentions_shadowed_implicit_global(then_expression)
                    || self.inline_argument_mentions_shadowed_implicit_global(else_expression)
            }
            Expression::Sequence(expressions) => expressions.iter().any(|expression| {
                self.inline_argument_mentions_shadowed_implicit_global(expression)
            }),
            Expression::Array(elements) => elements.iter().any(|element| match element {
                ArrayElement::Expression(expression) | ArrayElement::Spread(expression) => {
                    self.inline_argument_mentions_shadowed_implicit_global(expression)
                }
            }),
            Expression::Call { callee, arguments }
            | Expression::SuperCall { callee, arguments } => {
                self.inline_argument_mentions_shadowed_implicit_global(callee)
                    || arguments.iter().any(|argument| match argument {
                        CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                            self.inline_argument_mentions_shadowed_implicit_global(expression)
                        }
                    })
            }
            Expression::New { callee, arguments } => {
                self.inline_argument_mentions_shadowed_implicit_global(callee)
                    || arguments.iter().any(|argument| match argument {
                        CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                            self.inline_argument_mentions_shadowed_implicit_global(expression)
                        }
                    })
            }
            Expression::Object(entries) => entries.iter().any(|entry| match entry {
                crate::ir::hir::ObjectEntry::Data { key, value } => {
                    self.inline_argument_mentions_shadowed_implicit_global(key)
                        || self.inline_argument_mentions_shadowed_implicit_global(value)
                }
                crate::ir::hir::ObjectEntry::Getter { key, getter } => {
                    self.inline_argument_mentions_shadowed_implicit_global(key)
                        || self.inline_argument_mentions_shadowed_implicit_global(getter)
                }
                crate::ir::hir::ObjectEntry::Setter { key, setter } => {
                    self.inline_argument_mentions_shadowed_implicit_global(key)
                        || self.inline_argument_mentions_shadowed_implicit_global(setter)
                }
                crate::ir::hir::ObjectEntry::Spread(expression) => {
                    self.inline_argument_mentions_shadowed_implicit_global(expression)
                }
            }),
            Expression::NewTarget
            | Expression::This
            | Expression::Number(_)
            | Expression::BigInt(_)
            | Expression::String(_)
            | Expression::Bool(_)
            | Expression::Null
            | Expression::Undefined
            | Expression::Sent
            | Expression::Update { .. } => false,
        }
    }

    pub(in crate::backend::direct_wasm) fn user_function_references_captured_user_function(
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
            return false;
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
                function.body.iter().any(|statement| {
                    statement_references_user_function(statement, &captured_user_function_names)
                })
            })
    }
}
