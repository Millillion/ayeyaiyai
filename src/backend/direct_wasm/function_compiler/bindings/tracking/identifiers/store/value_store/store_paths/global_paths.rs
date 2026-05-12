use super::*;

fn expression_contains_object_spread(expression: &Expression) -> bool {
    match expression {
        Expression::Object(entries) => entries.iter().any(|entry| match entry {
            ObjectEntry::Data { key, value } => {
                expression_contains_object_spread(key) || expression_contains_object_spread(value)
            }
            ObjectEntry::Getter { key, getter } => {
                expression_contains_object_spread(key) || expression_contains_object_spread(getter)
            }
            ObjectEntry::Setter { key, setter } => {
                expression_contains_object_spread(key) || expression_contains_object_spread(setter)
            }
            ObjectEntry::Spread(_) => true,
        }),
        Expression::Array(elements) => elements.iter().any(|element| match element {
            ArrayElement::Expression(value) | ArrayElement::Spread(value) => {
                expression_contains_object_spread(value)
            }
        }),
        Expression::Member { object, property } => {
            expression_contains_object_spread(object) || expression_contains_object_spread(property)
        }
        Expression::SuperMember { property } => expression_contains_object_spread(property),
        Expression::Assign { value, .. }
        | Expression::AssignSuperMember { value, .. }
        | Expression::Await(value)
        | Expression::EnumerateKeys(value)
        | Expression::GetIterator(value)
        | Expression::IteratorClose(value)
        | Expression::Unary {
            expression: value, ..
        } => expression_contains_object_spread(value),
        Expression::AssignMember {
            object,
            property,
            value,
        } => {
            expression_contains_object_spread(object)
                || expression_contains_object_spread(property)
                || expression_contains_object_spread(value)
        }
        Expression::Binary { left, right, .. } => {
            expression_contains_object_spread(left) || expression_contains_object_spread(right)
        }
        Expression::Conditional {
            condition,
            then_expression,
            else_expression,
        } => {
            expression_contains_object_spread(condition)
                || expression_contains_object_spread(then_expression)
                || expression_contains_object_spread(else_expression)
        }
        Expression::Sequence(expressions) => {
            expressions.iter().any(expression_contains_object_spread)
        }
        Expression::Call { callee, arguments }
        | Expression::SuperCall { callee, arguments }
        | Expression::New { callee, arguments } => {
            expression_contains_object_spread(callee)
                || arguments
                    .iter()
                    .any(|argument| expression_contains_object_spread(argument.expression()))
        }
        Expression::Number(_)
        | Expression::BigInt(_)
        | Expression::String(_)
        | Expression::Bool(_)
        | Expression::Null
        | Expression::Undefined
        | Expression::NewTarget
        | Expression::Identifier(_)
        | Expression::This
        | Expression::Sent
        | Expression::Update { .. } => false,
    }
}

fn global_store_expression_references_internal_iterator_step(expression: &Expression) -> bool {
    match expression {
        Expression::Identifier(name) => {
            name.starts_with("__ayy_array_step_")
                || name.starts_with("__ayy_array_iter_value_")
                || name.starts_with("__ayy_array_iter_done_")
                || name.starts_with("__ayy_for_of_step_")
                || name.starts_with("__ayy_for_of_iter_value_")
                || name.starts_with("__ayy_for_of_iter_done_")
                || name.starts_with("__ayy_binding_value_")
        }
        Expression::Array(elements) => elements.iter().any(|element| match element {
            ArrayElement::Expression(value) | ArrayElement::Spread(value) => {
                global_store_expression_references_internal_iterator_step(value)
            }
        }),
        Expression::Object(entries) => entries.iter().any(|entry| match entry {
            ObjectEntry::Data { key, value } => {
                global_store_expression_references_internal_iterator_step(key)
                    || global_store_expression_references_internal_iterator_step(value)
            }
            ObjectEntry::Getter { key, getter } => {
                global_store_expression_references_internal_iterator_step(key)
                    || global_store_expression_references_internal_iterator_step(getter)
            }
            ObjectEntry::Setter { key, setter } => {
                global_store_expression_references_internal_iterator_step(key)
                    || global_store_expression_references_internal_iterator_step(setter)
            }
            ObjectEntry::Spread(value) => {
                global_store_expression_references_internal_iterator_step(value)
            }
        }),
        Expression::Binary { left, right, .. } => {
            global_store_expression_references_internal_iterator_step(left)
                || global_store_expression_references_internal_iterator_step(right)
        }
        Expression::Conditional {
            condition,
            then_expression,
            else_expression,
        } => {
            global_store_expression_references_internal_iterator_step(condition)
                || global_store_expression_references_internal_iterator_step(then_expression)
                || global_store_expression_references_internal_iterator_step(else_expression)
        }
        Expression::Member { object, property } => {
            global_store_expression_references_internal_iterator_step(object)
                || global_store_expression_references_internal_iterator_step(property)
        }
        Expression::Unary { expression, .. }
        | Expression::Await(expression)
        | Expression::EnumerateKeys(expression)
        | Expression::GetIterator(expression)
        | Expression::IteratorClose(expression) => {
            global_store_expression_references_internal_iterator_step(expression)
        }
        Expression::Assign { value, .. } => {
            global_store_expression_references_internal_iterator_step(value)
        }
        Expression::AssignMember {
            object,
            property,
            value,
        } => {
            global_store_expression_references_internal_iterator_step(object)
                || global_store_expression_references_internal_iterator_step(property)
                || global_store_expression_references_internal_iterator_step(value)
        }
        Expression::AssignSuperMember { property, value } => {
            global_store_expression_references_internal_iterator_step(property)
                || global_store_expression_references_internal_iterator_step(value)
        }
        Expression::Call { callee, arguments }
        | Expression::New { callee, arguments }
        | Expression::SuperCall { callee, arguments } => {
            global_store_expression_references_internal_iterator_step(callee)
                || arguments.iter().any(|argument| match argument {
                    CallArgument::Expression(value) | CallArgument::Spread(value) => {
                        global_store_expression_references_internal_iterator_step(value)
                    }
                })
        }
        Expression::SuperMember { property } => {
            global_store_expression_references_internal_iterator_step(property)
        }
        _ => false,
    }
}

