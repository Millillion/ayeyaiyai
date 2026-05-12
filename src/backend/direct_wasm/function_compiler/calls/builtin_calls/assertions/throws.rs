use super::*;

impl<'a> FunctionCompiler<'a> {
    fn assert_throws_inline_safe_generated_binding(name: &str) -> bool {
        name.starts_with("__ayy_")
    }

    fn assert_throws_inline_safe_statement(statement: &Statement) -> bool {
        match statement {
            Statement::Expression(_)
            | Statement::Throw(_)
            | Statement::Assign { .. }
            | Statement::AssignMember { .. }
            | Statement::Print { .. } => true,
            Statement::Let { name, .. } | Statement::Var { name, .. } => {
                Self::assert_throws_inline_safe_generated_binding(name)
            }
            Statement::Block { body } | Statement::Declaration { body } => {
                body.iter().all(Self::assert_throws_inline_safe_statement)
            }
            Statement::If {
                then_branch,
                else_branch,
                ..
            } => then_branch
                .iter()
                .chain(else_branch)
                .all(Self::assert_throws_inline_safe_statement),
            Statement::Try {
                body,
                catch_setup,
                catch_body,
                ..
            } => body
                .iter()
                .chain(catch_setup)
                .chain(catch_body)
                .all(Self::assert_throws_inline_safe_statement),
            Statement::While { labels, body, .. } | Statement::DoWhile { labels, body, .. }
                if labels.is_empty()
                    && body.iter().all(Self::assert_throws_inline_safe_statement) =>
            {
                true
            }
            Statement::For {
                labels,
                init,
                body,
                break_hook: _,
                ..
            } if labels.is_empty()
                && init.iter().all(Self::assert_throws_inline_safe_statement)
                && body.iter().all(Self::assert_throws_inline_safe_statement) =>
            {
                true
            }
            Statement::Break { label: None } | Statement::Continue { label: None } => true,
            Statement::Return(_)
            | Statement::Yield { .. }
            | Statement::YieldDelegate { .. }
            | Statement::Break { .. }
            | Statement::Continue { .. }
            | Statement::Labeled { .. }
            | Statement::Switch { .. }
            | Statement::While { .. }
            | Statement::DoWhile { .. }
            | Statement::For { .. }
            | Statement::With { .. } => false,
        }
    }

    fn assert_throws_inline_callback_body(
        &self,
        callback: &Expression,
    ) -> Option<(Vec<Statement>, bool)> {
        let LocalFunctionBinding::User(function_name) =
            self.resolve_function_binding_from_expression(callback)?
        else {
            return None;
        };
        let declaration = self.prepared_function_declaration(&function_name)?;
        if declaration.lexical_this
            && self
                .resolve_function_expression_capture_slots(callback)
                .is_some_and(|capture_slots| capture_slots.contains_key("this"))
        {
            return None;
        }
        if !declaration.params.is_empty()
            || !declaration
                .body
                .iter()
                .all(Self::assert_throws_inline_safe_statement)
        {
            if let [Statement::Return(expression)] = declaration.body.as_slice() {
                return Some((
                    vec![Statement::Expression(expression.clone())],
                    declaration.strict,
                ));
            }
            return None;
        }
        Some((
            vec![Statement::Block {
                body: declaration.body.clone(),
            }],
            declaration.strict,
        ))
    }

    pub(in crate::backend::direct_wasm) fn sync_assert_throws_iterator_bindings_for_body(
        &mut self,
        body: &[Statement],
    ) {
        for statement in body {
            self.sync_assert_throws_iterator_bindings_for_statement(statement);
        }
    }

