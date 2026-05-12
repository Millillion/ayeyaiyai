use super::*;

impl<'a> FunctionCompiler<'a> {
    pub(in crate::backend::direct_wasm) fn capture_slot_member_source_key(
        object: &Expression,
        property: &Expression,
    ) -> Option<String> {
        let Expression::Identifier(object_name) = object else {
            return None;
        };
        let Expression::String(property_name) = property else {
            return None;
        };
        Some(format!("__ayy_member_source:{object_name}:{property_name}"))
    }

    pub(in crate::backend::direct_wasm) fn capture_slot_member_source_key_parts(
        source_key: &str,
    ) -> Option<(String, String)> {
        let rest = source_key.strip_prefix("__ayy_member_source:")?;
        let (object_name, property_name) = rest.split_once(':')?;
        Some((object_name.to_string(), property_name.to_string()))
    }

    pub(in crate::backend::direct_wasm) fn capture_slot_member_source_deleted_binding_name(
        slot_name: &str,
    ) -> String {
        format!("{slot_name}__member_source_deleted")
    }

    pub(in crate::backend::direct_wasm) fn emit_capture_source_expression_value(
        &mut self,
        capture_name: &str,
        source_expression: &Expression,
    ) -> DirectResult<()> {
        if let Expression::Member { object, property } = source_expression
            && matches!(property.as_ref(), Expression::String(name) if name == capture_name)
        {
            let is_active_with_scope_member = self
                .state
                .emission
                .lexical_scopes
                .with_scopes
                .iter()
                .any(|scope_object| static_expression_matches(scope_object, object.as_ref()));
            if is_active_with_scope_member
                && self.resolve_proxy_binding_from_expression(object).is_none()
            {
                let blocked = self.emit_with_scope_unscopables_block_check(object, capture_name)?;
                if blocked {
                    self.push_i32_const(JS_UNDEFINED_TAG);
                    return Ok(());
                }
            }
        }
        self.emit_numeric_expression(source_expression)
    }

    fn returned_member_call_receiver_expression(value: &Expression) -> Option<Expression> {
        let Expression::Call { callee, .. } = value else {
            return None;
        };
        let Expression::Member { object, .. } = callee.as_ref() else {
            return None;
        };
        Some((**object).clone())
    }

    fn resolve_existing_returned_member_capture_slot(
        &self,
        object_name: &str,
        capture_name: &str,
    ) -> Option<String> {
        let prefix = format!("__ayy_closure_slot_{object_name}_{capture_name}_");
        self.state
            .runtime
            .locals
            .bindings
            .keys()
            .find(|candidate| candidate.starts_with(&prefix))
            .cloned()
    }

    fn emit_fresh_private_brand_capture_slot_value(&mut self) -> DirectResult<()> {
        let brand_local = self.allocate_temp_local();
        self.push_global_get(NEXT_PRIVATE_BRAND_GLOBAL_INDEX);
        self.push_local_set(brand_local);
        self.push_local_get(brand_local);
        self.push_i32_const(1);
        self.push_binary_op(BinaryOp::Add)?;
        self.push_global_set(NEXT_PRIVATE_BRAND_GLOBAL_INDEX);
        self.push_local_get(brand_local);
        Ok(())
    }

    fn should_emit_fresh_private_brand_capture_slot_value(
        member_private_brand_binding: Option<&String>,
        capture_name: &str,
        source_expression: &Expression,
    ) -> bool {
        member_private_brand_binding.is_some_and(|brand_binding| brand_binding == capture_name)
            && (matches!(source_expression, Expression::Object(entries) if entries.is_empty())
                || matches!(source_expression, Expression::Identifier(name) if name == capture_name))
    }

