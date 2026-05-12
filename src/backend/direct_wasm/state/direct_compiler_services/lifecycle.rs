use super::*;

impl DirectWasmCompiler {
    pub(in crate::backend::direct_wasm) fn reset_for_program_compilation(&mut self) {
        self.state.reset_for_program();
    }

    pub(in crate::backend::direct_wasm) fn clear_global_binding_state(&mut self, name: &str) {
        self.state.clear_global_binding_state(name);
    }

    pub(in crate::backend::direct_wasm) fn capture_assigned_nonlocal_binding_results(
        &self,
        assigned_nonlocal_bindings: &HashSet<String>,
    ) -> HashMap<String, Expression> {
        assigned_nonlocal_bindings
            .iter()
            .filter(|name| {
                self.state.global_has_binding(name) || self.state.global_has_implicit_binding(name)
            })
            .map(|name| {
                (
                    name.clone(),
                    self.state
                        .global_value_binding(name)
                        .cloned()
                        .unwrap_or(Expression::Undefined),
                )
            })
            .collect()
    }

    pub(in crate::backend::direct_wasm) fn apply_user_function_parameter_analysis(
        &mut self,
        program: &Program,
    ) {
        let parameter_analysis = self.collect_user_function_parameter_analysis(program);
        self.state
            .set_user_function_parameter_analysis(parameter_analysis);
    }

    pub(in crate::backend::direct_wasm) fn snapshot_global_binding_environment(
        &self,
    ) -> GlobalBindingEnvironment {
        self.state.snapshot_global_binding_environment()
    }

    pub(in crate::backend::direct_wasm) fn with_cloned_global_binding_state<T>(
        &self,
        f: impl FnOnce(&mut HashMap<String, Expression>, &mut HashMap<String, ObjectValueBinding>) -> T,
    ) -> T {
        let environment = self.snapshot_global_binding_environment();
        let mut value_bindings = environment.value_bindings.clone();
        let mut object_bindings = environment.object_bindings.clone();
        f(&mut value_bindings, &mut object_bindings)
    }

    pub(in crate::backend::direct_wasm) fn snapshot_top_level_static_state(
        &self,
    ) -> (
        HashMap<String, Expression>,
        HashMap<String, ObjectValueBinding>,
    ) {
        self.state.snapshot_top_level_static_state()
    }

    pub(in crate::backend::direct_wasm) fn snapshot_global_static_semantics(
        &self,
    ) -> GlobalStaticSemanticsSnapshot {
        self.state.snapshot_global_static_semantics()
    }

    pub(in crate::backend::direct_wasm) fn capture_prepared_module_layout(
        &self,
    ) -> PreparedModuleLayout {
        PreparedModuleLayout {
            user_type_arities: self.state.user_type_arities_snapshot(),
            user_functions: self.state.user_functions().to_vec(),
            global_initial_values: self.global_initial_values(),
        }
    }

    pub(in crate::backend::direct_wasm) fn snapshot_module_data(
        &self,
    ) -> (Vec<(u32, Vec<u8>)>, u32) {
        self.state.snapshot_module_data()
    }

    fn global_initial_values(&self) -> Vec<i32> {
        let base_index = NEXT_PRIVATE_BRAND_GLOBAL_INDEX + 1;
        let next_index = self.next_available_global_index();
        let mut initial_values =
            vec![JS_UNDEFINED_TAG; next_index.saturating_sub(base_index) as usize];

        let mut set_initial_value = |global_index: u32, initial_value: i32| {
            if let Some(slot) = global_index
                .checked_sub(base_index)
                .and_then(|offset| initial_values.get_mut(offset as usize))
            {
                *slot = initial_value;
            }
        };

        for binding in self.state.global_semantics.names.lexical_bindings.values() {
            set_initial_value(binding.initialized_index, 0);
        }

        for binding in self.state.global_semantics.names.implicit_bindings.values() {
            set_initial_value(binding.value_index, 0);
            set_initial_value(binding.present_index, 0);
        }

        for global_index in self
            .state
            .global_semantics
            .values
            .runtime_prototype_bindings
            .values()
            .filter_map(|binding| binding.global_index)
        {
            set_initial_value(global_index, 0);
        }

        initial_values
    }
}
