use super::super::*;
use super::StaticStatementExecutor;

pub(in crate::backend::direct_wasm) fn execute_static_statement_block<
    Executor: StaticStatementExecutor + ?Sized,
>(
    executor: &Executor,
    statements: &[Statement],
    environment: &mut Executor::Environment,
) -> Option<StaticStatementControl>
where
    Executor::Environment: StaticTransactionalEnvironment,
{
    let trace_static_exec = std::env::var_os("AYY_TRACE_STATIC_EXEC").is_some();
    for statement in statements {
        if trace_static_exec {
            eprintln!("static_exec:statement={statement:?}");
        }
        macro_rules! trace_unwrap {
            ($value:expr) => {
                match $value {
                    Some(value) => value,
                    None => {
                        if trace_static_exec {
                            eprintln!("static_exec:failed statement={statement:?}");
                        }
                        return None;
                    }
                }
            };
        }
        match statement {
            Statement::Declaration { body } | Statement::Block { body } => {
                if let StaticStatementControl::Return(result) =
                    trace_unwrap!(execute_static_statement_block(executor, body, environment))
                {
                    return Some(StaticStatementControl::Return(result));
                }
            }
            Statement::If {
                condition,
                then_branch,
                else_branch,
            } => {
                let condition = trace_unwrap!(executor.evaluate_condition(condition, environment));
                let branch = match condition {
                    Expression::Bool(true) => then_branch,
                    Expression::Bool(false) => else_branch,
                    _ => return None,
                };
                if let StaticStatementControl::Return(result) = trace_unwrap!(
                    execute_static_statement_block(executor, branch, environment)
                ) {
                    return Some(StaticStatementControl::Return(result));
                }
            }
            Statement::Var { name, value } | Statement::Let { name, value, .. } => {
                trace_unwrap!(executor.initialize_binding(name, value, environment));
            }
            Statement::Assign { name, value } => {
                trace_unwrap!(executor.assign_binding(name, value, environment));
            }
            Statement::AssignMember {
                object,
                property,
                value,
            } => {
                trace_unwrap!(executor.assign_member_binding(object, property, value, environment));
            }
            Statement::Print { values } => {
                trace_unwrap!(executor.execute_print_statement(values, environment));
            }
            Statement::Expression(expression) => {
                trace_unwrap!(executor.execute_expression_statement(expression, environment));
            }
            Statement::Throw(expression) => {
                return executor.execute_throw_statement(expression, environment);
            }
            Statement::Return(expression) => {
                return Some(StaticStatementControl::Return(trace_unwrap!(
                    executor.evaluate_return_value(expression, environment)
                )));
            }
            Statement::Try {
                body,
                catch_setup,
                catch_body,
                ..
            } => {
                return Some(trace_unwrap!(executor.execute_try_statement(
                    body,
                    catch_setup,
                    catch_body,
                    environment
                )));
            }
            _ => {
                if trace_static_exec {
                    eprintln!("static_exec:unsupported statement={statement:?}");
                }
                return None;
            }
        }
    }

    Some(StaticStatementControl::Continue)
}

pub(in crate::backend::direct_wasm) fn execute_static_statement_value<
    Executor: StaticStatementExecutor + ?Sized,
>(
    executor: &Executor,
    statements: &[Statement],
    environment: &mut Executor::Environment,
) -> Option<Option<Expression>>
where
    Executor::Environment: StaticTransactionalEnvironment,
{
    match execute_static_statement_block(executor, statements, environment)? {
        StaticStatementControl::Continue => Some(None),
        StaticStatementControl::Return(result) => Some(Some(result)),
    }
}
