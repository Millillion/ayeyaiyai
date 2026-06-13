use super::*;
#[path = "call_registration/apply_expansion.rs"]
mod apply_expansion;
#[path = "call_registration/plain_registration.rs"]
mod plain_registration;
#[path = "call_registration/stateful_registration.rs"]
mod stateful_registration;

impl DirectWasmCompiler {
    fn parameter_function_metadata_object_candidate(
        &self,
        param_name: &str,
        function_binding: Option<&LocalFunctionBinding>,
        object_candidate: Option<ObjectValueBinding>,
    ) -> Option<ObjectValueBinding> {
        let Some(function_binding) = function_binding else {
            return object_candidate;
        };
        match function_binding {
            LocalFunctionBinding::User(function_name)
                if self.registered_function(function_name).is_none()
                    && self.user_function(function_name).is_none() =>
            {
                return object_candidate;
            }
            LocalFunctionBinding::User(_) | LocalFunctionBinding::Builtin(_) => {}
        }

        let mut object_binding = object_candidate.unwrap_or_else(empty_object_value_binding);
        self.define_missing_parameter_function_metadata_descriptor(
            &mut object_binding,
            param_name,
            "name",
        );
        self.define_missing_parameter_function_metadata_descriptor(
            &mut object_binding,
            param_name,
            "length",
        );
        Some(object_binding)
    }

    fn define_missing_parameter_function_metadata_descriptor(
        &self,
        object_binding: &mut ObjectValueBinding,
        param_name: &str,
        property_name: &str,
    ) {
        let property = Expression::String(property_name.to_string());
        if object_binding_lookup_descriptor(object_binding, &property).is_some()
            || object_binding_lookup_value(object_binding, &property).is_some()
        {
            return;
        }

        object_binding_define_property_descriptor(
            object_binding,
            property.clone(),
            PropertyDescriptorBinding {
                value: Some(Expression::Identifier(
                    FunctionCompiler::runtime_object_property_shadow_binding_name(
                        param_name,
                        property_name,
                    ),
                )),
                configurable: true,
                enumerable: false,
                writable: Some(false),
                getter: None,
                setter: None,
                has_get: false,
                has_set: false,
            },
        );
    }
}
