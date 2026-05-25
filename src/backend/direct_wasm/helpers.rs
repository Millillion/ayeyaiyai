use super::*;

#[path = "helpers/eval_namespace.rs"]
mod eval_namespace;
#[path = "helpers/eval_rewrite.rs"]
mod eval_rewrite;
#[path = "helpers/object_values.rs"]
mod object_values;
#[path = "helpers/realm_names.rs"]
mod realm_names;

pub(in crate::backend::direct_wasm) use self::{
    eval_rewrite::namespace_eval_program_internal_function_names,
    object_values::{
        boxed_primitive_value_property_expression, date_value_property_expression,
        empty_object_value_binding, function_constructor_realm_id_property_expression,
        function_constructor_realm_object_prototype_property_expression,
        function_constructor_source_property_expression,
        map_collection_entries_property_expression, map_collection_kind_property_expression,
        map_collection_marker_property_expression, map_collection_size_property_expression,
        typed_array_length_property_expression, typed_array_name_property_expression,
        viewed_array_buffer_property_expression, weak_collection_entry_property_expression,
        weak_collection_kind_property_expression,
    },
    realm_names::{
        parse_test262_realm_eval_builtin, parse_test262_realm_global_identifier,
        parse_test262_realm_identifier, test262_realm_eval_builtin_name,
        test262_realm_global_identifier, test262_realm_identifier,
    },
};
