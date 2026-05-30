use super::*;

fn simple_regexp_pattern_is_plain_literal(pattern: &str) -> bool {
    !pattern.chars().any(|character| {
        matches!(
            character,
            '\\' | '.' | '^' | '$' | '*' | '+' | '?' | '(' | ')' | '[' | ']' | '{' | '}' | '|'
        )
    })
}

fn simple_regexp_single_character_class_matches(pattern: &str, subject: &str) -> Option<bool> {
    let class = pattern.strip_prefix('[')?.strip_suffix(']')?;
    if class.is_empty() || class.starts_with('^') || class.contains('\\') {
        return None;
    }

    let class_chars = class.chars().collect::<Vec<_>>();
    let mut index = 0;
    let mut ranges = Vec::new();
    while index < class_chars.len() {
        let start = class_chars[index];
        if index + 2 < class_chars.len() && class_chars[index + 1] == '-' {
            let end = class_chars[index + 2];
            if start > end {
                return None;
            }
            ranges.push((start, end));
            index += 3;
        } else if start == '-' {
            return None;
        } else {
            ranges.push((start, start));
            index += 1;
        }
    }

    Some(subject.chars().any(|subject_char| {
        ranges
            .iter()
            .any(|(start, end)| *start <= subject_char && subject_char <= *end)
    }))
}

fn simple_regexp_pattern_matches(pattern: &str, subject: &str, ignore_case: bool) -> Option<bool> {
    let normalized_pattern;
    let normalized_subject;
    let (pattern, subject) = if ignore_case {
        normalized_pattern = pattern.to_lowercase();
        normalized_subject = subject.to_lowercase();
        (normalized_pattern.as_str(), normalized_subject.as_str())
    } else {
        (pattern, subject)
    };

    if simple_regexp_pattern_is_plain_literal(pattern) {
        return Some(subject.contains(pattern));
    }
    if let Some(matches) = simple_regexp_single_character_class_matches(pattern, subject) {
        return Some(matches);
    }
    if let Some(required_prefix) = pattern.strip_suffix('?') {
        if simple_regexp_pattern_is_plain_literal(required_prefix) {
            let shortened = required_prefix
                .char_indices()
                .last()
                .map(|(index, _)| &required_prefix[..index])
                .unwrap_or("");
            return Some(
                subject.contains(required_prefix)
                    || (!shortened.is_empty() && subject.contains(shortened)),
            );
        }
    }
    None
}

fn js_number_fraction_digits(value: Option<f64>) -> Option<usize> {
    let value = value.unwrap_or(0.0);
    if !value.is_finite() {
        return None;
    }
    let digits = value.trunc();
    if !(0.0..=100.0).contains(&digits) {
        return None;
    }
    Some(digits as usize)
}

fn format_js_number_to_fixed(number: f64, digits: usize) -> String {
    if number.is_nan() {
        return "NaN".to_string();
    }
    if number.is_infinite() {
        return if number.is_sign_negative() {
            "-Infinity".to_string()
        } else {
            "Infinity".to_string()
        };
    }
    format!("{number:.digits$}")
}

fn format_js_number_to_exponential(number: f64, digits: usize) -> String {
    if number.is_nan() {
        return "NaN".to_string();
    }
    if number.is_infinite() {
        return if number.is_sign_negative() {
            "-Infinity".to_string()
        } else {
            "Infinity".to_string()
        };
    }
    let formatted = format!("{number:.digits$e}");
    let Some((mantissa, exponent_text)) = formatted.split_once('e') else {
        return formatted;
    };
    let exponent = exponent_text.parse::<i32>().unwrap_or(0);
    format!("{mantissa}e{exponent:+}")
}

impl<'a> FunctionCompiler<'a> {
    fn static_promise_with_resolvers_result() -> Expression {
        let resolved_promise = Expression::Call {
            callee: Box::new(Expression::Member {
                object: Box::new(Expression::Identifier("Promise".to_string())),
                property: Box::new(Expression::String("resolve".to_string())),
            }),
            arguments: vec![CallArgument::Expression(Expression::Undefined)],
        };
        Expression::Object(vec![
            ObjectEntry::Data {
                key: Expression::String("promise".to_string()),
                value: resolved_promise,
            },
            ObjectEntry::Data {
                key: Expression::String("resolve".to_string()),
                value: Expression::Identifier("__ayy_promise_with_resolvers_resolve".to_string()),
            },
            ObjectEntry::Data {
                key: Expression::String("reject".to_string()),
                value: Expression::Identifier("__ayy_promise_with_resolvers_reject".to_string()),
            },
        ])
    }

