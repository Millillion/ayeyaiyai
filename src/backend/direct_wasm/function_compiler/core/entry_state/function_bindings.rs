use super::*;

impl<'a> FunctionCompiler<'a> {
    pub(super) fn apply_special_function_bindings(
        bindings: &mut EntryBindingState,
        user_function: Option<&UserFunction>,
        declaration: Option<&FunctionDeclaration>,
        global_binding_environment: &GlobalBindingEnvironment,
    ) {
        if let Some(user_function) = user_function
            && let Some(function) = declaration
            && let Some(binding_name) = function
                .self_binding
                .as_ref()
                .or(function.top_level_binding.as_ref())
        {
            bindings.static_bindings.local_function_bindings.insert(
                binding_name.clone(),
                LocalFunctionBinding::User(user_function.name.clone()),
            );
            bindings.static_bindings.local_value_bindings.insert(
                binding_name.clone(),
                Expression::Identifier(user_function.name.clone()),
            );
            bindings
                .static_bindings
                .local_kinds
                .insert(binding_name.clone(), StaticValueKind::Function);
        }
        if let Some(user_function) = user_function
            && let Some(home_object_name) = user_function.home_object_binding.as_deref()
            && user_function.lexical_this
            && !home_object_name.ends_with(".prototype")
            && !bindings
                .static_bindings
                .local_value_bindings
                .contains_key("this")
        {
            bindings.static_bindings.local_value_bindings.insert(
                "this".to_string(),
                Expression::Identifier(home_object_name.to_string()),
            );
            if let Some(home_object_binding) = global_binding_environment
                .object_bindings
                .get(home_object_name)
                .cloned()
            {
                bindings
                    .static_bindings
                    .local_object_bindings
                    .insert("this".to_string(), home_object_binding);
                bindings
                    .static_bindings
                    .local_kinds
                    .insert("this".to_string(), StaticValueKind::Object);
            }
        }
        if declaration.is_some_and(|function| function.derived_constructor) {
            bindings
                .static_bindings
                .local_value_bindings
                .insert("this".to_string(), Expression::Undefined);
            bindings
                .static_bindings
                .local_kinds
                .insert("this".to_string(), StaticValueKind::Undefined);
            bindings
                .static_bindings
                .local_object_bindings
                .remove("this");
        }
    }
}
