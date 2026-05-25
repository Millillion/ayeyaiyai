use super::*;

impl<'a> FunctionCompiler<'a> {
    pub(in crate::backend::direct_wasm) fn resolve_user_function_display_name(
        &self,
        function_name: &str,
    ) -> Option<String> {
        if let Some(function) = self.resolve_registered_function_declaration(function_name)
            && let Some(display_name) = function_display_name(function)
        {
            return Some(display_name);
        }

        if let Some(display_name) = self
            .state
            .speculation
            .static_semantics
            .objects
            .member_getter_bindings
            .iter()
            .map(|(key, binding)| (key.clone(), binding.clone()))
            .chain(self.backend.global_member_getter_binding_entries())
            .find_map(|(key, binding)| {
                matches!(&binding, LocalFunctionBinding::User(name) if name == function_name)
                    .then(|| {
                        self.member_function_property_display_name(&key.property, Some("get "))
                    })
                    .flatten()
            })
        {
            return Some(display_name);
        }

        if let Some(display_name) = self
            .state
            .speculation
            .static_semantics
            .objects
            .member_setter_bindings
            .iter()
            .map(|(key, binding)| (key.clone(), binding.clone()))
            .chain(self.backend.global_member_setter_binding_entries())
            .find_map(|(key, binding)| {
                matches!(&binding, LocalFunctionBinding::User(name) if name == function_name)
                    .then(|| {
                        self.member_function_property_display_name(&key.property, Some("set "))
                    })
                    .flatten()
            })
        {
            return Some(display_name);
        }

        if let Some(display_name) = self
            .state
            .speculation
            .static_semantics
            .objects
            .member_function_bindings
            .iter()
            .map(|(key, binding)| (key.clone(), binding.clone()))
            .chain(self.backend.global_member_function_binding_entries())
            .find_map(|(key, binding)| {
                if self.member_function_binding_value_is_sequence_wrapped(&key, &binding) {
                    return None;
                }
                matches!(&binding, LocalFunctionBinding::User(name) if name == function_name)
                    .then(|| self.member_function_property_display_name(&key.property, None))
                    .flatten()
            })
        {
            return Some(display_name);
        }

        None
    }

    fn object_literal_member_function_value_is_sequence_wrapped(
        &self,
        expression: &Expression,
        property: &MemberFunctionBindingProperty,
        binding: &LocalFunctionBinding,
    ) -> bool {
        let Expression::Object(entries) = expression else {
            return false;
        };
        entries.iter().any(|entry| {
            let crate::ir::hir::ObjectEntry::Data { key, value } = entry else {
                return false;
            };
            let materialized_key = self
                .resolve_property_key_expression(key)
                .unwrap_or_else(|| self.materialize_static_expression(key));
            self.member_function_binding_property(&materialized_key)
                .as_ref()
                .is_some_and(|candidate| candidate == property)
                && matches!(value, Expression::Sequence(_))
                && self
                    .resolve_function_binding_from_expression(value)
                    .as_ref()
                    .is_some_and(|resolved| resolved == binding)
        })
    }

    fn member_function_binding_value_is_sequence_wrapped(
        &self,
        key: &MemberFunctionBindingKey,
        binding: &LocalFunctionBinding,
    ) -> bool {
        let MemberFunctionBindingTarget::Identifier(target_name) = &key.target else {
            return false;
        };
        if self
            .state
            .speculation
            .static_semantics
            .local_value_binding(target_name)
            .or_else(|| self.global_value_binding(target_name))
            .is_some_and(|expression| {
                self.object_literal_member_function_value_is_sequence_wrapped(
                    expression,
                    &key.property,
                    binding,
                )
            })
        {
            return true;
        }
        let property = match &key.property {
            MemberFunctionBindingProperty::String(name) => Expression::String(name.clone()),
            MemberFunctionBindingProperty::Symbol(name) => Expression::Identifier(name.clone()),
            MemberFunctionBindingProperty::SymbolExpression(_) => return false,
        };
        let Some(object_binding) = self
            .state
            .speculation
            .static_semantics
            .local_object_binding(target_name)
            .or_else(|| self.global_object_binding(target_name))
        else {
            return false;
        };
        let Some(value) = self.resolve_object_binding_property_value(object_binding, &property)
        else {
            return false;
        };
        matches!(value, Expression::Sequence(_))
            && self
                .resolve_function_binding_from_expression(&value)
                .as_ref()
                .is_some_and(|resolved| resolved == binding)
    }

