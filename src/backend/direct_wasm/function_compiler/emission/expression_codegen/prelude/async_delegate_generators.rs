use super::*;
mod branches;
mod consume_setup;
mod consume_step;
mod outcomes;
mod setup;
mod state;
mod step_result;
pub(in crate::backend::direct_wasm) use self::setup::InitialDelegateSnapshotBindings;

pub(super) enum AsyncDelegateConsumptionPreparation {
    NotApplicable,
    Outcome(StaticEvalOutcome),
    Ready(PreparedAsyncDelegateConsumption),
}

pub(super) struct PreparedAsyncDelegateConsumption {
    pub(super) binding_name: String,
    pub(super) current_static_index: Option<usize>,
    pub(super) index_local: u32,
    pub(super) property_name: String,
    pub(super) plan: AsyncYieldDelegateGeneratorPlan,
    pub(super) delegate_iterator_name: String,
    pub(super) delegate_next_name: String,
    pub(super) delegate_completion_name: String,
    pub(super) delegate_iterator_expression: Expression,
    pub(super) delegate_completion_expression: Expression,
    pub(super) delegate_snapshot_bindings: Option<HashMap<String, Expression>>,
    pub(super) scoped_snapshot_names: Vec<String>,
    pub(super) snapshot_current_argument: Expression,
    pub(super) step_result_name: String,
    pub(super) promise_value_name: String,
    pub(super) promise_done_name: String,
}

impl<'a> FunctionCompiler<'a> {
    fn current_function_mentions_promise_like_chain(&self) -> bool {
        self.current_function_name().is_some_and(|function_name| {
            self.registered_function_body_mentions_promise_like_chain(function_name)
        })
    }

    fn async_delegate_first_next_rejects(
        &self,
        source: &IteratorSourceKind,
        sent_value: &Expression,
    ) -> bool {
        let IteratorSourceKind::AsyncYieldDelegateGenerator { plan, .. } = source else {
            return false;
        };
        if let Some((is_async, steps, _, _)) =
            self.simple_generator_source_metadata(&plan.delegate_expression)
            && is_async
            && self.first_async_generator_step_rejects_on_next(&steps, sent_value)
        {
            return true;
        }
        let Some((steps, _)) =
            self.resolve_simple_async_yield_delegate_source(&plan.delegate_expression)
        else {
            return false;
        };
        let Some(step) = steps.first() else {
            return false;
        };
        match &step.outcome {
            SimpleGeneratorStepOutcome::Throw(_) => true,
            SimpleGeneratorStepOutcome::Yield(_) | SimpleGeneratorStepOutcome::YieldResult(_) => {
                self.first_async_generator_step_rejects_on_next(&steps, sent_value)
            }
        }
    }

    pub(in crate::backend::direct_wasm) fn emit_static_async_generator_delegate_return_after_caught_value_getter_throw(
        &mut self,
        object: &Expression,
        arguments: &[CallArgument],
    ) -> DirectResult<bool> {
        self.emit_static_async_generator_delegate_protocol_after_caught_value_getter_throw(
            object, arguments, "return",
        )
    }

    pub(in crate::backend::direct_wasm) fn emit_static_async_generator_delegate_throw_after_caught_value_getter_throw(
        &mut self,
        object: &Expression,
        arguments: &[CallArgument],
    ) -> DirectResult<bool> {
        self.emit_static_async_generator_delegate_protocol_after_caught_value_getter_throw(
            object, arguments, "throw",
        )
    }

