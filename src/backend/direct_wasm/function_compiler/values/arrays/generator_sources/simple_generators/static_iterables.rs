use super::*;

thread_local! {
    static ACTIVE_STATIC_ITERABLE_BINDING_SHAPES: RefCell<HashSet<String>> = RefCell::new(HashSet::new());
}

struct StaticIterableBindingGuard {
    key: String,
    _memo: crate::backend::direct_wasm::memo::ResolutionGuardScope,
}

impl StaticIterableBindingGuard {
    fn enter(expression: &Expression) -> Option<Self> {
        let key = format!("{expression:?}");
        let inserted = ACTIVE_STATIC_ITERABLE_BINDING_SHAPES
            .with(|active| active.borrow_mut().insert(key.clone()));
        if !inserted {
            crate::backend::direct_wasm::memo::note_resolution_guard_block();
        }
        inserted.then_some(Self {
            key,
            _memo: crate::backend::direct_wasm::memo::ResolutionGuardScope::enter_class(18),
        })
    }
}

impl Drop for StaticIterableBindingGuard {
    fn drop(&mut self) {
        ACTIVE_STATIC_ITERABLE_BINDING_SHAPES.with(|active| {
            active.borrow_mut().remove(&self.key);
        });
    }
}

impl<'a> FunctionCompiler<'a> {
    fn static_iterator_result_has_observable_return(
        &self,
        iterator_result_binding: &ObjectValueBinding,
    ) -> bool {
        let return_property = Expression::String("return".to_string());
        if let Some(descriptor) =
            object_binding_lookup_descriptor(iterator_result_binding, &return_property)
        {
            if descriptor.getter.is_some() || descriptor.has_get {
                return true;
            }
            return descriptor
                .value
                .as_ref()
                .is_some_and(|value| !matches!(value, Expression::Undefined | Expression::Null));
        }
        object_binding_lookup_value(iterator_result_binding, &return_property)
            .is_some_and(|value| !matches!(value, Expression::Undefined | Expression::Null))
    }

    fn static_iterator_object_has_observable_throw(
        &self,
        expression: &Expression,
        iterator_binding: &ObjectValueBinding,
    ) -> bool {
        let throw_property = Expression::String("throw".to_string());
        if self
            .resolve_member_function_binding(expression, &throw_property)
            .is_some()
            || self
                .resolve_member_getter_binding(expression, &throw_property)
                .is_some()
        {
            return true;
        }
        if let Some(descriptor) =
            object_binding_lookup_descriptor(iterator_binding, &throw_property)
        {
            if descriptor.getter.is_some() || descriptor.has_get {
                return true;
            }
            return descriptor
                .value
                .as_ref()
                .is_some_and(|value| !matches!(value, Expression::Undefined | Expression::Null));
        }
        object_binding_lookup_value(iterator_binding, &throw_property)
            .is_some_and(|value| !matches!(value, Expression::Undefined | Expression::Null))
    }

    fn static_function_terminal_return_value(
        &self,
        binding: &LocalFunctionBinding,
    ) -> Option<Expression> {
        fn terminal_return_value(statements: &[Statement]) -> Option<Expression> {
            match statements.last()? {
                Statement::Return(value) => Some(value.clone()),
                Statement::Block { body } | Statement::Declaration { body } => {
                    terminal_return_value(body)
                }
                _ => None,
            }
        }

        let LocalFunctionBinding::User(function_name) = binding else {
            return None;
        };
        let function = self.resolve_registered_function_declaration(function_name)?;
        terminal_return_value(&function.body)
    }

    fn static_iterator_object_return_close_effects(
        &self,
        expression: &Expression,
        iterator_binding: &ObjectValueBinding,
    ) -> Option<Vec<Statement>> {
        let return_property = Expression::String("return".to_string());
        let close_effect = || {
            vec![Statement::Expression(Expression::IteratorClose(Box::new(
                expression.clone(),
            )))]
        };
        let no_return_close_effects = || {
            if self.static_iterator_object_has_observable_throw(expression, iterator_binding) {
                close_effect()
            } else {
                Vec::new()
            }
        };
        if let Some(getter_binding) =
            self.resolve_member_getter_binding(expression, &return_property)
        {
            let outcome = self
                .resolve_static_function_outcome_from_binding_with_call_frame_and_context(
                    &getter_binding,
                    &[],
                    expression,
                    self.current_function_name(),
                )
                .or_else(|| {
                    self.resolve_static_function_outcome_from_binding_with_context(
                        &getter_binding,
                        &[],
                        self.current_function_name(),
                    )
                });
            if let Some(outcome) = outcome {
                return match outcome {
                    StaticEvalOutcome::Value(Expression::Undefined | Expression::Null) => {
                        Some(close_effect())
                    }
                    StaticEvalOutcome::Throw(_) => Some(close_effect()),
                    StaticEvalOutcome::Value(_) => None,
                };
            }
            if matches!(
                self.static_function_terminal_return_value(&getter_binding),
                Some(Expression::Undefined | Expression::Null)
            ) || self.function_binding_defaults_to_undefined(&getter_binding)
            {
                return Some(close_effect());
            }
            return None;
        }
        if self
            .resolve_member_function_binding(expression, &return_property)
            .is_some()
        {
            return Some(close_effect());
        }
        if let Some(descriptor) =
            object_binding_lookup_descriptor(iterator_binding, &return_property)
        {
            if let Some(getter) = &descriptor.getter {
                let getter_binding = self.resolve_function_binding_from_expression(getter)?;
                let outcome = self
                    .resolve_static_function_outcome_from_binding_with_call_frame_and_context(
                        &getter_binding,
                        &[],
                        expression,
                        self.current_function_name(),
                    )
                    .or_else(|| {
                        self.resolve_static_function_outcome_from_binding_with_context(
                            &getter_binding,
                            &[],
                            self.current_function_name(),
                        )
                    });
                if let Some(outcome) = outcome {
                    return match outcome {
                        StaticEvalOutcome::Value(Expression::Undefined | Expression::Null) => {
                            Some(close_effect())
                        }
                        StaticEvalOutcome::Throw(_) => Some(close_effect()),
                        StaticEvalOutcome::Value(_) => None,
                    };
                }
                if matches!(
                    self.static_function_terminal_return_value(&getter_binding),
                    Some(Expression::Undefined | Expression::Null)
                ) || self.function_binding_defaults_to_undefined(&getter_binding)
                {
                    return Some(close_effect());
                }
                return None;
            }
            if descriptor.has_get {
                return Some(close_effect());
            }
            if let Some(value) = descriptor.value.as_ref() {
                return matches!(value, Expression::Undefined | Expression::Null)
                    .then(|| no_return_close_effects());
            }
            return Some(no_return_close_effects());
        }
        if let Some(value) = object_binding_lookup_value(iterator_binding, &return_property) {
            return matches!(value, Expression::Undefined | Expression::Null)
                .then(|| no_return_close_effects());
        }
        Some(no_return_close_effects())
    }

