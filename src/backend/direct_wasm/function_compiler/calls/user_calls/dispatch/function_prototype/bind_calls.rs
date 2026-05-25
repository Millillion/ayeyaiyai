use super::*;

impl<'a> FunctionCompiler<'a> {
    pub(in crate::backend::direct_wasm) fn resolve_function_prototype_bind_call(
        &self,
        expression: &Expression,
        current_function_name: Option<&str>,
    ) -> Option<(Expression, Vec<CallArgument>, LocalFunctionBinding)> {
        let resolved = self
            .resolve_bound_alias_expression(expression)
            .filter(|resolved| !static_expression_matches(resolved, expression))
            .unwrap_or_else(|| expression.clone());
        let Expression::Call { callee, arguments } = resolved else {
            return None;
        };
        let Expression::Member { object, property } = callee.as_ref() else {
            return None;
        };
        if !matches!(property.as_ref(), Expression::String(name) if name == "bind") {
            return None;
        }
        let binding = self
            .resolve_function_binding_from_expression_with_context(object, current_function_name)?;
        let bound_arguments = arguments.into_iter().skip(1).collect::<Vec<_>>();
        Some((object.as_ref().clone(), bound_arguments, binding))
    }

    pub(in crate::backend::direct_wasm) fn emit_function_prototype_bind_call(
        &mut self,
        callee: &Expression,
        arguments: &[CallArgument],
    ) -> DirectResult<bool> {
        if let Expression::Member { object, property } = callee
            && matches!(property.as_ref(), Expression::String(name) if name == "bind")
            && let Some(LocalFunctionBinding::Builtin(function_name)) =
                self.resolve_function_binding_from_expression(object)
            && function_name == "Function.prototype.call"
            && let [
                CallArgument::Expression(target) | CallArgument::Spread(target),
                ..,
            ] = arguments
            && let Some(LocalFunctionBinding::Builtin(_)) =
                self.resolve_function_binding_from_expression(target)
        {
            self.emit_numeric_expression(object)?;
            self.state.emission.output.instructions.push(0x1a);
            for argument in arguments {
                match argument {
                    CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                        self.emit_numeric_expression(expression)?;
                        self.state.emission.output.instructions.push(0x1a);
                    }
                }
            }
            self.push_i32_const(JS_TYPEOF_FUNCTION_TAG);
            return Ok(true);
        }
        if let Expression::Member { object, property } = callee
            && matches!(property.as_ref(), Expression::String(name) if name == "bind")
            && self
                .resolve_function_binding_from_expression(object)
                .is_some()
        {
            self.emit_numeric_expression(object)?;
            self.state.emission.output.instructions.push(0x1a);
            for argument in arguments {
                match argument {
                    CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                        self.emit_numeric_expression(expression)?;
                        self.state.emission.output.instructions.push(0x1a);
                    }
                }
            }
            self.push_i32_const(JS_TYPEOF_FUNCTION_TAG);
            return Ok(true);
        }

        let Expression::Call {
            callee: bind_callee,
            arguments: _bind_arguments,
        } = callee
        else {
            return Ok(false);
        };
        let Expression::Member { object, property } = bind_callee.as_ref() else {
            return Ok(false);
        };
        if !matches!(property.as_ref(), Expression::String(name) if name == "bind") {
            return Ok(false);
        }

        let Some(function_binding) = self.resolve_function_binding_from_expression(object) else {
            return Ok(false);
        };
        self.emit_function_prototype_bind_call_with_resolved_binding(
            callee,
            arguments,
            function_binding,
        )
    }

    pub(in crate::backend::direct_wasm) fn emit_function_prototype_bind_call_with_resolved_binding(
        &mut self,
        callee: &Expression,
        arguments: &[CallArgument],
        function_binding: LocalFunctionBinding,
    ) -> DirectResult<bool> {
        let Expression::Call {
            callee: bind_callee,
            arguments: bind_arguments,
        } = callee
        else {
            return Ok(false);
        };
        let Expression::Member { object, property } = bind_callee.as_ref() else {
            return Ok(false);
        };
        if !matches!(property.as_ref(), Expression::String(name) if name == "bind") {
            return Ok(false);
        }

        if let LocalFunctionBinding::Builtin(function_name) = &function_binding {
            if function_name == "Function.prototype.call"
                && let [
                    CallArgument::Expression(target) | CallArgument::Spread(target),
                    ..,
                ] = bind_arguments.as_slice()
                && let Some(LocalFunctionBinding::Builtin(_)) =
                    self.resolve_function_binding_from_expression(target)
            {
                self.emit_numeric_expression(object)?;
                self.state.emission.output.instructions.push(0x1a);
                for argument in bind_arguments {
                    match argument {
                        CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                            self.emit_numeric_expression(expression)?;
                            self.state.emission.output.instructions.push(0x1a);
                        }
                    }
                }
                for argument in arguments {
                    match argument {
                        CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                            self.emit_numeric_expression(expression)?;
                            self.state.emission.output.instructions.push(0x1a);
                        }
                    }
                }
                self.push_i32_const(JS_TYPEOF_FUNCTION_TAG);
                return Ok(true);
            }
            return Ok(false);
        }
        let LocalFunctionBinding::User(function_name) = function_binding else {
            return Ok(false);
        };
        let Some(user_function) = self.user_function(&function_name).cloned() else {
            return Ok(false);
        };

        let capture_slots = self.resolve_function_expression_capture_slots(object);
        let expanded_bind_arguments = self.expand_call_arguments(bind_arguments);
        let lexical_this_expression = user_function
            .lexical_this
            .then(|| {
                capture_slots
                    .as_ref()
                    .and_then(|slots| slots.get("this"))
                    .map(|slot_name| Expression::Identifier(slot_name.clone()))
            })
            .flatten();
        let raw_this_expression = lexical_this_expression.unwrap_or_else(|| {
            expanded_bind_arguments
                .first()
                .cloned()
                .unwrap_or(Expression::Undefined)
        });
        let expanded_call_arguments = self.expand_call_arguments(arguments);
        let bound_call_arguments = expanded_bind_arguments
            .iter()
            .skip(1)
            .cloned()
            .chain(expanded_call_arguments)
            .map(CallArgument::Expression)
            .collect::<Vec<_>>();
        let materialized_call_arguments = bound_call_arguments
            .iter()
            .map(|argument| match argument {
                CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                    self.materialize_static_expression(expression)
                }
            })
            .collect::<Vec<_>>();
        let bound_call_argument_expressions = bound_call_arguments
            .iter()
            .map(|argument| match argument {
                CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                    expression.clone()
                }
            })
            .collect::<Vec<_>>();

        self.emit_numeric_expression(object)?;
        self.state.emission.output.instructions.push(0x1a);

        let this_expression = if matches!(raw_this_expression, Expression::This) {
            let this_hidden_name = self.allocate_named_hidden_local(
                "bind_this",
                self.infer_value_kind(&raw_this_expression)
                    .unwrap_or(StaticValueKind::Unknown),
            );
            let this_hidden_local = self
                .state
                .runtime
                .locals
                .get(&this_hidden_name)
                .copied()
                .expect("fresh bind hidden this local must exist");
            self.emit_numeric_expression(&raw_this_expression)?;
            self.push_local_set(this_hidden_local);
            self.update_capture_slot_binding_from_expression(
                &this_hidden_name,
                &raw_this_expression,
            )?;
            self.sync_capture_slot_runtime_object_shadows_from_expression(
                &this_hidden_name,
                &raw_this_expression,
            )?;
            Expression::Identifier(this_hidden_name)
        } else {
            raw_this_expression.clone()
        };
        let materialized_this_expression = self.materialize_static_expression(&this_expression);

        if capture_slots.is_none()
            && (user_function.strict || user_function.lexical_this)
            && self.can_inline_user_function_call_with_explicit_call_frame(
                &user_function,
                &materialized_call_arguments,
                &materialized_this_expression,
            )
        {
            let result_local = self.allocate_temp_local();
            if self.emit_inline_user_function_summary_with_explicit_call_frame(
                &user_function,
                &bound_call_argument_expressions,
                &materialized_this_expression,
                result_local,
            )? {
                self.push_local_get(result_local);
                return Ok(true);
            }
        }

        self.emit_user_function_call_with_function_this_binding(
            &user_function,
            &bound_call_arguments,
            &this_expression,
            capture_slots.as_ref(),
        )?;
        Ok(true)
    }
}
