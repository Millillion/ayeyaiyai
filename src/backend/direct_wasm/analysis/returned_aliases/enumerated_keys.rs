use super::*;

pub(in crate::backend::direct_wasm) fn collect_enumerated_keys_param_index(
    function: &FunctionDeclaration,
) -> Option<usize> {
    let returned_identifier = collect_returned_identifier(&function.body)?;
    let initialized_array = function.body.iter().any(|statement| {
        matches!(
            statement,
            Statement::Var { name, value }
                | Statement::Let { name, value, .. }
                | Statement::Assign { name, value }
                if name == &returned_identifier
                    && matches!(value, Expression::Array(elements) if elements.is_empty())
        )
    });
    if !initialized_array {
        return None;
    }

    function.body.iter().find_map(|statement| {
        match_enumerated_keys_collector_loop(statement, &returned_identifier, function)
    })
}

pub(in crate::backend::direct_wasm) fn match_enumerated_keys_collector_loop(
    statement: &Statement,
    returned_identifier: &str,
    function: &FunctionDeclaration,
) -> Option<usize> {
    let Statement::For {
        init,
        condition,
        update,
        body,
        ..
    } = statement
    else {
        return None;
    };

    let (target_name, param_index) = init.iter().find_map(|statement| match statement {
        Statement::Let { name, value, .. } | Statement::Var { name, value } => {
            let Expression::Identifier(param_name) = value else {
                return None;
            };
            function
                .params
                .iter()
                .position(|parameter| parameter.name == *param_name)
                .map(|param_index| (name.clone(), param_index))
        }
        _ => None,
    })?;

    let keys_name = init.iter().find_map(|statement| match statement {
        Statement::Let { name, value, .. } | Statement::Var { name, value } => {
            let Expression::EnumerateKeys(target) = value else {
                return None;
            };
            matches!(target.as_ref(), Expression::Identifier(current_target) if current_target == &target_name)
                .then(|| name.clone())
        }
        _ => None,
    })?;

    let index_name = match condition.as_ref()? {
        Expression::Binary {
            op: BinaryOp::LessThan,
            left,
            right,
        } => {
            let Expression::Identifier(index_name) = left.as_ref() else {
                return None;
            };
            matches!(
                right.as_ref(),
                Expression::Member { object, property }
                    if matches!(object.as_ref(), Expression::Identifier(name) if name == &keys_name)
                        && matches!(property.as_ref(), Expression::String(property_name) if property_name == "length")
            )
            .then(|| index_name.clone())?
        }
        _ => return None,
    };

    if !matches!(
        update.as_ref()?,
        Expression::Update {
            name,
            op: UpdateOp::Increment,
            ..
        } if name == &index_name
    ) {
        return None;
    }

    let loop_value_name =
        find_enumerated_loop_value_name(body, &keys_name, &index_name, &target_name)?;

    contains_enumerated_key_push(
        body,
        returned_identifier,
        &loop_value_name,
        &keys_name,
        &index_name,
        &target_name,
    )
    .then_some(param_index)
}

fn is_current_enumerated_key(expression: &Expression, keys_name: &str, index_name: &str) -> bool {
    matches!(
        expression,
        Expression::Member { object, property }
            if matches!(object.as_ref(), Expression::Identifier(current_keys) if current_keys == keys_name)
                && matches!(property.as_ref(), Expression::Identifier(current_index) if current_index == index_name)
    )
}

fn is_enumerated_key_guard(
    condition: &Expression,
    keys_name: &str,
    index_name: &str,
    target_name: &str,
) -> bool {
    matches!(
        condition,
        Expression::Binary {
            op: BinaryOp::In,
            left,
            right,
        } if is_current_enumerated_key(left, keys_name, index_name)
            && matches!(right.as_ref(), Expression::Identifier(current_target) if current_target == target_name)
    )
}

fn find_enumerated_loop_value_name(
    statements: &[Statement],
    keys_name: &str,
    index_name: &str,
    target_name: &str,
) -> Option<String> {
    statements.iter().find_map(|statement| match statement {
        Statement::Let { name, value, .. } | Statement::Var { name, value }
            if is_current_enumerated_key(value, keys_name, index_name) =>
        {
            Some(name.clone())
        }
        Statement::Block { body }
        | Statement::Declaration { body }
        | Statement::Labeled { body, .. } => {
            find_enumerated_loop_value_name(body, keys_name, index_name, target_name)
        }
        Statement::If {
            condition,
            then_branch,
            else_branch,
        } if else_branch.is_empty()
            && is_enumerated_key_guard(condition, keys_name, index_name, target_name) =>
        {
            find_enumerated_loop_value_name(then_branch, keys_name, index_name, target_name)
        }
        _ => None,
    })
}

fn contains_enumerated_key_push(
    statements: &[Statement],
    returned_identifier: &str,
    loop_value_name: &str,
    keys_name: &str,
    index_name: &str,
    target_name: &str,
) -> bool {
    statements.iter().any(|statement| match statement {
        Statement::Expression(Expression::Call { callee, arguments })
            if matches!(
                callee.as_ref(),
                Expression::Member { object, property }
                    if matches!(object.as_ref(), Expression::Identifier(name) if name == returned_identifier)
                        && matches!(property.as_ref(), Expression::String(property_name) if property_name == "push")
            ) && matches!(
                arguments.as_slice(),
                [CallArgument::Expression(Expression::Identifier(argument_name))]
                    if argument_name == loop_value_name
            ) =>
        {
            true
        }
        Statement::Block { body } | Statement::Declaration { body } | Statement::Labeled { body, .. } => {
            contains_enumerated_key_push(
                body,
                returned_identifier,
                loop_value_name,
                keys_name,
                index_name,
                target_name,
            )
        }
        Statement::If {
            condition,
            then_branch,
            else_branch,
        } if else_branch.is_empty()
            && is_enumerated_key_guard(condition, keys_name, index_name, target_name) =>
        {
            contains_enumerated_key_push(
                then_branch,
                returned_identifier,
                loop_value_name,
                keys_name,
                index_name,
                target_name,
            )
        }
        _ => false,
    })
}
