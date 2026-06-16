use super::*;

pub(in crate::backend::direct_wasm) fn evaluate_static_binary_expression<
    Executor: StaticExpressionEvaluation + ?Sized,
>(
    executor: &Executor,
    expression: &Expression,
    environment: &mut Executor::Environment,
) -> Option<Expression> {
    let Expression::Binary { op, left, right } = expression else {
        return None;
    };
    let left = executor
        .evaluate_expression(left, environment)
        .or_else(|| executor.materialize_expression(left, environment))?;
    if matches!(op, BinaryOp::LogicalAnd | BinaryOp::LogicalOr) {
        let left_truthy = static_eval_truthy(&left)?;
        return match op {
            BinaryOp::LogicalAnd if left_truthy => executor
                .evaluate_expression(right, environment)
                .or_else(|| executor.materialize_expression(right, environment)),
            BinaryOp::LogicalAnd => Some(left),
            BinaryOp::LogicalOr if left_truthy => Some(left),
            BinaryOp::LogicalOr => executor
                .evaluate_expression(right, environment)
                .or_else(|| executor.materialize_expression(right, environment)),
            _ => unreachable!("logical operator filtered above"),
        };
    }
    let right = executor
        .evaluate_expression(right, environment)
        .or_else(|| executor.materialize_expression(right, environment))?;
    match op {
        BinaryOp::Add => {
            if matches!(left, Expression::String(_)) || matches!(right, Expression::String(_)) {
                let left = static_eval_primitive_to_string(&left)?;
                let right = static_eval_primitive_to_string(&right)?;
                Some(Expression::String(format!("{left}{right}")))
            } else {
                match (
                    static_eval_primitive_to_number(&left),
                    static_eval_primitive_to_number(&right),
                ) {
                    (Some(lhs), Some(rhs)) => Some(Expression::Number(lhs + rhs)),
                    _ => None,
                }
            }
        }
        BinaryOp::Subtract => match (&left, &right) {
            (Expression::Number(lhs), Expression::Number(rhs)) => {
                Some(Expression::Number(lhs - rhs))
            }
            _ => None,
        },
        BinaryOp::Multiply => match (&left, &right) {
            (Expression::Number(lhs), Expression::Number(rhs)) => {
                Some(Expression::Number(lhs * rhs))
            }
            _ => None,
        },
        BinaryOp::Divide => match (&left, &right) {
            (Expression::Number(lhs), Expression::Number(rhs)) => {
                Some(Expression::Number(lhs / rhs))
            }
            _ => None,
        },
        BinaryOp::Modulo => match (
            static_eval_primitive_to_number(&left),
            static_eval_primitive_to_number(&right),
        ) {
            (Some(lhs), Some(rhs)) => Some(Expression::Number(lhs % rhs)),
            _ => None,
        },
        BinaryOp::Equal | BinaryOp::LooseEqual | BinaryOp::NotEqual | BinaryOp::LooseNotEqual => {
            let is_object_like = |expression: &Expression| {
                matches!(
                    expression,
                    Expression::Array(_)
                        | Expression::Object(_)
                        | Expression::New { .. }
                        | Expression::This
                        | Expression::Call { .. }
                        | Expression::Member { .. }
                )
            };
            let equal = match (&left, &right) {
                (Expression::Bool(lhs), Expression::Bool(rhs)) => lhs == rhs,
                (Expression::Number(lhs), Expression::Number(rhs)) => lhs == rhs,
                (Expression::String(lhs), Expression::String(rhs)) => lhs == rhs,
                (Expression::Null, Expression::Null)
                | (Expression::Undefined, Expression::Undefined) => true,
                (Expression::Null, Expression::Undefined)
                | (Expression::Undefined, Expression::Null)
                    if matches!(op, BinaryOp::LooseEqual | BinaryOp::LooseNotEqual) =>
                {
                    true
                }
                _ if is_object_like(&left) || is_object_like(&right) => return None,
                _ => false,
            };
            Some(Expression::Bool(match op {
                BinaryOp::Equal | BinaryOp::LooseEqual => equal,
                BinaryOp::NotEqual | BinaryOp::LooseNotEqual => !equal,
                _ => unreachable!("filtered above"),
            }))
        }
        BinaryOp::LessThan
        | BinaryOp::LessThanOrEqual
        | BinaryOp::GreaterThan
        | BinaryOp::GreaterThanOrEqual => {
            let ordering = match (&left, &right) {
                (Expression::Number(lhs), Expression::Number(rhs)) => lhs.partial_cmp(rhs)?,
                (Expression::String(lhs), Expression::String(rhs)) => lhs.cmp(rhs),
                _ => return None,
            };
            Some(Expression::Bool(match op {
                BinaryOp::LessThan => ordering == std::cmp::Ordering::Less,
                BinaryOp::LessThanOrEqual => ordering != std::cmp::Ordering::Greater,
                BinaryOp::GreaterThan => ordering == std::cmp::Ordering::Greater,
                BinaryOp::GreaterThanOrEqual => ordering != std::cmp::Ordering::Less,
                _ => unreachable!("filtered above"),
            }))
        }
        _ => None,
    }
}

