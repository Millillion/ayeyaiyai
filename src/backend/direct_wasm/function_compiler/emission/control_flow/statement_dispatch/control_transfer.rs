use super::*;

impl<'a> FunctionCompiler<'a> {
    pub(super) fn emit_control_transfer_statement(
        &mut self,
        statement: &Statement,
    ) -> DirectResult<()> {
        match statement {
            Statement::Break { label } => {
                let target_index = if let Some(label) = label.as_ref() {
                    match self.find_labeled_break(label)? {
                        Some(index) => index,
                        None => return Ok(()),
                    }
                } else {
                    match self
                        .state
                        .emission
                        .control_flow
                        .break_stack
                        .len()
                        .checked_sub(1)
                    {
                        Some(index) => index,
                        None => return Ok(()),
                    }
                };

                for context_index in
                    (target_index..self.state.emission.control_flow.break_stack.len()).rev()
                {
                    let break_hook = self.break_hook_for_target(
                        self.state.emission.control_flow.break_stack[context_index].break_target,
                    )?;
                    if let Some(break_hook) = break_hook {
                        self.emit_numeric_expression(&break_hook)?;
                        self.state.emission.output.instructions.push(0x1a);
                    }
                }

                let break_target =
                    self.state.emission.control_flow.break_stack[target_index].break_target;
                self.push_br(self.relative_depth(break_target));
                Ok(())
            }
            Statement::Continue { label } => {
                if label.is_some() {
                    let label = label
                        .as_ref()
                        .expect("labeled continue branch should include label");
                    let target_index = match self.find_labeled_loop_index(label)? {
                        Some(index) => index,
                        None => return Ok(()),
                    };
                    if target_index == self.state.emission.control_flow.loop_stack.len() - 1 {
                        let continue_target = {
                            let Some(loop_context) =
                                self.state.emission.control_flow.loop_stack.last()
                            else {
                                return Ok(());
                            };
                            loop_context.continue_target
                        };
                        self.push_br(self.relative_depth(continue_target));
                        return Ok(());
                    }

                    for loop_index in
                        (target_index + 1..self.state.emission.control_flow.loop_stack.len()).rev()
                    {
                        let break_target =
                            self.state.emission.control_flow.loop_stack[loop_index].break_target;
                        if let Some(break_hook) = self.break_hook_for_target(break_target)? {
                            self.emit_numeric_expression(&break_hook)?;
                            self.state.emission.output.instructions.push(0x1a);
                        }
                    }

                    let target =
                        self.state.emission.control_flow.loop_stack[target_index].continue_target;
                    self.push_br(self.relative_depth(target));
                    return Ok(());
                }
                let Some(loop_context) = self.state.emission.control_flow.loop_stack.last() else {
                    return Ok(());
                };
                let continue_target = loop_context.continue_target;
                self.push_br(self.relative_depth(continue_target));
                Ok(())
            }
            Statement::Return(expression) => {
                if !self.state.runtime.behavior.allow_return {
                    self.emit_numeric_expression(expression)?;
                    self.state.emission.output.instructions.push(0x1a);
                    self.push_i32_const(JS_UNDEFINED_TAG);
                    return Ok(());
                }
                if self.emit_self_tail_call_restart(expression)? {
                    return Ok(());
                }
                self.emit_numeric_expression(expression)?;
                for loop_index in (0..self.state.emission.control_flow.loop_stack.len()).rev() {
                    let break_target =
                        self.state.emission.control_flow.loop_stack[loop_index].break_target;
                    if let Some(break_hook) = self.break_hook_for_target(break_target)? {
                        self.emit_numeric_expression(&break_hook)?;
                        self.state.emission.output.instructions.push(0x1a);
                    }
                }
                self.clear_local_throw_state();
                self.clear_global_throw_state();
                self.state.emission.output.instructions.push(0x0f);
                Ok(())
            }
            Statement::Throw(expression) => {
                self.emit_numeric_expression(expression)?;
                self.push_local_set(self.state.runtime.throws.throw_value_local);
                self.push_i32_const(1);
                self.push_local_set(self.state.runtime.throws.throw_tag_local);
                self.emit_throw_from_locals()
            }
            Statement::Yield { value } => {
                self.emit_numeric_expression(value)?;
                self.state.emission.output.instructions.push(0x00);
                Ok(())
            }
            Statement::YieldDelegate { value } => {
                self.emit_numeric_expression(value)?;
                self.state.emission.output.instructions.push(0x00);
                Ok(())
            }
            _ => unreachable!("emit_control_transfer_statement called with non-control statement"),
        }
    }

