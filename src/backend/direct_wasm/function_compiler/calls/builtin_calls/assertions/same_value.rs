use super::*;

impl<'a> FunctionCompiler<'a> {
    fn emit_same_value_operand(&mut self, expression: &Expression) -> DirectResult<()> {
        self.emit_numeric_expression(expression)
    }

    fn same_value_static_number_to_string(value: f64) -> String {
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

    fn same_value_static_primitive_to_string(expression: &Expression) -> Option<String> {
        match expression {
            Expression::String(value) => Some(value.clone()),
            Expression::Number(value) => Some(Self::same_value_static_number_to_string(*value)),
            Expression::Bool(value) => Some(value.to_string()),
            Expression::Null => Some("null".to_string()),
            Expression::Undefined => Some("undefined".to_string()),
            Expression::BigInt(value) => Some(value.trim_end_matches('n').to_string()),
            _ => None,
        }
    }

    fn same_value_static_primitive_to_number(expression: &Expression) -> Option<f64> {
        match expression {
            Expression::Number(value) => Some(*value),
            Expression::Bool(value) => Some(if *value { 1.0 } else { 0.0 }),
            Expression::Null => Some(0.0),
            Expression::Undefined => Some(f64::NAN),
            _ => None,
        }
    }

    fn same_value_static_binary_expression_value(
        op: BinaryOp,
        left: &Expression,
        right: &Expression,
    ) -> Option<Expression> {
        match op {
            BinaryOp::Add => {
                if matches!(left, Expression::String(_)) || matches!(right, Expression::String(_)) {
                    let left = Self::same_value_static_primitive_to_string(left)?;
                    let right = Self::same_value_static_primitive_to_string(right)?;
                    Some(Expression::String(format!("{left}{right}")))
                } else {
                    Some(Expression::Number(
                        Self::same_value_static_primitive_to_number(left)?
                            + Self::same_value_static_primitive_to_number(right)?,
                    ))
                }
            }
            _ => None,
        }
    }

    fn same_value_operand_contains_member_access(expression: &Expression) -> bool {
        match expression {
            Expression::Member { .. } | Expression::SuperMember { .. } => true,
            Expression::Assign { value, .. }
            | Expression::Await(value)
            | Expression::EnumerateKeys(value)
            | Expression::GetIterator(value)
            | Expression::IteratorClose(value)
            | Expression::Unary {
                expression: value, ..
            } => Self::same_value_operand_contains_member_access(value),
            Expression::AssignMember {
                object,
                property,
                value,
            } => {
                Self::same_value_operand_contains_member_access(object)
                    || Self::same_value_operand_contains_member_access(property)
                    || Self::same_value_operand_contains_member_access(value)
            }
            Expression::AssignSuperMember { property, value } => {
                Self::same_value_operand_contains_member_access(property)
                    || Self::same_value_operand_contains_member_access(value)
            }
            Expression::Binary { left, right, .. } => {
                Self::same_value_operand_contains_member_access(left)
                    || Self::same_value_operand_contains_member_access(right)
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                Self::same_value_operand_contains_member_access(condition)
                    || Self::same_value_operand_contains_member_access(then_expression)
                    || Self::same_value_operand_contains_member_access(else_expression)
            }
            Expression::Sequence(expressions) => expressions
                .iter()
                .any(Self::same_value_operand_contains_member_access),
            Expression::Call { callee, arguments }
            | Expression::SuperCall { callee, arguments }
            | Expression::New { callee, arguments } => {
                Self::same_value_operand_contains_member_access(callee)
                    || arguments.iter().any(|argument| match argument {
                        CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                            Self::same_value_operand_contains_member_access(expression)
                        }
                    })
            }
            Expression::Array(elements) => elements.iter().any(|element| match element {
                ArrayElement::Expression(expression) | ArrayElement::Spread(expression) => {
                    Self::same_value_operand_contains_member_access(expression)
                }
            }),
            Expression::Object(entries) => entries.iter().any(|entry| match entry {
                ObjectEntry::Data { key, value }
                | ObjectEntry::Getter { key, getter: value }
                | ObjectEntry::Setter { key, setter: value } => {
                    Self::same_value_operand_contains_member_access(key)
                        || Self::same_value_operand_contains_member_access(value)
                }
                ObjectEntry::Spread(expression) => {
                    Self::same_value_operand_contains_member_access(expression)
                }
            }),
            Expression::Identifier(_)
            | Expression::Number(_)
            | Expression::BigInt(_)
            | Expression::String(_)
            | Expression::Bool(_)
            | Expression::Null
            | Expression::Undefined
            | Expression::This
            | Expression::NewTarget
            | Expression::Sent
            | Expression::Update { .. } => false,
        }
    }

    fn same_value_operand_static_evaluation_safe(&self, expression: &Expression) -> bool {
        if matches!(
            expression,
            Expression::Binary {
                op: BinaryOp::LessThan
                    | BinaryOp::LessThanOrEqual
                    | BinaryOp::GreaterThan
                    | BinaryOp::GreaterThanOrEqual,
                ..
            }
        ) {
            return false;
        }
        if inline_summary_side_effect_free_expression(expression) {
            return true;
        }
        match expression {
            Expression::Unary { expression, .. }
            | Expression::Await(expression)
            | Expression::EnumerateKeys(expression)
            | Expression::GetIterator(expression)
            | Expression::IteratorClose(expression) => {
                self.same_value_operand_static_evaluation_safe(expression)
            }
            Expression::Binary { left, right, .. } => {
                self.same_value_operand_static_evaluation_safe(left)
                    && self.same_value_operand_static_evaluation_safe(right)
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                self.same_value_operand_static_evaluation_safe(condition)
                    && self.same_value_operand_static_evaluation_safe(then_expression)
                    && self.same_value_operand_static_evaluation_safe(else_expression)
            }
            Expression::Sequence(expressions) => expressions
                .iter()
                .all(|expression| self.same_value_operand_static_evaluation_safe(expression)),
            Expression::Call { callee, arguments } => {
                if let Expression::Member { object, property } = callee.as_ref()
                    && matches!(object.as_ref(), Expression::Identifier(name) if name == "Object")
                    && matches!(property.as_ref(), Expression::String(name) if name == "getPrototypeOf")
                    && let [CallArgument::Expression(target)] = arguments.as_slice()
                {
                    return self
                        .resolve_static_object_prototype_expression(target)
                        .is_some()
                        && self.same_value_operand_static_evaluation_safe(target);
                }
                matches!(
                    callee.as_ref(),
                    Expression::Identifier(name)
                        if matches!(name.as_str(), "Object" | "Boolean" | "Number" | "String")
                            && self.is_unshadowed_builtin_identifier(name)
                ) && arguments.iter().all(|argument| match argument {
                    CallArgument::Expression(expression) => {
                        self.same_value_operand_static_evaluation_safe(expression)
                    }
                    CallArgument::Spread(_) => false,
                }) || (inline_summary_side_effect_free_expression(callee)
                    && arguments.iter().all(|argument| match argument {
                        CallArgument::Expression(expression) => {
                            self.same_value_operand_static_evaluation_safe(expression)
                        }
                        CallArgument::Spread(_) => false,
                    })
                    && self
                        .resolve_function_binding_from_expression_with_context(
                            callee,
                            self.current_function_name(),
                        )
                        .and_then(|binding| match binding {
                            LocalFunctionBinding::User(function_name) => {
                                self.user_function(&function_name)
                            }
                            LocalFunctionBinding::Builtin(_) => None,
                        })
                        .is_some_and(|user_function| {
                            self.collect_user_function_assigned_nonlocal_bindings(user_function)
                                .is_empty()
                                && self
                                    .collect_user_function_call_effect_nonlocal_bindings(
                                        user_function,
                                    )
                                    .is_empty()
                        })
                    && self
                        .resolve_static_call_result_expression_with_context(
                            callee,
                            arguments,
                            self.current_function_name(),
                        )
                        .is_some())
            }
            Expression::New { callee, arguments } => {
                inline_summary_side_effect_free_expression(callee)
                    && arguments.iter().all(|argument| match argument {
                        CallArgument::Expression(expression) => {
                            self.same_value_operand_static_evaluation_safe(expression)
                        }
                        CallArgument::Spread(_) => false,
                    })
                    && self
                        .resolve_function_binding_from_expression_with_context(
                            callee,
                            self.current_function_name(),
                        )
                        .and_then(|binding| match binding {
                            LocalFunctionBinding::User(function_name) => {
                                self.user_function(&function_name)
                            }
                            LocalFunctionBinding::Builtin(_) => None,
                        })
                        .is_some_and(|user_function| {
                            user_function.is_constructible()
                                && self
                                    .collect_user_function_assigned_nonlocal_bindings(user_function)
                                    .is_empty()
                                && self
                                    .collect_user_function_call_effect_nonlocal_bindings(
                                        user_function,
                                    )
                                    .is_empty()
                        })
            }
            Expression::Member { object, property } => {
                if matches!(object.as_ref(), Expression::Call { callee, arguments }
                if matches!(
                    callee.as_ref(),
                    Expression::Member { object, property }
                        if matches!(object.as_ref(), Expression::Identifier(name) if name == "Object")
                            && matches!(property.as_ref(), Expression::String(name) if name == "getPrototypeOf")
                ) && matches!(
                    arguments.as_slice(),
                    [CallArgument::Expression(target)]
                        if self.same_value_operand_static_evaluation_safe(target)
                )) && self.same_value_operand_static_evaluation_safe(property)
                {
                    let materialized = self.materialize_static_expression(expression);
                    if !static_expression_matches(&materialized, expression)
                        && self.same_value_assertion_is_primitive_literal_operand(&materialized)
                    {
                        return true;
                    }
                }
                matches!(object.as_ref(), Expression::Call { callee, arguments }
                if matches!(
                    callee.as_ref(),
                    Expression::Member { object, property }
                        if matches!(object.as_ref(), Expression::Identifier(name) if name == "Object")
                            && matches!(property.as_ref(), Expression::String(name) if name == "getPrototypeOf")
                ) && matches!(
                    arguments.as_slice(),
                    [CallArgument::Expression(target)]
                        if self.resolve_static_object_prototype_expression(target).is_some()
                            && self.same_value_operand_static_evaluation_safe(target)
                )) && self.same_value_operand_static_evaluation_safe(property)
            }
            _ => false,
        }
    }

