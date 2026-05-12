use super::async_next::SimpleGeneratorNextEffectConsumption;
use super::*;

impl<'a> FunctionCompiler<'a> {
    fn collect_simple_generator_open_iterator_sources_from_expression(
        expression: &Expression,
        sources: &mut Vec<String>,
    ) {
        if let Expression::IteratorClose(value) = expression
            && let Expression::Identifier(name) = value.as_ref()
        {
            sources.retain(|source| source != name);
        }
        match expression {
            Expression::Assign { value, .. }
            | Expression::Await(value)
            | Expression::EnumerateKeys(value)
            | Expression::IteratorClose(value)
            | Expression::Unary {
                expression: value, ..
            } => {
                Self::collect_simple_generator_open_iterator_sources_from_expression(value, sources)
            }
            Expression::Member { object, property } => {
                Self::collect_simple_generator_open_iterator_sources_from_expression(
                    object, sources,
                );
                Self::collect_simple_generator_open_iterator_sources_from_expression(
                    property, sources,
                );
            }
            Expression::AssignMember {
                object,
                property,
                value,
            } => {
                Self::collect_simple_generator_open_iterator_sources_from_expression(
                    object, sources,
                );
                Self::collect_simple_generator_open_iterator_sources_from_expression(
                    property, sources,
                );
                Self::collect_simple_generator_open_iterator_sources_from_expression(
                    value, sources,
                );
            }
            Expression::AssignSuperMember { property, value } => {
                Self::collect_simple_generator_open_iterator_sources_from_expression(
                    property, sources,
                );
                Self::collect_simple_generator_open_iterator_sources_from_expression(
                    value, sources,
                );
            }
            Expression::Binary { left, right, .. } => {
                Self::collect_simple_generator_open_iterator_sources_from_expression(left, sources);
                Self::collect_simple_generator_open_iterator_sources_from_expression(
                    right, sources,
                );
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                Self::collect_simple_generator_open_iterator_sources_from_expression(
                    condition, sources,
                );
                Self::collect_simple_generator_open_iterator_sources_from_expression(
                    then_expression,
                    sources,
                );
                Self::collect_simple_generator_open_iterator_sources_from_expression(
                    else_expression,
                    sources,
                );
            }
            Expression::Call { callee, arguments }
            | Expression::SuperCall { callee, arguments }
            | Expression::New { callee, arguments } => {
                Self::collect_simple_generator_open_iterator_sources_from_expression(
                    callee, sources,
                );
                for argument in arguments {
                    match argument {
                        CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                            Self::collect_simple_generator_open_iterator_sources_from_expression(
                                expression, sources,
                            );
                        }
                    }
                }
            }
            Expression::Array(elements) => {
                for element in elements {
                    if let ArrayElement::Expression(expression) = element {
                        Self::collect_simple_generator_open_iterator_sources_from_expression(
                            expression, sources,
                        );
                    }
                }
            }
            Expression::Object(entries) => {
                for entry in entries {
                    match entry {
                        ObjectEntry::Data { key, value } => {
                            Self::collect_simple_generator_open_iterator_sources_from_expression(
                                key, sources,
                            );
                            Self::collect_simple_generator_open_iterator_sources_from_expression(
                                value, sources,
                            );
                        }
                        ObjectEntry::Getter { key, .. } | ObjectEntry::Setter { key, .. } => {
                            Self::collect_simple_generator_open_iterator_sources_from_expression(
                                key, sources,
                            );
                        }
                        ObjectEntry::Spread(value) => {
                            Self::collect_simple_generator_open_iterator_sources_from_expression(
                                value, sources,
                            );
                        }
                    }
                }
            }
            Expression::Sequence(expressions) => {
                for expression in expressions {
                    Self::collect_simple_generator_open_iterator_sources_from_expression(
                        expression, sources,
                    );
                }
            }
            Expression::GetIterator(_)
            | Expression::SuperMember { .. }
            | Expression::Number(_)
            | Expression::BigInt(_)
            | Expression::String(_)
            | Expression::Bool(_)
            | Expression::Null
            | Expression::Undefined
            | Expression::Identifier(_)
            | Expression::This
            | Expression::Sent
            | Expression::NewTarget
            | Expression::Update { .. } => {}
        }
    }

    fn collect_simple_generator_open_iterator_sources_from_statement(
        statement: &Statement,
        sources: &mut Vec<String>,
    ) {
        match statement {
            Statement::Let { name, value, .. }
            | Statement::Var { name, value }
            | Statement::Assign { name, value } => {
                if matches!(value, Expression::GetIterator(_)) && !sources.contains(name) {
                    sources.push(name.clone());
                }
                Self::collect_simple_generator_open_iterator_sources_from_expression(
                    value, sources,
                );
            }
            Statement::Expression(expression)
            | Statement::Throw(expression)
            | Statement::Yield { value: expression }
            | Statement::YieldDelegate { value: expression }
            | Statement::Return(expression) => {
                Self::collect_simple_generator_open_iterator_sources_from_expression(
                    expression, sources,
                );
            }
            Statement::Print { values } => {
                for expression in values {
                    Self::collect_simple_generator_open_iterator_sources_from_expression(
                        expression, sources,
                    );
                }
            }
            Statement::AssignMember {
                object,
                property,
                value,
            } => {
                Self::collect_simple_generator_open_iterator_sources_from_expression(
                    object, sources,
                );
                Self::collect_simple_generator_open_iterator_sources_from_expression(
                    property, sources,
                );
                Self::collect_simple_generator_open_iterator_sources_from_expression(
                    value, sources,
                );
            }
            Statement::If {
                condition,
                then_branch,
                else_branch,
            } => {
                Self::collect_simple_generator_open_iterator_sources_from_expression(
                    condition, sources,
                );
                for statement in then_branch.iter().chain(else_branch) {
                    Self::collect_simple_generator_open_iterator_sources_from_statement(
                        statement, sources,
                    );
                }
            }
            Statement::Block { body }
            | Statement::Declaration { body }
            | Statement::Labeled { body, .. }
            | Statement::With { body, .. }
            | Statement::While { body, .. }
            | Statement::DoWhile { body, .. } => {
                for statement in body {
                    Self::collect_simple_generator_open_iterator_sources_from_statement(
                        statement, sources,
                    );
                }
            }
            Statement::For { init, body, .. } => {
                for statement in init.iter().chain(body) {
                    Self::collect_simple_generator_open_iterator_sources_from_statement(
                        statement, sources,
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
                    Self::collect_simple_generator_open_iterator_sources_from_statement(
                        statement, sources,
                    );
                }
            }
            Statement::Switch { cases, .. } => {
                for case in cases {
                    for statement in &case.body {
                        Self::collect_simple_generator_open_iterator_sources_from_statement(
                            statement, sources,
                        );
                    }
                }
            }
            Statement::Break { .. } | Statement::Continue { .. } => {}
        }
    }

    fn simple_generator_open_iterator_sources_at_suspension(effects: &[Statement]) -> Vec<String> {
        let mut sources = Vec::new();
        for statement in effects {
            Self::collect_simple_generator_open_iterator_sources_from_statement(
                statement,
                &mut sources,
            );
        }
        sources
    }

    pub(super) fn collect_simple_generator_scoped_effect_bindings_from_statement(
        statement: &Statement,
        bindings: &mut Vec<(String, String)>,
    ) {
        match statement {
            Statement::Let { name, .. } | Statement::Var { name, .. } => {
                if let Some(source_name) = scoped_binding_source_name(name) {
                    bindings.push((source_name.to_string(), name.clone()));
                }
            }
            Statement::Block { body }
            | Statement::Declaration { body }
            | Statement::Labeled { body, .. }
            | Statement::With { body, .. }
            | Statement::While { body, .. }
            | Statement::DoWhile { body, .. } => {
                for statement in body {
                    Self::collect_simple_generator_scoped_effect_bindings_from_statement(
                        statement, bindings,
                    );
                }
            }
            Statement::If {
                then_branch,
                else_branch,
                ..
            } => {
                for statement in then_branch.iter().chain(else_branch) {
                    Self::collect_simple_generator_scoped_effect_bindings_from_statement(
                        statement, bindings,
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
                    Self::collect_simple_generator_scoped_effect_bindings_from_statement(
                        statement, bindings,
                    );
                }
            }
            Statement::Switch { cases, .. } => {
                for case in cases {
                    for statement in &case.body {
                        Self::collect_simple_generator_scoped_effect_bindings_from_statement(
                            statement, bindings,
                        );
                    }
                }
            }
            Statement::For { init, body, .. } => {
                for statement in init.iter().chain(body) {
                    Self::collect_simple_generator_scoped_effect_bindings_from_statement(
                        statement, bindings,
                    );
                }
            }
            Statement::Assign { .. }
            | Statement::AssignMember { .. }
            | Statement::Print { .. }
            | Statement::Expression(_)
            | Statement::Throw(_)
            | Statement::Return(_)
            | Statement::Yield { .. }
            | Statement::YieldDelegate { .. }
            | Statement::Break { .. }
            | Statement::Continue { .. } => {}
        }
    }

    pub(super) fn simple_generator_scoped_effect_bindings(
        effects: &[Statement],
    ) -> Vec<(String, String)> {
        let mut bindings = Vec::new();
        for statement in effects {
            Self::collect_simple_generator_scoped_effect_bindings_from_statement(
                statement,
                &mut bindings,
            );
        }
        bindings
    }

    pub(super) fn collect_simple_generator_scoped_var_bindings_from_statement(
        statement: &Statement,
        names: &mut Vec<String>,
    ) {
        match statement {
            Statement::Var { name, .. } => {
                if scoped_binding_source_name(name).is_some() && !names.contains(name) {
                    names.push(name.clone());
                }
            }
            Statement::Block { body }
            | Statement::Declaration { body }
            | Statement::Labeled { body, .. }
            | Statement::With { body, .. }
            | Statement::While { body, .. }
            | Statement::DoWhile { body, .. } => {
                for statement in body {
                    Self::collect_simple_generator_scoped_var_bindings_from_statement(
                        statement, names,
                    );
                }
            }
            Statement::If {
                then_branch,
                else_branch,
                ..
            } => {
                for statement in then_branch.iter().chain(else_branch) {
                    Self::collect_simple_generator_scoped_var_bindings_from_statement(
                        statement, names,
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
                    Self::collect_simple_generator_scoped_var_bindings_from_statement(
                        statement, names,
                    );
                }
            }
            Statement::Switch { cases, .. } => {
                for case in cases {
                    for statement in &case.body {
                        Self::collect_simple_generator_scoped_var_bindings_from_statement(
                            statement, names,
                        );
                    }
                }
            }
            Statement::For { init, body, .. } => {
                for statement in init.iter().chain(body) {
                    Self::collect_simple_generator_scoped_var_bindings_from_statement(
                        statement, names,
                    );
                }
            }
            Statement::Let { .. }
            | Statement::Assign { .. }
            | Statement::AssignMember { .. }
            | Statement::Print { .. }
            | Statement::Expression(_)
            | Statement::Throw(_)
            | Statement::Return(_)
            | Statement::Yield { .. }
            | Statement::YieldDelegate { .. }
            | Statement::Break { .. }
            | Statement::Continue { .. } => {}
        }
    }

    pub(super) fn collect_simple_generator_scoped_var_bindings(
        effects: &[Statement],
        names: &mut Vec<String>,
    ) {
        for statement in effects {
            Self::collect_simple_generator_scoped_var_bindings_from_statement(statement, names);
        }
    }

    fn initialize_simple_generator_start_bindings(
        &mut self,
        steps: &[SimpleGeneratorStep],
        completion_effects: &[Statement],
    ) -> DirectResult<()> {
        let first_dynamic_local = self.state.runtime.locals.next_local_index;
        let mut scoped_var_names = Vec::new();
        for step in steps {
            self.register_bindings(&step.effects)?;
            Self::collect_simple_generator_scoped_var_bindings(
                &step.effects,
                &mut scoped_var_names,
            );
            self.register_bindings(&step.close_effects)?;
            Self::collect_simple_generator_scoped_var_bindings(
                &step.close_effects,
                &mut scoped_var_names,
            );
        }
        self.register_bindings(completion_effects)?;
        Self::collect_simple_generator_scoped_var_bindings(
            completion_effects,
            &mut scoped_var_names,
        );

        let mut initialized_indices = self
            .state
            .runtime
            .locals
            .bindings
            .values()
            .copied()
            .filter(|local_index| *local_index >= first_dynamic_local)
            .collect::<Vec<_>>();
        for name in scoped_var_names {
            if let Some((_, local_index)) = self.resolve_current_local_binding(&name)
                && !initialized_indices.contains(&local_index)
            {
                initialized_indices.push(local_index);
            }
        }
        initialized_indices.sort_unstable();
        initialized_indices.dedup();
        for local_index in initialized_indices {
            self.push_i32_const(JS_UNDEFINED_TAG);
            self.push_local_set(local_index);
        }
        Ok(())
    }

    fn emit_static_simple_generator_effects_in_eval_scope(
        &mut self,
        effects: &[Statement],
        strict_mode: bool,
    ) -> DirectResult<Option<StaticThrowValue>> {
        self.with_active_eval_lexical_scope(
            collect_direct_eval_lexical_binding_names(effects),
            |compiler| {
                let scoped_bindings = Self::simple_generator_scoped_effect_bindings(effects);
                for (source_name, scoped_name) in &scoped_bindings {
                    compiler
                        .state
                        .push_scoped_lexical_binding(source_name, scoped_name.clone());
                }
                let scoped_source_names = scoped_bindings
                    .iter()
                    .map(|(source_name, _)| source_name.clone())
                    .collect::<Vec<_>>();
                compiler.with_scoped_lexical_bindings_cleanup(scoped_source_names, |compiler| {
                    let mut prior_effects = Vec::new();
                    for effect in effects {
                        match compiler.consume_throwing_simple_generator_next_effect_with_prior(
                            effect,
                            &prior_effects,
                            strict_mode,
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
                                if compiler
                                    .try_emit_static_simple_generator_member_assignment_effect(
                                        effect,
                                        &prior_effects,
                                    )?
                                {
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
                })
            },
        )
    }

    pub(in crate::backend::direct_wasm) fn emit_fresh_simple_generator_return_call(
        &mut self,
        object: &Expression,
        arguments: &[CallArgument],
    ) -> DirectResult<bool> {
        let trace_return = std::env::var_os("AYY_TRACE_SIMPLE_GENERATOR_RETURN").is_some();
        if trace_return {
            eprintln!("simple_generator_return:start object={object:?} args={arguments:?}");
        }
        let Expression::Identifier(object_name) = object else {
            return Ok(false);
        };
        if trace_return {
            eprintln!("simple_generator_return:object_name={object_name}");
        }
        let binding_name = self
            .resolve_local_array_iterator_binding_name(object_name)
            .unwrap_or_else(|| object_name.clone());
        if trace_return {
            eprintln!("simple_generator_return:binding_name={binding_name}");
        }
        let Some(iterator_binding) = self
            .state
            .speculation
            .static_semantics
            .local_array_iterator_binding(&binding_name)
        else {
            if trace_return {
                eprintln!("simple_generator_return:no_iterator_binding");
            }
            return Ok(false);
        };
        if trace_return {
            eprintln!("simple_generator_return:binding_found");
        }
        let IteratorSourceKind::SimpleGenerator { steps, .. } = &iterator_binding.source else {
            return Ok(false);
        };
        if trace_return {
            eprintln!("simple_generator_return:source_steps={}", steps.len());
        }
        let current_index = iterator_binding.static_index.unwrap_or(0);
        let index_local = iterator_binding.index_local;
        let closed_index = steps.len().saturating_add(1);
        let step = current_index
            .checked_sub(1)
            .and_then(|index| steps.get(index))
            .map(|step| (step.effects.clone(), step.close_effects.clone()));
        if trace_return {
            eprintln!("simple_generator_return:current_index={current_index}");
        }
        if current_index == 0 {
            return Ok(false);
        }

        if trace_return {
            eprintln!("simple_generator_return:sent_value:start");
        }
        let sent_value = arguments
            .first()
            .map(|argument| match argument {
                CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                    self.materialize_static_expression(expression)
                }
            })
            .unwrap_or(Expression::Undefined);
        if trace_return {
            eprintln!("simple_generator_return:sent_value={sent_value:?}");
        }
        let call_expression = Expression::Call {
            callee: Box::new(Expression::Member {
                object: Box::new(object.clone()),
                property: Box::new(Expression::String("return".to_string())),
            }),
            arguments: arguments.to_vec(),
        };
        if let Some((step_effects, step_close_effects)) = step {
            if trace_return {
                eprintln!(
                    "simple_generator_return:step current_index={current_index} effects={} close_effects={}",
                    step_effects.len(),
                    step_close_effects.len()
                );
            }
            let mut sources =
                Self::simple_generator_open_iterator_sources_at_suspension(&step_effects);
            sources.reverse();
            for source in sources {
                if trace_return {
                    eprintln!("simple_generator_return:close_source source={source}");
                }
                let source_expression = Expression::Identifier(source);
                if trace_return {
                    eprintln!("simple_generator_return:resolve_close_target:start");
                }
                let close_target = self
                    .resolve_static_iterator_close_target(&source_expression, &step_effects)
                    .unwrap_or(source_expression);
                if trace_return {
                    eprintln!(
                        "simple_generator_return:resolve_close_target:done target={close_target:?}"
                    );
                }
                self.emit_numeric_expression(&Expression::IteratorClose(Box::new(close_target)))?;
                if trace_return {
                    eprintln!("simple_generator_return:iterator_close:done");
                }
                self.state.emission.output.instructions.push(0x1a);
            }

            if !step_close_effects.is_empty() {
                if trace_return {
                    eprintln!("simple_generator_return:close_effects:start");
                }
                let substituted_close_effects = step_close_effects
                    .iter()
                    .map(|effect| Self::substitute_sent_statement(effect, &sent_value))
                    .collect::<Vec<_>>();
                let substituted_close_effects = self
                    .expand_static_lowered_for_of_completion_effects(&substituted_close_effects);
                self.register_bindings(&substituted_close_effects)?;
                if let Some(throw_value) = self.emit_static_simple_generator_effects_in_eval_scope(
                    &substituted_close_effects,
                    self.state.speculation.execution_context.strict_mode,
                )? {
                    self.set_static_iterator_index_for_index_local(index_local, closed_index);
                    self.push_i32_const(closed_index as i32);
                    self.push_local_set(index_local);
                    self.state
                        .speculation
                        .static_semantics
                        .last_bound_user_function_call = Some(BoundUserFunctionCallSnapshot {
                        function_name: "__ayy_simple_generator_return".to_string(),
                        source_expression: Some(call_expression.clone()),
                        result_expression: None,
                        prototype_source_expression: None,
                        updated_bindings: HashMap::new(),
                    });
                    self.emit_static_throw_value(&throw_value)?;
                    return Ok(true);
                }
                if trace_return {
                    eprintln!("simple_generator_return:close_effects:done");
                }
            }
        }

        if trace_return {
            eprintln!("simple_generator_return:finish closed_index={closed_index}");
        }
        self.set_static_iterator_index_for_index_local(index_local, closed_index);
        self.push_i32_const(closed_index as i32);
        self.push_local_set(index_local);

        let result_expression = Expression::Object(vec![
            ObjectEntry::Data {
                key: Expression::String("done".to_string()),
                value: Expression::Bool(true),
            },
            ObjectEntry::Data {
                key: Expression::String("value".to_string()),
                value: sent_value,
            },
        ]);
        self.state
            .speculation
            .static_semantics
            .last_bound_user_function_call = Some(BoundUserFunctionCallSnapshot {
            function_name: "__ayy_simple_generator_return".to_string(),
            source_expression: Some(call_expression),
            result_expression: Some(result_expression),
            prototype_source_expression: None,
            updated_bindings: HashMap::new(),
        });
        self.push_i32_const(JS_TYPEOF_OBJECT_TAG);
        Ok(true)
    }

    pub(in crate::backend::direct_wasm) fn emit_fresh_simple_generator_next_call(
        &mut self,
        object: &Expression,
        arguments: &[CallArgument],
    ) -> DirectResult<bool> {
        let iter_result_object = |done: bool, value: Expression| {
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
        };
        let call_expression = Expression::Call {
            callee: Box::new(Expression::Member {
                object: Box::new(object.clone()),
                property: Box::new(Expression::String("next".to_string())),
            }),
            arguments: arguments.to_vec(),
        };
        if !self.state.emission.control_flow.loop_stack.is_empty() {
            return Ok(false);
        }
        if let Some(outcome) =
            self.consume_simple_async_generator_next_promise_outcome(object, arguments)?
        {
            let promise_reject_expression = |value: Expression| Expression::Call {
                callee: Box::new(Expression::Member {
                    object: Box::new(Expression::Identifier("Promise".to_string())),
                    property: Box::new(Expression::String("reject".to_string())),
                }),
                arguments: vec![CallArgument::Expression(value)],
            };
            let result_expression = match &outcome {
                StaticEvalOutcome::Value(value) => Some(value.clone()),
                StaticEvalOutcome::Throw(throw_value) => self
                    .resolve_static_throw_value_expression(throw_value)
                    .map(promise_reject_expression),
            };
            self.state
                .speculation
                .static_semantics
                .last_bound_user_function_call = Some(BoundUserFunctionCallSnapshot {
                function_name: "__ayy_simple_async_generator_next".to_string(),
                source_expression: Some(call_expression),
                result_expression,
                prototype_source_expression: None,
                updated_bindings: HashMap::new(),
            });
            self.push_i32_const(JS_TYPEOF_OBJECT_TAG);
            return Ok(true);
        }
        self.emit_simple_generator_call_time_prefix_effects(object)?;
        let Some((steps, completion_effects, completion_value)) = self
            .simple_generator_source_metadata(object)
            .map(|(_, steps, completion_effects, completion_value)| {
                (steps, completion_effects, completion_value)
            })
            .or_else(|| self.resolve_simple_generator_source(object))
            .or_else(|| self.resolve_array_prototype_simple_generator_source(object))
        else {
            return Ok(false);
        };
        let binding_name = if let Expression::Identifier(object_name) = object {
            let binding_name = self
                .resolve_local_array_iterator_binding_name(object_name)
                .unwrap_or_else(|| object_name.clone());
            let Some(_) = self
                .state
                .speculation
                .static_semantics
                .local_array_iterator_binding(&binding_name)
                .and_then(|binding| binding.static_index)
            else {
                if std::env::var_os("AYY_TRACE_SIMPLE_GENERATORS").is_some() {
                    eprintln!(
                        "simple_next_call:no-static-index object={object:?} binding={binding_name}"
                    );
                }
                return Ok(false);
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
                "simple_next_call object={object:?} binding={binding_name:?} current_index={current_index}"
            );
        }
        if current_index == 0 {
            self.initialize_simple_generator_start_bindings(&steps, &completion_effects)?;
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
                compiler.push_i32_const(next_index as i32);
                compiler.push_local_set(index_local);
            }
        };
        let sent_value = arguments
            .first()
            .map(|argument| match argument {
                CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                    self.materialize_static_expression(expression)
                }
            })
            .unwrap_or(Expression::Undefined);

        if binding_name.is_some() {
            self.emit_numeric_expression(object)?;
            self.state.emission.output.instructions.push(0x1a);
        }

        if let Some(step) = steps.get(current_index) {
            let substituted_effects = step
                .effects
                .iter()
                .map(|effect| Self::substitute_sent_statement(effect, &sent_value))
                .collect::<Vec<_>>();
            let substituted_effects =
                self.expand_static_lowered_for_of_completion_effects(&substituted_effects);
            self.register_bindings(&substituted_effects)?;
            if let Some(throw_value) = self.emit_static_simple_generator_effects_in_eval_scope(
                &substituted_effects,
                self.state.speculation.execution_context.strict_mode,
            )? {
                set_binding_index(self, steps.len().saturating_add(1));
                self.state
                    .speculation
                    .static_semantics
                    .last_bound_user_function_call = Some(BoundUserFunctionCallSnapshot {
                    function_name: "__ayy_simple_generator_next".to_string(),
                    source_expression: Some(call_expression.clone()),
                    result_expression: None,
                    prototype_source_expression: None,
                    updated_bindings: HashMap::new(),
                });
                self.emit_static_throw_value(&throw_value)?;
                return Ok(true);
            }
            match &step.outcome {
                SimpleGeneratorStepOutcome::Yield(value) => {
                    set_binding_index(self, current_index.saturating_add(1));
                    let yielded_value = Self::substitute_sent_expression(value, &sent_value);
                    self.state
                        .speculation
                        .static_semantics
                        .last_bound_user_function_call = Some(BoundUserFunctionCallSnapshot {
                        function_name: "__ayy_simple_generator_next".to_string(),
                        source_expression: Some(call_expression.clone()),
                        result_expression: Some(iter_result_object(false, yielded_value)),
                        prototype_source_expression: None,
                        updated_bindings: HashMap::new(),
                    });
                    self.push_i32_const(JS_TYPEOF_OBJECT_TAG);
                    Ok(true)
                }
                SimpleGeneratorStepOutcome::Throw(value) => {
                    set_binding_index(self, steps.len().saturating_add(1));
                    self.state
                        .speculation
                        .static_semantics
                        .last_bound_user_function_call = Some(BoundUserFunctionCallSnapshot {
                        function_name: "__ayy_simple_generator_next".to_string(),
                        source_expression: Some(call_expression.clone()),
                        result_expression: None,
                        prototype_source_expression: None,
                        updated_bindings: HashMap::new(),
                    });
                    self.emit_statement(&Statement::Throw(value.clone()))?;
                    Ok(true)
                }
            }
        } else {
            let completion_result_expression = if current_index == steps.len() {
                iter_result_object(true, self.materialize_static_expression(&completion_value))
            } else {
                iter_result_object(true, Expression::Undefined)
            };
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
                    .expand_static_lowered_for_of_completion_effects(
                        &substituted_completion_effects,
                    );
                self.register_bindings(&substituted_completion_effects)?;
                if let Some(throw_value) = self.emit_static_simple_generator_effects_in_eval_scope(
                    &substituted_completion_effects,
                    self.state.speculation.execution_context.strict_mode,
                )? {
                    set_binding_index(self, steps.len().saturating_add(1));
                    self.state
                        .speculation
                        .static_semantics
                        .last_bound_user_function_call = Some(BoundUserFunctionCallSnapshot {
                        function_name: "__ayy_simple_generator_next".to_string(),
                        source_expression: Some(call_expression.clone()),
                        result_expression: None,
                        prototype_source_expression: None,
                        updated_bindings: HashMap::new(),
                    });
                    self.emit_static_throw_value(&throw_value)?;
                    return Ok(true);
                }
            }
            self.state
                .speculation
                .static_semantics
                .last_bound_user_function_call = Some(BoundUserFunctionCallSnapshot {
                function_name: "__ayy_simple_generator_next".to_string(),
                source_expression: Some(call_expression),
                result_expression: Some(completion_result_expression),
                prototype_source_expression: None,
                updated_bindings: HashMap::new(),
            });
            self.push_i32_const(JS_TYPEOF_OBJECT_TAG);
            Ok(true)
        }
    }
}
