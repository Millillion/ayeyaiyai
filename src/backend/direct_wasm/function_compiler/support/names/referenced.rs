use super::*;

pub(in crate::backend::direct_wasm) fn collect_referenced_binding_names_from_statements(
    statements: &[Statement],
) -> HashSet<String> {
    let mut names = HashSet::new();
    for statement in statements {
        collect_referenced_binding_names_from_statement(statement, &mut names);
    }
    names
}

pub(in crate::backend::direct_wasm) fn statements_reference_this(statements: &[Statement]) -> bool {
    statements.iter().any(statement_references_this)
}

pub(in crate::backend::direct_wasm) fn statement_references_this(statement: &Statement) -> bool {
    match statement {
        Statement::Declaration { body }
        | Statement::Block { body }
        | Statement::Labeled { body, .. } => body.iter().any(statement_references_this),
        Statement::Var { value, .. }
        | Statement::Let { value, .. }
        | Statement::Expression(value)
        | Statement::Throw(value)
        | Statement::Return(value)
        | Statement::Yield { value }
        | Statement::YieldDelegate { value } => expression_references_this(value),
        Statement::Assign { value, .. } => expression_references_this(value),
        Statement::AssignMember {
            object,
            property,
            value,
        } => {
            expression_references_this(object)
                || expression_references_this(property)
                || expression_references_this(value)
        }
        Statement::Print { values } => values.iter().any(expression_references_this),
        Statement::With { object, body } => {
            expression_references_this(object) || body.iter().any(statement_references_this)
        }
        Statement::If {
            condition,
            then_branch,
            else_branch,
        } => {
            expression_references_this(condition)
                || then_branch.iter().any(statement_references_this)
                || else_branch.iter().any(statement_references_this)
        }
        Statement::Try {
            body,
            catch_setup,
            catch_body,
            ..
        } => {
            body.iter().any(statement_references_this)
                || catch_setup.iter().any(statement_references_this)
                || catch_body.iter().any(statement_references_this)
        }
        Statement::Switch {
            discriminant,
            cases,
            ..
        } => {
            expression_references_this(discriminant)
                || cases.iter().any(|case| {
                    case.test.as_ref().is_some_and(expression_references_this)
                        || case.body.iter().any(statement_references_this)
                })
        }
        Statement::For {
            init,
            condition,
            update,
            break_hook,
            body,
            ..
        } => {
            init.iter().any(statement_references_this)
                || condition.as_ref().is_some_and(expression_references_this)
                || update.as_ref().is_some_and(expression_references_this)
                || break_hook.as_ref().is_some_and(expression_references_this)
                || body.iter().any(statement_references_this)
        }
        Statement::While {
            condition,
            break_hook,
            body,
            ..
        }
        | Statement::DoWhile {
            condition,
            break_hook,
            body,
            ..
        } => {
            expression_references_this(condition)
                || break_hook.as_ref().is_some_and(expression_references_this)
                || body.iter().any(statement_references_this)
        }
        Statement::Break { .. } | Statement::Continue { .. } => false,
    }
}