    fn sync_assert_throws_iterator_bindings_for_statement(&mut self, statement: &Statement) {
        match statement {
            Statement::Expression(expression)
            | Statement::Return(expression)
            | Statement::Throw(expression)
            | Statement::Yield { value: expression }
            | Statement::YieldDelegate { value: expression }
            | Statement::Var {
                value: expression, ..
            }
            | Statement::Let {
                value: expression, ..
            }
            | Statement::Assign {
                value: expression, ..
            } => self.sync_assert_throws_iterator_bindings_for_expression(expression),
            Statement::AssignMember {
                object,
                property,
                value,
            } => {
                self.sync_assert_throws_iterator_bindings_for_expression(object);
                self.sync_assert_throws_iterator_bindings_for_expression(property);
                self.sync_assert_throws_iterator_bindings_for_expression(value);
            }
            Statement::Print { values } => {
                for value in values {
                    self.sync_assert_throws_iterator_bindings_for_expression(value);
                }
            }
            Statement::Block { body }
            | Statement::Declaration { body }
            | Statement::Labeled { body, .. }
            | Statement::With { body, .. } => {
                self.sync_assert_throws_iterator_bindings_for_body(body);
            }
            Statement::If {
                condition,
                then_branch,
                else_branch,
            } => {
                self.sync_assert_throws_iterator_bindings_for_expression(condition);
                self.sync_assert_throws_iterator_bindings_for_body(then_branch);
                self.sync_assert_throws_iterator_bindings_for_body(else_branch);
            }
            Statement::Try {
                body,
                catch_setup,
                catch_body,
                ..
            } => {
                self.sync_assert_throws_iterator_bindings_for_body(body);
                self.sync_assert_throws_iterator_bindings_for_body(catch_setup);
                self.sync_assert_throws_iterator_bindings_for_body(catch_body);
            }
            Statement::For {
                init,
                condition,
                update,
                break_hook,
                body,
                ..
            } => {
                self.sync_assert_throws_iterator_bindings_for_body(init);
                if let Some(condition) = condition {
                    self.sync_assert_throws_iterator_bindings_for_expression(condition);
                }
                if let Some(update) = update {
                    self.sync_assert_throws_iterator_bindings_for_expression(update);
                }
                if let Some(break_hook) = break_hook {
                    self.sync_assert_throws_iterator_bindings_for_expression(break_hook);
                }
                self.sync_assert_throws_iterator_bindings_for_body(body);
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
                self.sync_assert_throws_iterator_bindings_for_expression(condition);
                if let Some(break_hook) = break_hook {
                    self.sync_assert_throws_iterator_bindings_for_expression(break_hook);
                }
                self.sync_assert_throws_iterator_bindings_for_body(body);
            }
            Statement::Switch {
                discriminant,
                cases,
                ..
            } => {
                self.sync_assert_throws_iterator_bindings_for_expression(discriminant);
                for case in cases {
                    if let Some(test) = &case.test {
                        self.sync_assert_throws_iterator_bindings_for_expression(test);
                    }
                    self.sync_assert_throws_iterator_bindings_for_body(&case.body);
                }
            }
            Statement::Break { .. } | Statement::Continue { .. } => {}
        }
    }

    fn sync_assert_throws_iterator_bindings_for_expression(&mut self, expression: &Expression) {
        if let Expression::Call { callee, arguments } = expression
            && let Some(LocalFunctionBinding::User(function_name)) =
                self.resolve_function_binding_from_expression(callee)
            && let Some(user_function) = self.user_function(&function_name).cloned()
        {
            let argument_expressions = arguments
                .iter()
                .map(|argument| match argument {
                    CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                        expression.clone()
                    }
                })
                .collect::<Vec<_>>();
            self.sync_consumed_iterator_bindings_for_user_call(&user_function);
            self.sync_argument_iterator_bindings_for_user_call(
                &user_function,
                &argument_expressions,
            );
            self.sync_assert_throws_default_parameter_iterator_bindings(
                &user_function,
                &argument_expressions,
            );
            if let Some(declaration) = self
                .resolve_registered_function_declaration(&function_name)
                .cloned()
            {
                let mut iterator_close_updated_bindings = HashSet::new();
                self.collect_iterator_close_updated_binding_names_from_statements(
                    &declaration.body,
                    &mut iterator_close_updated_bindings,
                );
                self.invalidate_static_binding_metadata_for_names(&iterator_close_updated_bindings);
            }
        }

