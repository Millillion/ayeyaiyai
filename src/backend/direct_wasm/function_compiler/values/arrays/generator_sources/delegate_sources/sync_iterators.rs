use super::*;

impl<'a> FunctionCompiler<'a> {
    pub(in crate::backend::direct_wasm) fn resolve_simple_yield_delegate_source(
        &self,
        expression: &Expression,
        async_generator: bool,
    ) -> Option<(Vec<SimpleGeneratorStep>, Vec<Statement>, Expression)> {
        if let Expression::Identifier(name) = expression
            && self.simple_generator_identifier_read_is_unresolvable(name)
        {
            return Some((
                vec![SimpleGeneratorStep {
                    effects: Vec::new(),
                    close_effects: Vec::new(),
                    outcome: SimpleGeneratorStepOutcome::Throw(Expression::Call {
                        callee: Box::new(Expression::Identifier("ReferenceError".to_string())),
                        arguments: Vec::new(),
                    }),
                }],
                Vec::new(),
                Expression::Undefined,
            ));
        }
        if let Expression::Await(_) = expression {
            return match self.resolve_static_await_resolution_outcome(expression)? {
                StaticEvalOutcome::Value(value) => {
                    self.resolve_simple_yield_delegate_source(&value, async_generator)
                }
                StaticEvalOutcome::Throw(throw_value) => {
                    self.simple_generator_throw_step_with_completion(throw_value)
                }
            };
        }
        if async_generator
            && let Some(source) = self.resolve_simple_async_yield_delegate_source(expression)
        {
            return Some((source.0, source.1, Expression::Undefined));
        }
        if async_generator && self.expression_has_async_iterator_entry(expression) {
            return None;
        }

        if let Some(source) = self.resolve_iterator_source_kind(expression) {
            if let Some(flattened) =
                self.flatten_simple_yield_delegate_iterator_source_with_completion(&source)
            {
                return Some(flattened);
            }
        }

        if let Some(primitive) = self.resolve_static_primitive_expression_with_context(
            expression,
            self.current_function_name(),
        ) && !static_expression_matches(&primitive, expression)
            && let Some(source) =
                self.resolve_primitive_simple_yield_delegate_source(&primitive, async_generator)
        {
            return Some(source);
        }

        let materialized = self.materialize_static_expression(expression);
        if !static_expression_matches(&materialized, expression) {
            return self.resolve_simple_yield_delegate_source(&materialized, async_generator);
        }

        let iterator_property = self.materialize_static_expression(&symbol_iterator_expression());
        let call_outcome = if let Some(getter_binding) =
            self.resolve_member_getter_binding(expression, &iterator_property)
        {
            match self.resolve_static_function_outcome_from_binding_with_context(
                &getter_binding,
                &[],
                self.current_function_name(),
            )? {
                StaticEvalOutcome::Throw(throw_value) => {
                    return self.simple_generator_throw_step_with_completion(throw_value);
                }
                StaticEvalOutcome::Value(method_value) => {
                    self.resolve_static_sync_iterator_method_call_outcome(&method_value)?
                }
            }
        } else if let Some(function_binding) =
            self.resolve_member_function_binding(expression, &iterator_property)
        {
            let outcome = self.resolve_static_function_outcome_from_binding_with_context(
                &function_binding,
                &[],
                self.current_function_name(),
            )?;
            self.validate_static_sync_iterator_call_outcome(outcome)?
        } else {
            let object_binding = self.resolve_object_binding_from_expression(expression)?;
            let Some(method_value) =
                object_binding_lookup_value(&object_binding, &iterator_property)
            else {
                return Some((
                    vec![SimpleGeneratorStep {
                        effects: Vec::new(),
                        close_effects: Vec::new(),
                        outcome: SimpleGeneratorStepOutcome::Throw(Expression::Call {
                            callee: Box::new(Expression::Identifier("TypeError".to_string())),
                            arguments: Vec::new(),
                        }),
                    }],
                    Vec::new(),
                    Expression::Undefined,
                ));
            };
            self.resolve_static_sync_iterator_method_call_outcome(method_value)?
        };

        match call_outcome {
            StaticEvalOutcome::Throw(throw_value) => {
                self.simple_generator_throw_step_with_completion(throw_value)
            }
            StaticEvalOutcome::Value(iterator_value) => {
                let source = self.resolve_iterator_source_kind(&iterator_value)?;
                self.flatten_simple_yield_delegate_iterator_source_with_completion(&source)
            }
        }
    }

    pub(in crate::backend::direct_wasm) fn simple_generator_throw_step(
        &self,
        throw_value: StaticThrowValue,
    ) -> Option<(Vec<SimpleGeneratorStep>, Vec<Statement>)> {
        Some((
            vec![SimpleGeneratorStep {
                effects: Vec::new(),
                close_effects: Vec::new(),
                outcome: SimpleGeneratorStepOutcome::Throw(
                    self.resolve_static_throw_value_expression(&throw_value)?,
                ),
            }],
            Vec::new(),
        ))
    }

    fn simple_generator_throw_step_with_completion(
        &self,
        throw_value: StaticThrowValue,
    ) -> Option<(Vec<SimpleGeneratorStep>, Vec<Statement>, Expression)> {
        let (steps, effects) = self.simple_generator_throw_step(throw_value)?;
        Some((steps, effects, Expression::Undefined))
    }

