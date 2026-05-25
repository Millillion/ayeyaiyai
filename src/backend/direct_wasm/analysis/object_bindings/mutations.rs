use super::*;

pub(in crate::backend::direct_wasm) fn object_binding_lookup_value<'a>(
    object_binding: &'a ObjectValueBinding,
    property: &Expression,
) -> Option<&'a Expression> {
    if let Some(property_name) = static_property_name_from_expression(property) {
        return object_binding
            .string_properties
            .iter()
            .find(|(existing_name, _)| *existing_name == property_name)
            .map(|(_, value)| value);
    }
    object_binding
        .symbol_properties
        .iter()
        .find(|(existing_key, _)| existing_key == property)
        .map(|(_, value)| value)
}

pub(in crate::backend::direct_wasm) fn object_binding_lookup_descriptor<'a>(
    object_binding: &'a ObjectValueBinding,
    property: &Expression,
) -> Option<&'a PropertyDescriptorBinding> {
    let canonical_property = static_property_name_from_expression(property)
        .map(Expression::String)
        .unwrap_or_else(|| property.clone());
    object_binding
        .property_descriptors
        .iter()
        .find(|(existing_property, _)| *existing_property == canonical_property)
        .map(|(_, descriptor)| descriptor)
}

pub(in crate::backend::direct_wasm) fn object_binding_has_property(
    object_binding: &ObjectValueBinding,
    property: &Expression,
) -> bool {
    object_binding_lookup_value(object_binding, property).is_some()
}

pub(in crate::backend::direct_wasm) fn object_binding_is_extensible(
    object_binding: &ObjectValueBinding,
) -> bool {
    object_binding.extensible
}

pub(in crate::backend::direct_wasm) fn object_binding_prevent_extensions(
    object_binding: &mut ObjectValueBinding,
) {
    object_binding.extensible = false;
}

pub(in crate::backend::direct_wasm) fn object_binding_freeze(
    object_binding: &mut ObjectValueBinding,
) {
    object_binding.extensible = false;
    for (_, descriptor) in &mut object_binding.property_descriptors {
        descriptor.configurable = false;
        if descriptor.writable.is_some() {
            descriptor.writable = Some(false);
        }
    }

    for (property_name, value) in object_binding.string_properties.clone() {
        let property = Expression::String(property_name.clone());
        if object_binding_lookup_descriptor(object_binding, &property).is_some() {
            continue;
        }
        let enumerable = !object_binding
            .non_enumerable_string_properties
            .iter()
            .any(|hidden_name| hidden_name == &property_name);
        object_binding.property_descriptors.push((
            property,
            PropertyDescriptorBinding {
                value: Some(value),
                configurable: false,
                enumerable,
                writable: Some(false),
                getter: None,
                setter: None,
                has_get: false,
                has_set: false,
            },
        ));
    }

    for (property, value) in object_binding.symbol_properties.clone() {
        if object_binding_lookup_descriptor(object_binding, &property).is_some() {
            continue;
        }
        object_binding.property_descriptors.push((
            property,
            PropertyDescriptorBinding {
                value: Some(value),
                configurable: false,
                enumerable: true,
                writable: Some(false),
                getter: None,
                setter: None,
                has_get: false,
                has_set: false,
            },
        ));
    }
}

pub(in crate::backend::direct_wasm) fn object_binding_can_define_property(
    object_binding: &ObjectValueBinding,
    property: &Expression,
) -> bool {
    object_binding_is_extensible(object_binding)
        || object_binding_has_property(object_binding, property)
}

pub(in crate::backend::direct_wasm) fn object_binding_set_property(
    object_binding: &mut ObjectValueBinding,
    property: Expression,
    value: Expression,
) {
    if let Some(property_name) = static_property_name_from_expression(&property) {
        object_binding
            .property_descriptors
            .retain(|(existing_property, _)| {
                !matches!(existing_property, Expression::String(name) if name == &property_name)
            });
        if let Some((_, existing_value)) = object_binding
            .string_properties
            .iter_mut()
            .find(|(existing_name, _)| *existing_name == property_name)
        {
            *existing_value = value;
        } else {
            object_binding
                .string_properties
                .push((property_name.clone(), value));
        }
        object_binding_set_string_property_enumerable(object_binding, &property_name, true);
        return;
    }

    object_binding
        .property_descriptors
        .retain(|(existing_property, _)| existing_property != &property);
    if let Some((_, existing_value)) = object_binding
        .symbol_properties
        .iter_mut()
        .find(|(existing_key, _)| *existing_key == property)
    {
        *existing_value = value;
    } else {
        object_binding.symbol_properties.push((property, value));
    }
}

