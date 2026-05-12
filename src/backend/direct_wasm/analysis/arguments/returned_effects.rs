use super::*;

pub(in crate::backend::direct_wasm) fn collect_returned_arguments_effects(
    statements: &[Statement],
) -> ReturnedArgumentsEffects {
    let mut effects = ReturnedArgumentsEffects::default();
    for statement in statements {
        collect_returned_arguments_effects_from_statement(statement, &mut effects);
    }
    effects
}

fn apply_returned_arguments_named_assignment_effect(
    name: &str,
    value: &Expression,
    effects: &mut ReturnedArgumentsEffects,
) {
    let effect = ArgumentsPropertyEffect::Assign(value.clone());
    match name {
        "callee" => effects.callee = Some(effect),
        "length" => effects.length = Some(effect),
        _ => {}
    }
}

pub(in crate::backend::direct_wasm) fn collect_returned_arguments_effects_from_statement(
    statement: &Statement,
    effects: &mut ReturnedArgumentsEffects,
) {
    collect_returned_arguments_effects_from_statement_with_arguments_with(
        statement, effects, false,
    );
}

fn collect_returned_arguments_effects_from_statement_with_arguments_with(
    statement: &Statement,
    effects: &mut ReturnedArgumentsEffects,
    active_arguments_with: bool,
) {
    if active_arguments_with {
        match statement {
            Statement::Assign { name, value } | Statement::Var { name, value } => {
                apply_returned_arguments_named_assignment_effect(name, value, effects);
            }
            _ => {}
        }
    }
    match statement {
        Statement::Declaration { body }
        | Statement::Block { body }
        | Statement::Labeled { body, .. } => {
            for statement in body {
                collect_returned_arguments_effects_from_statement_with_arguments_with(
                    statement,
                    effects,
                    active_arguments_with,
                );
            }
        }
        Statement::Var { value, .. }
        | Statement::Let { value, .. }
        | Statement::Assign { value, .. }
        | Statement::Expression(value)
        | Statement::Throw(value)
        | Statement::Return(value)
        | Statement::Yield { value }
        | Statement::YieldDelegate { value } => {
            collect_returned_arguments_effects_from_expression(value, effects);
        }
        Statement::AssignMember {
            object,
            property,
            value,
        } => {
            collect_returned_arguments_effects_from_expression(value, effects);
            if let Some(property_name) = direct_arguments_named_property(object, property) {
                let effect = ArgumentsPropertyEffect::Assign(value.clone());
                match property_name {
                    "callee" => effects.callee = Some(effect),
                    "length" => effects.length = Some(effect),
                    _ => {}
                }
            }
        }
        Statement::With { object, body } => {
            let nested_arguments_with = is_arguments_identifier(object);
            for statement in body {
                collect_returned_arguments_effects_from_statement_with_arguments_with(
                    statement,
                    effects,
                    nested_arguments_with,
                );
            }
        }
        _ => {}
    }
}

pub(in crate::backend::direct_wasm) fn collect_returned_arguments_effects_from_expression(
    expression: &Expression,
    effects: &mut ReturnedArgumentsEffects,
) {
    match expression {
        Expression::AssignMember {
            object,
            property,
            value,
        } => {
            collect_returned_arguments_effects_from_expression(value, effects);
            if let Some(property_name) = direct_arguments_named_property(object, property) {
                let effect = ArgumentsPropertyEffect::Assign((**value).clone());
                match property_name {
                    "callee" => effects.callee = Some(effect),
                    "length" => effects.length = Some(effect),
                    _ => {}
                }
            }
        }
        Expression::Unary {
            op: UnaryOp::Delete,
            expression,
        } => {
            if let Expression::Member { object, property } = expression.as_ref() {
                if let Some(property_name) = direct_arguments_named_property(object, property) {
                    match property_name {
                        "callee" => effects.callee = Some(ArgumentsPropertyEffect::Delete),
                        "length" => effects.length = Some(ArgumentsPropertyEffect::Delete),
                        _ => {}
                    }
                }
            }
        }
        Expression::Sequence(expressions) => {
            for expression in expressions {
                collect_returned_arguments_effects_from_expression(expression, effects);
            }
        }
        Expression::Binary { left, right, .. } => {
            collect_returned_arguments_effects_from_expression(left, effects);
            collect_returned_arguments_effects_from_expression(right, effects);
        }
        Expression::Conditional {
            condition,
            then_expression,
            else_expression,
        } => {
            collect_returned_arguments_effects_from_expression(condition, effects);
            collect_returned_arguments_effects_from_expression(then_expression, effects);
            collect_returned_arguments_effects_from_expression(else_expression, effects);
        }
        Expression::Call { callee, arguments }
        | Expression::SuperCall { callee, arguments }
        | Expression::New { callee, arguments } => {
            collect_returned_arguments_effects_from_expression(callee, effects);
            for argument in arguments {
                match argument {
                    CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                        collect_returned_arguments_effects_from_expression(expression, effects);
                    }
                }
            }
        }
        Expression::Member { object, property } => {
            collect_returned_arguments_effects_from_expression(object, effects);
            collect_returned_arguments_effects_from_expression(property, effects);
        }
        Expression::Assign { value, .. }
        | Expression::AssignSuperMember { value, .. }
        | Expression::Await(value)
        | Expression::EnumerateKeys(value)
        | Expression::GetIterator(value)
        | Expression::IteratorClose(value)
        | Expression::Unary {
            expression: value, ..
        } => {
            collect_returned_arguments_effects_from_expression(value, effects);
        }
        Expression::SuperMember { property } => {
            collect_returned_arguments_effects_from_expression(property, effects);
        }
        _ => {}
    }
}

pub(in crate::backend::direct_wasm) fn direct_arguments_named_property(
    object: &Expression,
    property: &Expression,
) -> Option<&'static str> {
    if !is_arguments_identifier(object) {
        return None;
    }
    match property {
        Expression::String(property_name) if property_name == "callee" => Some("callee"),
        Expression::String(property_name) if property_name == "length" => Some("length"),
        _ => None,
    }
}
