use super::*;

impl<'a> FunctionCompiler<'a> {
    fn runtime_array_name_has_static_literal_value(&self, name: &str) -> bool {
        self.state
            .speculation
            .static_semantics
            .local_value_binding(name)
            .or_else(|| self.global_value_binding(name))
            .map(|value| self.materialize_static_expression(value))
            .is_some_and(|value| matches!(value, Expression::Array(_)))
    }

    pub(in crate::backend::direct_wasm) fn static_array_source_has_dynamic_length(
        &self,
        length_local: Option<u32>,
        runtime_name: Option<&str>,
    ) -> bool {
        if length_local.is_none() && runtime_name.is_none() {
            return false;
        }
        match runtime_name {
            Some(name) => !self.runtime_array_name_has_static_literal_value(name),
            None => true,
        }
    }

    fn runtime_name_fixed_typed_array_length(&self, name: &str) -> Option<usize> {
        let identifier = Expression::Identifier(name.to_string());
        self.resolve_typed_array_view_binding_from_expression(&identifier)
            .and_then(|view| view.fixed_length)
            .or_else(|| {
                let value = self
                    .state
                    .speculation
                    .static_semantics
                    .local_value_binding(name)
                    .or_else(|| self.global_value_binding(name))?;
                if static_expression_matches(value, &identifier) {
                    return None;
                }
                self.resolve_typed_array_view_binding_from_expression(value)
                    .and_then(|view| view.fixed_length)
            })
    }

    pub(in crate::backend::direct_wasm) fn static_array_source_exhaustion_length(
        &self,
        values_len: usize,
        length_local: Option<u32>,
        runtime_name: Option<&str>,
    ) -> Option<usize> {
        if !self.static_array_source_has_dynamic_length(length_local, runtime_name) {
            return Some(values_len);
        }
        runtime_name.and_then(|name| self.runtime_name_fixed_typed_array_length(name))
    }

