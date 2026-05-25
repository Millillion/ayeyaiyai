use super::*;
use crate::ir::hir::SwitchCase;

impl<'a> FunctionCompiler<'a> {
    fn bound_snapshot_strict_equal(left: &Expression, right: &Expression) -> Option<bool> {
        match (left, right) {
            (Expression::Bool(lhs), Expression::Bool(rhs)) => Some(lhs == rhs),
            (Expression::Number(lhs), Expression::Number(rhs)) => Some(lhs == rhs),
            (Expression::BigInt(lhs), Expression::BigInt(rhs)) => Some(lhs == rhs),
            (Expression::String(lhs), Expression::String(rhs)) => Some(lhs == rhs),
            (Expression::Null, Expression::Null)
            | (Expression::Undefined, Expression::Undefined) => Some(true),
            (Expression::Identifier(lhs), Expression::Identifier(rhs)) => Some(lhs == rhs),
            _ => Some(false),
        }
    }

    fn bound_snapshot_break_targets_switch(labels: &[String], label: Option<&String>) -> bool {
        match label {
            None => true,
            Some(label) => labels.iter().any(|candidate| candidate == label),
        }
    }

    fn execute_bound_snapshot_switch_body(
        &self,
        start_index: usize,
        labels: &[String],
        cases: &[SwitchCase],
        bindings: &mut HashMap<String, Expression>,
        current_function_name: Option<&str>,
    ) -> Option<BoundSnapshotControlFlow> {
        for case in cases.iter().skip(start_index) {
            let result = self.execute_bound_snapshot_statements(
                &case.body,
                bindings,
                current_function_name,
            )?;
            match result {
                BoundSnapshotControlFlow::None => {}
                BoundSnapshotControlFlow::Break(label)
                    if Self::bound_snapshot_break_targets_switch(labels, label.as_ref()) =>
                {
                    return Some(BoundSnapshotControlFlow::None);
                }
                other => return Some(other),
            }
        }
        Some(BoundSnapshotControlFlow::None)
    }

    fn bound_snapshot_switch_case_matches(
        &self,
        discriminant: &Expression,
        case: &SwitchCase,
        bindings: &mut HashMap<String, Expression>,
        current_function_name: Option<&str>,
    ) -> Option<bool> {
        let test = case.test.as_ref()?;
        if !matches!(
            (discriminant, test),
            (Expression::Identifier(_), Expression::Identifier(_))
        ) && static_expression_matches(discriminant, test)
        {
            return Some(true);
        }
        if let (Expression::Identifier(left), Expression::Identifier(right)) = (discriminant, test)
            && !self.bound_snapshot_identifier_is_value_builtin(left)
            && !self.bound_snapshot_identifier_is_value_builtin(right)
            && self.resolve_bound_snapshot_binding_name(left, bindings)
                == self.resolve_bound_snapshot_binding_name(right, bindings)
        {
            return Some(true);
        }
        let test =
            self.evaluate_bound_snapshot_expression(test, bindings, current_function_name)?;
        Self::bound_snapshot_strict_equal(discriminant, &test)
    }

    fn execute_bound_snapshot_switch_statement(
        &self,
        labels: &[String],
        discriminant: &Expression,
        cases: &[SwitchCase],
        bindings: &mut HashMap<String, Expression>,
        current_function_name: Option<&str>,
    ) -> Option<BoundSnapshotControlFlow> {
        let discriminant =
            self.evaluate_bound_snapshot_expression(discriminant, bindings, current_function_name)?;
        let default_index = cases.iter().position(|case| case.test.is_none());
        let start_index = if let Some(default_index) = default_index {
            let before_default_match = (0..default_index).find(|index| {
                self.bound_snapshot_switch_case_matches(
                    &discriminant,
                    &cases[*index],
                    bindings,
                    current_function_name,
                )
                .unwrap_or(false)
            });
            if let Some(index) = before_default_match {
                Some(index)
            } else {
                (default_index + 1..cases.len())
                    .find(|index| {
                        self.bound_snapshot_switch_case_matches(
                            &discriminant,
                            &cases[*index],
                            bindings,
                            current_function_name,
                        )
                        .unwrap_or(false)
                    })
                    .or(Some(default_index))
            }
        } else {
            (0..cases.len()).find(|index| {
                self.bound_snapshot_switch_case_matches(
                    &discriminant,
                    &cases[*index],
                    bindings,
                    current_function_name,
                )
                .unwrap_or(false)
            })
        };

        if let Some(start_index) = start_index {
            self.execute_bound_snapshot_switch_body(
                start_index,
                labels,
                cases,
                bindings,
                current_function_name,
            )
        } else {
            Some(BoundSnapshotControlFlow::None)
        }
    }

