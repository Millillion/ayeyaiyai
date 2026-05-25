use super::async_next::SimpleGeneratorNextEffectConsumption;
use super::*;

impl<'a> FunctionCompiler<'a> {
    pub(in crate::backend::direct_wasm) fn resolve_simple_generator_result_value_with_context(
        &self,
        value: &Expression,
        current_function_name: Option<&str>,
    ) -> Expression {
        match value {
            Expression::SuperMember { property } => self
                .resolve_static_super_member_value_with_context(
                    property,
                    current_function_name,
                    &Expression::This,
                )
                .unwrap_or_else(|| self.materialize_static_expression(value)),
            _ => self
                .resolve_static_primitive_expression_with_context(value, current_function_name)
                .unwrap_or_else(|| self.materialize_static_expression(value)),
        }
    }

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

    fn first_iterator_close_expression_in_expression(
        expression: &Expression,
    ) -> Option<Expression> {
        if let Expression::IteratorClose(value) = expression {
            return Some(value.as_ref().clone());
        }
        match expression {
            Expression::Assign { value, .. }
            | Expression::Await(value)
            | Expression::EnumerateKeys(value)
            | Expression::Unary {
                expression: value, ..
            } => Self::first_iterator_close_expression_in_expression(value),
            Expression::Member { object, property } => {
                Self::first_iterator_close_expression_in_expression(object)
                    .or_else(|| Self::first_iterator_close_expression_in_expression(property))
            }
            Expression::AssignMember {
                object,
                property,
                value,
            } => Self::first_iterator_close_expression_in_expression(object)
                .or_else(|| Self::first_iterator_close_expression_in_expression(property))
                .or_else(|| Self::first_iterator_close_expression_in_expression(value)),
            Expression::AssignSuperMember { property, value } => {
                Self::first_iterator_close_expression_in_expression(property)
                    .or_else(|| Self::first_iterator_close_expression_in_expression(value))
            }
            Expression::Binary { left, right, .. } => {
                Self::first_iterator_close_expression_in_expression(left)
                    .or_else(|| Self::first_iterator_close_expression_in_expression(right))
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => Self::first_iterator_close_expression_in_expression(condition)
                .or_else(|| Self::first_iterator_close_expression_in_expression(then_expression))
                .or_else(|| Self::first_iterator_close_expression_in_expression(else_expression)),
            Expression::Call { callee, arguments }
            | Expression::SuperCall { callee, arguments }
            | Expression::New { callee, arguments } => {
                Self::first_iterator_close_expression_in_expression(callee).or_else(|| {
                    arguments.iter().find_map(|argument| match argument {
                        CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                            Self::first_iterator_close_expression_in_expression(expression)
                        }
                    })
                })
            }
            Expression::Array(elements) => elements.iter().find_map(|element| match element {
                ArrayElement::Expression(expression) => {
                    Self::first_iterator_close_expression_in_expression(expression)
                }
                ArrayElement::Spread(expression) => {
                    Self::first_iterator_close_expression_in_expression(expression)
                }
            }),
            Expression::Object(entries) => entries.iter().find_map(|entry| match entry {
                ObjectEntry::Data { key, value } => {
                    Self::first_iterator_close_expression_in_expression(key)
                        .or_else(|| Self::first_iterator_close_expression_in_expression(value))
                }
                ObjectEntry::Getter { key, .. } | ObjectEntry::Setter { key, .. } => {
                    Self::first_iterator_close_expression_in_expression(key)
                }
                ObjectEntry::Spread(value) => {
                    Self::first_iterator_close_expression_in_expression(value)
                }
            }),
            Expression::Sequence(expressions) => expressions
                .iter()
                .find_map(Self::first_iterator_close_expression_in_expression),
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
            | Expression::Update { .. }
            | Expression::IteratorClose(_) => None,
        }
    }

    fn first_iterator_close_expression_in_statement(statement: &Statement) -> Option<Expression> {
        match statement {
            Statement::Let { value, .. }
            | Statement::Var { value, .. }
            | Statement::Assign { value, .. }
            | Statement::Expression(value)
            | Statement::Throw(value)
            | Statement::Yield { value }
            | Statement::YieldDelegate { value }
            | Statement::Return(value) => {
                Self::first_iterator_close_expression_in_expression(value)
            }
            Statement::Print { values } => values
                .iter()
                .find_map(Self::first_iterator_close_expression_in_expression),
            Statement::AssignMember {
                object,
                property,
                value,
            } => Self::first_iterator_close_expression_in_expression(object)
                .or_else(|| Self::first_iterator_close_expression_in_expression(property))
                .or_else(|| Self::first_iterator_close_expression_in_expression(value)),
            Statement::If {
                condition,
                then_branch,
                else_branch,
            } => Self::first_iterator_close_expression_in_expression(condition)
                .or_else(|| Self::first_iterator_close_expression_in_statements(then_branch))
                .or_else(|| Self::first_iterator_close_expression_in_statements(else_branch)),
            Statement::Block { body }
            | Statement::Declaration { body }
            | Statement::Labeled { body, .. }
            | Statement::With { body, .. }
            | Statement::While { body, .. }
            | Statement::DoWhile { body, .. } => {
                Self::first_iterator_close_expression_in_statements(body)
            }
            Statement::For { init, body, .. } => {
                Self::first_iterator_close_expression_in_statements(init)
                    .or_else(|| Self::first_iterator_close_expression_in_statements(body))
            }
            Statement::Try {
                body,
                catch_setup,
                catch_body,
                ..
            } => Self::first_iterator_close_expression_in_statements(body)
                .or_else(|| Self::first_iterator_close_expression_in_statements(catch_setup))
                .or_else(|| Self::first_iterator_close_expression_in_statements(catch_body)),
            Statement::Switch { cases, .. } => cases
                .iter()
                .find_map(|case| Self::first_iterator_close_expression_in_statements(&case.body)),
            Statement::Break { .. } | Statement::Continue { .. } => None,
        }
    }

