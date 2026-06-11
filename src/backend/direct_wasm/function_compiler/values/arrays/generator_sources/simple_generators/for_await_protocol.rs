use super::*;

/// Iterator state tracked while symbolically replaying a lowered
/// `for await (… of [..])` loop. The outer iterable is always a static array
/// in the supported shapes; destructuring patterns inside the loop body open
/// nested sync iterators that resolve to static step sequences.
enum ForAwaitProtocolIterator {
    StaticArray {
        values: Vec<Expression>,
        index: usize,
    },
    Steps {
        steps: Vec<SimpleGeneratorStep>,
        completion_value: Expression,
        /// Effects performed by the final (done) `next()` call, replayed when
        /// the completion step is consumed.
        completion_effects: Vec<Statement>,
        index: usize,
        /// The consumption-site iterator binding this state was seeded from
        /// (a generator object visible after the fold): its static index must
        /// be committed when the fold is consumed.
        binding_name: Option<String>,
        /// IteratorClose ran: the underlying generator is complete.
        closed: bool,
    },
}

#[derive(Default)]
struct ForAwaitProtocolContext {
    bindings: HashMap<String, Expression>,
    iterators: HashMap<String, ForAwaitProtocolIterator>,
    /// Nonlocal step effects (such as `nextCount = nextCount + 1` before a
    /// throwing `next`) encountered during the replay, in execution order.
    /// They must be re-emitted wherever the folded outcome is consumed.
    effects: Vec<Statement>,
    /// Names mutated by recorded effects: reads of these from the replay
    /// would observe stale static values, so they bail the fold.
    effect_names: HashSet<String>,
    /// Final static indices of tracked iterator bindings consumed by earlier
    /// iterations of the replay.
    committed_updates: HashMap<String, usize>,
}

enum ForAwaitProtocolControl {
    Completed,
    Return(Expression),
    Throw(Expression),
}

enum ForAwaitProtocolFlow {
    None,
    Return(Expression),
    Throw(Expression),
    Break(Option<String>),
}

const FOR_AWAIT_PROTOCOL_OUTER_VALUE_LIMIT: usize = 64;
const FOR_AWAIT_PROTOCOL_WHILE_LIMIT: usize = 1024;

impl<'a> FunctionCompiler<'a> {
    /// Resolves the completion of an async function body whose leading
    /// statement is a lowered for-await loop: a static protocol throw rejects
    /// the call's promise; a return or a normal loop completion (with no
    /// trailing statements) resolves it.
    pub(in crate::backend::direct_wasm) fn lowered_for_await_protocol_completion_outcome(
        &self,
        statements: &[Statement],
    ) -> Option<StaticEvalOutcome> {
        let (first, rest) = statements.split_first()?;
        match self.lowered_for_await_protocol_loop_control_flow(first)?.0 {
            ForAwaitProtocolControl::Throw(value) => {
                Some(StaticEvalOutcome::Throw(StaticThrowValue::Value(value)))
            }
            ForAwaitProtocolControl::Return(value) => Some(StaticEvalOutcome::Value(value)),
            ForAwaitProtocolControl::Completed => rest
                .is_empty()
                .then_some(StaticEvalOutcome::Value(Expression::Undefined)),
        }
    }

    /// Applies the observable side effects of a folded for-await protocol
    /// loop at the consumption site: nonlocal step effects are re-emitted and
    /// consumed tracked iterators (generator objects the destructure stepped
    /// or closed) advance to their final indices. Returns false when the body
    /// does not fold.
    pub(in crate::backend::direct_wasm) fn lowered_for_await_protocol_apply_call_effects(
        &mut self,
        statements: &[Statement],
    ) -> DirectResult<bool> {
        let Some((first, _)) = statements.split_first() else {
            return Ok(false);
        };
        let Some((_, effects)) = self.lowered_for_await_protocol_loop_control_flow(first) else {
            return Ok(false);
        };
        for effect in &effects {
            self.emit_statement(effect)?;
        }
        Ok(true)
    }

    /// Resolves a lowered for-await loop that statically throws through the
    /// destructuring iterator protocol to its throw value and pre-throw
    /// effects, for generator-step analysis: the loop becomes a throwing step
    /// instead of an opaque effect.
    pub(in crate::backend::direct_wasm) fn lowered_for_await_protocol_throw_step(
        &self,
        statement: &Statement,
    ) -> Option<(Expression, Vec<Statement>)> {
        match self.lowered_for_await_protocol_loop_control_flow(statement)? {
            (ForAwaitProtocolControl::Throw(value), effects) => Some((value, effects)),
            _ => None,
        }
    }

    /// Resolves a lowered for-await loop that completes its enclosing async
    /// generator with an undefined `return` to the effect statements its
    /// replay performed. Consumed tracked iterators are conveyed as
    /// `IteratorClose` effects so the standard close machinery applies their
    /// state at the consumption site.
    pub(in crate::backend::direct_wasm) fn lowered_for_await_protocol_return_effects(
        &self,
        statement: &Statement,
    ) -> Option<Vec<Statement>> {
        let (control, effects) = self.lowered_for_await_protocol_loop_control_flow(statement)?;
        let ForAwaitProtocolControl::Return(Expression::Undefined) = control else {
            return None;
        };
        Some(effects)
    }

    /// Resolves a lowered for-await loop that completes normally (the outer
    /// iterable exhausted without a throw, return, or break) to the effect
    /// statements its replay performed: enclosing-generator analysis can then
    /// continue past the loop instead of treating it as opaque.
    pub(in crate::backend::direct_wasm) fn lowered_for_await_protocol_completed_effects(
        &self,
        statement: &Statement,
    ) -> Option<Vec<Statement>> {
        let (control, effects) = self.lowered_for_await_protocol_loop_control_flow(statement)?;
        matches!(control, ForAwaitProtocolControl::Completed).then_some(effects)
    }

