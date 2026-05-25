use super::*;

impl<'a> FunctionCompiler<'a> {
    pub(in crate::backend::direct_wasm) fn resolve_function_binding_static_return_expression(
        &self,
        binding: &LocalFunctionBinding,
        arguments: &[Expression],
    ) -> Option<Expression> {
        let LocalFunctionBinding::User(function_name) = binding else {
            return None;
        };
        let user_function = self.user_function(function_name)?;
        if self.user_function_mentions_private_member_access(user_function)
            || self.user_function_mentions_direct_eval(user_function)
        {
            return None;
        }
        let summary = user_function.inline_summary.as_ref()?;
        let return_value = summary.return_value.as_ref()?;
        let call_arguments = arguments
            .iter()
            .cloned()
            .map(CallArgument::Expression)
            .collect::<Vec<_>>();
        Some(self.substitute_user_function_argument_bindings(
            return_value,
            user_function,
            &call_arguments,
        ))
    }

    pub(in crate::backend::direct_wasm) fn resolve_function_binding_static_return_bool(
        &self,
        binding: &LocalFunctionBinding,
        arguments: &[Expression],
    ) -> Option<bool> {
        self.resolve_function_binding_static_return_expression(binding, arguments)
            .and_then(|expression| self.resolve_static_boolean_expression(&expression))
    }

    pub(in crate::backend::direct_wasm) fn resolve_function_binding_static_return_object_binding(
        &self,
        binding: &LocalFunctionBinding,
        arguments: &[Expression],
    ) -> Option<ObjectValueBinding> {
        let expression =
            self.resolve_function_binding_static_return_expression(binding, arguments)?;
        self.resolve_object_binding_from_expression(&expression)
    }

    pub(in crate::backend::direct_wasm) fn emit_function_binding_side_effects_with_arguments(
        &mut self,
        binding: &LocalFunctionBinding,
        arguments: &[Expression],
    ) -> DirectResult<()> {
        self.with_suspended_with_scopes(|compiler| match binding {
            LocalFunctionBinding::User(function_name) => {
                let Some(user_function) = compiler.user_function(function_name).cloned() else {
                    return Ok(());
                };
                if compiler
                    .emit_inline_user_function_summary_with_arguments(&user_function, arguments)?
                {
                    compiler.state.emission.output.instructions.push(0x1a);
                } else {
                    let call_arguments = arguments
                        .iter()
                        .cloned()
                        .map(CallArgument::Expression)
                        .collect::<Vec<_>>();
                    compiler.emit_user_function_call(&user_function, &call_arguments)?;
                    compiler.state.emission.output.instructions.push(0x1a);
                }
                Ok(())
            }
            LocalFunctionBinding::Builtin(function_name) => {
                let call_arguments = arguments
                    .iter()
                    .cloned()
                    .map(CallArgument::Expression)
                    .collect::<Vec<_>>();
                if compiler.emit_builtin_call(function_name, &call_arguments)? {
                    compiler.state.emission.output.instructions.push(0x1a);
                }
                Ok(())
            }
        })
    }

    pub(in crate::backend::direct_wasm) fn emit_function_binding_effect_statements_with_arguments(
        &mut self,
        binding: &LocalFunctionBinding,
        arguments: &[Expression],
    ) -> DirectResult<()> {
        self.with_suspended_with_scopes(|compiler| match binding {
            LocalFunctionBinding::User(function_name) => {
                let Some(user_function) = compiler.user_function(function_name).cloned() else {
                    return Ok(());
                };
                let Some(function) = compiler
                    .resolve_registered_function_declaration(&user_function.name)
                    .cloned()
                else {
                    return Ok(());
                };
                let Some((_, effect_statements)) = function.body.split_last() else {
                    return Ok(());
                };
                let effect_statements = effect_statements.to_vec();
                let call_arguments = arguments
                    .iter()
                    .cloned()
                    .map(CallArgument::Expression)
                    .collect::<Vec<_>>();

                compiler.with_user_function_execution_context(&user_function, |compiler| {
                    for statement in &effect_statements {
                        if !compiler.emit_inline_user_function_effect_statement(
                            statement,
                            &user_function,
                            &call_arguments,
                        )? {
                            return Ok(());
                        }
                    }
                    Ok(())
                })
            }
            LocalFunctionBinding::Builtin(_) => Ok(()),
        })
    }

