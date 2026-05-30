use super::super::*;

fn await_resume_expression() -> Expression {
    Expression::Call {
        callee: Box::new(Expression::Identifier("__ayyAwaitResume".to_string())),
        arguments: vec![CallArgument::Expression(Expression::Sent)],
    }
}

fn static_await_value(expression: &Expression) -> Option<(Expression, bool)> {
    let Expression::Call { callee, arguments } = expression else {
        return None;
    };
    if matches!(callee.as_ref(), Expression::Identifier(name) if name == "__ayyDynamicImport") {
        let Some(CallArgument::Expression(Expression::Number(module_index))) = arguments.first()
        else {
            return None;
        };
        if !module_index.is_finite() || *module_index < 0.0 || module_index.fract() != 0.0 {
            return None;
        }
        let defer_phase = matches!(
            arguments.get(3),
            Some(CallArgument::Expression(Expression::String(phase)))
                if phase == "__ayy$importPhase$defer"
        );
        if !defer_phase {
            return None;
        }
        return Some((
            Expression::Identifier(format!(
                "__ayy_module_deferred_namespace_{}",
                *module_index as usize
            )),
            false,
        ));
    }

    if matches!(
        callee.as_ref(),
        Expression::Member { object, property }
            if matches!(object.as_ref(), Expression::Identifier(name) if name == "Promise")
                && matches!(property.as_ref(), Expression::String(name) if name == "resolve")
    ) {
        let value = arguments
            .first()
            .map(|argument| argument.expression().clone())
            .unwrap_or(Expression::Undefined);
        return Some((value, true));
    }

    None
}

fn static_builtin_non_thenable_await_value(expression: &Expression) -> Option<Expression> {
    match expression {
        Expression::Number(_)
        | Expression::BigInt(_)
        | Expression::String(_)
        | Expression::Bool(_)
        | Expression::Null
        | Expression::Undefined => Some(expression.clone()),
        Expression::Array(elements) if elements.is_empty() => Some(expression.clone()),
        Expression::Object(entries) if entries.is_empty() => Some(expression.clone()),
        Expression::Object(entries) if static_object_literal_has_no_then_property(entries) => {
            Some(expression.clone())
        }
        Expression::Identifier(name)
            if matches!(
                name.as_str(),
                "Array" | "Boolean" | "Map" | "Number" | "Object" | "Set" | "String"
            ) || name.starts_with("__ayy_fnexpr_")
                || name.starts_with("__ayy_arrow_") =>
        {
            Some(expression.clone())
        }
        _ => None,
    }
}

fn static_object_literal_has_no_then_property(entries: &[ObjectEntry]) -> bool {
    entries.iter().all(|entry| match entry {
        ObjectEntry::Data { key, .. }
        | ObjectEntry::Getter { key, .. }
        | ObjectEntry::Setter { key, .. } => static_property_key_is_not_then(key) == Some(true),
        ObjectEntry::Spread(_) => false,
    })
}

fn static_property_key_is_not_then(key: &Expression) -> Option<bool> {
    match key {
        Expression::String(value) => Some(value != "then"),
        Expression::Number(_)
        | Expression::BigInt(_)
        | Expression::Bool(_)
        | Expression::Null
        | Expression::Undefined => Some(true),
        Expression::Sequence(values) => values.last().and_then(static_property_key_is_not_then),
        _ => None,
    }
}

fn static_immediate_await_value(expression: &Expression) -> Option<(Expression, bool)> {
    static_await_value(expression)
        .or_else(|| static_builtin_non_thenable_await_value(expression).map(|value| (value, true)))
}

fn fold_static_awaits_in_array_element(element: ArrayElement) -> (ArrayElement, bool) {
    match element {
        ArrayElement::Expression(expression) => {
            let (expression, requires_async) = fold_static_awaits_in_expression(expression);
            (ArrayElement::Expression(expression), requires_async)
        }
        ArrayElement::Spread(expression) => {
            let (expression, requires_async) = fold_static_awaits_in_expression(expression);
            (ArrayElement::Spread(expression), requires_async)
        }
    }
}

