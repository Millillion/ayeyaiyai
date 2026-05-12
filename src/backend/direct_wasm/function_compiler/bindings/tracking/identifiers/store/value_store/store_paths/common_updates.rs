use super::*;

fn expression_references_internal_iterator_step(expression: &Expression) -> bool {
    match expression {
        Expression::Identifier(name) => {
            name.starts_with("__ayy_array_step_")
                || name.starts_with("__ayy_for_of_step_")
                || name.starts_with("__ayy_array_iter_value_")
                || name.starts_with("__ayy_for_of_iter_value_")
                || name.starts_with("__ayy_binding_value_")
        }
        Expression::Array(elements) => elements.iter().any(|element| match element {
            ArrayElement::Expression(value) | ArrayElement::Spread(value) => {
                expression_references_internal_iterator_step(value)
            }
        }),
        Expression::Object(entries) => entries.iter().any(|entry| match entry {
            ObjectEntry::Data { key, value } => {
                expression_references_internal_iterator_step(key)
                    || expression_references_internal_iterator_step(value)
            }
            ObjectEntry::Getter { key, getter } => {
                expression_references_internal_iterator_step(key)
                    || expression_references_internal_iterator_step(getter)
            }
            ObjectEntry::Setter { key, setter } => {
                expression_references_internal_iterator_step(key)
                    || expression_references_internal_iterator_step(setter)
            }
            ObjectEntry::Spread(value) => expression_references_internal_iterator_step(value),
        }),
        Expression::Binary { left, right, .. } => {
            expression_references_internal_iterator_step(left)
                || expression_references_internal_iterator_step(right)
        }
        Expression::Conditional {
            condition,
            then_expression,
            else_expression,
        } => {
            expression_references_internal_iterator_step(condition)
                || expression_references_internal_iterator_step(then_expression)
                || expression_references_internal_iterator_step(else_expression)
        }
        Expression::Member { object, property } => {
            expression_references_internal_iterator_step(object)
                || expression_references_internal_iterator_step(property)
        }
        Expression::Unary { expression, .. }
        | Expression::Await(expression)
        | Expression::EnumerateKeys(expression)
        | Expression::GetIterator(expression)
        | Expression::IteratorClose(expression) => {
            expression_references_internal_iterator_step(expression)
        }
        Expression::Assign { value, .. } => expression_references_internal_iterator_step(value),
        Expression::AssignMember {
            object,
            property,
            value,
        } => {
            expression_references_internal_iterator_step(object)
                || expression_references_internal_iterator_step(property)
                || expression_references_internal_iterator_step(value)
        }
        Expression::AssignSuperMember { property, value } => {
            expression_references_internal_iterator_step(property)
                || expression_references_internal_iterator_step(value)
        }
        Expression::Call { callee, arguments }
        | Expression::New { callee, arguments }
        | Expression::SuperCall { callee, arguments } => {
            expression_references_internal_iterator_step(callee)
                || arguments.iter().any(|argument| match argument {
                    CallArgument::Expression(value) | CallArgument::Spread(value) => {
                        expression_references_internal_iterator_step(value)
                    }
                })
        }
        Expression::SuperMember { property } => {
            expression_references_internal_iterator_step(property)
        }
        _ => false,
    }
}

impl<'a> FunctionCompiler<'a> {
    fn identifier_store_value_is_local_simple_async_generator_next_call(
        &self,
        expression: &Expression,
    ) -> bool {
        let Expression::Call { callee, .. } = expression else {
            return false;
        };
        let Expression::Member { object, property } = callee.as_ref() else {
            return false;
        };
        if !matches!(property.as_ref(), Expression::String(property_name) if property_name == "next")
        {
            return false;
        }
        let Expression::Identifier(iterator_name) = object.as_ref() else {
            return false;
        };
        let Some(binding_name) = self.resolve_local_array_iterator_binding_name(iterator_name)
        else {
            return false;
        };
        self.state
            .speculation
            .static_semantics
            .local_array_iterator_binding(&binding_name)
            .is_some_and(|binding| {
                matches!(
                    binding.source,
                    IteratorSourceKind::SimpleGenerator { is_async: true, .. }
                )
            })
    }

