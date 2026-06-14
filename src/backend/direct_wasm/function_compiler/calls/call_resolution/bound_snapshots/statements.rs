use super::*;
use crate::ir::hir::SwitchCase;
use std::collections::HashSet;

const BOUND_SNAPSHOT_LOOP_ITERATION_LIMIT: usize = 4096;

impl<'a> FunctionCompiler<'a> {
    fn bound_snapshot_static_throw_expression(
        &self,
        throw_value: &StaticThrowValue,
    ) -> Option<Expression> {
        self.resolve_static_throw_value_expression(throw_value)
            .map(|value| self.materialize_static_expression(&value))
    }

    fn evaluate_bound_snapshot_statement_value(
        &self,
        value: &Expression,
        bindings: &mut HashMap<String, Expression>,
        current_function_name: Option<&str>,
    ) -> Option<Result<Expression, Expression>> {
        if let Expression::Await(awaited) = value {
            let awaited_outcome = self
                .resolve_static_await_resolution_outcome(value)
                .or_else(|| {
                    let evaluated = self.evaluate_bound_snapshot_expression(
                        awaited,
                        bindings,
                        current_function_name,
                    )?;
                    self.resolve_static_await_resolution_outcome(&Expression::Await(Box::new(
                        evaluated,
                    )))
                })?;
            return Some(match awaited_outcome {
                StaticEvalOutcome::Value(value) => Ok(value),
                StaticEvalOutcome::Throw(throw_value) => {
                    Err(self.bound_snapshot_static_throw_expression(&throw_value)?)
                }
            });
        }
        Some(Ok(self.evaluate_bound_snapshot_expression(
            value,
            bindings,
            current_function_name,
        )?))
    }

    fn execute_bound_snapshot_try_statement(
        &self,
        body: &[Statement],
        catch_binding: Option<&String>,
        catch_setup: &[Statement],
        catch_body: &[Statement],
        bindings: &mut HashMap<String, Expression>,
        current_function_name: Option<&str>,
    ) -> Option<BoundSnapshotControlFlow> {
        let mut try_bindings = bindings.clone();
        match self.execute_bound_snapshot_statements(
            body,
            &mut try_bindings,
            current_function_name,
        )? {
            BoundSnapshotControlFlow::Throw(value) => {
                if let Some(catch_binding) = catch_binding {
                    let resolved_catch_binding = self
                        .resolve_bound_snapshot_binding_name(catch_binding, &try_bindings)
                        .to_string();
                    try_bindings.insert(resolved_catch_binding, value);
                }
                let setup_result = self.execute_bound_snapshot_statements(
                    catch_setup,
                    &mut try_bindings,
                    current_function_name,
                )?;
                if !matches!(setup_result, BoundSnapshotControlFlow::None) {
                    *bindings = try_bindings;
                    return Some(setup_result);
                }
                let catch_result = self.execute_bound_snapshot_statements(
                    catch_body,
                    &mut try_bindings,
                    current_function_name,
                )?;
                *bindings = try_bindings;
                Some(catch_result)
            }
            other => {
                *bindings = try_bindings;
                Some(other)
            }
        }
    }

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

    fn bound_snapshot_expression_is_fresh_boxed_primitive(&self, expression: &Expression) -> bool {
        matches!(
            expression,
            Expression::New { callee, .. }
                if matches!(
                    callee.as_ref(),
                    Expression::Identifier(name)
                        if matches!(name.as_str(), "Boolean" | "Number" | "String")
                            && self.is_unshadowed_builtin_identifier(name)
                )
        )
    }

    fn evaluate_bound_snapshot_call_arguments_for_effects(
        &self,
        arguments: &[CallArgument],
        bindings: &mut HashMap<String, Expression>,
        current_function_name: Option<&str>,
    ) -> Option<()> {
        for argument in arguments {
            self.evaluate_bound_snapshot_expression(
                argument.expression(),
                bindings,
                current_function_name,
            )?;
        }
        Some(())
    }

