use super::*;

impl<'a> FunctionCompiler<'a> {
    pub(in crate::backend::direct_wasm) fn resolve_static_bigint_value(
        &self,
        expression: &Expression,
    ) -> Option<StaticBigInt> {
        let materialized = self.materialize_static_expression(expression);
        if !static_expression_matches(&materialized, expression) {
            return self.resolve_static_bigint_value(&materialized);
        }
        match expression {
            Expression::BigInt(value) => parse_static_bigint_literal(value),
            Expression::Unary {
                op: UnaryOp::Negate,
                expression,
            } => Some(-self.resolve_static_bigint_value(expression)?),
            Expression::Binary {
                op:
                    op @ (BinaryOp::Add
                    | BinaryOp::Subtract
                    | BinaryOp::Multiply
                    | BinaryOp::Divide
                    | BinaryOp::Modulo
                    | BinaryOp::Exponentiate
                    | BinaryOp::BitwiseAnd
                    | BinaryOp::BitwiseOr
                    | BinaryOp::BitwiseXor
                    | BinaryOp::LeftShift
                    | BinaryOp::RightShift),
                left,
                right,
            } if self.infer_value_kind(left) == Some(StaticValueKind::BigInt)
                && self.infer_value_kind(right) == Some(StaticValueKind::BigInt) =>
            {
                let left_value = self.resolve_static_bigint_value(left)?;
                let right_value = self.resolve_static_bigint_value(right)?;
                let zero = StaticBigInt::from(0);
                Some(match op {
                    BinaryOp::Add => &left_value + &right_value,
                    BinaryOp::Subtract => &left_value - &right_value,
                    BinaryOp::Multiply => &left_value * &right_value,
                    BinaryOp::Divide => {
                        if right_value == zero {
                            return None;
                        }
                        &left_value / &right_value
                    }
                    BinaryOp::Modulo => {
                        if right_value == zero {
                            return None;
                        }
                        &left_value % &right_value
                    }
                    BinaryOp::Exponentiate => {
                        if right_value < zero {
                            return None;
                        }
                        left_value.pow(u32::try_from(right_value).ok()?)
                    }
                    BinaryOp::BitwiseAnd => left_value & right_value,
                    BinaryOp::BitwiseOr => left_value | right_value,
                    BinaryOp::BitwiseXor => left_value ^ right_value,
                    BinaryOp::LeftShift => {
                        let shift = i64::try_from(right_value).ok()?;
                        if shift >= 0 {
                            left_value << usize::try_from(shift).ok()?
                        } else {
                            left_value >> usize::try_from(-shift).ok()?
                        }
                    }
                    BinaryOp::RightShift => {
                        let shift = i64::try_from(right_value).ok()?;
                        if shift >= 0 {
                            left_value >> usize::try_from(shift).ok()?
                        } else {
                            left_value << usize::try_from(-shift).ok()?
                        }
                    }
                    _ => unreachable!("filtered above"),
                })
            }
            _ => None,
        }
    }
}
