use super::*;

pub(in crate::backend::direct_wasm) fn argument_index_from_expression(
    expression: &Expression,
) -> Option<u32> {
    match expression {
        Expression::Number(value) if value.is_finite() && value.fract() == 0.0 && *value >= 0.0 => {
            let index = *value as u64;
            (index <= u32::MAX as u64).then_some(index as u32)
        }
        Expression::String(text) => canonical_array_index_from_property_name(text),
        _ => None,
    }
}

pub(in crate::backend::direct_wasm) fn canonical_array_index_from_property_name(
    text: &str,
) -> Option<u32> {
    let index = text.parse::<u32>().ok()?;
    if index == u32::MAX || index.to_string() != text {
        return None;
    }
    Some(index)
}

pub(in crate::backend::direct_wasm) fn normalize_js_scientific_notation(text: String) -> String {
    let Some((mantissa, exponent)) = text.split_once('e') else {
        return text;
    };
    let Ok(exponent_value) = exponent.parse::<i32>() else {
        return text;
    };
    if exponent_value >= 0 {
        format!("{mantissa}e+{exponent_value}")
    } else {
        format!("{mantissa}e{exponent_value}")
    }
}

pub(in crate::backend::direct_wasm) fn js_number_property_name(value: f64) -> String {
    if value.is_nan() {
        return "NaN".to_string();
    }
    if value == 0.0 {
        return "0".to_string();
    }
    if value == f64::INFINITY {
        return "Infinity".to_string();
    }
    if value == f64::NEG_INFINITY {
        return "-Infinity".to_string();
    }

    let abs = value.abs();
    if abs >= 1e21 || abs < 1e-6 {
        return normalize_js_scientific_notation(format!("{value:e}"));
    }

    value.to_string()
}

fn js_to_uint32(value: f64) -> u32 {
    if !value.is_finite() || value == 0.0 {
        return 0;
    }
    let truncated = value.trunc();
    let modulo = truncated.rem_euclid(4_294_967_296.0);
    modulo as u32
}

fn js_to_int32(value: f64) -> i32 {
    js_to_uint32(value) as i32
}

pub(in crate::backend::direct_wasm) fn static_numeric_property_name_value(
    expression: &Expression,
) -> Option<f64> {
    match expression {
        Expression::Number(value) => Some(*value),
        Expression::Unary {
            op: UnaryOp::Plus,
            expression,
        } => static_numeric_property_name_value(expression),
        Expression::Unary {
            op: UnaryOp::Negate,
            expression,
        } => Some(-static_numeric_property_name_value(expression)?),
        Expression::Binary {
            op: BinaryOp::Add,
            left,
            right,
        } => Some(
            static_numeric_property_name_value(left)?
                + static_numeric_property_name_value(right)?,
        ),
        Expression::Binary {
            op: BinaryOp::Subtract,
            left,
            right,
        } => Some(
            static_numeric_property_name_value(left)?
                - static_numeric_property_name_value(right)?,
        ),
        Expression::Binary {
            op: BinaryOp::Multiply,
            left,
            right,
        } => Some(
            static_numeric_property_name_value(left)?
                * static_numeric_property_name_value(right)?,
        ),
        Expression::Binary {
            op: BinaryOp::Divide,
            left,
            right,
        } => Some(
            static_numeric_property_name_value(left)?
                / static_numeric_property_name_value(right)?,
        ),
        Expression::Binary {
            op: BinaryOp::Modulo,
            left,
            right,
        } => Some(
            static_numeric_property_name_value(left)?
                % static_numeric_property_name_value(right)?,
        ),
        Expression::Binary {
            op: BinaryOp::Exponentiate,
            left,
            right,
        } => Some(
            static_numeric_property_name_value(left)?
                .powf(static_numeric_property_name_value(right)?),
        ),
        Expression::Binary {
            op: BinaryOp::BitwiseAnd,
            left,
            right,
        } => Some(
            (js_to_int32(static_numeric_property_name_value(left)?)
                & js_to_int32(static_numeric_property_name_value(right)?)) as f64,
        ),
        Expression::Binary {
            op: BinaryOp::BitwiseOr,
            left,
            right,
        } => Some(
            (js_to_int32(static_numeric_property_name_value(left)?)
                | js_to_int32(static_numeric_property_name_value(right)?)) as f64,
        ),
        Expression::Binary {
            op: BinaryOp::BitwiseXor,
            left,
            right,
        } => Some(
            (js_to_int32(static_numeric_property_name_value(left)?)
                ^ js_to_int32(static_numeric_property_name_value(right)?)) as f64,
        ),
        Expression::Binary {
            op: BinaryOp::LeftShift,
            left,
            right,
        } => Some(
            (js_to_int32(static_numeric_property_name_value(left)?)
                << (js_to_uint32(static_numeric_property_name_value(right)?) & 0x1f)) as f64,
        ),
        Expression::Binary {
            op: BinaryOp::RightShift,
            left,
            right,
        } => Some(
            (js_to_int32(static_numeric_property_name_value(left)?)
                >> (js_to_uint32(static_numeric_property_name_value(right)?) & 0x1f)) as f64,
        ),
        Expression::Binary {
            op: BinaryOp::UnsignedRightShift,
            left,
            right,
        } => Some(
            (js_to_uint32(static_numeric_property_name_value(left)?)
                >> (js_to_uint32(static_numeric_property_name_value(right)?) & 0x1f)) as f64,
        ),
        _ => None,
    }
}

