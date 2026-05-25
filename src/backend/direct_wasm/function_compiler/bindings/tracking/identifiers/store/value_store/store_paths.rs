use super::*;

fn is_internal_array_iterator_binding_name(name: &str) -> bool {
    name.strip_prefix("__ayy_array_iter_")
        .is_some_and(|suffix| !suffix.is_empty() && suffix.chars().all(|ch| ch.is_ascii_digit()))
}

fn is_internal_array_step_binding_name(name: &str) -> bool {
    name.starts_with("__ayy_array_step_") || name.starts_with("__ayy_for_of_step_")
}

fn is_internal_assignment_temp(name: &str) -> bool {
    name.starts_with("__ayy_optional_base_")
        || name.starts_with("__ayy_target_object_")
        || name.starts_with("__ayy_target_property_")
        || name.starts_with("__ayy_postfix_previous_")
}

fn expression_is_identifier_iterator_method_call(expression: &Expression) -> bool {
    matches!(
        expression,
        Expression::Call { callee, .. }
            if matches!(
                callee.as_ref(),
                Expression::Member { object, property }
                    if matches!(object.as_ref(), Expression::Identifier(_))
                        && matches!(
                            property.as_ref(),
                            Expression::String(name)
                                if matches!(name.as_str(), "next" | "return" | "throw")
                        )
            )
    )
}

struct PreparedIdentifierStoreState {
    canonical_value_expression: Expression,
    tracked_value_expression: Expression,
    descriptor_binding_expression: Expression,
    tracked_object_expression: Expression,
    call_source_snapshot_expression: Option<Expression>,
    prototype_source_snapshot_expression: Option<Expression>,
    function_binding_expression: Expression,
    function_binding: Option<LocalFunctionBinding>,
    object_binding_expression: Expression,
    object_binding: Option<ObjectValueBinding>,
    kind: Option<StaticValueKind>,
    static_string_value: Option<String>,
    exact_static_number: Option<f64>,
    array_binding: Option<ArrayValueBinding>,
    module_assignment_expression: Expression,
    resolved_local_binding: Option<(String, u32)>,
    returned_descriptor_binding: Option<PropertyDescriptorBinding>,
    resolved_name: String,
    is_internal_array_iterator_binding: bool,
    is_internal_array_step_binding: bool,
    is_internal_iterator_temp: bool,
}

enum IdentifierReferenceStoreTarget {
    Current,
    ResolvedLocal(String, u32),
    Capture,
    DeclaredGlobal(u32),
    EvalLocal,
    ExistingImplicitGlobal(ImplicitGlobalBinding),
    NewImplicitGlobal,
}

impl PreparedIdentifierStoreState {
    fn prototype_source_expression(&self) -> &Expression {
        self.prototype_source_snapshot_expression
            .as_ref()
            .or(self.call_source_snapshot_expression.as_ref())
            .unwrap_or(&self.canonical_value_expression)
    }

    fn prototype_binding_expression(&self) -> &Expression {
        if self.prototype_source_snapshot_expression.is_some() {
            return self.prototype_source_expression();
        }
        if matches!(
            self.module_assignment_expression,
            Expression::Identifier(_) | Expression::This
        ) {
            &self.module_assignment_expression
        } else {
            self.prototype_source_expression()
        }
    }
}

impl<'a> FunctionCompiler<'a> {
    fn resolve_identifier_store_arguments_binding(
        &self,
        state: &PreparedIdentifierStoreState,
    ) -> Option<ArgumentsValueBinding> {
        let tracked =
            self.resolve_arguments_binding_from_expression(&state.tracked_value_expression);
        let canonical =
            self.resolve_arguments_binding_from_expression(&state.canonical_value_expression);
        if std::env::var_os("AYY_TRACE_IDENTIFIER_STORE").is_some() {
            eprintln!(
                "identifier_store:{}:arguments_binding tracked={} canonical={}",
                state.resolved_name,
                tracked.is_some(),
                canonical.is_some(),
            );
        }
        tracked.or(canonical)
    }

    fn identifier_store_arguments_binding_expression<'b>(
        &self,
        state: &'b PreparedIdentifierStoreState,
    ) -> &'b Expression {
        if self
            .resolve_arguments_binding_from_expression(&state.tracked_value_expression)
            .is_some()
            || self.is_direct_arguments_object(&state.tracked_value_expression)
        {
            &state.tracked_value_expression
        } else {
            &state.canonical_value_expression
        }
    }
}

