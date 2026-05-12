use super::*;

impl<'a> FunctionCompiler<'a> {
    fn lowered_pattern_inline_argument_is_generator_call(&self, expression: &Expression) -> bool {
        let Expression::Call { callee, arguments } = expression else {
            return false;
        };
        if !arguments.is_empty() {
            return false;
        }
        let Expression::Identifier(name) = callee.as_ref() else {
            return false;
        };
        self.resolve_registered_function_declaration(name)
            .is_some_and(|function| function.kind.is_generator() && !function.kind.is_async())
    }

    fn lowered_pattern_inline_argument_is_safe(&self, expression: &Expression) -> bool {
        if self.inline_safe_argument_expression(expression) {
            return true;
        }
        if self.lowered_pattern_inline_argument_is_generator_call(expression) {
            return true;
        }
        if let Expression::Identifier(name) = expression
            && let Some(binding_name) = self.resolve_local_array_iterator_binding_name(name)
        {
            return self
                .state
                .speculation
                .static_semantics
                .local_array_iterator_binding(&binding_name)
                .is_some();
        }
        false
    }

    fn lowered_pattern_inline_captures_are_safe(&self, user_function: &UserFunction) -> bool {
        self.backend
            .function_registry
            .analysis
            .user_function_capture_bindings
            .get(&user_function.name)
            .is_none_or(|captures| captures.keys().all(|name| name == "assert"))
    }

    fn lowered_pattern_inline_statement_is_supported(statement: &Statement) -> bool {
        match statement {
            Statement::Var { .. }
            | Statement::Let { .. }
            | Statement::Assign { .. }
            | Statement::AssignMember { .. }
            | Statement::Expression(_)
            | Statement::Print { .. }
            | Statement::Throw(_) => true,
            Statement::Block { body }
            | Statement::Declaration { body }
            | Statement::Labeled { body, .. } => body
                .iter()
                .all(Self::lowered_pattern_inline_statement_is_supported),
            Statement::If {
                then_branch,
                else_branch,
                ..
            } => then_branch
                .iter()
                .chain(else_branch)
                .all(Self::lowered_pattern_inline_statement_is_supported),
            Statement::While {
                labels,
                body,
                break_hook,
                ..
            } => {
                labels.is_empty()
                    && break_hook.is_none()
                    && body
                        .iter()
                        .all(Self::lowered_pattern_inline_statement_is_supported)
            }
            Statement::Return(_)
            | Statement::Yield { .. }
            | Statement::YieldDelegate { .. }
            | Statement::Break { .. }
            | Statement::Continue { .. }
            | Statement::With { .. }
            | Statement::DoWhile { .. }
            | Statement::For { .. }
            | Statement::Try { .. }
            | Statement::Switch { .. } => false,
        }
    }

