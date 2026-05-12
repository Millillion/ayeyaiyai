use super::*;

impl<'a> FunctionCompiler<'a> {
    pub(in crate::backend::direct_wasm) fn user_function_supports_emitted_specialized_function_summary(
        &self,
        user_function: &UserFunction,
        summary: &InlineFunctionSummary,
    ) -> bool {
        !user_function.is_async()
            && !user_function.is_generator()
            && !user_function.has_parameter_defaults()
            && user_function.extra_argument_indices.is_empty()
            && !user_function.has_lowered_pattern_parameters()
            && self
                .user_function_parameter_iterator_consumption_indices(user_function)
                .is_empty()
            && !self.user_function_mentions_private_member_access(user_function)
            && !self.user_function_mentions_direct_eval(user_function)
            && !self.user_function_contains_identifier_callee_call(user_function)
            && !(inline_summary_mentions_call_frame_state(summary) && !user_function.lexical_this)
    }

    pub(in crate::backend::direct_wasm) fn user_function_supports_specialized_function_summary(
        &self,
        user_function: &UserFunction,
        summary: &InlineFunctionSummary,
    ) -> bool {
        self.user_function_supports_emitted_specialized_function_summary(user_function, summary)
            && !self
                .backend
                .function_registry
                .analysis
                .user_function_capture_bindings
                .contains_key(&user_function.name)
            && !self.user_function_references_captured_user_function(user_function)
    }

    pub(in crate::backend::direct_wasm) fn resolve_specialized_function_value_from_expression(
        &self,
        expression: &Expression,
    ) -> Option<SpecializedFunctionValue> {
        match expression {
            Expression::Call { callee, arguments } | Expression::New { callee, arguments } => self
                .resolve_specialized_function_value_from_returned_call_expression(
                    callee, arguments,
                ),
            Expression::Member { object, property } => self
                .resolve_specialized_function_value_from_member_getter_expression(object, property),
            Expression::Identifier(name) => self
                .state
                .speculation
                .static_semantics
                .values
                .local_specialized_function_values
                .get(name)
                .cloned()
                .filter(|specialized| match &specialized.binding {
                    LocalFunctionBinding::User(function_name) => self
                        .user_function(function_name)
                        .is_some_and(|user_function| {
                            self.user_function_supports_specialized_function_summary(
                                user_function,
                                &specialized.summary,
                            )
                        }),
                    LocalFunctionBinding::Builtin(_) => true,
                })
                .or_else(|| {
                    self.backend
                        .global_semantics
                        .functions
                        .specialized_function_values
                        .get(name)
                        .cloned()
                        .filter(|specialized| match &specialized.binding {
                            LocalFunctionBinding::User(function_name) => self
                                .user_function(function_name)
                                .is_some_and(|user_function| {
                                    self.user_function_supports_specialized_function_summary(
                                        user_function,
                                        &specialized.summary,
                                    )
                                }),
                            LocalFunctionBinding::Builtin(_) => true,
                        })
                }),
            _ => None,
        }
    }

    fn resolve_specialized_function_value_from_member_getter_expression(
        &self,
        object: &Expression,
        property: &Expression,
    ) -> Option<SpecializedFunctionValue> {
        let getter_binding = self.resolve_member_getter_binding(object, property)?;
        let returned_expression = self
            .resolve_function_binding_static_return_expression_with_call_frame(
                &getter_binding,
                &[],
                object,
            )?;
        let template = self
            .resolve_function_value_template_from_expression(&returned_expression)
            .or_else(|| {
                self.resolve_specialized_function_value_from_expression(&returned_expression)
            })?;
        let capture_slots = self
            .resolve_member_function_capture_slots(object, property)
            .unwrap_or_default();
        if capture_slots.is_empty() {
            return Some(template);
        }
        let captured = match &template.binding {
            LocalFunctionBinding::User(function_name) => self
                .backend
                .function_registry
                .analysis
                .user_function_capture_bindings
                .get(function_name)
                .map(|captures| captures.keys().cloned().collect::<BTreeSet<_>>())
                .unwrap_or_else(|| self.collect_capture_bindings_from_summary(&template.summary)),
            LocalFunctionBinding::Builtin(_) => {
                self.collect_capture_bindings_from_summary(&template.summary)
            }
        };
        if captured.is_empty() {
            return Some(template);
        }
        let mut bindings = HashMap::new();
        for capture_name in captured {
            let slot_name = capture_slots.get(&capture_name)?;
            bindings.insert(capture_name, Expression::Identifier(slot_name.clone()));
        }
        Some(SpecializedFunctionValue {
            binding: template.binding.clone(),
            summary: rewrite_inline_function_summary_bindings(&template.summary, &bindings),
        })
    }