    fn identifier_store_capture_source_expression_for_local_value(
        &self,
        capture_name: &str,
        force_runtime_slot: bool,
    ) -> Option<(Expression, bool)> {
        if capture_name == "new.target" {
            return Some((Expression::NewTarget, true));
        }
        if capture_name == "this" {
            if let Some(hidden_name) = self.resolve_user_function_capture_hidden_name("this") {
                return Some((Expression::Identifier(hidden_name), true));
            }
            if self.current_function_name().is_some() {
                return Some((Expression::This, true));
            }
            return self
                .global_has_binding("this")
                .then(|| (Expression::Identifier("this".to_string()), false));
        }

        if let Some(hidden_name) = self.resolve_user_function_capture_hidden_name(capture_name) {
            return Some((Expression::Identifier(hidden_name), true));
        }
        if let Some(scope_object) = self.resolve_with_scope_binding_for_capture_source(capture_name)
        {
            return Some((
                Expression::Member {
                    object: Box::new(scope_object),
                    property: Box::new(Expression::String(capture_name.to_string())),
                },
                true,
            ));
        }
        if self.resolve_current_local_binding(capture_name).is_some() {
            return Some((Expression::Identifier(capture_name.to_string()), true));
        }
        if let Some(hidden_name) = self.resolve_eval_local_function_hidden_name(capture_name) {
            return Some((Expression::Identifier(hidden_name), true));
        }
        if self.global_has_binding(capture_name)
            || self.backend.global_has_lexical_binding(capture_name)
            || self.backend.global_function_binding(capture_name).is_some()
            || self.global_has_implicit_binding(capture_name)
        {
            return Some((
                Expression::Identifier(capture_name.to_string()),
                force_runtime_slot,
            ));
        }
        None
    }

    fn preserve_identifier_function_capture_slots_for_local_store(
        &mut self,
        state: &PreparedIdentifierStoreState,
    ) -> DirectResult<()> {
        let Some(LocalFunctionBinding::User(function_name)) = state.function_binding.as_ref()
        else {
            return Ok(());
        };
        let mut capture_bindings = self
            .backend
            .function_registry
            .analysis
            .user_function_capture_bindings
            .get(function_name)
            .cloned()
            .unwrap_or_default();
        self.add_active_with_scope_function_capture_bindings(function_name, &mut capture_bindings)?;
        if capture_bindings.is_empty() {
            return Ok(());
        }

        let trace_capture_bindings = std::env::var_os("AYY_TRACE_CAPTURE_BINDINGS").is_some();
        let mut capture_slots = BTreeMap::new();
        for capture_name in capture_bindings.keys() {
            let capture_originates_in_enclosing_local = self
                .assigned_user_function_capture_originates_in_enclosing_local(
                    function_name,
                    capture_name,
                );
            let parameter_default_snapshot = self
                .assigned_user_function_capture_needs_parameter_default_snapshot(
                    function_name,
                    capture_name,
                );
            let active_loop_capture = self.expression_depends_on_active_loop_assignment(
                &Expression::Identifier(capture_name.clone()),
            );
            let force_runtime_slot = capture_originates_in_enclosing_local
                || parameter_default_snapshot
                || active_loop_capture;
            if trace_capture_bindings {
                eprintln!(
                    "capture_slots local_store target={} function={} capture={} enclosing={} param_default={} active_loop={} force={}",
                    state.resolved_name,
                    function_name,
                    capture_name,
                    capture_originates_in_enclosing_local,
                    parameter_default_snapshot,
                    active_loop_capture,
                    force_runtime_slot,
                );
            }
            let Some((source_expression, source_is_runtime_local)) = self
                .identifier_store_capture_source_expression_for_local_value(
                    capture_name,
                    force_runtime_slot,
                )
            else {
                continue;
            };
            if source_is_runtime_local {
                let metadata_expression = self
                    .resolve_static_string_value(&source_expression)
                    .map(Expression::String)
                    .unwrap_or_else(|| source_expression.clone());
                let hidden_name = self.allocate_named_hidden_local(
                    &format!("closure_slot_{}_{}", state.resolved_name, capture_name),
                    self.infer_value_kind(&source_expression)
                        .unwrap_or(StaticValueKind::Unknown),
                );
                let hidden_local = self
                    .state
                    .runtime
                    .locals
                    .bindings
                    .get(&hidden_name)
                    .copied()
                    .expect("fresh closure capture slot local must exist");
                let source_statically_uninitialized =
                    if let Expression::Identifier(source_name) = &source_expression {
                        self.resolve_current_local_binding(source_name)
                            .as_ref()
                            .is_some_and(|(resolved_name, _)| {
                                self.local_lexical_capture_source_is_statically_uninitialized(
                                    resolved_name,
                                )
                            })
                    } else {
                        false
                    };
                let derived_constructor_this_capture = capture_name == "this"
                    && matches!(source_expression, Expression::This)
                    && self.current_function_is_derived_constructor();
                if source_statically_uninitialized || derived_constructor_this_capture {
                    self.push_i32_const(JS_UNDEFINED_TAG);
                } else if let Expression::Identifier(source_name) = &source_expression
                    && let Some((_, source_local)) = self.resolve_current_local_binding(source_name)
                {
                    self.push_local_get(source_local);
                } else {
                    self.emit_capture_source_expression_value(capture_name, &source_expression)?;
                }
                self.push_local_set(hidden_local);
                self.update_capture_slot_binding_from_expression(
                    &hidden_name,
                    &metadata_expression,
                )?;
                self.sync_capture_slot_runtime_object_shadows_from_expression(
                    &hidden_name,
                    &source_expression,
                )?;
                if let Expression::Identifier(source_binding_name) = &source_expression {
                    self.state
                        .speculation
                        .static_semantics
                        .capture_slot_source_bindings
                        .insert(hidden_name.clone(), source_binding_name.clone());
                } else if matches!(source_expression, Expression::This) {
                    self.state
                        .speculation
                        .static_semantics
                        .capture_slot_source_bindings
                        .insert(hidden_name.clone(), "this".to_string());
                } else if let Expression::Member { object, property } = &source_expression
                    && let Some(source_key) = Self::capture_slot_member_source_key(object, property)
                {
                    self.state
                        .speculation
                        .static_semantics
                        .capture_slot_source_bindings
                        .insert(hidden_name.clone(), source_key);
                }
                capture_slots.insert(capture_name.clone(), hidden_name);
            } else if let Expression::Identifier(source_binding_name) = source_expression {
                capture_slots.insert(capture_name.clone(), source_binding_name);
            }
        }

        if capture_slots.is_empty() {
            return Ok(());
        }

        let key = Self::identifier_function_value_capture_slots_key(&state.resolved_name);
        self.state
            .speculation
            .static_semantics
            .objects
            .member_function_capture_slots
            .insert(key.clone(), capture_slots.clone());
        if self.binding_key_is_global(&key) {
            self.backend
                .set_global_member_function_capture_slots(key, capture_slots);
        }

        Ok(())
    }

