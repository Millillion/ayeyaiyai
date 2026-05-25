use super::*;

fn is_internal_declaration_array_binding_name(name: &str) -> bool {
    name.strip_prefix("__ayy_decl_")
        .is_some_and(|suffix| !suffix.is_empty() && suffix.chars().all(|ch| ch.is_ascii_digit()))
}

impl<'a> FunctionCompiler<'a> {
    fn internal_iterator_value_source_cache_key(&self, expression: &Expression) -> Option<String> {
        let Expression::Identifier(name) = expression else {
            return None;
        };
        if !(name.starts_with("__ayy_array_iter_value_")
            || name.starts_with("__ayy_for_of_iter_value_"))
        {
            return None;
        }
        let value_binding = self
            .state
            .speculation
            .static_semantics
            .local_value_binding(name)
            .or_else(|| self.global_value_binding(name));
        Some(format!("{name}|{value_binding:?}"))
    }

    fn resolve_single_iterator_step_value_source_kind(
        &self,
        expression: &Expression,
    ) -> Option<IteratorSourceKind> {
        let Expression::Conditional {
            condition,
            then_expression,
            else_expression,
        } = expression
        else {
            return None;
        };
        if !matches!(else_expression.as_ref(), Expression::Undefined) {
            return None;
        }
        let Expression::Binary {
            op: BinaryOp::Equal,
            left,
            right,
        } = condition.as_ref()
        else {
            return None;
        };
        if !matches!(right.as_ref(), Expression::Bool(false)) {
            return None;
        }
        let Expression::Member {
            object: done_object,
            property: done_property,
        } = left.as_ref()
        else {
            return None;
        };
        if !matches!(done_property.as_ref(), Expression::String(name) if name == "done") {
            return None;
        }
        let Expression::Member {
            object: value_object,
            property: value_property,
        } = then_expression.as_ref()
        else {
            return None;
        };
        if !static_expression_matches(done_object, value_object)
            || !matches!(value_property.as_ref(), Expression::String(name) if name == "value")
        {
            return None;
        }
        let Some(IteratorStepBinding::Runtime {
            value_candidates, ..
        }) = self.resolve_iterator_step_binding_from_expression(done_object)
        else {
            return None;
        };
        let [candidate] = value_candidates.as_slice() else {
            return None;
        };
        self.resolve_iterator_source_kind(&self.materialize_static_expression(candidate))
    }

    fn resolve_iterator_step_member_value_source_kind(
        &self,
        expression: &Expression,
    ) -> Option<IteratorSourceKind> {
        let Expression::Member { object, property } = expression else {
            return None;
        };
        if !matches!(property.as_ref(), Expression::String(name) if name == "value") {
            return None;
        }
        let Some(IteratorStepBinding::Runtime {
            static_value,
            value_candidates,
            ..
        }) = self.resolve_iterator_step_binding_from_expression(object)
        else {
            return None;
        };
        if let Some(value) = static_value {
            return self.resolve_iterator_source_kind(&self.materialize_static_expression(&value));
        }
        let [candidate] = value_candidates.as_slice() else {
            return None;
        };
        self.resolve_iterator_source_kind(&self.materialize_static_expression(candidate))
    }

    fn resolve_for_in_key_member_string_iterator_source(
        &self,
        expression: &Expression,
    ) -> Option<IteratorSourceKind> {
        let Expression::Member { object, property } = expression else {
            return None;
        };
        let Expression::Identifier(object_name) = object.as_ref() else {
            return None;
        };
        if !object_name.starts_with("__ayy_for_in_keys_") {
            return None;
        }

        let array_binding = self.resolve_array_binding_from_expression(object)?;
        let materialized_property = self
            .resolve_property_key_expression(property)
            .unwrap_or_else(|| self.materialize_static_expression(property));
        let string_value =
            if let Some(index) = argument_index_from_expression(&materialized_property) {
                match array_binding.values.get(index as usize)? {
                    Some(Expression::String(value)) => value.clone(),
                    _ => return None,
                }
            } else if array_binding.values.len() == 1 {
                match array_binding.values.first()? {
                    Some(Expression::String(value)) => value.clone(),
                    _ => return None,
                }
            } else {
                return None;
            };

        Some(IteratorSourceKind::StaticArray {
            values: string_value
                .chars()
                .map(|character| Some(Expression::String(character.to_string())))
                .collect(),
            keys_only: false,
            length_local: None,
            runtime_name: None,
        })
    }