    fn emit_static_async_generator_delegate_protocol_after_caught_value_getter_throw(
        &mut self,
        object: &Expression,
        arguments: &[CallArgument],
        protocol_name: &str,
    ) -> DirectResult<bool> {
        if !arguments.is_empty() {
            return Ok(false);
        }
        let Some((delegate_expression, _returned_binding)) =
            self.resolve_async_generator_caught_yield_delegate_return_shape(object)
        else {
            return Ok(false);
        };
        let Some(throw_value) = self.resolve_yield_delegate_protocol_value_getter_throw(
            &delegate_expression,
            protocol_name,
        ) else {
            return Ok(false);
        };

        let call_expression = Expression::Call {
            callee: Box::new(Expression::Member {
                object: Box::new(object.clone()),
                property: Box::new(Expression::String(protocol_name.to_string())),
            }),
            arguments: arguments.to_vec(),
        };
        let result_expression = Expression::Object(vec![
            ObjectEntry::Data {
                key: Expression::String("done".to_string()),
                value: Expression::Bool(true),
            },
            ObjectEntry::Data {
                key: Expression::String("value".to_string()),
                value: throw_value,
            },
        ]);
        self.state
            .speculation
            .static_semantics
            .last_bound_user_function_call = Some(BoundUserFunctionCallSnapshot {
            function_name: format!("__ayy_async_generator_delegate_{protocol_name}"),
            source_expression: Some(call_expression),
            result_expression: Some(result_expression),
            prototype_source_expression: None,
            updated_bindings: HashMap::new(),
        });
        crate::backend::direct_wasm::memo::bump_static_state_generation();
        self.push_i32_const(JS_TYPEOF_OBJECT_TAG);
        Ok(true)
    }

    fn resolve_async_generator_caught_yield_delegate_return_shape(
        &self,
        object: &Expression,
    ) -> Option<(Expression, String)> {
        let Expression::Identifier(name) = object else {
            return None;
        };
        let source_expression = self
            .state
            .speculation
            .static_semantics
            .local_value_binding(name)
            .cloned()
            .or_else(|| self.global_value_binding(name).cloned())?;
        let Expression::Call { callee, arguments } = source_expression else {
            return None;
        };
        if !arguments.is_empty() {
            return None;
        }
        let Some(LocalFunctionBinding::User(function_name)) =
            self.resolve_function_binding_from_expression(&callee)
        else {
            return None;
        };
        if !self
            .user_function(&function_name)
            .is_some_and(|function| matches!(function.kind, FunctionKind::AsyncGenerator))
        {
            return None;
        }
        let function = self.resolve_registered_function_declaration(&function_name)?;
        let returned_binding = match function.body.last()? {
            Statement::Return(Expression::Identifier(name)) => name,
            _ => return None,
        };
        function.body.iter().find_map(|statement| {
            let Statement::Try {
                body,
                catch_binding: Some(catch_binding),
                catch_body,
                ..
            } = statement
            else {
                return None;
            };
            let delegate_expression = body.iter().find_map(|statement| match statement {
                Statement::YieldDelegate { value } => Some(value.clone()),
                _ => None,
            })?;
            let catch_assigns_returned_binding = catch_body.iter().any(|statement| {
                matches!(
                    statement,
                    Statement::Assign {
                        name,
                        value: Expression::Identifier(value_name),
                    } if name == returned_binding && value_name == catch_binding
                )
            });
            catch_assigns_returned_binding.then(|| (delegate_expression, returned_binding.clone()))
        })
    }

    fn resolve_yield_delegate_protocol_value_getter_throw(
        &self,
        delegate_expression: &Expression,
        protocol_name: &str,
    ) -> Option<Expression> {
        if !matches!(protocol_name, "return" | "throw") {
            return None;
        }
        let protocol_member = Expression::Member {
            object: Box::new(delegate_expression.clone()),
            property: Box::new(Expression::String(protocol_name.to_string())),
        };
        let Some(LocalFunctionBinding::User(function_name)) =
            self.resolve_function_binding_from_expression(&protocol_member)
        else {
            return None;
        };
        let returned_object = self
            .resolve_static_returned_object_binding_from_user_function_call(&function_name, &[])?;
        let done_property = Expression::String("done".to_string());
        if !object_binding_lookup_value(&returned_object, &done_property).is_some_and(|done| {
            matches!(
                self.materialize_static_expression(done),
                Expression::Bool(false)
            )
        }) {
            return None;
        }
        let value_property = Expression::String("value".to_string());
        let descriptor = object_binding_lookup_descriptor(&returned_object, &value_property)?;
        let getter = descriptor.getter.as_ref().filter(|_| descriptor.has_get)?;
        let Some(LocalFunctionBinding::User(getter_name)) =
            self.resolve_function_binding_from_expression(getter)
        else {
            return None;
        };
        self.resolve_static_user_function_terminal_throw(&getter_name)
    }