    pub(in crate::backend::direct_wasm) fn function_binding_defaults_to_undefined(
        &self,
        binding: &LocalFunctionBinding,
    ) -> bool {
        let LocalFunctionBinding::User(function_name) = binding else {
            return false;
        };
        self.user_function(function_name)
            .and_then(|user_function| user_function.inline_summary.as_ref())
            .is_some_and(|summary| summary.return_value.is_none())
    }

    pub(in crate::backend::direct_wasm) fn function_binding_always_throws(
        &self,
        binding: &LocalFunctionBinding,
    ) -> bool {
        let LocalFunctionBinding::User(function_name) = binding else {
            return false;
        };
        self.resolve_registered_function_declaration(function_name)
            .is_some_and(|function| matches!(function.body.as_slice(), [Statement::Throw(_)]))
    }

    pub(in crate::backend::direct_wasm) fn resolve_terminal_function_outcome_from_binding(
        &self,
        binding: &LocalFunctionBinding,
        arguments: &[Expression],
    ) -> Option<StaticEvalOutcome> {
        if let Some(outcome) = self.resolve_static_function_outcome_from_binding_with_context(
            binding,
            &arguments
                .iter()
                .cloned()
                .map(CallArgument::Expression)
                .collect::<Vec<_>>(),
            self.current_function_name(),
        ) {
            return Some(outcome);
        }
        let LocalFunctionBinding::User(function_name) = binding else {
            return None;
        };
        let user_function = self.user_function(function_name)?;
        if self.user_function_mentions_private_member_access(user_function)
            || self.user_function_mentions_direct_eval(user_function)
        {
            return None;
        }
        let function = self.resolve_registered_function_declaration(function_name)?;
        if function.body.is_empty() {
            return Some(StaticEvalOutcome::Value(Expression::Undefined));
        }
        let terminal_statement = function.body.last()?;
        let call_arguments = arguments
            .iter()
            .cloned()
            .map(CallArgument::Expression)
            .collect::<Vec<_>>();
        match terminal_statement {
            Statement::Return(expression) => Some(StaticEvalOutcome::Value(
                self.substitute_user_function_argument_bindings(
                    expression,
                    user_function,
                    &call_arguments,
                ),
            )),
            Statement::Throw(expression) => Some(StaticEvalOutcome::Throw(
                StaticThrowValue::Value(self.substitute_user_function_argument_bindings(
                    expression,
                    user_function,
                    &call_arguments,
                )),
            )),
            Statement::Expression(expression) => self
                .resolve_terminal_expression_throw_value(
                    &self.substitute_user_function_argument_bindings(
                        expression,
                        user_function,
                        &call_arguments,
                    ),
                )
                .map(|throw_value| StaticEvalOutcome::Throw(StaticThrowValue::Value(throw_value)))
                .or_else(|| {
                    self.resolve_terminal_user_function_outcome_with_static_execution(
                        user_function,
                        arguments,
                    )
                }),
            _ => self.resolve_terminal_user_function_outcome_with_static_execution(
                user_function,
                arguments,
            ),
        }
    }

