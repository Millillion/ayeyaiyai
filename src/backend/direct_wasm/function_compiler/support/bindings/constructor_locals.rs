use super::*;

pub(in crate::backend::direct_wasm) fn collect_function_constructor_local_bindings(
    function: &FunctionDeclaration,
) -> HashSet<String> {
    let mut bindings = collect_declared_bindings_from_statements_recursive(&function.body);
    bindings.extend(collect_compiler_generated_local_bindings(function));
    bindings.extend(collect_static_direct_eval_var_bindings(function));
    bindings.extend(
        function
            .params
            .iter()
            .map(|parameter| parameter.name.clone()),
    );
    for parameter in &function.params {
        if let Some(default) = &parameter.default {
            collect_static_direct_eval_var_bindings_from_expression(
                default,
                function.strict,
                &mut bindings,
            );
        }
    }
    if let Some(self_binding) = &function.self_binding {
        bindings.insert(self_binding.clone());
    }
    bindings.insert("arguments".to_string());
    bindings
}

fn is_compiler_generated_local_binding(name: &str) -> bool {
    name.starts_with("__ayy_optional_base_")
        || name.starts_with("__ayy_target_object_")
        || name.starts_with("__ayy_target_property_")
        || name.starts_with("__ayy_postfix_previous_")
}

fn collect_compiler_generated_local_bindings(function: &FunctionDeclaration) -> HashSet<String> {
    let mut assigned = HashSet::new();
    for statement in &function.body {
        collect_assigned_binding_names_from_statement(statement, &mut assigned);
    }
    for parameter in &function.params {
        if let Some(default) = &parameter.default {
            collect_assigned_binding_names_from_expression(default, &mut assigned);
        }
    }
    assigned
        .into_iter()
        .filter(|name| is_compiler_generated_local_binding(name))
        .collect()
}

pub(in crate::backend::direct_wasm) fn collect_static_direct_eval_var_bindings(
    function: &FunctionDeclaration,
) -> HashSet<String> {
    let mut bindings = HashSet::new();
    collect_static_direct_eval_var_bindings_from_statements(
        &function.body,
        function.strict,
        &mut bindings,
    );
    for parameter in &function.params {
        if let Some(default) = &parameter.default {
            collect_static_direct_eval_var_bindings_from_expression(
                default,
                function.strict,
                &mut bindings,
            );
        }
    }
    bindings
}

