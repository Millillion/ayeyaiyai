use super::*;

impl<'a> FunctionCompiler<'a> {
    pub(in crate::backend::direct_wasm) fn emit_specialized_function_value_call(
        &mut self,
        specialized: &SpecializedFunctionValue,
        arguments: &[CallArgument],
    ) -> DirectResult<bool> {
        let trace_capture_bindings = std::env::var_os("AYY_TRACE_CAPTURE_BINDINGS").is_some();
        let LocalFunctionBinding::User(function_name) = &specialized.binding else {
            if trace_capture_bindings {
                eprintln!(
                    "capture_bindings specialized_call:non_user binding={:?}",
                    specialized.binding
                );
            }
            return Ok(false);
        };
        let Some(user_function) = self
            .backend
            .function_registry
            .catalog
            .user_function(function_name)
            .cloned()
        else {
            if trace_capture_bindings {
                eprintln!(
                    "capture_bindings specialized_call:missing_user function={function_name}"
                );
            }
            return Ok(false);
        };
        if !self.user_function_supports_emitted_specialized_function_summary(
            &user_function,
            &specialized.summary,
        ) {
            if trace_capture_bindings {
                eprintln!(
                    "capture_bindings specialized_call:unsupported function={function_name} return={:?} effects={}",
                    specialized.summary.return_value,
                    specialized.summary.effects.len()
                );
            }
            return Ok(false);
        }
        if trace_capture_bindings {
            eprintln!(
                "capture_bindings specialized_call:emit function={function_name} return={:?} effects={} args={}",
                specialized.summary.return_value,
                specialized.summary.effects.len(),
                arguments.len()
            );
        }
        let result_expression = specialized
            .summary
            .return_value
            .as_ref()
            .map(|return_value| {
                self.substitute_user_function_argument_bindings(
                    return_value,
                    &user_function,
                    arguments,
                )
            });
        self.state
            .speculation
            .static_semantics
            .last_bound_user_function_call = Some(BoundUserFunctionCallSnapshot {
            function_name: function_name.clone(),
            source_expression: None,
            result_expression,
            prototype_source_expression: None,
            updated_bindings: HashMap::new(),
        });
        self.emit_inline_summary_with_call_arguments(
            &user_function,
            &specialized.summary,
            arguments,
        )?;
        Ok(true)
    }
}
