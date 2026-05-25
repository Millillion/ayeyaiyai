use super::*;

impl StaticFunctionConstructorLowerer {
    pub(super) fn try_lower_static_function_constructor(
        &mut self,
        callee: &Expression,
        arguments: &[CallArgument],
    ) -> Result<Option<Expression>> {
        let constructor_arguments = if self.is_function_constructor_callee(callee) {
            arguments
        } else if self.is_function_constructor_call_callee(callee) {
            let [_, constructor_arguments @ ..] = arguments else {
                return Ok(None);
            };
            constructor_arguments
        } else {
            return Ok(None);
        };

        let Some((parameter_source, body_source)) =
            self.function_constructor_compile_time_source_parts(constructor_arguments)
        else {
            return Ok(None);
        };

        let function_name = self.fresh_function_name();
        let wrapper_source =
            format!("function {function_name}({parameter_source}) {{\n{body_source}\n}}");
        let Ok(parsed) = crate::frontend::parse(&wrapper_source) else {
            return Ok(None);
        };
        let mut parsed_functions = parsed.functions;
        let Some(function_index) = parsed_functions
            .iter()
            .position(|function| function.name == function_name)
        else {
            bail!("failed to lower static Function constructor `{function_name}`");
        };
        let mut function = parsed_functions.remove(function_index);
        function.synthetic_capture_bindings.clear();
        function.private_brand_binding = None;
        self.renumber_template_object_sites_in_function(&mut function);

        for mut helper_function in parsed_functions {
            self.renumber_template_object_sites_in_function(&mut helper_function);
            self.existing_function_names
                .insert(helper_function.name.clone());
            let lowered_helper = self.lower_synthetic_function(helper_function)?;
            self.synthetic_functions.push(lowered_helper);
        }

        let lowered_function = self.lower_synthetic_function(function)?;
        self.synthetic_functions.push(lowered_function);
        Ok(Some(Expression::Identifier(function_name)))
    }

    pub(super) fn fresh_function_name(&mut self) -> String {
        loop {
            let candidate = format!("__ayy_function_ctor_{}", self.next_synthetic_function_id);
            self.next_synthetic_function_id += 1;
            if self.existing_function_names.insert(candidate.clone()) {
                return candidate;
            }
        }
    }

    pub(super) fn is_bound(&self, name: &str) -> bool {
        self.scopes.contains(name)
    }

    pub(super) fn is_global_identifier(&self, expression: &Expression, name: &str) -> bool {
        matches!(expression, Expression::Identifier(identifier) if identifier == name && !self.is_bound(identifier))
    }

    pub(super) fn is_string_literal(&self, expression: &Expression, value: &str) -> bool {
        matches!(expression, Expression::String(string) if string == value)
    }

    pub(super) fn is_function_constructor_callee(&self, callee: &Expression) -> bool {
        self.is_global_identifier(callee, "Function")
            || matches!(
                callee,
                Expression::Member { object, property }
                    if self.is_global_identifier(object, "globalThis")
                        && self.is_string_literal(property, "Function")
            )
            || matches!(
                callee,
                Expression::Member { object, property }
                    if self.is_test262_realm_global_value(object)
                        && self.is_string_literal(property, "Function")
            )
    }

    pub(super) fn is_function_constructor_call_callee(&self, callee: &Expression) -> bool {
        matches!(
            callee,
            Expression::Member { object, property }
                if self.is_function_constructor_callee(object)
                    && self.is_string_literal(property, "call")
        )
    }

    fn function_constructor_compile_time_source_parts(
        &self,
        arguments: &[CallArgument],
    ) -> Option<(String, String)> {
        let parts = arguments
            .iter()
            .map(|argument| match argument {
                CallArgument::Expression(expression) => {
                    self.resolve_compile_time_string(expression)
                }
                _ => None,
            })
            .collect::<Option<Vec<_>>>()?;

        let Some((body_source, parameter_sources)) = parts.split_last() else {
            return Some((String::new(), String::new()));
        };

        Some((parameter_sources.join(","), body_source.clone()))
    }
}
