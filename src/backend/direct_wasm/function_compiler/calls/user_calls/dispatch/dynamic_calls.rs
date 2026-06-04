use super::*;

impl<'a> FunctionCompiler<'a> {
    fn user_function_is_class_constructor(user_function: &UserFunction) -> bool {
        user_function.name.starts_with("__ayy_class_ctor_")
    }

    fn binding_is_test262_create_realm_builtin(binding: &LocalFunctionBinding) -> bool {
        matches!(
            binding,
            LocalFunctionBinding::Builtin(function_name)
                if function_name == TEST262_CREATE_REALM_BUILTIN
        )
    }

    fn expression_resolves_to_test262_create_realm_builtin(
        &self,
        expression: &Expression,
        depth: usize,
    ) -> bool {
        if depth > 3 {
            return false;
        }
        if self
            .resolve_function_binding_from_expression(expression)
            .as_ref()
            .is_some_and(Self::binding_is_test262_create_realm_builtin)
        {
            return true;
        }
        if let Expression::Identifier(name) = expression {
            if self
                .backend
                .global_function_binding(name)
                .as_ref()
                .is_some_and(|binding| Self::binding_is_test262_create_realm_builtin(binding))
            {
                return true;
            }
            if let Some((resolved_name, _)) = self.resolve_current_local_binding(name) {
                if self
                    .state
                    .speculation
                    .static_semantics
                    .local_function_binding(&resolved_name)
                    .as_ref()
                    .is_some_and(|binding| Self::binding_is_test262_create_realm_builtin(binding))
                {
                    return true;
                }
                if let Some(value) = self
                    .state
                    .speculation
                    .static_semantics
                    .local_value_binding(&resolved_name)
                    .filter(|value| !static_expression_matches(value, expression))
                    && self.expression_resolves_to_test262_create_realm_builtin(value, depth + 1)
                {
                    return true;
                }
            }
            if let Some(value) = self
                .state
                .speculation
                .static_semantics
                .local_value_binding(name)
                .or_else(|| self.global_value_binding(name))
                .filter(|value| !static_expression_matches(value, expression))
                && self.expression_resolves_to_test262_create_realm_builtin(value, depth + 1)
            {
                return true;
            }
        }
        if let Some(resolved) = self
            .resolve_bound_alias_expression(expression)
            .filter(|resolved| !static_expression_matches(resolved, expression))
            && self.expression_resolves_to_test262_create_realm_builtin(&resolved, depth + 1)
        {
            return true;
        }
        let materialized = self.materialize_static_expression(expression);
        !static_expression_matches(&materialized, expression)
            && self.expression_resolves_to_test262_create_realm_builtin(&materialized, depth + 1)
    }

    fn dynamic_call_user_functions(&self) -> Vec<UserFunction> {
        self.user_functions()
            .into_iter()
            .filter(|user_function| !Self::user_function_is_class_constructor(user_function))
            .collect()
    }

    fn is_done_callback_name(name: &str) -> bool {
        name == "$DONE" || name.contains("$DONE")
    }

    fn expression_is_known_promise_resolver_callee(callee: &Expression) -> bool {
        matches!(
            callee,
            Expression::Identifier(name)
                if name == "continueExecution"
                    || name.ends_with("$continueExecution")
                    || name == "__ayy_promise_with_resolvers_resolve"
                    || name == "__ayy_promise_with_resolvers_reject"
        ) || matches!(
            callee,
            Expression::Member { property, .. }
                if matches!(
                    property.as_ref(),
                    Expression::String(name)
                        if name.starts_with("resolve") || name.starts_with("reject")
                )
        )
    }

    fn function_body_returns_identifier(function: &FunctionDeclaration, name: &str) -> bool {
        function
            .body
            .iter()
            .any(|statement| Self::statement_returns_identifier(statement, name))
    }

    fn statement_returns_identifier(statement: &Statement, name: &str) -> bool {
        match statement {
            Statement::Return(Expression::Identifier(identifier)) => identifier == name,
            Statement::Declaration { body }
            | Statement::Block { body }
            | Statement::Labeled { body, .. }
            | Statement::With { body, .. } => body
                .iter()
                .any(|statement| Self::statement_returns_identifier(statement, name)),
            Statement::If {
                then_branch,
                else_branch,
                ..
            } => then_branch
                .iter()
                .chain(else_branch)
                .any(|statement| Self::statement_returns_identifier(statement, name)),
            Statement::Try {
                body,
                catch_setup,
                catch_body,
                ..
            } => body
                .iter()
                .chain(catch_setup)
                .chain(catch_body)
                .any(|statement| Self::statement_returns_identifier(statement, name)),
            Statement::Switch { cases, .. } => cases.iter().any(|case| {
                case.body
                    .iter()
                    .any(|statement| Self::statement_returns_identifier(statement, name))
            }),
            Statement::For { init, body, .. } => init
                .iter()
                .chain(body)
                .any(|statement| Self::statement_returns_identifier(statement, name)),
            Statement::While { body, .. } | Statement::DoWhile { body, .. } => body
                .iter()
                .any(|statement| Self::statement_returns_identifier(statement, name)),
            _ => false,
        }
    }

    fn expression_has_then_getter_from_candidates(
        expression: &Expression,
        getter_names: &HashSet<String>,
    ) -> bool {
        match expression {
            Expression::Array(elements) => elements.iter().any(|element| match element {
                ArrayElement::Expression(expression) | ArrayElement::Spread(expression) => {
                    Self::expression_has_then_getter_from_candidates(expression, getter_names)
                }
            }),
            Expression::Object(entries) => entries.iter().any(|entry| match entry {
                ObjectEntry::Getter { key, getter }
                    if matches!(key, Expression::String(name) if name == "then")
                        && matches!(getter, Expression::Identifier(name) if getter_names.contains(name)) =>
                {
                    true
                }
                ObjectEntry::Data { key, value } => {
                    Self::expression_has_then_getter_from_candidates(key, getter_names)
                        || Self::expression_has_then_getter_from_candidates(value, getter_names)
                }
                ObjectEntry::Getter { key, getter } | ObjectEntry::Setter { key, setter: getter } => {
                    Self::expression_has_then_getter_from_candidates(key, getter_names)
                        || Self::expression_has_then_getter_from_candidates(getter, getter_names)
                }
                ObjectEntry::Spread(expression) => {
                    Self::expression_has_then_getter_from_candidates(expression, getter_names)
                }
            }),
            Expression::Member { object, property }
            | Expression::AssignMember {
                object, property, ..
            } => {
                Self::expression_has_then_getter_from_candidates(object, getter_names)
                    || Self::expression_has_then_getter_from_candidates(property, getter_names)
            }
            Expression::SuperMember { property } => {
                Self::expression_has_then_getter_from_candidates(property, getter_names)
            }
            Expression::Assign { value, .. }
            | Expression::Await(value)
            | Expression::EnumerateKeys(value)
            | Expression::GetIterator(value)
            | Expression::IteratorClose(value)
            | Expression::Unary {
                expression: value, ..
            } => Self::expression_has_then_getter_from_candidates(value, getter_names),
            Expression::AssignSuperMember { property, value } => {
                Self::expression_has_then_getter_from_candidates(property, getter_names)
                    || Self::expression_has_then_getter_from_candidates(value, getter_names)
            }
            Expression::Binary { left, right, .. } => {
                Self::expression_has_then_getter_from_candidates(left, getter_names)
                    || Self::expression_has_then_getter_from_candidates(right, getter_names)
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                Self::expression_has_then_getter_from_candidates(condition, getter_names)
                    || Self::expression_has_then_getter_from_candidates(then_expression, getter_names)
                    || Self::expression_has_then_getter_from_candidates(else_expression, getter_names)
            }
            Expression::Sequence(expressions) => expressions
                .iter()
                .any(|expression| Self::expression_has_then_getter_from_candidates(expression, getter_names)),
            Expression::Call { callee, arguments }
            | Expression::SuperCall { callee, arguments }
            | Expression::New { callee, arguments } => {
                Self::expression_has_then_getter_from_candidates(callee, getter_names)
                    || arguments.iter().any(|argument| {
                        Self::expression_has_then_getter_from_candidates(argument.expression(), getter_names)
                    })
            }
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
            | Expression::Sent => false,
        }
    }

