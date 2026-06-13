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
        .or_else(|| {
            well_known_symbol_property_name(property).and_then(|property_name| {
                object_binding
                    .string_properties
                    .iter()
                    .find(|(existing_name, _)| *existing_name == property_name)
                    .map(|(_, value)| value)
            })
        })
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
        || object_binding_lookup_descriptor(object_binding, property).is_some()
}

pub(in crate::backend::direct_wasm) fn object_binding_is_static_property_key(
    property: &Expression,
) -> bool {
    static_property_name_from_expression(property).is_some()
        || well_known_symbol_property_name(property).is_some()
}

pub(in crate::backend::direct_wasm) fn object_binding_is_extensible(
    object_binding: &ObjectValueBinding,
) -> bool {
    object_binding.extensible
}

fn well_known_symbol_property_name(property: &Expression) -> Option<String> {
    let Expression::Member { object, property } = property else {
        return None;
    };
    if !matches!(object.as_ref(), Expression::Identifier(name) if name == "Symbol") {
        return None;
    }
    let Expression::String(name) = property.as_ref() else {
        return None;
    };
    Some(format!("Symbol.{name}"))
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
    mut descriptor: PropertyDescriptorBinding,
) {
    let canonical_property = static_property_name_from_expression(&property)
        .map(Expression::String)
        .unwrap_or_else(|| property.clone());
    // Per ValidateAndApplyPropertyDescriptor, fields absent from the incoming
    // descriptor retain the current attribute values: a `{set}` redefinition
    // keeps an existing getter, while a data redefinition drops accessors.
    if let Some((_, existing)) = object_binding
        .property_descriptors
        .iter()
        .find(|(existing_property, _)| *existing_property == canonical_property)
    {
        let introduces_accessor = descriptor.has_get
            || descriptor.has_set
            || descriptor.getter.is_some()
            || descriptor.setter.is_some();
        let introduces_data = descriptor.value.is_some() || descriptor.writable.is_some();
        if introduces_accessor && !introduces_data {
            if !descriptor.has_get && descriptor.getter.is_none() {
                descriptor.getter = existing.getter.clone();
                descriptor.has_get = existing.has_get;
            }
            if !descriptor.has_set && descriptor.setter.is_none() {
                descriptor.setter = existing.setter.clone();
                descriptor.has_set = existing.has_set;
            }
        } else if !introduces_accessor && !introduces_data {
            descriptor.value = existing.value.clone();
            descriptor.writable = existing.writable;
            descriptor.getter = existing.getter.clone();
            descriptor.setter = existing.setter.clone();
            descriptor.has_get = existing.has_get;
            descriptor.has_set = existing.has_set;
        }
    }
    let value = descriptor.value.clone().unwrap_or(Expression::Undefined);
    object_binding_define_property(
        object_binding,
        property.clone(),
        value,
        descriptor.enumerable,
    );
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
    for name in ordered_object_property_names(source) {
        let property = Expression::String(name.clone());
        if source
            .non_enumerable_string_properties
            .iter()
            .any(|hidden_name| hidden_name == &name)
            || object_binding_lookup_descriptor(source, &property)
                .is_some_and(|descriptor| !descriptor.enumerable)
        {
            continue;
        }
        let Some(value) = object_binding_lookup_value(source, &property) else {
            continue;
        };
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
