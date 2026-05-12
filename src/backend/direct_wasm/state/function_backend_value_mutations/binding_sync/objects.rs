use super::*;

impl<'a> FunctionCompilerBackend<'a> {
    pub(in crate::backend::direct_wasm) fn sync_global_object_binding(
        &mut self,
        name: &str,
        binding: Option<ObjectValueBinding>,
    ) {
        if std::env::var_os("AYY_TRACE_GLOBAL_OBJECT_SYNC").is_some() {
            let (strings, symbols, descriptors) = binding
                .as_ref()
                .map(|binding| {
                    (
                        binding.string_properties.len(),
                        binding.symbol_properties.len(),
                        binding.property_descriptors.len(),
                    )
                })
                .unwrap_or((0, 0, 0));
            eprintln!(
                "function_global_object_sync name={name} present={} strings={strings} symbols={symbols} descriptors={descriptors}",
                binding.is_some()
            );
        }
        self.global_semantics
            .values
            .sync_object_binding(name, binding);
    }

    pub(in crate::backend::direct_wasm) fn sync_global_proxy_binding(
        &mut self,
        name: &str,
        binding: Option<ProxyValueBinding>,
    ) {
        self.global_semantics
            .values
            .sync_proxy_binding(name, binding);
    }

    pub(in crate::backend::direct_wasm) fn sync_global_prototype_object_binding(
        &mut self,
        name: &str,
        binding: Option<ObjectValueBinding>,
    ) {
        self.global_semantics
            .values
            .sync_prototype_object_binding(name, binding);
    }

    pub(in crate::backend::direct_wasm) fn sync_global_object_prototype_expression(
        &mut self,
        name: &str,
        prototype: Option<Expression>,
    ) {
        self.global_semantics
            .values
            .sync_object_prototype_expression(name, prototype);
    }
}
