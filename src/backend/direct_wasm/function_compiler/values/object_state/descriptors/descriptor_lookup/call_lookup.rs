use super::*;

impl<'a> FunctionCompiler<'a> {
    pub(in crate::backend::direct_wasm) fn resolve_call_descriptor_binding(
        &self,
        callee: &Expression,
        arguments: &[CallArgument],
    ) -> Option<PropertyDescriptorBinding> {
        let direct_member_is_get_own_property_descriptor = matches!(
            callee,
            Expression::Member { object, property }
                if matches!(object.as_ref(), Expression::Identifier(name) if name == "Object" || name == "Reflect")
                    && matches!(
                        property.as_ref(),
                        Expression::String(name) if name == "getOwnPropertyDescriptor"
                    )
        );

        let (resolved_callee, callee_function_binding) =
            if direct_member_is_get_own_property_descriptor {
                (callee.clone(), None)
            } else if matches!(callee, Expression::Identifier(_)) {
                (
                    self.resolve_bound_alias_expression(callee)
                        .unwrap_or_else(|| self.materialize_static_expression(callee)),
                    self.resolve_function_binding_from_expression(callee),
                )
            } else {
                return None;
            };

        let is_get_own_property_descriptor_member = matches!(
            &resolved_callee,
            Expression::Member { object, property }
                if matches!(object.as_ref(), Expression::Identifier(name) if name == "Object" || name == "Reflect")
                    && matches!(
                        property.as_ref(),
                        Expression::String(name) if name == "getOwnPropertyDescriptor"
                    )
        );
        let is_get_own_property_descriptor_binding = matches!(
            callee_function_binding,
            Some(LocalFunctionBinding::Builtin(name)) if name == "Object.getOwnPropertyDescriptor"
        );
        if !is_get_own_property_descriptor_member && !is_get_own_property_descriptor_binding {
            return None;
        }
        let [
            CallArgument::Expression(target),
            CallArgument::Expression(property_name),
            ..,
        ] = arguments
        else {
            return None;
        };
        if let Expression::Identifier(identifier) = property_name
            && self.resolve_current_local_binding(identifier).is_some()
            && self
                .resolve_bound_alias_expression(property_name)
                .filter(|resolved| !static_expression_matches(resolved, property_name))
                .is_none()
            && self
                .resolve_symbol_identity_expression(property_name)
                .is_none()
        {
            return None;
        }
        let property = self
            .resolve_property_key_expression(property_name)
            .unwrap_or_else(|| self.materialize_static_expression(property_name));
        let string_property_name = static_property_name_from_expression(&property);
        if let Some(descriptor) =
            self.resolve_arguments_descriptor_binding(target, string_property_name.as_deref())
        {
            return Some(descriptor);
        }
        if string_property_name.as_deref() == Some("length")
            && self.resolve_array_binding_from_expression(target).is_some()
        {
            return Some(PropertyDescriptorBinding {
                value: Some(Expression::Member {
                    object: Box::new(target.clone()),
                    property: Box::new(Expression::String("length".to_string())),
                }),
                configurable: false,
                enumerable: false,
                writable: Some(true),
                getter: None,
                setter: None,
                has_get: false,
                has_set: false,
            });
        }
        if self.state.speculation.execution_context.top_level_function
            && matches!(target, Expression::This)
            && let Some(property_name) = string_property_name.as_deref()
        {
            return self.resolve_top_level_global_property_descriptor_binding(property_name);
        }
        let resolved_target = self
            .resolve_bound_alias_expression(target)
            .filter(|resolved| !static_expression_matches(resolved, target));
        let materialized_target = self.materialize_static_expression(target);
        if let Some(property_name) = string_property_name.as_deref()
            && let Some(descriptor) = self.resolve_function_property_descriptor_binding(
                target,
                resolved_target.as_ref(),
                &materialized_target,
                property_name,
            )
        {
            return Some(descriptor);
        }
        self.resolve_object_property_descriptor_binding(
            target,
            resolved_target.as_ref(),
            &materialized_target,
            &property,
            string_property_name.as_deref(),
        )
    }
}
