use super::*;

impl<'a> FunctionCompiler<'a> {
    fn iterator_close_callee_local_binding_sources(
        &self,
        binding: &LocalFunctionBinding,
    ) -> HashSet<String> {
        let LocalFunctionBinding::User(function_name) = binding else {
            return HashSet::new();
        };
        self.resolve_registered_function_declaration(function_name)
            .map(collect_function_constructor_local_bindings)
            .unwrap_or_default()
            .into_iter()
            .map(|name| {
                scoped_binding_source_name(&name)
                    .unwrap_or(&name)
                    .to_string()
            })
            .collect()
    }

    fn iterator_close_updated_binding_is_callee_local(
        name: &str,
        callee_local_sources: &HashSet<String>,
    ) -> bool {
        let source_name = scoped_binding_source_name(name).unwrap_or(name);
        callee_local_sources.contains(source_name)
    }

    fn iterator_close_return_result_is_definitely_non_object(
        &self,
        binding: &LocalFunctionBinding,
    ) -> bool {
        let outcome = self
            .resolve_terminal_function_outcome_from_binding(binding, &[])
            .or_else(|| {
                self.resolve_function_binding_static_return_expression(binding, &[])
                    .map(StaticEvalOutcome::Value)
            });
        let value = outcome.as_ref().and_then(|outcome| match outcome {
            StaticEvalOutcome::Value(value) => Some(self.materialize_static_expression(&value)),
            StaticEvalOutcome::Throw(_) => None,
        });
        let kind = value
            .as_ref()
            .and_then(|return_value| self.infer_value_kind(return_value));
        let result = kind.is_some_and(|kind| {
            matches!(
                kind,
                StaticValueKind::Undefined
                    | StaticValueKind::Null
                    | StaticValueKind::Bool
                    | StaticValueKind::Number
                    | StaticValueKind::String
                    | StaticValueKind::BigInt
                    | StaticValueKind::Symbol
            )
        });
        if std::env::var_os("AYY_TRACE_ITERATOR_CLOSE").is_some() {
            let outcome_kind = match &outcome {
                Some(StaticEvalOutcome::Value(_)) => "value",
                Some(StaticEvalOutcome::Throw(_)) => "throw",
                None => "none",
            };
            eprintln!(
                "iterator_close:return_result_kind binding={binding:?} outcome={outcome_kind} materialized={value:?} kind={kind:?} non_object={result}"
            );
        }
        result
    }

