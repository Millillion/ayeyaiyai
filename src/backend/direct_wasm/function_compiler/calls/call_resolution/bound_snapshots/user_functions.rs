use super::*;

fn expression_calls_user_function(expression: &Expression, names: &HashSet<String>) -> bool {
    match expression {
        Expression::Call { callee, arguments }
        | Expression::SuperCall { callee, arguments }
        | Expression::New { callee, arguments } => {
            expression_references_user_function(callee, names)
                || expression_calls_user_function(callee, names)
                || arguments.iter().any(|argument| match argument {
                    CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                        expression_calls_user_function(expression, names)
                    }
                })
        }
        Expression::Array(elements) => elements.iter().any(|element| match element {
            ArrayElement::Expression(expression) | ArrayElement::Spread(expression) => {
                expression_calls_user_function(expression, names)
            }
        }),
        Expression::Object(entries) => entries.iter().any(|entry| match entry {
            ObjectEntry::Data { key, value } => {
                expression_calls_user_function(key, names)
                    || expression_calls_user_function(value, names)
            }
            ObjectEntry::Getter { key, getter } => {
                expression_calls_user_function(key, names)
                    || expression_calls_user_function(getter, names)
            }
            ObjectEntry::Setter { key, setter } => {
                expression_calls_user_function(key, names)
                    || expression_calls_user_function(setter, names)
            }
            ObjectEntry::Spread(expression) => expression_calls_user_function(expression, names),
        }),
        Expression::Member { object, property } => {
            expression_calls_user_function(object, names)
                || expression_calls_user_function(property, names)
        }
        Expression::SuperMember { property } => expression_calls_user_function(property, names),
        Expression::Assign { value, .. }
        | Expression::Await(value)
        | Expression::EnumerateKeys(value)
        | Expression::GetIterator(value)
        | Expression::IteratorClose(value)
        | Expression::Unary {
            expression: value, ..
        } => expression_calls_user_function(value, names),
        Expression::AssignMember {
            object,
            property,
            value,
        } => {
            expression_calls_user_function(object, names)
                || expression_calls_user_function(property, names)
                || expression_calls_user_function(value, names)
        }
        Expression::AssignSuperMember { property, value } => {
            expression_calls_user_function(property, names)
                || expression_calls_user_function(value, names)
        }
        Expression::Binary { left, right, .. } => {
            expression_calls_user_function(left, names)
                || expression_calls_user_function(right, names)
        }
        Expression::Conditional {
            condition,
            then_expression,
            else_expression,
        } => {
            expression_calls_user_function(condition, names)
                || expression_calls_user_function(then_expression, names)
                || expression_calls_user_function(else_expression, names)
        }
        Expression::Sequence(expressions) => expressions
            .iter()
            .any(|expression| expression_calls_user_function(expression, names)),
        Expression::Update { .. }
        | Expression::Identifier(_)
        | Expression::Number(_)
        | Expression::BigInt(_)
        | Expression::String(_)
        | Expression::Bool(_)
        | Expression::Null
        | Expression::Undefined
        | Expression::NewTarget
        | Expression::This
        | Expression::Sent => false,
    }
}

