use super::*;

impl<'a> FunctionCompiler<'a> {
    pub(in crate::backend::direct_wasm) fn collect_preserved_binding_kinds_from_statement(
        &self,
        invalidated_bindings: &HashSet<String>,
        preserved_kinds: &mut HashMap<String, StaticValueKind>,
        blocked_bindings: &mut HashSet<String>,
        statement: &Statement,
    ) {
        match statement {
            Statement::Declaration { body }
            | Statement::Block { body }
            | Statement::Labeled { body, .. } => {
                for statement in body {
                    self.collect_preserved_binding_kinds_from_statement(
                        invalidated_bindings,
                        preserved_kinds,
                        blocked_bindings,
                        statement,
                    );
                }
            }
            Statement::Var { name, value } | Statement::Assign { name, value } => {
                // A var/assignment inside an active `with` scope may route to
                // the scope object's property instead of the binding, so the
                // assigned value's kind must not be claimed for the binding
                // (the binding may keep its pre-loop value entirely).
                let candidate = if self
                    .resolve_with_scope_binding_for_capture_source(name)
                    .is_some()
                {
                    None
                } else {
                    self.preserved_expression_kind(preserved_kinds, value)
                };
                self.merge_preserved_binding_kind(
                    invalidated_bindings,
                    preserved_kinds,
                    blocked_bindings,
                    name,
                    candidate,
                );
                self.collect_preserved_binding_kinds_from_expression(
                    invalidated_bindings,
                    preserved_kinds,
                    blocked_bindings,
                    value,
                );
            }
            Statement::Let { name, value, .. } => {
                self.merge_preserved_binding_kind(
                    invalidated_bindings,
                    preserved_kinds,
                    blocked_bindings,
                    name,
                    self.preserved_expression_kind(preserved_kinds, value),
                );
                self.collect_preserved_binding_kinds_from_expression(
                    invalidated_bindings,
                    preserved_kinds,
                    blocked_bindings,
                    value,
                );
            }
            Statement::AssignMember {
                object,
                property,
                value,
            } => {
                self.collect_preserved_binding_kinds_from_expression(
                    invalidated_bindings,
                    preserved_kinds,
                    blocked_bindings,
                    object,
                );
                self.collect_preserved_binding_kinds_from_expression(
                    invalidated_bindings,
                    preserved_kinds,
                    blocked_bindings,
                    property,
                );
                self.collect_preserved_binding_kinds_from_expression(
                    invalidated_bindings,
                    preserved_kinds,
                    blocked_bindings,
                    value,
                );
            }
            Statement::Expression(expression)
            | Statement::Throw(expression)
            | Statement::Return(expression)
            | Statement::Yield { value: expression }
            | Statement::YieldDelegate { value: expression } => {
                self.collect_preserved_binding_kinds_from_expression(
                    invalidated_bindings,
                    preserved_kinds,
                    blocked_bindings,
                    expression,
                );
            }
            Statement::Print { values } => {
                for value in values {
                    self.collect_preserved_binding_kinds_from_expression(
                        invalidated_bindings,
                        preserved_kinds,
                        blocked_bindings,
                        value,
                    );
                }
            }
            Statement::With { object, body } => {
                self.collect_preserved_binding_kinds_from_expression(
                    invalidated_bindings,
                    preserved_kinds,
                    blocked_bindings,
                    object,
                );
                // Assignments under a nested `with` may route to the scope
                // object rather than the binding; this pre-pass cannot model
                // that scope, so block kind preservation for those names.
                let mut nested_assigned_names = HashSet::new();
                for statement in body {
                    collect_assigned_binding_names_from_statement(
                        statement,
                        &mut nested_assigned_names,
                    );
                }
                for name in &nested_assigned_names {
                    self.merge_preserved_binding_kind(
                        invalidated_bindings,
                        preserved_kinds,
                        blocked_bindings,
                        name,
                        None,
                    );
                }
                for statement in body {
                    self.collect_preserved_binding_kinds_from_statement(
                        invalidated_bindings,
                        preserved_kinds,
                        blocked_bindings,
                        statement,
                    );
                }
            }
            Statement::If {
                condition,
                then_branch,
                else_branch,
            } => {
                self.collect_preserved_binding_kinds_from_expression(
                    invalidated_bindings,
                    preserved_kinds,
                    blocked_bindings,
                    condition,
                );
                for statement in then_branch {
                    self.collect_preserved_binding_kinds_from_statement(
                        invalidated_bindings,
                        preserved_kinds,
                        blocked_bindings,
                        statement,
                    );
                }
                for statement in else_branch {
                    self.collect_preserved_binding_kinds_from_statement(
                        invalidated_bindings,
                        preserved_kinds,
                        blocked_bindings,
                        statement,
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
                    self.collect_preserved_binding_kinds_from_statement(
                        invalidated_bindings,
                        preserved_kinds,
                        blocked_bindings,
                        statement,
                    );
                }
                for statement in catch_setup {
                    self.collect_preserved_binding_kinds_from_statement(
                        invalidated_bindings,
                        preserved_kinds,
                        blocked_bindings,
                        statement,
                    );
                }
                for statement in catch_body {
                    self.collect_preserved_binding_kinds_from_statement(
                        invalidated_bindings,
                        preserved_kinds,
                        blocked_bindings,
                        statement,
                    );
                }
            }
            Statement::Switch {
                discriminant,
                cases,
                ..
            } => {
                self.collect_preserved_binding_kinds_from_expression(
                    invalidated_bindings,
                    preserved_kinds,
                    blocked_bindings,
                    discriminant,
                );
                for case in cases {
                    if let Some(test) = &case.test {
                        self.collect_preserved_binding_kinds_from_expression(
                            invalidated_bindings,
                            preserved_kinds,
                            blocked_bindings,
                            test,
                        );
                    }
                    for statement in &case.body {
                        self.collect_preserved_binding_kinds_from_statement(
                            invalidated_bindings,
                            preserved_kinds,
                            blocked_bindings,
                            statement,
                        );
                    }
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
                    self.collect_preserved_binding_kinds_from_statement(
                        invalidated_bindings,
                        preserved_kinds,
                        blocked_bindings,
                        statement,
                    );
                }
                if let Some(condition) = condition {
                    self.collect_preserved_binding_kinds_from_expression(
                        invalidated_bindings,
                        preserved_kinds,
                        blocked_bindings,
                        condition,
                    );
                }
                if let Some(update) = update {
                    self.collect_preserved_binding_kinds_from_expression(
                        invalidated_bindings,
                        preserved_kinds,
                        blocked_bindings,
                        update,
                    );
                }
                if let Some(break_hook) = break_hook {
                    self.collect_preserved_binding_kinds_from_expression(
                        invalidated_bindings,
                        preserved_kinds,
                        blocked_bindings,
                        break_hook,
                    );
                }
                for statement in body {
                    self.collect_preserved_binding_kinds_from_statement(
                        invalidated_bindings,
                        preserved_kinds,
                        blocked_bindings,
                        statement,
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
                self.collect_preserved_binding_kinds_from_expression(
                    invalidated_bindings,
                    preserved_kinds,
                    blocked_bindings,
                    condition,
                );
                if let Some(break_hook) = break_hook {
                    self.collect_preserved_binding_kinds_from_expression(
                        invalidated_bindings,
                        preserved_kinds,
                        blocked_bindings,
                        break_hook,
                    );
                }
                for statement in body {
                    self.collect_preserved_binding_kinds_from_statement(
                        invalidated_bindings,
                        preserved_kinds,
                        blocked_bindings,
                        statement,
                    );
                }
            }
            Statement::Break { .. } | Statement::Continue { .. } => {}
        }
    }
}
