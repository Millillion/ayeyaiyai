use super::super::super::*;

const IDENTIFIER_FUNCTION_VALUE_CAPTURE_PROPERTY: &str = "__ayy[[FunctionValueCaptureSlots]]";

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
        if self.member_function_bindings.is_empty()
            && self.member_function_capture_slots.is_empty()
            && self.member_getter_bindings.is_empty()
            && self.member_setter_bindings.is_empty()
        {
            return;
        }
        if crate::ayy_env_flag!("AYY_TRACE_MEMBER_BINDINGS") {
            eprintln!(
                "global_member:clear_bindings_for_name name={name} include_prototype={include_prototype}"
            );
        }
        let function_binding_count = self.member_function_bindings.len();
        let capture_slot_count = self.member_function_capture_slots.len();
        let getter_binding_count = self.member_getter_bindings.len();
        let setter_binding_count = self.member_setter_bindings.len();
        self.member_function_bindings
            .retain(|key, _| !Self::target_matches_name(&key.target, name, include_prototype));
        self.member_function_capture_slots
            .retain(|key, _| {
                if !include_prototype
                    && matches!(&key.target, MemberFunctionBindingTarget::Identifier(target_name) if target_name == name)
                    && matches!(
                        &key.property,
                        MemberFunctionBindingProperty::String(property)
                            if property == IDENTIFIER_FUNCTION_VALUE_CAPTURE_PROPERTY
                    )
                {
                    return true;
                }
                !Self::target_matches_name(&key.target, name, include_prototype)
            });
        self.member_getter_bindings
            .retain(|key, _| !Self::target_matches_name(&key.target, name, include_prototype));
        self.member_setter_bindings
            .retain(|key, _| !Self::target_matches_name(&key.target, name, include_prototype));
        if self.member_function_bindings.len() != function_binding_count
            || self.member_function_capture_slots.len() != capture_slot_count
            || self.member_getter_bindings.len() != getter_binding_count
            || self.member_setter_bindings.len() != setter_binding_count
        {
            crate::backend::direct_wasm::memo::bump_static_state_generation();
        }
    }
}
