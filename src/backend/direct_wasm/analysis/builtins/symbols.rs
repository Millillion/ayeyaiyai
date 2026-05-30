use super::*;

pub(in crate::backend::direct_wasm) fn is_arguments_identifier(expression: &Expression) -> bool {
    matches!(
        expression,
        Expression::Identifier(name)
            if name == "arguments" || scoped_identifier_source_name(name) == Some("arguments")
    )
}

fn scoped_identifier_source_name(name: &str) -> Option<&str> {
    let rest = name.strip_prefix("__ayy_scope$")?;
    let (source_name, scope_id) = rest.rsplit_once('$')?;
    scope_id
        .chars()
        .all(|character| character.is_ascii_digit())
        .then_some(source_name)
}

pub(in crate::backend::direct_wasm) fn is_symbol_iterator_expression(
    expression: &Expression,
) -> bool {
    matches!(
        expression,
        Expression::Member { object, property }
            if matches!(object.as_ref(), Expression::Identifier(name) if name == "Symbol")
                && matches!(property.as_ref(), Expression::String(name) if name == "iterator")
    )
}

pub(in crate::backend::direct_wasm) fn is_symbol_to_string_tag_expression(
    expression: &Expression,
) -> bool {
    matches!(
        expression,
        Expression::Member { object, property }
            if matches!(object.as_ref(), Expression::Identifier(name) if name == "Symbol")
                && matches!(property.as_ref(), Expression::String(name) if name == "toStringTag")
    )
}

pub(in crate::backend::direct_wasm) fn symbol_iterator_expression() -> Expression {
    Expression::Member {
        object: Box::new(Expression::Identifier("Symbol".to_string())),
        property: Box::new(Expression::String("iterator".to_string())),
    }
}

pub(in crate::backend::direct_wasm) fn arguments_symbol_iterator_expression() -> Expression {
    Expression::Member {
        object: Box::new(Expression::Array(Vec::new())),
        property: Box::new(symbol_iterator_expression()),
    }
}

pub(in crate::backend::direct_wasm) fn symbol_to_primitive_expression() -> Expression {
    Expression::Member {
        object: Box::new(Expression::Identifier("Symbol".to_string())),
        property: Box::new(Expression::String("toPrimitive".to_string())),
    }
}