    fn same_value_assertion_primitive_result(
        &self,
        actual: &Expression,
        expected: &Expression,
    ) -> Option<bool> {
        match (actual, expected) {
            (Expression::Number(actual), Expression::Number(expected)) => {
                if actual.is_nan() && expected.is_nan() {
                    Some(true)
                } else if *actual == 0.0 && *expected == 0.0 {
                    Some(actual.is_sign_negative() == expected.is_sign_negative())
                } else {
                    Some(actual == expected)
                }
            }
            (Expression::BigInt(actual), Expression::BigInt(expected)) => {
                Some(parse_static_bigint_literal(actual)? == parse_static_bigint_literal(expected)?)
            }
            (Expression::String(actual), Expression::String(expected)) => Some(actual == expected),
            (Expression::Bool(actual), Expression::Bool(expected)) => Some(actual == expected),
            (Expression::Null, Expression::Null)
            | (Expression::Undefined, Expression::Undefined) => Some(true),
            _ => None,
        }
    }

    fn same_value_assertion_fast_object_property(
        &self,
        object: &Expression,
        property: &Expression,
        depth: usize,
    ) -> Option<Expression> {
        if depth == 0 {
            return None;
        }
        if let Some(value) = self.resolve_primitive_prototype_property_value(object, property) {
            return Some(value);
        }
        match object {
            Expression::Object(entries) => entries.iter().rev().find_map(|entry| {
                let ObjectEntry::Data { key, value } = entry else {
                    return None;
                };
                (static_property_name_from_expression(key).as_deref()
                    == static_property_name_from_expression(property).as_deref())
                .then(|| value.clone())
            }),
            Expression::Identifier(name) => {
                if let Some(value) = self
                    .state
                    .speculation
                    .static_semantics
                    .local_value_binding(name)
                    .or_else(|| self.global_value_binding(name))
                    && !static_expression_matches(value, object)
                    && let Some(resolved) =
                        self.same_value_assertion_fast_object_property(value, property, depth - 1)
                {
                    return Some(resolved);
                }
                self.state
                    .speculation
                    .static_semantics
                    .local_object_binding(name)
                    .or_else(|| self.backend.global_object_binding(name))
                    .and_then(|binding| object_binding_lookup_value(binding, property).cloned())
            }
            _ => None,
        }
    }

    fn same_value_assertion_fast_primitive_value(
        &self,
        expression: &Expression,
        depth: usize,
    ) -> Option<Expression> {
        if depth == 0 {
            return None;
        }
        match expression {
            Expression::Number(_)
            | Expression::BigInt(_)
            | Expression::String(_)
            | Expression::Bool(_)
            | Expression::Null
            | Expression::Undefined => Some(expression.clone()),
            Expression::Identifier(name)
                if matches!(name.as_str(), "undefined" | "NaN" | "Infinity")
                    && self.is_unshadowed_builtin_identifier(name) =>
            {
                match name.as_str() {
                    "undefined" => Some(Expression::Undefined),
                    "NaN" => Some(Expression::Number(f64::NAN)),
                    "Infinity" => Some(Expression::Number(f64::INFINITY)),
                    _ => None,
                }
            }
            Expression::Identifier(name) => self
                .state
                .speculation
                .static_semantics
                .local_value_binding(name)
                .or_else(|| self.global_value_binding(name))
                .filter(|value| !static_expression_matches(value, expression))
                .and_then(|value| self.same_value_assertion_fast_primitive_value(value, depth - 1)),
            Expression::Member { object, property } => {
                let property_name = static_property_name_from_expression(property)?;
                let property = Expression::String(property_name);
                let value =
                    self.same_value_assertion_fast_object_property(object, &property, depth - 1)?;
                self.same_value_assertion_fast_primitive_value(&value, depth - 1)
            }
            Expression::Binary { op, left, right } => {
                let left_value = self.same_value_assertion_fast_primitive_value(left, depth - 1)?;
                let right_value =
                    self.same_value_assertion_fast_primitive_value(right, depth - 1)?;
                Self::same_value_static_binary_expression_value(*op, &left_value, &right_value)
            }
            Expression::Call { callee, arguments } => self
                .same_value_assertion_fast_static_call_primitive_value(
                    callee,
                    arguments,
                    depth - 1,
                ),
            _ => None,
        }
    }

    fn same_value_assertion_fast_static_call_primitive_value(
        &self,
        callee: &Expression,
        arguments: &[CallArgument],
        depth: usize,
    ) -> Option<Expression> {
        if depth == 0
            || !arguments.iter().all(|argument| match argument {
                CallArgument::Expression(expression) => {
                    self.same_value_operand_static_evaluation_safe(expression)
                }
                CallArgument::Spread(_) => false,
            })
        {
            return None;
        }

        if matches!(callee, Expression::Identifier(name) if name == "eval")
            && let Some(argument_source) =
                self.static_eval_argument_source_from_arguments(arguments)
            && let Ok(program) = crate::frontend::parse_script_goal(&argument_source)
            && program.functions.is_empty()
            && let [Statement::Expression(expression)] = program.statements.as_slice()
        {
            return self
                .same_value_assertion_fast_primitive_value(expression, depth - 1)
                .or_else(|| self.same_value_assertion_direct_static_value(expression, depth - 1));
        }

        if let Some((value, result_function_name)) = self
            .resolve_static_member_builtin_call_result_with_context(
                callee,
                arguments,
                self.current_function_name(),
            )
        {
            let result_context = result_function_name
                .as_deref()
                .or_else(|| self.current_function_name());
            return self
                .resolve_static_primitive_expression_with_context(&value, result_context)
                .or_else(|| {
                    let materialized = self.materialize_static_expression(&value);
                    self.resolve_static_primitive_expression_with_context(
                        &materialized,
                        result_context,
                    )
                    .or_else(|| {
                        self.same_value_assertion_fast_primitive_value(&materialized, depth - 1)
                    })
                });
        }

        let LocalFunctionBinding::User(function_name) = self
            .resolve_function_binding_from_expression_with_context(
                callee,
                self.current_function_name(),
            )?
        else {
            return None;
        };
        let user_function = self.user_function(&function_name)?;
        if !self
            .collect_user_function_assigned_nonlocal_bindings(user_function)
            .is_empty()
            || !self
                .collect_user_function_call_effect_nonlocal_bindings(user_function)
                .is_empty()
        {
            return None;
        }

        let (value, result_function_name) = self
            .resolve_static_call_result_expression_with_context(
                callee,
                arguments,
                self.current_function_name(),
            )?;
        let result_context = result_function_name
            .as_deref()
            .or(Some(function_name.as_str()))
            .or_else(|| self.current_function_name());
        self.resolve_static_primitive_expression_with_context(&value, result_context)
            .or_else(|| {
                let materialized = self.materialize_static_expression(&value);
                self.resolve_static_primitive_expression_with_context(&materialized, result_context)
                    .or_else(|| {
                        self.same_value_assertion_fast_primitive_value(&materialized, depth - 1)
                    })
            })
    }

