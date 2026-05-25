use super::*;

mod bindings;
mod control_transfer;
mod expression_statements;
mod structured_control;

impl<'a> FunctionCompiler<'a> {
    pub(super) fn with_private_field_initializer_block<T>(
        &mut self,
        enabled: bool,
        f: impl FnOnce(&mut Self) -> DirectResult<T>,
    ) -> DirectResult<T> {
        let previous = self
            .state
            .speculation
            .execution_context
            .private_field_initializer_block;
        if enabled {
            self.state
                .speculation
                .execution_context
                .private_field_initializer_block = true;
        }
        let result = f(self);
        self.state
            .speculation
            .execution_context
            .private_field_initializer_block = previous;
        result
    }

    pub(super) fn with_class_field_initializer_eval_scope<T>(
        &mut self,
        enabled: bool,
        f: impl FnOnce(&mut Self) -> DirectResult<T>,
    ) -> DirectResult<T> {
        let previous = self
            .state
            .speculation
            .execution_context
            .direct_eval_in_class_field_initializer;
        if enabled {
            self.state
                .speculation
                .execution_context
                .direct_eval_in_class_field_initializer = true;
        }
        let result = f(self);
        self.state
            .speculation
            .execution_context
            .direct_eval_in_class_field_initializer = previous;
        result
    }

    pub(super) fn statement_uses_class_field_initializer_eval_rules(
        &self,
        statement: &Statement,
    ) -> bool {
        match statement {
            Statement::Expression(expression) => {
                self.is_class_field_initializer_define_property_call(expression)
            }
            Statement::AssignMember {
                object, property, ..
            } => {
                matches!(object, Expression::This | Expression::Identifier(_))
                    && matches!(property, Expression::String(name) if name.starts_with("__ayy$private$"))
            }
            _ => false,
        }
    }

    fn is_class_field_initializer_define_property_call(&self, expression: &Expression) -> bool {
        let Expression::Call { callee, arguments } = expression else {
            return false;
        };
        let Expression::Member { object, property } = callee.as_ref() else {
            return false;
        };
        if !matches!(object.as_ref(), Expression::Identifier(name) if name == "Object")
            || !matches!(property.as_ref(), Expression::String(name) if name == "defineProperty")
        {
            return false;
        }
        let [
            CallArgument::Expression(target),
            CallArgument::Expression(_property),
            CallArgument::Expression(descriptor),
        ] = arguments.as_slice()
        else {
            return false;
        };
        matches!(target, Expression::This | Expression::Identifier(_))
            && self.descriptor_expression_has_named_field(descriptor, "value")
    }

    pub(in crate::backend::direct_wasm) fn emit_statement(
        &mut self,
        statement: &Statement,
    ) -> DirectResult<()> {
        match statement {
            Statement::Declaration { .. }
            | Statement::Block { .. }
            | Statement::Labeled { .. }
            | Statement::With { .. }
            | Statement::If { .. }
            | Statement::Try { .. }
            | Statement::Switch { .. } => self.emit_structured_statement(statement),
            Statement::Var { .. }
            | Statement::Let { .. }
            | Statement::Assign { .. }
            | Statement::AssignMember { .. } => self.emit_binding_statement(statement),
            Statement::Expression(..) | Statement::Print { .. } => {
                self.emit_expression_statement(statement)
            }
            Statement::While {
                condition,
                body,
                break_hook,
                labels,
            } => {
                let environment = self.snapshot_static_resolution_environment();
                if Self::statement_allows_static_loop_elision(statement)
                    && self.sync_static_executable_statement_tracking_effects_from_environment(
                        statement,
                        environment.clone(),
                    )
                {
                    return Ok(());
                }
                if self
                    .try_emit_static_simple_generator_rest_collection_loop_statement(statement)?
                {
                    self.sync_static_executable_statement_tracking_effects_from_environment(
                        statement,
                        environment,
                    );
                    Ok(())
                } else {
                    self.emit_while(condition, break_hook.as_ref(), labels, body)?;
                    self.sync_static_executable_statement_tracking_effects_from_environment(
                        statement,
                        environment,
                    );
                    Ok(())
                }
            }
            Statement::DoWhile {
                condition,
                body,
                break_hook,
                labels,
            } => {
                let environment = self.snapshot_static_resolution_environment();
                if Self::statement_allows_static_loop_elision(statement)
                    && self.sync_static_executable_statement_tracking_effects_from_environment(
                        statement,
                        environment.clone(),
                    )
                {
                    return Ok(());
                }
                self.emit_do_while(condition, break_hook.as_ref(), labels, body)?;
                self.sync_static_executable_statement_tracking_effects_from_environment(
                    statement,
                    environment,
                );
                Ok(())
            }
            Statement::For {
                init,
                condition,
                update,
                break_hook,
                labels,
                body,
                per_iteration_bindings,
            } => {
                let environment = self.snapshot_static_resolution_environment();
                if Self::statement_allows_static_loop_elision(statement)
                    && self.sync_static_executable_statement_tracking_effects_from_environment(
                        statement,
                        environment.clone(),
                    )
                {
                    return Ok(());
                }
                self.emit_for(
                    labels,
                    init,
                    per_iteration_bindings,
                    condition.as_ref(),
                    update.as_ref(),
                    break_hook.as_ref(),
                    body,
                )?;
                self.sync_static_executable_statement_tracking_effects_from_environment(
                    statement,
                    environment,
                );
                Ok(())
            }
            Statement::Break { .. }
            | Statement::Continue { .. }
            | Statement::Return(..)
            | Statement::Throw(..)
            | Statement::Yield { .. }
            | Statement::YieldDelegate { .. } => self.emit_control_transfer_statement(statement),
        }
    }

    fn statement_allows_static_loop_elision(statement: &Statement) -> bool {
        match statement {
            Statement::Declaration { body } | Statement::Block { body } => {
                body.iter().all(Self::statement_allows_static_loop_elision)
            }
            Statement::Labeled { body, .. } => {
                body.iter().all(Self::statement_allows_static_loop_elision)
            }
            Statement::If {
                then_branch,
                else_branch,
                ..
            } => then_branch
                .iter()
                .chain(else_branch)
                .all(Self::statement_allows_static_loop_elision),
            Statement::For { init, body, .. } => init
                .iter()
                .chain(body)
                .all(Self::statement_allows_static_loop_elision),
            Statement::While { body, .. } | Statement::DoWhile { body, .. } => {
                body.iter().all(Self::statement_allows_static_loop_elision)
            }
            Statement::Var { .. }
            | Statement::Let { .. }
            | Statement::Assign { .. }
            | Statement::AssignMember { .. }
            | Statement::Throw(..) => true,
            Statement::Expression(..)
            | Statement::Print { .. }
            | Statement::With { .. }
            | Statement::Try { .. }
            | Statement::Switch { .. }
            | Statement::Break { .. }
            | Statement::Continue { .. }
            | Statement::Return(..)
            | Statement::Yield { .. }
            | Statement::YieldDelegate { .. } => false,
        }
    }
}