    fn resolve_static_user_function_terminal_throw(
        &self,
        function_name: &str,
    ) -> Option<Expression> {
        let function = self.resolve_registered_function_declaration(function_name)?;
        function.body.iter().find_map(|statement| match statement {
            Statement::Throw(value) => Some(value.clone()),
            _ => None,
        })
    }

    fn statement_allows_async_generator_simple_source_probe(statement: &Statement) -> bool {
        match statement {
            Statement::YieldDelegate { value } => matches!(value, Expression::Array(_)),
            Statement::Declaration { body }
            | Statement::Block { body }
            | Statement::Labeled { body, .. }
            | Statement::With { body, .. }
            | Statement::While { body, .. }
            | Statement::DoWhile { body, .. } => {
                Self::statements_allow_async_generator_simple_source_probe(body)
            }
            Statement::If {
                then_branch,
                else_branch,
                ..
            } => {
                Self::statements_allow_async_generator_simple_source_probe(then_branch)
                    && Self::statements_allow_async_generator_simple_source_probe(else_branch)
            }
            Statement::Try {
                body,
                catch_setup,
                catch_body,
                ..
            } => {
                Self::statements_allow_async_generator_simple_source_probe(body)
                    && Self::statements_allow_async_generator_simple_source_probe(catch_setup)
                    && Self::statements_allow_async_generator_simple_source_probe(catch_body)
            }
            Statement::Switch { cases, .. } => cases
                .iter()
                .all(|case| Self::statements_allow_async_generator_simple_source_probe(&case.body)),
            Statement::For { init, body, .. } => {
                Self::statements_allow_async_generator_simple_source_probe(init)
                    && Self::statements_allow_async_generator_simple_source_probe(body)
            }
            Statement::Var { .. }
            | Statement::Let { .. }
            | Statement::Assign { .. }
            | Statement::AssignMember { .. }
            | Statement::Print { .. }
            | Statement::Expression(_)
            | Statement::Throw(_)
            | Statement::Return(_)
            | Statement::Break { .. }
            | Statement::Continue { .. }
            | Statement::Yield { .. } => true,
        }
    }

    fn statements_allow_async_generator_simple_source_probe(statements: &[Statement]) -> bool {
        statements
            .iter()
            .all(Self::statement_allows_async_generator_simple_source_probe)
    }

    fn async_generator_call_allows_simple_source_probe(&self, expression: &Expression) -> bool {
        let Expression::Call { callee, .. } = expression else {
            return true;
        };
        let Some(LocalFunctionBinding::User(function_name)) =
            self.resolve_function_binding_from_expression(callee)
        else {
            return true;
        };
        let Some(user_function) = self.user_function(&function_name) else {
            return true;
        };
        if !matches!(user_function.kind, FunctionKind::AsyncGenerator) {
            return true;
        }
        self.resolve_registered_function_declaration(&function_name)
            .is_some_and(|function| {
                Self::statements_allow_async_generator_simple_source_probe(&function.body)
            })
    }