    pub(in crate::backend::direct_wasm) fn execute_bound_snapshot_statements(
        &self,
        statements: &[Statement],
        bindings: &mut HashMap<String, Expression>,
        current_function_name: Option<&str>,
    ) -> Option<BoundSnapshotControlFlow> {
        for statement in statements {
            match statement {
                Statement::Block { body } => {
                    if let Some(result) = self.execute_bound_snapshot_statements(
                        body,
                        bindings,
                        current_function_name,
                    ) && !matches!(result, BoundSnapshotControlFlow::None)
                    {
                        return Some(result);
                    }
                }
                Statement::If {
                    condition,
                    then_branch,
                    else_branch,
                } => {
                    let condition = self.evaluate_bound_snapshot_expression(
                        condition,
                        bindings,
                        current_function_name,
                    )?;
                    let branch = if matches!(condition, Expression::Bool(true)) {
                        then_branch
                    } else if matches!(condition, Expression::Bool(false)) {
                        else_branch
                    } else {
                        return None;
                    };
                    if let Some(result) = self.execute_bound_snapshot_statements(
                        branch,
                        bindings,
                        current_function_name,
                    ) && !matches!(result, BoundSnapshotControlFlow::None)
                    {
                        return Some(result);
                    }
                }
                Statement::Switch {
                    labels,
                    discriminant,
                    cases,
                    ..
                } => {
                    let result = self.execute_bound_snapshot_switch_statement(
                        labels,
                        discriminant,
                        cases,
                        bindings,
                        current_function_name,
                    )?;
                    if !matches!(result, BoundSnapshotControlFlow::None) {
                        return Some(result);
                    }
                }
                Statement::Return(value) => {
                    return Some(BoundSnapshotControlFlow::Return(
                        self.evaluate_bound_snapshot_expression(
                            value,
                            bindings,
                            current_function_name,
                        )?,
                    ));
                }
                Statement::Throw(value) => {
                    let throw_value = if let Expression::Identifier(name) = value {
                        Expression::Identifier(
                            self.resolve_bound_snapshot_binding_name(name, bindings)
                                .to_string(),
                        )
                    } else {
                        self.evaluate_bound_snapshot_expression(
                            value,
                            bindings,
                            current_function_name,
                        )?
                    };
                    return Some(BoundSnapshotControlFlow::Throw(throw_value));
                }
                Statement::Var { name, value } => {
                    let resolved_name = self
                        .resolve_bound_snapshot_binding_name(name, bindings)
                        .to_string();
                    if matches!(value, Expression::Undefined)
                        && bindings.contains_key(&resolved_name)
                    {
                        continue;
                    }
                    let evaluated_value = self.evaluate_bound_snapshot_expression(
                        value,
                        bindings,
                        current_function_name,
                    )?;
                    bindings.insert(resolved_name, evaluated_value);
                }
                Statement::Let { name, value, .. } | Statement::Assign { name, value } => {
                    let resolved_name = self
                        .resolve_bound_snapshot_binding_name(name, bindings)
                        .to_string();
                    let evaluated_value = self.evaluate_bound_snapshot_expression(
                        value,
                        bindings,
                        current_function_name,
                    )?;
                    let value = if let Expression::Identifier(value_name) = value
                        && matches!(
                            evaluated_value,
                            Expression::Array(_)
                                | Expression::Object(_)
                                | Expression::Identifier(_)
                        ) {
                        Expression::Identifier(
                            self.resolve_bound_snapshot_binding_name(value_name, bindings)
                                .to_string(),
                        )
                    } else {
                        evaluated_value
                    };
                    bindings.insert(resolved_name, value);
                }
                Statement::AssignMember {
                    object,
                    property,
                    value,
                } => {
                    self.evaluate_bound_snapshot_assign_member_expression(
                        object,
                        property,
                        value,
                        bindings,
                        current_function_name,
                    )?;
                }
                Statement::Expression(expression) => {
                    self.evaluate_bound_snapshot_expression(
                        expression,
                        bindings,
                        current_function_name,
                    )?;
                }
                Statement::Print { values } => {
                    for value in values {
                        self.evaluate_bound_snapshot_expression(
                            value,
                            bindings,
                            current_function_name,
                        )?;
                    }
                }
                Statement::Break { label } => {
                    return Some(BoundSnapshotControlFlow::Break(label.clone()));
                }
                _ => return None,
            }
        }
        Some(BoundSnapshotControlFlow::None)
    }

