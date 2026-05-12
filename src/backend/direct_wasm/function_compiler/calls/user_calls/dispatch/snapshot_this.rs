use super::*;

impl<'a> FunctionCompiler<'a> {
    pub(in crate::backend::direct_wasm) fn receiver_shadow_updated_via_parameter_writebacks(
        &self,
        this_expression: &Expression,
        writebacks: &[(String, String, Option<ObjectValueBinding>)],
    ) -> bool {
        self.resolve_user_function_call_receiver_shadow_owner(this_expression)
            .as_deref()
            .is_some_and(|target_owner| {
                writebacks
                    .iter()
                    .any(|(_, source_owner, _)| source_owner == target_owner)
            })
    }

    fn object_binding_contains_private_brand_marker(
        &self,
        object_binding: &ObjectValueBinding,
        private_brand_binding: &str,
    ) -> bool {
        let expected_value = self.materialize_static_expression(&Expression::Identifier(
            private_brand_binding.to_string(),
        ));
        ordered_object_property_names(object_binding)
            .into_iter()
            .any(|property_name| {
                self.resolve_object_binding_property_value(
                    object_binding,
                    &Expression::String(property_name),
                )
                .is_some_and(|value| {
                    let materialized_value = self.materialize_static_expression(&value);
                    static_expression_matches(&materialized_value, &expected_value)
                        || static_expression_matches(&expected_value, &materialized_value)
                })
            })
    }

    pub(in crate::backend::direct_wasm) fn user_function_call_allows_static_this_shadow_commit(
        &self,
        user_function: &UserFunction,
        this_expression: &Expression,
    ) -> bool {
        if user_function.lexical_this
            || !self.user_function_mentions_private_member_access(user_function)
        {
            return true;
        }
        let Some(private_brand_binding) = user_function.private_brand_binding.as_deref() else {
            return false;
        };
        self.resolve_user_function_call_receiver_shadow_owner(this_expression)
            .and_then(|owner| self.resolve_runtime_shadow_object_binding(&owner))
            .is_some_and(|object_binding| {
                self.object_binding_contains_private_brand_marker(
                    &object_binding,
                    private_brand_binding,
                )
            })
            || self
                .resolve_object_binding_from_expression(this_expression)
                .is_some_and(|object_binding| {
                    self.object_binding_contains_private_brand_marker(
                        &object_binding,
                        private_brand_binding,
                    )
                })
    }

    pub(in crate::backend::direct_wasm) fn resolve_static_snapshot_this_expression(
        &self,
        this_expression: &Expression,
    ) -> Expression {
        let resolved_this = self
            .resolve_bound_alias_expression(this_expression)
            .filter(|resolved| !static_expression_matches(resolved, this_expression))
            .unwrap_or_else(|| this_expression.clone());

        if !matches!(resolved_this, Expression::This) {
            return match &resolved_this {
                Expression::Identifier(name) => self
                    .state
                    .speculation
                    .static_semantics
                    .local_value_binding(name)
                    .cloned()
                    .or_else(|| {
                        self.backend
                            .global_semantics
                            .values
                            .value_bindings
                            .get(name)
                            .cloned()
                    })
                    .unwrap_or(resolved_this),
                _ => self.materialize_static_expression(&resolved_this),
            };
        }

        self.resolve_object_binding_from_expression(&Expression::This)
            .map(|binding| object_binding_to_expression(&binding))
            .or_else(|| {
                self.state
                    .speculation
                    .static_semantics
                    .local_value_binding("this")
                    .cloned()
            })
            .or_else(|| {
                self.backend
                    .global_semantics
                    .values
                    .value_bindings
                    .get("this")
                    .cloned()
            })
            .unwrap_or(resolved_this)
    }

    pub(in crate::backend::direct_wasm) fn resolve_user_function_call_receiver_shadow_owner(
        &self,
        this_expression: &Expression,
    ) -> Option<String> {
        match this_expression {
            Expression::Identifier(name) => {
                return self.runtime_object_property_shadow_owner_name_for_identifier(name);
            }
            Expression::This => return Some("this".to_string()),
            _ => {}
        }
        let resolved_this = self
            .resolve_bound_alias_expression(this_expression)
            .filter(|resolved| !static_expression_matches(resolved, this_expression))
            .unwrap_or_else(|| this_expression.clone());
        match resolved_this {
            Expression::Identifier(name) => {
                self.runtime_object_property_shadow_owner_name_for_identifier(&name)
            }
            Expression::This => Some("this".to_string()),
            _ => None,
        }
    }

