use super::*;

impl<'a> FunctionCompiler<'a> {
    pub(in crate::backend::direct_wasm) fn materialize_conditional_expression(
        &self,
        condition: &Expression,
        then_expression: &Expression,
        else_expression: &Expression,
    ) -> Expression {
        let materialized_condition = self.materialize_static_expression(condition);
        let condition_value = if self.if_condition_depends_on_active_loop_assignment(condition)
            || self.expression_has_dynamic_member_property_access(condition)
        {
            None
        } else {
            self.resolve_static_if_condition_value(&materialized_condition)
        }
        .or_else(|| {
            (!self.expression_has_dynamic_member_property_access(condition)
                && self.if_condition_depends_on_active_iterator_loop_assignment(condition))
            .then(|| self.resolve_static_loop_dependent_if_condition_value(condition))
            .flatten()
        });
        if let Some(condition_value) = condition_value {
            let branch = if condition_value {
                then_expression
            } else {
                else_expression
            };
            return self.materialize_static_expression(branch);
        }
        Expression::Conditional {
            condition: Box::new(materialized_condition),
            then_expression: Box::new(self.materialize_static_expression(then_expression)),
            else_expression: Box::new(self.materialize_static_expression(else_expression)),
        }
    }

    pub(in crate::backend::direct_wasm) fn materialize_call_expression(
        &self,
        expression: &Expression,
        callee: &Expression,
        arguments: &[CallArgument],
    ) -> Expression {
        let static_has_own_result = match callee {
            Expression::Member { property, .. } if matches!(property.as_ref(), Expression::String(name) if name == "hasOwnProperty") => {
                self.resolve_static_has_own_property_call_result(expression)
            }
            _ => None,
        };
        if let Some(value) = static_has_own_result
            .map(Expression::Bool)
            .or_else(|| {
                self.resolve_static_reflect_has_call_result(expression)
                    .map(Expression::Bool)
            })
            .or_else(|| {
                self.resolve_static_is_nan_call_result(expression)
                    .map(Expression::Bool)
            })
            .or_else(|| {
                self.resolve_static_object_is_call_result(expression)
                    .map(Expression::Bool)
            })
            .or_else(|| {
                self.resolve_static_array_is_array_call_result(expression)
                    .map(Expression::Bool)
            })
        {
            return value;
        }
        if arguments.is_empty()
            && let Expression::Identifier(function_name) = callee
            && let Some(value) = self.infer_static_class_init_call_result_expression(function_name)
        {
            return self.materialize_static_expression(&value);
        }
        if let Expression::Member { object, property } = callee
            && matches!(object.as_ref(), Expression::Identifier(name) if name == "Object")
            && matches!(property.as_ref(), Expression::String(name) if name == "getPrototypeOf")
            && let Some(CallArgument::Expression(target) | CallArgument::Spread(target)) =
                arguments.first()
            && let Some(prototype) = self.resolve_static_object_prototype_expression(target)
        {
            return self.materialize_static_expression(&prototype);
        }
        if arguments.is_empty()
            && let Expression::Member { object, property } = callee
            && let Expression::String(property_name) = property.as_ref()
            && matches!(property_name.as_str(), "toString" | "valueOf")
            && let Some(StaticEvalOutcome::Value(value)) = self
                .resolve_static_member_call_outcome_with_context(
                    object,
                    property_name,
                    self.current_function_name(),
                )
        {
            return self.materialize_static_expression(&value);
        }
        if matches!(callee, Expression::Identifier(_))
            && !self
                .resolve_user_function_from_expression(callee)
                .is_some_and(|user_function| {
                    user_function.is_async() || user_function.is_generator()
                })
            && let Some(value) = self.resolve_static_call_result_expression(callee, arguments)
        {
            return self.materialize_static_expression(&value);
        }
        if let Expression::Member { object, property } = callee
            && matches!(property.as_ref(), Expression::String(name) if name == "bind")
        {
            return Expression::Call {
                callee: Box::new(Expression::Member {
                    object: object.clone(),
                    property: property.clone(),
                }),
                arguments: arguments
                    .iter()
                    .map(|argument| match argument {
                        CallArgument::Expression(expression) => {
                            CallArgument::Expression(self.materialize_static_expression(expression))
                        }
                        CallArgument::Spread(expression) => {
                            CallArgument::Spread(self.materialize_static_expression(expression))
                        }
                    })
                    .collect(),
            };
        }
        materialize_recursive_expression(expression, true, true, &|nested| {
            Some(self.materialize_static_expression(nested))
        })
        .expect("function-side recursive materialization supports generic call rebuild")
    }