    pub(in crate::backend::direct_wasm) fn resolve_static_member_builtin_call_result_with_context(
        &self,
        callee: &Expression,
        arguments: &[CallArgument],
        current_function_name: Option<&str>,
    ) -> Option<(Expression, Option<String>)> {
        if let Expression::Member { object, property } = callee
            && matches!(object.as_ref(), Expression::Identifier(name) if name == "Promise")
            && matches!(property.as_ref(), Expression::String(name) if name == "withResolvers")
        {
            return Some((Self::static_promise_with_resolvers_result(), None));
        }

        if let Expression::Member { object, property } = callee
            && matches!(object.as_ref(), Expression::Identifier(name) if name == "Proxy")
            && matches!(property.as_ref(), Expression::String(name) if name == "revocable")
            && let [
                CallArgument::Expression(target) | CallArgument::Spread(target),
                CallArgument::Expression(handler) | CallArgument::Spread(handler),
                ..,
            ] = arguments
        {
            return Some((
                Expression::Object(vec![
                    ObjectEntry::Data {
                        key: Expression::String("proxy".to_string()),
                        value: Expression::New {
                            callee: Box::new(Expression::Identifier("Proxy".to_string())),
                            arguments: vec![
                                CallArgument::Expression(target.clone()),
                                CallArgument::Expression(handler.clone()),
                            ],
                        },
                    },
                    ObjectEntry::Data {
                        key: Expression::String("revoke".to_string()),
                        value: Expression::Identifier("__ayy_proxy_revoke".to_string()),
                    },
                ]),
                None,
            ));
        }

        if let Expression::Member { object, property } = callee
            && matches!(property.as_ref(), Expression::String(name) if name == "toString")
            && arguments.is_empty()
            && let Some(text) = self
                .resolve_static_symbol_to_string_value_with_context(object, current_function_name)
        {
            return Some((Expression::String(text), None));
        }

        if let Expression::Member { object, property } = callee
            && matches!(property.as_ref(), Expression::String(name) if name == "replace")
            && let [
                CallArgument::Expression(search_expression),
                CallArgument::Expression(replacement_expression),
            ] = arguments
            && let Some(text) = self.resolve_static_string_replace_result_with_context(
                object,
                search_expression,
                replacement_expression,
                current_function_name,
            )
        {
            return Some((Expression::String(text), None));
        }

        if let Expression::Member { object, property } = callee
            && let Expression::String(property_name) = property.as_ref()
            && matches!(property_name.as_str(), "toFixed" | "toExponential")
            && let Some(Expression::Number(number)) = self
                .resolve_static_boxed_primitive_value(object)
                .or_else(|| {
                    self.resolve_static_number_value(object)
                        .map(Expression::Number)
                })
        {
            let expanded_arguments = self.expand_call_arguments(arguments);
            let digits = js_number_fraction_digits(
                expanded_arguments
                    .first()
                    .and_then(|argument| self.resolve_static_number_value(argument)),
            )?;
            let value = match property_name.as_str() {
                "toFixed" => format_js_number_to_fixed(number, digits),
                "toExponential" => format_js_number_to_exponential(number, digits),
                _ => return None,
            };
            return Some((Expression::String(value), None));
        }

        if let Expression::Member { object, property } = callee
            && matches!(property.as_ref(), Expression::String(property_name) if property_name == "charAt")
        {
            let string_value = self
                .resolve_static_boxed_primitive_value(object)
                .and_then(|value| match value {
                    Expression::String(text) => Some(text),
                    _ => None,
                })
                .or_else(|| {
                    self.resolve_static_string_value_with_context(object, current_function_name)
                })?;
            let expanded_arguments = self.expand_call_arguments(arguments);
            let index_value = expanded_arguments
                .first()
                .and_then(|argument| self.resolve_static_number_value(argument))
                .unwrap_or(0.0);
            let index = if index_value.is_nan() {
                0.0
            } else {
                index_value.trunc()
            };
            let text = if index < 0.0 || !index.is_finite() {
                String::new()
            } else {
                string_value
                    .chars()
                    .nth(index as usize)
                    .map(|character| character.to_string())
                    .unwrap_or_default()
            };
            return Some((Expression::String(text), None));
        }

        if let Expression::Member { object, property } = callee
            && matches!(property.as_ref(), Expression::String(property_name) if property_name == "trim")
            && arguments.is_empty()
        {
            let string_value = self
                .resolve_static_boxed_primitive_value(object)
                .and_then(|value| match value {
                    Expression::String(text) => Some(text),
                    _ => None,
                })
                .or_else(|| {
                    self.resolve_static_string_value_with_context(object, current_function_name)
                });
            if let Some(text) = string_value {
                return Some((Expression::String(text.trim().to_string()), None));
            }
        }

        if let Expression::Member { object, property } = callee
            && let Expression::String(property_name) = property.as_ref()
            && matches!(property_name.as_str(), "get" | "has")
            && let [
                CallArgument::Expression(key) | CallArgument::Spread(key),
                ..,
            ] = arguments
            && let Some(object_binding) = self.resolve_object_binding_from_expression(object)
            && self.object_binding_is_static_weak_collection_kind(&object_binding, "WeakMap")
        {
            let value = self.static_weak_collection_entry_value(&object_binding, key);
            return match property_name.as_str() {
                "has" => Some((Expression::Bool(value.is_some()), None)),
                "get" => Some((value.unwrap_or(Expression::Undefined), None)),
                _ => None,
            };
        }

        if let Expression::Member { object, property } = callee
            && matches!(property.as_ref(), Expression::String(property_name) if property_name == "has")
            && let [
                CallArgument::Expression(key) | CallArgument::Spread(key),
                ..,
            ] = arguments
            && let Some(object_binding) = self.resolve_object_binding_from_expression(object)
            && self.object_binding_is_static_weak_collection_kind(&object_binding, "WeakSet")
        {
            return Some((
                Expression::Bool(
                    self.static_weak_collection_entry_value(&object_binding, key)
                        .is_some(),
                ),
                None,
            ));
        }

        if let Expression::Member { object, property } = callee
            && let Expression::String(property_name) = property.as_ref()
            && matches!(property_name.as_str(), "exec" | "test")
            && let [CallArgument::Expression(subject) | CallArgument::Spread(subject)] = arguments
            && let Some(matches) = self.resolve_static_simple_regexp_match_result(
                object,
                subject,
                current_function_name,
            )
        {
            return match property_name.as_str() {
                "test" => Some((Expression::Bool(matches), None)),
                "exec" if matches => Some((Expression::Object(Vec::new()), None)),
                "exec" => Some((Expression::Null, None)),
                _ => None,
            };
        }

        if let Expression::Member { object, property } = callee
            && matches!(object.as_ref(), Expression::Identifier(name) if name == "Object" || name == "Reflect")
            && matches!(property.as_ref(), Expression::String(name) if name == "getOwnPropertyDescriptor")
            && let [
                CallArgument::Expression(target),
                CallArgument::Expression(property_name),
                ..,
            ] = arguments
        {
            let resolved_property = self
                .resolve_property_key_expression(property_name)
                .unwrap_or_else(|| self.materialize_static_expression(property_name));
            if static_property_name_from_expression(&resolved_property).is_some()
                && self
                    .resolve_call_descriptor_binding(callee, arguments)
                    .is_none()
                && (self
                    .resolve_object_binding_from_expression(target)
                    .is_some()
                    || matches!(
                        target,
                        Expression::Identifier(name)
                            if FunctionCompiler::module_index_from_namespace_like_identifier(name)
                                .is_some()
                    ))
            {
                return Some((Expression::Undefined, None));
            }
        }

        if let Expression::Member { object, property } = callee
            && matches!(object.as_ref(), Expression::Identifier(name) if name == "Object" || name == "Reflect")
            && matches!(property.as_ref(), Expression::String(name) if name == "getPrototypeOf")
            && let [CallArgument::Expression(target), ..] = arguments
            && let Some(prototype) = self.resolve_static_object_prototype_expression(target)
        {
            return Some((prototype, None));
        }

        if let Expression::Member { object, property } = callee
            && matches!(object.as_ref(), Expression::Identifier(name) if name == "Object" || name == "Reflect")
            && matches!(property.as_ref(), Expression::String(name) if name == "isExtensible")
            && let [CallArgument::Expression(target), ..] = arguments
        {
            return self
                .resolve_static_object_extensibility(target)
                .map(|extensible| (Expression::Bool(extensible), None));
        }

        if let Expression::Member { object, property } = callee
            && matches!(object.as_ref(), Expression::Identifier(name) if name == "Object")
            && matches!(property.as_ref(), Expression::String(name) if name == "isSealed" || name == "isFrozen")
            && let [CallArgument::Expression(target), ..] = arguments
        {
            let target_is_module_namespace = self
                .resolve_object_binding_from_expression(target)
                .as_ref()
                .is_some_and(Self::object_binding_has_module_namespace_marker)
                || matches!(
                    target,
                    Expression::Identifier(name)
                        if FunctionCompiler::module_index_from_namespace_like_identifier(name)
                            .is_some()
                );
            if matches!(property.as_ref(), Expression::String(name) if name == "isFrozen")
                && target_is_module_namespace
            {
                return Some((Expression::Bool(false), None));
            }
            return self
                .resolve_static_object_extensibility(target)
                .map(|extensible| (Expression::Bool(!extensible), None));
        }

        if let Expression::Member { object, property } = callee
            && matches!(object.as_ref(), Expression::Identifier(name) if name == "Object")
            && matches!(property.as_ref(), Expression::String(name) if name == "preventExtensions")
            && let [CallArgument::Expression(target), ..] = arguments
        {
            return Some((target.clone(), None));
        }

        if let Expression::Member { object, property } = callee
            && matches!(object.as_ref(), Expression::Identifier(name) if name == "Object")
            && matches!(property.as_ref(), Expression::String(name) if name == "freeze" || name == "seal")
            && let [CallArgument::Expression(target), ..] = arguments
        {
            return Some((target.clone(), None));
        }

        if let Expression::Member { object, property } = callee
            && matches!(object.as_ref(), Expression::Identifier(name) if name == "Reflect")
            && matches!(property.as_ref(), Expression::String(name) if name == "has")
            && let [CallArgument::Expression(target), ..] = arguments
        {
            let property_expression = match arguments.get(1) {
                Some(CallArgument::Expression(property) | CallArgument::Spread(property)) => {
                    property.clone()
                }
                None => Expression::Undefined,
            };
            if let Some(has_property) =
                self.resolve_static_reflect_has_result(target, &property_expression)
            {
                return Some((Expression::Bool(has_property), None));
            }
        }

        if let Expression::Member { object, property } = callee
            && matches!(object.as_ref(), Expression::Identifier(name) if name == "Reflect")
            && matches!(property.as_ref(), Expression::String(name) if name == "preventExtensions")
            && let [CallArgument::Expression(_target), ..] = arguments
        {
            return Some((Expression::Bool(true), None));
        }

        if let Expression::Member { object, property } = callee
            && matches!(object.as_ref(), Expression::Identifier(name) if name == "Reflect")
            && matches!(property.as_ref(), Expression::String(name) if name == "setPrototypeOf")
            && let [
                CallArgument::Expression(target),
                CallArgument::Expression(prototype),
                ..,
            ] = arguments
        {
            if self
                .resolve_object_binding_from_expression(target)
                .as_ref()
                .is_some_and(Self::object_binding_has_module_namespace_marker)
                || matches!(
                    target,
                    Expression::Identifier(name)
                        if FunctionCompiler::module_index_from_namespace_like_identifier(name)
                            .is_some()
                )
            {
                return Some((
                    Expression::Bool(matches!(prototype, Expression::Null)),
                    None,
                ));
            }
        }

        None
    }

