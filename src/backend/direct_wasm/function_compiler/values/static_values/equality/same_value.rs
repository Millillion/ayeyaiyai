use super::*;
use std::collections::HashSet;

fn expression_is_dynamic_import_call(expression: &Expression) -> bool {
    matches!(
        expression,
        Expression::Call { callee, .. }
            if matches!(callee.as_ref(), Expression::Identifier(name) if name == "__ayyDynamicImport")
    )
}

impl<'a> FunctionCompiler<'a> {
    pub(in crate::backend::direct_wasm) fn resolve_static_same_value_result_with_context(
        &self,
        actual: &Expression,
        expected: &Expression,
        current_function_name: Option<&str>,
    ) -> Option<bool> {
        let member_call_has_lexical_new_target_capture = |expression: &Expression| {
            if let Expression::Call { callee, .. } = expression
                && let Expression::Member { object, property } = callee.as_ref()
            {
                return self
                    .resolve_member_function_capture_slots(object, property)
                    .is_some_and(|capture_slots| capture_slots.contains_key("new.target"));
            }
            false
        };
        if member_call_has_lexical_new_target_capture(actual)
            || member_call_has_lexical_new_target_capture(expected)
        {
            return None;
        }
        if self.same_value_expression_depends_on_dynamic_this(actual, current_function_name)
            || self.same_value_expression_depends_on_dynamic_this(expected, current_function_name)
        {
            return None;
        }

        let actual_primitive =
            self.resolve_static_primitive_expression_with_context(actual, current_function_name);
        let expected_primitive =
            self.resolve_static_primitive_expression_with_context(expected, current_function_name);
        let actual_has_primitive = actual_primitive.is_some();
        let expected_has_primitive = expected_primitive.is_some();

        if let (Some(actual_primitive), Some(expected_primitive)) =
            (actual_primitive, expected_primitive)
        {
            return match (actual_primitive, expected_primitive) {
                (Expression::Number(actual), Expression::Number(expected)) => {
                    if actual.is_nan() && expected.is_nan() {
                        Some(true)
                    } else if actual == 0.0 && expected == 0.0 {
                        Some(actual.is_sign_negative() == expected.is_sign_negative())
                    } else {
                        Some(actual == expected)
                    }
                }
                (Expression::BigInt(actual), Expression::BigInt(expected)) => Some(
                    parse_static_bigint_literal(&actual)?
                        == parse_static_bigint_literal(&expected)?,
                ),
                (Expression::String(actual), Expression::String(expected)) => {
                    Some(actual == expected)
                }
                (Expression::Bool(actual), Expression::Bool(expected)) => Some(actual == expected),
                (Expression::Null, Expression::Null)
                | (Expression::Undefined, Expression::Undefined) => Some(true),
                _ => Some(false),
            };
        }

        if actual_has_primitive != expected_has_primitive
            && (matches!(actual, Expression::Identifier(_))
                || matches!(expected, Expression::Identifier(_)))
        {
            return None;
        }

        let actual_reference_key = self.resolve_static_reference_identity_key(actual);
        let expected_reference_key = self.resolve_static_reference_identity_key(expected);
        if current_function_name.is_none()
            && ((actual_reference_key
                .as_deref()
                .is_some_and(|key| key.starts_with("new-object:"))
                && expected_reference_key.as_deref() == Some("this"))
                || (expected_reference_key
                    .as_deref()
                    .is_some_and(|key| key.starts_with("new-object:"))
                    && actual_reference_key.as_deref() == Some("this")))
        {
            return Some(false);
        }

        let materializes_to_top_level_this = |expression: &Expression| {
            current_function_name.is_none()
                && (matches!(expression, Expression::This)
                    || matches!(
                        self.materialize_static_expression(expression),
                        Expression::This
                    ))
        };
        if materializes_to_top_level_this(actual) ^ materializes_to_top_level_this(expected) {
            return None;
        }
        if current_function_name.is_none()
            && ((actual_reference_key.as_deref() == Some("this"))
                ^ (expected_reference_key.as_deref() == Some("this")))
        {
            return None;
        }
        if let (Some(actual_key), Some(expected_key)) =
            (actual_reference_key, expected_reference_key)
        {
            if actual_key != expected_key
                && (actual_key.contains("__ayy_scope$") || expected_key.contains("__ayy_scope$"))
            {
                return None;
            }
            return Some(actual_key == expected_key);
        }

        let apply_member_call_capture_slots =
            |source: &Expression, materialized: Expression| -> Expression {
                if let Expression::Call { callee, arguments } = source
                    && arguments.is_empty()
                    && let Expression::Member { object, property } = callee.as_ref()
                    && let Expression::String(property_name) = property.as_ref()
                    && let Some(StaticEvalOutcome::Value(value)) = self
                        .resolve_static_member_call_outcome_with_context(
                            object,
                            property_name,
                            current_function_name,
                        )
                {
                    value
                } else if let Expression::Call { callee, .. } = source
                    && let Expression::Member { object, property } = callee.as_ref()
                    && let Some(capture_slots) =
                        self.resolve_member_function_capture_slots(object, property)
                {
                    let substituted =
                        self.substitute_capture_slot_bindings(&materialized, &capture_slots);
                    if matches!(substituted, Expression::Identifier(_)) {
                        substituted
                    } else {
                        self.materialize_static_expression(&substituted)
                    }
                } else {
                    materialized
                }
            };
        let actual_materialized =
            apply_member_call_capture_slots(actual, self.materialize_static_expression(actual));
        let expected_materialized =
            apply_member_call_capture_slots(expected, self.materialize_static_expression(expected));

        if expression_is_dynamic_import_call(&actual_materialized)
            || expression_is_dynamic_import_call(&expected_materialized)
        {
            return Some(false);
        }

        let actual_materialized_primitive = self.resolve_static_primitive_expression_with_context(
            &actual_materialized,
            current_function_name,
        );
        let expected_materialized_primitive = self
            .resolve_static_primitive_expression_with_context(
                &expected_materialized,
                current_function_name,
            );
        if let (Some(actual_primitive), Some(expected_primitive)) = (
            actual_materialized_primitive,
            expected_materialized_primitive,
        ) {
            return match (actual_primitive, expected_primitive) {
                (Expression::Number(actual), Expression::Number(expected)) => {
                    if actual.is_nan() && expected.is_nan() {
                        Some(true)
                    } else if actual == 0.0 && expected == 0.0 {
                        Some(actual.is_sign_negative() == expected.is_sign_negative())
                    } else {
                        Some(actual == expected)
                    }
                }
                (Expression::BigInt(actual), Expression::BigInt(expected)) => Some(
                    parse_static_bigint_literal(&actual)?
                        == parse_static_bigint_literal(&expected)?,
                ),
                (Expression::String(actual), Expression::String(expected)) => {
                    Some(actual == expected)
                }
                (Expression::Bool(actual), Expression::Bool(expected)) => Some(actual == expected),
                (Expression::Null, Expression::Null)
                | (Expression::Undefined, Expression::Undefined) => Some(true),
                _ => Some(false),
            };
        }

        let actual_is_this = matches!(actual_materialized, Expression::This);
        let expected_is_this = matches!(expected_materialized, Expression::This);
        let has_static_reference_identity = |expression: &Expression| {
            self.resolve_object_binding_from_expression(expression)
                .is_some()
                || self
                    .resolve_array_binding_from_expression(expression)
                    .is_some()
                || self
                    .resolve_user_function_from_expression(expression)
                    .is_some()
        };

        if (actual_is_this && !expected_is_this)
            && has_static_reference_identity(&expected_materialized)
        {
            return Some(false);
        }

        if (expected_is_this && !actual_is_this)
            && has_static_reference_identity(&actual_materialized)
        {
            return Some(false);
        }

        let actual_symbol = self.resolve_symbol_identity_expression(&actual_materialized);
        let expected_symbol = self.resolve_symbol_identity_expression(&expected_materialized);

        if actual_symbol.is_some()
            && expected_symbol.is_none()
            && (expected_has_primitive
                || has_static_reference_identity(&expected_materialized)
                || expected_is_this)
        {
            return Some(false);
        }

        if expected_symbol.is_some()
            && actual_symbol.is_none()
            && (actual_has_primitive
                || has_static_reference_identity(&actual_materialized)
                || actual_is_this)
        {
            return Some(false);
        }

        if let (Some(actual_symbol), Some(expected_symbol)) = (actual_symbol, expected_symbol) {
            return Some(static_expression_matches(&actual_symbol, &expected_symbol));
        }

        if let (Some(actual_key), Some(expected_key)) = (
            self.resolve_static_reference_identity_key(&actual_materialized),
            self.resolve_static_reference_identity_key(&expected_materialized),
        ) {
            if actual_key != expected_key
                && (actual_key.contains("__ayy_scope$") || expected_key.contains("__ayy_scope$"))
            {
                return None;
            }
            return Some(actual_key == expected_key);
        }

        None
    }