#[path = "store_paths/capture_paths.rs"]
mod capture_paths;
#[path = "store_paths/common_updates.rs"]
mod common_updates;
#[path = "store_paths/global_paths.rs"]
mod global_paths;
#[path = "store_paths/local_paths.rs"]
mod local_paths;

impl<'a> FunctionCompiler<'a> {
    fn state_stores_static_iterator_method_result(
        state: &PreparedIdentifierStoreState,
    ) -> Option<&ObjectValueBinding> {
        if !expression_is_identifier_iterator_method_call(&state.canonical_value_expression)
            || state.call_source_snapshot_expression.is_none()
        {
            return None;
        }
        state.object_binding.as_ref()
    }

    fn preserve_static_iterator_method_result_bindings(
        &mut self,
        state: &PreparedIdentifierStoreState,
        object_binding: &ObjectValueBinding,
        sync_global_name: Option<&str>,
    ) {
        let value_expression = state.module_assignment_expression.clone();
        self.state
            .speculation
            .static_semantics
            .set_local_value_binding(&state.resolved_name, value_expression.clone());
        self.state
            .speculation
            .static_semantics
            .set_local_object_binding(&state.resolved_name, object_binding.clone());
        self.state
            .speculation
            .static_semantics
            .set_local_kind(&state.resolved_name, StaticValueKind::Object);
        self.state
            .speculation
            .static_semantics
            .clear_local_function_binding(&state.resolved_name);
        self.state
            .speculation
            .static_semantics
            .clear_local_array_binding(&state.resolved_name);
        self.state
            .speculation
            .static_semantics
            .clear_local_array_iterator_binding(&state.resolved_name);
        self.state
            .speculation
            .static_semantics
            .clear_local_iterator_step_binding(&state.resolved_name);
        self.state
            .parameters
            .direct_arguments_aliases
            .remove(&state.resolved_name);
        self.state
            .parameters
            .local_arguments_bindings
            .remove(&state.resolved_name);
        self.state
            .speculation
            .static_semantics
            .objects
            .local_descriptor_bindings
            .remove(&state.resolved_name);

        if let Some(global_name) = sync_global_name {
            self.backend
                .set_global_binding_kind(global_name, StaticValueKind::Object);
            self.backend
                .shared_global_semantics
                .set_global_binding_kind(global_name, StaticValueKind::Object);
            self.backend
                .sync_global_expression_binding(global_name, Some(value_expression.clone()));
            self.backend
                .shared_global_semantics
                .values
                .set_value_binding(global_name.to_string(), value_expression);
            self.backend
                .sync_global_object_binding(global_name, Some(object_binding.clone()));
            self.backend
                .shared_global_semantics
                .values
                .sync_object_binding(global_name, Some(object_binding.clone()));
            self.backend.sync_global_array_binding(global_name, None);
            self.backend
                .shared_global_semantics
                .values
                .sync_array_binding(global_name, None);
            self.backend
                .sync_global_arguments_binding(global_name, None);
            self.backend
                .shared_global_semantics
                .values
                .sync_arguments_binding(global_name, None);
            self.backend.sync_global_function_binding(global_name, None);
            self.backend
                .shared_global_semantics
                .clear_global_function_binding(global_name);
            self.update_global_property_descriptor_value(
                global_name,
                &state.module_assignment_expression,
            );
        }
    }

