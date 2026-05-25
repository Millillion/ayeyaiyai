use super::*;

impl<'a> FunctionCompiler<'a> {
    fn substitute_setter_receiver_this_binding(
        &self,
        expression: &Expression,
        receiver_expression: &Expression,
    ) -> Expression {
        match expression {
            Expression::This => receiver_expression.clone(),
            _ => expression.clone(),
        }
    }

    fn evaluate_simple_setter_statement_for_nonlocal_metadata(
        &self,
        statement: &Statement,
        environment: &mut StaticResolutionEnvironment,
    ) -> bool {
        match statement {
            Statement::Var { name, value } | Statement::Let { name, value, .. } => {
                let value = self
                    .evaluate_static_expression_with_state(value, environment)
                    .or_else(|| self.materialize_static_expression_with_state(value, environment))
                    .unwrap_or_else(|| value.clone());
                environment.set_local_binding(name.clone(), value);
                true
            }
            Statement::Assign { name, value } => {
                let value = self
                    .evaluate_static_expression_with_state(value, environment)
                    .or_else(|| self.materialize_static_expression_with_state(value, environment))
                    .unwrap_or_else(|| value.clone());
                environment.assign_binding_value(name.clone(), value);
                true
            }
            Statement::Expression(expression) => {
                let _ = self
                    .evaluate_static_expression_with_state(expression, environment)
                    .or_else(|| {
                        self.materialize_static_expression_with_state(expression, environment)
                    });
                true
            }
            Statement::Declaration { body }
            | Statement::Block { body }
            | Statement::Labeled { body, .. }
            | Statement::With { body, .. } => body.iter().all(|nested| {
                self.evaluate_simple_setter_statement_for_nonlocal_metadata(nested, environment)
            }),
            Statement::If {
                condition,
                then_branch,
                else_branch,
            } => match self.evaluate_static_expression_with_state(condition, environment) {
                Some(Expression::Bool(true)) => then_branch.iter().all(|nested| {
                    self.evaluate_simple_setter_statement_for_nonlocal_metadata(nested, environment)
                }),
                Some(Expression::Bool(false)) => else_branch.iter().all(|nested| {
                    self.evaluate_simple_setter_statement_for_nonlocal_metadata(nested, environment)
                }),
                _ => true,
            },
            Statement::Return(value) | Statement::Throw(value) => {
                let _ = self
                    .evaluate_static_expression_with_state(value, environment)
                    .or_else(|| self.materialize_static_expression_with_state(value, environment));
                false
            }
            _ => true,
        }
    }

    fn sync_simple_setter_function_value_capture_slots(
        &mut self,
        target_name: &str,
        value: &Expression,
        environment: &StaticResolutionEnvironment,
    ) -> DirectResult<()> {
        let Some(LocalFunctionBinding::User(function_name)) =
            self.resolve_function_binding_from_expression(value)
        else {
            return Ok(());
        };
        let Some(capture_bindings) = self.user_function_capture_bindings(&function_name) else {
            return Ok(());
        };
        if capture_bindings.is_empty() {
            return Ok(());
        }

        let mut capture_slots = BTreeMap::new();
        for capture_name in capture_bindings.keys() {
            let Some(source_expression) = environment.binding(capture_name).cloned() else {
                continue;
            };
            if !environment.local_binding(capture_name).is_some() {
                continue;
            }
            let hidden_name = format!("__ayy_closure_slot_{}_{}", target_name, capture_name);
            self.ensure_implicit_global_binding(&hidden_name);
            self.update_static_global_assignment_metadata(&hidden_name, &source_expression);
            capture_slots.insert(capture_name.clone(), hidden_name);
        }

        if !capture_slots.is_empty() {
            let key = Self::identifier_function_value_capture_slots_key(target_name);
            self.backend
                .set_global_member_function_capture_slots(key, capture_slots);
        }

        Ok(())
    }