    pub(in crate::backend::direct_wasm) fn resolve_for_await_step_value_iterator_target(
        &self,
        expression: &Expression,
    ) -> Option<Expression> {
        let Expression::Member { object, property } = expression else {
            return None;
        };
        if !matches!(property.as_ref(), Expression::String(name) if name == "value") {
            return None;
        }
        let Expression::Identifier(step_name) = object.as_ref() else {
            return None;
        };
        if !step_name.starts_with("__ayy_for_of_step_") {
            return None;
        }
        let awaited_expression = Expression::Await(Box::new(expression.clone()));
        if let Some(StaticEvalOutcome::Value(awaited_value)) =
            self.resolve_static_await_resolution_outcome(&awaited_expression)
        {
            if !static_expression_matches(&awaited_value, expression) {
                return Some(awaited_value);
            }
        }
        let materialized = self.materialize_static_expression(&awaited_expression);
        (!static_expression_matches(&materialized, &awaited_expression)).then_some(materialized)
    }

    pub(in crate::backend::direct_wasm) fn resolve_for_await_step_value_iterator_source_kind(
        &self,
        expression: &Expression,
    ) -> Option<IteratorSourceKind> {
        let awaited_value = self.resolve_for_await_step_value_iterator_target(expression)?;
        self.resolve_iterator_source_kind(&awaited_value)
    }

    pub(in crate::backend::direct_wasm) fn is_async_generator_call_expression(
        &self,
        expression: &Expression,
    ) -> bool {
        let Expression::Call { callee, .. } = expression else {
            return false;
        };
        let Some(LocalFunctionBinding::User(function_name)) =
            self.resolve_function_binding_from_expression(callee)
        else {
            return false;
        };
        self.user_function(&function_name)
            .is_some_and(|function| matches!(function.kind, FunctionKind::AsyncGenerator))
    }

    pub(in crate::backend::direct_wasm) fn tracked_direct_arguments_prefix_len(&self) -> u32 {
        let mut indices = self
            .state
            .parameters
            .arguments_slots
            .keys()
            .copied()
            .collect::<Vec<_>>();
        indices.sort_unstable();
        let mut next_index = 0;
        for index in indices {
            if index != next_index {
                break;
            }
            next_index += 1;
        }
        next_index
    }

