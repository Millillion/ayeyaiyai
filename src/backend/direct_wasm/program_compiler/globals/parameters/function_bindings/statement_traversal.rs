use super::*;

#[path = "statement_traversal/basic.rs"]
mod basic;
#[path = "statement_traversal/branches.rs"]
mod branches;
#[path = "statement_traversal/loops.rs"]
mod loops;

impl DirectWasmCompiler {
    pub(in crate::backend::direct_wasm) fn collect_parameter_bindings_from_statements(
        &self,
        statements: &[Statement],
        aliases: &mut HashMap<String, Option<LocalFunctionBinding>>,
        bindings: &mut HashMap<String, HashMap<String, Option<LocalFunctionBinding>>>,
        array_bindings: &mut HashMap<String, HashMap<String, Option<ArrayValueBinding>>>,
        object_bindings: &mut HashMap<String, HashMap<String, Option<ObjectValueBinding>>>,
    ) {
        self.collect_parameter_bindings_from_statements_in_function(
            statements,
            aliases,
            bindings,
            array_bindings,
            object_bindings,
            None,
        );
    }

    pub(in crate::backend::direct_wasm) fn collect_parameter_bindings_from_statements_in_function(
        &self,
        statements: &[Statement],
        aliases: &mut HashMap<String, Option<LocalFunctionBinding>>,
        bindings: &mut HashMap<String, HashMap<String, Option<LocalFunctionBinding>>>,
        array_bindings: &mut HashMap<String, HashMap<String, Option<ArrayValueBinding>>>,
        object_bindings: &mut HashMap<String, HashMap<String, Option<ObjectValueBinding>>>,
        current_function_name: Option<&str>,
    ) {
        for statement in statements {
            self.collect_parameter_bindings_from_statement_in_function(
                statement,
                aliases,
                bindings,
                array_bindings,
                object_bindings,
                current_function_name,
            );
        }
    }

    pub(in crate::backend::direct_wasm) fn collect_parameter_bindings_from_statement(
        &self,
        statement: &Statement,
        aliases: &mut HashMap<String, Option<LocalFunctionBinding>>,
        bindings: &mut HashMap<String, HashMap<String, Option<LocalFunctionBinding>>>,
        array_bindings: &mut HashMap<String, HashMap<String, Option<ArrayValueBinding>>>,
        object_bindings: &mut HashMap<String, HashMap<String, Option<ObjectValueBinding>>>,
    ) {
        self.collect_parameter_bindings_from_statement_in_function(
            statement,
            aliases,
            bindings,
            array_bindings,
            object_bindings,
            None,
        );
    }

    pub(in crate::backend::direct_wasm) fn collect_parameter_bindings_from_statement_in_function(
        &self,
        statement: &Statement,
        aliases: &mut HashMap<String, Option<LocalFunctionBinding>>,
        bindings: &mut HashMap<String, HashMap<String, Option<LocalFunctionBinding>>>,
        array_bindings: &mut HashMap<String, HashMap<String, Option<ArrayValueBinding>>>,
        object_bindings: &mut HashMap<String, HashMap<String, Option<ObjectValueBinding>>>,
        current_function_name: Option<&str>,
    ) {
        match statement {
            Statement::Declaration { body }
            | Statement::Block { body }
            | Statement::Labeled { body, .. } => self
                .collect_parameter_bindings_from_statements_in_function(
                    body,
                    aliases,
                    bindings,
                    array_bindings,
                    object_bindings,
                    current_function_name,
                ),
            Statement::Var { name, value } | Statement::Let { name, value, .. } => {
                self.handle_binding_assignment_parameter_statement(
                    name,
                    value,
                    aliases,
                    bindings,
                    array_bindings,
                    object_bindings,
                    current_function_name,
                );
            }
            Statement::Assign { name, value } => {
                self.handle_binding_assignment_parameter_statement(
                    name,
                    value,
                    aliases,
                    bindings,
                    array_bindings,
                    object_bindings,
                    current_function_name,
                );
            }
            Statement::Yield { value } | Statement::YieldDelegate { value } => {
                self.collect_parameter_bindings_from_expression_in_function(
                    value,
                    aliases,
                    bindings,
                    array_bindings,
                    object_bindings,
                    current_function_name,
                );
            }
            Statement::AssignMember {
                object,
                property,
                value,
            } => self.handle_assign_member_parameter_statement(
                object,
                property,
                value,
                aliases,
                bindings,
                array_bindings,
                object_bindings,
                current_function_name,
            ),
            Statement::Print { values } => self.handle_print_parameter_statement(
                values,
                aliases,
                bindings,
                array_bindings,
                object_bindings,
                current_function_name,
            ),
            Statement::Expression(expression)
            | Statement::Throw(expression)
            | Statement::Return(expression) => self
                .collect_parameter_bindings_from_expression_in_function(
                    expression,
                    aliases,
                    bindings,
                    array_bindings,
                    object_bindings,
                    current_function_name,
                ),
            Statement::If {
                condition,
                then_branch,
                else_branch,
            } => self.handle_if_parameter_statement(
                condition,
                then_branch,
                else_branch,
                aliases,
                bindings,
                array_bindings,
                object_bindings,
                current_function_name,
            ),
            Statement::While {
                condition,
                body,
                break_hook,
                ..
            } => self.handle_while_parameter_statement(
                condition,
                body,
                break_hook.as_ref(),
                aliases,
                bindings,
                array_bindings,
                object_bindings,
                current_function_name,
            ),
            Statement::DoWhile {
                condition,
                body,
                break_hook,
                ..
            } => self.handle_do_while_parameter_statement(
                condition,
                body,
                break_hook.as_ref(),
                aliases,
                bindings,
                array_bindings,
                object_bindings,
                current_function_name,
            ),
            Statement::For {
                init,
                condition,
                update,
                body,
                break_hook,
                per_iteration_bindings,
                ..
            } => self.handle_for_parameter_statement(
                init,
                condition.as_ref(),
                update.as_ref(),
                body,
                break_hook.as_ref(),
                per_iteration_bindings,
                aliases,
                bindings,
                array_bindings,
                object_bindings,
                current_function_name,
            ),
            Statement::With { object, body } => self.handle_with_parameter_statement(
                object,
                body,
                aliases,
                bindings,
                array_bindings,
                object_bindings,
                current_function_name,
            ),
            Statement::Try {
                body,
                catch_setup,
                catch_binding,
                catch_body,
                ..
            } => self.handle_try_parameter_statement(
                body,
                catch_setup,
                catch_binding.as_ref(),
                catch_body,
                aliases,
                bindings,
                array_bindings,
                object_bindings,
                current_function_name,
            ),
            Statement::Switch {
                discriminant,
                cases,
                bindings: case_bindings,
                ..
            } => self.handle_switch_parameter_statement(
                discriminant,
                cases,
                case_bindings,
                aliases,
                bindings,
                array_bindings,
                object_bindings,
                current_function_name,
            ),
            Statement::Break { .. } | Statement::Continue { .. } => {}
        }
    }
}