    fn emit_self_tail_call_restart(&mut self, expression: &Expression) -> DirectResult<bool> {
        let Some(restart_target) = self.state.emission.control_flow.tail_call_restart_target else {
            return Ok(false);
        };
        if !self.state.emission.control_flow.try_stack.is_empty()
            || !self.state.parameters.arguments_slots.is_empty()
        {
            return Ok(false);
        }

        let Expression::Call { callee, arguments } = expression else {
            return Ok(false);
        };
        let Expression::Identifier(callee_name) = callee.as_ref() else {
            return Ok(false);
        };

        let current_function_name = self.current_function_name().map(str::to_owned);
        let declaration_references_this_or_new_target = self
            .current_user_function_declaration()
            .is_some_and(|declaration| statements_reference_this_or_new_target(&declaration.body));
        let self_binding = self
            .current_user_function_declaration()
            .and_then(|declaration| declaration.self_binding.clone());
        let is_self_call = current_function_name.as_deref() == Some(callee_name.as_str())
            || self_binding.as_deref() == Some(callee_name.as_str());
        if !is_self_call {
            return Ok(false);
        }

        let Some(function) = self.current_user_function().cloned() else {
            return Ok(false);
        };
        if function.is_async()
            || function.is_generator()
            || function.has_parameter_defaults()
            || function.has_lowered_pattern_parameters()
            || !function.extra_argument_indices.is_empty()
            || declaration_references_this_or_new_target
            || arguments.len() != function.params.len()
            || arguments
                .iter()
                .any(|argument| matches!(argument, CallArgument::Spread(_)))
        {
            return Ok(false);
        }

        let Some(parameter_locals) = function
            .params
            .iter()
            .map(|parameter| self.state.runtime.locals.get(parameter).copied())
            .collect::<Option<Vec<_>>>()
        else {
            return Ok(false);
        };
        let argument_expressions = arguments
            .iter()
            .map(|argument| match argument {
                CallArgument::Expression(expression) => Some(expression.clone()),
                CallArgument::Spread(_) => None,
            })
            .collect::<Option<Vec<_>>>()
            .expect("spread arguments were rejected above");

        let argument_locals = argument_expressions
            .iter()
            .map(|_| self.allocate_temp_local())
            .collect::<Vec<_>>();
        for (argument, local) in argument_expressions
            .iter()
            .zip(argument_locals.iter().copied())
        {
            self.emit_numeric_expression(argument)?;
            self.push_local_set(local);
        }
        for (local, parameter_local) in argument_locals.iter().copied().zip(parameter_locals) {
            self.push_local_get(local);
            self.push_local_set(parameter_local);
        }
        for parameter_name in &function.params {
            self.state
                .clear_local_static_binding_metadata(parameter_name);
        }
        if let Some(actual_argument_count_local) = self.state.parameters.actual_argument_count_local
        {
            self.push_i32_const(argument_expressions.len() as i32);
            self.push_local_set(actual_argument_count_local);
        }
        self.clear_local_throw_state();
        self.clear_global_throw_state();
        self.push_br(self.relative_depth(restart_target));
        Ok(true)
    }
}

fn statements_reference_this_or_new_target(statements: &[Statement]) -> bool {
    statements
        .iter()
        .any(statement_references_this_or_new_target)
}