fn state_is_internal_iterator_step_value_store(state: &PreparedIdentifierStoreState) -> bool {
    (state.resolved_name.starts_with("__ayy_array_iter_done_")
        || state.resolved_name.starts_with("__ayy_destructure_value_")
        || state.resolved_name.starts_with("__ayy_binding_value_"))
        && (global_store_expression_references_internal_iterator_step(
            &state.canonical_value_expression,
        ) || global_store_expression_references_internal_iterator_step(
            &state.tracked_value_expression,
        ) || global_store_expression_references_internal_iterator_step(
            &state.module_assignment_expression,
        ))
}

fn state_stores_internal_iterator_step_value(state: &PreparedIdentifierStoreState) -> bool {
    global_store_expression_references_internal_iterator_step(&state.canonical_value_expression)
        || global_store_expression_references_internal_iterator_step(
            &state.tracked_value_expression,
        )
        || global_store_expression_references_internal_iterator_step(
            &state.module_assignment_expression,
        )
}

fn state_stores_internal_iterator_value_temp(state: &PreparedIdentifierStoreState) -> bool {
    (state.resolved_name.starts_with("__ayy_array_iter_value_")
        || state.resolved_name.starts_with("__ayy_for_of_iter_value_"))
        && state_stores_internal_iterator_step_value(state)
}

impl<'a> FunctionCompiler<'a> {
    fn store_function_references_nested_function_in_body(
        function: &FunctionDeclaration,
        nested_function_name: &str,
    ) -> bool {
        collect_referenced_binding_names_from_statements(&function.body)
            .contains(nested_function_name)
    }

    fn store_function_references_nested_function_in_parameter_default(
        function: &FunctionDeclaration,
        nested_function_name: &str,
    ) -> bool {
        function.params.iter().any(|parameter| {
            parameter.default.as_ref().is_some_and(|default| {
                let mut referenced = HashSet::new();
                collect_referenced_binding_names_from_expression(default, &mut referenced);
                referenced.contains(nested_function_name)
            })
        })
    }

    fn store_function_has_body_local_binding_source(
        function: &FunctionDeclaration,
        source_name: &str,
    ) -> bool {
        let mut bindings = collect_declared_bindings_from_statements_recursive(&function.body);
        bindings.extend(collect_static_direct_eval_var_bindings(function));
        bindings
            .into_iter()
            .any(|name| scoped_binding_source_name(&name).unwrap_or(&name) == source_name)
    }

    fn store_function_has_parameter_binding_source(
        function: &FunctionDeclaration,
        source_name: &str,
    ) -> bool {
        source_name == "arguments"
            || function.params.iter().any(|parameter| {
                scoped_binding_source_name(&parameter.name).unwrap_or(&parameter.name)
                    == source_name
            })
    }

    pub(in crate::backend::direct_wasm) fn assigned_user_function_capture_originates_in_enclosing_local(
        &self,
        function_name: &str,
        capture_name: &str,
    ) -> bool {
        let functions = self
            .user_functions()
            .into_iter()
            .filter_map(|function| self.prepared_function_declaration(&function.name).cloned())
            .collect::<Vec<_>>();
        functions.iter().any(|candidate| {
            if candidate.name == function_name {
                return false;
            }
            let referenced_in_body =
                Self::store_function_references_nested_function_in_body(candidate, function_name);
            let referenced_in_parameter_default =
                Self::store_function_references_nested_function_in_parameter_default(
                    candidate,
                    function_name,
                );
            let source_in_body =
                Self::store_function_has_body_local_binding_source(candidate, capture_name);
            let source_in_parameters =
                Self::store_function_has_parameter_binding_source(candidate, capture_name);

            (referenced_in_body && (source_in_body || source_in_parameters))
                || (referenced_in_parameter_default && source_in_parameters)
        })
    }

