use super::super::*;
use super::FunctionStaticEvalContext;

impl StaticAssignedMemberPolicySource for FunctionStaticEvalContext<'_, '_> {
    fn static_assign_member_binding_value(
        &self,
        object: &Expression,
        property: &Expression,
        value: &Expression,
        environment: &mut Self::Environment,
    ) -> Option<()> {
        if !is_private_property_name_expression(property) {
            return None;
        }
        let LocalFunctionBinding::User(function_name) = self
            .resolve_member_setter_binding_with_context(
                object,
                property,
                self.current_function_name(),
            )?
        else {
            return None;
        };
        let target_name = resolve_stateful_object_binding_name_in_environment(object, environment)?;
        let mut bindings = environment.local_bindings.clone();
        if let Some(object_binding) = environment.object_binding(&target_name).cloned() {
            bindings.insert(
                target_name.clone(),
                object_binding_to_expression(&object_binding),
            );
        }
        let this_binding = Expression::Identifier(target_name.clone());
        let (_, updated_bindings) = self
            .resolve_bound_snapshot_user_function_result_with_arguments_and_this(
                &function_name,
                &bindings,
                std::slice::from_ref(value),
                &this_binding,
            )?;
        let updated_this = updated_bindings
            .get("this")
            .or_else(|| updated_bindings.get(&target_name))?;
        let updated_object = self.resolve_object_binding_with_state(updated_this, environment)?;
        environment.set_object_binding(target_name, updated_object);
        Some(())
    }

    fn static_resolve_assigned_member_property_key(
        &self,
        property: &Expression,
        _environment: &mut Self::Environment,
    ) -> Option<Expression> {
        self.resolve_property_key(property)
    }

    fn static_should_seed_assigned_member_target_object_binding(
        &self,
        target_name: &str,
        _environment: &mut Self::Environment,
    ) -> bool {
        self.resolve_function_binding(&Expression::Identifier(target_name.to_string()))
            .is_some()
    }
}