    fn same_value_context_user_function(
        &self,
        current_function_name: Option<&str>,
    ) -> Option<&UserFunction> {
        current_function_name
            .and_then(|name| self.user_function(name))
            .or_else(|| self.current_user_function())
    }

    fn same_value_context_is_derived_constructor(
        &self,
        current_function_name: Option<&str>,
    ) -> bool {
        current_function_name
            .and_then(|name| self.user_function(name))
            .is_some_and(|function| self.user_function_is_derived_constructor(function))
            || (current_function_name.is_none() && self.current_function_is_derived_constructor())
    }

    fn same_value_expression_depends_on_dynamic_this(
        &self,
        expression: &Expression,
        current_function_name: Option<&str>,
    ) -> bool {
        let Some(user_function) = self.same_value_context_user_function(current_function_name)
        else {
            return false;
        };
        if user_function.lexical_this
            || self.same_value_context_is_derived_constructor(current_function_name)
        {
            return false;
        }
        self.expression_contains_dynamic_this_reference(expression, &mut HashSet::new())
    }

    fn expression_contains_dynamic_this_reference(
        &self,
        expression: &Expression,
        seen_aliases: &mut HashSet<String>,
    ) -> bool {
        match expression {
            Expression::This
            | Expression::SuperMember { .. }
            | Expression::AssignSuperMember { .. }
            | Expression::SuperCall { .. } => true,
            Expression::Identifier(name) => {
                if !seen_aliases.insert(name.clone()) {
                    return false;
                }
                self.resolve_bound_alias_expression(expression)
                    .filter(|resolved| !static_expression_matches(resolved, expression))
                    .is_some_and(|resolved| {
                        self.expression_contains_dynamic_this_reference(&resolved, seen_aliases)
                    })
            }
            Expression::Update { name, .. } => {
                if !seen_aliases.insert(name.clone()) {
                    return false;
                }
                let identifier = Expression::Identifier(name.clone());
                self.resolve_bound_alias_expression(&identifier)
                    .filter(|resolved| !static_expression_matches(resolved, &identifier))
                    .is_some_and(|resolved| {
                        self.expression_contains_dynamic_this_reference(&resolved, seen_aliases)
                    })
            }
            Expression::Member { object, property } => {
                self.expression_contains_dynamic_this_reference(object, seen_aliases)
                    || self.expression_contains_dynamic_this_reference(property, seen_aliases)
            }
            Expression::Assign { value, .. }
            | Expression::Await(value)
            | Expression::EnumerateKeys(value)
            | Expression::GetIterator(value)
            | Expression::IteratorClose(value)
            | Expression::Unary {
                expression: value, ..
            } => self.expression_contains_dynamic_this_reference(value, seen_aliases),
            Expression::AssignMember {
                object,
                property,
                value,
            } => {
                self.expression_contains_dynamic_this_reference(object, seen_aliases)
                    || self.expression_contains_dynamic_this_reference(property, seen_aliases)
                    || self.expression_contains_dynamic_this_reference(value, seen_aliases)
            }
            Expression::Binary { left, right, .. } => {
                self.expression_contains_dynamic_this_reference(left, seen_aliases)
                    || self.expression_contains_dynamic_this_reference(right, seen_aliases)
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                self.expression_contains_dynamic_this_reference(condition, seen_aliases)
                    || self
                        .expression_contains_dynamic_this_reference(then_expression, seen_aliases)
                    || self
                        .expression_contains_dynamic_this_reference(else_expression, seen_aliases)
            }
            Expression::Sequence(expressions) => expressions.iter().any(|expression| {
                self.expression_contains_dynamic_this_reference(expression, seen_aliases)
            }),
            Expression::Call { callee, arguments } | Expression::New { callee, arguments } => {
                self.expression_contains_dynamic_this_reference(callee, seen_aliases)
                    || arguments.iter().any(|argument| match argument {
                        CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                            self.expression_contains_dynamic_this_reference(
                                expression,
                                seen_aliases,
                            )
                        }
                    })
            }
            Expression::Array(elements) => elements.iter().any(|element| match element {
                ArrayElement::Expression(expression) | ArrayElement::Spread(expression) => {
                    self.expression_contains_dynamic_this_reference(expression, seen_aliases)
                }
            }),
            Expression::Object(entries) => entries.iter().any(|entry| match entry {
                ObjectEntry::Data { key, value } => {
                    self.expression_contains_dynamic_this_reference(key, seen_aliases)
                        || self.expression_contains_dynamic_this_reference(value, seen_aliases)
                }
                ObjectEntry::Getter { key, getter } => {
                    self.expression_contains_dynamic_this_reference(key, seen_aliases)
                        || self.expression_contains_dynamic_this_reference(getter, seen_aliases)
                }
                ObjectEntry::Setter { key, setter } => {
                    self.expression_contains_dynamic_this_reference(key, seen_aliases)
                        || self.expression_contains_dynamic_this_reference(setter, seen_aliases)
                }
                ObjectEntry::Spread(expression) => {
                    self.expression_contains_dynamic_this_reference(expression, seen_aliases)
                }
            }),
            Expression::Number(_)
            | Expression::BigInt(_)
            | Expression::String(_)
            | Expression::Bool(_)
            | Expression::Null
            | Expression::Undefined
            | Expression::NewTarget
            | Expression::Sent => false,
        }
    }

    pub(in crate::backend::direct_wasm) fn resolve_static_object_is_call_result(
        &self,
        expression: &Expression,
    ) -> Option<bool> {
        let Expression::Call { callee, arguments } = expression else {
            return None;
        };
        let Expression::Member { object, property } = callee.as_ref() else {
            return None;
        };
        if !matches!(object.as_ref(), Expression::Identifier(name) if name == "Object" && self.is_unshadowed_builtin_identifier(name))
        {
            return None;
        }
        if !matches!(property.as_ref(), Expression::String(name) if name == "is") {
            return None;
        }
        let [
            CallArgument::Expression(actual),
            CallArgument::Expression(expected),
            ..,
        ] = arguments.as_slice()
        else {
            return None;
        };
        self.resolve_static_same_value_result_with_context(
            actual,
            expected,
            self.current_function_name(),
        )
    }
}