    pub(in crate::backend::direct_wasm) fn update_capture_slot_binding_from_expression(
        &mut self,
        name: &str,
        value: &Expression,
    ) -> DirectResult<()> {
        self.state
            .speculation
            .static_semantics
            .capture_slot_initial_source_bindings
            .remove(name);
        let direct_source = self.direct_iterator_binding_source_expression(value);
        if matches!(value, Expression::Identifier(_))
            || matches!(direct_source, Some(Expression::Identifier(_)))
        {
            self.update_member_function_bindings_for_value(name, value, 0)?;
        }
        self.update_local_function_binding(name, value);
        self.update_local_specialized_function_value(name, value)?;
        self.update_local_proxy_binding(name, value);
        self.update_local_array_binding(name, value);
        self.update_local_resizable_array_buffer_binding(name, value)?;
        self.update_local_typed_array_view_binding(name, value)?;
        self.update_local_array_iterator_binding(name, value);
        self.update_local_iterator_step_binding(name, value);
        self.update_local_object_binding(name, value);
        self.update_local_arguments_binding(name, value);
        self.update_local_descriptor_binding(name, value);
        self.update_local_value_binding(name, value);
        self.update_object_prototype_binding_from_value(name, value);
        let value_kind = self
            .infer_value_kind(value)
            .unwrap_or(StaticValueKind::Unknown);
        self.state
            .speculation
            .static_semantics
            .set_local_kind(name, value_kind);
        Ok(())
    }

    pub(in crate::backend::direct_wasm) fn sync_capture_slot_runtime_object_shadows_from_expression(
        &mut self,
        name: &str,
        value: &Expression,
    ) -> DirectResult<()> {
        match value {
            Expression::Identifier(source_name) => {
                self.emit_runtime_object_property_shadow_copy(source_name, name)?;
            }
            Expression::This => {
                if let Some(owner_name) =
                    self.runtime_object_property_shadow_owner_name_for_identifier("this")
                {
                    self.emit_runtime_object_property_shadow_copy(&owner_name, name)?;
                } else if let Some(object_binding) =
                    self.resolve_object_binding_from_expression(value)
                {
                    let object_binding = self
                        .object_binding_with_constructed_constructor_shadow(object_binding, value);
                    self.emit_runtime_object_property_shadow_seed_from_binding(
                        name,
                        &object_binding,
                    )?;
                }
            }
            _ => {
                if let Some(object_binding) = self.resolve_object_binding_from_expression(value) {
                    let object_binding = self
                        .object_binding_with_constructed_constructor_shadow(object_binding, value);
                    self.emit_runtime_object_property_shadow_seed_from_binding(
                        name,
                        &object_binding,
                    )?;
                }
            }
        }
        Ok(())
    }

    pub(in crate::backend::direct_wasm) fn sync_closure_capture_slots_from_local_store(
        &mut self,
        resolved_name: &str,
        value_local: u32,
        value: &Expression,
    ) -> DirectResult<()> {
        let slot_names = self
            .state
            .speculation
            .static_semantics
            .capture_slot_source_bindings
            .iter()
            .filter_map(|(slot_name, source_name)| {
                (source_name == resolved_name).then(|| slot_name.clone())
            })
            .collect::<Vec<_>>();

        for slot_name in slot_names {
            let Some(slot_local) = self.state.runtime.locals.get(&slot_name).copied() else {
                continue;
            };
            self.push_local_get(value_local);
            self.push_local_set(slot_local);
            self.update_capture_slot_binding_from_expression(&slot_name, value)?;
            self.sync_capture_slot_runtime_object_shadows_from_expression(&slot_name, value)?;
            self.state
                .speculation
                .static_semantics
                .capture_slot_source_bindings
                .insert(slot_name, resolved_name.to_string());
        }

        Ok(())
    }

