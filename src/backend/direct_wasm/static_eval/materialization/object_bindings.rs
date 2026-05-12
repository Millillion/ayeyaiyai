use super::*;

pub(in crate::backend::direct_wasm) fn copy_enumerable_object_binding_properties(
    source_binding: &ObjectValueBinding,
    mut resolve_property_value: impl FnMut(&Expression) -> Option<Expression>,
) -> Option<ObjectValueBinding> {
    let mut copied_binding = empty_object_value_binding();
    for name in ordered_object_property_names(source_binding) {
        if source_binding
            .non_enumerable_string_properties
            .iter()
            .any(|hidden_name| hidden_name == &name)
        {
            continue;
        }
        let property = Expression::String(name.clone());
        let copied_value = resolve_property_value(&property)?;
        object_binding_set_property(&mut copied_binding, property, copied_value);
    }
    for (property, _) in &source_binding.symbol_properties {
        let copied_value = resolve_property_value(property)?;
        object_binding_set_property(&mut copied_binding, property.clone(), copied_value);
    }
    Some(copied_binding)
}

pub(in crate::backend::direct_wasm) fn resolve_copy_data_properties_binding<Context>(
    expression: &Expression,
    context: &mut Context,
    mut resolve_object_binding: impl FnMut(&Expression, &mut Context) -> Option<ObjectValueBinding>,
    mut resolve_member_getter_value: impl FnMut(
        &Expression,
        &Expression,
        &mut Context,
    ) -> Option<Expression>,
) -> Option<ObjectValueBinding> {
    if let Some(binding) = primitive_copy_data_properties_binding(expression) {
        return Some(binding);
    }
    let source_binding = resolve_object_binding(expression, context)?;
    copy_enumerable_object_binding_properties(&source_binding, |property| {
        if let Some(descriptor) = object_binding_lookup_descriptor(&source_binding, property) {
            let is_accessor = descriptor.has_get
                || descriptor.has_set
                || descriptor.getter.is_some()
                || descriptor.setter.is_some();
            if is_accessor {
                if descriptor.getter.is_none() {
                    return Some(Expression::Undefined);
                }
                return resolve_member_getter_value(expression, property, context)
                    .or(Some(Expression::Undefined));
            }
        }
        object_binding_lookup_value(&source_binding, property).cloned()
    })
}

fn primitive_copy_data_properties_binding(expression: &Expression) -> Option<ObjectValueBinding> {
    match expression {
        Expression::String(text) => Some(string_primitive_copy_data_properties_binding(text)),
        Expression::Number(_)
        | Expression::Bool(_)
        | Expression::BigInt(_)
        | Expression::Null
        | Expression::Undefined => Some(empty_object_value_binding()),
        Expression::Call { callee, .. } if matches!(callee.as_ref(), Expression::Identifier(name) if name == "Symbol") => {
            Some(empty_object_value_binding())
        }
        _ => None,
    }
}

fn string_primitive_copy_data_properties_binding(text: &str) -> ObjectValueBinding {
    let mut binding = empty_object_value_binding();
    for (index, character) in text.chars().enumerate() {
        object_binding_set_property(
            &mut binding,
            Expression::String(index.to_string()),
            Expression::String(character.to_string()),
        );
    }
    binding
}

pub(in crate::backend::direct_wasm) fn resolve_structural_object_binding<Context>(
    entries: &[ObjectEntry],
    context: &mut Context,
    mut materialize_expression: impl FnMut(&Expression, &mut Context) -> Option<Expression>,
    preserve_spread_expression: impl Fn(&Expression, &Context) -> bool,
    skip_spread_expression: impl Fn(&Expression, &Context) -> bool,
    mut resolve_copy_data_properties: impl FnMut(
        &Expression,
        &mut Context,
    ) -> Option<ObjectValueBinding>,
) -> Option<ObjectValueBinding> {
    resolve_structural_object_binding_dyn(
        entries,
        context,
        &mut materialize_expression,
        &preserve_spread_expression,
        &skip_spread_expression,
        &mut resolve_copy_data_properties,
    )
}

