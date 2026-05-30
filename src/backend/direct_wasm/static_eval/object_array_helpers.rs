use super::*;

pub(in crate::backend::direct_wasm) fn infer_enumerated_keys_binding_from_expression(
    expression: &Expression,
    resolve_array_binding: impl Fn(&Expression) -> Option<ArrayValueBinding>,
    resolve_object_binding: impl Fn(&Expression) -> Option<ObjectValueBinding>,
) -> Option<ArrayValueBinding> {
    if let Some(array_binding) = resolve_array_binding(expression) {
        return Some(enumerated_keys_from_array_binding(&array_binding));
    }
    if let Some(object_binding) = resolve_object_binding(expression) {
        if object_binding_has_module_namespace_marker(&object_binding) {
            return Some(module_namespace_own_property_names_from_object_binding(
                &object_binding,
            ));
        }
        return Some(enumerated_keys_from_object_binding(&object_binding));
    }
    None
}

pub(in crate::backend::direct_wasm) fn infer_own_property_names_binding_from_expression(
    expression: &Expression,
    resolve_array_binding: impl Fn(&Expression) -> Option<ArrayValueBinding>,
    resolve_object_binding: impl Fn(&Expression) -> Option<ObjectValueBinding>,
    has_function_property_shape: impl Fn(&Expression) -> bool,
) -> Option<ArrayValueBinding> {
    if let Some(array_binding) = resolve_array_binding(expression) {
        return Some(own_property_names_from_array_binding(&array_binding));
    }
    let object_binding = resolve_object_binding(expression);
    if std::env::var_os("AYY_TRACE_OWN_PROPERTY_NAMES").is_some() {
        eprintln!(
            "own_property_names expression={expression:?} function_shape={} object_props={:?}",
            has_function_property_shape(expression),
            object_binding
                .as_ref()
                .map(ordered_object_property_names)
                .unwrap_or_default()
        );
    }
    if let Some(object_binding) = object_binding.as_ref()
        && object_binding_has_module_namespace_marker(object_binding)
    {
        return Some(module_namespace_own_property_names_from_object_binding(
            object_binding,
        ));
    }
    if has_function_property_shape(expression) {
        return Some(own_property_names_from_function_binding(
            object_binding.as_ref(),
        ));
    }
    if let Some(object_binding) = object_binding {
        return Some(own_property_names_from_object_binding(&object_binding));
    }
    None
}

fn object_binding_has_module_namespace_marker(object_binding: &ObjectValueBinding) -> bool {
    object_binding
        .string_properties
        .iter()
        .any(|(name, value)| {
            name == "__ayy$module$namespace" && matches!(value, Expression::Bool(true))
        })
}

fn module_namespace_own_property_names_from_object_binding(
    object_binding: &ObjectValueBinding,
) -> ArrayValueBinding {
    let mut names = ordered_object_property_names(object_binding)
        .into_iter()
        .filter(|name| !name.starts_with("__ayy$"))
        .collect::<Vec<_>>();
    names.sort();
    names.dedup();
    ArrayValueBinding {
        values: names
            .into_iter()
            .map(|name| Some(Expression::String(name)))
            .collect(),
    }
}

pub(in crate::backend::direct_wasm) fn infer_own_property_symbols_binding_from_expression(
    expression: &Expression,
    resolve_object_binding: impl Fn(&Expression) -> Option<ObjectValueBinding>,
) -> Option<ArrayValueBinding> {
    let object_binding = resolve_object_binding(expression)?;
    Some(own_property_symbols_from_object_binding(&object_binding))
}

pub(in crate::backend::direct_wasm) fn infer_builtin_object_array_call_binding(
    callee: &Expression,
    arguments: &[CallArgument],
    infer_enumerated_keys_binding: impl Fn(&Expression) -> Option<ArrayValueBinding>,
    infer_own_property_names_binding: impl Fn(&Expression) -> Option<ArrayValueBinding>,
    infer_own_property_symbols_binding: impl Fn(&Expression) -> Option<ArrayValueBinding>,
) -> Option<ArrayValueBinding> {
    let Expression::Member { object, property } = callee else {
        return None;
    };
    let Expression::Identifier(object_name) = object.as_ref() else {
        return None;
    };
    let [CallArgument::Expression(target), ..] = arguments else {
        return None;
    };
    match (object_name.as_str(), property.as_ref()) {
        ("Object", Expression::String(name)) if name == "keys" => {
            infer_enumerated_keys_binding(target)
        }
        ("Object", Expression::String(name)) if name == "getOwnPropertyNames" => {
            infer_own_property_names_binding(target)
        }
        ("Object", Expression::String(name)) if name == "getOwnPropertySymbols" => {
            infer_own_property_symbols_binding(target)
        }
        ("Reflect", Expression::String(name)) if name == "ownKeys" => {
            let mut names = infer_own_property_names_binding(target)?;
            if let Some(symbols) = infer_own_property_symbols_binding(target) {
                names.values.extend(symbols.values);
            }
            Some(names)
        }
        _ => None,
    }
}
