use super::super::*;
use super::FunctionStaticEvalContext;

impl StaticExpressionEnvironmentSource for FunctionStaticEvalContext<'_, '_> {
    type Environment = StaticResolutionEnvironment;
}

impl StaticBindingLookupSource for FunctionStaticEvalContext<'_, '_> {}

impl StaticMemberDeletionSource for FunctionStaticEvalContext<'_, '_> {}

impl StaticEnvironmentObjectBindingSource for FunctionStaticEvalContext<'_, '_> {
    fn static_resolve_environment_object_binding(
        &self,
        binding_expression: &Expression,
        environment: &mut Self::Environment,
    ) -> Option<ObjectValueBinding> {
        self.resolve_object_binding_with_state(binding_expression, environment)
    }
}

impl StaticMissingMemberPolicySource for FunctionStaticEvalContext<'_, '_> {
    fn static_preserve_missing_member_expression(
        &self,
        _full_expression: &Expression,
        _object: &Expression,
        property: &Expression,
        _environment: &Self::Environment,
    ) -> bool {
        is_private_property_name_expression(property)
    }
}
