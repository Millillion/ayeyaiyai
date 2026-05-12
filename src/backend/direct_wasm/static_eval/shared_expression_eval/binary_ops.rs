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
    let left = executor.evaluate_expression(left, environment)?;
    let right = executor.evaluate_expression(right, environment)?;
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
        BinaryOp::Equal | BinaryOp::LooseEqual | BinaryOp::NotEqual | BinaryOp::LooseNotEqual => {
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
    if value.is_nan() {
        "NaN".to_string()
    } else if value == f64::INFINITY {
        "Infinity".to_string()
    } else if value == f64::NEG_INFINITY {
        "-Infinity".to_string()
    } else if value == 0.0 {
        "0".to_string()
    } else if value.is_finite() && value.fract() == 0.0 {
        (value as i64).to_string()
    } else {
        value.to_string()
    }
}
