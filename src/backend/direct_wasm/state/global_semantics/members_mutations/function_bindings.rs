use super::super::super::*;

impl GlobalMemberService {
    pub(in crate::backend::direct_wasm) fn set_function_binding(
        &mut self,
        key: MemberFunctionBindingKey,
        binding: LocalFunctionBinding,
    ) {
        if crate::ayy_env_flag!("AYY_TRACE_MEMBER_BINDINGS") {
            eprintln!("global_member:set_function key={key:?} binding={binding:?}");
        }
        self.member_function_bindings.insert(key, binding);
    }

    pub(in crate::backend::direct_wasm) fn clear_function_binding(
        &mut self,
        key: &MemberFunctionBindingKey,
    ) {
        if crate::ayy_env_flag!("AYY_TRACE_MEMBER_BINDINGS") {
            eprintln!("global_member:clear_function key={key:?}");
        }
        self.member_function_bindings.remove(key);
    }

    pub(in crate::backend::direct_wasm) fn set_function_capture_slots(
        &mut self,
        key: MemberFunctionBindingKey,
        capture_slots: BTreeMap<String, String>,
    ) {
        if crate::ayy_env_flag!("AYY_TRACE_MEMBER_BINDINGS") {
            eprintln!(
                "global_member:set_capture_slots key={key:?} capture_slots={capture_slots:?}"
            );
        }
        self.member_function_capture_slots
            .insert(key, capture_slots);
    }
}