    fn try_store_static_iterator_method_result_fast(
        &mut self,
        name: &str,
        value_local: u32,
        state: &PreparedIdentifierStoreState,
        target: &IdentifierReferenceStoreTarget,
    ) -> DirectResult<bool> {
        let Some(object_binding) = Self::state_stores_static_iterator_method_result(state) else {
            return Ok(false);
        };

        match target {
            IdentifierReferenceStoreTarget::ResolvedLocal(resolved_name, local_index) => {
                if self
                    .local_lexical_initialized_local(resolved_name)
                    .is_some()
                {
                    return Ok(false);
                }
                self.push_local_get(value_local);
                self.push_local_set(*local_index);
                self.preserve_static_iterator_method_result_bindings(state, object_binding, None);
            }
            IdentifierReferenceStoreTarget::DeclaredGlobal(global_index) => {
                if self.backend.lexical_global_binding(name).is_some() {
                    return Ok(false);
                }
                self.push_local_get(value_local);
                self.push_global_set(*global_index);
                self.preserve_static_iterator_method_result_bindings(
                    state,
                    object_binding,
                    Some(name),
                );
            }
            IdentifierReferenceStoreTarget::ExistingImplicitGlobal(binding) => {
                self.emit_store_implicit_global_from_local(*binding, value_local)?;
                self.preserve_static_iterator_method_result_bindings(
                    state,
                    object_binding,
                    Some(name),
                );
            }
            IdentifierReferenceStoreTarget::NewImplicitGlobal => {
                let binding = self.ensure_implicit_global_binding(name);
                self.emit_store_implicit_global_from_local(binding, value_local)?;
                self.preserve_static_iterator_method_result_bindings(
                    state,
                    object_binding,
                    Some(name),
                );
            }
            IdentifierReferenceStoreTarget::Current
                if state.resolved_local_binding.is_none()
                    && self
                        .resolve_user_function_capture_hidden_name(name)
                        .is_none()
                    && self.resolve_eval_local_function_hidden_name(name).is_none()
                    && self.backend.lexical_global_binding(name).is_none() =>
            {
                if let Some(global_index) = self.backend.global_binding_index(name) {
                    self.push_local_get(value_local);
                    self.push_global_set(global_index);
                    self.preserve_static_iterator_method_result_bindings(
                        state,
                        object_binding,
                        Some(name),
                    );
                } else if let Some(binding) = self.backend.implicit_global_binding(name) {
                    self.emit_store_implicit_global_from_local(binding, value_local)?;
                    self.preserve_static_iterator_method_result_bindings(
                        state,
                        object_binding,
                        Some(name),
                    );
                } else {
                    let binding = self.ensure_implicit_global_binding(name);
                    self.emit_store_implicit_global_from_local(binding, value_local)?;
                    self.preserve_static_iterator_method_result_bindings(
                        state,
                        object_binding,
                        Some(name),
                    );
                }
            }
            _ => return Ok(false),
        }

        if std::env::var_os("AYY_TRACE_IDENTIFIER_STORE").is_some() {
            eprintln!("identifier_store:{name}:static_iterator_result_fast");
        }
        Ok(true)
    }

    pub(in crate::backend::direct_wasm) fn resolve_current_local_binding_by_source_name(
        &self,
        source_name: &str,
    ) -> Option<(String, u32)> {
        if let Some(resolved) = self.resolve_current_local_binding(source_name) {
            return Some(resolved);
        }

        let mut matching_bindings = self
            .state
            .runtime
            .locals
            .bindings
            .iter()
            .filter(|(name, _)| {
                scoped_binding_source_name(name).is_some_and(|source| source == source_name)
            })
            .map(|(name, index)| (name.clone(), *index));
        let resolved = matching_bindings.next()?;
        matching_bindings.next().is_none().then_some(resolved)
    }

    fn store_prepared_identifier_value_local_with_mode(
        &mut self,
        name: &str,
        value_local: u32,
        prepared: PreparedIdentifierValueStore,
        initialize_declared_global: bool,
    ) -> DirectResult<()> {
        self.store_prepared_identifier_value_local_with_target(
            name,
            value_local,
            prepared,
            initialize_declared_global,
            IdentifierReferenceStoreTarget::Current,
        )
    }

    fn update_parameter_scope_arguments_static_metadata(
        &mut self,
        state: &PreparedIdentifierStoreState,
    ) {
        let name = Self::parameter_scope_arguments_binding_name();
        self.update_local_value_binding(name, &state.module_assignment_expression);

        if let Some(object_binding) = state.object_binding.clone() {
            self.state
                .speculation
                .static_semantics
                .set_local_object_binding(name, object_binding);
        } else {
            self.state
                .speculation
                .static_semantics
                .clear_local_object_binding(name);
        }

        if let Some(array_binding) = state.array_binding.clone() {
            self.state
                .speculation
                .static_semantics
                .set_local_array_binding(name, array_binding);
        } else {
            self.state
                .speculation
                .static_semantics
                .clear_local_array_binding(name);
        }

        if let Some(function_binding) = state.function_binding.clone() {
            self.state
                .speculation
                .static_semantics
                .set_local_function_binding(name, function_binding);
        } else {
            self.state
                .speculation
                .static_semantics
                .clear_local_function_binding(name);
        }

        self.state
            .speculation
            .static_semantics
            .set_local_kind(name, state.kind.unwrap_or(StaticValueKind::Unknown));
    }

