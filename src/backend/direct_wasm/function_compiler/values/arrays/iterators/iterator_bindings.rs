use super::*;

fn is_internal_array_iterator_binding_name(name: &str) -> bool {
    name.strip_prefix("__ayy_array_iter_")
        .is_some_and(|suffix| !suffix.is_empty() && suffix.chars().all(|ch| ch.is_ascii_digit()))
}

impl<'a> FunctionCompiler<'a> {
    pub(in crate::backend::direct_wasm) fn existing_iterator_binding_for_expression(
        &self,
        expression: &Expression,
        remaining_depth: usize,
    ) -> Option<ArrayIteratorBinding> {
        if remaining_depth == 0 {
            return None;
        }
        match expression {
            Expression::Identifier(name) => {
                let binding_name = self.resolve_local_array_iterator_binding_name(name)?;
                self.state
                    .speculation
                    .static_semantics
                    .local_array_iterator_binding(&binding_name)
                    .cloned()
            }
            Expression::Member { object, property } if matches!(property.as_ref(), Expression::String(name) if name == "value") =>
            {
                let step_values = if let Some(IteratorStepBinding::Runtime {
                    static_value,
                    value_candidates,
                    ..
                }) = self.resolve_iterator_step_binding_from_expression(object)
                {
                    let mut step_values = Vec::new();
                    if let Some(static_value) = static_value {
                        step_values.push(static_value);
                    }
                    step_values.extend(value_candidates);
                    step_values
                } else {
                    Vec::new()
                };
                for step_value in step_values {
                    let materialized = self.materialize_static_expression(&step_value);
                    if let Some(binding) = self
                        .existing_iterator_binding_for_expression(
                            &materialized,
                            remaining_depth.saturating_sub(1),
                        )
                        .or_else(|| {
                            self.existing_iterator_binding_for_expression(
                                &step_value,
                                remaining_depth.saturating_sub(1),
                            )
                        })
                    {
                        return Some(binding);
                    }
                }
                None
            }
            Expression::Conditional {
                then_expression,
                else_expression,
                ..
            } if matches!(else_expression.as_ref(), Expression::Undefined) => self
                .existing_iterator_binding_for_expression(
                    then_expression,
                    remaining_depth.saturating_sub(1),
                ),
            _ => {
                let materialized = self.materialize_static_expression(expression);
                (!static_expression_matches(&materialized, expression))
                    .then(|| {
                        self.existing_iterator_binding_for_expression(
                            &materialized,
                            remaining_depth.saturating_sub(1),
                        )
                    })
                    .flatten()
            }
        }
    }

    fn update_local_array_iterator_binding_with_source_and_shared_state(
        &mut self,
        name: &str,
        source: Option<IteratorSourceKind>,
        shared_state: Option<ArrayIteratorBinding>,
    ) {
        let Some(source) = source else {
            self.state
                .speculation
                .static_semantics
                .clear_local_array_iterator_binding(name);
            return;
        };
        let index_local = self
            .resolve_local_array_iterator_binding_name(name)
            .and_then(|binding_name| {
                self.state
                    .speculation
                    .static_semantics
                    .local_array_iterator_binding(&binding_name)
            })
            .map(|binding| binding.index_local)
            .or_else(|| shared_state.as_ref().map(|binding| binding.index_local))
            .unwrap_or_else(|| self.allocate_temp_local());
        let static_index = match &source {
            IteratorSourceKind::StaticArray { length_local, .. }
                if length_local.is_none() || is_internal_array_iterator_binding_name(name) =>
            {
                Some(0)
            }
            IteratorSourceKind::StaticArrayEntries { length_local, .. }
                if length_local.is_none() || is_internal_array_iterator_binding_name(name) =>
            {
                Some(0)
            }
            IteratorSourceKind::StaticMapEntries { length_local, .. }
                if length_local.is_none() || is_internal_array_iterator_binding_name(name) =>
            {
                Some(0)
            }
            IteratorSourceKind::SimpleGenerator { .. } => Some(0),
            IteratorSourceKind::AsyncYieldDelegateGenerator { .. } => Some(0),
            _ => None,
        };
        let static_index = shared_state
            .as_ref()
            .and_then(|binding| binding.static_index)
            .or(static_index);
        self.state
            .speculation
            .static_semantics
            .set_local_array_iterator_binding(
                name,
                ArrayIteratorBinding {
                    source,
                    index_local,
                    static_index,
                },
            );
        if std::env::var_os("AYY_TRACE_SIMPLE_GENERATORS").is_some() {
            let source_kind = match self
                .state
                .speculation
                .static_semantics
                .local_array_iterator_binding(name)
                .map(|binding| &binding.source)
            {
                Some(IteratorSourceKind::StaticArray { .. }) => "StaticArray",
                Some(IteratorSourceKind::StaticArrayEntries { .. }) => "StaticArrayEntries",
                Some(IteratorSourceKind::StaticMapEntries { .. }) => "StaticMapEntries",
                Some(IteratorSourceKind::SimpleGenerator { .. }) => "SimpleGenerator",
                Some(IteratorSourceKind::AsyncYieldDelegateGenerator { .. }) => {
                    "AsyncYieldDelegateGenerator"
                }
                Some(IteratorSourceKind::TypedArrayView { .. }) => "TypedArrayView",
                Some(IteratorSourceKind::DirectArguments { .. }) => "DirectArguments",
                None => "None",
            };
            eprintln!(
                "iterator_binding_update name={name} source={source_kind} static_index={static_index:?}"
            );
        }
        if shared_state.is_none() {
            self.push_i32_const(0);
            self.push_local_set(index_local);
        }
        self.state
            .speculation
            .static_semantics
            .set_local_kind(name, StaticValueKind::Object);
    }

    pub(in crate::backend::direct_wasm) fn update_local_array_iterator_binding_with_source(
        &mut self,
        name: &str,
        source: Option<IteratorSourceKind>,
    ) {
        self.update_local_array_iterator_binding_with_source_and_shared_state(name, source, None);
    }

    pub(in crate::backend::direct_wasm) fn update_local_array_iterator_binding(
        &mut self,
        name: &str,
        value: &Expression,
    ) {
        let source = self.resolve_local_array_iterator_source(value);
        let shared_state = match value {
            Expression::GetIterator(iterated) => {
                self.existing_iterator_binding_for_expression(iterated, 6)
            }
            _ => None,
        };
        self.update_local_array_iterator_binding_with_source_and_shared_state(
            name,
            source,
            shared_state,
        );
    }
}