    fn static_iterable_user_function_has_observable_effects(&self, function_name: &str) -> bool {
        let Some(user_function) = self.user_function(function_name) else {
            return true;
        };
        self.user_function_mentions_direct_eval(user_function)
            || self.user_function_references_captured_user_function(user_function)
            || user_function.has_lowered_pattern_parameters()
            || !self
                .user_function_parameter_iterator_consumption_indices(user_function)
                .is_empty()
            || !self
                .collect_user_function_assigned_nonlocal_bindings(user_function)
                .is_empty()
            || !self
                .collect_user_function_call_effect_nonlocal_bindings(user_function)
                .is_empty()
    }
    fn static_iterator_object_next_user_function(
        &self,
        function_name: &str,
    ) -> Option<&UserFunction> {
        let user_function = self.user_function(function_name)?;
        if self.user_function_mentions_direct_eval(user_function)
            || self.user_function_references_captured_user_function(user_function)
            || user_function.has_lowered_pattern_parameters()
            || !self
                .user_function_parameter_iterator_consumption_indices(user_function)
                .is_empty()
        {
            return None;
        }
        Some(user_function)
    }

    fn execute_static_iterator_object_next_function_outcome(
        &self,
        function_name: &str,
        bindings: &HashMap<String, Expression>,
        this_binding: &Expression,
        arguments: &[Expression],
        dynamic_capture_names: &[String],
    ) -> Option<(StaticEvalOutcome, HashMap<String, Expression>)> {
        let user_function = self.user_function(function_name)?;
        let mut call_bindings = bindings.clone();
        for capture_name in dynamic_capture_names {
            call_bindings.remove(capture_name);
        }
        for (index, parameter_name) in user_function.params.iter().enumerate() {
            call_bindings.insert(
                parameter_name.clone(),
                arguments
                    .get(index)
                    .cloned()
                    .unwrap_or(Expression::Undefined),
            );
        }
        let snapshot_result = if dynamic_capture_names.is_empty() {
            self.resolve_bound_snapshot_user_function_outcome_with_arguments_and_this(
                function_name,
                &call_bindings,
                arguments,
                this_binding,
            )
        } else {
            None
        };
        snapshot_result.or_else(|| {
            self.execute_simple_static_user_function_with_bindings(function_name, &call_bindings)
                .map(|(value, updated_bindings)| {
                    (StaticEvalOutcome::Value(value), updated_bindings)
                })
        })
    }

    /// Evaluates step expressions exactly where possible and falls back to a
    /// symbolic residual for pure arithmetic over nonlocals whose values are
    /// unknown at the consumption site (`nextCount + 1` stays `nextCount + 1`
    /// and is replayed against the live binding when the step is consumed).
    fn evaluate_symbolic_step_expression(
        &self,
        expression: &Expression,
        bindings: &HashMap<String, Expression>,
    ) -> Option<Expression> {
        if let Some(value) = self.evaluate_simple_static_expression_with_bindings(expression, bindings)
        {
            return Some(value);
        }
        match expression {
            Expression::Binary { op, left, right }
                if matches!(
                    op,
                    BinaryOp::Add
                        | BinaryOp::Subtract
                        | BinaryOp::Multiply
                        | BinaryOp::Divide
                        | BinaryOp::Modulo
                ) =>
            {
                let left = self.evaluate_symbolic_step_expression(left, bindings)?;
                let right = self.evaluate_symbolic_step_expression(right, bindings)?;
                Some(Expression::Binary {
                    op: *op,
                    left: Box::new(left),
                    right: Box::new(right),
                })
            }
            _ => None,
        }
    }

    /// A throw expression replayed at the consumption site must only mention
    /// bindings that resolve there (globals, user functions, builtins) — any
    /// residual reference to the next-function's own locals would rebind.
    fn static_iterator_throw_expression_is_portable(&self, expression: &Expression) -> bool {
        match expression {
            Expression::Number(_)
            | Expression::BigInt(_)
            | Expression::String(_)
            | Expression::Bool(_)
            | Expression::Null
            | Expression::Undefined => true,
            Expression::Identifier(name) => {
                self.resolve_current_local_binding(name).is_none()
                    && (self.backend.global_has_binding(name)
                        || self.backend.global_has_lexical_binding(name)
                        || self.backend.global_has_implicit_binding(name)
                        || self.backend.global_function_binding(name).is_some()
                        || self.contains_user_function(name)
                        || self.resolve_user_function_by_binding_name(name).is_some()
                        || self.is_unshadowed_builtin_identifier(name))
            }
            Expression::New { callee, arguments } | Expression::Call { callee, arguments } => {
                self.static_iterator_throw_expression_is_portable(callee)
                    && arguments.iter().all(|argument| match argument {
                        CallArgument::Expression(expression) => {
                            self.static_iterator_throw_expression_is_portable(expression)
                        }
                        CallArgument::Spread(_) => false,
                    })
            }
            Expression::Binary { left, right, .. } => {
                self.static_iterator_throw_expression_is_portable(left)
                    && self.static_iterator_throw_expression_is_portable(right)
            }
            Expression::Unary { expression, .. } => {
                self.static_iterator_throw_expression_is_portable(expression)
            }
            _ => false,
        }
    }