    fn store_prepared_identifier_value_local_with_target(
        &mut self,
        name: &str,
        value_local: u32,
        prepared: PreparedIdentifierValueStore,
        initialize_declared_global: bool,
        target: IdentifierReferenceStoreTarget,
    ) -> DirectResult<()> {
        let trace_identifier_store = std::env::var_os("AYY_TRACE_IDENTIFIER_STORE").is_some();
        let PreparedIdentifierValueStore {
            canonical_value_expression,
            tracked_value_expression,
            descriptor_binding_expression,
            tracked_object_expression,
            call_source_snapshot_expression,
            prototype_source_snapshot_expression,
            function_binding_expression,
            function_binding,
            object_binding_expression,
            object_binding,
            kind,
            static_string_value,
            exact_static_number,
            array_binding,
            module_assignment_expression,
            mut resolved_local_binding,
            returned_descriptor_binding,
            runtime_value_override,
        } = prepared;
        match &target {
            IdentifierReferenceStoreTarget::Current => {}
            IdentifierReferenceStoreTarget::ResolvedLocal(resolved_name, local_index) => {
                resolved_local_binding = Some((resolved_name.clone(), *local_index));
            }
            IdentifierReferenceStoreTarget::Capture
            | IdentifierReferenceStoreTarget::DeclaredGlobal(_)
            | IdentifierReferenceStoreTarget::EvalLocal
            | IdentifierReferenceStoreTarget::ExistingImplicitGlobal(_)
            | IdentifierReferenceStoreTarget::NewImplicitGlobal => {
                resolved_local_binding = None;
            }
        }

        let resolved_name = resolved_local_binding
            .as_ref()
            .map(|(resolved_name, _)| resolved_name.as_str())
            .unwrap_or(name)
            .to_string();
        let is_internal_array_iterator_binding =
            is_internal_array_iterator_binding_name(&resolved_name);
        let is_internal_array_step_binding = is_internal_array_step_binding_name(&resolved_name);
        let is_internal_iterator_temp =
            is_internal_array_iterator_binding || is_internal_array_step_binding;
        let state = PreparedIdentifierStoreState {
            canonical_value_expression,
            tracked_value_expression,
            descriptor_binding_expression,
            tracked_object_expression,
            call_source_snapshot_expression,
            prototype_source_snapshot_expression,
            function_binding_expression,
            function_binding,
            object_binding_expression,
            object_binding,
            kind,
            static_string_value,
            exact_static_number,
            array_binding,
            module_assignment_expression,
            resolved_local_binding,
            returned_descriptor_binding,
            resolved_name,
            is_internal_array_iterator_binding,
            is_internal_array_step_binding,
            is_internal_iterator_temp,
        };
        if let Some(runtime_value_override) = runtime_value_override {
            self.emit_numeric_expression(&runtime_value_override)?;
            self.push_local_set(value_local);
        }
        if trace_identifier_store {
            eprintln!("identifier_store:{name}:prepared");
        }

        if self.try_store_static_iterator_method_result_fast(name, value_local, &state, &target)? {
            if trace_identifier_store {
                eprintln!("identifier_store:{name}:done");
            }
            return Ok(());
        }

        if self.try_store_identifier_value_via_isolated_indirect_eval_path(
            name,
            value_local,
            &state,
            initialize_declared_global,
        )? {
            if trace_identifier_store {
                eprintln!("identifier_store:{name}:isolated_eval");
            }
            return Ok(());
        }

        if let Some(parameter_scope_arguments_local) =
            self.parameter_scope_arguments_local_for(name)
        {
            self.push_local_get(value_local);
            self.push_local_set(parameter_scope_arguments_local);
            self.update_parameter_scope_arguments_static_metadata(&state);
        }

        if trace_identifier_store {
            eprintln!("identifier_store:{name}:shared_updates:start");
        }
        let target_may_rebind_global = match &target {
            IdentifierReferenceStoreTarget::DeclaredGlobal(_)
            | IdentifierReferenceStoreTarget::ExistingImplicitGlobal(_) => true,
            IdentifierReferenceStoreTarget::Current => {
                state.resolved_local_binding.is_none()
                    && (self.backend.global_binding_index(name).is_some()
                        || self.backend.global_has_implicit_binding(name))
            }
            IdentifierReferenceStoreTarget::ResolvedLocal(_, _)
            | IdentifierReferenceStoreTarget::Capture
            | IdentifierReferenceStoreTarget::EvalLocal
            | IdentifierReferenceStoreTarget::NewImplicitGlobal => false,
        };
        if !initialize_declared_global && target_may_rebind_global {
            self.detach_global_reference_aliases_before_rebind(name, &state);
        }
        self.apply_identifier_store_shared_updates(value_local, &state)?;
        if trace_identifier_store {
            eprintln!("identifier_store:{name}:shared_updates:done");
        }

        if is_internal_assignment_temp(name)
            && state.resolved_local_binding.is_none()
            && matches!(
                target,
                IdentifierReferenceStoreTarget::Current
                    | IdentifierReferenceStoreTarget::ExistingImplicitGlobal(_)
                    | IdentifierReferenceStoreTarget::NewImplicitGlobal
            )
        {
            let local_index = self
                .ensure_named_internal_local(name, state.kind.unwrap_or(StaticValueKind::Unknown));
            if trace_identifier_store {
                eprintln!(
                    "identifier_store:{name}:store_internal_target_temp local_index={local_index}"
                );
            }
            self.store_identifier_value_to_resolved_local(
                name,
                value_local,
                name,
                local_index,
                &state,
            )?;
            if trace_identifier_store {
                eprintln!("identifier_store:{name}:done");
            }
            return Ok(());
        }

        match target {
            IdentifierReferenceStoreTarget::ResolvedLocal(resolved_name, local_index) => {
                if trace_identifier_store {
                    eprintln!(
                        "identifier_store:{name}:store_pre_resolved_local resolved_name={resolved_name} local_index={local_index}"
                    );
                }
                if initialize_declared_global {
                    self.initialize_identifier_value_to_resolved_local(
                        name,
                        value_local,
                        &resolved_name,
                        local_index,
                        &state,
                    )?;
                } else {
                    self.store_identifier_value_to_resolved_local(
                        name,
                        value_local,
                        &resolved_name,
                        local_index,
                        &state,
                    )?;
                }
            }
            IdentifierReferenceStoreTarget::Capture => {
                if trace_identifier_store {
                    eprintln!("identifier_store:{name}:store_pre_resolved_capture");
                }
                self.store_identifier_value_to_capture_binding(name, value_local, &state)?;
            }
            IdentifierReferenceStoreTarget::DeclaredGlobal(global_index) => {
                if trace_identifier_store {
                    eprintln!("identifier_store:{name}:store_pre_resolved_global");
                }
                if initialize_declared_global {
                    self.initialize_identifier_value_to_declared_global(
                        name,
                        value_local,
                        global_index,
                        &state,
                    )?;
                } else {
                    self.store_identifier_value_to_declared_global(
                        name,
                        value_local,
                        global_index,
                        &state,
                    )?;
                }
            }
            IdentifierReferenceStoreTarget::EvalLocal => {
                if trace_identifier_store {
                    eprintln!("identifier_store:{name}:store_pre_resolved_eval_local_hidden");
                }
                self.store_identifier_value_to_eval_local_hidden(name, value_local, &state)?;
            }
            IdentifierReferenceStoreTarget::ExistingImplicitGlobal(binding) => {
                if trace_identifier_store {
                    eprintln!("identifier_store:{name}:store_pre_resolved_implicit_existing");
                }
                self.store_identifier_value_to_implicit_global(name, value_local, binding, &state)?;
            }
            IdentifierReferenceStoreTarget::NewImplicitGlobal => {
                let binding = self.ensure_implicit_global_binding(name);
                if trace_identifier_store {
                    eprintln!("identifier_store:{name}:store_pre_resolved_implicit_new");
                }
                self.store_identifier_value_to_implicit_global(name, value_local, binding, &state)?;
            }
            IdentifierReferenceStoreTarget::Current => {
                self.store_prepared_identifier_value_local_current_target(
                    name,
                    value_local,
                    initialize_declared_global,
                    &state,
                    trace_identifier_store,
                )?;
            }
        }
        if trace_identifier_store {
            eprintln!("identifier_store:{name}:done");
        }

        Ok(())
    }

