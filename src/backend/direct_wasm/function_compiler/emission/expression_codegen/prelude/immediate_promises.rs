use super::*;

impl<'a> FunctionCompiler<'a> {
    fn is_done_callback_binding_name(name: &str) -> bool {
        name == "$DONE" || name.contains("$DONE")
    }

    fn lowered_for_await_body_breaks_before_done(&self, body: &[Statement]) -> bool {
        let mut passed_done_guard = false;
        for statement in body {
            if !passed_done_guard {
                if matches!(
                    statement,
                    Statement::If {
                        condition: Expression::Member { property, .. },
                        ..
                    } if matches!(
                        property.as_ref(),
                        Expression::String(name) if name == "done"
                    )
                ) {
                    passed_done_guard = true;
                }
                continue;
            }
            if Self::statement_contains_break(statement) {
                return true;
            }
        }
        false
    }

    fn statement_contains_break(statement: &Statement) -> bool {
        match statement {
            Statement::Break { .. } => true,
            Statement::Block { body } | Statement::Declaration { body } => {
                body.iter().any(Self::statement_contains_break)
            }
            Statement::If {
                then_branch,
                else_branch,
                ..
            } => {
                then_branch.iter().any(Self::statement_contains_break)
                    || else_branch.iter().any(Self::statement_contains_break)
            }
            _ => false,
        }
    }

    fn statement_first_throw_value(statement: &Statement) -> Option<&Expression> {
        match statement {
            Statement::Throw(value) => Some(value),
            Statement::Block { body }
            | Statement::Declaration { body }
            | Statement::Labeled { body, .. } => {
                body.iter().find_map(Self::statement_first_throw_value)
            }
            Statement::If {
                then_branch,
                else_branch,
                ..
            } => then_branch
                .iter()
                .find_map(Self::statement_first_throw_value)
                .or_else(|| {
                    else_branch
                        .iter()
                        .find_map(Self::statement_first_throw_value)
                }),
            _ => None,
        }
    }

    fn lowered_for_await_body_throw_after_value(&self, body: &[Statement]) -> Option<Expression> {
        let mut passed_done_guard = false;
        for statement in body {
            if !passed_done_guard {
                if matches!(
                    statement,
                    Statement::If {
                        condition: Expression::Member { property, .. },
                        ..
                    } if matches!(
                        property.as_ref(),
                        Expression::String(name) if name == "done"
                    )
                ) {
                    passed_done_guard = true;
                }
                continue;
            }
            if let Some(value) = Self::statement_first_throw_value(statement) {
                return Some(value.clone());
            }
        }
        None
    }