    fn resolve_terminal_user_function_outcome_with_static_execution(
        &self,
        user_function: &UserFunction,
        arguments: &[Expression],
    ) -> Option<StaticEvalOutcome> {
        let call_arguments = arguments
            .iter()
            .cloned()
            .map(CallArgument::Expression)
            .collect::<Vec<_>>();
        let this_binding =
            if self.should_box_sloppy_function_this(user_function, &Expression::Undefined) {
                Expression::This
            } else {
                Expression::Undefined
            };
        let mut execution = self.prepare_static_user_function_execution(
            &user_function.name,
            user_function,
            &call_arguments,
            &this_binding,
            None,
            HashMap::new(),
            |statement| statement,
        )?;
        let (terminal_statement, prefix) = execution.substituted_body.split_last()?;
        let prefix_result =
            self.execute_static_statements_with_state(prefix, &mut execution.environment);
        if let Some(return_value) = prefix_result? {
            return Some(StaticEvalOutcome::Value(return_value));
        }

        match terminal_statement {
            Statement::Return(expression) => Some(StaticEvalOutcome::Value(
                self.resolve_static_expression_value_with_state(
                    expression,
                    &mut execution.environment,
                ),
            )),
            Statement::Throw(expression) => Some(StaticEvalOutcome::Throw(
                StaticThrowValue::Value(self.resolve_static_expression_value_with_state(
                    expression,
                    &mut execution.environment,
                )),
            )),
            Statement::Expression(expression) => self
                .resolve_terminal_expression_throw_value_with_state(
                    expression,
                    &mut execution.environment,
                )
                .map(|throw_value| StaticEvalOutcome::Throw(StaticThrowValue::Value(throw_value))),
            Statement::AssignMember {
                object,
                property,
                value,
            } => self
                .resolve_terminal_expression_throw_value_with_state(
                    object,
                    &mut execution.environment,
                )
                .or_else(|| {
                    self.resolve_terminal_expression_throw_value_with_state(
                        property,
                        &mut execution.environment,
                    )
                })
                .or_else(|| {
                    self.resolve_terminal_expression_throw_value_with_state(
                        value,
                        &mut execution.environment,
                    )
                })
                .map(|throw_value| StaticEvalOutcome::Throw(StaticThrowValue::Value(throw_value))),
            _ => None,
        }
    }

    pub(in crate::backend::direct_wasm) fn resolve_static_expression_value_with_state(
        &self,
        expression: &Expression,
        environment: &mut StaticResolutionEnvironment,
    ) -> Expression {
        self.evaluate_static_expression_with_state(expression, environment)
            .or_else(|| self.materialize_static_expression_with_state(expression, environment))
            .unwrap_or_else(|| expression.clone())
    }

    fn resolve_static_binary_throw_value(
        &self,
        op: BinaryOp,
        left: &Expression,
        right: &Expression,
    ) -> Option<Expression> {
        let current_function_name = self.current_function_name();
        let outcome = match op {
            BinaryOp::Add => self.resolve_static_addition_outcome_with_context(
                left,
                right,
                current_function_name,
            ),
            BinaryOp::Subtract
            | BinaryOp::Multiply
            | BinaryOp::Divide
            | BinaryOp::Modulo
            | BinaryOp::Exponentiate
            | BinaryOp::BitwiseAnd
            | BinaryOp::BitwiseOr
            | BinaryOp::BitwiseXor
            | BinaryOp::LeftShift
            | BinaryOp::RightShift
            | BinaryOp::UnsignedRightShift => self
                .resolve_static_numeric_binary_outcome_with_context(
                    op,
                    left,
                    right,
                    current_function_name,
                ),
            BinaryOp::LessThan
            | BinaryOp::LessThanOrEqual
            | BinaryOp::GreaterThan
            | BinaryOp::GreaterThanOrEqual => self.resolve_static_relational_outcome_with_context(
                op,
                left,
                right,
                current_function_name,
            ),
            _ => None,
        }?;
        match outcome {
            StaticEvalOutcome::Throw(throw_value) => {
                self.resolve_static_throw_value_expression(&throw_value)
            }
            StaticEvalOutcome::Value(_) => None,
        }
    }

