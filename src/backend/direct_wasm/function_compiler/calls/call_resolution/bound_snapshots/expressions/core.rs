use super::*;

impl<'a> FunctionCompiler<'a> {
    pub(in crate::backend::direct_wasm) fn bound_snapshot_identifier_is_value_builtin(
        &self,
        name: &str,
    ) -> bool {
        matches!(name, "undefined" | "NaN" | "Infinity")
            && self.is_unshadowed_builtin_identifier(name)
    }

    pub(super) fn evaluate_bound_snapshot_identifier(
        &self,
        name: &str,
        expression: &Expression,
        bindings: &mut HashMap<String, Expression>,
        current_function_name: Option<&str>,
    ) -> Option<Expression> {
        let resolved_name = self.resolve_bound_snapshot_binding_name(name, bindings);
        if let Some(value) = bindings.get(resolved_name).cloned() {
            if !static_expression_matches(&value, expression) {
                if let Expression::Identifier(value_name) = &value {
                    if self.bound_snapshot_identifier_is_value_builtin(value_name) {
                        return self.evaluate_bound_snapshot_expression(&value, bindings, None);
                    }
                    if let Some(resolved) = self
                        .resolve_global_value_expression(&value)
                        .filter(|resolved| !static_expression_matches(resolved, &value))
                        && let Some(primitive) = self
                            .evaluate_bound_snapshot_expression(&resolved, bindings, None)
                            .or_else(|| {
                                self.resolve_static_primitive_expression_with_context(
                                    &resolved, None,
                                )
                            })
                        && matches!(
                            primitive,
                            Expression::Number(_)
                                | Expression::BigInt(_)
                                | Expression::String(_)
                                | Expression::Bool(_)
                                | Expression::Null
                                | Expression::Undefined
                        )
                    {
                        return Some(primitive);
                    }
                    return Some(value);
                }
                if !matches!(
                    value,
                    Expression::Number(_)
                        | Expression::BigInt(_)
                        | Expression::String(_)
                        | Expression::Bool(_)
                        | Expression::Null
                        | Expression::Undefined
                ) {
                    if let Some(evaluated) =
                        self.evaluate_bound_snapshot_expression(&value, bindings, None)
                    {
                        return Some(evaluated);
                    }
                }
                return Some(value);
            }
        }
        if resolved_name == "undefined" && self.is_unshadowed_builtin_identifier(resolved_name) {
            return Some(Expression::Undefined);
        }
        if resolved_name == "NaN" && self.is_unshadowed_builtin_identifier(resolved_name) {
            return Some(Expression::Number(f64::NAN));
        }
        if resolved_name == "Infinity" && self.is_unshadowed_builtin_identifier(resolved_name) {
            return Some(Expression::Number(f64::INFINITY));
        }
        if let Some(function) = current_function_name
            .and_then(|function_name| self.resolve_registered_function_declaration(function_name))
        {
            let declared_bindings =
                collect_declared_bindings_from_statements_recursive(&function.body);
            if declared_bindings.contains(resolved_name) {
                return Some(Expression::Undefined);
            }
        }
        let identifier = Expression::Identifier(resolved_name.to_string());
        if let Some(array_binding) = self.resolve_array_binding_from_expression(&identifier) {
            return Some(Expression::Array(
                array_binding
                    .values
                    .into_iter()
                    .map(|value| ArrayElement::Expression(value.unwrap_or(Expression::Undefined)))
                    .collect(),
            ));
        }
        if self
            .resolve_function_binding_from_expression_with_context(&identifier, current_function_name)
            .is_some()
        {
            return Some(identifier);
        }
        if let Some(object_binding) = self.resolve_object_binding_from_expression(&identifier) {
            if !object_binding.property_descriptors.is_empty() {
                return Some(identifier);
            }
            return Some(object_binding_to_expression(&object_binding));
        }
        if let Some(resolved) = self
            .resolve_bound_alias_expression(&identifier)
            .filter(|resolved| !static_expression_matches(resolved, &identifier))
        {
            return Some(self.materialize_static_expression(&resolved));
        }
        if let Some(resolved) = self
            .resolve_global_value_expression(&identifier)
            .filter(|resolved| !static_expression_matches(resolved, &identifier))
        {
            return self
                .evaluate_bound_snapshot_expression(&resolved, bindings, None)
                .or_else(|| Some(self.materialize_static_expression(&resolved)));
        }
        Some(identifier)
    }

