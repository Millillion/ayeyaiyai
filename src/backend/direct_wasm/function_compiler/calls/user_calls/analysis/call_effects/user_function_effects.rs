use super::*;

impl<'a> FunctionCompiler<'a> {
    fn sync_static_with_scope_member_assignment_effect(
        &mut self,
        object: &Expression,
        name: &str,
        value: &Expression,
    ) {
        if !self.scope_object_has_binding_property(object, name) {
            return;
        }
        let property = Expression::String(name.to_string());
        let materialized_value = self.reference_preserving_static_value_expression(value);
        self.update_member_function_assignment_binding(object, &property, value);
        if let Expression::Identifier(owner_name) = object {
            let object_binding = self
                .backend
                .global_semantics
                .values
                .object_bindings
                .entry(owner_name.clone())
                .or_insert_with(empty_object_value_binding);
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
                .or_insert_with(empty_object_value_binding);
            object_binding_set_property(shared_object_binding, property, materialized_value);
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
        let mut names = HashSet::new();
        let mut visited = HashSet::new();
        for iterator_name in iterator_names {
            let Some(iterated) =
                Self::find_iterator_source_expression_in_statements(&function.body, &iterator_name)
            else {
                continue;
            };
            let iterated = self.substitute_user_function_argument_bindings(
                &iterated,
                user_function,
                &call_arguments,
            );
            let iterator_call = Expression::Call {
                callee: Box::new(Expression::Member {
                    object: Box::new(iterated),
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