    pub(in crate::backend::direct_wasm) fn consume_async_yield_delegate_generator_promise_outcome(
        &mut self,
        object: &Expression,
        property_name: &str,
        arguments: &[CallArgument],
    ) -> DirectResult<Option<StaticEvalOutcome>> {
        if matches!(property_name, "return" | "throw")
            && arguments.is_empty()
            && let Some((delegate_expression, _)) =
                self.resolve_async_generator_caught_yield_delegate_return_shape(object)
            && let Some(throw_value) = self.resolve_yield_delegate_protocol_value_getter_throw(
                &delegate_expression,
                property_name,
            )
        {
            return Ok(Some(StaticEvalOutcome::Value(Expression::Object(vec![
                ObjectEntry::Data {
                    key: Expression::String("done".to_string()),
                    value: Expression::Bool(true),
                },
                ObjectEntry::Data {
                    key: Expression::String("value".to_string()),
                    value: throw_value,
                },
            ]))));
        }
        if let Expression::Call { callee, .. } = object
            && let Some(LocalFunctionBinding::User(function_name)) =
                self.resolve_function_binding_from_expression(callee)
            && let Some(user_function) = self.user_function(&function_name)
            && matches!(user_function.kind, FunctionKind::AsyncGenerator)
            && (user_function.has_parameter_defaults()
                || user_function.has_lowered_pattern_parameters()
                || !self
                    .user_function_parameter_iterator_consumption_indices(user_function)
                    .is_empty())
        {
            return Ok(None);
        }
        if let Expression::Identifier(name) = object {
            let binding_name = self
                .resolve_user_function_capture_hidden_name(name)
                .unwrap_or_else(|| name.clone());
            let has_binding = self
                .state
                .speculation
                .static_semantics
                .local_array_iterator_binding(&binding_name)
                .is_some();
            if !has_binding {
                let source_expression = self
                    .state
                    .speculation
                    .static_semantics
                    .local_value_binding(name)
                    .cloned()
                    .or_else(|| self.global_value_binding(name).cloned());
                if let Some(source_expression) = source_expression
                    && !static_expression_matches(&source_expression, object)
                {
                    let source = self.resolve_local_array_iterator_source(&source_expression);
                    if let Some(source @ IteratorSourceKind::AsyncYieldDelegateGenerator { .. }) =
                        source
                    {
                        let can_initialize_missing_index =
                            self.state.speculation.execution_context.top_level_function
                                || self.resolve_current_local_binding(name).is_some();
                        if property_name == "next"
                            && !can_initialize_missing_index
                            && self.current_function_mentions_promise_like_chain()
                            && self.async_delegate_first_next_rejects(
                                &source,
                                &self.promise_argument_expression(arguments, 0),
                            )
                        {
                            if crate::ayy_env_flag!("AYY_TRACE_SIMPLE_GENERATORS") {
                                eprintln!(
                                    "async_delegate_next:nonlocal-closed-after-rejected-first-step object={object:?} binding={binding_name}"
                                );
                            }
                            return Ok(Some(StaticEvalOutcome::Value(
                                Self::simple_generator_iterator_result_object(
                                    true,
                                    Expression::Undefined,
                                ),
                            )));
                        }
                        self.update_local_array_iterator_binding_with_source(
                            &binding_name,
                            Some(source),
                        );
                    }
                }
            }
        }
        if !matches!(object, Expression::Identifier(_))
            && {
                let has_simple_source = self
                    .async_generator_call_allows_simple_source_probe(object)
                    && self.resolve_simple_generator_source(object).is_some();
                !has_simple_source
            }
            && let Some(source @ IteratorSourceKind::AsyncYieldDelegateGenerator { .. }) =
                self.resolve_local_array_iterator_source(object)
        {
            let iterator_name =
                self.allocate_named_hidden_local("async_delegate_iter", StaticValueKind::Object);
            self.update_local_array_iterator_binding_with_source(&iterator_name, Some(source));
            return self.consume_async_yield_delegate_generator_promise_outcome(
                &Expression::Identifier(iterator_name),
                property_name,
                arguments,
            );
        }
        match self.prepare_async_yield_delegate_generator_consumption(
            object,
            property_name,
            arguments,
        )? {
            AsyncDelegateConsumptionPreparation::NotApplicable => Ok(None),
            AsyncDelegateConsumptionPreparation::Outcome(outcome) => Ok(Some(outcome)),
            AsyncDelegateConsumptionPreparation::Ready(prepared) => {
                self.consume_prepared_async_yield_delegate_generator_promise_outcome(prepared)
            }
        }
    }
}