    /// Steps a throwing `next()` whose pre-throw nonlocal assignments must be
    /// preserved as step effects. Unknown nonlocal initial values are seeded
    /// symbolically so `nextCount += 1; throw ...` yields the relative effect
    /// `nextCount = nextCount + 1` followed by the throw.
    fn execute_static_iterator_object_next_function_throw(
        &self,
        function_name: &str,
        bindings: &HashMap<String, Expression>,
        arguments: &[Expression],
        dynamic_capture_names: &[String],
        symbolic_effect_names: &HashSet<String>,
    ) -> Option<(Expression, HashMap<String, Expression>)> {
        let function = self.resolve_registered_function_declaration(function_name)?;
        let user_function = self.user_function(function_name)?;
        let mut local_bindings = bindings.clone();
        for capture_name in dynamic_capture_names {
            local_bindings.remove(capture_name);
        }
        for name in symbolic_effect_names {
            local_bindings
                .entry(name.clone())
                .or_insert_with(|| Expression::Identifier(name.clone()));
        }
        for (index, parameter_name) in user_function.params.iter().enumerate() {
            local_bindings.insert(
                parameter_name.clone(),
                arguments
                    .get(index)
                    .cloned()
                    .unwrap_or(Expression::Undefined),
            );
        }
        for statement in &function.body {
            match statement {
                Statement::Var { name, value } | Statement::Let { name, value, .. } => {
                    let value = self.evaluate_symbolic_step_expression(value, &local_bindings)?;
                    local_bindings.insert(name.clone(), value);
                }
                Statement::Assign { name, value } => {
                    let value = self.evaluate_symbolic_step_expression(value, &local_bindings)?;
                    local_bindings.insert(name.clone(), value);
                }
                Statement::Throw(value) => {
                    let throw_value = self
                        .evaluate_symbolic_step_expression(value, &local_bindings)
                        .unwrap_or_else(|| value.clone());
                    if !self.static_iterator_throw_expression_is_portable(&throw_value) {
                        return None;
                    }
                    return Some((throw_value, local_bindings));
                }
                Statement::Expression(expression) => {
                    self.evaluate_symbolic_step_expression(expression, &local_bindings)?;
                }
                Statement::Block { body } if body.is_empty() => {}
                _ => return None,
            }
        }
        None
    }

    fn merge_static_iterator_object_bindings(
        bindings: &HashMap<String, Expression>,
        updated_bindings: &HashMap<String, Expression>,
    ) -> HashMap<String, Expression> {
        let mut merged = bindings.clone();
        for (name, value) in updated_bindings {
            merged.insert(name.clone(), value.clone());
        }
        merged
    }

    fn canonical_static_object_identity_expression(&self, expression: &Expression) -> Expression {
        if self
            .resolve_static_reference_identity_key(expression)
            .is_some()
        {
            return expression.clone();
        }
        let Some(object_binding) = self.resolve_object_binding_from_expression(expression) else {
            return expression.clone();
        };

        let mut candidates = Vec::new();
        for (name, binding) in &self
            .state
            .speculation
            .static_semantics
            .objects
            .local_object_bindings
        {
            if binding == &object_binding {
                candidates.push(name.clone());
            }
        }
        for (name, binding) in &self.backend.global_semantics.values.object_bindings {
            if binding == &object_binding {
                candidates.push(name.clone());
            }
        }
        candidates.sort();
        candidates.dedup();
        match candidates.as_slice() {
            [name] => Expression::Identifier(name.clone()),
            _ => expression.clone(),
        }
    }

    fn static_iterator_object_step_effects(
        &self,
        previous_bindings: &HashMap<String, Expression>,
        updated_bindings: &HashMap<String, Expression>,
        effect_names: &HashSet<String>,
    ) -> Vec<Statement> {
        let mut effects = Vec::new();
        for (name, value) in updated_bindings {
            let source_name = scoped_binding_source_name(name).unwrap_or(name);
            if source_name == "this" || source_name == "arguments" {
                continue;
            }
            if !effect_names.contains(source_name) {
                continue;
            }
            let previous_value = previous_bindings
                .get(name)
                .or_else(|| previous_bindings.get(source_name));
            let value = self.canonical_static_object_identity_expression(value);
            if previous_value.is_some_and(|previous| {
                let previous = self.canonical_static_object_identity_expression(previous);
                static_expression_matches(&previous, &value)
            }) {
                continue;
            }
            effects.push(Statement::Assign {
                name: source_name.to_string(),
                value,
            });
        }
        effects
    }

    fn evaluate_static_iterator_step_field(
        &self,
        expression: Expression,
        bindings: &HashMap<String, Expression>,
        current_function_name: Option<&str>,
    ) -> Expression {
        self.evaluate_bound_snapshot_expression(
            &expression,
            &mut bindings.clone(),
            current_function_name,
        )
        .or_else(|| self.evaluate_simple_static_expression_with_bindings(&expression, bindings))
        .unwrap_or(expression)
    }

    fn static_throw_value_expression(&self, throw_value: &StaticThrowValue) -> Option<Expression> {
        self.resolve_static_throw_value_expression(throw_value)
    }

    fn static_iterator_next_function_or_throw(
        &self,
        iterator_expression: &Expression,
        iterator_binding: &ObjectValueBinding,
        current_function_name: Option<&str>,
    ) -> Option<Result<String, Expression>> {
        let next_property = Expression::String("next".to_string());
        let next_value = if let Some(descriptor) =
            object_binding_lookup_descriptor(iterator_binding, &next_property)
        {
            if let Some(getter) = &descriptor.getter {
                let getter_binding = self.resolve_function_binding_from_expression(getter)?;
                let outcome = self
                    .resolve_static_function_outcome_from_binding_with_call_frame_and_context(
                        &getter_binding,
                        &[],
                        iterator_expression,
                        current_function_name,
                    )
                    .or_else(|| {
                        self.resolve_static_function_outcome_from_binding_with_context(
                            &getter_binding,
                            &[],
                            current_function_name,
                        )
                    })?;
                match outcome {
                    StaticEvalOutcome::Value(value) => value,
                    StaticEvalOutcome::Throw(throw_value) => {
                        return Some(Err(self.static_throw_value_expression(&throw_value)?));
                    }
                }
            } else if descriptor.has_get {
                return None;
            } else if let Some(value) = &descriptor.value {
                value.clone()
            } else {
                Expression::Undefined
            }
        } else {
            object_binding_lookup_value(iterator_binding, &next_property)?.clone()
        };

        let LocalFunctionBinding::User(next_function_name) =
            self.resolve_function_binding_from_expression(&next_value)?
        else {
            return None;
        };
        Some(Ok(next_function_name))
    }