pub(in crate::backend::direct_wasm) fn object_binding_define_property(
    object_binding: &mut ObjectValueBinding,
    property: Expression,
    value: Expression,
    enumerable: bool,
) {
    if let Some(property_name) = static_property_name_from_expression(&property) {
        if let Some((_, existing_value)) = object_binding
            .string_properties
            .iter_mut()
            .find(|(existing_name, _)| *existing_name == property_name)
        {
            *existing_value = value;
        } else {
            object_binding
                .string_properties
                .push((property_name.clone(), value));
        }
        object_binding_set_string_property_enumerable(object_binding, &property_name, enumerable);
        return;
    }

    if let Some((_, existing_value)) = object_binding
        .symbol_properties
        .iter_mut()
        .find(|(existing_key, _)| *existing_key == property)
    {
        *existing_value = value;
    } else {
        object_binding.symbol_properties.push((property, value));
    }
}

pub(in crate::backend::direct_wasm) fn object_binding_define_property_descriptor(
    object_binding: &mut ObjectValueBinding,
    property: Expression,
    descriptor: PropertyDescriptorBinding,
) {
    let value = descriptor.value.clone().unwrap_or(Expression::Undefined);
    object_binding_define_property(
        object_binding,
        property.clone(),
        value,
        descriptor.enumerable,
    );
    let canonical_property = static_property_name_from_expression(&property)
        .map(Expression::String)
        .unwrap_or(property);
    if let Some((_, existing_descriptor)) = object_binding
        .property_descriptors
        .iter_mut()
        .find(|(existing_property, _)| *existing_property == canonical_property)
    {
        *existing_descriptor = descriptor;
    } else {
        object_binding
            .property_descriptors
            .push((canonical_property, descriptor));
    }
}

pub(in crate::backend::direct_wasm) fn object_binding_define_copied_data_property(
    object_binding: &mut ObjectValueBinding,
    property: Expression,
    value: Expression,
) {
    object_binding_define_property_descriptor(
        object_binding,
        property,
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

pub(in crate::backend::direct_wasm) fn object_binding_remove_property(
    object_binding: &mut ObjectValueBinding,
    property: &Expression,
) -> bool {
    if let Some(property_name) = static_property_name_from_expression(property) {
        let len_before = object_binding.string_properties.len();
        object_binding
            .string_properties
            .retain(|(existing_name, _)| *existing_name != property_name);
        object_binding
            .non_enumerable_string_properties
            .retain(|hidden_name| hidden_name != &property_name);
        object_binding
            .property_descriptors
            .retain(|(existing_property, _)| {
                !matches!(existing_property, Expression::String(name) if name == &property_name)
            });
        return object_binding.string_properties.len() != len_before;
    }

    let len_before = object_binding.symbol_properties.len();
    object_binding
        .symbol_properties
        .retain(|(existing_key, _)| existing_key != property);
    object_binding
        .property_descriptors
        .retain(|(existing_property, _)| existing_property != property);
    object_binding.symbol_properties.len() != len_before
}

pub(in crate::backend::direct_wasm) fn merge_enumerable_object_binding(
    target: &mut ObjectValueBinding,
    source: &ObjectValueBinding,
) {
    for (name, value) in &source.string_properties {
        let property = Expression::String(name.clone());
        if source
            .non_enumerable_string_properties
            .iter()
            .any(|hidden_name| hidden_name == name)
            || object_binding_lookup_descriptor(source, &property)
                .is_some_and(|descriptor| !descriptor.enumerable)
        {
            continue;
        }
        object_binding_define_copied_data_property(target, property, value.clone());
    }
    for (property, value) in &source.symbol_properties {
        if object_binding_lookup_descriptor(source, property)
            .is_some_and(|descriptor| !descriptor.enumerable)
        {
            continue;
        }
        object_binding_define_copied_data_property(target, property.clone(), value.clone());
    }
}
