use super::*;

const NULL_SUPER_CONSTRUCTOR_BINDING: &str = "__ayy_null_super_constructor";

impl<'a> FunctionCompiler<'a> {
    fn emit_user_function_capture_slot_source_value(
        &mut self,
        capture_name: &str,
        source_expression: &Expression,
    ) -> DirectResult<()> {
        if capture_name == "this" && matches!(source_expression, Expression::This) {
            self.push_global_get(CURRENT_THIS_GLOBAL_INDEX);
            return Ok(());
        }
        self.emit_numeric_expression(source_expression)
    }

    fn null_super_constructor_statement_arguments<'b>(
        statement: &'b Statement,
    ) -> Option<&'b [CallArgument]> {
        match statement {
            Statement::Expression(Expression::SuperCall { callee, arguments })
            | Statement::Var {
                value: Expression::SuperCall { callee, arguments },
                ..
            }
            | Statement::Let {
                value: Expression::SuperCall { callee, arguments },
                ..
            }
            | Statement::Assign {
                value: Expression::SuperCall { callee, arguments },
                ..
            }
            | Statement::Return(Expression::SuperCall { callee, arguments }) => {
                if matches!(callee.as_ref(), Expression::Identifier(name) if name == NULL_SUPER_CONSTRUCTOR_BINDING)
                {
                    Some(arguments.as_slice())
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn emit_null_super_constructor_construct(
        &mut self,
        user_function: &UserFunction,
    ) -> DirectResult<bool> {
        let Some(function) = self.resolve_registered_function_declaration(&user_function.name)
        else {
            return Ok(false);
        };
        let Some((super_index, super_arguments)) =
            function
                .body
                .iter()
                .enumerate()
                .find_map(|(index, statement)| {
                    Self::null_super_constructor_statement_arguments(statement)
                        .map(|arguments| (index, arguments.to_vec()))
                })
        else {
            return Ok(false);
        };
        let prefix = function.body[..super_index].to_vec();
        let saved_new_target_local = self.allocate_temp_local();
        self.push_global_get(CURRENT_NEW_TARGET_GLOBAL_INDEX);
        self.push_local_set(saved_new_target_local);
        let saved_this_local = self.allocate_temp_local();
        self.push_global_get(CURRENT_THIS_GLOBAL_INDEX);
        self.push_local_set(saved_this_local);
        self.push_i32_const(user_function_runtime_value(user_function));
        self.push_global_set(CURRENT_NEW_TARGET_GLOBAL_INDEX);
        self.push_i32_const(JS_UNDEFINED_TAG);
        self.push_global_set(CURRENT_THIS_GLOBAL_INDEX);
        for statement in &prefix {
            self.emit_statement(statement)?;
        }
        self.push_local_get(saved_new_target_local);
        self.push_global_set(CURRENT_NEW_TARGET_GLOBAL_INDEX);
        self.push_local_get(saved_this_local);
        self.push_global_set(CURRENT_THIS_GLOBAL_INDEX);
        self.emit_null_super_constructor_call(&super_arguments)?;
        Ok(true)
    }

    fn default_derived_constructor_builtin_super_name(
        &self,
        callee: &Expression,
        user_function: &UserFunction,
    ) -> Option<String> {
        if !self.user_function_is_derived_constructor(user_function) {
            return None;
        }
        let declaration = self.resolve_registered_function_declaration(&user_function.name)?;
        let [rest_parameter] = declaration.params.as_slice() else {
            return None;
        };
        if !rest_parameter.rest {
            return None;
        }
        let (super_callee, super_arguments) =
            self.resolve_derived_constructor_super_call(user_function)?;
        let [CallArgument::Spread(Expression::Identifier(spread_name))] = super_arguments else {
            return None;
        };
        if spread_name != &rest_parameter.name {
            return None;
        }

        let resolved_super = match super_callee {
            Expression::Identifier(name) => self
                .resolve_constructor_capture_source_bindings_from_expression(callee)
                .and_then(|bindings| bindings.get(name).cloned())
                .unwrap_or_else(|| super_callee.clone()),
            _ => super_callee.clone(),
        };
        let LocalFunctionBinding::Builtin(function_name) =
            self.resolve_function_binding_from_expression(&resolved_super)?
        else {
            return None;
        };
        Some(function_name)
    }

    fn emit_default_derived_builtin_construct(
        &mut self,
        callee: &Expression,
        user_function: &UserFunction,
        arguments: &[CallArgument],
    ) -> DirectResult<bool> {
        let Some(function_name) =
            self.default_derived_constructor_builtin_super_name(callee, user_function)
        else {
            return Ok(false);
        };
        if std::env::var_os("AYY_TRACE_CONSTRUCT_CALLS").is_some() {
            eprintln!(
                "construct_call:default_derived_builtin callee={callee:?} super={function_name}"
            );
        }
        self.state
            .speculation
            .static_semantics
            .last_bound_user_function_call = Some(BoundUserFunctionCallSnapshot {
            function_name: user_function.name.clone(),
            source_expression: Some(Expression::New {
                callee: Box::new(callee.clone()),
                arguments: arguments.to_vec(),
            }),
            result_expression: Some(Expression::New {
                callee: Box::new(Expression::Identifier(function_name.clone())),
                arguments: arguments.to_vec(),
            }),
            prototype_source_expression: None,
            updated_bindings: self
                .resolve_constructor_capture_source_bindings_from_expression(callee)
                .unwrap_or_default(),
        });
        self.emit_builtin_call_for_callee(callee, &function_name, arguments, true)
    }

    fn prepare_constructor_runtime_argument_bindings(
        &mut self,
        arguments: &[CallArgument],
    ) -> DirectResult<(Vec<CallArgument>, Vec<(String, String)>)> {
        let expanded_arguments = self.expand_call_arguments(arguments);
        let mut runtime_arguments = Vec::new();
        let mut shadow_writebacks = Vec::new();

        for (index, argument) in expanded_arguments.iter().enumerate() {
            let argument_needs_runtime_metadata = self
                .expression_depends_on_active_loop_assignment(argument)
                || self.expression_has_dynamic_member_property_access(argument);
            let argument_kind = if argument_needs_runtime_metadata {
                StaticValueKind::Unknown
            } else {
                self.infer_value_kind(argument)
                    .unwrap_or(StaticValueKind::Unknown)
            };
            let hidden_name =
                self.allocate_named_hidden_local(&format!("construct_arg_{index}"), argument_kind);
            let hidden_local = self
                .state
                .runtime
                .locals
                .get(&hidden_name)
                .copied()
                .expect("fresh constructor argument local must exist");
            let source_owner = match argument {
                Expression::Identifier(name) => {
                    self.runtime_object_property_shadow_owner_name_for_identifier(name)
                }
                Expression::This => {
                    self.runtime_object_property_shadow_owner_name_for_identifier("this")
                }
                _ => None,
            };

            self.emit_numeric_expression(argument)?;
            self.push_local_set(hidden_local);
            if !argument_needs_runtime_metadata {
                self.update_capture_slot_binding_from_expression(&hidden_name, argument)?;
                self.sync_capture_slot_runtime_object_shadows_from_expression(
                    &hidden_name,
                    argument,
                )?;
            }

            if let Some(source_owner) = source_owner
                && source_owner != hidden_name
            {
                shadow_writebacks.push((hidden_name.clone(), source_owner));
            }

            runtime_arguments.push(CallArgument::Expression(Expression::Identifier(
                hidden_name,
            )));
        }

        Ok((runtime_arguments, shadow_writebacks))
    }

    pub(in crate::backend::direct_wasm) fn initialize_user_function_capture_slots_from_expression(
        &mut self,
        expression: &Expression,
        user_function: &UserFunction,
    ) -> DirectResult<Option<BTreeMap<String, String>>> {
        if user_function.lexical_this
            && let Some((target, _, LocalFunctionBinding::User(function_name))) =
                self.resolve_function_prototype_bind_call(expression, self.current_function_name())
            && function_name == user_function.name
            && let Some(capture_slots) = self.resolve_function_expression_capture_slots(&target)
        {
            return Ok(Some(capture_slots));
        }
        if let Some(capture_slots) = self.resolve_function_expression_capture_slots(expression) {
            return Ok(Some(capture_slots));
        }
        let mut capture_bindings = self
            .backend
            .function_registry
            .analysis
            .user_function_capture_bindings
            .get(&user_function.name)
            .cloned()
            .unwrap_or_default();
        self.add_active_with_scope_function_capture_bindings(
            &user_function.name,
            &mut capture_bindings,
        )?;
        if capture_bindings.is_empty() {
            return Ok(None);
        }
        let Some(capture_source_bindings) =
            self.resolve_constructor_capture_source_bindings_from_expression(expression)
        else {
            return Ok(None);
        };

        let mut capture_slots = BTreeMap::new();
        for capture_name in capture_bindings.keys() {
            let scoped_source_object = self.resolve_with_scope_binding(capture_name)?;
            if !self.user_function_capture_source_is_locally_bound(capture_name)
                && scoped_source_object.is_none()
            {
                continue;
            }
            let source_expression = if let Some(scope_object) = scoped_source_object.as_ref() {
                Expression::Member {
                    object: Box::new(scope_object.clone()),
                    property: Box::new(Expression::String(capture_name.clone())),
                }
            } else {
                let Some(source_expression) = capture_source_bindings.get(capture_name).cloned()
                else {
                    return Ok(None);
                };
                source_expression
            };
            let source_expression = if matches!(
                &source_expression,
                Expression::Identifier(name) if name == capture_name
            ) {
                self.resolve_user_function_capture_hidden_name(capture_name)
                    .map(Expression::Identifier)
                    .unwrap_or(source_expression)
            } else {
                source_expression
            };
            let hidden_name = self.allocate_named_hidden_local(
                &format!("closure_slot_{}_{}", user_function.name, capture_name),
                self.infer_value_kind(&source_expression)
                    .unwrap_or(StaticValueKind::Unknown),
            );
            let hidden_local = self
                .state
                .runtime
                .locals
                .get(&hidden_name)
                .copied()
                .expect("fresh returned function capture slot local must exist");
            self.emit_user_function_capture_slot_source_value(capture_name, &source_expression)?;
            self.push_local_set(hidden_local);
            self.update_capture_slot_binding_from_expression(&hidden_name, &source_expression)?;
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
            } else if matches!(source_expression, Expression::This | Expression::NewTarget) {
                self.state
                    .speculation
                    .static_semantics
                    .capture_slot_source_bindings
                    .insert(hidden_name.clone(), capture_name.clone());
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
        }

        if capture_slots.is_empty() {
            return Ok(None);
        }
        Ok(Some(capture_slots))
    }

    pub(in crate::backend::direct_wasm) fn emit_user_function_construct(
        &mut self,
        callee: &Expression,
        user_function: &UserFunction,
        arguments: &[CallArgument],
    ) -> DirectResult<bool> {
        if !user_function.is_constructible() {
            return Ok(false);
        }
        if self.emit_null_super_constructor_construct(user_function)? {
            return Ok(true);
        }
        if self.emit_default_derived_builtin_construct(callee, user_function, arguments)? {
            return Ok(true);
        }

        let ordinary_this_expression = Expression::Object(Vec::new());
        let construct_this_expression = if self.user_function_is_derived_constructor(user_function)
        {
            &Expression::Undefined
        } else {
            &ordinary_this_expression
        };
        let capture_slots =
            self.initialize_user_function_capture_slots_from_expression(callee, user_function)?;
        let capture_source_bindings =
            self.resolve_constructor_capture_source_bindings_from_expression(callee);
        let constructor_ordinary_direct_eval = self
            .user_function_mentions_direct_eval(user_function)
            && !self
                .resolve_registered_function_declaration(&user_function.name)
                .is_some_and(|declaration| declaration.direct_eval_in_class_field_initializer);
        let constructor_static_resolution_allowed = arguments.iter().all(|argument| {
            let expression = match argument {
                CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                    expression
                }
            };
            !self.expression_depends_on_active_loop_assignment(expression)
                && !self.expression_has_dynamic_member_property_access(expression)
        });
        let constructor_result_outcome = (!constructor_ordinary_direct_eval
            && constructor_static_resolution_allowed)
            .then(|| {
                self.resolve_user_constructor_object_binding_outcome_for_function(
                    user_function,
                    arguments,
                    capture_source_bindings.as_ref(),
                )
            })
            .flatten();
        if let Some(Err(throw_value)) = constructor_result_outcome.as_ref() {
            self.emit_static_throw_value(throw_value)?;
            return Ok(true);
        }
        let constructor_return_resolution = (!constructor_ordinary_direct_eval
            && constructor_static_resolution_allowed)
            .then(|| {
                self.resolve_user_constructor_return_expression_with_explicit_status_for_function(
                    user_function,
                    arguments,
                    capture_source_bindings.as_ref(),
                )
            })
            .flatten()
            .filter(|(expression, _)| {
                self.resolve_object_binding_from_expression(expression)
                    .is_some()
                    || self
                        .resolve_array_binding_from_expression(expression)
                        .is_some()
                    || self
                        .resolve_function_binding_from_expression(expression)
                        .is_some()
            });
        let constructor_return_expression = constructor_return_resolution
            .as_ref()
            .map(|(expression, _)| expression.clone());
        let constructor_source_expression = Expression::New {
            callee: Box::new(callee.clone()),
            arguments: arguments.to_vec(),
        };
        let constructor_prototype_source_expression = constructor_return_resolution
            .as_ref()
            .and_then(|(expression, explicit)| {
                if !explicit {
                    return None;
                }
                if matches!(expression, Expression::This)
                    || matches!(
                        expression,
                        Expression::Identifier(name) if name == Self::STATIC_NEW_THIS_BINDING
                    )
                {
                    return Some(constructor_source_expression.clone());
                }
                Some(expression.clone())
            });
        let constructor_result_expression = constructor_return_expression.clone().or_else(|| {
            constructor_result_outcome
                .as_ref()
                .and_then(|outcome| outcome.as_ref().ok())
                .map(|binding| object_binding_to_expression(binding))
        });
        let constructor_updated_bindings = (!constructor_ordinary_direct_eval
            && constructor_static_resolution_allowed)
            .then(|| {
                self.resolve_user_constructor_updated_bindings_for_function(
                    user_function,
                    arguments,
                    capture_source_bindings.as_ref(),
                )
            })
            .flatten();
        let constructor_updated_bindings_for_sync = constructor_updated_bindings.clone();
        self.state
            .speculation
            .static_semantics
            .last_bound_user_function_call = Some(BoundUserFunctionCallSnapshot {
            function_name: user_function.name.clone(),
            source_expression: Some(constructor_source_expression),
            result_expression: constructor_result_expression,
            prototype_source_expression: constructor_prototype_source_expression,
            updated_bindings: constructor_updated_bindings
                .or_else(|| capture_source_bindings.clone())
                .unwrap_or_default(),
        });

        let (runtime_arguments, argument_shadow_writebacks) =
            self.prepare_constructor_runtime_argument_bindings(arguments)?;
        if let Some(capture_slots) = capture_slots.as_ref() {
            self.emit_user_function_call_with_new_target_and_this_expression_and_bound_captures_without_static_snapshot(
                user_function,
                &runtime_arguments,
                user_function_runtime_value(user_function),
                construct_this_expression,
                capture_slots,
            )?;
        } else {
            self.emit_user_function_call_with_new_target_and_this_expression_without_static_snapshot(
                user_function,
                &runtime_arguments,
                user_function_runtime_value(user_function),
                construct_this_expression,
            )?;
        }
        let constructor_return_local = self.allocate_temp_local();
        self.push_local_set(constructor_return_local);
        for (hidden_name, source_owner) in argument_shadow_writebacks {
            self.emit_runtime_object_property_shadow_copy(&hidden_name, &source_owner)?;
        }
        if let Some(updated_bindings) = constructor_updated_bindings_for_sync.as_ref() {
            let mut updated_names =
                self.collect_user_function_call_effect_nonlocal_bindings(user_function);
            updated_names.extend(
                self.collect_snapshot_updated_nonlocal_bindings(
                    user_function,
                    Some(updated_bindings),
                ),
            );
            let unresolved = self.sync_snapshot_user_function_call_effect_bindings(
                &updated_names,
                Some(updated_bindings),
                None,
            )?;
            if !unresolved.is_empty() {
                let preserved_kinds = unresolved
                    .iter()
                    .filter_map(|name| {
                        self.lookup_identifier_kind(name)
                            .map(|kind| (name.clone(), kind))
                    })
                    .collect::<HashMap<_, _>>();
                self.invalidate_static_binding_metadata_for_names_with_preserved_kinds(
                    &unresolved,
                    &preserved_kinds,
                );
            }
        }
        self.push_local_get(constructor_return_local);
        self.state.emission.output.instructions.push(0x1a);
        self.push_i32_const(JS_TYPEOF_OBJECT_TAG);
        Ok(true)
    }
}
