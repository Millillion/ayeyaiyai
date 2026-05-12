use super::*;

impl<'a> FunctionCompiler<'a> {
    pub(in crate::backend::direct_wasm) fn emit_object_set_prototype_of_call(
        &mut self,
        callee_object: &Expression,
        callee_property: &Expression,
        arguments: &[CallArgument],
    ) -> DirectResult<bool> {
        if !matches!(callee_object, Expression::Identifier(name) if name == "Object") {
            return Ok(false);
        }
        if !matches!(callee_property, Expression::String(name) if name == "setPrototypeOf") {
            return Ok(false);
        }

        let [target_argument, prototype_argument, rest @ ..] = arguments else {
            return Ok(false);
        };
        let (
            CallArgument::Expression(target_expression),
            CallArgument::Expression(prototype_expression),
        ) = (target_argument, prototype_argument)
        else {
            return Ok(false);
        };

        let mut runtime_bindings = Vec::new();
        let mut push_runtime_binding = |compiler: &Self, name: &str| {
            if runtime_bindings
                .iter()
                .any(|(existing_name, _)| existing_name == name)
            {
                return;
            }
            if let Some(binding) = compiler.global_runtime_prototype_binding(name).cloned() {
                runtime_bindings.push((name.to_string(), binding));
            }
        };
        if let Expression::Identifier(name) = target_expression {
            push_runtime_binding(self, name);
        }
        if let Expression::Identifier(name) = self.materialize_static_expression(target_expression)
        {
            push_runtime_binding(self, &name);
        }
        if let Some(Expression::Identifier(name)) = self
            .resolve_bound_alias_expression(target_expression)
            .filter(|resolved| !static_expression_matches(resolved, target_expression))
        {
            push_runtime_binding(self, &name);
        }
        let materialized_prototype = self.materialize_static_expression(prototype_expression);
        fn push_prototype_candidate(
            prototype_candidates: &mut Vec<Expression>,
            candidate: Expression,
        ) {
            if !prototype_candidates
                .iter()
                .any(|existing| static_expression_matches(existing, &candidate))
            {
                prototype_candidates.push(candidate);
            }
        }

        let mut prototype_candidates = Vec::new();
        push_prototype_candidate(&mut prototype_candidates, prototype_expression.clone());
        push_prototype_candidate(&mut prototype_candidates, materialized_prototype.clone());
        if let Some(resolved) = self
            .resolve_bound_alias_expression(prototype_expression)
            .filter(|resolved| !static_expression_matches(resolved, prototype_expression))
        {
            push_prototype_candidate(&mut prototype_candidates, resolved.clone());
            push_prototype_candidate(
                &mut prototype_candidates,
                self.materialize_static_expression(&resolved),
            );
        }
        for candidate in prototype_candidates.clone() {
            let Expression::Identifier(name) = candidate else {
                continue;
            };
            if let Some(function) = self.resolve_registered_function_declaration(&name) {
                if let Some(self_binding) = function.self_binding.as_ref() {
                    push_prototype_candidate(
                        &mut prototype_candidates,
                        Expression::Identifier(self_binding.clone()),
                    );
                }
                if let Some(top_level_binding) = function.top_level_binding.as_ref() {
                    push_prototype_candidate(
                        &mut prototype_candidates,
                        Expression::Identifier(top_level_binding.clone()),
                    );
                }
            }
            if let Some(value) = self.global_value_binding(&name).cloned() {
                push_prototype_candidate(&mut prototype_candidates, value.clone());
                push_prototype_candidate(
                    &mut prototype_candidates,
                    self.materialize_static_expression(&value),
                );
            }
        }

        self.emit_numeric_expression(target_expression)?;
        self.state.emission.output.instructions.push(0x1a);
        self.emit_numeric_expression(prototype_expression)?;
        self.state.emission.output.instructions.push(0x1a);
        self.discard_call_arguments(rest)?;

        for (_, binding) in runtime_bindings {
            if let Some(global_index) = binding.global_index
                && let Some(variant_index) = binding.variants.iter().position(|candidate| {
                    candidate.as_ref().is_some_and(|candidate| {
                        prototype_candidates
                            .iter()
                            .any(|prototype| static_expression_matches(candidate, prototype))
                    }) || (candidate.is_none()
                        && prototype_candidates
                            .iter()
                            .any(|prototype| matches!(prototype, Expression::Null)))
                })
            {
                self.push_i32_const(variant_index as i32);
                self.push_global_set(global_index);
            }
        }

        self.push_i32_const(JS_TYPEOF_OBJECT_TAG);
        Ok(true)
    }
}