    pub(in crate::backend::direct_wasm) fn resolve_iterator_source_kind(
        &self,
        expression: &Expression,
    ) -> Option<IteratorSourceKind> {
        let structural_key = format!("{expression:?}");
        let trace_focus = std::env::var("AYY_TRACE_ITERATOR_SOURCE_FOCUS").ok();
        let trace_iterator_source = std::env::var_os("AYY_TRACE_ITERATOR_SOURCE_KIND").is_some()
            && trace_focus
                .as_ref()
                .is_none_or(|focus| structural_key.contains(focus));
        macro_rules! trace_probe {
            ($label:expr) => {
                if trace_iterator_source {
                    eprintln!(
                        "iterator_source_kind:probe:{} expression={:?}",
                        $label, expression
                    );
                }
            };
        }
        macro_rules! trace_source {
            ($label:expr) => {
                if trace_iterator_source {
                    eprintln!(
                        "iterator_source_kind:{} expression={:?}",
                        $label, expression
                    );
                }
            };
        }
        let inserted = ACTIVE_ITERATOR_SOURCE_SHAPES
            .with(|active| active.borrow_mut().insert(structural_key.clone()));
        if !inserted {
            return None;
        }
        let _guard = IteratorSourceGuard {
            key: structural_key,
        };
        let expression_is_internal_iterator_value = matches!(
            expression,
            Expression::Identifier(name)
                if name.starts_with("__ayy_array_iter_value_")
                    || name.starts_with("__ayy_for_of_iter_value_")
        );
        let internal_iterator_value_cache_key =
            self.internal_iterator_value_source_cache_key(expression);
        if let Some(cache_key) = internal_iterator_value_cache_key.as_ref()
            && let Some(cached) = INTERNAL_ITERATOR_VALUE_SOURCE_CACHE
                .with(|cache| cache.borrow().get(cache_key).cloned())
        {
            return cached;
        }
        trace_probe!("direct-arguments:start");
        if self.is_direct_arguments_object(expression) {
            trace_source!("direct-arguments");
            return Some(IteratorSourceKind::DirectArguments {
                tracked_prefix_len: self.tracked_direct_arguments_prefix_len(),
            });
        }
        trace_probe!("await:start");
        if let Expression::Await(_) = expression
            && let Some(StaticEvalOutcome::Value(awaited_value)) =
                self.resolve_static_await_resolution_outcome(expression)
        {
            return self.resolve_iterator_source_kind(&awaited_value);
        }
        trace_probe!("sequence:start");
        if let Expression::Sequence(expressions) = expression
            && let Some(last) = expressions.last()
            && !static_expression_matches(last, expression)
        {
            return self.resolve_iterator_source_kind(last);
        }
        trace_probe!("typed-array-local:start");
        if let Expression::Identifier(name) = expression
            && self
                .state
                .speculation
                .static_semantics
                .has_local_typed_array_view_binding(name)
        {
            trace_source!("typed-array-local");
            return Some(IteratorSourceKind::TypedArrayView { name: name.clone() });
        }
        trace_probe!("typed-array-resolved:start");
        if let Expression::Identifier(name) = expression
            && let Some((resolved_name, _)) = self.resolve_current_local_binding(name)
            && self
                .state
                .speculation
                .static_semantics
                .has_local_typed_array_view_binding(&resolved_name)
        {
            return Some(IteratorSourceKind::TypedArrayView {
                name: resolved_name,
            });
        }
        trace_probe!("existing-local-array-iterator:start");
        if let Expression::Identifier(name) = expression
            && let Some(binding_name) = self.resolve_local_array_iterator_binding_name(name)
            && let Some(binding) = self
                .state
                .speculation
                .static_semantics
                .local_array_iterator_binding(&binding_name)
        {
            trace_source!("existing-local-array-iterator");
            return Some(binding.source.clone());
        }
        if expression_is_internal_iterator_value
            && let Expression::Identifier(name) = expression
            && let Some(value) = self
                .state
                .speculation
                .static_semantics
                .local_value_binding(name)
                .or_else(|| self.global_value_binding(name))
            && !static_expression_matches(value, expression)
        {
            if let Some(source) = self.resolve_iterator_source_kind(value) {
                if let Some(cache_key) = internal_iterator_value_cache_key.as_ref() {
                    INTERNAL_ITERATOR_VALUE_SOURCE_CACHE.with(|cache| {
                        cache
                            .borrow_mut()
                            .insert(cache_key.clone(), Some(source.clone()));
                    });
                }
                trace_source!("internal-iterator-value-binding");
                return Some(source);
            }
            let materialized_value = self.materialize_static_expression(value);
            if !static_expression_matches(&materialized_value, value)
                && !static_expression_matches(&materialized_value, expression)
                && let Some(source) = self.resolve_iterator_source_kind(&materialized_value)
            {
                if let Some(cache_key) = internal_iterator_value_cache_key.as_ref() {
                    INTERNAL_ITERATOR_VALUE_SOURCE_CACHE.with(|cache| {
                        cache
                            .borrow_mut()
                            .insert(cache_key.clone(), Some(source.clone()));
                    });
                }
                trace_source!("internal-iterator-materialized-value-binding");
                return Some(source);
            }
            if let Some(cache_key) = internal_iterator_value_cache_key.as_ref() {
                INTERNAL_ITERATOR_VALUE_SOURCE_CACHE.with(|cache| {
                    cache.borrow_mut().insert(cache_key.clone(), None);
                });
            }
            return None;
        }
        trace_probe!("single-step-value:start");
        if let Some(source) = self.resolve_single_iterator_step_value_source_kind(expression) {
            trace_source!("single-step-value");
            return Some(source);
        }
        trace_probe!("step-member-value:start");
        if let Some(source) = self.resolve_iterator_step_member_value_source_kind(expression) {
            trace_source!("step-member-value");
            return Some(source);
        }
        trace_probe!("array-prototype-simple-generator:start");
        if let Some((steps, completion_effects, completion_value)) =
            self.resolve_array_prototype_simple_generator_source(expression)
        {
            trace_source!("array-prototype-simple-generator");
            return Some(IteratorSourceKind::SimpleGenerator {
                is_async: false,
                steps,
                completion_effects,
                completion_value,
            });
        }
        trace_probe!("static-array:start");
        if let Some(source) = self.resolve_static_array_iterator_source_kind(expression, false) {
            trace_source!("static-array");
            return Some(source);
        }
        trace_probe!("static-map:start");
        if let Some(source) = self.resolve_static_map_iterator_source_kind(expression) {
            trace_source!("static-map");
            return Some(source);
        }
        trace_probe!("for-in-key-member-string:start");
        if let Some(source) = self.resolve_for_in_key_member_string_iterator_source(expression) {
            trace_source!("for-in-key-member-string");
            return Some(source);
        }
        trace_probe!("string:start");
        if let Expression::String(text) = expression {
            trace_source!("string");
            return Some(IteratorSourceKind::StaticArray {
                values: text
                    .chars()
                    .map(|character| Some(Expression::String(character.to_string())))
                    .collect(),
                keys_only: false,
                length_local: None,
                runtime_name: None,
            });
        }
        trace_probe!("identifier-static-iterator-object-simple-generator:start");
        if matches!(expression, Expression::Identifier(_))
            && let Some((steps, completion_effects, completion_value)) =
                self.resolve_static_iterator_object_simple_generator_source(expression)
        {
            trace_source!("identifier-static-iterator-object-simple-generator");
            return Some(IteratorSourceKind::SimpleGenerator {
                is_async: false,
                steps,
                completion_effects,
                completion_value,
            });
        }
        trace_probe!("identifier-binding:start");
        if let Expression::Identifier(name) = expression
            && let Some(value) = self
                .state
                .speculation
                .static_semantics
                .local_value_binding(name)
                .or_else(|| self.global_value_binding(name))
            && !static_expression_matches(value, expression)
            && let Some(source) = self.resolve_iterator_source_kind(value)
        {
            trace_source!("identifier-binding");
            return Some(source);
        }
        trace_probe!("simple-generator:start");
        if let Some((steps, completion_effects, completion_value)) =
            self.resolve_simple_generator_source(expression)
        {
            let is_async = matches!(
                expression,
                Expression::Call { callee, .. }
                    if self
                        .resolve_function_binding_from_expression(callee)
                        .and_then(|binding| match binding {
                            LocalFunctionBinding::User(function_name) => {
                                self.user_function(&function_name)
                            }
                            LocalFunctionBinding::Builtin(_) => None,
                        })
                        .is_some_and(|function| matches!(function.kind, FunctionKind::AsyncGenerator))
            );
            trace_source!("simple-generator");
            return Some(IteratorSourceKind::SimpleGenerator {
                is_async,
                steps,
                completion_effects,
                completion_value,
            });
        }
        trace_probe!("materialize:start");
        let materialized = self.materialize_static_expression(expression);
        trace_probe!("materialize:done");
        if !static_expression_matches(&materialized, expression) {
            if let Some(source) = self.resolve_iterator_source_kind(&materialized) {
                trace_source!("materialized");
                return Some(source);
            }
        }
        trace_probe!("call-shape:start");
        if let Some(source) = self.resolve_iterator_source_call_shape_kind(expression) {
            trace_source!("call-shape");
            return Some(source);
        }
        trace_probe!("effectful-call:start");
        if let Some((_, returned_expression, _)) =
            self.analyze_effectful_iterator_source_call(expression)
        {
            trace_source!("effectful-call");
            return self.resolve_iterator_source_kind(&returned_expression);
        }
        trace_probe!("static-iterator-object-simple-generator:start");
        if let Some((steps, completion_effects, completion_value)) =
            self.resolve_static_iterator_object_simple_generator_source(expression)
        {
            trace_source!("static-iterator-object-simple-generator");
            return Some(IteratorSourceKind::SimpleGenerator {
                is_async: false,
                steps,
                completion_effects,
                completion_value,
            });
        }
        trace_probe!("static-iterable-simple-generator:start");
        if let Some((steps, completion_effects, completion_value)) =
            self.resolve_static_iterable_simple_generator_source(expression)
        {
            trace_source!("static-iterable-simple-generator");
            return Some(IteratorSourceKind::SimpleGenerator {
                is_async: false,
                steps,
                completion_effects,
                completion_value,
            });
        }
        trace_probe!("static-iterable-binding:start");
        let binding = self.resolve_static_iterable_binding_from_expression(expression)?;
        trace_source!("static-iterable-binding");
        Some(IteratorSourceKind::StaticArray {
            values: binding.values,
            keys_only: false,
            length_local: None,
            runtime_name: None,
        })
    }