    fn sync_simple_setter_updated_binding_metadata(
        &mut self,
        source_name: &str,
        value: &Expression,
        user_function: &UserFunction,
        environment: &mut StaticResolutionEnvironment,
    ) -> DirectResult<()> {
        if source_name == "this"
            || source_name == "arguments"
            || user_function
                .params
                .iter()
                .any(|param| param == source_name)
            || user_function.scope_bindings.contains(source_name)
        {
            environment.assign_binding_value(source_name.to_string(), value.clone());
            return Ok(());
        }

        if self.global_has_binding(source_name) || self.global_has_implicit_binding(source_name) {
            self.update_static_global_assignment_metadata(source_name, value);
            self.update_global_specialized_function_value(source_name, value)?;
            self.sync_simple_setter_function_value_capture_slots(source_name, value, environment)?;
            self.update_global_property_descriptor_value(source_name, value);
        } else {
            self.sync_bound_capture_source_binding_metadata(source_name, value)?;
        }
        environment.assign_binding_value(source_name.to_string(), value.clone());
        Ok(())
    }

    fn simple_setter_bound_snapshot_bindings(
        environment: &StaticResolutionEnvironment,
    ) -> HashMap<String, Expression> {
        let mut bindings = environment
            .global_value_bindings
            .iter()
            .map(|(name, value)| (name.clone(), value.clone()))
            .collect::<HashMap<_, _>>();
        bindings.extend(
            environment
                .global_value_overrides
                .iter()
                .map(|(name, value)| (name.clone(), value.clone())),
        );
        bindings.extend(
            environment
                .local_bindings
                .iter()
                .map(|(name, value)| (name.clone(), value.clone())),
        );
        bindings
    }

    fn sync_simple_setter_expression_call_metadata(
        &mut self,
        expression: &Expression,
        user_function: &UserFunction,
        arguments: &[CallArgument],
        receiver_expression: &Expression,
        environment: &mut StaticResolutionEnvironment,
    ) -> DirectResult<bool> {
        if !matches!(
            expression,
            Expression::Call { .. } | Expression::SuperCall { .. }
        ) {
            return Ok(false);
        }

        let substituted =
            self.substitute_user_function_argument_bindings(expression, user_function, arguments);
        let substituted =
            self.substitute_setter_receiver_this_binding(&substituted, receiver_expression);
        let mut bindings = Self::simple_setter_bound_snapshot_bindings(environment);
        bindings.insert("this".to_string(), receiver_expression.clone());
        let previous_bindings = bindings.clone();
        if self
            .evaluate_bound_snapshot_expression(
                &substituted,
                &mut bindings,
                Some(&user_function.name),
            )
            .is_none()
        {
            return Ok(false);
        }

        for (name, value) in bindings {
            if previous_bindings
                .get(&name)
                .is_some_and(|previous| static_expression_matches(previous, &value))
            {
                continue;
            }
            let source_name = scoped_binding_source_name(&name).unwrap_or(&name);
            self.sync_simple_setter_updated_binding_metadata(
                source_name,
                &value,
                user_function,
                environment,
            )?;
        }

        Ok(true)
    }

    pub(in crate::backend::direct_wasm) fn sync_simple_setter_nonlocal_assignment_metadata(
        &mut self,
        setter_binding: &LocalFunctionBinding,
        value_expression: &Expression,
        receiver_expression: &Expression,
    ) -> DirectResult<()> {
        let LocalFunctionBinding::User(function_name) = setter_binding else {
            return Ok(());
        };
        let Some(user_function) = self.user_function(function_name).cloned() else {
            return Ok(());
        };
        let Some(function) = self
            .resolve_registered_function_declaration(function_name)
            .cloned()
        else {
            return Ok(());
        };
        let arguments = [CallArgument::Expression(value_expression.clone())];
        let mut environment = self.snapshot_static_resolution_environment();
        environment.set_local_binding("this".to_string(), receiver_expression.clone());
        for (index, param_name) in user_function.params.iter().enumerate() {
            let argument = match arguments.get(index) {
                Some(CallArgument::Expression(value) | CallArgument::Spread(value)) => {
                    value.clone()
                }
                None => Expression::Undefined,
            };
            environment.set_local_binding(param_name.clone(), argument);
        }

        for statement in &function.body {
            let Statement::Assign { name, value } = statement else {
                if let Statement::Expression(expression) = statement
                    && self.sync_simple_setter_expression_call_metadata(
                        expression,
                        &user_function,
                        &arguments,
                        receiver_expression,
                        &mut environment,
                    )?
                {
                    continue;
                }
                if !self.evaluate_simple_setter_statement_for_nonlocal_metadata(
                    statement,
                    &mut environment,
                ) {
                    break;
                }
                continue;
            };
            let source_name = scoped_binding_source_name(name).unwrap_or(name);
            if source_name == "this"
                || source_name == "arguments"
                || user_function.scope_bindings.contains(source_name)
            {
                self.evaluate_simple_setter_statement_for_nonlocal_metadata(
                    statement,
                    &mut environment,
                );
                continue;
            }

            let substituted =
                self.substitute_user_function_argument_bindings(value, &user_function, &arguments);
            let substituted =
                self.substitute_setter_receiver_this_binding(&substituted, receiver_expression);
            self.sync_simple_setter_updated_binding_metadata(
                source_name,
                &substituted,
                &user_function,
                &mut environment,
            )?;
        }

        Ok(())
    }