    pub(in crate::backend::direct_wasm) fn materialize_recursive_expression_default(
        &self,
        expression: &Expression,
    ) -> Expression {
        materialize_recursive_expression(expression, true, true, &|nested| {
            Some(self.materialize_static_expression(nested))
        })
        .unwrap_or_else(|| expression.clone())
    }

    /// Snapshots an effectful initializer/assignment value expression for
    /// static tracking. Compound assignments and updates mutate binding state
    /// while the surrounding expression is emitted, so the raw expression must
    /// not be stored for later re-resolution: re-evaluating it against the
    /// mutated state double-applies the operation (for example
    /// `var z = (x *= -1)` would re-read the updated `x`). This resolves
    /// identifier reads against the current (pre-emission) static state so the
    /// stored value matches evaluation order.
    pub(in crate::backend::direct_wasm) fn snapshot_effectful_expression_for_static_store(
        &self,
        expression: &Expression,
    ) -> Option<Expression> {
        if !Self::expression_contains_assignment_or_update(expression) {
            return None;
        }
        if self.expression_depends_on_active_loop_assignment(expression) {
            return None;
        }
        self.snapshot_expression_value_against_current_state(expression)
    }

    /// Same as [`Self::snapshot_effectful_expression_for_static_store`], but
    /// also snapshots pure right-hand sides that read the assignment target
    /// itself (the desugared form of `x op= y` is `x = x op y`); storing the
    /// raw self-referential expression would resolve against the already
    /// updated target.
    pub(in crate::backend::direct_wasm) fn snapshot_assignment_value_for_static_store(
        &self,
        name: &str,
        value: &Expression,
    ) -> Option<Expression> {
        let references_target = {
            let mut referenced_names = HashSet::new();
            collect_referenced_binding_names_from_expression(value, &mut referenced_names);
            referenced_names.contains(name)
        };
        if !references_target && !Self::expression_contains_assignment_or_update(value) {
            return None;
        }
        if self.expression_depends_on_active_loop_assignment(value) {
            return None;
        }
        self.snapshot_expression_value_against_current_state(value)
    }

    fn snapshot_expression_value_against_current_state(
        &self,
        expression: &Expression,
    ) -> Option<Expression> {
        match expression {
            Expression::Number(_)
            | Expression::BigInt(_)
            | Expression::String(_)
            | Expression::Bool(_)
            | Expression::Null
            | Expression::Undefined => Some(expression.clone()),
            Expression::Identifier(_) => {
                let materialized = self.materialize_static_expression(expression);
                if static_expression_matches(&materialized, expression)
                    || Self::expression_contains_assignment_or_update(&materialized)
                {
                    None
                } else {
                    Some(materialized)
                }
            }
            Expression::Update { name, op, prefix } => {
                let previous =
                    self.resolve_static_number_value(&Expression::Identifier(name.clone()))?;
                let next = match op {
                    UpdateOp::Increment => previous + 1.0,
                    UpdateOp::Decrement => previous - 1.0,
                };
                Some(Expression::Number(if *prefix { next } else { previous }))
            }
            Expression::Assign { name, value } => Some(Expression::Assign {
                name: name.clone(),
                value: Box::new(self.snapshot_expression_value_against_current_state(value)?),
            }),
            Expression::Unary { op, expression } => {
                if matches!(op, UnaryOp::Delete | UnaryOp::TypeOf) {
                    return None;
                }
                Some(Expression::Unary {
                    op: *op,
                    expression: Box::new(
                        self.snapshot_expression_value_against_current_state(expression)?,
                    ),
                })
            }
            Expression::Binary { op, left, right } => Some(Expression::Binary {
                op: *op,
                left: Box::new(self.snapshot_expression_value_against_current_state(left)?),
                right: Box::new(self.snapshot_expression_value_against_current_state(right)?),
            }),
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => Some(Expression::Conditional {
                condition: Box::new(
                    self.snapshot_expression_value_against_current_state(condition)?,
                ),
                then_expression: Box::new(
                    self.snapshot_expression_value_against_current_state(then_expression)?,
                ),
                else_expression: Box::new(
                    self.snapshot_expression_value_against_current_state(else_expression)?,
                ),
            }),
            _ => None,
        }
    }
}
