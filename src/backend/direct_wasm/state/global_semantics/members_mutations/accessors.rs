use super::super::super::*;

impl GlobalMemberService {
    pub(in crate::backend::direct_wasm) fn set_getter_binding(
        &mut self,
        key: MemberFunctionBindingKey,
        binding: LocalFunctionBinding,
    ) {
        crate::backend::direct_wasm::memo::bump_static_state_generation();
        if crate::ayy_env_flag!("AYY_TRACE_MEMBER_BINDINGS") {
            eprintln!("global_member:set_getter key={key:?} binding={binding:?}");
        }
        self.member_getter_bindings.insert(key, binding);
    }

    pub(in crate::backend::direct_wasm) fn clear_getter_binding(
        &mut self,
        key: &MemberFunctionBindingKey,
    ) {
        crate::backend::direct_wasm::memo::bump_static_state_generation();
        if crate::ayy_env_flag!("AYY_TRACE_MEMBER_BINDINGS") {
            eprintln!("global_member:clear_getter key={key:?}");
        }
        self.member_getter_bindings.remove(key);
    }

    pub(in crate::backend::direct_wasm) fn set_setter_binding(
        &mut self,
        key: MemberFunctionBindingKey,
        binding: LocalFunctionBinding,
    ) {
        crate::backend::direct_wasm::memo::bump_static_state_generation();
        if crate::ayy_env_flag!("AYY_TRACE_MEMBER_BINDINGS") {
            eprintln!("global_member:set_setter key={key:?} binding={binding:?}");
        }
        self.member_setter_bindings.insert(key, binding);
    }

    pub(in crate::backend::direct_wasm) fn clear_setter_binding(
        &mut self,
        key: &MemberFunctionBindingKey,
    ) {
        crate::backend::direct_wasm::memo::bump_static_state_generation();
        if crate::ayy_env_flag!("AYY_TRACE_MEMBER_BINDINGS") {
            eprintln!("global_member:clear_setter key={key:?}");
        }
        self.member_setter_bindings.remove(key);
    }
}