        match expression {
            Expression::Member { object, property } => {
                self.sync_assert_throws_iterator_bindings_for_expression(object);
                self.sync_assert_throws_iterator_bindings_for_expression(property);
            }
            Expression::SuperMember { property } => {
                self.sync_assert_throws_iterator_bindings_for_expression(property);
            }
            Expression::Assign { value, .. }
            | Expression::Await(value)
            | Expression::EnumerateKeys(value)
            | Expression::GetIterator(value)
            | Expression::IteratorClose(value)
            | Expression::Unary {
                expression: value, ..
            } => self.sync_assert_throws_iterator_bindings_for_expression(value),
            Expression::AssignMember {
                object,
                property,
                value,
            } => {
                self.sync_assert_throws_iterator_bindings_for_expression(object);
                self.sync_assert_throws_iterator_bindings_for_expression(property);
                self.sync_assert_throws_iterator_bindings_for_expression(value);
            }
            Expression::AssignSuperMember { property, value } => {
                self.sync_assert_throws_iterator_bindings_for_expression(property);
                self.sync_assert_throws_iterator_bindings_for_expression(value);
            }
            Expression::Binary { left, right, .. } => {
                self.sync_assert_throws_iterator_bindings_for_expression(left);
                self.sync_assert_throws_iterator_bindings_for_expression(right);
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                self.sync_assert_throws_iterator_bindings_for_expression(condition);
                self.sync_assert_throws_iterator_bindings_for_expression(then_expression);
                self.sync_assert_throws_iterator_bindings_for_expression(else_expression);
            }
            Expression::Sequence(expressions) => {
                for expression in expressions {
                    self.sync_assert_throws_iterator_bindings_for_expression(expression);
                }
            }
            Expression::Call { callee, arguments }
            | Expression::SuperCall { callee, arguments }
            | Expression::New { callee, arguments } => {
                self.sync_assert_throws_iterator_bindings_for_expression(callee);
                for argument in arguments {
                    match argument {
                        CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                            self.sync_assert_throws_iterator_bindings_for_expression(expression);
                        }
                    }
                }
            }
            Expression::Array(elements) => {
                for element in elements {
                    match element {
                        ArrayElement::Expression(expression) | ArrayElement::Spread(expression) => {
                            self.sync_assert_throws_iterator_bindings_for_expression(expression);
                        }
                    }
                }
            }
            Expression::Object(entries) => {
                for entry in entries {
                    match entry {
                        ObjectEntry::Data { key, value } => {
                            self.sync_assert_throws_iterator_bindings_for_expression(key);
                            self.sync_assert_throws_iterator_bindings_for_expression(value);
                        }
                        ObjectEntry::Getter { key, getter } => {
                            self.sync_assert_throws_iterator_bindings_for_expression(key);
                            self.sync_assert_throws_iterator_bindings_for_expression(getter);
                        }
                        ObjectEntry::Setter { key, setter } => {
                            self.sync_assert_throws_iterator_bindings_for_expression(key);
                            self.sync_assert_throws_iterator_bindings_for_expression(setter);
                        }
                        ObjectEntry::Spread(expression) => {
                            self.sync_assert_throws_iterator_bindings_for_expression(expression);
                        }
                    }
                }
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

    fn sync_assert_throws_default_parameter_iterator_bindings(
        &mut self,
        user_function: &UserFunction,
        arguments: &[Expression],
    ) {
        for (index, default) in user_function.parameter_defaults.iter().enumerate() {
            let default_used = match arguments.get(index) {
                None => true,
                Some(argument) => {
                    matches!(
                        self.materialize_static_expression(argument),
                        Expression::Undefined
                    )
                }
            };
            if !default_used
                || default.is_none()
                || !self.assert_throws_parameter_consumed_by_lowered_iterator(user_function, index)
            {
                continue;
            }
            let Some(Expression::Identifier(name)) = default else {
                continue;
            };
            let Some(binding_name) = self.resolve_local_array_iterator_binding_name(name) else {
                continue;
            };
            self.close_local_iterator_binding(&binding_name);
        }
    }

    fn assert_throws_parameter_consumed_by_lowered_iterator(
        &self,
        user_function: &UserFunction,
        index: usize,
    ) -> bool {
        let Some(param_name) = user_function.params.get(index) else {
            return false;
        };
        let Some(function) = self.resolve_registered_function_declaration(&user_function.name)
        else {
            return false;
        };
        let mut aliases = HashSet::from([param_name.clone()]);
        Self::assert_throws_statements_consume_iterator_alias(&function.body, &mut aliases)
    }

    fn assert_throws_statements_consume_iterator_alias(
        statements: &[Statement],
        aliases: &mut HashSet<String>,
    ) -> bool {
        for statement in statements {
            if Self::assert_throws_statement_consumes_iterator_alias(statement, aliases) {
                return true;
            }
        }
        false
    }

    fn assert_throws_statement_consumes_iterator_alias(
        statement: &Statement,
        aliases: &mut HashSet<String>,
    ) -> bool {
        match statement {
            Statement::Let { name, value, .. }
            | Statement::Var { name, value }
            | Statement::Assign { name, value } => {
                if Self::assert_throws_expression_consumes_iterator_alias(value, aliases) {
                    return true;
                }
                if let Expression::Identifier(source_name) = value
                    && aliases.contains(source_name)
                {
                    aliases.insert(name.clone());
                }
                false
            }
            Statement::Expression(expression)
            | Statement::Return(expression)
            | Statement::Throw(expression)
            | Statement::Yield { value: expression }
            | Statement::YieldDelegate { value: expression } => {
                Self::assert_throws_expression_consumes_iterator_alias(expression, aliases)
            }
            Statement::AssignMember {
                object,
                property,
                value,
            } => {
                Self::assert_throws_expression_consumes_iterator_alias(object, aliases)
                    || Self::assert_throws_expression_consumes_iterator_alias(property, aliases)
                    || Self::assert_throws_expression_consumes_iterator_alias(value, aliases)
            }
            Statement::Print { values } => values.iter().any(|value| {
                Self::assert_throws_expression_consumes_iterator_alias(value, aliases)
            }),
            Statement::Block { body }
            | Statement::Declaration { body }
            | Statement::Labeled { body, .. }
            | Statement::With { body, .. } => {
                let mut scoped_aliases = aliases.clone();
                Self::assert_throws_statements_consume_iterator_alias(body, &mut scoped_aliases)
            }
            Statement::If {
                condition,
                then_branch,
                else_branch,
            } => {
                if Self::assert_throws_expression_consumes_iterator_alias(condition, aliases) {
                    return true;
                }
                let mut then_aliases = aliases.clone();
                let mut else_aliases = aliases.clone();
                Self::assert_throws_statements_consume_iterator_alias(
                    then_branch,
                    &mut then_aliases,
                ) || Self::assert_throws_statements_consume_iterator_alias(
                    else_branch,
                    &mut else_aliases,
                )
            }
            Statement::Try {
                body,
                catch_setup,
                catch_body,
                ..
            } => {
                let mut body_aliases = aliases.clone();
                let mut setup_aliases = aliases.clone();
                let mut catch_aliases = aliases.clone();
                Self::assert_throws_statements_consume_iterator_alias(body, &mut body_aliases)
                    || Self::assert_throws_statements_consume_iterator_alias(
                        catch_setup,
                        &mut setup_aliases,
                    )
                    || Self::assert_throws_statements_consume_iterator_alias(
                        catch_body,
                        &mut catch_aliases,
                    )
            }
            Statement::For {
                init,
                condition,
                update,
                break_hook,
                body,
                ..
            } => {
                let mut scoped_aliases = aliases.clone();
                Self::assert_throws_statements_consume_iterator_alias(init, &mut scoped_aliases)
                    || condition.as_ref().is_some_and(|condition| {
                        Self::assert_throws_expression_consumes_iterator_alias(
                            condition,
                            &mut scoped_aliases,
                        )
                    })
                    || update.as_ref().is_some_and(|update| {
                        Self::assert_throws_expression_consumes_iterator_alias(
                            update,
                            &mut scoped_aliases,
                        )
                    })
                    || break_hook.as_ref().is_some_and(|break_hook| {
                        Self::assert_throws_expression_consumes_iterator_alias(
                            break_hook,
                            &mut scoped_aliases,
                        )
                    })
                    || Self::assert_throws_statements_consume_iterator_alias(
                        body,
                        &mut scoped_aliases,
                    )
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
                Self::assert_throws_expression_consumes_iterator_alias(condition, aliases)
                    || break_hook.as_ref().is_some_and(|break_hook| {
                        Self::assert_throws_expression_consumes_iterator_alias(break_hook, aliases)
                    })
                    || Self::assert_throws_statements_consume_iterator_alias(body, aliases)
            }
            Statement::Switch {
                discriminant,
                cases,
                ..
            } => {
                if Self::assert_throws_expression_consumes_iterator_alias(discriminant, aliases) {
                    return true;
                }
                cases.iter().any(|case| {
                    let mut case_aliases = aliases.clone();
                    case.test.as_ref().is_some_and(|test| {
                        Self::assert_throws_expression_consumes_iterator_alias(
                            test,
                            &mut case_aliases,
                        )
                    }) || Self::assert_throws_statements_consume_iterator_alias(
                        &case.body,
                        &mut case_aliases,
                    )
                })
            }
            Statement::Break { .. } | Statement::Continue { .. } => false,
        }
    }

    fn assert_throws_expression_consumes_iterator_alias(
        expression: &Expression,
        aliases: &mut HashSet<String>,
    ) -> bool {
        if let Expression::GetIterator(value) = expression
            && let Expression::Identifier(name) = value.as_ref()
            && aliases.contains(name)
        {
            return true;
        }
        match expression {
            Expression::Member { object, property } => {
                Self::assert_throws_expression_consumes_iterator_alias(object, aliases)
                    || Self::assert_throws_expression_consumes_iterator_alias(property, aliases)
            }
            Expression::SuperMember { property } => {
                Self::assert_throws_expression_consumes_iterator_alias(property, aliases)
            }
            Expression::Assign { value, .. }
            | Expression::Await(value)
            | Expression::EnumerateKeys(value)
            | Expression::GetIterator(value)
            | Expression::IteratorClose(value)
            | Expression::Unary {
                expression: value, ..
            } => Self::assert_throws_expression_consumes_iterator_alias(value, aliases),
            Expression::AssignMember {
                object,
                property,
                value,
            } => {
                Self::assert_throws_expression_consumes_iterator_alias(object, aliases)
                    || Self::assert_throws_expression_consumes_iterator_alias(property, aliases)
                    || Self::assert_throws_expression_consumes_iterator_alias(value, aliases)
            }
            Expression::AssignSuperMember { property, value } => {
                Self::assert_throws_expression_consumes_iterator_alias(property, aliases)
                    || Self::assert_throws_expression_consumes_iterator_alias(value, aliases)
            }
            Expression::Binary { left, right, .. } => {
                Self::assert_throws_expression_consumes_iterator_alias(left, aliases)
                    || Self::assert_throws_expression_consumes_iterator_alias(right, aliases)
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                Self::assert_throws_expression_consumes_iterator_alias(condition, aliases)
                    || Self::assert_throws_expression_consumes_iterator_alias(
                        then_expression,
                        aliases,
                    )
                    || Self::assert_throws_expression_consumes_iterator_alias(
                        else_expression,
                        aliases,
                    )
            }
            Expression::Sequence(expressions) => expressions.iter().any(|expression| {
                Self::assert_throws_expression_consumes_iterator_alias(expression, aliases)
            }),
            Expression::Call { callee, arguments }
            | Expression::SuperCall { callee, arguments }
            | Expression::New { callee, arguments } => {
                Self::assert_throws_expression_consumes_iterator_alias(callee, aliases)
                    || arguments.iter().any(|argument| match argument {
                        CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                            Self::assert_throws_expression_consumes_iterator_alias(
                                expression, aliases,
                            )
                        }
                    })
            }
            Expression::Array(elements) => elements.iter().any(|element| match element {
                ArrayElement::Expression(expression) | ArrayElement::Spread(expression) => {
                    Self::assert_throws_expression_consumes_iterator_alias(expression, aliases)
                }
            }),
            Expression::Object(entries) => entries.iter().any(|entry| match entry {
                ObjectEntry::Data { key, value } => {
                    Self::assert_throws_expression_consumes_iterator_alias(key, aliases)
                        || Self::assert_throws_expression_consumes_iterator_alias(value, aliases)
                }
                ObjectEntry::Getter { key, getter } => {
                    Self::assert_throws_expression_consumes_iterator_alias(key, aliases)
                        || Self::assert_throws_expression_consumes_iterator_alias(getter, aliases)
                }
                ObjectEntry::Setter { key, setter } => {
                    Self::assert_throws_expression_consumes_iterator_alias(key, aliases)
                        || Self::assert_throws_expression_consumes_iterator_alias(setter, aliases)
                }
                ObjectEntry::Spread(expression) => {
                    Self::assert_throws_expression_consumes_iterator_alias(expression, aliases)
                }
            }),
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
            | Expression::Update { .. } => false,
        }
    }