fn collect_static_direct_eval_var_bindings_from_statements(
    statements: &[Statement],
    caller_strict: bool,
    bindings: &mut HashSet<String>,
) {
    for statement in statements {
        match statement {
            Statement::Declaration { body }
            | Statement::Block { body }
            | Statement::Labeled { body, .. }
            | Statement::With { body, .. }
            | Statement::While { body, .. }
            | Statement::DoWhile { body, .. } => {
                collect_static_direct_eval_var_bindings_from_statements(
                    body,
                    caller_strict,
                    bindings,
                );
            }
            Statement::Var { value, .. }
            | Statement::Let { value, .. }
            | Statement::Assign { value, .. }
            | Statement::Expression(value)
            | Statement::Throw(value)
            | Statement::Return(value)
            | Statement::Yield { value }
            | Statement::YieldDelegate { value } => {
                collect_static_direct_eval_var_bindings_from_expression(
                    value,
                    caller_strict,
                    bindings,
                );
            }
            Statement::AssignMember {
                object,
                property,
                value,
            } => {
                collect_static_direct_eval_var_bindings_from_expression(
                    object,
                    caller_strict,
                    bindings,
                );
                collect_static_direct_eval_var_bindings_from_expression(
                    property,
                    caller_strict,
                    bindings,
                );
                collect_static_direct_eval_var_bindings_from_expression(
                    value,
                    caller_strict,
                    bindings,
                );
            }
            Statement::Print { values } => {
                for value in values {
                    collect_static_direct_eval_var_bindings_from_expression(
                        value,
                        caller_strict,
                        bindings,
                    );
                }
            }
            Statement::If {
                condition,
                then_branch,
                else_branch,
            } => {
                collect_static_direct_eval_var_bindings_from_expression(
                    condition,
                    caller_strict,
                    bindings,
                );
                collect_static_direct_eval_var_bindings_from_statements(
                    then_branch,
                    caller_strict,
                    bindings,
                );
                collect_static_direct_eval_var_bindings_from_statements(
                    else_branch,
                    caller_strict,
                    bindings,
                );
            }
            Statement::Try {
                body,
                catch_setup,
                catch_body,
                ..
            } => {
                collect_static_direct_eval_var_bindings_from_statements(
                    body,
                    caller_strict,
                    bindings,
                );
                collect_static_direct_eval_var_bindings_from_statements(
                    catch_setup,
                    caller_strict,
                    bindings,
                );
                collect_static_direct_eval_var_bindings_from_statements(
                    catch_body,
                    caller_strict,
                    bindings,
                );
            }
            Statement::Switch {
                discriminant,
                cases,
                ..
            } => {
                collect_static_direct_eval_var_bindings_from_expression(
                    discriminant,
                    caller_strict,
                    bindings,
                );
                for case in cases {
                    if let Some(test) = &case.test {
                        collect_static_direct_eval_var_bindings_from_expression(
                            test,
                            caller_strict,
                            bindings,
                        );
                    }
                    collect_static_direct_eval_var_bindings_from_statements(
                        &case.body,
                        caller_strict,
                        bindings,
                    );
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
                collect_static_direct_eval_var_bindings_from_statements(
                    init,
                    caller_strict,
                    bindings,
                );
                if let Some(condition) = condition {
                    collect_static_direct_eval_var_bindings_from_expression(
                        condition,
                        caller_strict,
                        bindings,
                    );
                }
                if let Some(update) = update {
                    collect_static_direct_eval_var_bindings_from_expression(
                        update,
                        caller_strict,
                        bindings,
                    );
                }
                if let Some(break_hook) = break_hook {
                    collect_static_direct_eval_var_bindings_from_expression(
                        break_hook,
                        caller_strict,
                        bindings,
                    );
                }
                collect_static_direct_eval_var_bindings_from_statements(
                    body,
                    caller_strict,
                    bindings,
                );
            }
            Statement::Break { .. } | Statement::Continue { .. } => {}
        }
    }
}

pub(in crate::backend::direct_wasm) fn collect_static_direct_eval_var_bindings_from_expression(
    expression: &Expression,
    caller_strict: bool,
    bindings: &mut HashSet<String>,
) {
    match expression {
        Expression::Call { callee, arguments } => {
            if matches!(callee.as_ref(), Expression::Identifier(name) if name == "eval")
                && let Some(CallArgument::Expression(Expression::String(source))) =
                    arguments.first()
            {
                collect_static_direct_eval_var_bindings_from_source(
                    source,
                    caller_strict,
                    bindings,
                );
            }
            collect_static_direct_eval_var_bindings_from_expression(
                callee,
                caller_strict,
                bindings,
            );
            for argument in arguments {
                match argument {
                    CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                        collect_static_direct_eval_var_bindings_from_expression(
                            expression,
                            caller_strict,
                            bindings,
                        );
                    }
                }
            }
        }
        Expression::SuperCall { callee, arguments } | Expression::New { callee, arguments } => {
            collect_static_direct_eval_var_bindings_from_expression(
                callee,
                caller_strict,
                bindings,
            );
            for argument in arguments {
                match argument {
                    CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                        collect_static_direct_eval_var_bindings_from_expression(
                            expression,
                            caller_strict,
                            bindings,
                        );
                    }
                }
            }
        }
        Expression::Array(elements) => {
            for element in elements {
                match element {
                    ArrayElement::Expression(expression) | ArrayElement::Spread(expression) => {
                        collect_static_direct_eval_var_bindings_from_expression(
                            expression,
                            caller_strict,
                            bindings,
                        );
                    }
                }
            }
        }
        Expression::Object(entries) => {
            for entry in entries {
                match entry {
                    ObjectEntry::Data { key, value } => {
                        collect_static_direct_eval_var_bindings_from_expression(
                            key,
                            caller_strict,
                            bindings,
                        );
                        collect_static_direct_eval_var_bindings_from_expression(
                            value,
                            caller_strict,
                            bindings,
                        );
                    }
                    ObjectEntry::Getter { key, getter } => {
                        collect_static_direct_eval_var_bindings_from_expression(
                            key,
                            caller_strict,
                            bindings,
                        );
                        collect_static_direct_eval_var_bindings_from_expression(
                            getter,
                            caller_strict,
                            bindings,
                        );
                    }
                    ObjectEntry::Setter { key, setter } => {
                        collect_static_direct_eval_var_bindings_from_expression(
                            key,
                            caller_strict,
                            bindings,
                        );
                        collect_static_direct_eval_var_bindings_from_expression(
                            setter,
                            caller_strict,
                            bindings,
                        );
                    }
                    ObjectEntry::Spread(expression) => {
                        collect_static_direct_eval_var_bindings_from_expression(
                            expression,
                            caller_strict,
                            bindings,
                        );
                    }
                }
            }
        }
        Expression::Member { object, property } => {
            collect_static_direct_eval_var_bindings_from_expression(
                object,
                caller_strict,
                bindings,
            );
            collect_static_direct_eval_var_bindings_from_expression(
                property,
                caller_strict,
                bindings,
            );
        }
        Expression::SuperMember { property } => {
            collect_static_direct_eval_var_bindings_from_expression(
                property,
                caller_strict,
                bindings,
            );
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
            collect_static_direct_eval_var_bindings_from_expression(value, caller_strict, bindings);
        }
        Expression::AssignMember {
            object,
            property,
            value,
        } => {
            collect_static_direct_eval_var_bindings_from_expression(
                object,
                caller_strict,
                bindings,
            );
            collect_static_direct_eval_var_bindings_from_expression(
                property,
                caller_strict,
                bindings,
            );
            collect_static_direct_eval_var_bindings_from_expression(value, caller_strict, bindings);
        }
        Expression::Binary { left, right, .. } => {
            collect_static_direct_eval_var_bindings_from_expression(left, caller_strict, bindings);
            collect_static_direct_eval_var_bindings_from_expression(right, caller_strict, bindings);
        }
        Expression::Conditional {
            condition,
            then_expression,
            else_expression,
        } => {
            collect_static_direct_eval_var_bindings_from_expression(
                condition,
                caller_strict,
                bindings,
            );
            collect_static_direct_eval_var_bindings_from_expression(
                then_expression,
                caller_strict,
                bindings,
            );
            collect_static_direct_eval_var_bindings_from_expression(
                else_expression,
                caller_strict,
                bindings,
            );
        }
        Expression::Sequence(expressions) => {
            for expression in expressions {
                collect_static_direct_eval_var_bindings_from_expression(
                    expression,
                    caller_strict,
                    bindings,
                );
            }
        }
        Expression::Number(_)
        | Expression::BigInt(_)
        | Expression::String(_)
        | Expression::Bool(_)
        | Expression::Null
        | Expression::Undefined
        | Expression::NewTarget
        | Expression::Identifier(_)
        | Expression::This
        | Expression::Sent
        | Expression::Update { .. } => {}
    }
}

fn collect_static_direct_eval_var_bindings_from_source(
    source: &str,
    caller_strict: bool,
    bindings: &mut HashSet<String>,
) {
    let eval_source = if caller_strict {
        let mut strict_source = String::from("\"use strict\";");
        strict_source.push_str(source);
        Cow::Owned(strict_source)
    } else {
        Cow::Borrowed(source)
    };
    let Ok(program) = frontend::parse(&eval_source) else {
        return;
    };
    if program.strict {
        return;
    }
    bindings.extend(collect_eval_var_names(&program));
}
