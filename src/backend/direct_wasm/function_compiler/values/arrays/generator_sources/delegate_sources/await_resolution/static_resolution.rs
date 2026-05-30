use super::*;

const MODULE_NAMESPACE_DESCRIPTOR_MODULE_INDEX: &str = "__ayy$module$namespace$moduleIndex";
const MODULE_REEXPORT_DESCRIPTOR_MODULE_INDEX: &str = "__ayy$module$reexport$moduleIndex";
const MODULE_REEXPORT_DESCRIPTOR_NAME: &str = "__ayy$module$reexport$name";
const DYNAMIC_IMPORT_DEFER_PHASE: &str = "__ayy$importPhase$defer";

thread_local! {
    static STATIC_DYNAMIC_IMPORT_MODULE_THROW_CACHE: RefCell<HashMap<String, Option<Expression>>> =
        RefCell::new(HashMap::new());
}

pub(super) fn reset_static_dynamic_import_caches() {
    STATIC_DYNAMIC_IMPORT_MODULE_THROW_CACHE.with(|cache| cache.borrow_mut().clear());
}

impl<'a> FunctionCompiler<'a> {
    fn static_module_promise_init_call_expression(&self, name: &str) -> Option<Expression> {
        if !name.starts_with("__ayy_module_promise_") {
            return None;
        }
        let resolved_local_name = self
            .resolve_current_local_binding(name)
            .map(|(resolved_name, _)| resolved_name);
        let active_name = resolved_local_name.as_deref().unwrap_or(name);
        let value = self
            .state
            .speculation
            .static_semantics
            .local_value_binding(active_name)
            .or_else(|| {
                resolved_local_name.as_deref().and_then(|resolved_name| {
                    self.state
                        .speculation
                        .static_semantics
                        .local_value_binding(resolved_name)
                })
            })
            .or_else(|| self.global_value_binding(name))?;
        match value {
            Expression::Call { callee, .. } if matches!(callee.as_ref(), Expression::Identifier(init_name) if init_name.starts_with("__ayy_module_init_")) => {
                Some(value.clone())
            }
            _ => None,
        }
    }

    fn static_module_init_call_throw_value(&self, callee: &Expression) -> Option<Expression> {
        let Expression::Identifier(init_name) = callee else {
            return None;
        };
        if !init_name.starts_with("__ayy_module_init_") {
            return None;
        }
        let init_function = self.resolve_registered_function_declaration(init_name)?;
        self.static_dynamic_import_module_throw_value(init_function)
    }

    fn awaited_resolution_should_remain_unresolved(
        &self,
        resolution: &Expression,
        current_function_name: Option<&str>,
    ) -> bool {
        if matches!(
            resolution,
            Expression::Call { callee, .. }
                if matches!(callee.as_ref(), Expression::Identifier(name) if name == "__ayyDynamicImport")
        ) || Self::call_is_promise_like_chain(resolution)
        {
            return true;
        }
        let Expression::Call { callee, .. } = resolution else {
            return false;
        };
        let Some(LocalFunctionBinding::User(function_name)) = self
            .resolve_function_binding_from_expression_with_context(callee, current_function_name)
        else {
            return false;
        };
        self.user_function(&function_name)
            .is_some_and(|function| function.is_async())
    }

