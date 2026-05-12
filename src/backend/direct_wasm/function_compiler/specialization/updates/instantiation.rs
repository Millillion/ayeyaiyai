use super::*;

impl<'a> FunctionCompiler<'a> {
    pub(in crate::backend::direct_wasm) fn instantiate_specialized_function_value(
        &mut self,
        template: &SpecializedFunctionValue,
    ) -> DirectResult<Option<SpecializedFunctionValue>> {
        let captured = self.collect_capture_bindings_from_summary(&template.summary);
        let trace_capture_bindings = std::env::var_os("AYY_TRACE_CAPTURE_BINDINGS").is_some();
        if trace_capture_bindings {
            eprintln!(
                "capture_bindings instantiate binding={:?} captured={captured:?} return={:?} effects={}",
                template.binding,
                template.summary.return_value,
                template.summary.effects.len()
            );
        }
        if captured.is_empty() {
            return Ok(None);
        }

        let mut bindings = HashMap::new();
        for name in captured {
            if let Some((resolved_name, _)) = self.resolve_current_local_binding(&name) {
                let hidden_name = self.allocate_named_hidden_local(
                    "capture",
                    self.lookup_identifier_kind(&resolved_name)
                        .unwrap_or(StaticValueKind::Unknown),
                );
                self.emit_numeric_expression(&Expression::Identifier(resolved_name.clone()))?;
                let hidden_local = self
                    .state
                    .runtime
                    .locals
                    .get(&hidden_name)
                    .copied()
                    .expect("hidden capture local should be allocated");
                self.push_local_set(hidden_local);
                self.alias_runtime_binding_metadata(&hidden_name, &resolved_name);
                bindings.insert(name, Expression::Identifier(hidden_name));
                continue;
            }

            let Some(scope_object) = self.resolve_with_scope_binding_for_specialization(&name)
            else {
                continue;
            };
            let hidden_name = self.allocate_named_hidden_local(
                "capture_scope",
                self.infer_value_kind(&scope_object)
                    .unwrap_or(StaticValueKind::Object),
            );
            self.emit_numeric_expression(&scope_object)?;
            let hidden_local = self
                .state
                .runtime
                .locals
                .get(&hidden_name)
                .copied()
                .expect("hidden capture scope local should be allocated");
            self.push_local_set(hidden_local);
            let scope_metadata_expression = self
                .resolve_object_binding_from_expression(&scope_object)
                .map(|binding| object_binding_to_expression(&binding))
                .unwrap_or_else(|| scope_object.clone());
            self.update_capture_slot_binding_from_expression(
                &hidden_name,
                &scope_metadata_expression,
            )?;
            self.sync_capture_slot_runtime_object_shadows_from_expression(
                &hidden_name,
                &scope_metadata_expression,
            )?;
            if trace_capture_bindings {
                eprintln!(
                    "capture_bindings instantiate_with name={name} hidden={hidden_name} scope={scope_object:?} metadata={scope_metadata_expression:?}",
                );
            }
            bindings.insert(
                name.clone(),
                Expression::Member {
                    object: Box::new(Expression::Identifier(hidden_name)),
                    property: Box::new(Expression::String(name)),
                },
            );
        }

        let summary = rewrite_inline_function_summary_bindings(&template.summary, &bindings);
        if trace_capture_bindings {
            eprintln!(
                "capture_bindings instantiate_result bindings={bindings:?} return={:?} effects={}",
                summary.return_value,
                summary.effects.len()
            );
        }

        Ok(Some(SpecializedFunctionValue {
            binding: template.binding.clone(),
            summary,
        }))
    }
}
