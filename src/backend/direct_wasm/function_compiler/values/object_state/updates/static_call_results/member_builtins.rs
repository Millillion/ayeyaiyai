use super::*;

fn simple_regexp_pattern_is_plain_literal(pattern: &str) -> bool {
    !pattern.chars().any(|character| {
        matches!(
            character,
            '\\' | '.' | '^' | '$' | '*' | '+' | '?' | '(' | ')' | '[' | ']' | '{' | '}' | '|'
        )
    })
}

fn simple_regexp_pattern_matches(pattern: &str, subject: &str) -> Option<bool> {
    if simple_regexp_pattern_is_plain_literal(pattern) {
        return Some(subject.contains(pattern));
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
    pub(super) fn resolve_static_member_builtin_call_result_with_context(
        &self,
        callee: &Expression,
        arguments: &[CallArgument],
        current_function_name: Option<&str>,
    ) -> Option<(Expression, Option<String>)> {
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
            && matches!(object.as_ref(), Expression::Identifier(name) if name == "Object")
            && matches!(property.as_ref(), Expression::String(name) if name == "getPrototypeOf")
            && let [CallArgument::Expression(target), ..] = arguments
            && let Some(prototype) = self.resolve_static_object_prototype_expression(target)
        {
            return Some((prototype, None));
        }

        if let Expression::Member { object, property } = callee
            && matches!(object.as_ref(), Expression::Identifier(name) if name == "Object")
            && matches!(property.as_ref(), Expression::String(name) if name == "isExtensible")
            && let [CallArgument::Expression(target), ..] = arguments
        {
            return self
                .resolve_static_object_extensibility(target)
                .map(|extensible| (Expression::Bool(extensible), None));
        }

        if let Expression::Member { object, property } = callee
            && matches!(object.as_ref(), Expression::Identifier(name) if name == "Object")
            && matches!(property.as_ref(), Expression::String(name) if name == "preventExtensions")
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
        let resolved = self
            .resolve_bound_alias_expression(regexp_expression)
            .unwrap_or_else(|| self.materialize_static_expression(regexp_expression));
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
        if !flags.is_empty() {
            return None;
        }

        let subject =
            self.resolve_static_string_concat_value(subject_expression, current_function_name)?;
        simple_regexp_pattern_matches(&pattern, &subject)
    }
}
