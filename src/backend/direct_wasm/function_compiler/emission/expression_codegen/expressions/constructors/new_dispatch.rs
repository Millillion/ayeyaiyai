use super::*;

impl<'a> FunctionCompiler<'a> {
    fn expression_is_dynamic_import_promise_new_callee(
        &self,
        expression: &Expression,
        depth: usize,
    ) -> bool {
        if depth > 8 {
            return false;
        }
        match expression {
            Expression::Call { callee, .. } => {
                if matches!(callee.as_ref(), Expression::Identifier(name) if name == "__ayyDynamicImport")
                {
                    return true;
                }
            }
            Expression::Sequence(expressions) => {
                return expressions.last().is_some_and(|last| {
                    self.expression_is_dynamic_import_promise_new_callee(last, depth + 1)
                });
            }
            _ => {}
        }
        if let Some(resolved) = self
            .resolve_bound_alias_expression(expression)
            .filter(|resolved| !static_expression_matches(resolved, expression))
            && self.expression_is_dynamic_import_promise_new_callee(&resolved, depth + 1)
        {
            return true;
        }
        let materialized = self.materialize_static_expression(expression);
        !static_expression_matches(&materialized, expression)
            && self.expression_is_dynamic_import_promise_new_callee(&materialized, depth + 1)
    }

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

    fn expression_is_known_non_constructible_object_new_callee(&self, callee: &Expression) -> bool {
        match callee {
            Expression::This | Expression::Object(_) | Expression::Array(_) => true,
            Expression::Identifier(name)
                if self.is_unshadowed_builtin_identifier(name)
                    && matches!(name.as_str(), "Math" | "JSON" | "Reflect" | "globalThis") =>
            {
                true
            }
            Expression::Identifier(name)
                if self.lookup_identifier_kind(name) == Some(StaticValueKind::Object)
                    && self
                        .resolve_function_binding_from_expression(callee)
                        .is_none() =>
            {
                true
            }
            _ => false,
        }
    }

    pub(in crate::backend::direct_wasm) fn builtin_function_is_constructible(
        function_name: &str,
    ) -> bool {
        native_error_runtime_value(function_name).is_some()
            || matches!(
                function_name,
                "Object"
                    | "Array"
                    | "ArrayBuffer"
                    | "SharedArrayBuffer"
                    | "DataView"
                    | "Date"
                    | "RegExp"
                    | "Map"
                    | "Set"
                    | "WeakMap"
                    | "WeakRef"
                    | "WeakSet"
                    | "Number"
                    | "String"
                    | "Boolean"
                    | "Function"
                    | "AsyncFunction"
                    | "GeneratorFunction"
                    | "AsyncGeneratorFunction"
                    | "Promise"
                    | "Uint8Array"
                    | "Int8Array"
                    | "Uint16Array"
                    | "Int16Array"
                    | "Uint32Array"
                    | "Int32Array"
                    | "Float32Array"
                    | "Float64Array"
                    | "Uint8ClampedArray"
                    | "BigInt64Array"
                    | "BigUint64Array"
            )
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
        if self.expression_is_dynamic_import_promise_new_callee(callee, 0)
            || self.expression_is_known_promise_instance_for_instanceof(callee)
            || self
                .resolve_static_primitive_expression_with_context(
                    callee,
                    self.current_function_name(),
                )
                .is_some()
            || self.resolve_static_boxed_primitive_value(callee).is_some()
            || self.expression_is_known_non_constructible_object_new_callee(callee)
        {
            self.emit_non_constructible_new_expression_throw(callee, arguments)?;
            return Ok(());
        }

        let new_expression = Expression::New {
            callee: Box::new(callee.clone()),
            arguments: arguments.to_vec(),
        };
        if self
            .resolve_static_constructed_function_metadata_object_binding(&new_expression)
            .is_some()
        {
            if trace_construct_calls {
                eprintln!("construct_call:static_constructed_function callee={callee:?}");
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
                        if matches!(callee, Expression::Call { .. } | Expression::New { .. })
                            && !self.emit_returned_function_value_call_side_effects(callee)?
                        {
                            self.emit_numeric_expression(callee)?;
                            self.state.emission.output.instructions.push(0x1a);
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
                    if !Self::builtin_function_is_constructible(&function_name) {
                        self.emit_non_constructible_new_expression_throw(callee, arguments)?;
                        return Ok(());
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