fn statement_calls_user_function(statement: &Statement, names: &HashSet<String>) -> bool {
    match statement {
        Statement::Declaration { body }
        | Statement::Block { body }
        | Statement::Labeled { body, .. } => body
            .iter()
            .any(|statement| statement_calls_user_function(statement, names)),
        Statement::Var { value, .. }
        | Statement::Let { value, .. }
        | Statement::Assign { value, .. }
        | Statement::Expression(value)
        | Statement::Throw(value)
        | Statement::Return(value)
        | Statement::Yield { value }
        | Statement::YieldDelegate { value } => expression_calls_user_function(value, names),
        Statement::AssignMember {
            object,
            property,
            value,
        } => {
            expression_calls_user_function(object, names)
                || expression_calls_user_function(property, names)
                || expression_calls_user_function(value, names)
        }
        Statement::Print { values } => values
            .iter()
            .any(|value| expression_calls_user_function(value, names)),
        Statement::With { object, body } => {
            expression_calls_user_function(object, names)
                || body
                    .iter()
                    .any(|statement| statement_calls_user_function(statement, names))
        }
        Statement::If {
            condition,
            then_branch,
            else_branch,
        } => {
            expression_calls_user_function(condition, names)
                || then_branch
                    .iter()
                    .any(|statement| statement_calls_user_function(statement, names))
                || else_branch
                    .iter()
                    .any(|statement| statement_calls_user_function(statement, names))
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
            .any(|statement| statement_calls_user_function(statement, names)),
        Statement::Switch {
            discriminant,
            cases,
            ..
        } => {
            expression_calls_user_function(discriminant, names)
                || cases.iter().any(|case| {
                    case.test
                        .as_ref()
                        .is_some_and(|test| expression_calls_user_function(test, names))
                        || case
                            .body
                            .iter()
                            .any(|statement| statement_calls_user_function(statement, names))
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
            init.iter()
                .any(|statement| statement_calls_user_function(statement, names))
                || condition
                    .as_ref()
                    .is_some_and(|condition| expression_calls_user_function(condition, names))
                || update
                    .as_ref()
                    .is_some_and(|update| expression_calls_user_function(update, names))
                || break_hook
                    .as_ref()
                    .is_some_and(|break_hook| expression_calls_user_function(break_hook, names))
                || body
                    .iter()
                    .any(|statement| statement_calls_user_function(statement, names))
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
            expression_calls_user_function(condition, names)
                || break_hook
                    .as_ref()
                    .is_some_and(|break_hook| expression_calls_user_function(break_hook, names))
                || body
                    .iter()
                    .any(|statement| statement_calls_user_function(statement, names))
        }
        Statement::Break { .. } | Statement::Continue { .. } => false,
    }
}

impl<'a> FunctionCompiler<'a> {
    fn user_function_calls_captured_user_function(&self, user_function: &UserFunction) -> bool {
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
                    statement_calls_user_function(statement, &captured_user_function_names)
                })
            })
    }

    fn materialize_bound_snapshot_bindings(
        &self,
        bindings: &HashMap<String, Expression>,
        current_function_name: Option<&str>,
    ) -> HashMap<String, Expression> {
        let mut materialized = bindings.clone();
        let binding_names = materialized.keys().cloned().collect::<Vec<_>>();
        for name in binding_names {
            let Some(current_value) = materialized.get(&name).cloned() else {
                continue;
            };
            if matches!(current_value, Expression::Identifier(_))
                && self
                    .resolve_static_reference_identity_key(&current_value)
                    .is_some()
            {
                continue;
            }
            let resolved = self
                .evaluate_bound_snapshot_expression(
                    &current_value,
                    &mut materialized,
                    current_function_name,
                )
                .or(Some(current_value));
            if let Some(resolved) = resolved {
                materialized.insert(name, resolved);
            }
        }
        materialized
    }

    fn should_preserve_bound_snapshot_throw_identity(
        &self,
        value: &Expression,
        current_function_name: Option<&str>,
    ) -> bool {
        matches!(value, Expression::Identifier(_))
            && (self.resolve_static_reference_identity_key(value).is_some()
                || self.resolve_object_binding_from_expression(value).is_some()
                || self.resolve_array_binding_from_expression(value).is_some()
                || self
                    .resolve_function_binding_from_expression_with_context(
                        value,
                        current_function_name,
                    )
                    .is_some())
    }

    pub(in crate::backend::direct_wasm) fn resolve_bound_snapshot_user_function_result(
        &self,
        function_name: &str,
        bindings: &HashMap<String, Expression>,
    ) -> Option<(Expression, HashMap<String, Expression>)> {
        self.resolve_bound_snapshot_user_function_result_with_arguments_and_this(
            function_name,
            bindings,
            &[],
            &Expression::Undefined,
        )
    }

    pub(in crate::backend::direct_wasm) fn resolve_bound_snapshot_user_function_result_with_arguments(
        &self,
        function_name: &str,
        bindings: &HashMap<String, Expression>,
        arguments: &[Expression],
    ) -> Option<(Expression, HashMap<String, Expression>)> {
        self.resolve_bound_snapshot_user_function_result_with_arguments_and_this(
            function_name,
            bindings,
            arguments,
            &Expression::Undefined,
        )
    }

    pub(in crate::backend::direct_wasm) fn resolve_bound_snapshot_user_function_result_with_arguments_and_this(
        &self,
        function_name: &str,
        bindings: &HashMap<String, Expression>,
        arguments: &[Expression],
        this_binding: &Expression,
    ) -> Option<(Expression, HashMap<String, Expression>)> {
        let (outcome, local_bindings) = self
            .resolve_bound_snapshot_user_function_outcome_with_arguments_and_this(
                function_name,
                bindings,
                arguments,
                this_binding,
            )?;
        Some((
            match outcome {
                StaticEvalOutcome::Value(value) => value,
                StaticEvalOutcome::Throw(_) => Expression::Undefined,
            },
            local_bindings,
        ))
    }

    pub(in crate::backend::direct_wasm) fn resolve_bound_snapshot_user_function_outcome_with_arguments_and_this(
        &self,
        function_name: &str,
        bindings: &HashMap<String, Expression>,
        arguments: &[Expression],
        this_binding: &Expression,
    ) -> Option<(StaticEvalOutcome, HashMap<String, Expression>)> {
        let function = self.resolve_registered_function_declaration(function_name)?;
        let user_function = self.user_function(function_name)?;
        if self
            .backend
            .function_registry
            .analysis
            .user_function_capture_bindings
            .get(&user_function.name)
            .is_some_and(|captures| captures.keys().any(|name| !bindings.contains_key(name)))
            || self.user_function_calls_captured_user_function(user_function)
        {
            return None;
        }
        if user_function.has_parameter_defaults() {
            return None;
        }
        if user_function.has_lowered_pattern_parameters() {
            return None;
        }
        if !self
            .user_function_parameter_iterator_consumption_indices(user_function)
            .is_empty()
        {
            return None;
        }
        if !user_function.params.is_empty() && !user_function.extra_argument_indices.is_empty() {
            return None;
        }
        let materialized_arguments = arguments
            .iter()
            .map(|argument| self.materialize_static_expression(argument))
            .collect::<Vec<_>>();
        let mut local_bindings = bindings.clone();
        for binding in &user_function.scope_bindings {
            local_bindings
                .entry(binding.clone())
                .or_insert(Expression::Undefined);
        }
        for (index, parameter_name) in user_function.params.iter().enumerate() {
            local_bindings.insert(
                parameter_name.clone(),
                materialized_arguments
                    .get(index)
                    .cloned()
                    .unwrap_or(Expression::Undefined),
            );
        }
        local_bindings.insert("this".to_string(), this_binding.clone());
        if let Expression::Identifier(this_name) = this_binding
            && !local_bindings.contains_key(this_name)
        {
            if let Some(object_binding) = self.resolve_object_binding_from_expression(this_binding)
            {
                local_bindings.insert(
                    this_name.clone(),
                    object_binding_to_expression(&object_binding),
                );
            } else if let Some(value) = self
                .state
                .speculation
                .static_semantics
                .local_value_binding(this_name)
                .cloned()
                .or_else(|| {
                    self.backend
                        .global_semantics
                        .values
                        .value_bindings
                        .get(this_name)
                        .cloned()
                })
            {
                local_bindings.insert(this_name.clone(), value);
            }
        }
        let arguments_shadowed = user_function.lexical_this
            || user_function.body_declares_arguments_binding
            || user_function.params.iter().any(|param| {
                param == "arguments"
                    || scoped_binding_source_name(param)
                        .is_some_and(|source_name| source_name == "arguments")
            });
        if !arguments_shadowed {
            local_bindings.insert(
                "arguments".to_string(),
                Expression::Array(
                    materialized_arguments
                        .iter()
                        .cloned()
                        .map(ArrayElement::Expression)
                        .collect(),
                ),
            );
        }
        let result = self.execute_bound_snapshot_statements(
            &function.body,
            &mut local_bindings,
            Some(function_name),
        )?;
        let mut materialized_bindings =
            self.materialize_bound_snapshot_bindings(&local_bindings, Some(function_name));
        let materialized_outcome = match result {
            BoundSnapshotControlFlow::None => BoundSnapshotControlFlow::None,
            BoundSnapshotControlFlow::Return(value) => BoundSnapshotControlFlow::Return(
                self.evaluate_bound_snapshot_expression(
                    &value,
                    &mut materialized_bindings,
                    Some(function_name),
                )
                .unwrap_or(value),
            ),
            BoundSnapshotControlFlow::Throw(value) => BoundSnapshotControlFlow::Throw(
                if self.should_preserve_bound_snapshot_throw_identity(
                    &value,
                    Some(function_name),
                ) {
                    value
                } else {
                    self.evaluate_bound_snapshot_expression(
                        &value,
                        &mut materialized_bindings,
                        Some(function_name),
                    )
                    .unwrap_or(value)
                },
            ),
            BoundSnapshotControlFlow::Break(_) => return None,
        };
        Some((
            match materialized_outcome {
                BoundSnapshotControlFlow::None => StaticEvalOutcome::Value(Expression::Undefined),
                BoundSnapshotControlFlow::Return(value) => StaticEvalOutcome::Value(value),
                BoundSnapshotControlFlow::Throw(value) => {
                    StaticEvalOutcome::Throw(StaticThrowValue::Value(value))
                }
                BoundSnapshotControlFlow::Break(_) => return None,
            },
            materialized_bindings,
        ))
    }

    pub(in crate::backend::direct_wasm) fn resolve_bound_snapshot_function_outcome_with_arguments_and_this(
        &self,
        binding: &LocalFunctionBinding,
        bindings: &HashMap<String, Expression>,
        arguments: &[Expression],
        this_binding: &Expression,
    ) -> Option<(StaticEvalOutcome, HashMap<String, Expression>)> {
        let LocalFunctionBinding::User(function_name) = binding else {
            return None;
        };
        self.resolve_bound_snapshot_user_function_outcome_with_arguments_and_this(
            function_name,
            bindings,
            arguments,
            this_binding,
        )
    }

    pub(in crate::backend::direct_wasm) fn apply_bound_snapshot_user_function_call_effects(
        &self,
        function_name: &str,
        arguments: &[Expression],
        this_binding: &Expression,
        bindings: &mut HashMap<String, Expression>,
        current_function_name: Option<&str>,
    ) -> Option<Expression> {
        let user_function = self.user_function(function_name)?;
        if user_function.is_async() || user_function.is_generator() {
            return None;
        }
        let evaluated_arguments = arguments
            .iter()
            .map(|argument| {
                self.evaluate_bound_snapshot_expression(argument, bindings, current_function_name)
            })
            .collect::<Option<Vec<_>>>()?;
        let (result, updated_bindings) = self
            .resolve_bound_snapshot_user_function_result_with_arguments_and_this(
                function_name,
                bindings,
                &evaluated_arguments,
                this_binding,
            )?;
        for (name, value) in updated_bindings {
            let source_name = scoped_binding_source_name(&name)
                .unwrap_or(&name)
                .to_string();
            if user_function.scope_bindings.contains(&source_name) {
                continue;
            }
            bindings.insert(source_name, value);
        }
        Some(result)
    }

    pub(in crate::backend::direct_wasm) fn resolve_bound_snapshot_function_result_with_arguments(
        &self,
        binding: &LocalFunctionBinding,
        bindings: &HashMap<String, Expression>,
        arguments: &[Expression],
    ) -> Option<(Expression, HashMap<String, Expression>)> {
        self.resolve_bound_snapshot_function_result_with_arguments_and_this(
            binding,
            bindings,
            arguments,
            &Expression::Undefined,
        )
    }

    pub(in crate::backend::direct_wasm) fn resolve_bound_snapshot_function_result_with_arguments_and_this(
        &self,
        binding: &LocalFunctionBinding,
        bindings: &HashMap<String, Expression>,
        arguments: &[Expression],
        this_binding: &Expression,
    ) -> Option<(Expression, HashMap<String, Expression>)> {
        let LocalFunctionBinding::User(function_name) = binding else {
            return None;
        };
        self.resolve_bound_snapshot_user_function_result_with_arguments_and_this(
            function_name,
            bindings,
            arguments,
            this_binding,
        )
    }

    pub(in crate::backend::direct_wasm) fn resolve_bound_snapshot_thenable_outcome(
        &self,
        binding: &LocalFunctionBinding,
        this_binding: &Expression,
        bindings: &mut HashMap<String, Expression>,
        _current_function_name: Option<&str>,
    ) -> Option<StaticEvalOutcome> {
        let (result, updated_bindings) = self
            .resolve_bound_snapshot_function_result_with_arguments_and_this(
                binding,
                bindings,
                &[
                    Expression::Identifier(SNAPSHOT_AWAIT_RESOLVE_BINDING.to_string()),
                    Expression::Identifier(SNAPSHOT_AWAIT_REJECT_BINDING.to_string()),
                ],
                this_binding,
            )?;
        *bindings = updated_bindings;
        let resolution = bindings
            .get(SNAPSHOT_AWAIT_RESOLUTION_VALUE)
            .cloned()
            .map(|value| self.sanitize_snapshot_await_marker_expression(&value));
        let rejection = bindings
            .get(SNAPSHOT_AWAIT_REJECTION_VALUE)
            .cloned()
            .map(|value| self.sanitize_snapshot_await_marker_expression(&value));
        for value in bindings.values_mut() {
            *value = self.sanitize_snapshot_await_marker_expression(value);
        }
        bindings.retain(|name, value| {
            name != SNAPSHOT_AWAIT_RESOLUTION_VALUE
                && name != SNAPSHOT_AWAIT_REJECTION_VALUE
                && name != SNAPSHOT_AWAIT_RESOLVE_BINDING
                && name != SNAPSHOT_AWAIT_REJECT_BINDING
                && !matches!(
                    value,
                    Expression::Identifier(marker)
                        if marker == SNAPSHOT_AWAIT_RESOLVE_BINDING
                            || marker == SNAPSHOT_AWAIT_REJECT_BINDING
                )
        });
        if let Some(resolution) = resolution {
            return self
                .resolve_static_await_resolution_outcome(&resolution)
                .or(Some(StaticEvalOutcome::Value(resolution)));
        }
        if let Some(rejection) = rejection {
            return Some(StaticEvalOutcome::Throw(StaticThrowValue::Value(rejection)));
        }
        match result {
            Expression::Undefined => None,
            _ => self.resolve_static_await_resolution_outcome(&result),
        }
    }
}