fn static_strict_equal_value(left: &Expression, right: &Expression) -> Option<bool> {
    match (left, right) {
        (Expression::Null, Expression::Null)
        | (Expression::Undefined, Expression::Undefined) => Some(true),
        (Expression::Null, Expression::Undefined)
        | (Expression::Undefined, Expression::Null) => Some(false),
        (Expression::Bool(left), Expression::Bool(right)) => Some(left == right),
        (Expression::String(left), Expression::String(right)) => Some(left == right),
        (Expression::BigInt(left), Expression::BigInt(right)) => Some(left == right),
        _ => Some(static_numeric_property_name_value(left)? == static_numeric_property_name_value(right)?),
    }
}

fn static_boolean_property_condition_value(expression: &Expression) -> Option<bool> {
    match expression {
        Expression::Bool(value) => Some(*value),
        Expression::Unary {
            op: UnaryOp::Not,
            expression,
        } => Some(!static_boolean_property_condition_value(expression)?),
        Expression::Binary {
            op: BinaryOp::Equal,
            left,
            right,
        } => static_strict_equal_value(left, right),
        Expression::Binary {
            op: BinaryOp::NotEqual,
            left,
            right,
        } => Some(!static_strict_equal_value(left, right)?),
        Expression::Binary {
            op: BinaryOp::LogicalAnd,
            left,
            right,
        } => {
            let left = static_boolean_property_condition_value(left)?;
            if left {
                static_boolean_property_condition_value(right)
            } else {
                Some(false)
            }
        }
        Expression::Binary {
            op: BinaryOp::LogicalOr,
            left,
            right,
        } => {
            let left = static_boolean_property_condition_value(left)?;
            if left {
                Some(true)
            } else {
                static_boolean_property_condition_value(right)
            }
        }
        _ => None,
    }
}

fn static_truthy_property_value(expression: &Expression) -> Option<bool> {
    match expression {
        Expression::Bool(value) => Some(*value),
        Expression::Null | Expression::Undefined => Some(false),
        Expression::String(value) => Some(!value.is_empty()),
        Expression::BigInt(value) => Some(value.trim_end_matches('n') != "0"),
        _ => {
            let value = static_numeric_property_name_value(expression)?;
            Some(value != 0.0 && !value.is_nan())
        }
    }
}

pub(in crate::backend::direct_wasm) fn static_property_name_from_expression(
    expression: &Expression,
) -> Option<String> {
    match expression {
        Expression::String(text) => Some(text.clone()),
        Expression::Bool(value) => Some(value.to_string()),
        Expression::BigInt(value) => Some(value.clone()),
        Expression::Null => Some("null".to_string()),
        Expression::Undefined => Some("undefined".to_string()),
        Expression::Await(value) => static_property_name_from_expression(value),
        Expression::Assign { value, .. } => static_property_name_from_expression(value),
        Expression::Binary {
            op: BinaryOp::LogicalAnd,
            left,
            right,
        } => {
            if static_truthy_property_value(left)? {
                static_property_name_from_expression(right)
            } else {
                static_property_name_from_expression(left)
            }
        }
        Expression::Binary {
            op: BinaryOp::LogicalOr,
            left,
            right,
        } => {
            if static_truthy_property_value(left)? {
                static_property_name_from_expression(left)
            } else {
                static_property_name_from_expression(right)
            }
        }
        Expression::Binary {
            op: BinaryOp::NullishCoalescing,
            left,
            right,
        } => {
            if matches!(left.as_ref(), Expression::Null | Expression::Undefined) {
                static_property_name_from_expression(right)
            } else {
                static_property_name_from_expression(left)
            }
        }
        Expression::Conditional {
            condition,
            then_expression,
            else_expression,
        } => {
            if static_boolean_property_condition_value(condition)? {
                static_property_name_from_expression(then_expression)
            } else {
                static_property_name_from_expression(else_expression)
            }
        }
        _ => static_numeric_property_name_value(expression).map(js_number_property_name),
    }
}

pub(in crate::backend::direct_wasm) fn is_private_property_name_expression(
    expression: &Expression,
) -> bool {
    static_property_name_from_expression(expression)
        .is_some_and(|name| name.starts_with("__ayy$private$"))
}

pub(in crate::backend::direct_wasm) fn private_brand_marker_property_name(
    private_property_name: &str,
) -> Option<String> {
    private_property_name
        .starts_with("__ayy$private$")
        .then(|| format!("__ayy$private_brand${private_property_name}"))
}

pub(in crate::backend::direct_wasm) fn private_brand_marker_property_expression(
    property: &Expression,
) -> Option<Expression> {
    private_brand_marker_property_name(&static_property_name_from_expression(property)?)
        .map(Expression::String)
}

pub(in crate::backend::direct_wasm) fn is_private_brand_marker_property_name(name: &str) -> bool {
    name.starts_with("__ayy$private_brand$")
}
