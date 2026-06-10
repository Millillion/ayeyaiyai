use super::*;

impl DirectWasmCompiler {
    pub(in crate::backend::direct_wasm) fn clear_owned_global_member_bindings_for_name(
        &mut self,
        name: &str,
    ) {
        crate::backend::direct_wasm::memo::bump_static_state_generation();
        GlobalMemberBindingClearAccess::clear_global_member_bindings_for_name(
            &mut self.state,
            name,
        );
    }

    pub(in crate::backend::direct_wasm) fn set_global_member_function_binding(
        &mut self,
        key: MemberFunctionBindingKey,
        binding: LocalFunctionBinding,
    ) {
        crate::backend::direct_wasm::memo::bump_static_state_generation();
        GlobalMemberFunctionMutationAccess::set_global_member_function_binding(
            &mut self.state,
            key,
            binding,
        );
    }

    pub(in crate::backend::direct_wasm) fn clear_global_member_function_binding(
        &mut self,
        key: &MemberFunctionBindingKey,
    ) {
        crate::backend::direct_wasm::memo::bump_static_state_generation();
        GlobalMemberFunctionMutationAccess::clear_global_member_function_binding(
            &mut self.state,
            key,
        );
    }

    pub(in crate::backend::direct_wasm) fn set_global_member_getter_binding(
        &mut self,
        key: MemberFunctionBindingKey,
        binding: LocalFunctionBinding,
    ) {
        crate::backend::direct_wasm::memo::bump_static_state_generation();
        GlobalMemberAccessorMutationAccess::set_global_member_getter_binding(
            &mut self.state,
            key,
            binding,
        );
    }

    pub(in crate::backend::direct_wasm) fn clear_global_member_getter_binding(
        &mut self,
        key: &MemberFunctionBindingKey,
    ) {
        crate::backend::direct_wasm::memo::bump_static_state_generation();
        GlobalMemberAccessorMutationAccess::clear_global_member_getter_binding(
            &mut self.state,
            key,
        );
    }

    pub(in crate::backend::direct_wasm) fn set_global_member_setter_binding(
        &mut self,
        key: MemberFunctionBindingKey,
        binding: LocalFunctionBinding,
    ) {
        crate::backend::direct_wasm::memo::bump_static_state_generation();
        GlobalMemberAccessorMutationAccess::set_global_member_setter_binding(
            &mut self.state,
            key,
            binding,
        );
    }

    pub(in crate::backend::direct_wasm) fn clear_global_member_setter_binding(
        &mut self,
        key: &MemberFunctionBindingKey,
    ) {
        crate::backend::direct_wasm::memo::bump_static_state_generation();
        GlobalMemberAccessorMutationAccess::clear_global_member_setter_binding(
            &mut self.state,
            key,
        );
    }

    pub(in crate::backend::direct_wasm) fn set_global_member_function_capture_slots(
        &mut self,
        key: MemberFunctionBindingKey,
        capture_slots: BTreeMap<String, String>,
    ) {
        crate::backend::direct_wasm::memo::bump_static_state_generation();
        GlobalMemberCaptureMutationAccess::set_global_member_function_capture_slots(
            &mut self.state,
            key,
            capture_slots,
        );
    }
}