    /// Symbolically replays a lowered `for await (<pattern> of [<elements>])`
    /// loop so destructuring iterator-protocol errors (poisoned `value`/`done`
    /// getters, throwing `next`/`Symbol.iterator` methods, throwing property
    /// defaults) resolve to a static throw completion. The enclosing async
    /// function's call can then fold to a rejected-promise outcome instead of
    /// deferring to a runtime rejection chain.
    fn lowered_for_await_protocol_loop_control_flow(
        &self,
        statement: &Statement,
    ) -> Option<(ForAwaitProtocolControl, Vec<Statement>)> {
        let trace = crate::ayy_env_flag!("AYY_TRACE_FOR_AWAIT_PROTOCOL");
        macro_rules! trace {
            ($($arg:tt)*) => {
                if trace {
                    eprintln!("for_await_protocol:{}", format_args!($($arg)*));
                }
            };
        }
        trace!("start statement={statement:?}");
        let Statement::For {
            labels,
            init,
            condition,
            update: None,
            body,
            ..
        } = statement
        else {
            return None;
        };
        if !labels.is_empty()
            || !matches!(condition, Some(Expression::Bool(true)) | None)
        {
            trace!("reject labels/condition");
            return None;
        }
        let (iterator_name, source) = init.iter().find_map(|statement| {
            let Statement::Let {
                name,
                value: Expression::GetIterator(source),
                ..
            } = statement
            else {
                return None;
            };
            name.starts_with("__ayy_for_await_iter_")
                .then(|| (name.as_str(), source.as_ref()))
        })?;
        let done_name = init.iter().find_map(|statement| {
            let Statement::Let {
                name,
                value: Expression::Bool(false),
                ..
            } = statement
            else {
                return None;
            };
            name.starts_with("__ayy_for_of_done_").then_some(name.as_str())
        })?;
        let mut outer_state = if let Expression::Array(elements) = source {
            if elements.len() > FOR_AWAIT_PROTOCOL_OUTER_VALUE_LIMIT {
                return None;
            }
            let values = elements
                .iter()
                .map(|element| match element {
                    ArrayElement::Expression(value) => Some(value.clone()),
                    ArrayElement::Spread(_) => None,
                })
                .collect::<Option<Vec<_>>>()?;
            ForAwaitProtocolIterator::StaticArray { values, index: 0 }
        } else {
            let resolved = self.for_await_protocol_outer_iterator_state(source);
            if resolved.is_none() {
                trace!("reject outer-source source={source:?}");
            }
            resolved?
        };

        let [
            Statement::Let {
                name: step_name,
                value: next_call,
                ..
            },
            Statement::If {
                condition: guard_condition,
                then_branch: guard_then,
                else_branch: guard_else,
            },
            Statement::Let {
                name: value_name,
                value: value_expression,
                ..
            },
            rest @ ..,
        ] = body.as_slice()
        else {
            return None;
        };
        if !Self::for_await_protocol_is_next_call(next_call, iterator_name) {
            trace!("reject next-call shape");
            return None;
        }
        if !Self::for_await_protocol_is_member_of(guard_condition, step_name, "done") {
            return None;
        }
        if !guard_else.is_empty() {
            return None;
        }
        let [
            Statement::Assign {
                name: assigned_done,
                value: Expression::Bool(true),
            },
            Statement::Break { label: None },
        ] = guard_then.as_slice()
        else {
            return None;
        };
        if assigned_done != done_name {
            return None;
        }
        let step_value_read = match value_expression {
            Expression::Await(awaited) => awaited.as_ref(),
            other => other,
        };
        if !Self::for_await_protocol_is_member_of(step_value_read, step_name, "value") {
            trace!("reject step-value shape");
            return None;
        }

        let mut effects = Vec::new();
        let mut effect_names = HashSet::new();
        let mut iterator_updates: HashMap<String, (usize, bool)> = HashMap::new();
        let collect_iterator_updates =
            |context: &ForAwaitProtocolContext, updates: &mut HashMap<String, (usize, bool)>| {
                for state in context.iterators.values() {
                    if let ForAwaitProtocolIterator::Steps {
                        steps,
                        index,
                        binding_name: Some(binding_name),
                        closed,
                        ..
                    } = state
                    {
                        let final_index = if *closed {
                            steps.len().saturating_add(1).max(*index)
                        } else {
                            *index
                        };
                        updates.insert(binding_name.clone(), (final_index, *closed));
                    }
                }
            };
        let done_property = Expression::String("done".to_string());
        let value_property = Expression::String("value".to_string());
        for _ in 0..FOR_AWAIT_PROTOCOL_WHILE_LIMIT {
            let mut context = ForAwaitProtocolContext::default();
            // Effects recorded by earlier iterations stay observable.
            context.effect_names = std::mem::take(&mut effect_names);
            context.committed_updates = iterator_updates
                .iter()
                .map(|(name, (index, _))| (name.clone(), *index))
                .collect();
            // The loop shape's leading `step = iterator.next()` call.
            let step = match self.for_await_protocol_iterator_next(&mut outer_state, &mut context)?
            {
                Ok(step) => step,
                Err(throw_value) => {
                    effects.extend(std::mem::take(&mut context.effects));
                    if !self.static_iterator_throw_expression_is_portable(&throw_value) {
                        return None;
                    }
                    return Some((ForAwaitProtocolControl::Throw(throw_value), effects));
                }
            };
            let outer_throw = |effects: &mut Vec<Statement>,
                               context: &mut ForAwaitProtocolContext,
                               throw_value: Expression| {
                effects.extend(std::mem::take(&mut context.effects));
                self.static_iterator_throw_expression_is_portable(&throw_value)
                    .then_some((ForAwaitProtocolControl::Throw(throw_value), std::mem::take(effects)))
            };
            let done = match self.for_await_protocol_member_value(&step, &done_property)? {
                Ok(done) => done,
                Err(throw_value) => {
                    return outer_throw(&mut effects, &mut context, throw_value);
                }
            };
            let done = Self::for_await_protocol_to_boolean(&done)?;
            if done {
                effects.extend(std::mem::take(&mut context.effects));
                return Some((ForAwaitProtocolControl::Completed, effects));
            }
            let value = match self.for_await_protocol_member_value(&step, &value_property)? {
                Ok(value) => value,
                Err(throw_value) => {
                    return outer_throw(&mut effects, &mut context, throw_value);
                }
            };
            // `for await` awaits each iteration value.
            let value = match self.for_await_protocol_await_value(value)? {
                Ok(value) => value,
                Err(throw_value) => {
                    return outer_throw(&mut effects, &mut context, throw_value);
                }
            };
            context.bindings.insert(value_name.clone(), value);
            // The loop shape already consumed the outer `next` call; the body
            // may still close the outer iterator (the lowered catch path).
            context
                .iterators
                .insert(iterator_name.to_string(), outer_state);
            let executed = self.execute_for_await_protocol_statements(rest, &mut context);
            if executed.is_none() {
                trace!("reject body-execution");
            }
            let executed = executed;
            outer_state = context.iterators.remove(iterator_name)?;
            let executed = executed?;
            effects.extend(std::mem::take(&mut context.effects));
            effect_names = std::mem::take(&mut context.effect_names);
            collect_iterator_updates(&context, &mut iterator_updates);
            match executed {
                ForAwaitProtocolFlow::None => {}
                ForAwaitProtocolFlow::Break(None) => {
                    return Some((ForAwaitProtocolControl::Completed, effects));
                }
                ForAwaitProtocolFlow::Break(Some(_)) => return None,
                ForAwaitProtocolFlow::Return(value) => {
                    if !self.static_iterator_throw_expression_is_portable(&value) {
                        return None;
                    }
                    return Some((ForAwaitProtocolControl::Return(value), effects));
                }
                ForAwaitProtocolFlow::Throw(value) => {
                    if !self.static_iterator_throw_expression_is_portable(&value) {
                        return None;
                    }
                    return Some((ForAwaitProtocolControl::Throw(value), effects));
                }
            }
        }
        None
    }

    /// Resolves a non-array outer `for await` iterable to a replayable
    /// iterator state: a tracked generator-object binding (sync or async —
    /// the loop natively drives async generators) or a directly resolvable
    /// simple-generator source.
    fn for_await_protocol_outer_iterator_state(
        &self,
        source: &Expression,
    ) -> Option<ForAwaitProtocolIterator> {
        if let Some(state) = self.for_await_protocol_tracked_iterator_state_with_async(
            source,
            &HashMap::new(),
            true,
        ) {
            if crate::ayy_env_flag!("AYY_EXPERIMENT_OUTER_UNTRACKED")
                && let ForAwaitProtocolIterator::Steps {
                    steps,
                    completion_value,
                    completion_effects,
                    index,
                    closed,
                    ..
                } = state
            {
                return Some(ForAwaitProtocolIterator::Steps {
                    steps,
                    completion_value,
                    completion_effects,
                    index,
                    binding_name: None,
                    closed,
                });
            }
            return Some(state);
        }
        let (steps, completion_effects, completion_value) = self
            .resolve_simple_generator_iterator_source_kind(source)
            .and_then(|kind| match kind {
                IteratorSourceKind::SimpleGenerator {
                    steps,
                    completion_effects,
                    completion_value,
                    ..
                } => Some((steps, completion_effects, completion_value)),
                _ => None,
            })?;
        if !self.for_await_protocol_effects_are_replayable(&completion_effects)
            || !self.for_await_protocol_steps_have_replayable_effects(&steps)
        {
            return None;
        }
        Some(ForAwaitProtocolIterator::Steps {
            steps,
            completion_value,
            completion_effects,
            index: 0,
            binding_name: None,
            closed: false,
        })
    }

    fn for_await_protocol_is_next_call(expression: &Expression, iterator_name: &str) -> bool {
        let Expression::Call { callee, arguments } = expression else {
            return false;
        };
        arguments.is_empty()
            && Self::for_await_protocol_is_member_of(callee, iterator_name, "next")
    }

    fn for_await_protocol_is_member_of(
        expression: &Expression,
        object_name: &str,
        property_name: &str,
    ) -> bool {
        let Expression::Member { object, property } = expression else {
            return false;
        };
        matches!(object.as_ref(), Expression::Identifier(name) if name == object_name)
            && matches!(property.as_ref(), Expression::String(name) if name == property_name)
    }