    fn lowered_pattern_inline_expression_reads_static_member_getter(
        &self,
        expression: &Expression,
    ) -> bool {
        match expression {
            Expression::Number(_)
            | Expression::BigInt(_)
            | Expression::String(_)
            | Expression::Bool(_)
            | Expression::Null
            | Expression::Undefined
            | Expression::NewTarget
            | Expression::Identifier(_)
            | Expression::This
            | Expression::Sent
            | Expression::Update { .. } => false,
            Expression::Array(elements) => elements.iter().any(|element| match element {
                ArrayElement::Expression(expression) | ArrayElement::Spread(expression) => {
                    self.lowered_pattern_inline_expression_reads_static_member_getter(expression)
                }
            }),
            Expression::Object(entries) => entries.iter().any(|entry| match entry {
                ObjectEntry::Data { key, value } => {
                    self.lowered_pattern_inline_expression_reads_static_member_getter(key)
                        || self.lowered_pattern_inline_expression_reads_static_member_getter(value)
                }
                ObjectEntry::Getter { key, getter } => {
                    self.lowered_pattern_inline_expression_reads_static_member_getter(key)
                        || self.lowered_pattern_inline_expression_reads_static_member_getter(getter)
                }
                ObjectEntry::Setter { key, setter } => {
                    self.lowered_pattern_inline_expression_reads_static_member_getter(key)
                        || self.lowered_pattern_inline_expression_reads_static_member_getter(setter)
                }
                ObjectEntry::Spread(expression) => {
                    self.lowered_pattern_inline_expression_reads_static_member_getter(expression)
                }
            }),
            Expression::Member { object, property } => {
                self.resolve_member_getter_binding(object, property)
                    .is_some()
                    || self.lowered_pattern_inline_expression_reads_static_member_getter(object)
                    || self.lowered_pattern_inline_expression_reads_static_member_getter(property)
            }
            Expression::SuperMember { property } => {
                self.lowered_pattern_inline_expression_reads_static_member_getter(property)
            }
            Expression::Assign { value, .. } | Expression::Await(value) => {
                self.lowered_pattern_inline_expression_reads_static_member_getter(value)
            }
            Expression::AssignMember {
                object,
                property,
                value,
            } => {
                self.resolve_member_getter_binding(object, property)
                    .is_some()
                    || self.lowered_pattern_inline_expression_reads_static_member_getter(object)
                    || self.lowered_pattern_inline_expression_reads_static_member_getter(property)
                    || self.lowered_pattern_inline_expression_reads_static_member_getter(value)
            }
            Expression::AssignSuperMember { property, value } => {
                self.lowered_pattern_inline_expression_reads_static_member_getter(property)
                    || self.lowered_pattern_inline_expression_reads_static_member_getter(value)
            }
            Expression::EnumerateKeys(value)
            | Expression::GetIterator(value)
            | Expression::IteratorClose(value)
            | Expression::Unary {
                expression: value, ..
            } => self.lowered_pattern_inline_expression_reads_static_member_getter(value),
            Expression::Binary { left, right, .. } => {
                self.lowered_pattern_inline_expression_reads_static_member_getter(left)
                    || self.lowered_pattern_inline_expression_reads_static_member_getter(right)
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                self.lowered_pattern_inline_expression_reads_static_member_getter(condition)
                    || self.lowered_pattern_inline_expression_reads_static_member_getter(
                        then_expression,
                    )
                    || self.lowered_pattern_inline_expression_reads_static_member_getter(
                        else_expression,
                    )
            }
            Expression::Sequence(expressions) => expressions.iter().any(|expression| {
                self.lowered_pattern_inline_expression_reads_static_member_getter(expression)
            }),
            Expression::Call { callee, arguments }
            | Expression::SuperCall { callee, arguments }
            | Expression::New { callee, arguments } => {
                self.lowered_pattern_inline_expression_reads_static_member_getter(callee)
                    || arguments.iter().any(|argument| {
                        self.lowered_pattern_inline_expression_reads_static_member_getter(
                            argument.expression(),
                        )
                    })
            }
        }
    }

