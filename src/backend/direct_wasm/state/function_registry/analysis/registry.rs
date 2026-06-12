use super::*;

#[derive(Default)]
pub(in crate::backend::direct_wasm) struct UserFunctionAnalysisRegistry {
    pub(in crate::backend::direct_wasm) user_function_parameter_analysis:
        UserFunctionParameterAnalysis,
    pub(in crate::backend::direct_wasm) eval_local_function_bindings:
        HashMap<String, HashMap<String, String>>,
    pub(in crate::backend::direct_wasm) user_function_capture_bindings:
        HashMap<String, HashMap<String, String>>,
    /// `with`-scope object chain that was lexically active where an internal
    /// function expression (`__ayy_fnexpr_*` / `__ayy_arrow_*`) appeared.
    /// Function bodies compile out-of-line after the start compile, so the
    /// definition-site chain must be re-seeded when the body is compiled for
    /// identifiers to resolve through the scope objects (object environment
    /// records) instead of falling through to globals.
    pub(in crate::backend::direct_wasm) function_definition_with_scopes:
        HashMap<String, Vec<Expression>>,
}

impl UserFunctionAnalysisRegistry {
    pub(in crate::backend::direct_wasm) fn reset_for_program(&mut self) {
        self.user_function_parameter_analysis.clear();
        self.eval_local_function_bindings.clear();
        self.user_function_capture_bindings.clear();
        self.function_definition_with_scopes.clear();
    }

    pub(in crate::backend::direct_wasm) fn function_definition_with_scopes(
        &self,
        function_name: &str,
    ) -> Option<&Vec<Expression>> {
        self.function_definition_with_scopes.get(function_name)
    }

    pub(in crate::backend::direct_wasm) fn set_function_definition_with_scopes(
        &mut self,
        function_name: &str,
        with_scopes: Vec<Expression>,
    ) {
        self.function_definition_with_scopes
            .insert(function_name.to_string(), with_scopes);
    }

    pub(in crate::backend::direct_wasm) fn set_parameter_analysis(
        &mut self,
        analysis: UserFunctionParameterAnalysis,
    ) {
        self.user_function_parameter_analysis = analysis;
    }

    pub(in crate::backend::direct_wasm) fn parameter_bindings_for(
        &self,
        function_name: &str,
    ) -> PreparedFunctionParameterBindings {
        self.user_function_parameter_analysis
            .bindings_for(function_name)
    }

    pub(in crate::backend::direct_wasm) fn user_function_capture_bindings_snapshot(
        &self,
    ) -> HashMap<String, HashMap<String, String>> {
        self.user_function_capture_bindings.clone()
    }

    pub(in crate::backend::direct_wasm) fn eval_local_function_bindings_snapshot(
        &self,
    ) -> HashMap<String, HashMap<String, String>> {
        self.eval_local_function_bindings.clone()
    }

    pub(in crate::backend::direct_wasm) fn eval_local_function_bindings(
        &self,
        function_name: &str,
    ) -> Option<&HashMap<String, String>> {
        self.eval_local_function_bindings.get(function_name)
    }

    pub(in crate::backend::direct_wasm) fn user_function_capture_bindings(
        &self,
        function_name: &str,
    ) -> Option<&HashMap<String, String>> {
        self.user_function_capture_bindings.get(function_name)
    }

    pub(in crate::backend::direct_wasm) fn record_eval_local_function_binding(
        &mut self,
        function_name: &str,
        binding_name: &str,
        hidden_name: &str,
    ) {
        self.eval_local_function_bindings
            .entry(function_name.to_string())
            .or_default()
            .insert(binding_name.to_string(), hidden_name.to_string());
    }

    pub(in crate::backend::direct_wasm) fn clear_user_function_capture_bindings(&mut self) {
        self.user_function_capture_bindings.clear();
    }

    pub(in crate::backend::direct_wasm) fn set_user_function_capture_bindings(
        &mut self,
        function_name: &str,
        captures: HashMap<String, String>,
    ) {
        self.user_function_capture_bindings
            .insert(function_name.to_string(), captures);
    }
}