    fn private_setter_receiver_brand_statically_guaranteed(
        &mut self,
        object: &Expression,
        receiver_object: &Expression,
        property: &Expression,
        _setter_binding: &LocalFunctionBinding,
    ) -> bool {
        self.resolve_runtime_object_property_shadow_binding(object, property)
            .or_else(|| {
                self.resolve_runtime_object_property_shadow_binding(receiver_object, property)
            })
            .is_some()
            || self
                .resolve_object_binding_from_expression(object)
                .or_else(|| self.resolve_object_binding_from_expression(receiver_object))
                .and_then(|object_binding| {
                    self.resolve_object_binding_property_value(&object_binding, property)
                })
                .is_some()
    }

    fn prepare_setter_receiver_runtime_shadow_state(
        &mut self,
        object: &Expression,
        receiver_hidden_name: &str,
        receiver_expression: &Expression,
    ) -> DirectResult<()> {
        let source_owner = match object {
            Expression::Identifier(name) => {
                self.runtime_object_property_shadow_owner_name_for_identifier(name)
            }
            Expression::This => {
                self.runtime_object_property_shadow_owner_name_for_identifier("this")
            }
            _ => None,
        };

        if let Some(source_owner) = source_owner {
            self.emit_runtime_object_property_shadow_copy(&source_owner, receiver_hidden_name)?;
            return Ok(());
        }

        if let Some(object_binding) =
            self.resolve_object_binding_from_expression(receiver_expression)
        {
            self.emit_runtime_object_property_shadow_seed_from_binding(
                receiver_hidden_name,
                &object_binding,
            )?;
        }

        Ok(())
    }

    fn emit_setter_receiver_shadow_commit_to_this(
        &mut self,
        receiver_hidden_name: &str,
    ) -> DirectResult<()> {
        let updated_receiver_binding =
            self.resolve_runtime_shadow_object_binding(receiver_hidden_name);
        let this_owner = self
            .runtime_object_property_shadow_owner_name_for_identifier("this")
            .unwrap_or_else(|| "this".to_string());
        self.emit_runtime_object_property_shadow_copy(receiver_hidden_name, &this_owner)?;
        if let Some(updated_receiver_binding) = updated_receiver_binding.as_ref() {
            self.sync_runtime_object_shadow_owner_static_metadata_from_binding(
                &this_owner,
                updated_receiver_binding,
            );
        }
        if this_owner != "this" {
            self.emit_runtime_object_property_shadow_copy(receiver_hidden_name, "this")?;
            if let Some(updated_receiver_binding) = updated_receiver_binding.as_ref() {
                self.sync_runtime_object_shadow_owner_static_metadata_from_binding(
                    "this",
                    updated_receiver_binding,
                );
            }
        }
        Ok(())
    }

    fn emit_setter_receiver_shadow_commit_to_identifier(
        &mut self,
        receiver_hidden_name: &str,
        identifier_name: &str,
    ) -> DirectResult<()> {
        let updated_receiver_binding =
            self.resolve_runtime_shadow_object_binding(receiver_hidden_name);
        let identifier_owner = self
            .runtime_object_property_shadow_owner_name_for_identifier(identifier_name)
            .unwrap_or_else(|| identifier_name.to_string());
        self.emit_runtime_object_property_shadow_copy(receiver_hidden_name, &identifier_owner)?;
        if let Some(updated_receiver_binding) = updated_receiver_binding.as_ref() {
            self.sync_runtime_object_shadow_owner_static_metadata_from_binding(
                &identifier_owner,
                updated_receiver_binding,
            );
        }
        if identifier_owner != identifier_name {
            self.emit_runtime_object_property_shadow_copy(receiver_hidden_name, identifier_name)?;
            if let Some(updated_receiver_binding) = updated_receiver_binding.as_ref() {
                self.sync_runtime_object_shadow_owner_static_metadata_from_binding(
                    identifier_name,
                    updated_receiver_binding,
                );
            }
        }
        Ok(())
    }