    fn expression_is_static_non_object_iterator_result(expression: &Expression) -> bool {
        matches!(
            expression,
            Expression::Number(_)
                | Expression::BigInt(_)
                | Expression::String(_)
                | Expression::Bool(_)
                | Expression::Null
                | Expression::Undefined
        )
    }

    fn resolve_static_iterator_step_value_outcome(
        &self,
        step_result: &Expression,
        step_object_binding: &ObjectValueBinding,
        step_bindings: &HashMap<String, Expression>,
        current_function_name: Option<&str>,
    ) -> Option<(SimpleGeneratorStepOutcome, bool)> {
        let value_property = Expression::String("value".to_string());
        if let Some(descriptor) =
            object_binding_lookup_descriptor(step_object_binding, &value_property)
        {
            if let Some(getter) = &descriptor.getter {
                let getter_binding = self.resolve_function_binding_from_expression(getter)?;
                let outcome = self
                    .resolve_static_function_outcome_from_binding_with_call_frame_and_context(
                        &getter_binding,
                        &[],
                        step_result,
                        current_function_name,
                    )
                    .or_else(|| {
                        self.resolve_static_function_outcome_from_binding_with_context(
                            &getter_binding,
                            &[],
                            current_function_name,
                        )
                    })?;
                return match outcome {
                    StaticEvalOutcome::Value(value) => {
                        let value = self.evaluate_static_iterator_step_field(
                            value,
                            step_bindings,
                            current_function_name,
                        );
                        Some((SimpleGeneratorStepOutcome::Yield(value), true))
                    }
                    StaticEvalOutcome::Throw(throw_value) => Some((
                        SimpleGeneratorStepOutcome::Throw(
                            self.static_throw_value_expression(&throw_value)?,
                        ),
                        true,
                    )),
                };
            }
            if descriptor.has_get {
                return Some((
                    SimpleGeneratorStepOutcome::Yield(Expression::Undefined),
                    true,
                ));
            }
            if let Some(value) = &descriptor.value {
                let value = self.evaluate_static_iterator_step_field(
                    value.clone(),
                    step_bindings,
                    current_function_name,
                );
                return Some((SimpleGeneratorStepOutcome::Yield(value), false));
            }
            return Some((
                SimpleGeneratorStepOutcome::Yield(Expression::Undefined),
                false,
            ));
        }

        let value = object_binding_lookup_value(step_object_binding, &value_property)
            .cloned()
            .unwrap_or(Expression::Undefined);
        let value =
            self.evaluate_static_iterator_step_field(value, step_bindings, current_function_name);
        Some((SimpleGeneratorStepOutcome::Yield(value), false))
    }

    pub(in crate::backend::direct_wasm) fn resolve_static_iterator_step_done_outcome(
        &self,
        step_result: &Expression,
        step_object_binding: &ObjectValueBinding,
        step_bindings: &HashMap<String, Expression>,
        current_function_name: Option<&str>,
    ) -> Option<Result<Expression, Expression>> {
        let done_property = Expression::String("done".to_string());
        let done = if let Some(descriptor) =
            object_binding_lookup_descriptor(step_object_binding, &done_property)
        {
            if let Some(getter) = &descriptor.getter {
                let getter_binding = self.resolve_function_binding_from_expression(getter)?;
                let outcome = self
                    .resolve_static_function_outcome_from_binding_with_call_frame_and_context(
                        &getter_binding,
                        &[],
                        step_result,
                        current_function_name,
                    )
                    .or_else(|| {
                        self.resolve_static_function_outcome_from_binding_with_context(
                            &getter_binding,
                            &[],
                            current_function_name,
                        )
                    })?;
                match outcome {
                    StaticEvalOutcome::Value(value) => value,
                    StaticEvalOutcome::Throw(throw_value) => {
                        return Some(Err(self.static_throw_value_expression(&throw_value)?));
                    }
                }
            } else if descriptor.has_get {
                return None;
            } else if let Some(value) = &descriptor.value {
                value.clone()
            } else {
                Expression::Undefined
            }
        } else {
            object_binding_lookup_value(step_object_binding, &done_property)
                .cloned()
                .unwrap_or(Expression::Bool(false))
        };
        let done =
            self.evaluate_static_iterator_step_field(done, step_bindings, current_function_name);
        Some(Ok(done))
    }

    fn resolve_static_iterator_step_completion_value_outcome(
        &self,
        step_result: &Expression,
        step_object_binding: &ObjectValueBinding,
        step_bindings: &HashMap<String, Expression>,
        current_function_name: Option<&str>,
    ) -> Option<Result<Expression, Expression>> {
        let (outcome, _) = self.resolve_static_iterator_step_value_outcome(
            step_result,
            step_object_binding,
            step_bindings,
            current_function_name,
        )?;
        match outcome {
            SimpleGeneratorStepOutcome::Yield(value)
            | SimpleGeneratorStepOutcome::YieldResult(value) => Some(Ok(value)),
            SimpleGeneratorStepOutcome::Throw(value) => Some(Err(value)),
        }
    }