    pub(in crate::backend::direct_wasm) fn sync_closure_capture_slots_from_member_store(
        &mut self,
        object: &Expression,
        property: &Expression,
        value_local: u32,
        value: &Expression,
    ) -> DirectResult<()> {
        let Some(source_key) = Self::capture_slot_member_source_key(object, property) else {
            return Ok(());
        };
        let slot_names = self
            .state
            .speculation
            .static_semantics
            .capture_slot_source_bindings
            .iter()
            .filter_map(|(slot_name, source_name)| {
                (source_name == &source_key).then(|| slot_name.clone())
            })
            .collect::<Vec<_>>();

        for slot_name in slot_names {
            if let Some(slot_local) = self.state.runtime.locals.get(&slot_name).copied() {
                self.push_local_get(value_local);
                self.push_local_set(slot_local);
                self.update_capture_slot_binding_from_expression(&slot_name, value)?;
                self.sync_capture_slot_runtime_object_shadows_from_expression(&slot_name, value)?;
                self.state
                    .speculation
                    .static_semantics
                    .capture_slot_source_bindings
                    .insert(slot_name, source_key.clone());
                continue;
            }

            if let Some(hidden_binding) = self.hidden_implicit_global_binding(&slot_name) {
                self.push_local_get(value_local);
                self.push_global_set(hidden_binding.value_index);
                self.push_i32_const(1);
                self.push_global_set(hidden_binding.present_index);
                self.update_static_global_assignment_metadata(&slot_name, value);
                self.update_global_property_descriptor_value(&slot_name, value);
                self.sync_capture_slot_runtime_object_shadows_from_expression(&slot_name, value)?;
                self.state
                    .speculation
                    .static_semantics
                    .capture_slot_source_bindings
                    .insert(slot_name, source_key.clone());
            }
        }

        Ok(())
    }

