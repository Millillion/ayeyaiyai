use super::*;

pub(in crate::backend::direct_wasm) fn user_function_runtime_value(
    user_function: &UserFunction,
) -> i32 {
    let offset = user_function
        .function_index
        .saturating_sub(USER_FUNCTION_BASE_INDEX);
    debug_assert!(offset < JS_USER_FUNCTION_VALUE_LIMIT as u32);
    JS_USER_FUNCTION_VALUE_BASE + offset as i32
}

pub(in crate::backend::direct_wasm) fn internal_function_name_hint(
    function_name: &str,
) -> Option<&str> {
    function_name
        .rsplit_once("__name_")
        .map(|(_, hinted_name)| hinted_name)
}

fn internal_function_display_name_hint(function_name: &str) -> Option<String> {
    internal_function_name_hint(function_name)
        .map(|hinted_name| scoped_binding_source_name(hinted_name).unwrap_or(hinted_name))
        .map(str::to_string)
}

fn class_constructor_self_binding_display_name(function: &FunctionDeclaration) -> Option<String> {
    if !function.name.starts_with("__ayy_class_ctor_") {
        return None;
    }
    let self_binding = function.self_binding.as_deref()?;
    if self_binding.starts_with("__ayy_class_expr_") {
        return None;
    }
    Some(
        scoped_binding_source_name(self_binding)
            .unwrap_or(self_binding)
            .to_string(),
    )
}

fn generated_anonymous_class_constructor(function: &FunctionDeclaration) -> bool {
    function.name.starts_with("__ayy_class_ctor_")
        && function
            .self_binding
            .as_deref()
            .is_some_and(|self_binding| self_binding.starts_with("__ayy_class_expr_"))
}

pub(in crate::backend::direct_wasm) fn function_display_name(
    function: &FunctionDeclaration,
) -> Option<String> {
    class_constructor_self_binding_display_name(function)
        .or_else(|| {
            (!function.name.starts_with("__ayy_class_ctor_"))
                .then(|| function.self_binding.clone())
                .flatten()
        })
        .clone()
        .or_else(|| function.top_level_binding.clone())
        .or_else(|| {
            (!generated_anonymous_class_constructor(function))
                .then(|| internal_function_display_name_hint(&function.name))
                .flatten()
        })
        .or_else(|| (!function.name.starts_with("__ayy_")).then(|| function.name.clone()))
}

pub(in crate::backend::direct_wasm) fn builtin_function_display_name(name: &str) -> &str {
    match name {
        FUNCTION_CONSTRUCTOR_FAMILY_BUILTIN => "Function",
        _ => name.rsplit('.').next().unwrap_or(name),
    }
}

pub(in crate::backend::direct_wasm) fn builtin_function_length(name: &str) -> Option<u32> {
    match name {
        "Array" | "Boolean" | "Function" | "Number" | "Object" | "String" | "Symbol" | "BigInt"
        | "Date" | "Error" | "EvalError" | "RangeError" | "ReferenceError" | "SyntaxError"
        | "TypeError" | "URIError" | "AggregateError" => Some(1),
        "RegExp" => Some(2),
        "Math.abs" | "Math.acos" | "Math.asin" | "Math.atan" | "Math.ceil" | "Math.cos"
        | "Math.exp" | "Math.floor" | "Math.log" | "Math.round" | "Math.sin" | "Math.sqrt"
        | "Math.tan" => Some(1),
        "Math.random" => Some(0),
        "Math.atan2" | "Math.max" | "Math.min" | "Math.pow" => Some(2),
        _ => None,
    }
}
