use super::*;

thread_local! {
    static ACTIVE_STATIC_ITERABLE_BINDING_SHAPES: RefCell<HashSet<String>> = RefCell::new(HashSet::new());
}

struct StaticIterableBindingGuard {
    key: String,
}

impl StaticIterableBindingGuard {
    fn enter(expression: &Expression) -> Option<Self> {
        let key = format!("{expression:?}");
        let inserted = ACTIVE_STATIC_ITERABLE_BINDING_SHAPES
            .with(|active| active.borrow_mut().insert(key.clone()));
        inserted.then_some(Self { key })
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

    fn execute_static_iterator_object_next_function(
        &self,
        function_name: &str,
        bindings: &HashMap<String, Expression>,
        this_binding: &Expression,
        arguments: &[Expression],
        dynamic_capture_names: &[String],
    ) -> Option<(Expression, HashMap<String, Expression>)> {
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
            self.resolve_bound_snapshot_user_function_result_with_arguments_and_this(
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
        })
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
        let trace = std::env::var_os("AYY_TRACE_STATIC_ITERATOR_OBJECT").is_some();
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
        trace!("initial_bindings={step_bindings:?}");

        let next_function_binding = LocalFunctionBinding::User(next_function_name.clone());
        let mut steps = Vec::new();
        for step_index in 0..256 {
            let next_argument = if step_index == 0 {
                Expression::Undefined
            } else {
                Expression::Sent
            };
            let next_call_arguments = [CallArgument::Expression(next_argument.clone())];
            let (step_result, updated_bindings) = if let Some(outcome) = self
                .resolve_static_function_outcome_from_binding_with_call_frame_and_context(
                    &next_function_binding,
                    &next_call_arguments,
                    expression,
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
                let Some((step_result, updated_bindings)) = self
                    .execute_static_iterator_object_next_function(
                        &next_function_name,
                        &step_bindings,
                        expression,
                        std::slice::from_ref(&next_argument),
                        &dynamic_capture_names,
                    )
                else {
                    trace!("reject next_execution step={step_index} bindings={step_bindings:?}");
                    return None;
                };
                (step_result, updated_bindings)
            };
            trace!("step={step_index} result={step_result:?} updated={updated_bindings:?}");
            let step_effects = self.static_iterator_object_step_effects(
                &step_bindings,
                &updated_bindings,
                &effect_names,
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