    fn evaluate_bound_snapshot_fresh_boxed_primitive_for_effects(
        &self,
        expression: &Expression,
        bindings: &mut HashMap<String, Expression>,
        current_function_name: Option<&str>,
    ) -> Option<()> {
        let Expression::New { arguments, .. } = expression else {
            return Some(());
        };
        self.evaluate_bound_snapshot_call_arguments_for_effects(
            arguments,
            bindings,
            current_function_name,
        )
    }

    fn bound_snapshot_boxed_primitive_identity_key(
        &self,
        expression: &Expression,
        bindings: &HashMap<String, Expression>,
    ) -> Option<String> {
        let mut visited = HashSet::new();
        self.bound_snapshot_boxed_primitive_identity_key_inner(expression, bindings, &mut visited)
    }

    fn bound_snapshot_boxed_primitive_identity_key_inner(
        &self,
        expression: &Expression,
        bindings: &HashMap<String, Expression>,
        visited: &mut HashSet<String>,
    ) -> Option<String> {
        let Expression::Identifier(name) = expression else {
            return None;
        };
        let resolved_name = self
            .resolve_bound_snapshot_binding_name(name, bindings)
            .to_string();
        if !visited.insert(resolved_name.clone()) {
            return None;
        }
        let value = bindings
            .get(&resolved_name)
            .or_else(|| self.global_value_binding(&resolved_name))
            .or_else(|| {
                (resolved_name != *name)
                    .then(|| self.global_value_binding(name))
                    .flatten()
            })?;
        if self.bound_snapshot_expression_is_fresh_boxed_primitive(value) {
            return Some(format!("boxed-primitive:{resolved_name}"));
        }
        if let Expression::Identifier(alias) = value
            && alias != name
        {
            return self
                .bound_snapshot_boxed_primitive_identity_key_inner(value, bindings, visited);
        }
        None
    }

    fn evaluate_bound_snapshot_switch_operand(
        &self,
        expression: &Expression,
        bindings: &mut HashMap<String, Expression>,
        current_function_name: Option<&str>,
    ) -> Option<Expression> {
        if self
            .bound_snapshot_boxed_primitive_identity_key(expression, bindings)
            .is_some()
        {
            return Some(expression.clone());
        }
        if self.bound_snapshot_expression_is_fresh_boxed_primitive(expression) {
            self.evaluate_bound_snapshot_fresh_boxed_primitive_for_effects(
                expression,
                bindings,
                current_function_name,
            )?;
            return Some(expression.clone());
        }
        self.evaluate_bound_snapshot_expression(expression, bindings, current_function_name)
    }

    fn bound_snapshot_switch_identity_match(
        &self,
        left: &Expression,
        right: &Expression,
        bindings: &HashMap<String, Expression>,
    ) -> Option<bool> {
        if self.bound_snapshot_expression_is_fresh_boxed_primitive(left)
            || self.bound_snapshot_expression_is_fresh_boxed_primitive(right)
        {
            return Some(false);
        }
        match (
            self.bound_snapshot_boxed_primitive_identity_key(left, bindings),
            self.bound_snapshot_boxed_primitive_identity_key(right, bindings),
        ) {
            (Some(left_key), Some(right_key)) => Some(left_key == right_key),
            (Some(_), None) | (None, Some(_)) => Some(false),
            (None, None) => None,
        }
    }

    fn bound_snapshot_break_targets_switch(labels: &[String], label: Option<&String>) -> bool {
        match label {
            None => true,
            Some(label) => labels.iter().any(|candidate| candidate == label),
        }
    }