    fn execute_for_await_protocol_statements(
        &self,
        statements: &[Statement],
        context: &mut ForAwaitProtocolContext,
    ) -> Option<ForAwaitProtocolFlow> {
        for statement in statements {
            if crate::ayy_env_flag!("AYY_TRACE_FOR_AWAIT_PROTOCOL") {
                eprintln!("for_await_protocol:execute {statement:?}");
            }
            match statement {
                Statement::Let { name, value, .. }
                | Statement::Var { name, value }
                | Statement::Assign { name, value } => {
                    if let Expression::GetIterator(source) = value {
                        let resolved = self.for_await_protocol_get_iterator(source, context);
                        if resolved.is_none()
                            && crate::ayy_env_flag!("AYY_TRACE_FOR_AWAIT_PROTOCOL")
                        {
                            eprintln!("for_await_protocol:reject-get-iterator {source:?}");
                        }
                        match resolved? {
                            Ok(state) => {
                                context.iterators.insert(name.clone(), state);
                                context
                                    .bindings
                                    .insert(name.clone(), Expression::Identifier(name.clone()));
                            }
                            Err(throw_value) => {
                                return Some(ForAwaitProtocolFlow::Throw(throw_value));
                            }
                        }
                        continue;
                    }
                    let value = match self.evaluate_for_await_protocol_expression(value, context)?
                    {
                        Ok(value) => value,
                        Err(throw_value) => {
                            return Some(ForAwaitProtocolFlow::Throw(throw_value));
                        }
                    };
                    // An assignment to a name with no local declaration in
                    // the replay targets a nonlocal (assignment-pattern
                    // stores like `for await ([x] of ...)`): it must be
                    // re-emitted at the fold site to stay observable.
                    if matches!(statement, Statement::Assign { .. })
                        && !name.starts_with("__ayy_")
                        && !context.bindings.contains_key(name)
                    {
                        if !self.static_iterator_throw_expression_is_portable(&value) {
                            return None;
                        }
                        context.effects.push(Statement::Assign {
                            name: name.clone(),
                            value: value.clone(),
                        });
                        context.effect_names.insert(name.clone());
                    }
                    context.bindings.insert(name.clone(), value);
                }
                Statement::If {
                    condition,
                    then_branch,
                    else_branch,
                } => {
                    let condition =
                        match self.evaluate_for_await_protocol_expression(condition, context)? {
                            Ok(value) => value,
                            Err(throw_value) => {
                                return Some(ForAwaitProtocolFlow::Throw(throw_value));
                            }
                        };
                    let branch = match condition {
                        Expression::Bool(true) => then_branch,
                        Expression::Bool(false) => else_branch,
                        _ => return None,
                    };
                    let result = self.execute_for_await_protocol_statements(branch, context)?;
                    if !matches!(result, ForAwaitProtocolFlow::None) {
                        return Some(result);
                    }
                }
                Statement::While {
                    labels,
                    condition,
                    break_hook: None,
                    body,
                } if labels.is_empty() => {
                    let mut iterations = 0;
                    loop {
                        if iterations >= FOR_AWAIT_PROTOCOL_WHILE_LIMIT {
                            return None;
                        }
                        iterations += 1;
                        let condition = match self
                            .evaluate_for_await_protocol_expression(condition, context)?
                        {
                            Ok(value) => value,
                            Err(throw_value) => {
                                return Some(ForAwaitProtocolFlow::Throw(throw_value));
                            }
                        };
                        match condition {
                            Expression::Bool(false) => break,
                            Expression::Bool(true) => {}
                            _ => return None,
                        }
                        match self.execute_for_await_protocol_statements(body, context)? {
                            ForAwaitProtocolFlow::None => {}
                            ForAwaitProtocolFlow::Break(None) => break,
                            ForAwaitProtocolFlow::Break(Some(_)) => return None,
                            other => return Some(other),
                        }
                    }
                }
                Statement::Try {
                    body,
                    catch_binding,
                    catch_setup,
                    catch_body,
                } => {
                    match self.execute_for_await_protocol_statements(body, context)? {
                        ForAwaitProtocolFlow::Throw(throw_value) => {
                            if let Some(catch_binding) = catch_binding {
                                context.bindings.insert(catch_binding.clone(), throw_value);
                            }
                            let setup_result =
                                self.execute_for_await_protocol_statements(catch_setup, context)?;
                            if !matches!(setup_result, ForAwaitProtocolFlow::None) {
                                return Some(setup_result);
                            }
                            let catch_result =
                                self.execute_for_await_protocol_statements(catch_body, context)?;
                            if !matches!(catch_result, ForAwaitProtocolFlow::None) {
                                return Some(catch_result);
                            }
                        }
                        ForAwaitProtocolFlow::None => {}
                        other => return Some(other),
                    }
                }
                Statement::Block { body } | Statement::Declaration { body } => {
                    let result = self.execute_for_await_protocol_statements(body, context)?;
                    if !matches!(result, ForAwaitProtocolFlow::None) {
                        return Some(result);
                    }
                }
                Statement::Throw(value) => {
                    return Some(
                        match self.evaluate_for_await_protocol_expression(value, context)? {
                            Ok(value) | Err(value) => ForAwaitProtocolFlow::Throw(value),
                        },
                    );
                }
                Statement::Return(value) => {
                    return Some(
                        match self.evaluate_for_await_protocol_expression(value, context)? {
                            Ok(value) => ForAwaitProtocolFlow::Return(value),
                            Err(throw_value) => ForAwaitProtocolFlow::Throw(throw_value),
                        },
                    );
                }
                Statement::Break { label } => {
                    return Some(ForAwaitProtocolFlow::Break(label.clone()));
                }
                Statement::Expression(expression) => {
                    if let Err(throw_value) =
                        self.evaluate_for_await_protocol_expression(expression, context)?
                    {
                        return Some(ForAwaitProtocolFlow::Throw(throw_value));
                    }
                }
                Statement::Print { values } => {
                    let mut evaluated_values = Vec::new();
                    for value in values {
                        match self.evaluate_for_await_protocol_expression(value, context)? {
                            Ok(value) => {
                                if !self.static_iterator_throw_expression_is_portable(&value) {
                                    return None;
                                }
                                evaluated_values.push(value);
                            }
                            Err(throw_value) => {
                                return Some(ForAwaitProtocolFlow::Throw(throw_value));
                            }
                        }
                    }
                    context.effects.push(Statement::Print {
                        values: evaluated_values,
                    });
                }
                _ => {
                    if crate::ayy_env_flag!("AYY_TRACE_FOR_AWAIT_PROTOCOL") {
                        eprintln!("for_await_protocol:reject-statement {statement:?}");
                    }
                    return None;
                }
            }
        }
        Some(ForAwaitProtocolFlow::None)
    }

    /// Resolves `GetIterator(source)` into a replayable iterator state, or a
    /// throw value when acquiring the iterator itself throws (poisoned or
    /// throwing `Symbol.iterator`).
    fn for_await_protocol_get_iterator(
        &self,
        source: &Expression,
        context: &mut ForAwaitProtocolContext,
    ) -> Option<Result<ForAwaitProtocolIterator, Expression>> {
        let source = match self.evaluate_for_await_protocol_expression(source, context)? {
            Ok(value) => value,
            Err(throw_value) => return Some(Err(throw_value)),
        };
        if let Expression::Array(elements) = &source {
            let values = elements
                .iter()
                .map(|element| match element {
                    ArrayElement::Expression(value) => Some(value.clone()),
                    ArrayElement::Spread(_) => None,
                })
                .collect::<Option<Vec<_>>>()?;
            return Some(Ok(ForAwaitProtocolIterator::StaticArray { values, index: 0 }));
        }
        if let Some(state) =
            self.for_await_protocol_tracked_iterator_state(&source, &context.committed_updates)
        {
            return Some(Ok(state));
        }
        if let Some((steps, completion_effects, completion_value)) = self
            .resolve_simple_generator_iterator_source_kind(&source)
            .and_then(|kind| match kind {
                IteratorSourceKind::SimpleGenerator {
                    is_async: false,
                    steps,
                    completion_effects,
                    completion_value,
                } => Some((steps, completion_effects, completion_value)),
                _ => None,
            })
            .or_else(|| self.resolve_static_iterable_simple_generator_source(&source))
        {
            if !self.for_await_protocol_effects_are_replayable(&completion_effects) {
                return None;
            }
            if !self.for_await_protocol_steps_have_replayable_effects_with_close(&steps, true) {
                return None;
            }
            return Some(Ok(ForAwaitProtocolIterator::Steps {
                steps,
                completion_value,
                completion_effects,
                index: 0,
                binding_name: None,
                closed: false,
            }));
        }
        let symbol_iterator = self.materialize_static_expression(&Expression::Member {
            object: Box::new(Expression::Identifier("Symbol".to_string())),
            property: Box::new(Expression::String("iterator".to_string())),
        });
        if let Some(getter_binding) = self.resolve_member_getter_binding(&source, &symbol_iterator)
        {
            let outcome = self.for_await_protocol_function_outcome(&getter_binding, &[], &source)?;
            match outcome {
                Err(throw_value) => return Some(Err(throw_value)),
                Ok(_) => return None,
            }
        }
        if let Some(method_binding) =
            self.resolve_member_function_binding(&source, &symbol_iterator)
        {
            match self.for_await_protocol_function_outcome(&method_binding, &[], &source)? {
                Err(throw_value) => return Some(Err(throw_value)),
                Ok(iterator_value) => {
                    let (steps, completion_effects, completion_value) = self
                        .resolve_static_iterator_object_simple_generator_source(&iterator_value)?;
                    if !self.for_await_protocol_effects_are_replayable(&completion_effects)
                        || !self
                            .for_await_protocol_steps_have_replayable_effects_with_close(
                                &steps, true,
                            )
                    {
                        return None;
                    }
                    return Some(Ok(ForAwaitProtocolIterator::Steps {
                        steps,
                        completion_value,
                        completion_effects,
                        index: 0,
                        binding_name: None,
                        closed: false,
                    }));
                }
            }
        }
        None
    }

