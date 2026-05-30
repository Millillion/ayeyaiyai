use super::*;

impl<'a> FunctionCompiler<'a> {
    pub(in crate::backend::direct_wasm) fn resolve_instanceof_getter_static_prototype_expression(
        &self,
        binding: &LocalFunctionBinding,
        right: &Expression,
    ) -> Option<Expression> {
        if let Some(value) = self.resolve_function_binding_static_return_expression_with_call_frame(
            binding,
            &[],
            right,
        ) {
            return Some(value);
        }

        let LocalFunctionBinding::User(function_name) = binding else {
            return None;
        };
        let user_function = self.user_function(function_name)?;
        let trace_instanceof = std::env::var_os("AYY_TRACE_INSTANCEOF").is_some();
        if self.user_function_mentions_direct_eval(user_function)
            || self.user_function_deletes_call_frame_arguments_member(user_function)
        {
            if trace_instanceof {
                eprintln!(
                    "instanceof:getter_return_reject function={function_name} direct_eval={} deletes_arguments={}",
                    self.user_function_mentions_direct_eval(user_function),
                    self.user_function_deletes_call_frame_arguments_member(user_function)
                );
            }
            return None;
        }

        let function = self.resolve_registered_function_declaration(function_name)?;
        let Statement::Return(return_value) = function.body.last()? else {
            if trace_instanceof {
                eprintln!(
                    "instanceof:getter_return_missing function={function_name} last={:?}",
                    function.body.last()
                );
            }
            return None;
        };
        let call_arguments = Vec::new();
        let arguments_binding = Expression::Array(Vec::new());
        Some(self.substitute_user_function_call_frame_bindings(
            return_value,
            user_function,
            &call_arguments,
            right,
            &arguments_binding,
        ))
    }

    fn normalize_instanceof_prototype_expression(&self, expression: &Expression) -> Expression {
        let expression = self
            .resolve_bound_alias_expression(expression)
            .filter(|resolved| !static_expression_matches(resolved, expression))
            .unwrap_or_else(|| self.materialize_static_expression(expression));
        if let Expression::Member { object, property } = &expression
            && matches!(property.as_ref(), Expression::String(name) if name == "prototype")
        {
            let resolved_object = self
                .resolve_bound_alias_expression(object)
                .filter(|resolved| !static_expression_matches(resolved, object))
                .unwrap_or_else(|| self.materialize_static_expression(object));
            if let Some(binding) = self.resolve_function_binding_from_expression(&resolved_object)
                && let Some(prototype_owner) = self.function_prototype_binding_owner_name(&binding)
            {
                return Expression::Member {
                    object: Box::new(Expression::Identifier(prototype_owner)),
                    property: property.clone(),
                };
            }
            if !static_expression_matches(&resolved_object, object) {
                return Expression::Member {
                    object: Box::new(resolved_object),
                    property: property.clone(),
                };
            }
        }
        expression
    }

    fn expression_static_prototype_chain_contains(
        &self,
        expression: &Expression,
        target_prototype: &Expression,
        visited: &mut Vec<Expression>,
    ) -> bool {
        let Some(prototype) = self.resolve_static_object_prototype_expression(expression) else {
            if std::env::var_os("AYY_TRACE_INSTANCEOF").is_some() {
                eprintln!(
                    "instanceof:chain-none expression={expression:?} target={target_prototype:?}"
                );
            }
            return false;
        };
        let prototype = self.normalize_instanceof_prototype_expression(&prototype);
        let target = self.normalize_instanceof_prototype_expression(target_prototype);
        if std::env::var_os("AYY_TRACE_INSTANCEOF").is_some() {
            eprintln!(
                "instanceof:chain expression={expression:?} prototype={prototype:?} target={target:?}"
            );
        }

        if static_expression_matches(&prototype, &target) {
            return true;
        }
        if visited
            .iter()
            .any(|visited| static_expression_matches(visited, &prototype))
        {
            return false;
        }
        visited.push(prototype.clone());
        self.expression_static_prototype_chain_contains(&prototype, &target, visited)
    }