    pub(in crate::backend::direct_wasm) fn resolve_static_iterable_simple_generator_source(
        &self,
        expression: &Expression,
    ) -> Option<(Vec<SimpleGeneratorStep>, Vec<Statement>, Expression)> {
        let object_binding = self.resolve_object_binding_from_expression(expression)?;
        let symbol_iterator = self.materialize_static_expression(&Expression::Member {
            object: Box::new(Expression::Identifier("Symbol".to_string())),
            property: Box::new(Expression::String("iterator".to_string())),
        });
        let iterator_method =
            object_binding_lookup_value(&object_binding, &symbol_iterator)?.clone();
        let LocalFunctionBinding::User(iterator_function_name) =
            self.resolve_function_binding_from_expression(&iterator_method)?
        else {
            return None;
        };
        if self.static_iterable_user_function_has_observable_effects(&iterator_function_name) {
            return None;
        }
        let (iterator_result, iterator_bindings) = self
            .execute_simple_static_user_function_with_bindings(
                &iterator_function_name,
                &HashMap::new(),
            )?;
        let iterator_result_binding =
            self.resolve_object_binding_from_expression(&iterator_result)?;
        if self.static_iterator_result_has_observable_return(&iterator_result_binding) {
            return None;
        }
        let next_function_name = match self.static_iterator_next_function_or_throw(
            &iterator_result,
            &iterator_result_binding,
            self.current_function_name(),
        )? {
            Ok(next_function_name) => next_function_name,
            Err(throw_value) => {
                return Some((
                    vec![SimpleGeneratorStep {
                        effects: Vec::new(),
                        close_effects: Vec::new(),
                        outcome: SimpleGeneratorStepOutcome::Throw(throw_value),
                    }],
                    Vec::new(),
                    Expression::Undefined,
                ));
            }
        };
        if self.static_iterable_user_function_has_observable_effects(&next_function_name) {
            return None;
        }
        let next_function_binding = LocalFunctionBinding::User(next_function_name.clone());
        let next_call_arguments = [CallArgument::Expression(Expression::Undefined)];

        let mut step_bindings = iterator_bindings;
        let mut steps = Vec::new();
        let mut saw_accessor_value = false;
        for _ in 0..256 {
            let (step_result, updated_bindings) = if let Some(outcome) = self
                .resolve_static_function_outcome_from_binding_with_call_frame_and_context(
                    &next_function_binding,
                    &next_call_arguments,
                    &iterator_result,
                    Some(&next_function_name),
                ) {
                match outcome {
                    StaticEvalOutcome::Value(value) => {
                        let value = self.evaluate_static_iterator_step_field(
                            value,
                            &step_bindings,
                            Some(&next_function_name),
                        );
                        (value, step_bindings.clone())
                    }
                    StaticEvalOutcome::Throw(throw_value) => {
                        steps.push(SimpleGeneratorStep {
                            effects: Vec::new(),
                            close_effects: Vec::new(),
                            outcome: SimpleGeneratorStepOutcome::Throw(
                                self.static_throw_value_expression(&throw_value)?,
                            ),
                        });
                        return Some((steps, Vec::new(), Expression::Undefined));
                    }
                }
            } else {
                self.execute_simple_static_user_function_with_bindings(
                    &next_function_name,
                    &step_bindings,
                )?
            };
            step_bindings = updated_bindings;
            let Some(step_object_binding) =
                self.resolve_object_binding_from_expression(&step_result)
            else {
                if Self::expression_is_static_non_object_iterator_result(&step_result) {
                    steps.push(SimpleGeneratorStep {
                        effects: Vec::new(),
                        close_effects: Vec::new(),
                        outcome: SimpleGeneratorStepOutcome::Throw(
                            self.static_throw_value_expression(&StaticThrowValue::NamedError(
                                "TypeError",
                            ))?,
                        ),
                    });
                    return Some((steps, Vec::new(), Expression::Undefined));
                }
                return None;
            };
            let done = match self.resolve_static_iterator_step_done_outcome(
                &step_result,
                &step_object_binding,
                &step_bindings,
                Some(&next_function_name),
            )? {
                Ok(done) => done,
                Err(throw_value) => {
                    steps.push(SimpleGeneratorStep {
                        effects: Vec::new(),
                        close_effects: Vec::new(),
                        outcome: SimpleGeneratorStepOutcome::Throw(throw_value),
                    });
                    return Some((steps, Vec::new(), Expression::Undefined));
                }
            };
            match done {
                Expression::Bool(true) => {
                    return saw_accessor_value.then_some((
                        steps,
                        Vec::new(),
                        Expression::Undefined,
                    ));
                }
                Expression::Bool(false) => {
                    let (outcome, used_accessor) = self
                        .resolve_static_iterator_step_value_outcome(
                            &step_result,
                            &step_object_binding,
                            &step_bindings,
                            Some(&next_function_name),
                        )?;
                    saw_accessor_value |= used_accessor;
                    let outcome_is_throw = matches!(outcome, SimpleGeneratorStepOutcome::Throw(_));
                    steps.push(SimpleGeneratorStep {
                        effects: Vec::new(),
                        close_effects: Vec::new(),
                        outcome,
                    });
                    if outcome_is_throw {
                        return Some((steps, Vec::new(), Expression::Undefined));
                    }
                }
                _ => return None,
            }
        }

        None
    }

    pub(in crate::backend::direct_wasm) fn resolve_static_iterator_object_simple_generator_source(
        &self,
        expression: &Expression,
    ) -> Option<(Vec<SimpleGeneratorStep>, Vec<Statement>, Expression)> {
        self.resolve_static_iterator_object_simple_generator_source_with_seed(expression, None)
    }