    /// Resolves an identifier that names an already-tracked iterator binding
    /// (a generator object bound at the consumption site) to a replayable
    /// state, starting from its current static index.
    fn for_await_protocol_tracked_iterator_state(
        &self,
        source: &Expression,
        committed_updates: &HashMap<String, usize>,
    ) -> Option<ForAwaitProtocolIterator> {
        self.for_await_protocol_tracked_iterator_state_with_async(source, committed_updates, false)
    }

    fn for_await_protocol_tracked_iterator_state_with_async(
        &self,
        source: &Expression,
        committed_updates: &HashMap<String, usize>,
        allow_async: bool,
    ) -> Option<ForAwaitProtocolIterator> {
        let Expression::Identifier(name) = source else {
            return None;
        };
        let binding_name = self
            .resolve_local_array_iterator_binding_name(name)
            .unwrap_or_else(|| name.clone());
        let binding = self
            .state
            .speculation
            .static_semantics
            .local_array_iterator_binding(&binding_name)?;
        let index = committed_updates
            .get(&binding_name)
            .copied()
            .or(binding.static_index)
            .unwrap_or(0);
        match &binding.source {
            IteratorSourceKind::SimpleGenerator {
                is_async,
                steps,
                completion_effects,
                completion_value,
            } => {
                if (*is_async && !allow_async)
                    || !completion_effects.is_empty()
                    || !self.for_await_protocol_steps_have_replayable_effects(steps)
                {
                    return None;
                }
                Some(ForAwaitProtocolIterator::Steps {
                    steps: steps.clone(),
                    completion_value: completion_value.clone(),
                    completion_effects: Vec::new(),
                    index,
                    binding_name: Some(binding_name.clone()),
                    closed: false,
                })
            }
            IteratorSourceKind::StaticArray {
                values,
                keys_only: false,
                ..
            } => Some(ForAwaitProtocolIterator::StaticArray {
                values: values
                    .iter()
                    .map(|value| value.clone().unwrap_or(Expression::Undefined))
                    .collect(),
                index,
            }),
            _ => None,
        }
    }

    /// Step effects can be replayed at the consumption site when they are
    /// assignments to source-named nonlocals whose values only mention
    /// portable bindings (`nextCount = nextCount + 1`).
    fn for_await_protocol_steps_have_replayable_effects(
        &self,
        steps: &[SimpleGeneratorStep],
    ) -> bool {
        self.for_await_protocol_steps_have_replayable_effects_with_close(steps, false)
    }

    /// `allow_observable_close` admits steps whose close effects are exactly
    /// the `IteratorClose(target)` shape: the replay models that close as an
    /// observable user `return()` call (untracked inner iterators only — the
    /// tracked-binding drain machinery assumes close-effect-free steps).
    fn for_await_protocol_steps_have_replayable_effects_with_close(
        &self,
        steps: &[SimpleGeneratorStep],
        allow_observable_close: bool,
    ) -> bool {
        steps.iter().all(|step| {
            let close_ok = step.close_effects.is_empty()
                || (allow_observable_close
                    && matches!(
                        step.close_effects.as_slice(),
                        [Statement::Expression(Expression::IteratorClose(_))]
                    ));
            close_ok && self.for_await_protocol_effects_are_replayable(&step.effects)
        })
    }

    fn for_await_protocol_effects_are_replayable(&self, effects: &[Statement]) -> bool {
        effects.iter().all(|effect| match effect {
            Statement::Assign { name, value } => {
                !name.starts_with("__ayy_")
                    && self.static_iterator_throw_expression_is_portable(value)
            }
            Statement::Expression(Expression::Update { name, .. }) => !name.starts_with("__ayy_"),
            _ => false,
        })
    }

    fn for_await_protocol_record_step_effects(
        &self,
        context: &mut ForAwaitProtocolContext,
        effects: &[Statement],
    ) {
        for effect in effects {
            match effect {
                Statement::Assign { name, value } => {
                    // Track the post-assignment value symbolically so later
                    // replay reads (in-loop assertions of step counters)
                    // observe the live value instead of bailing. Evaluation
                    // precedes the effect-name guard: the value's own reads
                    // see the pre-assignment state.
                    let evaluated =
                        self.evaluate_for_await_protocol_nonlocal_assignment(value, context);
                    context.effect_names.insert(name.clone());
                    if let Some(evaluated) = evaluated {
                        context.bindings.insert(name.clone(), evaluated);
                    } else {
                        context.bindings.remove(name);
                    }
                }
                Statement::Expression(Expression::Update { name, .. }) => {
                    context.effect_names.insert(name.clone());
                    context.bindings.remove(name);
                }
                _ => {}
            }
            context.effects.push(effect.clone());
        }
    }

    fn for_await_protocol_function_outcome(
        &self,
        binding: &LocalFunctionBinding,
        arguments: &[CallArgument],
        this_binding: &Expression,
    ) -> Option<Result<Expression, Expression>> {
        let outcome = self
            .resolve_static_function_outcome_from_binding_with_call_frame_and_context(
                binding,
                arguments,
                this_binding,
                self.current_function_name(),
            )
            .or_else(|| {
                self.resolve_static_function_outcome_from_binding_with_context(
                    binding,
                    arguments,
                    self.current_function_name(),
                )
            })?;
        Some(match outcome {
            StaticEvalOutcome::Value(value) => Ok(value),
            StaticEvalOutcome::Throw(throw_value) => {
                Err(self.resolve_static_throw_value_expression(&throw_value)?)
            }
        })
    }

    fn for_await_protocol_iterator_next(
        &self,
        state: &mut ForAwaitProtocolIterator,
        context: &mut ForAwaitProtocolContext,
    ) -> Option<Result<Expression, Expression>> {
        match state {
            ForAwaitProtocolIterator::StaticArray { values, index } => {
                let result = if *index < values.len() {
                    Self::for_await_protocol_step_object(false, values[*index].clone())
                } else {
                    Self::for_await_protocol_step_object(true, Expression::Undefined)
                };
                *index = index.saturating_add(1);
                Some(Ok(result))
            }
            ForAwaitProtocolIterator::Steps {
                steps,
                completion_value,
                completion_effects,
                index,
                binding_name,
                ..
            } => {
                let current = *index;
                *index = index.saturating_add(1);
                // Consumption of an iterator tracked at the fold site is
                // conveyed as a synthetic `next()` statement: re-emitting it
                // advances the binding and replays the step's own effects
                // through the standard machinery. Throwing steps stay
                // effect-recorded (their throw rides on the fold outcome).
                let binding_name = binding_name.clone();
                let synthetic_next = move |context: &mut ForAwaitProtocolContext| {
                    if let Some(binding_name) = &binding_name {
                        context.effects.push(Statement::Expression(Expression::Call {
                            callee: Box::new(Expression::Member {
                                object: Box::new(Expression::Identifier(binding_name.clone())),
                                property: Box::new(Expression::String("next".to_string())),
                            }),
                            arguments: Vec::new(),
                        }));
                        true
                    } else {
                        false
                    }
                };
                let Some(step) = steps.get(current) else {
                    // The completion-consuming `next()`: its effects replay
                    // exactly once (synthetic next statements re-emit them
                    // through the standard machinery for tracked bindings).
                    if !synthetic_next(context) && current == steps.len() {
                        let completion_effects = completion_effects.clone();
                        self.for_await_protocol_record_step_effects(context, &completion_effects);
                    }
                    return Some(Ok(Self::for_await_protocol_step_object(
                        true,
                        completion_value.clone(),
                    )));
                };
                if matches!(&step.outcome, SimpleGeneratorStepOutcome::Yield(_))
                    && synthetic_next(context)
                {
                    // The synthetic statement re-emits the step's own effects;
                    // only their names need registering so later replay reads
                    // of those nonlocals bail instead of observing stale
                    // static values.
                    for effect in &step.effects {
                        match effect {
                            Statement::Assign { name, .. }
                            | Statement::Expression(Expression::Update { name, .. }) => {
                                context.effect_names.insert(name.clone());
                            }
                            _ => {}
                        }
                    }
                } else {
                    self.for_await_protocol_record_step_effects(context, &step.effects);
                }
                match &step.outcome {
                    SimpleGeneratorStepOutcome::Yield(value) => Some(Ok(
                        Self::for_await_protocol_step_object(false, value.clone()),
                    )),
                    // A raw step-result object: resolve `done` through the
                    // protocol now (IteratorComplete runs eagerly), but defer
                    // the `value` member so a poisoned accessor only throws
                    // when the pattern actually reads it.
                    SimpleGeneratorStepOutcome::YieldResult(result) => {
                        let done = match self.for_await_protocol_member_value(
                            result,
                            &Expression::String("done".to_string()),
                        )? {
                            Ok(done) => done,
                            Err(throw_value) => return Some(Err(throw_value)),
                        };
                        let done = Self::for_await_protocol_to_boolean(&done)?;
                        Some(Ok(Expression::Object(vec![
                            ObjectEntry::Data {
                                key: Expression::String("done".to_string()),
                                value: Expression::Bool(done),
                            },
                            ObjectEntry::Data {
                                key: Expression::String("value".to_string()),
                                value: Expression::Member {
                                    object: Box::new(result.clone()),
                                    property: Box::new(Expression::String("value".to_string())),
                                },
                            },
                        ])))
                    }
                    SimpleGeneratorStepOutcome::Throw(value) => Some(Err(value.clone())),
                }
            }
        }
    }