    fn resolve_primitive_simple_yield_delegate_source(
        &self,
        primitive: &Expression,
        async_generator: bool,
    ) -> Option<(Vec<SimpleGeneratorStep>, Vec<Statement>, Expression)> {
        if async_generator {
            return None;
        }
        let iterator_property = self.materialize_static_expression(&symbol_iterator_expression());
        let LocalFunctionBinding::User(function_name) =
            self.resolve_member_function_binding(primitive, &iterator_property)?
        else {
            return None;
        };
        let user_function = self.user_function(&function_name)?;
        if !user_function.is_generator()
            || !user_function.params.is_empty()
            || user_function.has_parameter_defaults()
            || !user_function.extra_argument_indices.is_empty()
        {
            return None;
        }
        let function = self.resolve_registered_function_declaration(&function_name)?;
        let mut call_argument_values = Vec::new();
        let mut arguments_values = Vec::new();
        let substituted_body = self
            .substitute_simple_generator_statements_with_call_frame_bindings(
                &function.body,
                user_function,
                function.mapped_arguments && !function.strict,
                &mut call_argument_values,
                &mut arguments_values,
                primitive,
            )?;
        let substituted_body =
            self.expand_static_lowered_for_of_completion_effects(&substituted_body);
        let (substituted_body, completion_value) =
            self.split_simple_generator_completion(substituted_body)?;
        let mut steps = Vec::new();
        let mut effects = Vec::new();
        self.analyze_simple_generator_statements(
            &substituted_body,
            false,
            &mut steps,
            &mut effects,
        )?;
        Some((steps, effects, completion_value))
    }

    pub(in crate::backend::direct_wasm) fn resolve_static_sync_iterator_method_call_outcome(
        &self,
        method_value: &Expression,
    ) -> Option<StaticEvalOutcome> {
        let current_function_name = self.current_function_name();
        if let Some(primitive) = self
            .resolve_static_primitive_expression_with_context(method_value, current_function_name)
        {
            return match primitive {
                Expression::Undefined | Expression::Null => Some(StaticEvalOutcome::Throw(
                    StaticThrowValue::NamedError("TypeError"),
                )),
                _ => Some(StaticEvalOutcome::Throw(StaticThrowValue::NamedError(
                    "TypeError",
                ))),
            };
        }

        let Some(binding) = self.resolve_function_binding_from_expression_with_context(
            method_value,
            current_function_name,
        ) else {
            return Some(StaticEvalOutcome::Throw(StaticThrowValue::NamedError(
                "TypeError",
            )));
        };

        let outcome = self.resolve_static_function_outcome_from_binding_with_context(
            &binding,
            &[],
            current_function_name,
        )?;
        self.validate_static_sync_iterator_call_outcome(outcome)
    }

    pub(in crate::backend::direct_wasm) fn validate_static_sync_iterator_call_outcome(
        &self,
        outcome: StaticEvalOutcome,
    ) -> Option<StaticEvalOutcome> {
        match &outcome {
            StaticEvalOutcome::Throw(_) => Some(outcome),
            StaticEvalOutcome::Value(iterator_value) => {
                if self.resolve_iterator_source_kind(iterator_value).is_some()
                    || self
                        .resolve_object_binding_from_expression(iterator_value)
                        .is_some()
                    || matches!(
                        self.infer_value_kind(iterator_value),
                        Some(StaticValueKind::Object | StaticValueKind::Function)
                    )
                {
                    Some(outcome)
                } else {
                    Some(StaticEvalOutcome::Throw(StaticThrowValue::NamedError(
                        "TypeError",
                    )))
                }
            }
        }
    }

    pub(in crate::backend::direct_wasm) fn flatten_simple_yield_delegate_iterator_source(
        &self,
        source: &IteratorSourceKind,
    ) -> Option<(Vec<SimpleGeneratorStep>, Vec<Statement>)> {
        self.flatten_simple_yield_delegate_iterator_source_with_completion(source)
            .map(|(steps, completion_effects, _)| (steps, completion_effects))
    }

    fn flatten_simple_yield_delegate_iterator_source_with_completion(
        &self,
        source: &IteratorSourceKind,
    ) -> Option<(Vec<SimpleGeneratorStep>, Vec<Statement>, Expression)> {
        match source {
            IteratorSourceKind::SimpleGenerator {
                steps,
                completion_effects,
                completion_value,
                ..
            } => Some((
                steps.clone(),
                completion_effects.clone(),
                completion_value.clone(),
            )),
            IteratorSourceKind::StaticArray {
                values,
                keys_only,
                length_local,
                runtime_name,
            } if length_local.is_none() && runtime_name.is_none() => Some((
                values
                    .iter()
                    .enumerate()
                    .map(|(index, value)| SimpleGeneratorStep {
                        effects: Vec::new(),
                        close_effects: Vec::new(),
                        outcome: SimpleGeneratorStepOutcome::Yield(if *keys_only {
                            Expression::Number(index as f64)
                        } else {
                            value.clone().unwrap_or(Expression::Undefined)
                        }),
                    })
                    .collect(),
                Vec::new(),
                Expression::Undefined,
            )),
            _ => None,
        }
    }
}