    pub(in crate::backend::direct_wasm) fn resolve_static_simple_regexp_exec_no_match(
        &self,
        regexp_expression: &Expression,
        subject_expression: &Expression,
        current_function_name: Option<&str>,
    ) -> Option<bool> {
        self.resolve_static_simple_regexp_match_result(
            regexp_expression,
            subject_expression,
            current_function_name,
        )
        .map(|matches| !matches)
    }

    pub(in crate::backend::direct_wasm) fn resolve_static_simple_regexp_match_result(
        &self,
        regexp_expression: &Expression,
        subject_expression: &Expression,
        current_function_name: Option<&str>,
    ) -> Option<bool> {
        let resolved_alias = self
            .resolve_bound_alias_expression(regexp_expression)
            .filter(|resolved| !static_expression_matches(resolved, regexp_expression));
        let materialized = self.materialize_static_expression(regexp_expression);
        let resolved = [Some(regexp_expression), resolved_alias.as_ref(), Some(&materialized)]
            .into_iter()
            .flatten()
            .find(|candidate| {
                matches!(
                    candidate,
                    Expression::Call { callee, .. } | Expression::New { callee, .. }
                        if matches!(callee.as_ref(), Expression::Identifier(name) if name == "RegExp" && self.is_unshadowed_builtin_identifier(name))
                )
            })?;
        let (callee, arguments) = match resolved {
            Expression::Call { callee, arguments } | Expression::New { callee, arguments } => {
                (callee, arguments)
            }
            _ => return None,
        };
        let Expression::Identifier(name) = callee.as_ref() else {
            return None;
        };
        if name != "RegExp" || !self.is_unshadowed_builtin_identifier(name) {
            return None;
        }

        let pattern = match arguments.first() {
            Some(CallArgument::Expression(argument) | CallArgument::Spread(argument)) => {
                self.resolve_static_string_concat_value(argument, current_function_name)?
            }
            None => String::new(),
        };
        let flags = match arguments.get(1) {
            Some(CallArgument::Expression(argument) | CallArgument::Spread(argument)) => {
                self.resolve_static_string_concat_value(argument, current_function_name)?
            }
            None => String::new(),
        };
        let ignore_case = match flags.as_str() {
            "" => false,
            "i" => true,
            _ => return None,
        };

        if flags.contains('g') || flags.contains('y') {
            return None;
        }

        let subject =
            self.resolve_static_string_concat_value(subject_expression, current_function_name)?;
        simple_regexp_pattern_matches(&pattern, &subject, ignore_case)
    }
}
