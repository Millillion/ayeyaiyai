use super::*;

impl<'a> FunctionCompiler<'a> {
    pub(in crate::backend::direct_wasm) fn expression_is_active_with_scope_object(
        &self,
        expression: &Expression,
    ) -> bool {
        self.state
            .emission
            .lexical_scopes
            .with_scopes
            .iter()
            .any(|scope| static_expression_matches(scope, expression))
    }

    pub(in crate::backend::direct_wasm) fn with_suspended_with_scopes_if_active_scope_object<T>(
        &mut self,
        expression: &Expression,
        f: impl FnOnce(&mut Self) -> DirectResult<T>,
    ) -> DirectResult<T> {
        if self.expression_is_active_with_scope_object(expression) {
            self.with_suspended_with_scopes(f)
        } else {
            f(self)
        }
    }

    pub(in crate::backend::direct_wasm) fn with_suspended_with_scopes<T>(
        &mut self,
        f: impl FnOnce(&mut Self) -> DirectResult<T>,
    ) -> DirectResult<T> {
        let previous_with_scopes = self.state.take_with_scopes();
        let result = f(self);
        self.state.restore_with_scopes(previous_with_scopes);
        result
    }
}
