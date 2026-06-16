use super::*;

pub(in crate::backend::direct_wasm) trait StaticUserFunctionBindingExecutor:
    StaticExpressionExecutor
{
    fn resolve_static_user_function_declaration(
        &self,
        function_name: &str,
    ) -> Option<&FunctionDeclaration>;

    fn resolve_static_user_function_body(&self, function_name: &str) -> Option<&[Statement]> {
        Some(
            &self
                .resolve_static_user_function_declaration(function_name)?
                .body,
        )
    }

    fn resolve_static_user_function_metadata(&self, function_name: &str) -> Option<&UserFunction>;

    fn substitute_static_user_function_argument_bindings(
        &self,
        expression: &Expression,
        user_function: &UserFunction,
        arguments: &[CallArgument],
    ) -> Expression;

    fn static_user_function_argument_requires_runtime(&self, expression: &Expression) -> bool;

    fn materialize_inline_static_user_function_return(
        &self,
        expression: &Expression,
        environment: &mut Self::Environment,
    ) -> Option<Expression>
    where
        Self::Environment: StaticTransactionalEnvironment,
    {
        let mut environment = environment.fork_environment();
        self.materialize_expression(expression, &mut environment)
    }

    fn inline_static_user_function_binding(
        &self,
        function_name: &str,
        arguments: &[CallArgument],
        environment: &mut Self::Environment,
    ) -> Option<Expression>
    where
        Self::Environment: StaticTransactionalEnvironment,
    {
        let user_function = self.resolve_static_user_function_metadata(function_name)?;
        let summary = user_function.inline_summary.as_ref()?;
        if !summary.effects.is_empty() {
            return None;
        }
        let return_value = summary.return_value.as_ref()?;
        if !inline_summary_side_effect_free_expression(return_value) {
            return None;
        }
        let substituted = self.substitute_static_user_function_argument_bindings(
            return_value,
            user_function,
            arguments,
        );
        self.materialize_inline_static_user_function_return(&substituted, environment)
    }
}

pub(in crate::backend::direct_wasm) trait StaticUserFunctionBindingSource {
    fn static_user_function_declaration(&self, function_name: &str)
    -> Option<&FunctionDeclaration>;

    fn static_user_function_metadata(&self, function_name: &str) -> Option<&UserFunction>;

    fn substitute_static_user_function_arguments(
        &self,
        expression: &Expression,
        user_function: &UserFunction,
        arguments: &[CallArgument],
    ) -> Expression;

    fn static_user_function_argument_requires_runtime(&self, _expression: &Expression) -> bool {
        false
    }
}

impl<T> StaticUserFunctionBindingExecutor for T
where
    T: StaticExpressionExecutor + StaticUserFunctionBindingSource + ?Sized,
{
    fn resolve_static_user_function_declaration(
        &self,
        function_name: &str,
    ) -> Option<&FunctionDeclaration> {
        self.static_user_function_declaration(function_name)
    }

    fn resolve_static_user_function_metadata(&self, function_name: &str) -> Option<&UserFunction> {
        self.static_user_function_metadata(function_name)
    }

    fn substitute_static_user_function_argument_bindings(
        &self,
        expression: &Expression,
        user_function: &UserFunction,
        arguments: &[CallArgument],
    ) -> Expression {
        self.substitute_static_user_function_arguments(expression, user_function, arguments)
    }

    fn static_user_function_argument_requires_runtime(&self, expression: &Expression) -> bool {
        StaticUserFunctionBindingSource::static_user_function_argument_requires_runtime(
            self, expression,
        )
    }
}

fn static_user_function_statement_contains_loop(statement: &Statement) -> bool {
    match statement {
        Statement::For { .. } | Statement::While { .. } | Statement::DoWhile { .. } => true,
        Statement::Declaration { body }
        | Statement::Block { body }
        | Statement::Labeled { body, .. }
        | Statement::With { body, .. } => static_user_function_body_contains_loop(body),
        Statement::If {
            then_branch,
            else_branch,
            ..
        } => {
            static_user_function_body_contains_loop(then_branch)
                || static_user_function_body_contains_loop(else_branch)
        }
        Statement::Try {
            body,
            catch_setup,
            catch_body,
            ..
        } => {
            static_user_function_body_contains_loop(body)
                || static_user_function_body_contains_loop(catch_setup)
                || static_user_function_body_contains_loop(catch_body)
        }
        Statement::Switch { cases, .. } => cases
            .iter()
            .any(|case| static_user_function_body_contains_loop(&case.body)),
        _ => false,
    }
}

fn static_user_function_body_contains_loop(statements: &[Statement]) -> bool {
    statements
        .iter()
        .any(static_user_function_statement_contains_loop)
}

pub(in crate::backend::direct_wasm) fn execute_static_function_body<Executor, Environment>(
    executor: &Executor,
    statements: &[Statement],
    environment: &mut Environment,
) -> Option<Expression>
where
    Executor: StaticStatementExecutor<Environment = Environment> + ?Sized,
    Environment: StaticFunctionExecutionEnvironment + StaticTransactionalEnvironment,
{
    environment.clear_function_locals();
    execute_static_statement_value(executor, statements, environment)
        .map(|result| result.unwrap_or(Expression::Undefined))
}

pub(in crate::backend::direct_wasm) fn execute_static_function_body_in_environment<
    Executor,
    Environment,
>(
    executor: &Executor,
    statements: &[Statement],
    environment: &mut Environment,
    effect_mode: StaticFunctionEffectMode,
) -> Option<Expression>
where
    Executor: StaticStatementExecutor<Environment = Environment> + ?Sized,
    Environment: StaticTransactionalEnvironment,
{
    let mut function_environment = environment.fork_environment();
    let result = execute_static_function_body(executor, statements, &mut function_environment)?;
    if matches!(effect_mode, StaticFunctionEffectMode::Commit) {
        environment.commit_environment(function_environment);
    }
    Some(result)
}

pub(in crate::backend::direct_wasm) fn execute_static_user_function_binding_in_environment<
    Executor,
>(
    executor: &Executor,
    binding: &LocalFunctionBinding,
    arguments: &[CallArgument],
    environment: &mut Executor::Environment,
    effect_mode: StaticFunctionEffectMode,
) -> Option<Expression>
where
    Executor: StaticUserFunctionBindingExecutor + ?Sized,
    Executor::Environment: StaticTransactionalEnvironment,
{
    let LocalFunctionBinding::User(function_name) = binding else {
        return None;
    };
    // The synthesized spread-iterate helper exists precisely to run the
    // iterator protocol at runtime (observable GetIterator/next() calls and
    // their errors); never fold it statically.
    if function_name == crate::ir::hir::SPREAD_ITERATE_HELPER_NAME {
        return None;
    }
    if let Some(result) =
        executor.inline_static_user_function_binding(function_name, arguments, environment)
    {
        return Some(result);
    }
    if arguments.iter().any(|argument| {
        executor.static_user_function_argument_requires_runtime(argument.expression())
    }) {
        return None;
    }
    let statements = executor.resolve_static_user_function_body(function_name)?;
    if static_user_function_body_contains_loop(statements) {
        return None;
    }
    execute_static_function_body_in_environment(executor, statements, environment, effect_mode)
}