fn resolve_structural_object_binding_dyn<Context>(
    entries: &[ObjectEntry],
    context: &mut Context,
    materialize_expression: &mut dyn FnMut(&Expression, &mut Context) -> Option<Expression>,
    preserve_spread_expression: &dyn Fn(&Expression, &Context) -> bool,
    skip_spread_expression: &dyn Fn(&Expression, &Context) -> bool,
    resolve_copy_data_properties: &mut dyn FnMut(
        &Expression,
        &mut Context,
    ) -> Option<ObjectValueBinding>,
) -> Option<ObjectValueBinding> {
    let mut object_binding = empty_object_value_binding();
    for entry in entries {
        match entry {
            ObjectEntry::Data { key, value } => {
                let key = materialize_expression(key, context)?;
                let mut value = materialize_expression(value, context)?;
                if let Expression::Object(entries) = &value
                    && entries
                        .iter()
                        .any(|entry| matches!(entry, ObjectEntry::Spread(_)))
                    && let Some(nested_binding) = resolve_structural_object_binding_dyn(
                        entries,
                        context,
                        materialize_expression,
                        preserve_spread_expression,
                        skip_spread_expression,
                        resolve_copy_data_properties,
                    )
                {
                    value = object_binding_to_expression(&nested_binding);
                }
                object_binding_define_property_descriptor(
                    &mut object_binding,
                    key,
                    PropertyDescriptorBinding {
                        value: Some(value),
                        configurable: true,
                        enumerable: true,
                        writable: Some(true),
                        getter: None,
                        setter: None,
                        has_get: false,
                        has_set: false,
                    },
                );
            }
            ObjectEntry::Getter { key, getter } => {
                let key = materialize_expression(key, context)?;
                let existing = object_binding_lookup_descriptor(&object_binding, &key).cloned();
                object_binding_define_property_descriptor(
                    &mut object_binding,
                    key,
                    PropertyDescriptorBinding {
                        value: None,
                        configurable: true,
                        enumerable: true,
                        writable: None,
                        getter: Some(materialize_expression(getter, context)?),
                        setter: existing
                            .as_ref()
                            .and_then(|descriptor| descriptor.setter.clone()),
                        has_get: true,
                        has_set: existing
                            .as_ref()
                            .is_some_and(|descriptor| descriptor.has_set),
                    },
                );
            }
            ObjectEntry::Setter { key, setter } => {
                let key = materialize_expression(key, context)?;
                let existing = object_binding_lookup_descriptor(&object_binding, &key).cloned();
                object_binding_define_property_descriptor(
                    &mut object_binding,
                    key,
                    PropertyDescriptorBinding {
                        value: None,
                        configurable: true,
                        enumerable: true,
                        writable: None,
                        getter: existing
                            .as_ref()
                            .and_then(|descriptor| descriptor.getter.clone()),
                        setter: Some(materialize_expression(setter, context)?),
                        has_get: existing
                            .as_ref()
                            .is_some_and(|descriptor| descriptor.has_get),
                        has_set: true,
                    },
                );
            }
            ObjectEntry::Spread(expression) => {
                let spread_expression = if preserve_spread_expression(expression, context) {
                    expression.clone()
                } else {
                    materialize_expression(expression, context)?
                };
                if matches!(spread_expression, Expression::Null | Expression::Undefined)
                    || skip_spread_expression(&spread_expression, context)
                {
                    continue;
                }
                let spread_binding = resolve_copy_data_properties(&spread_expression, context)?;
                merge_enumerable_object_binding(&mut object_binding, &spread_binding);
            }
        }
    }
    Some(object_binding)
}

pub(in crate::backend::direct_wasm) fn resolve_structural_object_binding_in_environment<
    Executor,
    Environment,
    MaterializeExpression,
    PreserveSpreadExpression,
    ResolveObjectBinding,
    ResolveMemberGetterValue,
>(
    executor: &Executor,
    entries: &[ObjectEntry],
    environment: &mut Environment,
    materialize_expression: &MaterializeExpression,
    preserve_spread_expression: &PreserveSpreadExpression,
    resolve_object_binding: &ResolveObjectBinding,
    resolve_member_getter_value: &ResolveMemberGetterValue,
) -> Option<ObjectValueBinding>
where
    Executor: StaticIdentifierMaterializer + ?Sized,
    MaterializeExpression: Fn(&Expression, &mut Environment) -> Option<Expression>,
    PreserveSpreadExpression: Fn(&Expression, &Environment) -> bool,
    ResolveObjectBinding: Fn(&Expression, &mut Environment) -> Option<ObjectValueBinding>,
    ResolveMemberGetterValue: Fn(&Expression, &Expression, &mut Environment) -> Option<Expression>,
{
    resolve_structural_object_binding(
        entries,
        environment,
        |expression, environment| materialize_expression(expression, environment),
        |spread_expression, environment| preserve_spread_expression(spread_expression, environment),
        |spread_expression, _| {
            matches!(
                spread_expression,
                Expression::Identifier(name)
                    if name == "undefined"
                        && executor.is_unshadowed_builtin_identifier(name)
            )
        },
        |spread_expression, environment| {
            resolve_copy_data_properties_binding(
                spread_expression,
                environment,
                |expression, environment| resolve_object_binding(expression, environment),
                |object, property, environment| {
                    resolve_member_getter_value(object, property, environment)
                },
            )
        },
    )
}

pub(in crate::backend::direct_wasm) fn resolve_specialized_object_binding_expression<Context>(
    expression: &Expression,
    context: &mut Context,
    mut resolve_array_binding: impl FnMut(&Expression, &mut Context) -> Option<ArrayValueBinding>,
    mut resolve_object_entries: impl FnMut(&[ObjectEntry], &mut Context) -> Option<ObjectValueBinding>,
    is_object_create_call: impl Fn(&Expression, &mut Context) -> bool,
    mut resolve_fallback: impl FnMut(&Expression, &mut Context) -> Option<ObjectValueBinding>,
) -> Option<ObjectValueBinding> {
    match expression {
        Expression::Sequence(expressions) => {
            let last = expressions.last()?;
            resolve_specialized_object_binding_expression(
                last,
                context,
                resolve_array_binding,
                resolve_object_entries,
                is_object_create_call,
                resolve_fallback,
            )
        }
        Expression::Array(_) => resolve_array_binding(expression, context)
            .map(|binding| object_binding_from_array_binding(&binding)),
        Expression::Object(entries) => resolve_object_entries(entries, context),
        Expression::Call { .. } if is_object_create_call(expression, context) => {
            Some(empty_object_value_binding())
        }
        _ => resolve_fallback(expression, context),
    }
}
