use super::{ArrayElement, BinaryOp, CallArgument, ObjectEntry, UnaryOp, UpdateOp};

const JS_SURROGATE_SENTINEL_BASE: u32 = 0xe000;

pub fn js_surrogate_code_unit_to_sentinel(unit: u16) -> Option<char> {
    (0xd800..=0xdfff)
        .contains(&unit)
        .then(|| char::from_u32(JS_SURROGATE_SENTINEL_BASE + u32::from(unit - 0xd800)))
        .flatten()
}

pub fn js_surrogate_sentinel_to_code_unit(value: char) -> Option<u16> {
    let scalar = value as u32;
    (JS_SURROGATE_SENTINEL_BASE..JS_SURROGATE_SENTINEL_BASE + 0x800)
        .contains(&scalar)
        .then_some(0xd800 + (scalar - JS_SURROGATE_SENTINEL_BASE) as u16)
}

pub fn js_string_utf16_code_units(text: &str) -> Vec<u16> {
    let mut units = Vec::new();
    for value in text.chars() {
        if let Some(unit) = js_surrogate_sentinel_to_code_unit(value) {
            units.push(unit);
        } else {
            let mut buffer = [0u16; 2];
            units.extend_from_slice(value.encode_utf16(&mut buffer));
        }
    }
    units
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    Number(f64),
    BigInt(String),
    String(String),
    Bool(bool),
    Null,
    Undefined,
    NewTarget,
    Array(Vec<ArrayElement>),
    Object(Vec<ObjectEntry>),
    Identifier(String),
    This,
    Sent,
    Member {
        object: Box<Expression>,
        property: Box<Expression>,
    },
    SuperMember {
        property: Box<Expression>,
    },
    Assign {
        name: String,
        value: Box<Expression>,
    },
    AssignMember {
        object: Box<Expression>,
        property: Box<Expression>,
        value: Box<Expression>,
    },
    AssignSuperMember {
        property: Box<Expression>,
        value: Box<Expression>,
    },
    Await(Box<Expression>),
    EnumerateKeys(Box<Expression>),
    GetIterator(Box<Expression>),
    IteratorClose(Box<Expression>),
    Unary {
        op: UnaryOp,
        expression: Box<Expression>,
    },
    Binary {
        op: BinaryOp,
        left: Box<Expression>,
        right: Box<Expression>,
    },
    Conditional {
        condition: Box<Expression>,
        then_expression: Box<Expression>,
        else_expression: Box<Expression>,
    },
    Sequence(Vec<Expression>),
    Call {
        callee: Box<Expression>,
        arguments: Vec<CallArgument>,
    },
    SuperCall {
        callee: Box<Expression>,
        arguments: Vec<CallArgument>,
    },
    New {
        callee: Box<Expression>,
        arguments: Vec<CallArgument>,
    },
    Update {
        name: String,
        op: UpdateOp,
        prefix: bool,
    },
}