    pub(in crate::backend::direct_wasm) fn emit_assert_throws_call(
        &mut self,
        arguments: &[CallArgument],
    ) -> DirectResult<bool> {
        let [
            CallArgument::Expression(expected_error),
            CallArgument::Expression(callback),
            rest @ ..,
        ] = arguments
        else {
            return Ok(false);
        };

        self.emit_numeric_expression(expected_error)?;
        self.state.emission.output.instructions.push(0x1a);
        for argument in rest {
            match argument {
                CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                    self.emit_numeric_expression(expression)?;
                    self.state.emission.output.instructions.push(0x1a);
                }
            }
        }
        let callback_name =
            self.allocate_named_hidden_local("assert_throws_callback", StaticValueKind::Unknown);
        self.emit_statement(&Statement::Let {
            name: callback_name.clone(),
            mutable: false,
            value: callback.clone(),
        })?;

        let caught_name =
            self.allocate_named_hidden_local("assert_throws_caught", StaticValueKind::Bool);
        self.emit_statement(&Statement::Let {
            name: caught_name.clone(),
            mutable: true,
            value: Expression::Bool(false),
        })?;
        let caught_local = self.lookup_local(&caught_name)?;

        let inline_body = self.assert_throws_inline_callback_body(callback);
        if std::env::var_os("AYY_TRACE_ASSERTIONS").is_some() {
            eprintln!(
                "assert_throws_call inline={} callback={callback:?}",
                inline_body.is_some()
            );
        }
        let fallback_callee = if self
            .resolve_function_expression_capture_slots(callback)
            .is_some()
        {
            callback.clone()
        } else {
            Expression::Identifier(callback_name)
        };
        let fallback_body = vec![Statement::Expression(Expression::Call {
            callee: Box::new(fallback_callee),
            arguments: Vec::new(),
        })];
        let (try_body, inline_strict_mode) = inline_body
            .map(|(body, strict_mode)| (body, Some(strict_mode)))
            .unwrap_or((fallback_body, None));
        let iterator_sync_body = try_body.clone();
        let try_statement = Statement::Try {
            body: try_body,
            catch_binding: None,
            catch_setup: Vec::new(),
            catch_body: vec![Statement::Assign {
                name: caught_name,
                value: Expression::Bool(true),
            }],
        };
        if let Some(strict_mode) = inline_strict_mode {
            self.with_strict_mode(strict_mode, |compiler| {
                compiler.emit_statement(&try_statement)
            })?;
        } else {
            self.emit_statement(&try_statement)?;
        }
        self.sync_assert_throws_iterator_bindings_for_body(&iterator_sync_body);