    fn iterator_next_cache_source_expression<'b>(
        &self,
        state: &'b PreparedIdentifierStoreState,
    ) -> Option<&'b Expression> {
        [
            state.call_source_snapshot_expression.as_ref(),
            Some(&state.canonical_value_expression),
            Some(&state.tracked_value_expression),
        ]
        .into_iter()
        .flatten()
        .find(|expression| {
            matches!(expression, Expression::GetIterator(_))
                || matches!(
                    expression,
                    Expression::Call { callee, arguments }
                        if arguments.is_empty()
                            && matches!(
                                callee.as_ref(),
                                Expression::Member { property, .. }
                                    if is_symbol_iterator_expression(property)
                            )
                )
        })
    }

    fn get_iterator_call_result_expression_for_cached_next(
        &self,
        callee: &Expression,
        arguments: &[CallArgument],
    ) -> Option<Expression> {
        self.resolve_static_call_result_expression_with_context(
            callee,
            arguments,
            self.current_function_name(),
        )
        .map(|(value, _)| value)
        .or_else(|| {
            self.get_iterator_call_terminal_return_expression_for_cached_next(callee, arguments)
        })
    }

    fn statement_is_ignorable_for_cached_iterator_call_return(statement: &Statement) -> bool {
        match statement {
            Statement::Block { body } | Statement::Declaration { body } => body
                .iter()
                .all(Self::statement_is_ignorable_for_cached_iterator_call_return),
            Statement::Expression(Expression::Call { .. }) => true,
            _ => false,
        }
    }

    fn get_iterator_call_terminal_return_expression_for_cached_next(
        &self,
        callee: &Expression,
        arguments: &[CallArgument],
    ) -> Option<Expression> {
        if !arguments.is_empty() {
            return None;
        }
        let LocalFunctionBinding::User(function_name) = self
            .resolve_function_binding_from_expression_with_context(
                callee,
                self.current_function_name(),
            )?
        else {
            return None;
        };
        let function = self.resolve_registered_function_declaration(&function_name)?;
        let (terminal_statement, effect_statements) = function.body.split_last()?;
        if !effect_statements
            .iter()
            .all(Self::statement_is_ignorable_for_cached_iterator_call_return)
        {
            return None;
        }
        let Statement::Return(return_value) = terminal_statement else {
            return None;
        };
        let capture_slots = self
            .resolve_function_expression_capture_slots(callee)
            .or_else(|| match callee {
                Expression::Member { object, property } => {
                    self.resolve_member_function_capture_slots(object, property)
                }
                _ => None,
            });
        let return_value = capture_slots
            .as_ref()
            .map(|capture_slots| self.substitute_capture_slot_bindings(return_value, capture_slots))
            .unwrap_or_else(|| return_value.clone());
        let materialized = self.materialize_static_expression(&return_value);
        Some(if static_expression_matches(&materialized, &return_value) {
            return_value
        } else {
            materialized
        })
    }

    fn iterator_object_expression_for_cached_next(
        &self,
        source: &Expression,
    ) -> Option<Expression> {
        let next_property = Expression::String("next".to_string());
        match source {
            Expression::GetIterator(iterated) => {
                let iterated = self.materialize_static_expression(iterated);
                if self
                    .resolve_member_function_binding(&iterated, &next_property)
                    .is_some()
                    || self
                        .resolve_member_getter_binding(&iterated, &next_property)
                        .is_some()
                {
                    return Some(iterated);
                }
                let iterator_property =
                    self.materialize_static_expression(&symbol_iterator_expression());
                let callee = Expression::Member {
                    object: Box::new(iterated),
                    property: Box::new(iterator_property),
                };
                self.get_iterator_call_result_expression_for_cached_next(&callee, &[])
            }
            Expression::Call { callee, arguments }
                if arguments.is_empty()
                    && matches!(
                        callee.as_ref(),
                        Expression::Member { property, .. }
                            if is_symbol_iterator_expression(property)
                    ) =>
            {
                self.get_iterator_call_result_expression_for_cached_next(callee, arguments)
            }
            _ => None,
        }
    }

    fn terminal_return_value_for_cached_iterator_next(body: &[Statement]) -> Option<Expression> {
        match body.last()? {
            Statement::Return(value) => Some(value.clone()),
            Statement::Block { body } | Statement::Declaration { body } => {
                Self::terminal_return_value_for_cached_iterator_next(body)
            }
            _ => None,
        }
    }

    fn user_getter_terminal_return_value_for_cached_iterator_next(
        &self,
        binding: &LocalFunctionBinding,
    ) -> Option<Expression> {
        let LocalFunctionBinding::User(function_name) = binding else {
            return None;
        };
        let function = self.resolve_registered_function_declaration(function_name)?;
        Self::terminal_return_value_for_cached_iterator_next(&function.body)
    }

    fn resolve_iterator_next_method_for_cache(
        &self,
        iterator_object: &Expression,
    ) -> Option<(LocalFunctionBinding, Option<BTreeMap<String, String>>, bool)> {
        let next_property = Expression::String("next".to_string());
        if let Some(function_binding) =
            self.resolve_member_function_binding(iterator_object, &next_property)
        {
            let capture_slots =
                self.resolve_member_function_capture_slots(iterator_object, &next_property);
            return Some((function_binding, capture_slots, false));
        }

        let getter_binding = self.resolve_member_getter_binding(iterator_object, &next_property)?;
        let returned_expression = self
            .resolve_function_binding_static_return_expression_with_call_frame(
                &getter_binding,
                &[],
                iterator_object,
            )
            .or_else(|| {
                self.user_getter_terminal_return_value_for_cached_iterator_next(&getter_binding)
            })?;
        let function_binding =
            self.resolve_function_binding_from_expression(&returned_expression)?;
        let capture_slots = self
            .resolve_function_expression_capture_slots(&returned_expression)
            .or_else(|| {
                self.resolve_member_function_capture_slots(iterator_object, &next_property)
            });
        Some((function_binding, capture_slots, true))
    }

    fn cache_identifier_store_iterator_next_method(
        &mut self,
        state: &PreparedIdentifierStoreState,
    ) -> DirectResult<()> {
        let trace = std::env::var_os("AYY_TRACE_ITERATOR_NEXT_CACHE").is_some();
        self.state
            .speculation
            .static_semantics
            .arrays
            .clear_cached_iterator_next_method_binding(&state.resolved_name);

        let Some(source_expression) = self.iterator_next_cache_source_expression(state) else {
            if trace {
                eprintln!(
                    "iterator_next_cache:{} no_source canonical={:?} tracked={:?}",
                    state.resolved_name,
                    state.canonical_value_expression,
                    state.tracked_value_expression
                );
            }
            return Ok(());
        };
        let Some(iterator_object) =
            self.iterator_object_expression_for_cached_next(source_expression)
        else {
            if trace {
                eprintln!(
                    "iterator_next_cache:{} no_iterator_object source={:?}",
                    state.resolved_name, source_expression
                );
            }
            return Ok(());
        };
        let Some((function_binding, capture_slots, should_emit_getter)) =
            self.resolve_iterator_next_method_for_cache(&iterator_object)
        else {
            if trace {
                eprintln!(
                    "iterator_next_cache:{} no_next iterator={:?}",
                    state.resolved_name, iterator_object
                );
            }
            return Ok(());
        };
        if trace {
            eprintln!(
                "iterator_next_cache:{} set iterator={:?} binding={:?} getter={}",
                state.resolved_name, iterator_object, function_binding, should_emit_getter
            );
        }

        if should_emit_getter {
            self.emit_numeric_expression(&Expression::Member {
                object: Box::new(iterator_object.clone()),
                property: Box::new(Expression::String("next".to_string())),
            })?;
            self.state.emission.output.instructions.push(0x1a);
        }

        self.state
            .speculation
            .static_semantics
            .arrays
            .set_cached_iterator_next_method_binding(
                &state.resolved_name,
                CachedIteratorNextMethodBinding {
                    function_binding,
                    this_expression: iterator_object,
                    capture_slots,
                },
            );
        Ok(())
    }

    fn resolve_identifier_store_shadow_source_owner(
        &self,
        expression: &Expression,
    ) -> Option<String> {
        match expression {
            Expression::Identifier(name) => {
                self.runtime_object_property_shadow_owner_name_for_identifier(name)
            }
            Expression::This => {
                self.runtime_object_property_shadow_owner_name_for_identifier("this")
            }
            _ => None,
        }
    }

    pub(super) fn sync_identifier_store_runtime_object_shadows(
        &mut self,
        target_name: &str,
        fallback_owner: &str,
        state: &PreparedIdentifierStoreState,
    ) -> DirectResult<()> {
        if state.is_internal_array_step_binding {
            return Ok(());
        }
        let trace_identifier_store = std::env::var_os("AYY_TRACE_IDENTIFIER_STORE").is_some();
        let target_owner = self
            .runtime_object_property_shadow_owner_name_for_identifier(target_name)
            .unwrap_or_else(|| fallback_owner.to_string());
        let source_owner = self
            .resolve_identifier_store_shadow_source_owner(&state.canonical_value_expression)
            .or_else(|| {
                self.resolve_identifier_store_shadow_source_owner(
                    &state.module_assignment_expression,
                )
            });
        let object_binding = self
            .state
            .speculation
            .static_semantics
            .local_object_binding(&state.resolved_name)
            .cloned()
            .or_else(|| {
                self.resolve_object_binding_from_expression(&state.object_binding_expression)
            })
            .or_else(|| {
                source_owner.as_deref().and_then(|source_owner| {
                    self.resolve_runtime_shadow_object_binding(source_owner)
                })
            })
            .map(|object_binding| {
                self.rewrite_static_new_this_object_binding_for_owner(
                    &object_binding,
                    &target_owner,
                )
            });

        if trace_identifier_store {
            eprintln!(
                "identifier_store:{target_name}:runtime_shadows target_owner={target_owner} source_owner={source_owner:?} object_binding_present={} object_binding_props={:?} object_binding_hidden={:?}",
                object_binding.is_some(),
                object_binding
                    .as_ref()
                    .map(ordered_object_property_names)
                    .unwrap_or_default(),
                object_binding
                    .as_ref()
                    .map(|binding| binding.non_enumerable_string_properties.clone())
                    .unwrap_or_default(),
            );
        }

        if let Some(object_binding) = object_binding.as_ref() {
            self.state
                .speculation
                .static_semantics
                .set_local_object_binding(&target_owner, object_binding.clone());
            let target_kind = if state.function_binding.is_some()
                || self
                    .resolve_function_binding_from_expression(&Expression::Identifier(
                        target_owner.clone(),
                    ))
                    .is_some()
            {
                StaticValueKind::Function
            } else {
                StaticValueKind::Object
            };
            self.state
                .speculation
                .static_semantics
                .set_local_kind(&target_owner, target_kind);
            if self.binding_name_is_global(&target_owner)
                || self.global_has_binding(&target_owner)
                || self.backend.global_has_lexical_binding(&target_owner)
                || self.global_has_implicit_binding(&target_owner)
            {
                self.backend
                    .sync_global_object_binding(&target_owner, Some(object_binding.clone()));
                self.backend
                    .set_global_binding_kind(&target_owner, target_kind);
            }
        }

        if source_owner.as_deref() == Some(target_owner.as_str()) {
            return Ok(());
        }

        self.clear_runtime_object_property_shadow_prefix(&target_owner);

        if let Some(source_owner) = source_owner {
            self.emit_runtime_object_property_shadow_copy(&source_owner, &target_owner)?;
            if let Some(object_binding) = object_binding.as_ref() {
                self.sync_runtime_object_property_shadow_static_metadata_from_binding(
                    &target_owner,
                    object_binding,
                );
            }
            return Ok(());
        }

        if let Some(object_binding) = object_binding {
            self.emit_runtime_object_property_shadow_seed_from_binding(
                &target_owner,
                &object_binding,
            )?;
            self.sync_runtime_object_property_shadow_static_metadata_from_binding(
                &target_owner,
                &object_binding,
            );
        }

        Ok(())
    }

    fn resolve_identifier_store_iterator_binding_source(
        &mut self,
        state: &PreparedIdentifierStoreState,
    ) -> Option<IteratorSourceKind> {
        let iterator_source_expression = match state
            .call_source_snapshot_expression
            .as_ref()
            .unwrap_or(&state.canonical_value_expression)
        {
            Expression::GetIterator(_) | Expression::Call { .. }
                if self
                    .resolve_simple_generator_source(
                        state
                            .call_source_snapshot_expression
                            .as_ref()
                            .unwrap_or(&state.canonical_value_expression),
                    )
                    .is_some()
                    || self
                        .resolve_async_yield_delegate_generator_plan(
                            state
                                .call_source_snapshot_expression
                                .as_ref()
                                .unwrap_or(&state.canonical_value_expression),
                            "__ayy_async_delegate_completion",
                        )
                        .is_some() =>
            {
                state
                    .call_source_snapshot_expression
                    .as_ref()
                    .unwrap_or(&state.canonical_value_expression)
            }
            _ => &state.tracked_value_expression,
        };
        self.resolve_local_array_iterator_source(iterator_source_expression)
    }

    fn resolve_identifier_store_iterator_binding_alias(
        &self,
        state: &PreparedIdentifierStoreState,
    ) -> Option<ArrayIteratorBinding> {
        fn identifier_from_iterator_alias_expression(expression: &Expression) -> Option<&str> {
            match expression {
                Expression::Identifier(name) => Some(name),
                Expression::GetIterator(iterated) => match iterated.as_ref() {
                    Expression::Identifier(name) => Some(name),
                    _ => None,
                },
                _ => None,
            }
        }

        let expressions = [
            state.call_source_snapshot_expression.as_ref(),
            Some(&state.canonical_value_expression),
            Some(&state.tracked_value_expression),
        ];
        for expression in expressions.into_iter().flatten() {
            if let Expression::GetIterator(iterated) = expression
                && let Some(binding) = self.existing_iterator_binding_for_expression(iterated, 6)
            {
                return Some(binding);
            }
            let Some(source_name) = identifier_from_iterator_alias_expression(expression) else {
                continue;
            };
            let Some(binding_name) = self.resolve_local_array_iterator_binding_name(source_name)
            else {
                continue;
            };
            if let Some(binding) = self
                .state
                .speculation
                .static_semantics
                .local_array_iterator_binding(&binding_name)
                .cloned()
            {
                return Some(binding);
            }
        }
        None
    }

    fn sync_identifier_store_iterator_entry_array_binding(
        &mut self,
        target_name: &str,
        value: &Expression,
    ) -> DirectResult<Option<ArrayValueBinding>> {
        let Expression::Member { object, property } = value else {
            return Ok(None);
        };
        if !matches!(property.as_ref(), Expression::String(name) if name == "value") {
            return Ok(None);
        }
        let Some(IteratorStepBinding::Runtime {
            entry_array: Some(entry_array),
            ..
        }) = self.resolve_iterator_step_binding_from_expression(object)
        else {
            return Ok(None);
        };

        let array_binding = ArrayValueBinding {
            values: vec![None, None],
        };
        let length_local = self.ensure_runtime_array_length_local(target_name);
        self.push_i32_const(2);
        self.push_local_set(length_local);

        let index_slot = self.ensure_runtime_array_slot_entry(target_name, 0);
        self.push_local_get(entry_array.index_local);
        self.push_local_set(index_slot.value_local);
        self.push_i32_const(1);
        self.push_local_set(index_slot.present_local);

        let value_slot = self.ensure_runtime_array_slot_entry(target_name, 1);
        self.push_local_get(entry_array.value_local);
        self.push_local_set(value_slot.value_local);
        self.push_i32_const(1);
        self.push_local_set(value_slot.present_local);

        self.state
            .speculation
            .static_semantics
            .set_local_array_binding(target_name, array_binding.clone());
        self.state
            .speculation
            .static_semantics
            .clear_tracked_array_specialized_function_values(target_name);
        if self.binding_name_is_global(target_name) {
            self.backend
                .sync_global_array_binding(target_name, Some(array_binding.clone()));
        }
        self.state
            .speculation
            .static_semantics
            .set_local_kind(target_name, StaticValueKind::Object);
        Ok(Some(array_binding))
    }

    pub(super) fn apply_identifier_store_shared_updates(
        &mut self,
        value_local: u32,
        state: &PreparedIdentifierStoreState,
    ) -> DirectResult<()> {
        let trace_identifier_store = std::env::var_os("AYY_TRACE_IDENTIFIER_STORE").is_some();
        let trace_step = |label: &str| {
            if trace_identifier_store {
                eprintln!("identifier_store:{}:shared:{label}", state.resolved_name);
            }
        };
        if state.resolved_name.starts_with("__ayy_target_object_")
            || state.resolved_name.starts_with("__ayy_target_property_")
        {
            self.state
                .speculation
                .static_semantics
                .set_local_value_binding(
                    &state.resolved_name,
                    state.canonical_value_expression.clone(),
                );
        }
        let value_references_internal_iterator_step =
            expression_references_internal_iterator_step(&state.canonical_value_expression)
                || expression_references_internal_iterator_step(&state.tracked_value_expression);
        let value_is_local_simple_async_generator_next_call = self
            .identifier_store_value_is_local_simple_async_generator_next_call(
                &state.canonical_value_expression,
            );
        let value_is_get_iterator =
            matches!(state.canonical_value_expression, Expression::GetIterator(_))
                || matches!(state.tracked_value_expression, Expression::GetIterator(_));
        trace_step("member_bindings:start");
        if !state.is_internal_array_step_binding {
            if !value_references_internal_iterator_step
                && !value_is_local_simple_async_generator_next_call
            {
                self.update_member_function_bindings_for_value(
                    &state.resolved_name,
                    &state.canonical_value_expression,
                    value_local,
                )?;
            }
            if (!value_references_internal_iterator_step || value_is_get_iterator)
                && !value_is_local_simple_async_generator_next_call
            {
                self.cache_identifier_store_iterator_next_method(state)?;
            }
        }
        trace_step("member_bindings:done");
        if !state.is_internal_iterator_temp && !value_references_internal_iterator_step {
            let specialized_value_expression = match &state.canonical_value_expression {
                Expression::Member { object, property }
                    if self
                        .resolve_member_getter_binding(object, property)
                        .is_some() =>
                {
                    &state.canonical_value_expression
                }
                _ => &state.tracked_value_expression,
            };
            trace_step("local_function:start");
            if let Some(function_binding) = state.function_binding.clone() {
                self.state
                    .speculation
                    .static_semantics
                    .set_local_function_binding(&state.resolved_name, function_binding);
            } else {
                self.update_local_function_binding(
                    &state.resolved_name,
                    &state.function_binding_expression,
                );
            }
            self.preserve_identifier_function_capture_slots_for_local_store(state)?;
            trace_step("local_function:done");
            trace_step("specialized_function:start");
            if !value_is_local_simple_async_generator_next_call {
                self.update_local_specialized_function_value(
                    &state.resolved_name,
                    specialized_value_expression,
                )?;
            }
            trace_step("specialized_function:done");
            trace_step("proxy:start");
            self.update_local_proxy_binding(&state.resolved_name, &state.tracked_value_expression);
            trace_step("proxy:done");
            if !(matches!(
                state.canonical_value_expression,
                Expression::Call { .. } | Expression::New { .. }
            ) && matches!(state.tracked_value_expression, Expression::Object(_)))
            {
                trace_step("object_literal_members:start");
                self.update_object_literal_member_bindings_for_value(
                    &state.resolved_name,
                    &state.tracked_object_expression,
                );
                trace_step("object_literal_members:done");
            }
            trace_step("array_binding:start");
            let should_copy_runtime_array_source =
                matches!(state.tracked_value_expression, Expression::Identifier(_))
                    && self
                        .runtime_array_binding_name_for_expression(&state.tracked_value_expression)
                        .is_some();
            let target_is_global_runtime_array_store = should_copy_runtime_array_source
                && (self.is_named_global_array_binding(&state.resolved_name)
                    || self
                        .backend
                        .global_binding_index(&state.resolved_name)
                        .is_some()
                    || self
                        .backend
                        .global_has_implicit_binding(&state.resolved_name));
            if self
                .sync_identifier_store_iterator_entry_array_binding(
                    &state.resolved_name,
                    &state.tracked_value_expression,
                )?
                .is_some()
            {
                trace_step("array_binding:entry_iterator");
            } else if target_is_global_runtime_array_store {
                self.state
                    .speculation
                    .static_semantics
                    .clear_local_array_binding(&state.resolved_name);
                self.state
                    .speculation
                    .static_semantics
                    .set_local_kind(&state.resolved_name, StaticValueKind::Object);
                self.backend
                    .mark_global_array_with_runtime_state(&state.resolved_name);
                if self.binding_name_is_global(&state.resolved_name) {
                    self.backend
                        .sync_global_array_binding(&state.resolved_name, None);
                }
                trace_step("array_binding:global_runtime_source");
            } else if let Some(array_binding) = state
                .array_binding
                .as_ref()
                .filter(|_| !should_copy_runtime_array_source)
            {
                let length_local = self.ensure_runtime_array_length_local(&state.resolved_name);
                self.push_i32_const(array_binding.values.len() as i32);
                self.push_local_set(length_local);
                self.ensure_runtime_array_slots_for_binding(&state.resolved_name, array_binding);
                self.state
                    .speculation
                    .static_semantics
                    .set_local_array_binding(&state.resolved_name, array_binding.clone());
                self.state
                    .speculation
                    .static_semantics
                    .clear_tracked_array_specialized_function_values(&state.resolved_name);
                if self.binding_name_is_global(&state.resolved_name) {
                    self.backend.sync_global_array_binding(
                        &state.resolved_name,
                        Some(array_binding.clone()),
                    );
                }
                self.state
                    .speculation
                    .static_semantics
                    .set_local_kind(&state.resolved_name, StaticValueKind::Object);
            } else {
                self.update_local_array_binding(
                    &state.resolved_name,
                    &state.tracked_value_expression,
                );
            }
            trace_step("array_binding:done");
            trace_step("resizable_array_buffer:start");
            self.update_local_resizable_array_buffer_binding(
                &state.resolved_name,
                &state.tracked_value_expression,
            )?;
            trace_step("resizable_array_buffer:done");
            trace_step("typed_array_view:start");
            self.update_local_typed_array_view_binding(
                &state.resolved_name,
                &state.tracked_value_expression,
            )?;
            trace_step("typed_array_view:done");
        }
        trace_step("iterator_step:start");
        self.update_local_iterator_step_binding(
            &state.resolved_name,
            &state.tracked_value_expression,
        );
        trace_step("iterator_step:done");
        if state.is_internal_array_step_binding {
            self.state
                .speculation
                .static_semantics
                .set_local_kind(&state.resolved_name, StaticValueKind::Object);
        }
        trace_step("object_binding:start");
        self.update_local_object_binding(&state.resolved_name, &state.object_binding_expression);
        let skip_static_object_seeds =
            expression_references_internal_iterator_step(&state.tracked_value_expression)
                || expression_references_internal_iterator_step(
                    state.prototype_source_expression(),
                );
        if !skip_static_object_seeds {
            self.seed_local_date_object_binding(
                &state.resolved_name,
                &state.tracked_value_expression,
            );
            self.seed_local_native_error_object_binding(
                &state.resolved_name,
                &state.tracked_value_expression,
            );
            self.seed_local_constructed_function_object_binding(
                &state.resolved_name,
                &state.tracked_value_expression,
            );
            self.seed_local_viewed_array_buffer_object_binding(
                &state.resolved_name,
                &state.tracked_value_expression,
            );
            self.seed_local_typed_array_object_binding(
                &state.resolved_name,
                state.prototype_source_expression(),
            );
        }
        trace_step("object_binding:done");
        if !state.is_internal_iterator_temp {
            trace_step("arguments_binding:start");
            let arguments_binding_expression =
                self.identifier_store_arguments_binding_expression(state);
            self.update_local_arguments_binding(&state.resolved_name, arguments_binding_expression);
            trace_step("arguments_binding:done");
            trace_step("descriptor_binding:start");
            self.update_local_descriptor_binding(
                &state.resolved_name,
                &state.descriptor_binding_expression,
            );
            trace_step("descriptor_binding:done");
            if let Some(descriptor) = state.returned_descriptor_binding.clone() {
                self.state
                    .speculation
                    .static_semantics
                    .objects
                    .local_descriptor_bindings
                    .insert(state.resolved_name.clone(), descriptor);
            }
        }
        if state.is_internal_array_step_binding {
            return Ok(());
        }
        if value_is_local_simple_async_generator_next_call {
            self.state
                .speculation
                .static_semantics
                .arrays
                .clear_cached_iterator_next_method_binding(&state.resolved_name);
            self.update_local_array_iterator_binding_with_source(&state.resolved_name, None);
            return Ok(());
        }
        trace_step("iterator_source:start");
        let iterator_binding_alias = self.resolve_identifier_store_iterator_binding_alias(state);
        let iterator_binding_source = self.resolve_identifier_store_iterator_binding_source(state);
        trace_step("iterator_source:done");
        trace_step("array_iterator_binding:start");
        if let Some(iterator_binding_alias) = iterator_binding_alias {
            if std::env::var_os("AYY_TRACE_SIMPLE_GENERATORS").is_some() {
                eprintln!(
                    "iterator_binding_alias name={} static_index={:?}",
                    state.resolved_name, iterator_binding_alias.static_index
                );
            }
            self.state
                .speculation
                .static_semantics
                .set_local_array_iterator_binding(&state.resolved_name, iterator_binding_alias);
            self.state
                .speculation
                .static_semantics
                .set_local_kind(&state.resolved_name, StaticValueKind::Object);
        } else {
            self.update_local_array_iterator_binding_with_source(
                &state.resolved_name,
                iterator_binding_source,
            );
        }
        if state.is_internal_array_iterator_binding {
            let value_binding = match (
                self.state
                    .speculation
                    .static_semantics
                    .local_value_binding(&state.resolved_name)
                    .cloned(),
                &state.canonical_value_expression,
                &state.module_assignment_expression,
            ) {
                (
                    _,
                    Expression::GetIterator(_) | Expression::Call { .. },
                    Expression::Object(_),
                ) => state.canonical_value_expression.clone(),
                (Some(existing), _, Expression::Object(_)) => existing,
                _ => state.module_assignment_expression.clone(),
            };
            self.state
                .speculation
                .static_semantics
                .set_local_value_binding(&state.resolved_name, value_binding);
        }
        trace_step("array_iterator_binding:done");
        Ok(())
    }
}
