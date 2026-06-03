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
        let has_simple_source = self.async_generator_call_allows_simple_source_probe(object)
            && self.resolve_simple_generator_source(object).is_some();
        if !matches!(object, Expression::Identifier(_))
            && !has_simple_source
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
