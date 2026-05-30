use super::*;

impl<'a> FunctionCompiler<'a> {
    fn user_function_capture_source_is_unshadowed_builtin(&self, source_name: &str) -> bool {
        (matches!(source_name, "NaN" | "Infinity" | "undefined")
            || builtin_function_runtime_value(source_name).is_some())
            && self.is_unshadowed_builtin_identifier(source_name)
    }

    fn capture_source_expression(&self, source_name: &str) -> Expression {
        self.capture_source_expression_with_this_override(source_name, None)
    }

    fn capture_source_expression_with_this_override(
        &self,
        source_name: &str,
        this_expression_override: Option<&Expression>,
    ) -> Expression {
        if source_name == "this" {
            this_expression_override
                .cloned()
                .or_else(|| {
                    self.resolve_user_function_capture_hidden_name("this")
                        .map(Expression::Identifier)
                })
                .unwrap_or(Expression::This)
        } else if source_name == "new.target" {
            self.resolve_user_function_capture_hidden_name("new.target")
                .map(Expression::Identifier)
                .unwrap_or(Expression::NewTarget)
        } else {
            Expression::Identifier(source_name.to_string())
        }
    }

    fn function_capture_immutable_class_alias_source_name(
        &self,
        function_name: &str,
        source_name: &str,
    ) -> Option<String> {
        if matches!(source_name, "this" | "new.target") {
            return None;
        }
        let function = self
            .prepared_function_declaration(function_name)
            .or_else(|| self.resolve_registered_function_declaration(function_name))?;
        if function
            .immutable_class_bindings
            .iter()
            .any(|binding| binding == source_name)
        {
            return None;
        }
        let current_function_name = self.current_function_name()?;
        let current_function = self
            .prepared_function_declaration(current_function_name)
            .or_else(|| self.resolve_registered_function_declaration(current_function_name))?;
        function
            .immutable_class_bindings
            .iter()
            .find(|class_binding| {
                self.resolve_current_local_binding(class_binding).is_some()
                    && Self::statements_bind_alias_to_identifier(
                        &current_function.body,
                        source_name,
                        class_binding,
                    )
            })
            .cloned()
    }

    fn statements_bind_alias_to_identifier(
        statements: &[Statement],
        alias_name: &str,
        source_name: &str,
    ) -> bool {
        statements.iter().any(|statement| {
            Self::statement_binds_alias_to_identifier(statement, alias_name, source_name)
        })
    }

    fn statement_binds_alias_to_identifier(
        statement: &Statement,
        alias_name: &str,
        source_name: &str,
    ) -> bool {
        match statement {
            Statement::Let { name, value, .. }
            | Statement::Var { name, value }
            | Statement::Assign { name, value } => {
                scoped_binding_source_name(name).unwrap_or(name) == alias_name
                    && matches!(value, Expression::Identifier(value_name) if value_name == source_name)
            }
            Statement::Block { body }
            | Statement::Declaration { body }
            | Statement::Labeled { body, .. }
            | Statement::With { body, .. } => {
                Self::statements_bind_alias_to_identifier(body, alias_name, source_name)
            }
            Statement::If {
                then_branch,
                else_branch,
                ..
            } => {
                Self::statements_bind_alias_to_identifier(then_branch, alias_name, source_name)
                    || Self::statements_bind_alias_to_identifier(
                        else_branch,
                        alias_name,
                        source_name,
                    )
            }
            Statement::Try {
                body,
                catch_setup,
                catch_body,
                ..
            } => {
                Self::statements_bind_alias_to_identifier(body, alias_name, source_name)
                    || Self::statements_bind_alias_to_identifier(
                        catch_setup,
                        alias_name,
                        source_name,
                    )
                    || Self::statements_bind_alias_to_identifier(
                        catch_body,
                        alias_name,
                        source_name,
                    )
            }
            Statement::For { init, body, .. } => {
                Self::statements_bind_alias_to_identifier(init, alias_name, source_name)
                    || Self::statements_bind_alias_to_identifier(body, alias_name, source_name)
            }
            Statement::While { body, .. } | Statement::DoWhile { body, .. } => {
                Self::statements_bind_alias_to_identifier(body, alias_name, source_name)
            }
            Statement::Switch { cases, .. } => cases.iter().any(|case| {
                Self::statements_bind_alias_to_identifier(&case.body, alias_name, source_name)
            }),
            _ => false,
        }
    }