    fn static_promise_handler_expression<'b>(
        &self,
        argument: Option<&'b CallArgument>,
    ) -> Option<&'b Expression> {
        match argument? {
            CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                Some(expression)
            }
        }
    }

    fn resolve_static_immediate_promise_handler_outcome(
        &self,
        handler: &Expression,
        argument: &Expression,
        current_function_name: Option<&str>,
    ) -> Option<StaticEvalOutcome> {
        let binding = self.resolve_function_binding_from_expression_with_context(
            handler,
            current_function_name,
        )?;
        let LocalFunctionBinding::User(_) = binding else {
            return Some(StaticEvalOutcome::Value(argument.clone()));
        };
        let returned = self.resolve_function_binding_static_return_expression_with_call_frame(
            &binding,
            &[argument.clone()],
            &Expression::Undefined,
        )?;
        let returned = self.materialize_static_expression(&returned);
        self.resolve_static_immediate_promise_chain_outcome(&returned, current_function_name)
            .or_else(|| self.resolve_static_await_resolution_outcome(&returned))
            .or(Some(StaticEvalOutcome::Value(returned)))
    }

    fn resolve_static_immediate_promise_chain_outcome(
        &self,
        expression: &Expression,
        current_function_name: Option<&str>,
    ) -> Option<StaticEvalOutcome> {
        let Expression::Call { callee, arguments } = expression else {
            return None;
        };
        if let Expression::Member { object, property } = callee.as_ref()
            && matches!(object.as_ref(), Expression::Identifier(name) if name == "Promise")
            && let Expression::String(property_name) = property.as_ref()
        {
            let settled_argument = arguments
                .first()
                .map(|argument| self.materialize_static_expression(argument.expression()));
            return match property_name.as_str() {
                "resolve" => Some(match settled_argument {
                    Some(argument) => self
                        .resolve_static_immediate_promise_chain_outcome(
                            &argument,
                            current_function_name,
                        )
                        .or_else(|| self.resolve_static_await_resolution_outcome(&argument))
                        .unwrap_or(StaticEvalOutcome::Value(argument)),
                    None => StaticEvalOutcome::Value(Expression::Undefined),
                }),
                "reject" => Some(StaticEvalOutcome::Throw(StaticThrowValue::Value(
                    settled_argument.unwrap_or(Expression::Undefined),
                ))),
                _ => None,
            };
        }

        let Expression::Member { object, property } = callee.as_ref() else {
            return None;
        };
        let Expression::String(property_name) = property.as_ref() else {
            return None;
        };
        if property_name != "then" && property_name != "catch" {
            return None;
        }

        let base_outcome =
            self.resolve_static_immediate_promise_chain_outcome(object, current_function_name)?;
        let (handler, passthrough) = match (property_name.as_str(), base_outcome) {
            ("then", StaticEvalOutcome::Value(value)) => (
                self.static_promise_handler_expression(arguments.first()),
                StaticEvalOutcome::Value(value),
            ),
            ("then", StaticEvalOutcome::Throw(throw_value)) => (
                self.static_promise_handler_expression(arguments.get(1)),
                StaticEvalOutcome::Throw(throw_value),
            ),
            ("catch", StaticEvalOutcome::Value(value)) => (None, StaticEvalOutcome::Value(value)),
            ("catch", StaticEvalOutcome::Throw(throw_value)) => (
                self.static_promise_handler_expression(arguments.first()),
                StaticEvalOutcome::Throw(throw_value),
            ),
            _ => unreachable!("promise chain property filtered above"),
        };
        let Some(handler) = handler else {
            return Some(passthrough);
        };
        let argument = match &passthrough {
            StaticEvalOutcome::Value(value) => value.clone(),
            StaticEvalOutcome::Throw(throw_value) => {
                self.resolve_static_throw_value_expression(throw_value)?
            }
        };
        self.resolve_static_immediate_promise_handler_outcome(
            handler,
            &argument,
            current_function_name,
        )
    }

    pub(in crate::backend::direct_wasm) fn resolve_static_await_resolution_outcome(
        &self,
        resolution: &Expression,
    ) -> Option<StaticEvalOutcome> {
        let current_function_name = self.current_function_name();
        if let Some(outcome) = self.static_module_dependency_promise_outcome(resolution) {
            return Some(outcome);
        }
        if let Expression::Identifier(name) = resolution
            && let Some(init_call) = self.static_module_promise_init_call_expression(name)
        {
            return self.resolve_static_await_resolution_outcome(&init_call);
        }
        if let Expression::Await(value) = resolution {
            let materialized = self.materialize_static_expression(value);
            if let Some(outcome) = self.resolve_static_await_resolution_outcome(&materialized) {
                return Some(outcome);
            }
            if self.awaited_resolution_should_remain_unresolved(value, current_function_name)
                || self.awaited_resolution_should_remain_unresolved(
                    &materialized,
                    current_function_name,
                )
            {
                return None;
            }
            return Some(StaticEvalOutcome::Value(materialized));
        }
        if let Expression::New { callee, arguments } = resolution
            && matches!(callee.as_ref(), Expression::Identifier(name) if name == "Promise")
            && let Some(outcome) =
                self.resolve_static_promise_constructor_outcome(arguments, current_function_name)
        {
            return Some(outcome);
        }
        if let Some(outcome) =
            self.resolve_static_immediate_promise_chain_outcome(resolution, current_function_name)
        {
            return Some(outcome);
        }
        if let Expression::Call { callee, arguments } = resolution {
            if let Some(throw_value) = self.static_module_init_call_throw_value(callee) {
                return Some(StaticEvalOutcome::Throw(StaticThrowValue::Value(
                    throw_value,
                )));
            }
            if let Some(outcome) = self.resolve_static_dynamic_import_outcome(callee, arguments) {
                return Some(outcome);
            }
            if let Some(binding) = self.resolve_function_binding_from_expression_with_context(
                callee,
                current_function_name,
            ) {
                match &binding {
                    LocalFunctionBinding::Builtin(name) if name == "Promise.resolve" => {
                        let settled_argument = arguments.first().map(|argument| match argument {
                            CallArgument::Expression(expression)
                            | CallArgument::Spread(expression) => {
                                self.materialize_static_expression(expression)
                            }
                        });
                        return Some(match settled_argument {
                            Some(argument) => self
                                .resolve_static_await_resolution_outcome(&argument)
                                .unwrap_or(StaticEvalOutcome::Value(argument)),
                            None => StaticEvalOutcome::Value(Expression::Undefined),
                        });
                    }
                    LocalFunctionBinding::Builtin(name) if name == "Promise.reject" => {
                        let settled_argument = arguments.first().map(|argument| match argument {
                            CallArgument::Expression(expression)
                            | CallArgument::Spread(expression) => {
                                self.materialize_static_expression(expression)
                            }
                        });
                        return Some(StaticEvalOutcome::Throw(StaticThrowValue::Value(
                            settled_argument.unwrap_or(Expression::Undefined),
                        )));
                    }
                    LocalFunctionBinding::User(_) => {
                        let call_arguments = self.expand_call_arguments(arguments);
                        let this_binding = match callee.as_ref() {
                            Expression::Member { object, .. } => {
                                self.materialize_static_expression(object)
                            }
                            Expression::SuperMember { .. } => Expression::This,
                            _ => Expression::Undefined,
                        };
                        if let Some(value) = self
                            .resolve_function_binding_static_return_expression_with_call_frame(
                                &binding,
                                &call_arguments,
                                &this_binding,
                            )
                        {
                            return self
                                .resolve_static_await_resolution_outcome(&value)
                                .or(Some(StaticEvalOutcome::Value(value)));
                        }
                    }
                    _ => {}
                }
                if let Some(outcome) = self
                    .resolve_static_function_outcome_from_binding_with_context(
                        &binding,
                        arguments,
                        current_function_name,
                    )
                {
                    return Some(match outcome {
                        StaticEvalOutcome::Value(value) => self
                            .resolve_static_await_resolution_outcome(&value)
                            .unwrap_or(StaticEvalOutcome::Value(value)),
                        StaticEvalOutcome::Throw(throw_value) => {
                            StaticEvalOutcome::Throw(throw_value)
                        }
                    });
                }
            }
            if let Expression::Member { object, property } = callee.as_ref()
                && matches!(object.as_ref(), Expression::Identifier(name) if name == "Promise")
                && let Expression::String(property_name) = property.as_ref()
            {
                let settled_argument = arguments.first().map(|argument| match argument {
                    CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                        self.materialize_static_expression(expression)
                    }
                });
                match property_name.as_str() {
                    "resolve" => {
                        return Some(match settled_argument {
                            Some(argument) => self
                                .resolve_static_await_resolution_outcome(&argument)
                                .unwrap_or(StaticEvalOutcome::Value(argument)),
                            None => StaticEvalOutcome::Value(Expression::Undefined),
                        });
                    }
                    "reject" => {
                        return Some(StaticEvalOutcome::Throw(StaticThrowValue::Value(
                            settled_argument.unwrap_or(Expression::Undefined),
                        )));
                    }
                    _ => {}
                }
            }
            if let Some(result) = self.resolve_static_call_result_expression(callee, arguments) {
                return self
                    .resolve_static_await_resolution_outcome(&result)
                    .or(Some(StaticEvalOutcome::Value(result)));
            }
        }
        let materialized = self.materialize_static_expression(resolution);
        if !static_expression_matches(&materialized, resolution) {
            return self
                .resolve_static_await_resolution_outcome(&materialized)
                .map(|outcome| match outcome {
                    StaticEvalOutcome::Value(value)
                        if static_expression_matches(&value, &materialized) =>
                    {
                        StaticEvalOutcome::Value(resolution.clone())
                    }
                    other => other,
                });
        }
        if let Expression::Call { callee, arguments } = &materialized
            && let Expression::Member { object, property } = callee.as_ref()
            && matches!(object.as_ref(), Expression::Identifier(name) if name == "Promise")
            && let Expression::String(property_name) = property.as_ref()
        {
            let settled_argument = arguments.first().map(|argument| match argument {
                CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                    self.materialize_static_expression(expression)
                }
            });
            match property_name.as_str() {
                "resolve" => {
                    return Some(match settled_argument {
                        Some(argument) => self
                            .resolve_static_await_resolution_outcome(&argument)
                            .unwrap_or(StaticEvalOutcome::Value(argument)),
                        None => StaticEvalOutcome::Value(Expression::Undefined),
                    });
                }
                "reject" => {
                    return Some(StaticEvalOutcome::Throw(StaticThrowValue::Value(
                        settled_argument.unwrap_or(Expression::Undefined),
                    )));
                }
                _ => {}
            }
        }
        if self
            .resolve_static_primitive_expression_with_context(&materialized, current_function_name)
            .is_some()
        {
            return Some(StaticEvalOutcome::Value(materialized));
        }
        if !self.static_expression_is_object_like(&materialized) {
            return Some(StaticEvalOutcome::Value(materialized));
        }
        if self.expression_is_static_regexp_instance(&materialized) {
            return Some(StaticEvalOutcome::Value(materialized));
        }

        let then_property = Expression::String("then".to_string());
        let mut snapshot_bindings = HashMap::new();
        let then_outcome = match &materialized {
            Expression::Object(entries) => self
                .resolve_bound_snapshot_object_member_outcome(
                    entries,
                    &then_property,
                    &mut snapshot_bindings,
                    current_function_name,
                )
                .or_else(|| {
                    self.resolve_static_property_get_outcome(&materialized, &then_property)
                })?,
            _ => self.resolve_static_property_get_outcome(&materialized, &then_property)?,
        };
        match then_outcome {
            StaticEvalOutcome::Throw(throw_value) => Some(StaticEvalOutcome::Throw(throw_value)),
            StaticEvalOutcome::Value(then_value) => {
                if matches!(then_value, Expression::Undefined | Expression::Null) {
                    return Some(StaticEvalOutcome::Value(materialized));
                }
                let Some(binding) = self.resolve_function_binding_from_expression_with_context(
                    &then_value,
                    current_function_name,
                ) else {
                    return Some(StaticEvalOutcome::Value(materialized));
                };
                if let Some(outcome) = self.resolve_bound_snapshot_thenable_outcome(
                    &binding,
                    &materialized,
                    &mut snapshot_bindings,
                    current_function_name,
                ) {
                    return Some(outcome);
                }
                match self.resolve_static_function_outcome_from_binding_with_context(
                    &binding,
                    &[],
                    current_function_name,
                )? {
                    StaticEvalOutcome::Throw(throw_value) => {
                        Some(StaticEvalOutcome::Throw(throw_value))
                    }
                    StaticEvalOutcome::Value(_) => None,
                }
            }
        }
    }

    pub(in crate::backend::direct_wasm) fn resolve_static_dynamic_import_outcome(
        &self,
        callee: &Expression,
        arguments: &[CallArgument],
    ) -> Option<StaticEvalOutcome> {
        if !matches!(callee, Expression::Identifier(name) if name == "__ayyDynamicImport") {
            return None;
        }
        if let Some(outcome) = self.static_dynamic_import_options_rejection(arguments) {
            return Some(outcome);
        }
        let module_index = self.dynamic_import_literal_module_index(arguments)?;
        let init_name = format!("__ayy_module_init_{module_index}");
        let init_function = self.resolve_registered_function_declaration(&init_name)?;
        if let Some(throw_value) = self.static_dynamic_import_module_throw_value(init_function) {
            return Some(StaticEvalOutcome::Throw(StaticThrowValue::Value(
                throw_value,
            )));
        }
        let import_type = self.static_dynamic_import_attribute_type(arguments);
        if Self::static_dynamic_import_is_defer_phase(arguments) {
            return Some(StaticEvalOutcome::Value(Expression::Identifier(format!(
                "__ayy_module_deferred_namespace_{module_index}"
            ))));
        }
        Some(StaticEvalOutcome::Value(
            self.static_dynamic_import_module_namespace_value(
                module_index,
                init_function,
                import_type.as_deref(),
                &mut HashSet::new(),
            ),
        ))
    }

    fn dynamic_import_literal_module_index(&self, arguments: &[CallArgument]) -> Option<usize> {
        let argument_expression = match arguments.first()? {
            CallArgument::Expression(expression) => expression,
            CallArgument::Spread(_) => return None,
        };
        let argument = self.materialize_static_expression(argument_expression);
        if let Some(module_index) = Self::static_dynamic_import_numeric_module_index(&argument) {
            return Some(module_index);
        }
        let specifier = self
            .static_dynamic_import_specifier_string(argument_expression)
            .or_else(|| self.static_dynamic_import_specifier_string(&argument))?;
        self.dynamic_import_module_index_from_specifier_table(arguments, &specifier)
    }

    fn static_dynamic_import_is_defer_phase(arguments: &[CallArgument]) -> bool {
        matches!(
            arguments.get(3),
            Some(CallArgument::Expression(Expression::String(phase))) if phase == DYNAMIC_IMPORT_DEFER_PHASE
        )
    }

    fn static_dynamic_import_numeric_module_index(argument: &Expression) -> Option<usize> {
        let Expression::Number(module_index) = argument else {
            return None;
        };
        if !module_index.is_finite() || *module_index < 0.0 || module_index.fract() != 0.0 {
            return None;
        }
        Some(*module_index as usize)
    }

    fn dynamic_import_module_index_from_specifier_table(
        &self,
        arguments: &[CallArgument],
        specifier: &str,
    ) -> Option<usize> {
        let table = match arguments.get(2)? {
            CallArgument::Expression(expression) => self.materialize_static_expression(expression),
            CallArgument::Spread(_) => return None,
        };
        let Expression::Object(entries) = table else {
            return None;
        };
        entries.iter().find_map(|entry| {
            let ObjectEntry::Data { key, value } = entry else {
                return None;
            };
            let key_text = static_property_name_from_expression(key)
                .or_else(|| self.resolve_static_string_value(key))?;
            if key_text != specifier {
                return None;
            }
            let value = self.materialize_static_expression(value);
            Self::static_dynamic_import_numeric_module_index(&value)
        })
    }

    fn static_dynamic_import_specifier_string(&self, expression: &Expression) -> Option<String> {
        let current_function_name = self.current_function_name();
        if let Some(primitive) =
            self.resolve_static_primitive_expression_with_context(expression, current_function_name)
        {
            return self.resolve_static_string_concat_value(&primitive, current_function_name);
        }
        let materialized = self.materialize_static_expression(expression);
        if !static_expression_matches(&materialized, expression)
            && let Some(primitive) = self.resolve_static_primitive_expression_with_context(
                &materialized,
                current_function_name,
            )
        {
            return self.resolve_static_string_concat_value(&primitive, current_function_name);
        }

        let resolved = self
            .resolve_bound_alias_expression(expression)
            .unwrap_or_else(|| materialized.clone());
        let coercion_target = if matches!(expression, Expression::Identifier(_)) {
            expression
        } else {
            &resolved
        };
        for method_name in ["toString", "valueOf"] {
            let outcome = self.resolve_static_member_call_outcome_with_context(
                coercion_target,
                method_name,
                current_function_name,
            );
            match outcome {
                Some(StaticEvalOutcome::Value(value)) => {
                    if let Some(primitive) = self.resolve_static_primitive_expression_with_context(
                        &value,
                        current_function_name,
                    ) {
                        return self
                            .resolve_static_string_concat_value(&primitive, current_function_name);
                    }
                }
                Some(StaticEvalOutcome::Throw(_)) => return None,
                None => {}
            }
        }
        None
    }

    pub(in crate::backend::direct_wasm) fn emit_static_dynamic_import_module_init_effects(
        &mut self,
        arguments: &[CallArgument],
    ) -> DirectResult<()> {
        let Some(module_index) = self.dynamic_import_literal_module_index(arguments) else {
            return Ok(());
        };
        let init_name = format!("__ayy_module_init_{module_index}");
        if self.current_function_name() == Some(init_name.as_str()) {
            return Ok(());
        }
        let Some(init_declaration) = self.resolve_registered_function_declaration(&init_name)
        else {
            return Ok(());
        };
        if init_declaration.params.len() != 1 {
            return Ok(());
        }
        if self
            .static_dynamic_import_module_throw_value(init_declaration)
            .is_some()
        {
            return Ok(());
        }
        let live_initializers =
            self.static_dynamic_import_live_binding_initializers(init_declaration);
        for (hidden_name, initial_value) in live_initializers {
            let binding = self.ensure_implicit_global_binding(&hidden_name);
            self.push_global_get(binding.present_index);
            self.state.emission.output.instructions.push(0x45);
            self.state.emission.output.instructions.push(0x04);
            self.state
                .emission
                .output
                .instructions
                .push(EMPTY_BLOCK_TYPE);
            self.push_control_frame();
            self.emit_numeric_expression(&initial_value)?;
            self.push_global_set(binding.value_index);
            self.push_i32_const(1);
            self.push_global_set(binding.present_index);
            self.state.emission.output.instructions.push(0x0b);
            self.pop_control_frame();
            self.update_static_global_assignment_metadata(&hidden_name, &initial_value);
            self.update_global_specialized_function_value(&hidden_name, &initial_value)?;
        }
        let Some(init_function) = self.user_function(&init_name).cloned() else {
            return Ok(());
        };
        let arguments = vec![CallArgument::Expression(Expression::Identifier(format!(
            "__ayy_module_namespace_{module_index}"
        )))];
        self.emit_user_function_call(&init_function, &arguments)?;
        self.state.emission.output.instructions.push(0x1a);
        Ok(())
    }

    pub(in crate::backend::direct_wasm) fn static_dynamic_import_live_binding_initializers(
        &self,
        init_function: &FunctionDeclaration,
    ) -> Vec<(String, Expression)> {
        let local_bindings = self.static_dynamic_import_module_local_bindings(init_function);
        let mut initializers = Vec::new();
        let mut seen = HashSet::new();

        for statement in &init_function.body {
            let Some((_, getter_name, _, _)) = self.static_dynamic_import_export_getter(statement)
            else {
                continue;
            };
            let Some((hidden_name, initial_value)) = self
                .static_dynamic_import_getter_live_binding_initializer(
                    &getter_name,
                    &local_bindings,
                )
            else {
                continue;
            };
            if seen.insert(hidden_name.clone()) {
                initializers.push((hidden_name, initial_value));
            }
        }

        initializers
    }

    fn static_dynamic_import_options_rejection(
        &self,
        arguments: &[CallArgument],
    ) -> Option<StaticEvalOutcome> {
        let options_expression = match arguments.get(1)? {
            CallArgument::Expression(expression) => expression,
            CallArgument::Spread(_) => return None,
        };
        let materialized_options = self.materialize_static_expression(options_expression);
        let options = self
            .resolve_static_primitive_expression_with_context(
                &materialized_options,
                self.current_function_name(),
            )
            .unwrap_or(materialized_options);
        if matches!(options, Expression::Undefined) {
            return None;
        }
        if matches!(options, Expression::Null)
            || self
                .resolve_static_primitive_expression_with_context(
                    &options,
                    self.current_function_name(),
                )
                .is_some()
        {
            return Some(StaticEvalOutcome::Throw(StaticThrowValue::NamedError(
                "TypeError",
            )));
        }
        if !self.static_expression_is_object_like(&options)
            && !matches!(options, Expression::Object(_))
        {
            return None;
        }

        let with_property = Expression::String("with".to_string());
        let with_outcome = match &options {
            Expression::Object(entries) => {
                let mut snapshot_bindings = HashMap::new();
                self.resolve_bound_snapshot_object_member_outcome(
                    entries,
                    &with_property,
                    &mut snapshot_bindings,
                    self.current_function_name(),
                )
                .or_else(|| self.resolve_static_property_get_outcome(&options, &with_property))
            }
            _ => self.resolve_static_property_get_outcome(&options, &with_property),
        }?;
        match with_outcome {
            StaticEvalOutcome::Throw(throw_value) => Some(StaticEvalOutcome::Throw(throw_value)),
            StaticEvalOutcome::Value(with_value) => {
                let with_value = self
                    .resolve_static_primitive_expression_with_context(
                        &with_value,
                        self.current_function_name(),
                    )
                    .unwrap_or(with_value);
                if matches!(with_value, Expression::Undefined) {
                    return None;
                }
                if matches!(with_value, Expression::Null)
                    || self
                        .resolve_static_primitive_expression_with_context(
                            &with_value,
                            self.current_function_name(),
                        )
                        .is_some()
                {
                    return Some(StaticEvalOutcome::Throw(StaticThrowValue::NamedError(
                        "TypeError",
                    )));
                }
                if let Some(outcome) =
                    self.static_dynamic_import_attributes_enumeration_rejection(&with_value)
                {
                    return Some(outcome);
                }
                if let Some(outcome) =
                    self.static_dynamic_import_attributes_value_rejection(&with_value)
                {
                    return Some(outcome);
                }
                None
            }
        }
    }

    fn static_dynamic_import_attribute_type(&self, arguments: &[CallArgument]) -> Option<String> {
        let value = self.static_dynamic_import_attribute_value(arguments, "type")?;
        let value = self
            .resolve_static_primitive_expression_with_context(&value, self.current_function_name())
            .unwrap_or(value);
        match value {
            Expression::String(value) => Some(value),
            _ => None,
        }
    }

    fn static_dynamic_import_attribute_value(
        &self,
        arguments: &[CallArgument],
        name: &str,
    ) -> Option<Expression> {
        let attributes = self.static_dynamic_import_attributes_value(arguments)?;
        let property = Expression::String(name.to_string());
        let outcome = match &attributes {
            Expression::Object(entries) => {
                let mut snapshot_bindings = HashMap::new();
                self.resolve_bound_snapshot_object_member_outcome(
                    entries,
                    &property,
                    &mut snapshot_bindings,
                    self.current_function_name(),
                )
                .or_else(|| self.resolve_static_property_get_outcome(&attributes, &property))
            }
            _ => self.resolve_static_property_get_outcome(&attributes, &property),
        }?;
        match outcome {
            StaticEvalOutcome::Value(value) => Some(value),
            StaticEvalOutcome::Throw(_) => None,
        }
    }

    fn static_dynamic_import_attributes_value(
        &self,
        arguments: &[CallArgument],
    ) -> Option<Expression> {
        let options_expression = match arguments.get(1)? {
            CallArgument::Expression(expression) => expression,
            CallArgument::Spread(_) => return None,
        };
        let materialized_options = self.materialize_static_expression(options_expression);
        let options = self
            .resolve_static_primitive_expression_with_context(
                &materialized_options,
                self.current_function_name(),
            )
            .unwrap_or(materialized_options);
        if matches!(options, Expression::Undefined | Expression::Null)
            || self
                .resolve_static_primitive_expression_with_context(
                    &options,
                    self.current_function_name(),
                )
                .is_some()
        {
            return None;
        }

        let with_property = Expression::String("with".to_string());
        let with_outcome = match &options {
            Expression::Object(entries) => {
                let mut snapshot_bindings = HashMap::new();
                self.resolve_bound_snapshot_object_member_outcome(
                    entries,
                    &with_property,
                    &mut snapshot_bindings,
                    self.current_function_name(),
                )
                .or_else(|| self.resolve_static_property_get_outcome(&options, &with_property))
            }
            _ => self.resolve_static_property_get_outcome(&options, &with_property),
        }?;
        match with_outcome {
            StaticEvalOutcome::Value(value) => Some(
                self.resolve_static_primitive_expression_with_context(
                    &value,
                    self.current_function_name(),
                )
                .unwrap_or(value),
            ),
            StaticEvalOutcome::Throw(_) => None,
        }
    }

    fn static_dynamic_import_attributes_enumeration_rejection(
        &self,
        attributes: &Expression,
    ) -> Option<StaticEvalOutcome> {
        let proxy_binding = self.resolve_proxy_binding_from_expression(attributes)?;
        let outcome = self.static_dynamic_import_proxy_own_keys_outcome(&proxy_binding)?;
        match outcome {
            StaticEvalOutcome::Throw(throw_value) => Some(StaticEvalOutcome::Throw(throw_value)),
            StaticEvalOutcome::Value(_) => None,
        }
    }

    fn static_dynamic_import_attributes_value_rejection(
        &self,
        attributes: &Expression,
    ) -> Option<StaticEvalOutcome> {
        let Expression::Object(entries) = attributes else {
            return None;
        };
        let keys = self.static_dynamic_import_object_attribute_keys(entries)?;
        let mut snapshot_bindings = HashMap::new();
        for key in keys {
            let outcome = self
                .resolve_bound_snapshot_object_member_outcome(
                    entries,
                    &key,
                    &mut snapshot_bindings,
                    self.current_function_name(),
                )
                .or_else(|| self.resolve_static_property_get_outcome(attributes, &key))?;
            let value = match outcome {
                StaticEvalOutcome::Throw(throw_value) => {
                    return Some(StaticEvalOutcome::Throw(throw_value));
                }
                StaticEvalOutcome::Value(value) => self
                    .resolve_static_primitive_expression_with_context(
                        &value,
                        self.current_function_name(),
                    )
                    .unwrap_or(value),
            };
            if !matches!(value, Expression::String(_)) {
                return Some(StaticEvalOutcome::Throw(StaticThrowValue::NamedError(
                    "TypeError",
                )));
            }
        }
        None
    }

    fn static_dynamic_import_object_attribute_keys(
        &self,
        entries: &[ObjectEntry],
    ) -> Option<Vec<Expression>> {
        let mut keys = Vec::new();
        for entry in entries {
            match entry {
                ObjectEntry::Data { key, .. }
                | ObjectEntry::Getter { key, .. }
                | ObjectEntry::Setter { key, .. } => {
                    let key = self
                        .resolve_property_key_expression(key)
                        .unwrap_or_else(|| self.materialize_static_expression(key));
                    if matches!(key, Expression::String(_)) {
                        keys.push(key);
                    }
                }
                ObjectEntry::Spread(expression) => {
                    let spread = self.materialize_static_expression(expression);
                    let Expression::Object(spread_entries) = spread else {
                        return None;
                    };
                    keys.extend(self.static_dynamic_import_object_attribute_keys(&spread_entries)?);
                }
            }
        }
        Some(keys)
    }

    fn static_dynamic_import_proxy_own_keys_outcome(
        &self,
        proxy_binding: &ProxyValueBinding,
    ) -> Option<StaticEvalOutcome> {
        let own_keys_binding = proxy_binding.own_keys_binding.as_ref()?;
        let arguments = [CallArgument::Expression(proxy_binding.target.clone())];
        self.resolve_static_function_outcome_from_binding_with_call_frame_and_context(
            own_keys_binding,
            &arguments,
            &proxy_binding.handler,
            self.current_function_name(),
        )
    }

    pub(in crate::backend::direct_wasm) fn emit_static_dynamic_import_options_effects(
        &mut self,
        arguments: &[CallArgument],
    ) -> DirectResult<()> {
        let options_expression = match arguments.get(1) {
            Some(CallArgument::Expression(expression)) => expression,
            _ => return Ok(()),
        };
        let materialized_options = self.materialize_static_expression(options_expression);
        let options = self
            .resolve_static_primitive_expression_with_context(
                &materialized_options,
                self.current_function_name(),
            )
            .unwrap_or(materialized_options);
        if matches!(options, Expression::Undefined | Expression::Null)
            || self
                .resolve_static_primitive_expression_with_context(
                    &options,
                    self.current_function_name(),
                )
                .is_some()
        {
            return Ok(());
        }

        let with_property = Expression::String("with".to_string());
        let with_outcome = match &options {
            Expression::Object(entries) => {
                let mut snapshot_bindings = HashMap::new();
                self.resolve_bound_snapshot_object_member_outcome(
                    entries,
                    &with_property,
                    &mut snapshot_bindings,
                    self.current_function_name(),
                )
                .or_else(|| self.resolve_static_property_get_outcome(&options, &with_property))
            }
            _ => self.resolve_static_property_get_outcome(&options, &with_property),
        };
        let Some(StaticEvalOutcome::Value(with_value)) = with_outcome else {
            return Ok(());
        };
        let with_value = self
            .resolve_static_primitive_expression_with_context(
                &with_value,
                self.current_function_name(),
            )
            .unwrap_or(with_value);
        let Some(proxy_binding) = self.resolve_proxy_binding_from_expression(&with_value) else {
            return Ok(());
        };
        let Some(StaticEvalOutcome::Value(keys_value)) =
            self.static_dynamic_import_proxy_own_keys_outcome(&proxy_binding)
        else {
            return Ok(());
        };
        let Expression::Array(keys) = self.materialize_static_expression(&keys_value) else {
            return Ok(());
        };
        let Some(LocalFunctionBinding::User(get_function_name)) = proxy_binding.get_binding.clone()
        else {
            return Ok(());
        };
        let Some(get_function) = self.user_function(&get_function_name).cloned() else {
            return Ok(());
        };
        for key in keys {
            let ArrayElement::Expression(key) = key else {
                continue;
            };
            let key = self.materialize_static_expression(&key);
            if !matches!(key, Expression::String(_)) {
                continue;
            }
            if !self.static_dynamic_import_attribute_key_is_enumerable(&proxy_binding, &key) {
                continue;
            }
            let get_arguments = [
                CallArgument::Expression(proxy_binding.target.clone()),
                CallArgument::Expression(key),
                CallArgument::Expression(with_value.clone()),
            ];
            self.emit_user_function_call_with_new_target_and_this_expression(
                &get_function,
                &get_arguments,
                JS_UNDEFINED_TAG,
                &proxy_binding.handler,
            )?;
            self.state.emission.output.instructions.push(0x1a);
        }
        Ok(())
    }

    fn static_dynamic_import_attribute_key_is_enumerable(
        &self,
        proxy_binding: &ProxyValueBinding,
        key: &Expression,
    ) -> bool {
        let Some(descriptor_binding) = proxy_binding.get_own_property_descriptor_binding.as_ref()
        else {
            return false;
        };
        let descriptor_arguments = [
            CallArgument::Expression(proxy_binding.target.clone()),
            CallArgument::Expression(key.clone()),
        ];
        let Some(StaticEvalOutcome::Value(descriptor)) = self
            .resolve_static_function_outcome_from_binding_with_call_frame_and_context(
                descriptor_binding,
                &descriptor_arguments,
                &proxy_binding.handler,
                self.current_function_name(),
            )
        else {
            return false;
        };
        if matches!(descriptor, Expression::Undefined | Expression::Null) {
            return false;
        }
        let enumerable_property = Expression::String("enumerable".to_string());
        let enumerable_outcome = match &descriptor {
            Expression::Object(entries) => {
                let mut snapshot_bindings = HashMap::new();
                self.resolve_bound_snapshot_object_member_outcome(
                    entries,
                    &enumerable_property,
                    &mut snapshot_bindings,
                    self.current_function_name(),
                )
                .or_else(|| {
                    self.resolve_static_property_get_outcome(&descriptor, &enumerable_property)
                })
            }
            _ => self.resolve_static_property_get_outcome(&descriptor, &enumerable_property),
        };
        let Some(StaticEvalOutcome::Value(enumerable)) = enumerable_outcome else {
            return false;
        };
        self.resolve_static_boolean_expression(&enumerable)
            .unwrap_or(false)
    }

    fn static_dynamic_import_module_throw_value(
        &self,
        init_function: &FunctionDeclaration,
    ) -> Option<Expression> {
        if let Some(cached) = STATIC_DYNAMIC_IMPORT_MODULE_THROW_CACHE
            .with(|cache| cache.borrow().get(&init_function.name).cloned())
        {
            return cached;
        }
        let trace = std::env::var_os("AYY_TRACE_DYNAMIC_IMPORT_AWAIT").is_some();
        let mut local_bindings = self.static_dynamic_import_module_local_bindings(init_function);
        if trace {
            eprintln!(
                "dynamic_import_await:scan_module function={} locals={:?}",
                init_function.name, local_bindings
            );
        }
        for statement in &init_function.body {
            if trace {
                eprintln!("dynamic_import_await:scan_statement {statement:?}");
            }
            if let Some(throw_value) =
                self.static_dynamic_import_statement_throw_value(statement, &mut local_bindings)
            {
                if trace {
                    eprintln!("dynamic_import_await:found_throw {throw_value:?}");
                }
                STATIC_DYNAMIC_IMPORT_MODULE_THROW_CACHE.with(|cache| {
                    cache
                        .borrow_mut()
                        .insert(init_function.name.clone(), Some(throw_value.clone()));
                });
                return Some(throw_value);
            }
        }
        if trace {
            eprintln!(
                "dynamic_import_await:no_throw function={}",
                init_function.name
            );
        }
        STATIC_DYNAMIC_IMPORT_MODULE_THROW_CACHE.with(|cache| {
            cache.borrow_mut().insert(init_function.name.clone(), None);
        });
        None
    }

    pub(in crate::backend::direct_wasm) fn static_dynamic_import_module_throw_value_by_index(
        &self,
        module_index: usize,
    ) -> Option<Expression> {
        let init_name = format!("__ayy_module_init_{module_index}");
        let init_function = self.resolve_registered_function_declaration(&init_name)?;
        self.static_dynamic_import_module_throw_value(init_function)
    }

    fn static_dynamic_import_statement_throw_value(
        &self,
        statement: &Statement,
        local_bindings: &mut HashMap<String, Expression>,
    ) -> Option<Expression> {
        match statement {
            Statement::Throw(expression) => Some(self.materialize_static_expression(expression)),
            Statement::Yield { value } | Statement::YieldDelegate { value } => {
                self.static_dynamic_import_await_throw_value(value, local_bindings)
            }
            Statement::Expression(Expression::Await(value)) => {
                self.static_dynamic_import_await_throw_value(value, local_bindings)
            }
            Statement::Var {
                value: Expression::Await(value),
                ..
            }
            | Statement::Let {
                value: Expression::Await(value),
                ..
            }
            | Statement::Assign {
                value: Expression::Await(value),
                ..
            }
            | Statement::Return(Expression::Await(value)) => {
                self.static_dynamic_import_await_throw_value(value, local_bindings)
            }
            Statement::Var { name, value }
            | Statement::Let { name, value, .. }
            | Statement::Assign { name, value } => {
                let value = Self::static_dynamic_import_localized_expression(value, local_bindings);
                local_bindings.insert(name.clone(), self.materialize_static_expression(&value));
                None
            }
            Statement::Block { body } | Statement::Declaration { body } => {
                self.static_dynamic_import_statements_throw_value(body, local_bindings)
            }
            Statement::Labeled { body, .. } => {
                self.static_dynamic_import_statements_throw_value(body, local_bindings)
            }
            Statement::If {
                condition,
                then_branch,
                else_branch,
            } => {
                let condition =
                    Self::static_dynamic_import_localized_expression(condition, local_bindings);
                if let Some(taken) = self.resolve_static_boolean_expression(&condition) {
                    let branch = if taken { then_branch } else { else_branch };
                    return self
                        .static_dynamic_import_statements_throw_value(branch, local_bindings);
                }

                let mut then_bindings = local_bindings.clone();
                if let Some(throw_value) = self
                    .static_dynamic_import_statements_throw_value(then_branch, &mut then_bindings)
                {
                    return Some(throw_value);
                }
                let mut else_bindings = local_bindings.clone();
                self.static_dynamic_import_statements_throw_value(else_branch, &mut else_bindings)
            }
            Statement::Try {
                body,
                catch_binding,
                catch_setup,
                catch_body,
            } => {
                let assert_throws_caught_marker = if catch_binding.is_none()
                    && catch_setup.is_empty()
                    && let [
                        Statement::Assign {
                            name,
                            value: Expression::Bool(true),
                        },
                    ] = catch_body.as_slice()
                    && name.starts_with("__ayy_assert_throws_caught_")
                {
                    Some(name.clone())
                } else {
                    None
                };
                let mut body_bindings = local_bindings.clone();
                let Some(throw_value) =
                    self.static_dynamic_import_statements_throw_value(body, &mut body_bindings)
                else {
                    if let Some(marker) = assert_throws_caught_marker {
                        body_bindings.insert(marker, Expression::Bool(true));
                    }
                    *local_bindings = body_bindings;
                    return None;
                };

                let mut catch_bindings = local_bindings.clone();
                if let Some(catch_binding) = catch_binding {
                    catch_bindings.insert(catch_binding.clone(), throw_value);
                }
                if let Some(catch_throw_value) = self
                    .static_dynamic_import_statements_throw_value(catch_setup, &mut catch_bindings)
                {
                    return Some(catch_throw_value);
                }
                if let Some(catch_throw_value) = self
                    .static_dynamic_import_statements_throw_value(catch_body, &mut catch_bindings)
                {
                    return Some(catch_throw_value);
                }
                *local_bindings = catch_bindings;
                None
            }
            _ => None,
        }
    }

    fn static_dynamic_import_statements_throw_value(
        &self,
        statements: &[Statement],
        local_bindings: &mut HashMap<String, Expression>,
    ) -> Option<Expression> {
        for statement in statements {
            if let Some(throw_value) =
                self.static_dynamic_import_statement_throw_value(statement, local_bindings)
            {
                return Some(throw_value);
            }
        }
        None
    }

    fn static_dynamic_import_localized_expression(
        expression: &Expression,
        local_bindings: &HashMap<String, Expression>,
    ) -> Expression {
        match expression {
            Expression::Identifier(name) => {
                Self::static_dynamic_import_local_binding_value(name, local_bindings)
                    .unwrap_or_else(|| expression.clone())
            }
            Expression::Await(value)
            | Expression::EnumerateKeys(value)
            | Expression::GetIterator(value)
            | Expression::IteratorClose(value) => {
                let value = Self::static_dynamic_import_localized_expression(value, local_bindings);
                match expression {
                    Expression::Await(_) => Expression::Await(Box::new(value)),
                    Expression::EnumerateKeys(_) => Expression::EnumerateKeys(Box::new(value)),
                    Expression::GetIterator(_) => Expression::GetIterator(Box::new(value)),
                    Expression::IteratorClose(_) => Expression::IteratorClose(Box::new(value)),
                    _ => unreachable!("filtered above"),
                }
            }
            Expression::Unary { op, expression } => Expression::Unary {
                op: *op,
                expression: Box::new(Self::static_dynamic_import_localized_expression(
                    expression,
                    local_bindings,
                )),
            },
            Expression::Binary { op, left, right } => Expression::Binary {
                op: *op,
                left: Box::new(Self::static_dynamic_import_localized_expression(
                    left,
                    local_bindings,
                )),
                right: Box::new(Self::static_dynamic_import_localized_expression(
                    right,
                    local_bindings,
                )),
            },
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => Expression::Conditional {
                condition: Box::new(Self::static_dynamic_import_localized_expression(
                    condition,
                    local_bindings,
                )),
                then_expression: Box::new(Self::static_dynamic_import_localized_expression(
                    then_expression,
                    local_bindings,
                )),
                else_expression: Box::new(Self::static_dynamic_import_localized_expression(
                    else_expression,
                    local_bindings,
                )),
            },
            Expression::Sequence(expressions) => Expression::Sequence(
                expressions
                    .iter()
                    .map(|expression| {
                        Self::static_dynamic_import_localized_expression(expression, local_bindings)
                    })
                    .collect(),
            ),
            Expression::Member { object, property } => Expression::Member {
                object: Box::new(Self::static_dynamic_import_localized_expression(
                    object,
                    local_bindings,
                )),
                property: Box::new(Self::static_dynamic_import_localized_expression(
                    property,
                    local_bindings,
                )),
            },
            Expression::Call { callee, arguments } => Expression::Call {
                callee: Box::new(Self::static_dynamic_import_localized_expression(
                    callee,
                    local_bindings,
                )),
                arguments: arguments
                    .iter()
                    .map(|argument| match argument {
                        CallArgument::Expression(expression) => CallArgument::Expression(
                            Self::static_dynamic_import_localized_expression(
                                expression,
                                local_bindings,
                            ),
                        ),
                        CallArgument::Spread(expression) => {
                            CallArgument::Spread(Self::static_dynamic_import_localized_expression(
                                expression,
                                local_bindings,
                            ))
                        }
                    })
                    .collect(),
            },
            Expression::Assign { name, value } => Expression::Assign {
                name: name.clone(),
                value: Box::new(Self::static_dynamic_import_localized_expression(
                    value,
                    local_bindings,
                )),
            },
            _ => expression.clone(),
        }
    }

    fn static_dynamic_import_await_throw_value(
        &self,
        value: &Expression,
        local_bindings: &HashMap<String, Expression>,
    ) -> Option<Expression> {
        let trace = std::env::var_os("AYY_TRACE_DYNAMIC_IMPORT_AWAIT").is_some();
        let localized = Self::static_dynamic_import_localized_expression(value, local_bindings);
        if expression_references_module_dependency_param(&localized) {
            if trace {
                eprintln!(
                    "dynamic_import_await:await_unresolved_module_dep value={value:?} localized={localized:?}"
                );
            }
            return None;
        }
        let awaited = match &localized {
            Expression::Identifier(name) => {
                Self::static_dynamic_import_local_binding_value(name, local_bindings)
                    .unwrap_or_else(|| self.materialize_static_expression(&localized))
            }
            _ => self.materialize_static_expression(&localized),
        };
        if trace {
            eprintln!("dynamic_import_await:await value={value:?} awaited={awaited:?}");
        }
        match self.resolve_static_await_resolution_outcome(&awaited)? {
            StaticEvalOutcome::Throw(throw_value) => {
                let resolved = self.resolve_static_throw_value_expression(&throw_value);
                if trace {
                    let throw_label = match &throw_value {
                        StaticThrowValue::Value(_) => "value",
                        StaticThrowValue::NamedError(name) => name,
                    };
                    eprintln!(
                        "dynamic_import_await:await_throw throw={throw_label} resolved={resolved:?}"
                    );
                }
                resolved
            }
            StaticEvalOutcome::Value(value) => {
                if trace {
                    eprintln!("dynamic_import_await:await_value {value:?}");
                }
                None
            }
        }
    }

    fn static_dynamic_import_module_namespace_value(
        &self,
        module_index: usize,
        init_function: &FunctionDeclaration,
        import_type: Option<&str>,
        visited: &mut HashSet<usize>,
    ) -> Expression {
        if !visited.insert(module_index) {
            return Expression::Identifier(format!("__ayy_module_namespace_{module_index}"));
        }
        let local_bindings = self.static_dynamic_import_module_local_bindings(init_function);
        if import_type == Some("text")
            && let Some(value) = local_bindings.get(&format!("__ayy_text_default_{module_index}"))
        {
            visited.remove(&module_index);
            return Expression::Object(vec![ObjectEntry::Data {
                key: Expression::String("default".to_string()),
                value: value.clone(),
            }]);
        }

        let mut entries = vec![
            ObjectEntry::Data {
                key: Expression::Member {
                    object: Box::new(Expression::Identifier("Symbol".to_string())),
                    property: Box::new(Expression::String("toStringTag".to_string())),
                },
                value: Expression::String("Module".to_string()),
            },
            ObjectEntry::Data {
                key: Expression::String("__ayy$module$namespace".to_string()),
                value: Expression::Bool(true),
            },
            ObjectEntry::Data {
                key: Expression::String(MODULE_NAMESPACE_DESCRIPTOR_MODULE_INDEX.to_string()),
                value: Expression::Number(module_index as f64),
            },
        ];
        for statement in &init_function.body {
            let Some((export_name, getter_name, namespace_module_index, reexport_source)) =
                self.static_dynamic_import_export_getter(statement)
            else {
                continue;
            };
            if let Some(namespace_module_index) = namespace_module_index {
                let nested_value = self
                    .resolve_registered_function_declaration(&format!(
                        "__ayy_module_init_{namespace_module_index}"
                    ))
                    .map(|nested_init| {
                        self.static_dynamic_import_module_namespace_value(
                            namespace_module_index,
                            nested_init,
                            None,
                            visited,
                        )
                    })
                    .unwrap_or_else(|| {
                        Expression::Identifier(format!(
                            "__ayy_module_namespace_{namespace_module_index}"
                        ))
                    });
                entries.push(ObjectEntry::Data {
                    key: Expression::String(export_name),
                    value: nested_value,
                });
                continue;
            }
            if let Some((reexport_module_index, imported_name)) = reexport_source {
                let reexport_value = self
                    .resolve_registered_function_declaration(&format!(
                        "__ayy_module_init_{reexport_module_index}"
                    ))
                    .map(|reexport_init| {
                        self.static_dynamic_import_module_namespace_value(
                            reexport_module_index,
                            reexport_init,
                            None,
                            visited,
                        )
                    })
                    .and_then(|namespace| {
                        static_module_namespace_expression_property(&namespace, &imported_name)
                    })
                    .unwrap_or_else(|| Expression::Member {
                        object: Box::new(Expression::Identifier(format!(
                            "__ayy_module_namespace_{reexport_module_index}"
                        ))),
                        property: Box::new(Expression::String(imported_name)),
                    });
                entries.push(ObjectEntry::Data {
                    key: Expression::String(export_name),
                    value: reexport_value,
                });
                continue;
            }
            let Some(value) =
                self.static_dynamic_import_getter_value(&getter_name, &local_bindings)
            else {
                visited.remove(&module_index);
                return Expression::Identifier(format!("__ayy_module_namespace_{module_index}"));
            };
            if expression_references_module_dependency_param(&value) {
                visited.remove(&module_index);
                return Expression::Identifier(format!("__ayy_module_namespace_{module_index}"));
            }
            entries.push(ObjectEntry::Data {
                key: Expression::String(export_name),
                value,
            });
        }
        visited.remove(&module_index);
        if entries.is_empty() {
            Expression::Identifier(format!("__ayy_module_namespace_{module_index}"))
        } else {
            Expression::Object(entries)
        }
    }

    fn static_dynamic_import_module_local_bindings(
        &self,
        init_function: &FunctionDeclaration,
    ) -> HashMap<String, Expression> {
        let mut bindings = HashMap::new();
        for statement in &init_function.body {
            self.collect_static_dynamic_import_module_local_bindings(statement, &mut bindings);
        }
        bindings
    }

    fn static_dynamic_import_module_local_bindings_with_continuations(
        &self,
        module_index: usize,
        init_function: &FunctionDeclaration,
    ) -> HashMap<String, Expression> {
        let mut bindings = self.static_dynamic_import_module_local_bindings(init_function);
        let prefix = format!("__ayy_module_async_continuation_{module_index}_");
        let mut continuations = self
            .user_functions()
            .into_iter()
            .filter_map(|function| {
                function
                    .name
                    .starts_with(&prefix)
                    .then(|| function.name.clone())
            })
            .collect::<Vec<_>>();
        continuations.sort_by_key(|name| {
            name.strip_prefix(&prefix)
                .and_then(|suffix| suffix.parse::<usize>().ok())
                .unwrap_or(usize::MAX)
        });

        for continuation_name in continuations {
            if let Some(continuation) =
                self.resolve_registered_function_declaration(&continuation_name)
            {
                self.collect_static_dynamic_import_module_continuation_local_bindings(
                    &continuation.body,
                    &mut bindings,
                );
            }
        }

        bindings
    }

    fn expression_is_static_await_resume_sent(expression: &Expression) -> bool {
        matches!(
            expression,
            Expression::Call { callee, arguments }
                if matches!(callee.as_ref(), Expression::Identifier(name) if name == "__ayyAwaitResume")
                    && matches!(
                        arguments.as_slice(),
                        [CallArgument::Expression(Expression::Sent)]
                    )
        )
    }

    fn collect_static_dynamic_import_module_continuation_local_bindings(
        &self,
        statements: &[Statement],
        bindings: &mut HashMap<String, Expression>,
    ) {
        let mut last_yield_value = None::<Expression>;
        for statement in statements {
            match statement {
                Statement::Yield { value } => {
                    last_yield_value = Some(self.materialize_static_expression(value));
                }
                Statement::Var { name, value }
                | Statement::Let { name, value, .. }
                | Statement::Assign { name, value } => {
                    let value = if Self::expression_is_static_await_resume_sent(value) {
                        last_yield_value
                            .take()
                            .unwrap_or_else(|| self.materialize_static_expression(value))
                    } else {
                        last_yield_value = None;
                        self.materialize_static_expression(value)
                    };
                    bindings.insert(name.clone(), value);
                }
                Statement::Declaration { body }
                | Statement::Block { body }
                | Statement::Labeled { body, .. } => {
                    self.collect_static_dynamic_import_module_continuation_local_bindings(
                        body, bindings,
                    );
                    last_yield_value = None;
                }
                _ => {
                    last_yield_value = None;
                }
            }
        }
    }

    fn collect_static_dynamic_import_module_local_bindings(
        &self,
        statement: &Statement,
        bindings: &mut HashMap<String, Expression>,
    ) {
        match statement {
            Statement::Var { name, value }
            | Statement::Let { name, value, .. }
            | Statement::Assign { name, value } => {
                bindings.insert(name.clone(), self.materialize_static_expression(value));
            }
            Statement::Declaration { body }
            | Statement::Block { body }
            | Statement::Labeled { body, .. } => {
                for statement in body {
                    self.collect_static_dynamic_import_module_local_bindings(statement, bindings);
                }
            }
            _ => {}
        }
    }

    fn static_dynamic_import_local_binding_value(
        name: &str,
        local_bindings: &HashMap<String, Expression>,
    ) -> Option<Expression> {
        let mut current = name.to_string();
        let mut visited = HashSet::new();
        loop {
            if !visited.insert(current.clone()) {
                return None;
            }
            let value = local_bindings.get(&current)?.clone();
            let Expression::Identifier(alias) = &value else {
                return Some(value);
            };
            if !local_bindings.contains_key(alias) {
                return Some(value);
            }
            current = alias.clone();
        }
    }

    fn static_dynamic_import_export_getter(
        &self,
        statement: &Statement,
    ) -> Option<(String, String, Option<usize>, Option<(usize, String)>)> {
        let Statement::Expression(Expression::Call { callee, arguments }) = statement else {
            return None;
        };
        let Expression::Member { object, property } = callee.as_ref() else {
            return None;
        };
        if !matches!(object.as_ref(), Expression::Identifier(name) if name == "Object")
            || !matches!(property.as_ref(), Expression::String(name) if name == "defineProperty")
        {
            return None;
        }
        let [
            CallArgument::Expression(Expression::Identifier(exports_name)),
            CallArgument::Expression(Expression::String(export_name)),
            CallArgument::Expression(Expression::Object(descriptor_entries)),
            ..,
        ] = arguments.as_slice()
        else {
            return None;
        };
        if exports_name != "exports" || export_name.starts_with("__ayy$") {
            return None;
        }
        let getter_name = descriptor_entries.iter().find_map(|entry| match entry {
            ObjectEntry::Data { key, value }
                if matches!(key, Expression::String(name) if name == "get") =>
            {
                match value {
                    Expression::Identifier(name) => Some(name.clone()),
                    _ => None,
                }
            }
            _ => None,
        })?;
        Some((
            export_name.clone(),
            getter_name,
            static_module_namespace_descriptor_module_index(descriptor_entries),
            static_module_reexport_descriptor_source(descriptor_entries),
        ))
    }

    fn static_dynamic_import_getter_value(
        &self,
        getter_name: &str,
        local_bindings: &HashMap<String, Expression>,
    ) -> Option<Expression> {
        let getter = self.resolve_registered_function_declaration(getter_name)?;
        let [Statement::Return(return_value)] = getter.body.as_slice() else {
            return None;
        };
        Some(match return_value {
            Expression::Identifier(name) => {
                let value = Self::static_dynamic_import_local_binding_value(name, local_bindings)
                    .unwrap_or_else(|| return_value.clone());
                self.static_dynamic_import_live_binding_value(name, &value, getter_name)
                    .unwrap_or(value)
            }
            Expression::Member { object, property } if matches!(object.as_ref(), Expression::Identifier(name) if name.starts_with("__ayy_module_dep_")) =>
            {
                let Expression::String(property_name) = property.as_ref() else {
                    return Some(self.materialize_static_expression(return_value));
                };
                let value =
                    Self::static_dynamic_import_local_binding_value(property_name, local_bindings)
                        .unwrap_or_else(|| self.materialize_static_expression(return_value));
                self.static_dynamic_import_live_binding_value(property_name, &value, getter_name)
                    .unwrap_or(value)
            }
            _ => self.materialize_static_expression(return_value),
        })
    }

    fn static_dynamic_import_getter_live_binding_initializer(
        &self,
        getter_name: &str,
        local_bindings: &HashMap<String, Expression>,
    ) -> Option<(String, Expression)> {
        let getter = self.resolve_registered_function_declaration(getter_name)?;
        let [Statement::Return(Expression::Identifier(binding_name))] = getter.body.as_slice()
        else {
            return None;
        };
        let initial_value =
            Self::static_dynamic_import_local_binding_value(binding_name, local_bindings)?;
        let Expression::Identifier(hidden_name) = self.static_dynamic_import_live_binding_value(
            binding_name,
            &initial_value,
            getter_name,
        )?
        else {
            return None;
        };

        Some((hidden_name, initial_value))
    }

    fn static_dynamic_import_getter_binding_initializer(
        &self,
        getter_name: &str,
        local_bindings: &HashMap<String, Expression>,
    ) -> Option<(String, Expression)> {
        let getter = self.resolve_registered_function_declaration(getter_name)?;
        let [Statement::Return(Expression::Identifier(return_name))] = getter.body.as_slice()
        else {
            return None;
        };
        if let Some(value) =
            Self::static_dynamic_import_global_namespace_capture_expression(return_name)
        {
            return Some((return_name.clone(), value));
        }
        if let Some((source_name, value)) = self
            .static_dynamic_import_getter_global_namespace_capture_source(getter_name, return_name)
        {
            return Some((source_name, value));
        }
        let binding_name = if local_bindings.contains_key(return_name) {
            return_name.clone()
        } else {
            self.user_function_capture_bindings(getter_name)?
                .iter()
                .find_map(|(binding_name, hidden_name)| {
                    (hidden_name == return_name && local_bindings.contains_key(binding_name))
                        .then(|| binding_name.clone())
                })?
        };
        let initial_value =
            Self::static_dynamic_import_local_binding_value(&binding_name, local_bindings)?;
        Some((binding_name, initial_value))
    }

    pub(in crate::backend::direct_wasm) fn resolve_static_dynamic_import_namespace_live_binding_member_value(
        &self,
        module_index: usize,
        property: &Expression,
    ) -> Option<Expression> {
        let property = self
            .resolve_property_key_expression(property)
            .unwrap_or_else(|| self.materialize_static_expression(property));
        let property = static_property_name_from_expression(&property)
            .map(Expression::String)
            .unwrap_or(property);
        if is_symbol_to_string_tag_expression(&property) {
            return Some(Expression::String("Module".to_string()));
        }
        let Expression::String(export_name) = property else {
            return None;
        };
        let mut visited = HashSet::new();
        self.resolve_static_dynamic_import_namespace_live_binding_member_value_by_name(
            module_index,
            &export_name,
            &mut visited,
        )
    }

    fn resolve_static_dynamic_import_namespace_live_binding_member_value_by_name(
        &self,
        module_index: usize,
        export_name: &str,
        visited: &mut HashSet<(usize, String)>,
    ) -> Option<Expression> {
        let key = (module_index, export_name.to_string());
        if !visited.insert(key.clone()) {
            return None;
        }
        let result = self
            .resolve_static_dynamic_import_namespace_live_binding_member_value_by_name_inner(
                module_index,
                export_name,
                visited,
            );
        visited.remove(&key);
        result
    }

    fn resolve_static_dynamic_import_namespace_live_binding_member_value_by_name_inner(
        &self,
        module_index: usize,
        export_name: &str,
        visited: &mut HashSet<(usize, String)>,
    ) -> Option<Expression> {
        let init_function = self.resolve_registered_function_declaration(&format!(
            "__ayy_module_init_{module_index}"
        ))?;
        let local_bindings = self.static_dynamic_import_module_local_bindings_with_continuations(
            module_index,
            init_function,
        );
        for statement in &init_function.body {
            let Some((candidate_name, getter_name, namespace_module_index, reexport_source)) =
                self.static_dynamic_import_export_getter(statement)
            else {
                continue;
            };
            if candidate_name != export_name {
                continue;
            }
            if let Some(namespace_module_index) = namespace_module_index {
                let mut namespace_visited = HashSet::new();
                return Some(
                    self.resolve_registered_function_declaration(&format!(
                        "__ayy_module_init_{namespace_module_index}"
                    ))
                    .map(|namespace_init| {
                        self.static_dynamic_import_module_namespace_value(
                            namespace_module_index,
                            namespace_init,
                            None,
                            &mut namespace_visited,
                        )
                    })
                    .unwrap_or_else(|| {
                        Expression::Identifier(format!(
                            "__ayy_module_namespace_{namespace_module_index}"
                        ))
                    }),
                );
            }
            if let Some((reexport_module_index, imported_name)) = reexport_source {
                return self
                    .resolve_static_dynamic_import_namespace_live_binding_member_value_by_name(
                        reexport_module_index,
                        &imported_name,
                        visited,
                    );
            }
            if let Some(hidden_name) = self
                .user_function_capture_bindings(&getter_name)
                .and_then(|captures| captures.get(export_name).cloned())
                && self
                    .implicit_global_binding(&hidden_name)
                    .or_else(|| self.hidden_implicit_global_binding(&hidden_name))
                    .is_some()
            {
                return Some(Expression::Identifier(hidden_name));
            }
            let value = self.static_dynamic_import_getter_value(&getter_name, &local_bindings)?;
            if let Expression::Identifier(name) = &value
                && name.starts_with("__ayy_capture_binding__")
                && self
                    .implicit_global_binding(name)
                    .or_else(|| self.hidden_implicit_global_binding(name))
                    .is_some()
            {
                return Some(value);
            }
            if !expression_references_module_dependency_param(&value) {
                return Some(value);
            }
        }
        None
    }

    pub(in crate::backend::direct_wasm) fn resolve_static_dynamic_import_namespace_own_property_names_binding(
        &self,
        module_index: usize,
    ) -> Option<ArrayValueBinding> {
        let init_function = self.resolve_registered_function_declaration(&format!(
            "__ayy_module_init_{module_index}"
        ))?;
        let mut names = Vec::new();
        for statement in &init_function.body {
            let Some((export_name, _, _, _)) = self.static_dynamic_import_export_getter(statement)
            else {
                continue;
            };
            if export_name.starts_with("__ayy$") {
                continue;
            }
            names.push(export_name);
        }
        names.sort();
        names.dedup();
        Some(ArrayValueBinding {
            values: names
                .into_iter()
                .map(|name| Some(Expression::String(name)))
                .collect(),
        })
    }

    pub(in crate::backend::direct_wasm) fn resolve_static_dynamic_import_namespace_own_property_symbols_binding(
        &self,
        _module_index: usize,
    ) -> ArrayValueBinding {
        ArrayValueBinding {
            values: vec![Some(Expression::Member {
                object: Box::new(Expression::Identifier("Symbol".to_string())),
                property: Box::new(Expression::String("toStringTag".to_string())),
            })],
        }
    }

    pub(in crate::backend::direct_wasm) fn resolve_static_dynamic_import_namespace_live_binding_member_initializer_value(
        &self,
        module_index: usize,
        property: &Expression,
    ) -> Option<Expression> {
        self.resolve_static_dynamic_import_namespace_live_binding_member_binding_initializer_value(
            module_index,
            property,
        )
        .map(|(_, initializer)| initializer)
    }

    pub(in crate::backend::direct_wasm) fn resolve_static_dynamic_import_namespace_live_binding_member_binding_initializer_value(
        &self,
        module_index: usize,
        property: &Expression,
    ) -> Option<(String, Expression)> {
        let property = self
            .resolve_property_key_expression(property)
            .unwrap_or_else(|| self.materialize_static_expression(property));
        let property = static_property_name_from_expression(&property)
            .map(Expression::String)
            .unwrap_or(property);
        let Expression::String(export_name) = property else {
            return None;
        };
        let mut visited = HashSet::new();
        self.resolve_static_dynamic_import_namespace_live_binding_member_binding_initializer_value_by_name(
            module_index,
            &export_name,
            &mut visited,
        )
    }

    fn resolve_static_dynamic_import_namespace_live_binding_member_binding_initializer_value_by_name(
        &self,
        module_index: usize,
        export_name: &str,
        visited: &mut HashSet<(usize, String)>,
    ) -> Option<(String, Expression)> {
        let key = (module_index, export_name.to_string());
        if !visited.insert(key.clone()) {
            return None;
        }
        let result = self.resolve_static_dynamic_import_namespace_live_binding_member_binding_initializer_value_by_name_inner(
            module_index,
            export_name,
            visited,
        );
        visited.remove(&key);
        result
    }

    fn resolve_static_dynamic_import_namespace_live_binding_member_binding_initializer_value_by_name_inner(
        &self,
        module_index: usize,
        export_name: &str,
        visited: &mut HashSet<(usize, String)>,
    ) -> Option<(String, Expression)> {
        let init_function = self.resolve_registered_function_declaration(&format!(
            "__ayy_module_init_{module_index}"
        ))?;
        let local_bindings = self.static_dynamic_import_module_local_bindings_with_continuations(
            module_index,
            init_function,
        );
        for statement in &init_function.body {
            let Some((candidate_name, getter_name, namespace_module_index, reexport_source)) =
                self.static_dynamic_import_export_getter(statement)
            else {
                continue;
            };
            if candidate_name != export_name || namespace_module_index.is_some() {
                continue;
            }
            if let Some((reexport_module_index, imported_name)) = reexport_source {
                return self.resolve_static_dynamic_import_namespace_live_binding_member_binding_initializer_value_by_name(
                    reexport_module_index,
                    &imported_name,
                    visited,
                );
            }
            if let Some((binding_name, initializer)) =
                self.static_dynamic_import_getter_binding_initializer(&getter_name, &local_bindings)
            {
                return Some((binding_name, initializer));
            }
        }
        None
    }

    fn static_dynamic_import_live_binding_value(
        &self,
        binding_name: &str,
        value: &Expression,
        getter_name: &str,
    ) -> Option<Expression> {
        if let Some(value) =
            Self::static_dynamic_import_global_namespace_capture_expression(binding_name)
        {
            return Some(value);
        }
        if let Some(hidden_name) = self
            .user_function_capture_bindings(getter_name)
            .and_then(|captures| captures.get(binding_name).cloned())
        {
            return Some(Expression::Identifier(hidden_name));
        }

        if let Expression::Identifier(function_name) = value
            && let Some(hidden_name) = self
                .user_function_capture_bindings(function_name)
                .and_then(|captures| captures.get(binding_name).cloned())
        {
            return Some(Expression::Identifier(hidden_name));
        }

        for user_function in self.user_functions() {
            let Some(hidden_name) = self
                .user_function_capture_bindings(&user_function.name)
                .and_then(|captures| captures.get(binding_name).cloned())
            else {
                continue;
            };
            if user_function.name != getter_name {
                return Some(Expression::Identifier(hidden_name));
            }
        }

        None
    }

    fn static_dynamic_import_global_namespace_capture_expression(
        binding_name: &str,
    ) -> Option<Expression> {
        if binding_name.starts_with("__ayy_module_namespace_")
            || binding_name.starts_with("__ayy_module_deferred_namespace_")
        {
            return Some(Expression::Identifier(binding_name.to_string()));
        }
        None
    }

    fn static_dynamic_import_getter_global_namespace_capture_source(
        &self,
        getter_name: &str,
        hidden_name: &str,
    ) -> Option<(String, Expression)> {
        self.user_function_capture_bindings(getter_name)?
            .iter()
            .find_map(|(source_name, capture_hidden_name)| {
                (capture_hidden_name == hidden_name)
                    .then(|| {
                        Self::static_dynamic_import_global_namespace_capture_expression(source_name)
                            .map(|value| (source_name.clone(), value))
                    })
                    .flatten()
            })
    }

    fn resolve_static_promise_constructor_outcome(
        &self,
        arguments: &[CallArgument],
        current_function_name: Option<&str>,
    ) -> Option<StaticEvalOutcome> {
        let executor = match arguments.first()? {
            CallArgument::Expression(expression) => expression,
            CallArgument::Spread(_) => return None,
        };
        let materialized_executor = self.materialize_static_expression(executor);
        let binding = self
            .resolve_function_binding_from_expression_with_context(executor, current_function_name)
            .or_else(|| {
                self.resolve_function_binding_from_expression_with_context(
                    &materialized_executor,
                    current_function_name,
                )
            })?;
        let LocalFunctionBinding::User(function_name) = binding else {
            return None;
        };
        let function = self.resolve_registered_function_declaration(&function_name)?;
        let resolve_name = function.params.first().map(|param| param.name.as_str())?;
        let reject_name = function.params.get(1).map(|param| param.name.as_str());

        for statement in &function.body {
            if let Some(outcome) = self.resolve_static_promise_executor_statement_outcome(
                statement,
                resolve_name,
                reject_name,
            ) {
                return Some(outcome);
            }
        }
        None
    }

    fn resolve_static_promise_executor_statement_outcome(
        &self,
        statement: &Statement,
        resolve_name: &str,
        reject_name: Option<&str>,
    ) -> Option<StaticEvalOutcome> {
        match statement {
            Statement::Expression(expression) | Statement::Return(expression) => self
                .resolve_static_promise_executor_expression_outcome(
                    expression,
                    resolve_name,
                    reject_name,
                ),
            Statement::Block { body } | Statement::Declaration { body } => {
                for statement in body {
                    if let Some(outcome) = self.resolve_static_promise_executor_statement_outcome(
                        statement,
                        resolve_name,
                        reject_name,
                    ) {
                        return Some(outcome);
                    }
                }
                None
            }
            _ => None,
        }
    }

    fn resolve_static_promise_executor_expression_outcome(
        &self,
        expression: &Expression,
        resolve_name: &str,
        reject_name: Option<&str>,
    ) -> Option<StaticEvalOutcome> {
        let Expression::Call { callee, arguments } = expression else {
            return None;
        };
        let Expression::Identifier(callee_name) = callee.as_ref() else {
            return None;
        };
        let settled_argument = arguments.first().map(|argument| match argument {
            CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                self.materialize_static_expression(expression)
            }
        });
        if callee_name == resolve_name {
            let value = settled_argument.unwrap_or(Expression::Undefined);
            return Some(
                self.resolve_static_await_resolution_outcome(&value)
                    .unwrap_or(StaticEvalOutcome::Value(value)),
            );
        }
        if reject_name.is_some_and(|reject_name| callee_name == reject_name) {
            return Some(StaticEvalOutcome::Throw(StaticThrowValue::Value(
                settled_argument.unwrap_or(Expression::Undefined),
            )));
        }
        None
    }
}