    pub(in crate::backend::direct_wasm) fn array_prototype_symbol_iterator_deleted_binding_name()
    -> &'static str {
        "__ayy_builtin_deleted__Array_prototype_Symbol_iterator"
    }

    pub(in crate::backend::direct_wasm) fn is_array_prototype_symbol_iterator_member(
        &self,
        object: &Expression,
        property: &Expression,
    ) -> bool {
        let object_is_array_prototype = |candidate: &Expression| {
            matches!(
                candidate,
                Expression::Member {
                    object: prototype_owner,
                    property: prototype_property,
                } if matches!(prototype_owner.as_ref(), Expression::Identifier(name) if name == "Array")
                    && matches!(prototype_property.as_ref(), Expression::String(name) if name == "prototype")
            )
        };
        let property_is_symbol_iterator = |candidate: &Expression| {
            is_symbol_iterator_expression(candidate)
                || self
                    .well_known_symbol_name(candidate)
                    .is_some_and(|name| name == "Symbol.iterator")
        };
        let materialized_object = self.materialize_static_expression(object);
        let materialized_property = self.materialize_static_expression(property);
        (object_is_array_prototype(object) || object_is_array_prototype(&materialized_object))
            && (property_is_symbol_iterator(property)
                || property_is_symbol_iterator(&materialized_property))
    }

    pub(in crate::backend::direct_wasm) fn emit_array_prototype_symbol_iterator_deleted_marker(
        &mut self,
        deleted: bool,
    ) -> DirectResult<()> {
        let value = Expression::Bool(deleted);
        let binding_name = Self::array_prototype_symbol_iterator_deleted_binding_name();
        self.update_static_global_assignment_metadata(binding_name, &value);
        let binding = self.ensure_implicit_global_binding(binding_name);
        self.backend
            .shared_global_semantics
            .set_global_binding_kind(binding_name, StaticValueKind::Bool);
        self.backend
            .shared_global_semantics
            .values
            .set_value_binding(binding_name.to_string(), value.clone());
        self.emit_numeric_expression(&value)?;
        self.push_global_set(binding.value_index);
        self.push_i32_const(1);
        self.push_global_set(binding.present_index);
        Ok(())
    }

    pub(in crate::backend::direct_wasm) fn array_prototype_symbol_iterator_is_deleted(
        &self,
    ) -> bool {
        matches!(
            self.global_value_binding(Self::array_prototype_symbol_iterator_deleted_binding_name()),
            Some(Expression::Bool(true))
        )
    }

    pub(in crate::backend::direct_wasm) fn array_prototype_symbol_iterator_deleted_affects(
        &self,
        expression: &Expression,
    ) -> bool {
        self.array_prototype_symbol_iterator_is_deleted()
            && self
                .resolve_array_binding_from_expression(expression)
                .is_some()
    }

    fn iterator_close_source_expression(&self, expression: &Expression) -> Option<Expression> {
        let Expression::Identifier(name) = expression else {
            return None;
        };
        self.state
            .speculation
            .static_semantics
            .local_value_binding(name)
            .cloned()
    }

    fn iterator_close_target_expression(&self, expression: &Expression) -> Option<Expression> {
        let source = self.iterator_close_source_expression(expression)?;
        let target = match source {
            Expression::GetIterator(iterated) => {
                self.resolve_static_get_iterator_value(iterated.as_ref(), &[])?
            }
            Expression::Call { callee, arguments } => self
                .resolve_static_call_result_expression_with_context(
                    callee.as_ref(),
                    &arguments,
                    self.current_function_name(),
                )
                .map(|(value, _)| value)?,
            other => other,
        };
        (!static_expression_matches(&target, expression)).then_some(target)
    }

    fn iterator_close_source_return_getter_binding(
        &self,
        expression: &Expression,
        return_property: &Expression,
    ) -> Option<LocalFunctionBinding> {
        if let Some(binding) = self.resolve_member_getter_binding(expression, return_property) {
            return Some(binding);
        }
        let source = self.iterator_close_source_expression(expression)?;
        let iterator_call = match &source {
            Expression::GetIterator(iterated) => Expression::Call {
                callee: Box::new(Expression::Member {
                    object: Box::new((**iterated).clone()),
                    property: Box::new(symbol_iterator_expression()),
                }),
                arguments: Vec::new(),
            },
            Expression::Call { callee, arguments }
                if arguments.is_empty()
                    && matches!(
                        callee.as_ref(),
                        Expression::Member { property, .. }
                            if is_symbol_iterator_expression(property)
                    ) =>
            {
                source.clone()
            }
            _ => return self.resolve_member_getter_binding(&source, return_property),
        };
        if let Expression::Call { callee, arguments } = &iterator_call
            && let Some((returned, _)) = self.resolve_static_call_result_expression_with_context(
                callee,
                arguments,
                self.current_function_name(),
            )
            && let Some(binding) = self.resolve_member_getter_binding(&returned, return_property)
        {
            return Some(binding);
        }
        self.inherited_member_getter_bindings(&iterator_call)
            .into_iter()
            .find(|binding| {
                static_expression_matches(
                    &Expression::String(binding.property.clone()),
                    return_property,
                )
            })
            .map(|binding| binding.binding)
    }

    fn iterator_close_should_call_return(
        &self,
        expression: &Expression,
        return_property: &Expression,
    ) -> bool {
        let mut known_nullish_value = false;
        if let Some(object_binding) = self.resolve_object_binding_from_expression(expression) {
            if let Some(descriptor) =
                object_binding_lookup_descriptor(&object_binding, return_property)
            {
                if descriptor.getter.is_some() {
                    return true;
                }
                if let Some(value) = descriptor.value.as_ref() {
                    if !matches!(value, Expression::Undefined | Expression::Null) {
                        return true;
                    }
                    known_nullish_value = true;
                }
                if descriptor.has_get {
                    known_nullish_value = true;
                }
            }
            if let Some(value) = object_binding_lookup_value(&object_binding, return_property) {
                if !matches!(value, Expression::Undefined | Expression::Null) {
                    return true;
                }
                known_nullish_value = true;
            }
        }
        if self
            .resolve_member_function_binding(expression, return_property)
            .is_some()
            || self
                .iterator_close_source_return_getter_binding(expression, return_property)
                .is_some()
        {
            return true;
        }
        !known_nullish_value
            && self
                .resolve_member_getter_binding(expression, return_property)
                .is_some()
    }

    fn terminal_return_value(body: &[Statement]) -> Option<Expression> {
        match body.last()? {
            Statement::Return(value) => Some(value.clone()),
            Statement::Block { body } | Statement::Declaration { body } => {
                Self::terminal_return_value(body)
            }
            _ => None,
        }
    }

    fn user_getter_terminal_return_value(
        &self,
        binding: &LocalFunctionBinding,
    ) -> Option<Expression> {
        let LocalFunctionBinding::User(function_name) = binding else {
            return None;
        };
        let function = self.resolve_registered_function_declaration(function_name)?;
        Self::terminal_return_value(&function.body)
    }

    fn iterator_close_static_return_property_value(
        &self,
        expression: &Expression,
        return_property: &Expression,
    ) -> Option<Expression> {
        let lookup = |compiler: &Self, target: &Expression| {
            let object_binding = compiler.resolve_object_binding_from_expression(target)?;
            if let Some(descriptor) =
                object_binding_lookup_descriptor(&object_binding, return_property)
                && descriptor.getter.is_none()
                && let Some(value) = descriptor.value.as_ref()
            {
                return Some(value.clone());
            }
            object_binding_lookup_value(&object_binding, return_property).cloned()
        };

        lookup(self, expression).or_else(|| {
            self.iterator_close_source_expression(expression)
                .and_then(|source| lookup(self, &source))
        })
    }

    pub(in crate::backend::direct_wasm) fn resolve_iterator_close_updated_bindings(
        &self,
        expression: &Expression,
        property: &Expression,
    ) -> Option<HashMap<String, Expression>> {
        let trace_iterator_close = std::env::var_os("AYY_TRACE_ITERATOR_CLOSE").is_some();
        if let Expression::Identifier(name) = expression
            && let Some(iterator_binding) = self
                .state
                .speculation
                .static_semantics
                .local_array_iterator_binding(name)
            && matches!(
                &iterator_binding.source,
                IteratorSourceKind::StaticArray { .. }
                    | IteratorSourceKind::StaticArrayEntries { .. }
                    | IteratorSourceKind::StaticMapEntries { .. }
                    | IteratorSourceKind::TypedArrayView { .. }
                    | IteratorSourceKind::DirectArguments { .. }
            )
        {
            if trace_iterator_close {
                eprintln!("iterator_close:updated_bindings skip_array_like name={name}");
            }
            return None;
        }
        let this_expression = self
            .iterator_close_target_expression(expression)
            .unwrap_or_else(|| expression.clone());
        let Some(binding) = self.resolve_member_function_binding(expression, property) else {
            let Some(getter_binding) =
                self.iterator_close_source_return_getter_binding(expression, property)
            else {
                return None;
            };
            let LocalFunctionBinding::User(function_name) = &getter_binding else {
                return None;
            };
            let function = self.resolve_registered_function_declaration(&function_name)?;
            let mut assigned_names = HashSet::new();
            for statement in &function.body {
                collect_assigned_binding_names_from_statement(statement, &mut assigned_names);
            }
            if assigned_names.is_empty() {
                return None;
            }
            let callee_local_sources =
                self.iterator_close_callee_local_binding_sources(&getter_binding);
            let mut snapshot_names =
                collect_referenced_binding_names_from_statements(&function.body);
            snapshot_names.extend(assigned_names);
            let snapshot_bindings = snapshot_names
                .into_iter()
                .filter(|name| {
                    !Self::iterator_close_updated_binding_is_callee_local(
                        name,
                        &callee_local_sources,
                    ) && self.should_sync_async_delegate_snapshot_binding(name)
                })
                .map(|name| {
                    let value =
                        self.materialize_static_expression(&Expression::Identifier(name.clone()));
                    (name, value)
                })
                .collect::<HashMap<_, _>>();
            if trace_iterator_close {
                eprintln!(
                    "iterator_close:getter_snapshot_seed function={function_name} keys={:?}",
                    snapshot_bindings.keys().collect::<Vec<_>>()
                );
            }
            let result = self.resolve_bound_snapshot_function_result_with_arguments_and_this(
                &getter_binding,
                &snapshot_bindings,
                &[],
                &this_expression,
            );
            if trace_iterator_close {
                eprintln!(
                    "iterator_close:getter_snapshot_result result_present={} updated={:?}",
                    result.is_some(),
                    result.as_ref().map(|(_, updated)| updated)
                );
            }
            return result.map(|(_, mut updated_bindings)| {
                updated_bindings.retain(|name, _| {
                    !Self::iterator_close_updated_binding_is_callee_local(
                        name,
                        &callee_local_sources,
                    )
                });
                updated_bindings
            });
        };
        let callee_local_sources = self.iterator_close_callee_local_binding_sources(&binding);
        let mut snapshot_bindings = match &binding {
            LocalFunctionBinding::User(function_name) => self
                .resolve_registered_function_declaration(function_name)
                .map(|function| {
                    collect_referenced_binding_names_from_statements(&function.body)
                        .into_iter()
                        .filter(|name| {
                            !Self::iterator_close_updated_binding_is_callee_local(
                                name,
                                &callee_local_sources,
                            ) && self.should_sync_async_delegate_snapshot_binding(name)
                        })
                        .map(|name| {
                            let identifier = Expression::Identifier(name.clone());
                            (name, self.materialize_static_expression(&identifier))
                        })
                        .collect::<HashMap<_, _>>()
                })
                .unwrap_or_default(),
            LocalFunctionBinding::Builtin(_) => HashMap::new(),
        };
        if trace_iterator_close {
            eprintln!(
                "iterator_close:snapshot_seed binding={binding:?} keys={:?}",
                snapshot_bindings.keys().collect::<Vec<_>>()
            );
        }
        snapshot_bindings.extend(
            self.resolve_member_function_capture_slots(expression, property)
                .map(|capture_slots| {
                    capture_slots
                        .into_iter()
                        .map(|(capture_name, slot_name)| {
                            (
                                capture_name,
                                self.snapshot_bound_capture_slot_expression(&slot_name),
                            )
                        })
                        .collect::<HashMap<_, _>>()
                })
                .unwrap_or_default(),
        );
        let result = self.resolve_bound_snapshot_function_result_with_arguments_and_this(
            &binding,
            &snapshot_bindings,
            &[],
            &this_expression,
        );
        if trace_iterator_close {
            eprintln!(
                "iterator_close:snapshot_result result_present={} updated={:?}",
                result.is_some(),
                result.as_ref().map(|(_, updated)| updated)
            );
        }
        result.map(|(_, mut updated_bindings)| {
            updated_bindings.retain(|name, _| {
                !Self::iterator_close_updated_binding_is_callee_local(name, &callee_local_sources)
            });
            updated_bindings
        })
    }

    fn sync_iterator_close_call_snapshot_bindings(
        &mut self,
        updated_bindings: Option<HashMap<String, Expression>>,
        call_expression: &Expression,
    ) -> DirectResult<()> {
        let Some(updated_bindings) = updated_bindings.or_else(|| {
            self.state
                .speculation
                .static_semantics
                .last_bound_user_function_call
                .as_ref()
                .filter(|snapshot| {
                    snapshot
                        .source_expression
                        .as_ref()
                        .is_some_and(|source| static_expression_matches(source, call_expression))
                })
                .map(|snapshot| snapshot.updated_bindings.clone())
        }) else {
            return Ok(());
        };
        self.sync_async_delegate_snapshot_bindings(&updated_bindings)?;
        for (name, value) in updated_bindings {
            let source_name = scoped_binding_source_name(&name)
                .unwrap_or(&name)
                .to_string();
            if source_name.starts_with("__ayy_array_iter_") {
                continue;
            }
            if !self.should_sync_async_delegate_snapshot_binding(&source_name) {
                continue;
            }
            self.sync_bound_capture_source_binding_metadata(&source_name, &value)?;
        }
        Ok(())
    }

    pub(in crate::backend::direct_wasm) fn emit_enumerate_keys_expression(
        &mut self,
        expression: &Expression,
    ) -> DirectResult<()> {
        let module_index = self.module_namespace_index_from_expression(expression);
        self.emit_numeric_expression(expression)?;
        self.state.emission.output.instructions.push(0x1a);
        if let Some(module_index) = module_index
            && let Some(names) = self
                .resolve_static_dynamic_import_namespace_own_property_names_binding(module_index)
        {
            for name in names.values.into_iter().flatten() {
                let Expression::String(name) = name else {
                    continue;
                };
                let property = Expression::String(name);
                let live_value = self
                    .resolve_static_dynamic_import_namespace_live_binding_member_value(
                        module_index,
                        &property,
                    );
                if let Some(live_value) = live_value.as_ref()
                    && self.module_namespace_live_value_is_readable_in_current_context(live_value)
                {
                    self.emit_numeric_expression(live_value)?;
                    self.state.emission.output.instructions.push(0x1a);
                    continue;
                }
                if let Some((binding_name, _)) = self
                    .resolve_static_dynamic_import_namespace_live_binding_member_binding_initializer_value(
                        module_index,
                        &property,
                    )
                {
                    let binding = Expression::Identifier(binding_name);
                    if self.module_namespace_live_value_is_readable_in_current_context(&binding) {
                        self.emit_numeric_expression(&binding)?;
                        self.state.emission.output.instructions.push(0x1a);
                    }
                }
            }
        }
        self.push_i32_const(JS_TYPEOF_OBJECT_TAG);
        Ok(())
    }

    pub(in crate::backend::direct_wasm) fn emit_get_iterator_expression(
        &mut self,
        expression: &Expression,
    ) -> DirectResult<()> {
        let trace_get_iterator = std::env::var_os("AYY_TRACE_GET_ITERATOR").is_some();
        if trace_get_iterator {
            eprintln!("get_iterator:start expression={expression:?}");
        }
        if let Expression::Assign { name, .. } = expression {
            self.emit_numeric_expression(expression)?;
            self.state.emission.output.instructions.push(0x1a);
            return self.emit_get_iterator_expression(&Expression::Identifier(name.clone()));
        }
        if let Expression::Identifier(name) = expression
            && self
                .resolve_local_array_iterator_binding_name(name)
                .is_some()
        {
            self.push_i32_const(JS_TYPEOF_OBJECT_TAG);
            return Ok(());
        }
        if matches!(
            expression,
            Expression::Identifier(name)
                if name.starts_with("__ayy_array_iter_value_")
                    || name.starts_with("__ayy_for_of_iter_value_")
        ) {
            if trace_get_iterator {
                if let Expression::Identifier(name) = expression {
                    let value = self
                        .state
                        .speculation
                        .static_semantics
                        .local_value_binding(name);
                    eprintln!(
                        "get_iterator:internal_value_alias name={name} value={value:?} typed_view={}",
                        self.resolve_typed_array_view_binding_from_expression(expression)
                            .is_some()
                    );
                }
            }
        }
        if matches!(
            expression,
            Expression::Identifier(name)
                if name.starts_with("__ayy_array_iter_value_")
                    || name.starts_with("__ayy_for_of_iter_value_")
        ) && let Some(view) = self.resolve_typed_array_view_binding_from_expression(expression)
        {
            if self
                .typed_array_view_static_out_of_bounds(&view)
                .unwrap_or(false)
            {
                return self.emit_named_error_throw("TypeError");
            }
            self.push_i32_const(JS_TYPEOF_OBJECT_TAG);
            return Ok(());
        }
        let expression_is_internal_iterator_value = matches!(
            expression,
            Expression::Identifier(name)
                if name.starts_with("__ayy_array_iter_value_")
                    || name.starts_with("__ayy_for_of_iter_value_")
        );
        if !expression_is_internal_iterator_value
            && let Some((function_name, returned_expression, effect_statements)) =
                self.analyze_effectful_iterator_source_call(expression)
        {
            if trace_get_iterator {
                eprintln!("get_iterator:effectful_source function={function_name}");
            }
            self.with_named_function_execution_context(function_name, |compiler| {
                for statement in &effect_statements {
                    compiler.emit_statement(statement)?;
                }
                Ok(())
            })?;
            return self
                .emit_numeric_expression(&Expression::GetIterator(Box::new(returned_expression)));
        }
        if trace_get_iterator {
            eprintln!("get_iterator:materialize:start");
        }
        let materialized_expression = if expression_is_internal_iterator_value {
            expression.clone()
        } else {
            self.materialize_static_expression(expression)
        };
        if trace_get_iterator {
            eprintln!("get_iterator:materialize:done materialized={materialized_expression:?}");
        }
        let iterator_target = if !static_expression_matches(&materialized_expression, expression) {
            &materialized_expression
        } else {
            expression
        };
        let static_array_iterator_target = if let Expression::Identifier(name) = iterator_target {
            self.static_array_binding_expression(name)
        } else {
            None
        };
        let iterator_target_is_internal_iterator_value = matches!(
            iterator_target,
            Expression::Identifier(name)
                if name.starts_with("__ayy_array_iter_value_")
                    || name.starts_with("__ayy_for_of_iter_value_")
        ) && static_array_iterator_target
            .is_none();
        let iterator_target = static_array_iterator_target
            .as_ref()
            .unwrap_or(iterator_target);
        let awaited_iterator_target;
        let iterator_target = if !iterator_target_is_internal_iterator_value
            && let Some(outcome) = self.resolve_static_await_resolution_outcome(iterator_target)
        {
            match outcome {
                StaticEvalOutcome::Throw(throw_value) => {
                    return self.emit_static_throw_value(&throw_value);
                }
                StaticEvalOutcome::Value(value) => {
                    awaited_iterator_target = value;
                    &awaited_iterator_target
                }
            }
        } else {
            iterator_target
        };
        if trace_get_iterator {
            eprintln!("get_iterator:array_proto_deleted:start");
        }
        if !iterator_target_is_internal_iterator_value
            && self.array_prototype_symbol_iterator_deleted_affects(iterator_target)
        {
            if trace_get_iterator {
                eprintln!("get_iterator:array_proto_deleted:hit");
            }
            return self.emit_named_error_throw("TypeError");
        }
        if let Expression::Identifier(name) = iterator_target
            && self
                .resolve_local_array_iterator_binding_name(&name)
                .is_some()
        {
            if trace_get_iterator {
                eprintln!("get_iterator:local_iterator_binding:hit");
            }
            self.push_i32_const(JS_TYPEOF_OBJECT_TAG);
            return Ok(());
        }
        if trace_get_iterator {
            eprintln!("get_iterator:throw_value:start");
        }
        if !iterator_target_is_internal_iterator_value
            && let Some(throw_value) =
                self.resolve_static_get_iterator_throw_value(iterator_target, &[])
        {
            if trace_get_iterator {
                eprintln!("get_iterator:throw_value");
            }
            return self.emit_static_throw_value(&throw_value);
        }
        if trace_get_iterator {
            eprintln!("get_iterator:source_kind:first:start target={iterator_target:?}");
        }
        if self.resolve_iterator_source_kind(iterator_target).is_some() {
            if trace_get_iterator {
                eprintln!("get_iterator:source_kind:first:hit");
            }
            self.push_i32_const(JS_TYPEOF_OBJECT_TAG);
            return Ok(());
        }
        if trace_get_iterator {
            eprintln!("get_iterator:for_await_source:start");
        }
        if self
            .resolve_for_await_step_value_iterator_source_kind(iterator_target)
            .is_some()
        {
            if trace_get_iterator {
                eprintln!("get_iterator:for_await_source:hit");
            }
            self.push_i32_const(JS_TYPEOF_OBJECT_TAG);
            return Ok(());
        }
        if let Expression::Identifier(name) = expression {
            if let Some(view) = self.typed_array_view_binding_for_name(name) {
                if self
                    .typed_array_view_static_out_of_bounds(&view)
                    .unwrap_or(false)
                {
                    return self.emit_named_error_throw("TypeError");
                }
                if let Some(oob_local) = self
                    .state
                    .speculation
                    .static_semantics
                    .runtime_typed_array_oob_local(name)
                {
                    self.push_local_get(oob_local);
                    self.state.emission.output.instructions.push(0x04);
                    self.state
                        .emission
                        .output
                        .instructions
                        .push(EMPTY_BLOCK_TYPE);
                    self.push_control_frame();
                    self.emit_named_error_throw("TypeError")?;
                    self.state.emission.output.instructions.push(0x0b);
                    self.pop_control_frame();
                }
                self.emit_numeric_expression(expression)?;
                self.state.emission.output.instructions.push(0x1a);
                self.push_i32_const(JS_TYPEOF_OBJECT_TAG);
                return Ok(());
            }
        }
        if trace_get_iterator {
            eprintln!("get_iterator:source_kind:second:start");
        }
        if matches!(
            self.resolve_iterator_source_kind(iterator_target),
            Some(
                IteratorSourceKind::StaticArray { .. }
                    | IteratorSourceKind::StaticArrayEntries { .. }
                    | IteratorSourceKind::StaticMapEntries { .. }
                    | IteratorSourceKind::SimpleGenerator { .. }
                    | IteratorSourceKind::DirectArguments { .. }
            )
        ) {
            if trace_get_iterator {
                eprintln!("get_iterator:source_kind:second:hit");
            }
            self.push_i32_const(JS_TYPEOF_OBJECT_TAG);
            return Ok(());
        }
        if trace_get_iterator {
            eprintln!("get_iterator:infer_kind:start");
        }
        if matches!(
            self.infer_value_kind(iterator_target),
            Some(
                StaticValueKind::Undefined
                    | StaticValueKind::Null
                    | StaticValueKind::Bool
                    | StaticValueKind::Number
                    | StaticValueKind::BigInt
                    | StaticValueKind::Symbol
            )
        ) {
            if trace_get_iterator {
                eprintln!("get_iterator:infer_kind:primitive");
            }
            return self.emit_named_error_throw("TypeError");
        }
        if trace_get_iterator {
            eprintln!("get_iterator:has_next:start");
        }
        let has_next_method = self
            .resolve_object_binding_from_expression(iterator_target)
            .and_then(|object_binding| {
                object_binding_lookup_value(
                    &object_binding,
                    &Expression::String("next".to_string()),
                )
                .cloned()
            })
            .and_then(|value| self.resolve_function_binding_from_expression(&value))
            .is_some()
            || self
                .resolve_member_function_binding(
                    iterator_target,
                    &Expression::String("next".to_string()),
                )
                .is_some();
        if has_next_method {
            if trace_get_iterator {
                eprintln!("get_iterator:has_next:hit");
            }
            self.emit_numeric_expression(iterator_target)?;
            return Ok(());
        }
        if trace_get_iterator {
            eprintln!("get_iterator:symbol_iterator:start");
        }
        let iterator_property = self.materialize_static_expression(&symbol_iterator_expression());
        if self
            .resolve_member_function_binding(iterator_target, &iterator_property)
            .is_some()
            || self
                .resolve_member_getter_binding(iterator_target, &iterator_property)
                .is_some()
        {
            return self.emit_numeric_expression(&Expression::Call {
                callee: Box::new(Expression::Member {
                    object: Box::new(iterator_target.clone()),
                    property: Box::new(iterator_property),
                }),
                arguments: Vec::new(),
            });
        }
        self.emit_numeric_expression(iterator_target)?;
        self.state.emission.output.instructions.push(0x1a);
        self.push_i32_const(JS_TYPEOF_OBJECT_TAG);
        Ok(())
    }

    pub(in crate::backend::direct_wasm) fn emit_iterator_close_expression(
        &mut self,
        expression: &Expression,
    ) -> DirectResult<()> {
        let trace_iterator_close = std::env::var_os("AYY_TRACE_ITERATOR_CLOSE").is_some();
        if let Expression::Identifier(name) = expression
            && let Some(iterator_binding) = self
                .state
                .speculation
                .static_semantics
                .local_array_iterator_binding(name)
                .cloned()
            && matches!(
                iterator_binding.source,
                IteratorSourceKind::StaticArray { .. }
                    | IteratorSourceKind::StaticArrayEntries { .. }
                    | IteratorSourceKind::StaticMapEntries { .. }
                    | IteratorSourceKind::TypedArrayView { .. }
                    | IteratorSourceKind::DirectArguments { .. }
            )
        {
            if trace_iterator_close {
                eprintln!("iterator_close:path early_array_like_without_return name={name}");
            }
            self.push_i32_const(JS_UNDEFINED_TAG);
            return Ok(());
        }
        let return_property = Expression::String("return".to_string());
        let capture_source_bindings =
            self.resolve_member_function_capture_source_bindings(expression, &return_property);
        let should_call_return =
            self.iterator_close_should_call_return(expression, &return_property);
        if trace_iterator_close {
            eprintln!(
                "iterator_close:start expr={expression:?} should_call_return={should_call_return} captures={capture_source_bindings:?}"
            );
        }
        if let Expression::Identifier(name) = expression
            && let Some(iterator_binding) = self
                .state
                .speculation
                .static_semantics
                .local_array_iterator_binding(name)
                .cloned()
        {
            let state_local = iterator_binding.index_local;
            match iterator_binding.source {
                IteratorSourceKind::SimpleGenerator { steps, .. } => {
                    if trace_iterator_close {
                        eprintln!("iterator_close:path simple_generator name={name}");
                    }
                    if self.emit_fresh_simple_generator_return_call(expression, &[])? {
                        self.state.emission.output.instructions.push(0x1a);
                        self.push_i32_const(JS_UNDEFINED_TAG);
                        return Ok(());
                    }
                    let closed_state = (steps.len() + 1) as i32;
                    self.push_i32_const(closed_state);
                    self.push_local_set(state_local);
                    self.push_i32_const(JS_UNDEFINED_TAG);
                    return Ok(());
                }
                IteratorSourceKind::StaticArray { .. }
                | IteratorSourceKind::StaticArrayEntries { .. }
                | IteratorSourceKind::StaticMapEntries { .. }
                | IteratorSourceKind::TypedArrayView { .. }
                | IteratorSourceKind::DirectArguments { .. }
                    if !should_call_return =>
                {
                    if trace_iterator_close {
                        eprintln!("iterator_close:path array_like_without_return name={name}");
                    }
                    self.push_i32_const(JS_UNDEFINED_TAG);
                    return Ok(());
                }
                _ => {}
            }
        }
        let should_call_return =
            self.iterator_close_should_call_return(expression, &return_property);
        if should_call_return {
            if trace_iterator_close {
                eprintln!("iterator_close:path call_return expr={expression:?}");
            }
            let call_target = self
                .iterator_close_target_expression(expression)
                .unwrap_or_else(|| expression.clone());
            let return_binding = self
                .resolve_member_function_binding(expression, &return_property)
                .or_else(|| self.resolve_member_function_binding(&call_target, &return_property));
            let getter_binding =
                self.iterator_close_source_return_getter_binding(expression, &return_property);
            let getter_outcome = getter_binding.as_ref().and_then(|binding| {
                self.resolve_terminal_function_outcome_from_binding(binding, &[])
            });
            let return_member = Expression::Member {
                object: Box::new(call_target.clone()),
                property: Box::new(return_property.clone()),
            };
            if matches!(getter_outcome, Some(StaticEvalOutcome::Throw(_))) {
                self.emit_numeric_expression(&return_member)?;
                self.state.emission.output.instructions.push(0x1a);
                self.push_i32_const(JS_UNDEFINED_TAG);
                return Ok(());
            }
            let static_return_property_value =
                self.iterator_close_static_return_property_value(expression, &return_property);
            let getter_return_value = getter_outcome
                .as_ref()
                .and_then(|outcome| match outcome {
                    StaticEvalOutcome::Value(value) => Some(value.clone()),
                    StaticEvalOutcome::Throw(_) => None,
                })
                .or_else(|| {
                    getter_binding.as_ref().and_then(|binding| {
                        self.resolve_function_binding_static_return_expression_with_call_frame(
                            binding,
                            &[],
                            expression,
                        )
                        .or_else(|| self.user_getter_terminal_return_value(binding))
                        .or_else(|| {
                            self.function_binding_defaults_to_undefined(binding)
                                .then_some(Expression::Undefined)
                        })
                    })
                });
            if matches!(
                getter_return_value,
                Some(Expression::Undefined | Expression::Null)
            ) {
                let getter_updated_bindings =
                    self.resolve_iterator_close_updated_bindings(expression, &return_property);
                if let Some(LocalFunctionBinding::User(function_name)) = getter_binding.as_ref() {
                    let capture_slots = self
                        .resolve_member_function_capture_slots(&call_target, &return_property)
                        .or_else(|| {
                            self.resolve_member_function_capture_slots(expression, &return_property)
                        });
                    self.emit_member_getter_call_with_bound_this(
                        function_name,
                        &call_target,
                        capture_slots.as_ref(),
                    )?;
                } else {
                    self.emit_numeric_expression(&return_member)?;
                }
                self.sync_iterator_close_call_snapshot_bindings(
                    getter_updated_bindings,
                    &return_member,
                )?;
                self.state.emission.output.instructions.push(0x1a);
                self.push_i32_const(JS_UNDEFINED_TAG);
                return Ok(());
            }
            if matches!(
                static_return_property_value,
                Some(Expression::Undefined | Expression::Null)
            ) {
                self.emit_numeric_expression(&return_member)?;
                self.state.emission.output.instructions.push(0x1a);
                self.push_i32_const(JS_UNDEFINED_TAG);
                return Ok(());
            }
            if getter_return_value
                .as_ref()
                .and_then(|value| self.infer_value_kind(value))
                .is_some_and(|kind| kind != StaticValueKind::Function)
                || static_return_property_value.as_ref().is_some_and(|value| {
                    self.resolve_function_binding_from_expression(value)
                        .is_none()
                        && self.infer_value_kind(value) != Some(StaticValueKind::Function)
                })
            {
                self.emit_numeric_expression(&return_member)?;
                self.state.emission.output.instructions.push(0x1a);
                self.emit_named_error_throw("TypeError")?;
                self.push_i32_const(JS_UNDEFINED_TAG);
                return Ok(());
            }
            let return_result_is_non_object = return_binding.as_ref().is_some_and(|binding| {
                self.iterator_close_return_result_is_definitely_non_object(binding)
            });
            let static_updated_bindings =
                self.resolve_iterator_close_updated_bindings(expression, &return_property);
            let return_callee = return_member;
            let return_call = Expression::Call {
                callee: Box::new(return_callee.clone()),
                arguments: Vec::new(),
            };
            let mut user_return_function = None;
            let mut user_return_body = None;
            if let Some(LocalFunctionBinding::User(function_name)) = return_binding.as_ref()
                && let Some(user_function) = self.user_function(function_name).cloned()
            {
                user_return_body = self
                    .resolve_registered_function_declaration(function_name)
                    .map(|declaration| declaration.body.clone());
                if let Some(capture_slots) = self.resolve_member_function_capture_slots(
                    expression,
                    &Expression::String("return".to_string()),
                ) {
                    self.emit_user_function_call_with_new_target_and_this_expression_and_bound_captures_without_static_snapshot(
                        &user_function,
                        &[],
                        JS_UNDEFINED_TAG,
                        &call_target,
                        &capture_slots,
                    )?;
                } else if let Some(capture_slots) =
                    self.resolve_function_expression_capture_slots(&return_callee)
                {
                    self.emit_user_function_call_with_new_target_and_this_expression_and_bound_captures_without_static_snapshot(
                        &user_function,
                        &[],
                        JS_UNDEFINED_TAG,
                        &call_target,
                        &capture_slots,
                    )?;
                } else {
                    self.emit_user_function_call_with_new_target_and_this_expression_without_static_snapshot(
                        &user_function,
                        &[],
                        JS_UNDEFINED_TAG,
                        &call_target,
                    )?;
                }
                user_return_function = Some(user_function);
            } else {
                self.emit_numeric_expression(&return_call)?;
            }
            self.sync_iterator_close_call_snapshot_bindings(static_updated_bindings, &return_call)?;
            if let (Some(user_function), Some(body)) =
                (user_return_function.as_ref(), user_return_body.as_deref())
            {
                self.sync_static_iterator_close_arguments_assignments(user_function, &[], body);
            }
            self.state.emission.output.instructions.push(0x1a);
            if !capture_source_bindings.is_empty() {
                self.state
                    .runtime
                    .locals
                    .runtime_dynamic_bindings
                    .extend(capture_source_bindings);
            }
            if return_result_is_non_object {
                return self.emit_named_error_throw("TypeError");
            }
            self.push_i32_const(JS_UNDEFINED_TAG);
            return Ok(());
        }
        if trace_iterator_close {
            eprintln!("iterator_close:path drop_value expr={expression:?}");
        }
        self.emit_numeric_expression(expression)?;
        self.state.emission.output.instructions.push(0x1a);
        self.push_i32_const(JS_UNDEFINED_TAG);
        Ok(())
    }

    pub(in crate::backend::direct_wasm) fn emit_await_expression(
        &mut self,
        expression: &Expression,
    ) -> DirectResult<()> {
        if let Expression::Identifier(_) = expression {
            let then_property = Expression::String("then".to_string());
            if let Some(object_binding) = self.resolve_object_binding_from_expression(expression)
                && object_binding_lookup_value(&object_binding, &then_property).is_none()
                && object_binding_lookup_descriptor(&object_binding, &then_property).is_none()
            {
                self.push_i32_const(JS_TYPEOF_OBJECT_TAG);
                return Ok(());
            }
        }
        if let Some(outcome) = self.resolve_static_await_resolution_outcome(expression) {
            match outcome {
                StaticEvalOutcome::Value(awaited_value) => {
                    self.emit_numeric_expression(&awaited_value)?;
                }
                StaticEvalOutcome::Throw(throw_value) => {
                    self.emit_static_throw_value(&throw_value)?;
                }
            }
            return Ok(());
        }
        self.emit_numeric_expression(expression)?;
        if let Some(snapshot_result) = self
            .state
            .speculation
            .static_semantics
            .last_bound_user_function_call
            .as_ref()
            .and_then(|snapshot| {
                self.user_function(&snapshot.function_name)
                    .filter(|function| function.is_async())
                    .and_then(|_| snapshot.result_expression.clone())
            })
        {
            self.state.emission.output.instructions.push(0x1a);
            if let Some(outcome) = self.resolve_static_await_resolution_outcome(&snapshot_result) {
                return self.emit_static_eval_outcome(&outcome);
            }
            return self.emit_numeric_expression(&snapshot_result);
        }
        self.state.emission.output.instructions.push(0x1a);
        self.push_i32_const(JS_UNDEFINED_TAG);
        Ok(())
    }
}
