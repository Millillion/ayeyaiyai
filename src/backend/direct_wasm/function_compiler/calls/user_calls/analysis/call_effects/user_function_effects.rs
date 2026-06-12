use super::*;

/// Collects `(owner, property)` pairs for every `delete owner.property`
/// (string-keyed, identifier- or `this`-based owner) inside a statement.
fn collect_member_delete_targets_from_statement(
    statement: &Statement,
    targets: &mut Vec<(String, String)>,
) {
    struct MemberDeleteCollector<'t> {
        targets: &'t mut Vec<(String, String)>,
    }
    impl crate::ir::visit::Visitor for MemberDeleteCollector<'_> {
        fn visit_expression(&mut self, expression: &Expression) {
            if let Expression::Unary {
                op: UnaryOp::Delete,
                expression: operand,
            } = expression
                && let Expression::Member { object, property } = operand.as_ref()
                && let Expression::String(property_name) = property.as_ref()
            {
                let owner_name = match object.as_ref() {
                    Expression::Identifier(name) => Some(name.clone()),
                    Expression::This => Some("this".to_string()),
                    _ => None,
                };
                if let Some(owner_name) = owner_name {
                    self.targets.push((owner_name, property_name.clone()));
                }
            }
            crate::ir::visit::walk_expression(self, expression);
        }
    }
    let mut collector = MemberDeleteCollector { targets };
    crate::ir::visit::Visitor::visit_statement(&mut collector, statement);
}

impl<'a> FunctionCompiler<'a> {
    fn argument_expression_cannot_introduce_call_effects(expression: &Expression) -> bool {
        match expression {
            Expression::Number(_)
            | Expression::BigInt(_)
            | Expression::String(_)
            | Expression::Bool(_)
            | Expression::Null
            | Expression::Undefined => true,
            Expression::Array(elements) => elements.iter().all(|element| match element {
                ArrayElement::Expression(expression) | ArrayElement::Spread(expression) => {
                    Self::argument_expression_cannot_introduce_call_effects(expression)
                }
            }),
            Expression::Object(entries) => entries.iter().all(|entry| match entry {
                ObjectEntry::Data { key, value } => {
                    Self::argument_expression_cannot_introduce_call_effects(key)
                        && Self::argument_expression_cannot_introduce_call_effects(value)
                }
                ObjectEntry::Getter { key, getter } => {
                    Self::argument_expression_cannot_introduce_call_effects(key)
                        && Self::argument_expression_cannot_introduce_call_effects(getter)
                }
                ObjectEntry::Setter { key, setter } => {
                    Self::argument_expression_cannot_introduce_call_effects(key)
                        && Self::argument_expression_cannot_introduce_call_effects(setter)
                }
                ObjectEntry::Spread(expression) => {
                    Self::argument_expression_cannot_introduce_call_effects(expression)
                }
            }),
            _ => false,
        }
    }