    pub(super) fn evaluate_bound_snapshot_this_expression(
        &self,
        expression: &Expression,
        bindings: &mut HashMap<String, Expression>,
        current_function_name: Option<&str>,
    ) -> Option<Expression> {
        match bindings.get("this").cloned() {
            Some(binding) => {
                if matches!(binding, Expression::This)
                    || static_expression_matches(&binding, expression)
                {
                    return None;
                }
                if matches!(binding, Expression::Identifier(_))
                    && self
                        .resolve_static_reference_identity_key(&binding)
                        .is_some()
                {
                    return Some(binding);
                }
                self.evaluate_bound_snapshot_expression(&binding, bindings, current_function_name)
                    .or_else(|| Some(self.materialize_static_expression(&binding)))
            }
            None => Some(Expression::Undefined),
        }
    }

    pub(super) fn evaluate_bound_snapshot_binary_expression(
        &self,
        op: BinaryOp,
        left: &Expression,
        right: &Expression,
        bindings: &mut HashMap<String, Expression>,
        current_function_name: Option<&str>,
    ) -> Option<Expression> {
        let left =
            self.evaluate_bound_snapshot_expression(left, bindings, current_function_name)?;
        let right =
            self.evaluate_bound_snapshot_expression(right, bindings, current_function_name)?;
        match op {
            BinaryOp::Add => {
                if matches!(left, Expression::String(_)) || matches!(right, Expression::String(_)) {
                    let left = bound_snapshot_primitive_to_string(&left)?;
                    let right = bound_snapshot_primitive_to_string(&right)?;
                    Some(Expression::String(format!("{left}{right}")))
                } else {
                    match (
                        bound_snapshot_primitive_to_number(&left),
                        bound_snapshot_primitive_to_number(&right),
                    ) {
                        (Some(lhs), Some(rhs)) => Some(Expression::Number(lhs + rhs)),
                        _ => None,
                    }
                }
            }
            BinaryOp::GreaterThanOrEqual => match (&left, &right) {
                (Expression::Number(lhs), Expression::Number(rhs)) => {
                    Some(Expression::Bool(lhs >= rhs))
                }
                _ => None,
            },
            BinaryOp::LogicalAnd => {
                if self.resolve_static_boolean_expression(&left)? {
                    Some(right)
                } else {
                    Some(left)
                }
            }
            BinaryOp::LogicalOr => {
                if self.resolve_static_boolean_expression(&left)? {
                    Some(left)
                } else {
                    Some(right)
                }
            }
            BinaryOp::NullishCoalescing => {
                if matches!(left, Expression::Null | Expression::Undefined) {
                    Some(right)
                } else {
                    Some(left)
                }
            }
            BinaryOp::Equal
            | BinaryOp::LooseEqual
            | BinaryOp::NotEqual
            | BinaryOp::LooseNotEqual => {
                let equal = match (&left, &right) {
                    (Expression::Bool(lhs), Expression::Bool(rhs)) => lhs == rhs,
                    (Expression::Number(lhs), Expression::Number(rhs)) => lhs == rhs,
                    (Expression::String(lhs), Expression::String(rhs)) => lhs == rhs,
                    (Expression::Null, Expression::Null)
                    | (Expression::Undefined, Expression::Undefined) => true,
                    (Expression::Null, Expression::Undefined)
                    | (Expression::Undefined, Expression::Null)
                        if matches!(op, BinaryOp::LooseEqual | BinaryOp::LooseNotEqual) =>
                    {
                        true
                    }
                    _ => return None,
                };
                Some(Expression::Bool(match op {
                    BinaryOp::Equal | BinaryOp::LooseEqual => equal,
                    BinaryOp::NotEqual | BinaryOp::LooseNotEqual => !equal,
                    _ => unreachable!("filtered above"),
                }))
            }
            _ => None,
        }
    }