    pub(in crate::backend::direct_wasm) fn apply_bound_snapshot_member_assignment(
        &self,
        object: &Expression,
        property: &Expression,
        value: &Expression,
        bindings: &mut HashMap<String, Expression>,
        current_function_name: Option<&str>,
    ) -> Option<Expression> {
        let mut evaluated_object = None;
        let binding_names = match object {
            Expression::Identifier(object_name) => vec![
                self.resolve_bound_snapshot_binding_name(object_name, bindings)
                    .to_string(),
            ],
            Expression::This => {
                let this_binding = bindings.get("this").cloned()?;
                match this_binding {
                    Expression::Identifier(object_name) => vec![
                        self.resolve_bound_snapshot_binding_name(&object_name, bindings)
                            .to_string(),
                    ],
                    _ => vec!["this".to_string()],
                }
            }
            _ => {
                let object_value = self.evaluate_bound_snapshot_expression(
                    object,
                    bindings,
                    current_function_name,
                )?;
                let binding_names = match &object_value {
                    Expression::Identifier(object_name) => vec![
                        self.resolve_bound_snapshot_binding_name(object_name, bindings)
                            .to_string(),
                    ],
                    Expression::This => vec!["this".to_string()],
                    _ => return None,
                };
                evaluated_object = Some(object_value);
                binding_names
            }
        };
        let property =
            self.evaluate_bound_snapshot_expression(property, bindings, current_function_name)?;
        let value =
            self.evaluate_bound_snapshot_expression(value, bindings, current_function_name)?;
        if std::env::var_os("AYY_TRACE_THIS_FLOW").is_some() && matches!(object, Expression::This) {
            eprintln!(
                "this_flow bound_snapshot_member_assignment before current_fn={current_function_name:?} this_binding={:?} property={property:?} value={value:?}",
                bindings.get("this")
            );
        }
        let current_object = binding_names
            .iter()
            .find_map(|object_name| bindings.get(object_name).cloned())
            .unwrap_or_else(|| {
                evaluated_object
                    .clone()
                    .or_else(|| {
                        self.evaluate_bound_snapshot_expression(
                            object,
                            bindings,
                            current_function_name,
                        )
                    })
                    .unwrap_or(Expression::Undefined)
            });
        let mut object_binding = self.resolve_object_binding_from_expression(&current_object)?;
        object_binding_set_property(&mut object_binding, property, value.clone());
        let updated_object = object_binding_to_expression(&object_binding);
        if std::env::var_os("AYY_TRACE_THIS_FLOW").is_some() && matches!(object, Expression::This) {
            eprintln!(
                "this_flow bound_snapshot_member_assignment after current_fn={current_function_name:?} updated_object={updated_object:?}"
            );
        }
        for object_name in binding_names {
            bindings.insert(object_name, updated_object.clone());
        }
        Some(value)
    }