    fn resolve_static_map_iterator_source_kind(
        &self,
        expression: &Expression,
    ) -> Option<IteratorSourceKind> {
        let object_binding = self.resolve_object_binding_from_expression(expression)?;
        if !self.object_binding_is_static_map(&object_binding) {
            return None;
        }
        let values = self.static_map_entries_from_binding(&object_binding)?;
        let collection_kind = self.static_map_kind_from_binding(&object_binding)?;
        let collection_name = self.static_collection_identifier_name(expression);
        let length_local = collection_name.as_ref().and_then(|name| {
            self.state
                .speculation
                .static_semantics
                .runtime_array_length_local(name)
        });
        if collection_kind == "Map" {
            let (key_runtime_name, value_runtime_name) = collection_name
                .as_ref()
                .filter(|_| length_local.is_some())
                .map(|name| {
                    (
                        Self::static_map_key_runtime_name(name),
                        Self::static_map_value_runtime_name(name),
                    )
                })
                .map_or((None, None), |(key, value)| (Some(key), Some(value)));
            return Some(IteratorSourceKind::StaticMapEntries {
                values,
                length_local,
                key_runtime_name,
                value_runtime_name,
            });
        }
        let runtime_name = collection_name.filter(|name| {
            self.state
                .speculation
                .static_semantics
                .runtime_array_length_local(name)
                .is_some()
        });
        Some(IteratorSourceKind::StaticArray {
            values,
            keys_only: false,
            length_local,
            runtime_name,
        })
    }