    pub(super) fn evaluate_bound_snapshot_unary_expression(
        &self,
        op: UnaryOp,
        expression: &Expression,
        bindings: &mut HashMap<String, Expression>,
        current_function_name: Option<&str>,
    ) -> Option<Expression> {
        match op {
            UnaryOp::Void => {
                self.evaluate_bound_snapshot_expression(
                    expression,
                    bindings,
                    current_function_name,
                )?;
                Some(Expression::Undefined)
            }
            UnaryOp::Plus => {
                let value = self.evaluate_bound_snapshot_expression(
                    expression,
                    bindings,
                    current_function_name,
                )?;
                bound_snapshot_primitive_to_number(&value).map(Expression::Number)
            }
            UnaryOp::Negate => {
                let value = self.evaluate_bound_snapshot_expression(
                    expression,
                    bindings,
                    current_function_name,
                )?;
                bound_snapshot_primitive_to_number(&value).map(|number| Expression::Number(-number))
            }
            UnaryOp::Not => {
                let value = self.evaluate_bound_snapshot_expression(
                    expression,
                    bindings,
                    current_function_name,
                )?;
                self.resolve_static_boolean_expression(&value)
                    .map(|truthy| Expression::Bool(!truthy))
            }
            UnaryOp::BitwiseNot | UnaryOp::TypeOf | UnaryOp::Delete => None,
        }
    }

    pub(super) fn evaluate_bound_snapshot_member_expression(
        &self,
        object: &Expression,
        property: &Expression,
        bindings: &mut HashMap<String, Expression>,
        current_function_name: Option<&str>,
    ) -> Option<Expression> {
        let object =
            self.evaluate_bound_snapshot_expression(object, bindings, current_function_name)?;
        let property = self.resolve_property_key_expression(property).or_else(|| {
            self.evaluate_bound_snapshot_expression(property, bindings, current_function_name)
        })?;
        if matches!(object, Expression::This)
            && let Expression::String(property_name) = &property
            && let Some(descriptor) =
                self.resolve_top_level_global_property_descriptor_binding(property_name)
            && let Some(value) = descriptor.value
        {
            return Some(value);
        }
        match (object, property) {
            (Expression::Array(elements), property) => {
                if matches!(property, Expression::String(ref name) if name == "length") {
                    return Some(Expression::Number(elements.len() as f64));
                }
                let index = argument_index_from_expression(&property)? as usize;
                match elements.get(index) {
                    Some(ArrayElement::Expression(value)) => Some(value.clone()),
                    Some(ArrayElement::Spread(_)) => None,
                    None => Some(Expression::Undefined),
                }
            }
            (Expression::Object(entries), property) => self
                .resolve_bound_snapshot_object_member_value(
                    &entries,
                    &property,
                    bindings,
                    current_function_name,
                ),
            _ => None,
        }
    }

    pub(super) fn evaluate_bound_snapshot_update_expression(
        &self,
        name: &str,
        op: UpdateOp,
        prefix: bool,
        bindings: &mut HashMap<String, Expression>,
    ) -> Option<Expression> {
        let resolved_name = self
            .resolve_bound_snapshot_binding_name(name, bindings)
            .to_string();
        let current = bindings.get(&resolved_name)?.clone();
        let Expression::Number(current_number) = current else {
            return None;
        };
        let next_number = match op {
            UpdateOp::Increment => current_number + 1.0,
            UpdateOp::Decrement => current_number - 1.0,
        };
        bindings.insert(resolved_name, Expression::Number(next_number));
        Some(if prefix {
            Expression::Number(next_number)
        } else {
            Expression::Number(current_number)
        })
    }
}

fn bound_snapshot_primitive_to_string(expression: &Expression) -> Option<String> {
    match expression {
        Expression::String(value) => Some(value.clone()),
        Expression::Number(value) => Some(bound_snapshot_number_to_string(*value)),
        Expression::Bool(value) => Some(value.to_string()),
        Expression::Null => Some("null".to_string()),
        Expression::Undefined => Some("undefined".to_string()),
        Expression::BigInt(value) => Some(value.trim_end_matches('n').to_string()),
        _ => None,
    }
}

fn bound_snapshot_primitive_to_number(expression: &Expression) -> Option<f64> {
    match expression {
        Expression::Number(value) => Some(*value),
        Expression::Bool(value) => Some(if *value { 1.0 } else { 0.0 }),
        Expression::Null => Some(0.0),
        Expression::Undefined => Some(f64::NAN),
        _ => None,
    }
}

fn bound_snapshot_number_to_string(value: f64) -> String {
    if value.is_nan() {
        "NaN".to_string()
    } else if value == f64::INFINITY {
        "Infinity".to_string()
    } else if value == f64::NEG_INFINITY {
        "-Infinity".to_string()
    } else if value == 0.0 {
        "0".to_string()
    } else if value.is_finite() && value.fract() == 0.0 {
        (value as i64).to_string()
    } else {
        value.to_string()
    }
}