    pub(in crate::backend::direct_wasm) fn bound_snapshot_array_expression(
        &self,
        expression: &Expression,
        bindings: &HashMap<String, Expression>,
    ) -> Option<Vec<ArrayElement>> {
        match expression {
            Expression::Array(elements) => Some(elements.clone()),
            Expression::Identifier(name) => {
                let resolved_name = self.resolve_bound_snapshot_binding_name(name, bindings);
                if let Some(Expression::Array(elements)) = bindings.get(resolved_name) {
                    return Some(elements.clone());
                }
                let array_binding = self.resolve_array_binding_from_expression(
                    &Expression::Identifier(resolved_name.to_string()),
                )?;
                Some(
                    array_binding
                        .values
                        .into_iter()
                        .map(|value| {
                            ArrayElement::Expression(value.unwrap_or(Expression::Undefined))
                        })
                        .collect(),
                )
            }
            _ => None,
        }
    }

    pub(in crate::backend::direct_wasm) fn apply_bound_snapshot_array_push(
        &self,
        object: &Expression,
        arguments: &[CallArgument],
        bindings: &mut HashMap<String, Expression>,
        current_function_name: Option<&str>,
    ) -> Option<Expression> {
        let resolved_object_name = match object {
            Expression::Identifier(name) => Some(
                self.resolve_bound_snapshot_binding_name(name, bindings)
                    .to_string(),
            ),
            _ => None,
        };
        let object_value = match self.evaluate_bound_snapshot_expression(
            object,
            bindings,
            current_function_name,
        ) {
            Some(value) => value,
            None => {
                #[cfg(test)]
                eprintln!("bound_snapshot_array_push object_eval_none object={object:?}");
                return None;
            }
        };
        let mut elements = match self.bound_snapshot_array_expression(&object_value, bindings) {
            Some(elements) => elements,
            None => {
                #[cfg(test)]
                eprintln!(
                    "bound_snapshot_array_push object_not_array object={object:?} value={object_value:?}"
                );
                return None;
            }
        };
        for argument in arguments {
            match argument {
                CallArgument::Expression(expression) => {
                    let value = match self.evaluate_bound_snapshot_expression(
                        expression,
                        bindings,
                        current_function_name,
                    ) {
                        Some(value) => value,
                        None => {
                            #[cfg(test)]
                            eprintln!(
                                "bound_snapshot_array_push argument_eval_none expression={expression:?}"
                            );
                            return None;
                        }
                    };
                    elements.push(ArrayElement::Expression(value));
                }
                CallArgument::Spread(expression) => {
                    let value = match self.evaluate_bound_snapshot_expression(
                        expression,
                        bindings,
                        current_function_name,
                    ) {
                        Some(value) => value,
                        None => {
                            #[cfg(test)]
                            eprintln!(
                                "bound_snapshot_array_push spread_eval_none expression={expression:?}"
                            );
                            return None;
                        }
                    };
                    let spread_elements = match self
                        .bound_snapshot_array_expression(&value, bindings)
                    {
                        Some(elements) => elements,
                        None => {
                            #[cfg(test)]
                            eprintln!(
                                "bound_snapshot_array_push spread_not_array expression={expression:?} value={value:?}"
                            );
                            return None;
                        }
                    };
                    for element in spread_elements {
                        let ArrayElement::Expression(value) = element else {
                            return None;
                        };
                        elements.push(ArrayElement::Expression(value));
                    }
                }
            }
        }
        if let Some(resolved_object_name) = resolved_object_name {
            bindings.insert(resolved_object_name, Expression::Array(elements.clone()));
        }
        Some(Expression::Number(elements.len() as f64))
    }
}