fn fold_static_awaits_in_object_entry(entry: ObjectEntry) -> (ObjectEntry, bool) {
    match entry {
        ObjectEntry::Data { key, value } => {
            let (key, key_requires_async) = fold_static_awaits_in_expression(key);
            let (value, value_requires_async) = fold_static_awaits_in_expression(value);
            (
                ObjectEntry::Data { key, value },
                key_requires_async || value_requires_async,
            )
        }
        ObjectEntry::Getter { key, getter } => {
            let (key, key_requires_async) = fold_static_awaits_in_expression(key);
            let (getter, getter_requires_async) = fold_static_awaits_in_expression(getter);
            (
                ObjectEntry::Getter { key, getter },
                key_requires_async || getter_requires_async,
            )
        }
        ObjectEntry::Setter { key, setter } => {
            let (key, key_requires_async) = fold_static_awaits_in_expression(key);
            let (setter, setter_requires_async) = fold_static_awaits_in_expression(setter);
            (
                ObjectEntry::Setter { key, setter },
                key_requires_async || setter_requires_async,
            )
        }
        ObjectEntry::Spread(expression) => {
            let (expression, requires_async) = fold_static_awaits_in_expression(expression);
            (ObjectEntry::Spread(expression), requires_async)
        }
    }
}

fn fold_static_awaits_in_call_argument(argument: CallArgument) -> (CallArgument, bool) {
    match argument {
        CallArgument::Expression(expression) => {
            let (expression, requires_async) = fold_static_awaits_in_expression(expression);
            (CallArgument::Expression(expression), requires_async)
        }
        CallArgument::Spread(expression) => {
            let (expression, requires_async) = fold_static_awaits_in_expression(expression);
            (CallArgument::Spread(expression), requires_async)
        }
    }
}

