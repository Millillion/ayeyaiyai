use super::*;

impl<'a> FunctionCompiler<'a> {
    fn emit_non_constructible_new_expression_throw(
        &mut self,
        callee: &Expression,
        arguments: &[CallArgument],
    ) -> DirectResult<()> {
        self.emit_numeric_expression(callee)?;
        self.state.emission.output.instructions.push(0x1a);
        for argument in arguments {
            match argument {
                CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                    self.emit_numeric_expression(expression)?;
                    self.state.emission.output.instructions.push(0x1a);
                }
            }
        }
        self.emit_named_error_throw("TypeError")?;
        self.push_i32_const(JS_UNDEFINED_TAG);
        Ok(())
    }

    pub(in crate::backend::direct_wasm) fn emit_new_expression(
        &mut self,
        callee: &Expression,
        arguments: &[CallArgument],
    ) -> DirectResult<()> {
        let trace_construct_calls = std::env::var_os("AYY_TRACE_CONSTRUCT_CALLS").is_some();
        if let Some((target, mut bound_arguments, LocalFunctionBinding::User(function_name))) =
            self.resolve_function_prototype_bind_call(callee, self.current_function_name())
            && let Some(user_function) = self.user_function(&function_name).cloned()
        {
            if trace_construct_calls {
                eprintln!(
                    "construct_call:bound_user callee={callee:?} target={target:?} binding={function_name} bound_arguments={bound_arguments:?} call_arguments={arguments:?}"
                );
            }
            bound_arguments.extend(arguments.iter().cloned());
            if !user_function.is_constructible() {
                self.emit_non_constructible_new_expression_throw(callee, arguments)?;
                return Ok(());
            }
            if self.emit_user_function_construct(&target, &user_function, &bound_arguments)? {
                if let Some(snapshot) = self
                    .state
                    .speculation
                    .static_semantics
                    .last_bound_user_function_call
                    .as_mut()
                {
                    snapshot.source_expression = Some(Expression::New {
                        callee: Box::new(callee.clone()),
                        arguments: arguments.to_vec(),
                    });
                }
                return Ok(());
            }
        }
        if let Expression::Identifier(name) = callee
            && name == "Proxy"
            && self.is_unshadowed_builtin_identifier(name)
        {
            for argument in arguments {
                match argument {
                    CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                        self.emit_numeric_expression(expression)?;
                        self.state.emission.output.instructions.push(0x1a);
                    }
                }
            }
            self.push_i32_const(JS_TYPEOF_OBJECT_TAG);
            return Ok(());
        }

        if let Some(function_binding) = self.resolve_function_binding_from_expression(callee) {
            match function_binding {
                LocalFunctionBinding::User(function_name) => {
                    if trace_construct_calls {
                        eprintln!("construct_call:user callee={callee:?} binding={function_name}");
                    }
                    if let Some(user_function) = self.user_function(&function_name).cloned() {
                        if !user_function.is_constructible() {
                            self.emit_non_constructible_new_expression_throw(callee, arguments)?;
                            return Ok(());
                        }
                        if self.emit_user_function_construct(callee, &user_function, arguments)? {
                            return Ok(());
                        }
                    }
                }
                LocalFunctionBinding::Builtin(function_name) => {
                    if trace_construct_calls {
                        eprintln!(
                            "construct_call:builtin callee={callee:?} binding={function_name}"
                        );
                    }
                    if self.emit_builtin_call_for_callee(callee, &function_name, arguments, true)? {
                        return Ok(());
                    }
                }
            }
        }
        if trace_construct_calls {
            eprintln!("construct_call:fallback callee={callee:?}");
        }

        if let Expression::Identifier(name) = callee {
            if self.emit_builtin_call(name, arguments)? {
                return Ok(());
            }

            if let Some(native_error_value) = native_error_runtime_value(name) {
                for argument in arguments {
                    match argument {
                        CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                            self.emit_numeric_expression(expression)?;
                            self.state.emission.output.instructions.push(0x1a);
                        }
                    }
                }
                self.push_i32_const(native_error_value);
                return Ok(());
            }
        }
        if matches!(
            callee,
            Expression::Member { .. } | Expression::SuperMember { .. }
        ) {
            self.emit_numeric_expression(callee)?;
            self.state.emission.output.instructions.push(0x1a);
            for argument in arguments {
                match argument {
                    CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                        self.emit_numeric_expression(expression)?;
                    }
                }
                self.state.emission.output.instructions.push(0x1a);
            }
            self.emit_named_error_throw("TypeError")?;
            self.push_i32_const(JS_UNDEFINED_TAG);
            return Ok(());
        }
        self.emit_numeric_expression(callee)?;
        self.state.emission.output.instructions.push(0x1a);
        for argument in arguments {
            match argument {
                CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                    self.emit_numeric_expression(expression)?;
                }
            }
            self.state.emission.output.instructions.push(0x1a);
        }
        self.push_i32_const(JS_TYPEOF_OBJECT_TAG);
        Ok(())
    }
}