    fn store_prepared_identifier_value_local_current_target(
        &mut self,
        name: &str,
        value_local: u32,
        initialize_declared_global: bool,
        state: &PreparedIdentifierStoreState,
        trace_identifier_store: bool,
    ) -> DirectResult<()> {
        if let Some((resolved_name, local_index)) = state.resolved_local_binding.as_ref() {
            if trace_identifier_store {
                eprintln!("identifier_store:{name}:store_local");
            }
            if initialize_declared_global {
                self.initialize_identifier_value_to_resolved_local(
                    name,
                    value_local,
                    resolved_name,
                    *local_index,
                    &state,
                )?;
            } else {
                self.store_identifier_value_to_resolved_local(
                    name,
                    value_local,
                    resolved_name,
                    *local_index,
                    &state,
                )?;
            }
        } else if self
            .resolve_user_function_capture_hidden_name(name)
            .is_some()
        {
            if trace_identifier_store {
                eprintln!("identifier_store:{name}:store_capture");
            }
            self.store_identifier_value_to_capture_binding(name, value_local, &state)?;
        } else if let Some(global_index) = self.backend.global_binding_index(name) {
            if trace_identifier_store {
                eprintln!("identifier_store:{name}:store_declared_global");
            }
            if initialize_declared_global {
                self.initialize_identifier_value_to_declared_global(
                    name,
                    value_local,
                    global_index,
                    &state,
                )?;
            } else {
                self.store_identifier_value_to_declared_global(
                    name,
                    value_local,
                    global_index,
                    &state,
                )?;
            }
        } else if self.resolve_eval_local_function_hidden_name(name).is_some() {
            if trace_identifier_store {
                eprintln!("identifier_store:{name}:store_eval_local_hidden");
            }
            self.store_identifier_value_to_eval_local_hidden(name, value_local, &state)?;
        } else if let Some(binding) = self.backend.implicit_global_binding(name) {
            if trace_identifier_store {
                eprintln!("identifier_store:{name}:store_implicit_existing");
            }
            self.store_identifier_value_to_implicit_global(name, value_local, binding, &state)?;
        } else {
            let binding = self.ensure_implicit_global_binding(name);
            if trace_identifier_store {
                eprintln!("identifier_store:{name}:store_implicit_new");
            }
            self.store_identifier_value_to_implicit_global(name, value_local, binding, &state)?;
        }
        Ok(())
    }

