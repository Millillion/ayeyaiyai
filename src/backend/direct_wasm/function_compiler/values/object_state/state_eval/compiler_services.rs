use super::super::super::*;
use super::FunctionStaticEvalContext;

impl<'a> FunctionCompiler<'a> {
    fn module_namespace_index_from_object_binding(binding: &ObjectValueBinding) -> Option<usize> {
        fn number_to_index(value: &Expression) -> Option<usize> {
            let Expression::Number(index) = value else {
                return None;
            };
            if index.is_finite() && *index >= 0.0 && index.fract() == 0.0 {
                Some(*index as usize)
            } else {
                None
            }
        }

        if !Self::object_binding_has_module_namespace_marker(binding) {
            return None;
        }

        binding
            .string_properties
            .iter()
            .find_map(|(key, value)| {
                (key == "__ayy$module$namespace$moduleIndex")
                    .then(|| number_to_index(value))
                    .flatten()
            })
            .or_else(|| {
                binding
                    .property_descriptors
                    .iter()
                    .find_map(|(property, descriptor)| {
                        matches!(
                            property,
                            Expression::String(key)
                                if key == "__ayy$module$namespace$moduleIndex"
                        )
                        .then(|| descriptor.value.as_ref().and_then(number_to_index))
                        .flatten()
                    })
            })
    }

    pub(in crate::backend::direct_wasm) fn module_namespace_index_from_expression(
        &self,
        expression: &Expression,
    ) -> Option<usize> {
        if let Expression::Identifier(name) = expression
            && let Some(module_index) = Self::module_index_from_namespace_like_identifier(name)
        {
            return Some(module_index);
        }

        if let Expression::Identifier(name) = expression {
            let resolved_local_name = self
                .resolve_current_local_binding(name)
                .map(|(resolved_name, _)| resolved_name);
            let mut candidate_names = Vec::new();
            if let Some(resolved_name) = resolved_local_name.as_ref() {
                candidate_names.push(resolved_name.as_str());
            }
            candidate_names.push(name.as_str());
            candidate_names.sort_unstable();
            candidate_names.dedup();

            for candidate_name in candidate_names {
                let value = self
                    .state
                    .speculation
                    .static_semantics
                    .local_value_binding(candidate_name)
                    .or_else(|| {
                        resolved_local_name
                            .is_none()
                            .then(|| self.global_value_binding(candidate_name))
                            .flatten()
                    });
                let Some(value) = value else {
                    continue;
                };
                if let Expression::Identifier(alias_name) = value
                    && let Some(module_index) =
                        Self::module_index_from_namespace_like_identifier(alias_name)
                {
                    return Some(module_index);
                }
                if let Some(module_index) = self
                    .resolve_object_binding_from_expression(value)
                    .as_ref()
                    .and_then(Self::module_namespace_index_from_object_binding)
                {
                    return Some(module_index);
                }
            }
        }

        let materialized = self.materialize_static_expression(expression);
        if !static_expression_matches(&materialized, expression)
            && let Expression::Identifier(name) = &materialized
            && let Some(module_index) = Self::module_index_from_namespace_like_identifier(name)
        {
            return Some(module_index);
        }

        self.resolve_object_binding_from_expression(expression)
            .as_ref()
            .and_then(Self::module_namespace_index_from_object_binding)
            .or_else(|| {
                (!static_expression_matches(&materialized, expression))
                    .then(|| {
                        self.resolve_object_binding_from_expression(&materialized)
                            .as_ref()
                            .and_then(Self::module_namespace_index_from_object_binding)
                    })
                    .flatten()
            })
    }

    fn static_module_namespace_object_array_call_binding(
        &self,
        callee: &Expression,
        arguments: &[CallArgument],
    ) -> Option<ArrayValueBinding> {
        let Expression::Member { object, property } = callee else {
            return None;
        };
        let Expression::Identifier(object_name) = object.as_ref() else {
            return None;
        };
        let [
            CallArgument::Expression(target) | CallArgument::Spread(target),
            ..,
        ] = arguments
        else {
            return None;
        };
        let module_index = self.module_namespace_index_from_expression(target)?;

        match (object_name.as_str(), property.as_ref()) {
            ("Object", Expression::String(name))
                if matches!(name.as_str(), "keys" | "getOwnPropertyNames") =>
            {
                self.resolve_static_dynamic_import_namespace_own_property_names_binding(
                    module_index,
                )
            }
            ("Object", Expression::String(name)) if name == "getOwnPropertySymbols" => Some(
                self.resolve_static_dynamic_import_namespace_own_property_symbols_binding(
                    module_index,
                ),
            ),
            ("Reflect", Expression::String(name)) if name == "ownKeys" => {
                let mut names = self
                    .resolve_static_dynamic_import_namespace_own_property_names_binding(
                        module_index,
                    )?;
                let symbols = self
                    .resolve_static_dynamic_import_namespace_own_property_symbols_binding(
                        module_index,
                    );
                names.values.extend(symbols.values);
                Some(names)
            }
            _ => None,
        }
    }

    pub(in crate::backend::direct_wasm) fn static_eval_context(
        &self,
    ) -> FunctionStaticEvalContext<'_, 'a> {
        FunctionStaticEvalContext::new(self)
    }

    pub(in crate::backend::direct_wasm) fn evaluate_static_expression_with_state(
        &self,
        expression: &Expression,
        environment: &mut StaticResolutionEnvironment,
    ) -> Option<Expression> {
        let context = self.static_eval_context();
        context.evaluate_static_expression_with_state(expression, environment)
    }

    pub(in crate::backend::direct_wasm) fn materialize_static_expression_with_state(
        &self,
        expression: &Expression,
        environment: &StaticResolutionEnvironment,
    ) -> Option<Expression> {
        let context = self.static_eval_context();
        context.materialize_static_expression_with_state(expression, environment)
    }

    pub(in crate::backend::direct_wasm) fn execute_static_statements_with_state(
        &self,
        statements: &[Statement],
        environment: &mut StaticResolutionEnvironment,
    ) -> Option<Option<Expression>> {
        let context = self.static_eval_context();
        context.execute_static_statements_with_state(statements, environment)
    }

    pub(in crate::backend::direct_wasm) fn static_enumerated_keys_binding(
        &self,
        expression: &Expression,
    ) -> Option<ArrayValueBinding> {
        let context = self.static_eval_context();
        StaticBuiltinArrayBindingResolver::static_enumerated_keys_binding(&context, expression)
    }

    pub(in crate::backend::direct_wasm) fn static_builtin_object_array_call_binding(
        &self,
        callee: &Expression,
        arguments: &[CallArgument],
    ) -> Option<ArrayValueBinding> {
        if let Some(binding) =
            self.static_module_namespace_object_array_call_binding(callee, arguments)
        {
            return Some(binding);
        }
        let context = self.static_eval_context();
        StaticBuiltinArrayBindingResolver::static_builtin_object_array_call_binding(
            &context, callee, arguments,
        )
    }
}