    pub(in crate::backend::direct_wasm) fn initialize_returned_member_capture_slots_for_bindings(
        &mut self,
        name: &str,
        value: &Expression,
        value_local: u32,
        bindings: &[ReturnedMemberFunctionBinding],
    ) -> DirectResult<HashMap<String, BTreeMap<String, String>>> {
        let Some((user_function, arguments)) = self.resolve_user_function_call_target(value) else {
            return Ok(HashMap::new());
        };
        if bindings.is_empty() {
            return Ok(HashMap::new());
        }
        let Some(function) = self
            .resolve_registered_function_declaration(&user_function.name)
            .cloned()
        else {
            return Ok(HashMap::new());
        };
        let call_snapshot_bindings = self
            .state
            .speculation
            .static_semantics
            .last_bound_user_function_call
            .as_ref()
            .filter(|snapshot| snapshot.function_name == user_function.name)
            .map(|snapshot| snapshot.updated_bindings.clone());
        let returned_identifier = collect_returned_identifier(&function.body);
        let local_aliases = collect_returned_member_local_aliases(&function.body);
        let mut function_local_bindings =
            collect_declared_bindings_from_statements_recursive(&function.body);
        function_local_bindings.extend(
            function
                .params
                .iter()
                .map(|parameter| parameter.name.clone()),
        );
        let trace_inherited_bindings = std::env::var_os("AYY_TRACE_INHERITED_BINDINGS").is_some();
        let mut initialized_slots: BTreeMap<String, String> = BTreeMap::new();
        let mut property_slots: HashMap<String, BTreeMap<String, String>> = HashMap::new();
        let returned_object_binding = self.resolve_object_binding_from_expression(value);
        let returned_call_receiver = Self::returned_member_call_receiver_expression(value);
        let value_creates_fresh_returned_member =
            matches!(value, Expression::Call { .. } | Expression::New { .. });

        for binding in bindings {
            let LocalFunctionBinding::User(member_function_name) = &binding.binding else {
                continue;
            };
            let property_expression = Expression::String(binding.property.clone());
            if !value_creates_fresh_returned_member
                && let Some(existing_slots) = self
                    .resolve_member_function_capture_slots(value, &property_expression)
                    .or_else(|| match value {
                        Expression::New { callee, .. } => {
                            let Expression::Identifier(callee_name) = callee.as_ref() else {
                                return None;
                            };
                            self.resolve_member_function_capture_slots(
                                &Expression::Identifier(callee_name.clone()),
                                &property_expression,
                            )
                        }
                        _ => None,
                    })
            {
                if !existing_slots
                    .values()
                    .any(|slot_name| slot_name.starts_with("__ayy_capture_binding__"))
                {
                    property_slots.insert(binding.property.clone(), existing_slots);
                    continue;
                }
            }
            let member_private_brand_binding = self
                .user_function(member_function_name)
                .and_then(|function| function.private_brand_binding.clone());
            let capture_bindings = if let Some(captures) = self
                .backend
                .function_registry
                .analysis
                .user_function_capture_bindings
                .get(member_function_name)
                .filter(|captures| !captures.is_empty())
                .cloned()
            {
                captures
            } else if let Some(returned_expression) = self
                .resolve_function_binding_static_return_expression_with_call_frame(
                    &binding.binding,
                    &[],
                    &returned_identifier
                        .as_ref()
                        .map(|name| Expression::Identifier(name.clone()))
                        .unwrap_or(Expression::Undefined),
                )
                && let Some(LocalFunctionBinding::User(returned_function_name)) =
                    self.resolve_function_binding_from_expression(&returned_expression)
                && let Some(captures) = self
                    .backend
                    .function_registry
                    .analysis
                    .user_function_capture_bindings
                    .get(&returned_function_name)
                    .filter(|captures| !captures.is_empty())
                    .cloned()
            {
                captures
            } else {
                continue;
            };
            if trace_inherited_bindings {
                eprintln!(
                    "returned_member_capture_slots:value={value:?} member={member_function_name} captures={capture_bindings:?} aliases={local_aliases:?} receiver={returned_call_receiver:?}"
                );
            }
            let mut capture_slots = BTreeMap::new();
            for capture_name in capture_bindings.keys() {
                let slot_name = if let Some(existing) = initialized_slots.get(capture_name) {
                    existing.clone()
                } else if !value_creates_fresh_returned_member
                    && let Some(existing) =
                        self.resolve_existing_returned_member_capture_slot(name, capture_name)
                {
                    initialized_slots.insert(capture_name.clone(), existing.clone());
                    existing
                } else {
                    let (source_expression, source_uses_value_local) = if returned_identifier
                        .as_ref()
                        .is_some_and(|returned_identifier| capture_name == returned_identifier)
                        || returned_identifier
                            .as_ref()
                            .is_some_and(|returned_identifier| {
                                scoped_binding_source_name(returned_identifier)
                                    .is_some_and(|source_name| source_name == capture_name)
                            }) {
                        (Expression::Identifier(name.to_string()), true)
                    } else if capture_name == "new.target"
                        && let Expression::New { callee, .. } = value
                    {
                        (callee.as_ref().clone(), false)
                    } else if let Some(alias) = local_aliases.get(capture_name) {
                        let substituted = self.substitute_user_function_argument_bindings(
                            alias,
                            &user_function,
                            &arguments,
                        );
                        (
                            if matches!(substituted, Expression::This) {
                                returned_call_receiver.clone().unwrap_or(substituted)
                            } else {
                                substituted
                            },
                            false,
                        )
                    } else if let Some(param_name) = user_function.params.iter().find(|param| {
                        *param == capture_name
                            || scoped_binding_source_name(param)
                                .is_some_and(|source_name| source_name == capture_name)
                    }) {
                        (
                            self.substitute_user_function_argument_bindings(
                                &Expression::Identifier(param_name.clone()),
                                &user_function,
                                &arguments,
                            ),
                            false,
                        )
                    } else if let Some(snapshot_value) =
                        call_snapshot_bindings.as_ref().and_then(|bindings| {
                            bindings.get(capture_name).or_else(|| {
                                scoped_binding_source_name(capture_name)
                                    .and_then(|source_name| bindings.get(source_name))
                            })
                        })
                    {
                        let source_binding_name = scoped_binding_source_name(capture_name)
                            .unwrap_or(capture_name)
                            .to_string();
                        if capture_name == "this" && matches!(value, Expression::New { .. }) {
                            (Expression::Identifier(name.to_string()), true)
                        } else {
                            (
                                if self.user_function_capture_source_is_locally_bound(
                                    &source_binding_name,
                                ) {
                                    Expression::Identifier(source_binding_name)
                                } else {
                                    snapshot_value.clone()
                                },
                                false,
                            )
                        }
                    } else if capture_name == "this" && matches!(value, Expression::New { .. }) {
                        (Expression::Identifier(name.to_string()), true)
                    } else if member_private_brand_binding
                        .as_ref()
                        .is_some_and(|brand_binding| brand_binding == capture_name)
                        && let Some(brand_value) =
                            returned_object_binding.as_ref().and_then(|object_binding| {
                                ordered_object_property_names(object_binding)
                                    .into_iter()
                                    .find(|property_name| {
                                        property_name.starts_with("__ayy$private$")
                                    })
                                    .and_then(|property_name| {
                                        object_binding_lookup_value(
                                            object_binding,
                                            &Expression::String(property_name),
                                        )
                                        .cloned()
                                    })
                            })
                    {
                        (brand_value, false)
                    } else {
                        (Expression::Identifier(capture_name.clone()), false)
                    };
                    if trace_inherited_bindings {
                        eprintln!(
                            "returned_member_capture_slot:init property={} capture={} source={source_expression:?}",
                            binding.property, capture_name
                        );
                    }
                    let hidden_kind = self
                        .infer_value_kind(&source_expression)
                        .unwrap_or(StaticValueKind::Unknown);
                    let use_global_capture_slot = self.binding_name_is_global(name)
                        || self.global_has_binding(name)
                        || self.backend.global_has_lexical_binding(name)
                        || self.global_has_implicit_binding(name);
                    let hidden_name = if use_global_capture_slot {
                        let hidden_name = format!("__ayy_closure_slot_{}_{}", name, capture_name);
                        let hidden_binding = self.ensure_implicit_global_binding(&hidden_name);
                        if source_uses_value_local {
                            self.push_local_get(value_local);
                        } else if Self::should_emit_fresh_private_brand_capture_slot_value(
                            member_private_brand_binding.as_ref(),
                            capture_name,
                            &source_expression,
                        ) {
                            self.emit_fresh_private_brand_capture_slot_value()?;
                        } else {
                            self.emit_numeric_expression(&source_expression)?;
                        }
                        self.push_global_set(hidden_binding.value_index);
                        self.push_i32_const(1);
                        self.push_global_set(hidden_binding.present_index);
                        if !capture_name.starts_with("__ayy_class_brand_") {
                            self.update_static_global_assignment_metadata(
                                &hidden_name,
                                &source_expression,
                            );
                        }
                        self.sync_capture_slot_runtime_object_shadows_from_expression(
                            &hidden_name,
                            &source_expression,
                        )?;
                        hidden_name
                    } else {
                        let hidden_name = self.allocate_named_hidden_local(
                            &format!("closure_slot_{}_{}", name, capture_name),
                            hidden_kind,
                        );
                        let hidden_local = self
                            .state
                            .runtime
                            .locals
                            .get(&hidden_name)
                            .copied()
                            .expect("fresh closure capture slot local must exist");
                        if source_uses_value_local {
                            self.push_local_get(value_local);
                        } else if Self::should_emit_fresh_private_brand_capture_slot_value(
                            member_private_brand_binding.as_ref(),
                            capture_name,
                            &source_expression,
                        ) {
                            self.emit_fresh_private_brand_capture_slot_value()?;
                        } else {
                            self.emit_numeric_expression(&source_expression)?;
                        }
                        self.push_local_set(hidden_local);
                        self.update_capture_slot_binding_from_expression(
                            &hidden_name,
                            &source_expression,
                        )?;
                        self.sync_capture_slot_runtime_object_shadows_from_expression(
                            &hidden_name,
                            &source_expression,
                        )?;
                        hidden_name
                    };
                    if let Expression::Identifier(source_binding_name) = &source_expression
                        && (!function_local_bindings.contains(capture_name)
                            || source_binding_name == capture_name)
                    {
                        self.state
                            .speculation
                            .static_semantics
                            .capture_slot_source_bindings
                            .insert(hidden_name.clone(), source_binding_name.clone());
                    }
                    if let Expression::Identifier(source_binding_name) = &source_expression {
                        self.state
                            .speculation
                            .static_semantics
                            .capture_slot_initial_source_bindings
                            .insert(hidden_name.clone(), source_binding_name.clone());
                    }
                    initialized_slots.insert(capture_name.clone(), hidden_name.clone());
                    hidden_name
                };
                capture_slots.insert(capture_name.clone(), slot_name);
            }
            if !capture_slots.is_empty() {
                property_slots.insert(binding.property.clone(), capture_slots);
            }
        }

        Ok(property_slots)
    }