    fn for_await_protocol_step_object(done: bool, value: Expression) -> Expression {
        Expression::Object(vec![
            ObjectEntry::Data {
                key: Expression::String("done".to_string()),
                value: Expression::Bool(done),
            },
            ObjectEntry::Data {
                key: Expression::String("value".to_string()),
                value,
            },
        ])
    }

    fn for_await_protocol_close_iterator(
        &self,
        state: &mut ForAwaitProtocolIterator,
        context: &mut ForAwaitProtocolContext,
    ) -> Option<Result<Expression, Expression>> {
        match state {
            // Array iterators have no observable `return`; closing is a no-op.
            ForAwaitProtocolIterator::StaticArray { .. } => Some(Ok(Expression::Undefined)),
            // Tracked-binding step sources are only admitted when their steps
            // carry no close effects, so closing is unobservable beyond
            // completing the underlying generator. A close of a tracked
            // binding is conveyed as draining synthetic `next()` statements:
            // re-emitting them advances the binding to its completed state
            // through the standard step machinery (the remaining steps are
            // effect-free). Untracked sources may carry an `IteratorClose`
            // close effect: that close is an observable user `return()` call
            // the replay models directly.
            ForAwaitProtocolIterator::Steps {
                steps,
                index,
                closed,
                binding_name,
                ..
            } => {
                if *closed {
                    return Some(Ok(Expression::Undefined));
                }
                if let Some(binding_name) = binding_name {
                    let drained = steps.len().saturating_add(1);
                    for _ in *index..drained {
                        context.effects.push(Statement::Expression(Expression::Call {
                            callee: Box::new(Expression::Member {
                                object: Box::new(Expression::Identifier(binding_name.clone())),
                                property: Box::new(Expression::String("next".to_string())),
                            }),
                            arguments: Vec::new(),
                        }));
                    }
                    *index = drained;
                    *closed = true;
                    return Some(Ok(Expression::Undefined));
                }
                let close_target = steps.iter().find_map(|step| {
                    let [Statement::Expression(Expression::IteratorClose(target))] =
                        step.close_effects.as_slice()
                    else {
                        return None;
                    };
                    Some(target.as_ref().clone())
                });
                *closed = true;
                let Some(close_target) = close_target else {
                    return Some(Ok(Expression::Undefined));
                };
                self.for_await_protocol_replay_iterator_return(&close_target, context)
            }
        }
    }

    /// Replays an observable `return()` call performed by IteratorClose: its
    /// nonlocal assignment effects are recorded for re-emission at the fold
    /// site, a non-object result resolves to the protocol TypeError, and a
    /// throwing `return()` propagates its throw value (the lowered statement
    /// structure already distinguishes normal-completion closes, which
    /// propagate, from throw-completion closes, which swallow).
    fn for_await_protocol_replay_iterator_return(
        &self,
        close_target: &Expression,
        context: &mut ForAwaitProtocolContext,
    ) -> Option<Result<Expression, Expression>> {
        let return_property = Expression::String("return".to_string());
        if let Some(getter_binding) =
            self.resolve_member_getter_binding(close_target, &return_property)
        {
            return match self.for_await_protocol_function_outcome(
                &getter_binding,
                &[],
                close_target,
            )? {
                Err(throw_value) => Some(Err(throw_value)),
                Ok(_) => None,
            };
        }
        let LocalFunctionBinding::User(function_name) =
            self.resolve_member_function_binding(close_target, &return_property)?
        else {
            return None;
        };
        let function = self.resolve_registered_function_declaration(&function_name)?;
        let user_function = self.user_function(&function_name)?;
        if !user_function.params.is_empty()
            || self.user_function_mentions_direct_eval(user_function)
        {
            return None;
        }
        let type_error = || {
            Expression::New {
                callee: Box::new(Expression::Identifier("TypeError".to_string())),
                arguments: Vec::new(),
            }
        };
        for statement in &function.body {
            match statement {
                Statement::Assign { name, value } => {
                    if name.starts_with("__ayy_") {
                        return None;
                    }
                    // `this` inside the replayed `return()` is the close
                    // target; a bare `arguments` reference is the call's
                    // (empty) arguments object, observably an empty list.
                    let value = &Self::substitute_this_in_expression(value, close_target);
                    let value = &if matches!(value, Expression::Identifier(name) if name == "arguments")
                    {
                        Expression::Array(Vec::new())
                    } else {
                        value.clone()
                    };
                    if !self.static_iterator_throw_expression_is_portable(value) {
                        return None;
                    }
                    let statement = &Statement::Assign {
                        name: name.clone(),
                        value: value.clone(),
                    };
                    // Track the post-assignment value symbolically when it
                    // evaluates, so later replay reads (in-loop assertions of
                    // the close counter) observe the updated value instead of
                    // bailing on a stale static binding.
                    let evaluated =
                        self.evaluate_for_await_protocol_nonlocal_assignment(value, context);
                    context.effects.push(statement.clone());
                    context.effect_names.insert(name.clone());
                    if let Some(evaluated) = evaluated {
                        context.bindings.insert(name.clone(), evaluated);
                    } else {
                        context.bindings.remove(name);
                    }
                }
                Statement::Expression(Expression::Update { name, .. }) => {
                    if name.starts_with("__ayy_") {
                        return None;
                    }
                    context.effects.push(statement.clone());
                    context.effect_names.insert(name.clone());
                    context.bindings.remove(name);
                }
                Statement::Block { body } if body.is_empty() => {}
                Statement::Throw(value) => {
                    if !self.static_iterator_throw_expression_is_portable(value) {
                        return None;
                    }
                    return Some(Err(value.clone()));
                }
                Statement::Return(value) => {
                    return Some(match value {
                        Expression::Object(_) | Expression::Array(_) | Expression::New { .. } => {
                            Ok(Expression::Undefined)
                        }
                        Expression::Number(_)
                        | Expression::BigInt(_)
                        | Expression::String(_)
                        | Expression::Bool(_)
                        | Expression::Null
                        | Expression::Undefined => Err(type_error()),
                        _ => return None,
                    });
                }
                _ => return None,
            }
        }
        // Implicit `return undefined`: not an object, the close throws.
        Some(Err(type_error()))
    }

    /// Evaluates a nonlocal assignment's value during a `return()` replay to
    /// its concrete result where possible (`doneCallCount + 1` with a known
    /// static `doneCallCount`), so the symbolic context tracks the live value.
    fn evaluate_for_await_protocol_nonlocal_assignment(
        &self,
        value: &Expression,
        context: &mut ForAwaitProtocolContext,
    ) -> Option<Expression> {
        match self.evaluate_for_await_protocol_expression(value, context) {
            Some(Ok(evaluated)) => match evaluated {
                Expression::Number(_)
                | Expression::BigInt(_)
                | Expression::String(_)
                | Expression::Bool(_)
                | Expression::Null
                | Expression::Undefined
                // Identifiers stay symbolic (reference identity); literal
                // object shapes keep their member reads replayable.
                | Expression::Identifier(_)
                | Expression::Object(_)
                | Expression::Array(_) => Some(evaluated),
                _ => None,
            },
            _ => None,
        }
    }