fn statement_references_this_or_new_target(statement: &Statement) -> bool {
    match statement {
        Statement::Declaration { body }
        | Statement::Block { body }
        | Statement::Labeled { body, .. }
        | Statement::With { body, .. }
        | Statement::DoWhile { body, .. }
        | Statement::While { body, .. } => statements_reference_this_or_new_target(body),
        Statement::Var { value, .. }
        | Statement::Let { value, .. }
        | Statement::Assign { value, .. }
        | Statement::Expression(value)
        | Statement::Throw(value)
        | Statement::Return(value)
        | Statement::Yield { value }
        | Statement::YieldDelegate { value } => expression_references_this_or_new_target(value),
        Statement::AssignMember {
            object,
            property,
            value,
        } => {
            expression_references_this_or_new_target(object)
                || expression_references_this_or_new_target(property)
                || expression_references_this_or_new_target(value)
        }
        Statement::Print { values } => values.iter().any(expression_references_this_or_new_target),
        Statement::If {
            condition,
            then_branch,
            else_branch,
        } => {
            expression_references_this_or_new_target(condition)
                || statements_reference_this_or_new_target(then_branch)
                || statements_reference_this_or_new_target(else_branch)
        }
        Statement::Try {
            body,
            catch_setup,
            catch_body,
            ..
        } => {
            statements_reference_this_or_new_target(body)
                || statements_reference_this_or_new_target(catch_setup)
                || statements_reference_this_or_new_target(catch_body)
        }
        Statement::Switch {
            discriminant,
            cases,
            ..
        } => {
            expression_references_this_or_new_target(discriminant)
                || cases
                    .iter()
                    .any(|case| statements_reference_this_or_new_target(&case.body))
        }
        Statement::For {
            init,
            condition,
            update,
            break_hook,
            body,
            ..
        } => {
            statements_reference_this_or_new_target(init)
                || condition
                    .as_ref()
                    .is_some_and(expression_references_this_or_new_target)
                || update
                    .as_ref()
                    .is_some_and(expression_references_this_or_new_target)
                || break_hook
                    .as_ref()
                    .is_some_and(expression_references_this_or_new_target)
                || statements_reference_this_or_new_target(body)
        }
        Statement::Break { .. } | Statement::Continue { .. } => false,
    }
}

fn expression_references_this_or_new_target(expression: &Expression) -> bool {
    match expression {
        Expression::This | Expression::NewTarget | Expression::SuperMember { .. } => true,
        Expression::Array(elements) => elements.iter().any(|element| match element {
            ArrayElement::Expression(expression) | ArrayElement::Spread(expression) => {
                expression_references_this_or_new_target(expression)
            }
        }),
        Expression::Object(entries) => entries.iter().any(|entry| match entry {
            ObjectEntry::Data { key, value } => {
                expression_references_this_or_new_target(key)
                    || expression_references_this_or_new_target(value)
            }
            ObjectEntry::Getter { key, getter } => {
                expression_references_this_or_new_target(key)
                    || expression_references_this_or_new_target(getter)
            }
            ObjectEntry::Setter { key, setter } => {
                expression_references_this_or_new_target(key)
                    || expression_references_this_or_new_target(setter)
            }
            ObjectEntry::Spread(expression) => expression_references_this_or_new_target(expression),
        }),
        Expression::Member { object, property } => {
            expression_references_this_or_new_target(object)
                || expression_references_this_or_new_target(property)
        }
        Expression::Assign { value, .. }
        | Expression::Await(value)
        | Expression::EnumerateKeys(value)
        | Expression::GetIterator(value)
        | Expression::IteratorClose(value)
        | Expression::Unary {
            expression: value, ..
        } => expression_references_this_or_new_target(value),
        Expression::AssignMember {
            object,
            property,
            value,
        } => {
            expression_references_this_or_new_target(object)
                || expression_references_this_or_new_target(property)
                || expression_references_this_or_new_target(value)
        }
        Expression::AssignSuperMember { .. } | Expression::SuperCall { .. } => true,
        Expression::Binary { left, right, .. } => {
            expression_references_this_or_new_target(left)
                || expression_references_this_or_new_target(right)
        }
        Expression::Conditional {
            condition,
            then_expression,
            else_expression,
        } => {
            expression_references_this_or_new_target(condition)
                || expression_references_this_or_new_target(then_expression)
                || expression_references_this_or_new_target(else_expression)
        }
        Expression::Sequence(expressions) => expressions
            .iter()
            .any(expression_references_this_or_new_target),
        Expression::Call { callee, arguments } | Expression::New { callee, arguments } => {
            expression_references_this_or_new_target(callee)
                || arguments
                    .iter()
                    .any(|argument| expression_references_this_or_new_target(argument.expression()))
        }
        Expression::Number(_)
        | Expression::BigInt(_)
        | Expression::String(_)
        | Expression::Bool(_)
        | Expression::Null
        | Expression::Undefined
        | Expression::Identifier(_)
        | Expression::Sent
        | Expression::Update { .. } => false,
    }
}