    pub(super) fn assigned_user_function_capture_needs_parameter_default_snapshot(
        &self,
        function_name: &str,
        capture_name: &str,
    ) -> bool {
        let functions = self
            .user_functions()
            .into_iter()
            .filter_map(|function| self.prepared_function_declaration(&function.name).cloned())
            .collect::<Vec<_>>();
        functions.iter().any(|candidate| {
            candidate.name != function_name
                && Self::store_function_references_nested_function_in_parameter_default(
                    candidate,
                    function_name,
                )
                && Self::store_function_has_body_local_binding_source(candidate, capture_name)
                && !Self::store_function_has_parameter_binding_source(candidate, capture_name)
        })
    }

    fn parameter_default_can_reference_current_local(&self, capture_name: &str) -> bool {
        if !self.state.parameters.in_parameter_default_initialization {
            return true;
        }
        if self
            .current_static_direct_eval_var_binding_source_name(capture_name)
            .is_some()
        {
            return true;
        }
        capture_name == "arguments"
            || self
                .state
                .parameters
                .parameter_names
                .iter()
                .any(|parameter| {
                    scoped_binding_source_name(parameter).unwrap_or(parameter) == capture_name
                })
    }

    pub(super) fn current_static_direct_eval_var_binding_source_name(
        &self,
        capture_name: &str,
    ) -> Option<String> {
        let current_function_name = self.current_function_name()?;
        let function = self.prepared_function_declaration(current_function_name)?;
        let capture_source_name = scoped_binding_source_name(capture_name).unwrap_or(capture_name);
        collect_static_direct_eval_var_bindings(function)
            .into_iter()
            .find_map(|name| {
                let source_name = scoped_binding_source_name(&name).unwrap_or(&name);
                (source_name == capture_source_name).then(|| source_name.to_string())
            })
    }

    pub(super) fn current_static_direct_eval_closure_slot_name(
        &self,
        source_name: &str,
    ) -> Option<String> {
        let current_function_name = self.current_function_name()?;
        Some(format!(
            "__ayy_closure_env_{}_{}",
            current_function_name, source_name
        ))
    }

    fn initialize_static_direct_eval_closure_capture_slot(
        &mut self,
        source_name: &str,
    ) -> DirectResult<String> {
        let hidden_name = self
            .current_static_direct_eval_closure_slot_name(source_name)
            .expect("current direct eval closure slot requires a current function");
        let hidden_binding = self.ensure_implicit_global_binding(&hidden_name);
        let source_expression = Expression::Identifier(source_name.to_string());

        if self.resolve_current_local_binding(source_name).is_some()
            || self
                .resolve_user_function_capture_hidden_name(source_name)
                .is_some()
            || self
                .resolve_eval_local_function_hidden_name(source_name)
                .is_some()
        {
            self.emit_numeric_expression(&source_expression)?;
        } else {
            self.push_i32_const(JS_UNDEFINED_TAG);
        }
        self.push_global_set(hidden_binding.value_index);
        self.push_i32_const(1);
        self.push_global_set(hidden_binding.present_index);

        self.update_static_global_assignment_metadata(&hidden_name, &source_expression);
        self.preserve_exact_static_global_number_binding(&hidden_name, &source_expression);
        self.update_global_specialized_function_value(&hidden_name, &source_expression)?;
        self.ensure_global_property_descriptor_value(&hidden_name, &source_expression, true);
        self.sync_capture_slot_runtime_object_shadows_from_expression(
            &hidden_name,
            &source_expression,
        )?;
        self.state
            .speculation
            .static_semantics
            .capture_slot_source_bindings
            .insert(hidden_name.clone(), source_name.to_string());

        Ok(hidden_name)
    }

    pub(super) fn sync_static_direct_eval_closure_capture_slot_from_local(
        &mut self,
        resolved_name: &str,
        value_local: u32,
        state: &PreparedIdentifierStoreState,
    ) -> DirectResult<()> {
        let Some(source_name) =
            self.current_static_direct_eval_var_binding_source_name(resolved_name)
        else {
            return Ok(());
        };
        let Some(hidden_name) = self.current_static_direct_eval_closure_slot_name(&source_name)
        else {
            return Ok(());
        };
        let Some(hidden_binding) = self.implicit_global_binding(&hidden_name) else {
            return Ok(());
        };

        self.push_local_get(value_local);
        self.push_global_set(hidden_binding.value_index);
        self.push_i32_const(1);
        self.push_global_set(hidden_binding.present_index);

        self.update_static_global_assignment_metadata(
            &hidden_name,
            &state.module_assignment_expression,
        );
        self.preserve_exact_static_global_number_binding(
            &hidden_name,
            &state.module_assignment_expression,
        );
        self.update_global_specialized_function_value(
            &hidden_name,
            &state.module_assignment_expression,
        )?;
        self.update_global_property_descriptor_value(
            &hidden_name,
            &state.module_assignment_expression,
        );
        self.sync_capture_slot_runtime_object_shadows_from_expression(
            &hidden_name,
            &state.module_assignment_expression,
        )?;
        self.state
            .speculation
            .static_semantics
            .capture_slot_source_bindings
            .insert(hidden_name, source_name);

        Ok(())
    }

