use super::*;

impl<'a> FunctionCompiler<'a> {
    fn static_iterator_method_result_is_object(
        &self,
        value: &Expression,
        current_function_name: Option<&str>,
    ) -> bool {
        matches!(value, Expression::Object(_) | Expression::Array(_))
            || self.resolve_object_binding_from_expression(value).is_some()
            || self.resolve_array_binding_from_expression(value).is_some()
            || self
                .resolve_function_binding_from_expression_with_context(value, current_function_name)
                .is_some()
            || matches!(
                self.infer_value_kind(value),
                Some(StaticValueKind::Object | StaticValueKind::Function)
            )
    }

    pub(in crate::backend::direct_wasm) fn resolve_simple_async_yield_delegate_source(
        &self,
        expression: &Expression,
    ) -> Option<(Vec<SimpleGeneratorStep>, Vec<Statement>)> {
        if let Some(array_binding) = self.resolve_array_binding_from_expression(expression) {
            return Some((
                array_binding
                    .values
                    .into_iter()
                    .map(|value| SimpleGeneratorStep {
                        effects: Vec::new(),
                        close_effects: Vec::new(),
                        outcome: SimpleGeneratorStepOutcome::Yield(
                            value.unwrap_or(Expression::Undefined),
                        ),
                    })
                    .collect(),
                Vec::new(),
            ));
        }
        let async_iterator_property = self.materialize_static_expression(&Expression::Member {
            object: Box::new(Expression::Identifier("Symbol".to_string())),
            property: Box::new(Expression::String("asyncIterator".to_string())),
        });
        let iterator_property = self.materialize_static_expression(&symbol_iterator_expression());
        let current_function_name = self.current_function_name();
        let resolve_iterator_method_call_outcome = |compiler: &Self,
                                                    property: &Expression|
         -> Option<Option<StaticEvalOutcome>> {
            if let Some(getter_binding) =
                compiler.resolve_member_getter_binding(expression, property)
            {
                return match compiler.resolve_static_function_outcome_from_binding_with_context(
                    &getter_binding,
                    &[],
                    current_function_name,
                )? {
                    StaticEvalOutcome::Throw(throw_value) => {
                        Some(Some(StaticEvalOutcome::Throw(throw_value)))
                    }
                    StaticEvalOutcome::Value(method_value) => {
                        if matches!(method_value, Expression::Undefined | Expression::Null) {
                            return Some(None);
                        }
                        Some(Some(
                            compiler
                                .resolve_static_sync_iterator_method_call_outcome(&method_value)?,
                        ))
                    }
                };
            }
            if let Some(function_binding) =
                compiler.resolve_member_function_binding(expression, property)
            {
                return Some(Some(compiler.validate_static_sync_iterator_call_outcome(
                    compiler.resolve_static_function_outcome_from_binding_with_context(
                        &function_binding,
                        &[],
                        current_function_name,
                    )?,
                )?));
            }
            if let Some(object_binding) =
                compiler.resolve_object_binding_from_expression(expression)
            {
                let Some(method_value) = object_binding_lookup_value(&object_binding, property)
                else {
                    return Some(None);
                };
                if matches!(method_value, Expression::Undefined | Expression::Null) {
                    return Some(None);
                }
                return Some(Some(
                    compiler.resolve_static_sync_iterator_method_call_outcome(method_value)?,
                ));
            }
            Some(None)
        };
        let call_outcome =
            match resolve_iterator_method_call_outcome(self, &async_iterator_property)? {
                Some(outcome) => outcome,
                None => match resolve_iterator_method_call_outcome(self, &iterator_property)? {
                    Some(outcome) => outcome,
                    None => {
                        if let Some(source) = self.resolve_iterator_source_kind(expression)
                            && let Some(flattened) =
                                self.flatten_simple_yield_delegate_iterator_source(&source)
                        {
                            return Some(flattened);
                        }
                        return None;
                    }
                },
            };

        match call_outcome {
            StaticEvalOutcome::Throw(throw_value) => self.simple_generator_throw_step(throw_value),
            StaticEvalOutcome::Value(iterator_value) => {
                if !self
                    .static_iterator_method_result_is_object(&iterator_value, current_function_name)
                {
                    return self
                        .simple_generator_throw_step(StaticThrowValue::NamedError("TypeError"));
                }
                if let Some(source) = self.resolve_iterator_source_kind(&iterator_value) {
                    if let Some(flattened) =
                        self.flatten_simple_yield_delegate_iterator_source(&source)
                    {
                        return Some(flattened);
                    }
                }
                let return_property = Expression::String("return".to_string());
                let throw_property = Expression::String("throw".to_string());
                if self.expression_has_defined_member_entry(&iterator_value, &return_property)
                    || self.expression_has_defined_member_entry(&iterator_value, &throw_property)
                {
                    return None;
                }
                self.resolve_simple_async_iterator_next_source(&iterator_value)
            }
        }
    }
}