    fn lowered_pattern_inline_statement_reads_static_member_getter(
        &self,
        statement: &Statement,
    ) -> bool {
        match statement {
            Statement::Declaration { body }
            | Statement::Block { body }
            | Statement::Labeled { body, .. } => body.iter().any(|statement| {
                self.lowered_pattern_inline_statement_reads_static_member_getter(statement)
            }),
            Statement::Var { value, .. }
            | Statement::Let { value, .. }
            | Statement::Assign { value, .. }
            | Statement::Expression(value)
            | Statement::Throw(value)
            | Statement::Return(value)
            | Statement::Yield { value }
            | Statement::YieldDelegate { value } => {
                self.lowered_pattern_inline_expression_reads_static_member_getter(value)
            }
            Statement::AssignMember {
                object,
                property,
                value,
            } => {
                self.resolve_member_getter_binding(object, property)
                    .is_some()
                    || self.lowered_pattern_inline_expression_reads_static_member_getter(object)
                    || self.lowered_pattern_inline_expression_reads_static_member_getter(property)
                    || self.lowered_pattern_inline_expression_reads_static_member_getter(value)
            }
            Statement::Print { values } => values.iter().any(|value| {
                self.lowered_pattern_inline_expression_reads_static_member_getter(value)
            }),
            Statement::Break { .. } | Statement::Continue { .. } => false,
            Statement::With { object, body } => {
                self.lowered_pattern_inline_expression_reads_static_member_getter(object)
                    || body.iter().any(|statement| {
                        self.lowered_pattern_inline_statement_reads_static_member_getter(statement)
                    })
            }
            Statement::If {
                condition,
                then_branch,
                else_branch,
            } => {
                self.lowered_pattern_inline_expression_reads_static_member_getter(condition)
                    || then_branch.iter().any(|statement| {
                        self.lowered_pattern_inline_statement_reads_static_member_getter(statement)
                    })
                    || else_branch.iter().any(|statement| {
                        self.lowered_pattern_inline_statement_reads_static_member_getter(statement)
                    })
            }
            Statement::Try {
                body,
                catch_setup,
                catch_body,
                ..
            } => body
                .iter()
                .chain(catch_setup)
                .chain(catch_body)
                .any(|statement| {
                    self.lowered_pattern_inline_statement_reads_static_member_getter(statement)
                }),
            Statement::Switch {
                discriminant,
                cases,
                ..
            } => {
                self.lowered_pattern_inline_expression_reads_static_member_getter(discriminant)
                    || cases.iter().any(|case| {
                        case.test.as_ref().is_some_and(|test| {
                            self.lowered_pattern_inline_expression_reads_static_member_getter(test)
                        }) || case.body.iter().any(|statement| {
                            self.lowered_pattern_inline_statement_reads_static_member_getter(
                                statement,
                            )
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
                    self.lowered_pattern_inline_statement_reads_static_member_getter(statement)
                }) || condition.as_ref().is_some_and(|condition| {
                    self.lowered_pattern_inline_expression_reads_static_member_getter(condition)
                }) || update.as_ref().is_some_and(|update| {
                    self.lowered_pattern_inline_expression_reads_static_member_getter(update)
                }) || break_hook.as_ref().is_some_and(|break_hook| {
                    self.lowered_pattern_inline_expression_reads_static_member_getter(break_hook)
                }) || body.iter().any(|statement| {
                    self.lowered_pattern_inline_statement_reads_static_member_getter(statement)
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
                self.lowered_pattern_inline_expression_reads_static_member_getter(condition)
                    || break_hook.as_ref().is_some_and(|break_hook| {
                        self.lowered_pattern_inline_expression_reads_static_member_getter(
                            break_hook,
                        )
                    })
                    || body.iter().any(|statement| {
                        self.lowered_pattern_inline_statement_reads_static_member_getter(statement)
                    })
            }
        }
    }

    pub(in crate::backend::direct_wasm) fn emit_inline_lowered_pattern_user_function_with_arguments(
        &mut self,
        user_function: &UserFunction,
        arguments: &[Expression],
        this_expression: &Expression,
    ) -> DirectResult<bool> {
        let trace_user_calls = std::env::var_os("AYY_TRACE_USER_CALLS").is_some();
        let consumes_parameter_iterator = !self
            .user_function_parameter_iterator_consumption_indices(user_function)
            .is_empty();
        if trace_user_calls {
            eprintln!(
                "lowered_pattern_inline:check target={} lowered={} consumes_iterator={} args={arguments:?}",
                user_function.name,
                user_function.has_lowered_pattern_parameters(),
                consumes_parameter_iterator
            );
        }
        if !(user_function.has_lowered_pattern_parameters() || consumes_parameter_iterator)
            || user_function.is_async()
            || user_function.is_generator()
            || user_function.has_parameter_defaults()
            || self.user_function_mentions_direct_eval(user_function)
            || self.user_function_contains_identifier_callee_call(user_function)
            || self.user_function_may_read_restricted_function_property(user_function)
            || !self.lowered_pattern_inline_captures_are_safe(user_function)
            || self.user_function_references_captured_user_function(user_function)
            || !user_function.extra_argument_indices.is_empty()
            || !self.inline_safe_argument_expression(this_expression)
            || !arguments
                .iter()
                .all(|argument| self.lowered_pattern_inline_argument_is_safe(argument))
            || self.inline_argument_mentions_shadowed_implicit_global(this_expression)
            || arguments
                .iter()
                .any(|argument| self.inline_argument_mentions_shadowed_implicit_global(argument))
        {
            if trace_user_calls {
                eprintln!(
                    "lowered_pattern_inline:reject target={} async={} generator={} defaults={} private={} eval={} identifier_callee={} restricted={} captures={} captured_ref={} extra_args={} this_safe={} args_safe={} this_shadow={} args_shadow={}",
                    user_function.name,
                    user_function.is_async(),
                    user_function.is_generator(),
                    user_function.has_parameter_defaults(),
                    self.user_function_mentions_private_member_access(user_function),
                    self.user_function_mentions_direct_eval(user_function),
                    self.user_function_contains_identifier_callee_call(user_function),
                    self.user_function_may_read_restricted_function_property(user_function),
                    !self.lowered_pattern_inline_captures_are_safe(user_function),
                    self.user_function_references_captured_user_function(user_function),
                    !user_function.extra_argument_indices.is_empty(),
                    self.inline_safe_argument_expression(this_expression),
                    arguments
                        .iter()
                        .all(|argument| self.lowered_pattern_inline_argument_is_safe(argument)),
                    self.inline_argument_mentions_shadowed_implicit_global(this_expression),
                    arguments
                        .iter()
                        .any(|argument| self
                            .inline_argument_mentions_shadowed_implicit_global(argument))
                );
            }
            return Ok(false);
        }
        let Some(function) = self
            .resolve_registered_function_declaration(&user_function.name)
            .cloned()
        else {
            return Ok(false);
        };
        if !function
            .body
            .iter()
            .all(Self::lowered_pattern_inline_statement_is_supported)
        {
            if trace_user_calls {
                eprintln!(
                    "lowered_pattern_inline:reject-body target={}",
                    user_function.name
                );
            }
            return Ok(false);
        }

        let mut bindings = HashMap::new();
        for (index, parameter) in function.params.iter().enumerate() {
            let value = if parameter.rest {
                Expression::Array(
                    arguments
                        .iter()
                        .skip(index)
                        .cloned()
                        .map(ArrayElement::Expression)
                        .collect(),
                )
            } else {
                arguments
                    .get(index)
                    .cloned()
                    .unwrap_or(Expression::Undefined)
            };
            bindings.insert(parameter.name.clone(), value);
        }
        let body = function
            .body
            .iter()
            .map(|statement| self.substitute_statement_bindings(statement, &bindings))
            .collect::<Vec<_>>();
        if body.iter().any(|statement| {
            self.lowered_pattern_inline_statement_reads_static_member_getter(statement)
        }) {
            if trace_user_calls {
                eprintln!(
                    "lowered_pattern_inline:reject-static-getter target={}",
                    user_function.name
                );
            }
            return Ok(false);
        }

        self.emit_numeric_expression(this_expression)?;
        self.state.emission.output.instructions.push(0x1a);

        self.with_user_function_execution_context(user_function, |compiler| {
            if trace_user_calls {
                eprintln!(
                    "lowered_pattern_inline:emit target={} statements={}",
                    user_function.name,
                    body.len()
                );
            }
            if compiler.emit_static_lowered_pattern_inline_body(&body)? {
                return Ok(true);
            }
            compiler.push_i32_const(JS_UNDEFINED_TAG);
            Ok(true)
        })
    }

    pub(in crate::backend::direct_wasm) fn emit_inline_user_function_summary_with_arguments(
        &mut self,
        user_function: &UserFunction,
        arguments: &[Expression],
    ) -> DirectResult<bool> {
        let call_arguments = arguments
            .iter()
            .cloned()
            .map(CallArgument::Expression)
            .collect::<Vec<_>>();

        if let Some(summary) = user_function.inline_summary.as_ref()
            && !self.user_function_contains_local_declaration(user_function)
            && !self
                .user_function_creates_descriptor_binding_with_arguments(user_function, arguments)
        {
            self.emit_inline_summary_with_call_arguments(user_function, summary, &call_arguments)?;
            return Ok(true);
        }

        let Some(function) = self
            .resolve_registered_function_declaration(&user_function.name)
            .cloned()
        else {
            return Ok(false);
        };
        let Some((terminal_statement, effect_statements)) = function.body.split_last() else {
            return Ok(false);
        };

        self.with_user_function_execution_context(user_function, |compiler| {
            for statement in effect_statements {
                if !compiler.emit_inline_user_function_effect_statement(
                    statement,
                    user_function,
                    &call_arguments,
                )? {
                    return Ok(false);
                }
            }
            compiler.emit_inline_user_function_terminal_statement(
                terminal_statement,
                user_function,
                &call_arguments,
            )
        })
    }
}