    fn sync_static_with_scope_member_assignment_effect(
        &mut self,
        object: &Expression,
        name: &str,
        value: &Expression,
    ) {
        if !self.scope_object_has_binding_property(object, name) {
            return;
        }
        if self.static_with_scope_unscopables_blocks_for_specialization(object, name) != Some(false)
        {
            return;
        }
        let property = Expression::String(name.to_string());
        // A with-scoped compound assignment whose read runs a self-deleting
        // accessor (`get x() { delete this.x; ... }`) removes the binding
        // before the store; a strict store then throws instead of writing, so
        // claiming the post-store value here would resurrect the property.
        // Mark the deletion in the runtime shadow pair and the static model
        // instead, and let presence queries defer to the runtime state.
        if let Expression::Identifier(owner_name) = object
            && self
                .resolve_object_binding_from_expression(object)
                .is_some_and(|object_binding| {
                    self.static_in_object_property_getter_may_delete_property(
                        object,
                        &object_binding,
                        &property,
                    )
                })
        {
            let owner_name = owner_name.clone();
            let deleted_binding = self
                .runtime_object_property_shadow_deleted_binding_by_property(&owner_name, &property);
            self.push_i32_const(JS_UNDEFINED_TAG);
            self.push_global_set(deleted_binding.value_index);
            self.push_i32_const(1);
            self.push_global_set(deleted_binding.present_index);
            let shadow_binding =
                self.runtime_object_property_shadow_binding_by_property(&owner_name, &property);
            self.push_i32_const(JS_UNDEFINED_TAG);
            self.push_global_set(shadow_binding.value_index);
            self.push_i32_const(0);
            self.push_global_set(shadow_binding.present_index);
            let deleted_shadow_name =
                Self::runtime_object_property_deleted_shadow_name(&owner_name, &property);
            self.backend
                .record_emitted_delete_shadow(&deleted_shadow_name);
            crate::backend::direct_wasm::memo::bump_static_state_generation();
            self.scrub_scoped_property_static_claims_after_may_throw_store(object, name);
            return;
        }
        let materialized_value = self.reference_preserving_static_value_expression(value);
        self.update_member_function_assignment_binding(object, &property, value);
        if let Expression::Identifier(owner_name) = object {
            // Seed absent entries from the currently-resolved binding so the
            // scope object's other properties survive the sync; an empty seed
            // would drop them and later reads would resolve to undefined.
            let current_object_binding = self
                .resolve_object_binding_from_expression(object)
                .unwrap_or_else(empty_object_value_binding);
            let object_binding = self
                .backend
                .global_semantics
                .values
                .object_bindings
                .entry(owner_name.clone())
                .or_insert_with(|| current_object_binding.clone());
            object_binding_set_property(
                object_binding,
                property.clone(),
                materialized_value.clone(),
            );
            let updated_object_binding = object_binding.clone();
            let shared_object_binding = self
                .backend
                .shared_global_semantics
                .values
                .object_bindings
                .entry(owner_name.clone())
                .or_insert_with(|| current_object_binding.clone());
            object_binding_set_property(shared_object_binding, property, materialized_value);
            crate::backend::direct_wasm::memo::bump_static_state_generation();
            self.clear_runtime_object_property_shadow_static_metadata_prefix(owner_name);
            self.sync_runtime_object_property_shadow_static_metadata_from_binding(
                owner_name,
                &updated_object_binding,
            );
        }
    }

    fn sync_static_with_scope_member_assignment_effects_from_statement(
        &mut self,
        statement: &Statement,
        active_with_object: Option<&Expression>,
    ) -> bool {
        if let Some(object) = active_with_object {
            match statement {
                Statement::Assign { name, value } | Statement::Var { name, value } => {
                    self.sync_static_with_scope_member_assignment_effect(object, name, value);
                }
                _ => {}
            }
        }
        match statement {
            Statement::With { object, body } => {
                return self.sync_static_with_scope_member_assignment_effects_from_statements(
                    body,
                    Some(object),
                );
            }
            Statement::Declaration { body } | Statement::Block { body } => {
                return self.sync_static_with_scope_member_assignment_effects_from_statements(
                    body,
                    active_with_object,
                );
            }
            Statement::Labeled { body, .. } => {
                return self.sync_static_with_scope_member_assignment_effects_from_statements(
                    body,
                    active_with_object,
                );
            }
            Statement::If {
                then_branch,
                else_branch,
                ..
            } => {
                let then_terminal = self
                    .sync_static_with_scope_member_assignment_effects_from_statements(
                        then_branch,
                        active_with_object,
                    );
                let else_terminal = self
                    .sync_static_with_scope_member_assignment_effects_from_statements(
                        else_branch,
                        active_with_object,
                    );
                return then_terminal && else_terminal;
            }
            Statement::Try {
                body,
                catch_setup,
                catch_body,
                ..
            } => {
                for statement in body.iter().chain(catch_setup).chain(catch_body) {
                    self.sync_static_with_scope_member_assignment_effects_from_statement(
                        statement,
                        active_with_object,
                    );
                }
            }
            Statement::Switch { cases, .. } => {
                for case in cases {
                    for statement in &case.body {
                        self.sync_static_with_scope_member_assignment_effects_from_statement(
                            statement,
                            active_with_object,
                        );
                    }
                }
            }
            Statement::For { init, body, .. } => {
                for statement in init.iter().chain(body) {
                    self.sync_static_with_scope_member_assignment_effects_from_statement(
                        statement,
                        active_with_object,
                    );
                }
            }
            Statement::While { body, .. } | Statement::DoWhile { body, .. } => {
                for statement in body {
                    self.sync_static_with_scope_member_assignment_effects_from_statement(
                        statement,
                        active_with_object,
                    );
                }
            }
            Statement::Throw(_)
            | Statement::Return(_)
            | Statement::Break { .. }
            | Statement::Continue { .. } => return true,
            _ => {}
        }
        false
    }

