use super::*;

pub(in crate::backend::direct_wasm) enum SimpleGeneratorNextEffectConsumption {
    NotApplicable,
    EmittedNoThrow,
    Threw(StaticThrowValue),
}

struct StaticIteratorUnrollStep {
    effects: Vec<Statement>,
    value: Expression,
}

struct StaticIteratorUnroll {
    steps: Vec<StaticIteratorUnrollStep>,
    completion_effects: Vec<Statement>,
}

impl<'a> FunctionCompiler<'a> {
    fn initialize_fresh_simple_generator_scoped_var_bindings(&mut self, effects: &[Statement]) {
        let mut scoped_var_names = Vec::new();
        Self::collect_simple_generator_scoped_var_bindings(effects, &mut scoped_var_names);
        for name in scoped_var_names {
            if let Some((_, local_index)) = self.resolve_current_local_binding(&name) {
                self.push_i32_const(JS_UNDEFINED_TAG);
                self.push_local_set(local_index);
            }
        }
    }

    fn is_static_iterator_step_binding_name(name: &str) -> bool {
        name.starts_with("__ayy_array_step_") || name.starts_with("__ayy_for_of_step_")
    }

    fn cached_simple_generator_binding_value(value: &Expression) -> Expression {
        match value {
            Expression::Sequence(expressions) => expressions
                .last()
                .map(Self::cached_simple_generator_binding_value)
                .unwrap_or(Expression::Undefined),
            other => other.clone(),
        }
    }

    pub(in crate::backend::direct_wasm) fn set_static_iterator_index_for_index_local(
        &mut self,
        index_local: u32,
        next_index: usize,
    ) {
        for binding in self
            .state
            .speculation
            .static_semantics
            .arrays
            .local_array_iterator_bindings
            .values_mut()
        {
            if binding.index_local == index_local {
                binding.static_index = Some(next_index);
            }
        }
    }

    fn emit_static_simple_generator_iterator_effects(
        &mut self,
        effects: &[Statement],
        sent_value: &Expression,
        strict_mode: bool,
    ) -> DirectResult<()> {
        let substituted_effects = effects
            .iter()
            .map(|effect| Self::substitute_sent_statement(effect, sent_value))
            .collect::<Vec<_>>();
        let substituted_effects =
            self.expand_static_lowered_for_of_completion_effects(&substituted_effects);
        self.register_bindings(&substituted_effects)?;
        let scoped_bindings = Self::simple_generator_scoped_effect_bindings(&substituted_effects);
        for (source_name, scoped_name) in &scoped_bindings {
            self.state
                .push_scoped_lexical_binding(source_name, scoped_name.clone());
        }
        let scoped_source_names = scoped_bindings
            .iter()
            .map(|(source_name, _)| source_name.clone())
            .collect::<Vec<_>>();
        self.with_scoped_lexical_bindings_cleanup(scoped_source_names, |compiler| {
            let mut prior_effects = Vec::new();
            for effect in &substituted_effects {
                match compiler.consume_throwing_simple_generator_next_effect_with_prior(
                    effect,
                    &prior_effects,
                    strict_mode,
                )? {
                    SimpleGeneratorNextEffectConsumption::Threw(throw_value) => {
                        compiler.emit_static_throw_value(&throw_value)?;
                    }
                    SimpleGeneratorNextEffectConsumption::EmittedNoThrow => {}
                    SimpleGeneratorNextEffectConsumption::NotApplicable => {
                        if compiler
                            .try_emit_static_simple_generator_binding_effect(effect, &prior_effects)?
                        {
                            prior_effects.push(effect.clone());
                            continue;
                        }
                        if compiler
                            .try_emit_static_simple_generator_call_effect(effect, &prior_effects)?
                        {
                            prior_effects.push(effect.clone());
                            continue;
                        }
                        if compiler.try_emit_static_simple_generator_member_assignment_effect(
                            effect,
                            &prior_effects,
                        )? {
                            prior_effects.push(effect.clone());
                            continue;
                        }
                        compiler.sync_visible_runtime_bindings_for_statements(
                            std::slice::from_ref(effect),
                        )?;
                        compiler.emit_statement(effect)?;
                    }
                }
                prior_effects.push(effect.clone());
            }
            Ok(())
        })
    }