    /// `seed_bindings` carries the closure state produced by statically
    /// executing an iterator-producing method (its locals after execution),
    /// so `next()` closures over that state can be stepped statically.
    /// Seeded names with no binding at the consumption site are internal to
    /// the closure: they are threaded through the steps but never emitted as
    /// observable step effects.
    pub(in crate::backend::direct_wasm) fn resolve_static_iterator_object_simple_generator_source_with_seed(
        &self,
        expression: &Expression,
        seed_bindings: Option<&HashMap<String, Expression>>,
    ) -> Option<(Vec<SimpleGeneratorStep>, Vec<Statement>, Expression)> {
        let trace = crate::ayy_env_flag!("AYY_TRACE_STATIC_ITERATOR_OBJECT");
        macro_rules! trace {
            ($($arg:tt)*) => {
                if trace {
                    eprintln!("static_iterator_object:{}", format_args!($($arg)*));
                }
            };
        }
        trace!("start expression={expression:?}");
        let Some(object_binding) = self.resolve_object_binding_from_expression(expression) else {
            trace!("reject object_binding");
            return None;
        };
        let Some(close_effects) =
            self.static_iterator_object_return_close_effects(expression, &object_binding)
        else {
            trace!("reject observable_return");
            return None;
        };
        let next_function_name = match self.static_iterator_next_function_or_throw(
            expression,
            &object_binding,
            self.current_function_name(),
        )? {
            Ok(next_function_name) => next_function_name,
            Err(throw_value) => {
                trace!("next_getter_throw");
                return Some((
                    vec![SimpleGeneratorStep {
                        effects: Vec::new(),
                        close_effects: Vec::new(),
                        outcome: SimpleGeneratorStepOutcome::Throw(throw_value),
                    }],
                    Vec::new(),
                    Expression::Undefined,
                ));
            }
        };
        let Some(next_user_function) =
            self.static_iterator_object_next_user_function(&next_function_name)
        else {
            trace!("reject next_user_function function={next_function_name}");
            return None;
        };
        let mut effect_names =
            self.collect_user_function_assigned_nonlocal_bindings(next_user_function);
        effect_names
            .extend(self.collect_user_function_call_effect_nonlocal_bindings(next_user_function));
        let capture_names = self
            .user_function_capture_bindings(&next_function_name)
            .map(|bindings| bindings.keys().cloned().collect::<Vec<_>>())
            .unwrap_or_default();
        let dynamic_capture_names = capture_names
            .iter()
            .filter(|name| !effect_names.contains(*name))
            .cloned()
            .collect::<Vec<_>>();
        trace!("next_function={next_function_name} effect_names={effect_names:?}");

        let mut step_bindings = HashMap::new();
        for name in effect_names.iter().chain(capture_names.iter()) {
            if let Some(value) = self
                .state
                .speculation
                .static_semantics
                .local_value_binding(name)
                .or_else(|| self.global_value_binding(name))
            {
                step_bindings.insert(name.clone(), value.clone());
            }
        }
        // Closure-internal state from the seeding iterator method: fill any
        // names the consumption site cannot resolve, and exclude them from
        // observable step effects (they are invisible outside the closure).
        let mut internal_seed_names = HashSet::new();
        if let Some(seed_bindings) = seed_bindings {
            for (name, value) in seed_bindings {
                if !step_bindings.contains_key(name) {
                    step_bindings.insert(name.clone(), value.clone());
                    internal_seed_names.insert(name.clone());
                }
            }
        }
        let external_effect_names = effect_names
            .iter()
            .filter(|name| !internal_seed_names.contains(*name))
            .cloned()
            .collect::<HashSet<_>>();
        trace!("initial_bindings={step_bindings:?} internal={internal_seed_names:?}");

        let next_function_binding = LocalFunctionBinding::User(next_function_name.clone());
        let mut steps = Vec::new();
        for step_index in 0..256 {
            let next_argument = if step_index == 0 {
                Expression::Undefined
            } else {
                Expression::Sent
            };
            let next_call_arguments = [CallArgument::Expression(next_argument.clone())];
            // A `next` that assigns nonlocal state must be stepped through the
            // binding-threading executor: the call-frame resolver below knows
            // nothing of `step_bindings`, so it would fold each step against
            // stale (or missing) closure state.
            let call_frame_outcome = effect_names
                .is_empty()
                .then(|| {
                    self.resolve_static_function_outcome_from_binding_with_call_frame_and_context(
                        &next_function_binding,
                        &next_call_arguments,
                        expression,
                        Some(&next_function_name),
                    )
                })
                .flatten();
            let (step_result, updated_bindings) = if let Some(outcome) = call_frame_outcome {
                match outcome {
                    StaticEvalOutcome::Value(value) => {
                        let value = self.evaluate_static_iterator_step_field(
                            value,
                            &step_bindings,
                            Some(&next_function_name),
                        );
                        (value, step_bindings.clone())
                    }
                    StaticEvalOutcome::Throw(throw_value) => {
                        let Some(throw_expression) =
                            self.static_throw_value_expression(&throw_value)
                        else {
                            trace!("reject next_throw_expression step={step_index}");
                            return None;
                        };
                        steps.push(SimpleGeneratorStep {
                            effects: Vec::new(),
                            close_effects: Vec::new(),
                            outcome: SimpleGeneratorStepOutcome::Throw(throw_expression),
                        });
                        trace!("next_throw step={step_index}");
                        return Some((steps, Vec::new(), Expression::Undefined));
                    }
                }
            } else {
                let executed = self.execute_static_iterator_object_next_function_outcome(
                    &next_function_name,
                    &step_bindings,
                    expression,
                    std::slice::from_ref(&next_argument),
                    &dynamic_capture_names,
                );
                let value_result = match executed {
                    Some((StaticEvalOutcome::Value(value), updated_bindings)) => {
                        Some((value, updated_bindings))
                    }
                    Some((StaticEvalOutcome::Throw(throw_value), updated_bindings)) => {
                        // A throwing next() must surface as a Throw step that
                        // still carries its pre-throw nonlocal assignments.
                        if let Some(throw_expression) =
                            self.static_throw_value_expression(&throw_value)
                            && self.static_iterator_throw_expression_is_portable(&throw_expression)
                        {
                            let step_effects = self.static_iterator_object_step_effects(
                                &step_bindings,
                                &updated_bindings,
                                &external_effect_names,
                            );
                            steps.push(SimpleGeneratorStep {
                                effects: step_effects,
                                close_effects: Vec::new(),
                                outcome: SimpleGeneratorStepOutcome::Throw(throw_expression),
                            });
                            trace!("next_throw_with_effects step={step_index}");
                            return Some((steps, Vec::new(), Expression::Undefined));
                        }
                        None
                    }
                    None => None,
                };
                match value_result {
                    Some(result) => result,
                    None => {
                        // Bindings may be unknown at this site (invalidated by
                        // the surrounding call boundary); replay the throwing
                        // next() symbolically so `nextCount += 1; throw ...`
                        // still registers its pre-throw effects.
                        let symbolic_effect_names = external_effect_names
                            .iter()
                            .filter(|name| !step_bindings.contains_key(*name))
                            .cloned()
                            .collect::<HashSet<_>>();
                        let Some((throw_expression, updated_bindings)) = self
                            .execute_static_iterator_object_next_function_throw(
                                &next_function_name,
                                &step_bindings,
                                std::slice::from_ref(&next_argument),
                                &dynamic_capture_names,
                                &symbolic_effect_names,
                            )
                        else {
                            trace!(
                                "reject next_execution step={step_index} bindings={step_bindings:?}"
                            );
                            return None;
                        };
                        let mut throw_step_bindings = step_bindings.clone();
                        for name in &symbolic_effect_names {
                            throw_step_bindings
                                .entry(name.clone())
                                .or_insert_with(|| Expression::Identifier(name.clone()));
                        }
                        let step_effects = self.static_iterator_object_step_effects(
                            &throw_step_bindings,
                            &updated_bindings,
                            &external_effect_names,
                        );
                        steps.push(SimpleGeneratorStep {
                            effects: step_effects,
                            close_effects: Vec::new(),
                            outcome: SimpleGeneratorStepOutcome::Throw(throw_expression),
                        });
                        trace!("next_throw_symbolic step={step_index}");
                        return Some((steps, Vec::new(), Expression::Undefined));
                    }
                }
            };
            trace!("step={step_index} result={step_result:?} updated={updated_bindings:?}");
            let step_effects = self.static_iterator_object_step_effects(
                &step_bindings,
                &updated_bindings,
                &external_effect_names,
            );
            step_bindings =
                Self::merge_static_iterator_object_bindings(&step_bindings, &updated_bindings);
            let Some(step_object_binding) =
                self.resolve_object_binding_from_expression(&step_result)
            else {
                if Self::expression_is_static_non_object_iterator_result(&step_result) {
                    steps.push(SimpleGeneratorStep {
                        effects: step_effects,
                        close_effects: Vec::new(),
                        outcome: SimpleGeneratorStepOutcome::Throw(
                            self.static_throw_value_expression(&StaticThrowValue::NamedError(
                                "TypeError",
                            ))?,
                        ),
                    });
                    trace!("non_object_step_result step={step_index}");
                    return Some((steps, Vec::new(), Expression::Undefined));
                }
                trace!("reject step_object_binding step={step_index} result={step_result:?}");
                return None;
            };
            let done = match self.resolve_static_iterator_step_done_outcome(
                &step_result,
                &step_object_binding,
                &step_bindings,
                Some(&next_function_name),
            )? {
                Ok(done) => done,
                Err(throw_value) => {
                    steps.push(SimpleGeneratorStep {
                        effects: step_effects,
                        close_effects: Vec::new(),
                        outcome: SimpleGeneratorStepOutcome::Throw(throw_value),
                    });
                    trace!("done_throw step={step_index}");
                    return Some((steps, Vec::new(), Expression::Undefined));
                }
            };
            trace!("step={step_index} done={done:?} effects={step_effects:?}");
            match done {
                Expression::Bool(true) => {
                    let value = match self.resolve_static_iterator_step_completion_value_outcome(
                        &step_result,
                        &step_object_binding,
                        &step_bindings,
                        Some(&next_function_name),
                    )? {
                        Ok(value) => value,
                        Err(throw_value) => {
                            steps.push(SimpleGeneratorStep {
                                effects: step_effects,
                                close_effects: Vec::new(),
                                outcome: SimpleGeneratorStepOutcome::Throw(throw_value),
                            });
                            trace!("completion_value_throw step={step_index}");
                            return Some((steps, Vec::new(), Expression::Undefined));
                        }
                    };
                    trace!("done completion step={step_index} value={value:?}");
                    return Some((steps, step_effects, value));
                }
                Expression::Bool(false) => {
                    trace!("yield step={step_index} outcome=result");
                    steps.push(SimpleGeneratorStep {
                        effects: step_effects,
                        close_effects: close_effects.clone(),
                        outcome: SimpleGeneratorStepOutcome::YieldResult(step_result),
                    });
                }
                _ => {
                    trace!("reject non_boolean_done step={step_index} done={done:?}");
                    return None;
                }
            }
        }

        if !steps.is_empty()
            && (!close_effects.is_empty()
                || steps
                    .iter()
                    .all(|step| matches!(step.outcome, SimpleGeneratorStepOutcome::YieldResult(_))))
        {
            trace!(
                "step_limit returning closeable prefix steps={}",
                steps.len()
            );
            return Some((steps, Vec::new(), Expression::Undefined));
        }
        trace!("reject step_limit");
        None
    }