    pub(in crate::backend::direct_wasm) fn object_literal_member_function_display_name(
        &self,
        expression: &Expression,
        slot: u8,
    ) -> Option<String> {
        let Expression::Member { object, property } = expression else {
            return None;
        };
        let Expression::Object(entries) = object.as_ref() else {
            return None;
        };
        let materialized_property = self
            .resolve_property_key_expression(property)
            .unwrap_or_else(|| self.materialize_static_expression(property));
        let target_property = self.member_function_binding_property(&materialized_property)?;
        let mut state = (None, None, None);

        for entry in entries {
            let (key, binding, entry_slot) = match entry {
                crate::ir::hir::ObjectEntry::Data { key, value } => {
                    (key, self.resolve_function_binding_from_expression(value), 0)
                }
                crate::ir::hir::ObjectEntry::Getter { key, getter } => (
                    key,
                    self.resolve_function_binding_from_expression(getter),
                    1,
                ),
                crate::ir::hir::ObjectEntry::Setter { key, setter } => (
                    key,
                    self.resolve_function_binding_from_expression(setter),
                    2,
                ),
                crate::ir::hir::ObjectEntry::Spread(_) => return None,
            };
            let materialized_key = self
                .resolve_property_key_expression(key)
                .unwrap_or_else(|| self.materialize_static_expression(key));
            let Some(property_name) = self.member_function_binding_property(&materialized_key)
            else {
                continue;
            };
            if property_name != target_property {
                continue;
            }
            match entry_slot {
                0 => {
                    state.0 = binding.map(|_| property_name);
                    state.1 = None;
                    state.2 = None;
                }
                1 => {
                    state.0 = None;
                    state.1 = binding.map(|_| property_name);
                }
                2 => {
                    state.0 = None;
                    state.2 = binding.map(|_| property_name);
                }
                _ => {}
            }
        }

        let (property, prefix) = match slot {
            0 => (state.0.as_ref()?, None),
            1 => (state.1.as_ref()?, Some("get ")),
            2 => (state.2.as_ref()?, Some("set ")),
            _ => return None,
        };
        self.member_function_property_display_name(property, prefix)
    }

    pub(in crate::backend::direct_wasm) fn member_function_property_display_name(
        &self,
        property: &MemberFunctionBindingProperty,
        prefix: Option<&str>,
    ) -> Option<String> {
        let base_name = match property {
            MemberFunctionBindingProperty::String(name) => Some(name.clone()),
            MemberFunctionBindingProperty::Symbol(name) => {
                self.symbol_function_name_fragment(&Expression::Identifier(name.clone()))
            }
            MemberFunctionBindingProperty::SymbolExpression(_) => None,
        }?;

        Some(match prefix {
            Some(prefix) => format!("{prefix}{base_name}"),
            None => base_name,
        })
    }

    fn symbol_function_name_fragment(&self, expression: &Expression) -> Option<String> {
        if let Expression::Identifier(name) = expression
            && let Some(value) = self
                .state
                .speculation
                .static_semantics
                .local_value_binding(name)
                .or_else(|| self.global_value_binding(name))
            && !static_expression_matches(value, expression)
        {
            return self.symbol_function_name_fragment(value);
        }

        let current_function_name = self.current_function_name();
        let symbol_text = self
            .resolve_static_symbol_to_string_value_with_context(expression, current_function_name)
            .or_else(|| {
                let materialized = self.materialize_static_expression(expression);
                (!static_expression_matches(&materialized, expression)).then(|| {
                    self.resolve_static_symbol_to_string_value_with_context(
                        &materialized,
                        current_function_name,
                    )
                })?
            })?;
        if let Some(description) = symbol_text
            .strip_prefix("Symbol(")
            .and_then(|suffix| suffix.strip_suffix(')'))
        {
            if description.is_empty() {
                return Some(String::new());
            }
            return Some(format!("[{description}]"));
        }
        Some(format!("[{symbol_text}]"))
    }
}