    fn evaluate_for_await_protocol_expression(
        &self,
        expression: &Expression,
        context: &mut ForAwaitProtocolContext,
    ) -> Option<Result<Expression, Expression>> {
        match expression {
            Expression::Number(_)
            | Expression::BigInt(_)
            | Expression::String(_)
            | Expression::Bool(_)
            | Expression::Null
            | Expression::Undefined => Some(Ok(expression.clone())),
            Expression::Identifier(name) => {
                if let Some(value) = context.bindings.get(name) {
                    return Some(Ok(value.clone()));
                }
                // A read of a name an earlier recorded effect mutated would
                // observe a stale static value.
                if context.effect_names.contains(name) {
                    return None;
                }
                // Free identifiers must resolve at the consumption site; an
                // unresolvable reference (which would throw at runtime) bails
                // the fold instead of folding to a benign global read. An
                // implicit-global slot is not enough: it may be uninitialized
                // at runtime.
                if !self.for_await_protocol_identifier_resolves(name) {
                    return None;
                }
                Some(Ok(expression.clone()))
            }
            Expression::Object(_) | Expression::Array(_) => Some(Ok(expression.clone())),
            Expression::Await(inner) => {
                let value = match self.evaluate_for_await_protocol_expression(inner, context)? {
                    Ok(value) => value,
                    Err(throw_value) => return Some(Err(throw_value)),
                };
                self.for_await_protocol_await_value(value)
            }
            Expression::Member { object, property } => {
                let property_key =
                    match self.for_await_protocol_property_key(property, context)? {
                        Ok(key) => key,
                        Err(throw_value) => return Some(Err(throw_value)),
                    };
                let object = match self.evaluate_for_await_protocol_expression(object, context)? {
                    Ok(value) => value,
                    Err(throw_value) => return Some(Err(throw_value)),
                };
                self.for_await_protocol_member_value(&object, &property_key)
            }
            Expression::Call { callee, arguments } => {
                // The plain harness `assert(value[, message])` call: passes
                // only for the literal boolean true.
                if matches!(callee.as_ref(), Expression::Identifier(name) if name == "assert")
                    && !context.bindings.contains_key("assert")
                    && (1..=2).contains(&arguments.len())
                    && self.for_await_protocol_identifier_resolves("Test262Error")
                {
                    let CallArgument::Expression(argument) = arguments.first()? else {
                        return None;
                    };
                    let value = match self.evaluate_for_await_protocol_expression(argument, context)? {
                        Ok(value) => value,
                        Err(throw_value) => return Some(Err(throw_value)),
                    };
                    let passed = matches!(value, Expression::Bool(true));
                    if !passed
                        && !matches!(
                            value,
                            Expression::Bool(_)
                                | Expression::Number(_)
                                | Expression::String(_)
                                | Expression::Null
                                | Expression::Undefined
                        )
                    {
                        // Non-boolean object-like values fail `=== true` too,
                        // but only commit when the shape is unambiguous.
                        if !matches!(value, Expression::Object(_) | Expression::Array(_)) {
                            return None;
                        }
                    }
                    return Some(if passed {
                        Ok(Expression::Undefined)
                    } else {
                        Err(Expression::New {
                            callee: Box::new(Expression::Identifier("Test262Error".to_string())),
                            arguments: Vec::new(),
                        })
                    });
                }
                if let Expression::Member { object, property } = callee.as_ref()
                    && let Expression::Identifier(object_name) = object.as_ref()
                    && let Expression::String(method_name) = property.as_ref()
                {
                    // Harness same-value assertions: the emit-time machinery
                    // intercepts these calls by shape (their bodies do not
                    // resolve through the generic static outcome machinery),
                    // so the replay mirrors that interception.
                    if object_name == "assert"
                        && !context.bindings.contains_key("assert")
                        && matches!(method_name.as_str(), "sameValue" | "notSameValue")
                        && (2..=3).contains(&arguments.len())
                        && self.for_await_protocol_identifier_resolves("Test262Error")
                    {
                        let mut evaluated = Vec::new();
                        for argument in arguments.iter().take(2) {
                            let CallArgument::Expression(argument) = argument else {
                                return None;
                            };
                            match self.evaluate_for_await_protocol_expression(argument, context)? {
                                Ok(value) => evaluated.push(value),
                                Err(throw_value) => return Some(Err(throw_value)),
                            }
                        }
                        let [actual, expected] = evaluated.as_slice() else {
                            return None;
                        };
                        // Numbers compare with `Object.is` semantics: NaN is
                        // self-equal and signed zeros are distinct. Matching
                        // non-number value expressions (the same identifier
                        // binding on both sides) are identity-equal.
                        let equal = if let (Expression::Number(actual), Expression::Number(expected)) =
                            (actual, expected)
                        {
                            (actual.is_nan() && expected.is_nan())
                                || (actual == expected
                                    && actual.is_sign_positive() == expected.is_sign_positive())
                        } else if static_expression_matches(actual, expected)
                            && matches!(
                                actual,
                                Expression::Identifier(_)
                                    | Expression::String(_)
                                    | Expression::BigInt(_)
                                    | Expression::Bool(_)
                                    | Expression::Null
                                    | Expression::Undefined
                            )
                        {
                            true
                        } else {
                            let actual = self.for_await_protocol_comparable_value(actual.clone());
                            let expected =
                                self.for_await_protocol_comparable_value(expected.clone());
                            Self::for_await_protocol_values_equal(&actual, &expected, false)?
                        };
                        let passed = if method_name == "sameValue" {
                            equal
                        } else {
                            !equal
                        };
                        return Some(if passed {
                            Ok(Expression::Undefined)
                        } else {
                            Err(Expression::New {
                                callee: Box::new(Expression::Identifier(
                                    "Test262Error".to_string(),
                                )),
                                arguments: Vec::new(),
                            })
                        });
                    }
                    if method_name == "next"
                        && arguments.is_empty()
                        && context.iterators.contains_key(object_name)
                    {
                        let mut state = context.iterators.remove(object_name)?;
                        let result = self.for_await_protocol_iterator_next(&mut state, context);
                        context.iterators.insert(object_name.clone(), state);
                        return result;
                    }
                    if method_name == "push"
                        && matches!(
                            context.bindings.get(object_name),
                            Some(Expression::Array(_))
                        )
                    {
                        let mut pushed = Vec::new();
                        for argument in arguments {
                            let CallArgument::Expression(argument) = argument else {
                                return None;
                            };
                            match self
                                .evaluate_for_await_protocol_expression(argument, context)?
                            {
                                Ok(value) => pushed.push(ArrayElement::Expression(value)),
                                Err(throw_value) => return Some(Err(throw_value)),
                            }
                        }
                        let Some(Expression::Array(elements)) =
                            context.bindings.get_mut(object_name)
                        else {
                            return None;
                        };
                        elements.extend(pushed);
                        let length = elements.len();
                        return Some(Ok(Expression::Number(length as f64)));
                    }
                }
                let mut call_arguments = Vec::new();
                for argument in arguments {
                    let CallArgument::Expression(argument) = argument else {
                        return None;
                    };
                    match self.evaluate_for_await_protocol_expression(argument, context)? {
                        Ok(value) => call_arguments.push(CallArgument::Expression(value)),
                        Err(throw_value) => return Some(Err(throw_value)),
                    }
                }
                let binding = self.resolve_function_binding_from_expression(callee);
                if binding.is_none() && crate::ayy_env_flag!("AYY_TRACE_FOR_AWAIT_PROTOCOL") {
                    eprintln!("for_await_protocol:reject-call-binding {callee:?}");
                }
                let binding = binding?;
                let outcome = self
                    .resolve_static_function_outcome_from_binding_with_context(
                        &binding,
                        &call_arguments,
                        self.current_function_name(),
                    );
                if outcome.is_none() && crate::ayy_env_flag!("AYY_TRACE_FOR_AWAIT_PROTOCOL") {
                    eprintln!(
                        "for_await_protocol:reject-call-outcome callee={callee:?} args={call_arguments:?}"
                    );
                }
                let outcome = outcome?;
                Some(match outcome {
                    StaticEvalOutcome::Value(value) => Ok(value),
                    StaticEvalOutcome::Throw(throw_value) => {
                        Err(self.resolve_static_throw_value_expression(&throw_value)?)
                    }
                })
            }
            Expression::New { callee, arguments } => {
                let Expression::Identifier(constructor_name) = callee.as_ref() else {
                    return None;
                };
                if !matches!(
                    constructor_name.as_str(),
                    "Error"
                        | "TypeError"
                        | "RangeError"
                        | "ReferenceError"
                        | "SyntaxError"
                        | "EvalError"
                        | "URIError"
                        | "AggregateError"
                        | "Test262Error"
                ) || !self.is_unshadowed_builtin_identifier(constructor_name)
                {
                    return None;
                }
                let mut evaluated_arguments = Vec::new();
                for argument in arguments {
                    let CallArgument::Expression(argument) = argument else {
                        return None;
                    };
                    match self.evaluate_for_await_protocol_expression(argument, context)? {
                        Ok(value) => evaluated_arguments.push(CallArgument::Expression(value)),
                        Err(throw_value) => return Some(Err(throw_value)),
                    }
                }
                Some(Ok(Expression::New {
                    callee: callee.clone(),
                    arguments: evaluated_arguments,
                }))
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                let condition =
                    match self.evaluate_for_await_protocol_expression(condition, context)? {
                        Ok(value) => value,
                        Err(throw_value) => return Some(Err(throw_value)),
                    };
                match condition {
                    Expression::Bool(true) => {
                        self.evaluate_for_await_protocol_expression(then_expression, context)
                    }
                    Expression::Bool(false) => {
                        self.evaluate_for_await_protocol_expression(else_expression, context)
                    }
                    _ => None,
                }
            }
            Expression::Binary { op, left, right } => {
                self.evaluate_for_await_protocol_binary(*op, left, right, context)
            }
            Expression::Unary {
                op: UnaryOp::Not,
                expression,
            } => {
                let value = match self.evaluate_for_await_protocol_expression(expression, context)?
                {
                    Ok(value) => value,
                    Err(throw_value) => return Some(Err(throw_value)),
                };
                let value = Self::for_await_protocol_to_boolean(&value)?;
                Some(Ok(Expression::Bool(!value)))
            }
            Expression::IteratorClose(target) => {
                let Expression::Identifier(name) = target.as_ref() else {
                    return None;
                };
                let mut state = context.iterators.remove(name)?;
                let result = self.for_await_protocol_close_iterator(&mut state, context);
                context.iterators.insert(name.clone(), state);
                result
            }
            Expression::Sequence(expressions) => {
                let mut last = Expression::Undefined;
                for expression in expressions {
                    match self.evaluate_for_await_protocol_expression(expression, context)? {
                        Ok(value) => last = value,
                        Err(throw_value) => return Some(Err(throw_value)),
                    }
                }
                Some(Ok(last))
            }
            _ => {
                if crate::ayy_env_flag!("AYY_TRACE_FOR_AWAIT_PROTOCOL") {
                    eprintln!("for_await_protocol:reject-expression {expression:?}");
                }
                None
            }
        }
    }