    fn emit_identifier_store_capture_source_expression(
        &mut self,
        capture_name: &str,
        source_expression: &Expression,
        prefer_global_identifier: bool,
    ) -> DirectResult<()> {
        if prefer_global_identifier && let Expression::Identifier(name) = source_expression {
            if let Some(global_index) = self.resolve_global_binding_index(name) {
                if let Some(binding) = self.backend.lexical_global_binding(name) {
                    self.push_global_get(binding.initialized_index);
                    self.state.emission.output.instructions.push(0x04);
                    self.state.emission.output.instructions.push(I32_TYPE);
                    self.push_control_frame();
                    self.push_global_get(global_index);
                    self.state.emission.output.instructions.push(0x05);
                    self.emit_named_error_throw("ReferenceError")?;
                    self.state.emission.output.instructions.push(0x0b);
                    self.pop_control_frame();
                } else {
                    self.push_global_get(global_index);
                }
                return Ok(());
            }
            if let Some(binding) = self.implicit_global_binding(name) {
                self.push_global_get(binding.present_index);
                self.state.emission.output.instructions.push(0x04);
                self.state.emission.output.instructions.push(I32_TYPE);
                self.push_control_frame();
                self.push_global_get(binding.value_index);
                self.state.emission.output.instructions.push(0x05);
                self.emit_named_error_throw("ReferenceError")?;
                self.state.emission.output.instructions.push(0x0b);
                self.pop_control_frame();
                return Ok(());
            }
        }
        if let Expression::Identifier(name) = source_expression
            && let Some((_, local_index)) = self.resolve_current_local_binding(name)
        {
            self.push_local_get(local_index);
            return Ok(());
        }
        self.emit_capture_source_expression_value(capture_name, source_expression)
    }