    fn simple_generator_next_effect_call<'b>(
        statement: &'b Statement,
    ) -> Option<(&'b Expression, &'b [CallArgument])> {
        let expression = match statement {
            Statement::Let { value, .. }
            | Statement::Var { value, .. }
            | Statement::Assign { value, .. }
            | Statement::Expression(value) => value,
            _ => return None,
        };
        let Expression::Call { callee, arguments } = expression else {
            return None;
        };
        let Expression::Member { object, property } = callee.as_ref() else {
            return None;
        };
        if !matches!(property.as_ref(), Expression::String(name) if name == "next") {
            return None;
        }
        Some((object.as_ref(), arguments))
    }

    fn get_iterator_effect_source(statement: &Statement) -> Option<&Expression> {
        let expression = match statement {
            Statement::Let { value, .. }
            | Statement::Var { value, .. }
            | Statement::Assign { value, .. }
            | Statement::Expression(value) => value,
            _ => return None,
        };
        let Expression::GetIterator(source) = expression else {
            return None;
        };
        Some(source.as_ref())
    }

    fn iterator_close_effect_source(statement: &Statement) -> Option<&Expression> {
        let expression = match statement {
            Statement::Expression(value) => value,
            _ => return None,
        };
        let Expression::IteratorClose(source) = expression else {
            return None;
        };
        Some(source.as_ref())
    }

    fn resolve_static_iterator_step_member_value_expression(
        &self,
        expression: &Expression,
    ) -> Option<Expression> {
        let Expression::Member { object, property } = expression else {
            return None;
        };
        if !matches!(property.as_ref(), Expression::String(name) if name == "value") {
            return None;
        }
        let IteratorStepBinding::Runtime {
            static_value,
            value_candidates,
            ..
        } = self.resolve_iterator_step_binding_from_expression(object)?;
        if let Some(value) = static_value {
            return Some(self.materialize_static_expression(&value));
        }
        let [candidate] = value_candidates.as_slice() else {
            return None;
        };
        Some(self.materialize_static_expression(candidate))
    }

    pub(in crate::backend::direct_wasm) fn resolve_static_get_iterator_throw_value(
        &self,
        source: &Expression,
        prior_effects: &[Statement],
    ) -> Option<StaticThrowValue> {
        let mut iterator_target = self
            .simple_generator_effect_expression(source, prior_effects)
            .or_else(|| match source {
                Expression::Identifier(name) => self.static_array_binding_expression(name),
                _ => None,
            })
            .or_else(|| self.resolve_static_iterator_step_member_value_expression(source))
            .unwrap_or_else(|| self.materialize_static_expression(source));
        if let Some(outcome) = self.resolve_static_await_resolution_outcome(&iterator_target) {
            match outcome {
                StaticEvalOutcome::Throw(throw_value) => return Some(throw_value),
                StaticEvalOutcome::Value(value) => iterator_target = value,
            }
        }
        let materialized = self.materialize_static_expression(&iterator_target);
        if !static_expression_matches(&materialized, &iterator_target) {
            iterator_target = materialized;
            if let Some(outcome) = self.resolve_static_await_resolution_outcome(&iterator_target) {
                match outcome {
                    StaticEvalOutcome::Throw(throw_value) => return Some(throw_value),
                    StaticEvalOutcome::Value(value) => iterator_target = value,
                }
            }
        }
        if self.array_prototype_symbol_iterator_deleted_affects(&iterator_target) {
            return Some(StaticThrowValue::NamedError("TypeError"));
        }
        if self
            .resolve_iterator_source_kind(&iterator_target)
            .is_some()
        {
            return None;
        }
        if matches!(
            self.infer_value_kind(&iterator_target),
            Some(
                StaticValueKind::Undefined
                    | StaticValueKind::Null
                    | StaticValueKind::Bool
                    | StaticValueKind::Number
                    | StaticValueKind::BigInt
                    | StaticValueKind::Symbol
            )
        ) {
            return Some(StaticThrowValue::NamedError("TypeError"));
        }

        let iterator_property = self.materialize_static_expression(&symbol_iterator_expression());
        let current_function_name = self.current_function_name();
        if let Some(getter_binding) =
            self.resolve_member_getter_binding(&iterator_target, &iterator_property)
        {
            match self.resolve_static_function_outcome_from_binding_with_context(
                &getter_binding,
                &[],
                current_function_name,
            )? {
                StaticEvalOutcome::Throw(throw_value) => return Some(throw_value),
                StaticEvalOutcome::Value(method_value) => {
                    return self.resolve_static_iterator_method_throw_value(&method_value);
                }
            }
        } else if let Some(function_binding) =
            self.resolve_member_function_binding(&iterator_target, &iterator_property)
        {
            return match self.resolve_static_function_outcome_from_binding_with_context(
                &function_binding,
                &[],
                current_function_name,
            )? {
                StaticEvalOutcome::Throw(throw_value) => Some(throw_value),
                StaticEvalOutcome::Value(_) => None,
            };
        } else if let Some(object_binding) =
            self.resolve_object_binding_from_expression(&iterator_target)
        {
            let Some(method_value) =
                object_binding_lookup_value(&object_binding, &iterator_property).cloned()
            else {
                return Some(StaticThrowValue::NamedError("TypeError"));
            };
            return self.resolve_static_iterator_method_throw_value(&method_value);
        } else {
            return None;
        }
    }

    fn resolve_static_iterator_method_throw_value(
        &self,
        method_value: &Expression,
    ) -> Option<StaticThrowValue> {
        let current_function_name = self.current_function_name();
        if self
            .resolve_static_primitive_expression_with_context(method_value, current_function_name)
            .is_some()
        {
            return Some(StaticThrowValue::NamedError("TypeError"));
        }
        let Some(binding) = self.resolve_function_binding_from_expression_with_context(
            method_value,
            current_function_name,
        ) else {
            return Some(StaticThrowValue::NamedError("TypeError"));
        };
        match self.resolve_static_function_outcome_from_binding_with_context(
            &binding,
            &[],
            current_function_name,
        )? {
            StaticEvalOutcome::Throw(throw_value) => Some(throw_value),
            StaticEvalOutcome::Value(value) => match self.infer_value_kind(&value) {
                Some(StaticValueKind::Object | StaticValueKind::Function) => None,
                Some(
                    StaticValueKind::Undefined
                    | StaticValueKind::Null
                    | StaticValueKind::Bool
                    | StaticValueKind::Number
                    | StaticValueKind::String
                    | StaticValueKind::BigInt
                    | StaticValueKind::Symbol,
                ) => Some(StaticThrowValue::NamedError("TypeError")),
                Some(StaticValueKind::Unknown) | None => None,
            },
        }
    }

    fn resolve_static_iterator_method_value(
        &self,
        method_value: &Expression,
    ) -> Option<Expression> {
        let current_function_name = self.current_function_name();
        if self
            .resolve_static_primitive_expression_with_context(method_value, current_function_name)
            .is_some()
        {
            return None;
        }
        let binding = self.resolve_function_binding_from_expression_with_context(
            method_value,
            current_function_name,
        )?;
        match self.resolve_static_function_outcome_from_binding_with_context(
            &binding,
            &[],
            current_function_name,
        )? {
            StaticEvalOutcome::Throw(_) => None,
            StaticEvalOutcome::Value(value) => Some(value),
        }
    }

    pub(in crate::backend::direct_wasm) fn resolve_static_get_iterator_value(
        &self,
        source: &Expression,
        prior_effects: &[Statement],
    ) -> Option<Expression> {
        let mut iterator_target = self
            .simple_generator_effect_expression(source, prior_effects)
            .or_else(|| match source {
                Expression::Identifier(name) => self.static_array_binding_expression(name),
                _ => None,
            })
            .unwrap_or_else(|| self.materialize_static_expression(source));
        if let Some(outcome) = self.resolve_static_await_resolution_outcome(&iterator_target) {
            match outcome {
                StaticEvalOutcome::Throw(_) => return None,
                StaticEvalOutcome::Value(value) => iterator_target = value,
            }
        }
        let materialized = self.materialize_static_expression(&iterator_target);
        if !static_expression_matches(&materialized, &iterator_target) {
            iterator_target = materialized;
            if let Some(outcome) = self.resolve_static_await_resolution_outcome(&iterator_target) {
                match outcome {
                    StaticEvalOutcome::Throw(_) => return None,
                    StaticEvalOutcome::Value(value) => iterator_target = value,
                }
            }
        }
        if self
            .resolve_iterator_source_kind(&iterator_target)
            .is_some()
        {
            return Some(iterator_target);
        }
        if let Some(awaited_iterator_target) =
            self.resolve_for_await_step_value_iterator_target(&iterator_target)
            && self
                .resolve_iterator_source_kind(&awaited_iterator_target)
                .is_some()
        {
            return Some(awaited_iterator_target);
        }
        if matches!(
            self.infer_value_kind(&iterator_target),
            Some(StaticValueKind::Undefined | StaticValueKind::Null)
        ) {
            return None;
        }

        let iterator_property = self.materialize_static_expression(&symbol_iterator_expression());
        let current_function_name = self.current_function_name();
        if std::env::var_os("AYY_TRACE_SIMPLE_GENERATORS").is_some() {
            eprintln!(
                "simple_generator_static_get_iterator_value source={source:?} target={iterator_target:?} property={iterator_property:?} function_binding={:?} getter_binding={:?} object_binding={}",
                self.resolve_member_function_binding(&iterator_target, &iterator_property),
                self.resolve_member_getter_binding(&iterator_target, &iterator_property),
                self.resolve_object_binding_from_expression(&iterator_target)
                    .is_some(),
            );
        }
        if let Some(getter_binding) =
            self.resolve_member_getter_binding(&iterator_target, &iterator_property)
        {
            let StaticEvalOutcome::Value(method_value) = self
                .resolve_static_function_outcome_from_binding_with_context(
                    &getter_binding,
                    &[],
                    current_function_name,
                )?
            else {
                return None;
            };
            return self.resolve_static_iterator_method_value(&method_value);
        }
        if let Some(function_binding) =
            self.resolve_member_function_binding(&iterator_target, &iterator_property)
        {
            return match self.resolve_static_function_outcome_from_binding_with_context(
                &function_binding,
                &[],
                current_function_name,
            )? {
                StaticEvalOutcome::Throw(_) => None,
                StaticEvalOutcome::Value(value) => Some(value),
            };
        }
        if let Some(object_binding) = self.resolve_object_binding_from_expression(&iterator_target)
        {
            let method_value =
                object_binding_lookup_value(&object_binding, &iterator_property)?.clone();
            return self.resolve_static_iterator_method_value(&method_value);
        }
        None
    }

    pub(in crate::backend::direct_wasm) fn resolve_static_iterator_close_target(
        &self,
        expression: &Expression,
        prior_effects: &[Statement],
    ) -> Option<Expression> {
        if let Expression::Identifier(name) = expression {
            for (index, effect) in prior_effects.iter().enumerate().rev() {
                match effect {
                    Statement::Let {
                        name: effect_name,
                        value,
                        ..
                    }
                    | Statement::Var {
                        name: effect_name,
                        value,
                    }
                    | Statement::Assign {
                        name: effect_name,
                        value,
                    } if effect_name == name => {
                        if let Expression::GetIterator(source) = value {
                            return self.resolve_static_get_iterator_value(
                                source,
                                &prior_effects[..index],
                            );
                        }
                        let prior = &prior_effects[..index];
                        if let Some(value) = self.simple_generator_effect_expression(value, prior) {
                            return Some(value);
                        }
                        let materialized = self.materialize_static_expression(value);
                        if !static_expression_matches(&materialized, value) {
                            return Some(materialized);
                        }
                        return None;
                    }
                    _ => {}
                }
            }
        }

        self.simple_generator_effect_expression(expression, prior_effects)
            .or_else(|| {
                let materialized = self.materialize_static_expression(expression);
                (!static_expression_matches(&materialized, expression)).then_some(materialized)
            })
    }

    fn resolve_static_iterator_next_result(
        &mut self,
        expression: &Expression,
        prior_effects: &[Statement],
    ) -> DirectResult<Option<Expression>> {
        let Expression::Call { callee, arguments } = expression else {
            return Ok(None);
        };
        if !arguments.is_empty() {
            return Ok(None);
        }
        let Expression::Member { object, property } = callee.as_ref() else {
            return Ok(None);
        };
        if !matches!(property.as_ref(), Expression::String(name) if name == "next") {
            return Ok(None);
        }
        if let Expression::Identifier(iterator_name) = object.as_ref() {
            let iterator_binding_name = self
                .resolve_local_array_iterator_binding_name(iterator_name)
                .unwrap_or_else(|| iterator_name.clone());
            if let Some(iterator_binding) = self
                .state
                .speculation
                .static_semantics
                .local_array_iterator_binding(&iterator_binding_name)
                .cloned()
            {
                match iterator_binding.source {
                    IteratorSourceKind::StaticArray {
                        values, keys_only, ..
                    } => {
                        let current_index = iterator_binding.static_index.unwrap_or(0);
                        let done = current_index >= values.len();
                        let value = if done {
                            Expression::Undefined
                        } else if keys_only {
                            Expression::Number(current_index as f64)
                        } else {
                            values
                                .get(current_index)
                                .and_then(|value| value.clone())
                                .unwrap_or(Expression::Undefined)
                        };
                        let next_index = current_index.saturating_add(1);
                        self.set_static_iterator_index_for_index_local(
                            iterator_binding.index_local,
                            next_index,
                        );
                        self.push_i32_const(next_index as i32);
                        self.push_local_set(iterator_binding.index_local);
                        return Ok(Some(Expression::Object(vec![
                            ObjectEntry::Data {
                                key: Expression::String("done".to_string()),
                                value: Expression::Bool(done),
                            },
                            ObjectEntry::Data {
                                key: Expression::String("value".to_string()),
                                value,
                            },
                        ])));
                    }
                    IteratorSourceKind::SimpleGenerator {
                        steps,
                        completion_effects,
                        completion_value,
                        ..
                    } => {
                        let current_index = iterator_binding.static_index.unwrap_or(0);
                        let sent_value = Expression::Undefined;
                        let (done, value, next_index) = if let Some(step) = steps.get(current_index)
                        {
                            self.emit_static_simple_generator_iterator_effects(
                                &step.effects,
                                &sent_value,
                                self.state.speculation.execution_context.strict_mode,
                            )?;
                            match &step.outcome {
                                SimpleGeneratorStepOutcome::Yield(value) => (
                                    false,
                                    Self::substitute_sent_expression(value, &sent_value),
                                    current_index.saturating_add(1),
                                ),
                                SimpleGeneratorStepOutcome::Throw(value) => {
                                    let next_index = steps.len().saturating_add(1);
                                    self.set_static_iterator_index_for_index_local(
                                        iterator_binding.index_local,
                                        next_index,
                                    );
                                    self.push_i32_const(next_index as i32);
                                    self.push_local_set(iterator_binding.index_local);
                                    self.emit_static_throw_value(&StaticThrowValue::Value(
                                        Self::substitute_sent_expression(value, &sent_value),
                                    ))?;
                                    return Ok(Some(Expression::Object(vec![
                                        ObjectEntry::Data {
                                            key: Expression::String("done".to_string()),
                                            value: Expression::Bool(true),
                                        },
                                        ObjectEntry::Data {
                                            key: Expression::String("value".to_string()),
                                            value: Expression::Undefined,
                                        },
                                    ])));
                                }
                            }
                        } else if current_index == steps.len() {
                            self.emit_static_simple_generator_iterator_effects(
                                &completion_effects,
                                &sent_value,
                                self.state.speculation.execution_context.strict_mode,
                            )?;
                            (
                                true,
                                Self::substitute_sent_expression(&completion_value, &sent_value),
                                steps.len().saturating_add(1),
                            )
                        } else {
                            (true, Expression::Undefined, steps.len().saturating_add(1))
                        };
                        self.set_static_iterator_index_for_index_local(
                            iterator_binding.index_local,
                            next_index,
                        );
                        self.push_i32_const(next_index as i32);
                        self.push_local_set(iterator_binding.index_local);
                        return Ok(Some(Expression::Object(vec![
                            ObjectEntry::Data {
                                key: Expression::String("done".to_string()),
                                value: Expression::Bool(done),
                            },
                            ObjectEntry::Data {
                                key: Expression::String("value".to_string()),
                                value,
                            },
                        ])));
                    }
                    _ => {}
                }
            }
        }
        let iterator_target = self
            .resolve_static_iterator_close_target(object, prior_effects)
            .unwrap_or_else(|| object.as_ref().clone());
        let Some(LocalFunctionBinding::User(function_name)) =
            self.resolve_member_function_binding(&iterator_target, property)
        else {
            return Ok(None);
        };
        let Some(function) = self.resolve_registered_function_declaration(&function_name) else {
            return Ok(None);
        };
        let body = function.body.clone();
        for statement in &body {
            match statement {
                Statement::Return(value) => {
                    let value = self
                        .resolve_static_simple_generator_effect_expression(value)
                        .unwrap_or_else(|| self.materialize_static_expression(value));
                    return Ok(Some(value));
                }
                Statement::Throw(_) => return Ok(None),
                _ => {
                    self.sync_visible_runtime_bindings_for_statements(std::slice::from_ref(
                        statement,
                    ))?;
                    self.emit_statement(statement)?;
                }
            }
        }
        Ok(None)
    }

    fn consume_static_iterator_next_throw_value(
        &mut self,
        expression: &Expression,
        prior_effects: &[Statement],
    ) -> DirectResult<Option<StaticThrowValue>> {
        let Expression::Call { callee, arguments } = expression else {
            return Ok(None);
        };
        if !arguments.is_empty() {
            return Ok(None);
        }
        let Expression::Member { object, property } = callee.as_ref() else {
            return Ok(None);
        };
        if !matches!(property.as_ref(), Expression::String(name) if name == "next") {
            return Ok(None);
        }
        let iterator_target = self
            .resolve_static_iterator_close_target(object, prior_effects)
            .unwrap_or_else(|| object.as_ref().clone());
        let Some(LocalFunctionBinding::User(function_name)) =
            self.resolve_member_function_binding(&iterator_target, property)
        else {
            return Ok(None);
        };
        let Some(function) = self.resolve_registered_function_declaration(&function_name) else {
            return Ok(None);
        };
        let body = function.body.clone();
        let mut prefix_statements = Vec::new();
        for statement in &body {
            match statement {
                Statement::Throw(value) => {
                    for prefix_statement in &prefix_statements {
                        self.sync_visible_runtime_bindings_for_statements(std::slice::from_ref(
                            prefix_statement,
                        ))?;
                        self.emit_statement(prefix_statement)?;
                    }
                    return Ok(Some(StaticThrowValue::Value(value.clone())));
                }
                Statement::Return(_) => return Ok(None),
                _ => prefix_statements.push(statement.clone()),
            }
        }
        Ok(None)
    }

    fn statement_has_throwing_simple_generator_next_effect(
        &self,
        statement: &Statement,
    ) -> DirectResult<bool> {
        if let Statement::Block { body } = statement {
            for effect in body {
                if self.statement_has_throwing_simple_generator_next_effect(effect)? {
                    return Ok(true);
                }
            }
            return Ok(false);
        }

        let Some((object, _)) = Self::simple_generator_next_effect_call(statement) else {
            return Ok(false);
        };
        let Some((_, steps, _, _)) = self.simple_generator_source_metadata(object) else {
            return Ok(false);
        };
        let binding_name = if let Expression::Identifier(name) = object {
            self.resolve_local_array_iterator_binding_name(name)
                .unwrap_or_else(|| name.clone())
        } else {
            return Ok(false);
        };
        let current_index = self
            .state
            .speculation
            .static_semantics
            .local_array_iterator_binding(&binding_name)
            .and_then(|binding| binding.static_index)
            .unwrap_or(0);
        Ok(matches!(
            steps.get(current_index).map(|step| &step.outcome),
            Some(SimpleGeneratorStepOutcome::Throw(_))
        ))
    }

    fn consume_throwing_simple_generator_next_effect(
        &mut self,
        statement: &Statement,
    ) -> DirectResult<SimpleGeneratorNextEffectConsumption> {
        self.consume_throwing_simple_generator_next_effect_with_prior(
            statement,
            &[],
            self.state.speculation.execution_context.strict_mode,
        )
    }

    fn resolve_static_effect_member_value(&self, expression: &Expression) -> Option<Expression> {
        let Expression::Member { object, property } = expression else {
            return None;
        };
        let materialized_property = self.materialize_static_expression(property);
        if let Some(value) =
            self.static_array_member_expression(object.as_ref(), &materialized_property)
        {
            return Some(value);
        }
        if !self.expression_depends_on_active_loop_assignment(object)
            && let Some(step_binding) = self.resolve_iterator_step_binding_from_expression(object)
            && let Expression::String(property_name) = &materialized_property
        {
            match (property_name.as_str(), step_binding) {
                (
                    "done",
                    IteratorStepBinding::Runtime {
                        static_done: Some(done),
                        ..
                    },
                ) => return Some(Expression::Bool(done)),
                (
                    "value",
                    IteratorStepBinding::Runtime {
                        static_value: Some(value),
                        ..
                    },
                ) => return Some(self.materialize_static_expression(&value)),
                _ => {}
            }
        }
        if let Some(object_binding) = self.resolve_object_binding_from_expression(object) {
            if let Some(value) =
                object_binding_lookup_value(&object_binding, &materialized_property)
            {
                return Some(self.materialize_static_expression(value));
            }
            if matches!(
                materialized_property,
                Expression::String(_) | Expression::Number(_)
            ) {
                return Some(Expression::Undefined);
            }
            return None;
        }
        let Expression::Identifier(object_name) = object.as_ref() else {
            return None;
        };
        let resolved_object_name = self
            .resolve_current_local_binding(object_name)
            .map(|(resolved_name, _)| resolved_name)
            .unwrap_or_else(|| object_name.clone());
        let stored_value = self
            .state
            .speculation
            .static_semantics
            .local_value_binding(&resolved_object_name)
            .cloned()?;
        let Expression::Call { callee, arguments } = stored_value else {
            return None;
        };
        let Expression::Member {
            object: call_object,
            property: call_property,
        } = callee.as_ref()
        else {
            return None;
        };
        let binding = self.resolve_member_function_binding(call_object, call_property)?;
        let capture_bindings = self
            .resolve_member_function_capture_slots(call_object, call_property)
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
            })
            .unwrap_or_default();
        let call_arguments = arguments
            .iter()
            .map(|argument| match argument {
                CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                    self.materialize_static_expression(expression)
                }
            })
            .collect::<Vec<_>>();
        let value = if let LocalFunctionBinding::User(function_name) = &binding {
            self.resolve_static_return_expression_from_user_function_call(
                function_name,
                &arguments,
                Some(&capture_bindings),
            )
        } else {
            None
        }
        .or_else(|| {
            let (outcome, _) = self
                .resolve_bound_snapshot_function_outcome_with_arguments_and_this(
                    &binding,
                    &capture_bindings,
                    &call_arguments,
                    call_object,
                )?;
            let StaticEvalOutcome::Value(value) = outcome else {
                return None;
            };
            Some(value)
        })?;
        let object_binding = self.resolve_object_binding_from_expression(&value)?;
        if let Some(value) = object_binding_lookup_value(&object_binding, &materialized_property) {
            return Some(self.materialize_static_expression(value));
        }
        if matches!(
            materialized_property,
            Expression::String(_) | Expression::Number(_)
        ) {
            return Some(Expression::Undefined);
        }
        None
    }

    fn static_effect_equal_values(left: &Expression, right: &Expression) -> Option<bool> {
        match (left, right) {
            (Expression::Undefined, Expression::Undefined)
            | (Expression::Null, Expression::Null) => Some(true),
            (Expression::Bool(left), Expression::Bool(right)) => Some(left == right),
            (Expression::Number(left), Expression::Number(right)) => Some(left == right),
            (Expression::String(left), Expression::String(right)) => Some(left == right),
            _ if static_expression_matches(left, right) => Some(true),
            _ => None,
        }
    }

    pub(in crate::backend::direct_wasm) fn static_array_binding_expression(
        &self,
        name: &str,
    ) -> Option<Expression> {
        let binding = self
            .state
            .speculation
            .static_semantics
            .local_array_binding(name)
            .or_else(|| {
                self.backend
                    .global_semantics
                    .values
                    .array_bindings
                    .get(name)
            })?;
        Some(Expression::Array(
            binding
                .values
                .iter()
                .map(|value| {
                    ArrayElement::Expression(value.clone().unwrap_or(Expression::Undefined))
                })
                .collect(),
        ))
    }

    fn static_array_member_expression(
        &self,
        object: &Expression,
        property: &Expression,
    ) -> Option<Expression> {
        let Expression::Identifier(name) = object else {
            return None;
        };
        let Expression::Array(elements) = self.static_array_binding_expression(name)? else {
            return None;
        };
        if matches!(property, Expression::String(property_name) if property_name == "length") {
            return Some(Expression::Number(elements.len() as f64));
        }
        let index = match property {
            Expression::Number(index) if *index >= 0.0 && index.fract() == 0.0 => {
                Some(*index as usize)
            }
            Expression::String(property_name) => property_name.parse::<usize>().ok(),
            _ => None,
        };
        Some(
            index
                .and_then(|index| elements.get(index))
                .map(|element| match element {
                    ArrayElement::Expression(value) => value.clone(),
                    ArrayElement::Spread(_) => Expression::Undefined,
                })
                .unwrap_or(Expression::Undefined),
        )
    }

    fn should_preserve_static_binding_identifier_reference(
        &self,
        value: &Expression,
        resolved_value: &Expression,
    ) -> bool {
        let Expression::Identifier(name) = value else {
            return false;
        };
        if self
            .resolve_local_array_iterator_binding_name(name)
            .is_some()
        {
            return true;
        }
        if !matches!(resolved_value, Expression::Object(_)) {
            return false;
        }
        let identifier = Expression::Identifier(name.clone());
        let iterator_property = self.materialize_static_expression(&symbol_iterator_expression());
        self.resolve_member_function_binding(&identifier, &iterator_property)
            .is_some()
            || self
                .resolve_member_getter_binding(&identifier, &iterator_property)
                .is_some()
    }

    fn resolve_static_simple_generator_effect_expression(
        &self,
        expression: &Expression,
    ) -> Option<Expression> {
        match expression {
            Expression::Undefined
            | Expression::Null
            | Expression::Bool(_)
            | Expression::Number(_)
            | Expression::String(_)
            | Expression::BigInt(_) => Some(expression.clone()),
            Expression::Identifier(name) => {
                let resolved_name = self
                    .resolve_current_local_binding(name)
                    .map(|(resolved_name, _)| resolved_name)
                    .unwrap_or_else(|| name.clone());
                if let Some(value) = self
                    .static_array_binding_expression(&resolved_name)
                    .or_else(|| self.static_array_binding_expression(name))
                {
                    return Some(value);
                }
                self.state
                    .speculation
                    .static_semantics
                    .local_value_binding(&resolved_name)
                    .cloned()
                    .or_else(|| self.global_value_binding(name).cloned())
                    .and_then(|value| {
                        if static_expression_matches(&value, expression) {
                            return None;
                        }
                        let value = Self::cached_simple_generator_binding_value(&value);
                        self.resolve_static_simple_generator_effect_expression(&value)
                            .or_else(|| Some(self.materialize_static_expression(&value)))
                    })
                    .or_else(|| {
                        self.resolve_function_binding_from_expression(expression)
                            .is_some()
                            .then(|| expression.clone())
                    })
            }
            Expression::Member { .. } => self.resolve_static_effect_member_value(expression),
            Expression::Binary { op, left, right } => {
                let left = self
                    .resolve_static_simple_generator_effect_expression(left)
                    .unwrap_or_else(|| self.materialize_static_expression(left));
                let right = self
                    .resolve_static_simple_generator_effect_expression(right)
                    .unwrap_or_else(|| self.materialize_static_expression(right));
                match op {
                    BinaryOp::Add => match (&left, &right) {
                        (Expression::Number(left), Expression::Number(right)) => {
                            Some(Expression::Number(left + right))
                        }
                        _ => None,
                    },
                    BinaryOp::Subtract => match (&left, &right) {
                        (Expression::Number(left), Expression::Number(right)) => {
                            Some(Expression::Number(left - right))
                        }
                        _ => None,
                    },
                    BinaryOp::Multiply => match (&left, &right) {
                        (Expression::Number(left), Expression::Number(right)) => {
                            Some(Expression::Number(left * right))
                        }
                        _ => None,
                    },
                    BinaryOp::Divide => match (&left, &right) {
                        (Expression::Number(left), Expression::Number(right)) => {
                            Some(Expression::Number(left / right))
                        }
                        _ => None,
                    },
                    BinaryOp::Equal => {
                        Self::static_effect_equal_values(&left, &right).map(Expression::Bool)
                    }
                    BinaryOp::NotEqual => Self::static_effect_equal_values(&left, &right)
                        .map(|equal| Expression::Bool(!equal)),
                    BinaryOp::LessThan => match (&left, &right) {
                        (Expression::Number(left), Expression::Number(right)) => {
                            Some(Expression::Bool(left < right))
                        }
                        _ => None,
                    },
                    BinaryOp::LessThanOrEqual => match (&left, &right) {
                        (Expression::Number(left), Expression::Number(right)) => {
                            Some(Expression::Bool(left <= right))
                        }
                        _ => None,
                    },
                    BinaryOp::GreaterThan => match (&left, &right) {
                        (Expression::Number(left), Expression::Number(right)) => {
                            Some(Expression::Bool(left > right))
                        }
                        _ => None,
                    },
                    BinaryOp::GreaterThanOrEqual => match (&left, &right) {
                        (Expression::Number(left), Expression::Number(right)) => {
                            Some(Expression::Bool(left >= right))
                        }
                        _ => None,
                    },
                    _ => None,
                }
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                if matches!(else_expression.as_ref(), Expression::Undefined)
                    && matches!(
                        condition.as_ref(),
                        Expression::Binary { op: BinaryOp::Equal, left, right }
                            if matches!(left.as_ref(), Expression::Member { object, property }
                                if matches!(object.as_ref(), Expression::Identifier(name) if Self::is_static_iterator_step_binding_name(name))
                                    && matches!(property.as_ref(), Expression::String(name) if name == "done"))
                                && matches!(right.as_ref(), Expression::Bool(false))
                    )
                    && matches!(
                        then_expression.as_ref(),
                        Expression::Member { object, property }
                            if matches!(object.as_ref(), Expression::Identifier(name) if Self::is_static_iterator_step_binding_name(name))
                                && matches!(property.as_ref(), Expression::String(name) if name == "value")
                    )
                {
                    return self
                        .resolve_static_simple_generator_effect_expression(then_expression)
                        .or(Some(Expression::Undefined));
                }
                let condition = self
                    .resolve_static_simple_generator_effect_expression(condition)
                    .unwrap_or_else(|| self.materialize_static_expression(condition));
                match condition {
                    Expression::Bool(true) => {
                        self.resolve_static_simple_generator_effect_expression(then_expression)
                            .or_else(|| {
                                matches!(
                                    then_expression.as_ref(),
                                    Expression::Member { object, property }
                                        if matches!(object.as_ref(), Expression::Identifier(name) if Self::is_static_iterator_step_binding_name(name))
                                            && matches!(property.as_ref(), Expression::String(name) if name == "value")
                                )
                                .then_some(Expression::Undefined)
                            })
                    }
                    Expression::Bool(false) => {
                        self.resolve_static_simple_generator_effect_expression(else_expression)
                    }
                    _ => None,
                }
            }
            Expression::Array(elements) => Some(Expression::Array(
                elements
                    .iter()
                    .map(|element| match element {
                        ArrayElement::Expression(value) => Some(ArrayElement::Expression(
                            self.resolve_static_simple_generator_effect_expression(value)
                                .unwrap_or_else(|| self.materialize_static_expression(value)),
                        )),
                        ArrayElement::Spread(_) => None,
                    })
                    .collect::<Option<Vec<_>>>()?,
            )),
            Expression::Object(entries) => Some(Expression::Object(
                entries
                    .iter()
                    .map(|entry| match entry {
                        ObjectEntry::Data { key, value } => Some(ObjectEntry::Data {
                            key: self.materialize_static_expression(key),
                            value: self
                                .resolve_static_simple_generator_effect_expression(value)
                                .unwrap_or_else(|| self.materialize_static_expression(value)),
                        }),
                        _ => None,
                    })
                    .collect::<Option<Vec<_>>>()?,
            )),
            _ => None,
        }
    }

    pub(in crate::backend::direct_wasm) fn try_emit_static_simple_generator_binding_effect(
        &mut self,
        statement: &Statement,
        prior_effects: &[Statement],
    ) -> DirectResult<bool> {
        match self.consume_throwing_simple_generator_next_effect_with_prior(
            statement,
            prior_effects,
            self.state.speculation.execution_context.strict_mode,
        )? {
            SimpleGeneratorNextEffectConsumption::Threw(throw_value) => {
                self.emit_static_throw_value(&throw_value)?;
                return Ok(true);
            }
            SimpleGeneratorNextEffectConsumption::EmittedNoThrow => return Ok(true),
            SimpleGeneratorNextEffectConsumption::NotApplicable => {}
        }

        let resolved_value = match statement {
            Statement::Let { name, value, .. }
            | Statement::Var { name, value }
            | Statement::Assign { name, value } => self
                .resolve_static_iterator_next_result(value, prior_effects)?
                .or_else(|| match value {
                    Expression::GetIterator(source) => self
                        .resolve_static_get_iterator_value(source, prior_effects)
                        .map(|resolved_source| {
                            if self
                                .resolve_iterator_source_kind(&resolved_source)
                                .is_some()
                            {
                                Expression::GetIterator(Box::new(resolved_source))
                            } else {
                                resolved_source
                            }
                        }),
                    _ => None,
                })
                .or_else(|| {
                    let resolved = self.simple_generator_effect_expression(value, prior_effects)?;
                    if matches!(
                        (&resolved, value),
                        (Expression::Call { .. }, Expression::Call { .. })
                    ) {
                        return None;
                    }
                    (!static_expression_matches(&resolved, value)).then_some(resolved)
                })
                .or_else(|| {
                    let mut effects = prior_effects.to_vec();
                    effects.push(statement.clone());
                    let identifier = Expression::Identifier(name.clone());
                    let resolved =
                        self.simple_generator_effect_expression(&identifier, &effects)?;
                    if matches!(
                        (&resolved, value),
                        (Expression::Call { .. }, Expression::Call { .. })
                    ) {
                        return None;
                    }
                    (!static_expression_matches(&resolved, &identifier)).then_some(resolved)
                })
                .or_else(|| match value {
                    Expression::Identifier(value_name) => {
                        self.static_array_binding_expression(value_name)
                    }
                    _ => None,
                })
                .or_else(|| self.resolve_static_simple_generator_effect_expression(value)),
            _ => None,
        };
        let Some(resolved_value) = resolved_value else {
            return Ok(false);
        };
        if std::env::var_os("AYY_TRACE_SIMPLE_GENERATOR_ASSIGNMENT").is_some() {
            if let Statement::Let { name, value, .. }
            | Statement::Var { name, value }
            | Statement::Assign { name, value } = statement
            {
                eprintln!(
                    "simple_generator_binding_effect name={name} value={value:?} resolved={resolved_value:?}"
                );
            }
        }
        if matches!(
            statement,
            Statement::Let { value, .. }
                | Statement::Var { value, .. }
                | Statement::Assign { value, .. }
                if self.should_preserve_static_binding_identifier_reference(value, &resolved_value)
        ) {
            return Ok(false);
        }
        match statement {
            Statement::Let { name, mutable, .. } => {
                self.emit_statement(&Statement::Let {
                    name: name.clone(),
                    mutable: *mutable,
                    value: resolved_value,
                })?;
                Ok(true)
            }
            Statement::Var { name, .. } => {
                self.emit_statement(&Statement::Var {
                    name: name.clone(),
                    value: resolved_value,
                })?;
                Ok(true)
            }
            Statement::Assign { name, .. } => {
                self.emit_statement(&Statement::Assign {
                    name: name.clone(),
                    value: resolved_value,
                })?;
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    pub(in crate::backend::direct_wasm) fn try_emit_static_simple_generator_member_assignment_effect(
        &mut self,
        statement: &Statement,
        prior_effects: &[Statement],
    ) -> DirectResult<bool> {
        let Statement::AssignMember {
            object,
            property,
            value,
        } = statement
        else {
            return Ok(false);
        };

        let resolved_property = self
            .simple_generator_effect_expression(property, prior_effects)
            .or_else(|| self.resolve_static_simple_generator_effect_expression(property))
            .or_else(|| self.resolve_property_key_expression(property));
        let resolved_value = if matches!(value, Expression::Identifier(_))
            && self
                .runtime_array_binding_name_for_expression(value)
                .is_some()
        {
            None
        } else {
            self.simple_generator_effect_expression(value, prior_effects)
                .or_else(|| self.resolve_static_simple_generator_effect_expression(value))
        };
        if std::env::var_os("AYY_TRACE_SIMPLE_GENERATOR_ASSIGNMENT").is_some() {
            eprintln!(
                "simple_generator_member_assignment object={object:?} property={property:?} resolved_property={resolved_property:?} value={value:?} resolved_value={resolved_value:?}"
            );
        }
        if resolved_property.is_none() && resolved_value.is_none() {
            return Ok(false);
        }

        let resolved_statement = Statement::AssignMember {
            object: object.clone(),
            property: resolved_property.unwrap_or_else(|| property.clone()),
            value: resolved_value.unwrap_or_else(|| value.clone()),
        };
        self.sync_visible_runtime_bindings_for_statements(std::slice::from_ref(
            &resolved_statement,
        ))?;
        self.emit_statement(&resolved_statement)?;
        Ok(true)
    }

    pub(in crate::backend::direct_wasm) fn try_emit_static_simple_generator_call_effect(
        &mut self,
        statement: &Statement,
        prior_effects: &[Statement],
    ) -> DirectResult<bool> {
        let Statement::Expression(Expression::Call { callee, arguments }) = statement else {
            return Ok(false);
        };
        let Expression::Member { object, property } = callee.as_ref() else {
            return Ok(false);
        };
        if !matches!(property.as_ref(), Expression::String(name) if name == "push") {
            return Ok(false);
        }
        let resolved_arguments = arguments
            .iter()
            .map(|argument| {
                let CallArgument::Expression(expression) = argument else {
                    return None;
                };
                self.simple_generator_effect_expression(expression, prior_effects)
                    .or_else(|| self.resolve_static_simple_generator_effect_expression(expression))
                    .map(CallArgument::Expression)
            })
            .collect::<Option<Vec<_>>>();
        let Some(resolved_arguments) = resolved_arguments else {
            return Ok(false);
        };
        if self.emit_tracked_array_push_call(object, &resolved_arguments)? {
            self.state.emission.output.instructions.push(0x1a);
            return Ok(true);
        }
        Ok(false)
    }

    fn resolve_static_simple_generator_prior_binding_value(
        &self,
        name: &str,
        prior_effects: &[Statement],
    ) -> Option<Expression> {
        for (index, effect) in prior_effects.iter().enumerate().rev() {
            match effect {
                Statement::Let {
                    name: effect_name,
                    value,
                    ..
                }
                | Statement::Var {
                    name: effect_name,
                    value,
                }
                | Statement::Assign {
                    name: effect_name,
                    value,
                } if effect_name == name => {
                    if let Some(value) =
                        self.simple_generator_effect_expression(value, &prior_effects[..index])
                    {
                        return Some(value);
                    }
                    if let Some(value) =
                        self.resolve_static_simple_generator_effect_expression(value)
                    {
                        return Some(value);
                    }
                }
                _ => {}
            }
        }
        None
    }

    fn resolve_static_simple_generator_condition_with_prior(
        &self,
        condition: &Expression,
        prior_effects: &[Statement],
    ) -> Option<Expression> {
        let Expression::Binary { op, left, right } = condition else {
            return None;
        };
        if *op != BinaryOp::Equal {
            return None;
        }
        let Expression::Identifier(name) = left.as_ref() else {
            return None;
        };
        let left = self.resolve_static_simple_generator_prior_binding_value(name, prior_effects)?;
        let right = self
            .resolve_static_simple_generator_effect_expression(right)
            .unwrap_or_else(|| self.materialize_static_expression(right));
        let equal = Self::static_effect_equal_values(&left, &right);
        equal.map(Expression::Bool)
    }

    fn done_false_condition_identifier(condition: &Expression) -> Option<&str> {
        let Expression::Binary { op, left, right } = condition else {
            return None;
        };
        if *op != BinaryOp::Equal || !matches!(right.as_ref(), Expression::Bool(false)) {
            return None;
        }
        let Expression::Identifier(name) = left.as_ref() else {
            return None;
        };
        Some(name)
    }

    fn member_matches_identifier_property(
        expression: &Expression,
        object_name: &str,
        property_name: &str,
    ) -> bool {
        matches!(
            expression,
            Expression::Member { object, property }
                if matches!(object.as_ref(), Expression::Identifier(name) if name == object_name)
                    && matches!(property.as_ref(), Expression::String(name) if name == property_name)
        )
    }

    fn is_static_rest_push_for_step(statement: &Statement, step_name: &str) -> bool {
        let Statement::Expression(Expression::Call { callee, arguments }) = statement else {
            return false;
        };
        let Expression::Member { object, property } = callee.as_ref() else {
            return false;
        };
        if !matches!(object.as_ref(), Expression::Identifier(_))
            || !matches!(property.as_ref(), Expression::String(name) if name == "push")
            || arguments.len() != 1
        {
            return false;
        }
        matches!(
            arguments.first(),
            Some(CallArgument::Expression(argument))
                if Self::member_matches_identifier_property(argument, step_name, "value")
        )
    }

    fn is_static_rest_collection_loop_shape(statement: &Statement) -> bool {
        let Statement::While {
            condition,
            break_hook: None,
            body,
            ..
        } = statement
        else {
            return false;
        };
        let Some(done_name) = Self::done_false_condition_identifier(condition) else {
            return false;
        };
        let [next_statement, done_statement, push_statement] = body.as_slice() else {
            return false;
        };
        let Statement::Let {
            name: step_name,
            value: Expression::Call { callee, arguments },
            ..
        } = next_statement
        else {
            return false;
        };
        let Expression::Member { object, property } = callee.as_ref() else {
            return false;
        };
        if !arguments.is_empty()
            || !matches!(object.as_ref(), Expression::Identifier(_))
            || !matches!(property.as_ref(), Expression::String(name) if name == "next")
        {
            return false;
        }
        if !matches!(
            done_statement,
            Statement::Assign { name, value }
                if name == done_name
                    && Self::member_matches_identifier_property(value, step_name, "done")
        ) {
            return false;
        }
        let Statement::If {
            condition,
            then_branch,
            else_branch,
        } = push_statement
        else {
            return false;
        };
        if !else_branch.is_empty()
            || Self::done_false_condition_identifier(condition) != Some(done_name)
            || then_branch.len() != 1
        {
            return false;
        }
        Self::is_static_rest_push_for_step(&then_branch[0], step_name)
    }

    fn static_rest_collection_loop_iterator_name(statement: &Statement) -> Option<&str> {
        if !Self::is_static_rest_collection_loop_shape(statement) {
            return None;
        }
        let Statement::While { body, .. } = statement else {
            return None;
        };
        let Some(Statement::Let {
            value: Expression::Call { callee, .. },
            ..
        }) = body.first()
        else {
            return None;
        };
        let Expression::Member { object, .. } = callee.as_ref() else {
            return None;
        };
        let Expression::Identifier(iterator_name) = object.as_ref() else {
            return None;
        };
        Some(iterator_name)
    }

    fn prior_static_rest_collection_exhausted_iterator(
        expression: &Expression,
        prior_effects: &[Statement],
    ) -> bool {
        let Expression::Identifier(iterator_name) = expression else {
            return false;
        };
        prior_effects.iter().rev().any(|effect| {
            Self::static_rest_collection_loop_iterator_name(effect) == Some(iterator_name.as_str())
        })
    }

    fn consume_static_simple_generator_rest_collection_loop(
        &mut self,
        statement: &Statement,
        prior_effects: &[Statement],
        strict_mode: bool,
    ) -> DirectResult<Option<SimpleGeneratorNextEffectConsumption>> {
        if !Self::is_static_rest_collection_loop_shape(statement) {
            return Ok(None);
        }
        let Statement::While {
            condition, body, ..
        } = statement
        else {
            return Ok(None);
        };
        let mut loop_prior_effects = prior_effects.to_vec();
        let mut iterations = 0usize;
        loop {
            let condition = self
                .resolve_static_simple_generator_effect_expression(condition)
                .or_else(|| {
                    self.resolve_static_simple_generator_condition_with_prior(
                        condition,
                        &loop_prior_effects,
                    )
                });
            match condition {
                Some(Expression::Bool(false)) => {
                    return Ok(Some(SimpleGeneratorNextEffectConsumption::EmittedNoThrow));
                }
                Some(Expression::Bool(true)) => {}
                _ => return Ok(None),
            }
            iterations += 1;
            if iterations > 1024 {
                return Err(Unsupported(
                    "static simple generator rest collection exceeded unroll limit",
                ));
            }
            for effect in body {
                match self.consume_throwing_simple_generator_next_effect_with_prior(
                    effect,
                    &loop_prior_effects,
                    strict_mode,
                )? {
                    SimpleGeneratorNextEffectConsumption::Threw(throw_value) => {
                        return Ok(Some(SimpleGeneratorNextEffectConsumption::Threw(
                            throw_value,
                        )));
                    }
                    SimpleGeneratorNextEffectConsumption::EmittedNoThrow => {}
                    SimpleGeneratorNextEffectConsumption::NotApplicable => {
                        if self.try_emit_static_simple_generator_binding_effect(
                            effect,
                            &loop_prior_effects,
                        )? {
                            loop_prior_effects.push(effect.clone());
                            continue;
                        }
                        if self.try_emit_static_simple_generator_call_effect(
                            effect,
                            &loop_prior_effects,
                        )? {
                            loop_prior_effects.push(effect.clone());
                            continue;
                        }
                        if self.try_emit_static_simple_generator_member_assignment_effect(
                            effect,
                            &loop_prior_effects,
                        )? {
                            loop_prior_effects.push(effect.clone());
                            continue;
                        }
                        self.sync_visible_runtime_bindings_for_statements(std::slice::from_ref(
                            effect,
                        ))?;
                        self.emit_statement(effect)?;
                    }
                }
                loop_prior_effects.push(effect.clone());
            }
        }
    }

    pub(in crate::backend::direct_wasm) fn try_emit_static_simple_generator_rest_collection_loop_statement(
        &mut self,
        statement: &Statement,
    ) -> DirectResult<bool> {
        let Some(iterator_name) = Self::static_rest_collection_loop_iterator_name(statement) else {
            return Ok(false);
        };
        let iterator_binding_name = self
            .resolve_local_array_iterator_binding_name(iterator_name)
            .unwrap_or_else(|| iterator_name.to_string());
        if self
            .state
            .speculation
            .static_semantics
            .local_array_iterator_binding(&iterator_binding_name)
            .is_none()
        {
            return Ok(false);
        }
        let strict_mode = self.state.speculation.execution_context.strict_mode;
        let Some(consumption) =
            self.consume_static_simple_generator_rest_collection_loop(statement, &[], strict_mode)?
        else {
            return Ok(false);
        };

        match consumption {
            SimpleGeneratorNextEffectConsumption::Threw(throw_value) => {
                self.emit_static_throw_value(&throw_value)?;
            }
            SimpleGeneratorNextEffectConsumption::EmittedNoThrow => {}
            SimpleGeneratorNextEffectConsumption::NotApplicable => return Ok(false),
        }
        Ok(true)
    }

    fn consume_static_iterator_close_effect(
        &mut self,
        expression: &Expression,
        prior_effects: &[Statement],
    ) -> DirectResult<Option<SimpleGeneratorNextEffectConsumption>> {
        let return_property = Expression::String("return".to_string());
        let close_target = self
            .resolve_static_iterator_close_target(expression, prior_effects)
            .unwrap_or_else(|| expression.clone());
        if std::env::var_os("AYY_TRACE_SIMPLE_GENERATORS").is_some() {
            eprintln!(
                "simple_generator_static_close expression={expression:?} close_target={close_target:?}"
            );
        }
        let Some(LocalFunctionBinding::User(function_name)) =
            self.resolve_member_function_binding(&close_target, &return_property)
        else {
            if std::env::var_os("AYY_TRACE_SIMPLE_GENERATORS").is_some() {
                eprintln!(
                    "simple_generator_static_close no_return_binding close_target={close_target:?}"
                );
            }
            return Ok((!static_expression_matches(&close_target, expression)
                || matches!(close_target, Expression::Array(_) | Expression::Object(_)))
            .then_some(SimpleGeneratorNextEffectConsumption::EmittedNoThrow));
        };
        if std::env::var_os("AYY_TRACE_SIMPLE_GENERATORS").is_some() {
            eprintln!("simple_generator_static_close return_function={function_name}");
        }
        let Some(user_function) = self.user_function(&function_name).cloned() else {
            return Ok(None);
        };
        let Some(function) = self.resolve_registered_function_declaration(&function_name) else {
            return Ok(None);
        };
        let body = function.body.clone();
        let mut prefix_statements = Vec::new();
        for statement in &body {
            if let Statement::Throw(value) = statement {
                for prefix_statement in &prefix_statements {
                    self.sync_visible_runtime_bindings_for_statements(std::slice::from_ref(
                        prefix_statement,
                    ))?;
                    self.emit_statement(prefix_statement)?;
                }
                return Ok(Some(SimpleGeneratorNextEffectConsumption::Threw(
                    StaticThrowValue::Value(value.clone()),
                )));
            }
            if let Statement::Return(value) = statement {
                let return_value = self
                    .resolve_static_simple_generator_effect_expression(value)
                    .unwrap_or_else(|| self.materialize_static_expression(value));
                let return_kind = self.infer_value_kind(&return_value);
                self.emit_user_function_call_with_function_this_binding(
                    &user_function,
                    &[],
                    &close_target,
                    None,
                )?;
                self.sync_static_iterator_close_arguments_assignments(&user_function, &[], &body);
                self.state.emission.output.instructions.push(0x1a);
                if !matches!(
                    return_kind,
                    Some(StaticValueKind::Object | StaticValueKind::Function)
                ) {
                    return Ok(Some(SimpleGeneratorNextEffectConsumption::Threw(
                        StaticThrowValue::NamedError("TypeError"),
                    )));
                }
                return Ok(Some(SimpleGeneratorNextEffectConsumption::EmittedNoThrow));
            }
            prefix_statements.push(statement.clone());
        }
        self.emit_user_function_call_with_function_this_binding(
            &user_function,
            &[],
            &close_target,
            None,
        )?;
        self.sync_static_iterator_close_arguments_assignments(&user_function, &[], &body);
        self.state.emission.output.instructions.push(0x1a);
        Ok(Some(SimpleGeneratorNextEffectConsumption::Threw(
            StaticThrowValue::NamedError("TypeError"),
        )))
    }

    fn collect_static_iterator_close_arguments_assignment_targets(
        statement: &Statement,
        targets: &mut Vec<String>,
    ) {
        match statement {
            Statement::Assign {
                name,
                value: Expression::Identifier(value_name),
            }
            | Statement::Var {
                name,
                value: Expression::Identifier(value_name),
            }
            | Statement::Let {
                name,
                value: Expression::Identifier(value_name),
                ..
            } if value_name == "arguments" => {
                if !targets.contains(name) {
                    targets.push(name.clone());
                }
            }
            Statement::Expression(Expression::Assign { name, value }) if matches!(value.as_ref(), Expression::Identifier(value_name) if value_name == "arguments") => {
                if !targets.contains(name) {
                    targets.push(name.clone());
                }
            }
            Statement::Declaration { body }
            | Statement::Block { body }
            | Statement::Labeled { body, .. }
            | Statement::With { body, .. }
            | Statement::While { body, .. }
            | Statement::DoWhile { body, .. } => {
                for statement in body {
                    Self::collect_static_iterator_close_arguments_assignment_targets(
                        statement, targets,
                    );
                }
            }
            Statement::If {
                then_branch,
                else_branch,
                ..
            } => {
                for statement in then_branch.iter().chain(else_branch) {
                    Self::collect_static_iterator_close_arguments_assignment_targets(
                        statement, targets,
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
                    Self::collect_static_iterator_close_arguments_assignment_targets(
                        statement, targets,
                    );
                }
            }
            Statement::Switch { cases, .. } => {
                for case in cases {
                    for statement in &case.body {
                        Self::collect_static_iterator_close_arguments_assignment_targets(
                            statement, targets,
                        );
                    }
                }
            }
            Statement::For { init, body, .. } => {
                for statement in init.iter().chain(body) {
                    Self::collect_static_iterator_close_arguments_assignment_targets(
                        statement, targets,
                    );
                }
            }
            _ => {}
        }
    }

    pub(in crate::backend::direct_wasm) fn sync_static_iterator_close_arguments_assignments(
        &mut self,
        user_function: &UserFunction,
        arguments: &[Expression],
        body: &[Statement],
    ) {
        let mut targets = Vec::new();
        for statement in body {
            Self::collect_static_iterator_close_arguments_assignment_targets(
                statement,
                &mut targets,
            );
        }
        if targets.is_empty() {
            return;
        }
        let binding = ArgumentsValueBinding::for_user_function(user_function, arguments.to_vec());
        for target in targets {
            if user_function.scope_bindings.contains(&target) {
                continue;
            }
            if self.resolve_current_local_binding(&target).is_some()
                || self.parameter_scope_arguments_local_for(&target).is_some()
            {
                self.state
                    .parameters
                    .local_arguments_bindings
                    .insert(target.clone(), binding.clone());
                self.state
                    .speculation
                    .static_semantics
                    .set_local_kind(&target, StaticValueKind::Object);
            } else {
                self.backend
                    .sync_global_arguments_binding(&target, Some(binding.clone()));
            }
        }
    }

    fn assignment_targets_immutable_binding(&self, name: &str) -> bool {
        self.assignment_targets_immutable_class_binding(name)
            || self
                .resolve_current_local_binding(name)
                .is_some_and(|(resolved_name, _)| self.local_binding_is_immutable(&resolved_name))
            || self.user_function_capture_binding_is_immutable(name)
            || self
                .backend
                .lexical_global_binding(name)
                .is_some_and(|binding| !binding.mutable)
    }

    fn assignment_targets_strict_unresolvable_reference(
        &self,
        name: &str,
        strict_mode: bool,
    ) -> bool {
        strict_mode
            && self.parameter_scope_arguments_local_for(name).is_none()
            && self.resolve_current_local_binding(name).is_none()
            && self.backend.global_binding_index(name).is_none()
            && self
                .resolve_user_function_capture_hidden_name(name)
                .is_none()
            && self.resolve_eval_local_function_hidden_name(name).is_none()
    }

    fn try_consume_static_simple_generator_setter_member_assignment_effect(
        &mut self,
        statement: &Statement,
    ) -> DirectResult<Option<SimpleGeneratorNextEffectConsumption>> {
        let Statement::AssignMember {
            object,
            property,
            value,
        } = statement
        else {
            return Ok(None);
        };

        let resolved_property = self
            .resolve_static_simple_generator_effect_expression(property)
            .or_else(|| self.resolve_property_key_expression(property))
            .unwrap_or_else(|| property.clone());
        let resolved_value = self
            .resolve_static_simple_generator_effect_expression(value)
            .unwrap_or_else(|| self.materialize_static_expression(value));
        let Some(function_binding) = self.resolve_member_setter_binding(object, &resolved_property)
        else {
            return Ok(None);
        };

        let setter_arguments = vec![CallArgument::Expression(resolved_value.clone())];
        if let Some(StaticEvalOutcome::Throw(throw_value)) = self
            .resolve_static_function_outcome_from_binding_with_call_frame_and_context(
                &function_binding,
                &setter_arguments,
                object,
                self.current_function_name(),
            )
        {
            return Ok(Some(SimpleGeneratorNextEffectConsumption::Threw(
                throw_value,
            )));
        }

        if let LocalFunctionBinding::User(function_name) = &function_binding
            && let Some(user_function) = self.user_function(function_name).cloned()
            && let Some(function) = self.resolve_registered_function_declaration(function_name)
        {
            let call_arguments = vec![CallArgument::Expression(resolved_value.clone())];
            let arguments_values = vec![resolved_value.clone()];
            let arguments_binding =
                self.simple_generator_arguments_binding_expression(&arguments_values);
            let substituted_body = function
                .body
                .iter()
                .map(|statement| {
                    self.substitute_statement_call_frame_bindings(
                        statement,
                        &user_function,
                        &call_arguments,
                        object,
                        &arguments_binding,
                    )
                })
                .collect::<Vec<_>>();
            self.register_bindings(&substituted_body)?;
            let mut setter_prior_effects = Vec::new();
            for effect in &substituted_body {
                if matches!(effect, Statement::Return(_)) {
                    break;
                }
                match self.consume_throwing_simple_generator_next_effect_with_prior(
                    effect,
                    &setter_prior_effects,
                    user_function.strict,
                )? {
                    SimpleGeneratorNextEffectConsumption::Threw(throw_value) => {
                        return Ok(Some(SimpleGeneratorNextEffectConsumption::Threw(
                            throw_value,
                        )));
                    }
                    SimpleGeneratorNextEffectConsumption::EmittedNoThrow => {}
                    SimpleGeneratorNextEffectConsumption::NotApplicable => {
                        if self.try_emit_static_simple_generator_binding_effect(
                            effect,
                            &setter_prior_effects,
                        )? {
                            setter_prior_effects.push(effect.clone());
                            continue;
                        }
                        if self.try_emit_static_simple_generator_call_effect(
                            effect,
                            &setter_prior_effects,
                        )? {
                            setter_prior_effects.push(effect.clone());
                            continue;
                        }
                        if self.try_emit_static_simple_generator_member_assignment_effect(
                            effect,
                            &setter_prior_effects,
                        )? {
                            setter_prior_effects.push(effect.clone());
                            continue;
                        }
                        self.sync_visible_runtime_bindings_for_statements(std::slice::from_ref(
                            effect,
                        ))?;
                        self.emit_statement(effect)?;
                    }
                }
                setter_prior_effects.push(effect.clone());
            }
            return Ok(Some(SimpleGeneratorNextEffectConsumption::EmittedNoThrow));
        }

        self.emit_statement(&Statement::AssignMember {
            object: object.clone(),
            property: resolved_property,
            value: resolved_value,
        })?;
        Ok(Some(SimpleGeneratorNextEffectConsumption::EmittedNoThrow))
    }

    pub(in crate::backend::direct_wasm) fn consume_throwing_simple_generator_next_effect_with_prior(
        &mut self,
        statement: &Statement,
        prior_effects: &[Statement],
        strict_mode: bool,
    ) -> DirectResult<SimpleGeneratorNextEffectConsumption> {
        if std::env::var_os("AYY_TRACE_SIMPLE_GENERATORS").is_some() {
            eprintln!(
                "simple_generator_effect_consume strict={strict_mode} statement={statement:?}"
            );
        }
        if let Statement::Block { body } = statement {
            let mut block_prior_effects = Vec::new();
            for effect in body {
                match self.consume_throwing_simple_generator_next_effect_with_prior(
                    effect,
                    &block_prior_effects,
                    strict_mode,
                )? {
                    SimpleGeneratorNextEffectConsumption::Threw(throw_value) => {
                        return Ok(SimpleGeneratorNextEffectConsumption::Threw(throw_value));
                    }
                    SimpleGeneratorNextEffectConsumption::EmittedNoThrow => {}
                    SimpleGeneratorNextEffectConsumption::NotApplicable => {}
                }
                block_prior_effects.push(effect.clone());
            }
            return Ok(SimpleGeneratorNextEffectConsumption::NotApplicable);
        }

        if let Statement::If {
            condition,
            then_branch,
            else_branch,
        } = statement
        {
            let condition = self
                .resolve_static_simple_generator_condition_with_prior(condition, prior_effects)
                .or_else(|| self.resolve_static_simple_generator_effect_expression(condition))
                .unwrap_or_else(|| self.materialize_static_expression(condition));
            let branch = match condition {
                Expression::Bool(true) => then_branch,
                Expression::Bool(false) => else_branch,
                _ => return Ok(SimpleGeneratorNextEffectConsumption::NotApplicable),
            };
            let mut branch_prior_effects = prior_effects.to_vec();
            for effect in branch {
                match self.consume_throwing_simple_generator_next_effect_with_prior(
                    effect,
                    &branch_prior_effects,
                    strict_mode,
                )? {
                    SimpleGeneratorNextEffectConsumption::Threw(throw_value) => {
                        return Ok(SimpleGeneratorNextEffectConsumption::Threw(throw_value));
                    }
                    SimpleGeneratorNextEffectConsumption::EmittedNoThrow => {}
                    SimpleGeneratorNextEffectConsumption::NotApplicable => {
                        if self.try_emit_static_simple_generator_binding_effect(
                            effect,
                            &branch_prior_effects,
                        )? {
                            branch_prior_effects.push(effect.clone());
                            continue;
                        }
                        if self.try_emit_static_simple_generator_call_effect(
                            effect,
                            &branch_prior_effects,
                        )? {
                            branch_prior_effects.push(effect.clone());
                            continue;
                        }
                        if self.try_emit_static_simple_generator_member_assignment_effect(
                            effect,
                            &branch_prior_effects,
                        )? {
                            branch_prior_effects.push(effect.clone());
                            continue;
                        }
                        self.sync_visible_runtime_bindings_for_statements(std::slice::from_ref(
                            effect,
                        ))?;
                        self.emit_statement(effect)?;
                    }
                }
                branch_prior_effects.push(effect.clone());
            }
            return Ok(SimpleGeneratorNextEffectConsumption::EmittedNoThrow);
        }

        let assignment_target_name = match statement {
            Statement::Assign { name, .. }
            | Statement::Expression(Expression::Assign { name, .. }) => Some(name.as_str()),
            _ => None,
        };
        if let Some(name) = assignment_target_name {
            if self.assignment_targets_immutable_binding(name) {
                return Ok(SimpleGeneratorNextEffectConsumption::Threw(
                    StaticThrowValue::NamedError("TypeError"),
                ));
            }
            if self.assignment_targets_strict_unresolvable_reference(name, strict_mode) {
                return Ok(SimpleGeneratorNextEffectConsumption::Threw(
                    StaticThrowValue::NamedError("ReferenceError"),
                ));
            }
        }

        if let Some(consumption) =
            self.try_consume_static_simple_generator_setter_member_assignment_effect(statement)?
        {
            return Ok(consumption);
        }

        if let Some(source) = Self::get_iterator_effect_source(statement)
            && let Some(throw_value) =
                self.resolve_static_get_iterator_throw_value(source, prior_effects)
        {
            return Ok(SimpleGeneratorNextEffectConsumption::Threw(throw_value));
        }

        if let Some(source) = Self::iterator_close_effect_source(statement) {
            if Self::prior_static_rest_collection_exhausted_iterator(source, prior_effects) {
                return Ok(SimpleGeneratorNextEffectConsumption::EmittedNoThrow);
            }
            if let Some(consumption) =
                self.consume_static_iterator_close_effect(source, prior_effects)?
            {
                return Ok(consumption);
            }
        }

        if let Some(consumption) = self.consume_static_simple_generator_rest_collection_loop(
            statement,
            prior_effects,
            strict_mode,
        )? {
            return Ok(consumption);
        }

        let statement_expression = match statement {
            Statement::Let { value, .. }
            | Statement::Var { value, .. }
            | Statement::Assign { value, .. }
            | Statement::Expression(value) => Some(value),
            _ => None,
        };
        if let Some(expression) = statement_expression
            && let Some(throw_value) =
                self.consume_static_iterator_next_throw_value(expression, prior_effects)?
        {
            return Ok(SimpleGeneratorNextEffectConsumption::Threw(throw_value));
        }

        let Some((object, arguments)) = Self::simple_generator_next_effect_call(statement) else {
            return Ok(SimpleGeneratorNextEffectConsumption::NotApplicable);
        };
        let Some((_, steps, _, _)) = self.simple_generator_source_metadata(object) else {
            return Ok(SimpleGeneratorNextEffectConsumption::NotApplicable);
        };
        let binding_name = if let Expression::Identifier(name) = object {
            self.resolve_local_array_iterator_binding_name(name)
                .unwrap_or_else(|| name.clone())
        } else {
            return Ok(SimpleGeneratorNextEffectConsumption::NotApplicable);
        };
        let current_index = self
            .state
            .speculation
            .static_semantics
            .local_array_iterator_binding(&binding_name)
            .and_then(|binding| binding.static_index)
            .unwrap_or(0);
        let Some(step) = steps.get(current_index) else {
            return Ok(SimpleGeneratorNextEffectConsumption::NotApplicable);
        };
        let SimpleGeneratorStepOutcome::Throw(value) = &step.outcome else {
            return Ok(SimpleGeneratorNextEffectConsumption::NotApplicable);
        };

        let sent_value = arguments
            .first()
            .map(|argument| match argument {
                CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                    self.materialize_static_expression(expression)
                }
            })
            .unwrap_or(Expression::Undefined);
        let substituted_effects = step
            .effects
            .iter()
            .map(|effect| Self::substitute_sent_statement(effect, &sent_value))
            .collect::<Vec<_>>();
        for effect in &substituted_effects {
            match self.consume_throwing_simple_generator_next_effect(effect)? {
                SimpleGeneratorNextEffectConsumption::Threw(throw_value) => {
                    return Ok(SimpleGeneratorNextEffectConsumption::Threw(throw_value));
                }
                SimpleGeneratorNextEffectConsumption::EmittedNoThrow => {}
                SimpleGeneratorNextEffectConsumption::NotApplicable => {
                    self.sync_visible_runtime_bindings_for_statements(std::slice::from_ref(
                        effect,
                    ))?;
                    self.emit_statement(effect)?;
                }
            }
        }

        if let Some(index_local) = self
            .state
            .speculation
            .static_semantics
            .local_array_iterator_binding(&binding_name)
            .map(|binding| binding.index_local)
        {
            let next_index = steps.len().saturating_add(1);
            self.set_static_iterator_index_for_index_local(index_local, next_index);
            self.push_i32_const(next_index as i32);
            self.push_local_set(index_local);
        }

        Ok(SimpleGeneratorNextEffectConsumption::Threw(
            StaticThrowValue::Value(Self::substitute_sent_expression(value, &sent_value)),
        ))
    }

    pub(in crate::backend::direct_wasm) fn emit_static_lowered_pattern_inline_body(
        &mut self,
        body: &[Statement],
    ) -> DirectResult<bool> {
        self.register_bindings(body)?;
        let mut prior_effects = Vec::new();
        for statement in body {
            match self.consume_throwing_simple_generator_next_effect_with_prior(
                statement,
                &prior_effects,
                self.state.speculation.execution_context.strict_mode,
            )? {
                SimpleGeneratorNextEffectConsumption::Threw(throw_value) => {
                    self.emit_static_throw_value(&throw_value)?;
                    return Ok(true);
                }
                SimpleGeneratorNextEffectConsumption::EmittedNoThrow => {}
                SimpleGeneratorNextEffectConsumption::NotApplicable => {
                    if self.try_emit_static_simple_generator_binding_effect(
                        statement,
                        &prior_effects,
                    )? {
                        prior_effects.push(statement.clone());
                        continue;
                    }
                    if self
                        .try_emit_static_simple_generator_call_effect(statement, &prior_effects)?
                    {
                        prior_effects.push(statement.clone());
                        continue;
                    }
                    if self.try_emit_static_simple_generator_member_assignment_effect(
                        statement,
                        &prior_effects,
                    )? {
                        prior_effects.push(statement.clone());
                        continue;
                    }
                    self.emit_statement(statement)?;
                }
            }
            prior_effects.push(statement.clone());
        }
        Ok(false)
    }

    fn statement_contains_unhandled_loop_transfer(statement: &Statement) -> bool {
        match statement {
            Statement::Break { .. } | Statement::Continue { .. } => true,
            Statement::Declaration { body }
            | Statement::Block { body }
            | Statement::Labeled { body, .. }
            | Statement::With { body, .. } => body
                .iter()
                .any(Self::statement_contains_unhandled_loop_transfer),
            Statement::If {
                then_branch,
                else_branch,
                ..
            } => {
                then_branch
                    .iter()
                    .any(Self::statement_contains_unhandled_loop_transfer)
                    || else_branch
                        .iter()
                        .any(Self::statement_contains_unhandled_loop_transfer)
            }
            Statement::Try {
                body,
                catch_setup,
                catch_body,
                ..
            } => {
                body.iter()
                    .any(Self::statement_contains_unhandled_loop_transfer)
                    || catch_setup
                        .iter()
                        .any(Self::statement_contains_unhandled_loop_transfer)
                    || catch_body
                        .iter()
                        .any(Self::statement_contains_unhandled_loop_transfer)
            }
            Statement::Switch { cases, .. } => cases.iter().any(|case| {
                case.body
                    .iter()
                    .any(Self::statement_contains_unhandled_loop_transfer)
            }),
            Statement::While { .. } if Self::is_static_rest_collection_loop_shape(statement) => {
                false
            }
            Statement::For { .. } | Statement::While { .. } | Statement::DoWhile { .. } => true,
            _ => false,
        }
    }

    fn static_iterator_steps_for_unroll(
        &self,
        source_expression: &Expression,
    ) -> Option<StaticIteratorUnroll> {
        match self.resolve_iterator_source_kind(source_expression)? {
            IteratorSourceKind::StaticArray {
                values,
                keys_only,
                length_local: None,
                runtime_name: None,
            } => Some(StaticIteratorUnroll {
                steps: values
                    .into_iter()
                    .enumerate()
                    .map(|(index, value)| {
                        let value = if keys_only {
                            Expression::Number(index as f64)
                        } else {
                            value.unwrap_or(Expression::Undefined)
                        };
                        StaticIteratorUnrollStep {
                            effects: Vec::new(),
                            value,
                        }
                    })
                    .collect(),
                completion_effects: Vec::new(),
            }),
            IteratorSourceKind::SimpleGenerator {
                steps,
                completion_effects,
                ..
            } => {
                let mut unroll_steps = Vec::with_capacity(steps.len());
                for step in steps {
                    let SimpleGeneratorStepOutcome::Yield(value) = step.outcome else {
                        return None;
                    };
                    unroll_steps.push(StaticIteratorUnrollStep {
                        effects: step.effects,
                        value: Self::substitute_sent_expression(&value, &Expression::Undefined),
                    });
                }
                Some(StaticIteratorUnroll {
                    steps: unroll_steps,
                    completion_effects,
                })
            }
            _ => None,
        }
    }

    fn simple_generator_source_strict(&self, object: &Expression) -> Option<bool> {
        if let Expression::Identifier(name) = object
            && let Some(value) = self
                .state
                .speculation
                .static_semantics
                .local_value_binding(name)
                .or_else(|| self.global_value_binding(name))
            && !static_expression_matches(value, object)
        {
            return self.simple_generator_source_strict(value);
        }
        let materialized = self.materialize_static_expression(object);
        if !static_expression_matches(&materialized, object) {
            return self.simple_generator_source_strict(&materialized);
        }

        let Expression::Call { callee, .. } = object else {
            return None;
        };
        let LocalFunctionBinding::User(function_name) =
            self.resolve_function_binding_from_expression(callee)?
        else {
            return None;
        };
        self.user_function(&function_name)
            .map(|user_function| user_function.strict)
    }

    fn expand_static_lowered_for_of_at(
        &self,
        statements: &[Statement],
        index: usize,
    ) -> Option<(Vec<Statement>, usize)> {
        let Statement::Let {
            name: iterator_name,
            value: Expression::GetIterator(source_expression),
            ..
        } = statements.get(index)?
        else {
            return None;
        };
        let unroll = self.static_iterator_steps_for_unroll(source_expression)?;
        let done_index = ((index + 1)..statements.len()).find(|candidate| {
            matches!(
                statements.get(*candidate),
                Some(Statement::Let {
                    value: Expression::Bool(false),
                    ..
                })
            )
        })?;
        let Statement::While {
            condition: Expression::Bool(true),
            body,
            ..
        } = statements.get(done_index + 1)?
        else {
            return None;
        };
        let Some(Statement::Let {
            name: step_name,
            value: Expression::Call { callee, arguments },
            ..
        }) = body.first()
        else {
            return None;
        };
        let Expression::Member { object, property } = callee.as_ref() else {
            return None;
        };
        if !arguments.is_empty()
            || !matches!(object.as_ref(), Expression::Identifier(name) if name == iterator_name)
            || !matches!(property.as_ref(), Expression::String(name) if name == "next")
        {
            return None;
        }
        let Some(Statement::If { condition, .. }) = body.get(1) else {
            return None;
        };
        if !matches!(
            condition,
            Expression::Member { object, property }
                if matches!(object.as_ref(), Expression::Identifier(name) if name == step_name)
                    && matches!(property.as_ref(), Expression::String(name) if name == "done")
        ) {
            return None;
        }
        let iteration_body = body.get(2..)?;
        if iteration_body
            .iter()
            .any(Self::statement_contains_unhandled_loop_transfer)
        {
            return None;
        }

        let mut expanded = statements[(index + 1)..done_index].to_vec();
        for step in unroll.steps {
            let mut block_body = step.effects;
            block_body.push(Statement::Let {
                name: step_name.clone(),
                mutable: true,
                value: Expression::Object(vec![
                    ObjectEntry::Data {
                        key: Expression::String("done".to_string()),
                        value: Expression::Bool(false),
                    },
                    ObjectEntry::Data {
                        key: Expression::String("value".to_string()),
                        value: step.value,
                    },
                ]),
            });
            block_body.extend(iteration_body.iter().cloned());
            expanded.push(Statement::Block { body: block_body });
        }
        expanded.extend(unroll.completion_effects);
        Some((expanded, done_index + 2))
    }

    pub(in crate::backend::direct_wasm) fn expand_static_lowered_for_of_completion_effects(
        &self,
        statements: &[Statement],
    ) -> Vec<Statement> {
        let mut expanded = Vec::new();
        let mut index = 0;
        while index < statements.len() {
            if let Some((mut unrolled, next_index)) =
                self.expand_static_lowered_for_of_at(statements, index)
            {
                expanded.append(&mut unrolled);
                index = next_index;
                continue;
            }
            expanded.push(statements[index].clone());
            index += 1;
        }
        expanded
    }

    pub(in crate::backend::direct_wasm) fn consume_simple_async_generator_next_promise_outcome(
        &mut self,
        object: &Expression,
        arguments: &[CallArgument],
    ) -> DirectResult<Option<StaticEvalOutcome>> {
        if self.simple_generator_has_eager_call_time_prefix(object) {
            return Ok(None);
        }
        self.emit_simple_generator_call_time_prefix_effects(object)?;
        let Some((is_async, steps, completion_effects, completion_value)) =
            self.simple_generator_source_metadata(object)
        else {
            return Ok(None);
        };
        if !is_async {
            return Ok(None);
        }
        let source_strict = self
            .simple_generator_source_strict(object)
            .unwrap_or(self.state.speculation.execution_context.strict_mode);
        let sent_value = arguments
            .first()
            .map(|argument| match argument {
                CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                    self.materialize_static_expression(expression)
                }
            })
            .unwrap_or(Expression::Undefined);

        let binding_name = if let Expression::Identifier(name) = object {
            let binding_name = self
                .resolve_local_array_iterator_binding_name(name)
                .unwrap_or_else(|| name.clone());
            let Some(_) = self
                .state
                .speculation
                .static_semantics
                .local_array_iterator_binding(&binding_name)
                .and_then(|binding| binding.static_index)
            else {
                if std::env::var_os("AYY_TRACE_SIMPLE_GENERATORS").is_some() {
                    eprintln!(
                        "simple_async_next:no-static-index object={object:?} binding={binding_name}"
                    );
                }
                return Ok(None);
            };
            Some(binding_name)
        } else {
            None
        };
        let current_index = binding_name
            .as_ref()
            .and_then(|binding_name| {
                self.state
                    .speculation
                    .static_semantics
                    .local_array_iterator_binding(binding_name)
                    .and_then(|binding| binding.static_index)
            })
            .unwrap_or(0);
        if std::env::var_os("AYY_TRACE_SIMPLE_GENERATORS").is_some() {
            eprintln!(
                "simple_async_next object={object:?} binding={binding_name:?} current_index={current_index}"
            );
        }
        let set_binding_index = |compiler: &mut Self, next_index: usize| {
            if let Some(binding_name) = binding_name.as_ref()
                && let Some(index_local) = compiler
                    .state
                    .speculation
                    .static_semantics
                    .local_array_iterator_binding(binding_name)
                    .map(|binding| binding.index_local)
            {
                compiler.set_static_iterator_index_for_index_local(index_local, next_index);
            }
        };

        if let Some(step) = steps.get(current_index) {
            let substituted_effects = step
                .effects
                .iter()
                .map(|effect| Self::substitute_sent_statement(effect, &sent_value))
                .collect::<Vec<_>>();
            let substituted_effects =
                self.expand_static_lowered_for_of_completion_effects(&substituted_effects);
            self.register_bindings(&substituted_effects)?;
            if binding_name.is_none() {
                self.initialize_fresh_simple_generator_scoped_var_bindings(&substituted_effects);
            }
            let scoped_bindings = Self::simple_generator_scoped_effect_bindings(&substituted_effects);
            for (source_name, scoped_name) in &scoped_bindings {
                self.state
                    .push_scoped_lexical_binding(source_name, scoped_name.clone());
            }
            let scoped_source_names = scoped_bindings
                .iter()
                .map(|(source_name, _)| source_name.clone())
                .collect::<Vec<_>>();
            let throw_value = self.with_scoped_lexical_bindings_cleanup(
                scoped_source_names,
                |compiler| {
                    let mut prior_effects = Vec::new();
                    for effect in &substituted_effects {
                        match compiler.consume_throwing_simple_generator_next_effect_with_prior(
                            effect,
                            &prior_effects,
                            source_strict,
                        )? {
                            SimpleGeneratorNextEffectConsumption::Threw(throw_value) => {
                                return Ok(Some(throw_value));
                            }
                            SimpleGeneratorNextEffectConsumption::EmittedNoThrow => {}
                            SimpleGeneratorNextEffectConsumption::NotApplicable => {
                                if compiler.try_emit_static_simple_generator_binding_effect(
                                    effect,
                                    &prior_effects,
                                )? {
                                    prior_effects.push(effect.clone());
                                    continue;
                                }
                                if compiler.try_emit_static_simple_generator_call_effect(
                                    effect,
                                    &prior_effects,
                                )? {
                                    prior_effects.push(effect.clone());
                                    continue;
                                }
                                if compiler.try_emit_static_simple_generator_member_assignment_effect(
                                    effect,
                                    &prior_effects,
                                )? {
                                    prior_effects.push(effect.clone());
                                    continue;
                                }
                                compiler.sync_visible_runtime_bindings_for_statements(
                                    std::slice::from_ref(effect),
                                )?;
                                compiler.emit_statement(effect)?;
                            }
                        }
                        prior_effects.push(effect.clone());
                    }
                    Ok(None)
                },
            )?;
            if let Some(throw_value) = throw_value {
                set_binding_index(self, steps.len().saturating_add(1));
                return Ok(Some(StaticEvalOutcome::Throw(throw_value)));
            }
            if std::env::var_os("AYY_TRACE_SIMPLE_GENERATORS").is_some() {
                eprintln!(
                    "simple_async_next:effects_done current_index={current_index}"
                );
            }
            return Ok(Some(match &step.outcome {
                SimpleGeneratorStepOutcome::Yield(value) => {
                    if std::env::var_os("AYY_TRACE_SIMPLE_GENERATORS").is_some() {
                        eprintln!(
                            "simple_async_next:yield_outcome:start value={value:?} sent={sent_value:?}"
                        );
                    }
                    let mut yielded_value = Self::substitute_sent_expression(value, &sent_value);
                    if let Some(array_binding) =
                        self.resolve_array_binding_from_expression(&yielded_value)
                    {
                        yielded_value = Expression::Array(
                            array_binding
                                .values
                                .into_iter()
                                .map(|value| {
                                    ArrayElement::Expression(
                                        value.unwrap_or(Expression::Undefined),
                                    )
                                })
                                .collect(),
                        );
                    }
                    if std::env::var_os("AYY_TRACE_SIMPLE_GENERATORS").is_some() {
                        eprintln!(
                            "simple_async_next:yield_outcome:resolved value={yielded_value:?}"
                        );
                    }
                    let yielded_value = if matches!(yielded_value, Expression::Array(_)) {
                        yielded_value
                    } else {
                        match self.resolve_static_await_resolution_outcome(&yielded_value) {
                            Some(StaticEvalOutcome::Throw(throw_value)) => {
                                set_binding_index(self, steps.len().saturating_add(1));
                                return Ok(Some(StaticEvalOutcome::Throw(throw_value)));
                            }
                            Some(StaticEvalOutcome::Value(awaited_value)) => awaited_value,
                            None => yielded_value,
                        }
                    };
                    if std::env::var_os("AYY_TRACE_SIMPLE_GENERATORS").is_some() {
                        eprintln!(
                            "simple_async_next:yield_outcome:awaited value={yielded_value:?}"
                        );
                    }
                    set_binding_index(self, current_index.saturating_add(1));
                    if std::env::var_os("AYY_TRACE_SIMPLE_GENERATORS").is_some() {
                        eprintln!(
                            "simple_async_next:yield_outcome:return current_index={current_index}"
                        );
                    }
                    StaticEvalOutcome::Value(Expression::Object(vec![
                        ObjectEntry::Data {
                            key: Expression::String("done".to_string()),
                            value: Expression::Bool(false),
                        },
                        ObjectEntry::Data {
                            key: Expression::String("value".to_string()),
                            value: yielded_value,
                        },
                    ]))
                }
                SimpleGeneratorStepOutcome::Throw(value) => {
                    set_binding_index(self, steps.len().saturating_add(1));
                    StaticEvalOutcome::Throw(StaticThrowValue::Value(value.clone()))
                }
            }));
        }

        let next_index = if current_index >= steps.len() {
            steps.len().saturating_add(1)
        } else {
            current_index.saturating_add(1)
        };
        set_binding_index(self, next_index);
        if current_index == steps.len() {
            let substituted_completion_effects = completion_effects
                .iter()
                .map(|effect| Self::substitute_sent_statement(effect, &sent_value))
                .collect::<Vec<_>>();
            let substituted_completion_effects = self
                .expand_static_lowered_for_of_completion_effects(&substituted_completion_effects);
            self.register_bindings(&substituted_completion_effects)?;
            if binding_name.is_none() {
                self.initialize_fresh_simple_generator_scoped_var_bindings(
                    &substituted_completion_effects,
                );
            }
            let scoped_bindings =
                Self::simple_generator_scoped_effect_bindings(&substituted_completion_effects);
            for (source_name, scoped_name) in &scoped_bindings {
                self.state
                    .push_scoped_lexical_binding(source_name, scoped_name.clone());
            }
            let scoped_source_names = scoped_bindings
                .iter()
                .map(|(source_name, _)| source_name.clone())
                .collect::<Vec<_>>();
            let throw_value = self.with_scoped_lexical_bindings_cleanup(
                scoped_source_names,
                |compiler| {
                    let mut prior_effects = Vec::new();
                    for effect in &substituted_completion_effects {
                        match compiler.consume_throwing_simple_generator_next_effect_with_prior(
                            effect,
                            &prior_effects,
                            source_strict,
                        )? {
                            SimpleGeneratorNextEffectConsumption::Threw(throw_value) => {
                                return Ok(Some(throw_value));
                            }
                            SimpleGeneratorNextEffectConsumption::EmittedNoThrow => {}
                            SimpleGeneratorNextEffectConsumption::NotApplicable => {
                                if compiler.try_emit_static_simple_generator_binding_effect(
                                    effect,
                                    &prior_effects,
                                )? {
                                    prior_effects.push(effect.clone());
                                    continue;
                                }
                                if compiler.try_emit_static_simple_generator_call_effect(
                                    effect,
                                    &prior_effects,
                                )? {
                                    prior_effects.push(effect.clone());
                                    continue;
                                }
                                if compiler.try_emit_static_simple_generator_member_assignment_effect(
                                    effect,
                                    &prior_effects,
                                )? {
                                    prior_effects.push(effect.clone());
                                    continue;
                                }
                                compiler.sync_visible_runtime_bindings_for_statements(
                                    std::slice::from_ref(effect),
                                )?;
                                compiler.emit_statement(effect)?;
                            }
                        }
                        prior_effects.push(effect.clone());
                    }
                    Ok(None)
                },
            )?;
            if let Some(throw_value) = throw_value {
                set_binding_index(self, steps.len().saturating_add(1));
                return Ok(Some(StaticEvalOutcome::Throw(throw_value)));
            }
        }
        Ok(Some(StaticEvalOutcome::Value(Expression::Object(vec![
            ObjectEntry::Data {
                key: Expression::String("done".to_string()),
                value: Expression::Bool(true),
            },
            ObjectEntry::Data {
                key: Expression::String("value".to_string()),
                value: if current_index == steps.len() {
                    Self::substitute_sent_expression(&completion_value, &sent_value)
                } else {
                    Expression::Undefined
                },
            },
        ]))))
    }
}