    fn capture_prepare_function_references_nested_function_in_body(
        function: &FunctionDeclaration,
        nested_function_name: &str,
    ) -> bool {
        collect_referenced_binding_names_from_statements(&function.body)
            .contains(nested_function_name)
    }

    fn capture_prepare_function_references_nested_function_in_parameter_default(
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

    fn capture_prepare_function_has_body_local_binding_source(
        function: &FunctionDeclaration,
        source_name: &str,
    ) -> bool {
        collect_declared_bindings_from_statements_recursive(&function.body)
            .into_iter()
            .any(|name| scoped_binding_source_name(&name).unwrap_or(&name) == source_name)
    }

    fn capture_prepare_function_has_parameter_binding_source(
        function: &FunctionDeclaration,
        source_name: &str,
    ) -> bool {
        (!function.lexical_this && source_name == "arguments")
            || function.params.iter().any(|parameter| {
                scoped_binding_source_name(&parameter.name).unwrap_or(&parameter.name)
                    == source_name
            })
    }

    pub(in crate::backend::direct_wasm) fn user_function_capture_originates_in_enclosing_local(
        &self,
        function_name: &str,
        source_name: &str,
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
                Self::capture_prepare_function_references_nested_function_in_body(
                    candidate,
                    function_name,
                );
            let referenced_in_parameter_default =
                Self::capture_prepare_function_references_nested_function_in_parameter_default(
                    candidate,
                    function_name,
                );
            let source_in_body = Self::capture_prepare_function_has_body_local_binding_source(
                candidate,
                source_name,
            );
            let source_in_parameters =
                Self::capture_prepare_function_has_parameter_binding_source(candidate, source_name);

            (referenced_in_body && (source_in_body || source_in_parameters))
                || (referenced_in_parameter_default && source_in_parameters)
        })
    }

    pub(in crate::backend::direct_wasm) fn prepare_user_function_capture_bindings(
        &mut self,
        user_function: &UserFunction,
    ) -> DirectResult<Vec<PreparedCaptureBinding>> {
        let Some(capture_bindings) = self.user_function_capture_bindings(&user_function.name)
        else {
            return Ok(Vec::new());
        };

        let mut prepared = Vec::new();
        for (source_name, hidden_name) in capture_bindings {
            let binding = self
                .implicit_global_binding(&hidden_name)
                .unwrap_or_else(|| self.ensure_implicit_global_binding(&hidden_name));
            let saved_value_local = self.allocate_temp_local();
            let saved_present_local = self.allocate_temp_local();
            self.push_global_get(binding.value_index);
            self.push_local_set(saved_value_local);
            self.push_global_get(binding.present_index);
            self.push_local_set(saved_present_local);
            prepared.push(PreparedCaptureBinding {
                binding,
                source_name,
                hidden_name,
                saved_value_local,
                saved_present_local,
            });
        }

        Ok(prepared)
    }

    fn emit_user_function_capture_source_value(
        &mut self,
        source_name: &str,
        source_expression: &Expression,
        prefer_global_source: bool,
    ) -> DirectResult<()> {
        if source_name == "new.target" {
            self.push_global_get(CURRENT_NEW_TARGET_GLOBAL_INDEX);
            return Ok(());
        }
        if is_internal_user_function_identifier(source_name)
            && let Some(runtime_value) = self.user_function_runtime_value(source_name)
        {
            self.push_i32_const(runtime_value);
            return Ok(());
        }
        if prefer_global_source {
            if let Some(global_index) = self.resolve_global_binding_index(source_name) {
                return self.emit_declared_global_binding_read(source_name, global_index);
            }
            if let Some(binding) = self.implicit_global_binding(source_name) {
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
        if self.user_function_capture_source_is_unshadowed_builtin(source_name) {
            if let Some(runtime_value) = builtin_function_runtime_value(source_name) {
                self.push_i32_const(runtime_value);
                return Ok(());
            }
            match source_name {
                "NaN" => {
                    self.push_i32_const(JS_NAN_TAG);
                    return Ok(());
                }
                "Infinity" => {
                    return self.emit_numeric_expression(&Expression::Number(f64::INFINITY));
                }
                "undefined" => {
                    self.push_i32_const(JS_UNDEFINED_TAG);
                    return Ok(());
                }
                _ => {}
            }
        }
        if source_name.starts_with("__ayy_class_brand_")
            && self.emit_private_brand_runtime_value_for_binding_name(source_name)?
        {
            return Ok(());
        }
        if source_name.starts_with("__ayy_class_brand_") {
            return self.emit_private_brand_direct_or_synthetic_runtime_value_for_binding_name(
                source_name,
            );
        }
        self.emit_numeric_expression(source_expression)
    }

    fn sync_user_function_capture_runtime_object_shadows_for_source(
        &mut self,
        hidden_name: &str,
        source_name: &str,
        source_expression: &Expression,
    ) -> DirectResult<()> {
        if source_name == "new.target" {
            return Ok(());
        }
        if source_name == "this" {
            let owner_name = match source_expression {
                Expression::Identifier(name) => {
                    self.runtime_object_property_shadow_owner_name_for_identifier(name)
                }
                Expression::This => {
                    self.runtime_object_property_shadow_owner_name_for_identifier("this")
                }
                _ => None,
            };
            if let Some(owner_name) = owner_name
                && owner_name != hidden_name
            {
                self.emit_runtime_object_property_shadow_copy(&owner_name, hidden_name)?;
            } else if let Some(object_binding) =
                self.resolve_object_binding_from_expression(source_expression)
            {
                self.emit_runtime_object_property_shadow_seed_from_binding(
                    hidden_name,
                    &object_binding,
                )?;
            }
        } else {
            self.emit_runtime_object_property_shadow_copy(source_name, hidden_name)?;
        }
        Ok(())
    }

    pub(in crate::backend::direct_wasm) fn local_lexical_capture_source_is_statically_uninitialized(
        &self,
        resolved_name: &str,
    ) -> bool {
        self.local_lexical_initialized_local(resolved_name)
            .is_some()
            && self
                .state
                .speculation
                .static_semantics
                .local_value_binding(resolved_name)
                .is_none()
            && self
                .state
                .speculation
                .static_semantics
                .local_function_binding(resolved_name)
                .is_none()
            && self
                .state
                .speculation
                .static_semantics
                .local_array_binding(resolved_name)
                .is_none()
            && self
                .state
                .speculation
                .static_semantics
                .local_object_binding(resolved_name)
                .is_none()
            && self
                .state
                .speculation
                .static_semantics
                .local_proxy_binding(resolved_name)
                .is_none()
    }

    pub(in crate::backend::direct_wasm) fn emit_prepare_user_function_capture_globals(
        &mut self,
        function_name: &str,
    ) -> DirectResult<()> {
        self.emit_prepare_user_function_capture_globals_with_this_expression(function_name, None)
    }

    pub(in crate::backend::direct_wasm) fn emit_prepare_user_function_capture_globals_with_this_expression(
        &mut self,
        function_name: &str,
        this_expression_override: Option<&Expression>,
    ) -> DirectResult<()> {
        let Some(capture_bindings) = self.user_function_capture_bindings(function_name) else {
            return Ok(());
        };

        for (source_name, hidden_name) in capture_bindings {
            let binding = self
                .implicit_global_binding(&hidden_name)
                .unwrap_or_else(|| self.ensure_implicit_global_binding(&hidden_name));
            let capture_originates_in_enclosing_local = self
                .user_function_capture_originates_in_enclosing_local(function_name, &source_name);
            let capture_source_name = self
                .function_capture_immutable_class_alias_source_name(function_name, &source_name)
                .unwrap_or_else(|| source_name.clone());
            let source_is_directly_bound = if source_name == "this" || source_name == "new.target" {
                true
            } else {
                self.parameter_scope_arguments_local_for(&capture_source_name)
                    .is_some()
                    || (self.is_current_arguments_binding_name(&capture_source_name)
                        && self.has_arguments_object())
                    || self
                        .resolve_current_local_binding(&capture_source_name)
                        .is_some()
                    || self
                        .state
                        .speculation
                        .static_semantics
                        .has_local_function_binding(&capture_source_name)
                    || (is_internal_user_function_identifier(&capture_source_name)
                        && self.contains_user_function(&capture_source_name))
                    || self
                        .resolve_eval_local_function_hidden_name(&capture_source_name)
                        .is_some()
                    || self
                        .resolve_user_function_capture_hidden_name(&capture_source_name)
                        .is_some()
                    || (!capture_originates_in_enclosing_local
                        && (self.global_has_binding(&capture_source_name)
                            || self
                                .backend
                                .global_has_lexical_binding(&capture_source_name)
                            || self
                                .backend
                                .global_function_binding(&capture_source_name)
                                .is_some()))
                    || self.user_function_capture_source_is_unshadowed_builtin(&capture_source_name)
            };
            if !source_is_directly_bound {
                continue;
            }
            let source_expression = self.capture_source_expression_with_this_override(
                &capture_source_name,
                this_expression_override,
            );
            let resolved_local_binding = self.resolve_current_local_binding(&capture_source_name);
            let prefer_global_source = !capture_originates_in_enclosing_local
                && resolved_local_binding.is_none()
                && (self.global_has_binding(&capture_source_name)
                    || self.global_has_implicit_binding(&capture_source_name)
                    || self
                        .backend
                        .global_has_lexical_binding(&capture_source_name)
                    || self
                        .backend
                        .global_function_binding(&capture_source_name)
                        .is_some());
            let value_local = self.allocate_temp_local();
            let lexical_initialized_local = resolved_local_binding
                .as_ref()
                .and_then(|(resolved_name, _)| self.local_lexical_initialized_local(resolved_name));
            if std::env::var_os("AYY_TRACE_CAPTURE_BINDINGS").is_some() {
                eprintln!(
                    "capture_prepare fn={function_name} source={source_name} capture_source={capture_source_name} hidden={hidden_name} resolved={:?} initialized_local={:?} statically_uninitialized={}",
                    resolved_local_binding,
                    lexical_initialized_local,
                    resolved_local_binding
                        .as_ref()
                        .is_some_and(|(resolved_name, _)| {
                            self.local_lexical_capture_source_is_statically_uninitialized(
                                resolved_name,
                            )
                        })
                );
            }
            if resolved_local_binding
                .as_ref()
                .is_some_and(|(resolved_name, _)| {
                    self.local_lexical_capture_source_is_statically_uninitialized(resolved_name)
                })
            {
                self.clear_user_function_capture_static_metadata(&hidden_name);
                self.push_i32_const(JS_UNDEFINED_TAG);
                self.push_global_set(binding.value_index);
                self.push_i32_const(0);
                self.push_global_set(binding.present_index);
                continue;
            }
            self.sync_user_function_capture_static_metadata_from_expression(
                &hidden_name,
                &source_expression,
            );
            if let Some(initialized_local) = lexical_initialized_local {
                self.push_local_get(initialized_local);
                self.state.emission.output.instructions.push(0x04);
                self.state
                    .emission
                    .output
                    .instructions
                    .push(EMPTY_BLOCK_TYPE);
                self.push_control_frame();
                self.emit_user_function_capture_source_value(
                    &capture_source_name,
                    &source_expression,
                    prefer_global_source,
                )?;
                self.push_local_set(value_local);
                self.push_local_get(value_local);
                self.push_global_set(binding.value_index);
                self.push_i32_const(1);
                self.push_global_set(binding.present_index);
                self.sync_user_function_capture_runtime_object_shadows_for_source(
                    &hidden_name,
                    &capture_source_name,
                    &source_expression,
                )?;
                self.state.emission.output.instructions.push(0x05);
                self.push_i32_const(JS_UNDEFINED_TAG);
                self.push_global_set(binding.value_index);
                self.push_i32_const(0);
                self.push_global_set(binding.present_index);
                self.state.emission.output.instructions.push(0x0b);
                self.pop_control_frame();
            } else {
                self.emit_user_function_capture_source_value(
                    &capture_source_name,
                    &source_expression,
                    prefer_global_source,
                )?;
                self.push_local_set(value_local);
                self.push_local_get(value_local);
                self.push_global_set(binding.value_index);
                self.sync_user_function_capture_runtime_object_shadows_for_source(
                    &hidden_name,
                    &capture_source_name,
                    &source_expression,
                )?;
            }
            if lexical_initialized_local.is_none() {
                self.push_i32_const(1);
                self.push_global_set(binding.present_index);
            }
        }

        Ok(())
    }

    pub(in crate::backend::direct_wasm) fn user_function_capture_source_is_locally_bound(
        &self,
        name: &str,
    ) -> bool {
        if name == "this" {
            return true;
        }
        if name == "new.target" {
            return true;
        }
        self.parameter_scope_arguments_local_for(name).is_some()
            || (self.is_current_arguments_binding_name(name) && self.has_arguments_object())
            || self.resolve_current_local_binding(name).is_some()
            || self
                .state
                .speculation
                .static_semantics
                .has_local_function_binding(name)
            || (is_internal_user_function_identifier(name) && self.contains_user_function(name))
            || self.resolve_eval_local_function_hidden_name(name).is_some()
            || self
                .resolve_user_function_capture_hidden_name(name)
                .is_some()
            || self.global_has_binding(name)
            || self.global_has_implicit_binding(name)
            || self.user_function_capture_source_is_unshadowed_builtin(name)
    }

    pub(in crate::backend::direct_wasm) fn clear_user_function_capture_static_metadata(
        &mut self,
        hidden_name: &str,
    ) {
        self.backend
            .clear_global_static_binding_metadata(hidden_name);
        self.backend
            .clear_shared_global_static_binding_metadata(hidden_name);
    }

    pub(in crate::backend::direct_wasm) fn sync_user_function_capture_static_metadata(
        &mut self,
        source_name: &str,
        hidden_name: &str,
    ) {
        let source_expression = self.capture_source_expression(source_name);
        self.sync_user_function_capture_static_metadata_from_expression(
            hidden_name,
            &source_expression,
        );
    }

    fn sync_user_function_capture_static_metadata_from_expression(
        &mut self,
        hidden_name: &str,
        source_expression: &Expression,
    ) {
        let inferred_kind = self.infer_value_kind(&source_expression);
        let resolved_value = self.resolve_bound_alias_expression(&source_expression);
        let expression_binding =
            resolved_value.filter(|value| !static_expression_matches(value, &source_expression));
        let array_binding = self.resolve_array_binding_from_expression(&source_expression);
        let object_binding = self.resolve_object_binding_from_expression(&source_expression);
        let function_binding = self.resolve_function_binding_from_expression(&source_expression);
        let resizable_buffer_binding =
            self.resolve_resizable_array_buffer_binding_from_expression(&source_expression);
        let typed_array_view_binding =
            self.resolve_typed_array_view_binding_from_expression(&source_expression);

        self.backend
            .sync_global_expression_binding(hidden_name, expression_binding.clone());
        if let Some(value) = expression_binding {
            self.backend
                .shared_global_semantics
                .values
                .set_value_binding(hidden_name.to_string(), value);
        } else {
            self.backend
                .shared_global_semantics
                .values
                .clear_value_binding(hidden_name);
        }
        self.backend
            .sync_global_array_binding(hidden_name, array_binding.clone());
        self.backend
            .shared_global_semantics
            .values
            .sync_array_binding(hidden_name, array_binding);
        self.backend
            .sync_global_object_binding(hidden_name, object_binding.clone());
        self.backend
            .shared_global_semantics
            .values
            .sync_object_binding(hidden_name, object_binding);
        self.backend
            .sync_global_function_binding(hidden_name, function_binding.clone());
        if let Some(function_binding) = function_binding {
            self.backend
                .shared_global_semantics
                .set_global_function_binding(hidden_name, function_binding);
        } else {
            self.backend
                .shared_global_semantics
                .clear_global_function_binding(hidden_name);
        }
        self.backend
            .global_semantics
            .values
            .sync_resizable_array_buffer_binding(hidden_name, resizable_buffer_binding.clone());
        self.backend
            .shared_global_semantics
            .values
            .sync_resizable_array_buffer_binding(hidden_name, resizable_buffer_binding);
        self.backend
            .global_semantics
            .values
            .sync_typed_array_view_binding(hidden_name, typed_array_view_binding.clone());
        self.backend
            .shared_global_semantics
            .values
            .sync_typed_array_view_binding(hidden_name, typed_array_view_binding.clone());
        if let Some(view) = typed_array_view_binding
            && let Some(buffer_binding) = self
                .state
                .speculation
                .static_semantics
                .local_resizable_array_buffer_binding(&view.buffer_name)
                .cloned()
                .or_else(|| {
                    self.global_resizable_array_buffer_binding(&view.buffer_name)
                        .cloned()
                })
        {
            self.backend
                .global_semantics
                .values
                .sync_resizable_array_buffer_binding(
                    &view.buffer_name,
                    Some(buffer_binding.clone()),
                );
            self.backend
                .shared_global_semantics
                .values
                .sync_resizable_array_buffer_binding(&view.buffer_name, Some(buffer_binding));
        }
        if let Expression::Identifier(source_name) = source_expression
            && source_name != hidden_name
        {
            self.copy_member_bindings_for_alias(hidden_name, source_name);
        }

        if let Some(kind) = inferred_kind {
            self.backend.set_global_binding_kind(hidden_name, kind);
        } else {
            self.clear_global_binding_kind(hidden_name);
        }
    }

    pub(in crate::backend::direct_wasm) fn restore_user_function_capture_bindings(
        &mut self,
        prepared: &[PreparedCaptureBinding],
    ) {
        for binding in prepared.iter().rev() {
            if !self.prepared_capture_binding_should_restore_after_call(binding) {
                continue;
            }
            self.push_local_get(binding.saved_value_local);
            self.push_global_set(binding.binding.value_index);
            self.push_local_get(binding.saved_present_local);
            self.push_global_set(binding.binding.present_index);
        }
    }

    fn prepared_capture_binding_should_restore_after_call(
        &self,
        binding: &PreparedCaptureBinding,
    ) -> bool {
        matches!(binding.source_name.as_str(), "this" | "new.target")
            || binding.source_name.starts_with("__ayy_class_brand_")
            || binding.source_name.starts_with("__ayy_class_super_")
            || self.user_function_capture_source_is_locally_bound(&binding.source_name)
    }

    fn preferred_this_capture_target_owner<'b>(
        &self,
        this_capture_target_owner: Option<&'b str>,
    ) -> Option<&'b str> {
        this_capture_target_owner.filter(|owner| !owner.contains("saved_this_shadow"))
    }

    pub(in crate::backend::direct_wasm) fn sync_user_function_capture_source_bindings(
        &mut self,
        prepared: &[PreparedCaptureBinding],
        assigned_nonlocal_bindings: &HashSet<String>,
        call_effect_nonlocal_bindings: &HashSet<String>,
        updated_nonlocal_bindings: &HashSet<String>,
        updated_bindings: Option<&HashMap<String, Expression>>,
        this_capture_target_owner: Option<&str>,
    ) -> DirectResult<()> {
        for binding in prepared {
            if !self.user_function_capture_source_is_locally_bound(&binding.source_name) {
                continue;
            }
            let source_aliases_this = if binding.source_name == "this"
                || binding.source_name.starts_with("__ayy_class_brand_")
            {
                false
            } else {
                let source_expression = Expression::Identifier(binding.source_name.clone());
                self.resolve_bound_alias_expression(&source_expression)
                    .is_some_and(|resolved| match resolved {
                        Expression::This => true,
                        Expression::Identifier(name) => name == "this",
                        _ => false,
                    })
            };
            let value_local = self.allocate_temp_local();
            self.push_global_get(binding.binding.value_index);
            self.push_local_set(value_local);
            let source_is_dynamic = self.sync_user_function_capture_source_static_metadata(
                &binding.source_name,
                &binding.hidden_name,
                assigned_nonlocal_bindings,
                call_effect_nonlocal_bindings,
                updated_nonlocal_bindings,
                updated_bindings,
            )?;
            if source_is_dynamic {
                self.state
                    .runtime
                    .locals
                    .runtime_dynamic_bindings
                    .insert(binding.source_name.clone());
            } else {
                self.state
                    .runtime
                    .locals
                    .runtime_dynamic_bindings
                    .remove(&binding.source_name);
            }
            if binding.source_name == "this" {
                if let Some(owner_name) = self
                    .preferred_this_capture_target_owner(this_capture_target_owner)
                    .map(str::to_string)
                    .or_else(|| self.resolve_user_function_capture_hidden_name("this"))
                    .or_else(|| this_capture_target_owner.map(str::to_string))
                    .or_else(|| {
                        self.runtime_object_property_shadow_owner_name_for_identifier("this")
                    })
                    && owner_name != binding.hidden_name
                {
                    self.emit_runtime_object_property_shadow_copy(
                        &binding.hidden_name,
                        &owner_name,
                    )?;
                }
                continue;
            }
            if binding.source_name == "new.target" {
                continue;
            }
            if binding.source_name.starts_with("__ayy_class_brand_")
                || binding.source_name.starts_with("__ayy_class_super_")
            {
                self.emit_runtime_object_property_shadow_copy(
                    &binding.hidden_name,
                    &binding.source_name,
                )?;
                continue;
            }
            let source_is_immutable_local = self
                .resolve_current_local_binding(&binding.source_name)
                .is_some_and(|(resolved_name, _)| self.local_binding_is_immutable(&resolved_name))
                || self.binding_is_immutable_function_self_binding_source(&binding.source_name)
                || self
                    .backend
                    .lexical_global_binding(&binding.source_name)
                    .is_some_and(|global_binding| !global_binding.mutable);
            if !source_is_immutable_local {
                self.emit_sync_identifier_runtime_value_from_local(
                    &binding.source_name,
                    value_local,
                )?;
            }
            self.emit_runtime_object_property_shadow_copy(
                &binding.hidden_name,
                &binding.source_name,
            )?;
            if source_aliases_this {
                let this_owner = self
                    .runtime_object_property_shadow_owner_name_for_identifier("this")
                    .unwrap_or_else(|| "this".to_string());
                self.emit_runtime_object_property_shadow_copy(&binding.hidden_name, &this_owner)?;
                if this_owner != "this" {
                    self.emit_runtime_object_property_shadow_copy(&binding.hidden_name, "this")?;
                }
                if let Some(object_binding) =
                    self.resolve_runtime_shadow_object_binding(&binding.hidden_name)
                {
                    self.sync_runtime_object_property_shadow_static_metadata_from_binding(
                        &this_owner,
                        &object_binding,
                    );
                    if this_owner != "this" {
                        self.sync_runtime_object_property_shadow_static_metadata_from_binding(
                            "this",
                            &object_binding,
                        );
                    }
                }
            }
        }
        Ok(())
    }

    pub(in crate::backend::direct_wasm) fn sync_current_function_capture_runtime_values_for_call_effects(
        &mut self,
        names: &HashSet<String>,
    ) -> DirectResult<()> {
        let syncs = names
            .iter()
            .filter(|source_name| source_name.as_str() != "this")
            .filter(|source_name| source_name.as_str() != "new.target")
            .filter(|source_name| {
                self.global_has_binding(source_name)
                    || self.global_has_implicit_binding(source_name)
                    || self.backend.global_has_lexical_binding(source_name)
            })
            .filter_map(|source_name| {
                self.resolve_user_function_capture_hidden_name(source_name)
                    .map(|hidden_name| (source_name.clone(), hidden_name))
            })
            .collect::<Vec<_>>();

        for (source_name, hidden_name) in syncs {
            let binding = self
                .implicit_global_binding(&hidden_name)
                .unwrap_or_else(|| self.ensure_implicit_global_binding(&hidden_name));
            let value_local = self.allocate_temp_local();
            let source_expression = Expression::Identifier(source_name.clone());
            self.emit_user_function_capture_source_value(&source_name, &source_expression, true)?;
            self.push_local_set(value_local);
            self.push_local_get(value_local);
            self.push_global_set(binding.value_index);
            self.push_i32_const(1);
            self.push_global_set(binding.present_index);
            self.clear_user_function_capture_static_metadata(&hidden_name);
            self.sync_user_function_capture_runtime_object_shadows_for_source(
                &hidden_name,
                &source_name,
                &source_expression,
            )?;
        }

        Ok(())
    }

    pub(in crate::backend::direct_wasm) fn sync_user_function_capture_source_static_metadata(
        &mut self,
        source_name: &str,
        hidden_name: &str,
        assigned_nonlocal_bindings: &HashSet<String>,
        call_effect_nonlocal_bindings: &HashSet<String>,
        updated_nonlocal_bindings: &HashSet<String>,
        updated_bindings: Option<&HashMap<String, Expression>>,
    ) -> DirectResult<bool> {
        let invalidate_source = |compiler: &mut Self, preserve_kind: bool| {
            let names = HashSet::from([source_name.to_string()]);
            if preserve_kind {
                if let Some(kind) = compiler
                    .backend
                    .global_semantics
                    .names
                    .kinds
                    .get(hidden_name)
                    .copied()
                    .or_else(|| compiler.lookup_identifier_kind(source_name))
                {
                    let preserved_kinds = HashMap::from([(source_name.to_string(), kind)]);
                    compiler.invalidate_static_binding_metadata_for_names_with_preserved_kinds(
                        &names,
                        &preserved_kinds,
                    );
                    return;
                }
            }
            compiler.invalidate_static_binding_metadata_for_names(&names);
        };

        if (!assigned_nonlocal_bindings.contains(source_name)
            && updated_nonlocal_bindings.contains(source_name)
            || (!assigned_nonlocal_bindings.contains(source_name)
                && call_effect_nonlocal_bindings.contains(source_name)
                && updated_bindings
                    .and_then(|bindings| bindings.get(source_name))
                    .is_none()))
            && self
                .backend
                .global_semantics
                .names
                .bindings
                .contains_key(source_name)
        {
            invalidate_source(self, true);
            return Ok(true);
        }

        let hidden_expression = Expression::Identifier(hidden_name.to_string());
        let resolved_hidden_value = self.resolve_bound_alias_expression(&hidden_expression);
        if assigned_nonlocal_bindings.contains(source_name) {
            if let Some(value) = updated_bindings.and_then(|bindings| bindings.get(source_name)) {
                self.sync_bound_capture_source_binding_metadata(source_name, value)?;
                return Ok(false);
            }
            invalidate_source(self, false);
            return Ok(true);
        }

        match resolved_hidden_value {
            Some(Expression::Identifier(name)) if name == hidden_name => {
                invalidate_source(self, true);
                Ok(true)
            }
            Some(value) => {
                self.sync_bound_capture_source_binding_metadata(source_name, &value)?;
                Ok(false)
            }
            None => {
                invalidate_source(self, false);
                Ok(true)
            }
        }
    }
}
