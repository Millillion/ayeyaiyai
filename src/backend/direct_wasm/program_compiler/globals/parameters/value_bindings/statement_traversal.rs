use super::*;

impl DirectWasmCompiler {
    pub(in crate::backend::direct_wasm) fn collect_parameter_value_bindings_from_statements(
        &self,
        statements: &[Statement],
        aliases: &mut HashMap<String, Option<LocalFunctionBinding>>,
        bindings: &mut HashMap<String, HashMap<String, Option<Expression>>>,
    ) {
        self.collect_parameter_value_bindings_from_statements_in_function(
            statements, aliases, bindings, None,
        );
    }

    pub(in crate::backend::direct_wasm) fn collect_parameter_value_bindings_from_statements_in_function(
        &self,
        statements: &[Statement],
        aliases: &mut HashMap<String, Option<LocalFunctionBinding>>,
        bindings: &mut HashMap<String, HashMap<String, Option<Expression>>>,
        current_function_name: Option<&str>,
    ) {
        for statement in statements {
            self.collect_parameter_value_bindings_from_statement_in_function(
                statement,
                aliases,
                bindings,
                current_function_name,
            );
        }
    }

    pub(in crate::backend::direct_wasm) fn collect_parameter_value_bindings_from_statement(
        &self,
        statement: &Statement,
        aliases: &mut HashMap<String, Option<LocalFunctionBinding>>,
        bindings: &mut HashMap<String, HashMap<String, Option<Expression>>>,
    ) {
        self.collect_parameter_value_bindings_from_statement_in_function(
            statement, aliases, bindings, None,
        );
    }