    fn resolve_specialized_function_value_from_returned_call_expression(
        &self,
        callee: &Expression,
        arguments: &[CallArgument],
    ) -> Option<SpecializedFunctionValue> {
        let LocalFunctionBinding::User(outer_function_name) = self
            .resolve_function_binding_from_expression_with_context(
                callee,
                self.current_function_name(),
            )?
        else {
            return None;
        };
        let outer_user_function = self.user_function(&outer_function_name)?;
        let outer_function =
            self.resolve_registered_function_declaration(&outer_user_function.name)?;
        let returned_function_name = collect_returned_identifier(&outer_function.body)?;
        let inner_user_function = self.user_function(&returned_function_name)?;
        if !self.user_function_supports_emitted_specialized_function_summary(
            inner_user_function,
            inner_user_function.inline_summary.as_ref()?,
        ) {
            return None;
        }
        let summary = inner_user_function.inline_summary.as_ref()?;
        let outer_effects =
            self.static_returned_function_call_inline_effects(outer_user_function, outer_function)?;

        let local_aliases = collect_returned_member_local_aliases(&outer_function.body);
        let with_scope_objects = collect_returned_identifier_with_scope_objects(
            &outer_function.body,
            &returned_function_name,
        )
        .unwrap_or_default();
        let captured = self
            .backend
            .function_registry
            .analysis
            .user_function_capture_bindings
            .get(&returned_function_name)
            .map(|bindings| bindings.keys().cloned().collect::<BTreeSet<_>>())
            .unwrap_or_else(|| self.collect_capture_bindings_from_summary(summary));
        let mut bindings = HashMap::new();

        for capture_name in captured {
            let bound_expression = if let Some(scope_expression) =
                with_scope_objects.iter().rev().find_map(|scope_object| {
                    let aliased_scope_object = resolve_returned_member_local_alias_expression(
                        scope_object,
                        &local_aliases,
                    );
                    let substituted_scope_object = self.substitute_user_function_argument_bindings(
                        &aliased_scope_object,
                        outer_user_function,
                        arguments,
                    );
                    self.scope_object_has_binding_property(&substituted_scope_object, &capture_name)
                        .then_some(substituted_scope_object)
                }) {
                self.materialize_static_expression(&Expression::Member {
                    object: Box::new(scope_expression),
                    property: Box::new(Expression::String(capture_name.clone())),
                })
            } else if let Some(alias) = local_aliases.get(&capture_name) {
                self.substitute_user_function_argument_bindings(
                    alias,
                    outer_user_function,
                    arguments,
                )
            } else if let Some(param_name) = outer_user_function.params.iter().find(|param| {
                *param == &capture_name
                    || scoped_binding_source_name(param)
                        .is_some_and(|source_name| source_name == capture_name)
            }) {
                self.substitute_user_function_argument_bindings(
                    &Expression::Identifier(param_name.clone()),
                    outer_user_function,
                    arguments,
                )
            } else {
                Expression::Identifier(capture_name.clone())
            };

            if !inline_summary_side_effect_free_expression(&bound_expression) {
                return None;
            }
            bindings.insert(capture_name, bound_expression);
        }

        let mut summary = rewrite_inline_function_summary_bindings(summary, &bindings);
        if !outer_effects.is_empty() {
            let mut effects = outer_effects;
            effects.extend(summary.effects);
            summary.effects = effects;
        }

        Some(SpecializedFunctionValue {
            binding: LocalFunctionBinding::User(returned_function_name),
            summary,
        })
    }

    fn static_returned_function_call_inline_effects(
        &self,
        outer_user_function: &UserFunction,
        outer_function: &FunctionDeclaration,
    ) -> Option<Vec<InlineFunctionEffect>> {
        let (effects, found_return) = self
            .static_returned_function_call_inline_effects_in_statements(
                outer_user_function,
                &outer_function.body,
            )?;
        found_return.then_some(effects)
    }

    fn static_returned_function_call_inline_effects_in_statements(
        &self,
        outer_user_function: &UserFunction,
        statements: &[Statement],
    ) -> Option<(Vec<InlineFunctionEffect>, bool)> {
        let mut effects = Vec::new();
        for statement in statements {
            if matches!(statement, Statement::Return(Expression::Identifier(_))) {
                return Some((effects, true));
            }
            match statement {
                Statement::Var { .. } | Statement::Let { .. } => {}
                Statement::Assign { name, value } => {
                    effects.push(InlineFunctionEffect::Assign {
                        name: name.clone(),
                        value: value.clone(),
                    });
                }
                Statement::Expression(Expression::Assign { name, value }) => {
                    effects.push(InlineFunctionEffect::Assign {
                        name: name.clone(),
                        value: value.as_ref().clone(),
                    });
                }
                Statement::Expression(Expression::Update { name, op, prefix }) => {
                    effects.push(InlineFunctionEffect::Update {
                        name: name.clone(),
                        op: *op,
                        prefix: *prefix,
                    });
                }
                Statement::Expression(Expression::Call { callee, arguments }) if matches!(callee.as_ref(), Expression::Identifier(name) if name == "eval") =>
                {
                    let eval_effects = self.static_direct_eval_inline_effects_with_context(
                        arguments,
                        Some(&outer_user_function.name),
                    )?;
                    effects.extend(eval_effects);
                }
                Statement::Block { body }
                | Statement::Declaration { body }
                | Statement::With { body, .. } => {
                    let (nested_effects, found_return) = self
                        .static_returned_function_call_inline_effects_in_statements(
                            outer_user_function,
                            body,
                        )?;
                    effects.extend(nested_effects);
                    if found_return {
                        return Some((effects, true));
                    }
                }
                _ => return None,
            }
        }
        Some((effects, false))
    }

    pub(in crate::backend::direct_wasm) fn resolve_function_value_template_from_expression(
        &self,
        expression: &Expression,
    ) -> Option<SpecializedFunctionValue> {
        let binding = self.resolve_function_binding_from_expression(expression)?;
        let LocalFunctionBinding::User(function_name) = &binding else {
            return None;
        };
        let user_function = self.user_function(function_name)?;
        let summary = user_function.inline_summary.as_ref()?;
        if !self.user_function_supports_specialized_function_summary(user_function, summary) {
            return None;
        }
        Some(SpecializedFunctionValue {
            binding,
            summary: summary.clone(),
        })
    }
}