    /// Bridges a `Symbol.iterator` iterable to the iterator-object simple
    /// generator classifier: statically executes the (effect-free) iterator
    /// method and classifies the returned iterator object, which supports
    /// observable `return()` close effects and infinite step prefixes that
    /// `resolve_static_iterable_simple_generator_source` rejects.
    pub(in crate::backend::direct_wasm) fn resolve_static_iterable_iterator_object_simple_generator_source(
        &self,
        expression: &Expression,
    ) -> Option<(Vec<SimpleGeneratorStep>, Vec<Statement>, Expression)> {
        let trace = crate::ayy_env_flag!("AYY_TRACE_STATIC_ITERATOR_OBJECT");
        let object_binding = self.resolve_object_binding_from_expression(expression)?;
        let symbol_iterator = self.materialize_static_expression(&Expression::Member {
            object: Box::new(Expression::Identifier("Symbol".to_string())),
            property: Box::new(Expression::String("iterator".to_string())),
        });
        let Some(iterator_method) =
            object_binding_lookup_value(&object_binding, &symbol_iterator).cloned()
        else {
            if trace {
                eprintln!("static_iterable_iterator_object:reject symbol_iterator_lookup");
            }
            return None;
        };
        let Some(LocalFunctionBinding::User(iterator_function_name)) =
            self.resolve_function_binding_from_expression(&iterator_method)
        else {
            if trace {
                eprintln!("static_iterable_iterator_object:reject iterator_function_binding");
            }
            return None;
        };
        // The iterator method body itself must be effect-free (per its inline
        // summary, which covers direct effects only), but effects of the
        // methods on the iterator object it returns (such as an observable
        // `return()`) are classified downstream as close effects.
        let iterator_user_function = self.user_function(&iterator_function_name)?;
        // Nested closures on the returned iterator object may capture
        // nonlocals (an observable `return()` for example); those are modeled
        // downstream by the iterator-object classifier, so captured-function
        // references alone do not disqualify the iterator method here.
        if self.user_function_mentions_direct_eval(iterator_user_function)
            || iterator_user_function.has_lowered_pattern_parameters()
            || !self
                .user_function_parameter_iterator_consumption_indices(iterator_user_function)
                .is_empty()
            || !iterator_user_function
                .inline_summary
                .as_ref()
                .is_some_and(|summary| summary.effects.is_empty())
        {
            if trace {
                eprintln!(
                    "static_iterable_iterator_object:reject iterator_function_effects function={iterator_function_name} direct_eval={} captured_fn={} patterns={} iter_consumption={} summary_present={} effects_len={:?}",
                    self.user_function_mentions_direct_eval(iterator_user_function),
                    self.user_function_references_captured_user_function(iterator_user_function),
                    iterator_user_function.has_lowered_pattern_parameters(),
                    !self
                        .user_function_parameter_iterator_consumption_indices(
                            iterator_user_function
                        )
                        .is_empty(),
                    iterator_user_function.inline_summary.is_some(),
                    iterator_user_function
                        .inline_summary
                        .as_ref()
                        .map(|summary| summary.effects.len())
                );
            }
            return None;
        }
        let Some((iterator_result, iterator_bindings)) = self
            .execute_simple_static_user_function_with_bindings(
                &iterator_function_name,
                &HashMap::new(),
            )
        else {
            if trace {
                eprintln!(
                    "static_iterable_iterator_object:reject iterator_execution function={iterator_function_name}"
                );
            }
            return None;
        };
        if trace {
            eprintln!("static_iterable_iterator_object:result {iterator_result:?}");
        }
        self.resolve_static_iterator_object_simple_generator_source_with_seed(
            &iterator_result,
            Some(&iterator_bindings),
        )
    }

