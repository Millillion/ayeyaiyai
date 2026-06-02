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
        if !matches!(object, Expression::Identifier(_))
            && self.resolve_simple_generator_source(object).is_none()
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