    fn same_value_assertion_fast_primitive_result(
        &self,
        actual: &Expression,
        expected: &Expression,
    ) -> Option<bool> {
        let actual_value = self.same_value_assertion_fast_primitive_value(actual, 6)?;
        let expected_value = self.same_value_assertion_fast_primitive_value(expected, 6)?;
        self.same_value_assertion_primitive_result(&actual_value, &expected_value)
    }

    fn same_value_assertion_direct_array_binding(
        &self,
        expression: &Expression,
        depth: usize,
    ) -> Option<ArrayValueBinding> {
        if depth == 0 {
            return None;
        }
        let array_binding_from_value = |value: Expression| {
            let Expression::Array(elements) = value else {
                return None;
            };
            let mut values = Vec::new();
            for element in elements {
                let ArrayElement::Expression(value) = element else {
                    return None;
                };
                values.push(Some(value));
            }
            Some(ArrayValueBinding { values })
        };
        match expression {
            Expression::Identifier(name) => {
                let existing = self
                    .state
                    .speculation
                    .static_semantics
                    .local_array_binding(name)
                    .cloned()
                    .or_else(|| self.backend.global_array_binding(name).cloned());
                let derived = self
                    .state
                    .speculation
                    .static_semantics
                    .local_value_binding(name)
                    .or_else(|| self.global_value_binding(name))
                    .and_then(|value| {
                        self.same_value_assertion_direct_call_array_binding(value, depth - 1)
                            .or_else(|| {
                                self.same_value_assertion_direct_static_value(value, depth - 1)
                                    .or_else(|| Some(value.clone()))
                                    .and_then(|value| array_binding_from_value(value))
                            })
                    });
                if existing
                    .as_ref()
                    .is_none_or(|binding| binding.values.is_empty())
                    && derived.is_some()
                {
                    derived
                } else {
                    existing.or(derived)
                }
            }
            Expression::Call { .. } => {
                self.same_value_assertion_direct_call_array_binding(expression, depth - 1)
            }
            Expression::Array(elements) => {
                let mut values = Vec::new();
                for element in elements {
                    let ArrayElement::Expression(value) = element else {
                        return None;
                    };
                    values.push(Some(value.clone()));
                }
                Some(ArrayValueBinding { values })
            }
            _ => match self.same_value_assertion_direct_static_value(expression, depth - 1)? {
                Expression::Array(elements) => {
                    array_binding_from_value(Expression::Array(elements))
                }
                Expression::Identifier(name) => self
                    .state
                    .speculation
                    .static_semantics
                    .local_array_binding(&name)
                    .cloned()
                    .or_else(|| self.backend.global_array_binding(&name).cloned()),
                _ => None,
            },
        }
    }

    fn same_value_assertion_binding_seed_value(
        &self,
        name: &str,
        bindings: &std::collections::HashMap<String, Expression>,
    ) -> Option<Expression> {
        bindings
            .get(name)
            .cloned()
            .or_else(|| {
                self.state
                    .speculation
                    .static_semantics
                    .local_value_binding(name)
                    .cloned()
            })
            .or_else(|| self.global_value_binding(name).cloned())
            .or_else(|| {
                self.prepared_program
                    .required_global_static_semantics()
                    .values
                    .value_binding(name)
                    .cloned()
            })
    }

    fn same_value_assertion_evaluate_simple_static_expression(
        &self,
        expression: &Expression,
        bindings: &mut std::collections::HashMap<String, Expression>,
        depth: usize,
    ) -> Option<Expression> {
        if depth == 0 {
            return None;
        }
        match expression {
            Expression::Number(_)
            | Expression::BigInt(_)
            | Expression::String(_)
            | Expression::Bool(_)
            | Expression::Null
            | Expression::Undefined => Some(expression.clone()),
            Expression::Identifier(name) => {
                self.same_value_assertion_binding_seed_value(name, bindings)
            }
            Expression::Update { name, op, prefix } => {
                let current = self.same_value_assertion_binding_seed_value(name, bindings)?;
                let current_number = match current {
                    Expression::Number(value) => value,
                    Expression::Bool(true) => 1.0,
                    Expression::Bool(false) | Expression::Null => 0.0,
                    Expression::Undefined => f64::NAN,
                    _ => return None,
                };
                let next_number = match op {
                    UpdateOp::Increment => current_number + 1.0,
                    UpdateOp::Decrement => current_number - 1.0,
                };
                let next = Expression::Number(next_number);
                bindings.insert(name.clone(), next.clone());
                if *prefix {
                    Some(next)
                } else {
                    Some(Expression::Number(current_number))
                }
            }
            Expression::Sequence(expressions) => {
                let mut value = Expression::Undefined;
                for expression in expressions {
                    value = self
                        .same_value_assertion_evaluate_simple_static_expression(
                            expression,
                            bindings,
                            depth - 1,
                        )
                        .or_else(|| {
                            self.same_value_assertion_direct_static_value(expression, depth - 1)
                        })?;
                }
                Some(value)
            }
            Expression::Unary {
                op: UnaryOp::Plus,
                expression,
            } => {
                let value = self.same_value_assertion_evaluate_simple_static_expression(
                    expression,
                    bindings,
                    depth - 1,
                )?;
                static_numeric_property_name_value(&value).map(Expression::Number)
            }
            Expression::Unary {
                op: UnaryOp::Negate,
                expression,
            } => {
                let value = self.same_value_assertion_evaluate_simple_static_expression(
                    expression,
                    bindings,
                    depth - 1,
                )?;
                static_numeric_property_name_value(&value).map(|number| Expression::Number(-number))
            }
            Expression::Binary { op, left, right } => {
                let left_value = self
                    .same_value_assertion_evaluate_simple_static_expression(
                        left,
                        bindings,
                        depth - 1,
                    )
                    .or_else(|| self.same_value_assertion_direct_static_value(left, depth - 1))?;
                let right_value = self
                    .same_value_assertion_evaluate_simple_static_expression(
                        right,
                        bindings,
                        depth - 1,
                    )
                    .or_else(|| self.same_value_assertion_direct_static_value(right, depth - 1))?;
                Self::same_value_static_binary_expression_value(*op, &left_value, &right_value)
            }
            _ => self.same_value_assertion_direct_static_value(expression, depth - 1),
        }
    }

    fn same_value_assertion_evaluate_object_literal_key(
        &self,
        key: &Expression,
        bindings: &mut std::collections::HashMap<String, Expression>,
        depth: usize,
    ) -> Option<Expression> {
        if depth == 0 {
            return None;
        }
        let evaluated =
            self.same_value_assertion_evaluate_simple_static_expression(key, bindings, depth - 1);
        if let Some(evaluated) = evaluated.as_ref() {
            if let Some(property_name) = static_property_name_from_expression(evaluated) {
                return Some(Expression::String(property_name));
            }
            if let Some(resolved) = self.resolve_property_key_expression(evaluated) {
                return Some(resolved);
            }
        }
        self.resolve_property_key_expression(key)
            .or(evaluated)
            .and_then(|resolved| {
                static_property_name_from_expression(&resolved)
                    .map(Expression::String)
                    .or(Some(resolved))
            })
    }