    fn first_iterator_close_expression_in_statements(
        statements: &[Statement],
    ) -> Option<Expression> {
        statements
            .iter()
            .find_map(Self::first_iterator_close_expression_in_statement)
    }

    fn replace_first_iterator_close_statement(
        statement: &Statement,
        replacement: &Statement,
        replaced: &mut bool,
    ) -> Statement {
        if !*replaced && let Statement::Expression(Expression::IteratorClose(_)) = statement {
            *replaced = true;
            return replacement.clone();
        }

        match statement {
            Statement::Block { body } => Statement::Block {
                body: Self::replace_first_iterator_close_statements(body, replacement, replaced),
            },
            Statement::Declaration { body } => Statement::Declaration {
                body: Self::replace_first_iterator_close_statements(body, replacement, replaced),
            },
            Statement::Labeled { labels, body } => Statement::Labeled {
                labels: labels.clone(),
                body: Self::replace_first_iterator_close_statements(body, replacement, replaced),
            },
            Statement::With { object, body } => Statement::With {
                object: object.clone(),
                body: Self::replace_first_iterator_close_statements(body, replacement, replaced),
            },
            Statement::While {
                labels,
                condition,
                break_hook,
                body,
            } => Statement::While {
                labels: labels.clone(),
                condition: condition.clone(),
                break_hook: break_hook.clone(),
                body: Self::replace_first_iterator_close_statements(body, replacement, replaced),
            },
            Statement::DoWhile {
                labels,
                condition,
                break_hook,
                body,
            } => Statement::DoWhile {
                labels: labels.clone(),
                body: Self::replace_first_iterator_close_statements(body, replacement, replaced),
                condition: condition.clone(),
                break_hook: break_hook.clone(),
            },
            Statement::For {
                labels,
                init,
                per_iteration_bindings,
                condition,
                update,
                break_hook,
                body,
            } => Statement::For {
                labels: labels.clone(),
                init: Self::replace_first_iterator_close_statements(init, replacement, replaced),
                per_iteration_bindings: per_iteration_bindings.clone(),
                condition: condition.clone(),
                update: update.clone(),
                break_hook: break_hook.clone(),
                body: Self::replace_first_iterator_close_statements(body, replacement, replaced),
            },
            Statement::If {
                condition,
                then_branch,
                else_branch,
            } => Statement::If {
                condition: condition.clone(),
                then_branch: Self::replace_first_iterator_close_statements(
                    then_branch,
                    replacement,
                    replaced,
                ),
                else_branch: Self::replace_first_iterator_close_statements(
                    else_branch,
                    replacement,
                    replaced,
                ),
            },
            Statement::Try {
                body,
                catch_binding,
                catch_setup,
                catch_body,
            } => Statement::Try {
                body: Self::replace_first_iterator_close_statements(body, replacement, replaced),
                catch_binding: catch_binding.clone(),
                catch_setup: Self::replace_first_iterator_close_statements(
                    catch_setup,
                    replacement,
                    replaced,
                ),
                catch_body: Self::replace_first_iterator_close_statements(
                    catch_body,
                    replacement,
                    replaced,
                ),
            },
            Statement::Switch {
                labels,
                bindings,
                discriminant,
                cases,
            } => Statement::Switch {
                labels: labels.clone(),
                bindings: bindings.clone(),
                discriminant: discriminant.clone(),
                cases: cases
                    .iter()
                    .map(|case| crate::ir::hir::SwitchCase {
                        test: case.test.clone(),
                        body: Self::replace_first_iterator_close_statements(
                            &case.body,
                            replacement,
                            replaced,
                        ),
                    })
                    .collect(),
            },
            Statement::Let { .. }
            | Statement::Var { .. }
            | Statement::Assign { .. }
            | Statement::AssignMember { .. }
            | Statement::Print { .. }
            | Statement::Expression(_)
            | Statement::Throw(_)
            | Statement::Return(_)
            | Statement::Yield { .. }
            | Statement::YieldDelegate { .. }
            | Statement::Break { .. }
            | Statement::Continue { .. } => statement.clone(),
        }
    }

    fn replace_first_iterator_close_statements(
        statements: &[Statement],
        replacement: &Statement,
        replaced: &mut bool,
    ) -> Vec<Statement> {
        statements
            .iter()
            .map(|statement| {
                Self::replace_first_iterator_close_statement(statement, replacement, replaced)
            })
            .collect()
    }

    fn close_effects_with_first_iterator_close_replaced(
        statements: &[Statement],
        replacement: Statement,
    ) -> Option<Vec<Statement>> {
        let mut replaced = false;
        let statements =
            Self::replace_first_iterator_close_statements(statements, &replacement, &mut replaced);
        replaced.then_some(statements)
    }

    fn resolve_static_user_function_return_outcome_after_prefix_effects(
        &self,
        user_function: &UserFunction,
        body: &[Statement],
        arguments: &[CallArgument],
        this_binding: &Expression,
    ) -> Option<StaticEvalOutcome> {
        if user_function.has_parameter_defaults() {
            return None;
        }
        let arguments_binding = Expression::Array(
            arguments
                .iter()
                .map(|argument| match argument {
                    CallArgument::Expression(expression) => {
                        ArrayElement::Expression(expression.clone())
                    }
                    CallArgument::Spread(expression) => ArrayElement::Spread(expression.clone()),
                })
                .collect(),
        );
        for statement in body {
            match statement {
                Statement::Return(expression) => {
                    let value = self.substitute_user_function_call_frame_bindings(
                        expression,
                        user_function,
                        arguments,
                        this_binding,
                        &arguments_binding,
                    );
                    return Some(StaticEvalOutcome::Value(
                        self.resolve_static_super_members_in_call_frame_return(
                            &value,
                            &user_function.name,
                            this_binding,
                        ),
                    ));
                }
                Statement::Throw(expression) => {
                    let value = self.substitute_user_function_call_frame_bindings(
                        expression,
                        user_function,
                        arguments,
                        this_binding,
                        &arguments_binding,
                    );
                    return Some(StaticEvalOutcome::Throw(StaticThrowValue::Value(
                        self.resolve_static_super_members_in_call_frame_return(
                            &value,
                            &user_function.name,
                            this_binding,
                        ),
                    )));
                }
                Statement::Assign { .. }
                | Statement::AssignMember { .. }
                | Statement::Expression(_)
                | Statement::Let { .. }
                | Statement::Var { .. } => {}
                Statement::Print { .. }
                | Statement::Yield { .. }
                | Statement::YieldDelegate { .. }
                | Statement::Block { .. }
                | Statement::Declaration { .. }
                | Statement::Labeled { .. }
                | Statement::With { .. }
                | Statement::If { .. }
                | Statement::Switch { .. }
                | Statement::For { .. }
                | Statement::While { .. }
                | Statement::DoWhile { .. }
                | Statement::Try { .. }
                | Statement::Break { .. }
                | Statement::Continue { .. } => return None,
            }
        }
        None
    }

