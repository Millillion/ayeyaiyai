use super::super::*;
use super::StaticStatementExecutor;

const STATIC_LOOP_ITERATION_LIMIT: usize = 4096;

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
    let trace_static_loop = std::env::var_os("AYY_TRACE_STATIC_LOOP").is_some();
    let trace_static_condition = std::env::var_os("AYY_TRACE_STATIC_CONDITION").is_some();
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
            Statement::Declaration { body }
            | Statement::Block { body }
            | Statement::Labeled { body, .. } => {
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
                if trace_static_condition {
                    eprintln!("static_exec:if condition_start={condition:?}");
                }
                let condition = trace_unwrap!(executor.evaluate_condition(condition, environment));
                if trace_static_condition {
                    eprintln!("static_exec:if condition_value={condition:?}");
                }
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
            Statement::Var { name, value } => {
                trace_unwrap!(executor.declare_var_binding(name, value, environment));
            }
            Statement::Let { name, value, .. } => {
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
            Statement::For {
                labels,
                init,
                per_iteration_bindings,
                condition,
                update,
                break_hook,
                body,
            } => {
                if !labels.is_empty() || !per_iteration_bindings.is_empty() || break_hook.is_some()
                {
                    if trace_static_exec {
                        eprintln!("static_exec:unsupported for metadata statement={statement:?}");
                    }
                    return None;
                }

                if let StaticStatementControl::Return(result) =
                    trace_unwrap!(execute_static_statement_block(executor, init, environment))
                {
                    return Some(StaticStatementControl::Return(result));
                }

                let mut completed = false;
                for iteration in 0..STATIC_LOOP_ITERATION_LIMIT {
                    if (trace_static_exec || trace_static_loop) && iteration % 128 == 0 {
                        eprintln!("static_exec:for iteration={iteration}");
                    }
                    if let Some(condition) = condition {
                        if trace_static_condition {
                            eprintln!("static_exec:for condition_start={condition:?}");
                        }
                        match trace_unwrap!(executor.evaluate_condition(condition, environment)) {
                            Expression::Bool(true) => {}
                            Expression::Bool(false) => {
                                completed = true;
                                break;
                            }
                            _ => return None,
                        }
                    }

                    if let StaticStatementControl::Return(result) =
                        trace_unwrap!(execute_static_statement_block(executor, body, environment))
                    {
                        return Some(StaticStatementControl::Return(result));
                    }

                    if let Some(update) = update {
                        trace_unwrap!(executor.execute_expression_statement(update, environment));
                    }
                }
                if !completed {
                    return None;
                }
            }
            Statement::While {
                labels,
                condition,
                break_hook,
                body,
            } => {
                if !labels.is_empty() || break_hook.is_some() {
                    if trace_static_exec {
                        eprintln!("static_exec:unsupported while metadata statement={statement:?}");
                    }
                    return None;
                }

                let mut completed = false;
                for _ in 0..STATIC_LOOP_ITERATION_LIMIT {
                    match trace_unwrap!(executor.evaluate_condition(condition, environment)) {
                        Expression::Bool(true) => {}
                        Expression::Bool(false) => {
                            completed = true;
                            break;
                        }
                        _ => return None,
                    }

                    if let StaticStatementControl::Return(result) =
                        trace_unwrap!(execute_static_statement_block(executor, body, environment))
                    {
                        return Some(StaticStatementControl::Return(result));
                    }
                }
                if !completed {
                    return None;
                }
            }
            Statement::DoWhile {
                labels,
                condition,
                break_hook,
                body,
            } => {
                if !labels.is_empty() || break_hook.is_some() {
                    if trace_static_exec {
                        eprintln!(
                            "static_exec:unsupported do-while metadata statement={statement:?}"
                        );
                    }
                    return None;
                }

                let mut completed = false;
                for _ in 0..STATIC_LOOP_ITERATION_LIMIT {
                    if let StaticStatementControl::Return(result) =
                        trace_unwrap!(execute_static_statement_block(executor, body, environment))
                    {
                        return Some(StaticStatementControl::Return(result));
                    }

                    match trace_unwrap!(executor.evaluate_condition(condition, environment)) {
                        Expression::Bool(true) => {}
                        Expression::Bool(false) => {
                            completed = true;
                            break;
                        }
                        _ => return None,
                    }
                }
                if !completed {
                    return None;
                }
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