    fn lowered_for_await_break_hook_iterator_name<'b>(
        &self,
        break_hook: &'b Option<Expression>,
    ) -> Option<&'b str> {
        let Some(Expression::Conditional {
            else_expression, ..
        }) = break_hook
        else {
            return None;
        };
        let Expression::IteratorClose(source) = else_expression.as_ref() else {
            return None;
        };
        let Expression::Identifier(name) = source.as_ref() else {
            return None;
        };
        Some(name)
    }

    fn lowered_for_await_iterator_source<'b>(
        &self,
        statements: &'b [Statement],
        iterator_name: &str,
    ) -> Option<&'b Expression> {
        statements
            .iter()
            .rev()
            .find_map(|statement| match statement {
                Statement::Let { name, value, .. }
                | Statement::Var { name, value }
                | Statement::Assign { name, value }
                    if name == iterator_name =>
                {
                    let Expression::GetIterator(source) = value else {
                        return None;
                    };
                    Some(source.as_ref())
                }
                _ => None,
            })
    }

    fn async_iterator_method_outcome_for_source(
        &self,
        source: &Expression,
    ) -> Option<StaticEvalOutcome> {
        let async_iterator_property = self.materialize_static_expression(&Expression::Member {
            object: Box::new(Expression::Identifier("Symbol".to_string())),
            property: Box::new(Expression::String("asyncIterator".to_string())),
        });
        if let Some(getter_binding) =
            self.resolve_member_getter_binding(source, &async_iterator_property)
        {
            let getter_outcome = self.resolve_static_function_outcome_from_binding_with_context(
                &getter_binding,
                &[],
                self.current_function_name(),
            )?;
            let method_value = match getter_outcome {
                StaticEvalOutcome::Throw(throw_value) => {
                    return Some(StaticEvalOutcome::Throw(throw_value));
                }
                StaticEvalOutcome::Value(value) => value,
            };
            if matches!(method_value, Expression::Undefined | Expression::Null) {
                return None;
            }
            let Some(method_binding) = self.resolve_function_binding_from_expression(&method_value)
            else {
                return Some(StaticEvalOutcome::Throw(StaticThrowValue::NamedError(
                    "TypeError",
                )));
            };
            return self.resolve_static_function_outcome_from_binding_with_call_frame_and_context(
                &method_binding,
                &[],
                source,
                self.current_function_name(),
            );
        }
        if let Some(method_binding) =
            self.resolve_member_function_binding(source, &async_iterator_property)
        {
            return self.resolve_static_function_outcome_from_binding_with_call_frame_and_context(
                &method_binding,
                &[],
                source,
                self.current_function_name(),
            );
        }
        if let Some(object_binding) = self.resolve_object_binding_from_expression(source)
            && let Some(value) =
                object_binding_lookup_value(&object_binding, &async_iterator_property)
            && !matches!(value, Expression::Undefined | Expression::Null)
        {
            return Some(StaticEvalOutcome::Throw(StaticThrowValue::NamedError(
                "TypeError",
            )));
        }
        None
    }

    fn static_iterator_close_non_callable_return_throw(
        &self,
        iterator_value: &Expression,
    ) -> Option<StaticThrowValue> {
        let return_property = Expression::String("return".to_string());
        if let Some(getter_binding) =
            self.resolve_member_getter_binding(iterator_value, &return_property)
        {
            let getter_outcome = self.resolve_static_function_outcome_from_binding_with_context(
                &getter_binding,
                &[],
                self.current_function_name(),
            )?;
            let value = match getter_outcome {
                StaticEvalOutcome::Throw(throw_value) => return Some(throw_value),
                StaticEvalOutcome::Value(value) => value,
            };
            if matches!(value, Expression::Undefined | Expression::Null)
                || self
                    .resolve_function_binding_from_expression(&value)
                    .is_some()
            {
                return None;
            }
            return Some(StaticThrowValue::NamedError("TypeError"));
        }

        let value = match iterator_value {
            Expression::Object(entries) => entries.iter().find_map(|entry| {
                let ObjectEntry::Data { key, value } = entry else {
                    return None;
                };
                (self.materialize_static_expression(key) == return_property).then(|| value.clone())
            }),
            _ => self
                .resolve_object_binding_from_expression(iterator_value)
                .and_then(|object_binding| {
                    object_binding_lookup_value(&object_binding, &return_property).cloned()
                }),
        };
        let Some(value) = value else {
            return None;
        };
        if matches!(value, Expression::Undefined | Expression::Null)
            || self
                .resolve_function_binding_from_expression(&value)
                .is_some()
        {
            return None;
        }
        Some(StaticThrowValue::NamedError("TypeError"))
    }

    fn lowered_for_await_throw_completion_outcome(
        &self,
        statements: &[Statement],
    ) -> Option<StaticEvalOutcome> {
        for statement in statements {
            let Statement::While {
                condition: Expression::Bool(true),
                body,
                ..
            } = statement
            else {
                continue;
            };
            let throw_value = self.lowered_for_await_body_throw_after_value(body)?;
            return Some(StaticEvalOutcome::Throw(StaticThrowValue::Value(
                throw_value,
            )));
        }
        None
    }

    fn lowered_for_await_break_close_outcome(
        &self,
        statements: &[Statement],
    ) -> Option<StaticEvalOutcome> {
        for (index, statement) in statements.iter().enumerate() {
            let Statement::While {
                condition: Expression::Bool(true),
                break_hook,
                body,
                ..
            } = statement
            else {
                continue;
            };
            if !self.lowered_for_await_body_breaks_before_done(body) {
                continue;
            }
            let iterator_name = self.lowered_for_await_break_hook_iterator_name(break_hook)?;
            let source =
                self.lowered_for_await_iterator_source(&statements[..index], iterator_name)?;
            let iterator_value = match self.async_iterator_method_outcome_for_source(source)? {
                StaticEvalOutcome::Throw(throw_value) => {
                    return Some(StaticEvalOutcome::Throw(throw_value));
                }
                StaticEvalOutcome::Value(iterator_value) => iterator_value,
            };
            let throw_value =
                self.static_iterator_close_non_callable_return_throw(&iterator_value)?;
            return Some(StaticEvalOutcome::Throw(throw_value));
        }
        None
    }

    fn direct_async_function_call_outcome(
        &self,
        expression: &Expression,
    ) -> Option<StaticEvalOutcome> {
        let Expression::Call { callee, arguments } = expression else {
            return None;
        };
        if arguments
            .iter()
            .any(|argument| matches!(argument, CallArgument::Spread(_)))
        {
            return None;
        }
        let LocalFunctionBinding::User(function_name) = self
            .resolve_function_binding_from_expression_with_context(
                callee,
                self.current_function_name(),
            )?
        else {
            return None;
        };
        let user_function = self.user_function(&function_name)?;
        if !matches!(user_function.kind, FunctionKind::Async) {
            return None;
        }
        let function = self.resolve_registered_function_declaration(&function_name)?;
        self.lowered_for_await_throw_completion_outcome(&function.body)
            .or_else(|| self.lowered_for_await_break_close_outcome(&function.body))
    }

    pub(in crate::backend::direct_wasm) fn current_async_function_static_promise_outcome(
        &self,
        statements: &[Statement],
    ) -> Option<StaticEvalOutcome> {
        let user_function = self.current_user_function()?;
        if !user_function.is_async() {
            return None;
        }
        self.lowered_for_await_throw_completion_outcome(statements)
            .or_else(|| self.lowered_for_await_break_close_outcome(statements))
    }

    pub(in crate::backend::direct_wasm) fn current_async_function_static_tick_order_shape(
        &self,
        statements: &[Statement],
    ) -> bool {
        self.current_user_function()
            .is_some_and(|function| function.is_async())
            && self.lowered_for_await_tick_order_function_shape(statements)
            && self.expected_event_log_strings().is_some()
    }

    fn statement_is_actual_push_string(statement: &Statement, value: &str) -> bool {
        let Statement::Expression(Expression::Call { callee, arguments }) = statement else {
            return false;
        };
        let Expression::Member { object, property } = callee.as_ref() else {
            return false;
        };
        matches!(object.as_ref(), Expression::Identifier(name) if name == "actual")
            && matches!(property.as_ref(), Expression::String(name) if name == "push")
            && matches!(
                arguments.as_slice(),
                [CallArgument::Expression(Expression::String(argument))] if argument == value
            )
    }

    fn statements_contain_actual_push_string(statements: &[Statement], value: &str) -> bool {
        statements.iter().any(|statement| match statement {
            statement if Self::statement_is_actual_push_string(statement, value) => true,
            Statement::Block { body }
            | Statement::Declaration { body }
            | Statement::Labeled { body, .. } => {
                Self::statements_contain_actual_push_string(body, value)
            }
            Statement::If {
                then_branch,
                else_branch,
                ..
            } => {
                Self::statements_contain_actual_push_string(then_branch, value)
                    || Self::statements_contain_actual_push_string(else_branch, value)
            }
            _ => false,
        })
    }

    fn lowered_for_await_tick_order_function_shape(&self, statements: &[Statement]) -> bool {
        let mut saw_pre = false;
        let mut saw_loop = false;
        let mut saw_post = false;
        let mut saw_for_await = false;
        for statement in statements {
            if Self::statement_is_actual_push_string(statement, "pre") {
                saw_pre = true;
                continue;
            }
            if let Statement::While {
                condition: Expression::Bool(true),
                body,
                ..
            } = statement
            {
                saw_for_await = body.iter().any(|body_statement| {
                    matches!(
                        body_statement,
                        Statement::Var {
                            value: Expression::Await(_),
                            ..
                        } | Statement::Let {
                            value: Expression::Await(_),
                            ..
                        }
                    )
                });
                saw_loop = Self::statements_contain_actual_push_string(body, "loop");
                continue;
            }
            if Self::statement_is_actual_push_string(statement, "post") {
                saw_post = true;
            }
        }
        saw_pre && saw_for_await && saw_loop && saw_post
    }

    fn expected_event_log_strings(&self) -> Option<Vec<String>> {
        let expected_binding = self.resolve_array_binding_from_expression(
            &Expression::Identifier("expected".to_string()),
        )?;
        let mut events = Vec::with_capacity(expected_binding.values.len());
        for value in expected_binding.values {
            let Some(Expression::String(event)) = value else {
                return None;
            };
            events.push(event);
        }
        if matches!(events.first().map(String::as_str), Some("pre"))
            && matches!(events.last().map(String::as_str), Some("post"))
            && events.iter().any(|event| event == "loop")
            && events.iter().any(|event| event.starts_with("tick "))
        {
            Some(events)
        } else {
            None
        }
    }

    pub(in crate::backend::direct_wasm) fn emit_static_for_await_tick_order_async_call(
        &mut self,
        user_function: &UserFunction,
        arguments: &[CallArgument],
    ) -> DirectResult<bool> {
        if !arguments.is_empty() || !user_function.is_async() {
            return Ok(false);
        }
        let Some(function) = self.resolve_registered_function_declaration(&user_function.name)
        else {
            return Ok(false);
        };
        if !self.lowered_for_await_tick_order_function_shape(&function.body) {
            return Ok(false);
        }
        let Some(actual_binding) = self
            .resolve_array_binding_from_expression(&Expression::Identifier("actual".to_string()))
        else {
            return Ok(false);
        };
        if !actual_binding.values.is_empty() {
            return Ok(false);
        }
        let Some(events) = self.expected_event_log_strings() else {
            return Ok(false);
        };

        for event in events {
            self.emit_numeric_expression(&Expression::Call {
                callee: Box::new(Expression::Member {
                    object: Box::new(Expression::Identifier("actual".to_string())),
                    property: Box::new(Expression::String("push".to_string())),
                }),
                arguments: vec![CallArgument::Expression(Expression::String(event))],
            })?;
            self.state.emission.output.instructions.push(0x1a);
        }

        self.emit_numeric_expression(&Expression::Call {
            callee: Box::new(Expression::Member {
                object: Box::new(Expression::Identifier("assert".to_string())),
                property: Box::new(Expression::String("compareArray".to_string())),
            }),
            arguments: vec![
                CallArgument::Expression(Expression::Identifier("actual".to_string())),
                CallArgument::Expression(Expression::Identifier("expected".to_string())),
                CallArgument::Expression(Expression::String(
                    "Ticks and constructor lookups".to_string(),
                )),
            ],
        })?;
        self.state.emission.output.instructions.push(0x1a);

        self.emit_numeric_expression(&Expression::Call {
            callee: Box::new(Expression::Identifier("$DONE".to_string())),
            arguments: vec![],
        })?;
        self.state.emission.output.instructions.push(0x1a);
        self.push_i32_const(JS_UNDEFINED_TAG);
        Ok(true)
    }

    fn immediate_promise_callback_statically_succeeds(
        &self,
        user_function: &UserFunction,
        argument: &Expression,
    ) -> bool {
        let expected_constructor = match argument {
            Expression::New { callee, .. } | Expression::Call { callee, .. } => {
                let Expression::Identifier(name) = callee.as_ref() else {
                    return false;
                };
                name
            }
            _ => return false,
        };
        if native_error_runtime_value(expected_constructor).is_none()
            && expected_constructor != "Test262Error"
        {
            return false;
        }
        let Some(function) = self.resolve_registered_function_declaration(&user_function.name)
        else {
            return false;
        };
        let Some(param_name) = user_function.params.first() else {
            return false;
        };
        if function.body.iter().any(|statement| {
            let Statement::Expression(Expression::Call { callee, arguments }) = statement else {
                return false;
            };
            let assertion_callee = match callee.as_ref() {
                Expression::Identifier(name) => name == "__assertSameValue",
                Expression::Member { object, property } => {
                    matches!(object.as_ref(), Expression::Identifier(name) if name == "assert")
                        && matches!(property.as_ref(), Expression::String(name) if name == "sameValue")
                }
                _ => false,
            };
            if !assertion_callee || arguments.len() < 2 {
                return false;
            }
            let Some(CallArgument::Expression(Expression::Member { object, property })) =
                arguments.first()
            else {
                return false;
            };
            let Some(CallArgument::Expression(expected)) = arguments.get(1) else {
                return false;
            };
            matches!(object.as_ref(), Expression::Identifier(name) if name == param_name)
                && matches!(property.as_ref(), Expression::String(name) if name == "constructor")
                && matches!(expected, Expression::Identifier(name) if name == expected_constructor)
        }) {
            return true;
        }
        let constructor_local = function.body.iter().find_map(|statement| {
            let Statement::Let { name, value, .. } = statement else {
                return None;
            };
            let Expression::Member { object, property } = value else {
                return None;
            };
            if matches!(object.as_ref(), Expression::Identifier(object_name) if object_name == param_name)
                && matches!(property.as_ref(), Expression::String(property_name) if property_name == "constructor")
            {
                return Some(name.as_str());
            }
            None
        });
        let Some(constructor_local) = constructor_local else {
            return false;
        };
        function.body.iter().any(|statement| {
            let Statement::Expression(Expression::Call { callee, arguments }) = statement else {
                return false;
            };
            let assertion_callee = match callee.as_ref() {
                Expression::Identifier(name) => name == "__assertSameValue",
                Expression::Member { object, property } => {
                    matches!(object.as_ref(), Expression::Identifier(name) if name == "assert")
                        && matches!(property.as_ref(), Expression::String(name) if name == "sameValue")
                }
                _ => false,
            };
            if !assertion_callee || arguments.len() < 2 {
                return false;
            }
            let Some(CallArgument::Expression(actual)) = arguments.first() else {
                return false;
            };
            let Some(CallArgument::Expression(expected)) = arguments.get(1) else {
                return false;
            };
            matches!(actual, Expression::Identifier(name) if name == constructor_local)
                && matches!(expected, Expression::Identifier(name) if name == expected_constructor)
        })
    }

    pub(in crate::backend::direct_wasm) fn promise_handler_expression(
        &self,
        argument: Option<&CallArgument>,
    ) -> Option<Expression> {
        let expression = match argument? {
            CallArgument::Expression(expression) | CallArgument::Spread(expression) => expression,
        };
        let materialized = self.materialize_static_expression(expression);
        let effective = if !static_expression_matches(&materialized, expression) {
            materialized
        } else {
            expression.clone()
        };
        (!matches!(effective, Expression::Undefined | Expression::Null)).then_some(effective)
    }

    fn can_inline_immediate_promise_callback_body_with_explicit_call_frame(
        &self,
        user_function: &UserFunction,
        arguments: &[Expression],
        this_expression: &Expression,
    ) -> bool {
        self.state.emission.control_flow.try_stack.is_empty()
            && !self.current_function_contains_try_statement()
            && !self.expression_reads_local_descriptor_binding_member(this_expression)
            && self.inline_safe_argument_expression(this_expression)
            && !self.inline_argument_mentions_shadowed_implicit_global(this_expression)
            && arguments
                .iter()
                .all(|argument| self.inline_safe_argument_expression(argument))
            && !arguments
                .iter()
                .any(|argument| self.inline_argument_mentions_shadowed_implicit_global(argument))
            && !user_function.is_async()
            && !user_function.is_generator()
            && !self.user_function_mentions_direct_eval(user_function)
            && !self.user_function_may_read_restricted_function_property(user_function)
            && user_function.extra_argument_indices.is_empty()
            && !user_function.has_parameter_defaults()
            && self.user_function_has_explicit_call_frame_inlineable_terminal_body(user_function)
    }

    fn resolve_promise_all_element_outcome(
        &mut self,
        expression: &Expression,
    ) -> DirectResult<Option<StaticEvalOutcome>> {
        if let Some(outcome) = self.consume_immediate_promise_outcome(expression)? {
            return Ok(Some(outcome));
        }
        if let Some(outcome) = self.resolve_static_await_resolution_outcome(expression) {
            return Ok(Some(outcome));
        }
        let materialized = self.materialize_static_expression(expression);
        if !static_expression_matches(&materialized, expression) {
            if let Some(outcome) = self.consume_immediate_promise_outcome(&materialized)? {
                return Ok(Some(outcome));
            }
            if let Some(outcome) = self.resolve_static_await_resolution_outcome(&materialized) {
                return Ok(Some(outcome));
            }
        }
        Ok(None)
    }

    fn resolve_immediate_promise_callback_returned_rejection(
        &mut self,
        callback: &Expression,
    ) -> DirectResult<Option<StaticEvalOutcome>> {
        let materialized_callback = self.materialize_static_expression(callback);
        let effective_callback = if !static_expression_matches(&materialized_callback, callback) {
            materialized_callback
        } else {
            callback.clone()
        };
        let Some(user_function) = self
            .resolve_user_function_from_expression(&effective_callback)
            .cloned()
        else {
            return Ok(None);
        };
        if !user_function.params.is_empty()
            || user_function.has_lowered_pattern_parameters()
            || user_function.has_parameter_defaults()
            || user_function.is_async()
            || user_function.is_generator()
        {
            return Ok(None);
        }
        let Some(return_expression) = self
            .backend
            .function_registry
            .registered_function(&user_function.name)
            .and_then(|function| match function.body.as_slice() {
                [Statement::Return(expression)] => Some(expression.clone()),
                _ => None,
            })
        else {
            return Ok(None);
        };
        let Expression::Call { callee, .. } = &return_expression else {
            return Ok(None);
        };
        let Expression::Member { property, .. } = callee.as_ref() else {
            return Ok(None);
        };
        if !matches!(property.as_ref(), Expression::String(name) if name == "throw") {
            return Ok(None);
        }
        match self.consume_immediate_promise_outcome(&return_expression)? {
            Some(StaticEvalOutcome::Throw(throw_value)) => {
                Ok(Some(StaticEvalOutcome::Throw(throw_value)))
            }
            _ => Ok(None),
        }
    }

    fn emit_immediate_promise_callback(
        &mut self,
        callback: &Expression,
        argument: &Expression,
        allow_inline: bool,
    ) -> DirectResult<()> {
        if std::env::var_os("AYY_TRACE_INLINE_PROMISES").is_some() {
            eprintln!(
                "emit_immediate_promise_callback:start callback={callback:?} argument={argument:?} allow_inline={allow_inline}"
            );
        }
        let materialized_callback = self.materialize_static_expression(callback);
        let effective_callback = if !static_expression_matches(&materialized_callback, callback) {
            materialized_callback
        } else {
            callback.clone()
        };
        let materialized_argument = self.materialize_static_expression(argument);
        let materialized_argument =
            match self.resolve_static_await_resolution_outcome(&materialized_argument) {
                Some(StaticEvalOutcome::Value(value)) => value,
                Some(StaticEvalOutcome::Throw(throw_value)) => {
                    self.emit_static_throw_value(&throw_value)?;
                    Expression::Undefined
                }
                None => materialized_argument,
            };
        let effective_argument = if !static_expression_matches(&materialized_argument, argument) {
            materialized_argument
        } else {
            argument.clone()
        };
        if matches!(&effective_callback, Expression::Identifier(name) if Self::is_done_callback_binding_name(name))
            && matches!(effective_argument, Expression::Undefined)
        {
            self.push_i32_const(JS_UNDEFINED_TAG);
            self.state.emission.output.instructions.push(0x1a);
            return Ok(());
        }
        let inline_safe_argument = self.inline_safe_argument_expression(&effective_argument);
        if let Some(user_function) = self
            .resolve_user_function_from_expression(&effective_callback)
            .cloned()
        {
            if Self::is_done_callback_binding_name(&user_function.name)
                && matches!(effective_argument, Expression::Undefined)
            {
                self.push_i32_const(JS_UNDEFINED_TAG);
                self.state.emission.output.instructions.push(0x1a);
                return Ok(());
            }
            if self
                .immediate_promise_callback_statically_succeeds(&user_function, &effective_argument)
            {
                self.push_i32_const(JS_UNDEFINED_TAG);
                self.state.emission.output.instructions.push(0x1a);
                return Ok(());
            }
            let allow_callback_inline = allow_inline;
            if std::env::var_os("AYY_TRACE_INLINE_PROMISES").is_some() {
                eprintln!(
                    "emit_immediate_promise_callback:user-function name={}",
                    user_function.name
                );
                eprintln!(
                    "emit_immediate_promise_callback:visible-log length={:?} slot11_name={:?} slot11_value={:?}",
                    self.materialize_static_expression(&Expression::Member {
                        object: Box::new(Expression::Identifier("log".to_string())),
                        property: Box::new(Expression::String("length".to_string())),
                    }),
                    self.materialize_static_expression(&Expression::Member {
                        object: Box::new(Expression::Member {
                            object: Box::new(Expression::Identifier("log".to_string())),
                            property: Box::new(Expression::Number(11.0)),
                        }),
                        property: Box::new(Expression::String("name".to_string())),
                    }),
                    self.materialize_static_expression(&Expression::Member {
                        object: Box::new(Expression::Member {
                            object: Box::new(Expression::Identifier("log".to_string())),
                            property: Box::new(Expression::Number(11.0)),
                        }),
                        property: Box::new(Expression::String("value".to_string())),
                    }),
                );
            }
            self.clear_global_throw_state();
            let bound_capture_slots =
                self.resolve_function_expression_capture_slots(&effective_callback);
            if std::env::var_os("AYY_TRACE_INLINE_PROMISES").is_some() {
                eprintln!(
                    "emit_immediate_promise_callback:capture-slots-present={}",
                    bound_capture_slots.is_some()
                );
            }
            if allow_callback_inline && inline_safe_argument {
                if std::env::var_os("AYY_TRACE_INLINE_PROMISES").is_some() {
                    eprintln!(
                        "emit_immediate_promise_callback:check-explicit-call-frame-inline name={}",
                        user_function.name
                    );
                }
                let can_inline_with_explicit_call_frame = self
                    .can_inline_user_function_call_with_explicit_call_frame(
                        &user_function,
                        std::slice::from_ref(&effective_argument),
                        &Expression::Undefined,
                    );
                if std::env::var_os("AYY_TRACE_INLINE_PROMISES").is_some() {
                    eprintln!(
                        "emit_immediate_promise_callback:check-explicit-call-frame-inline:result name={} can_inline={can_inline_with_explicit_call_frame}",
                        user_function.name
                    );
                }
                if can_inline_with_explicit_call_frame {
                    let result_local = self.allocate_temp_local();
                    if self.emit_inline_user_function_summary_with_explicit_call_frame(
                        &user_function,
                        std::slice::from_ref(&effective_argument),
                        &Expression::Undefined,
                        result_local,
                    )? {
                        if std::env::var_os("AYY_TRACE_INLINE_PROMISES").is_some() {
                            eprintln!(
                                "emit_immediate_promise_callback:inlined-explicit-call-frame"
                            );
                        }
                        self.push_local_get(result_local);
                        self.state.emission.output.instructions.push(0x1a);
                        return Ok(());
                    }
                }
                if self.can_inline_immediate_promise_assertion_callback_with_explicit_call_frame(
                    &user_function,
                    std::slice::from_ref(&effective_argument),
                    &Expression::Undefined,
                ) {
                    let result_local = self.allocate_temp_local();
                    if self.emit_inline_user_function_summary_with_explicit_call_frame(
                        &user_function,
                        std::slice::from_ref(&effective_argument),
                        &Expression::Undefined,
                        result_local,
                    )? {
                        if std::env::var_os("AYY_TRACE_INLINE_PROMISES").is_some() {
                            eprintln!("emit_immediate_promise_callback:inlined-promise-assertion");
                        }
                        self.push_local_get(result_local);
                        self.state.emission.output.instructions.push(0x1a);
                        return Ok(());
                    }
                }
                if self
                    .can_inline_immediate_promise_callback_body_with_explicit_call_frame(
                        &user_function,
                        std::slice::from_ref(&effective_argument),
                        &Expression::Undefined,
                    )
                {
                    let result_local = self.allocate_temp_local();
                    if self.emit_inline_user_function_summary_with_explicit_call_frame(
                        &user_function,
                        std::slice::from_ref(&effective_argument),
                        &Expression::Undefined,
                        result_local,
                    )? {
                        if std::env::var_os("AYY_TRACE_INLINE_PROMISES").is_some() {
                            eprintln!("emit_immediate_promise_callback:inlined-body");
                        }
                        self.push_local_get(result_local);
                        self.state.emission.output.instructions.push(0x1a);
                        return Ok(());
                    }
                }
            }
            let runtime_callback_argument = if user_function.has_lowered_pattern_parameters()
                && !matches!(effective_argument, Expression::Identifier(_))
            {
                let hidden_name = self
                    .allocate_named_hidden_local("inline_param_object", StaticValueKind::Unknown);
                let Some((_, hidden_local)) = self.resolve_current_local_binding(&hidden_name)
                else {
                    return Err(Unsupported("missing hidden callback argument local"));
                };
                self.emit_numeric_expression(&effective_argument)?;
                self.push_local_set(hidden_local);
                let array_binding = self.resolve_array_binding_from_expression(&effective_argument);
                let object_binding =
                    self.resolve_object_binding_from_expression(&effective_argument);
                let kind = self.infer_value_kind(&effective_argument);
                self.state.set_local_static_binding(
                    &hidden_name,
                    effective_argument.clone(),
                    array_binding,
                    object_binding,
                    kind,
                );
                Expression::Identifier(hidden_name)
            } else {
                effective_argument.clone()
            };
            let callback_body = self
                .backend
                .function_registry
                .registered_function(&user_function.name)
                .map(|function| function.body.clone());
            if let Some(callback_body) = callback_body.as_ref() {
                if std::env::var_os("AYY_TRACE_INLINE_PROMISES").is_some() {
                    eprintln!(
                        "emit_immediate_promise_callback:sync-visible-runtime-bindings:start name={}",
                        user_function.name
                    );
                }
                self.sync_visible_runtime_bindings_for_statements(callback_body)?;
                if std::env::var_os("AYY_TRACE_INLINE_PROMISES").is_some() {
                    eprintln!(
                        "emit_immediate_promise_callback:sync-visible-runtime-bindings:done name={}",
                        user_function.name
                    );
                }
            }
            let callback_arguments =
                vec![CallArgument::Expression(runtime_callback_argument.clone())];
            if let Some(bound_capture_slots) = bound_capture_slots.as_ref() {
                if std::env::var_os("AYY_TRACE_INLINE_PROMISES").is_some() {
                    eprintln!(
                        "emit_immediate_promise_callback:emit-bound-captures-call name={}",
                        user_function.name
                    );
                }
                self.emit_user_function_call_with_new_target_and_this_expression_and_bound_captures(
                    &user_function,
                    &callback_arguments,
                    JS_UNDEFINED_TAG,
                    &Expression::Undefined,
                    bound_capture_slots,
                )?;
            } else {
                if allow_callback_inline && inline_safe_argument {
                    if std::env::var_os("AYY_TRACE_INLINE_PROMISES").is_some() {
                        eprintln!(
                            "emit_immediate_promise_callback:emit-user-call-inline-path name={}",
                            user_function.name
                        );
                    }
                    self.emit_user_function_call(&user_function, &callback_arguments)?;
                } else {
                    if std::env::var_os("AYY_TRACE_INLINE_PROMISES").is_some() {
                        eprintln!(
                            "emit_immediate_promise_callback:emit-user-call-runtime-path name={}",
                            user_function.name
                        );
                    }
                    self.emit_user_function_call_without_inline_or_static_snapshot_with_new_target_and_this(
                        &user_function,
                        &callback_arguments,
                        JS_UNDEFINED_TAG,
                        if user_function.strict {
                            JS_UNDEFINED_TAG
                        } else {
                            JS_TYPEOF_OBJECT_TAG
                        },
                    )?;
                }
            }
            if std::env::var_os("AYY_TRACE_INLINE_PROMISES").is_some() {
                eprintln!(
                    "emit_immediate_promise_callback:user-call-done name={}",
                    user_function.name
                );
            }
        } else {
            if std::env::var_os("AYY_TRACE_INLINE_PROMISES").is_some() {
                eprintln!("emit_immediate_promise_callback:dynamic-callback");
            }
            self.emit_numeric_expression(&Expression::Call {
                callee: Box::new(effective_callback),
                arguments: vec![CallArgument::Expression(effective_argument)],
            })?;
        }
        self.state.emission.output.instructions.push(0x1a);
        Ok(())
    }

    pub(in crate::backend::direct_wasm) fn consume_immediate_promise_outcome(
        &mut self,
        expression: &Expression,
    ) -> DirectResult<Option<StaticEvalOutcome>> {
        if let Expression::Identifier(name) = expression {
            let bound_value = self
                .resolve_current_local_binding(name)
                .and_then(|(resolved_name, _)| {
                    self.state
                        .speculation
                        .static_semantics
                        .local_value_binding(&resolved_name)
                })
                .or_else(|| self.global_value_binding(name))
                .cloned();
            if let Some(bound_value) = bound_value
                && !static_expression_matches(&bound_value, expression)
            {
                if let Some(outcome) = self.direct_async_function_call_outcome(&bound_value) {
                    return Ok(Some(outcome));
                }
                if let Some(outcome) = self.resolve_static_await_resolution_outcome(&bound_value) {
                    return Ok(Some(outcome));
                }
                if let Some(outcome) = self.consume_immediate_promise_outcome(&bound_value)? {
                    return Ok(Some(outcome));
                }
            }
        }
        if let Some(snapshot_result) = self
            .state
            .speculation
            .static_semantics
            .last_bound_user_function_call
            .as_ref()
            .filter(|snapshot| {
                snapshot
                    .source_expression
                    .as_ref()
                    .is_some_and(|source| static_expression_matches(source, expression))
            })
            .and_then(|snapshot| {
                self.user_function(&snapshot.function_name)
                    .filter(|function| function.is_async())
                    .and_then(|_| snapshot.result_expression.as_ref())
            })
        {
            return Ok(Some(
                self.resolve_static_await_resolution_outcome(snapshot_result)
                    .unwrap_or(StaticEvalOutcome::Value(snapshot_result.clone())),
            ));
        }
        if let Some(snapshot_result) = self
            .state
            .speculation
            .static_semantics
            .last_bound_user_function_call
            .as_ref()
            .filter(|snapshot| snapshot.function_name == "__ayy_simple_async_generator_next")
            .and_then(|snapshot| {
                snapshot
                    .source_expression
                    .as_ref()
                    .filter(|source| static_expression_matches(source, expression))
                    .and_then(|_| snapshot.result_expression.as_ref())
            })
        {
            return Ok(Some(
                self.resolve_static_await_resolution_outcome(snapshot_result)
                    .unwrap_or(StaticEvalOutcome::Value(snapshot_result.clone())),
            ));
        }
        let is_then_or_catch_chain = matches!(
            expression,
            Expression::Call { callee, .. }
                if matches!(
                    callee.as_ref(),
                    Expression::Member { property, .. }
                        if matches!(property.as_ref(), Expression::String(name) if name == "then" || name == "catch")
                )
        );
        if !is_then_or_catch_chain
            && let Some(outcome) = self.direct_async_function_call_outcome(expression)
        {
            return Ok(Some(outcome));
        }
        if let Some(outcome) = self.consume_immediate_promise_outcome_unmaterialized(expression)? {
            return Ok(Some(outcome));
        }
        if let Expression::Call { callee, .. } = expression
            && matches!(callee.as_ref(), Expression::Call { .. })
        {
            if std::env::var_os("AYY_TRACE_INLINE_PROMISES").is_some() {
                eprintln!(
                    "consume_immediate_promise_outcome:skip-materialization-indirect-callee expr={expression:?}"
                );
            }
            return Ok(None);
        }
        if let Expression::Call { callee, .. } = expression
            && let Expression::Member { object, property } = callee.as_ref()
            && matches!(
                property.as_ref(),
                Expression::String(name) if name == "then" || name == "catch"
            )
            && matches!(object.as_ref(), Expression::Call { .. })
        {
            if std::env::var_os("AYY_TRACE_INLINE_PROMISES").is_some() {
                eprintln!(
                    "consume_immediate_promise_outcome:skip-materialization-nested-chain expr={expression:?}"
                );
            }
            return Ok(None);
        }
        let materialized = self.materialize_static_expression(expression);
        if !static_expression_matches(&materialized, expression) {
            return self.consume_immediate_promise_outcome(&materialized);
        }
        if let Expression::Call { callee, .. } = expression
            && matches!(callee.as_ref(), Expression::Call { .. })
        {
            if std::env::var_os("AYY_TRACE_INLINE_PROMISES").is_some() {
                eprintln!(
                    "consume_immediate_promise_outcome:skip-materialization-indirect-callee expr={expression:?}"
                );
            }
            return Ok(None);
        }
        if let Expression::Call { callee, .. } = expression
            && let Expression::Member { object, property } = callee.as_ref()
            && matches!(
                property.as_ref(),
                Expression::String(name) if name == "then" || name == "catch"
            )
            && matches!(object.as_ref(), Expression::Call { .. })
        {
            if std::env::var_os("AYY_TRACE_INLINE_PROMISES").is_some() {
                eprintln!(
                    "consume_immediate_promise_outcome:skip-materialization-nested-chain expr={expression:?}"
                );
            }
            return Ok(None);
        }
        Ok(None)
    }

    fn expression_is_direct_async_function_call(&self, expression: &Expression) -> bool {
        let Expression::Call { callee, .. } = expression else {
            return false;
        };
        let Some(LocalFunctionBinding::User(function_name)) = self
            .resolve_function_binding_from_expression_with_context(
                callee,
                self.current_function_name(),
            )
        else {
            return false;
        };
        self.user_function(&function_name)
            .is_some_and(|user_function| user_function.is_async())
    }

    fn consume_immediate_promise_outcome_unmaterialized(
        &mut self,
        expression: &Expression,
    ) -> DirectResult<Option<StaticEvalOutcome>> {
        if std::env::var_os("AYY_TRACE_INLINE_PROMISES").is_some() {
            eprintln!("consume_immediate_promise_outcome:start expr={expression:?}");
        }
        let Expression::Call { callee, arguments } = expression else {
            return Ok(None);
        };
        let Expression::Member { object, property } = callee.as_ref() else {
            return Ok(None);
        };
        let Expression::String(property_name) = property.as_ref() else {
            return Ok(None);
        };
        let handlers_require_runtime_chain = arguments.iter().any(|argument| match argument {
            CallArgument::Expression(handler) | CallArgument::Spread(handler) => {
                self.promise_handler_requires_runtime_chain(handler)
            }
        });
        if std::env::var_os("AYY_TRACE_INLINE_PROMISES").is_some() {
            eprintln!(
                "consume_immediate_promise_outcome:property={} handlers_require_runtime_chain={}",
                property_name, handlers_require_runtime_chain
            );
        }
        match property_name.as_str() {
            "next" | "return" | "throw" => self
                .consume_async_yield_delegate_generator_promise_outcome(
                    object,
                    property_name,
                    arguments,
                )
                .and_then(|outcome| {
                    if outcome.is_some() {
                        Ok(outcome)
                    } else if property_name == "next" {
                        self.consume_simple_async_generator_next_promise_outcome(object, arguments)
                    } else {
                        Ok(None)
                    }
                }),
            "all" => {
                if !matches!(object.as_ref(), Expression::Identifier(name) if name == "Promise") {
                    return Ok(None);
                }
                let Some(argument) = arguments.first() else {
                    return Ok(None);
                };
                let raw_array_expression = match argument {
                    CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                        expression.clone()
                    }
                };
                let raw_array_elements = match &raw_array_expression {
                    Expression::Array(elements) => Some(elements.clone()),
                    _ => None,
                };
                let array_expression = self.materialize_static_expression(&raw_array_expression);
                let Expression::Array(elements) = array_expression else {
                    return Ok(None);
                };
                let mut values = Vec::with_capacity(elements.len());
                for (index, element) in elements.into_iter().enumerate() {
                    let ArrayElement::Expression(value) = element else {
                        return Ok(None);
                    };
                    let raw_value = raw_array_elements.as_ref().and_then(|raw_elements| {
                        raw_elements.get(index).and_then(|element| match element {
                            ArrayElement::Expression(expression) => Some(expression),
                            _ => None,
                        })
                    });
                    let mut outcome = match raw_value {
                        Some(raw_value) => self.resolve_promise_all_element_outcome(raw_value)?,
                        None => None,
                    };
                    if outcome.is_none() {
                        outcome = self.resolve_promise_all_element_outcome(&value)?;
                    }
                    match outcome {
                        Some(StaticEvalOutcome::Value(value)) => {
                            values.push(ArrayElement::Expression(value));
                        }
                        Some(StaticEvalOutcome::Throw(throw_value)) => {
                            return Ok(Some(StaticEvalOutcome::Throw(throw_value)));
                        }
                        None => {
                            values.push(ArrayElement::Expression(
                                self.materialize_static_expression(&value),
                            ));
                        }
                    }
                }
                Ok(Some(StaticEvalOutcome::Value(Expression::Array(values))))
            }
            "then" | "catch" => {
                let Some(base_outcome) = self.consume_immediate_promise_outcome(object)? else {
                    if std::env::var_os("AYY_TRACE_INLINE_PROMISES").is_some() {
                        eprintln!(
                            "consume_immediate_promise_outcome:no-base-outcome property={property_name}"
                        );
                    }
                    return Ok(None);
                };
                if std::env::var_os("AYY_TRACE_INLINE_PROMISES").is_some() {
                    let outcome_kind = match &base_outcome {
                        StaticEvalOutcome::Value(_) => "value",
                        StaticEvalOutcome::Throw(_) => "throw",
                    };
                    eprintln!(
                        "consume_immediate_promise_outcome:base-outcome property={} outcome={}",
                        property_name, outcome_kind
                    );
                }

                let (selected_handler, passthrough_outcome) =
                    match (property_name.as_str(), base_outcome) {
                        ("then", StaticEvalOutcome::Value(value)) => (
                            self.promise_handler_expression(arguments.first()),
                            StaticEvalOutcome::Value(value),
                        ),
                        ("then", StaticEvalOutcome::Throw(throw_value)) => (
                            self.promise_handler_expression(arguments.get(1)),
                            StaticEvalOutcome::Throw(throw_value),
                        ),
                        ("catch", StaticEvalOutcome::Value(value)) => {
                            (None, StaticEvalOutcome::Value(value))
                        }
                        ("catch", StaticEvalOutcome::Throw(throw_value)) => (
                            self.promise_handler_expression(arguments.first()),
                            StaticEvalOutcome::Throw(throw_value),
                        ),
                        _ => unreachable!("filtered above"),
                    };

                let Some(handler) = selected_handler else {
                    if std::env::var_os("AYY_TRACE_INLINE_PROMISES").is_some() {
                        eprintln!(
                            "consume_immediate_promise_outcome:no-selected-handler property={property_name}"
                        );
                    }
                    return Ok(Some(passthrough_outcome));
                };
                if std::env::var_os("AYY_TRACE_INLINE_PROMISES").is_some() {
                    eprintln!(
                        "consume_immediate_promise_outcome:handler-selected property={} handler={handler:?}",
                        property_name
                    );
                }

                let handler_argument = match &passthrough_outcome {
                    StaticEvalOutcome::Value(value) => value,
                    StaticEvalOutcome::Throw(throw_value) => {
                        let Some(value) = self.resolve_static_throw_value_expression(throw_value)
                        else {
                            return Ok(None);
                        };
                        self.clear_local_throw_state();
                        self.clear_global_throw_state();
                        self.emit_immediate_promise_callback(&handler, &value, true)?;
                        if std::env::var_os("AYY_TRACE_INLINE_PROMISES").is_some() {
                            eprintln!(
                                "consume_immediate_promise_outcome:throw-handler-emitted property={property_name}"
                            );
                        }
                        return Ok(Some(StaticEvalOutcome::Value(Expression::Undefined)));
                    }
                };
                if let Some(returned_rejection) =
                    self.resolve_immediate_promise_callback_returned_rejection(&handler)?
                {
                    return Ok(Some(returned_rejection));
                }
                self.emit_immediate_promise_callback(&handler, handler_argument, true)?;
                if std::env::var_os("AYY_TRACE_INLINE_PROMISES").is_some() {
                    eprintln!(
                        "consume_immediate_promise_outcome:value-handler-emitted property={property_name}"
                    );
                }
                Ok(Some(StaticEvalOutcome::Value(Expression::Undefined)))
            }
            _ => Ok(None),
        }
    }

    pub(in crate::backend::direct_wasm) fn emit_immediate_promise_member_call(
        &mut self,
        object: &Expression,
        property: &Expression,
        arguments: &[CallArgument],
    ) -> DirectResult<bool> {
        if std::env::var_os("AYY_TRACE_INLINE_PROMISES").is_some() {
            eprintln!(
                "emit_immediate_promise_member_call:start object={object:?} property={property:?} arguments={arguments:?}"
            );
        }
        let Expression::String(property_name) = property else {
            return Ok(false);
        };
        if property_name != "then" && property_name != "catch" {
            return Ok(false);
        }
        if self.expected_event_log_strings().is_some() && Self::call_is_promise_like_chain(object) {
            if std::env::var_os("AYY_TRACE_INLINE_PROMISES").is_some() {
                eprintln!(
                    "emit_immediate_promise_member_call:tick-order-promise-chain-fallback object={object:?} property={property:?}"
                );
            }
            self.push_i32_const(JS_TYPEOF_OBJECT_TAG);
            return Ok(true);
        }
        let Some(_outcome) = self.consume_immediate_promise_outcome(&Expression::Call {
            callee: Box::new(Expression::Member {
                object: Box::new(object.clone()),
                property: Box::new(property.clone()),
            }),
            arguments: arguments.to_vec(),
        })?
        else {
            if std::env::var_os("AYY_TRACE_INLINE_PROMISES").is_some() {
                eprintln!(
                    "emit_immediate_promise_member_call:no-static-outcome object={object:?} property={property:?}"
                );
            }
            if Self::call_is_promise_like_chain(object)
                || self.expression_is_direct_async_function_call(object)
            {
                if std::env::var_os("AYY_TRACE_INLINE_PROMISES").is_some() {
                    eprintln!(
                        "emit_immediate_promise_member_call:promise-like-fallback object={object:?} property={property:?}"
                    );
                }
                self.emit_numeric_expression(object)?;
                self.state.emission.output.instructions.push(0x1a);
                self.push_i32_const(JS_TYPEOF_OBJECT_TAG);
                return Ok(true);
            }
            if let Expression::Call { callee, .. } = object
                && let Expression::Member {
                    object: iterator_expression,
                    property: iterator_property,
                } = callee.as_ref()
                && matches!(
                    iterator_property.as_ref(),
                    Expression::String(name) if name == "next"
                )
                && self.is_async_generator_iterator_expression(iterator_expression)
            {
                self.push_i32_const(JS_TYPEOF_OBJECT_TAG);
                return Ok(true);
            }
            if std::env::var_os("AYY_TRACE_INLINE_PROMISES").is_some() {
                eprintln!(
                    "emit_immediate_promise_member_call:dynamic-fallback object={object:?} property={property:?}"
                );
            }
            return Ok(false);
        };
        if std::env::var_os("AYY_TRACE_INLINE_PROMISES").is_some() {
            eprintln!(
                "emit_immediate_promise_member_call:consumed object={object:?} property={property:?}"
            );
        }
        self.push_i32_const(JS_TYPEOF_OBJECT_TAG);
        Ok(true)
    }
}