    fn sync_static_with_scope_member_assignment_effects_from_statements(
        &mut self,
        statements: &[Statement],
        active_with_object: Option<&Expression>,
    ) -> bool {
        for statement in statements {
            if self.sync_static_with_scope_member_assignment_effects_from_statement(
                statement,
                active_with_object,
            ) {
                return true;
            }
        }
        false
    }

    pub(in crate::backend::direct_wasm) fn sync_static_with_scope_member_assignment_effects(
        &mut self,
        user_function: &UserFunction,
    ) {
        let Some(function) = self
            .resolve_registered_function_declaration(&user_function.name)
            .cloned()
        else {
            return;
        };
        for statement in &function.body {
            if self.sync_static_with_scope_member_assignment_effects_from_statement(statement, None)
            {
                break;
            }
        }
        let mut visited = HashSet::new();
        self.register_transitive_member_delete_shadows_for_function(&user_function.name, &mut visited);
    }

    /// Registers runtime delete-shadow pairs for every `delete obj.prop` a
    /// called function (or a function it transitively references) may
    /// perform on a global-object-bound owner. The callee bodies compile
    /// out-of-line after the caller, so presence queries (`'prop' in obj`)
    /// emitted in the caller must already know to defer to the runtime
    /// deleted-shadow pair instead of folding the static property table.
    fn register_transitive_member_delete_shadows_for_function(
        &mut self,
        function_name: &str,
        visited: &mut HashSet<String>,
    ) {
        if !visited.insert(function_name.to_string()) {
            return;
        }
        let Some(function) = self
            .resolve_registered_function_declaration(function_name)
            .cloned()
        else {
            return;
        };
        let mut deletes = Vec::new();
        let mut referenced = HashSet::new();
        for statement in &function.body {
            collect_member_delete_targets_from_statement(statement, &mut deletes);
            collect_referenced_binding_names_from_statement(statement, &mut referenced);
        }
        for (owner_name, property_name) in deletes {
            let owner_is_global_object_bound = self.global_object_binding(&owner_name).is_some()
                || self
                    .backend
                    .shared_global_semantics
                    .values
                    .object_binding(&owner_name)
                    .is_some();
            if !owner_is_global_object_bound {
                continue;
            }
            let property = Expression::String(property_name);
            self.runtime_object_property_shadow_deleted_binding_by_property(&owner_name, &property);
            self.record_emitted_delete_shadow_for(&owner_name, &property);
            crate::backend::direct_wasm::memo::bump_static_state_generation();
        }
        for name in referenced {
            if is_internal_user_function_identifier(&name) && self.user_function(&name).is_some() {
                self.register_transitive_member_delete_shadows_for_function(&name, visited);
            }
        }
    }

    pub(in crate::backend::direct_wasm) fn collect_user_function_call_effect_nonlocal_bindings(
        &self,
        user_function: &UserFunction,
    ) -> HashSet<String> {
        let mut visited = HashSet::new();
        self.collect_user_function_call_effect_nonlocal_bindings_for_name(
            &user_function.name,
            &mut visited,
        )
    }

    pub(in crate::backend::direct_wasm) fn collect_user_function_call_effect_nonlocal_bindings_for_name(
        &self,
        function_name: &str,
        visited: &mut HashSet<String>,
    ) -> HashSet<String> {
        if !visited.insert(function_name.to_string()) {
            return HashSet::new();
        }
        let Some(user_function) = self.user_function(function_name) else {
            return HashSet::new();
        };
        let mut names = self.collect_user_function_assigned_nonlocal_bindings(user_function);
        let Some(function) = self.resolve_registered_function_declaration(function_name) else {
            return names;
        };
        for parameter in &function.params {
            if let Some(default) = &parameter.default {
                self.collect_expression_call_effect_nonlocal_bindings(
                    default,
                    Some(function_name),
                    &mut names,
                    visited,
                );
            }
        }
        if !self
            .user_function_parameter_iterator_consumption_indices(user_function)
            .is_empty()
        {
            return names;
        }
        for statement in &function.body {
            self.collect_statement_call_effect_nonlocal_bindings(
                statement,
                Some(function_name),
                &mut names,
                visited,
            );
        }
        names
    }