    pub(in crate::backend::direct_wasm) fn resolve_terminal_expression_throw_value_with_state(
        &self,
        expression: &Expression,
        environment: &mut StaticResolutionEnvironment,
    ) -> Option<Expression> {
        match expression {
            Expression::Call { callee, arguments } => {
                let callee = match callee.as_ref() {
                    Expression::Identifier(name) => {
                        environment.binding(name).cloned().unwrap_or_else(|| {
                            self.materialize_static_expression_with_state(callee, environment)
                                .unwrap_or_else(|| {
                                    self.resolve_static_expression_value_with_state(
                                        callee,
                                        environment,
                                    )
                                })
                        })
                    }
                    _ => self
                        .materialize_static_expression_with_state(callee, environment)
                        .unwrap_or_else(|| {
                            self.resolve_static_expression_value_with_state(callee, environment)
                        }),
                };
                let argument_values = arguments
                    .iter()
                    .map(|argument| match argument {
                        CallArgument::Expression(expression) => {
                            Some(self.resolve_static_expression_value_with_state(
                                expression,
                                environment,
                            ))
                        }
                        CallArgument::Spread(_) => None,
                    })
                    .collect::<Option<Vec<_>>>()?;
                let binding = self.resolve_function_binding_from_expression(&callee)?;
                match self
                    .resolve_terminal_function_outcome_from_binding(&binding, &argument_values)?
                {
                    StaticEvalOutcome::Throw(throw_value) => {
                        self.resolve_static_throw_value_expression(&throw_value)
                    }
                    _ => None,
                }
            }
            Expression::AssignMember {
                object,
                property,
                value,
            } => self
                .resolve_terminal_expression_throw_value_with_state(object, environment)
                .or_else(|| {
                    self.resolve_terminal_expression_throw_value_with_state(property, environment)
                })
                .or_else(|| {
                    self.resolve_terminal_expression_throw_value_with_state(value, environment)
                }),
            Expression::AssignSuperMember { property, value } => self
                .resolve_terminal_expression_throw_value_with_state(property, environment)
                .or_else(|| {
                    self.resolve_terminal_expression_throw_value_with_state(value, environment)
                }),
            Expression::Assign { value, .. } => {
                self.resolve_terminal_expression_throw_value_with_state(value, environment)
            }
            Expression::Member { object, property } => self
                .resolve_terminal_expression_throw_value_with_state(object, environment)
                .or_else(|| {
                    self.resolve_terminal_expression_throw_value_with_state(property, environment)
                }),
            Expression::SuperMember { property } => {
                self.resolve_terminal_expression_throw_value_with_state(property, environment)
            }
            Expression::Binary { op, left, right } => self
                .resolve_terminal_expression_throw_value_with_state(left, environment)
                .or_else(|| {
                    self.resolve_terminal_expression_throw_value_with_state(right, environment)
                })
                .or_else(|| {
                    let left_value =
                        self.resolve_static_expression_value_with_state(left, environment);
                    let right_value =
                        self.resolve_static_expression_value_with_state(right, environment);
                    self.resolve_static_binary_throw_value(*op, &left_value, &right_value)
                }),
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => self
                .resolve_terminal_expression_throw_value_with_state(condition, environment)
                .or_else(|| {
                    self.resolve_terminal_expression_throw_value_with_state(
                        then_expression,
                        environment,
                    )
                })
                .or_else(|| {
                    self.resolve_terminal_expression_throw_value_with_state(
                        else_expression,
                        environment,
                    )
                }),
            Expression::Sequence(expressions) => {
                for expression in expressions {
                    if let Some(throw_value) = self
                        .resolve_terminal_expression_throw_value_with_state(expression, environment)
                    {
                        return Some(throw_value);
                    }
                }
                None
            }
            Expression::Unary {
                op: UnaryOp::Plus | UnaryOp::Negate,
                expression,
            } => self
                .resolve_terminal_expression_throw_value_with_state(expression, environment)
                .or_else(|| {
                    let value =
                        self.resolve_static_expression_value_with_state(expression, environment);
                    self.resolve_terminal_expression_throw_value(&value)
                        .or_else(|| {
                            let plan = self
                                .resolve_ordinary_to_primitive_plan_with_state(
                                    expression,
                                    environment,
                                )
                                .or_else(|| self.resolve_ordinary_to_primitive_plan(&value))?;
                            plan.steps.iter().find_map(|step| match &step.outcome {
                                StaticEvalOutcome::Throw(throw_value) => {
                                    self.resolve_static_throw_value_expression(throw_value)
                                }
                                StaticEvalOutcome::Value(_) => None,
                            })
                        })
                }),
            Expression::Await(expression)
            | Expression::EnumerateKeys(expression)
            | Expression::GetIterator(expression)
            | Expression::IteratorClose(expression)
            | Expression::Unary { expression, .. } => {
                self.resolve_terminal_expression_throw_value_with_state(expression, environment)
            }
            _ => self.resolve_terminal_expression_throw_value(
                &self.resolve_static_expression_value_with_state(expression, environment),
            ),
        }
    }