    pub(in crate::backend::direct_wasm) fn resolve_returned_member_capture_bindings_for_value(
        &self,
        value: &Expression,
    ) -> Option<HashMap<String, HashMap<String, Expression>>> {
        let (user_function, arguments) = self.resolve_user_function_call_target(value)?;
        if user_function.returned_member_function_bindings.is_empty() {
            return None;
        }
        let function = self
            .resolve_registered_function_declaration(&user_function.name)?
            .clone();
        let returned_identifier = collect_returned_identifier(&function.body)?;
        let local_aliases = collect_returned_member_local_aliases(&function.body);
        let mut property_bindings = HashMap::new();
        let returned_object_binding = self.resolve_object_binding_from_expression(value);
        let returned_call_receiver = Self::returned_member_call_receiver_expression(value);

        for binding in &user_function.returned_member_function_bindings {
            let LocalFunctionBinding::User(member_function_name) = &binding.binding else {
                continue;
            };
            let member_private_brand_binding = self
                .user_function(member_function_name)
                .and_then(|function| function.private_brand_binding.clone());
            let Some(captures) = self
                .backend
                .function_registry
                .analysis
                .user_function_capture_bindings
                .get(member_function_name)
            else {
                property_bindings.insert(binding.property.clone(), HashMap::new());
                continue;
            };
            let mut capture_bindings = HashMap::new();
            for capture_name in captures.keys() {
                let source_expression = if capture_name == &returned_identifier {
                    value.clone()
                } else if let Some(alias) = local_aliases.get(capture_name) {
                    let substituted = self.substitute_user_function_argument_bindings(
                        alias,
                        &user_function,
                        &arguments,
                    );
                    if matches!(substituted, Expression::This) {
                        returned_call_receiver.clone().unwrap_or(substituted)
                    } else {
                        substituted
                    }
                } else if let Some(param_name) = user_function.params.iter().find(|param| {
                    *param == capture_name
                        || scoped_binding_source_name(param)
                            .is_some_and(|source_name| source_name == capture_name)
                }) {
                    self.substitute_user_function_argument_bindings(
                        &Expression::Identifier(param_name.clone()),
                        &user_function,
                        &arguments,
                    )
                } else if member_private_brand_binding
                    .as_ref()
                    .is_some_and(|brand_binding| brand_binding == capture_name)
                    && let Some(brand_value) =
                        returned_object_binding.as_ref().and_then(|object_binding| {
                            ordered_object_property_names(object_binding)
                                .into_iter()
                                .find(|property_name| property_name.starts_with("__ayy$private$"))
                                .and_then(|property_name| {
                                    object_binding_lookup_value(
                                        object_binding,
                                        &Expression::String(property_name),
                                    )
                                    .cloned()
                                })
                        })
                {
                    brand_value
                } else {
                    Expression::Identifier(capture_name.clone())
                };
                capture_bindings.insert(capture_name.clone(), source_expression);
            }
            property_bindings.insert(binding.property.clone(), capture_bindings);
        }

        Some(property_bindings)
    }
}