    /// Awaiting a value resolves to the value itself when it is statically
    /// known to not be a thenable.
    fn for_await_protocol_await_value(
        &self,
        value: Expression,
    ) -> Option<Result<Expression, Expression>> {
        match &value {
            Expression::Number(_)
            | Expression::BigInt(_)
            | Expression::String(_)
            | Expression::Bool(_)
            | Expression::Null
            | Expression::Undefined => Some(Ok(value)),
            Expression::Object(entries) => {
                let then_property = Expression::String("then".to_string());
                entries
                    .iter()
                    .all(|entry| match entry {
                        ObjectEntry::Data { key, .. }
                        | ObjectEntry::Getter { key, .. }
                        | ObjectEntry::Setter { key, .. } => {
                            self.materialize_static_expression(key) != then_property
                        }
                        _ => false,
                    })
                    .then_some(Ok(value))
            }
            Expression::Array(_) => Some(Ok(value)),
            Expression::Identifier(_) | Expression::New { .. } => {
                let then_property = Expression::String("then".to_string());
                let object_binding = self.resolve_object_binding_from_expression(&value)?;
                if object_binding_lookup_value(&object_binding, &then_property).is_some()
                    || object_binding_lookup_descriptor(&object_binding, &then_property).is_some()
                {
                    return None;
                }
                Some(Ok(value))
            }
            _ => match self
                .resolve_static_await_resolution_outcome(&Expression::Await(Box::new(value)))?
            {
                StaticEvalOutcome::Value(value) => Some(Ok(value)),
                StaticEvalOutcome::Throw(throw_value) => {
                    Some(Err(self.resolve_static_throw_value_expression(&throw_value)?))
                }
            },
        }
    }

    fn for_await_protocol_property_key(
        &self,
        property: &Expression,
        context: &mut ForAwaitProtocolContext,
    ) -> Option<Result<Expression, Expression>> {
        if let Expression::String(_) = property {
            return Some(Ok(property.clone()));
        }
        let evaluated = match self.evaluate_for_await_protocol_expression(property, context)? {
            Ok(value) => value,
            Err(throw_value) => return Some(Err(throw_value)),
        };
        let key = self
            .resolve_property_key_expression(&evaluated)
            .unwrap_or(evaluated);
        matches!(key, Expression::String(_) | Expression::Number(_)).then_some(Ok(key))
    }

    fn for_await_protocol_member_value(
        &self,
        object: &Expression,
        property: &Expression,
    ) -> Option<Result<Expression, Expression>> {
        match object {
            Expression::Object(entries) => {
                for entry in entries {
                    match entry {
                        ObjectEntry::Data { key, value } => {
                            if &self.materialize_static_expression(key) == property {
                                // A member-shaped entry value is a deferred
                                // read (constructed for lazy step `value`
                                // resolution): resolve it now, dispatching
                                // accessors.
                                if let Expression::Member {
                                    object: deferred_object,
                                    property: deferred_property,
                                } = value
                                {
                                    let deferred_property = self
                                        .resolve_property_key_expression(deferred_property)
                                        .unwrap_or_else(|| deferred_property.as_ref().clone());
                                    return self.for_await_protocol_member_value(
                                        deferred_object,
                                        &deferred_property,
                                    );
                                }
                                return Some(Ok(value.clone()));
                            }
                        }
                        ObjectEntry::Getter { key, getter } => {
                            if &self.materialize_static_expression(key) == property {
                                let binding =
                                    self.resolve_function_binding_from_expression(getter)?;
                                return self.for_await_protocol_function_outcome(
                                    &binding,
                                    &[],
                                    object,
                                );
                            }
                        }
                        ObjectEntry::Setter { key, .. } => {
                            if &self.materialize_static_expression(key) == property {
                                return Some(Ok(Expression::Undefined));
                            }
                        }
                        _ => return None,
                    }
                }
                Some(Ok(Expression::Undefined))
            }
            Expression::Array(elements) => {
                if matches!(property, Expression::String(name) if name == "length") {
                    return Some(Ok(Expression::Number(elements.len() as f64)));
                }
                let index = argument_index_from_expression(property)? as usize;
                match elements.get(index) {
                    Some(ArrayElement::Expression(value)) => Some(Ok(value.clone())),
                    Some(ArrayElement::Spread(_)) => None,
                    None => Some(Ok(Expression::Undefined)),
                }
            }
            Expression::Null | Expression::Undefined => None,
            _ => {
                if let Some(getter_binding) = self.resolve_member_getter_binding(object, property)
                {
                    return self.for_await_protocol_function_outcome(
                        &getter_binding,
                        &[],
                        object,
                    );
                }
                let object_binding = self.resolve_object_binding_from_expression(object)?;
                if let Some(descriptor) =
                    object_binding_lookup_descriptor(&object_binding, property)
                {
                    if let Some(getter) = &descriptor.getter {
                        let binding = self.resolve_function_binding_from_expression(getter)?;
                        return self.for_await_protocol_function_outcome(&binding, &[], object);
                    }
                    if descriptor.has_get {
                        return None;
                    }
                    return Some(Ok(descriptor
                        .value
                        .clone()
                        .unwrap_or(Expression::Undefined)));
                }
                Some(Ok(object_binding_lookup_value(&object_binding, property)
                    .cloned()
                    .unwrap_or(Expression::Undefined)))
            }
        }
    }

    fn for_await_protocol_identifier_resolves(&self, name: &str) -> bool {
        if crate::ayy_env_flag!("AYY_TRACE_FOR_AWAIT_PROTOCOL") {
            eprintln!(
                "for_await_protocol:identifier_resolves name={name} local={} global={} lexical={} fn={} user={} by_binding={} builtin={}",
                self.resolve_current_local_binding(name).is_some(),
                self.backend.global_has_binding(name),
                self.backend.global_has_lexical_binding(name),
                self.backend.global_function_binding(name).is_some(),
                self.contains_user_function(name),
                self.resolve_user_function_by_binding_name(name).is_some(),
                self.is_unshadowed_builtin_identifier(name),
            );
        }
        if self.resolve_current_local_binding(name).is_some() {
            return false;
        }
        self.backend.global_has_binding(name)
            || self.backend.global_has_lexical_binding(name)
            || self.backend.global_function_binding(name).is_some()
            || self.contains_user_function(name)
            || self.resolve_user_function_by_binding_name(name).is_some()
            || (self.is_unshadowed_builtin_identifier(name)
                && (native_error_runtime_value(name).is_some()
                    || builtin_function_runtime_value(name).is_some()
                    || matches!(
                        name,
                        "undefined"
                            | "NaN"
                            | "Infinity"
                            | "Symbol"
                            | "Object"
                            | "Array"
                            | "Promise"
                            | "Math"
                            | "JSON"
                            | "Reflect"
                            | "Test262Error"
                    )))
    }