    fn try_emit_yield_delegate_return_close(
        &mut self,
        step_close_effects: &[Statement],
        sent_value: &Expression,
        call_expression: &Expression,
        index_local: u32,
        closed_index: usize,
        strict_mode: bool,
    ) -> DirectResult<bool> {
        let trace_return = std::env::var_os("AYY_TRACE_SIMPLE_GENERATOR_RETURN").is_some();
        let Some(close_expression) =
            Self::first_iterator_close_expression_in_statements(step_close_effects)
        else {
            if trace_return {
                eprintln!("simple_generator_return:delegate_close:no_iterator_close");
            }
            return Ok(false);
        };
        let return_property = Expression::String("return".to_string());
        let close_target = self
            .resolve_static_iterator_close_target(&close_expression, &[])
            .unwrap_or_else(|| close_expression.clone());
        let return_arguments = vec![CallArgument::Expression(sent_value.clone())];
        let return_binding = self
            .resolve_member_function_binding(&close_target, &return_property)
            .or_else(|| self.resolve_member_function_binding(&close_expression, &return_property));
        if trace_return {
            eprintln!(
                "simple_generator_return:delegate_close target={close_target:?} binding={return_binding:?}"
            );
        }
        let Some(return_binding) = return_binding else {
            return Ok(false);
        };
        let return_outcome = self
            .resolve_static_function_outcome_from_binding_with_call_frame_and_context(
                &return_binding,
                &return_arguments,
                &close_target,
                self.current_function_name(),
            )
            .or_else(|| {
                self.resolve_static_function_outcome_from_binding_with_context(
                    &return_binding,
                    &return_arguments,
                    self.current_function_name(),
                )
            })
            .or_else(|| {
                let LocalFunctionBinding::User(function_name) = &return_binding else {
                    return None;
                };
                let user_function = self.user_function(function_name)?;
                let function = self.resolve_registered_function_declaration(function_name)?;
                self.resolve_static_user_function_return_outcome_after_prefix_effects(
                    user_function,
                    &function.body,
                    &return_arguments,
                    &close_target,
                )
            });
        let Some(StaticEvalOutcome::Value(return_value)) = return_outcome else {
            if trace_return {
                eprintln!("simple_generator_return:delegate_close:no_static_value");
            }
            return Ok(false);
        };
        let return_value = self.resolve_simple_generator_result_value_with_context(
            &return_value,
            self.current_function_name(),
        );
        let Some(return_object_binding) =
            self.resolve_object_binding_from_expression(&return_value)
        else {
            if trace_return {
                eprintln!(
                    "simple_generator_return:delegate_close:return_not_object value={return_value:?}"
                );
            }
            return Ok(false);
        };
        let done_property = Expression::String("done".to_string());
        if object_binding_lookup_descriptor(&return_object_binding, &done_property)
            .is_some_and(|descriptor| descriptor.getter.is_some() || descriptor.has_get)
        {
            if trace_return {
                eprintln!("simple_generator_return:delegate_close:done_accessor");
            }
            return Ok(false);
        }
        let done_member = Expression::Member {
            object: Box::new(return_value.clone()),
            property: Box::new(done_property),
        };
        let materialized_done_member = self.materialize_static_expression(&done_member);
        if trace_return
            && let Some(shadow_binding_name) = self
                .runtime_object_property_shadow_binding_name_for_expression(
                    &return_value,
                    &Expression::String("done".to_string()),
                )
        {
            eprintln!(
                "simple_generator_return:delegate_close:done_shadow name={shadow_binding_name} implicit={} value={:?} kind={:?} defer={}",
                self.global_has_implicit_binding(&shadow_binding_name),
                self.global_value_binding(&shadow_binding_name),
                self.global_binding_kind(&shadow_binding_name),
                self.runtime_object_property_shadow_binding_should_defer_static_resolution(
                    &shadow_binding_name
                )
            );
        }
        let static_shadow_done = self
            .runtime_object_property_shadow_binding_name_for_expression(
                &return_value,
                &Expression::String("done".to_string()),
            )
            .and_then(|shadow_binding_name| {
                self.global_value_binding(&shadow_binding_name)
                    .and_then(|value| self.resolve_static_boolean_expression(value))
            });
        let done = if let Some(done) =
            self.resolve_static_boolean_expression(&materialized_done_member)
        {
            done
        } else if let Some(done) = static_shadow_done {
            done
        } else if static_expression_matches(&materialized_done_member, &done_member) {
            if trace_return {
                eprintln!(
                    "simple_generator_return:delegate_close:runtime_done done={materialized_done_member:?}"
                );
            }
            return Ok(false);
        } else {
            let Some(Ok(done_expression)) = self.resolve_static_iterator_step_done_outcome(
                &return_value,
                &return_object_binding,
                &HashMap::new(),
                self.current_function_name(),
            ) else {
                if trace_return {
                    eprintln!("simple_generator_return:delegate_close:no_static_done");
                }
                return Ok(false);
            };
            let Some(done) = self.resolve_static_boolean_expression(&done_expression) else {
                if trace_return {
                    eprintln!(
                        "simple_generator_return:delegate_close:unknown_done done={done_expression:?}"
                    );
                }
                return Ok(false);
            };
            done
        };
        if trace_return {
            eprintln!(
                "simple_generator_return:delegate_close:done done={done} materialized={materialized_done_member:?}"
            );
        }
        let return_call = Expression::Call {
            callee: Box::new(Expression::Member {
                object: Box::new(close_target.clone()),
                property: Box::new(return_property.clone()),
            }),
            arguments: return_arguments.clone(),
        };
        let returned_value_member = Expression::Member {
            object: Box::new(return_value.clone()),
            property: Box::new(Expression::String("value".to_string())),
        };
        let completed_close_effects = if done {
            let Some(effects) = Self::close_effects_with_first_iterator_close_replaced(
                step_close_effects,
                Statement::Expression(returned_value_member.clone()),
            ) else {
                return Ok(false);
            };
            Some(effects)
        } else {
            None
        };
        match return_binding {
            LocalFunctionBinding::User(function_name) => {
                let Some(user_function) = self.user_function(&function_name).cloned() else {
                    return Ok(false);
                };
                let function_body = self
                    .resolve_registered_function_declaration(&function_name)
                    .map(|function| function.body.clone())
                    .unwrap_or_default();
                self.emit_user_function_call_with_function_this_binding(
                    &user_function,
                    &return_arguments,
                    &close_target,
                    None,
                )?;
                self.sync_static_iterator_close_arguments_assignments(
                    &user_function,
                    &[sent_value.clone()],
                    &function_body,
                );
            }
            LocalFunctionBinding::Builtin(_) => {
                self.emit_numeric_expression(&return_call)?;
            }
        }
        self.state.emission.output.instructions.push(0x1a);
        if let Some(completed_close_effects) = completed_close_effects {
            if trace_return {
                eprintln!("simple_generator_return:delegate_close:done_true");
            }
            if let Some(throw_value) = self.emit_static_simple_generator_effects_in_eval_scope(
                &completed_close_effects,
                strict_mode,
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
            self.set_static_iterator_index_for_index_local(index_local, closed_index);
            self.push_i32_const(closed_index as i32);
            self.push_local_set(index_local);
            let returned_value = self.resolve_simple_generator_result_value_with_context(
                &self.materialize_static_expression(&returned_value_member),
                self.current_function_name(),
            );
            self.state
                .speculation
                .static_semantics
                .last_bound_user_function_call = Some(BoundUserFunctionCallSnapshot {
                function_name: "__ayy_simple_generator_return".to_string(),
                source_expression: Some(call_expression.clone()),
                result_expression: Some(Expression::Object(vec![
                    ObjectEntry::Data {
                        key: Expression::String("done".to_string()),
                        value: Expression::Bool(true),
                    },
                    ObjectEntry::Data {
                        key: Expression::String("value".to_string()),
                        value: returned_value,
                    },
                ])),
                prototype_source_expression: None,
                updated_bindings: HashMap::new(),
            });
            self.push_i32_const(JS_TYPEOF_OBJECT_TAG);
            return Ok(true);
        }

        self.state
            .speculation
            .static_semantics
            .last_bound_user_function_call = Some(BoundUserFunctionCallSnapshot {
            function_name: "__ayy_simple_generator_return".to_string(),
            source_expression: Some(call_expression.clone()),
            result_expression: Some(self.materialize_static_expression(&return_value)),
            prototype_source_expression: None,
            updated_bindings: HashMap::new(),
        });
        if trace_return {
            eprintln!("simple_generator_return:delegate_close:done_false_suspended");
        }
        self.push_i32_const(JS_TYPEOF_OBJECT_TAG);
        Ok(true)
    }

    fn emit_yield_delegate_throw_completion_after_replacement(
        &mut self,
        step_close_effects: &[Statement],
        replacement: Statement,
        result_value: Expression,
        completion_effects: &[Statement],
        sent_value: &Expression,
        source_function_name: Option<&str>,
        call_expression: &Expression,
        index_local: u32,
        closed_index: usize,
        strict_mode: bool,
    ) -> DirectResult<bool> {
        let Some(completed_effects) =
            Self::close_effects_with_first_iterator_close_replaced(step_close_effects, replacement)
        else {
            return Ok(false);
        };
        if let Some(throw_value) = self
            .emit_static_simple_generator_effects_in_eval_scope(&completed_effects, strict_mode)?
        {
            self.set_static_iterator_index_for_index_local(index_local, closed_index);
            self.push_i32_const(closed_index as i32);
            self.push_local_set(index_local);
            self.state
                .speculation
                .static_semantics
                .last_bound_user_function_call = Some(BoundUserFunctionCallSnapshot {
                function_name: "__ayy_simple_generator_throw".to_string(),
                source_expression: Some(call_expression.clone()),
                result_expression: None,
                prototype_source_expression: None,
                updated_bindings: HashMap::new(),
            });
            self.emit_static_throw_value(&throw_value)?;
            return Ok(true);
        }

        self.set_static_iterator_index_for_index_local(index_local, closed_index);
        self.push_i32_const(closed_index as i32);
        self.push_local_set(index_local);
        let resolved_completion_value = self.resolve_simple_generator_result_value_with_context(
            &self.materialize_static_expression(&result_value),
            source_function_name,
        );
        let substituted_completion_effects = completion_effects
            .iter()
            .map(|effect| Self::substitute_sent_statement(effect, sent_value))
            .collect::<Vec<_>>();
        let substituted_completion_effects =
            self.expand_static_lowered_for_of_completion_effects(&substituted_completion_effects);
        let substituted_completion_effects = Self::apply_yield_result_completion_value_to_effects(
            substituted_completion_effects,
            &resolved_completion_value,
        );
        self.register_bindings(&substituted_completion_effects)?;
        if let Some(throw_value) = self.emit_static_simple_generator_effects_in_eval_scope(
            &substituted_completion_effects,
            strict_mode,
        )? {
            self.state
                .speculation
                .static_semantics
                .last_bound_user_function_call = Some(BoundUserFunctionCallSnapshot {
                function_name: "__ayy_simple_generator_throw".to_string(),
                source_expression: Some(call_expression.clone()),
                result_expression: None,
                prototype_source_expression: None,
                updated_bindings: HashMap::new(),
            });
            self.emit_static_throw_value(&throw_value)?;
            return Ok(true);
        }
        self.state
            .speculation
            .static_semantics
            .last_bound_user_function_call = Some(BoundUserFunctionCallSnapshot {
            function_name: "__ayy_simple_generator_throw".to_string(),
            source_expression: Some(call_expression.clone()),
            result_expression: Some(Expression::Object(vec![
                ObjectEntry::Data {
                    key: Expression::String("done".to_string()),
                    value: Expression::Bool(true),
                },
                ObjectEntry::Data {
                    key: Expression::String("value".to_string()),
                    value: resolved_completion_value,
                },
            ])),
            prototype_source_expression: None,
            updated_bindings: HashMap::new(),
        });
        self.push_i32_const(JS_TYPEOF_OBJECT_TAG);
        Ok(true)
    }

    pub(in crate::backend::direct_wasm) fn emit_fresh_simple_generator_throw_call(
        &mut self,
        object: &Expression,
        arguments: &[CallArgument],
    ) -> DirectResult<bool> {
        let trace_throw = std::env::var_os("AYY_TRACE_SIMPLE_GENERATOR_THROW").is_some();
        let Expression::Identifier(object_name) = object else {
            return Ok(false);
        };
        let binding_name = self
            .resolve_local_array_iterator_binding_name(object_name)
            .unwrap_or_else(|| object_name.clone());
        let Some(iterator_binding) = self
            .state
            .speculation
            .static_semantics
            .local_array_iterator_binding(&binding_name)
        else {
            return Ok(false);
        };
        let (steps, completion_effects) = match &iterator_binding.source {
            IteratorSourceKind::SimpleGenerator {
                steps,
                completion_effects,
                ..
            } => (steps.clone(), completion_effects.clone()),
            _ => return Ok(false),
        };
        let current_index = iterator_binding.static_index.unwrap_or(0);
        if current_index == 0 {
            return Ok(false);
        }
        let index_local = iterator_binding.index_local;
        let closed_index = steps.len().saturating_add(1);
        let Some((step_close_effects, step_outcome)) = current_index
            .checked_sub(1)
            .and_then(|index| steps.get(index))
            .map(|step| (step.close_effects.clone(), step.outcome.clone()))
        else {
            return Ok(false);
        };
        if !matches!(step_outcome, SimpleGeneratorStepOutcome::YieldResult(_)) {
            return Ok(false);
        }

        let sent_value = arguments
            .first()
            .map(|argument| match argument {
                CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                    self.materialize_static_expression(expression)
                }
            })
            .unwrap_or(Expression::Undefined);
        let call_expression = Expression::Call {
            callee: Box::new(Expression::Member {
                object: Box::new(object.clone()),
                property: Box::new(Expression::String("throw".to_string())),
            }),
            arguments: arguments.to_vec(),
        };
        let source_function_name = self.simple_generator_source_function_name(object);
        let substituted_close_effects = step_close_effects
            .iter()
            .map(|effect| Self::substitute_sent_statement(effect, &sent_value))
            .collect::<Vec<_>>();
        let substituted_close_effects =
            self.expand_static_lowered_for_of_completion_effects(&substituted_close_effects);
        if substituted_close_effects.is_empty() {
            return Ok(false);
        }
        self.register_bindings(&substituted_close_effects)?;
        let Some(close_expression) =
            Self::first_iterator_close_expression_in_statements(&substituted_close_effects)
        else {
            return Ok(false);
        };
        let close_target = self
            .resolve_static_iterator_close_target(&close_expression, &[])
            .unwrap_or_else(|| close_expression.clone());
        let throw_property = Expression::String("throw".to_string());
        let throw_binding = self
            .resolve_member_function_binding(&close_target, &throw_property)
            .or_else(|| self.resolve_member_function_binding(&close_expression, &throw_property));
        let throw_getter_binding = self
            .resolve_member_getter_binding(&close_target, &throw_property)
            .or_else(|| self.resolve_member_getter_binding(&close_expression, &throw_property));
        if trace_throw {
            eprintln!(
                "simple_generator_throw:delegate target={close_target:?} binding={throw_binding:?}"
            );
        }
        if throw_binding.is_none()
            && let Some(getter_binding) = throw_getter_binding.as_ref()
            && let Some(StaticEvalOutcome::Throw(throw_value)) = self
                .resolve_static_function_outcome_from_binding_with_call_frame_and_context(
                    getter_binding,
                    &[],
                    &close_target,
                    self.current_function_name(),
                )
                .or_else(|| {
                    self.resolve_static_function_outcome_from_binding_with_context(
                        getter_binding,
                        &[],
                        self.current_function_name(),
                    )
                })
        {
            let Some(throw_expression) = self.resolve_static_throw_value_expression(&throw_value)
            else {
                return Ok(false);
            };
            let strict_mode = self.state.speculation.execution_context.strict_mode;
            return self.emit_yield_delegate_throw_completion_after_replacement(
                &substituted_close_effects,
                Statement::Throw(throw_expression),
                Expression::Undefined,
                &completion_effects,
                &sent_value,
                source_function_name.as_deref(),
                &call_expression,
                index_local,
                closed_index,
                strict_mode,
            );
        }
        if throw_binding.is_none() {
            if let Some(getter_binding) = throw_getter_binding.as_ref() {
                let getter_outcome = self
                    .resolve_static_function_outcome_from_binding_with_call_frame_and_context(
                        getter_binding,
                        &[],
                        &close_target,
                        self.current_function_name(),
                    )
                    .or_else(|| {
                        self.resolve_static_function_outcome_from_binding_with_context(
                            getter_binding,
                            &[],
                            self.current_function_name(),
                        )
                    })
                    .or_else(|| {
                        let LocalFunctionBinding::User(function_name) = getter_binding else {
                            return None;
                        };
                        let user_function = self.user_function(function_name)?;
                        let function =
                            self.resolve_registered_function_declaration(function_name)?;
                        match self
                            .resolve_static_user_function_return_outcome_after_prefix_effects(
                                user_function,
                                &function.body,
                                &[],
                                &close_target,
                            )? {
                            StaticEvalOutcome::Value(value) => {
                                Some(StaticEvalOutcome::Value(value))
                            }
                            StaticEvalOutcome::Throw(_) => None,
                        }
                    });
                let replacement = match getter_outcome {
                    Some(StaticEvalOutcome::Throw(_)) => unreachable!(
                        "throwing yield* throw getter is handled before protocol violation close"
                    ),
                    Some(StaticEvalOutcome::Value(getter_value)) => {
                        if !matches!(getter_value, Expression::Undefined | Expression::Null) {
                            let Some(type_error) = self.resolve_static_throw_value_expression(
                                &StaticThrowValue::NamedError("TypeError"),
                            ) else {
                                return Ok(false);
                            };
                            Statement::Throw(type_error)
                        } else {
                            let Some(type_error) = self.resolve_static_throw_value_expression(
                                &StaticThrowValue::NamedError("TypeError"),
                            ) else {
                                return Ok(false);
                            };
                            Statement::Block {
                                body: vec![
                                    Statement::Expression(Expression::IteratorClose(Box::new(
                                        close_target.clone(),
                                    ))),
                                    Statement::Throw(type_error),
                                ],
                            }
                        }
                    }
                    None if self.function_binding_defaults_to_undefined(getter_binding) => {
                        let Some(type_error) = self.resolve_static_throw_value_expression(
                            &StaticThrowValue::NamedError("TypeError"),
                        ) else {
                            return Ok(false);
                        };
                        Statement::Block {
                            body: vec![
                                Statement::Expression(Expression::IteratorClose(Box::new(
                                    close_target.clone(),
                                ))),
                                Statement::Throw(type_error),
                            ],
                        }
                    }
                    None => return Ok(false),
                };
                if let LocalFunctionBinding::User(function_name) = getter_binding
                    && let Some(user_function) = self.user_function(function_name).cloned()
                {
                    let capture_slots = self
                        .resolve_member_function_capture_slots(&close_target, &throw_property)
                        .or_else(|| {
                            self.resolve_member_function_capture_slots(
                                &close_expression,
                                &throw_property,
                            )
                        });
                    self.emit_user_function_call_with_function_this_binding(
                        &user_function,
                        &[],
                        &close_target,
                        capture_slots.as_ref(),
                    )?;
                    self.state.emission.output.instructions.push(0x1a);
                }
                let strict_mode = self.state.speculation.execution_context.strict_mode;
                return self.emit_yield_delegate_throw_completion_after_replacement(
                    &substituted_close_effects,
                    replacement,
                    Expression::Undefined,
                    &completion_effects,
                    &sent_value,
                    source_function_name.as_deref(),
                    &call_expression,
                    index_local,
                    closed_index,
                    strict_mode,
                );
            };
            let Some(type_error) = self
                .resolve_static_throw_value_expression(&StaticThrowValue::NamedError("TypeError"))
            else {
                return Ok(false);
            };
            let strict_mode = self.state.speculation.execution_context.strict_mode;
            return self.emit_yield_delegate_throw_completion_after_replacement(
                &substituted_close_effects,
                Statement::Block {
                    body: vec![
                        Statement::Expression(Expression::IteratorClose(Box::new(
                            close_target.clone(),
                        ))),
                        Statement::Throw(type_error),
                    ],
                },
                Expression::Undefined,
                &completion_effects,
                &sent_value,
                source_function_name.as_deref(),
                &call_expression,
                index_local,
                closed_index,
                strict_mode,
            );
        }
        let Some(throw_binding) = throw_binding else {
            return Ok(false);
        };
        let throw_arguments = vec![CallArgument::Expression(sent_value.clone())];
        let throw_outcome = self
            .resolve_static_function_outcome_from_binding_with_call_frame_and_context(
                &throw_binding,
                &throw_arguments,
                &close_target,
                self.current_function_name(),
            )
            .or_else(|| {
                self.resolve_static_function_outcome_from_binding_with_context(
                    &throw_binding,
                    &throw_arguments,
                    self.current_function_name(),
                )
            })
            .or_else(|| {
                let LocalFunctionBinding::User(function_name) = &throw_binding else {
                    return None;
                };
                let user_function = self.user_function(function_name)?;
                let function = self.resolve_registered_function_declaration(function_name)?;
                self.resolve_static_user_function_return_outcome_after_prefix_effects(
                    user_function,
                    &function.body,
                    &throw_arguments,
                    &close_target,
                )
            });
        let Some(throw_outcome) = throw_outcome else {
            return Ok(false);
        };

        let strict_mode = self.state.speculation.execution_context.strict_mode;
        match throw_outcome {
            StaticEvalOutcome::Throw(throw_value) => {
                let Some(throw_expression) =
                    self.resolve_static_throw_value_expression(&throw_value)
                else {
                    return Ok(false);
                };
                return self.emit_yield_delegate_throw_completion_after_replacement(
                    &substituted_close_effects,
                    Statement::Throw(throw_expression),
                    Expression::Undefined,
                    &completion_effects,
                    &sent_value,
                    source_function_name.as_deref(),
                    &call_expression,
                    index_local,
                    closed_index,
                    strict_mode,
                );
            }
            StaticEvalOutcome::Value(inner_result) => {
                let inner_result = self.resolve_simple_generator_result_value_with_context(
                    &inner_result,
                    self.current_function_name(),
                );
                if let LocalFunctionBinding::User(function_name) = throw_binding
                    && let Some(user_function) = self.user_function(&function_name).cloned()
                {
                    let function_body = self
                        .resolve_registered_function_declaration(&function_name)
                        .map(|function| function.body.clone())
                        .unwrap_or_default();
                    self.emit_user_function_call_with_function_this_binding(
                        &user_function,
                        &throw_arguments,
                        &close_target,
                        None,
                    )?;
                    self.sync_static_iterator_close_arguments_assignments(
                        &user_function,
                        &[sent_value.clone()],
                        &function_body,
                    );
                    self.state.emission.output.instructions.push(0x1a);
                }

                let Some(inner_object_binding) =
                    self.resolve_object_binding_from_expression(&inner_result)
                else {
                    if matches!(
                        self.infer_value_kind(&inner_result),
                        Some(
                            StaticValueKind::Undefined
                                | StaticValueKind::Null
                                | StaticValueKind::Bool
                                | StaticValueKind::Number
                                | StaticValueKind::String
                                | StaticValueKind::BigInt
                                | StaticValueKind::Symbol
                        )
                    ) {
                        let Some(type_error) = self.resolve_static_throw_value_expression(
                            &StaticThrowValue::NamedError("TypeError"),
                        ) else {
                            return Ok(false);
                        };
                        return self.emit_yield_delegate_throw_completion_after_replacement(
                            &substituted_close_effects,
                            Statement::Throw(type_error),
                            Expression::Undefined,
                            &completion_effects,
                            &sent_value,
                            source_function_name.as_deref(),
                            &call_expression,
                            index_local,
                            closed_index,
                            strict_mode,
                        );
                    }
                    return Ok(false);
                };
                let done_property = Expression::String("done".to_string());
                let done_member = Expression::Member {
                    object: Box::new(inner_result.clone()),
                    property: Box::new(done_property.clone()),
                };
                let materialized_done_member =
                    self.materialize_simple_generator_yield_result_done_member(&done_member);
                let static_shadow_done = self
                    .runtime_object_property_shadow_binding_name_for_expression(
                        &inner_result,
                        &done_property,
                    )
                    .and_then(|shadow_binding_name| {
                        self.global_value_binding(&shadow_binding_name)
                            .and_then(|value| self.resolve_static_boolean_expression(value))
                    });
                let done_accessor =
                    object_binding_lookup_descriptor(&inner_object_binding, &done_property)
                        .is_some_and(|descriptor| {
                            descriptor.getter.is_some() || descriptor.has_get
                        });
                let done = if done_accessor {
                    match self.resolve_static_iterator_step_done_outcome(
                        &inner_result,
                        &inner_object_binding,
                        &HashMap::new(),
                        self.current_function_name(),
                    ) {
                        Some(Ok(done_expression)) => {
                            let Some(done) =
                                self.resolve_static_boolean_expression(&done_expression)
                            else {
                                return Ok(false);
                            };
                            done
                        }
                        Some(Err(throw_value)) => {
                            return self.emit_yield_delegate_throw_completion_after_replacement(
                                &substituted_close_effects,
                                Statement::Throw(throw_value),
                                Expression::Undefined,
                                &completion_effects,
                                &sent_value,
                                source_function_name.as_deref(),
                                &call_expression,
                                index_local,
                                closed_index,
                                strict_mode,
                            );
                        }
                        None => return Ok(false),
                    }
                } else if let Some(done) = static_shadow_done {
                    done
                } else if let Some(done) =
                    self.resolve_static_boolean_expression(&materialized_done_member)
                {
                    done
                } else {
                    match self.resolve_static_iterator_step_done_outcome(
                        &inner_result,
                        &inner_object_binding,
                        &HashMap::new(),
                        self.current_function_name(),
                    ) {
                        Some(Ok(done_expression)) => {
                            let Some(done) =
                                self.resolve_static_boolean_expression(&done_expression)
                            else {
                                return Ok(false);
                            };
                            done
                        }
                        Some(Err(throw_value)) => {
                            return self.emit_yield_delegate_throw_completion_after_replacement(
                                &substituted_close_effects,
                                Statement::Throw(throw_value),
                                Expression::Undefined,
                                &completion_effects,
                                &sent_value,
                                source_function_name.as_deref(),
                                &call_expression,
                                index_local,
                                closed_index,
                                strict_mode,
                            );
                        }
                        None => return Ok(false),
                    }
                };

                if !done {
                    self.state
                        .speculation
                        .static_semantics
                        .last_bound_user_function_call = Some(BoundUserFunctionCallSnapshot {
                        function_name: "__ayy_simple_generator_throw".to_string(),
                        source_expression: Some(call_expression),
                        result_expression: Some(self.materialize_static_expression(&inner_result)),
                        prototype_source_expression: None,
                        updated_bindings: HashMap::new(),
                    });
                    self.push_i32_const(JS_TYPEOF_OBJECT_TAG);
                    return Ok(true);
                }

                let value_member = Expression::Member {
                    object: Box::new(inner_result),
                    property: Box::new(Expression::String("value".to_string())),
                };
                self.emit_yield_delegate_throw_completion_after_replacement(
                    &substituted_close_effects,
                    Statement::Expression(value_member.clone()),
                    value_member,
                    &completion_effects,
                    &sent_value,
                    source_function_name.as_deref(),
                    &call_expression,
                    index_local,
                    closed_index,
                    strict_mode,
                )
            }
        }
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
            .map(|step| {
                (
                    step.effects.clone(),
                    step.close_effects.clone(),
                    step.outcome.clone(),
                )
            });
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
        if let Some((step_effects, step_close_effects, step_outcome)) = step {
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
                if matches!(step_outcome, SimpleGeneratorStepOutcome::YieldResult(_))
                    && self.try_emit_yield_delegate_return_close(
                        &substituted_close_effects,
                        &sent_value,
                        &call_expression,
                        index_local,
                        closed_index,
                        self.state.speculation.execution_context.strict_mode,
                    )?
                {
                    return Ok(true);
                }
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

    fn materialize_simple_generator_yield_result_done_member(
        &self,
        done_member: &Expression,
    ) -> Expression {
        let materialized = self.materialize_static_expression(done_member);
        if matches!(materialized, Expression::Bool(_)) {
            return materialized;
        }

        let Expression::Member { object, property } = done_member else {
            return materialized;
        };
        let Some(shadow_binding_name) =
            self.runtime_object_property_shadow_binding_name_for_expression(object, property)
        else {
            return materialized;
        };
        let Some(shadow_value) = self.global_value_binding(&shadow_binding_name).cloned() else {
            return materialized;
        };
        self.materialize_static_expression(&shadow_value)
    }

    fn apply_yield_result_completion_value_to_effects(
        effects: Vec<Statement>,
        completion_value: &Expression,
    ) -> Vec<Statement> {
        effects
            .into_iter()
            .map(|effect| match effect {
                Statement::Let {
                    name,
                    mutable,
                    value: Expression::Undefined,
                } if name.starts_with("__ayy_generator_sent_") => Statement::Let {
                    name,
                    mutable,
                    value: completion_value.clone(),
                },
                Statement::Var {
                    name,
                    value: Expression::Undefined,
                } if name.starts_with("__ayy_generator_sent_") => Statement::Var {
                    name,
                    value: completion_value.clone(),
                },
                other => other,
            })
            .collect()
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
        let source_function_name = self.simple_generator_source_function_name(object);
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
                    let yielded_value = self.resolve_simple_generator_result_value_with_context(
                        &yielded_value,
                        source_function_name.as_deref(),
                    );
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
                SimpleGeneratorStepOutcome::YieldResult(result) => {
                    let substituted_result = Self::substitute_sent_expression(result, &sent_value);
                    let done_member = Expression::Member {
                        object: Box::new(substituted_result.clone()),
                        property: Box::new(Expression::String("done".to_string())),
                    };
                    let materialized_done_member =
                        self.materialize_simple_generator_yield_result_done_member(&done_member);
                    if std::env::var_os("AYY_TRACE_SIMPLE_GENERATORS").is_some() {
                        eprintln!(
                            "simple_next_call:yield_result result={substituted_result:?} done={materialized_done_member:?}"
                        );
                    }
                    if matches!(materialized_done_member, Expression::Bool(true)) {
                        set_binding_index(self, steps.len().saturating_add(1));
                        let value_member = Expression::Member {
                            object: Box::new(substituted_result),
                            property: Box::new(Expression::String("value".to_string())),
                        };
                        self.emit_numeric_expression(&value_member)?;
                        self.state.emission.output.instructions.push(0x1a);
                        let resolved_completion_value = self
                            .resolve_simple_generator_result_value_with_context(
                                &self.materialize_static_expression(&value_member),
                                source_function_name.as_deref(),
                            );
                        let substituted_completion_effects = completion_effects
                            .iter()
                            .map(|effect| Self::substitute_sent_statement(effect, &sent_value))
                            .collect::<Vec<_>>();
                        let substituted_completion_effects = self
                            .expand_static_lowered_for_of_completion_effects(
                                &substituted_completion_effects,
                            );
                        let substituted_completion_effects =
                            Self::apply_yield_result_completion_value_to_effects(
                                substituted_completion_effects,
                                &resolved_completion_value,
                            );
                        self.register_bindings(&substituted_completion_effects)?;
                        if let Some(throw_value) = self
                            .emit_static_simple_generator_effects_in_eval_scope(
                                &substituted_completion_effects,
                                self.state.speculation.execution_context.strict_mode,
                            )?
                        {
                            self.state
                                .speculation
                                .static_semantics
                                .last_bound_user_function_call =
                                Some(BoundUserFunctionCallSnapshot {
                                    function_name: "__ayy_simple_generator_next".to_string(),
                                    source_expression: Some(call_expression.clone()),
                                    result_expression: None,
                                    prototype_source_expression: None,
                                    updated_bindings: HashMap::new(),
                                });
                            self.emit_static_throw_value(&throw_value)?;
                            return Ok(true);
                        }
                        self.state
                            .speculation
                            .static_semantics
                            .last_bound_user_function_call = Some(BoundUserFunctionCallSnapshot {
                            function_name: "__ayy_simple_generator_next".to_string(),
                            source_expression: Some(call_expression.clone()),
                            result_expression: Some(iter_result_object(
                                true,
                                resolved_completion_value,
                            )),
                            prototype_source_expression: None,
                            updated_bindings: HashMap::new(),
                        });
                        self.push_i32_const(JS_TYPEOF_OBJECT_TAG);
                        return Ok(true);
                    }
                    set_binding_index(self, current_index.saturating_add(1));
                    let yielded_result = self.materialize_static_expression(&substituted_result);
                    self.state
                        .speculation
                        .static_semantics
                        .last_bound_user_function_call = Some(BoundUserFunctionCallSnapshot {
                        function_name: "__ayy_simple_generator_next".to_string(),
                        source_expression: Some(call_expression.clone()),
                        result_expression: Some(yielded_result),
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
            let next_index = if current_index >= steps.len() {
                steps.len().saturating_add(1)
            } else {
                current_index.saturating_add(1)
            };
            set_binding_index(self, next_index);
            let completion_result_expression = if current_index == steps.len() {
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
                let resolved_completion_value = self
                    .simple_generator_effect_expression(
                        &completion_value,
                        &substituted_completion_effects,
                    )
                    .unwrap_or_else(|| self.materialize_static_expression(&completion_value));
                let resolved_completion_value = self
                    .resolve_simple_generator_result_value_with_context(
                        &resolved_completion_value,
                        source_function_name.as_deref(),
                    );
                iter_result_object(true, resolved_completion_value)
            } else {
                iter_result_object(true, Expression::Undefined)
            };
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