    pub(in crate::backend::direct_wasm) fn resolve_iterator_step_static_outcome(
        &self,
        iterator_binding: &ArrayIteratorBinding,
        current_static_index: Option<usize>,
        sent_value: &Expression,
    ) -> (Option<bool>, Option<Expression>) {
        let outcome = match &iterator_binding.source {
            IteratorSourceKind::StaticArray {
                values,
                keys_only,
                length_local,
                runtime_name,
            } => {
                let has_dynamic_length = self
                    .static_array_source_has_dynamic_length(*length_local, runtime_name.as_deref());
                let exhaustion_length = self.static_array_source_exhaustion_length(
                    values.len(),
                    *length_local,
                    runtime_name.as_deref(),
                );
                let static_done = current_static_index.and_then(|index| {
                    if exhaustion_length.is_some_and(|length| index >= length) {
                        Some(true)
                    } else if !has_dynamic_length {
                        Some(false)
                    } else {
                        None
                    }
                });
                let static_value = current_static_index.and_then(|index| {
                    if exhaustion_length.is_some_and(|length| index >= length) {
                        Some(Expression::Undefined)
                    } else if index >= values.len() && has_dynamic_length {
                        None
                    } else if index >= values.len() {
                        Some(Expression::Undefined)
                    } else if *keys_only {
                        Some(Expression::Number(index as f64))
                    } else {
                        Some(
                            values
                                .get(index)
                                .and_then(|value| value.clone())
                                .unwrap_or(Expression::Undefined),
                        )
                    }
                });
                let static_value = static_value.map(|value| {
                    if current_static_index.is_some_and(|index| index >= values.len()) {
                        Expression::Undefined
                    } else {
                        value
                    }
                });
                (static_done, static_value)
            }
            IteratorSourceKind::StaticArrayEntries {
                values,
                length_local,
                runtime_name,
            } => {
                let has_dynamic_length = self
                    .static_array_source_has_dynamic_length(*length_local, runtime_name.as_deref());
                let exhaustion_length = self.static_array_source_exhaustion_length(
                    values.len(),
                    *length_local,
                    runtime_name.as_deref(),
                );
                let static_done = current_static_index.and_then(|index| {
                    if exhaustion_length.is_some_and(|length| index >= length) {
                        Some(true)
                    } else if !has_dynamic_length {
                        Some(false)
                    } else {
                        None
                    }
                });
                let static_value = current_static_index.and_then(|index| {
                    if exhaustion_length.is_some_and(|length| index >= length) {
                        Some(Expression::Undefined)
                    } else if index >= values.len() && has_dynamic_length {
                        None
                    } else if index >= values.len() {
                        Some(Expression::Undefined)
                    } else {
                        Some(Expression::Array(vec![
                            ArrayElement::Expression(Expression::Number(index as f64)),
                            ArrayElement::Expression(
                                values
                                    .get(index)
                                    .and_then(|value| value.clone())
                                    .unwrap_or(Expression::Undefined),
                            ),
                        ]))
                    }
                });
                let static_value = static_value.map(|value| {
                    if current_static_index.is_some_and(|index| index >= values.len()) {
                        Expression::Undefined
                    } else {
                        value
                    }
                });
                (static_done, static_value)
            }
            IteratorSourceKind::StaticMapEntries {
                values,
                length_local,
                key_runtime_name,
                value_runtime_name,
            } => {
                let has_dynamic_length = length_local.is_some()
                    || key_runtime_name.is_some()
                    || value_runtime_name.is_some();
                let static_done = (!has_dynamic_length)
                    .then(|| current_static_index.map(|index| index >= values.len()))
                    .flatten();
                let static_value = current_static_index.and_then(|index| {
                    if index >= values.len() && has_dynamic_length {
                        None
                    } else if index >= values.len() {
                        Some(Expression::Undefined)
                    } else {
                        values
                            .get(index)
                            .and_then(|value| value.clone())
                            .or(Some(Expression::Undefined))
                    }
                });
                let static_value = static_value.map(|value| {
                    if current_static_index.is_some_and(|index| index >= values.len()) {
                        Expression::Undefined
                    } else {
                        value
                    }
                });
                (static_done, static_value)
            }
            IteratorSourceKind::SimpleGenerator {
                steps,
                completion_value,
                ..
            } => {
                match current_static_index {
                    Some(index) if index < steps.len() => match &steps[index].outcome {
                        SimpleGeneratorStepOutcome::Yield(value) => (
                            Some(false),
                            Some(self.materialize_static_expression(
                                &Self::substitute_sent_expression(value, sent_value),
                            )),
                        ),
                        SimpleGeneratorStepOutcome::YieldResult(result) => (
                            Some(false),
                            Some(self.simple_generator_yield_result_value(result, sent_value)),
                        ),
                        SimpleGeneratorStepOutcome::Throw(_) => (None, None),
                    },
                    Some(index) if index == steps.len() => (
                        Some(true),
                        Some(self.materialize_static_expression(
                            &Self::substitute_sent_expression(completion_value, sent_value),
                        )),
                    ),
                    Some(_) => (Some(true), Some(Expression::Undefined)),
                    None => (None, None),
                }
            }
            IteratorSourceKind::TypedArrayView { name } => {
                let view = self.typed_array_view_binding_for_name(name);
                let static_length = view
                    .as_ref()
                    .and_then(|view| self.typed_array_view_static_length(view));
                let static_values = view
                    .as_ref()
                    .and_then(|view| self.typed_array_view_static_values(view));
                let static_done = static_length
                    .and_then(|length| current_static_index.map(|index| index >= length));
                let static_value = current_static_index.and_then(|index| {
                    if static_done == Some(true) {
                        Some(Expression::Undefined)
                    } else {
                        static_values
                            .as_ref()
                            .and_then(|values| values.values.get(index).cloned().flatten())
                            .or(Some(Expression::Undefined))
                    }
                });
                (static_done, static_value)
            }
            _ => (None, None),
        };
        if std::env::var_os("AYY_TRACE_SIMPLE_GENERATOR_ASSIGNMENT").is_some() {
            let source_kind = match &iterator_binding.source {
                IteratorSourceKind::StaticArray { values, .. } => {
                    format!("StaticArray(len={})", values.len())
                }
                IteratorSourceKind::StaticArrayEntries { values, .. } => {
                    format!("StaticArrayEntries(len={})", values.len())
                }
                IteratorSourceKind::StaticMapEntries { values, .. } => {
                    format!("StaticMapEntries(len={})", values.len())
                }
                IteratorSourceKind::SimpleGenerator { steps, .. } => {
                    format!("SimpleGenerator(steps={})", steps.len())
                }
                IteratorSourceKind::AsyncYieldDelegateGenerator { .. } => {
                    "AsyncYieldDelegateGenerator".to_string()
                }
                IteratorSourceKind::TypedArrayView { .. } => "TypedArrayView".to_string(),
                IteratorSourceKind::DirectArguments { .. } => "DirectArguments".to_string(),
            };
            eprintln!(
                "iterator_static_outcome source={source_kind} index={current_static_index:?} sent={sent_value:?} outcome={outcome:?}"
            );
        }
        outcome
    }
}
