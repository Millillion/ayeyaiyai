use super::*;

impl<'a> FunctionCompiler<'a> {
    fn collect_lexical_this_member_assignment_properties_from_expression(
        &self,
        expression: &Expression,
        properties: &mut BTreeMap<String, Expression>,
    ) {
        match expression {
            Expression::AssignMember {
                object,
                property,
                value,
            } => {
                if matches!(object.as_ref(), Expression::This) {
                    let property = self.canonical_object_property_expression(property);
                    if let Some(property_name) = static_property_name_from_expression(&property) {
                        properties.insert(property_name, property);
                    }
                }
                self.collect_lexical_this_member_assignment_properties_from_expression(
                    object, properties,
                );
                self.collect_lexical_this_member_assignment_properties_from_expression(
                    property, properties,
                );
                self.collect_lexical_this_member_assignment_properties_from_expression(
                    value, properties,
                );
            }
            Expression::AssignSuperMember { property, value } => {
                let property = self.canonical_object_property_expression(property);
                if let Some(property_name) = static_property_name_from_expression(&property) {
                    properties.insert(property_name, property);
                }
                self.collect_lexical_this_member_assignment_properties_from_expression(
                    value, properties,
                );
            }
            Expression::Array(elements) => {
                for element in elements {
                    match element {
                        ArrayElement::Expression(value) | ArrayElement::Spread(value) => self
                            .collect_lexical_this_member_assignment_properties_from_expression(
                                value, properties,
                            ),
                    }
                }
            }
            Expression::Object(entries) => {
                for entry in entries {
                    match entry {
                        ObjectEntry::Data { key, value } => {
                            self.collect_lexical_this_member_assignment_properties_from_expression(
                                key, properties,
                            );
                            self.collect_lexical_this_member_assignment_properties_from_expression(
                                value, properties,
                            );
                        }
                        ObjectEntry::Getter { key, getter } => {
                            self.collect_lexical_this_member_assignment_properties_from_expression(
                                key, properties,
                            );
                            self.collect_lexical_this_member_assignment_properties_from_expression(
                                getter, properties,
                            );
                        }
                        ObjectEntry::Setter { key, setter } => {
                            self.collect_lexical_this_member_assignment_properties_from_expression(
                                key, properties,
                            );
                            self.collect_lexical_this_member_assignment_properties_from_expression(
                                setter, properties,
                            );
                        }
                        ObjectEntry::Spread(value) => self
                            .collect_lexical_this_member_assignment_properties_from_expression(
                                value, properties,
                            ),
                    }
                }
            }
            Expression::Member { object, property } => {
                self.collect_lexical_this_member_assignment_properties_from_expression(
                    object, properties,
                );
                self.collect_lexical_this_member_assignment_properties_from_expression(
                    property, properties,
                );
            }
            Expression::Assign { value, .. }
            | Expression::Await(value)
            | Expression::EnumerateKeys(value)
            | Expression::GetIterator(value)
            | Expression::IteratorClose(value)
            | Expression::Unary {
                expression: value, ..
            } => self.collect_lexical_this_member_assignment_properties_from_expression(
                value, properties,
            ),
            Expression::Binary { left, right, .. } => {
                self.collect_lexical_this_member_assignment_properties_from_expression(
                    left, properties,
                );
                self.collect_lexical_this_member_assignment_properties_from_expression(
                    right, properties,
                );
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                self.collect_lexical_this_member_assignment_properties_from_expression(
                    condition, properties,
                );
                self.collect_lexical_this_member_assignment_properties_from_expression(
                    then_expression,
                    properties,
                );
                self.collect_lexical_this_member_assignment_properties_from_expression(
                    else_expression,
                    properties,
                );
            }
            Expression::Sequence(expressions) => {
                for expression in expressions {
                    self.collect_lexical_this_member_assignment_properties_from_expression(
                        expression, properties,
                    );
                }
            }
            Expression::Call { callee, arguments }
            | Expression::New { callee, arguments }
            | Expression::SuperCall { callee, arguments } => {
                self.collect_lexical_this_member_assignment_properties_from_expression(
                    callee, properties,
                );
                for argument in arguments {
                    match argument {
                        CallArgument::Expression(value) | CallArgument::Spread(value) => self
                            .collect_lexical_this_member_assignment_properties_from_expression(
                                value, properties,
                            ),
                    }
                }
            }
            Expression::SuperMember { property } => {
                self.collect_lexical_this_member_assignment_properties_from_expression(
                    property, properties,
                );
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
            | Expression::Update { .. } => {}
        }
    }

    fn collect_lexical_this_member_assignment_properties_from_statements(
        &self,
        statements: &[Statement],
        properties: &mut BTreeMap<String, Expression>,
    ) {
        for statement in statements {
            match statement {
                Statement::Declaration { body }
                | Statement::Block { body }
                | Statement::Labeled { body, .. } => {
                    self.collect_lexical_this_member_assignment_properties_from_statements(
                        body, properties,
                    );
                }
                Statement::Var { value, .. }
                | Statement::Let { value, .. }
                | Statement::Assign { value, .. } => self
                    .collect_lexical_this_member_assignment_properties_from_expression(
                        value, properties,
                    ),
                Statement::Print { values } => {
                    for value in values {
                        self.collect_lexical_this_member_assignment_properties_from_expression(
                            value, properties,
                        );
                    }
                }
                Statement::Expression(value)
                | Statement::Throw(value)
                | Statement::Return(value)
                | Statement::Yield { value }
                | Statement::YieldDelegate { value } => {
                    self.collect_lexical_this_member_assignment_properties_from_expression(
                        value, properties,
                    );
                }
                Statement::AssignMember {
                    object,
                    property,
                    value,
                } => {
                    if matches!(object, Expression::This) {
                        let property = self.canonical_object_property_expression(property);
                        if let Some(property_name) = static_property_name_from_expression(&property)
                        {
                            properties.insert(property_name, property);
                        }
                    }
                    self.collect_lexical_this_member_assignment_properties_from_expression(
                        object, properties,
                    );
                    self.collect_lexical_this_member_assignment_properties_from_expression(
                        property, properties,
                    );
                    self.collect_lexical_this_member_assignment_properties_from_expression(
                        value, properties,
                    );
                }
                Statement::If {
                    condition,
                    then_branch,
                    else_branch,
                } => {
                    self.collect_lexical_this_member_assignment_properties_from_expression(
                        condition, properties,
                    );
                    self.collect_lexical_this_member_assignment_properties_from_statements(
                        then_branch,
                        properties,
                    );
                    self.collect_lexical_this_member_assignment_properties_from_statements(
                        else_branch,
                        properties,
                    );
                }
                Statement::With { object, body } => {
                    self.collect_lexical_this_member_assignment_properties_from_expression(
                        object, properties,
                    );
                    self.collect_lexical_this_member_assignment_properties_from_statements(
                        body, properties,
                    );
                }
                Statement::Try {
                    body,
                    catch_setup,
                    catch_body,
                    ..
                } => {
                    self.collect_lexical_this_member_assignment_properties_from_statements(
                        body, properties,
                    );
                    self.collect_lexical_this_member_assignment_properties_from_statements(
                        catch_setup,
                        properties,
                    );
                    self.collect_lexical_this_member_assignment_properties_from_statements(
                        catch_body, properties,
                    );
                }
                Statement::Switch {
                    discriminant,
                    cases,
                    ..
                } => {
                    self.collect_lexical_this_member_assignment_properties_from_expression(
                        discriminant,
                        properties,
                    );
                    for case in cases {
                        if let Some(test) = &case.test {
                            self.collect_lexical_this_member_assignment_properties_from_expression(
                                test, properties,
                            );
                        }
                        self.collect_lexical_this_member_assignment_properties_from_statements(
                            &case.body, properties,
                        );
                    }
                }
                Statement::For {
                    init,
                    condition,
                    update,
                    break_hook,
                    body,
                    ..
                } => {
                    self.collect_lexical_this_member_assignment_properties_from_statements(
                        init, properties,
                    );
                    if let Some(condition) = condition {
                        self.collect_lexical_this_member_assignment_properties_from_expression(
                            condition, properties,
                        );
                    }
                    if let Some(update) = update {
                        self.collect_lexical_this_member_assignment_properties_from_expression(
                            update, properties,
                        );
                    }
                    if let Some(break_hook) = break_hook {
                        self.collect_lexical_this_member_assignment_properties_from_expression(
                            break_hook, properties,
                        );
                    }
                    self.collect_lexical_this_member_assignment_properties_from_statements(
                        body, properties,
                    );
                }
                Statement::While {
                    condition,
                    break_hook,
                    body,
                    ..
                }
                | Statement::DoWhile {
                    condition,
                    break_hook,
                    body,
                    ..
                } => {
                    self.collect_lexical_this_member_assignment_properties_from_expression(
                        condition, properties,
                    );
                    if let Some(break_hook) = break_hook {
                        self.collect_lexical_this_member_assignment_properties_from_expression(
                            break_hook, properties,
                        );
                    }
                    self.collect_lexical_this_member_assignment_properties_from_statements(
                        body, properties,
                    );
                }
                Statement::Break { .. } | Statement::Continue { .. } => {}
            }
        }
    }

    fn lexical_this_member_assignment_properties(
        &self,
        user_function: &UserFunction,
    ) -> BTreeMap<String, Expression> {
        let mut properties = BTreeMap::new();
        if !user_function.lexical_this {
            return properties;
        }
        if let Some(declaration) = self.prepared_function_declaration(&user_function.name) {
            self.collect_lexical_this_member_assignment_properties_from_statements(
                &declaration.body,
                &mut properties,
            );
        }
        properties
    }

    fn emit_present_runtime_object_property_shadow_copy(
        &mut self,
        source_owner: &str,
        target_owner: &str,
        property: &Expression,
    ) -> DirectResult<()> {
        let source_binding =
            self.runtime_object_property_shadow_binding_by_property(source_owner, property);
        let target_binding =
            self.runtime_object_property_shadow_binding_by_property(target_owner, property);
        let target_deleted =
            self.runtime_object_property_shadow_deleted_binding_by_property(target_owner, property);
        self.push_global_get(source_binding.present_index);
        self.state.emission.output.instructions.push(0x04);
        self.state
            .emission
            .output
            .instructions
            .push(EMPTY_BLOCK_TYPE);
        self.push_control_frame();
        self.push_i32_const(JS_UNDEFINED_TAG);
        self.push_global_set(target_deleted.value_index);
        self.push_i32_const(0);
        self.push_global_set(target_deleted.present_index);
        self.push_global_get(source_binding.value_index);
        self.push_global_set(target_binding.value_index);
        self.push_i32_const(1);
        self.push_global_set(target_binding.present_index);
        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();
        Ok(())
    }

    fn emit_lexical_this_member_assignment_shadow_writebacks(
        &mut self,
        user_function: &UserFunction,
        capture_hidden_name: &str,
        source_binding_name: &str,
    ) -> DirectResult<()> {
        if source_binding_name == "this"
            || source_binding_name == "new.target"
            || Self::capture_slot_member_source_key_parts(source_binding_name).is_some()
        {
            return Ok(());
        }
        let properties = self.lexical_this_member_assignment_properties(user_function);
        if properties.is_empty() {
            return Ok(());
        }
        let target_owner = self
            .runtime_object_property_shadow_owner_name_for_identifier(source_binding_name)
            .unwrap_or_else(|| source_binding_name.to_string());
        for property in properties.values() {
            self.emit_present_runtime_object_property_shadow_copy(
                capture_hidden_name,
                &target_owner,
                property,
            )?;
        }
        Ok(())
    }

    pub(in crate::backend::direct_wasm) fn bound_capture_slots_include_member_source(
        &self,
        capture_slots: &BTreeMap<String, String>,
    ) -> bool {
        capture_slots.values().any(|slot_name| {
            self.resolve_capture_slot_source_binding_name(slot_name)
                .as_deref()
                .and_then(Self::capture_slot_member_source_key_parts)
                .is_some()
        })
    }

    fn bound_capture_source_binding_is_available(&self, source_name: &str) -> bool {
        if Self::capture_slot_member_source_key_parts(source_name).is_some() {
            return true;
        }
        source_name == "this"
            || source_name == "new.target"
            || self.state.runtime.locals.bindings.contains_key(source_name)
            || self.resolve_current_local_binding(source_name).is_some()
            || self
                .resolve_user_function_capture_hidden_name(source_name)
                .is_some()
            || self
                .resolve_eval_local_function_hidden_name(source_name)
                .is_some()
            || self.global_has_binding(source_name)
            || self.backend.global_has_lexical_binding(source_name)
            || self.global_has_implicit_binding(source_name)
            || self.backend.global_function_binding(source_name).is_some()
    }

    fn invalidate_bound_capture_source_binding_static_metadata(&mut self, source_name: &str) {
        if let Some((resolved_name, _)) = self.resolve_current_local_binding(source_name) {
            self.state
                .clear_local_static_binding_metadata(&resolved_name);
        } else if self.state.runtime.locals.bindings.contains_key(source_name)
            || self
                .parameter_scope_arguments_local_for(source_name)
                .is_some()
        {
            self.state.clear_local_static_binding_metadata(source_name);
        } else {
            self.clear_static_identifier_binding_metadata(source_name);
            self.backend
                .shared_global_semantics
                .clear_global_binding_state(source_name);
            self.backend
                .shared_global_semantics
                .clear_global_object_literal_member_bindings_for_name(source_name);
        }
    }

    fn user_function_capture_targets_immutable_class_binding(
        &self,
        user_function: &UserFunction,
        capture_name: &str,
    ) -> bool {
        let Some(declaration) = self.resolve_registered_function_declaration(&user_function.name)
        else {
            return false;
        };
        let capture_source_name = scoped_binding_source_name(capture_name).unwrap_or(capture_name);
        declaration.immutable_class_bindings.iter().any(|binding| {
            let binding_source_name = scoped_binding_source_name(binding).unwrap_or(binding);
            binding == capture_name
                || binding == capture_source_name
                || binding_source_name == capture_name
                || binding_source_name == capture_source_name
        })
    }

    fn emit_sync_bound_capture_member_source_from_local(
        &mut self,
        source_name: &str,
        value_local: u32,
        value: &Expression,
    ) -> DirectResult<bool> {
        let Some((object_name, property_name)) =
            Self::capture_slot_member_source_key_parts(source_name)
        else {
            return Ok(false);
        };
        let object = Expression::Identifier(object_name);
        let store_local = if matches!(value, Expression::Identifier(_)) {
            value_local
        } else {
            let store_local = self.allocate_temp_local();
            self.with_suspended_with_scopes_if_active_scope_object(&object, |compiler| {
                compiler.emit_numeric_expression(value)
            })?;
            self.push_local_set(store_local);
            store_local
        };
        let property = Expression::String(property_name.clone());
        let deleted_binding =
            self.with_suspended_with_scopes_if_active_scope_object(&object, |compiler| {
                Ok(compiler
                    .resolve_runtime_object_property_shadow_deleted_binding(&object, &property))
            })?;
        if let Some(deleted_binding) = deleted_binding {
            self.push_global_get(deleted_binding.present_index);
            self.state.emission.output.instructions.push(0x04);
            self.state
                .emission
                .output
                .instructions
                .push(EMPTY_BLOCK_TYPE);
            self.push_control_frame();
            self.state.emission.output.instructions.push(0x05);
            self.emit_scoped_property_store_from_local(
                &object,
                &property_name,
                store_local,
                value,
            )?;
            self.state.emission.output.instructions.push(0x1a);
            self.state.emission.output.instructions.push(0x0b);
            self.pop_control_frame();
        } else {
            self.emit_scoped_property_store_from_local(
                &object,
                &property_name,
                store_local,
                value,
            )?;
            self.state.emission.output.instructions.push(0x1a);
        }
        Ok(true)
    }

    fn preserve_snapshot_binding_prototype(&self, name: &str, value: &Expression) -> Expression {
        let Expression::Object(entries) = value else {
            return value.clone();
        };
        let identifier = Expression::Identifier(name.to_string());
        let Some(prototype) = self.resolve_static_object_prototype_expression(&identifier) else {
            return value.clone();
        };

        let mut next_entries = entries
            .iter()
            .filter(|entry| {
                !matches!(
                    entry,
                    crate::ir::hir::ObjectEntry::Data { key, .. }
                        if matches!(key, Expression::String(property_name) if property_name == "__proto__")
                )
            })
            .cloned()
            .collect::<Vec<_>>();
        next_entries.insert(
            0,
            crate::ir::hir::ObjectEntry::Data {
                key: Expression::String("__proto__".to_string()),
                value: prototype,
            },
        );
        Expression::Object(next_entries)
    }

    fn collect_static_capture_member_update_from_expression(
        expression: &Expression,
        capture_name: &str,
        update: &mut Option<Expression>,
    ) {
        match expression {
            Expression::Assign { name, value } => {
                Self::collect_static_capture_member_update_from_expression(
                    value,
                    capture_name,
                    update,
                );
                if name == capture_name {
                    *update = Some((**value).clone());
                }
            }
            Expression::Unary {
                op: UnaryOp::Delete,
                expression,
            } => {
                if matches!(expression.as_ref(), Expression::Identifier(name) if name == capture_name)
                {
                    *update = Some(Expression::Undefined);
                }
            }
            Expression::Sequence(expressions) => {
                for expression in expressions {
                    Self::collect_static_capture_member_update_from_expression(
                        expression,
                        capture_name,
                        update,
                    );
                }
            }
            Expression::AssignMember {
                object,
                property,
                value,
            } => {
                for expression in [object.as_ref(), property.as_ref(), value.as_ref()] {
                    Self::collect_static_capture_member_update_from_expression(
                        expression,
                        capture_name,
                        update,
                    );
                }
            }
            Expression::Binary { left, right, .. } => {
                Self::collect_static_capture_member_update_from_expression(
                    left,
                    capture_name,
                    update,
                );
                Self::collect_static_capture_member_update_from_expression(
                    right,
                    capture_name,
                    update,
                );
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                Self::collect_static_capture_member_update_from_expression(
                    condition,
                    capture_name,
                    update,
                );
                Self::collect_static_capture_member_update_from_expression(
                    then_expression,
                    capture_name,
                    update,
                );
                Self::collect_static_capture_member_update_from_expression(
                    else_expression,
                    capture_name,
                    update,
                );
            }
            Expression::Call { callee, arguments }
            | Expression::New { callee, arguments }
            | Expression::SuperCall { callee, arguments } => {
                Self::collect_static_capture_member_update_from_expression(
                    callee,
                    capture_name,
                    update,
                );
                for argument in arguments {
                    Self::collect_static_capture_member_update_from_expression(
                        argument.expression(),
                        capture_name,
                        update,
                    );
                }
            }
            Expression::Array(elements) => {
                for element in elements {
                    match element {
                        ArrayElement::Expression(value) | ArrayElement::Spread(value) => {
                            Self::collect_static_capture_member_update_from_expression(
                                value,
                                capture_name,
                                update,
                            );
                        }
                    }
                }
            }
            Expression::Object(entries) => {
                for entry in entries {
                    match entry {
                        ObjectEntry::Data { key, value } => {
                            Self::collect_static_capture_member_update_from_expression(
                                key,
                                capture_name,
                                update,
                            );
                            Self::collect_static_capture_member_update_from_expression(
                                value,
                                capture_name,
                                update,
                            );
                        }
                        ObjectEntry::Getter { key, getter } => {
                            Self::collect_static_capture_member_update_from_expression(
                                key,
                                capture_name,
                                update,
                            );
                            Self::collect_static_capture_member_update_from_expression(
                                getter,
                                capture_name,
                                update,
                            );
                        }
                        ObjectEntry::Setter { key, setter } => {
                            Self::collect_static_capture_member_update_from_expression(
                                key,
                                capture_name,
                                update,
                            );
                            Self::collect_static_capture_member_update_from_expression(
                                setter,
                                capture_name,
                                update,
                            );
                        }
                        ObjectEntry::Spread(value) => {
                            Self::collect_static_capture_member_update_from_expression(
                                value,
                                capture_name,
                                update,
                            );
                        }
                    }
                }
            }
            Expression::Member { object, property } => {
                Self::collect_static_capture_member_update_from_expression(
                    object,
                    capture_name,
                    update,
                );
                Self::collect_static_capture_member_update_from_expression(
                    property,
                    capture_name,
                    update,
                );
            }
            Expression::SuperMember { property }
            | Expression::AssignSuperMember { property, .. } => {
                Self::collect_static_capture_member_update_from_expression(
                    property,
                    capture_name,
                    update,
                );
            }
            Expression::Await(value)
            | Expression::EnumerateKeys(value)
            | Expression::GetIterator(value)
            | Expression::IteratorClose(value)
            | Expression::Unary {
                expression: value, ..
            } => {
                Self::collect_static_capture_member_update_from_expression(
                    value,
                    capture_name,
                    update,
                );
            }
            Expression::Update { .. }
            | Expression::Number(_)
            | Expression::BigInt(_)
            | Expression::String(_)
            | Expression::Bool(_)
            | Expression::Null
            | Expression::Undefined
            | Expression::NewTarget
            | Expression::Identifier(_)
            | Expression::This
            | Expression::Sent => {}
        }
    }

    fn collect_static_capture_member_update_from_statement(
        statement: &Statement,
        capture_name: &str,
        update: &mut Option<Expression>,
    ) -> bool {
        match statement {
            Statement::Assign { name, value } => {
                Self::collect_static_capture_member_update_from_expression(
                    value,
                    capture_name,
                    update,
                );
                if name == capture_name {
                    *update = Some(value.clone());
                }
                false
            }
            Statement::Expression(expression) => {
                Self::collect_static_capture_member_update_from_expression(
                    expression,
                    capture_name,
                    update,
                );
                false
            }
            Statement::Return(expression) | Statement::Throw(expression) => {
                Self::collect_static_capture_member_update_from_expression(
                    expression,
                    capture_name,
                    update,
                );
                true
            }
            Statement::Declaration { body }
            | Statement::Block { body }
            | Statement::Labeled { body, .. }
            | Statement::With { body, .. } => {
                for statement in body {
                    if Self::collect_static_capture_member_update_from_statement(
                        statement,
                        capture_name,
                        update,
                    ) {
                        return true;
                    }
                }
                false
            }
            Statement::Print { values } => {
                for value in values {
                    Self::collect_static_capture_member_update_from_expression(
                        value,
                        capture_name,
                        update,
                    );
                }
                false
            }
            Statement::Var { value, .. } | Statement::Let { value, .. } => {
                Self::collect_static_capture_member_update_from_expression(
                    value,
                    capture_name,
                    update,
                );
                false
            }
            Statement::AssignMember {
                object,
                property,
                value,
            } => {
                for expression in [object, property, value] {
                    Self::collect_static_capture_member_update_from_expression(
                        expression,
                        capture_name,
                        update,
                    );
                }
                false
            }
            Statement::If {
                condition,
                then_branch,
                else_branch,
            } => {
                Self::collect_static_capture_member_update_from_expression(
                    condition,
                    capture_name,
                    update,
                );
                let mut then_update = update.clone();
                let then_terminal = then_branch.iter().any(|statement| {
                    Self::collect_static_capture_member_update_from_statement(
                        statement,
                        capture_name,
                        &mut then_update,
                    )
                });
                let mut else_update = update.clone();
                let else_terminal = else_branch.iter().any(|statement| {
                    Self::collect_static_capture_member_update_from_statement(
                        statement,
                        capture_name,
                        &mut else_update,
                    )
                });
                if then_update == else_update {
                    *update = then_update;
                }
                then_terminal && else_terminal
            }
            Statement::Try {
                body,
                catch_setup,
                catch_body,
                ..
            } => {
                for statement in body.iter().chain(catch_setup).chain(catch_body) {
                    if Self::collect_static_capture_member_update_from_statement(
                        statement,
                        capture_name,
                        update,
                    ) {
                        return true;
                    }
                }
                false
            }
            Statement::Switch {
                discriminant,
                cases,
                ..
            } => {
                Self::collect_static_capture_member_update_from_expression(
                    discriminant,
                    capture_name,
                    update,
                );
                for case in cases {
                    if let Some(test) = &case.test {
                        Self::collect_static_capture_member_update_from_expression(
                            test,
                            capture_name,
                            update,
                        );
                    }
                    for statement in &case.body {
                        if Self::collect_static_capture_member_update_from_statement(
                            statement,
                            capture_name,
                            update,
                        ) {
                            return true;
                        }
                    }
                }
                false
            }
            Statement::For {
                init,
                condition,
                update: loop_update,
                break_hook,
                body,
                ..
            } => {
                for statement in init.iter().chain(body) {
                    if Self::collect_static_capture_member_update_from_statement(
                        statement,
                        capture_name,
                        update,
                    ) {
                        return true;
                    }
                }
                for expression in condition.iter().chain(loop_update).chain(break_hook) {
                    Self::collect_static_capture_member_update_from_expression(
                        expression,
                        capture_name,
                        update,
                    );
                }
                false
            }
            Statement::While {
                condition,
                break_hook,
                body,
                ..
            }
            | Statement::DoWhile {
                condition,
                break_hook,
                body,
                ..
            } => {
                Self::collect_static_capture_member_update_from_expression(
                    condition,
                    capture_name,
                    update,
                );
                if let Some(break_hook) = break_hook {
                    Self::collect_static_capture_member_update_from_expression(
                        break_hook,
                        capture_name,
                        update,
                    );
                }
                for statement in body {
                    if Self::collect_static_capture_member_update_from_statement(
                        statement,
                        capture_name,
                        update,
                    ) {
                        return true;
                    }
                }
                false
            }
            Statement::Yield { value } | Statement::YieldDelegate { value } => {
                Self::collect_static_capture_member_update_from_expression(
                    value,
                    capture_name,
                    update,
                );
                false
            }
            Statement::Break { .. } | Statement::Continue { .. } => false,
        }
    }

    fn static_capture_member_update_expression_for_user_function(
        &self,
        user_function: &UserFunction,
        capture_name: &str,
    ) -> Option<Expression> {
        let function = self.resolve_registered_function_declaration(&user_function.name)?;
        let mut update = None;
        for statement in &function.body {
            if Self::collect_static_capture_member_update_from_statement(
                statement,
                capture_name,
                &mut update,
            ) {
                break;
            }
        }
        update
    }

    pub(in crate::backend::direct_wasm) fn snapshot_bound_capture_slot_expression(
        &self,
        slot_name: &str,
    ) -> Expression {
        if let Some(source_expression) =
            self.resolve_capture_slot_static_source_expression(slot_name)
        {
            return source_expression;
        }
        if let Some(array_binding) = self
            .resolve_array_binding_from_expression(&Expression::Identifier(slot_name.to_string()))
        {
            return Expression::Array(
                array_binding
                    .values
                    .iter()
                    .map(|value| {
                        ArrayElement::Expression(value.clone().unwrap_or(Expression::Undefined))
                    })
                    .collect(),
            );
        }
        if let Some(object_binding) = self
            .resolve_object_binding_from_expression(&Expression::Identifier(slot_name.to_string()))
        {
            if !object_binding.property_descriptors.is_empty() {
                return Expression::Identifier(slot_name.to_string());
            }
            return object_binding_to_expression(&object_binding);
        }
        let identifier = Expression::Identifier(slot_name.to_string());
        if let Some(value) = self
            .resolve_bound_alias_expression(&identifier)
            .filter(|value| !static_expression_matches(value, &identifier))
        {
            if self.resolve_simple_generator_source(&value).is_some() {
                return value;
            }
            return self.materialize_static_expression(&value);
        }
        identifier
    }

    pub(in crate::backend::direct_wasm) fn snapshot_prepared_bound_user_function_capture_bindings(
        &self,
        prepared: &[PreparedBoundCaptureBinding],
    ) -> HashMap<String, Expression> {
        prepared
            .iter()
            .map(|binding| {
                let snapshot = self.snapshot_bound_capture_slot_expression(&binding.slot_name);
                let snapshot = if matches!(snapshot, Expression::Undefined)
                    && let Some(source_binding_name) = binding.source_binding_name.as_ref()
                    && !matches!(source_binding_name.as_str(), "this" | "new.target")
                    && Self::capture_slot_member_source_key_parts(source_binding_name).is_none()
                {
                    Expression::Identifier(source_binding_name.clone())
                } else {
                    snapshot
                };
                (binding.capture_name.clone(), snapshot)
            })
            .collect()
    }

    pub(in crate::backend::direct_wasm) fn prepare_bound_user_function_capture_bindings(
        &mut self,
        user_function: &UserFunction,
        capture_slots: &BTreeMap<String, String>,
    ) -> DirectResult<Vec<PreparedBoundCaptureBinding>> {
        let trace_capture_bindings = std::env::var_os("AYY_TRACE_CAPTURE_BINDINGS").is_some();
        let Some(capture_bindings) = self.user_function_capture_bindings(&user_function.name)
        else {
            if trace_capture_bindings {
                eprintln!(
                    "capture_bindings bound_prepare_start target={} slots={capture_slots:?} captures=None",
                    user_function.name
                );
            }
            return Ok(Vec::new());
        };
        if trace_capture_bindings {
            eprintln!(
                "capture_bindings bound_prepare_start target={} slots={capture_slots:?} captures={capture_bindings:?}",
                user_function.name
            );
        }

        let mut prepared = Vec::new();
        for (capture_name, capture_hidden_name) in capture_bindings {
            let Some(slot_name) = capture_slots.get(&capture_name) else {
                if trace_capture_bindings {
                    eprintln!(
                        "capture_bindings bound_skip target={} capture={} reason=no_slot",
                        user_function.name, capture_name
                    );
                }
                continue;
            };
            let (slot_local, source_binding_name) = if slot_name == "this" {
                let source_expression = self
                    .resolve_user_function_capture_hidden_name("this")
                    .map(Expression::Identifier)
                    .unwrap_or(Expression::This);
                let slot_local_name = self.allocate_named_hidden_local(
                    &format!("bound_capture_slot_{}_{}", user_function.name, capture_name),
                    StaticValueKind::Unknown,
                );
                let slot_local = self
                    .state
                    .runtime
                    .locals
                    .bindings
                    .get(&slot_local_name)
                    .copied()
                    .expect("fresh bound capture slot local must exist");
                self.emit_numeric_expression(&source_expression)?;
                self.push_local_set(slot_local);
                let source_binding_name = match source_expression {
                    Expression::Identifier(name) => Some(name),
                    Expression::This => Some("this".to_string()),
                    _ => None,
                };
                (slot_local, source_binding_name)
            } else if let Some(slot_local) =
                self.state.runtime.locals.bindings.get(slot_name).copied()
            {
                let slot_is_member_closure = slot_name.starts_with("__ayy_member_closure_slot_");
                let explicit_slot_source = self
                    .state
                    .speculation
                    .static_semantics
                    .capture_slot_source_bindings
                    .get(slot_name)
                    .cloned();
                let source_binding_name = explicit_slot_source
                    .or_else(|| {
                        if slot_is_member_closure {
                            return None;
                        }
                        self.runtime_object_property_shadow_owner_name_for_identifier(slot_name)
                    })
                    .or_else(|| {
                        if slot_is_member_closure {
                            return None;
                        }
                        self.state
                            .speculation
                            .static_semantics
                            .local_value_binding(slot_name)
                            .and_then(|value| {
                                let Expression::Identifier(name) =
                                    self.materialize_static_expression(value)
                                else {
                                    return None;
                                };
                                Some(name)
                            })
                    })
                    .map(|source_name| {
                        if source_name.starts_with("__ayy_bound_capture_slot_")
                            && !slot_name.starts_with("__ayy_")
                        {
                            slot_name.clone()
                        } else {
                            source_name
                        }
                    })
                    .map(|source_name| self.capture_slot_live_source_binding_name(&source_name))
                    .filter(|source_name| {
                        self.bound_capture_source_binding_is_available(source_name)
                    });
                if capture_name == "this" && source_binding_name.as_deref() == Some("this") {
                    let dynamic_slot_local_name = self.allocate_named_hidden_local(
                        &format!("bound_capture_slot_{}_{}", user_function.name, capture_name),
                        StaticValueKind::Unknown,
                    );
                    let dynamic_slot_local = self
                        .state
                        .runtime
                        .locals
                        .bindings
                        .get(&dynamic_slot_local_name)
                        .copied()
                        .expect("fresh bound capture slot local must exist");
                    if self.current_function_is_derived_constructor() {
                        self.emit_numeric_expression(&Expression::This)?;
                    } else {
                        self.push_local_get(slot_local);
                        self.push_local_tee(dynamic_slot_local);
                        self.push_i32_const(JS_UNDEFINED_TAG);
                        self.push_binary_op(BinaryOp::Equal)?;
                        self.state.emission.output.instructions.push(0x04);
                        self.state.emission.output.instructions.push(I32_TYPE);
                        self.push_control_frame();
                        self.emit_named_error_throw("ReferenceError")?;
                        self.push_i32_const(JS_UNDEFINED_TAG);
                        self.state.emission.output.instructions.push(0x05);
                        self.push_local_get(dynamic_slot_local);
                        self.state.emission.output.instructions.push(0x0b);
                        self.pop_control_frame();
                    }
                    self.push_local_set(dynamic_slot_local);
                    (dynamic_slot_local, source_binding_name)
                } else {
                    (slot_local, source_binding_name)
                }
            } else if let Some(global_binding) = self.hidden_implicit_global_binding(slot_name) {
                let slot_local_name = self.allocate_named_hidden_local(
                    &format!("bound_capture_slot_{}_{}", user_function.name, capture_name),
                    StaticValueKind::Unknown,
                );
                let slot_local = self
                    .state
                    .runtime
                    .locals
                    .bindings
                    .get(&slot_local_name)
                    .copied()
                    .expect("fresh bound capture slot local must exist");
                if capture_name.starts_with("__ayy_class_brand_") {
                    self.push_global_get(global_binding.present_index);
                    self.state.emission.output.instructions.push(0x04);
                    self.state.emission.output.instructions.push(I32_TYPE);
                    self.push_control_frame();
                    self.push_global_get(global_binding.value_index);
                    self.state.emission.output.instructions.push(0x05);
                    if !self.emit_private_brand_runtime_value_for_binding_name(&capture_name)? {
                        self.emit_private_brand_direct_or_synthetic_runtime_value_for_binding_name(
                            &capture_name,
                        )?;
                    }
                    self.state.emission.output.instructions.push(0x0b);
                    self.pop_control_frame();
                } else if capture_name == "this"
                    && self
                        .state
                        .speculation
                        .static_semantics
                        .capture_slot_source_bindings
                        .get(slot_name)
                        .is_some_and(|source_name| source_name == "this")
                {
                    if self.current_function_is_derived_constructor() {
                        self.emit_numeric_expression(&Expression::This)?;
                    } else {
                        self.push_global_get(global_binding.value_index);
                        self.push_local_tee(slot_local);
                        self.push_i32_const(JS_UNDEFINED_TAG);
                        self.push_binary_op(BinaryOp::Equal)?;
                        self.state.emission.output.instructions.push(0x04);
                        self.state.emission.output.instructions.push(I32_TYPE);
                        self.push_control_frame();
                        self.emit_named_error_throw("ReferenceError")?;
                        self.push_i32_const(JS_UNDEFINED_TAG);
                        self.state.emission.output.instructions.push(0x05);
                        self.push_local_get(slot_local);
                        self.state.emission.output.instructions.push(0x0b);
                        self.pop_control_frame();
                    }
                } else {
                    self.push_global_get(global_binding.value_index);
                }
                self.push_local_set(slot_local);
                let source_binding_name = self
                    .state
                    .speculation
                    .static_semantics
                    .capture_slot_source_bindings
                    .get(slot_name)
                    .cloned()
                    .unwrap_or_else(|| slot_name.clone());
                let source_binding_name =
                    self.capture_slot_live_source_binding_name(&source_binding_name);
                (slot_local, Some(source_binding_name))
            } else if self.global_has_binding(slot_name)
                || self.backend.global_has_lexical_binding(slot_name)
                || self.global_has_implicit_binding(slot_name)
                || self.backend.global_function_binding(slot_name).is_some()
                || (is_internal_user_function_identifier(slot_name)
                    && self.user_function_runtime_value(slot_name).is_some())
            {
                let slot_local_name = self.allocate_named_hidden_local(
                    &format!("bound_capture_slot_{}_{}", user_function.name, capture_name),
                    StaticValueKind::Unknown,
                );
                let slot_local = self
                    .state
                    .runtime
                    .locals
                    .bindings
                    .get(&slot_local_name)
                    .copied()
                    .expect("fresh bound capture slot local must exist");
                self.emit_identifier_expression_value(slot_name)?;
                self.push_local_set(slot_local);
                (slot_local, Some(slot_name.clone()))
            } else {
                if trace_capture_bindings {
                    eprintln!(
                        "capture_bindings bound_skip target={} capture={} slot={} reason=unavailable local={} hidden_global={} global={} lexical={} implicit={} fn_global={}",
                        user_function.name,
                        capture_name,
                        slot_name,
                        self.state.runtime.locals.bindings.contains_key(slot_name),
                        self.hidden_implicit_global_binding(slot_name).is_some(),
                        self.global_has_binding(slot_name),
                        self.backend.global_has_lexical_binding(slot_name),
                        self.global_has_implicit_binding(slot_name),
                        self.backend.global_function_binding(slot_name).is_some()
                    );
                }
                continue;
            };
            if let Some(source_binding_name) = source_binding_name.as_ref() {
                self.state
                    .speculation
                    .static_semantics
                    .capture_slot_source_bindings
                    .insert(capture_hidden_name.clone(), source_binding_name.clone());
            }
            let binding = self
                .implicit_global_binding(&capture_hidden_name)
                .unwrap_or_else(|| self.ensure_implicit_global_binding(&capture_hidden_name));
            if trace_capture_bindings {
                eprintln!(
                    "capture_bindings bound_prepare target={} capture={} hidden={} slot={} source={:?} value_index={} present_index={}",
                    user_function.name,
                    capture_name,
                    capture_hidden_name,
                    slot_name,
                    source_binding_name,
                    binding.value_index,
                    binding.present_index,
                );
            }
            let saved_value_local = self.allocate_temp_local();
            let saved_present_local = self.allocate_temp_local();
            self.push_global_get(binding.value_index);
            self.push_local_set(saved_value_local);
            self.push_global_get(binding.present_index);
            self.push_local_set(saved_present_local);
            prepared.push(PreparedBoundCaptureBinding {
                binding,
                capture_name,
                capture_hidden_name,
                slot_name: slot_name.clone(),
                source_binding_name,
                slot_local,
                saved_value_local,
                saved_present_local,
            });
        }

        Ok(prepared)
    }

    pub(in crate::backend::direct_wasm) fn emit_prepare_bound_user_function_capture_globals(
        &mut self,
        prepared: &[PreparedBoundCaptureBinding],
    ) -> DirectResult<()> {
        for binding in prepared {
            let lexical_source_initialized_local =
                binding
                    .source_binding_name
                    .as_ref()
                    .and_then(|source_name| {
                        self.resolve_current_local_binding(source_name).and_then(
                            |(resolved_name, _)| {
                                self.local_lexical_initialized_local(&resolved_name)
                                    .map(|initialized_local| (resolved_name, initialized_local))
                            },
                        )
                    });
            if let Some((resolved_name, _)) = lexical_source_initialized_local.as_ref()
                && self.local_lexical_capture_source_is_statically_uninitialized(resolved_name)
            {
                self.clear_user_function_capture_static_metadata(&binding.capture_hidden_name);
                self.push_i32_const(JS_UNDEFINED_TAG);
                self.push_global_set(binding.binding.value_index);
                self.push_i32_const(0);
                self.push_global_set(binding.binding.present_index);
                continue;
            }
            let metadata_source_name = binding
                .source_binding_name
                .as_deref()
                .unwrap_or(&binding.slot_name);
            self.sync_user_function_capture_static_metadata(
                metadata_source_name,
                &binding.capture_hidden_name,
            );
            self.alias_runtime_binding_metadata(&binding.capture_hidden_name, &binding.slot_name);
            if let Some(source_binding_name) = binding.source_binding_name.as_ref() {
                self.alias_runtime_binding_metadata(
                    &binding.capture_hidden_name,
                    source_binding_name,
                );
                self.state
                    .speculation
                    .static_semantics
                    .capture_slot_source_bindings
                    .insert(
                        binding.capture_hidden_name.clone(),
                        source_binding_name.clone(),
                    );
            }
            if let Some((_, initialized_local)) = lexical_source_initialized_local {
                self.push_local_get(initialized_local);
                self.state.emission.output.instructions.push(0x04);
                self.state
                    .emission
                    .output
                    .instructions
                    .push(EMPTY_BLOCK_TYPE);
                self.push_control_frame();
                self.push_local_get(binding.slot_local);
                self.push_global_set(binding.binding.value_index);
                self.push_i32_const(1);
                self.push_global_set(binding.binding.present_index);
                self.state.emission.output.instructions.push(0x05);
                self.push_i32_const(JS_UNDEFINED_TAG);
                self.push_global_set(binding.binding.value_index);
                self.push_i32_const(0);
                self.push_global_set(binding.binding.present_index);
                self.state.emission.output.instructions.push(0x0b);
                self.pop_control_frame();
            } else {
                self.push_local_get(binding.slot_local);
                self.push_global_set(binding.binding.value_index);
                self.push_i32_const(1);
                self.push_global_set(binding.binding.present_index);
            }
            if let Some(source_binding_name) = binding.source_binding_name.as_ref() {
                if source_binding_name == "this" {
                    if let Some(owner_name) =
                        self.runtime_object_property_shadow_owner_name_for_identifier("this")
                        && owner_name != binding.capture_hidden_name
                    {
                        self.emit_runtime_object_property_shadow_copy(
                            &owner_name,
                            &binding.capture_hidden_name,
                        )?;
                    }
                } else if source_binding_name != "new.target" {
                    self.emit_runtime_object_property_shadow_copy(
                        source_binding_name,
                        &binding.capture_hidden_name,
                    )?;
                }
            } else if binding.slot_name != binding.capture_hidden_name
                && !binding.slot_name.starts_with("__ayy_class_brand_")
            {
                self.emit_runtime_object_property_shadow_copy(
                    &binding.slot_name,
                    &binding.capture_hidden_name,
                )?;
            }

            let deleted_marker_name =
                Self::capture_slot_member_source_deleted_binding_name(&binding.capture_hidden_name);
            let deleted_marker = self.ensure_implicit_global_binding(&deleted_marker_name);
            if let Some(source_binding_name) = binding.source_binding_name.as_ref()
                && let Some((object_name, property_name)) =
                    Self::capture_slot_member_source_key_parts(source_binding_name)
            {
                let object = Expression::Identifier(object_name);
                let property = Expression::String(property_name);
                if let Some(deleted_binding) =
                    self.resolve_runtime_object_property_shadow_deleted_binding(&object, &property)
                {
                    self.push_global_get(deleted_binding.present_index);
                    self.push_global_set(deleted_marker.present_index);
                    continue;
                }
            }
            self.push_i32_const(0);
            self.push_global_set(deleted_marker.present_index);
        }
        Ok(())
    }

    pub(in crate::backend::direct_wasm) fn sync_bound_user_function_capture_slots(
        &mut self,
        user_function: &UserFunction,
        prepared: &[PreparedBoundCaptureBinding],
        updated_bindings: Option<&HashMap<String, Expression>>,
        this_capture_target_owner: Option<&str>,
    ) -> DirectResult<()> {
        let assigned_nonlocal_bindings =
            self.collect_user_function_assigned_nonlocal_bindings(user_function);
        for binding in prepared {
            let shadow_writeback_name =
                binding.source_binding_name.as_ref().cloned().or_else(|| {
                    (binding.slot_name != binding.capture_hidden_name
                        && !binding.slot_name.starts_with("__ayy_class_brand_"))
                    .then(|| binding.slot_name.clone())
                });
            let source_aliases_this = shadow_writeback_name.as_ref().is_some_and(|name| {
                if name == "this" || name == "new.target" || name.starts_with("__ayy_class_brand_")
                {
                    return false;
                }
                let source_expression = Expression::Identifier(name.clone());
                self.resolve_bound_alias_expression(&source_expression)
                    .is_some_and(|resolved| match resolved {
                        Expression::This => true,
                        Expression::Identifier(name) => name == "this",
                        _ => false,
                    })
            });
            let value_local = self.allocate_temp_local();
            self.push_global_get(binding.binding.value_index);
            self.push_local_set(value_local);
            let capture_targets_immutable_class_binding = self
                .user_function_capture_targets_immutable_class_binding(
                    user_function,
                    &binding.capture_name,
                );
            if capture_targets_immutable_class_binding {
                continue;
            }
            let hidden_capture_expression =
                Expression::Identifier(binding.capture_hidden_name.clone());
            let updated_capture_binding =
                if user_function.lexical_this && binding.capture_name == "this" {
                    None
                } else {
                    updated_bindings.and_then(|bindings| bindings.get(&binding.capture_name))
                };
            let capture_writeback_is_dynamic = updated_capture_binding.is_none()
                && self.user_function_mentions_direct_eval(user_function);
            let capture_writeback_may_update_source = capture_writeback_is_dynamic
                || assigned_nonlocal_bindings.contains(&binding.capture_name)
                || binding
                    .source_binding_name
                    .as_ref()
                    .is_some_and(|source_name| {
                        let source_name =
                            scoped_binding_source_name(source_name).unwrap_or(source_name);
                        assigned_nonlocal_bindings.contains(source_name)
                    });
            let source_writeback_expression = updated_capture_binding
                .map(|value| {
                    if binding.source_binding_name.as_deref() == Some("this") {
                        hidden_capture_expression.clone()
                    } else {
                        value.clone()
                    }
                })
                .unwrap_or_else(|| {
                    if binding
                        .source_binding_name
                        .as_deref()
                        .and_then(Self::capture_slot_member_source_key_parts)
                        .is_some()
                    {
                        if let Some(updated_expression) = self
                            .static_capture_member_update_expression_for_user_function(
                                user_function,
                                &binding.capture_name,
                            )
                        {
                            return updated_expression;
                        }
                        Expression::Identifier(binding.slot_name.clone())
                    } else {
                        hidden_capture_expression.clone()
                    }
                });
            if let Some(value) = updated_capture_binding {
                let metadata_value = if binding.source_binding_name.as_deref() == Some("this") {
                    &hidden_capture_expression
                } else {
                    value
                };
                if binding.source_binding_name.as_deref() == Some("this") {
                    if std::env::var_os("AYY_TRACE_THIS_FLOW").is_some() {
                        eprintln!(
                            "this_flow bound_capture_sync fn={:?} capture_hidden={} updated_value={value:?} global_binding={:?}",
                            self.current_function_name(),
                            binding.capture_hidden_name,
                            self.global_object_binding(&binding.capture_hidden_name)
                                .map(|binding| binding.string_properties.clone())
                        );
                    }
                    if let Some(object_binding) = self.resolve_runtime_shadow_object_binding("this")
                    {
                        self.backend.sync_global_object_binding(
                            &binding.capture_hidden_name,
                            Some(object_binding.clone()),
                        );
                        self.backend.set_global_binding_kind(
                            &binding.capture_hidden_name,
                            StaticValueKind::Object,
                        );
                        self.backend
                            .shared_global_semantics
                            .values
                            .object_bindings
                            .insert(binding.capture_hidden_name.clone(), object_binding.clone());
                        self.backend
                            .shared_global_semantics
                            .set_global_binding_kind(
                                &binding.capture_hidden_name,
                                StaticValueKind::Object,
                            );
                        self.sync_runtime_object_property_shadow_static_metadata_from_binding(
                            &binding.capture_hidden_name,
                            &object_binding,
                        );
                        self.sync_runtime_object_property_shadow_static_metadata_from_binding(
                            "this",
                            &object_binding,
                        );
                    }
                    self.state
                        .runtime
                        .locals
                        .runtime_dynamic_bindings
                        .insert("this".to_string());
                    self.update_local_value_binding("this", metadata_value);
                    self.update_local_object_binding("this", metadata_value);
                } else {
                    self.update_capture_slot_binding_from_expression(
                        &binding.slot_name,
                        metadata_value,
                    )?;
                }
                if let Some(source_binding_name) = &binding.source_binding_name {
                    if source_binding_name == "new.target" {
                    } else if Self::capture_slot_member_source_key_parts(source_binding_name)
                        .is_some()
                    {
                    } else if source_binding_name != "this" {
                        self.sync_bound_capture_source_binding_metadata(
                            source_binding_name,
                            metadata_value,
                        )?;
                        self.emit_runtime_object_property_shadow_copy(
                            &binding.capture_hidden_name,
                            source_binding_name,
                        )?;
                    }
                }
            } else {
                if binding.source_binding_name.as_deref() == Some("this") {
                    if std::env::var_os("AYY_TRACE_THIS_FLOW").is_some() {
                        eprintln!(
                            "this_flow bound_capture_sync fn={:?} capture_hidden={} global_binding={:?}",
                            self.current_function_name(),
                            binding.capture_hidden_name,
                            self.global_object_binding(&binding.capture_hidden_name)
                                .map(|binding| binding.string_properties.clone())
                        );
                    }
                    self.state
                        .runtime
                        .locals
                        .runtime_dynamic_bindings
                        .insert("this".to_string());
                    self.update_local_value_binding("this", &hidden_capture_expression);
                    self.update_local_object_binding("this", &hidden_capture_expression);
                } else {
                    if capture_writeback_is_dynamic {
                        self.state
                            .clear_local_static_binding_metadata(&binding.slot_name);
                    } else {
                        self.update_capture_slot_binding_from_expression(
                            &binding.slot_name,
                            &hidden_capture_expression,
                        )?;
                    }
                }
            }
            self.push_local_get(value_local);
            self.push_local_set(binding.slot_local);
            if let Some(source_binding_name) = binding.source_binding_name.as_ref() {
                if source_binding_name == "new.target" {
                    continue;
                } else if source_binding_name == "this" {
                    self.push_local_get(value_local);
                    self.push_global_set(CURRENT_THIS_GLOBAL_INDEX);
                    if let Some(owner_name) =
                        this_capture_target_owner.map(str::to_string).or_else(|| {
                            self.runtime_object_property_shadow_owner_name_for_identifier("this")
                        })
                        && owner_name != binding.capture_hidden_name
                    {
                        self.emit_runtime_object_property_shadow_copy(
                            &binding.capture_hidden_name,
                            &owner_name,
                        )?;
                    }
                } else if self.emit_sync_bound_capture_member_source_from_local(
                    source_binding_name,
                    value_local,
                    &source_writeback_expression,
                )? {
                    if matches!(
                        &source_writeback_expression,
                        Expression::Identifier(slot_name) if slot_name == &binding.slot_name
                    ) {
                        self.state
                            .clear_local_static_binding_metadata(&binding.slot_name);
                    }
                } else if let Some(hidden_binding) =
                    self.hidden_implicit_global_binding(source_binding_name)
                {
                    self.push_local_get(value_local);
                    self.push_global_set(hidden_binding.value_index);
                    self.push_i32_const(1);
                    self.push_global_set(hidden_binding.present_index);
                } else if !source_binding_name.starts_with("__ayy_class_brand_")
                    && !source_binding_name.starts_with("__ayy_class_super_")
                {
                    let source_is_immutable_local = self
                        .resolve_current_local_binding(source_binding_name)
                        .is_some_and(|(resolved_name, _)| {
                            self.local_binding_is_immutable(&resolved_name)
                        })
                        || self
                            .binding_is_immutable_function_self_binding_source(source_binding_name)
                        || self
                            .backend
                            .lexical_global_binding(source_binding_name)
                            .is_some_and(|global_binding| !global_binding.mutable);
                    if !source_is_immutable_local {
                        self.emit_sync_identifier_runtime_value_from_local(
                            source_binding_name,
                            value_local,
                        )?;
                    }
                }
                if updated_capture_binding.is_none()
                    && capture_writeback_may_update_source
                    && source_binding_name != "this"
                    && source_binding_name != "new.target"
                    && Self::capture_slot_member_source_key_parts(source_binding_name).is_none()
                    && !source_binding_name.starts_with("__ayy_class_brand_")
                    && !source_binding_name.starts_with("__ayy_class_super_")
                {
                    self.invalidate_bound_capture_source_binding_static_metadata(
                        source_binding_name,
                    );
                }
                if source_binding_name != "this"
                    && Self::capture_slot_member_source_key_parts(source_binding_name).is_none()
                {
                    self.emit_runtime_object_property_shadow_copy(
                        &binding.capture_hidden_name,
                        source_binding_name,
                    )?;
                    if user_function.lexical_this && binding.capture_name == "this" {
                        self.emit_lexical_this_member_assignment_shadow_writebacks(
                            user_function,
                            &binding.capture_hidden_name,
                            source_binding_name,
                        )?;
                    }
                    if let Some(object_binding) =
                        self.resolve_runtime_shadow_object_binding(&binding.capture_hidden_name)
                    {
                        self.sync_global_member_function_bindings_from_object_binding(
                            source_binding_name,
                            &object_binding,
                        );
                    }
                    if source_aliases_this {
                        let this_owner = this_capture_target_owner
                            .map(str::to_string)
                            .or_else(|| {
                                self.runtime_object_property_shadow_owner_name_for_identifier(
                                    "this",
                                )
                            })
                            .unwrap_or_else(|| "this".to_string());
                        self.emit_runtime_object_property_shadow_copy(
                            &binding.capture_hidden_name,
                            &this_owner,
                        )?;
                        if this_owner != "this" {
                            self.emit_runtime_object_property_shadow_copy(
                                &binding.capture_hidden_name,
                                "this",
                            )?;
                        }
                        if let Some(object_binding) =
                            self.resolve_runtime_shadow_object_binding(&binding.capture_hidden_name)
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
            }
            if binding.source_binding_name.is_none()
                && let Some(shadow_writeback_name) = shadow_writeback_name
                    .as_deref()
                    .filter(|name| *name != "this")
            {
                self.emit_runtime_object_property_shadow_copy(
                    &binding.capture_hidden_name,
                    shadow_writeback_name,
                )?;
                if source_aliases_this {
                    let this_owner = this_capture_target_owner
                        .map(str::to_string)
                        .or_else(|| {
                            self.runtime_object_property_shadow_owner_name_for_identifier("this")
                        })
                        .unwrap_or_else(|| "this".to_string());
                    self.emit_runtime_object_property_shadow_copy(
                        &binding.capture_hidden_name,
                        &this_owner,
                    )?;
                    if this_owner != "this" {
                        self.emit_runtime_object_property_shadow_copy(
                            &binding.capture_hidden_name,
                            "this",
                        )?;
                    }
                    if let Some(object_binding) =
                        self.resolve_runtime_shadow_object_binding(&binding.capture_hidden_name)
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
        }
        Ok(())
    }

    fn sync_global_member_function_bindings_from_object_binding(
        &mut self,
        name: &str,
        object_binding: &ObjectValueBinding,
    ) {
        for (property_name, value) in &object_binding.string_properties {
            let property = MemberFunctionBindingProperty::String(property_name.clone());
            let key = MemberFunctionBindingKey {
                target: MemberFunctionBindingTarget::Identifier(name.to_string()),
                property,
            };
            if let Some(binding) = self.resolve_function_binding_from_expression(value) {
                self.backend
                    .set_global_member_function_binding(key.clone(), binding);
            } else {
                self.backend.clear_global_member_function_binding(&key);
            }
            self.backend.clear_global_member_getter_binding(&key);
            self.backend.clear_global_member_setter_binding(&key);
        }
        for (property_expression, value) in &object_binding.symbol_properties {
            let Some(property) = self.member_function_binding_property(property_expression) else {
                continue;
            };
            let key = MemberFunctionBindingKey {
                target: MemberFunctionBindingTarget::Identifier(name.to_string()),
                property,
            };
            if let Some(binding) = self.resolve_function_binding_from_expression(value) {
                self.backend
                    .set_global_member_function_binding(key.clone(), binding);
            } else {
                self.backend.clear_global_member_function_binding(&key);
            }
            self.backend.clear_global_member_getter_binding(&key);
            self.backend.clear_global_member_setter_binding(&key);
        }
    }

    pub(in crate::backend::direct_wasm) fn sync_bound_capture_source_binding_metadata(
        &mut self,
        name: &str,
        value: &Expression,
    ) -> DirectResult<()> {
        let preserved_value = self.preserve_snapshot_binding_prototype(name, value);
        let is_globally_bound =
            self.global_has_binding(name) || self.global_has_implicit_binding(name);
        let preserve_source_binding_metadata =
            self.user_function_capture_source_is_locally_bound(name);

        if preserve_source_binding_metadata {
            self.update_capture_slot_binding_from_expression(name, &preserved_value)?;
        } else {
            self.state.clear_local_static_binding_metadata(name);
        }

        if is_globally_bound
            && !matches!(preserved_value, Expression::Identifier(ref identifier) if identifier == name)
        {
            if std::env::var_os("AYY_TRACE_CAPTURE_BINDINGS").is_some() {
                eprintln!("capture_bindings sync_source name={name} value={preserved_value:?}");
            }
            self.update_static_global_assignment_metadata(name, &preserved_value);
            if let Expression::Object(entries) = &preserved_value {
                for entry in entries {
                    let crate::ir::hir::ObjectEntry::Data { key, value } = entry else {
                        continue;
                    };
                    let Some(property) = self.member_function_binding_property(key) else {
                        continue;
                    };
                    let key = MemberFunctionBindingKey {
                        target: MemberFunctionBindingTarget::Identifier(name.to_string()),
                        property,
                    };
                    if let Some(binding) = self.resolve_function_binding_from_expression(value) {
                        self.backend
                            .set_global_member_function_binding(key.clone(), binding);
                    } else {
                        self.backend.clear_global_member_function_binding(&key);
                    }
                    self.backend.clear_global_member_getter_binding(&key);
                    self.backend.clear_global_member_setter_binding(&key);
                }
            }
            self.update_global_specialized_function_value(name, &preserved_value)?;
            self.update_global_property_descriptor_value(name, &preserved_value);
        }

        Ok(())
    }

    pub(in crate::backend::direct_wasm) fn restore_bound_user_function_capture_bindings(
        &mut self,
        prepared: &[PreparedBoundCaptureBinding],
    ) {
        for binding in prepared.iter().rev() {
            self.push_local_get(binding.saved_value_local);
            self.push_global_set(binding.binding.value_index);
            self.push_local_get(binding.saved_present_local);
            self.push_global_set(binding.binding.present_index);
        }
    }
}