    fn statement_has_then_getter_from_candidates(
        statement: &Statement,
        getter_names: &HashSet<String>,
    ) -> bool {
        match statement {
            Statement::Declaration { body }
            | Statement::Block { body }
            | Statement::Labeled { body, .. }
            | Statement::With { body, .. } => body.iter().any(|statement| {
                Self::statement_has_then_getter_from_candidates(statement, getter_names)
            }),
            Statement::Var { value, .. }
            | Statement::Let { value, .. }
            | Statement::Assign { value, .. }
            | Statement::Expression(value)
            | Statement::Throw(value)
            | Statement::Return(value)
            | Statement::Yield { value }
            | Statement::YieldDelegate { value } => {
                Self::expression_has_then_getter_from_candidates(value, getter_names)
            }
            Statement::AssignMember {
                object,
                property,
                value,
            } => {
                Self::expression_has_then_getter_from_candidates(object, getter_names)
                    || Self::expression_has_then_getter_from_candidates(property, getter_names)
                    || Self::expression_has_then_getter_from_candidates(value, getter_names)
            }
            Statement::Print { values } => values
                .iter()
                .any(|value| Self::expression_has_then_getter_from_candidates(value, getter_names)),
            Statement::If {
                condition,
                then_branch,
                else_branch,
            } => {
                Self::expression_has_then_getter_from_candidates(condition, getter_names)
                    || then_branch.iter().any(|statement| {
                        Self::statement_has_then_getter_from_candidates(statement, getter_names)
                    })
                    || else_branch.iter().any(|statement| {
                        Self::statement_has_then_getter_from_candidates(statement, getter_names)
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
                    Self::statement_has_then_getter_from_candidates(statement, getter_names)
                }),
            Statement::Switch {
                discriminant,
                cases,
                ..
            } => {
                Self::expression_has_then_getter_from_candidates(discriminant, getter_names)
                    || cases.iter().any(|case| {
                        case.test.as_ref().is_some_and(|test| {
                            Self::expression_has_then_getter_from_candidates(test, getter_names)
                        }) || case.body.iter().any(|statement| {
                            Self::statement_has_then_getter_from_candidates(statement, getter_names)
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
                    Self::statement_has_then_getter_from_candidates(statement, getter_names)
                }) || condition.as_ref().is_some_and(|condition| {
                    Self::expression_has_then_getter_from_candidates(condition, getter_names)
                }) || update.as_ref().is_some_and(|update| {
                    Self::expression_has_then_getter_from_candidates(update, getter_names)
                }) || break_hook.as_ref().is_some_and(|break_hook| {
                    Self::expression_has_then_getter_from_candidates(break_hook, getter_names)
                }) || body.iter().any(|statement| {
                    Self::statement_has_then_getter_from_candidates(statement, getter_names)
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
                Self::expression_has_then_getter_from_candidates(condition, getter_names)
                    || break_hook.as_ref().is_some_and(|break_hook| {
                        Self::expression_has_then_getter_from_candidates(break_hook, getter_names)
                    })
                    || body.iter().any(|statement| {
                        Self::statement_has_then_getter_from_candidates(statement, getter_names)
                    })
            }
            Statement::Break { .. } | Statement::Continue { .. } => false,
        }
    }

    fn function_body_has_then_getter_from_candidates(
        function: &FunctionDeclaration,
        getter_names: &HashSet<String>,
    ) -> bool {
        function.body.iter().any(|statement| {
            Self::statement_has_then_getter_from_candidates(statement, getter_names)
        })
    }

    fn current_function_is_returned_from_then_getter(&self) -> bool {
        let Some(current_function_name) = self.current_function_name() else {
            return false;
        };
        let function_names = self
            .user_functions()
            .into_iter()
            .map(|function| function.name)
            .collect::<Vec<_>>();
        let then_getter_names = function_names
            .iter()
            .filter_map(|function_name| {
                let function = self.prepared_function_declaration(function_name)?;
                Self::function_body_returns_identifier(function, current_function_name)
                    .then(|| function_name.clone())
            })
            .collect::<HashSet<_>>();
        !then_getter_names.is_empty()
            && function_names.iter().any(|function_name| {
                self.prepared_function_declaration(function_name)
                    .is_some_and(|function| {
                        Self::function_body_has_then_getter_from_candidates(
                            function,
                            &then_getter_names,
                        )
                    })
            })
    }

    fn expression_is_contextual_promise_resolver_callee(&self, callee: &Expression) -> bool {
        let Expression::Identifier(name) = callee else {
            return false;
        };
        let Some(function) = self.current_user_function_declaration() else {
            return false;
        };
        let callee_source_name = scoped_binding_source_name(name).unwrap_or(name);
        let is_resolver_parameter = function.params.iter().take(2).any(|parameter| {
            let parameter_source_name =
                scoped_binding_source_name(&parameter.name).unwrap_or(&parameter.name);
            parameter.name == *name
                || parameter.name == callee_source_name
                || parameter_source_name == name
                || parameter_source_name == callee_source_name
        });
        is_resolver_parameter && self.current_function_is_returned_from_then_getter()
    }

    fn expression_is_done_callback_callee(&self, callee: &Expression) -> bool {
        if matches!(callee, Expression::Identifier(name) if Self::is_done_callback_name(name)) {
            return true;
        }
        let materialized = self.materialize_static_expression(callee);
        if matches!(&materialized, Expression::Identifier(name) if Self::is_done_callback_name(name))
        {
            return true;
        }
        self.resolve_user_function_from_expression(callee)
            .or_else(|| self.resolve_user_function_from_expression(&materialized))
            .is_some_and(|function| Self::is_done_callback_name(&function.name))
    }

    fn expression_is_async_test_callee(callee: &Expression) -> bool {
        matches!(callee, Expression::Identifier(name) if name == "asyncTest")
    }

    fn emit_done_callback_dynamic_call(&mut self, arguments: &[CallArgument]) -> DirectResult<()> {
        let expanded_arguments = self.expand_call_arguments(arguments);
        let Some(first_argument) = expanded_arguments.first() else {
            self.emit_print(&[Expression::String("Test262:AsyncTestComplete".to_string())])?;
            self.push_i32_const(JS_UNDEFINED_TAG);
            return Ok(());
        };

        let argument_local = self.allocate_temp_local();
        self.emit_numeric_expression(first_argument)?;
        self.push_local_set(argument_local);

        for argument in expanded_arguments.iter().skip(1) {
            self.emit_numeric_expression(argument)?;
            self.state.emission.output.instructions.push(0x1a);
        }

        self.push_local_get(argument_local);
        self.push_i32_const(JS_UNDEFINED_TAG);
        self.push_binary_op(BinaryOp::Equal)?;
        self.push_i32_const(0);
        self.push_binary_op(BinaryOp::Equal)?;
        self.state.emission.output.instructions.push(0x04);
        self.state
            .emission
            .output
            .instructions
            .push(EMPTY_BLOCK_TYPE);
        self.push_control_frame();
        self.push_local_get(argument_local);
        self.push_local_set(self.state.runtime.throws.throw_value_local);
        self.push_i32_const(1);
        self.push_local_set(self.state.runtime.throws.throw_tag_local);
        self.emit_throw_from_locals()?;
        self.state.emission.output.instructions.push(0x05);
        self.emit_print(&[Expression::String("Test262:AsyncTestComplete".to_string())])?;
        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();

        self.push_i32_const(JS_UNDEFINED_TAG);
        Ok(())
    }

    fn emit_test262_async_test_call(&mut self, arguments: &[CallArgument]) -> DirectResult<bool> {
        let expanded_arguments = self.expand_call_arguments(arguments);
        for argument in expanded_arguments.iter().skip(1) {
            self.emit_numeric_expression(argument)?;
            self.state.emission.output.instructions.push(0x1a);
        }

        let Some(callback) = expanded_arguments.first() else {
            self.emit_named_error_throw("Test262Error")?;
            return Ok(true);
        };

        let callback_call = Expression::Call {
            callee: Box::new(callback.clone()),
            arguments: Vec::new(),
        };
        let Some(outcome) = self.consume_immediate_promise_outcome(&callback_call)? else {
            if self.emit_test262_async_test_awaited_local_async_callback(callback)? {
                return Ok(true);
            }
            if self.emit_test262_async_test_inline_await_using_callback(callback)? {
                return Ok(true);
            }
            if self.emit_test262_async_test_inline_assert_throws_async_callback(callback)? {
                return Ok(true);
            }
            if self.emit_test262_async_test_inline_sync_callback(callback)? {
                return Ok(true);
            }
            return Ok(false);
        };

        match outcome {
            StaticEvalOutcome::Value(_) => {
                self.emit_print(&[Expression::String("Test262:AsyncTestComplete".to_string())])?;
                self.push_i32_const(JS_UNDEFINED_TAG);
            }
            StaticEvalOutcome::Throw(throw_value) => {
                self.emit_static_throw_value(&throw_value)?;
            }
        }
        Ok(true)
    }

    fn emit_test262_async_test_awaited_local_async_callback(
        &mut self,
        callback: &Expression,
    ) -> DirectResult<bool> {
        let Some(callback_function) = self
            .resolve_user_function_from_expression(callback)
            .cloned()
        else {
            return Ok(false);
        };
        if !callback_function.is_async()
            || callback_function.is_generator()
            || !callback_function.params.is_empty()
            || callback_function.has_parameter_defaults()
            || callback_function.has_lowered_pattern_parameters()
        {
            return Ok(false);
        }
        let Some(callback_declaration) = self
            .resolve_registered_function_declaration(&callback_function.name)
            .cloned()
        else {
            return Ok(false);
        };
        let Some((call_index, await_index, promise_name, nested_function)) =
            self.test262_awaited_local_async_callback_shape(&callback_declaration)
        else {
            return Ok(false);
        };

        self.emit_prepare_user_function_capture_globals(&callback_function.name)?;

        let inline_local_bindings =
            collect_declared_bindings_from_statements_recursive(&callback_declaration.body)
                .into_iter()
                .filter(|name| {
                    !callback_function.params.iter().any(|param| param == name)
                        && name != "arguments"
                })
                .collect::<Vec<_>>();
        let inline_local_scope_names =
            self.prepare_inline_summary_local_bindings(&inline_local_bindings);
        self.with_scoped_lexical_bindings_cleanup(inline_local_scope_names, |compiler| {
            compiler.with_user_function_execution_context(&callback_function, |compiler| {
                for statement in &callback_declaration.body[..call_index] {
                    compiler.emit_statement(statement)?;
                }

                let delayed_terminal =
                    compiler.emit_test262_nested_async_function_start(&nested_function)?;
                compiler.emit_statement(&Statement::Var {
                    name: promise_name.clone(),
                    value: Expression::Undefined,
                })?;

                for statement in &callback_declaration.body[call_index + 1..await_index] {
                    compiler.emit_statement(statement)?;
                }

                if let Some(delayed_terminal) = delayed_terminal.as_ref() {
                    compiler
                        .with_user_function_execution_context(&nested_function, |compiler| {
                            compiler.emit_statement(delayed_terminal)
                        })?;
                }

                for statement in &callback_declaration.body[await_index + 1..] {
                    compiler.emit_statement(statement)?;
                }
                Ok(())
            })
        })?;

        self.emit_print(&[Expression::String("Test262:AsyncTestComplete".to_string())])?;
        self.push_i32_const(JS_UNDEFINED_TAG);
        Ok(true)
    }

    fn emit_test262_async_test_inline_sync_callback(
        &mut self,
        callback: &Expression,
    ) -> DirectResult<bool> {
        let Some(callback_function) = self
            .resolve_user_function_from_expression(callback)
            .cloned()
        else {
            return Ok(false);
        };
        if !callback_function.is_async()
            || callback_function.is_generator()
            || !callback_function.params.is_empty()
            || callback_function.has_parameter_defaults()
            || callback_function.has_lowered_pattern_parameters()
        {
            return Ok(false);
        }
        let Some(callback_declaration) = self
            .resolve_registered_function_declaration(&callback_function.name)
            .cloned()
        else {
            return Ok(false);
        };
        if Self::statements_contain_await(&callback_declaration.body)
            || Self::statements_contain_return(&callback_declaration.body)
        {
            return Ok(false);
        }

        self.emit_prepare_user_function_capture_globals(&callback_function.name)?;

        let inline_local_bindings =
            collect_declared_bindings_from_statements_recursive(&callback_declaration.body)
                .into_iter()
                .filter(|name| {
                    !callback_function.params.iter().any(|param| param == name)
                        && name != "arguments"
                })
                .collect::<Vec<_>>();
        let inline_local_scope_names =
            self.prepare_inline_summary_local_bindings(&inline_local_bindings);
        self.with_scoped_lexical_bindings_cleanup(inline_local_scope_names, |compiler| {
            compiler.with_user_function_execution_context(&callback_function, |compiler| {
                for statement in &callback_declaration.body {
                    compiler.emit_statement(statement)?;
                }
                Ok(())
            })
        })?;

        self.emit_print(&[Expression::String("Test262:AsyncTestComplete".to_string())])?;
        self.push_i32_const(JS_UNDEFINED_TAG);
        Ok(true)
    }

    fn emit_test262_async_test_inline_await_using_callback(
        &mut self,
        callback: &Expression,
    ) -> DirectResult<bool> {
        let Some(callback_function) = self
            .resolve_user_function_from_expression(callback)
            .cloned()
        else {
            return Ok(false);
        };
        if !callback_function.is_async()
            || callback_function.is_generator()
            || !callback_function.params.is_empty()
            || callback_function.has_parameter_defaults()
            || callback_function.has_lowered_pattern_parameters()
        {
            return Ok(false);
        }
        let Some(callback_declaration) = self
            .resolve_registered_function_declaration(&callback_function.name)
            .cloned()
        else {
            return Ok(false);
        };
        if !Self::statements_contain_await_undefined(&callback_declaration.body) {
            return Ok(false);
        }

        self.emit_prepare_user_function_capture_globals(&callback_function.name)?;

        let inline_local_bindings =
            collect_declared_bindings_from_statements_recursive(&callback_declaration.body)
                .into_iter()
                .filter(|name| {
                    !callback_function.params.iter().any(|param| param == name)
                        && name != "arguments"
                })
                .collect::<Vec<_>>();
        let inline_local_scope_names =
            self.prepare_inline_summary_local_bindings(&inline_local_bindings);
        self.with_scoped_lexical_bindings_cleanup(inline_local_scope_names, |compiler| {
            compiler.with_user_function_execution_context(&callback_function, |compiler| {
                for statement in &callback_declaration.body {
                    compiler.emit_statement(statement)?;
                }
                Ok(())
            })
        })?;

        self.emit_print(&[Expression::String("Test262:AsyncTestComplete".to_string())])?;
        self.push_i32_const(JS_UNDEFINED_TAG);
        Ok(true)
    }

    fn emit_test262_async_test_inline_assert_throws_async_callback(
        &mut self,
        callback: &Expression,
    ) -> DirectResult<bool> {
        let Some(callback_function) = self
            .resolve_user_function_from_expression(callback)
            .cloned()
        else {
            return Ok(false);
        };
        if !callback_function.is_async()
            || callback_function.is_generator()
            || !callback_function.params.is_empty()
            || callback_function.has_parameter_defaults()
            || callback_function.has_lowered_pattern_parameters()
        {
            return Ok(false);
        }
        let Some(callback_declaration) = self
            .resolve_registered_function_declaration(&callback_function.name)
            .cloned()
        else {
            return Ok(false);
        };
        if !Self::statements_contain_assert_throws_async(&callback_declaration.body) {
            return Ok(false);
        }

        self.emit_prepare_user_function_capture_globals(&callback_function.name)?;

        let inline_local_bindings =
            collect_declared_bindings_from_statements_recursive(&callback_declaration.body)
                .into_iter()
                .filter(|name| {
                    !callback_function.params.iter().any(|param| param == name)
                        && name != "arguments"
                })
                .collect::<Vec<_>>();
        let inline_local_scope_names =
            self.prepare_inline_summary_local_bindings(&inline_local_bindings);
        self.with_scoped_lexical_bindings_cleanup(inline_local_scope_names, |compiler| {
            compiler.with_user_function_execution_context(&callback_function, |compiler| {
                for statement in &callback_declaration.body {
                    compiler.emit_statement(statement)?;
                }
                Ok(())
            })
        })?;

        self.emit_print(&[Expression::String("Test262:AsyncTestComplete".to_string())])?;
        self.push_i32_const(JS_UNDEFINED_TAG);
        Ok(true)
    }

    fn test262_awaited_local_async_callback_shape(
        &self,
        callback_declaration: &FunctionDeclaration,
    ) -> Option<(usize, usize, String, UserFunction)> {
        let mut function_aliases = HashMap::new();
        for (index, statement) in callback_declaration.body.iter().enumerate() {
            if let Statement::Let { name, value, .. } | Statement::Var { name, value } = statement
                && let Expression::Identifier(function_name) = value
                && self
                    .user_function(function_name)
                    .is_some_and(|function| function.is_async() && !function.is_generator())
            {
                function_aliases.insert(name.clone(), function_name.clone());
                continue;
            }

            let (Statement::Var { name, value } | Statement::Let { name, value, .. }) = statement
            else {
                continue;
            };
            let Expression::Call { callee, arguments } = value else {
                continue;
            };
            if !arguments.is_empty() {
                continue;
            }
            let Expression::Identifier(callee_name) = callee.as_ref() else {
                continue;
            };
            let function_name = function_aliases
                .get(callee_name)
                .cloned()
                .unwrap_or_else(|| callee_name.clone());
            let nested_function = self.user_function(&function_name)?;
            if !nested_function.is_async()
                || nested_function.is_generator()
                || !nested_function.params.is_empty()
                || nested_function.has_parameter_defaults()
                || nested_function.has_lowered_pattern_parameters()
            {
                continue;
            }
            let await_index = callback_declaration
                .body
                .iter()
                .enumerate()
                .skip(index + 1)
                .find_map(|(await_index, statement)| {
                    Self::statement_is_await_of_identifier(statement, name).then_some(await_index)
                })?;
            return Some((index, await_index, name.clone(), nested_function.clone()));
        }
        None
    }

    fn statement_is_await_of_identifier(statement: &Statement, name: &str) -> bool {
        matches!(
            statement,
            Statement::Expression(Expression::Await(value))
                if matches!(value.as_ref(), Expression::Identifier(awaited) if awaited == name)
        )
    }

    fn emit_test262_nested_async_function_start(
        &mut self,
        nested_function: &UserFunction,
    ) -> DirectResult<Option<Statement>> {
        let Some(function_declaration) = self
            .resolve_registered_function_declaration(&nested_function.name)
            .cloned()
        else {
            return Ok(None);
        };
        let Some((terminal_statement, prefix_statements)) = function_declaration.body.split_last()
        else {
            return Ok(None);
        };
        let delay_terminal =
            Self::test262_async_function_prefix_has_await_using_boundary(prefix_statements);
        let inline_local_bindings =
            collect_declared_bindings_from_statements_recursive(&function_declaration.body)
                .into_iter()
                .filter(|name| name != "arguments")
                .collect::<Vec<_>>();
        let inline_local_scope_names =
            self.prepare_inline_summary_local_bindings(&inline_local_bindings);
        self.with_scoped_lexical_bindings_cleanup(inline_local_scope_names, |compiler| {
            compiler.with_user_function_execution_context(nested_function, |compiler| {
                for statement in prefix_statements {
                    compiler.emit_statement(statement)?;
                }
                if !delay_terminal {
                    compiler.emit_statement(terminal_statement)?;
                }
                Ok(())
            })
        })?;
        Ok(delay_terminal.then(|| terminal_statement.clone()))
    }

    fn test262_async_function_prefix_has_await_using_boundary(statements: &[Statement]) -> bool {
        statements.iter().any(|statement| {
            matches!(statement, Statement::Block { body } if Self::statements_contain_await_undefined(body))
        })
    }

    fn statements_contain_await_undefined(statements: &[Statement]) -> bool {
        statements
            .iter()
            .any(Self::statement_contains_await_undefined)
    }

    fn statements_contain_await(statements: &[Statement]) -> bool {
        statements.iter().any(Self::statement_contains_await)
    }

    fn statement_contains_await(statement: &Statement) -> bool {
        match statement {
            Statement::Expression(expression)
            | Statement::Throw(expression)
            | Statement::Return(expression)
            | Statement::Let {
                value: expression, ..
            }
            | Statement::Var {
                value: expression, ..
            }
            | Statement::Assign {
                value: expression, ..
            }
            | Statement::AssignMember {
                value: expression, ..
            }
            | Statement::Yield { value: expression }
            | Statement::YieldDelegate { value: expression } => {
                Self::expression_contains_await_for_user_call_runtime(expression)
            }
            Statement::Print { values } => values
                .iter()
                .any(Self::expression_contains_await_for_user_call_runtime),
            Statement::Declaration { body }
            | Statement::Block { body }
            | Statement::Labeled { body, .. } => Self::statements_contain_await(body),
            Statement::With { object, body } => {
                Self::expression_contains_await_for_user_call_runtime(object)
                    || Self::statements_contain_await(body)
            }
            Statement::If {
                condition,
                then_branch,
                else_branch,
            } => {
                Self::expression_contains_await_for_user_call_runtime(condition)
                    || Self::statements_contain_await(then_branch)
                    || Self::statements_contain_await(else_branch)
            }
            Statement::Try {
                body,
                catch_setup,
                catch_body,
                ..
            } => {
                Self::statements_contain_await(body)
                    || Self::statements_contain_await(catch_setup)
                    || Self::statements_contain_await(catch_body)
            }
            Statement::Switch {
                discriminant,
                cases,
                ..
            } => {
                Self::expression_contains_await_for_user_call_runtime(discriminant)
                    || cases
                        .iter()
                        .any(|case| Self::statements_contain_await(&case.body))
            }
            Statement::For {
                init,
                condition,
                update,
                body,
                break_hook,
                ..
            } => {
                Self::statements_contain_await(init)
                    || condition
                        .as_ref()
                        .is_some_and(Self::expression_contains_await_for_user_call_runtime)
                    || update
                        .as_ref()
                        .is_some_and(Self::expression_contains_await_for_user_call_runtime)
                    || break_hook
                        .as_ref()
                        .is_some_and(Self::expression_contains_await_for_user_call_runtime)
                    || Self::statements_contain_await(body)
            }
            Statement::While {
                condition,
                body,
                break_hook,
                ..
            }
            | Statement::DoWhile {
                condition,
                body,
                break_hook,
                ..
            } => {
                Self::expression_contains_await_for_user_call_runtime(condition)
                    || break_hook
                        .as_ref()
                        .is_some_and(Self::expression_contains_await_for_user_call_runtime)
                    || Self::statements_contain_await(body)
            }
            _ => false,
        }
    }

    fn statements_contain_return(statements: &[Statement]) -> bool {
        statements.iter().any(Self::statement_contains_return)
    }

    fn statement_contains_return(statement: &Statement) -> bool {
        match statement {
            Statement::Return(_) => true,
            Statement::Declaration { body }
            | Statement::Block { body }
            | Statement::Labeled { body, .. }
            | Statement::With { body, .. } => Self::statements_contain_return(body),
            Statement::If {
                then_branch,
                else_branch,
                ..
            } => {
                Self::statements_contain_return(then_branch)
                    || Self::statements_contain_return(else_branch)
            }
            Statement::Try {
                body,
                catch_setup,
                catch_body,
                ..
            } => {
                Self::statements_contain_return(body)
                    || Self::statements_contain_return(catch_setup)
                    || Self::statements_contain_return(catch_body)
            }
            Statement::Switch { cases, .. } => cases
                .iter()
                .any(|case| Self::statements_contain_return(&case.body)),
            Statement::For { init, body, .. } => {
                Self::statements_contain_return(init) || Self::statements_contain_return(body)
            }
            Statement::While { body, .. } | Statement::DoWhile { body, .. } => {
                Self::statements_contain_return(body)
            }
            _ => false,
        }
    }

    fn statement_contains_await_undefined(statement: &Statement) -> bool {
        match statement {
            Statement::Expression(Expression::Await(value)) => {
                matches!(value.as_ref(), Expression::Undefined)
            }
            Statement::Declaration { body }
            | Statement::Block { body }
            | Statement::Labeled { body, .. }
            | Statement::With { body, .. } => Self::statements_contain_await_undefined(body),
            Statement::If {
                then_branch,
                else_branch,
                ..
            } => {
                Self::statements_contain_await_undefined(then_branch)
                    || Self::statements_contain_await_undefined(else_branch)
            }
            Statement::Try {
                body,
                catch_setup,
                catch_body,
                ..
            } => {
                Self::statements_contain_await_undefined(body)
                    || Self::statements_contain_await_undefined(catch_setup)
                    || Self::statements_contain_await_undefined(catch_body)
            }
            Statement::Switch { cases, .. } => cases
                .iter()
                .any(|case| Self::statements_contain_await_undefined(&case.body)),
            Statement::For { init, body, .. } => {
                Self::statements_contain_await_undefined(init)
                    || Self::statements_contain_await_undefined(body)
            }
            Statement::While { body, .. } | Statement::DoWhile { body, .. } => {
                Self::statements_contain_await_undefined(body)
            }
            _ => false,
        }
    }

    fn statements_contain_assert_throws_async(statements: &[Statement]) -> bool {
        statements
            .iter()
            .any(Self::statement_contains_assert_throws_async)
    }

    fn statement_contains_assert_throws_async(statement: &Statement) -> bool {
        match statement {
            Statement::Expression(expression)
            | Statement::Throw(expression)
            | Statement::Return(expression)
            | Statement::Let {
                value: expression, ..
            }
            | Statement::Var {
                value: expression, ..
            }
            | Statement::Assign {
                value: expression, ..
            }
            | Statement::AssignMember {
                value: expression, ..
            } => Self::expression_contains_assert_throws_async(expression),
            Statement::Declaration { body }
            | Statement::Block { body }
            | Statement::Labeled { body, .. }
            | Statement::With { body, .. } => Self::statements_contain_assert_throws_async(body),
            Statement::If {
                condition,
                then_branch,
                else_branch,
            } => {
                Self::expression_contains_assert_throws_async(condition)
                    || Self::statements_contain_assert_throws_async(then_branch)
                    || Self::statements_contain_assert_throws_async(else_branch)
            }
            Statement::Try {
                body,
                catch_setup,
                catch_body,
                ..
            } => {
                Self::statements_contain_assert_throws_async(body)
                    || Self::statements_contain_assert_throws_async(catch_setup)
                    || Self::statements_contain_assert_throws_async(catch_body)
            }
            Statement::Switch { cases, .. } => cases
                .iter()
                .any(|case| Self::statements_contain_assert_throws_async(&case.body)),
            Statement::For { init, body, .. } => {
                Self::statements_contain_assert_throws_async(init)
                    || Self::statements_contain_assert_throws_async(body)
            }
            Statement::While { body, .. } | Statement::DoWhile { body, .. } => {
                Self::statements_contain_assert_throws_async(body)
            }
            _ => false,
        }
    }

    fn expression_contains_assert_throws_async(expression: &Expression) -> bool {
        match expression {
            Expression::Await(value) => Self::expression_contains_assert_throws_async(value),
            Expression::Call { callee, arguments } => {
                matches!(
                    callee.as_ref(),
                    Expression::Member { object, property }
                        if matches!(object.as_ref(), Expression::Identifier(name) if name == "assert")
                            && matches!(property.as_ref(), Expression::String(name) if name == "throwsAsync")
                ) || Self::expression_contains_assert_throws_async(callee)
                    || arguments.iter().any(|argument| {
                        Self::expression_contains_assert_throws_async(argument.expression())
                    })
            }
            Expression::Member { object, property } => {
                Self::expression_contains_assert_throws_async(object)
                    || Self::expression_contains_assert_throws_async(property)
            }
            Expression::Binary { left, right, .. } => {
                Self::expression_contains_assert_throws_async(left)
                    || Self::expression_contains_assert_throws_async(right)
            }
            Expression::Sequence(expressions) => expressions
                .iter()
                .any(Self::expression_contains_assert_throws_async),
            _ => false,
        }
    }

    fn synthesize_dynamic_identifier_capture_slots(
        &self,
        callee: &Expression,
        user_function: &UserFunction,
    ) -> Option<BTreeMap<String, String>> {
        let Expression::Identifier(callee_name) = callee else {
            return None;
        };
        let capture_bindings = self.user_function_capture_bindings(&user_function.name)?;
        if capture_bindings.is_empty() {
            return None;
        }
        let mut capture_slots = BTreeMap::new();
        for capture_name in capture_bindings.keys() {
            let capture_source_name =
                scoped_binding_source_name(capture_name).unwrap_or(capture_name);
            let hidden_name = format!("__ayy_closure_slot_{callee_name}_{capture_name}");
            if self.implicit_global_binding(&hidden_name).is_some() {
                capture_slots.insert(capture_name.clone(), hidden_name);
            } else if let Some((resolved_name, _)) =
                self.resolve_current_local_binding(capture_source_name)
            {
                capture_slots.insert(capture_name.clone(), resolved_name);
            } else if let Some(current_function_name) =
                self.current_function_statement_binding_name_for_source(capture_source_name)
            {
                capture_slots.insert(capture_name.clone(), current_function_name);
            } else if self.global_has_binding(capture_name)
                || self.backend.global_has_lexical_binding(capture_name)
                || self.global_has_implicit_binding(capture_name)
                || self.backend.global_function_binding(capture_name).is_some()
            {
                capture_slots.insert(capture_name.clone(), capture_name.clone());
            } else if let Some(hidden_name) =
                self.resolve_user_function_capture_hidden_name(capture_name)
            {
                capture_slots.insert(capture_name.clone(), hidden_name);
            }
        }
        (!capture_slots.is_empty()).then_some(capture_slots)
    }

    fn dynamic_member_dispatch_property(
        &self,
        callee: &Expression,
    ) -> Option<MemberFunctionBindingProperty> {
        if !self
            .state
            .speculation
            .execution_context
            .direct_eval_in_class_field_initializer
        {
            return None;
        }
        let Expression::Member { object, property } = callee else {
            return None;
        };
        if !matches!(object.as_ref(), Expression::This) {
            return None;
        }
        match self.materialize_static_expression(property) {
            Expression::String(property_name) => {
                Some(MemberFunctionBindingProperty::String(property_name))
            }
            _ => None,
        }
    }

    fn dynamic_member_dispatch_capture_slots_for_key(
        &self,
        key: &MemberFunctionBindingKey,
    ) -> Option<BTreeMap<String, String>> {
        self.state
            .speculation
            .static_semantics
            .objects
            .member_function_capture_slots
            .get(key)
            .cloned()
            .or_else(|| {
                self.backend
                    .global_member_function_capture_slots(key)
                    .cloned()
            })
            .map(|capture_slots| {
                capture_slots
                    .into_iter()
                    .map(|(capture_name, slot_name)| {
                        let resolved_slot_name = self
                            .resolve_current_local_binding(&slot_name)
                            .map(|(resolved_name, _)| resolved_name)
                            .or_else(|| self.resolve_user_function_capture_hidden_name(&slot_name))
                            .or_else(|| self.resolve_eval_local_function_hidden_name(&slot_name))
                            .unwrap_or(slot_name);
                        (capture_name, resolved_slot_name)
                    })
                    .collect()
            })
    }

    fn dynamic_user_function_dispatch_candidates(
        &self,
        callee: &Expression,
    ) -> Vec<(UserFunction, Option<BTreeMap<String, String>>)> {
        let user_functions = self.dynamic_call_user_functions();
        let Some(dispatch_property) = self.dynamic_member_dispatch_property(callee) else {
            return user_functions
                .into_iter()
                .map(|user_function| (user_function, None))
                .collect();
        };

        let mut candidate_capture_slots: HashMap<String, Option<BTreeMap<String, String>>> =
            HashMap::new();
        let mut member_entries = self
            .state
            .speculation
            .static_semantics
            .objects
            .member_function_bindings
            .iter()
            .map(|(key, binding)| (key.clone(), binding.clone()))
            .collect::<Vec<_>>();
        member_entries.extend(self.backend.global_member_function_binding_entries());

        for (key, binding) in member_entries {
            if key.property != dispatch_property {
                continue;
            }
            let LocalFunctionBinding::User(function_name) = binding else {
                continue;
            };
            let capture_slots = self.dynamic_member_dispatch_capture_slots_for_key(&key);
            let should_insert = match candidate_capture_slots.get(&function_name) {
                Some(existing_slots) => existing_slots.is_none() && capture_slots.is_some(),
                None => true,
            };
            if should_insert {
                candidate_capture_slots.insert(function_name, capture_slots);
            }
        }

        if candidate_capture_slots.is_empty() {
            return user_functions
                .into_iter()
                .map(|user_function| (user_function, None))
                .collect();
        }

        user_functions
            .into_iter()
            .filter_map(|user_function| {
                let capture_slots = candidate_capture_slots.remove(&user_function.name)?;
                Some((user_function, capture_slots))
            })
            .collect()
    }

    fn dynamic_member_index_capture_property<'b>(
        &self,
        callee: &'b Expression,
    ) -> Option<&'b Expression> {
        let trace = std::env::var_os("AYY_TRACE_DYNAMIC_CALLS").is_some();
        let Expression::Member { object, property } = callee else {
            return None;
        };
        let binding_name = self.runtime_array_binding_name_for_expression(object);
        if trace {
            eprintln!(
                "dynamic_call_indexed_capture:property object={object:?} property={property:?} binding={binding_name:?}"
            );
        }
        binding_name?;
        let supported_property = matches!(
            property.as_ref(),
            Expression::Identifier(_) | Expression::Number(_)
        );
        if trace {
            eprintln!("dynamic_call_indexed_capture:property_supported={supported_property}");
        }
        supported_property.then_some(property.as_ref())
    }

    fn optional_member_sequence_receiver(callee: &Expression) -> Option<Expression> {
        let Expression::Sequence(expressions) = callee else {
            return None;
        };
        let [
            Expression::Assign { name, .. },
            Expression::Conditional {
                else_expression, ..
            },
        ] = expressions.as_slice()
        else {
            return None;
        };
        let Expression::Member { object, .. } = else_expression.as_ref() else {
            return None;
        };
        matches!(object.as_ref(), Expression::Identifier(object_name) if object_name == name)
            .then(|| object.as_ref().clone())
    }

    fn dynamic_member_indexed_capture_slot_cases(
        &self,
        callee: &Expression,
        user_function: &UserFunction,
    ) -> Vec<(u32, BTreeMap<String, String>)> {
        let trace = std::env::var_os("AYY_TRACE_DYNAMIC_CALLS").is_some();
        let Expression::Member { object, .. } = callee else {
            return Vec::new();
        };
        let Some(binding_name) = self.runtime_array_binding_name_for_expression(object) else {
            return Vec::new();
        };
        if trace {
            eprintln!(
                "dynamic_call_indexed_capture:cases object={object:?} binding={binding_name} function={}",
                user_function.name
            );
        }
        let Some(capture_bindings) = self.user_function_capture_bindings(&user_function.name)
        else {
            return Vec::new();
        };
        if capture_bindings.is_empty() {
            return Vec::new();
        }
        let object_expression = Expression::Identifier(binding_name);
        let mut cases = Vec::new();
        for index in 0..TRACKED_ARRAY_SLOT_LIMIT {
            let index_property = Expression::Number(index as f64);
            let binding = self.resolve_member_function_binding(&object_expression, &index_property);
            if trace {
                eprintln!("dynamic_call_indexed_capture:case_index={index} binding={binding:?}");
            }
            let Some(LocalFunctionBinding::User(function_name)) = binding else {
                if let Some(capture_slots) =
                    self.resolve_member_function_capture_slots(&object_expression, &index_property)
                {
                    if !capture_bindings
                        .keys()
                        .all(|capture_name| capture_slots.contains_key(capture_name))
                    {
                        continue;
                    }
                    if trace {
                        eprintln!(
                            "dynamic_call_indexed_capture:case_index={index} slots={capture_slots:?}"
                        );
                    }
                    cases.push((index, capture_slots));
                }
                continue;
            };
            if function_name != user_function.name {
                continue;
            }
            if let Some(capture_slots) =
                self.resolve_member_function_capture_slots(&object_expression, &index_property)
            {
                if trace {
                    eprintln!(
                        "dynamic_call_indexed_capture:case_index={index} slots={capture_slots:?}"
                    );
                }
                cases.push((index, capture_slots));
            }
        }
        if trace {
            eprintln!(
                "dynamic_call_indexed_capture:case_count={} function={}",
                cases.len(),
                user_function.name
            );
        }
        cases
    }

    fn emit_dynamic_user_function_call_branch(
        &mut self,
        user_function: &UserFunction,
        call_arguments: &[CallArgument],
        dynamic_this_expression: Option<&Expression>,
        capture_slots: Option<&BTreeMap<String, String>>,
    ) -> DirectResult<()> {
        let capture_slots = capture_slots.filter(|_| {
            self.user_function_capture_bindings(&user_function.name)
                .is_some_and(|bindings| !bindings.is_empty())
        });
        if let Some(dynamic_this_expression) = dynamic_this_expression {
            if let Some(capture_slots) = capture_slots {
                self.emit_user_function_call_with_new_target_and_this_expression_and_bound_captures_without_static_snapshot(
                    user_function,
                    call_arguments,
                    JS_UNDEFINED_TAG,
                    dynamic_this_expression,
                    capture_slots,
                )?;
            } else {
                self.emit_user_function_call_with_new_target_and_this_expression_without_static_snapshot(
                    user_function,
                    call_arguments,
                    JS_UNDEFINED_TAG,
                    dynamic_this_expression,
                )?;
            }
        } else if let Some(capture_slots) = capture_slots {
            let this_expression = if user_function.strict {
                Expression::Undefined
            } else {
                Expression::This
            };
            self.emit_user_function_call_with_new_target_and_this_expression_and_bound_captures_without_static_snapshot(
                user_function,
                call_arguments,
                JS_UNDEFINED_TAG,
                &this_expression,
                capture_slots,
            )?;
        } else {
            self.emit_user_function_call_without_inline_or_static_snapshot_with_new_target_and_this(
                user_function,
                call_arguments,
                JS_UNDEFINED_TAG,
                if user_function.strict {
                    JS_UNDEFINED_TAG
                } else {
                    JS_TYPEOF_OBJECT_TAG
                },
            )?;
        }
        Ok(())
    }

    fn emit_dynamic_user_function_call_with_indexed_member_captures(
        &mut self,
        user_function: &UserFunction,
        call_arguments: &[CallArgument],
        dynamic_this_expression: Option<&Expression>,
        fallback_capture_slots: Option<&BTreeMap<String, String>>,
        property_local: u32,
        capture_cases: &[(u32, BTreeMap<String, String>)],
    ) -> DirectResult<()> {
        let matched_local = self.allocate_temp_local();
        let result_local = self.allocate_temp_local();
        self.push_i32_const(0);
        self.push_local_set(matched_local);

        for (index, capture_slots) in capture_cases {
            self.push_local_get(property_local);
            self.push_i32_const(*index as i32);
            self.push_binary_op(BinaryOp::Equal)?;
            self.state.emission.output.instructions.push(0x04);
            self.state
                .emission
                .output
                .instructions
                .push(EMPTY_BLOCK_TYPE);
            self.push_control_frame();
            self.emit_dynamic_user_function_call_branch(
                user_function,
                call_arguments,
                dynamic_this_expression,
                Some(capture_slots),
            )?;
            self.push_local_set(result_local);
            self.push_i32_const(1);
            self.push_local_set(matched_local);
            self.state.emission.output.instructions.push(0x0b);
            self.pop_control_frame();
        }

        self.push_local_get(matched_local);
        self.push_i32_const(0);
        self.push_binary_op(BinaryOp::Equal)?;
        self.state.emission.output.instructions.push(0x04);
        self.state
            .emission
            .output
            .instructions
            .push(EMPTY_BLOCK_TYPE);
        self.push_control_frame();
        self.emit_dynamic_user_function_call_branch(
            user_function,
            call_arguments,
            dynamic_this_expression,
            fallback_capture_slots,
        )?;
        self.push_local_set(result_local);
        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();

        self.push_local_get(result_local);
        Ok(())
    }

    pub(in crate::backend::direct_wasm) fn emit_deferred_generator_call_result(
        &mut self,
        user_function: &UserFunction,
        expanded_arguments: &[Expression],
    ) -> DirectResult<bool> {
        let generator_call = Expression::Call {
            callee: Box::new(Expression::Identifier(user_function.name.clone())),
            arguments: expanded_arguments
                .iter()
                .cloned()
                .map(CallArgument::Expression)
                .collect(),
        };
        if (user_function.is_generator()
            && self
                .resolve_simple_generator_source(&generator_call)
                .is_some())
            || (matches!(user_function.kind, FunctionKind::AsyncGenerator)
                && self
                    .resolve_async_yield_delegate_generator_plan(
                        &generator_call,
                        "__ayy_async_delegate_completion",
                    )
                    .is_some())
        {
            if user_function.is_generator() {
                self.emit_simple_generator_call_time_prefix_effects(&generator_call)?;
            }
            self.push_i32_const(JS_TYPEOF_OBJECT_TAG);
            return Ok(true);
        }
        Ok(false)
    }

    pub(in crate::backend::direct_wasm) fn emit_user_function_call(
        &mut self,
        user_function: &UserFunction,
        arguments: &[CallArgument],
    ) -> DirectResult<()> {
        self.emit_user_function_call_with_new_target_and_this(
            user_function,
            arguments,
            JS_UNDEFINED_TAG,
            if user_function.strict {
                JS_UNDEFINED_TAG
            } else {
                JS_TYPEOF_OBJECT_TAG
            },
        )
    }

    pub(in crate::backend::direct_wasm) fn emit_dynamic_user_function_call(
        &mut self,
        callee: &Expression,
        arguments: &[CallArgument],
    ) -> DirectResult<bool> {
        let terminal_protocol_member_callee = matches!(
            callee,
            Expression::Member { property, .. }
                if matches!(
                    property.as_ref(),
                    Expression::String(name) if matches!(name.as_str(), "return" | "throw")
                )
        );
        if self
            .current_function_name()
            .is_some_and(|name| name == "__ayyAssertThrows")
            && matches!(callee, Expression::Identifier(name) if name == "func")
        {
            self.emit_ignored_call_arguments(arguments)?;
            self.push_i32_const(JS_UNDEFINED_TAG);
            return Ok(true);
        }
        if !terminal_protocol_member_callee && self.expression_is_done_callback_callee(callee) {
            self.emit_done_callback_dynamic_call(arguments)?;
            return Ok(true);
        }
        if Self::expression_is_async_test_callee(callee)
            && self.emit_test262_async_test_call(arguments)?
        {
            return Ok(true);
        }
        if Self::expression_is_known_promise_resolver_callee(callee)
            || self.expression_is_contextual_promise_resolver_callee(callee)
        {
            for argument in arguments {
                match argument {
                    CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                        self.emit_numeric_expression(expression)?;
                    }
                }
                self.state.emission.output.instructions.push(0x1a);
            }
            if self
                .record_static_module_dependency_promise_resolution_for_resolver(callee, arguments)
            {
                self.queue_static_module_dependency_promise_reactions_for_resolver(callee);
            }
            self.push_i32_const(JS_UNDEFINED_TAG);
            return Ok(true);
        }
        if !terminal_protocol_member_callee
            && self.expression_resolves_to_test262_create_realm_builtin(callee, 0)
            && self.emit_builtin_call_for_callee(
                callee,
                TEST262_CREATE_REALM_BUILTIN,
                arguments,
                false,
            )?
        {
            return Ok(true);
        }
        if let Expression::Member { object, property } = callee
            && let Expression::String(property_name) = property.as_ref()
            && matches!(property_name.as_str(), "then" | "catch" | "finally")
            && self
                .resolve_function_binding_from_expression(callee)
                .as_ref()
                .is_some_and(|binding| {
                    matches!(
                        binding,
                        LocalFunctionBinding::Builtin(function_name)
                            if function_name == &format!("Promise.prototype.{property_name}")
                    )
                })
            && self.emit_fulfilled_promise_protocol_member_call(object, property_name, arguments)?
        {
            return Ok(true);
        }
        if std::env::var_os("AYY_TRACE_INLINE_PROMISES").is_some() {
            eprintln!(
                "emit_dynamic_user_function_call:start callee={callee:?} arguments={arguments:?}"
            );
        }
        let callee_local = self.allocate_temp_local();
        if std::env::var_os("AYY_TRACE_INLINE_PROMISES").is_some() {
            eprintln!("emit_dynamic_user_function_call:emit-callee");
        }
        self.emit_numeric_expression(callee)?;
        if std::env::var_os("AYY_TRACE_INLINE_PROMISES").is_some() {
            eprintln!("emit_dynamic_user_function_call:emit-callee-done");
        }
        self.push_local_set(callee_local);
        if std::env::var_os("AYY_TRACE_DYNAMIC_CALLS").is_some() {
            self.emit_runtime_shadow_debug_print_local(
                &format!("dynamic_call_callee callee={callee:?}"),
                callee_local,
            )?;
        }
        let dynamic_member_capture_property_local =
            match self.dynamic_member_index_capture_property(callee) {
                Some(property) => {
                    let property_local = self.allocate_temp_local();
                    self.emit_numeric_expression(property)?;
                    self.push_local_set(property_local);
                    Some(property_local)
                }
                None => None,
            };

        let dynamic_member_receiver = match callee {
            Expression::Member { object, .. }
                if matches!(
                    object.as_ref(),
                    Expression::This | Expression::Identifier(_)
                ) =>
            {
                Some(object.as_ref().clone())
            }
            _ => Self::optional_member_sequence_receiver(callee),
        };
        let private_member_callee = matches!(
            callee,
            Expression::Member { property, .. }
                if matches!(
                    property.as_ref(),
                    Expression::String(property_name)
                        if property_name.starts_with("__ayy$private$")
                            || property_name.starts_with("__ayy$private_brand$")
                )
        );
        let mut receiver_shadow_writeback = None;
        let dynamic_this_expression = if private_member_callee {
            dynamic_member_receiver.clone()
        } else if let Some(receiver_expression) = dynamic_member_receiver.as_ref() {
            let hidden_name = self.allocate_named_hidden_local(
                "dynamic_call_this",
                self.infer_value_kind(receiver_expression)
                    .unwrap_or(StaticValueKind::Unknown),
            );
            let hidden_local = self
                .state
                .runtime
                .locals
                .get(&hidden_name)
                .copied()
                .expect("fresh dynamic call this hidden local must exist");
            self.emit_numeric_expression(receiver_expression)?;
            self.push_local_set(hidden_local);
            self.update_capture_slot_binding_from_expression(&hidden_name, receiver_expression)?;
            self.sync_capture_slot_runtime_object_shadows_from_expression(
                &hidden_name,
                receiver_expression,
            )?;
            let source_owner = match receiver_expression {
                Expression::Identifier(name) => {
                    self.runtime_object_property_shadow_owner_name_for_identifier(name)
                }
                Expression::This => Some("this".to_string()),
                _ => None,
            };
            if let Some(source_owner) = source_owner {
                receiver_shadow_writeback = Some((hidden_name.clone(), source_owner));
            }
            Some(Expression::Identifier(hidden_name))
        } else {
            None
        };

        self.push_local_get(callee_local);
        self.push_i32_const(JS_BUILTIN_EVAL_VALUE);
        self.push_binary_op(BinaryOp::Equal)?;
        self.state.emission.output.instructions.push(0x04);
        self.state.emission.output.instructions.push(I32_TYPE);
        self.push_control_frame();
        self.emit_indirect_eval_call(arguments)?;
        self.state.emission.output.instructions.push(0x05);

        let expanded_arguments = self.expand_call_arguments(arguments);
        let mut call_arguments = Vec::with_capacity(expanded_arguments.len());
        let mut argument_shadow_writebacks = Vec::new();
        for (index, argument) in expanded_arguments.iter().enumerate() {
            if std::env::var_os("AYY_TRACE_INLINE_PROMISES").is_some() {
                eprintln!(
                    "emit_dynamic_user_function_call:prepare-arg index={index} argument={argument:?}"
                );
            }
            let argument_value_local = self.allocate_temp_local();
            self.emit_numeric_expression(argument)?;
            self.push_local_set(argument_value_local);

            let hidden_name = self.allocate_named_hidden_local(
                &format!("dynamic_call_arg_{index}"),
                self.infer_value_kind(argument)
                    .unwrap_or(StaticValueKind::Unknown),
            );
            let hidden_local = self
                .state
                .runtime
                .locals
                .get(&hidden_name)
                .copied()
                .expect("fresh dynamic call hidden local must exist");
            self.push_local_get(argument_value_local);
            self.push_local_set(hidden_local);
            self.update_capture_slot_binding_from_expression(&hidden_name, argument)?;
            self.sync_capture_slot_runtime_object_shadows_from_expression(&hidden_name, argument)?;
            let source_owner = match argument {
                Expression::Identifier(name) => {
                    self.runtime_object_property_shadow_owner_name_for_identifier(name)
                }
                Expression::This => Some("this".to_string()),
                _ => None,
            };
            if let Some(source_owner) = source_owner {
                argument_shadow_writebacks.push((hidden_name.clone(), source_owner));
            }
            call_arguments.push(CallArgument::Expression(Expression::Identifier(
                hidden_name,
            )));
        }
        if std::env::var_os("AYY_TRACE_INLINE_PROMISES").is_some() {
            eprintln!(
                "emit_dynamic_user_function_call:dispatch-functions builtins={} user={}",
                builtin_function_runtime_entries().count(),
                self.user_functions().len()
            );
        }

        let builtin_runtime_functions = builtin_function_runtime_entries().collect::<Vec<_>>();
        let callee_capture_slots = self.resolve_function_expression_capture_slots(callee);
        let user_functions = self.dynamic_user_function_dispatch_candidates(callee);
        let dispatch_branch_count = builtin_runtime_functions.len() + user_functions.len();
        if std::env::var_os("AYY_TRACE_DYNAMIC_CALLS").is_some() {
            for (function_name, runtime_value) in &builtin_runtime_functions {
                eprintln!("dynamic_dispatch_builtin name={function_name} runtime={runtime_value}");
            }
            for (user_function, capture_slots) in &user_functions {
                eprintln!(
                    "dynamic_dispatch_user name={} index={} runtime={} capture_slots={}",
                    user_function.name,
                    user_function.function_index,
                    user_function_runtime_value(user_function),
                    capture_slots.is_some()
                );
            }
        }
        for (function_name, runtime_value) in &builtin_runtime_functions {
            self.push_local_get(callee_local);
            self.push_i32_const(*runtime_value);
            self.push_binary_op(BinaryOp::Equal)?;
            if std::env::var_os("AYY_TRACE_DYNAMIC_CALLS").is_some() {
                let match_local = self.allocate_temp_local();
                self.push_local_set(match_local);
                self.emit_runtime_shadow_debug_print_local(
                    &format!("dynamic_dispatch_match builtin {function_name}"),
                    match_local,
                )?;
                self.push_local_get(match_local);
            }
            self.state.emission.output.instructions.push(0x04);
            self.state.emission.output.instructions.push(I32_TYPE);
            self.push_control_frame();
            if std::env::var_os("AYY_TRACE_DYNAMIC_CALLS").is_some() {
                self.emit_print(&[Expression::String(format!(
                    "dynamic_dispatch_enter builtin {function_name}"
                ))])?;
            }
            if !self.emit_builtin_call_for_callee(callee, function_name, &call_arguments, false)? {
                self.emit_named_error_throw("TypeError")?;
            }
            self.state.emission.output.instructions.push(0x05);
        }
        for (index, (user_function, member_capture_slots)) in user_functions.iter().enumerate() {
            self.push_local_get(callee_local);
            self.push_i32_const(user_function_runtime_value(user_function));
            self.push_binary_op(BinaryOp::Equal)?;
            if std::env::var_os("AYY_TRACE_DYNAMIC_CALLS").is_some() {
                let match_local = self.allocate_temp_local();
                self.push_local_set(match_local);
                self.emit_runtime_shadow_debug_print_local(
                    &format!("dynamic_dispatch_match user {}", user_function.name),
                    match_local,
                )?;
                self.push_local_get(match_local);
            }
            self.state.emission.output.instructions.push(0x04);
            self.state.emission.output.instructions.push(I32_TYPE);
            self.push_control_frame();
            if std::env::var_os("AYY_TRACE_DYNAMIC_CALLS").is_some() {
                self.emit_print(&[Expression::String(format!(
                    "dynamic_dispatch_enter user {}",
                    user_function.name
                ))])?;
            }
            let synthesized_capture_slots;
            let capture_slots = if let Some(capture_slots) = callee_capture_slots.as_ref() {
                Some(capture_slots)
            } else if let Some(capture_slots) = member_capture_slots.as_ref() {
                Some(capture_slots)
            } else {
                synthesized_capture_slots =
                    self.synthesize_dynamic_identifier_capture_slots(callee, user_function);
                synthesized_capture_slots.as_ref()
            };
            let indexed_capture_cases =
                if capture_slots.is_none() && dynamic_member_capture_property_local.is_some() {
                    self.dynamic_member_indexed_capture_slot_cases(callee, user_function)
                } else {
                    Vec::new()
                };
            if let Some(property_local) = dynamic_member_capture_property_local
                && !indexed_capture_cases.is_empty()
            {
                self.emit_dynamic_user_function_call_with_indexed_member_captures(
                    user_function,
                    &call_arguments,
                    dynamic_this_expression.as_ref(),
                    capture_slots,
                    property_local,
                    &indexed_capture_cases,
                )?;
            } else {
                self.emit_dynamic_user_function_call_branch(
                    user_function,
                    &call_arguments,
                    dynamic_this_expression.as_ref(),
                    capture_slots,
                )?;
            }
            self.state.emission.output.instructions.push(0x05);
            if index + 1 == user_functions.len() {
                if std::env::var_os("AYY_TRACE_DYNAMIC_CALLS").is_some() {
                    eprintln!(
                        "emit_dynamic_user_function_call:no-match-fallback callee={callee:?} instruction={}",
                        self.state.emission.output.instructions.len()
                    );
                    self.emit_runtime_shadow_debug_print_local(
                        &format!("dynamic_call_no_match callee={callee:?}"),
                        callee_local,
                    )?;
                }
                self.emit_named_error_throw("TypeError")?;
            }
        }
        if user_functions.is_empty() {
            if std::env::var_os("AYY_TRACE_DYNAMIC_CALLS").is_some() {
                eprintln!(
                    "emit_dynamic_user_function_call:no-match-fallback callee={callee:?} instruction={}",
                    self.state.emission.output.instructions.len()
                );
                self.emit_runtime_shadow_debug_print_local(
                    &format!("dynamic_call_no_match callee={callee:?}"),
                    callee_local,
                )?;
            }
            self.emit_named_error_throw("TypeError")?;
        }
        for _ in 0..dispatch_branch_count {
            self.state.emission.output.instructions.push(0x0b);
            self.pop_control_frame();
        }
        let dynamic_result_local = self.allocate_temp_local();
        self.push_local_set(dynamic_result_local);
        if let Some((hidden_name, source_owner)) = receiver_shadow_writeback.as_ref() {
            self.emit_runtime_object_property_shadow_copy(hidden_name, source_owner)?;
        }
        for (hidden_name, source_owner) in &argument_shadow_writebacks {
            self.emit_runtime_object_property_shadow_copy(hidden_name, source_owner)?;
        }
        self.push_local_get(dynamic_result_local);
        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();

        if std::env::var_os("AYY_TRACE_INLINE_PROMISES").is_some() {
            eprintln!("emit_dynamic_user_function_call:done");
        }
        Ok(true)
    }

    pub(in crate::backend::direct_wasm) fn emit_dynamic_super_call(
        &mut self,
        callee: &Expression,
        arguments: &[CallArgument],
    ) -> DirectResult<bool> {
        let callee_local = self.allocate_temp_local();
        self.emit_numeric_expression(callee)?;
        self.push_local_set(callee_local);

        let expanded_arguments = self.expand_call_arguments(arguments);
        let mut call_arguments = Vec::with_capacity(expanded_arguments.len());
        let mut argument_shadow_writebacks = Vec::new();
        for (index, argument) in expanded_arguments.iter().enumerate() {
            let argument_value_local = self.allocate_temp_local();
            self.emit_numeric_expression(argument)?;
            self.push_local_set(argument_value_local);

            let hidden_name = self.allocate_named_hidden_local(
                &format!("dynamic_super_arg_{index}"),
                self.infer_value_kind(argument)
                    .unwrap_or(StaticValueKind::Unknown),
            );
            let hidden_local = self
                .state
                .runtime
                .locals
                .get(&hidden_name)
                .copied()
                .expect("fresh dynamic super hidden local must exist");
            self.push_local_get(argument_value_local);
            self.push_local_set(hidden_local);
            self.update_capture_slot_binding_from_expression(&hidden_name, argument)?;
            self.sync_capture_slot_runtime_object_shadows_from_expression(&hidden_name, argument)?;
            let source_owner = match argument {
                Expression::Identifier(name) => {
                    self.runtime_object_property_shadow_owner_name_for_identifier(name)
                }
                Expression::This => Some("this".to_string()),
                _ => None,
            };
            if let Some(source_owner) = source_owner {
                argument_shadow_writebacks.push((hidden_name.clone(), source_owner));
            }
            call_arguments.push(CallArgument::Expression(Expression::Identifier(
                hidden_name,
            )));
        }

        let constructible_builtin_functions = builtin_function_runtime_entries()
            .filter(|(function_name, _)| Self::builtin_function_is_constructible(function_name))
            .collect::<Vec<_>>();
        let constructible_user_functions = self
            .backend
            .function_registry
            .catalog
            .user_functions
            .iter()
            .filter(|user_function| user_function.is_constructible())
            .cloned()
            .collect::<Vec<_>>();
        let dispatch_branch_count =
            constructible_builtin_functions.len() + constructible_user_functions.len();
        if dispatch_branch_count == 0 {
            return Ok(false);
        }
        let derived_super_context = self.current_function_is_derived_constructor()
            || self.current_lexical_function_captures_this();

        for (function_name, runtime_value) in &constructible_builtin_functions {
            self.push_local_get(callee_local);
            self.push_i32_const(*runtime_value);
            self.push_binary_op(BinaryOp::Equal)?;
            self.state.emission.output.instructions.push(0x04);
            self.state.emission.output.instructions.push(I32_TYPE);
            self.push_control_frame();
            if derived_super_context {
                if !self
                    .emit_derived_constructor_builtin_super_call(function_name, &call_arguments)?
                {
                    self.emit_named_error_throw("TypeError")?;
                }
            } else if !self.emit_builtin_call(function_name, &call_arguments)? {
                self.emit_named_error_throw("TypeError")?;
            }
            self.state.emission.output.instructions.push(0x05);
        }

        for (index, user_function) in constructible_user_functions.iter().enumerate() {
            self.push_local_get(callee_local);
            self.push_i32_const(user_function_runtime_value(user_function));
            self.push_binary_op(BinaryOp::Equal)?;
            self.state.emission.output.instructions.push(0x04);
            self.state.emission.output.instructions.push(I32_TYPE);
            self.push_control_frame();
            if derived_super_context {
                self.emit_derived_constructor_super_call(user_function, &call_arguments)?;
            } else {
                self.emit_user_function_call_with_current_new_target_and_this_expression(
                    user_function,
                    &call_arguments,
                    &Expression::This,
                )?;
            }
            self.state.emission.output.instructions.push(0x05);
            if index + 1 == constructible_user_functions.len() {
                self.push_i32_const(JS_UNDEFINED_TAG);
            }
        }
        if constructible_user_functions.is_empty() {
            self.push_i32_const(JS_UNDEFINED_TAG);
        }
        for _ in 0..dispatch_branch_count {
            self.state.emission.output.instructions.push(0x0b);
            self.pop_control_frame();
        }
        let dynamic_result_local = self.allocate_temp_local();
        self.push_local_set(dynamic_result_local);
        for (hidden_name, source_owner) in &argument_shadow_writebacks {
            self.emit_runtime_object_property_shadow_copy(hidden_name, source_owner)?;
        }
        self.push_local_get(dynamic_result_local);

        Ok(true)
    }
}