fn fold_static_awaits_in_expression(expression: Expression) -> (Expression, bool) {
    match expression {
        Expression::Array(elements) => {
            let mut requires_async = false;
            let elements = elements
                .into_iter()
                .map(|element| {
                    let (element, element_requires_async) =
                        fold_static_awaits_in_array_element(element);
                    requires_async |= element_requires_async;
                    element
                })
                .collect();
            (Expression::Array(elements), requires_async)
        }
        Expression::Object(entries) => {
            let mut requires_async = false;
            let entries = entries
                .into_iter()
                .map(|entry| {
                    let (entry, entry_requires_async) = fold_static_awaits_in_object_entry(entry);
                    requires_async |= entry_requires_async;
                    entry
                })
                .collect();
            (Expression::Object(entries), requires_async)
        }
        Expression::Member { object, property } => {
            let (object, object_requires_async) = fold_static_awaits_in_expression(*object);
            let (property, property_requires_async) = fold_static_awaits_in_expression(*property);
            (
                Expression::Member {
                    object: Box::new(object),
                    property: Box::new(property),
                },
                object_requires_async || property_requires_async,
            )
        }
        Expression::SuperMember { property } => {
            let (property, requires_async) = fold_static_awaits_in_expression(*property);
            (
                Expression::SuperMember {
                    property: Box::new(property),
                },
                requires_async,
            )
        }
        Expression::Assign { name, value } => {
            let (value, requires_async) = fold_static_awaits_in_expression(*value);
            (
                Expression::Assign {
                    name,
                    value: Box::new(value),
                },
                requires_async,
            )
        }
        Expression::AssignMember {
            object,
            property,
            value,
        } => {
            let (object, object_requires_async) = fold_static_awaits_in_expression(*object);
            let (property, property_requires_async) = fold_static_awaits_in_expression(*property);
            let (value, value_requires_async) = fold_static_awaits_in_expression(*value);
            (
                Expression::AssignMember {
                    object: Box::new(object),
                    property: Box::new(property),
                    value: Box::new(value),
                },
                object_requires_async || property_requires_async || value_requires_async,
            )
        }
        Expression::AssignSuperMember { property, value } => {
            let (property, property_requires_async) = fold_static_awaits_in_expression(*property);
            let (value, value_requires_async) = fold_static_awaits_in_expression(*value);
            (
                Expression::AssignSuperMember {
                    property: Box::new(property),
                    value: Box::new(value),
                },
                property_requires_async || value_requires_async,
            )
        }
        Expression::Await(value) => {
            if let Some((value, preserves_async)) = static_immediate_await_value(&value) {
                let (value, nested_requires_async) = fold_static_awaits_in_expression(value);
                (value, preserves_async || nested_requires_async)
            } else {
                let (value, requires_async) = fold_static_awaits_in_expression(*value);
                (Expression::Await(Box::new(value)), requires_async)
            }
        }
        Expression::EnumerateKeys(value) => {
            let (value, requires_async) = fold_static_awaits_in_expression(*value);
            (Expression::EnumerateKeys(Box::new(value)), requires_async)
        }
        Expression::GetIterator(value) => {
            let (value, requires_async) = fold_static_awaits_in_expression(*value);
            (Expression::GetIterator(Box::new(value)), requires_async)
        }
        Expression::IteratorClose(value) => {
            let (value, requires_async) = fold_static_awaits_in_expression(*value);
            (Expression::IteratorClose(Box::new(value)), requires_async)
        }
        Expression::Unary { op, expression } => {
            let (expression, requires_async) = fold_static_awaits_in_expression(*expression);
            (
                Expression::Unary {
                    op,
                    expression: Box::new(expression),
                },
                requires_async,
            )
        }
        Expression::Binary { op, left, right } => {
            let (left, left_requires_async) = fold_static_awaits_in_expression(*left);
            let (right, right_requires_async) = fold_static_awaits_in_expression(*right);
            (
                Expression::Binary {
                    op,
                    left: Box::new(left),
                    right: Box::new(right),
                },
                left_requires_async || right_requires_async,
            )
        }
        Expression::Conditional {
            condition,
            then_expression,
            else_expression,
        } => {
            let (condition, condition_requires_async) =
                fold_static_awaits_in_expression(*condition);
            let (then_expression, then_requires_async) =
                fold_static_awaits_in_expression(*then_expression);
            let (else_expression, else_requires_async) =
                fold_static_awaits_in_expression(*else_expression);
            (
                Expression::Conditional {
                    condition: Box::new(condition),
                    then_expression: Box::new(then_expression),
                    else_expression: Box::new(else_expression),
                },
                condition_requires_async || then_requires_async || else_requires_async,
            )
        }
        Expression::Sequence(expressions) => {
            let mut requires_async = false;
            let expressions = expressions
                .into_iter()
                .map(|expression| {
                    let (expression, expression_requires_async) =
                        fold_static_awaits_in_expression(expression);
                    requires_async |= expression_requires_async;
                    expression
                })
                .collect();
            (Expression::Sequence(expressions), requires_async)
        }
        Expression::Call { callee, arguments } => {
            let (callee, callee_requires_async) = fold_static_awaits_in_expression(*callee);
            let mut requires_async = callee_requires_async;
            let arguments = arguments
                .into_iter()
                .map(|argument| {
                    let (argument, argument_requires_async) =
                        fold_static_awaits_in_call_argument(argument);
                    requires_async |= argument_requires_async;
                    argument
                })
                .collect();
            (
                Expression::Call {
                    callee: Box::new(callee),
                    arguments,
                },
                requires_async,
            )
        }
        Expression::SuperCall { callee, arguments } => {
            let (callee, callee_requires_async) = fold_static_awaits_in_expression(*callee);
            let mut requires_async = callee_requires_async;
            let arguments = arguments
                .into_iter()
                .map(|argument| {
                    let (argument, argument_requires_async) =
                        fold_static_awaits_in_call_argument(argument);
                    requires_async |= argument_requires_async;
                    argument
                })
                .collect();
            (
                Expression::SuperCall {
                    callee: Box::new(callee),
                    arguments,
                },
                requires_async,
            )
        }
        Expression::New { callee, arguments } => {
            let (callee, callee_requires_async) = fold_static_awaits_in_expression(*callee);
            let mut requires_async = callee_requires_async;
            let arguments = arguments
                .into_iter()
                .map(|argument| {
                    let (argument, argument_requires_async) =
                        fold_static_awaits_in_call_argument(argument);
                    requires_async |= argument_requires_async;
                    argument
                })
                .collect();
            (
                Expression::New {
                    callee: Box::new(callee),
                    arguments,
                },
                requires_async,
            )
        }
        other => (other, false),
    }
}

