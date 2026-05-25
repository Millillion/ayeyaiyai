use super::*;

#[derive(Clone, PartialEq)]
pub(in crate::backend::direct_wasm) struct ObjectValueBinding {
    pub(in crate::backend::direct_wasm) string_properties: Vec<(String, Expression)>,
    pub(in crate::backend::direct_wasm) symbol_properties: Vec<(Expression, Expression)>,
    pub(in crate::backend::direct_wasm) property_descriptors:
        Vec<(Expression, PropertyDescriptorBinding)>,
    pub(in crate::backend::direct_wasm) non_enumerable_string_properties: Vec<String>,
    pub(in crate::backend::direct_wasm) runtime_symbol_properties: bool,
    pub(in crate::backend::direct_wasm) extensible: bool,
}

#[derive(Clone)]
pub(in crate::backend::direct_wasm) struct ProxyValueBinding {
    pub(in crate::backend::direct_wasm) target: Expression,
    pub(in crate::backend::direct_wasm) handler: Expression,
    pub(in crate::backend::direct_wasm) get_binding: Option<LocalFunctionBinding>,
    pub(in crate::backend::direct_wasm) has_binding: Option<LocalFunctionBinding>,
    pub(in crate::backend::direct_wasm) set_binding: Option<LocalFunctionBinding>,
    pub(in crate::backend::direct_wasm) get_own_property_descriptor_binding:
        Option<LocalFunctionBinding>,
    pub(in crate::backend::direct_wasm) define_property_binding: Option<LocalFunctionBinding>,
    pub(in crate::backend::direct_wasm) own_keys_binding: Option<LocalFunctionBinding>,
}