pub(in crate::backend::direct_wasm) fn evaluate_static_unary_expression<
    Executor: StaticExpressionEvaluation + ?Sized,
>(
    executor: &Executor,
    expression: &Expression,
    environment: &mut Executor::Environment,
) -> Option<Expression> {
    let Expression::Unary { op, expression } = expression else {
        return None;
    };
    match op {
        UnaryOp::Plus => {
            let value = executor
                .evaluate_expression(expression, environment)
                .or_else(|| executor.materialize_expression(expression, environment))?;
            static_eval_primitive_to_number(&value).map(Expression::Number)
        }
        UnaryOp::Negate => {
            let value = executor
                .evaluate_expression(expression, environment)
                .or_else(|| executor.materialize_expression(expression, environment))?;
            static_eval_primitive_to_number(&value).map(|value| Expression::Number(-value))
        }
        UnaryOp::Not => {
            let value = executor
                .evaluate_expression(expression, environment)
                .or_else(|| executor.materialize_expression(expression, environment))?;
            static_eval_truthy(&value).map(|truthy| Expression::Bool(!truthy))
        }
        UnaryOp::Void => Some(Expression::Undefined),
        _ => None,
    }
}

fn static_eval_truthy(expression: &Expression) -> Option<bool> {
    Some(match expression {
        Expression::Bool(value) => *value,
        Expression::Number(value) => *value != 0.0 && !value.is_nan(),
        Expression::String(value) => !value.is_empty(),
        Expression::Null | Expression::Undefined => false,
        Expression::BigInt(value) => {
            let digits = value.trim_end_matches('n');
            digits != "0" && digits != "-0"
        }
        Expression::Array(_) | Expression::Object(_) => true,
        _ => return None,
    })
}

fn static_eval_primitive_to_string(expression: &Expression) -> Option<String> {
    match expression {
        Expression::String(value) => Some(value.clone()),
        Expression::Number(value) => Some(static_eval_number_to_string(*value)),
        Expression::Bool(value) => Some(value.to_string()),
        Expression::Null => Some("null".to_string()),
        Expression::Undefined => Some("undefined".to_string()),
        Expression::BigInt(value) => Some(value.trim_end_matches('n').to_string()),
        _ => None,
    }
}

fn static_eval_primitive_to_number(expression: &Expression) -> Option<f64> {
    match expression {
        Expression::Number(value) => Some(*value),
        Expression::Bool(value) => Some(if *value { 1.0 } else { 0.0 }),
        Expression::Null => Some(0.0),
        Expression::Undefined => Some(f64::NAN),
        _ => None,
    }
}

fn static_eval_number_to_string(value: f64) -> String {
    js_number_to_string(value)
}