    fn emit_private_setter_receiver_brand_check(
        &mut self,
        object: &Expression,
        receiver_object: &Expression,
        property: &Expression,
        setter_binding: &LocalFunctionBinding,
    ) -> DirectResult<()> {
        if private_brand_marker_property_expression(property).is_some() {
            return self.emit_private_data_field_brand_check_after_base_or_throw(object, property);
        }

        let shadow_binding = self
            .resolve_runtime_object_property_shadow_binding(object, property)
            .or_else(|| {
                self.resolve_runtime_object_property_shadow_binding(receiver_object, property)
            });
        let deleted_binding = self
            .resolve_runtime_object_property_shadow_deleted_binding(object, property)
            .or_else(|| {
                self.resolve_runtime_object_property_shadow_deleted_binding(
                    receiver_object,
                    property,
                )
            });
        let fallback_value = self
            .resolve_object_binding_from_expression(object)
            .and_then(|object_binding| {
                self.resolve_object_binding_property_value(&object_binding, property)
            })
            .or_else(|| {
                self.resolve_object_binding_from_expression(receiver_object)
                    .and_then(|object_binding| {
                        self.resolve_object_binding_property_value(&object_binding, property)
                    })
            });

        let emit_match_or_throw = |compiler: &mut Self, value: &Expression| -> DirectResult<()> {
            let value_local = compiler.allocate_temp_local();
            if !compiler.emit_private_brand_marker_runtime_value(object, property, value)? {
                compiler.emit_numeric_expression(value)?;
            }
            compiler.push_local_set(value_local);
            compiler.emit_private_member_binding_match_from_local(setter_binding, value_local)?;
            compiler.state.emission.output.instructions.push(0x04);
            compiler
                .state
                .emission
                .output
                .instructions
                .push(EMPTY_BLOCK_TYPE);
            compiler.push_control_frame();
            compiler.state.emission.output.instructions.push(0x05);
            compiler.emit_private_data_field_brand_check_after_base_or_throw(object, property)?;
            compiler.state.emission.output.instructions.push(0x0b);
            compiler.pop_control_frame();
            Ok(())
        };

        let emit_shadow_or_fallback = |compiler: &mut Self| -> DirectResult<()> {
            if let Some(shadow_binding) = shadow_binding {
                compiler.push_global_get(shadow_binding.present_index);
                compiler.state.emission.output.instructions.push(0x04);
                compiler
                    .state
                    .emission
                    .output
                    .instructions
                    .push(EMPTY_BLOCK_TYPE);
                compiler.push_control_frame();
                let value_local = compiler.allocate_temp_local();
                compiler.push_global_get(shadow_binding.value_index);
                compiler.push_local_set(value_local);
                compiler
                    .emit_private_member_binding_match_from_local(setter_binding, value_local)?;
                compiler.state.emission.output.instructions.push(0x04);
                compiler
                    .state
                    .emission
                    .output
                    .instructions
                    .push(EMPTY_BLOCK_TYPE);
                compiler.push_control_frame();
                compiler.state.emission.output.instructions.push(0x05);
                compiler
                    .emit_private_data_field_brand_check_after_base_or_throw(object, property)?;
                compiler.state.emission.output.instructions.push(0x0b);
                compiler.pop_control_frame();
                compiler.state.emission.output.instructions.push(0x05);
                if let Some(fallback_value) = fallback_value.as_ref() {
                    emit_match_or_throw(compiler, fallback_value)?;
                } else {
                    compiler.emit_named_error_throw("TypeError")?;
                }
                compiler.state.emission.output.instructions.push(0x0b);
                compiler.pop_control_frame();
                return Ok(());
            }

            if let Some(fallback_value) = fallback_value.as_ref() {
                emit_match_or_throw(compiler, fallback_value)?;
                return Ok(());
            }

            compiler.emit_named_error_throw("TypeError")?;
            Ok(())
        };

        if let Some(deleted_binding) = deleted_binding {
            self.push_global_get(deleted_binding.present_index);
            self.state.emission.output.instructions.push(0x04);
            self.state
                .emission
                .output
                .instructions
                .push(EMPTY_BLOCK_TYPE);
            self.push_control_frame();
            self.emit_named_error_throw("TypeError")?;
            self.state.emission.output.instructions.push(0x05);
            emit_shadow_or_fallback(self)?;
            self.state.emission.output.instructions.push(0x0b);
            self.pop_control_frame();
            return Ok(());
        }

        emit_shadow_or_fallback(self)
    }

