use super::*;

impl<'a> FunctionCompiler<'a> {
    pub(in crate::backend::direct_wasm) fn synced_prepared_user_function_capture_source_bindings(
        &self,
        prepared: &[PreparedCaptureBinding],
    ) -> HashSet<String> {
        prepared
            .iter()
            .filter_map(|binding| {
                self.user_function_capture_source_is_locally_bound(&binding.source_name)
                    .then_some(binding.source_name.clone())
            })
            .collect()
    }

    pub(in crate::backend::direct_wasm) fn synced_prepared_bound_user_function_capture_source_bindings(
        &self,
        prepared: &[PreparedBoundCaptureBinding],
    ) -> HashSet<String> {
        prepared
            .iter()
            .filter_map(|binding| {
                let source_name = binding.source_binding_name.as_ref()?;
                if Self::capture_slot_member_source_key_parts(source_name).is_some()
                    || source_name.starts_with("__ayy_closure_slot_")
                {
                    Some(binding.capture_name.clone())
                } else {
                    Some(source_name.clone())
                }
            })
            .collect()
    }

    pub(in crate::backend::direct_wasm) fn snapshot_user_function_capture_source_bindings(
        &self,
        prepared: &[PreparedCaptureBinding],
    ) -> HashMap<String, Expression> {
        prepared
            .iter()
            .filter(|binding| {
                self.user_function_capture_source_is_locally_bound(&binding.source_name)
            })
            .map(|binding| {
                (
                    binding.source_name.clone(),
                    self.snapshot_bound_capture_slot_expression(&binding.source_name),
                )
            })
            .collect()
    }

    pub(in crate::backend::direct_wasm) fn sync_snapshot_user_function_call_effect_bindings(
        &mut self,
        names: &HashSet<String>,
        updated_bindings: Option<&HashMap<String, Expression>>,
        fallback_bindings: Option<&HashMap<String, Expression>>,
    ) -> DirectResult<HashSet<String>> {
        let mut unresolved = HashSet::new();
        for name in names {
            let Some(value) = updated_bindings
                .and_then(|bindings| bindings.get(name))
                .or_else(|| fallback_bindings.and_then(|bindings| bindings.get(name)))
            else {
                unresolved.insert(name.clone());
                continue;
            };
            self.sync_bound_capture_source_binding_metadata(name, value)?;
            self.state
                .runtime
                .locals
                .remove_runtime_dynamic_binding(name);
        }
        Ok(unresolved)
    }

    pub(in crate::backend::direct_wasm) fn invalidate_raw_assigned_global_metadata_after_user_call(
        &mut self,
        user_function: &UserFunction,
    ) {
        let names = self
            .collect_user_function_raw_assigned_binding_names(user_function)
            .into_iter()
            .filter(|name| {
                self.global_has_binding(name)
                    || self.global_has_implicit_binding(name)
                    || self.backend.global_has_lexical_binding(name)
            })
            .collect::<HashSet<_>>();
        if names.is_empty() {
            return;
        }
        self.invalidate_static_binding_metadata_for_names_with_preserved_kinds(
            &names,
            &HashMap::new(),
        );
    }
}