    fn same_value_assertion_direct_object_literal_binding(
        &self,
        entries: &[ObjectEntry],
        depth: usize,
    ) -> Option<ObjectValueBinding> {
        if depth == 0 {
            return None;
        }
        let mut object_binding = empty_object_value_binding();
        let mut bindings = std::collections::HashMap::new();
        for entry in entries {
            let ObjectEntry::Data { key, value } = entry else {
                return None;
            };
            if object_entry_is_literal_proto_setter(entry) {
                continue;
            }
            let key =
                self.same_value_assertion_evaluate_object_literal_key(key, &mut bindings, depth)?;
            let value = self
                .same_value_assertion_evaluate_simple_static_expression(
                    value,
                    &mut bindings,
                    depth - 1,
                )
                .or_else(|| self.same_value_assertion_direct_static_value(value, depth - 1))?;
            object_binding_set_property(&mut object_binding, key, value);
        }
        Some(object_binding)
    }

    fn same_value_assertion_direct_object_binding(
        &self,
        expression: &Expression,
        depth: usize,
    ) -> Option<ObjectValueBinding> {
        if depth == 0 {
            return None;
        }
        match expression {
            Expression::Object(entries) => {
                self.same_value_assertion_direct_object_literal_binding(entries, depth - 1)
            }
            Expression::Identifier(name) => {
                let value_binding = self
                    .state
                    .speculation
                    .static_semantics
                    .local_value_binding(name)
                    .or_else(|| self.global_value_binding(name));
                let existing = self
                    .state
                    .speculation
                    .static_semantics
                    .local_object_binding(name)
                    .cloned()
                    .or_else(|| self.backend.global_object_binding(name).cloned());
                let derived = value_binding.and_then(|value| {
                    self.same_value_assertion_direct_object_binding(value, depth - 1)
                });
                if existing
                    .as_ref()
                    .is_none_or(|binding| ordered_object_property_names(binding).is_empty())
                    && derived.is_some()
                {
                    derived
                } else {
                    existing.or(derived)
                }
            }
            _ => match self.same_value_assertion_direct_static_value(expression, depth - 1)? {
                Expression::Object(entries) => {
                    self.same_value_assertion_direct_object_literal_binding(&entries, depth - 1)
                }
                Expression::Identifier(name) => self.same_value_assertion_direct_object_binding(
                    &Expression::Identifier(name),
                    depth - 1,
                ),
                _ => None,
            },
        }
    }