    pub(in crate::backend::direct_wasm) fn emit_setter_member_assignment(
        &mut self,
        object: &Expression,
        property: &Expression,
        value: &Expression,
    ) -> DirectResult<bool> {
        let Some(function_binding) = self.resolve_member_setter_binding(object, property) else {
            return Ok(false);
        };

        let receiver_hidden_name = self.allocate_named_hidden_local(
            "setter_receiver",
            self.infer_value_kind(object)
                .unwrap_or(StaticValueKind::Unknown),
        );
        let receiver_local = self
            .state
            .runtime
            .locals
            .get(&receiver_hidden_name)
            .copied()
            .expect("fresh setter receiver hidden local must exist");
        let value_hidden_name = self.allocate_named_hidden_local(
            "setter_value",
            self.infer_value_kind(value)
                .unwrap_or(StaticValueKind::Unknown),
        );
        let value_local = self
            .state
            .runtime
            .locals
            .get(&value_hidden_name)
            .copied()
            .expect("fresh setter value hidden local must exist");
        let value_references_internal_iterator_step =
            assign_member_expression_references_internal_iterator_step(value);
        let resolved_iterator_step_value = value_references_internal_iterator_step
            .then(|| self.resolve_static_iterator_step_assignment_value(value))
            .flatten();
        let metadata_value = resolved_iterator_step_value.as_ref().unwrap_or(value);
        self.emit_numeric_expression(object)?;
        self.push_local_set(receiver_local);
        self.emit_numeric_expression(value)?;
        self.push_local_set(value_local);
        let receiver_snapshot_expression = match object {
            Expression::This => {
                self.seed_local_this_object_binding();
                Expression::This
            }
            _ => object.clone(),
        };
        self.update_local_value_binding(&receiver_hidden_name, &receiver_snapshot_expression);
        self.update_local_object_binding(&receiver_hidden_name, &receiver_snapshot_expression);
        if value_references_internal_iterator_step && resolved_iterator_step_value.is_none() {
            self.update_local_value_binding(&value_hidden_name, value);
            self.state
                .speculation
                .static_semantics
                .set_local_kind(&value_hidden_name, StaticValueKind::Unknown);
        } else {
            self.update_capture_slot_binding_from_expression(&value_hidden_name, metadata_value)?;
        }
        let receiver_expression = Expression::Identifier(receiver_hidden_name.clone());
        let _receiver_shadow_owner =
            self.runtime_object_property_shadow_owner_name_for_identifier(&receiver_hidden_name);
        let object_aliases_this =
            self.resolve_bound_alias_expression(object)
                .is_some_and(|resolved| match resolved {
                    Expression::This => true,
                    Expression::Identifier(name) => name == "this",
                    _ => false,
                });
        let private_property = is_private_property_name_expression(
            &self.canonical_object_property_expression(property),
        );
        if private_property {
            let _ = self.resolve_runtime_object_property_shadow_binding(object, property);
            let _ = self.resolve_runtime_object_property_shadow_deleted_binding(object, property);
        }
        let can_commit_static_receiver_update = !private_property
            || self.private_setter_receiver_brand_statically_guaranteed(
                object,
                &receiver_expression,
                property,
                &function_binding,
            );
        self.prepare_setter_receiver_runtime_shadow_state(
            object,
            &receiver_hidden_name,
            &receiver_expression,
        )?;
        if private_property {
            self.emit_private_setter_receiver_brand_check(
                object,
                &receiver_expression,
                property,
                &function_binding,
            )?;
        }
        self.emit_property_key_expression_effects(property)?;
        if self.emit_function_binding_call_with_function_this_binding_from_argument_locals(
            &function_binding,
            &[value_local],
            1,
            &receiver_expression,
        )? {
            if let LocalFunctionBinding::User(function_name) = &function_binding
                && let Some(user_function) = self.user_function(function_name).cloned()
            {
                let names =
                    self.collect_user_function_call_effect_nonlocal_bindings(&user_function);
                if !names.is_empty() {
                    self.invalidate_static_binding_metadata_for_names(&names);
                }
            }
            self.state.emission.output.instructions.push(0x1a);
        }
        if !value_references_internal_iterator_step || resolved_iterator_step_value.is_some() {
            self.sync_simple_setter_nonlocal_assignment_metadata(
                &function_binding,
                metadata_value,
                &receiver_expression,
            )?;
        }
        if can_commit_static_receiver_update || private_property {
            match object {
                Expression::Identifier(name) => {
                    self.emit_setter_receiver_shadow_commit_to_identifier(
                        &receiver_hidden_name,
                        name,
                    )?;
                    if object_aliases_this {
                        self.emit_setter_receiver_shadow_commit_to_this(&receiver_hidden_name)?;
                    }
                }
                Expression::This => {
                    self.emit_setter_receiver_shadow_commit_to_this(&receiver_hidden_name)?;
                }
                _ => {}
            }
        }
        if can_commit_static_receiver_update || private_property {
            if let Some(updated_receiver) = self
                .state
                .speculation
                .static_semantics
                .last_bound_user_function_call
                .as_ref()
                .and_then(|snapshot| {
                    snapshot
                        .updated_bindings
                        .get(&receiver_hidden_name)
                        .cloned()
                })
            {
                if let Some(object_binding) =
                    self.resolve_object_binding_from_expression(&updated_receiver)
                {
                    if let Some(owner_name) = self
                        .runtime_object_property_shadow_owner_name_for_identifier(
                            &receiver_hidden_name,
                        )
                    {
                        self.sync_runtime_object_property_shadow_static_metadata_from_binding(
                            &owner_name,
                            &object_binding,
                        );
                    }
                    if matches!(object, Expression::This) {
                        if let Some(this_owner) =
                            self.runtime_object_property_shadow_owner_name_for_identifier("this")
                            && this_owner != "this"
                        {
                            self.sync_runtime_object_property_shadow_static_metadata_from_binding(
                                &this_owner,
                                &object_binding,
                            );
                        }
                        self.sync_runtime_object_property_shadow_static_metadata_from_binding(
                            "this",
                            &object_binding,
                        );
                    } else if object_aliases_this {
                        if let Expression::Identifier(name) = object
                            && let Some(identifier_owner) =
                                self.runtime_object_property_shadow_owner_name_for_identifier(name)
                            && identifier_owner != *name
                        {
                            self.sync_runtime_object_property_shadow_static_metadata_from_binding(
                                &identifier_owner,
                                &object_binding,
                            );
                        }
                        if let Some(this_owner) =
                            self.runtime_object_property_shadow_owner_name_for_identifier("this")
                            && this_owner != "this"
                        {
                            self.sync_runtime_object_property_shadow_static_metadata_from_binding(
                                &this_owner,
                                &object_binding,
                            );
                        }
                        self.sync_runtime_object_property_shadow_static_metadata_from_binding(
                            "this",
                            &object_binding,
                        );
                    }
                }
                match object {
                    Expression::Identifier(name) => {
                        self.update_local_value_binding(name, &updated_receiver);
                        self.update_local_object_binding(name, &updated_receiver);
                        if let Some(identifier_owner) =
                            self.runtime_object_property_shadow_owner_name_for_identifier(name)
                            && identifier_owner != *name
                        {
                            self.update_local_value_binding(&identifier_owner, &updated_receiver);
                            self.update_local_object_binding(&identifier_owner, &updated_receiver);
                        }
                        if object_aliases_this {
                            self.update_local_value_binding("this", &updated_receiver);
                            self.update_local_object_binding("this", &updated_receiver);
                        }
                    }
                    Expression::This => {
                        self.update_local_value_binding("this", &updated_receiver);
                        self.update_local_object_binding("this", &updated_receiver);
                    }
                    _ => {}
                }
            }
        }
        self.push_local_get(value_local);
        Ok(true)
    }
}