pub(in crate::backend::direct_wasm) fn collect_referenced_binding_names_from_statement(
    statement: &Statement,
    names: &mut HashSet<String>,
) {
    match statement {
        Statement::Declaration { body }
        | Statement::Block { body }
        | Statement::Labeled { body, .. } => {
            for statement in body {
                collect_referenced_binding_names_from_statement(statement, names);
            }
        }
        Statement::Var { value, .. }
        | Statement::Let { value, .. }
        | Statement::Expression(value)
        | Statement::Throw(value)
        | Statement::Return(value)
        | Statement::Yield { value }
        | Statement::YieldDelegate { value } => {
            collect_referenced_binding_names_from_expression(value, names);
        }
        Statement::Assign { name, value } => {
            names.insert(name.clone());
            collect_referenced_binding_names_from_expression(value, names);
        }
        Statement::AssignMember {
            object,
            property,
            value,
        } => {
            collect_referenced_binding_names_from_expression(object, names);
            collect_referenced_binding_names_from_expression(property, names);
            collect_referenced_binding_names_from_expression(value, names);
        }
        Statement::Print { values } => {
            for value in values {
                collect_referenced_binding_names_from_expression(value, names);
            }
        }
        Statement::With { object, body } => {
            collect_referenced_binding_names_from_expression(object, names);
            for statement in body {
                collect_referenced_binding_names_from_statement(statement, names);
            }
        }
        Statement::If {
            condition,
            then_branch,
            else_branch,
        } => {
            collect_referenced_binding_names_from_expression(condition, names);
            for statement in then_branch {
                collect_referenced_binding_names_from_statement(statement, names);
            }
            for statement in else_branch {
                collect_referenced_binding_names_from_statement(statement, names);
            }
        }
        Statement::Try {
            body,
            catch_setup,
            catch_body,
            ..
        } => {
            for statement in body {
                collect_referenced_binding_names_from_statement(statement, names);
            }
            for statement in catch_setup {
                collect_referenced_binding_names_from_statement(statement, names);
            }
            for statement in catch_body {
                collect_referenced_binding_names_from_statement(statement, names);
            }
        }
        Statement::Switch {
            discriminant,
            cases,
            ..
        } => {
            collect_referenced_binding_names_from_expression(discriminant, names);
            for case in cases {
                if let Some(test) = &case.test {
                    collect_referenced_binding_names_from_expression(test, names);
                }
                for statement in &case.body {
                    collect_referenced_binding_names_from_statement(statement, names);
                }
            }
        }
        Statement::For {
            init,
            condition,
            update,
            break_hook,
            body,
            ..
        } => {
            for statement in init {
                collect_referenced_binding_names_from_statement(statement, names);
            }
            if let Some(condition) = condition {
                collect_referenced_binding_names_from_expression(condition, names);
            }
            if let Some(update) = update {
                collect_referenced_binding_names_from_expression(update, names);
            }
            if let Some(break_hook) = break_hook {
                collect_referenced_binding_names_from_expression(break_hook, names);
            }
            for statement in body {
                collect_referenced_binding_names_from_statement(statement, names);
            }
        }
        Statement::While {
            condition,
            break_hook,
            body,
            ..
        }
        | Statement::DoWhile {
            condition,
            break_hook,
            body,
            ..
        } => {
            collect_referenced_binding_names_from_expression(condition, names);
            if let Some(break_hook) = break_hook {
                collect_referenced_binding_names_from_expression(break_hook, names);
            }
            for statement in body {
                collect_referenced_binding_names_from_statement(statement, names);
            }
        }
        Statement::Break { .. } | Statement::Continue { .. } => {}
    }
}

pub(in crate::backend::direct_wasm) fn expression_references_this(expression: &Expression) -> bool {
    match expression {
        Expression::This => true,
        Expression::Identifier(_)
        | Expression::Update { .. }
        | Expression::Number(_)
        | Expression::BigInt(_)
        | Expression::String(_)
        | Expression::Bool(_)
        | Expression::Null
        | Expression::Undefined
        | Expression::NewTarget
        | Expression::Sent => false,
        Expression::Member { object, property } => {
            expression_references_this(object) || expression_references_this(property)
        }
        Expression::SuperMember { .. } => true,
        Expression::Assign { value, .. } => expression_references_this(value),
        Expression::AssignMember {
            object,
            property,
            value,
        } => {
            expression_references_this(object)
                || expression_references_this(property)
                || expression_references_this(value)
        }
        Expression::AssignSuperMember { .. } => true,
        Expression::Await(value)
        | Expression::EnumerateKeys(value)
        | Expression::GetIterator(value)
        | Expression::IteratorClose(value)
        | Expression::Unary {
            expression: value, ..
        } => expression_references_this(value),
        Expression::Binary { left, right, .. } => {
            expression_references_this(left) || expression_references_this(right)
        }
        Expression::Conditional {
            condition,
            then_expression,
            else_expression,
        } => {
            expression_references_this(condition)
                || expression_references_this(then_expression)
                || expression_references_this(else_expression)
        }
        Expression::Sequence(expressions) => expressions.iter().any(expression_references_this),
        Expression::SuperCall { .. } => true,
        Expression::Call { callee, arguments } | Expression::New { callee, arguments } => {
            expression_references_this(callee)
                || arguments.iter().any(|argument| match argument {
                    CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                        expression_references_this(expression)
                    }
                })
        }
        Expression::Array(elements) => elements.iter().any(|element| match element {
            ArrayElement::Expression(expression) | ArrayElement::Spread(expression) => {
                expression_references_this(expression)
            }
        }),
        Expression::Object(entries) => entries.iter().any(|entry| match entry {
            ObjectEntry::Data { key, value } => {
                expression_references_this(key) || expression_references_this(value)
            }
            ObjectEntry::Getter { key, getter } => {
                expression_references_this(key) || expression_references_this(getter)
            }
            ObjectEntry::Setter { key, setter } => {
                expression_references_this(key) || expression_references_this(setter)
            }
            ObjectEntry::Spread(expression) => expression_references_this(expression),
        }),
    }
}