    fn identifier_store_capture_source_expression(
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
        if self.parameter_default_can_reference_current_local(capture_name)
            && self.resolve_current_local_binding(capture_name).is_some()
        {
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

    fn preserve_identifier_function_capture_slots_for_global_store(
        &mut self,
        name: &str,
        state: &PreparedIdentifierStoreState,
    ) -> DirectResult<()> {
        let trace_capture_bindings = std::env::var_os("AYY_TRACE_CAPTURE_BINDINGS").is_some();
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

        let call_capture_source_bindings =
            state
                .call_source_snapshot_expression
                .as_ref()
                .and_then(|source| {
                    self.resolve_constructor_capture_source_bindings_from_expression(source)
                });
        let mut capture_slots = BTreeMap::new();
        for capture_name in capture_bindings.keys() {
            if let Some(source_name) =
                self.current_static_direct_eval_var_binding_source_name(capture_name)
            {
                let hidden_name =
                    self.initialize_static_direct_eval_closure_capture_slot(&source_name)?;
                capture_slots.insert(capture_name.clone(), hidden_name);
                continue;
            }
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
                    "capture_slots global_store target={name} function={function_name} capture={capture_name} enclosing={capture_originates_in_enclosing_local} param_default={parameter_default_snapshot} active_loop={active_loop_capture} force={force_runtime_slot}",
                );
            }
            let call_capture_source_expression =
                call_capture_source_bindings.as_ref().and_then(|bindings| {
                    bindings.get(capture_name).cloned().filter(|source| {
                        !matches!(source, Expression::Identifier(name) if name == capture_name)
                    })
                });
            let Some((source_expression, source_is_runtime_local)) = call_capture_source_expression
                .map(|source| (source, true))
                .or_else(|| {
                    self.identifier_store_capture_source_expression(
                        capture_name,
                        force_runtime_slot,
                    )
                })
            else {
                continue;
            };
            if source_is_runtime_local {
                let metadata_expression = self
                    .resolve_static_string_value(&source_expression)
                    .map(Expression::String)
                    .unwrap_or_else(|| source_expression.clone());
                let hidden_name = format!("__ayy_closure_slot_{}_{}", name, capture_name);
                let hidden_binding = self.ensure_implicit_global_binding(&hidden_name);
                let derived_constructor_this_capture = capture_name == "this"
                    && matches!(source_expression, Expression::This)
                    && self.current_function_is_derived_constructor();
                if derived_constructor_this_capture {
                    self.push_i32_const(JS_UNDEFINED_TAG);
                } else {
                    self.emit_identifier_store_capture_source_expression(
                        capture_name,
                        &source_expression,
                        parameter_default_snapshot,
                    )?;
                }
                self.push_global_set(hidden_binding.value_index);
                self.push_i32_const(if derived_constructor_this_capture { 0 } else { 1 });
                self.push_global_set(hidden_binding.present_index);
                if !capture_name.starts_with("__ayy_class_brand_") {
                    self.update_static_global_assignment_metadata(
                        &hidden_name,
                        &metadata_expression,
                    );
                    let function_binding =
                        self.resolve_function_binding_from_expression(&metadata_expression);
                    self.preserve_static_global_function_binding(
                        &hidden_name,
                        function_binding.as_ref(),
                    );
                    self.preserve_exact_static_global_number_binding(
                        &hidden_name,
                        &metadata_expression,
                    );
                    self.update_global_specialized_function_value(
                        &hidden_name,
                        &metadata_expression,
                    )?;
                    self.ensure_global_property_descriptor_value(
                        &hidden_name,
                        &metadata_expression,
                        true,
                    );
                }
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

        if !capture_slots.is_empty() {
            if trace_capture_bindings {
                eprintln!(
                    "capture_slots global_store_set target={name} function={function_name} slots={capture_slots:?}",
                );
            }
            let key = Self::identifier_function_value_capture_slots_key(name);
            self.backend
                .set_global_member_function_capture_slots(key, capture_slots);
        }
        Ok(())
    }

    pub(in crate::backend::direct_wasm) fn add_active_with_scope_function_capture_bindings(
        &mut self,
        function_name: &str,
        capture_bindings: &mut HashMap<String, String>,
    ) -> DirectResult<()> {
        if self.state.emission.lexical_scopes.with_scopes.is_empty() {
            return Ok(());
        }
        let Some(function) = self
            .resolve_registered_function_declaration(function_name)
            .cloned()
        else {
            return Ok(());
        };

        let scope_bindings = collect_function_constructor_local_bindings(&function)
            .into_iter()
            .map(|name| {
                scoped_binding_source_name(&name)
                    .unwrap_or(&name)
                    .to_string()
            })
            .collect::<HashSet<_>>();
        let mut referenced = collect_referenced_binding_names_from_statements(&function.body);
        for parameter in &function.params {
            if let Some(default) = &parameter.default {
                collect_referenced_binding_names_from_expression(default, &mut referenced);
            }
        }
        for statement in &function.body {
            collect_assigned_binding_names_from_statement(statement, &mut referenced);
        }

        let mut added = false;
        for name in referenced {
            let source_name = scoped_binding_source_name(&name)
                .unwrap_or(&name)
                .to_string();
            if scope_bindings.contains(&source_name)
                || capture_bindings.contains_key(&source_name)
                || self
                    .resolve_with_scope_binding_for_capture_source(&source_name)
                    .is_none()
            {
                continue;
            }
            let hidden_name = format!("__ayy_capture_binding__{function_name}__{source_name}");
            self.ensure_implicit_global_binding(&hidden_name);
            capture_bindings.insert(source_name, hidden_name);
            added = true;
        }

        if added {
            self.backend
                .function_registry
                .analysis
                .set_user_function_capture_bindings(function_name, capture_bindings.clone());
        }

        Ok(())
    }

    fn emit_store_declared_lexical_global_from_local(
        &mut self,
        global_index: u32,
        binding: LexicalGlobalBinding,
        value_local: u32,
    ) -> DirectResult<()> {
        self.push_global_get(binding.initialized_index);
        self.state.emission.output.instructions.push(0x04);
        self.state
            .emission
            .output
            .instructions
            .push(EMPTY_BLOCK_TYPE);
        self.push_control_frame();
        if binding.mutable {
            self.push_local_get(value_local);
            self.push_global_set(global_index);
        } else {
            self.emit_named_error_throw("TypeError")?;
        }
        self.state.emission.output.instructions.push(0x05);
        self.emit_named_error_throw("ReferenceError")?;
        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();
        Ok(())
    }

    fn preserve_identifier_store_global_metadata(
        &mut self,
        name: &str,
        state: &PreparedIdentifierStoreState,
        ensure_descriptor: bool,
    ) -> DirectResult<()> {
        if self.identifier_store_state_depends_on_active_loop_assignment(state) {
            self.clear_global_binding_state(name);
            self.preserve_active_loop_safe_global_metadata(name, state)?;
            return Ok(());
        }
        let object_binding =
            self.resolve_object_binding_from_expression(&state.object_binding_expression);
        let metadata_assignment_expression = if (name.starts_with("__ayy_target_object_")
            || name.starts_with("__ayy_target_property_"))
            && matches!(
                state.canonical_value_expression,
                Expression::Identifier(_) | Expression::This
            ) {
            state.canonical_value_expression.clone()
        } else if expression_contains_object_spread(&state.module_assignment_expression) {
            object_binding
                .as_ref()
                .map(object_binding_to_expression)
                .unwrap_or_else(|| state.module_assignment_expression.clone())
        } else {
            state.module_assignment_expression.clone()
        };
        self.update_object_prototype_binding_from_value(name, state.prototype_binding_expression());
        self.update_static_global_assignment_metadata(name, &metadata_assignment_expression);
        if let Some(object_binding) = object_binding {
            self.backend
                .sync_global_object_binding(name, Some(object_binding));
        }
        self.seed_global_boxed_primitive_object_binding(name, state.prototype_source_expression());
        self.seed_global_date_object_binding(name, state.prototype_source_expression());
        self.seed_global_native_error_object_binding(name, state.prototype_source_expression());
        self.seed_global_constructed_function_object_binding(
            name,
            state.prototype_source_expression(),
        );
        self.seed_global_viewed_array_buffer_object_binding(
            name,
            state.prototype_source_expression(),
        );
        self.seed_global_typed_array_object_binding(name, state.prototype_source_expression());
        if let Some(array_binding) = state.array_binding.as_ref() {
            self.backend
                .sync_global_array_binding(name, Some(array_binding.clone()));
        }
        self.preserve_exact_static_global_string_binding(
            name,
            state.exact_static_number,
            state.static_string_value.as_ref(),
        );
        self.preserve_static_global_function_binding(name, state.function_binding.as_ref());
        self.preserve_identifier_function_capture_slots_for_global_store(name, state)?;
        self.backend.sync_global_arguments_binding(
            name,
            self.resolve_identifier_store_arguments_binding(state),
        );
        self.preserve_exact_static_global_number_binding(name, &metadata_assignment_expression);
        self.update_global_specialized_function_value(name, &metadata_assignment_expression)?;
        if ensure_descriptor {
            self.ensure_global_property_descriptor_value(
                name,
                &metadata_assignment_expression,
                true,
            );
        } else {
            self.update_global_property_descriptor_value(name, &metadata_assignment_expression);
        }
        Ok(())
    }

    fn preserve_active_loop_safe_global_metadata(
        &mut self,
        name: &str,
        state: &PreparedIdentifierStoreState,
    ) -> DirectResult<()> {
        if name.starts_with("__ayy_target_property_") {
            self.preserve_exact_static_global_string_binding(
                name,
                state.exact_static_number,
                state.static_string_value.as_ref(),
            );
        }
        if (name.starts_with("__ayy_target_object_") || name.starts_with("__ayy_target_property_"))
            && matches!(
                state.canonical_value_expression,
                Expression::Identifier(_) | Expression::This
            )
        {
            self.backend.sync_global_expression_binding(
                name,
                Some(state.canonical_value_expression.clone()),
            );
            self.backend
                .shared_global_semantics
                .values
                .set_value_binding(name.to_string(), state.canonical_value_expression.clone());
        }
        if matches!(
            state.kind,
            Some(StaticValueKind::Object | StaticValueKind::Function | StaticValueKind::String)
        ) {
            self.backend
                .set_global_binding_kind(name, state.kind.expect("matched Some above"));
        }
        if let Some(object_binding) =
            self.resolve_object_binding_from_expression(&state.object_binding_expression)
        {
            self.backend
                .sync_global_object_binding(name, Some(object_binding));
            self.backend
                .set_global_binding_kind(name, StaticValueKind::Object);
        }
        if state.function_binding.is_some()
            && !self
                .expression_depends_on_active_loop_assignment(&state.function_binding_expression)
        {
            self.preserve_static_global_function_binding(name, state.function_binding.as_ref());
            self.update_global_specialized_function_value(
                name,
                &state.function_binding_expression,
            )?;
        }
        Ok(())
    }

    fn identifier_store_has_preservable_global_metadata(
        &self,
        state: &PreparedIdentifierStoreState,
    ) -> bool {
        if self.identifier_store_state_depends_on_active_loop_assignment(state) {
            return matches!(
                state.kind,
                Some(StaticValueKind::Object | StaticValueKind::Function | StaticValueKind::String)
            ) || state.function_binding.is_some()
                && !self.expression_depends_on_active_loop_assignment(
                    &state.function_binding_expression,
                );
        }
        state.kind.is_some()
            || state.function_binding.is_some()
            || state.static_string_value.is_some()
            || state.exact_static_number.is_some()
            || state.array_binding.is_some()
            || state.returned_descriptor_binding.is_some()
            || self
                .resolve_function_binding_from_expression(&state.function_binding_expression)
                .is_some()
            || self
                .resolve_object_binding_from_expression(&state.object_binding_expression)
                .is_some()
            || self
                .resolve_identifier_store_arguments_binding(state)
                .is_some()
            || self
                .resolve_descriptor_binding_from_expression(&state.descriptor_binding_expression)
                .is_some()
    }

    fn identifier_store_state_depends_on_active_loop_assignment(
        &self,
        state: &PreparedIdentifierStoreState,
    ) -> bool {
        self.expression_depends_on_active_loop_assignment(&state.canonical_value_expression)
            || self.expression_depends_on_active_loop_assignment(&state.tracked_value_expression)
            || self
                .expression_depends_on_active_loop_assignment(&state.descriptor_binding_expression)
            || self.expression_depends_on_active_loop_assignment(&state.tracked_object_expression)
            || self.expression_depends_on_active_loop_assignment(&state.function_binding_expression)
            || self.expression_depends_on_active_loop_assignment(&state.object_binding_expression)
            || self
                .expression_depends_on_active_loop_assignment(&state.module_assignment_expression)
    }

    pub(super) fn try_store_identifier_value_via_isolated_indirect_eval_path(
        &mut self,
        name: &str,
        value_local: u32,
        state: &PreparedIdentifierStoreState,
    ) -> DirectResult<bool> {
        if !self
            .state
            .speculation
            .execution_context
            .isolated_indirect_eval
            || state.resolved_local_binding.is_some()
            || self.parameter_scope_arguments_local_for(name).is_some()
        {
            return Ok(false);
        }

        if let Some(global_index) = self.backend.global_binding_index(name) {
            if self.backend.lexical_global_binding(name).is_some() {
                self.store_identifier_value_to_declared_global(
                    name,
                    value_local,
                    global_index,
                    state,
                )?;
            } else {
                self.preserve_identifier_store_global_metadata(name, state, false)?;
                self.push_local_get(value_local);
                self.push_global_set(global_index);
            }
            return Ok(true);
        }
        if self.emit_store_eval_local_function_binding_from_local(name, value_local)? {
            return Ok(true);
        }
        if let Some(binding) = self.backend.implicit_global_binding(name) {
            self.preserve_identifier_store_global_metadata(name, state, true)?;
            self.emit_store_implicit_global_from_local(binding, value_local)?;
            return Ok(true);
        }
        let binding = self.ensure_implicit_global_binding(name);
        self.preserve_identifier_store_global_metadata(name, state, true)?;
        self.emit_store_implicit_global_from_local(binding, value_local)?;
        Ok(true)
    }

    pub(super) fn store_identifier_value_to_declared_global(
        &mut self,
        name: &str,
        value_local: u32,
        global_index: u32,
        state: &PreparedIdentifierStoreState,
    ) -> DirectResult<()> {
        if (state.is_internal_array_step_binding
            || state_stores_internal_iterator_step_value(state))
            && (name.starts_with("__ayy_") || self.backend.lexical_global_binding(name).is_none())
        {
            if state_stores_internal_iterator_value_temp(state) {
                let kind = state.kind.unwrap_or(StaticValueKind::Unknown);
                self.backend.set_global_binding_kind(name, kind);
                self.backend
                    .shared_global_semantics
                    .set_global_binding_kind(name, kind);
                self.backend.sync_global_expression_binding(
                    name,
                    Some(state.module_assignment_expression.clone()),
                );
                self.backend
                    .shared_global_semantics
                    .values
                    .set_value_binding(
                        name.to_string(),
                        state.module_assignment_expression.clone(),
                    );
            }
            self.push_local_get(value_local);
            self.push_global_set(global_index);
            return Ok(());
        }
        if let Some(binding) = self.backend.lexical_global_binding(name) {
            if binding.mutable {
                let static_self_store = matches!(
                    &state.canonical_value_expression,
                    Expression::Identifier(source_name) if source_name == name
                ) && matches!(
                    &state.module_assignment_expression,
                    Expression::Identifier(source_name) if source_name == name
                );
                if static_self_store {
                    self.emit_store_declared_lexical_global_from_local(
                        global_index,
                        binding,
                        value_local,
                    )?;
                    self.sync_identifier_store_runtime_object_shadows(name, name, state)?;
                    if self.emit_force_global_runtime_array_state_from_internal_rest_source(
                        name,
                        &state.tracked_value_expression,
                    )? {
                    } else if self.emit_sync_global_runtime_array_state_from_runtime_source(
                        name,
                        &state.tracked_value_expression,
                    )? {
                    } else if let Some(array_binding) = state.array_binding.as_ref() {
                        self.emit_sync_global_runtime_array_state_from_binding(
                            name,
                            array_binding,
                        )?;
                    }
                    return Ok(());
                }
                let had_static_initialization_metadata = self.global_value_binding(name).is_some()
                    || self.global_binding_kind(name).is_some()
                    || self.backend.global_function_binding(name).is_some()
                    || self.backend.global_array_binding(name).is_some()
                    || self.backend.global_object_binding(name).is_some();
                let has_new_static_metadata =
                    self.identifier_store_has_preservable_global_metadata(state);
                self.clear_global_binding_state(name);
                if had_static_initialization_metadata || has_new_static_metadata {
                    self.preserve_identifier_store_global_metadata(name, state, false)?;
                }
            }
            self.emit_store_declared_lexical_global_from_local(global_index, binding, value_local)?;
            self.sync_identifier_store_runtime_object_shadows(name, name, state)?;
            if self.emit_force_global_runtime_array_state_from_internal_rest_source(
                name,
                &state.tracked_value_expression,
            )? {
            } else if self.emit_sync_global_runtime_array_state_from_runtime_source(
                name,
                &state.tracked_value_expression,
            )? {
            } else if let Some(array_binding) = state.array_binding.as_ref() {
                self.emit_sync_global_runtime_array_state_from_binding(name, array_binding)?;
            }
            return Ok(());
        }

        if !self
            .state
            .speculation
            .execution_context
            .isolated_indirect_eval
        {
            self.preserve_identifier_store_global_metadata(name, state, false)?;
        }
        self.push_local_get(value_local);
        self.push_global_set(global_index);
        self.sync_identifier_store_runtime_object_shadows(name, name, state)?;
        if self.emit_force_global_runtime_array_state_from_internal_rest_source(
            name,
            &state.tracked_value_expression,
        )? {
        } else if self.emit_sync_global_runtime_array_state_from_runtime_source(
            name,
            &state.tracked_value_expression,
        )? {
        } else if let Some(array_binding) = state.array_binding.as_ref() {
            self.emit_sync_global_runtime_array_state_from_binding(name, array_binding)?;
        }
        Ok(())
    }

    pub(super) fn initialize_identifier_value_to_declared_global(
        &mut self,
        name: &str,
        value_local: u32,
        global_index: u32,
        state: &PreparedIdentifierStoreState,
    ) -> DirectResult<()> {
        if state.is_internal_array_step_binding || state_stores_internal_iterator_step_value(state)
        {
            if state_stores_internal_iterator_value_temp(state) {
                let kind = state.kind.unwrap_or(StaticValueKind::Unknown);
                self.backend.set_global_binding_kind(name, kind);
                self.backend
                    .shared_global_semantics
                    .set_global_binding_kind(name, kind);
                self.backend.sync_global_expression_binding(
                    name,
                    Some(state.module_assignment_expression.clone()),
                );
                self.backend
                    .shared_global_semantics
                    .values
                    .set_value_binding(
                        name.to_string(),
                        state.module_assignment_expression.clone(),
                    );
            }
            self.push_local_get(value_local);
            self.push_global_set(global_index);
            if let Some(binding) = self.backend.lexical_global_binding(name) {
                self.push_i32_const(1);
                self.push_global_set(binding.initialized_index);
            }
            return Ok(());
        }
        if !self
            .state
            .speculation
            .execution_context
            .isolated_indirect_eval
        {
            self.preserve_identifier_store_global_metadata(name, state, false)?;
        }
        self.push_local_get(value_local);
        self.push_global_set(global_index);
        if let Some(binding) = self.backend.lexical_global_binding(name) {
            self.push_i32_const(1);
            self.push_global_set(binding.initialized_index);
        }
        self.sync_identifier_store_runtime_object_shadows(name, name, state)?;
        if self.emit_force_global_runtime_array_state_from_internal_rest_source(
            name,
            &state.tracked_value_expression,
        )? {
        } else if self.emit_sync_global_runtime_array_state_from_runtime_source(
            name,
            &state.tracked_value_expression,
        )? {
        } else if let Some(array_binding) = state.array_binding.as_ref() {
            self.emit_sync_global_runtime_array_state_from_binding(name, array_binding)?;
        }
        Ok(())
    }

    pub(super) fn store_identifier_value_to_implicit_global(
        &mut self,
        name: &str,
        value_local: u32,
        binding: ImplicitGlobalBinding,
        state: &PreparedIdentifierStoreState,
    ) -> DirectResult<()> {
        if state.is_internal_array_step_binding || state_stores_internal_iterator_step_value(state)
        {
            self.emit_store_implicit_global_from_local(binding, value_local)?;
            self.sync_identifier_store_runtime_object_shadows(name, name, state)?;
            return Ok(());
        }
        if !self
            .state
            .speculation
            .execution_context
            .isolated_indirect_eval
        {
            self.preserve_identifier_store_global_metadata(name, state, true)?;
        }
        self.emit_store_implicit_global_from_local(binding, value_local)?;
        self.sync_identifier_store_runtime_object_shadows(name, name, state)?;
        if self.emit_force_global_runtime_array_state_from_internal_rest_source(
            name,
            &state.tracked_value_expression,
        )? {
        } else if self.emit_sync_global_runtime_array_state_from_runtime_source(
            name,
            &state.tracked_value_expression,
        )? {
        } else if let Some(array_binding) = state.array_binding.as_ref() {
            self.emit_sync_global_runtime_array_state_from_binding(name, array_binding)?;
        }
        Ok(())
    }
}
