use super::super::super::*;

impl GlobalMemberService {
    fn target_matches_name(
        target: &MemberFunctionBindingTarget,
        name: &str,
        include_prototype: bool,
    ) -> bool {
        matches!(target, MemberFunctionBindingTarget::Identifier(target_name) if target_name == name)
            || (include_prototype
                && matches!(
                    target,
                    MemberFunctionBindingTarget::Prototype(target_name) if target_name == name
                ))
    }

    pub(in crate::backend::direct_wasm) fn clear_bindings_for_name(
        &mut self,
        name: &str,
        include_prototype: bool,
    ) {
        if std::env::var_os("AYY_TRACE_MEMBER_BINDINGS").is_some() {
            eprintln!(
                "global_member:clear_bindings_for_name name={name} include_prototype={include_prototype}"
            );
        }
        self.member_function_bindings
            .retain(|key, _| !Self::target_matches_name(&key.target, name, include_prototype));
        self.member_function_capture_slots
            .retain(|key, _| !Self::target_matches_name(&key.target, name, include_prototype));
        self.member_getter_bindings
            .retain(|key, _| !Self::target_matches_name(&key.target, name, include_prototype));
        self.member_setter_bindings
            .retain(|key, _| !Self::target_matches_name(&key.target, name, include_prototype));
    }
}