    pub(in crate::backend::direct_wasm) fn resolve_static_iterable_binding_from_expression(
        &self,
        expression: &Expression,
    ) -> Option<ArrayValueBinding> {
        let _guard = StaticIterableBindingGuard::enter(expression)?;
        if let Some(binding) = self.resolve_static_user_iterator_binding(expression) {
            return Some(binding);
        }
        let object_binding = self.resolve_object_binding_from_expression(expression)?;
        let symbol_iterator = self.materialize_static_expression(&Expression::Member {
            object: Box::new(Expression::Identifier("Symbol".to_string())),
            property: Box::new(Expression::String("iterator".to_string())),
        });
        let iterator_method =
            object_binding_lookup_value(&object_binding, &symbol_iterator)?.clone();
        let LocalFunctionBinding::User(iterator_function_name) =
            self.resolve_function_binding_from_expression(&iterator_method)?
        else {
            return None;
        };
        if self.static_iterable_user_function_has_observable_effects(&iterator_function_name) {
            return None;
        }
        let (iterator_result, iterator_bindings) = self
            .execute_simple_static_user_function_with_bindings(
                &iterator_function_name,
                &HashMap::new(),
            )?;
        let iterator_result_binding =
            self.resolve_object_binding_from_expression(&iterator_result)?;
        if self.static_iterator_result_has_observable_return(&iterator_result_binding) {
            return None;
        }
        let next_value = object_binding_lookup_value(
            &iterator_result_binding,
            &Expression::String("next".to_string()),
        )?
        .clone();
        let LocalFunctionBinding::User(next_function_name) =
            self.resolve_function_binding_from_expression(&next_value)?
        else {
            return None;
        };
        if self.static_iterable_user_function_has_observable_effects(&next_function_name) {
            return None;
        }

        let mut step_bindings = iterator_bindings;
        let mut values = Vec::new();
        for _ in 0..256 {
            let (step_result, updated_bindings) = self
                .execute_simple_static_user_function_with_bindings(
                    &next_function_name,
                    &step_bindings,
                )?;
            step_bindings = updated_bindings;
            let step_object_binding = self.resolve_object_binding_from_expression(&step_result)?;
            let done = object_binding_lookup_value(
                &step_object_binding,
                &Expression::String("done".to_string()),
            )
            .cloned()
            .unwrap_or(Expression::Bool(false));
            let value = object_binding_lookup_value(
                &step_object_binding,
                &Expression::String("value".to_string()),
            )
            .cloned()
            .unwrap_or(Expression::Undefined);
            let done = self.evaluate_static_iterator_step_field(
                done,
                &step_bindings,
                Some(&next_function_name),
            );
            let value = self.evaluate_static_iterator_step_field(
                value,
                &step_bindings,
                Some(&next_function_name),
            );
            match done {
                Expression::Bool(true) => return Some(ArrayValueBinding { values }),
                Expression::Bool(false) => values.push(Some(value)),
                _ => return None,
            }
        }

        None
    }

    pub(in crate::backend::direct_wasm) fn resolve_static_user_iterator_binding(
        &self,
        expression: &Expression,
    ) -> Option<ArrayValueBinding> {
        let (user_function, _) = self.resolve_user_function_call_target(expression)?;
        if user_function
            .returned_member_function_bindings
            .iter()
            .any(|binding| binding.property == "return")
            || user_function
                .returned_member_value_bindings
                .iter()
                .any(|binding| {
                    binding.property == "return"
                        && !matches!(binding.value, Expression::Undefined | Expression::Null)
                })
        {
            return None;
        }
        let next_binding = user_function
            .returned_member_function_bindings
            .iter()
            .find(|binding| binding.property == "next")?;
        let LocalFunctionBinding::User(next_function_name) = &next_binding.binding else {
            return None;
        };
        let mut property_bindings =
            self.resolve_returned_member_capture_bindings_for_value(expression)?;
        let capture_bindings = property_bindings.remove("next")?;

        let mut bindings = capture_bindings;
        let mut values = Vec::new();
        for _ in 0..256 {
            let (step_result, updated_bindings) =
                self.resolve_bound_snapshot_user_function_result(next_function_name, &bindings)?;
            bindings = updated_bindings;
            let step_object_binding = self.resolve_object_binding_from_expression(&step_result)?;
            let done = object_binding_lookup_value(
                &step_object_binding,
                &Expression::String("done".to_string()),
            )
            .cloned()
            .unwrap_or(Expression::Bool(false));
            let value = object_binding_lookup_value(
                &step_object_binding,
                &Expression::String("value".to_string()),
            )
            .cloned()
            .unwrap_or(Expression::Undefined);
            let done =
                self.evaluate_static_iterator_step_field(done, &bindings, Some(next_function_name));
            let value = self.evaluate_static_iterator_step_field(
                value,
                &bindings,
                Some(next_function_name),
            );
            match done {
                Expression::Bool(true) => return Some(ArrayValueBinding { values }),
                Expression::Bool(false) => values.push(Some(value)),
                _ => return None,
            }
        }

        None
    }
}
