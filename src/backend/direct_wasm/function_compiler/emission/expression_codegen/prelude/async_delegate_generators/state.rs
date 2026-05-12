use super::*;

impl<'a> FunctionCompiler<'a> {
    pub(super) fn persist_async_yield_delegate_generator_snapshot_state(
        &mut self,
        binding_name: &str,
        static_index: Option<usize>,
        snapshot_bindings: Option<HashMap<String, Expression>>,
    ) {
        if std::env::var_os("AYY_TRACE_ASYNC_DELEGATES").is_some() {
            eprintln!(
                "async_delegate_persist binding={} static_index={:?} snapshot={}",
                binding_name,
                static_index,
                snapshot_bindings.is_some()
            );
        }
        if let Some(binding) = self
            .state
            .speculation
            .static_semantics
            .local_array_iterator_binding_mut(binding_name)
            && let IteratorSourceKind::AsyncYieldDelegateGenerator {
                snapshot_bindings: stored_snapshot_bindings,
                ..
            } = &mut binding.source
        {
            binding.static_index = static_index;
            *stored_snapshot_bindings = snapshot_bindings;
        }
    }

    pub(super) fn sync_persisted_async_yield_delegate_generator_snapshot_state(
        &mut self,
        binding_name: &str,
    ) -> DirectResult<()> {
        if let Some(snapshot_bindings) = self
            .state
            .speculation
            .static_semantics
            .local_array_iterator_binding(binding_name)
            .and_then(|binding| match &binding.source {
                IteratorSourceKind::AsyncYieldDelegateGenerator {
                    snapshot_bindings, ..
                } => snapshot_bindings.clone(),
                _ => None,
            })
        {
            self.sync_async_delegate_snapshot_bindings(&snapshot_bindings)?;
        }
        Ok(())
    }
}