    pub(in crate::backend::direct_wasm) fn collect_parameter_value_bindings_from_statement_in_function(
        &self,
        statement: &Statement,
        aliases: &mut HashMap<String, Option<LocalFunctionBinding>>,
        bindings: &mut HashMap<String, HashMap<String, Option<Expression>>>,
        current_function_name: Option<&str>,
    ) {
        match statement {
            Statement::Declaration { body }
            | Statement::Block { body }
            | Statement::Labeled { body, .. }
            | Statement::With { body, .. } => {
                self.collect_parameter_value_bindings_from_statements_in_function(
                    body,
                    aliases,
                    bindings,
                    current_function_name,
                );
            }
            Statement::Var { name, value } | Statement::Let { name, value, .. } => {
                self.collect_parameter_value_bindings_from_expression_in_function(
                    value,
                    aliases,
                    bindings,
                    current_function_name,
                );
                aliases.insert(
                    name.clone(),
                    self.resolve_function_binding_from_expression_with_aliases(value, aliases),
                );
            }
            Statement::Assign { name, value } => {
                self.collect_parameter_value_bindings_from_expression_in_function(
                    value,
                    aliases,
                    bindings,
                    current_function_name,
                );
                aliases.insert(
                    name.clone(),
                    self.resolve_function_binding_from_expression_with_aliases(value, aliases),
                );
            }
            Statement::AssignMember {
                object,
                property,
                value,
            } => {
                self.collect_parameter_value_bindings_from_expression_in_function(
                    object,
                    aliases,
                    bindings,
                    current_function_name,
                );
                self.collect_parameter_value_bindings_from_expression_in_function(
                    property,
                    aliases,
                    bindings,
                    current_function_name,
                );
                self.collect_parameter_value_bindings_from_expression_in_function(
                    value,
                    aliases,
                    bindings,
                    current_function_name,
                );
            }
            Statement::Print { values } => {
                for value in values {
                    self.collect_parameter_value_bindings_from_expression_in_function(
                        value,
                        aliases,
                        bindings,
                        current_function_name,
                    );
                }
            }
            Statement::Expression(expression)
            | Statement::Throw(expression)
            | Statement::Return(expression)
            | Statement::Yield { value: expression }
            | Statement::YieldDelegate { value: expression } => {
                self.collect_parameter_value_bindings_from_expression_in_function(
                    expression,
                    aliases,
                    bindings,
                    current_function_name,
                );
            }
            Statement::If {
                condition,
                then_branch,
                else_branch,
            } => {
                self.collect_parameter_value_bindings_from_expression_in_function(
                    condition,
                    aliases,
                    bindings,
                    current_function_name,
                );
                let baseline_aliases = aliases.clone();
                let mut then_aliases = baseline_aliases.clone();
                let mut else_aliases = baseline_aliases.clone();
                self.collect_parameter_value_bindings_from_statements_in_function(
                    then_branch,
                    &mut then_aliases,
                    bindings,
                    current_function_name,
                );
                self.collect_parameter_value_bindings_from_statements_in_function(
                    else_branch,
                    &mut else_aliases,
                    bindings,
                    current_function_name,
                );
                *aliases = self
                    .merge_aliases_for_branches(&baseline_aliases, &[&then_aliases, &else_aliases]);
            }
            Statement::Try {
                body,
                catch_binding,
                catch_setup,
                catch_body,
            } => {
                let baseline_aliases = aliases.clone();
                let mut body_aliases = baseline_aliases.clone();
                self.collect_parameter_value_bindings_from_statements_in_function(
                    body,
                    &mut body_aliases,
                    bindings,
                    current_function_name,
                );

                let mut catch_aliases = baseline_aliases.clone();
                if let Some(binding) = catch_binding {
                    catch_aliases.insert(binding.clone(), None);
                }
                self.collect_parameter_value_bindings_from_statements_in_function(
                    catch_setup,
                    &mut catch_aliases,
                    bindings,
                    current_function_name,
                );
                self.collect_parameter_value_bindings_from_statements_in_function(
                    catch_body,
                    &mut catch_aliases,
                    bindings,
                    current_function_name,
                );
                *aliases = self.merge_aliases_for_branches(
                    &baseline_aliases,
                    &[&body_aliases, &catch_aliases],
                );
            }
            Statement::Switch {
                discriminant,
                cases,
                ..
            } => {
                self.collect_parameter_value_bindings_from_expression_in_function(
                    discriminant,
                    aliases,
                    bindings,
                    current_function_name,
                );
                let baseline_aliases = aliases.clone();
                let mut branch_aliases = Vec::new();
                for case in cases {
                    let mut case_aliases = baseline_aliases.clone();
                    if let Some(test) = &case.test {
                        self.collect_parameter_value_bindings_from_expression_in_function(
                            test,
                            &mut case_aliases,
                            bindings,
                            current_function_name,
                        );
                    }
                    self.collect_parameter_value_bindings_from_statements_in_function(
                        &case.body,
                        &mut case_aliases,
                        bindings,
                        current_function_name,
                    );
                    branch_aliases.push(case_aliases);
                }
                let branch_refs = branch_aliases.iter().collect::<Vec<_>>();
                *aliases = self.merge_aliases_for_branches(&baseline_aliases, &branch_refs);
            }
            Statement::For {
                init,
                condition,
                update,
                break_hook,
                body,
                ..
            } => {
                self.collect_parameter_value_bindings_from_statements_in_function(
                    init,
                    aliases,
                    bindings,
                    current_function_name,
                );
                if let Some(condition) = condition {
                    self.collect_parameter_value_bindings_from_expression_in_function(
                        condition,
                        aliases,
                        bindings,
                        current_function_name,
                    );
                }
                if let Some(update) = update {
                    self.collect_parameter_value_bindings_from_expression_in_function(
                        update,
                        aliases,
                        bindings,
                        current_function_name,
                    );
                }
                if let Some(break_hook) = break_hook {
                    self.collect_parameter_value_bindings_from_expression_in_function(
                        break_hook,
                        aliases,
                        bindings,
                        current_function_name,
                    );
                }
                let baseline_aliases = aliases.clone();
                let mut body_aliases = baseline_aliases.clone();
                self.collect_parameter_value_bindings_from_statements_in_function(
                    body,
                    &mut body_aliases,
                    bindings,
                    current_function_name,
                );
                *aliases = self.merge_aliases_for_optional_body(&baseline_aliases, &body_aliases);
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
                self.collect_parameter_value_bindings_from_expression_in_function(
                    condition,
                    aliases,
                    bindings,
                    current_function_name,
                );
                if let Some(break_hook) = break_hook {
                    self.collect_parameter_value_bindings_from_expression_in_function(
                        break_hook,
                        aliases,
                        bindings,
                        current_function_name,
                    );
                }
                let baseline_aliases = aliases.clone();
                let mut body_aliases = baseline_aliases.clone();
                self.collect_parameter_value_bindings_from_statements_in_function(
                    body,
                    &mut body_aliases,
                    bindings,
                    current_function_name,
                );
                *aliases = self.merge_aliases_for_optional_body(&baseline_aliases, &body_aliases);
            }
            Statement::Break { .. } | Statement::Continue { .. } => {}
        }
    }
}