fn fold_static_awaits_in_statements(statements: Vec<Statement>) -> (Vec<Statement>, bool) {
    let mut requires_async = false;
    let statements = statements
        .into_iter()
        .map(|statement| {
            let (statement, statement_requires_async) = fold_static_awaits_in_statement(statement);
            requires_async |= statement_requires_async;
            statement
        })
        .collect();
    (statements, requires_async)
}

fn fold_static_awaits_in_switch_cases(cases: Vec<SwitchCase>) -> (Vec<SwitchCase>, bool) {
    let mut requires_async = false;
    let cases = cases
        .into_iter()
        .map(|case| {
            let (test, test_requires_async) = match case.test {
                Some(test) => {
                    let (test, requires_async) = fold_static_awaits_in_expression(test);
                    (Some(test), requires_async)
                }
                None => (None, false),
            };
            let (body, body_requires_async) = fold_static_awaits_in_statements(case.body);
            requires_async |= test_requires_async || body_requires_async;
            SwitchCase { test, body }
        })
        .collect();
    (cases, requires_async)
}

fn fold_static_awaits_in_statement(statement: Statement) -> (Statement, bool) {
    match statement {
        Statement::Declaration { body } => {
            let (body, requires_async) = fold_static_awaits_in_statements(body);
            (Statement::Declaration { body }, requires_async)
        }
        Statement::Block { body } => {
            let (body, requires_async) = fold_static_awaits_in_statements(body);
            (Statement::Block { body }, requires_async)
        }
        Statement::Labeled { labels, body } => {
            let (body, requires_async) = fold_static_awaits_in_statements(body);
            (Statement::Labeled { labels, body }, requires_async)
        }
        Statement::Var { name, value } => {
            let (value, requires_async) = fold_static_awaits_in_expression(value);
            (Statement::Var { name, value }, requires_async)
        }
        Statement::Let {
            name,
            mutable,
            value,
        } => {
            let (value, requires_async) = fold_static_awaits_in_expression(value);
            (
                Statement::Let {
                    name,
                    mutable,
                    value,
                },
                requires_async,
            )
        }
        Statement::Assign { name, value } => {
            let (value, requires_async) = fold_static_awaits_in_expression(value);
            (Statement::Assign { name, value }, requires_async)
        }
        Statement::AssignMember {
            object,
            property,
            value,
        } => {
            let (object, object_requires_async) = fold_static_awaits_in_expression(object);
            let (property, property_requires_async) = fold_static_awaits_in_expression(property);
            let (value, value_requires_async) = fold_static_awaits_in_expression(value);
            (
                Statement::AssignMember {
                    object,
                    property,
                    value,
                },
                object_requires_async || property_requires_async || value_requires_async,
            )
        }
        Statement::Print { values } => {
            let mut requires_async = false;
            let values = values
                .into_iter()
                .map(|value| {
                    let (value, value_requires_async) = fold_static_awaits_in_expression(value);
                    requires_async |= value_requires_async;
                    value
                })
                .collect();
            (Statement::Print { values }, requires_async)
        }
        Statement::Expression(expression) => {
            let (expression, requires_async) = fold_static_awaits_in_expression(expression);
            (Statement::Expression(expression), requires_async)
        }
        Statement::Throw(expression) => {
            let (expression, requires_async) = fold_static_awaits_in_expression(expression);
            (Statement::Throw(expression), requires_async)
        }
        Statement::Return(expression) => {
            let (expression, requires_async) = fold_static_awaits_in_expression(expression);
            (Statement::Return(expression), requires_async)
        }
        Statement::Yield { value } => {
            let (value, requires_async) = fold_static_awaits_in_expression(value);
            (Statement::Yield { value }, requires_async)
        }
        Statement::YieldDelegate { value } => {
            let (value, requires_async) = fold_static_awaits_in_expression(value);
            (Statement::YieldDelegate { value }, requires_async)
        }
        Statement::With { object, body } => {
            let (object, object_requires_async) = fold_static_awaits_in_expression(object);
            let (body, body_requires_async) = fold_static_awaits_in_statements(body);
            (
                Statement::With { object, body },
                object_requires_async || body_requires_async,
            )
        }
        Statement::If {
            condition,
            then_branch,
            else_branch,
        } => {
            let (condition, condition_requires_async) = fold_static_awaits_in_expression(condition);
            let (then_branch, then_requires_async) = fold_static_awaits_in_statements(then_branch);
            let (else_branch, else_requires_async) = fold_static_awaits_in_statements(else_branch);
            (
                Statement::If {
                    condition,
                    then_branch,
                    else_branch,
                },
                condition_requires_async || then_requires_async || else_requires_async,
            )
        }
        Statement::Try {
            body,
            catch_binding,
            catch_setup,
            catch_body,
        } => {
            let (body, body_requires_async) = fold_static_awaits_in_statements(body);
            let (catch_setup, setup_requires_async) = fold_static_awaits_in_statements(catch_setup);
            let (catch_body, catch_requires_async) = fold_static_awaits_in_statements(catch_body);
            (
                Statement::Try {
                    body,
                    catch_binding,
                    catch_setup,
                    catch_body,
                },
                body_requires_async || setup_requires_async || catch_requires_async,
            )
        }
        Statement::Switch {
            labels,
            bindings,
            discriminant,
            cases,
        } => {
            let (discriminant, discriminant_requires_async) =
                fold_static_awaits_in_expression(discriminant);
            let (cases, cases_requires_async) = fold_static_awaits_in_switch_cases(cases);
            (
                Statement::Switch {
                    labels,
                    bindings,
                    discriminant,
                    cases,
                },
                discriminant_requires_async || cases_requires_async,
            )
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
            let (init, init_requires_async) = fold_static_awaits_in_statements(init);
            let (condition, condition_requires_async) = match condition {
                Some(condition) => {
                    let (condition, requires_async) = fold_static_awaits_in_expression(condition);
                    (Some(condition), requires_async)
                }
                None => (None, false),
            };
            let (update, update_requires_async) = match update {
                Some(update) => {
                    let (update, requires_async) = fold_static_awaits_in_expression(update);
                    (Some(update), requires_async)
                }
                None => (None, false),
            };
            let (break_hook, break_hook_requires_async) = match break_hook {
                Some(break_hook) => {
                    let (break_hook, requires_async) = fold_static_awaits_in_expression(break_hook);
                    (Some(break_hook), requires_async)
                }
                None => (None, false),
            };
            let (body, body_requires_async) = fold_static_awaits_in_statements(body);
            (
                Statement::For {
                    labels,
                    init,
                    per_iteration_bindings,
                    condition,
                    update,
                    break_hook,
                    body,
                },
                init_requires_async
                    || condition_requires_async
                    || update_requires_async
                    || break_hook_requires_async
                    || body_requires_async,
            )
        }
        Statement::While {
            labels,
            condition,
            break_hook,
            body,
        } => {
            let (condition, condition_requires_async) = fold_static_awaits_in_expression(condition);
            let (break_hook, break_hook_requires_async) = match break_hook {
                Some(break_hook) => {
                    let (break_hook, requires_async) = fold_static_awaits_in_expression(break_hook);
                    (Some(break_hook), requires_async)
                }
                None => (None, false),
            };
            let (body, body_requires_async) = fold_static_awaits_in_statements(body);
            (
                Statement::While {
                    labels,
                    condition,
                    break_hook,
                    body,
                },
                condition_requires_async || break_hook_requires_async || body_requires_async,
            )
        }
        Statement::DoWhile {
            labels,
            condition,
            break_hook,
            body,
        } => {
            let (condition, condition_requires_async) = fold_static_awaits_in_expression(condition);
            let (break_hook, break_hook_requires_async) = match break_hook {
                Some(break_hook) => {
                    let (break_hook, requires_async) = fold_static_awaits_in_expression(break_hook);
                    (Some(break_hook), requires_async)
                }
                None => (None, false),
            };
            let (body, body_requires_async) = fold_static_awaits_in_statements(body);
            (
                Statement::DoWhile {
                    labels,
                    condition,
                    break_hook,
                    body,
                },
                condition_requires_async || break_hook_requires_async || body_requires_async,
            )
        }
        other => (other, false),
    }
}