    /// Resolves a replay operand to a concrete number for arithmetic:
    /// identifiers materialize through the static binding state (stale reads
    /// of effect-mutated names were already rejected upstream by the
    /// identifier evaluation).
    fn for_await_protocol_number_value(&self, value: &Expression) -> Option<f64> {
        match value {
            Expression::Number(value) => Some(*value),
            Expression::Identifier(name) => {
                let materialized = self.materialize_static_expression(value);
                match materialized {
                    Expression::Number(value) => Some(value),
                    Expression::Identifier(materialized_name) if &materialized_name == name => {
                        None
                    }
                    materialized => self.for_await_protocol_number_value(&materialized),
                }
            }
            _ => None,
        }
    }

    fn for_await_protocol_to_boolean(value: &Expression) -> Option<bool> {
        match value {
            Expression::Bool(value) => Some(*value),
            Expression::Null | Expression::Undefined => Some(false),
            Expression::Number(value) => Some(*value != 0.0 && !value.is_nan()),
            Expression::String(value) => Some(!value.is_empty()),
            Expression::Object(_) | Expression::Array(_) | Expression::New { .. } => Some(true),
            _ => None,
        }
    }

    fn evaluate_for_await_protocol_binary(
        &self,
        op: BinaryOp,
        left: &Expression,
        right: &Expression,
        context: &mut ForAwaitProtocolContext,
    ) -> Option<Result<Expression, Expression>> {
        if matches!(op, BinaryOp::LogicalAnd | BinaryOp::LogicalOr) {
            let left = match self.evaluate_for_await_protocol_expression(left, context)? {
                Ok(value) => value,
                Err(throw_value) => return Some(Err(throw_value)),
            };
            let Expression::Bool(left) = left else {
                return None;
            };
            let short_circuit = match op {
                BinaryOp::LogicalAnd => !left,
                BinaryOp::LogicalOr => left,
                _ => unreachable!("filtered above"),
            };
            if short_circuit {
                return Some(Ok(Expression::Bool(left)));
            }
            let right = match self.evaluate_for_await_protocol_expression(right, context)? {
                Ok(value) => value,
                Err(throw_value) => return Some(Err(throw_value)),
            };
            let Expression::Bool(right) = right else {
                return None;
            };
            return Some(Ok(Expression::Bool(right)));
        }
        if matches!(
            op,
            BinaryOp::Add
                | BinaryOp::Subtract
                | BinaryOp::Multiply
                | BinaryOp::Divide
                | BinaryOp::Modulo
                | BinaryOp::LessThan
                | BinaryOp::LessThanOrEqual
                | BinaryOp::GreaterThan
                | BinaryOp::GreaterThanOrEqual
        ) {
            let left = match self.evaluate_for_await_protocol_expression(left, context)? {
                Ok(value) => value,
                Err(throw_value) => return Some(Err(throw_value)),
            };
            let right = match self.evaluate_for_await_protocol_expression(right, context)? {
                Ok(value) => value,
                Err(throw_value) => return Some(Err(throw_value)),
            };
            let left = self.for_await_protocol_number_value(&left)?;
            let right = self.for_await_protocol_number_value(&right)?;
            return Some(Ok(match op {
                BinaryOp::Add => Expression::Number(left + right),
                BinaryOp::Subtract => Expression::Number(left - right),
                BinaryOp::Multiply => Expression::Number(left * right),
                BinaryOp::Divide => Expression::Number(left / right),
                BinaryOp::Modulo => Expression::Number(left % right),
                BinaryOp::LessThan => Expression::Bool(left < right),
                BinaryOp::LessThanOrEqual => Expression::Bool(left <= right),
                BinaryOp::GreaterThan => Expression::Bool(left > right),
                BinaryOp::GreaterThanOrEqual => Expression::Bool(left >= right),
                _ => unreachable!("filtered above"),
            }));
        }
        if !matches!(
            op,
            BinaryOp::Equal
                | BinaryOp::NotEqual
                | BinaryOp::LooseEqual
                | BinaryOp::LooseNotEqual
        ) {
            return None;
        }
        let left = match self.evaluate_for_await_protocol_expression(left, context)? {
            Ok(value) => value,
            Err(throw_value) => return Some(Err(throw_value)),
        };
        let right = match self.evaluate_for_await_protocol_expression(right, context)? {
            Ok(value) => value,
            Err(throw_value) => return Some(Err(throw_value)),
        };
        let loose = matches!(op, BinaryOp::LooseEqual | BinaryOp::LooseNotEqual);
        let left = self.for_await_protocol_comparable_value(left);
        let right = self.for_await_protocol_comparable_value(right);
        let equal = Self::for_await_protocol_values_equal(&left, &right, loose)?;
        let result = match op {
            BinaryOp::Equal | BinaryOp::LooseEqual => equal,
            BinaryOp::NotEqual | BinaryOp::LooseNotEqual => !equal,
            _ => unreachable!("filtered above"),
        };
        Some(Ok(Expression::Bool(result)))
    }

    /// Resolves an identifier operand to a comparable shape: identifiers
    /// bound to static objects or functions compare like object values (never
    /// nullish), and identifiers that materialize to primitives compare as
    /// those primitives.
    fn for_await_protocol_comparable_value(&self, value: Expression) -> Expression {
        let Expression::Identifier(_) = &value else {
            return value;
        };
        if self.resolve_object_binding_from_expression(&value).is_some()
            || self.resolve_function_binding_from_expression(&value).is_some()
            // Generator objects are not plain object bindings but are still
            // objects (never nullish).
            || self
                .resolve_simple_generator_iterator_source_kind(&value)
                .is_some()
            || self
                .for_await_protocol_tracked_iterator_state(&value, &HashMap::new())
                .is_some()
        {
            return Expression::Object(Vec::new());
        }
        let materialized = self.materialize_static_expression(&value);
        if !static_expression_matches(&materialized, &value)
            && matches!(
                materialized,
                Expression::Number(_)
                    | Expression::BigInt(_)
                    | Expression::String(_)
                    | Expression::Bool(_)
                    | Expression::Null
                    | Expression::Undefined
            )
        {
            return materialized;
        }
        value
    }

    /// Equality over the value shapes the lowered destructure compares:
    /// nullish guards (`value == null`) and done-flag checks
    /// (`done == false`). Object-like values are never loosely equal to
    /// primitives in these positions.
    fn for_await_protocol_values_equal(
        left: &Expression,
        right: &Expression,
        loose: bool,
    ) -> Option<bool> {
        let nullish =
            |value: &Expression| matches!(value, Expression::Null | Expression::Undefined);
        if nullish(left) || nullish(right) {
            if nullish(left) && nullish(right) {
                if loose {
                    return Some(true);
                }
                return Some(match (left, right) {
                    (Expression::Null, Expression::Null)
                    | (Expression::Undefined, Expression::Undefined) => true,
                    _ => false,
                });
            }
            return match (left, right) {
                (
                    Expression::Object(_)
                    | Expression::Array(_)
                    | Expression::New { .. }
                    | Expression::Number(_)
                    | Expression::BigInt(_)
                    | Expression::String(_)
                    | Expression::Bool(_),
                    _,
                )
                | (
                    _,
                    Expression::Object(_)
                    | Expression::Array(_)
                    | Expression::New { .. }
                    | Expression::Number(_)
                    | Expression::BigInt(_)
                    | Expression::String(_)
                    | Expression::Bool(_),
                ) => Some(false),
                _ => None,
            };
        }
        match (left, right) {
            (Expression::Bool(lhs), Expression::Bool(rhs)) => Some(lhs == rhs),
            (Expression::Number(lhs), Expression::Number(rhs)) => Some(lhs == rhs),
            (Expression::String(lhs), Expression::String(rhs)) => Some(lhs == rhs),
            (Expression::Bool(_), Expression::Object(_) | Expression::Array(_))
            | (Expression::Object(_) | Expression::Array(_), Expression::Bool(_))
            | (Expression::Number(_), Expression::Object(_) | Expression::Array(_))
            | (Expression::Object(_) | Expression::Array(_), Expression::Number(_))
            | (Expression::String(_), Expression::Object(_) | Expression::Array(_))
            | (Expression::Object(_) | Expression::Array(_), Expression::String(_)) => None,
            (Expression::Bool(_), Expression::Number(_) | Expression::String(_))
            | (Expression::Number(_) | Expression::String(_), Expression::Bool(_))
            | (Expression::Number(_), Expression::String(_))
            | (Expression::String(_), Expression::Number(_)) => None,
            _ => None,
        }
    }
}