fn static_module_namespace_descriptor_module_index(entries: &[ObjectEntry]) -> Option<usize> {
    entries.iter().find_map(|entry| match entry {
        ObjectEntry::Data {
            key: Expression::String(key),
            value: Expression::Number(index),
        } if key == MODULE_NAMESPACE_DESCRIPTOR_MODULE_INDEX
            && index.is_finite()
            && *index >= 0.0
            && index.fract() == 0.0 =>
        {
            Some(*index as usize)
        }
        _ => None,
    })
}

fn static_module_reexport_descriptor_source(entries: &[ObjectEntry]) -> Option<(usize, String)> {
    let module_index = entries.iter().find_map(|entry| match entry {
        ObjectEntry::Data {
            key: Expression::String(key),
            value: Expression::Number(index),
        } if key == MODULE_REEXPORT_DESCRIPTOR_MODULE_INDEX
            && index.is_finite()
            && *index >= 0.0
            && index.fract() == 0.0 =>
        {
            Some(*index as usize)
        }
        _ => None,
    })?;
    let imported_name = entries.iter().find_map(|entry| match entry {
        ObjectEntry::Data {
            key: Expression::String(key),
            value: Expression::String(name),
        } if key == MODULE_REEXPORT_DESCRIPTOR_NAME => Some(name.clone()),
        _ => None,
    })?;
    Some((module_index, imported_name))
}