pub(in crate::backend::direct_wasm) fn collect_referenced_binding_names_from_expression(
    expression: &Expression,
    names: &mut HashSet<String>,
) {
    match expression {
        Expression::Identifier(name) | Expression::Update { name, .. } => {
            names.insert(name.clone());
        }
        Expression::Member { object, property } => {
            collect_referenced_binding_names_from_expression(object, names);
            collect_referenced_binding_names_from_expression(property, names);
        }
        Expression::SuperMember { property } => {
            collect_referenced_binding_names_from_expression(property, names);
        }
        Expression::Assign { name, value } => {
            names.insert(name.clone());
            collect_referenced_binding_names_from_expression(value, names);
        }
        Expression::AssignMember {
            object,
            property,
            value,
        } => {
            collect_referenced_binding_names_from_expression(object, names);
            collect_referenced_binding_names_from_expression(property, names);
            collect_referenced_binding_names_from_expression(value, names);
        }
        Expression::AssignSuperMember { property, value } => {
            collect_referenced_binding_names_from_expression(property, names);
            collect_referenced_binding_names_from_expression(value, names);
        }
        Expression::Await(value)
        | Expression::EnumerateKeys(value)
        | Expression::GetIterator(value)
        | Expression::IteratorClose(value)
        | Expression::Unary {
            expression: value, ..
        } => collect_referenced_binding_names_from_expression(value, names),
        Expression::Binary { left, right, .. } => {
            collect_referenced_binding_names_from_expression(left, names);
            collect_referenced_binding_names_from_expression(right, names);
        }
        Expression::Conditional {
            condition,
            then_expression,
            else_expression,
        } => {
            collect_referenced_binding_names_from_expression(condition, names);
            collect_referenced_binding_names_from_expression(then_expression, names);
            collect_referenced_binding_names_from_expression(else_expression, names);
        }
        Expression::Sequence(expressions) => {
            for expression in expressions {
                collect_referenced_binding_names_from_expression(expression, names);
            }
        }
        Expression::Call { callee, arguments }
        | Expression::SuperCall { callee, arguments }
        | Expression::New { callee, arguments } => {
            collect_referenced_binding_names_from_expression(callee, names);
            for argument in arguments {
                match argument {
                    CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                        collect_referenced_binding_names_from_expression(expression, names);
                    }
                }
            }
        }
        Expression::Array(elements) => {
            for element in elements {
                match element {
                    ArrayElement::Expression(expression) | ArrayElement::Spread(expression) => {
                        collect_referenced_binding_names_from_expression(expression, names);
                    }
                }
            }
        }
        Expression::Object(entries) => {
            for entry in entries {
                match entry {
                    ObjectEntry::Data { key, value } => {
                        collect_referenced_binding_names_from_expression(key, names);
                        collect_referenced_binding_names_from_expression(value, names);
                    }
                    ObjectEntry::Getter { key, getter } => {
                        collect_referenced_binding_names_from_expression(key, names);
                        collect_referenced_binding_names_from_expression(getter, names);
                    }
                    ObjectEntry::Setter { key, setter } => {
                        collect_referenced_binding_names_from_expression(key, names);
                        collect_referenced_binding_names_from_expression(setter, names);
                    }
                    ObjectEntry::Spread(expression) => {
                        collect_referenced_binding_names_from_expression(expression, names);
                    }
                }
            }
        }
        Expression::Number(_)
        | Expression::BigInt(_)
        | Expression::String(_)
        | Expression::Bool(_)
        | Expression::Null
        | Expression::Undefined
        | Expression::This
        | Expression::NewTarget
        | Expression::Sent => {}
    }
}
