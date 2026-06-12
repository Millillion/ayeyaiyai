use super::*;

impl DirectWasmCompiler {
    fn resolve_scoped_class_static_member_binding(
        &self,
        object: &Expression,
        property: &Expression,
    ) -> Option<LocalFunctionBinding> {
        let Expression::Identifier(object_name) = object else {
            return None;
        };
        let property = self.global_member_function_binding_property(property)?;
        let mut matches = self
            .global_member_function_binding_entries()
            .into_iter()
            .filter_map(|(key, binding)| {
                if key.property != property {
                    return None;
                }
                let MemberFunctionBindingTarget::Identifier(target_name) = key.target else {
                    return None;
                };
                (target_name == *object_name
                    || scoped_binding_source_name(&target_name) == Some(object_name.as_str()))
                .then_some(binding)
            });
        let binding = matches.next()?;
        matches.next().is_none().then_some(binding)
    }

    /// Resolves a scope-renamed identifier's source name to the unique user
    /// function carrying it as a self binding (the name a function
    /// expression or statement binds for its own body, e.g. recursive
    /// `function f(n) { ... f(n - 1) ... }` references).
    fn resolve_unique_self_binding_function_for_alias_analysis(
        &self,
        source_name: &str,
    ) -> Option<LocalFunctionBinding> {
        let mut matches = self.state.user_functions().iter().filter_map(|function| {
            let declaration = self.registered_function(&function.name)?;
            declaration
                .self_binding
                .as_deref()
                .filter(|self_binding| *self_binding == source_name)
                .map(|_| LocalFunctionBinding::User(function.name.clone()))
        });
        let binding = matches.next()?;
        matches.next().is_none().then_some(binding)
    }

    pub(in crate::backend::direct_wasm) fn resolve_function_binding_from_expression_with_aliases(
        &self,
        expression: &Expression,
        aliases: &HashMap<String, Option<LocalFunctionBinding>>,
    ) -> Option<LocalFunctionBinding> {
        match expression {
            Expression::Identifier(name) => {
                if let Some(function_binding) = aliases.get(name) {
                    return function_binding.clone();
                }
                if is_internal_user_function_identifier(name) && self.contains_user_function(name) {
                    Some(LocalFunctionBinding::User(name.clone()))
                } else if let Some(function_binding) = self.global_function_binding(name) {
                    Some(function_binding.clone())
                } else if name == "eval" || infer_call_result_kind(name).is_some() {
                    Some(LocalFunctionBinding::Builtin(name.clone()))
                } else if let Some(source_name) = scoped_binding_source_name(name) {
                    // Scope-renamed self-binding references (e.g. the
                    // recursive `__ayy_scope$f$1(...)` call inside
                    // `function f`) must resolve to the source function so
                    // recursive call sites participate in parameter binding
                    // analysis; otherwise the entry call's static argument
                    // is treated as the only call-site value and gets baked
                    // into the recursive callee.
                    if let Some(function_binding) = aliases.get(source_name) {
                        function_binding.clone()
                    } else if let Some(function_binding) =
                        self.global_function_binding(source_name)
                    {
                        Some(function_binding.clone())
                    } else {
                        self.resolve_unique_self_binding_function_for_alias_analysis(source_name)
                    }
                } else {
                    self.resolve_unique_self_binding_function_for_alias_analysis(name)
                }
            }
            Expression::Member { object, property } => {
                if let Some(key) = self.global_member_function_binding_key(object, property)
                    && let Some(binding) = self.global_member_function_binding(&key)
                {
                    return Some(binding.clone());
                }
                if let Some(binding) =
                    self.resolve_scoped_class_static_member_binding(object, property)
                {
                    return Some(binding);
                }

                let materialized = self.materialize_global_expression(expression);
                if !static_expression_matches(&materialized, expression) {
                    if matches!(materialized, Expression::Undefined) {
                        // Prototype lookups can still resolve the member even when own-property
                        // materialization bottoms out.
                    } else {
                        return self.resolve_function_binding_from_expression_with_aliases(
                            &materialized,
                            aliases,
                        );
                    }
                }

                self.infer_global_function_binding(expression)
            }
            Expression::Sequence(expressions) => expressions.last().and_then(|expression| {
                self.resolve_function_binding_from_expression_with_aliases(expression, aliases)
            }),
            _ => {
                let materialized = self.materialize_global_expression(expression);
                if !static_expression_matches(&materialized, expression) {
                    return self.resolve_function_binding_from_expression_with_aliases(
                        &materialized,
                        aliases,
                    );
                }
                self.infer_global_function_binding(expression)
            }
        }
    }
}
