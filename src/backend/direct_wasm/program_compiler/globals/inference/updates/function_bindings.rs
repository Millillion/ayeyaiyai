use super::*;

impl DirectWasmCompiler {
    pub(in crate::backend::direct_wasm) fn synthesize_global_function_to_string(
        &self,
        function_name: &str,
    ) -> String {
        let Some(function) = self.registered_function(function_name) else {
            return format!("function {function_name}() {{}}");
        };
        let params = function
            .params
            .iter()
            .map(|param| param.name.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        let prefix = match function.kind {
            FunctionKind::Ordinary => "function",
            FunctionKind::Generator => "function*",
            FunctionKind::Async => "async function",
            FunctionKind::AsyncGenerator => "async function*",
        };
        match function_display_name(function) {
            Some(name) if !name.is_empty() => format!("{prefix} {name}({params}) {{}}"),
            _ => format!("{prefix}({params}) {{}}"),
        }
    }

    pub(in crate::backend::direct_wasm) fn synthesize_global_function_binding_to_string(
        &self,
        binding: &LocalFunctionBinding,
    ) -> String {
        match binding {
            LocalFunctionBinding::User(function_name) => {
                self.synthesize_global_function_to_string(function_name)
            }
            LocalFunctionBinding::Builtin(function_name) => format!(
                "function {}() {{}}",
                builtin_function_display_name(function_name)
            ),
        }
    }

    fn static_global_property_name_from_generator_call(
        &self,
        callee: &Expression,
        arguments: &[CallArgument],
    ) -> Option<String> {
        if !arguments.is_empty() {
            return None;
        }
        let binding = self.infer_global_function_binding(callee)?;
        let LocalFunctionBinding::User(function_name) = binding else {
            return None;
        };
        let function = self.user_function(&function_name)?;
        if !function.is_generator() {
            return None;
        }
        let return_value = function.inline_summary.as_ref()?.return_value.as_ref()?;
        static_property_name_from_expression(&self.materialize_global_expression(return_value))
            .or_else(|| static_property_name_from_expression(return_value))
    }

    fn static_global_property_name_from_string_call(
        &self,
        callee: &Expression,
        arguments: &[CallArgument],
    ) -> Option<String> {
        if !matches!(callee, Expression::Identifier(name)
            if name == "String"
                && !self.global_has_binding(name)
                && !self.global_has_lexical_binding(name))
        {
            return None;
        }
        let Some(CallArgument::Expression(argument) | CallArgument::Spread(argument)) =
            arguments.first()
        else {
            return Some(String::new());
        };
        self.static_global_property_name_from_expression(argument)
    }

    fn static_global_property_name_from_expression(
        &self,
        expression: &Expression,
    ) -> Option<String> {
        if let Some(property_name) = static_property_name_from_expression(expression) {
            return Some(property_name);
        }
        if let Expression::Call { callee, arguments } = expression {
            if let Some(property_name) =
                self.static_global_property_name_from_string_call(callee, arguments)
            {
                return Some(property_name);
            }
            if let Some(property_name) =
                self.static_global_property_name_from_generator_call(callee, arguments)
            {
                return Some(property_name);
            }
        }
        let materialized = self.materialize_global_expression(expression);
        if static_expression_matches(&materialized, expression) {
            return None;
        }
        static_property_name_from_expression(&materialized).or_else(|| {
            if let Expression::Call { callee, arguments } = &materialized {
                self.static_global_property_name_from_string_call(callee, arguments)
                    .or_else(|| {
                        self.static_global_property_name_from_generator_call(callee, arguments)
                    })
            } else {
                None
            }
        })
    }

    fn direct_global_function_property_key_binding(
        &self,
        expression: &Expression,
    ) -> Option<LocalFunctionBinding> {
        let Expression::Identifier(name) = expression else {
            return None;
        };
        if let Some(binding) = self.global_function_binding(name) {
            return Some(binding.clone());
        }
        if is_internal_user_function_identifier(name) && self.contains_user_function(name) {
            Some(LocalFunctionBinding::User(name.clone()))
        } else if builtin_identifier_kind(name) == Some(StaticValueKind::Function) {
            Some(LocalFunctionBinding::Builtin(name.clone()))
        } else {
            None
        }
    }

    pub(in crate::backend::direct_wasm) fn infer_global_function_binding(
        &self,
        expression: &Expression,
    ) -> Option<LocalFunctionBinding> {
        let materialized = self.materialize_global_expression(expression);
        if !static_expression_matches(&materialized, expression) {
            if matches!(expression, Expression::Member { .. })
                && matches!(materialized, Expression::Undefined)
            {
                // Prototype lookups can remain valid even when own-property materialization
                // bottoms out to `undefined`.
            } else {
                return self.infer_global_function_binding(&materialized);
            }
        }
        match expression {
            Expression::Identifier(name) => {
                if let Some(binding) = self.global_function_binding(name) {
                    return Some(binding.clone());
                }
                if is_internal_user_function_identifier(name) && self.contains_user_function(name) {
                    Some(LocalFunctionBinding::User(name.clone()))
                } else if builtin_identifier_kind(name) == Some(StaticValueKind::Function) {
                    Some(LocalFunctionBinding::Builtin(name.clone()))
                } else {
                    None
                }
            }
            Expression::Member { object, property } => {
                let materialized_property = self.materialize_global_expression(property);
                if let Some(array_binding) = self.infer_global_array_binding(object)
                    && let Some(index) = argument_index_from_expression(&materialized_property)
                    && let Some(Some(value)) = array_binding.values.get(index as usize)
                    && let Some(binding) = self.infer_global_function_binding(value)
                {
                    return Some(binding);
                }
                if let Some(object_binding) = self.infer_global_object_binding(object)
                    && let Some(value) =
                        object_binding_lookup_value(&object_binding, &materialized_property)
                    && let Some(binding) = self.infer_global_function_binding(value)
                {
                    return Some(binding);
                }
                if let Expression::Identifier(name) = object.as_ref()
                    && let Some(prototype) = self.global_object_prototype_expression(name)
                {
                    if let Some(key) =
                        self.global_member_function_binding_key(prototype, &materialized_property)
                        && let Some(binding) = self.global_member_function_binding(&key)
                    {
                        return Some(binding.clone());
                    }
                    if let Some(object_binding) = self.infer_global_object_binding(prototype)
                        && let Some(value) =
                            object_binding_lookup_value(&object_binding, &materialized_property)
                        && let Some(binding) = self.infer_global_function_binding(value)
                    {
                        return Some(binding);
                    }
                }
                None
            }
            _ => None,
        }
    }

    pub(in crate::backend::direct_wasm) fn global_member_function_binding_property(
        &self,
        property: &Expression,
    ) -> Option<MemberFunctionBindingProperty> {
        let materialized = self.materialize_global_expression(property);
        for candidate in [property, &materialized] {
            if let Some(symbol_name) = match candidate {
                Expression::Member { object, property }
                    if matches!(object.as_ref(), Expression::Identifier(name)
                        if name == "Symbol"
                            && !self.global_has_binding(name)
                            && !self.global_has_lexical_binding(name))
                        && matches!(property.as_ref(), Expression::String(_)) =>
                {
                    let Expression::String(symbol_name) = property.as_ref() else {
                        unreachable!("filtered above");
                    };
                    Some(format!("Symbol.{symbol_name}"))
                }
                _ => None,
            } {
                return Some(MemberFunctionBindingProperty::Symbol(symbol_name));
            }
            if let Expression::Identifier(name) = candidate
                && self.global_expression_is_static_symbol_property_key(candidate)
            {
                return Some(MemberFunctionBindingProperty::Symbol(name.clone()));
            }
            if self.global_expression_is_static_symbol_property_key(candidate) {
                return Some(MemberFunctionBindingProperty::SymbolExpression(format!(
                    "{candidate:?}"
                )));
            }
        }
        if let Some(property_name) = self.static_global_property_name_from_expression(&materialized)
        {
            return Some(MemberFunctionBindingProperty::String(property_name));
        }
        for candidate in [property, &materialized] {
            if let Some(binding) = self.direct_global_function_property_key_binding(candidate) {
                return Some(MemberFunctionBindingProperty::String(
                    self.synthesize_global_function_binding_to_string(&binding),
                ));
            }
        }
        match &materialized {
            Expression::Member { object, property }
                if matches!(object.as_ref(), Expression::Identifier(name) if name == "Symbol")
                    && matches!(property.as_ref(), Expression::String(_)) =>
            {
                let Expression::String(symbol_name) = property.as_ref() else {
                    unreachable!("filtered above");
                };
                Some(MemberFunctionBindingProperty::Symbol(format!(
                    "Symbol.{symbol_name}"
                )))
            }
            Expression::Call { callee, .. } if matches!(callee.as_ref(), Expression::Identifier(name) if name == "Symbol") => {
                Some(MemberFunctionBindingProperty::SymbolExpression(format!(
                    "{materialized:?}"
                )))
            }
            _ => None,
        }
    }

    pub(in crate::backend::direct_wasm) fn global_member_function_binding_key(
        &self,
        object: &Expression,
        property: &Expression,
    ) -> Option<MemberFunctionBindingKey> {
        let target = match object {
            Expression::Identifier(name) => MemberFunctionBindingTarget::Identifier(name.clone()),
            Expression::Member {
                object,
                property: target_property,
            } if matches!(target_property.as_ref(), Expression::String(name) if name == "prototype") =>
            {
                let Expression::Identifier(name) = object.as_ref() else {
                    return None;
                };
                MemberFunctionBindingTarget::Prototype(name.clone())
            }
            Expression::New { callee, .. } => {
                let Expression::Identifier(name) = callee.as_ref() else {
                    return None;
                };
                MemberFunctionBindingTarget::Prototype(name.clone())
            }
            _ => return None,
        };
        let property = self.global_member_function_binding_property(property)?;
        Some(MemberFunctionBindingKey { target, property })
    }
}