    fn resolve_static_array_iterator_source_kind(
        &self,
        expression: &Expression,
        keys_only: bool,
    ) -> Option<IteratorSourceKind> {
        if self.array_prototype_symbol_iterator_deleted_affects(expression) {
            return None;
        }
        let array_binding = self.resolve_array_binding_from_expression(expression)?;
        let length_local = match expression {
            Expression::Identifier(name) if is_internal_declaration_array_binding_name(name) => {
                None
            }
            Expression::Identifier(name)
                if self.is_named_global_array_binding(name)
                    && (!self.state.speculation.execution_context.top_level_function
                        || self.uses_global_runtime_array_state(name)) =>
            {
                None
            }
            _ => self.runtime_array_length_local_for_expression(expression),
        };
        Some(IteratorSourceKind::StaticArray {
            values: array_binding.values,
            keys_only,
            length_local,
            runtime_name: match expression {
                Expression::Identifier(name)
                    if is_internal_declaration_array_binding_name(name) =>
                {
                    None
                }
                Expression::Identifier(name)
                    if self
                        .runtime_array_length_local_for_expression(expression)
                        .is_some()
                        || (self.is_named_global_array_binding(name)
                            && (!self.state.speculation.execution_context.top_level_function
                                || self.uses_global_runtime_array_state(name))) =>
                {
                    Some(name.clone())
                }
                _ => None,
            },
        })
    }

    fn resolve_iterator_source_call_shape_kind(
        &self,
        expression: &Expression,
    ) -> Option<IteratorSourceKind> {
        let Expression::Call { callee, arguments } = expression else {
            return None;
        };
        if !arguments.is_empty() {
            return None;
        }
        let Expression::Member { object, property } = callee.as_ref() else {
            return None;
        };
        if is_symbol_iterator_expression(property) {
            return self.resolve_iterator_source_kind(object);
        }
        if matches!(property.as_ref(), Expression::String(name) if name == "keys") {
            return self.resolve_static_array_iterator_source_kind(object, true);
        }
        if matches!(property.as_ref(), Expression::String(name) if name == "entries") {
            let IteratorSourceKind::StaticArray {
                values,
                length_local,
                runtime_name,
                ..
            } = self.resolve_static_array_iterator_source_kind(object, false)?
            else {
                return None;
            };
            return Some(IteratorSourceKind::StaticArrayEntries {
                values,
                length_local,
                runtime_name,
            });
        }
        None
    }
}