    fn same_value_assertion_direct_call_array_binding(
        &self,
        expression: &Expression,
        depth: usize,
    ) -> Option<ArrayValueBinding> {
        if depth == 0 {
            return None;
        }
        let Expression::Call { callee, arguments } = expression else {
            return None;
        };
        let Expression::Member { object, property } = callee.as_ref() else {
            return None;
        };
        let Some(target) = arguments.first().and_then(|argument| match argument {
            CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                Some(expression)
            }
        }) else {
            return None;
        };
        let object_binding = self.same_value_assertion_direct_object_binding(target, depth - 1)?;
        match (object.as_ref(), property.as_ref()) {
            (Expression::Identifier(name), Expression::String(property))
                if name == "Object" && property == "getOwnPropertyNames" =>
            {
                Some(own_property_names_from_object_binding(&object_binding))
            }
            (Expression::Identifier(name), Expression::String(property))
                if name == "Object" && property == "keys" =>
            {
                Some(enumerated_keys_from_object_binding(&object_binding))
            }
            _ => None,
        }
    }

    fn same_value_assertion_direct_object_property(
        &self,
        object: &Expression,
        property: &Expression,
        depth: usize,
    ) -> Option<Expression> {
        if depth == 0 {
            return None;
        }
        if let Some(value) = self.resolve_primitive_prototype_property_value(object, property) {
            return Some(value);
        }
        if let Some(value) = self
            .same_value_assertion_direct_object_binding(object, depth - 1)
            .and_then(|binding| object_binding_lookup_value(&binding, property).cloned())
        {
            return Some(value);
        }
        let lookup_direct_object = |entries: &[ObjectEntry]| {
            let property_name = static_property_name_from_expression(property)?;
            entries.iter().rev().find_map(|entry| {
                let ObjectEntry::Data { key, value } = entry else {
                    return None;
                };
                (static_property_name_from_expression(key).as_deref()
                    == Some(property_name.as_str()))
                .then(|| value.clone())
            })
        };
        match object {
            Expression::Object(entries) => lookup_direct_object(entries),
            Expression::Identifier(name) => self
                .state
                .speculation
                .static_semantics
                .local_object_binding(name)
                .or_else(|| self.backend.global_object_binding(name))
                .and_then(|binding| object_binding_lookup_value(binding, property).cloned())
                .or_else(|| {
                    self.state
                        .speculation
                        .static_semantics
                        .local_value_binding(name)
                        .or_else(|| self.global_value_binding(name))
                        .and_then(|value| {
                            self.same_value_assertion_direct_static_value(value, depth - 1)
                                .or_else(|| Some(value.clone()))
                        })
                        .and_then(|value| match value {
                            Expression::Object(entries) => lookup_direct_object(&entries),
                            _ => None,
                        })
                }),
            _ => match self.same_value_assertion_direct_static_value(object, depth - 1)? {
                Expression::Object(entries) => lookup_direct_object(&entries),
                Expression::Identifier(name) => self
                    .state
                    .speculation
                    .static_semantics
                    .local_object_binding(&name)
                    .or_else(|| self.backend.global_object_binding(&name))
                    .and_then(|binding| object_binding_lookup_value(binding, property).cloned()),
                _ => None,
            },
        }
    }

    fn same_value_assertion_direct_static_value(
        &self,
        expression: &Expression,
        depth: usize,
    ) -> Option<Expression> {
        if depth == 0 {
            return None;
        }
        match expression {
            Expression::Number(_)
            | Expression::BigInt(_)
            | Expression::String(_)
            | Expression::Bool(_)
            | Expression::Null
            | Expression::Undefined
            | Expression::Array(_)
            | Expression::Object(_) => Some(expression.clone()),
            Expression::Identifier(name)
                if matches!(name.as_str(), "undefined" | "NaN" | "Infinity")
                    && self.is_unshadowed_builtin_identifier(name) =>
            {
                match name.as_str() {
                    "undefined" => Some(Expression::Undefined),
                    "NaN" => Some(Expression::Number(f64::NAN)),
                    "Infinity" => Some(Expression::Number(f64::INFINITY)),
                    _ => None,
                }
            }
            Expression::Identifier(name) => self
                .state
                .speculation
                .static_semantics
                .local_value_binding(name)
                .or_else(|| self.global_value_binding(name))
                .and_then(|value| {
                    self.same_value_assertion_direct_static_value(value, depth - 1)
                        .or_else(|| Some(value.clone()))
                }),
            Expression::Member { object, property } => {
                if let Expression::Call { callee, arguments } = object.as_ref()
                    && matches!(
                        callee.as_ref(),
                        Expression::Member { object, property }
                            if matches!(object.as_ref(), Expression::Identifier(name) if name == "Object")
                                && matches!(property.as_ref(), Expression::String(name) if name == "getPrototypeOf")
                    )
                    && let Some(CallArgument::Expression(target) | CallArgument::Spread(target)) =
                        arguments.first()
                    && let Some(prototype) = self.resolve_static_object_prototype_expression(target)
                {
                    let prototype_member = Expression::Member {
                        object: Box::new(prototype),
                        property: property.clone(),
                    };
                    if let Some(value) =
                        self.same_value_assertion_direct_static_value(&prototype_member, depth - 1)
                    {
                        return Some(value);
                    }
                    let materialized = self.materialize_static_expression(&prototype_member);
                    if self.same_value_assertion_is_primitive_literal_operand(&materialized) {
                        return Some(materialized);
                    }
                }
                let resolved_property = if self.same_value_operand_static_evaluation_safe(property)
                {
                    self.same_value_assertion_direct_static_value(property, depth - 1)
                        .filter(|value| static_property_name_from_expression(value).is_some())
                } else {
                    None
                };
                let property = resolved_property.as_ref().unwrap_or(property.as_ref());
                if let Some(getter_binding) = self.resolve_member_getter_binding(object, property)
                    && let Some(value) = self.resolve_static_getter_value_from_binding_with_context(
                        &getter_binding,
                        object,
                        self.current_function_name(),
                    )
                {
                    if let Some(primitive) = self.resolve_static_primitive_expression_with_context(
                        &value,
                        self.current_function_name(),
                    ) {
                        return Some(primitive);
                    }
                    let materialized = self.materialize_static_expression(&value);
                    if let Some(primitive) = self.resolve_static_primitive_expression_with_context(
                        &materialized,
                        self.current_function_name(),
                    ) {
                        return Some(primitive);
                    }
                    return self
                        .same_value_assertion_direct_static_value(&value, depth - 1)
                        .or(Some(value));
                }
                if matches!(
                    static_property_name_from_expression(property).as_deref(),
                    Some("length")
                ) && let Some(array_binding) =
                    self.same_value_assertion_direct_array_binding(object, depth - 1)
                {
                    return Some(Expression::Number(array_binding.values.len() as f64));
                }
                if let Some(index) = argument_index_from_expression(property)
                    && let Some(array_binding) =
                        self.same_value_assertion_direct_array_binding(object, depth - 1)
                    && let Some(Some(value)) = array_binding.values.get(index as usize)
                {
                    return self
                        .same_value_assertion_direct_static_value(value, depth - 1)
                        .or_else(|| Some(value.clone()));
                }
                if let Some(value) =
                    self.same_value_assertion_direct_object_property(object, property, depth - 1)
                {
                    return self
                        .same_value_assertion_direct_static_value(&value, depth - 1)
                        .or(Some(value));
                }
                None
            }
            Expression::Binary { op, left, right } => {
                let left_value = self.same_value_assertion_direct_static_value(left, depth - 1)?;
                let right_value =
                    self.same_value_assertion_direct_static_value(right, depth - 1)?;
                Self::same_value_static_binary_expression_value(*op, &left_value, &right_value)
            }
            Expression::Unary {
                op: UnaryOp::TypeOf,
                expression,
            } => self
                .same_value_assertion_direct_typeof_string(expression, depth - 1)
                .map(Expression::String),
            _ => None,
        }
    }

    fn same_value_assertion_direct_typeof_string(
        &self,
        expression: &Expression,
        depth: usize,
    ) -> Option<String> {
        if depth == 0 {
            return None;
        }
        let value = self
            .same_value_assertion_direct_static_value(expression, depth - 1)
            .unwrap_or_else(|| expression.clone());
        if self.same_value_assertion_direct_value_is_function(&value) {
            return Some("function".to_string());
        }
        match value {
            Expression::Number(_) => Some("number".to_string()),
            Expression::BigInt(_) => Some("bigint".to_string()),
            Expression::String(_) => Some("string".to_string()),
            Expression::Bool(_) => Some("boolean".to_string()),
            Expression::Null => Some("object".to_string()),
            Expression::Undefined => Some("undefined".to_string()),
            Expression::Array(_) | Expression::Object(_) | Expression::This => {
                Some("object".to_string())
            }
            Expression::Identifier(ref name) => self
                .backend
                .global_property_descriptor(name)
                .or_else(|| {
                    self.backend
                        .shared_global_semantics
                        .values
                        .property_descriptor(name)
                })
                .is_some_and(|state| state.has_get || state.getter.is_some())
                .then_some(None)
                .unwrap_or_else(|| {
                    self.state
                        .speculation
                        .static_semantics
                        .local_kind(name)
                        .or_else(|| self.global_binding_kind(name))
                        .or_else(|| builtin_identifier_kind(name))
                        .and_then(StaticValueKind::as_typeof_str)
                        .map(str::to_string)
                }),
            _ => None,
        }
    }

    fn same_value_assertion_direct_value_is_function(&self, expression: &Expression) -> bool {
        match expression {
            Expression::Identifier(name) => {
                self.state
                    .speculation
                    .static_semantics
                    .local_function_binding(name)
                    .is_some()
                    || self.backend.global_function_binding(name).is_some()
                    || builtin_identifier_kind(name) == Some(StaticValueKind::Function)
                    || self
                        .backend
                        .function_registry
                        .catalog
                        .user_function(name)
                        .is_some()
            }
            _ => false,
        }
    }

    fn same_value_assertion_direct_static_result(
        &self,
        actual: &Expression,
        expected: &Expression,
    ) -> Option<bool> {
        let actual_value = self
            .same_value_assertion_direct_static_value(actual, 10)
            .unwrap_or_else(|| actual.clone());
        let expected_value = self
            .same_value_assertion_direct_static_value(expected, 10)
            .unwrap_or_else(|| expected.clone());
        self.same_value_assertion_primitive_result(&actual_value, &expected_value)
    }

    fn same_value_assertion_is_primitive_literal_operand(&self, expression: &Expression) -> bool {
        matches!(
            expression,
            Expression::Number(_)
                | Expression::BigInt(_)
                | Expression::String(_)
                | Expression::Bool(_)
                | Expression::Null
                | Expression::Undefined
        ) || matches!(
            expression,
            Expression::Identifier(name)
                if matches!(name.as_str(), "undefined" | "NaN" | "Infinity")
                    && self.is_unshadowed_builtin_identifier(name)
        )
    }

    fn same_value_assertion_member_chain_depth(expression: &Expression) -> usize {
        match expression {
            Expression::Member { object, .. } => {
                1 + Self::same_value_assertion_member_chain_depth(object)
            }
            Expression::Unary {
                op: UnaryOp::TypeOf,
                expression,
            } => Self::same_value_assertion_member_chain_depth(expression),
            _ => 0,
        }
    }

    fn same_value_assertion_contains_symbol_to_primitive_reference(
        expression: &Expression,
    ) -> bool {
        match expression {
            Expression::Member { object, property }
                if matches!(object.as_ref(), Expression::Identifier(name) if name == "Symbol")
                    && matches!(property.as_ref(), Expression::String(name) if name == "toPrimitive") =>
            {
                true
            }
            Expression::Member { object, property } => {
                Self::same_value_assertion_contains_symbol_to_primitive_reference(object)
                    || Self::same_value_assertion_contains_symbol_to_primitive_reference(property)
            }
            Expression::SuperMember { property } => {
                Self::same_value_assertion_contains_symbol_to_primitive_reference(property)
            }
            Expression::Unary { expression, .. }
            | Expression::Await(expression)
            | Expression::EnumerateKeys(expression)
            | Expression::GetIterator(expression)
            | Expression::IteratorClose(expression)
            | Expression::Assign {
                value: expression, ..
            } => Self::same_value_assertion_contains_symbol_to_primitive_reference(expression),
            Expression::AssignMember {
                object,
                property,
                value,
            } => {
                Self::same_value_assertion_contains_symbol_to_primitive_reference(object)
                    || Self::same_value_assertion_contains_symbol_to_primitive_reference(property)
                    || Self::same_value_assertion_contains_symbol_to_primitive_reference(value)
            }
            Expression::AssignSuperMember { property, value } => {
                Self::same_value_assertion_contains_symbol_to_primitive_reference(property)
                    || Self::same_value_assertion_contains_symbol_to_primitive_reference(value)
            }
            Expression::Binary { left, right, .. } => {
                Self::same_value_assertion_contains_symbol_to_primitive_reference(left)
                    || Self::same_value_assertion_contains_symbol_to_primitive_reference(right)
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                Self::same_value_assertion_contains_symbol_to_primitive_reference(condition)
                    || Self::same_value_assertion_contains_symbol_to_primitive_reference(
                        then_expression,
                    )
                    || Self::same_value_assertion_contains_symbol_to_primitive_reference(
                        else_expression,
                    )
            }
            Expression::Sequence(expressions) => expressions
                .iter()
                .any(Self::same_value_assertion_contains_symbol_to_primitive_reference),
            Expression::Call { callee, arguments }
            | Expression::SuperCall { callee, arguments }
            | Expression::New { callee, arguments } => {
                Self::same_value_assertion_contains_symbol_to_primitive_reference(callee)
                    || arguments.iter().any(|argument| match argument {
                        CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                            Self::same_value_assertion_contains_symbol_to_primitive_reference(
                                expression,
                            )
                        }
                    })
            }
            Expression::Array(elements) => elements.iter().any(|element| match element {
                ArrayElement::Expression(expression) | ArrayElement::Spread(expression) => {
                    Self::same_value_assertion_contains_symbol_to_primitive_reference(expression)
                }
            }),
            Expression::Object(entries) => entries.iter().any(|entry| match entry {
                ObjectEntry::Data { key, value }
                | ObjectEntry::Getter { key, getter: value }
                | ObjectEntry::Setter { key, setter: value } => {
                    Self::same_value_assertion_contains_symbol_to_primitive_reference(key)
                        || Self::same_value_assertion_contains_symbol_to_primitive_reference(value)
                }
                ObjectEntry::Spread(expression) => {
                    Self::same_value_assertion_contains_symbol_to_primitive_reference(expression)
                }
            }),
            Expression::Identifier(_)
            | Expression::Number(_)
            | Expression::BigInt(_)
            | Expression::String(_)
            | Expression::Bool(_)
            | Expression::Null
            | Expression::Undefined
            | Expression::This
            | Expression::NewTarget
            | Expression::Sent
            | Expression::Update { .. } => false,
        }
    }

    fn same_value_assertion_is_typeof_string_operand_pair(
        actual: &Expression,
        expected: &Expression,
    ) -> bool {
        matches!(
            (actual, expected),
            (
                Expression::Unary {
                    op: UnaryOp::TypeOf,
                    ..
                },
                Expression::String(_)
            ) | (
                Expression::String(_),
                Expression::Unary {
                    op: UnaryOp::TypeOf,
                    ..
                }
            )
        ) || matches!(
            (actual, expected),
            (Expression::String(text), _) | (_, Expression::String(text))
                if parse_typeof_tag_optional(text).is_some()
        )
    }

    fn same_value_assertion_should_skip_broad_static_result(
        &self,
        actual: &Expression,
        expected: &Expression,
        direct_static_result: Option<bool>,
    ) -> bool {
        if direct_static_result == Some(false) {
            return true;
        }
        if Self::same_value_assertion_contains_symbol_to_primitive_reference(actual)
            || Self::same_value_assertion_contains_symbol_to_primitive_reference(expected)
        {
            return true;
        }
        (self.same_value_assertion_is_primitive_literal_operand(actual)
            && Self::same_value_assertion_member_chain_depth(expected) > 0)
            || (self.same_value_assertion_is_primitive_literal_operand(expected)
                && Self::same_value_assertion_member_chain_depth(actual) > 0)
    }

    fn same_value_assertion_needs_runtime_identifier_check(
        &self,
        actual: &Expression,
        expected: &Expression,
    ) -> bool {
        fn is_syntactic_primitive(
            compiler: &FunctionCompiler<'_>,
            expression: &Expression,
        ) -> bool {
            matches!(
                expression,
                Expression::Number(_)
                    | Expression::BigInt(_)
                    | Expression::String(_)
                    | Expression::Bool(_)
                    | Expression::Null
                    | Expression::Undefined
            ) || matches!(
                expression,
                Expression::Identifier(name)
                    if matches!(name.as_str(), "undefined" | "NaN")
                        && compiler.is_unshadowed_builtin_identifier(name)
            )
        }

        matches!(actual, Expression::Identifier(_)) && is_syntactic_primitive(self, expected)
            || matches!(expected, Expression::Identifier(_)) && is_syntactic_primitive(self, actual)
    }

    fn import_meta_identifier_module_index(name: &str) -> Option<&str> {
        let suffix = name.strip_prefix("__ayy_import_meta_").or_else(|| {
            name.rsplit_once("__ayy_import_meta_")
                .map(|(_, suffix)| suffix)
        })?;
        let digit_count = suffix
            .bytes()
            .take_while(|byte| byte.is_ascii_digit())
            .count();
        (digit_count > 0 && digit_count == suffix.len()).then_some(suffix)
    }

    fn same_value_import_meta_identity_probe_safe(expression: &Expression) -> bool {
        match expression {
            Expression::Identifier(name) => {
                Self::import_meta_identifier_module_index(name).is_some()
            }
            Expression::Call { callee, arguments } => {
                if matches!(callee.as_ref(), Expression::Identifier(name) if name == "__ayyImportMeta")
                    && matches!(
                        arguments.as_slice(),
                        [] | [CallArgument::Expression(Expression::Number(_))]
                            | [CallArgument::Spread(Expression::Number(_))]
                    )
                {
                    return true;
                }
                matches!(
                    callee.as_ref(),
                    Expression::Member { object, .. }
                        if matches!(
                            object.as_ref(),
                            Expression::Identifier(name) if name.starts_with("__ayy_module_dep_")
                        )
                ) && arguments.is_empty()
            }
            Expression::Member { object, .. } => {
                matches!(
                    object.as_ref(),
                    Expression::Identifier(name) if name.starts_with("__ayy_module_dep_")
                )
            }
            _ => false,
        }
    }

    fn module_index_from_namespace_identifier(name: &str) -> Option<&str> {
        let suffix = name
            .strip_prefix("__ayy_module_dep_")
            .or_else(|| name.strip_prefix("__ayy_module_namespace_"))?;
        let digit_count = suffix
            .bytes()
            .take_while(|byte| byte.is_ascii_digit())
            .count();
        (digit_count > 0).then_some(&suffix[..digit_count])
    }

    fn same_value_import_meta_identity_key(expression: &Expression) -> Option<String> {
        match expression {
            Expression::Identifier(name) => Self::import_meta_identifier_module_index(name)
                .map(|module_index| format!("import-meta:{module_index}")),
            Expression::Call { callee, arguments } => {
                if matches!(callee.as_ref(), Expression::Identifier(name) if name == "__ayyImportMeta")
                {
                    let module_key = match arguments.as_slice() {
                        [] => "global".to_string(),
                        [CallArgument::Expression(Expression::Number(index))]
                        | [CallArgument::Spread(Expression::Number(index))]
                            if index.is_finite() =>
                        {
                            let integer = index.trunc();
                            if integer != *index || integer < 0.0 {
                                return None;
                            }
                            format!("{integer:.0}")
                        }
                        _ => return None,
                    };
                    return Some(format!("import-meta:{module_key}"));
                }
                if let Expression::Member { object, property } = callee.as_ref()
                    && arguments.is_empty()
                    && matches!(property.as_ref(), Expression::String(name) if name == "getMeta")
                    && let Expression::Identifier(namespace) = object.as_ref()
                    && let Some(module_index) =
                        Self::module_index_from_namespace_identifier(namespace)
                {
                    return Some(format!("import-meta:{module_index}"));
                }
                None
            }
            Expression::Member { object, property } if matches!(property.as_ref(), Expression::String(name) if name == "meta") =>
            {
                let Expression::Identifier(namespace) = object.as_ref() else {
                    return None;
                };
                Self::module_index_from_namespace_identifier(namespace)
                    .map(|module_index| format!("import-meta:{module_index}"))
            }
            _ => None,
        }
    }

    pub(in crate::backend::direct_wasm) fn emit_same_value_assertion(
        &mut self,
        name: &str,
        arguments: &[CallArgument],
    ) -> DirectResult<bool> {
        let [
            CallArgument::Expression(actual),
            CallArgument::Expression(expected),
            ..,
        ] = arguments
        else {
            return Ok(false);
        };
        let assertion_failure = match name {
            "__assertSameValue" => BinaryOp::NotEqual,
            "__assertNotSameValue" => BinaryOp::Equal,
            _ => return Ok(false),
        };
        let trace_assertions = std::env::var_os("AYY_TRACE_ASSERTIONS").is_some();
        if trace_assertions {
            eprintln!(
                "same_value_assertion:start name={name} actual={actual:?} expected={expected:?} fn={:?}",
                self.current_function_name()
            );
        }
        let fast_extra_arguments_side_effect_free =
            arguments.iter().skip(2).all(|argument| match argument {
                CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                    inline_summary_side_effect_free_expression(expression)
                }
            });
        if fast_extra_arguments_side_effect_free
            && !self.assertion_requires_runtime_same_value_fallback()
            && let Some(static_result) =
                self.same_value_assertion_fast_primitive_result(actual, expected)
        {
            let assertion_passes = match assertion_failure {
                BinaryOp::NotEqual => static_result,
                BinaryOp::Equal => !static_result,
                _ => false,
            };
            if assertion_passes {
                if trace_assertions {
                    eprintln!(
                        "same_value_assertion:fast_static_success name={name} actual={actual:?} expected={expected:?} fn={:?}",
                        self.current_function_name()
                    );
                }
                self.push_i32_const(JS_UNDEFINED_TAG);
                return Ok(true);
            }
        }
        let actual_local = self.allocate_temp_local();
        let expected_local = self.allocate_temp_local();
        let operands_side_effect_free = inline_summary_side_effect_free_expression(actual)
            && inline_summary_side_effect_free_expression(expected);
        let operands_static_evaluation_safe = self
            .same_value_operand_static_evaluation_safe(actual)
            && self.same_value_operand_static_evaluation_safe(expected);
        let direct_static_result = self.same_value_assertion_direct_static_result(actual, expected);
        let handled_as_typeof =
            Self::same_value_assertion_is_typeof_string_operand_pair(actual, expected);
        let skip_broad_static_result = self.same_value_assertion_should_skip_broad_static_result(
            actual,
            expected,
            direct_static_result,
        );
        let reference_identity_can_affect_result = !self
            .same_value_assertion_is_primitive_literal_operand(actual)
            && !self.same_value_assertion_is_primitive_literal_operand(expected);
        let import_meta_identity_probe_safe =
            Self::same_value_import_meta_identity_probe_safe(actual)
                && Self::same_value_import_meta_identity_probe_safe(expected);
        let (actual_reference_identity, expected_reference_identity) =
            if reference_identity_can_affect_result
                && (operands_side_effect_free || import_meta_identity_probe_safe)
            {
                (
                    Self::same_value_import_meta_identity_key(actual)
                        .or_else(|| self.resolve_static_reference_identity_key(actual)),
                    Self::same_value_import_meta_identity_key(expected)
                        .or_else(|| self.resolve_static_reference_identity_key(expected)),
                )
            } else {
                (None, None)
            };
        if trace_assertions {
            eprintln!(
                "same_value_assertion:identity actual={actual_reference_identity:?} expected={expected_reference_identity:?} fn={:?}",
                self.current_function_name()
            );
        }
        let has_static_reference_identity =
            actual_reference_identity.is_some() && expected_reference_identity.is_some();
        let needs_runtime_identifier_check =
            self.same_value_assertion_needs_runtime_identifier_check(actual, expected);
        let identifier_operands_are_unshadowed_primitives =
            [actual, expected].iter().all(|expression| {
                !matches!(expression, Expression::Identifier(_))
                    || matches!(
                        expression,
                        Expression::Identifier(name)
                            if matches!(name.as_str(), "undefined" | "NaN" | "Infinity")
                                && self.is_unshadowed_builtin_identifier(name)
                    )
            });
        let extra_arguments_side_effect_free = fast_extra_arguments_side_effect_free;
        let materialized_skip_static_result = if skip_broad_static_result
            && operands_static_evaluation_safe
            && extra_arguments_side_effect_free
            && !needs_runtime_identifier_check
            && !self.assertion_requires_runtime_same_value_fallback()
            && !handled_as_typeof
        {
            let actual_materialized = self.materialize_static_expression(actual);
            let expected_materialized = self.materialize_static_expression(expected);
            let actual_value = self
                .resolve_static_primitive_expression_with_context(
                    &actual_materialized,
                    self.current_function_name(),
                )
                .unwrap_or(actual_materialized);
            let expected_value = self
                .resolve_static_primitive_expression_with_context(
                    &expected_materialized,
                    self.current_function_name(),
                )
                .unwrap_or(expected_materialized);
            self.same_value_assertion_primitive_result(&actual_value, &expected_value)
        } else {
            None
        };
        if extra_arguments_side_effect_free
            && let (Some(actual_key), Some(expected_key)) = (
                actual_reference_identity.as_ref(),
                expected_reference_identity.as_ref(),
            )
        {
            let static_result = actual_key == expected_key;
            let assertion_passes = match assertion_failure {
                BinaryOp::NotEqual => static_result,
                BinaryOp::Equal => !static_result,
                _ => false,
            };
            if assertion_passes {
                self.push_i32_const(JS_UNDEFINED_TAG);
                return Ok(true);
            }
        }
        let operands_contain_member_access =
            Self::same_value_operand_contains_member_access(actual)
                || Self::same_value_operand_contains_member_access(expected);
        if operands_static_evaluation_safe
            && extra_arguments_side_effect_free
            && !needs_runtime_identifier_check
            && !self.assertion_requires_runtime_same_value_fallback()
            && !handled_as_typeof
            && let Some(static_result) = direct_static_result.or_else(|| {
                if skip_broad_static_result {
                    None
                } else {
                    self.resolve_static_same_value_result_with_context(
                        actual,
                        expected,
                        self.current_function_name(),
                    )
                }
            })
        {
            let assertion_passes = match assertion_failure {
                BinaryOp::NotEqual => static_result,
                BinaryOp::Equal => !static_result,
                _ => false,
            };
            if assertion_passes {
                if trace_assertions {
                    eprintln!(
                        "same_value_assertion:static_success name={name} actual={actual:?} expected={expected:?} fn={:?}",
                        self.current_function_name()
                    );
                }
                self.push_i32_const(JS_UNDEFINED_TAG);
                return Ok(true);
            }
        }
        let can_use_effectful_static_boolean_call_result = extra_arguments_side_effect_free
            && !needs_runtime_identifier_check
            && !operands_side_effect_free
            && (matches!(actual, Expression::Call { .. })
                && matches!(expected, Expression::Bool(_))
                || matches!(expected, Expression::Call { .. })
                    && matches!(actual, Expression::Bool(_)));
        if can_use_effectful_static_boolean_call_result
            && let Some(static_result) = self.resolve_static_same_value_result_with_context(
                actual,
                expected,
                self.current_function_name(),
            )
        {
            let assertion_passes = match assertion_failure {
                BinaryOp::NotEqual => static_result,
                BinaryOp::Equal => !static_result,
                _ => false,
            };
            if assertion_passes {
                if !inline_summary_side_effect_free_expression(actual) {
                    self.emit_same_value_operand(actual)?;
                    self.state.emission.output.instructions.push(0x1a);
                }
                if !inline_summary_side_effect_free_expression(expected) {
                    self.emit_same_value_operand(expected)?;
                    self.state.emission.output.instructions.push(0x1a);
                }
                self.push_i32_const(JS_UNDEFINED_TAG);
                return Ok(true);
            }
        }
        if operands_side_effect_free
            && !operands_contain_member_access
            && !needs_runtime_identifier_check
            && !handled_as_typeof
            && let (Some(actual_text), Some(expected_text)) = (
                self.resolve_static_string_value(actual),
                self.resolve_static_string_value(expected),
            )
        {
            self.push_i32_const((actual_text == expected_text) as i32);
            self.push_local_set(actual_local);
            if assertion_failure == BinaryOp::NotEqual {
                self.push_local_get(actual_local);
                self.state.emission.output.instructions.push(0x45);
                self.push_local_set(actual_local);
            }
        } else if operands_static_evaluation_safe
            && extra_arguments_side_effect_free
            && !needs_runtime_identifier_check
            && !handled_as_typeof
            && (matches!(actual, Expression::String(_))
                || matches!(expected, Expression::String(_)))
            && let (Some(actual_text), Some(expected_text)) = (
                self.resolve_static_string_value_with_context(actual, self.current_function_name()),
                self.resolve_static_string_value_with_context(
                    expected,
                    self.current_function_name(),
                ),
            )
        {
            self.push_i32_const((actual_text == expected_text) as i32);
            self.push_local_set(actual_local);
            if assertion_failure == BinaryOp::NotEqual {
                self.push_local_get(actual_local);
                self.state.emission.output.instructions.push(0x45);
                self.push_local_set(actual_local);
            }
        } else if skip_broad_static_result
            && operands_static_evaluation_safe
            && extra_arguments_side_effect_free
            && !needs_runtime_identifier_check
            && !self.assertion_requires_runtime_same_value_fallback()
            && !handled_as_typeof
            && let Some(static_result) = self.resolve_static_same_value_result_with_context(
                actual,
                expected,
                self.current_function_name(),
            )
        {
            self.push_i32_const(static_result as i32);
            self.push_local_set(actual_local);
            if assertion_failure == BinaryOp::NotEqual {
                self.push_local_get(actual_local);
                self.state.emission.output.instructions.push(0x45);
                self.push_local_set(actual_local);
            }
        } else if let Some(static_result) = materialized_skip_static_result {
            self.push_i32_const(static_result as i32);
            self.push_local_set(actual_local);
            if assertion_failure == BinaryOp::NotEqual {
                self.push_local_get(actual_local);
                self.state.emission.output.instructions.push(0x45);
                self.push_local_set(actual_local);
            }
        } else {
            if handled_as_typeof {
                if self.emit_typeof_string_comparison(actual, expected, assertion_failure)?
                    || self.emit_runtime_typeof_tag_string_comparison(
                        actual,
                        expected,
                        assertion_failure,
                    )?
                {
                    self.push_local_set(actual_local);
                } else {
                    self.push_i32_const(0);
                    self.push_local_set(actual_local);
                }
            } else if !needs_runtime_identifier_check
                && (matches!(actual, Expression::String(_))
                    || matches!(expected, Expression::String(_)))
                && (matches!(actual, Expression::Call { .. })
                    || matches!(expected, Expression::Call { .. }))
                && let Some(static_result) = self.resolve_static_same_value_result_with_context(
                    actual,
                    expected,
                    self.current_function_name(),
                )
            {
                if !inline_summary_side_effect_free_expression(actual) {
                    self.emit_same_value_operand(actual)?;
                    self.state.emission.output.instructions.push(0x1a);
                }
                if !inline_summary_side_effect_free_expression(expected) {
                    self.emit_same_value_operand(expected)?;
                    self.state.emission.output.instructions.push(0x1a);
                }
                let assertion_passes = match assertion_failure {
                    BinaryOp::NotEqual => static_result,
                    BinaryOp::Equal => !static_result,
                    _ => false,
                };
                self.push_i32_const(assertion_passes as i32);
                self.push_local_set(actual_local);
            } else if !needs_runtime_identifier_check
                && (matches!(actual, Expression::String(_))
                    || matches!(expected, Expression::String(_)))
            {
                self.emit_numeric_expression(&Expression::Binary {
                    op: BinaryOp::Equal,
                    left: Box::new(actual.clone()),
                    right: Box::new(expected.clone()),
                })?;
                self.push_local_set(actual_local);
                if assertion_failure == BinaryOp::NotEqual {
                    self.push_local_get(actual_local);
                    self.state.emission.output.instructions.push(0x45);
                    self.push_local_set(actual_local);
                }
            } else if !needs_runtime_identifier_check
                && self.emit_runtime_static_string_equality_comparison(
                    actual,
                    expected,
                    BinaryOp::Equal,
                )?
            {
                self.push_local_set(actual_local);
                if assertion_failure == BinaryOp::NotEqual {
                    self.push_local_get(actual_local);
                    self.state.emission.output.instructions.push(0x45);
                    self.push_local_set(actual_local);
                }
            } else {
                let can_use_static_same_value_result = !skip_broad_static_result
                    && !needs_runtime_identifier_check
                    && (!self.assertion_requires_runtime_same_value_fallback()
                        || has_static_reference_identity)
                    && operands_static_evaluation_safe
                    && (matches!(actual, Expression::This)
                        || matches!(expected, Expression::This)
                        || self.resolve_array_binding_from_expression(actual).is_some()
                        || self
                            .resolve_array_binding_from_expression(expected)
                            .is_some()
                        || self
                            .resolve_object_binding_from_expression(actual)
                            .is_some()
                        || self
                            .resolve_object_binding_from_expression(expected)
                            .is_some()
                        || self.resolve_user_function_from_expression(actual).is_some()
                        || self
                            .resolve_user_function_from_expression(expected)
                            .is_some()
                        || has_static_reference_identity
                        || identifier_operands_are_unshadowed_primitives
                        || (!matches!(actual, Expression::Identifier(_))
                            && !matches!(expected, Expression::Identifier(_))));
                let static_same_value_result = if can_use_static_same_value_result {
                    if trace_assertions {
                        eprintln!(
                            "same_value_assertion:resolve_static start actual={actual:?} expected={expected:?} fn={:?}",
                            self.current_function_name()
                        );
                    }
                    let result = self.resolve_static_same_value_result_with_context(
                        actual,
                        expected,
                        self.current_function_name(),
                    );
                    if trace_assertions {
                        eprintln!(
                            "same_value_assertion:resolve_static result={result:?} actual={actual:?} expected={expected:?} fn={:?}",
                            self.current_function_name()
                        );
                    }
                    result
                } else {
                    if trace_assertions {
                        eprintln!(
                            "same_value_assertion:resolve_static skipped actual={actual:?} expected={expected:?} side_effect_free={operands_side_effect_free} fn={:?}",
                            self.current_function_name()
                        );
                    }
                    None
                };
                if let Some(result) = static_same_value_result {
                    self.push_i32_const(result as i32);
                    self.push_local_set(actual_local);
                    if assertion_failure == BinaryOp::NotEqual {
                        self.push_local_get(actual_local);
                        self.state.emission.output.instructions.push(0x45);
                        self.push_local_set(actual_local);
                    }
                } else {
                    if trace_assertions {
                        eprintln!(
                            "same_value_assertion:emit_operands start actual={actual:?} expected={expected:?} fn={:?}",
                            self.current_function_name()
                        );
                    }
                    self.emit_same_value_operand(actual)?;
                    self.push_local_set(actual_local);
                    self.emit_same_value_operand(expected)?;
                    self.push_local_set(expected_local);
                    self.emit_same_value_result_from_locals(
                        actual_local,
                        expected_local,
                        actual_local,
                    )?;
                    if assertion_failure == BinaryOp::NotEqual {
                        self.push_local_get(actual_local);
                        self.state.emission.output.instructions.push(0x45);
                        self.push_local_set(actual_local);
                    }
                    if trace_assertions {
                        eprintln!(
                            "same_value_assertion:emit_operands done actual={actual:?} expected={expected:?} fn={:?}",
                            self.current_function_name()
                        );
                    }
                }
            }
        }
        for argument in arguments.iter().skip(2) {
            match argument {
                CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                    self.emit_numeric_expression(expression)?;
                    self.state.emission.output.instructions.push(0x1a);
                }
            }
        }
        self.push_local_get(actual_local);
        self.state.emission.output.instructions.push(0x04);
        self.state
            .emission
            .output
            .instructions
            .push(EMPTY_BLOCK_TYPE);
        self.push_control_frame();
        self.emit_error_throw()?;
        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();
        self.push_i32_const(JS_UNDEFINED_TAG);
        Ok(true)
    }
}