    pub(in crate::backend::direct_wasm) fn emit_store_identifier_value_local_with_reference_target(
        &mut self,
        name: &str,
        value_expression: &Expression,
        value_local: u32,
        resolved_local_binding: Option<(String, u32)>,
        capture_binding: bool,
        declared_global_index: Option<u32>,
        eval_local_binding: bool,
        implicit_global_binding: Option<ImplicitGlobalBinding>,
        unresolvable_reference: bool,
    ) -> DirectResult<()> {
        if self.assignment_targets_immutable_class_binding(name) {
            if std::env::var_os("AYY_TRACE_IDENTIFIER_STORE").is_some() {
                eprintln!("identifier_store:{name}:immutable_class_binding_type_error");
            }
            self.emit_named_error_throw("TypeError")?;
            return Ok(());
        }
        if self.assignment_targets_immutable_function_self_binding(name) {
            if std::env::var_os("AYY_TRACE_IDENTIFIER_STORE").is_some() {
                eprintln!("identifier_store:{name}:immutable_function_self_binding");
            }
            if self.state.speculation.execution_context.strict_mode {
                self.emit_named_error_throw("TypeError")?;
            }
            return Ok(());
        }
        let prepared = self.prepare_identifier_value_store(name, value_expression);
        let target = if let Some((resolved_name, local_index)) = resolved_local_binding {
            IdentifierReferenceStoreTarget::ResolvedLocal(resolved_name, local_index)
        } else if capture_binding {
            IdentifierReferenceStoreTarget::Capture
        } else if let Some(global_index) = declared_global_index {
            IdentifierReferenceStoreTarget::DeclaredGlobal(global_index)
        } else if eval_local_binding {
            IdentifierReferenceStoreTarget::EvalLocal
        } else if let Some(binding) = implicit_global_binding {
            IdentifierReferenceStoreTarget::ExistingImplicitGlobal(binding)
        } else if unresolvable_reference {
            IdentifierReferenceStoreTarget::NewImplicitGlobal
        } else {
            IdentifierReferenceStoreTarget::Current
        };
        self.store_prepared_identifier_value_local_with_target(
            name,
            value_local,
            prepared,
            false,
            target,
        )
    }

    pub(super) fn store_prepared_identifier_value_local(
        &mut self,
        name: &str,
        value_local: u32,
        prepared: PreparedIdentifierValueStore,
    ) -> DirectResult<()> {
        self.store_prepared_identifier_value_local_with_mode(name, value_local, prepared, false)
    }

    pub(super) fn store_prepared_identifier_value_local_for_initialization(
        &mut self,
        name: &str,
        value_local: u32,
        prepared: PreparedIdentifierValueStore,
    ) -> DirectResult<()> {
        self.store_prepared_identifier_value_local_with_mode(name, value_local, prepared, true)
    }
}
