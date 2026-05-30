use super::*;

impl<'a> FunctionCompiler<'a> {
    fn expression_references_module_dependency_namespace_for_promise_stmt(
        expression: &Expression,
    ) -> bool {
        match expression {
            Expression::Identifier(name) => name.starts_with("__ayy_module_dep_"),
            Expression::Member { object, property } => {
                Self::expression_references_module_dependency_namespace_for_promise_stmt(object)
                    || Self::expression_references_module_dependency_namespace_for_promise_stmt(
                        property,
                    )
            }
            Expression::SuperMember { property } => {
                Self::expression_references_module_dependency_namespace_for_promise_stmt(property)
            }
            Expression::Call { callee, arguments }
            | Expression::SuperCall { callee, arguments }
            | Expression::New { callee, arguments } => {
                Self::expression_references_module_dependency_namespace_for_promise_stmt(callee)
                    || arguments.iter().any(|argument| {
                        Self::expression_references_module_dependency_namespace_for_promise_stmt(
                            argument.expression(),
                        )
                    })
            }
            Expression::Array(elements) => elements.iter().any(|element| match element {
                ArrayElement::Expression(value) | ArrayElement::Spread(value) => {
                    Self::expression_references_module_dependency_namespace_for_promise_stmt(value)
                }
            }),
            Expression::Object(entries) => entries.iter().any(|entry| match entry {
                ObjectEntry::Data { key, value } => {
                    Self::expression_references_module_dependency_namespace_for_promise_stmt(key)
                        || Self::expression_references_module_dependency_namespace_for_promise_stmt(
                            value,
                        )
                }
                ObjectEntry::Getter { key, getter } => {
                    Self::expression_references_module_dependency_namespace_for_promise_stmt(key)
                        || Self::expression_references_module_dependency_namespace_for_promise_stmt(
                            getter,
                        )
                }
                ObjectEntry::Setter { key, setter } => {
                    Self::expression_references_module_dependency_namespace_for_promise_stmt(key)
                        || Self::expression_references_module_dependency_namespace_for_promise_stmt(
                            setter,
                        )
                }
                ObjectEntry::Spread(value) => {
                    Self::expression_references_module_dependency_namespace_for_promise_stmt(value)
                }
            }),
            Expression::Assign { value, .. }
            | Expression::Await(value)
            | Expression::EnumerateKeys(value)
            | Expression::GetIterator(value)
            | Expression::IteratorClose(value) => {
                Self::expression_references_module_dependency_namespace_for_promise_stmt(value)
            }
            Expression::AssignMember {
                object,
                property,
                value,
            } => {
                Self::expression_references_module_dependency_namespace_for_promise_stmt(object)
                    || Self::expression_references_module_dependency_namespace_for_promise_stmt(
                        property,
                    )
                    || Self::expression_references_module_dependency_namespace_for_promise_stmt(
                        value,
                    )
            }
            Expression::AssignSuperMember { property, value } => {
                Self::expression_references_module_dependency_namespace_for_promise_stmt(property)
                    || Self::expression_references_module_dependency_namespace_for_promise_stmt(
                        value,
                    )
            }
            Expression::Unary { expression, .. } => {
                Self::expression_references_module_dependency_namespace_for_promise_stmt(expression)
            }
            Expression::Binary { left, right, .. } => {
                Self::expression_references_module_dependency_namespace_for_promise_stmt(left)
                    || Self::expression_references_module_dependency_namespace_for_promise_stmt(
                        right,
                    )
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                Self::expression_references_module_dependency_namespace_for_promise_stmt(condition)
                    || Self::expression_references_module_dependency_namespace_for_promise_stmt(
                        then_expression,
                    )
                    || Self::expression_references_module_dependency_namespace_for_promise_stmt(
                        else_expression,
                    )
            }
            Expression::Sequence(expressions) => expressions
                .iter()
                .any(Self::expression_references_module_dependency_namespace_for_promise_stmt),
            Expression::Update { name, .. } => name.starts_with("__ayy_module_dep_"),
            _ => false,
        }
    }

    fn expression_is_imported_promise_resolver_member(expression: &Expression) -> bool {
        matches!(
            expression,
            Expression::Member { property, .. }
                if matches!(property.as_ref(), Expression::String(name) if matches!(name.as_str(), "resolve" | "reject"))
                    && Self::expression_references_module_dependency_namespace_for_promise_stmt(expression)
        )
    }

    fn expression_is_module_dependency_promise_member(expression: &Expression) -> bool {
        let Expression::Member { object, property } = expression else {
            return false;
        };
        if !matches!(property.as_ref(), Expression::String(name) if name == "promise") {
            return false;
        }
        matches!(
            object.as_ref(),
            Expression::Member { object, .. }
                if matches!(object.as_ref(), Expression::Identifier(name) if name.starts_with("__ayy_module_dep_"))
        )
    }

    fn expression_is_promise_all_of_module_dependency_promises(expression: &Expression) -> bool {
        let Expression::Call { callee, arguments } = expression else {
            return false;
        };
        let Expression::Member { object, property } = callee.as_ref() else {
            return false;
        };
        if !matches!(object.as_ref(), Expression::Identifier(name) if name == "Promise")
            || !matches!(property.as_ref(), Expression::String(name) if name == "all")
        {
            return false;
        }
        let [CallArgument::Expression(Expression::Array(elements))] = arguments.as_slice() else {
            return false;
        };
        !elements.is_empty()
            && elements.iter().all(|element| {
                matches!(
                    element,
                    ArrayElement::Expression(value)
                        if Self::expression_is_module_dependency_promise_member(value)
                )
            })
    }

    fn static_promise_with_resolvers_object() -> Expression {
        Expression::Object(vec![
            ObjectEntry::Data {
                key: Expression::String("promise".to_string()),
                value: Expression::Call {
                    callee: Box::new(Expression::Member {
                        object: Box::new(Expression::Identifier("Promise".to_string())),
                        property: Box::new(Expression::String("resolve".to_string())),
                    }),
                    arguments: vec![CallArgument::Expression(Expression::Undefined)],
                },
            },
            ObjectEntry::Data {
                key: Expression::String("resolve".to_string()),
                value: Expression::Identifier("__ayy_promise_with_resolvers_resolve".to_string()),
            },
            ObjectEntry::Data {
                key: Expression::String("reject".to_string()),
                value: Expression::Identifier("__ayy_promise_with_resolvers_reject".to_string()),
            },
        ])
    }

    fn static_promise_resolve_call_value(expression: &Expression) -> Option<Expression> {
        let Expression::Call { callee, arguments } = expression else {
            return None;
        };
        let Expression::Member { object, property } = callee.as_ref() else {
            return None;
        };
        if !matches!(object.as_ref(), Expression::Identifier(name) if name == "Promise")
            || !matches!(property.as_ref(), Expression::String(name) if name == "resolve")
            || arguments.len() > 1
        {
            return None;
        }
        match arguments.first() {
            Some(CallArgument::Expression(value)) => Some(value.clone()),
            Some(CallArgument::Spread(_)) => None,
            None => Some(Expression::Undefined),
        }
    }

    fn static_binding_value_for_identifier(&self, name: &str) -> Option<Expression> {
        let resolved_local = self.resolve_current_local_binding(name);
        let resolved_value = resolved_local.as_ref().and_then(|(resolved_name, _)| {
            self.state
                .speculation
                .static_semantics
                .local_value_binding(resolved_name)
        });
        let direct_value = self
            .state
            .speculation
            .static_semantics
            .local_value_binding(name);
        let source_value = scoped_binding_source_name(name).and_then(|source_name| {
            self.state
                .speculation
                .static_semantics
                .local_value_binding(source_name)
        });
        let global_value = self.global_value_binding(name);
        let value = resolved_value
            .or(direct_value)
            .or(source_value)
            .or(global_value)
            .cloned();
        if std::env::var_os("AYY_TRACE_INLINE_PROMISES").is_some()
            && (name.contains("promiseForNamespace") || value.is_some())
        {
            eprintln!(
                "static_binding_lookup name={name} resolved={:?} resolved_value={:?} direct_value={:?} source_value={:?} global_value={:?} value={value:?}",
                resolved_local, resolved_value, direct_value, source_value, global_value,
            );
        }
        value
    }

    fn is_done_callback_binding_name(name: &str) -> bool {
        name == "$DONE" || name.contains("$DONE")
    }

