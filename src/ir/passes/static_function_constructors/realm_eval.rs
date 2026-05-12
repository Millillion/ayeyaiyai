use super::*;

impl StaticFunctionConstructorLowerer {
    pub(super) fn try_lower_static_member_eval_function_expression(
        &mut self,
        callee: &Expression,
        arguments: &[CallArgument],
    ) -> Result<Option<Expression>> {
        if !self.is_member_eval_callee(callee) {
            return Ok(None);
        }

        let Some(argument_source) = first_string_argument(arguments) else {
            return Ok(None);
        };
        let Ok(program) = crate::frontend::parse_script_goal(argument_source) else {
            return Ok(None);
        };

        let Some(parsed_function_name) = function_completion_name(&program) else {
            return Ok(None);
        };
        let Some(function) = program
            .functions
            .into_iter()
            .find(|function| function.name == parsed_function_name)
        else {
            return Ok(None);
        };

        let lowered_function_name = self.fresh_function_name();
        let mut function = function;
        function.name = lowered_function_name.clone();
        let lowered_function = self.lower_realm_eval_function_expression(function)?;
        self.synthetic_functions.push(lowered_function);
        Ok(Some(Expression::Identifier(lowered_function_name)))
    }

    fn lower_realm_eval_function_expression(
        &mut self,
        mut function: FunctionDeclaration,
    ) -> Result<FunctionDeclaration> {
        function.top_level_binding = None;
        function.register_global = false;

        let saved_scopes = std::mem::take(&mut self.scopes);
        self.scopes.push(self.global_scope.clone());
        let result = self.lower_function(function);
        self.scopes = saved_scopes;
        result
    }

    fn is_member_eval_callee(&self, callee: &Expression) -> bool {
        matches!(
            callee,
            Expression::Member { property, .. } if self.is_string_literal(property, "eval")
        )
    }
}

fn first_string_argument(arguments: &[CallArgument]) -> Option<&str> {
    match arguments.first()? {
        CallArgument::Expression(Expression::String(source)) => Some(source),
        _ => None,
    }
}

fn function_completion_name(program: &Program) -> Option<String> {
    let function_names = program
        .functions
        .iter()
        .map(|function| function.name.as_str())
        .collect::<HashSet<_>>();
    let completion = statement_list_completion_expression(&program.statements)?;
    expression_function_completion_name(completion, &function_names)
}

fn statement_list_completion_expression(statements: &[Statement]) -> Option<&Expression> {
    let mut completion = None;
    for statement in statements {
        if let Some(statement_completion) = statement_completion_expression(statement) {
            completion = Some(statement_completion);
        }
    }
    completion
}

fn statement_completion_expression(statement: &Statement) -> Option<&Expression> {
    match statement {
        Statement::Expression(expression) => Some(expression),
        Statement::Block { body } | Statement::Declaration { body } => {
            statement_list_completion_expression(body)
        }
        Statement::Labeled { body, .. }
        | Statement::DoWhile { body, .. }
        | Statement::While { body, .. }
        | Statement::For { body, .. } => statement_list_completion_expression(body),
        Statement::If {
            then_branch,
            else_branch,
            ..
        } => {
            let then_completion = statement_list_completion_expression(then_branch)?;
            let else_completion = statement_list_completion_expression(else_branch)?;
            (then_completion == else_completion).then_some(then_completion)
        }
        _ => None,
    }
}

fn expression_function_completion_name(
    expression: &Expression,
    function_names: &HashSet<&str>,
) -> Option<String> {
    match expression {
        Expression::Identifier(name) if function_names.contains(name.as_str()) => {
            Some(name.clone())
        }
        Expression::Sequence(expressions) => expressions
            .last()
            .and_then(|expression| expression_function_completion_name(expression, function_names)),
        Expression::Conditional {
            then_expression,
            else_expression,
            ..
        } => {
            let then_name = expression_function_completion_name(then_expression, function_names)?;
            let else_name = expression_function_completion_name(else_expression, function_names)?;
            (then_name == else_name).then_some(then_name)
        }
        _ => None,
    }
}