fn static_module_namespace_expression_property(
    namespace: &Expression,
    property_name: &str,
) -> Option<Expression> {
    let Expression::Object(entries) = namespace else {
        return None;
    };
    entries.iter().find_map(|entry| match entry {
        ObjectEntry::Data {
            key: Expression::String(name),
            value,
        } if name == property_name => Some(value.clone()),
        _ => None,
    })
}

fn expression_references_module_dependency_param(expression: &Expression) -> bool {
    match expression {
        Expression::Identifier(name) => name.starts_with("__ayy_module_dep_"),
        Expression::Array(elements) => elements.iter().any(|element| match element {
            ArrayElement::Expression(expression) | ArrayElement::Spread(expression) => {
                expression_references_module_dependency_param(expression)
            }
        }),
        Expression::Object(entries) => entries.iter().any(|entry| match entry {
            ObjectEntry::Data { key, value } => {
                expression_references_module_dependency_param(key)
                    || expression_references_module_dependency_param(value)
            }
            ObjectEntry::Getter { key, getter } => {
                expression_references_module_dependency_param(key)
                    || expression_references_module_dependency_param(getter)
            }
            ObjectEntry::Setter { key, setter } => {
                expression_references_module_dependency_param(key)
                    || expression_references_module_dependency_param(setter)
            }
            ObjectEntry::Spread(expression) => {
                expression_references_module_dependency_param(expression)
            }
        }),
        Expression::Member { object, property } => {
            expression_references_module_dependency_param(object)
                || expression_references_module_dependency_param(property)
        }
        Expression::AssignMember {
            object,
            property,
            value,
        } => {
            expression_references_module_dependency_param(object)
                || expression_references_module_dependency_param(property)
                || expression_references_module_dependency_param(value)
        }
        Expression::SuperMember { property } => {
            expression_references_module_dependency_param(property)
        }
        Expression::AssignSuperMember { property, value } => {
            expression_references_module_dependency_param(property)
                || expression_references_module_dependency_param(value)
        }
        Expression::Assign { value, .. }
        | Expression::Await(value)
        | Expression::EnumerateKeys(value)
        | Expression::GetIterator(value)
        | Expression::IteratorClose(value)
        | Expression::Unary {
            expression: value, ..
        } => expression_references_module_dependency_param(value),
        Expression::Binary { left, right, .. } => {
            expression_references_module_dependency_param(left)
                || expression_references_module_dependency_param(right)
        }
        Expression::Conditional {
            condition,
            then_expression,
            else_expression,
        } => {
            expression_references_module_dependency_param(condition)
                || expression_references_module_dependency_param(then_expression)
                || expression_references_module_dependency_param(else_expression)
        }
        Expression::Sequence(expressions) => expressions
            .iter()
            .any(expression_references_module_dependency_param),
        Expression::Call { callee, arguments }
        | Expression::SuperCall { callee, arguments }
        | Expression::New { callee, arguments } => {
            expression_references_module_dependency_param(callee)
                || arguments.iter().any(|argument| {
                    expression_references_module_dependency_param(argument.expression())
                })
        }
        Expression::Number(_)
        | Expression::BigInt(_)
        | Expression::String(_)
        | Expression::Bool(_)
        | Expression::Null
        | Expression::Undefined
        | Expression::NewTarget
        | Expression::This
        | Expression::Sent
        | Expression::Update { .. } => false,
    }
}
