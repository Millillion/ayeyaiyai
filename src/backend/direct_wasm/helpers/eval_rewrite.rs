use super::*;

fn rewrite_eval_program_internal_function_name(
    name: &mut String,
    rename_map: &HashMap<String, String>,
) {
    if let Some(renamed) = rename_map.get(name) {
        *name = renamed.clone();
    }
}

fn rewrite_eval_program_internal_function_names_in_expression(
    expression: &mut Expression,
    rename_map: &HashMap<String, String>,
) {
    match expression {
        Expression::Array(elements) => {
            for element in elements {
                match element {
                    ArrayElement::Expression(expression) | ArrayElement::Spread(expression) => {
                        rewrite_eval_program_internal_function_names_in_expression(
                            expression, rename_map,
                        );
                    }
                }
            }
        }
        Expression::Object(entries) => {
            for entry in entries {
                match entry {
                    ObjectEntry::Data { key, value } => {
                        rewrite_eval_program_internal_function_names_in_expression(key, rename_map);
                        rewrite_eval_program_internal_function_names_in_expression(
                            value, rename_map,
                        );
                    }
                    ObjectEntry::Getter { key, getter } => {
                        rewrite_eval_program_internal_function_names_in_expression(key, rename_map);
                        rewrite_eval_program_internal_function_names_in_expression(
                            getter, rename_map,
                        );
                    }
                    ObjectEntry::Setter { key, setter } => {
                        rewrite_eval_program_internal_function_names_in_expression(key, rename_map);
                        rewrite_eval_program_internal_function_names_in_expression(
                            setter, rename_map,
                        );
                    }
                    ObjectEntry::Spread(expression) => {
                        rewrite_eval_program_internal_function_names_in_expression(
                            expression, rename_map,
                        );
                    }
                }
            }
        }
        Expression::Identifier(name) | Expression::Update { name, .. } => {
            rewrite_eval_program_internal_function_name(name, rename_map);
        }
        Expression::Member { object, property } => {
            rewrite_eval_program_internal_function_names_in_expression(object, rename_map);
            rewrite_eval_program_internal_function_names_in_expression(property, rename_map);
        }
        Expression::SuperMember { property }
        | Expression::Await(property)
        | Expression::EnumerateKeys(property)
        | Expression::GetIterator(property)
        | Expression::IteratorClose(property)
        | Expression::Unary {
            expression: property,
            ..
        } => rewrite_eval_program_internal_function_names_in_expression(property, rename_map),
        Expression::Assign { name, value } => {
            rewrite_eval_program_internal_function_name(name, rename_map);
            rewrite_eval_program_internal_function_names_in_expression(value, rename_map);
        }
        Expression::AssignMember {
            object,
            property,
            value,
        } => {
            rewrite_eval_program_internal_function_names_in_expression(object, rename_map);
            rewrite_eval_program_internal_function_names_in_expression(property, rename_map);
            rewrite_eval_program_internal_function_names_in_expression(value, rename_map);
        }
        Expression::AssignSuperMember { property, value } => {
            rewrite_eval_program_internal_function_names_in_expression(property, rename_map);
            rewrite_eval_program_internal_function_names_in_expression(value, rename_map);
        }
        Expression::Binary { left, right, .. } => {
            rewrite_eval_program_internal_function_names_in_expression(left, rename_map);
            rewrite_eval_program_internal_function_names_in_expression(right, rename_map);
        }
        Expression::Conditional {
            condition,
            then_expression,
            else_expression,
        } => {
            rewrite_eval_program_internal_function_names_in_expression(condition, rename_map);
            rewrite_eval_program_internal_function_names_in_expression(then_expression, rename_map);
            rewrite_eval_program_internal_function_names_in_expression(else_expression, rename_map);
        }
        Expression::Sequence(expressions) => {
            for expression in expressions {
                rewrite_eval_program_internal_function_names_in_expression(expression, rename_map);
            }
        }
        Expression::Call { callee, arguments }
        | Expression::SuperCall { callee, arguments }
        | Expression::New { callee, arguments } => {
            rewrite_eval_program_internal_function_names_in_expression(callee, rename_map);
            for argument in arguments {
                match argument {
                    CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                        rewrite_eval_program_internal_function_names_in_expression(
                            expression, rename_map,
                        );
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
        | Expression::NewTarget
        | Expression::This
        | Expression::Sent => {}
    }
}

fn rewrite_eval_program_internal_function_names_in_statement(
    statement: &mut Statement,
    rename_map: &HashMap<String, String>,
) {
    match statement {
        Statement::Declaration { body }
        | Statement::Block { body }
        | Statement::Labeled { body, .. } => {
            for statement in body {
                rewrite_eval_program_internal_function_names_in_statement(statement, rename_map);
            }
        }
        Statement::Var { name, value } | Statement::Let { name, value, .. } => {
            rewrite_eval_program_internal_function_name(name, rename_map);
            rewrite_eval_program_internal_function_names_in_expression(value, rename_map);
        }
        Statement::Assign { name, value } => {
            rewrite_eval_program_internal_function_name(name, rename_map);
            rewrite_eval_program_internal_function_names_in_expression(value, rename_map);
        }
        Statement::AssignMember {
            object,
            property,
            value,
        } => {
            rewrite_eval_program_internal_function_names_in_expression(object, rename_map);
            rewrite_eval_program_internal_function_names_in_expression(property, rename_map);
            rewrite_eval_program_internal_function_names_in_expression(value, rename_map);
        }
        Statement::Print { values } => {
            for value in values {
                rewrite_eval_program_internal_function_names_in_expression(value, rename_map);
            }
        }
        Statement::Expression(expression)
        | Statement::Throw(expression)
        | Statement::Return(expression)
        | Statement::Yield { value: expression }
        | Statement::YieldDelegate { value: expression } => {
            rewrite_eval_program_internal_function_names_in_expression(expression, rename_map);
        }
        Statement::With { object, body } => {
            rewrite_eval_program_internal_function_names_in_expression(object, rename_map);
            for statement in body {
                rewrite_eval_program_internal_function_names_in_statement(statement, rename_map);
            }
        }
        Statement::If {
            condition,
            then_branch,
            else_branch,
        } => {
            rewrite_eval_program_internal_function_names_in_expression(condition, rename_map);
            for statement in then_branch {
                rewrite_eval_program_internal_function_names_in_statement(statement, rename_map);
            }
            for statement in else_branch {
                rewrite_eval_program_internal_function_names_in_statement(statement, rename_map);
            }
        }
        Statement::Try {
            catch_binding,
            body,
            catch_setup,
            catch_body,
        } => {
            if let Some(catch_binding) = catch_binding {
                rewrite_eval_program_internal_function_name(catch_binding, rename_map);
            }
            for statement in body {
                rewrite_eval_program_internal_function_names_in_statement(statement, rename_map);
            }
            for statement in catch_setup {
                rewrite_eval_program_internal_function_names_in_statement(statement, rename_map);
            }
            for statement in catch_body {
                rewrite_eval_program_internal_function_names_in_statement(statement, rename_map);
            }
        }
        Statement::Switch {
            bindings,
            discriminant,
            cases,
            ..
        } => {
            for binding in bindings {
                rewrite_eval_program_internal_function_name(binding, rename_map);
            }
            rewrite_eval_program_internal_function_names_in_expression(discriminant, rename_map);
            for case in cases {
                if let Some(test) = &mut case.test {
                    rewrite_eval_program_internal_function_names_in_expression(test, rename_map);
                }
                for statement in &mut case.body {
                    rewrite_eval_program_internal_function_names_in_statement(
                        statement, rename_map,
                    );
                }
            }
        }
        Statement::For {
            init,
            per_iteration_bindings,
            condition,
            update,
            break_hook,
            body,
            ..
        } => {
            for statement in init {
                rewrite_eval_program_internal_function_names_in_statement(statement, rename_map);
            }
            for binding in per_iteration_bindings {
                rewrite_eval_program_internal_function_name(binding, rename_map);
            }
            if let Some(condition) = condition {
                rewrite_eval_program_internal_function_names_in_expression(condition, rename_map);
            }
            if let Some(update) = update {
                rewrite_eval_program_internal_function_names_in_expression(update, rename_map);
            }
            if let Some(break_hook) = break_hook {
                rewrite_eval_program_internal_function_names_in_expression(break_hook, rename_map);
            }
            for statement in body {
                rewrite_eval_program_internal_function_names_in_statement(statement, rename_map);
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
            rewrite_eval_program_internal_function_names_in_expression(condition, rename_map);
            if let Some(break_hook) = break_hook {
                rewrite_eval_program_internal_function_names_in_expression(break_hook, rename_map);
            }
            for statement in body {
                rewrite_eval_program_internal_function_names_in_statement(statement, rename_map);
            }
        }
        Statement::Break { .. } | Statement::Continue { .. } => {}
    }
}

fn mark_eval_template_object_site_key(site_key: &mut String) {
    if let Some(site_id) = site_key.strip_prefix("template-site:") {
        *site_key = format!("eval-template-site:{site_id}");
    }
}

fn mark_eval_template_object_sites_in_expression(expression: &mut Expression) {
    match expression {
        Expression::Array(elements) => {
            for element in elements {
                match element {
                    ArrayElement::Expression(expression) | ArrayElement::Spread(expression) => {
                        mark_eval_template_object_sites_in_expression(expression);
                    }
                }
            }
        }
        Expression::Object(entries) => {
            for entry in entries {
                match entry {
                    ObjectEntry::Data { key, value } => {
                        mark_eval_template_object_sites_in_expression(key);
                        mark_eval_template_object_sites_in_expression(value);
                    }
                    ObjectEntry::Getter { key, getter } => {
                        mark_eval_template_object_sites_in_expression(key);
                        mark_eval_template_object_sites_in_expression(getter);
                    }
                    ObjectEntry::Setter { key, setter } => {
                        mark_eval_template_object_sites_in_expression(key);
                        mark_eval_template_object_sites_in_expression(setter);
                    }
                    ObjectEntry::Spread(expression) => {
                        mark_eval_template_object_sites_in_expression(expression);
                    }
                }
            }
        }
        Expression::Member { object, property } => {
            mark_eval_template_object_sites_in_expression(object);
            mark_eval_template_object_sites_in_expression(property);
        }
        Expression::SuperMember { property }
        | Expression::Await(property)
        | Expression::EnumerateKeys(property)
        | Expression::GetIterator(property)
        | Expression::IteratorClose(property)
        | Expression::Unary {
            expression: property,
            ..
        } => mark_eval_template_object_sites_in_expression(property),
        Expression::Assign { value, .. } => {
            mark_eval_template_object_sites_in_expression(value);
        }
        Expression::AssignMember {
            object,
            property,
            value,
        } => {
            mark_eval_template_object_sites_in_expression(object);
            mark_eval_template_object_sites_in_expression(property);
            mark_eval_template_object_sites_in_expression(value);
        }
        Expression::AssignSuperMember { property, value } => {
            mark_eval_template_object_sites_in_expression(property);
            mark_eval_template_object_sites_in_expression(value);
        }
        Expression::Binary { left, right, .. } => {
            mark_eval_template_object_sites_in_expression(left);
            mark_eval_template_object_sites_in_expression(right);
        }
        Expression::Conditional {
            condition,
            then_expression,
            else_expression,
        } => {
            mark_eval_template_object_sites_in_expression(condition);
            mark_eval_template_object_sites_in_expression(then_expression);
            mark_eval_template_object_sites_in_expression(else_expression);
        }
        Expression::Sequence(expressions) => {
            for expression in expressions {
                mark_eval_template_object_sites_in_expression(expression);
            }
        }
        Expression::Call { callee, arguments }
        | Expression::SuperCall { callee, arguments }
        | Expression::New { callee, arguments } => {
            let is_template_object_call = matches!(callee.as_ref(), Expression::Identifier(name) if name == "__ayyTemplateObject");
            if is_template_object_call
                && let Some(
                    CallArgument::Expression(Expression::String(site_key))
                    | CallArgument::Spread(Expression::String(site_key)),
                ) = arguments.first_mut()
            {
                mark_eval_template_object_site_key(site_key);
            }
            mark_eval_template_object_sites_in_expression(callee);
            for argument in arguments {
                match argument {
                    CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                        mark_eval_template_object_sites_in_expression(expression);
                    }
                }
            }
        }
        Expression::Identifier(_)
        | Expression::Update { .. }
        | Expression::Number(_)
        | Expression::BigInt(_)
        | Expression::String(_)
        | Expression::Bool(_)
        | Expression::Null
        | Expression::Undefined
        | Expression::NewTarget
        | Expression::This
        | Expression::Sent => {}
    }
}

fn mark_eval_template_object_sites_in_statement(statement: &mut Statement) {
    match statement {
        Statement::Declaration { body }
        | Statement::Block { body }
        | Statement::Labeled { body, .. } => {
            for statement in body {
                mark_eval_template_object_sites_in_statement(statement);
            }
        }
        Statement::Var { value, .. }
        | Statement::Let { value, .. }
        | Statement::Assign { value, .. } => {
            mark_eval_template_object_sites_in_expression(value);
        }
        Statement::AssignMember {
            object,
            property,
            value,
        } => {
            mark_eval_template_object_sites_in_expression(object);
            mark_eval_template_object_sites_in_expression(property);
            mark_eval_template_object_sites_in_expression(value);
        }
        Statement::Print { values } => {
            for value in values {
                mark_eval_template_object_sites_in_expression(value);
            }
        }
        Statement::Expression(expression)
        | Statement::Throw(expression)
        | Statement::Return(expression)
        | Statement::Yield { value: expression }
        | Statement::YieldDelegate { value: expression } => {
            mark_eval_template_object_sites_in_expression(expression);
        }
        Statement::With { object, body } => {
            mark_eval_template_object_sites_in_expression(object);
            for statement in body {
                mark_eval_template_object_sites_in_statement(statement);
            }
        }
        Statement::If {
            condition,
            then_branch,
            else_branch,
        } => {
            mark_eval_template_object_sites_in_expression(condition);
            for statement in then_branch {
                mark_eval_template_object_sites_in_statement(statement);
            }
            for statement in else_branch {
                mark_eval_template_object_sites_in_statement(statement);
            }
        }
        Statement::Try {
            body,
            catch_setup,
            catch_body,
            ..
        } => {
            for statement in body {
                mark_eval_template_object_sites_in_statement(statement);
            }
            for statement in catch_setup {
                mark_eval_template_object_sites_in_statement(statement);
            }
            for statement in catch_body {
                mark_eval_template_object_sites_in_statement(statement);
            }
        }
        Statement::Switch {
            discriminant,
            cases,
            ..
        } => {
            mark_eval_template_object_sites_in_expression(discriminant);
            for case in cases {
                if let Some(test) = &mut case.test {
                    mark_eval_template_object_sites_in_expression(test);
                }
                for statement in &mut case.body {
                    mark_eval_template_object_sites_in_statement(statement);
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
                mark_eval_template_object_sites_in_statement(statement);
            }
            if let Some(condition) = condition {
                mark_eval_template_object_sites_in_expression(condition);
            }
            if let Some(update) = update {
                mark_eval_template_object_sites_in_expression(update);
            }
            if let Some(break_hook) = break_hook {
                mark_eval_template_object_sites_in_expression(break_hook);
            }
            for statement in body {
                mark_eval_template_object_sites_in_statement(statement);
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
            mark_eval_template_object_sites_in_expression(condition);
            if let Some(break_hook) = break_hook {
                mark_eval_template_object_sites_in_expression(break_hook);
            }
            for statement in body {
                mark_eval_template_object_sites_in_statement(statement);
            }
        }
        Statement::Break { .. } | Statement::Continue { .. } => {}
    }
}

pub(in crate::backend::direct_wasm) fn namespace_eval_program_internal_function_names(
    program: &mut Program,
    current_function_name: Option<&str>,
    source: &str,
) {
    let namespace =
        super::eval_namespace::eval_program_function_namespace(current_function_name, source);
    let rename_map = program
        .functions
        .iter()
        .filter(|function| is_internal_user_function_identifier(&function.name))
        .map(|function| {
            (
                function.name.clone(),
                super::eval_namespace::namespaced_internal_eval_function_name(
                    &function.name,
                    &namespace,
                ),
            )
        })
        .collect::<HashMap<_, _>>();

    for statement in &mut program.statements {
        mark_eval_template_object_sites_in_statement(statement);
    }
    for function in &mut program.functions {
        for statement in &mut function.body {
            mark_eval_template_object_sites_in_statement(statement);
        }
        for parameter in &mut function.params {
            if let Some(default) = &mut parameter.default {
                mark_eval_template_object_sites_in_expression(default);
            }
        }
    }

    if rename_map.is_empty() {
        return;
    }

    for statement in &mut program.statements {
        rewrite_eval_program_internal_function_names_in_statement(statement, &rename_map);
    }
    for function in &mut program.functions {
        rewrite_eval_program_internal_function_name(&mut function.name, &rename_map);
        if let Some(binding) = &mut function.top_level_binding {
            rewrite_eval_program_internal_function_name(binding, &rename_map);
        }
        if let Some(binding) = &mut function.self_binding {
            rewrite_eval_program_internal_function_name(binding, &rename_map);
        }
        for parameter in &mut function.params {
            rewrite_eval_program_internal_function_name(&mut parameter.name, &rename_map);
            if let Some(default) = &mut parameter.default {
                rewrite_eval_program_internal_function_names_in_expression(default, &rename_map);
            }
        }
        for statement in &mut function.body {
            rewrite_eval_program_internal_function_names_in_statement(statement, &rename_map);
        }
    }
}
