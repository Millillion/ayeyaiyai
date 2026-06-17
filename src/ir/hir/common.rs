use super::{Expression, Statement};

#[derive(Debug, Clone, PartialEq)]
pub enum ObjectEntry {
    Data { key: Expression, value: Expression },
    Getter { key: Expression, getter: Expression },
    Setter { key: Expression, setter: Expression },
    Spread(Expression),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ArrayElement {
    Expression(Expression),
    Spread(Expression),
}

pub const ARRAY_ELISION_SENTINEL: &str = "__ayy$array$elision";

pub fn array_elision_expression() -> Expression {
    Expression::Identifier(ARRAY_ELISION_SENTINEL.to_string())
}

pub fn expression_is_array_elision(expression: &Expression) -> bool {
    matches!(expression, Expression::Identifier(name) if name == ARRAY_ELISION_SENTINEL)
}

#[derive(Debug, Clone, PartialEq)]
pub enum CallArgument {
    Expression(Expression),
    Spread(Expression),
}

impl CallArgument {
    pub fn expression(&self) -> &Expression {
        match self {
            Self::Expression(expression) | Self::Spread(expression) => expression,
        }
    }

    pub fn expression_mut(&mut self) -> &mut Expression {
        match self {
            Self::Expression(expression) | Self::Spread(expression) => expression,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SwitchCase {
    pub test: Option<Expression>,
    pub body: Vec<Statement>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Negate,
    Plus,
    Not,
    BitwiseNot,
    TypeOf,
    Void,
    Delete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateOp {
    Increment,
    Decrement,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,
    Exponentiate,
    BitwiseAnd,
    BitwiseOr,
    BitwiseXor,
    LeftShift,
    RightShift,
    UnsignedRightShift,
    In,
    InstanceOf,
    LooseEqual,
    LooseNotEqual,
    Equal,
    NotEqual,
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,
    LogicalAnd,
    LogicalOr,
    NullishCoalescing,
}
