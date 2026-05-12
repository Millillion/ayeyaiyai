use super::*;

pub(in crate::backend::direct_wasm) const BOXED_PRIMITIVE_VALUE_PROPERTY: &str =
    "__ayy[[BoxedPrimitiveValue]]";
pub(in crate::backend::direct_wasm) const DATE_VALUE_PROPERTY: &str = "__ayy[[DateValue]]";
pub(in crate::backend::direct_wasm) const FUNCTION_CONSTRUCTOR_SOURCE_PROPERTY: &str =
    "__ayy[[FunctionConstructorSource]]";
pub(in crate::backend::direct_wasm) const MAP_COLLECTION_MARKER_PROPERTY: &str =
    "__ayy[[BuiltinMap]]";
pub(in crate::backend::direct_wasm) const MAP_COLLECTION_KIND_PROPERTY: &str = "__ayy[[MapKind]]";
pub(in crate::backend::direct_wasm) const MAP_COLLECTION_SIZE_PROPERTY: &str = "__ayy[[MapSize]]";
pub(in crate::backend::direct_wasm) const MAP_COLLECTION_ENTRIES_PROPERTY: &str =
    "__ayy[[MapEntries]]";
pub(in crate::backend::direct_wasm) const WEAK_COLLECTION_KIND_PROPERTY: &str =
    "__ayy[[WeakCollectionKind]]";
pub(in crate::backend::direct_wasm) const WEAK_COLLECTION_ENTRY_PREFIX: &str =
    "__ayy[[WeakCollectionEntry]]:";
pub(in crate::backend::direct_wasm) const VIEWED_ARRAY_BUFFER_PROPERTY: &str =
    "__ayy[[ViewedArrayBuffer]]";
pub(in crate::backend::direct_wasm) const TYPED_ARRAY_NAME_PROPERTY: &str =
    "__ayy[[TypedArrayName]]";
pub(in crate::backend::direct_wasm) const TYPED_ARRAY_LENGTH_PROPERTY: &str =
    "__ayy[[TypedArrayLength]]";

pub(in crate::backend::direct_wasm) fn boxed_primitive_value_property_expression() -> Expression {
    Expression::String(BOXED_PRIMITIVE_VALUE_PROPERTY.to_string())
}

pub(in crate::backend::direct_wasm) fn date_value_property_expression() -> Expression {
    Expression::String(DATE_VALUE_PROPERTY.to_string())
}

pub(in crate::backend::direct_wasm) fn function_constructor_source_property_expression()
-> Expression {
    Expression::String(FUNCTION_CONSTRUCTOR_SOURCE_PROPERTY.to_string())
}

pub(in crate::backend::direct_wasm) fn map_collection_marker_property_expression() -> Expression {
    Expression::String(MAP_COLLECTION_MARKER_PROPERTY.to_string())
}

pub(in crate::backend::direct_wasm) fn map_collection_kind_property_expression() -> Expression {
    Expression::String(MAP_COLLECTION_KIND_PROPERTY.to_string())
}

pub(in crate::backend::direct_wasm) fn map_collection_size_property_expression() -> Expression {
    Expression::String(MAP_COLLECTION_SIZE_PROPERTY.to_string())
}

pub(in crate::backend::direct_wasm) fn map_collection_entries_property_expression() -> Expression {
    Expression::String(MAP_COLLECTION_ENTRIES_PROPERTY.to_string())
}

pub(in crate::backend::direct_wasm) fn weak_collection_kind_property_expression() -> Expression {
    Expression::String(WEAK_COLLECTION_KIND_PROPERTY.to_string())
}

pub(in crate::backend::direct_wasm) fn weak_collection_entry_property_expression(
    key: &str,
) -> Expression {
    Expression::String(format!("{WEAK_COLLECTION_ENTRY_PREFIX}{key}"))
}

pub(in crate::backend::direct_wasm) fn viewed_array_buffer_property_expression() -> Expression {
    Expression::String(VIEWED_ARRAY_BUFFER_PROPERTY.to_string())
}

pub(in crate::backend::direct_wasm) fn typed_array_name_property_expression() -> Expression {
    Expression::String(TYPED_ARRAY_NAME_PROPERTY.to_string())
}

pub(in crate::backend::direct_wasm) fn typed_array_length_property_expression() -> Expression {
    Expression::String(TYPED_ARRAY_LENGTH_PROPERTY.to_string())
}

pub(in crate::backend::direct_wasm) fn empty_object_value_binding() -> ObjectValueBinding {
    ObjectValueBinding {
        string_properties: Vec::new(),
        symbol_properties: Vec::new(),
        property_descriptors: Vec::new(),
        non_enumerable_string_properties: Vec::new(),
        runtime_symbol_properties: false,
        extensible: true,
    }
}
