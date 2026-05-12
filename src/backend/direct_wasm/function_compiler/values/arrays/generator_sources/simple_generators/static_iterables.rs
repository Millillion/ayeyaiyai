use super::*;

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
        let mut steps = Vec::new();
        let mut saw_accessor_value = false;
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
            let done = self.evaluate_static_iterator_step_field(
                done,
                &step_bindings,
                Some(&next_function_name),
            );
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

    pub(in crate::backend::direct_wasm) fn resolve_static_iterable_binding_from_expression(
        &self,
        expression: &Expression,
    ) -> Option<ArrayValueBinding> {
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
