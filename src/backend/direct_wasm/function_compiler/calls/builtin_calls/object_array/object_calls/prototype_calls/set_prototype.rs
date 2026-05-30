use super::*;

impl<'a> FunctionCompiler<'a> {
    pub(in crate::backend::direct_wasm) fn emit_object_set_prototype_of_call(
        &mut self,
        callee_object: &Expression,
        callee_property: &Expression,
        arguments: &[CallArgument],
    ) -> DirectResult<bool> {
        let reflect_call =
            matches!(callee_object, Expression::Identifier(name) if name == "Reflect");
        if !matches!(callee_object, Expression::Identifier(name) if name == "Object" || name == "Reflect")
        {
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
        let mut synced_static_prototype_target = false;
        if let Expression::Member {
            object: target_object,
            property: target_property,
        } = target_expression
            && matches!(target_property.as_ref(), Expression::String(name) if name == "prototype")
            && let Some((realm_id, constructor_name)) =
                self.test262_realm_constructor_member(target_object)
        {
            let realm_prototype_key = format!(
                "{}.{}.prototype",
                test262_realm_global_identifier(realm_id),
                constructor_name
            );
            self.backend.sync_global_object_prototype_expression(
                &realm_prototype_key,
                Some(materialized_prototype.clone()),
            );
            synced_static_prototype_target = true;
        } else if let Expression::Member {
            object: target_object,
            property: target_property,
        } = target_expression
            && matches!(target_property.as_ref(), Expression::String(name) if name == "prototype")
            && let Expression::Identifier(constructor_name) = target_object.as_ref()
        {
            let prototype_key = format!("{constructor_name}.prototype");
            self.backend.sync_global_object_prototype_expression(
                &prototype_key,
                Some(materialized_prototype.clone()),
            );
            synced_static_prototype_target = true;
        }
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

        if !synced_static_prototype_target {
            let static_prototype = prototype_candidates
                .iter()
                .find(|candidate| {
                    matches!(
                        candidate,
                        Expression::Identifier(name)
                            if Self::module_index_from_namespace_like_identifier(name).is_some()
                    )
                })
                .cloned()
                .unwrap_or_else(|| match materialized_prototype.clone() {
                    Expression::Sequence(expressions) => {
                        expressions.last().cloned().unwrap_or(Expression::Undefined)
                    }
                    prototype => prototype,
                });
            let mut target_names = Vec::new();
            if let Expression::Identifier(name) = target_expression {
                target_names.push(name.clone());
            }
            if let Expression::Identifier(name) =
                self.materialize_static_expression(target_expression)
                && !target_names.iter().any(|existing| existing == &name)
            {
                target_names.push(name);
            }
            if let Some(Expression::Identifier(name)) = self
                .resolve_bound_alias_expression(target_expression)
                .filter(|resolved| !static_expression_matches(resolved, target_expression))
                && !target_names.iter().any(|existing| existing == &name)
            {
                target_names.push(name);
            }
            for target_name in target_names {
                if Self::module_index_from_namespace_like_identifier(&target_name).is_none() {
                    self.backend.sync_global_object_prototype_expression(
                        &target_name,
                        Some(static_prototype.clone()),
                    );
                }
            }
        }

        if !synced_static_prototype_target {
            self.emit_numeric_expression(target_expression)?;
            self.state.emission.output.instructions.push(0x1a);
        }
        self.emit_numeric_expression(prototype_expression)?;
        self.state.emission.output.instructions.push(0x1a);
        self.discard_call_arguments(rest)?;

        let target_binding = self
            .resolve_object_binding_from_expression(target_expression)
            .or_else(|| match target_expression {
                Expression::Identifier(name) => {
                    self.resolve_identifier_object_binding_fallback(name)
                }
                _ => None,
            });
        let target_is_module_namespace = target_binding
            .as_ref()
            .is_some_and(Self::object_binding_has_module_namespace_marker)
            || matches!(
                target_expression,
                Expression::Identifier(name)
                    if FunctionCompiler::module_index_from_namespace_like_identifier(name).is_some()
            );
        if target_is_module_namespace {
            if !matches!(materialized_prototype, Expression::Null) {
                if reflect_call {
                    self.push_i32_const(0);
                } else {
                    self.emit_named_error_throw("TypeError")?;
                }
            } else if reflect_call {
                self.push_i32_const(1);
            } else {
                self.push_i32_const(JS_TYPEOF_OBJECT_TAG);
            }
            return Ok(true);
        }

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

        if reflect_call {
            self.push_i32_const(1);
        } else {
            self.push_i32_const(JS_TYPEOF_OBJECT_TAG);
        }
        Ok(true)
    }
}