fn asyncify_statement(statement: Statement) -> (Vec<Statement>, bool) {
    match statement {
        Statement::Expression(Expression::Await(value)) => {
            if let Some((_, preserves_async)) = static_immediate_await_value(&value) {
                (Vec::new(), preserves_async)
            } else {
                (
                    vec![
                        Statement::Yield { value: *value },
                        Statement::Expression(await_resume_expression()),
                    ],
                    true,
                )
            }
        }
        Statement::Var {
            name,
            value: Expression::Await(value),
        } => {
            if let Some((value, preserves_async)) = static_immediate_await_value(&value) {
                (vec![Statement::Var { name, value }], preserves_async)
            } else {
                (
                    vec![
                        Statement::Yield { value: *value },
                        Statement::Var {
                            name,
                            value: await_resume_expression(),
                        },
                    ],
                    true,
                )
            }
        }
        Statement::Let {
            name,
            mutable,
            value: Expression::Await(value),
        } => {
            if let Some((value, preserves_async)) = static_immediate_await_value(&value) {
                (
                    vec![Statement::Let {
                        name,
                        mutable,
                        value,
                    }],
                    preserves_async,
                )
            } else {
                (
                    vec![
                        Statement::Yield { value: *value },
                        Statement::Let {
                            name,
                            mutable,
                            value: await_resume_expression(),
                        },
                    ],
                    true,
                )
            }
        }
        Statement::Assign {
            name,
            value: Expression::Await(value),
        } => {
            if let Some((value, preserves_async)) = static_immediate_await_value(&value) {
                (vec![Statement::Assign { name, value }], preserves_async)
            } else {
                (
                    vec![
                        Statement::Yield { value: *value },
                        Statement::Assign {
                            name,
                            value: await_resume_expression(),
                        },
                    ],
                    true,
                )
            }
        }
        Statement::Return(Expression::Await(value)) => {
            if let Some((value, preserves_async)) = static_immediate_await_value(&value) {
                (vec![Statement::Return(value)], preserves_async)
            } else {
                (
                    vec![
                        Statement::Yield { value: *value },
                        Statement::Return(await_resume_expression()),
                    ],
                    true,
                )
            }
        }
        Statement::If {
            condition: Expression::Await(value),
            then_branch,
            else_branch,
        } => {
            let (then_branch, then_requires_async) = fold_static_awaits_in_statements(then_branch);
            let (else_branch, else_requires_async) = fold_static_awaits_in_statements(else_branch);
            if let Some((value, preserves_async)) = static_immediate_await_value(&value) {
                let (condition, condition_requires_async) = fold_static_awaits_in_expression(value);
                (
                    vec![Statement::If {
                        condition,
                        then_branch,
                        else_branch,
                    }],
                    preserves_async
                        || condition_requires_async
                        || then_requires_async
                        || else_requires_async,
                )
            } else {
                (
                    vec![
                        Statement::Yield { value: *value },
                        Statement::If {
                            condition: await_resume_expression(),
                            then_branch,
                            else_branch,
                        },
                    ],
                    true,
                )
            }
        }
        other => {
            let (statement, requires_async) = fold_static_awaits_in_statement(other);
            (vec![statement], requires_async)
        }
    }
}

pub(crate) fn asyncify_statements(statements: Vec<Statement>) -> (Vec<Statement>, bool) {
    let mut asyncified = Vec::new();
    let mut changed = false;

    for statement in statements {
        let (mut lowered, statement_changed) = asyncify_statement(statement);
        changed |= statement_changed;
        asyncified.append(&mut lowered);
    }

    (asyncified, changed)
}