        self.push_local_get(caught_local);
        self.push_i32_const(0);
        self.push_binary_op(BinaryOp::Equal)?;
        self.state.emission.output.instructions.push(0x04);
        self.state
            .emission
            .output
            .instructions
            .push(EMPTY_BLOCK_TYPE);
        self.push_control_frame();
        self.emit_error_throw()?;
        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();
        self.push_i32_const(JS_UNDEFINED_TAG);
        Ok(true)
    }

    pub(in crate::backend::direct_wasm) fn emit_assert_throws_statement(
        &mut self,
        expression: &Expression,
    ) -> DirectResult<bool> {
        let Expression::Call { callee, arguments } = expression else {
            return Ok(false);
        };
        let Expression::Identifier(name) = callee.as_ref() else {
            return Ok(false);
        };
        if name != "__ayyAssertThrows" {
            return Ok(false);
        }

        let [
            CallArgument::Expression(expected_error),
            CallArgument::Expression(callback),
            rest @ ..,
        ] = arguments.as_slice()
        else {
            return Ok(false);
        };

        self.emit_numeric_expression(expected_error)?;
        self.state.emission.output.instructions.push(0x1a);
        for argument in rest {
            match argument {
                CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                    self.emit_numeric_expression(expression)?;
                    self.state.emission.output.instructions.push(0x1a);
                }
            }
        }

        let callback_name =
            self.allocate_named_hidden_local("assert_throws_callback", StaticValueKind::Unknown);
        self.emit_statement(&Statement::Let {
            name: callback_name.clone(),
            mutable: false,
            value: callback.clone(),
        })?;

        let caught_name =
            self.allocate_named_hidden_local("assert_throws_caught", StaticValueKind::Bool);
        self.emit_statement(&Statement::Let {
            name: caught_name.clone(),
            mutable: true,
            value: Expression::Bool(false),
        })?;
        let caught_local = self.lookup_local(&caught_name)?;

        let inline_body = self.assert_throws_inline_callback_body(callback);
        if std::env::var_os("AYY_TRACE_ASSERTIONS").is_some() {
            eprintln!(
                "assert_throws_statement inline={} callback={callback:?}",
                inline_body.is_some()
            );
        }
        let fallback_callee = if self
            .resolve_function_expression_capture_slots(callback)
            .is_some()
        {
            callback.clone()
        } else {
            Expression::Identifier(callback_name)
        };
        let fallback_body = vec![Statement::Expression(Expression::Call {
            callee: Box::new(fallback_callee),
            arguments: Vec::new(),
        })];
        let (try_body, inline_strict_mode) = inline_body
            .map(|(body, strict_mode)| (body, Some(strict_mode)))
            .unwrap_or((fallback_body, None));
        let iterator_sync_body = try_body.clone();
        let try_statement = Statement::Try {
            body: try_body,
            catch_binding: None,
            catch_setup: Vec::new(),
            catch_body: vec![Statement::Assign {
                name: caught_name,
                value: Expression::Bool(true),
            }],
        };
        if let Some(strict_mode) = inline_strict_mode {
            self.with_strict_mode(strict_mode, |compiler| {
                compiler.emit_statement(&try_statement)
            })?;
        } else {
            self.emit_statement(&try_statement)?;
        }
        self.sync_assert_throws_iterator_bindings_for_body(&iterator_sync_body);

        self.push_local_get(caught_local);
        self.push_i32_const(0);
        self.push_binary_op(BinaryOp::Equal)?;
        self.state.emission.output.instructions.push(0x04);
        self.state
            .emission
            .output
            .instructions
            .push(EMPTY_BLOCK_TYPE);
        self.push_control_frame();
        self.emit_error_throw()?;
        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();
        Ok(true)
    }
}