    fn collect_static_this_member_write_property_names_from_expression(
        &self,
        expression: &Expression,
        property_names: &mut BTreeSet<String>,
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
                        property_names.insert(property_name);
                    }
                }
                self.collect_static_this_member_write_property_names_from_expression(
                    object,
                    property_names,
                );
                self.collect_static_this_member_write_property_names_from_expression(
                    property,
                    property_names,
                );
                self.collect_static_this_member_write_property_names_from_expression(
                    value,
                    property_names,
                );
            }
            Expression::Assign { value, .. }
            | Expression::Await(value)
            | Expression::EnumerateKeys(value)
            | Expression::GetIterator(value)
            | Expression::IteratorClose(value)
            | Expression::Unary {
                expression: value, ..
            } => {
                self.collect_static_this_member_write_property_names_from_expression(
                    value,
                    property_names,
                );
            }
            Expression::AssignSuperMember { property, value } => {
                self.collect_static_this_member_write_property_names_from_expression(
                    property,
                    property_names,
                );
                self.collect_static_this_member_write_property_names_from_expression(
                    value,
                    property_names,
                );
            }
            Expression::Member { object, property } => {
                self.collect_static_this_member_write_property_names_from_expression(
                    object,
                    property_names,
                );
                self.collect_static_this_member_write_property_names_from_expression(
                    property,
                    property_names,
                );
            }
            Expression::SuperMember { property } => {
                self.collect_static_this_member_write_property_names_from_expression(
                    property,
                    property_names,
                );
            }
            Expression::Binary { left, right, .. } => {
                self.collect_static_this_member_write_property_names_from_expression(
                    left,
                    property_names,
                );
                self.collect_static_this_member_write_property_names_from_expression(
                    right,
                    property_names,
                );
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                self.collect_static_this_member_write_property_names_from_expression(
                    condition,
                    property_names,
                );
                self.collect_static_this_member_write_property_names_from_expression(
                    then_expression,
                    property_names,
                );
                self.collect_static_this_member_write_property_names_from_expression(
                    else_expression,
                    property_names,
                );
            }
            Expression::Sequence(expressions) => {
                for expression in expressions {
                    self.collect_static_this_member_write_property_names_from_expression(
                        expression,
                        property_names,
                    );
                }
            }
            Expression::Call { callee, arguments }
            | Expression::SuperCall { callee, arguments }
            | Expression::New { callee, arguments } => {
                self.collect_static_this_member_write_property_names_from_expression(
                    callee,
                    property_names,
                );
                for argument in arguments {
                    match argument {
                        CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                            self.collect_static_this_member_write_property_names_from_expression(
                                expression,
                                property_names,
                            );
                        }
                    }
                }
            }
            Expression::Array(elements) => {
                for element in elements {
                    match element {
                        ArrayElement::Expression(expression) | ArrayElement::Spread(expression) => {
                            self.collect_static_this_member_write_property_names_from_expression(
                                expression,
                                property_names,
                            );
                        }
                    }
                }
            }
            Expression::Object(entries) => {
                for entry in entries {
                    match entry {
                        ObjectEntry::Data { key, value } => {
                            self.collect_static_this_member_write_property_names_from_expression(
                                key,
                                property_names,
                            );
                            self.collect_static_this_member_write_property_names_from_expression(
                                value,
                                property_names,
                            );
                        }
                        ObjectEntry::Getter { key, getter } => {
                            self.collect_static_this_member_write_property_names_from_expression(
                                key,
                                property_names,
                            );
                            self.collect_static_this_member_write_property_names_from_expression(
                                getter,
                                property_names,
                            );
                        }
                        ObjectEntry::Setter { key, setter } => {
                            self.collect_static_this_member_write_property_names_from_expression(
                                key,
                                property_names,
                            );
                            self.collect_static_this_member_write_property_names_from_expression(
                                setter,
                                property_names,
                            );
                        }
                        ObjectEntry::Spread(expression) => {
                            self.collect_static_this_member_write_property_names_from_expression(
                                expression,
                                property_names,
                            );
                        }
                    }
                }
            }
            Expression::Update { .. }
            | Expression::Identifier(_)
            | Expression::Number(_)
            | Expression::BigInt(_)
            | Expression::String(_)
            | Expression::Bool(_)
            | Expression::Null
            | Expression::Undefined
            | Expression::This
            | Expression::NewTarget
            | Expression::Sent => {}
        }
    }

    fn collect_static_this_member_write_property_names_from_statement(
        &self,
        statement: &Statement,
        property_names: &mut BTreeSet<String>,
    ) {
        match statement {
            Statement::Declaration { body }
            | Statement::Block { body }
            | Statement::Labeled { body, .. }
            | Statement::With { body, .. } => {
                for statement in body {
                    self.collect_static_this_member_write_property_names_from_statement(
                        statement,
                        property_names,
                    );
                }
            }
            Statement::Expression(expression)
            | Statement::Return(expression)
            | Statement::Throw(expression)
            | Statement::Yield { value: expression }
            | Statement::YieldDelegate { value: expression }
            | Statement::Var {
                value: expression, ..
            }
            | Statement::Let {
                value: expression, ..
            } => {
                self.collect_static_this_member_write_property_names_from_expression(
                    expression,
                    property_names,
                );
            }
            Statement::Print { values } => {
                for value in values {
                    self.collect_static_this_member_write_property_names_from_expression(
                        value,
                        property_names,
                    );
                }
            }
            Statement::Assign { value, .. } => {
                self.collect_static_this_member_write_property_names_from_expression(
                    value,
                    property_names,
                );
            }
            Statement::AssignMember {
                object,
                property,
                value,
            } => {
                if matches!(object, Expression::This) {
                    let property = self.canonical_object_property_expression(property);
                    if let Some(property_name) = static_property_name_from_expression(&property) {
                        property_names.insert(property_name);
                    }
                }
                self.collect_static_this_member_write_property_names_from_expression(
                    object,
                    property_names,
                );
                self.collect_static_this_member_write_property_names_from_expression(
                    property,
                    property_names,
                );
                self.collect_static_this_member_write_property_names_from_expression(
                    value,
                    property_names,
                );
            }
            Statement::If {
                condition,
                then_branch,
                else_branch,
            } => {
                self.collect_static_this_member_write_property_names_from_expression(
                    condition,
                    property_names,
                );
                for statement in then_branch {
                    self.collect_static_this_member_write_property_names_from_statement(
                        statement,
                        property_names,
                    );
                }
                for statement in else_branch {
                    self.collect_static_this_member_write_property_names_from_statement(
                        statement,
                        property_names,
                    );
                }
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
                self.collect_static_this_member_write_property_names_from_expression(
                    condition,
                    property_names,
                );
                if let Some(break_hook) = break_hook {
                    self.collect_static_this_member_write_property_names_from_expression(
                        break_hook,
                        property_names,
                    );
                }
                for statement in body {
                    self.collect_static_this_member_write_property_names_from_statement(
                        statement,
                        property_names,
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
                for statement in init {
                    self.collect_static_this_member_write_property_names_from_statement(
                        statement,
                        property_names,
                    );
                }
                if let Some(condition) = condition {
                    self.collect_static_this_member_write_property_names_from_expression(
                        condition,
                        property_names,
                    );
                }
                if let Some(update) = update {
                    self.collect_static_this_member_write_property_names_from_expression(
                        update,
                        property_names,
                    );
                }
                if let Some(break_hook) = break_hook {
                    self.collect_static_this_member_write_property_names_from_expression(
                        break_hook,
                        property_names,
                    );
                }
                for statement in body {
                    self.collect_static_this_member_write_property_names_from_statement(
                        statement,
                        property_names,
                    );
                }
            }
            Statement::Try {
                body,
                catch_setup,
                catch_body,
                ..
            } => {
                for statement in body {
                    self.collect_static_this_member_write_property_names_from_statement(
                        statement,
                        property_names,
                    );
                }
                for statement in catch_setup {
                    self.collect_static_this_member_write_property_names_from_statement(
                        statement,
                        property_names,
                    );
                }
                for statement in catch_body {
                    self.collect_static_this_member_write_property_names_from_statement(
                        statement,
                        property_names,
                    );
                }
            }
            Statement::Switch {
                discriminant,
                cases,
                ..
            } => {
                self.collect_static_this_member_write_property_names_from_expression(
                    discriminant,
                    property_names,
                );
                for case in cases {
                    if let Some(test) = &case.test {
                        self.collect_static_this_member_write_property_names_from_expression(
                            test,
                            property_names,
                        );
                    }
                    for statement in &case.body {
                        self.collect_static_this_member_write_property_names_from_statement(
                            statement,
                            property_names,
                        );
                    }
                }
            }
            Statement::Break { .. } | Statement::Continue { .. } => {}
        }
    }

    fn user_function_static_this_member_write_property_names(
        &self,
        user_function: &UserFunction,
    ) -> BTreeSet<String> {
        let Some(function) = self.resolve_registered_function_declaration(&user_function.name)
        else {
            return BTreeSet::new();
        };
        let mut property_names = BTreeSet::new();
        for statement in &function.body {
            self.collect_static_this_member_write_property_names_from_statement(
                statement,
                &mut property_names,
            );
        }
        property_names
    }

    fn collect_static_this_member_write_property_values_from_statement(
        &self,
        statement: &Statement,
        property_values: &mut BTreeMap<String, Expression>,
    ) {
        match statement {
            Statement::Declaration { body }
            | Statement::Block { body }
            | Statement::Labeled { body, .. }
            | Statement::With { body, .. } => {
                for statement in body {
                    self.collect_static_this_member_write_property_values_from_statement(
                        statement,
                        property_values,
                    );
                    if Self::statement_unconditionally_transfers_control(statement) {
                        break;
                    }
                }
            }
            Statement::Expression(expression)
            | Statement::Return(expression)
            | Statement::Throw(expression)
            | Statement::Yield { value: expression }
            | Statement::YieldDelegate { value: expression }
            | Statement::Var {
                value: expression, ..
            }
            | Statement::Let {
                value: expression, ..
            }
            | Statement::Assign {
                value: expression, ..
            } => {
                self.collect_static_this_member_write_property_values_from_expression(
                    expression,
                    property_values,
                );
            }
            Statement::Print { values } => {
                for value in values {
                    self.collect_static_this_member_write_property_values_from_expression(
                        value,
                        property_values,
                    );
                }
            }
            Statement::AssignMember {
                object,
                property,
                value,
            } => {
                if matches!(object, Expression::This) {
                    let property = self.canonical_object_property_expression(property);
                    if let Some(property_name) = static_property_name_from_expression(&property) {
                        property_values.insert(
                            property_name,
                            self.reference_preserving_static_value_expression(value),
                        );
                    }
                }
                self.collect_static_this_member_write_property_values_from_expression(
                    object,
                    property_values,
                );
                self.collect_static_this_member_write_property_values_from_expression(
                    property,
                    property_values,
                );
                self.collect_static_this_member_write_property_values_from_expression(
                    value,
                    property_values,
                );
            }
            Statement::If {
                condition,
                then_branch,
                else_branch,
            } => {
                self.collect_static_this_member_write_property_values_from_expression(
                    condition,
                    property_values,
                );
                for statement in then_branch {
                    self.collect_static_this_member_write_property_values_from_statement(
                        statement,
                        property_values,
                    );
                }
                for statement in else_branch {
                    self.collect_static_this_member_write_property_values_from_statement(
                        statement,
                        property_values,
                    );
                }
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
                self.collect_static_this_member_write_property_values_from_expression(
                    condition,
                    property_values,
                );
                if let Some(break_hook) = break_hook {
                    self.collect_static_this_member_write_property_values_from_expression(
                        break_hook,
                        property_values,
                    );
                }
                for statement in body {
                    self.collect_static_this_member_write_property_values_from_statement(
                        statement,
                        property_values,
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
                for statement in init {
                    self.collect_static_this_member_write_property_values_from_statement(
                        statement,
                        property_values,
                    );
                }
                if let Some(condition) = condition {
                    self.collect_static_this_member_write_property_values_from_expression(
                        condition,
                        property_values,
                    );
                }
                if let Some(update) = update {
                    self.collect_static_this_member_write_property_values_from_expression(
                        update,
                        property_values,
                    );
                }
                if let Some(break_hook) = break_hook {
                    self.collect_static_this_member_write_property_values_from_expression(
                        break_hook,
                        property_values,
                    );
                }
                for statement in body {
                    self.collect_static_this_member_write_property_values_from_statement(
                        statement,
                        property_values,
                    );
                }
            }
            Statement::Try {
                body,
                catch_setup,
                catch_body,
                ..
            } => {
                for statement in body {
                    self.collect_static_this_member_write_property_values_from_statement(
                        statement,
                        property_values,
                    );
                }
                for statement in catch_setup {
                    self.collect_static_this_member_write_property_values_from_statement(
                        statement,
                        property_values,
                    );
                }
                for statement in catch_body {
                    self.collect_static_this_member_write_property_values_from_statement(
                        statement,
                        property_values,
                    );
                }
            }
            Statement::Switch {
                discriminant,
                cases,
                ..
            } => {
                self.collect_static_this_member_write_property_values_from_expression(
                    discriminant,
                    property_values,
                );
                for case in cases {
                    if let Some(test) = &case.test {
                        self.collect_static_this_member_write_property_values_from_expression(
                            test,
                            property_values,
                        );
                    }
                    for statement in &case.body {
                        self.collect_static_this_member_write_property_values_from_statement(
                            statement,
                            property_values,
                        );
                    }
                }
            }
            Statement::Break { .. } | Statement::Continue { .. } => {}
        }
    }

    fn collect_static_this_member_write_property_values_from_expression(
        &self,
        expression: &Expression,
        property_values: &mut BTreeMap<String, Expression>,
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
                        property_values.insert(
                            property_name,
                            self.reference_preserving_static_value_expression(value),
                        );
                    }
                }
                self.collect_static_this_member_write_property_values_from_expression(
                    object,
                    property_values,
                );
                self.collect_static_this_member_write_property_values_from_expression(
                    property,
                    property_values,
                );
                self.collect_static_this_member_write_property_values_from_expression(
                    value,
                    property_values,
                );
            }
            Expression::Assign { value, .. }
            | Expression::Await(value)
            | Expression::EnumerateKeys(value)
            | Expression::GetIterator(value)
            | Expression::IteratorClose(value)
            | Expression::Unary {
                expression: value, ..
            } => {
                self.collect_static_this_member_write_property_values_from_expression(
                    value,
                    property_values,
                );
            }
            _ => {
                self.collect_static_this_member_write_property_names_from_expression(
                    expression,
                    &mut BTreeSet::new(),
                );
            }
        }
    }

    fn user_function_static_this_member_write_property_values(
        &self,
        user_function: &UserFunction,
    ) -> BTreeMap<String, Expression> {
        let Some(function) = self.resolve_registered_function_declaration(&user_function.name)
        else {
            return BTreeMap::new();
        };
        let mut property_values = BTreeMap::new();
        for statement in &function.body {
            self.collect_static_this_member_write_property_values_from_statement(
                statement,
                &mut property_values,
            );
            if Self::statement_unconditionally_transfers_control(statement) {
                break;
            }
        }
        property_values
    }

    fn emit_global_this_shadow_commit_for_property_names(
        &mut self,
        property_values: &BTreeMap<String, Expression>,
    ) -> DirectResult<()> {
        let updated_this_binding = self.resolve_runtime_shadow_object_binding("this");
        for (property_name, materialized_value) in property_values {
            let property = Expression::String(property_name.clone());
            let shadow_binding =
                self.runtime_object_property_shadow_binding_by_names("this", property_name);
            let deleted_binding =
                self.runtime_object_property_shadow_deleted_binding_by_property("this", &property);
            let target_binding = self.ensure_implicit_global_binding(property_name);

            self.push_global_get(deleted_binding.present_index);
            self.state.emission.output.instructions.push(0x04);
            self.state
                .emission
                .output
                .instructions
                .push(EMPTY_BLOCK_TYPE);
            self.push_control_frame();
            self.push_i32_const(0);
            self.push_global_set(target_binding.present_index);
            self.push_i32_const(JS_UNDEFINED_TAG);
            self.push_global_set(target_binding.value_index);
            self.state.emission.output.instructions.push(0x05);
            self.push_global_get(shadow_binding.present_index);
            self.state.emission.output.instructions.push(0x04);
            self.state
                .emission
                .output
                .instructions
                .push(EMPTY_BLOCK_TYPE);
            self.push_control_frame();
            self.push_global_get(shadow_binding.value_index);
            self.push_global_set(target_binding.value_index);
            self.push_i32_const(1);
            self.push_global_set(target_binding.present_index);
            self.state.emission.output.instructions.push(0x0b);
            self.pop_control_frame();
            self.state.emission.output.instructions.push(0x0b);
            self.pop_control_frame();

            self.update_static_global_assignment_metadata(property_name, materialized_value);
            self.update_global_property_descriptor_value(property_name, materialized_value);
        }

        if let Some(updated_this_binding) = updated_this_binding {
            let updated_this_expression = object_binding_to_expression(&updated_this_binding);
            self.update_local_value_binding("this", &updated_this_expression);
            self.update_local_object_binding("this", &updated_this_expression);
        }

        Ok(())
    }

    pub(in crate::backend::direct_wasm) fn prepare_user_function_runtime_this_shadow_state(
        &mut self,
        this_expression: &Expression,
    ) -> DirectResult<Option<String>> {
        let target_owner = self.resolve_user_function_call_receiver_shadow_owner(this_expression);
        if std::env::var_os("AYY_TRACE_RUNTIME_SHADOWS").is_some() {
            eprintln!(
                "runtime_this_shadow_prepare fn={:?} this_expression={this_expression:?} target_owner={target_owner:?}",
                self.current_function_name(),
            );
        }
        let saved_shadow_owner = (target_owner.as_deref() != Some("this")).then(|| {
            self.allocate_named_hidden_local("saved_this_shadow", StaticValueKind::Object)
        });

        if let Some(saved_shadow_owner) = saved_shadow_owner.as_deref() {
            self.emit_runtime_object_property_shadow_copy("this", saved_shadow_owner)?;
            self.clear_runtime_object_property_shadow_prefix("this");
        }

        if let Some(target_owner) = target_owner.as_deref().filter(|owner| *owner != "this") {
            self.emit_runtime_object_property_shadow_copy(target_owner, "this")?;
        } else if target_owner.is_none()
            && let Some(object_binding) =
                self.resolve_object_binding_from_expression(this_expression)
        {
            self.emit_runtime_object_property_shadow_seed_from_binding("this", &object_binding)?;
        }

        Ok(saved_shadow_owner)
    }

    pub(in crate::backend::direct_wasm) fn finalize_user_function_runtime_this_shadow_state(
        &mut self,
        user_function: &UserFunction,
        this_expression: &Expression,
        updated_bindings: Option<&HashMap<String, Expression>>,
        saved_shadow_owner: Option<&str>,
        allow_static_receiver_update: bool,
        receiver_updated_via_parameter_writeback: bool,
        receiver_may_require_invalidation: bool,
    ) -> DirectResult<()> {
        let target_owner = self.resolve_user_function_call_receiver_shadow_owner(this_expression);
        let explicit_updated_this = updated_bindings.and_then(|bindings| bindings.get("this"));
        if std::env::var_os("AYY_TRACE_RUNTIME_SHADOWS").is_some() {
            eprintln!(
                "runtime_this_shadow_finalize fn={:?} this_expression={this_expression:?} target_owner={target_owner:?} updated_this={:?} allow_static_receiver_update={} receiver_updated_via_parameter_writeback={}",
                self.current_function_name(),
                explicit_updated_this,
                allow_static_receiver_update,
                receiver_updated_via_parameter_writeback,
            );
        }

        if let Some(target_owner) = target_owner.as_deref().filter(|owner| *owner != "this") {
            let updated_receiver_binding = self.resolve_runtime_shadow_object_binding("this");
            let should_copy_runtime_this_shadow =
                explicit_updated_this.is_some() || !receiver_updated_via_parameter_writeback;
            if should_copy_runtime_this_shadow {
                self.emit_runtime_object_property_shadow_copy("this", target_owner)?;
            }
            if std::env::var_os("AYY_TRACE_RUNTIME_SHADOWS").is_some() {
                eprintln!(
                    "runtime_this_shadow_sync fn={:?} target_owner={target_owner} updated_receiver_binding_present={} copied={should_copy_runtime_this_shadow}",
                    self.current_function_name(),
                    updated_receiver_binding.is_some(),
                );
            }
            if let Some(updated_receiver_binding) = updated_receiver_binding {
                if allow_static_receiver_update && explicit_updated_this.is_some() {
                    self.sync_runtime_object_property_shadow_static_metadata_from_binding(
                        target_owner,
                        &updated_receiver_binding,
                    );
                    let updated_receiver_expression =
                        object_binding_to_expression(&updated_receiver_binding);
                    let resolved_identifier_name = match this_expression {
                        Expression::Identifier(name) => self
                            .resolve_current_local_binding(name)
                            .map(|(resolved_name, _)| resolved_name)
                            .filter(|resolved_name| resolved_name != name),
                        _ => None,
                    };
                    match this_expression {
                        Expression::Identifier(name)
                            if self.binding_name_is_global(name)
                                || self.global_has_binding(name)
                                || self.global_has_implicit_binding(name) =>
                        {
                            if let Some(resolved_name) = resolved_identifier_name.as_deref() {
                                self.update_local_value_binding(
                                    resolved_name,
                                    &updated_receiver_expression,
                                );
                                self.update_local_object_binding(
                                    resolved_name,
                                    &updated_receiver_expression,
                                );
                            }
                            self.update_local_value_binding(name, &updated_receiver_expression);
                            self.update_local_object_binding(name, &updated_receiver_expression);
                            self.update_static_global_assignment_metadata(
                                name,
                                &updated_receiver_expression,
                            );
                            if std::env::var_os("AYY_TRACE_RUNTIME_SHADOWS").is_some() {
                                eprintln!(
                                    "runtime_this_shadow_global_update name={name} updated_receiver_expression={updated_receiver_expression:?} direct_resolved_object={:?} value={:?} resolved_object={:?}",
                                    self.resolve_object_binding_from_expression(
                                        &updated_receiver_expression,
                                    )
                                    .map(|binding| object_binding_to_expression(&binding)),
                                    self.global_value_binding(name).cloned(),
                                    self.resolve_object_binding_from_expression(
                                        &Expression::Identifier(name.to_string(),)
                                    )
                                    .map(|binding| object_binding_to_expression(&binding)),
                                );
                            }
                        }
                        Expression::Identifier(name) => {
                            if let Some(resolved_name) = resolved_identifier_name.as_deref() {
                                self.update_local_value_binding(
                                    resolved_name,
                                    &updated_receiver_expression,
                                );
                                self.update_local_object_binding(
                                    resolved_name,
                                    &updated_receiver_expression,
                                );
                            }
                            self.update_local_value_binding(name, &updated_receiver_expression);
                            self.update_local_object_binding(name, &updated_receiver_expression);
                        }
                        _ => {}
                    }
                } else if !allow_static_receiver_update
                    && !receiver_updated_via_parameter_writeback
                    && receiver_may_require_invalidation
                {
                    self.clear_runtime_object_property_shadow_prefix(target_owner);
                    match this_expression {
                        Expression::Identifier(name)
                            if self.binding_name_is_global(name)
                                || self.global_has_binding(name)
                                || self.global_has_implicit_binding(name) =>
                        {
                            self.backend.clear_global_static_binding_metadata(name);
                            self.state.clear_local_static_binding_metadata(name);
                        }
                        Expression::Identifier(name) => {
                            self.state.clear_local_static_binding_metadata(name);
                        }
                        Expression::This => {
                            self.state.clear_local_static_binding_metadata("this");
                        }
                        _ => {}
                    }
                }
            }
        } else if target_owner.as_deref() == Some("this")
            && receiver_may_require_invalidation
            && self.state.speculation.execution_context.top_level_function
        {
            let property_values =
                self.user_function_static_this_member_write_property_values(user_function);
            if !property_values.is_empty() {
                if std::env::var_os("AYY_TRACE_RUNTIME_SHADOWS").is_some() {
                    let property_names = property_values.keys().collect::<Vec<_>>();
                    eprintln!("runtime_this_shadow_global_commit properties={property_names:?}");
                }
                self.emit_global_this_shadow_commit_for_property_names(&property_values)?;
            }
        }

        if let Some(saved_shadow_owner) = saved_shadow_owner {
            self.clear_runtime_object_property_shadow_prefix("this");
            self.emit_runtime_object_property_shadow_copy(saved_shadow_owner, "this")?;
        } else if target_owner.as_deref() != Some("this") {
            self.clear_runtime_object_property_shadow_prefix("this");
        }

        if allow_static_receiver_update && let Some(updated_this) = explicit_updated_this {
            match this_expression {
                Expression::Identifier(name) => {
                    self.update_local_value_binding(name, updated_this);
                    self.update_local_object_binding(name, updated_this);
                }
                Expression::This => {
                    self.update_local_value_binding("this", updated_this);
                    self.update_local_object_binding("this", updated_this);
                }
                _ => {}
            }
        }

        Ok(())
    }
}