    pub(in crate::backend::direct_wasm) fn collect_user_function_argument_call_effect_nonlocal_bindings(
        &self,
        user_function: &UserFunction,
        arguments: &[Expression],
    ) -> HashSet<String> {
        if !self
            .user_function_parameter_iterator_consumption_indices(user_function)
            .is_empty()
        {
            return HashSet::new();
        }
        let Some(function) = self.resolve_registered_function_declaration(&user_function.name)
        else {
            return HashSet::new();
        };
        let call_arguments = arguments
            .iter()
            .cloned()
            .map(CallArgument::Expression)
            .collect::<Vec<_>>();
        let mut iterator_names = Vec::new();
        Self::collect_iterator_close_binding_names_from_statements(
            &function.body,
            &mut iterator_names,
        );
        if iterator_names.is_empty()
            && arguments
                .iter()
                .all(Self::argument_expression_cannot_introduce_call_effects)
        {
            return HashSet::new();
        }
        let mut names = HashSet::new();
        let mut visited = HashSet::new();
        let mut argument_bindings = HashMap::new();
        for (index, parameter) in function.params.iter().enumerate() {
            let value = if parameter.rest {
                Expression::Array(
                    arguments
                        .iter()
                        .skip(index)
                        .cloned()
                        .map(ArrayElement::Expression)
                        .collect(),
                )
            } else {
                arguments
                    .get(index)
                    .cloned()
                    .unwrap_or(Expression::Undefined)
            };
            argument_bindings.insert(parameter.name.clone(), value);
        }
        for statement in &function.body {
            let substituted = self.substitute_statement_bindings(statement, &argument_bindings);
            self.collect_statement_call_effect_nonlocal_bindings(
                &substituted,
                Some(&user_function.name),
                &mut names,
                &mut visited,
            );
        }
        for iterator_name in iterator_names {
            let Some(iterated) =
                Self::find_iterator_source_expression_in_statements(&function.body, &iterator_name)
            else {
                continue;
            };
            for candidate in Self::iterator_iterated_value_candidates_in_statements(
                &function.body,
                &iterated,
                0,
            ) {
                let candidate = self.substitute_user_function_argument_bindings(
                    &candidate,
                    user_function,
                    &call_arguments,
                );
                let iterator_call = Expression::Call {
                    callee: Box::new(Expression::Member {
                        object: Box::new(candidate),
                        property: Box::new(symbol_iterator_expression()),
                    }),
                    arguments: Vec::new(),
                };
                let Some(LocalFunctionBinding::User(function_name)) = self
                    .inherited_member_function_bindings(&iterator_call)
                    .into_iter()
                    .find(|binding| binding.property == "return")
                    .map(|binding| binding.binding)
                else {
                    continue;
                };
                names.extend(
                    self.collect_user_function_call_effect_nonlocal_bindings_for_name(
                        &function_name,
                        &mut visited,
                    ),
                );
            }
        }
        names
    }

    pub(in crate::backend::direct_wasm) fn invalidate_user_function_call_effect_nonlocal_bindings_except(
        &mut self,
        user_function: &UserFunction,
        preserved_names: &HashSet<String>,
    ) {
        let names = self
            .collect_user_function_call_effect_nonlocal_bindings(user_function)
            .difference(preserved_names)
            .cloned()
            .collect::<HashSet<_>>();
        if !names.is_empty() {
            let preserved_kinds = names
                .iter()
                .filter_map(|name| {
                    self.lookup_identifier_kind(name)
                        .map(|kind| (name.clone(), kind))
                })
                .collect::<HashMap<_, _>>();
            self.invalidate_static_binding_metadata_for_names_with_preserved_kinds(
                &names,
                &preserved_kinds,
            );
        }
    }
}