    fn bound_snapshot_break_targets_loop(labels: &[String], label: Option<&String>) -> bool {
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
        if self.bound_snapshot_expression_is_fresh_boxed_primitive(test) {
            self.evaluate_bound_snapshot_fresh_boxed_primitive_for_effects(
                test,
                bindings,
                current_function_name,
            )?;
            return Some(false);
        }
        if let Some(matches) =
            self.bound_snapshot_switch_identity_match(discriminant, test, bindings)
        {
            return Some(matches);
        }
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
            self.evaluate_bound_snapshot_switch_operand(test, bindings, current_function_name)?;
        if let Some(matches) =
            self.bound_snapshot_switch_identity_match(discriminant, &test, bindings)
        {
            return Some(matches);
        }
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
        let discriminant = self.evaluate_bound_snapshot_switch_operand(
            discriminant,
            bindings,
            current_function_name,
        )?;
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

    /// Evaluates `new <ErrorCtor>(args)` throw values whose constructor is an
    /// unshadowed global error constructor and whose arguments all evaluate
    /// under the snapshot bindings; the rebuilt construction references no
    /// callee locals, so it is safe to replay outside the callee scope.
    fn bound_snapshot_scope_independent_error_construction(
        &self,
        expression: &Expression,
        bindings: &mut HashMap<String, Expression>,
        current_function_name: Option<&str>,
    ) -> Option<Expression> {
        let Expression::New { callee, arguments } = expression else {
            return None;
        };
        let Expression::Identifier(constructor_name) = callee.as_ref() else {
            return None;
        };
        if !matches!(
            constructor_name.as_str(),
            "Error"
                | "TypeError"
                | "RangeError"
                | "ReferenceError"
                | "SyntaxError"
                | "EvalError"
                | "URIError"
                | "AggregateError"
                | "Test262Error"
        ) || !self.is_unshadowed_builtin_identifier(constructor_name)
        {
            return None;
        }
        let evaluated_arguments = arguments
            .iter()
            .map(|argument| match argument {
                CallArgument::Expression(expression) => self
                    .evaluate_bound_snapshot_expression(expression, bindings, current_function_name)
                    .map(CallArgument::Expression),
                CallArgument::Spread(_) => None,
            })
            .collect::<Option<Vec<_>>>()?;
        Some(Expression::New {
            callee: callee.clone(),
            arguments: evaluated_arguments,
        })
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
                    // A `None` from the nested execution means the snapshot
                    // could not resolve the block, not that it completed
                    // normally; swallowing it would let execution continue
                    // past statements (such as conditional returns) that may
                    // run at runtime.
                    let result = self.execute_bound_snapshot_statements(
                        body,
                        bindings,
                        current_function_name,
                    )?;
                    if !matches!(result, BoundSnapshotControlFlow::None) {
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
                    let result = self.execute_bound_snapshot_statements(
                        branch,
                        bindings,
                        current_function_name,
                    )?;
                    if !matches!(result, BoundSnapshotControlFlow::None) {
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
                Statement::For {
                    labels,
                    init,
                    per_iteration_bindings,
                    condition,
                    update,
                    break_hook,
                    body,
                } => {
                    if !per_iteration_bindings.is_empty() || break_hook.is_some() {
                        return None;
                    }

                    let init_result = self.execute_bound_snapshot_statements(
                        init,
                        bindings,
                        current_function_name,
                    )?;
                    if !matches!(init_result, BoundSnapshotControlFlow::None) {
                        return Some(init_result);
                    }

                    let mut completed = false;
                    for _ in 0..BOUND_SNAPSHOT_LOOP_ITERATION_LIMIT {
                        if let Some(condition) = condition {
                            let condition = self.evaluate_bound_snapshot_expression(
                                condition,
                                bindings,
                                current_function_name,
                            )?;
                            match condition {
                                Expression::Bool(true) => {}
                                Expression::Bool(false) => {
                                    completed = true;
                                    break;
                                }
                                _ => return None,
                            }
                        }

                        let body_result = self.execute_bound_snapshot_statements(
                            body,
                            bindings,
                            current_function_name,
                        )?;
                        match body_result {
                            BoundSnapshotControlFlow::None => {}
                            BoundSnapshotControlFlow::Break(label)
                                if Self::bound_snapshot_break_targets_loop(
                                    labels,
                                    label.as_ref(),
                                ) =>
                            {
                                completed = true;
                                break;
                            }
                            other => return Some(other),
                        }

                        if let Some(update) = update
                            && let Err(throw_value) = self.evaluate_bound_snapshot_statement_value(
                                update,
                                bindings,
                                current_function_name,
                            )?
                        {
                            return Some(BoundSnapshotControlFlow::Throw(throw_value));
                        }
                    }

                    if !completed {
                        return None;
                    }
                }
                Statement::Return(value) => {
                    let value = match self.evaluate_bound_snapshot_statement_value(
                        value,
                        bindings,
                        current_function_name,
                    )? {
                        Ok(value) => value,
                        Err(throw_value) => {
                            return Some(BoundSnapshotControlFlow::Throw(throw_value));
                        }
                    };
                    return Some(BoundSnapshotControlFlow::Return(value));
                }
                Statement::Throw(value) => {
                    let throw_value = if let Expression::Identifier(name) = value {
                        Expression::Identifier(
                            self.resolve_bound_snapshot_captured_self_binding_name(
                                name,
                                bindings,
                                current_function_name,
                            )
                            .unwrap_or_else(|| {
                                self.resolve_bound_snapshot_binding_name(name, bindings)
                                    .to_string()
                            }),
                        )
                    } else if let Some(evaluated) = self.evaluate_bound_snapshot_expression(
                        value,
                        bindings,
                        current_function_name,
                    ) {
                        evaluated
                    } else {
                        // `throw new <ErrorCtor>(...)` with scope-independent
                        // arguments is a common iterator-protocol shape; keep
                        // the construction so effects before the throw are
                        // not discarded with the whole snapshot.
                        self.bound_snapshot_scope_independent_error_construction(
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
                    let evaluated_value = match self.evaluate_bound_snapshot_statement_value(
                        value,
                        bindings,
                        current_function_name,
                    )? {
                        Ok(value) => value,
                        Err(throw_value) => {
                            return Some(BoundSnapshotControlFlow::Throw(throw_value));
                        }
                    };
                    bindings.insert(resolved_name, evaluated_value);
                }
                Statement::Let { name, value, .. } => {
                    let resolved_name = self
                        .resolve_bound_snapshot_binding_name(name, bindings)
                        .to_string();
                    let evaluated_value = match self.evaluate_bound_snapshot_statement_value(
                        value,
                        bindings,
                        current_function_name,
                    )? {
                        Ok(value) => value,
                        Err(throw_value) => {
                            return Some(BoundSnapshotControlFlow::Throw(throw_value));
                        }
                    };
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
                Statement::Assign { name, value } => {
                    let evaluated_value = match self.evaluate_bound_snapshot_statement_value(
                        value,
                        bindings,
                        current_function_name,
                    )? {
                        Ok(value) => value,
                        Err(throw_value) => {
                            return Some(BoundSnapshotControlFlow::Throw(throw_value));
                        }
                    };
                    if self
                        .resolve_bound_snapshot_captured_self_binding_name(
                            name,
                            bindings,
                            current_function_name,
                        )
                        .is_some()
                    {
                        if self.bound_snapshot_current_function_is_strict(current_function_name) {
                            return None;
                        }
                        continue;
                    }
                    let resolved_name = self
                        .resolve_bound_snapshot_binding_name(name, bindings)
                        .to_string();
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
                    if let Err(throw_value) = self.evaluate_bound_snapshot_statement_value(
                        expression,
                        bindings,
                        current_function_name,
                    )? {
                        return Some(BoundSnapshotControlFlow::Throw(throw_value));
                    }
                }
                // Print is I/O: a snapshot that "succeeds" past it would fold
                // the call to its return value and silently drop the output.
                Statement::Print { .. } => return None,
                Statement::Try {
                    body,
                    catch_binding,
                    catch_setup,
                    catch_body,
                } => {
                    let result = self.execute_bound_snapshot_try_statement(
                        body,
                        catch_binding.as_ref(),
                        catch_setup,
                        catch_body,
                        bindings,
                        current_function_name,
                    )?;
                    if !matches!(result, BoundSnapshotControlFlow::None) {
                        return Some(result);
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
        if crate::ayy_env_flag!("AYY_TRACE_THIS_FLOW") && matches!(object, Expression::This) {
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
        if crate::ayy_env_flag!("AYY_TRACE_THIS_FLOW") && matches!(object, Expression::This) {
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