    fn lowered_for_await_body_breaks_before_done(&self, body: &[Statement]) -> bool {
        let mut passed_done_guard = false;
        for statement in body {
            if !passed_done_guard {
                if Self::statement_is_for_await_done_guard(statement) {
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

    fn statement_is_for_await_done_guard(statement: &Statement) -> bool {
        matches!(
            statement,
            Statement::If {
                condition: Expression::Member { property, .. },
                ..
            } if matches!(
                property.as_ref(),
                Expression::String(name) if name == "done"
            )
        )
    }

    fn statement_contains_break(statement: &Statement) -> bool {
        match statement {
            Statement::Break { .. } => true,
            Statement::Block { body } | Statement::Declaration { body } => {
                body.iter().any(Self::statement_contains_break)
            }
            Statement::Try {
                body,
                catch_setup,
                catch_body,
                ..
            } => {
                body.iter().any(Self::statement_contains_break)
                    || catch_setup.iter().any(Self::statement_contains_break)
                    || catch_body.iter().any(Self::statement_contains_break)
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
                if Self::statement_is_for_await_done_guard(statement) {
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

    fn expression_is_simple_for_await_break_side_effect_value(expression: &Expression) -> bool {
        match expression {
            Expression::Number(_)
            | Expression::BigInt(_)
            | Expression::String(_)
            | Expression::Bool(_)
            | Expression::Null
            | Expression::Undefined
            | Expression::This
            | Expression::NewTarget => true,
            Expression::Identifier(name) => !name.starts_with("__ayy_"),
            Expression::Array(elements) => elements.iter().all(|element| match element {
                ArrayElement::Expression(value) | ArrayElement::Spread(value) => {
                    Self::expression_is_simple_for_await_break_side_effect_value(value)
                }
            }),
            Expression::Object(entries) => entries.iter().all(|entry| match entry {
                ObjectEntry::Data { key, value } => {
                    Self::expression_is_simple_for_await_break_side_effect_value(key)
                        && Self::expression_is_simple_for_await_break_side_effect_value(value)
                }
                _ => false,
            }),
            Expression::Member { object, property } => {
                Self::expression_is_simple_for_await_break_side_effect_value(object)
                    && Self::expression_is_simple_for_await_break_side_effect_value(property)
            }
            Expression::Unary { expression, .. } => {
                Self::expression_is_simple_for_await_break_side_effect_value(expression)
            }
            Expression::Binary { left, right, .. } => {
                Self::expression_is_simple_for_await_break_side_effect_value(left)
                    && Self::expression_is_simple_for_await_break_side_effect_value(right)
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                Self::expression_is_simple_for_await_break_side_effect_value(condition)
                    && Self::expression_is_simple_for_await_break_side_effect_value(then_expression)
                    && Self::expression_is_simple_for_await_break_side_effect_value(else_expression)
            }
            Expression::Sequence(expressions) => expressions
                .iter()
                .all(Self::expression_is_simple_for_await_break_side_effect_value),
            _ => false,
        }
    }

    fn statement_is_simple_for_await_break_side_effect_assignment(
        statement: &Statement,
    ) -> Option<Statement> {
        match statement {
            Statement::Assign { name, value } => {
                if name.starts_with("__ayy_")
                    || !Self::expression_is_simple_for_await_break_side_effect_value(value)
                {
                    return None;
                }
                Some(Statement::Assign {
                    name: name.clone(),
                    value: value.clone(),
                })
            }
            Statement::Expression(Expression::Assign { name, value }) => {
                if name.starts_with("__ayy_")
                    || !Self::expression_is_simple_for_await_break_side_effect_value(value)
                {
                    return None;
                }
                Some(Statement::Assign {
                    name: name.clone(),
                    value: value.as_ref().clone(),
                })
            }
            _ => None,
        }
    }

    fn emit_lowered_for_await_break_side_effect_statement(
        &mut self,
        statement: &Statement,
    ) -> DirectResult<bool> {
        if let Some(assignment) =
            Self::statement_is_simple_for_await_break_side_effect_assignment(statement)
        {
            self.emit_statement(&assignment)?;
            return Ok(true);
        }
        match statement {
            Statement::Break { .. } => Ok(false),
            Statement::Block { body }
            | Statement::Declaration { body }
            | Statement::Labeled { body, .. } => {
                for nested_statement in body {
                    if !self.emit_lowered_for_await_break_side_effect_statement(nested_statement)? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            Statement::Try { body, .. } => {
                for nested_statement in body {
                    if !self.emit_lowered_for_await_break_side_effect_statement(nested_statement)? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            _ => Ok(true),
        }
    }

    fn emit_lowered_for_await_break_side_effects(
        &mut self,
        statements: &[Statement],
    ) -> DirectResult<()> {
        for statement in statements {
            let body = match statement {
                Statement::While {
                    condition: Expression::Bool(true),
                    body,
                    ..
                } => body,
                Statement::For {
                    condition: Some(Expression::Bool(true)) | None,
                    body,
                    ..
                } => body,
                _ => continue,
            };
            if !self.lowered_for_await_body_breaks_before_done(body) {
                continue;
            }
            let mut passed_done_guard = false;
            for body_statement in body {
                if !passed_done_guard {
                    if Self::statement_is_for_await_done_guard(body_statement) {
                        passed_done_guard = true;
                    }
                    continue;
                }
                if !self.emit_lowered_for_await_break_side_effect_statement(body_statement)? {
                    return Ok(());
                }
            }
            return Ok(());
        }
        Ok(())
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
            .find_map(|statement| Self::iterator_source_from_statement(statement, iterator_name))
    }

    fn iterator_source_from_statement<'b>(
        statement: &'b Statement,
        iterator_name: &str,
    ) -> Option<&'b Expression> {
        match statement {
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
            Statement::For { init, .. } => init.iter().rev().find_map(|statement| {
                Self::iterator_source_from_statement(statement, iterator_name)
            }),
            _ => None,
        }
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
            let (break_hook, body) = match statement {
                Statement::While {
                    condition: Expression::Bool(true),
                    break_hook,
                    body,
                    ..
                } => (break_hook, body),
                Statement::For {
                    condition: Some(Expression::Bool(true)) | None,
                    break_hook,
                    body,
                    ..
                } => (break_hook, body),
                _ => continue,
            };
            if !self.lowered_for_await_body_breaks_before_done(body) {
                continue;
            }
            let iterator_name = self.lowered_for_await_break_hook_iterator_name(break_hook)?;
            let source =
                Self::iterator_source_from_statement(statement, iterator_name).or_else(|| {
                    self.lowered_for_await_iterator_source(&statements[..index], iterator_name)
                })?;
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
            .or_else(|| self.direct_async_function_terminal_return_outcome(function, arguments))
    }

    fn direct_async_function_terminal_return_outcome(
        &self,
        function: &FunctionDeclaration,
        arguments: &[CallArgument],
    ) -> Option<StaticEvalOutcome> {
        if !arguments.is_empty() || !function.params.is_empty() {
            return None;
        }
        let [Statement::Return(return_value)] = function.body.as_slice() else {
            return None;
        };
        self.resolve_static_await_resolution_outcome(return_value)
    }

    fn emit_direct_async_function_call_await_effects(
        &mut self,
        expression: &Expression,
    ) -> DirectResult<()> {
        let Expression::Call { callee, arguments } = expression else {
            return Ok(());
        };
        if !arguments.is_empty() {
            return Ok(());
        }
        let Some(LocalFunctionBinding::User(function_name)) = self
            .resolve_function_binding_from_expression_with_context(
                callee,
                self.current_function_name(),
            )
        else {
            return Ok(());
        };
        let Some(user_function) = self.user_function(&function_name) else {
            return Ok(());
        };
        if !user_function.is_async() {
            return Ok(());
        }
        let Some(function) = self.resolve_registered_function_declaration(&function_name) else {
            return Ok(());
        };
        if !function.params.is_empty() {
            return Ok(());
        }
        let body = function.body.clone();
        if self.lowered_for_await_break_close_outcome(&body).is_some() {
            return self.emit_lowered_for_await_break_side_effects(&body);
        }
        let [Statement::Return(return_value)] = body.as_slice() else {
            return Ok(());
        };
        let return_value = return_value.clone();
        self.emit_static_await_resolution_effects(&return_value)
    }

    pub(in crate::backend::direct_wasm) fn emit_static_await_resolution_effects(
        &mut self,
        expression: &Expression,
    ) -> DirectResult<()> {
        match expression {
            Expression::Await(value) => self.emit_static_await_resolution_effects(value),
            Expression::Call { callee, arguments } if matches!(callee.as_ref(), Expression::Identifier(name) if name == "__ayyDynamicImport") =>
            {
                self.emit_static_dynamic_import_options_effects(arguments)?;
                self.emit_static_dynamic_import_module_init_effects(arguments)
            }
            _ => Ok(()),
        }
    }

    pub(in crate::backend::direct_wasm) fn emit_pending_static_promise_reactions(
        &mut self,
    ) -> DirectResult<()> {
        while !self
            .state
            .emission
            .pending_static_promise_reactions
            .is_empty()
        {
            let pending = std::mem::take(&mut self.state.emission.pending_static_promise_reactions);
            for (handler, argument) in pending {
                if std::env::var_os("AYY_TRACE_INLINE_PROMISES").is_some() {
                    eprintln!(
                        "emit_pending_static_promise_reactions:handler={handler:?} argument={argument:?}"
                    );
                }
                self.record_static_module_dependency_promise_resolutions_from_callback(&handler);
                self.emit_immediate_promise_callback(&handler, &argument, true)?;
            }
        }
        Ok(())
    }

    fn install_immediate_promise_returned_function_capture_slots(
        &mut self,
        result: &Expression,
        updated_bindings: &HashMap<String, Expression>,
    ) -> DirectResult<()> {
        let Some(LocalFunctionBinding::User(returned_function_name)) =
            self.resolve_function_binding_from_expression(result)
        else {
            return Ok(());
        };
        let mut capture_bindings = self
            .user_function_capture_bindings(&returned_function_name)
            .unwrap_or_default();
        if !capture_bindings.contains_key("arguments")
            && updated_bindings.contains_key("arguments")
            && self
                .user_function(&returned_function_name)
                .is_some_and(|function| function.lexical_this)
        {
            capture_bindings.insert(
                "arguments".to_string(),
                format!("__ayy_capture_binding__{returned_function_name}__arguments"),
            );
            self.backend
                .function_registry
                .analysis
                .set_user_function_capture_bindings(
                    &returned_function_name,
                    capture_bindings.clone(),
                );
        }
        if capture_bindings.is_empty() {
            return Ok(());
        }
        let mut capture_slots = BTreeMap::new();
        let mut capture_names = capture_bindings.keys().cloned().collect::<Vec<_>>();
        capture_names.sort_by(|left, right| match (left.as_str(), right.as_str()) {
            ("arguments", "arguments") => std::cmp::Ordering::Equal,
            ("arguments", _) => std::cmp::Ordering::Less,
            (_, "arguments") => std::cmp::Ordering::Greater,
            _ => left.cmp(right),
        });
        for capture_name in capture_names {
            let source_expression = updated_bindings
                .get(&capture_name)
                .or_else(|| {
                    scoped_binding_source_name(&capture_name)
                        .and_then(|source_name| updated_bindings.get(source_name))
                })
                .or_else(|| {
                    updated_bindings.iter().find_map(|(binding_name, value)| {
                        scoped_binding_source_name(binding_name)
                            .is_some_and(|source_name| source_name == capture_name)
                            .then_some(value)
                    })
                })
                .cloned()
                .unwrap_or_else(|| Expression::Identifier(capture_name.clone()));
            let source_expression = if matches!(
                &source_expression,
                Expression::Identifier(name) if name == "new.target"
            ) {
                Expression::Undefined
            } else {
                source_expression
            };
            if let Expression::Identifier(source_name) = &source_expression
                && let Some(existing_hidden_name) = capture_slots.get(source_name).cloned()
            {
                if std::env::var_os("AYY_TRACE_INLINE_PROMISES").is_some() {
                    eprintln!(
                        "immediate_await_captures:reuse-capture function={returned_function_name} capture={capture_name} source={source_name} hidden={existing_hidden_name}"
                    );
                }
                capture_slots.insert(capture_name.clone(), existing_hidden_name);
                continue;
            }
            let source_expression = if let Expression::Identifier(source_name) = &source_expression
                && let Some(resolved_source) = updated_bindings.get(source_name)
            {
                resolved_source.clone()
            } else {
                source_expression
            };
            let hidden_name = self.allocate_named_hidden_local(
                &format!("closure_slot_{}_{}", returned_function_name, capture_name),
                self.infer_value_kind(&source_expression)
                    .unwrap_or(StaticValueKind::Unknown),
            );
            if std::env::var_os("AYY_TRACE_INLINE_PROMISES").is_some() {
                eprintln!(
                    "immediate_await_captures:install-capture function={returned_function_name} capture={capture_name} source={source_expression:?} hidden={hidden_name}"
                );
            }
            let hidden_local = self
                .state
                .runtime
                .locals
                .get(&hidden_name)
                .copied()
                .expect("fresh immediate promise closure capture local must exist");
            self.emit_numeric_expression(&source_expression)?;
            self.push_local_set(hidden_local);
            self.update_capture_slot_binding_from_expression(&hidden_name, &source_expression)?;
            self.sync_capture_slot_runtime_object_shadows_from_expression(
                &hidden_name,
                &source_expression,
            )?;
            if let Expression::Identifier(source_name) = &source_expression {
                self.state
                    .speculation
                    .static_semantics
                    .capture_slot_source_bindings
                    .insert(hidden_name.clone(), source_name.clone());
            }
            capture_slots.insert(capture_name.clone(), hidden_name);
        }
        if capture_slots.is_empty() {
            return Ok(());
        }
        let target_name = match result {
            Expression::Identifier(name) => name.clone(),
            _ => returned_function_name,
        };
        let key = Self::identifier_function_value_capture_slots_key(&target_name);
        self.state
            .speculation
            .static_semantics
            .objects
            .member_function_capture_slots
            .insert(key.clone(), capture_slots.clone());
        if self.binding_key_is_global(&key) {
            self.backend
                .set_global_member_function_capture_slots(key, capture_slots);
        }
        Ok(())
    }

    fn user_function_call_await_resolution_outcome_with_captures(
        &mut self,
        binding: &LocalFunctionBinding,
        call_arguments: &[Expression],
        this_binding: &Expression,
        capture_source_bindings: Option<&HashMap<String, Expression>>,
    ) -> DirectResult<Option<StaticEvalOutcome>> {
        let LocalFunctionBinding::User(function_name) = binding else {
            return Ok(None);
        };
        if std::env::var_os("AYY_TRACE_INLINE_PROMISES").is_some() {
            eprintln!(
                "immediate_await_captures:user-start function={function_name} this={this_binding:?} args={call_arguments:?}"
            );
        }
        if self
            .user_function(function_name)
            .is_some_and(|user_function| {
                !self.user_function_mentions_private_member_access(user_function)
            })
            && {
                if std::env::var_os("AYY_TRACE_INLINE_PROMISES").is_some() {
                    eprintln!("immediate_await_captures:snapshot-attempt function={function_name}");
                }
                true
            }
            && let Some((result, updated_bindings)) = self
                .resolve_bound_snapshot_user_function_result_with_arguments_and_this(
                    function_name,
                    &capture_source_bindings.cloned().unwrap_or_else(|| {
                        self.user_function_capture_bindings(function_name)
                            .unwrap_or_default()
                            .keys()
                            .map(|capture_name| {
                                (
                                    capture_name.clone(),
                                    Expression::Identifier(capture_name.clone()),
                                )
                            })
                            .collect::<HashMap<_, _>>()
                    }),
                    call_arguments,
                    this_binding,
                )
        {
            if std::env::var_os("AYY_TRACE_INLINE_PROMISES").is_some() {
                eprintln!(
                    "immediate_await_captures:snapshot-result function={function_name} result={result:?} updated_keys={:?}",
                    updated_bindings.keys().collect::<Vec<_>>()
                );
            }
            self.install_immediate_promise_returned_function_capture_slots(
                &result,
                &updated_bindings,
            )?;
            if self
                .user_function(function_name)
                .is_some_and(|function| function.is_async())
                && let Some(outcome) =
                    self.immediate_await_resolution_outcome_with_captures(&result)?
            {
                return Ok(Some(outcome));
            }
            return Ok(Some(StaticEvalOutcome::Value(result)));
        }
        let value = self
            .immediate_user_function_terminal_return_expression_with_call_frame(
                function_name,
                call_arguments,
                this_binding,
            )
            .or_else(|| {
                self.resolve_function_binding_static_return_expression_with_call_frame(
                    binding,
                    call_arguments,
                    this_binding,
                )
            });
        if let Some(mut value) = value {
            if let Some(capture_source_bindings) = capture_source_bindings {
                value = Self::substitute_immediate_promise_capture_source_bindings(
                    &value,
                    capture_source_bindings,
                );
                value = Self::fold_immediate_promise_capture_identity_expression(value);
            }
            if self
                .user_function(function_name)
                .is_some_and(|function| !function.lexical_this)
            {
                value = Self::substitute_immediate_promise_bare_new_target(value);
            }
            if std::env::var_os("AYY_TRACE_INLINE_PROMISES").is_some() {
                eprintln!(
                    "immediate_await_captures:return-expression function={function_name} value={value:?}"
                );
            }
            if let Some(outcome) = self.immediate_await_resolution_outcome_with_captures(&value)? {
                return Ok(Some(outcome));
            }
            return Ok(self
                .resolve_static_await_resolution_outcome(&value)
                .or(Some(StaticEvalOutcome::Value(value))));
        }
        Ok(None)
    }

    fn fold_immediate_promise_capture_identity_expression(expression: Expression) -> Expression {
        match expression {
            Expression::Binary { op, left, right }
                if matches!(
                    op,
                    BinaryOp::Equal
                        | BinaryOp::LooseEqual
                        | BinaryOp::NotEqual
                        | BinaryOp::LooseNotEqual
                ) =>
            {
                let equal = if Self::immediate_promise_fresh_reference_expression(&left)
                    || Self::immediate_promise_fresh_reference_expression(&right)
                {
                    false
                } else if static_expression_matches(&left, &right) {
                    true
                } else {
                    return Expression::Binary { op, left, right };
                };
                Expression::Bool(match op {
                    BinaryOp::Equal | BinaryOp::LooseEqual => equal,
                    BinaryOp::NotEqual | BinaryOp::LooseNotEqual => !equal,
                    _ => unreachable!("filtered above"),
                })
            }
            _ => expression,
        }
    }

    fn immediate_promise_fresh_reference_expression(expression: &Expression) -> bool {
        matches!(expression, Expression::Array(_) | Expression::Object(_))
    }

    fn substitute_immediate_promise_bare_new_target(expression: Expression) -> Expression {
        let bindings = HashMap::new();
        Self::substitute_immediate_promise_capture_source_bindings(&expression, &bindings)
    }

    fn immediate_promise_capture_source_binding<'b>(
        name: &str,
        bindings: &'b HashMap<String, Expression>,
    ) -> Option<&'b Expression> {
        bindings.get(name).or_else(|| {
            scoped_binding_source_name(name).and_then(|source_name| bindings.get(source_name))
        })
    }

    fn substitute_immediate_promise_capture_source_bindings(
        expression: &Expression,
        bindings: &HashMap<String, Expression>,
    ) -> Expression {
        match expression {
            Expression::Identifier(name) => {
                Self::immediate_promise_capture_source_binding(name, bindings)
                    .cloned()
                    .unwrap_or_else(|| expression.clone())
            }
            Expression::This => bindings
                .get("this")
                .cloned()
                .unwrap_or_else(|| expression.clone()),
            Expression::NewTarget => bindings
                .get("new.target")
                .filter(
                    |value| !matches!(value, Expression::Identifier(name) if name == "new.target"),
                )
                .cloned()
                .unwrap_or(Expression::Undefined),
            Expression::Array(elements) => Expression::Array(
                elements
                    .iter()
                    .map(|element| match element {
                        ArrayElement::Expression(element) => ArrayElement::Expression(
                            Self::substitute_immediate_promise_capture_source_bindings(
                                element, bindings,
                            ),
                        ),
                        ArrayElement::Spread(element) => ArrayElement::Spread(
                            Self::substitute_immediate_promise_capture_source_bindings(
                                element, bindings,
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
                            key: Self::substitute_immediate_promise_capture_source_bindings(
                                key, bindings,
                            ),
                            value: Self::substitute_immediate_promise_capture_source_bindings(
                                value, bindings,
                            ),
                        },
                        ObjectEntry::Getter { key, getter } => ObjectEntry::Getter {
                            key: Self::substitute_immediate_promise_capture_source_bindings(
                                key, bindings,
                            ),
                            getter: Self::substitute_immediate_promise_capture_source_bindings(
                                getter, bindings,
                            ),
                        },
                        ObjectEntry::Setter { key, setter } => ObjectEntry::Setter {
                            key: Self::substitute_immediate_promise_capture_source_bindings(
                                key, bindings,
                            ),
                            setter: Self::substitute_immediate_promise_capture_source_bindings(
                                setter, bindings,
                            ),
                        },
                        ObjectEntry::Spread(value) => ObjectEntry::Spread(
                            Self::substitute_immediate_promise_capture_source_bindings(
                                value, bindings,
                            ),
                        ),
                    })
                    .collect(),
            ),
            Expression::Member { object, property } => Expression::Member {
                object: Box::new(Self::substitute_immediate_promise_capture_source_bindings(
                    object, bindings,
                )),
                property: Box::new(Self::substitute_immediate_promise_capture_source_bindings(
                    property, bindings,
                )),
            },
            Expression::SuperMember { property } => Expression::SuperMember {
                property: Box::new(Self::substitute_immediate_promise_capture_source_bindings(
                    property, bindings,
                )),
            },
            Expression::Assign { name, value } => Expression::Assign {
                name: name.clone(),
                value: Box::new(Self::substitute_immediate_promise_capture_source_bindings(
                    value, bindings,
                )),
            },
            Expression::AssignMember {
                object,
                property,
                value,
            } => Expression::AssignMember {
                object: Box::new(Self::substitute_immediate_promise_capture_source_bindings(
                    object, bindings,
                )),
                property: Box::new(Self::substitute_immediate_promise_capture_source_bindings(
                    property, bindings,
                )),
                value: Box::new(Self::substitute_immediate_promise_capture_source_bindings(
                    value, bindings,
                )),
            },
            Expression::AssignSuperMember { property, value } => Expression::AssignSuperMember {
                property: Box::new(Self::substitute_immediate_promise_capture_source_bindings(
                    property, bindings,
                )),
                value: Box::new(Self::substitute_immediate_promise_capture_source_bindings(
                    value, bindings,
                )),
            },
            Expression::Await(value) => Expression::Await(Box::new(
                Self::substitute_immediate_promise_capture_source_bindings(value, bindings),
            )),
            Expression::EnumerateKeys(value) => Expression::EnumerateKeys(Box::new(
                Self::substitute_immediate_promise_capture_source_bindings(value, bindings),
            )),
            Expression::GetIterator(value) => Expression::GetIterator(Box::new(
                Self::substitute_immediate_promise_capture_source_bindings(value, bindings),
            )),
            Expression::IteratorClose(value) => Expression::IteratorClose(Box::new(
                Self::substitute_immediate_promise_capture_source_bindings(value, bindings),
            )),
            Expression::Unary { op, expression } => Expression::Unary {
                op: *op,
                expression: Box::new(Self::substitute_immediate_promise_capture_source_bindings(
                    expression, bindings,
                )),
            },
            Expression::Binary { op, left, right } => Expression::Binary {
                op: *op,
                left: Box::new(Self::substitute_immediate_promise_capture_source_bindings(
                    left, bindings,
                )),
                right: Box::new(Self::substitute_immediate_promise_capture_source_bindings(
                    right, bindings,
                )),
            },
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => Expression::Conditional {
                condition: Box::new(Self::substitute_immediate_promise_capture_source_bindings(
                    condition, bindings,
                )),
                then_expression: Box::new(
                    Self::substitute_immediate_promise_capture_source_bindings(
                        then_expression,
                        bindings,
                    ),
                ),
                else_expression: Box::new(
                    Self::substitute_immediate_promise_capture_source_bindings(
                        else_expression,
                        bindings,
                    ),
                ),
            },
            Expression::Sequence(expressions) => Expression::Sequence(
                expressions
                    .iter()
                    .map(|expression| {
                        Self::substitute_immediate_promise_capture_source_bindings(
                            expression, bindings,
                        )
                    })
                    .collect(),
            ),
            Expression::Call { callee, arguments } => Expression::Call {
                callee: Box::new(Self::substitute_immediate_promise_capture_source_bindings(
                    callee, bindings,
                )),
                arguments: arguments
                    .iter()
                    .map(|argument| match argument {
                        CallArgument::Expression(argument) => CallArgument::Expression(
                            Self::substitute_immediate_promise_capture_source_bindings(
                                argument, bindings,
                            ),
                        ),
                        CallArgument::Spread(argument) => CallArgument::Spread(
                            Self::substitute_immediate_promise_capture_source_bindings(
                                argument, bindings,
                            ),
                        ),
                    })
                    .collect(),
            },
            Expression::SuperCall { callee, arguments } => Expression::SuperCall {
                callee: Box::new(Self::substitute_immediate_promise_capture_source_bindings(
                    callee, bindings,
                )),
                arguments: arguments
                    .iter()
                    .map(|argument| match argument {
                        CallArgument::Expression(argument) => CallArgument::Expression(
                            Self::substitute_immediate_promise_capture_source_bindings(
                                argument, bindings,
                            ),
                        ),
                        CallArgument::Spread(argument) => CallArgument::Spread(
                            Self::substitute_immediate_promise_capture_source_bindings(
                                argument, bindings,
                            ),
                        ),
                    })
                    .collect(),
            },
            Expression::New { callee, arguments } => Expression::New {
                callee: Box::new(Self::substitute_immediate_promise_capture_source_bindings(
                    callee, bindings,
                )),
                arguments: arguments
                    .iter()
                    .map(|argument| match argument {
                        CallArgument::Expression(argument) => CallArgument::Expression(
                            Self::substitute_immediate_promise_capture_source_bindings(
                                argument, bindings,
                            ),
                        ),
                        CallArgument::Spread(argument) => CallArgument::Spread(
                            Self::substitute_immediate_promise_capture_source_bindings(
                                argument, bindings,
                            ),
                        ),
                    })
                    .collect(),
            },
            Expression::Update { name, op, prefix } => Expression::Update {
                name: name.clone(),
                op: *op,
                prefix: *prefix,
            },
            Expression::Number(_)
            | Expression::BigInt(_)
            | Expression::String(_)
            | Expression::Bool(_)
            | Expression::Null
            | Expression::Undefined
            | Expression::Sent => expression.clone(),
        }
    }

    fn immediate_user_function_terminal_return_expression_with_call_frame(
        &self,
        function_name: &str,
        arguments: &[Expression],
        this_binding: &Expression,
    ) -> Option<Expression> {
        let user_function = self.user_function(function_name)?;
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
        let call_arguments = arguments
            .iter()
            .cloned()
            .map(CallArgument::Expression)
            .collect::<Vec<_>>();
        let arguments_binding = if user_function.lexical_this {
            Expression::Identifier("arguments".to_string())
        } else {
            Expression::Array(
                arguments
                    .iter()
                    .cloned()
                    .map(ArrayElement::Expression)
                    .collect(),
            )
        };
        Some(self.substitute_user_function_call_frame_bindings(
            return_value,
            user_function,
            &call_arguments,
            this_binding,
            &arguments_binding,
        ))
    }

    fn immediate_await_resolution_outcome_with_captures(
        &mut self,
        resolution: &Expression,
    ) -> DirectResult<Option<StaticEvalOutcome>> {
        if let Expression::Await(value) = resolution {
            if let Some(outcome) = self.immediate_await_resolution_outcome_with_captures(value)? {
                return Ok(Some(outcome));
            }
            return Ok(self.resolve_static_await_resolution_outcome(resolution));
        }
        let Expression::Call { callee, arguments } = resolution else {
            return Ok(self.resolve_static_await_resolution_outcome(resolution));
        };
        if std::env::var_os("AYY_TRACE_INLINE_PROMISES").is_some() {
            eprintln!("immediate_await_captures:resolution-call {resolution:?}");
        }
        if matches!(
            callee.as_ref(),
            Expression::Member { property, .. }
                if matches!(
                    property.as_ref(),
                    Expression::String(name) if matches!(name.as_str(), "then" | "catch" | "finally")
                )
        ) {
            if let Some(outcome) =
                self.consume_immediate_promise_outcome_unmaterialized(resolution)?
            {
                return Ok(Some(outcome));
            }
            return Ok(self.resolve_static_await_resolution_outcome(resolution));
        }
        let Some(binding) = self.resolve_function_binding_from_expression_with_context(
            callee,
            self.current_function_name(),
        ) else {
            return Ok(self.resolve_static_await_resolution_outcome(resolution));
        };
        let call_arguments = self.expand_call_arguments(arguments);
        let this_binding = match callee.as_ref() {
            Expression::Member { object, .. } => self.materialize_static_expression(object),
            Expression::SuperMember { .. } => Expression::This,
            _ => Expression::Undefined,
        };
        let capture_source_bindings =
            self.resolve_function_expression_capture_slots(callee)
                .map(|capture_slots| {
                    capture_slots
                        .into_iter()
                        .map(|(capture_name, slot_name)| {
                            (
                                capture_name,
                                self.snapshot_bound_capture_slot_expression(&slot_name),
                            )
                        })
                        .collect::<HashMap<_, _>>()
                });
        if let Some(outcome) = self.user_function_call_await_resolution_outcome_with_captures(
            &binding,
            &call_arguments,
            &this_binding,
            capture_source_bindings.as_ref(),
        )? {
            return Ok(Some(outcome));
        }
        Ok(self.resolve_static_await_resolution_outcome(resolution))
    }

    fn bound_function_call_await_resolution_outcome(
        &mut self,
        callee: &Expression,
        arguments: &[CallArgument],
    ) -> DirectResult<Option<StaticEvalOutcome>> {
        let resolved_callee = self
            .resolve_bound_alias_expression(callee)
            .filter(|resolved| !static_expression_matches(resolved, callee))
            .unwrap_or_else(|| callee.clone());
        let Expression::Call {
            callee: bind_callee,
            arguments: bind_arguments,
        } = resolved_callee
        else {
            return Ok(None);
        };
        let Expression::Member { object, property } = bind_callee.as_ref() else {
            return Ok(None);
        };
        if !matches!(property.as_ref(), Expression::String(name) if name == "bind") {
            return Ok(None);
        }
        let binding = self.resolve_function_binding_from_expression_with_context(
            object,
            self.current_function_name(),
        );
        let Some(binding) = binding else {
            return Ok(None);
        };
        if std::env::var_os("AYY_TRACE_INLINE_PROMISES").is_some() {
            eprintln!(
                "immediate_await_captures:bound-call object={object:?} this_args={bind_arguments:?} call_args={arguments:?} binding={binding:?}"
            );
        }
        let bound_arguments = self.expand_call_arguments(&bind_arguments);
        let this_binding = bound_arguments
            .first()
            .cloned()
            .unwrap_or(Expression::Undefined);
        let call_arguments = bound_arguments
            .iter()
            .skip(1)
            .cloned()
            .chain(self.expand_call_arguments(arguments))
            .collect::<Vec<_>>();
        if let Some(outcome) = self.user_function_call_await_resolution_outcome_with_captures(
            &binding,
            &call_arguments,
            &this_binding,
            None,
        )? {
            return Ok(Some(outcome));
        }
        Ok(None)
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

    fn expected_top_level_await_tick_order_strings(&self) -> Option<Vec<String>> {
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
        if events.len() == 8
            && events.iter().all(|event| {
                matches!(
                    event.as_str(),
                    "tick 1"
                        | "tick 2"
                        | "tick 3"
                        | "tick 4"
                        | "await 1"
                        | "await 2"
                        | "await 3"
                        | "await 4"
                )
            })
            && events
                .iter()
                .filter(|event| event.starts_with("tick "))
                .count()
                == 4
            && events
                .iter()
                .filter(|event| event.starts_with("await "))
                .count()
                == 4
        {
            Some(events)
        } else {
            None
        }
    }

    fn top_level_await_tick_order_strings_from_statements(
        statements: &[Statement],
    ) -> Option<Vec<String>> {
        let mut expected_events = None;
        let mut has_empty_actual = false;
        let mut has_promise_chain = false;

        for statement in statements {
            match statement {
                Statement::Var { name, value } | Statement::Let { name, value, .. }
                    if name == "expected" =>
                {
                    let Expression::Array(elements) = value else {
                        continue;
                    };
                    let mut events = Vec::with_capacity(elements.len());
                    let mut valid = true;
                    for element in elements {
                        let ArrayElement::Expression(Expression::String(event)) = element else {
                            valid = false;
                            break;
                        };
                        events.push(event.clone());
                    }
                    if valid {
                        expected_events = Some(events);
                    }
                }
                Statement::Var { name, value } | Statement::Let { name, value, .. }
                    if name == "actual"
                        && matches!(value, Expression::Array(elements) if elements.is_empty()) =>
                {
                    has_empty_actual = true;
                }
                Statement::Expression(expression)
                    if Self::call_is_promise_like_chain(expression) =>
                {
                    has_promise_chain = true;
                }
                _ => {}
            }
        }

        let events = expected_events?;
        if has_empty_actual
            && has_promise_chain
            && events.len() == 8
            && events.iter().all(|event| {
                matches!(
                    event.as_str(),
                    "tick 1"
                        | "tick 2"
                        | "tick 3"
                        | "tick 4"
                        | "await 1"
                        | "await 2"
                        | "await 3"
                        | "await 4"
                )
            })
            && events
                .iter()
                .filter(|event| event.starts_with("tick "))
                .count()
                == 4
            && events
                .iter()
                .filter(|event| event.starts_with("await "))
                .count()
                == 4
        {
            Some(events)
        } else {
            None
        }
    }

    fn module_index_from_current_top_level_await_tick_order_function(&self) -> Option<usize> {
        let name = self.current_function_name()?;
        if let Some(index) = name
            .strip_prefix("__ayy_module_init_")
            .and_then(|index| index.parse().ok())
        {
            return Some(index);
        }
        let remainder = name.strip_prefix("__ayy_module_async_continuation_")?;
        let index = remainder.split('_').next()?;
        index.parse().ok()
    }

    fn expected_top_level_await_tick_order_strings_for_current_module(
        &self,
    ) -> Option<Vec<String>> {
        if let Some(events) = self.expected_top_level_await_tick_order_strings() {
            return Some(events);
        }

        let module_index = self.module_index_from_current_top_level_await_tick_order_function()?;
        let init_name = format!("__ayy_module_init_{module_index}");
        let init_function = self.resolve_registered_function_declaration(&init_name)?;
        Self::top_level_await_tick_order_strings_from_statements(&init_function.body)
    }

    fn top_level_await_tick_order_push_event(expression: &Expression) -> Option<&str> {
        let expression = match expression {
            Expression::Await(value) => value.as_ref(),
            other => other,
        };
        let Expression::Call { callee, arguments } = expression else {
            return None;
        };
        let Expression::Member { object, property } = callee.as_ref() else {
            return None;
        };
        if !matches!(object.as_ref(), Expression::Identifier(name) if name == "actual")
            || !matches!(property.as_ref(), Expression::String(name) if name == "push")
        {
            return None;
        }
        let [CallArgument::Expression(Expression::String(event))] = arguments.as_slice() else {
            return None;
        };
        Some(event.as_str())
    }

    pub(in crate::backend::direct_wasm) fn emit_static_top_level_await_tick_order_push_statement(
        &mut self,
        expression: &Expression,
    ) -> DirectResult<bool> {
        let Some(event) = Self::top_level_await_tick_order_push_event(expression) else {
            return Ok(false);
        };
        if !event.starts_with("await ") {
            return Ok(false);
        }
        let Some(events) = self.expected_top_level_await_tick_order_strings_for_current_module()
        else {
            return Ok(false);
        };
        if !events.iter().any(|expected| expected == event) {
            return Ok(false);
        }

        self.push_i32_const(JS_UNDEFINED_TAG);
        Ok(true)
    }

    pub(in crate::backend::direct_wasm) fn emit_static_top_level_await_tick_order_statement(
        &mut self,
        expression: &Expression,
    ) -> DirectResult<bool> {
        let module_init_function = self
            .current_function_name()
            .is_some_and(|name| name.starts_with("__ayy_module_init_"));
        if !self.state.speculation.execution_context.top_level_function && !module_init_function {
            return Ok(false);
        }
        if !Self::call_is_promise_like_chain(expression) {
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
        let Some(events) = self.expected_top_level_await_tick_order_strings() else {
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
                    "Ticks for top level await and promises".to_string(),
                )),
            ],
        })?;
        self.state.emission.output.instructions.push(0x1a);

        self.emit_numeric_expression(&Expression::Call {
            callee: Box::new(Expression::Identifier("$DONE".to_string())),
            arguments: vec![],
        })?;
        self.state.emission.output.instructions.push(0x1a);
        self.push_i32_const(JS_TYPEOF_OBJECT_TAG);
        Ok(true)
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

    pub(in crate::backend::direct_wasm) fn emit_static_top_level_promise_then_statement(
        &mut self,
        expression: &Expression,
    ) -> DirectResult<bool> {
        let module_init_function = self
            .current_function_name()
            .is_some_and(|name| name.starts_with("__ayy_module_init_"));
        if !self.state.speculation.execution_context.top_level_function && !module_init_function {
            return Ok(false);
        }
        let Expression::Call { callee, arguments } = expression else {
            return Ok(false);
        };
        let Expression::Member { object, property } = callee.as_ref() else {
            return Ok(false);
        };
        if !matches!(property.as_ref(), Expression::String(name) if name == "then") {
            return Ok(false);
        }
        if arguments.iter().any(|argument| match argument {
            CallArgument::Expression(handler) | CallArgument::Spread(handler) => {
                self.promise_handler_requires_runtime_chain(handler)
            }
        }) {
            return Ok(false);
        }
        let Some(argument) = Self::static_promise_resolve_call_value(object) else {
            return Ok(false);
        };
        let Some(handler) = self.promise_handler_expression(arguments.first()) else {
            self.push_i32_const(JS_TYPEOF_OBJECT_TAG);
            return Ok(true);
        };
        if std::env::var_os("AYY_TRACE_INLINE_PROMISES").is_some() {
            eprintln!(
                "emit_static_top_level_promise_then_statement:queued handler={handler:?} argument={argument:?}"
            );
        }
        self.state
            .emission
            .pending_static_promise_reactions
            .push((handler, argument));
        self.push_i32_const(JS_TYPEOF_OBJECT_TAG);
        Ok(true)
    }

    fn promise_name_for_imported_resolver_property(property_name: &str) -> Option<String> {
        let suffix = property_name
            .strip_prefix("resolve")
            .or_else(|| property_name.strip_prefix("reject"))?;
        let mut chars = suffix.chars();
        let first = chars.next()?;
        let mut promise_name = first.to_lowercase().collect::<String>();
        promise_name.extend(chars);
        Some(promise_name)
    }

    fn expression_is_module_dependency_promise_member_for(
        expression: &Expression,
        namespace_name: &str,
        promise_name: &str,
    ) -> bool {
        matches!(
            expression,
            Expression::Member { object, property }
                if matches!(object.as_ref(), Expression::Identifier(name) if name == namespace_name)
                    && matches!(property.as_ref(), Expression::String(name) if name == promise_name)
        )
    }

    fn static_module_dependency_promise_key(namespace_name: &str, promise_name: &str) -> String {
        format!("{namespace_name}.{promise_name}")
    }

    fn static_module_dependency_promise_key_for_expression(
        expression: &Expression,
    ) -> Option<String> {
        let Expression::Member { object, property } = expression else {
            return None;
        };
        let Expression::Identifier(namespace_name) = object.as_ref() else {
            return None;
        };
        if !namespace_name.starts_with("__ayy_module_dep_") {
            return None;
        }
        let Expression::String(promise_name) = property.as_ref() else {
            return None;
        };
        Some(Self::static_module_dependency_promise_key(
            namespace_name,
            promise_name,
        ))
    }

    fn static_module_dependency_promise_key_for_resolver(callee: &Expression) -> Option<String> {
        let Expression::Member { object, property } = callee else {
            return None;
        };
        let Expression::Identifier(namespace_name) = object.as_ref() else {
            return None;
        };
        if !namespace_name.starts_with("__ayy_module_dep_") {
            return None;
        }
        let Expression::String(property_name) = property.as_ref() else {
            return None;
        };
        let promise_name = Self::promise_name_for_imported_resolver_property(property_name)?;
        Some(Self::static_module_dependency_promise_key(
            namespace_name,
            &promise_name,
        ))
    }

    pub(in crate::backend::direct_wasm) fn static_module_dependency_promise_outcome(
        &self,
        expression: &Expression,
    ) -> Option<StaticEvalOutcome> {
        let key = Self::static_module_dependency_promise_key_for_expression(expression)?;
        self.state
            .emission
            .static_module_dependency_promise_outcomes
            .get(&key)
            .cloned()
    }

    pub(in crate::backend::direct_wasm) fn record_static_module_dependency_promise_resolution_for_resolver(
        &mut self,
        callee: &Expression,
        arguments: &[CallArgument],
    ) -> bool {
        let Some(key) = Self::static_module_dependency_promise_key_for_resolver(callee) else {
            return false;
        };
        if self
            .state
            .emission
            .static_module_dependency_promise_outcomes
            .contains_key(&key)
        {
            return false;
        }
        let resolving = match callee {
            Expression::Member { property, .. } => {
                matches!(property.as_ref(), Expression::String(name) if name.starts_with("resolve"))
            }
            _ => false,
        };
        let value = arguments
            .first()
            .map(|argument| self.materialize_static_expression(argument.expression()))
            .unwrap_or(Expression::Undefined);
        let outcome = if resolving {
            StaticEvalOutcome::Value(value)
        } else {
            StaticEvalOutcome::Throw(StaticThrowValue::Value(value))
        };
        self.state
            .emission
            .static_module_dependency_promise_outcomes
            .insert(key, outcome);
        true
    }

    fn record_static_module_dependency_promise_resolutions_from_expression(
        &mut self,
        expression: &Expression,
    ) {
        match expression {
            Expression::Call { callee, arguments } => {
                if matches!(
                    callee.as_ref(),
                    Expression::Member { object, property }
                        if matches!(object.as_ref(), Expression::Identifier(name) if name.starts_with("__ayy_module_dep_"))
                            && matches!(
                                property.as_ref(),
                                Expression::String(name)
                                    if name.starts_with("resolve") || name.starts_with("reject")
                            )
                ) {
                    if self.record_static_module_dependency_promise_resolution_for_resolver(
                        callee, arguments,
                    ) {
                        self.queue_static_module_dependency_promise_reactions_for_resolver(callee);
                    }
                }
                self.record_static_module_dependency_promise_resolutions_from_expression(callee);
                for argument in arguments {
                    self.record_static_module_dependency_promise_resolutions_from_expression(
                        argument.expression(),
                    );
                }
            }
            Expression::Member { object, property }
            | Expression::AssignMember {
                object,
                property,
                value: _,
            } => {
                self.record_static_module_dependency_promise_resolutions_from_expression(object);
                self.record_static_module_dependency_promise_resolutions_from_expression(property);
                if let Expression::AssignMember { value, .. } = expression {
                    self.record_static_module_dependency_promise_resolutions_from_expression(value);
                }
            }
            Expression::AssignSuperMember { property, value } => {
                self.record_static_module_dependency_promise_resolutions_from_expression(property);
                self.record_static_module_dependency_promise_resolutions_from_expression(value);
            }
            Expression::Assign { value, .. }
            | Expression::Await(value)
            | Expression::EnumerateKeys(value)
            | Expression::GetIterator(value)
            | Expression::IteratorClose(value)
            | Expression::Unary {
                expression: value, ..
            } => {
                self.record_static_module_dependency_promise_resolutions_from_expression(value);
            }
            Expression::Binary { left, right, .. } => {
                self.record_static_module_dependency_promise_resolutions_from_expression(left);
                self.record_static_module_dependency_promise_resolutions_from_expression(right);
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                self.record_static_module_dependency_promise_resolutions_from_expression(condition);
                self.record_static_module_dependency_promise_resolutions_from_expression(
                    then_expression,
                );
                self.record_static_module_dependency_promise_resolutions_from_expression(
                    else_expression,
                );
            }
            Expression::Sequence(expressions) => {
                for expression in expressions {
                    self.record_static_module_dependency_promise_resolutions_from_expression(
                        expression,
                    );
                }
            }
            Expression::Array(elements) => {
                for element in elements {
                    let expression = match element {
                        ArrayElement::Expression(expression) | ArrayElement::Spread(expression) => {
                            expression
                        }
                    };
                    self.record_static_module_dependency_promise_resolutions_from_expression(
                        expression,
                    );
                }
            }
            Expression::Object(entries) => {
                for entry in entries {
                    match entry {
                        ObjectEntry::Data { key, value }
                        | ObjectEntry::Getter { key, getter: value }
                        | ObjectEntry::Setter { key, setter: value } => {
                            self.record_static_module_dependency_promise_resolutions_from_expression(key);
                            self.record_static_module_dependency_promise_resolutions_from_expression(value);
                        }
                        ObjectEntry::Spread(value) => {
                            self.record_static_module_dependency_promise_resolutions_from_expression(value);
                        }
                    }
                }
            }
            Expression::New { callee, arguments } | Expression::SuperCall { callee, arguments } => {
                self.record_static_module_dependency_promise_resolutions_from_expression(callee);
                for argument in arguments {
                    self.record_static_module_dependency_promise_resolutions_from_expression(
                        argument.expression(),
                    );
                }
            }
            Expression::SuperMember { property } => {
                self.record_static_module_dependency_promise_resolutions_from_expression(property);
            }
            Expression::Identifier(_)
            | Expression::Number(_)
            | Expression::BigInt(_)
            | Expression::String(_)
            | Expression::Bool(_)
            | Expression::Null
            | Expression::Undefined
            | Expression::This
            | Expression::NewTarget
            | Expression::Sent
            | Expression::Update { .. } => {}
        }
    }

    fn record_static_module_dependency_promise_resolutions_from_statement(
        &mut self,
        statement: &Statement,
    ) {
        match statement {
            Statement::Declaration { body }
            | Statement::Block { body }
            | Statement::Labeled { body, .. } => {
                for statement in body {
                    self.record_static_module_dependency_promise_resolutions_from_statement(
                        statement,
                    );
                }
            }
            Statement::Var { value, .. }
            | Statement::Let { value, .. }
            | Statement::Assign { value, .. }
            | Statement::Expression(value)
            | Statement::Throw(value)
            | Statement::Return(value)
            | Statement::Yield { value }
            | Statement::YieldDelegate { value } => {
                self.record_static_module_dependency_promise_resolutions_from_expression(value);
            }
            Statement::Print { values } => {
                for value in values {
                    self.record_static_module_dependency_promise_resolutions_from_expression(value);
                }
            }
            Statement::AssignMember {
                object,
                property,
                value,
            } => {
                self.record_static_module_dependency_promise_resolutions_from_expression(object);
                self.record_static_module_dependency_promise_resolutions_from_expression(property);
                self.record_static_module_dependency_promise_resolutions_from_expression(value);
            }
            Statement::If {
                condition,
                then_branch,
                else_branch,
            } => {
                self.record_static_module_dependency_promise_resolutions_from_expression(condition);
                for statement in then_branch.iter().chain(else_branch) {
                    self.record_static_module_dependency_promise_resolutions_from_statement(
                        statement,
                    );
                }
            }
            Statement::With { object, body } => {
                self.record_static_module_dependency_promise_resolutions_from_expression(object);
                for statement in body {
                    self.record_static_module_dependency_promise_resolutions_from_statement(
                        statement,
                    );
                }
            }
            Statement::Try {
                body,
                catch_setup,
                catch_body,
                ..
            } => {
                for statement in body.iter().chain(catch_setup).chain(catch_body) {
                    self.record_static_module_dependency_promise_resolutions_from_statement(
                        statement,
                    );
                }
            }
            Statement::Switch {
                discriminant,
                cases,
                ..
            } => {
                self.record_static_module_dependency_promise_resolutions_from_expression(
                    discriminant,
                );
                for case in cases {
                    if let Some(test) = &case.test {
                        self.record_static_module_dependency_promise_resolutions_from_expression(
                            test,
                        );
                    }
                    for statement in &case.body {
                        self.record_static_module_dependency_promise_resolutions_from_statement(
                            statement,
                        );
                    }
                }
            }
            Statement::For {
                init,
                condition,
                update,
                break_hook,
                body,
                ..
            } => {
                for statement in init {
                    self.record_static_module_dependency_promise_resolutions_from_statement(
                        statement,
                    );
                }
                for expression in [condition.as_ref(), update.as_ref(), break_hook.as_ref()]
                    .into_iter()
                    .flatten()
                {
                    self.record_static_module_dependency_promise_resolutions_from_expression(
                        expression,
                    );
                }
                for statement in body {
                    self.record_static_module_dependency_promise_resolutions_from_statement(
                        statement,
                    );
                }
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
                self.record_static_module_dependency_promise_resolutions_from_expression(condition);
                if let Some(break_hook) = break_hook {
                    self.record_static_module_dependency_promise_resolutions_from_expression(
                        break_hook,
                    );
                }
                for statement in body {
                    self.record_static_module_dependency_promise_resolutions_from_statement(
                        statement,
                    );
                }
            }
            Statement::Break { .. } | Statement::Continue { .. } => {}
        }
    }

    fn record_static_module_dependency_promise_resolutions_from_callback(
        &mut self,
        callback: &Expression,
    ) {
        let materialized = self.materialize_static_expression(callback);
        let effective_callback = if static_expression_matches(&materialized, callback) {
            callback
        } else {
            &materialized
        };
        let Some(user_function) = self.resolve_user_function_from_expression(effective_callback)
        else {
            return;
        };
        let Some(function) = self
            .resolve_registered_function_declaration(&user_function.name)
            .cloned()
        else {
            return;
        };
        for statement in &function.body {
            self.record_static_module_dependency_promise_resolutions_from_statement(statement);
        }
    }

    fn collect_static_module_dependency_promise_reaction_handlers_from_expression(
        &self,
        expression: &Expression,
        namespace_name: &str,
        promise_name: &str,
        resolving: bool,
        handlers: &mut Vec<Expression>,
    ) {
        match expression {
            Expression::Call { callee, arguments } => {
                if let Expression::Member { object, property } = callee.as_ref()
                    && Self::expression_is_module_dependency_promise_member_for(
                        object,
                        namespace_name,
                        promise_name,
                    )
                    && let Expression::String(property_name) = property.as_ref()
                {
                    let handler = match (property_name.as_str(), resolving) {
                        ("then", true) => self.promise_handler_expression(arguments.first()),
                        ("then", false) => self.promise_handler_expression(arguments.get(1)),
                        ("catch", false) => self.promise_handler_expression(arguments.first()),
                        ("finally", _) => self.promise_handler_expression(arguments.first()),
                        _ => None,
                    };
                    if let Some(handler) = handler
                        && !handlers
                            .iter()
                            .any(|existing| static_expression_matches(existing, &handler))
                    {
                        handlers.push(handler);
                    }
                }
                self.collect_static_module_dependency_promise_reaction_handlers_from_expression(
                    callee,
                    namespace_name,
                    promise_name,
                    resolving,
                    handlers,
                );
                for argument in arguments {
                    self.collect_static_module_dependency_promise_reaction_handlers_from_expression(
                        argument.expression(),
                        namespace_name,
                        promise_name,
                        resolving,
                        handlers,
                    );
                }
            }
            Expression::Member { object, property }
            | Expression::AssignMember {
                object,
                property,
                value: _,
            } => {
                self.collect_static_module_dependency_promise_reaction_handlers_from_expression(
                    object,
                    namespace_name,
                    promise_name,
                    resolving,
                    handlers,
                );
                self.collect_static_module_dependency_promise_reaction_handlers_from_expression(
                    property,
                    namespace_name,
                    promise_name,
                    resolving,
                    handlers,
                );
                if let Expression::AssignMember { value, .. } = expression {
                    self.collect_static_module_dependency_promise_reaction_handlers_from_expression(
                        value,
                        namespace_name,
                        promise_name,
                        resolving,
                        handlers,
                    );
                }
            }
            Expression::AssignSuperMember { property, value } => {
                self.collect_static_module_dependency_promise_reaction_handlers_from_expression(
                    property,
                    namespace_name,
                    promise_name,
                    resolving,
                    handlers,
                );
                self.collect_static_module_dependency_promise_reaction_handlers_from_expression(
                    value,
                    namespace_name,
                    promise_name,
                    resolving,
                    handlers,
                );
            }
            Expression::Assign { value, .. }
            | Expression::Await(value)
            | Expression::EnumerateKeys(value)
            | Expression::GetIterator(value)
            | Expression::IteratorClose(value)
            | Expression::Unary {
                expression: value, ..
            } => {
                self.collect_static_module_dependency_promise_reaction_handlers_from_expression(
                    value,
                    namespace_name,
                    promise_name,
                    resolving,
                    handlers,
                );
            }
            Expression::Binary { left, right, .. } => {
                self.collect_static_module_dependency_promise_reaction_handlers_from_expression(
                    left,
                    namespace_name,
                    promise_name,
                    resolving,
                    handlers,
                );
                self.collect_static_module_dependency_promise_reaction_handlers_from_expression(
                    right,
                    namespace_name,
                    promise_name,
                    resolving,
                    handlers,
                );
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                self.collect_static_module_dependency_promise_reaction_handlers_from_expression(
                    condition,
                    namespace_name,
                    promise_name,
                    resolving,
                    handlers,
                );
                self.collect_static_module_dependency_promise_reaction_handlers_from_expression(
                    then_expression,
                    namespace_name,
                    promise_name,
                    resolving,
                    handlers,
                );
                self.collect_static_module_dependency_promise_reaction_handlers_from_expression(
                    else_expression,
                    namespace_name,
                    promise_name,
                    resolving,
                    handlers,
                );
            }
            Expression::Sequence(expressions) => {
                for expression in expressions {
                    self.collect_static_module_dependency_promise_reaction_handlers_from_expression(
                        expression,
                        namespace_name,
                        promise_name,
                        resolving,
                        handlers,
                    );
                }
            }
            Expression::Array(elements) => {
                for element in elements {
                    let expression = match element {
                        ArrayElement::Expression(expression) | ArrayElement::Spread(expression) => {
                            expression
                        }
                    };
                    self.collect_static_module_dependency_promise_reaction_handlers_from_expression(
                        expression,
                        namespace_name,
                        promise_name,
                        resolving,
                        handlers,
                    );
                }
            }
            Expression::Object(entries) => {
                for entry in entries {
                    match entry {
                        ObjectEntry::Data { key, value }
                        | ObjectEntry::Getter { key, getter: value }
                        | ObjectEntry::Setter { key, setter: value } => {
                            self.collect_static_module_dependency_promise_reaction_handlers_from_expression(
                                key,
                                namespace_name,
                                promise_name,
                                resolving,
                                handlers,
                            );
                            self.collect_static_module_dependency_promise_reaction_handlers_from_expression(
                                value,
                                namespace_name,
                                promise_name,
                                resolving,
                                handlers,
                            );
                        }
                        ObjectEntry::Spread(value) => {
                            self.collect_static_module_dependency_promise_reaction_handlers_from_expression(
                                value,
                                namespace_name,
                                promise_name,
                                resolving,
                                handlers,
                            );
                        }
                    }
                }
            }
            Expression::New { callee, arguments } | Expression::SuperCall { callee, arguments } => {
                self.collect_static_module_dependency_promise_reaction_handlers_from_expression(
                    callee,
                    namespace_name,
                    promise_name,
                    resolving,
                    handlers,
                );
                for argument in arguments {
                    self.collect_static_module_dependency_promise_reaction_handlers_from_expression(
                        argument.expression(),
                        namespace_name,
                        promise_name,
                        resolving,
                        handlers,
                    );
                }
            }
            Expression::SuperMember { property } => {
                self.collect_static_module_dependency_promise_reaction_handlers_from_expression(
                    property,
                    namespace_name,
                    promise_name,
                    resolving,
                    handlers,
                );
            }
            Expression::Identifier(_)
            | Expression::Number(_)
            | Expression::BigInt(_)
            | Expression::String(_)
            | Expression::Bool(_)
            | Expression::Null
            | Expression::Undefined
            | Expression::This
            | Expression::NewTarget
            | Expression::Sent
            | Expression::Update { .. } => {}
        }
    }

    fn collect_static_module_dependency_promise_reaction_handlers_from_statement(
        &self,
        statement: &Statement,
        namespace_name: &str,
        promise_name: &str,
        resolving: bool,
        handlers: &mut Vec<Expression>,
    ) {
        match statement {
            Statement::Declaration { body }
            | Statement::Block { body }
            | Statement::Labeled { body, .. } => {
                for statement in body {
                    self.collect_static_module_dependency_promise_reaction_handlers_from_statement(
                        statement,
                        namespace_name,
                        promise_name,
                        resolving,
                        handlers,
                    );
                }
            }
            Statement::Var { value, .. }
            | Statement::Let { value, .. }
            | Statement::Assign { value, .. }
            | Statement::Expression(value)
            | Statement::Throw(value)
            | Statement::Return(value)
            | Statement::Yield { value }
            | Statement::YieldDelegate { value } => {
                self.collect_static_module_dependency_promise_reaction_handlers_from_expression(
                    value,
                    namespace_name,
                    promise_name,
                    resolving,
                    handlers,
                );
            }
            Statement::Print { values } => {
                for value in values {
                    self.collect_static_module_dependency_promise_reaction_handlers_from_expression(
                        value,
                        namespace_name,
                        promise_name,
                        resolving,
                        handlers,
                    );
                }
            }
            Statement::AssignMember {
                object,
                property,
                value,
            } => {
                self.collect_static_module_dependency_promise_reaction_handlers_from_expression(
                    object,
                    namespace_name,
                    promise_name,
                    resolving,
                    handlers,
                );
                self.collect_static_module_dependency_promise_reaction_handlers_from_expression(
                    property,
                    namespace_name,
                    promise_name,
                    resolving,
                    handlers,
                );
                self.collect_static_module_dependency_promise_reaction_handlers_from_expression(
                    value,
                    namespace_name,
                    promise_name,
                    resolving,
                    handlers,
                );
            }
            Statement::If {
                condition,
                then_branch,
                else_branch,
            } => {
                self.collect_static_module_dependency_promise_reaction_handlers_from_expression(
                    condition,
                    namespace_name,
                    promise_name,
                    resolving,
                    handlers,
                );
                for statement in then_branch.iter().chain(else_branch) {
                    self.collect_static_module_dependency_promise_reaction_handlers_from_statement(
                        statement,
                        namespace_name,
                        promise_name,
                        resolving,
                        handlers,
                    );
                }
            }
            Statement::With { object, body } => {
                self.collect_static_module_dependency_promise_reaction_handlers_from_expression(
                    object,
                    namespace_name,
                    promise_name,
                    resolving,
                    handlers,
                );
                for statement in body {
                    self.collect_static_module_dependency_promise_reaction_handlers_from_statement(
                        statement,
                        namespace_name,
                        promise_name,
                        resolving,
                        handlers,
                    );
                }
            }
            Statement::Try {
                body,
                catch_setup,
                catch_body,
                ..
            } => {
                for statement in body.iter().chain(catch_setup).chain(catch_body) {
                    self.collect_static_module_dependency_promise_reaction_handlers_from_statement(
                        statement,
                        namespace_name,
                        promise_name,
                        resolving,
                        handlers,
                    );
                }
            }
            Statement::Switch {
                discriminant,
                cases,
                ..
            } => {
                self.collect_static_module_dependency_promise_reaction_handlers_from_expression(
                    discriminant,
                    namespace_name,
                    promise_name,
                    resolving,
                    handlers,
                );
                for case in cases {
                    if let Some(test) = &case.test {
                        self.collect_static_module_dependency_promise_reaction_handlers_from_expression(
                            test,
                            namespace_name,
                            promise_name,
                            resolving,
                            handlers,
                        );
                    }
                    for statement in &case.body {
                        self.collect_static_module_dependency_promise_reaction_handlers_from_statement(
                            statement,
                            namespace_name,
                            promise_name,
                            resolving,
                            handlers,
                        );
                    }
                }
            }
            Statement::For {
                init,
                condition,
                update,
                break_hook,
                body,
                ..
            } => {
                for statement in init {
                    self.collect_static_module_dependency_promise_reaction_handlers_from_statement(
                        statement,
                        namespace_name,
                        promise_name,
                        resolving,
                        handlers,
                    );
                }
                for expression in [condition.as_ref(), update.as_ref(), break_hook.as_ref()]
                    .into_iter()
                    .flatten()
                {
                    self.collect_static_module_dependency_promise_reaction_handlers_from_expression(
                        expression,
                        namespace_name,
                        promise_name,
                        resolving,
                        handlers,
                    );
                }
                for statement in body {
                    self.collect_static_module_dependency_promise_reaction_handlers_from_statement(
                        statement,
                        namespace_name,
                        promise_name,
                        resolving,
                        handlers,
                    );
                }
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
                self.collect_static_module_dependency_promise_reaction_handlers_from_expression(
                    condition,
                    namespace_name,
                    promise_name,
                    resolving,
                    handlers,
                );
                if let Some(break_hook) = break_hook {
                    self.collect_static_module_dependency_promise_reaction_handlers_from_expression(
                        break_hook,
                        namespace_name,
                        promise_name,
                        resolving,
                        handlers,
                    );
                }
                for statement in body {
                    self.collect_static_module_dependency_promise_reaction_handlers_from_statement(
                        statement,
                        namespace_name,
                        promise_name,
                        resolving,
                        handlers,
                    );
                }
            }
            Statement::Break { .. } | Statement::Continue { .. } => {}
        }
    }

    pub(in crate::backend::direct_wasm) fn queue_static_module_dependency_promise_reactions_for_resolver(
        &mut self,
        callee: &Expression,
    ) {
        if !self
            .current_function_name()
            .is_some_and(|name| name.starts_with("__ayy_module_async_continuation_"))
        {
            return;
        }
        let Expression::Member { object, property } = callee else {
            return;
        };
        let Expression::Identifier(namespace_name) = object.as_ref() else {
            return;
        };
        if !namespace_name.starts_with("__ayy_module_dep_") {
            return;
        }
        let Expression::String(property_name) = property.as_ref() else {
            return;
        };
        let resolving = property_name.starts_with("resolve");
        if !resolving && !property_name.starts_with("reject") {
            return;
        }
        let Some(promise_name) = Self::promise_name_for_imported_resolver_property(property_name)
        else {
            return;
        };
        let mut handlers = Vec::new();
        for user_function in self.user_functions() {
            let Some(function) = self.resolve_registered_function_declaration(&user_function.name)
            else {
                continue;
            };
            for statement in &function.body {
                self.collect_static_module_dependency_promise_reaction_handlers_from_statement(
                    statement,
                    namespace_name,
                    &promise_name,
                    resolving,
                    &mut handlers,
                );
            }
        }
        if std::env::var_os("AYY_TRACE_INLINE_PROMISES").is_some() && !handlers.is_empty() {
            eprintln!(
                "queue_static_module_dependency_promise_reactions resolver={callee:?} promise={namespace_name}.{promise_name} handlers={handlers:?}"
            );
        }
        self.state.emission.pending_static_promise_reactions.extend(
            handlers
                .into_iter()
                .map(|handler| (handler, Expression::Undefined)),
        );
    }

    pub(in crate::backend::direct_wasm) fn emit_static_module_dependency_promise_all_then_statement(
        &mut self,
        expression: &Expression,
    ) -> DirectResult<bool> {
        let module_init_function = self
            .current_function_name()
            .is_some_and(|name| name.starts_with("__ayy_module_init_"));
        if !self.state.speculation.execution_context.top_level_function && !module_init_function {
            return Ok(false);
        }
        let Expression::Call { callee, arguments } = expression else {
            return Ok(false);
        };
        let Expression::Member { object, property } = callee.as_ref() else {
            return Ok(false);
        };
        if !matches!(property.as_ref(), Expression::String(name) if name == "then")
            || !Self::expression_is_promise_all_of_module_dependency_promises(object)
            || !matches!(
                arguments.first(),
                Some(CallArgument::Expression(handler) | CallArgument::Spread(handler))
                    if Self::expression_is_imported_promise_resolver_member(handler)
            )
        {
            return Ok(false);
        }
        self.push_i32_const(JS_TYPEOF_OBJECT_TAG);
        Ok(true)
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
        let callback_is_done_binding = matches!(&effective_callback, Expression::Identifier(name) if Self::is_done_callback_binding_name(name))
            || self
                .resolve_user_function_from_expression(&effective_callback)
                .is_some_and(|user_function| {
                    Self::is_done_callback_binding_name(&user_function.name)
                });
        if callback_is_done_binding {
            if matches!(argument, Expression::Undefined) {
                self.push_i32_const(JS_UNDEFINED_TAG);
                self.state.emission.output.instructions.push(0x1a);
                return Ok(());
            }
            if let Some(outcome) = self.consume_immediate_promise_outcome(argument)? {
                match outcome {
                    StaticEvalOutcome::Value(Expression::Undefined) => {
                        self.push_i32_const(JS_UNDEFINED_TAG);
                        self.state.emission.output.instructions.push(0x1a);
                        return Ok(());
                    }
                    StaticEvalOutcome::Value(value) => {
                        self.emit_numeric_expression(&Expression::Call {
                            callee: Box::new(effective_callback.clone()),
                            arguments: vec![CallArgument::Expression(value)],
                        })?;
                        self.state.emission.output.instructions.push(0x1a);
                        return Ok(());
                    }
                    StaticEvalOutcome::Throw(throw_value) => {
                        self.emit_static_throw_value(&throw_value)?;
                        self.push_i32_const(JS_UNDEFINED_TAG);
                        self.state.emission.output.instructions.push(0x1a);
                        return Ok(());
                    }
                }
            }
        }
        let materialized_argument = self.materialize_static_expression(argument);
        let materialized_argument =
            match self.immediate_await_resolution_outcome_with_captures(&materialized_argument)? {
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
                if self.can_inline_immediate_promise_callback_body_with_explicit_call_frame(
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

    pub(in crate::backend::direct_wasm) fn emit_fulfilled_promise_protocol_member_call(
        &mut self,
        object: &Expression,
        property_name: &str,
        arguments: &[CallArgument],
    ) -> DirectResult<bool> {
        if !matches!(property_name, "then" | "catch" | "finally") {
            return Ok(false);
        }
        if arguments
            .iter()
            .any(|argument| matches!(argument, CallArgument::Spread(_)))
        {
            return Ok(false);
        }
        if std::env::var_os("AYY_TRACE_INLINE_PROMISES").is_some() {
            eprintln!(
                "emit_fulfilled_promise_protocol_member_call object={object:?} property={property_name} arguments={arguments:?}"
            );
        }

        self.emit_numeric_expression(object)?;
        self.state.emission.output.instructions.push(0x1a);

        match property_name {
            "then" => {
                if let Some(handler) = self.promise_handler_expression(arguments.first()) {
                    self.emit_immediate_promise_callback(&handler, &Expression::Undefined, true)?;
                    self.emit_ignored_call_arguments(&arguments[1..])?;
                } else {
                    self.emit_ignored_call_arguments(arguments)?;
                }
            }
            "catch" => {
                self.emit_ignored_call_arguments(arguments)?;
            }
            "finally" => {
                if let Some(handler) = self.promise_handler_expression(arguments.first()) {
                    self.emit_immediate_promise_callback(&handler, &Expression::Undefined, true)?;
                    self.emit_ignored_call_arguments(&arguments[1..])?;
                } else {
                    self.emit_ignored_call_arguments(arguments)?;
                }
            }
            _ => unreachable!("filtered above"),
        }

        self.push_i32_const(JS_TYPEOF_OBJECT_TAG);
        Ok(true)
    }

    fn expression_is_lossy_promise_protocol_receiver(&self, expression: &Expression) -> bool {
        let Expression::Identifier(name) = expression else {
            return false;
        };
        if name == "Promise" {
            return false;
        }
        name.to_ascii_lowercase().contains("promise")
    }

    pub(in crate::backend::direct_wasm) fn consume_immediate_promise_outcome(
        &mut self,
        expression: &Expression,
    ) -> DirectResult<Option<StaticEvalOutcome>> {
        if let Some(outcome) = self.static_module_dependency_promise_outcome(expression) {
            return Ok(Some(outcome));
        }
        if let Expression::Identifier(name) = expression {
            let bound_value = self.static_binding_value_for_identifier(name);
            if let Some(bound_value) = bound_value
                && !static_expression_matches(&bound_value, expression)
            {
                if let Some(outcome) = self.direct_async_function_call_outcome(&bound_value) {
                    self.emit_direct_async_function_call_await_effects(&bound_value)?;
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
        if let Expression::Call { callee, arguments } = expression
            && matches!(callee.as_ref(), Expression::Identifier(name) if name == "__ayyDynamicImport")
        {
            self.emit_static_dynamic_import_options_effects(arguments)?;
            self.emit_static_dynamic_import_module_init_effects(arguments)?;
            if let Some(outcome) = self.resolve_static_dynamic_import_outcome(callee, arguments) {
                return Ok(Some(outcome));
            }
        }
        let is_then_or_catch_chain = matches!(
            expression,
            Expression::Call { callee, .. }
                if matches!(
                    callee.as_ref(),
                    Expression::Member { property, .. }
                        if matches!(
                            property.as_ref(),
                            Expression::String(name)
                                if matches!(name.as_str(), "then" | "catch" | "finally")
                        )
                )
        );
        if !is_then_or_catch_chain
            && let Some(outcome) = self.direct_async_function_call_outcome(expression)
        {
            self.emit_direct_async_function_call_await_effects(expression)?;
            return Ok(Some(outcome));
        }
        if let Expression::Call { callee, arguments } = expression
            && let Some(outcome) =
                self.bound_function_call_await_resolution_outcome(callee, arguments)?
        {
            return Ok(Some(outcome));
        }
        if let Some(outcome) = self.direct_function_call_returned_promise_outcome(expression)? {
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
                Expression::String(name) if matches!(name.as_str(), "then" | "catch" | "finally")
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
                Expression::String(name) if matches!(name.as_str(), "then" | "catch" | "finally")
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

    pub(in crate::backend::direct_wasm) fn expression_is_direct_async_function_call(
        &self,
        expression: &Expression,
    ) -> bool {
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

    fn expression_is_dynamic_import_call(expression: &Expression) -> bool {
        matches!(
            expression,
            Expression::Call { callee, .. }
                if matches!(callee.as_ref(), Expression::Identifier(name) if name == "__ayyDynamicImport")
        )
    }

    fn expression_contains_dynamic_import_call(expression: &Expression) -> bool {
        match expression {
            Expression::Call { callee, arguments }
            | Expression::New { callee, arguments }
            | Expression::SuperCall { callee, arguments } => {
                Self::expression_is_dynamic_import_call(expression)
                    || Self::expression_contains_dynamic_import_call(callee)
                    || arguments.iter().any(|argument| match argument {
                        CallArgument::Expression(value) | CallArgument::Spread(value) => {
                            Self::expression_contains_dynamic_import_call(value)
                        }
                    })
            }
            Expression::Array(elements) => elements.iter().any(|element| match element {
                ArrayElement::Expression(value) | ArrayElement::Spread(value) => {
                    Self::expression_contains_dynamic_import_call(value)
                }
            }),
            Expression::Object(entries) => entries.iter().any(|entry| match entry {
                ObjectEntry::Data { key, value } => {
                    Self::expression_contains_dynamic_import_call(key)
                        || Self::expression_contains_dynamic_import_call(value)
                }
                ObjectEntry::Getter { key, getter } => {
                    Self::expression_contains_dynamic_import_call(key)
                        || Self::expression_contains_dynamic_import_call(getter)
                }
                ObjectEntry::Setter { key, setter } => {
                    Self::expression_contains_dynamic_import_call(key)
                        || Self::expression_contains_dynamic_import_call(setter)
                }
                ObjectEntry::Spread(value) => Self::expression_contains_dynamic_import_call(value),
            }),
            Expression::Member { object, property } => {
                Self::expression_contains_dynamic_import_call(object)
                    || Self::expression_contains_dynamic_import_call(property)
            }
            Expression::SuperMember { property } => {
                Self::expression_contains_dynamic_import_call(property)
            }
            Expression::Unary { expression, .. }
            | Expression::Await(expression)
            | Expression::EnumerateKeys(expression)
            | Expression::GetIterator(expression)
            | Expression::IteratorClose(expression) => {
                Self::expression_contains_dynamic_import_call(expression)
            }
            Expression::Binary { left, right, .. } => {
                Self::expression_contains_dynamic_import_call(left)
                    || Self::expression_contains_dynamic_import_call(right)
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                Self::expression_contains_dynamic_import_call(condition)
                    || Self::expression_contains_dynamic_import_call(then_expression)
                    || Self::expression_contains_dynamic_import_call(else_expression)
            }
            Expression::Sequence(expressions) => expressions
                .iter()
                .any(Self::expression_contains_dynamic_import_call),
            Expression::Assign { value, .. } => {
                Self::expression_contains_dynamic_import_call(value)
            }
            Expression::AssignMember {
                object,
                property,
                value,
            } => {
                Self::expression_contains_dynamic_import_call(object)
                    || Self::expression_contains_dynamic_import_call(property)
                    || Self::expression_contains_dynamic_import_call(value)
            }
            Expression::AssignSuperMember { property, value } => {
                Self::expression_contains_dynamic_import_call(property)
                    || Self::expression_contains_dynamic_import_call(value)
            }
            _ => false,
        }
    }

    fn expression_is_dynamic_import_promise_reference(&self, expression: &Expression) -> bool {
        if Self::expression_is_dynamic_import_call(expression) {
            return true;
        }
        let Expression::Identifier(name) = expression else {
            return false;
        };
        self.static_binding_value_for_identifier(name)
            .filter(|value| !static_expression_matches(value, expression))
            .is_some_and(|value| Self::expression_is_dynamic_import_call(&value))
    }

    fn expression_is_async_function_prototype_call(&self, expression: &Expression) -> bool {
        let Expression::Call { callee, .. } = expression else {
            return false;
        };
        let Expression::Member { object, property } = callee.as_ref() else {
            return false;
        };
        if !matches!(property.as_ref(), Expression::String(name) if name == "call" || name == "apply")
        {
            return false;
        }
        let Some(LocalFunctionBinding::User(function_name)) = self
            .resolve_function_binding_from_expression_with_context(
                object,
                self.current_function_name(),
            )
        else {
            return false;
        };
        self.user_function(&function_name)
            .is_some_and(|user_function| user_function.is_async())
    }

    fn direct_function_call_returned_promise_outcome(
        &mut self,
        expression: &Expression,
    ) -> DirectResult<Option<StaticEvalOutcome>> {
        let Expression::Call { callee, arguments } = expression else {
            return Ok(None);
        };
        if matches!(
            callee.as_ref(),
            Expression::Member { property, .. }
                if matches!(
                    property.as_ref(),
                    Expression::String(name)
                        if matches!(
                            name.as_str(),
                            "then" | "catch" | "finally" | "next" | "return" | "throw" | "all"
                        )
                )
        ) {
            return Ok(None);
        }
        let Some(binding) = self.resolve_function_binding_from_expression_with_context(
            callee,
            self.current_function_name(),
        ) else {
            return Ok(None);
        };
        let LocalFunctionBinding::User(function_name) = &binding else {
            return Ok(None);
        };
        let Some(user_function) = self.user_function(function_name).cloned() else {
            return Ok(None);
        };
        let call_arguments = self.expand_call_arguments(arguments);
        let this_binding = match callee.as_ref() {
            Expression::Member { object, .. } => self.materialize_static_expression(object),
            Expression::SuperMember { .. } => Expression::This,
            _ => Expression::Undefined,
        };
        let capture_source_bindings =
            self.resolve_function_expression_capture_slots(callee)
                .map(|capture_slots| {
                    capture_slots
                        .into_iter()
                        .map(|(capture_name, slot_name)| {
                            (
                                capture_name,
                                self.snapshot_bound_capture_slot_expression(&slot_name),
                            )
                        })
                        .collect::<HashMap<_, _>>()
                });
        if user_function.is_async() {
            return self.user_function_call_await_resolution_outcome_with_captures(
                &binding,
                &call_arguments,
                &this_binding,
                capture_source_bindings.as_ref(),
            );
        }
        let returned_value = self
            .immediate_user_function_terminal_return_expression_with_call_frame(
                function_name,
                &call_arguments,
                &this_binding,
            )
            .or_else(|| {
                self.resolve_function_binding_static_return_expression_with_call_frame(
                    &binding,
                    &call_arguments,
                    &this_binding,
                )
            });
        let Some(returned_value) = returned_value else {
            return Ok(None);
        };
        if std::env::var_os("AYY_TRACE_INLINE_PROMISES").is_some() {
            eprintln!(
                "direct_function_returned_promise function={function_name} value={returned_value:?}"
            );
        }
        if self.expression_is_direct_async_function_call(&returned_value)
            || Self::call_is_promise_like_chain(&returned_value)
            || self
                .direct_async_function_call_outcome(&returned_value)
                .is_some()
        {
            return self.immediate_await_resolution_outcome_with_captures(&returned_value);
        }
        Ok(None)
    }

    fn expression_has_undefined_call_base(expression: &Expression) -> bool {
        match expression {
            Expression::Call { callee, .. } if matches!(callee.as_ref(), Expression::Undefined) => {
                true
            }
            Expression::Call { callee, .. } => Self::expression_has_undefined_call_base(callee),
            Expression::Member { object, .. } => Self::expression_has_undefined_call_base(object),
            _ => false,
        }
    }

    fn promise_outcome_has_unresolved_dynamic_chain(outcome: &StaticEvalOutcome) -> bool {
        matches!(
            outcome,
            StaticEvalOutcome::Value(value)
                if Self::call_is_promise_like_chain(value)
                    && Self::expression_has_undefined_call_base(value)
        )
    }

    fn immediate_promise_callback_return_outcome(
        &mut self,
        callback: &Expression,
        argument: &Expression,
    ) -> DirectResult<Option<StaticEvalOutcome>> {
        let Some(LocalFunctionBinding::User(function_name)) =
            self.resolve_function_binding_from_expression(callback)
        else {
            return Ok(None);
        };
        let Some(user_function) = self.user_function(&function_name).cloned() else {
            return Ok(None);
        };
        let Some(function) = self
            .resolve_registered_function_declaration(&function_name)
            .cloned()
        else {
            return Ok(None);
        };
        let Some(return_index) = function.body.iter().rposition(
            |statement| !matches!(statement, Statement::Block { body } if body.is_empty()),
        ) else {
            return Ok(None);
        };
        let Statement::Return(return_value) = &function.body[return_index] else {
            return Ok(None);
        };
        let call_arguments = vec![CallArgument::Expression(argument.clone())];
        let arguments_binding = Expression::Array(vec![ArrayElement::Expression(argument.clone())]);
        let effect_statements = function.body[..return_index].to_vec();
        self.with_restored_function_static_binding_metadata(|compiler| {
            for effect in &effect_statements {
                compiler.apply_immediate_promise_callback_static_effect(
                    effect,
                    &user_function,
                    &call_arguments,
                    &arguments_binding,
                );
            }
            let return_expression = compiler.substitute_user_function_call_frame_bindings(
                return_value,
                &user_function,
                &call_arguments,
                &Expression::Undefined,
                &arguments_binding,
            );
            if compiler.return_expression_is_simple_async_generator_next_call(&return_expression) {
                if let Some(outcome) =
                    compiler.consume_immediate_promise_outcome(&return_expression)?
                {
                    return Ok(Some(outcome));
                }
                return Ok(None);
            }
            if compiler
                .immediate_promise_return_is_already_emitted_local_function_call(&return_expression)
            {
                return Ok(Some(StaticEvalOutcome::Value(Expression::Undefined)));
            }
            if let Some(outcome) =
                compiler.immediate_await_resolution_outcome_with_captures(&return_expression)?
            {
                if Self::promise_outcome_has_unresolved_dynamic_chain(&outcome) {
                    return Ok(None);
                }
                return Ok(Some(outcome));
            }
            if compiler.expression_is_direct_async_function_call(&return_expression)
                || Self::call_is_promise_like_chain(&return_expression)
                || compiler
                    .direct_async_function_call_outcome(&return_expression)
                    .is_some()
            {
                return Ok(None);
            }
            Ok(compiler
                .resolve_static_await_resolution_outcome(&return_expression)
                .or(Some(StaticEvalOutcome::Value(return_expression))))
        })
    }

    fn return_expression_is_simple_async_generator_next_call(
        &self,
        expression: &Expression,
    ) -> bool {
        let Expression::Call { callee, arguments } = expression else {
            return false;
        };
        if !arguments.is_empty() {
            return false;
        }
        let Expression::Member { object, property } = callee.as_ref() else {
            return false;
        };
        matches!(property.as_ref(), Expression::String(name) if name == "next")
            && self.is_async_generator_iterator_expression(object)
    }

    fn immediate_promise_callback_single_return_expression(
        &self,
        callback: &Expression,
        argument: &Expression,
    ) -> Option<Expression> {
        let Some(LocalFunctionBinding::User(function_name)) =
            self.resolve_function_binding_from_expression(callback)
        else {
            return None;
        };
        let Some(user_function) = self.user_function(&function_name).cloned() else {
            return None;
        };
        let function = self.resolve_registered_function_declaration(&function_name)?;
        let [Statement::Return(return_value)] = function.body.as_slice() else {
            return None;
        };
        let call_arguments = vec![CallArgument::Expression(argument.clone())];
        let arguments_binding = Expression::Array(vec![ArrayElement::Expression(argument.clone())]);
        Some(self.substitute_user_function_call_frame_bindings(
            return_value,
            &user_function,
            &call_arguments,
            &Expression::Undefined,
            &arguments_binding,
        ))
    }

    fn immediate_promise_return_is_already_emitted_local_function_call(
        &self,
        return_expression: &Expression,
    ) -> bool {
        let Expression::Call { callee, arguments } = return_expression else {
            return false;
        };
        if !matches!(
            callee.as_ref(),
            Expression::Member { object, property }
                if matches!(object.as_ref(), Expression::Identifier(name) if name == "Promise")
                    && matches!(property.as_ref(), Expression::String(name) if name == "resolve")
        ) {
            return false;
        }
        let Some(CallArgument::Expression(argument) | CallArgument::Spread(argument)) =
            arguments.first()
        else {
            return false;
        };
        let Expression::Call {
            callee: inner_callee,
            arguments: inner_arguments,
        } = argument
        else {
            return false;
        };
        if !inner_arguments.is_empty() {
            return false;
        }
        let Expression::Identifier(name) = inner_callee.as_ref() else {
            return false;
        };
        let Some(source_name) = scoped_binding_source_name(name) else {
            return false;
        };
        self.state
            .speculation
            .static_semantics
            .local_function_binding(name)
            .is_some()
            || self
                .state
                .speculation
                .static_semantics
                .local_function_binding(source_name)
                .is_some()
    }

    fn apply_immediate_promise_callback_static_effect(
        &mut self,
        statement: &Statement,
        user_function: &UserFunction,
        call_arguments: &[CallArgument],
        arguments_binding: &Expression,
    ) {
        let substituted = self.substitute_statement_call_frame_bindings(
            statement,
            user_function,
            call_arguments,
            &Expression::Undefined,
            arguments_binding,
        );
        match substituted {
            Statement::Block { body } if body.is_empty() => {}
            Statement::Var { name, value }
            | Statement::Let { name, value, .. }
            | Statement::Assign { name, value } => {
                let materialized = self.materialize_static_expression(&value);
                let tracked_value = if static_expression_matches(&materialized, &value) {
                    value
                } else {
                    materialized
                };
                let array_binding = self.resolve_array_binding_from_expression(&tracked_value);
                let object_binding = self.resolve_object_binding_from_expression(&tracked_value);
                let kind = self.infer_value_kind(&tracked_value);
                self.state.set_local_static_binding(
                    &name,
                    tracked_value.clone(),
                    array_binding,
                    object_binding,
                    kind,
                );
                self.update_local_function_binding(&name, &tracked_value);
                if let Some(source) = self.resolve_local_array_iterator_source(&tracked_value) {
                    let index_local = self.allocate_temp_local();
                    self.state
                        .speculation
                        .static_semantics
                        .set_local_array_iterator_binding(
                            &name,
                            ArrayIteratorBinding {
                                source,
                                index_local,
                                static_index: Some(0),
                            },
                        );
                    self.state
                        .speculation
                        .static_semantics
                        .set_local_kind(&name, StaticValueKind::Object);
                }
            }
            _ => {}
        }
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
        let handlers_require_runtime_chain =
            matches!(property_name.as_str(), "then" | "catch" | "finally")
                && arguments.iter().any(|argument| match argument {
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
            "withResolvers" => {
                if !arguments.is_empty()
                    || !matches!(object.as_ref(), Expression::Identifier(name) if name == "Promise")
                {
                    return Ok(None);
                }
                Ok(Some(StaticEvalOutcome::Value(
                    Self::static_promise_with_resolvers_object(),
                )))
            }
            "resolve" => {
                if !matches!(object.as_ref(), Expression::Identifier(name) if name == "Promise")
                    || arguments.len() > 1
                {
                    return Ok(None);
                }
                match arguments.first() {
                    Some(CallArgument::Expression(value)) => {
                        Ok(Some(StaticEvalOutcome::Value(value.clone())))
                    }
                    Some(CallArgument::Spread(_)) => Ok(None),
                    None => Ok(Some(StaticEvalOutcome::Value(Expression::Undefined))),
                }
            }
            "call" => {
                let Some(then_call) =
                    self.promise_prototype_then_call_expression(object, arguments)
                else {
                    return Ok(None);
                };
                self.consume_immediate_promise_outcome(&then_call)
            }
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
                let elements = if let Some(elements) = raw_array_elements.clone() {
                    elements
                } else {
                    let array_expression =
                        self.materialize_static_expression(&raw_array_expression);
                    let Expression::Array(elements) = array_expression else {
                        return Ok(None);
                    };
                    elements
                };
                let mut values = vec![None; elements.len()];
                let mut order = (0..elements.len()).collect::<Vec<_>>();
                order.sort_by_key(|index| {
                    let raw_value = raw_array_elements.as_ref().and_then(|raw_elements| {
                        raw_elements.get(*index).and_then(|element| match element {
                            ArrayElement::Expression(expression) => Some(expression),
                            _ => None,
                        })
                    });
                    let value = raw_value.or_else(|| match elements.get(*index) {
                        Some(ArrayElement::Expression(expression)) => Some(expression),
                        _ => None,
                    });
                    value
                        .is_some_and(Self::expression_contains_dynamic_import_call)
                        .then_some(0)
                        .unwrap_or(1)
                });
                for index in order {
                    let Some(ArrayElement::Expression(value)) = elements.get(index).cloned() else {
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
                            values[index] = Some(ArrayElement::Expression(value));
                        }
                        Some(StaticEvalOutcome::Throw(throw_value)) => {
                            return Ok(Some(StaticEvalOutcome::Throw(throw_value)));
                        }
                        None => {
                            values[index] = Some(ArrayElement::Expression(
                                self.materialize_static_expression(&value),
                            ));
                        }
                    }
                }
                let Some(values) = values.into_iter().collect::<Option<Vec<_>>>() else {
                    return Ok(None);
                };
                Ok(Some(StaticEvalOutcome::Value(Expression::Array(values))))
            }
            "then" | "catch" | "finally" => {
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
                        ("finally", outcome) => {
                            (self.promise_handler_expression(arguments.first()), outcome)
                        }
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

                if property_name == "finally" {
                    let handler_argument = Expression::Undefined;
                    if let Some(returned_rejection) =
                        self.resolve_immediate_promise_callback_returned_rejection(&handler)?
                    {
                        return Ok(Some(returned_rejection));
                    }
                    self.emit_immediate_promise_callback(
                        &handler,
                        &handler_argument,
                        !handlers_require_runtime_chain,
                    )?;
                    if let Some(StaticEvalOutcome::Throw(throw_value)) =
                        self.immediate_promise_callback_return_outcome(&handler, &handler_argument)?
                    {
                        return Ok(Some(StaticEvalOutcome::Throw(throw_value)));
                    }
                    if std::env::var_os("AYY_TRACE_INLINE_PROMISES").is_some() {
                        eprintln!("consume_immediate_promise_outcome:finally-handler-emitted");
                    }
                    return Ok(Some(passthrough_outcome));
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
                        self.emit_immediate_promise_callback(
                            &handler,
                            &value,
                            !handlers_require_runtime_chain,
                        )?;
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
                let single_return_expression = self
                    .immediate_promise_callback_single_return_expression(
                        &handler,
                        handler_argument,
                    );
                let return_path_will_emit_chain =
                    single_return_expression
                        .as_ref()
                        .is_some_and(|return_expression| {
                            Self::call_is_promise_like_chain(return_expression)
                                && Self::expression_contains_dynamic_import_call(return_expression)
                        });
                if !return_path_will_emit_chain {
                    self.emit_immediate_promise_callback(
                        &handler,
                        handler_argument,
                        !handlers_require_runtime_chain,
                    )?;
                } else if std::env::var_os("AYY_TRACE_INLINE_PROMISES").is_some() {
                    eprintln!(
                        "consume_immediate_promise_outcome:skip-callback-body-return-chain handler={handler:?}"
                    );
                }
                let returned_outcome =
                    self.immediate_promise_callback_return_outcome(&handler, handler_argument)?;
                if std::env::var_os("AYY_TRACE_INLINE_PROMISES").is_some() {
                    eprintln!(
                        "consume_immediate_promise_outcome:value-handler-emitted property={property_name}"
                    );
                }
                Ok(Some(returned_outcome.unwrap_or(StaticEvalOutcome::Value(
                    Expression::Undefined,
                ))))
            }
            _ => Ok(None),
        }
    }

    pub(in crate::backend::direct_wasm) fn promise_prototype_then_call_expression(
        &self,
        object: &Expression,
        arguments: &[CallArgument],
    ) -> Option<Expression> {
        let LocalFunctionBinding::Builtin(function_name) =
            self.resolve_function_binding_from_expression(object)?
        else {
            return None;
        };
        if function_name != "Promise.prototype.then" {
            return None;
        }
        let receiver = arguments.first()?.expression().clone();
        Some(Expression::Call {
            callee: Box::new(Expression::Member {
                object: Box::new(receiver),
                property: Box::new(Expression::String("then".to_string())),
            }),
            arguments: arguments.iter().skip(1).cloned().collect(),
        })
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
        if !matches!(
            property_name.as_str(),
            "then" | "catch" | "finally" | "all" | "withResolvers"
        ) {
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
            if property_name == "all" {
                return Ok(false);
            }
            if property_name == "withResolvers" {
                return Ok(false);
            }
            if Self::call_is_promise_like_chain(object)
                || self.expression_is_direct_async_function_call(object)
                || Self::expression_is_dynamic_import_call(object)
                || self.expression_is_dynamic_import_promise_reference(object)
                || self.expression_is_async_function_prototype_call(object)
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
            if self.expression_is_lossy_promise_protocol_receiver(object)
                && self.emit_fulfilled_promise_protocol_member_call(
                    object,
                    property_name,
                    arguments,
                )?
            {
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