    pub(in crate::backend::direct_wasm) fn resolve_terminal_expression_throw_value(
        &self,
        expression: &Expression,
    ) -> Option<Expression> {
        match expression {
            Expression::Call { .. } => {
                match self.resolve_terminal_call_expression_outcome(expression)? {
                    StaticEvalOutcome::Throw(throw_value) => {
                        self.resolve_static_throw_value_expression(&throw_value)
                    }
                    _ => None,
                }
            }
            Expression::AssignMember {
                object,
                property,
                value,
            } => self
                .resolve_terminal_expression_throw_value(object)
                .or_else(|| self.resolve_terminal_expression_throw_value(property))
                .or_else(|| self.resolve_terminal_expression_throw_value(value)),
            Expression::AssignSuperMember { property, value } => self
                .resolve_terminal_expression_throw_value(property)
                .or_else(|| self.resolve_terminal_expression_throw_value(value)),
            Expression::Binary { op, left, right } => self
                .resolve_terminal_expression_throw_value(left)
                .or_else(|| self.resolve_terminal_expression_throw_value(right))
                .or_else(|| self.resolve_static_binary_throw_value(*op, left, right)),
            Expression::Sequence(expressions) => {
                for expression in expressions {
                    if let Some(throw_value) =
                        self.resolve_terminal_expression_throw_value(expression)
                    {
                        return Some(throw_value);
                    }
                }
                None
            }
            Expression::Unary {
                op: UnaryOp::Plus | UnaryOp::Negate,
                expression,
            } => self
                .resolve_terminal_expression_throw_value(expression)
                .or_else(|| {
                    let plan = self.resolve_ordinary_to_primitive_plan(expression)?;
                    plan.steps.iter().find_map(|step| match &step.outcome {
                        StaticEvalOutcome::Throw(throw_value) => {
                            self.resolve_static_throw_value_expression(throw_value)
                        }
                        StaticEvalOutcome::Value(_) => None,
                    })
                }),
            Expression::Await(expression)
            | Expression::EnumerateKeys(expression)
            | Expression::GetIterator(expression)
            | Expression::IteratorClose(expression)
            | Expression::Unary { expression, .. } => {
                self.resolve_terminal_expression_throw_value(expression)
            }
            _ => None,
        }
    }

    pub(in crate::backend::direct_wasm) fn resolve_static_throw_value_expression(
        &self,
        throw_value: &StaticThrowValue,
    ) -> Option<Expression> {
        match throw_value {
            StaticThrowValue::Value(throw_value) => Some(throw_value.clone()),
            StaticThrowValue::NamedError(name) => Some(Expression::Call {
                callee: Box::new(Expression::Identifier((*name).to_string())),
                arguments: vec![],
            }),
        }
    }

    pub(in crate::backend::direct_wasm) fn resolve_terminal_call_expression_outcome(
        &self,
        expression: &Expression,
    ) -> Option<StaticEvalOutcome> {
        let Expression::Call { callee, arguments } = expression else {
            return None;
        };
        if matches!(
            callee.as_ref(),
            Expression::Member { property, .. }
                if matches!(property.as_ref(), Expression::String(name) if name == "call" || name == "apply")
        ) {
            return None;
        }
        if let Expression::Member { object, property } = callee.as_ref()
            && matches!(property.as_ref(), Expression::String(name) if name == "then" || name == "catch")
        {
            let object_is_async_user_call = if let Expression::Call {
                callee: object_callee,
                ..
            } = object.as_ref()
            {
                self.resolve_function_binding_from_expression(object_callee)
                    .is_some_and(|binding| {
                        let LocalFunctionBinding::User(function_name) = binding else {
                            return false;
                        };
                        self.user_function(&function_name)
                            .is_some_and(|function| function.is_async())
                    })
            } else {
                false
            };
            if Self::call_is_promise_like_chain(object) || object_is_async_user_call {
                return None;
            }
        }
        if matches!(
            callee.as_ref(),
            Expression::Identifier(name) if name == "eval"
        ) && matches!(
            self.resolve_function_binding_from_expression(callee),
            Some(LocalFunctionBinding::Builtin(function_name)) if function_name == "eval"
        ) && let Some(outcome) = self.resolve_static_direct_eval_outcome(arguments)
        {
            return Some(outcome);
        }
        let binding = self.resolve_function_binding_from_expression(callee)?;
        let argument_expressions = arguments
            .iter()
            .map(|argument| match argument {
                CallArgument::Expression(expression) => Some(expression.clone()),
                CallArgument::Spread(_) => None,
            })
            .collect::<Option<Vec<_>>>()?;
        self.resolve_terminal_function_outcome_from_binding(&binding, &argument_expressions)
    }
}