    pub(in crate::backend::direct_wasm) fn expression_inherits_from_prototype_for_instanceof(
        &self,
        left: &Expression,
        prototype: &Expression,
    ) -> bool {
        if self.module_namespace_index_from_expression(left).is_some() {
            return false;
        }
        if let Some(resolved) = self
            .resolve_bound_alias_expression(prototype)
            .filter(|resolved| !static_expression_matches(resolved, prototype))
        {
            return self.expression_inherits_from_prototype_for_instanceof(left, &resolved);
        }
        let materialized_prototype = self.materialize_static_expression(prototype);
        if !static_expression_matches(&materialized_prototype, prototype) {
            return self
                .expression_inherits_from_prototype_for_instanceof(left, &materialized_prototype);
        }
        if self.expression_static_prototype_chain_contains(
            left,
            &materialized_prototype,
            &mut Vec::new(),
        ) {
            return true;
        }
        let Expression::Member { object, property } = &materialized_prototype else {
            return false;
        };
        if !matches!(property.as_ref(), Expression::String(name) if name == "prototype") {
            return false;
        }
        let Expression::Identifier(constructor_name) = object.as_ref() else {
            return false;
        };
        match constructor_name.as_str() {
            "Array" => self.expression_is_known_array_value(left),
            "Function" | "AsyncFunction" | "GeneratorFunction" | "AsyncGeneratorFunction" => {
                self.expression_is_known_function_value_for_instanceof(left)
            }
            "Object" => self.expression_is_known_object_like_value_for_instanceof(left),
            "Promise" => self.expression_is_known_promise_instance_for_instanceof(left),
            "WeakMap" | "WeakRef" | "WeakSet" => {
                self.expression_is_known_constructor_instance_for_instanceof(left, constructor_name)
            }
            "Error" => self.expression_is_known_native_error_instance_for_instanceof(left, "Error"),
            name if native_error_runtime_value(name).is_some() => {
                self.expression_is_known_native_error_instance_for_instanceof(left, name)
            }
            name => self.expression_is_known_constructor_instance_for_instanceof(left, name),
        }
    }

    pub(in crate::backend::direct_wasm) fn resolve_instanceof_prototype_expression(
        &self,
        right: &Expression,
    ) -> Option<Expression> {
        let prototype_property = Expression::String("prototype".to_string());
        if let Some(binding) = self.resolve_member_getter_binding(right, &prototype_property) {
            return self.resolve_instanceof_getter_static_prototype_expression(&binding, right);
        }
        if let Some(object_binding) = self.resolve_object_binding_from_expression(right)
            && let Some(value) =
                object_binding_lookup_value(&object_binding, &prototype_property).cloned()
        {
            return Some(value);
        }
        let materialized_right = self.materialize_static_expression(right);
        if !static_expression_matches(&materialized_right, right) {
            return self.resolve_instanceof_prototype_expression(&materialized_right);
        }
        let prototype_member = Expression::Member {
            object: Box::new(right.clone()),
            property: Box::new(prototype_property.clone()),
        };
        let materialized_prototype_member = self.materialize_static_expression(&prototype_member);
        if !static_expression_matches(&materialized_prototype_member, &prototype_member)
            && (self.expression_is_known_non_object_value_for_instanceof(
                &materialized_prototype_member,
            ) || self.expression_is_known_object_like_value_for_instanceof(
                &materialized_prototype_member,
            ) || matches!(
                &materialized_prototype_member,
                Expression::Member { property, .. }
                    if matches!(property.as_ref(), Expression::String(name) if name == "prototype")
            ))
        {
            return Some(materialized_prototype_member);
        }
        if let Some(binding) = self.resolve_function_binding_from_expression(&materialized_right) {
            let prototype_owner = self
                .function_prototype_binding_owner_name(&binding)
                .unwrap_or_else(|| match &materialized_right {
                    Expression::Identifier(name) => name.clone(),
                    _ => String::new(),
                });
            return Some(Expression::Member {
                object: Box::new(Expression::Identifier(prototype_owner)),
                property: Box::new(prototype_property),
            });
        }
        if matches!(
            &materialized_right,
            Expression::Identifier(name) if infer_call_result_kind(name).is_some()
        ) {
            return Some(Expression::Member {
                object: Box::new(materialized_right),
                property: Box::new(prototype_property),
            });
        }
        None
    }
}
