use super::*;

const NULL_SUPER_CONSTRUCTOR_BINDING: &str = "__ayy_null_super_constructor";

impl<'a> FunctionCompiler<'a> {
    fn current_derived_super_constructor_binding(&self) -> Option<LocalFunctionBinding> {
        let current_function_name = self.current_function_name()?;
        let function = self.resolve_registered_function_declaration(current_function_name)?;
        if !function.derived_constructor {
            return None;
        }
        let self_binding = function.self_binding.as_deref()?;
        let super_constructor = self
            .global_object_prototype_expression(self_binding)
            .cloned()?;
        let materialized_super_constructor = match &super_constructor {
            Expression::Identifier(name) => self
                .resolve_static_class_init_local_alias_expression(name)
                .or_else(|| {
                    self.global_value_binding(name)
                        .filter(|resolved| !static_expression_matches(resolved, &super_constructor))
                        .cloned()
                })
                .filter(|resolved| !static_expression_matches(resolved, &super_constructor))
                .map(|resolved| self.materialize_static_expression(&resolved))
                .unwrap_or_else(|| self.materialize_static_expression(&super_constructor)),
            _ => self.materialize_static_expression(&super_constructor),
        };
        self.resolve_function_binding_from_expression(&materialized_super_constructor)
    }

    fn emit_super_arguments_then_type_error(
        &mut self,
        arguments: &[CallArgument],
    ) -> DirectResult<()> {
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
        Ok(())
    }

    fn emit_current_derived_super_constructor_override(
        &mut self,
        arguments: &[CallArgument],
    ) -> DirectResult<bool> {
        let Some(function_binding) = self.current_derived_super_constructor_binding() else {
            return Ok(false);
        };
        match function_binding {
            LocalFunctionBinding::User(function_name) => {
                let Some(user_function) = self.user_function(&function_name).cloned() else {
                    return Ok(false);
                };
                if !user_function.is_constructible() {
                    self.emit_super_arguments_then_type_error(arguments)?;
                    return Ok(true);
                }
                self.emit_derived_constructor_super_call(&user_function, arguments)?;
                Ok(true)
            }
            LocalFunctionBinding::Builtin(function_name) => {
                if !Self::builtin_function_is_constructible(&function_name) {
                    self.emit_super_arguments_then_type_error(arguments)?;
                    return Ok(true);
                }
                if self.emit_derived_constructor_builtin_super_call(&function_name, arguments)? {
                    return Ok(true);
                }
                Ok(false)
            }
        }
    }

    pub(in crate::backend::direct_wasm) fn emit_null_super_constructor_call(
        &mut self,
        arguments: &[CallArgument],
    ) -> DirectResult<()> {
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
        Ok(())
    }

    pub(in crate::backend::direct_wasm) fn emit_super_call_expression(
        &mut self,
        callee: &Expression,
        arguments: &[CallArgument],
    ) -> DirectResult<()> {
        if matches!(callee, Expression::Identifier(name) if name == NULL_SUPER_CONSTRUCTOR_BINDING)
        {
            return self.emit_null_super_constructor_call(arguments);
        }
        if self.current_function_is_derived_constructor()
            && self.emit_current_derived_super_constructor_override(arguments)?
        {
            return Ok(());
        }
        if let Some(function_binding) = self.resolve_function_binding_from_expression(callee) {
            match function_binding {
                LocalFunctionBinding::User(function_name) => {
                    if let Some(user_function) = self.user_function(&function_name).cloned() {
                        if self.current_function_is_derived_constructor()
                            || self.current_lexical_function_captures_this()
                        {
                            self.emit_derived_constructor_super_call(&user_function, arguments)?;
                            return Ok(());
                        }
                        self.emit_user_function_call_with_current_new_target_and_this_expression(
                            &user_function,
                            arguments,
                            &Expression::This,
                        )?;
                        return Ok(());
                    }
                }
                LocalFunctionBinding::Builtin(function_name) => {
                    if (self.current_function_is_derived_constructor()
                        || self.current_lexical_function_captures_this())
                        && self.emit_derived_constructor_builtin_super_call(
                            &function_name,
                            arguments,
                        )?
                    {
                        return Ok(());
                    }
                    if self.emit_builtin_call(&function_name, arguments)? {
                        return Ok(());
                    }
                }
            }
        }

        if self.emit_dynamic_super_call(callee, arguments)? {
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
        self.push_i32_const(JS_UNDEFINED_TAG);
        Ok(())
    }
}
