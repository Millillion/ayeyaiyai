use super::*;

impl<'a> FunctionCompiler<'a> {
    pub(in crate::backend::direct_wasm) fn with_eval_template_cache_epoch<T>(
        &mut self,
        callback: impl FnOnce(&mut Self) -> DirectResult<T>,
    ) -> DirectResult<T> {
        let current_epoch = self.ensure_implicit_global_binding(EVAL_TEMPLATE_CURRENT_EPOCH_GLOBAL);
        let next_epoch = self.ensure_implicit_global_binding(EVAL_TEMPLATE_NEXT_EPOCH_GLOBAL);
        let saved_epoch_local = self.allocate_temp_local();

        self.push_global_get(current_epoch.value_index);
        self.push_local_set(saved_epoch_local);

        self.push_global_get(next_epoch.value_index);
        self.push_i32_const(1);
        self.state.emission.output.instructions.push(0x6a);
        self.push_global_set(next_epoch.value_index);

        self.push_global_get(next_epoch.value_index);
        self.push_global_set(current_epoch.value_index);

        let result = callback(self);

        self.push_local_get(saved_epoch_local);
        self.push_global_set(current_epoch.value_index);

        result
    }

    fn with_class_field_initializer_eval_new_target_undefined<T>(
        &mut self,
        enabled: bool,
        callback: impl FnOnce(&mut Self) -> DirectResult<T>,
    ) -> DirectResult<T> {
        if !enabled {
            return callback(self);
        }

        let saved_new_target_local = self.allocate_temp_local();
        self.push_global_get(CURRENT_NEW_TARGET_GLOBAL_INDEX);
        self.push_local_set(saved_new_target_local);
        self.push_i32_const(JS_UNDEFINED_TAG);
        self.push_global_set(CURRENT_NEW_TARGET_GLOBAL_INDEX);
        let result = callback(self);
        self.push_local_get(saved_new_target_local);
        self.push_global_set(CURRENT_NEW_TARGET_GLOBAL_INDEX);
        result
    }

    fn with_static_class_field_initializer_eval_this<T>(
        &mut self,
        enabled: bool,
        callback: impl FnOnce(&mut Self) -> DirectResult<T>,
    ) -> DirectResult<T> {
        let Some(current_function_name) = self.current_function_name().map(str::to_string) else {
            return callback(self);
        };
        if !enabled || !current_function_name.starts_with("__ayy_class_init_") {
            return callback(self);
        }
        let Some(class_binding_name) = self.class_binding_name_for_function(&current_function_name)
        else {
            return callback(self);
        };

        let saved_this_local = self.allocate_temp_local();
        self.push_global_get(CURRENT_THIS_GLOBAL_INDEX);
        self.push_local_set(saved_this_local);
        if let Some((_, class_local)) = self.resolve_current_local_binding(&class_binding_name) {
            self.push_local_get(class_local);
        } else {
            self.emit_identifier_expression_value(&class_binding_name)?;
        }
        self.push_global_set(CURRENT_THIS_GLOBAL_INDEX);
        if let Some(initialized_local) = self.local_lexical_initialized_local(&class_binding_name) {
            self.push_i32_const(1);
            self.push_local_set(initialized_local);
        }
        let result = callback(self);
        self.push_local_get(saved_this_local);
        self.push_global_set(CURRENT_THIS_GLOBAL_INDEX);
        result
    }

    pub(in crate::backend::direct_wasm) fn emit_eval_call(
        &mut self,
        arguments: &[CallArgument],
    ) -> DirectResult<bool> {
        let Some(argument) = arguments.first() else {
            self.push_i32_const(JS_UNDEFINED_TAG);
            return Ok(true);
        };

        let emit_argument_discard =
            |compiler: &mut Self, argument: &CallArgument| -> DirectResult<()> {
                match argument {
                    CallArgument::Expression(expression) => {
                        compiler.emit_numeric_expression(expression)?;
                        compiler.state.emission.output.instructions.push(0x1a);
                    }
                    CallArgument::Spread(expression) => {
                        compiler.emit_numeric_expression(expression)?;
                        compiler.state.emission.output.instructions.push(0x1a);
                    }
                }
                Ok(())
            };

        match argument {
            CallArgument::Expression(expression)
                if self.emit_eval_comment_pattern(expression)? =>
            {
                for argument in arguments.iter().skip(1) {
                    emit_argument_discard(self, argument)?;
                }
                Ok(true)
            }
            CallArgument::Expression(expression)
                if self.resolve_static_string_value(expression).is_some() =>
            {
                let raw_source = self
                    .resolve_static_string_value(expression)
                    .expect("guard checked static string eval source");
                let argument_source = if self.state.speculation.execution_context.strict_mode {
                    let mut strict_argument_source = String::from("\"use strict\";");
                    strict_argument_source.push_str(&raw_source);
                    Cow::Owned(strict_argument_source)
                } else {
                    Cow::Borrowed(raw_source.as_str())
                };

                for argument in arguments.iter().skip(1) {
                    emit_argument_discard(self, argument)?;
                }

                let contextual_program =
                    self.parse_eval_program_in_current_function_context(&argument_source);
                let program = if let Some(program) = contextual_program {
                    program
                } else if let Ok(program) = frontend::parse_script_goal(&argument_source) {
                    program
                } else {
                    self.emit_named_error_throw("SyntaxError")?;
                    return Ok(true);
                };
                let mut program = lower_eval_static_function_constructors(program);
                namespace_eval_program_internal_function_names(
                    &mut program,
                    self.current_function_name(),
                    &raw_source,
                );
                self.normalize_eval_scoped_bindings_to_source_names(&mut program);

                if self.eval_arguments_initializer_conflict(&program) {
                    self.emit_named_error_throw("SyntaxError")?;
                    return Ok(true);
                }

                if self.eval_arguments_declaration_conflicts(&program) {
                    self.emit_named_error_throw("SyntaxError")?;
                    return Ok(true);
                }

                if self.eval_parameter_var_declaration_conflicts(&program) {
                    self.emit_named_error_throw("SyntaxError")?;
                    return Ok(true);
                }

                if self.eval_program_declares_var_collision_with_global_lexical(&program) {
                    self.emit_named_error_throw("SyntaxError")?;
                    return Ok(true);
                }

                if self.eval_program_declares_var_collision_with_active_lexical(&program) {
                    self.emit_named_error_throw("SyntaxError")?;
                    return Ok(true);
                }

                if self.eval_program_declares_non_definable_global_function(&program) {
                    self.emit_named_error_throw("TypeError")?;
                    return Ok(true);
                }

                if self.eval_program_declares_non_declarable_global_var(&program, false) {
                    self.emit_named_error_throw("TypeError")?;
                    return Ok(true);
                }

                let preexisting_locals = self
                    .state
                    .runtime
                    .locals
                    .keys()
                    .cloned()
                    .collect::<HashSet<_>>();
                let eval_local_function_declarations = if program.strict {
                    HashMap::new()
                } else {
                    collect_eval_local_function_declarations(
                        &program.statements,
                        &program
                            .functions
                            .iter()
                            .filter(|function| is_eval_local_function_candidate(function))
                            .map(|function| function.name.clone())
                            .collect::<HashSet<_>>(),
                    )
                };
                self.prepare_eval_lexical_bindings(
                    &mut program.statements,
                    &eval_local_function_declarations,
                )?;
                self.prepare_eval_var_bindings(&mut program.statements, program.strict)?;
                self.register_bindings_skipping_eval_local_function_declarations(
                    &program.statements,
                    &eval_local_function_declarations,
                )?;
                self.instantiate_eval_var_bindings(&program, &preexisting_locals, true)?;
                self.instantiate_eval_global_functions(&program.functions, true)?;
                self.instantiate_eval_local_functions(&eval_local_function_declarations)?;
                let class_field_initializer_eval = self
                    .state
                    .speculation
                    .execution_context
                    .direct_eval_in_class_field_initializer;

                self.with_strict_mode(program.strict, |compiler| {
                    compiler.with_class_field_initializer_eval_new_target_undefined(
                        class_field_initializer_eval,
                        |compiler| {
                            compiler.with_static_class_field_initializer_eval_this(
                                class_field_initializer_eval,
                                |compiler| {
                                    compiler.with_active_eval_lexical_scope(
                                        collect_direct_eval_lexical_binding_names(
                                            &program.statements,
                                        ),
                                        |compiler| {
                                            compiler.with_eval_template_cache_epoch(|compiler| {
                                                let completion_local =
                                                    compiler.allocate_temp_local();
                                                compiler.push_i32_const(JS_UNDEFINED_TAG);
                                                compiler.push_local_set(completion_local);
                                                let eval_statements = program
                                                    .statements
                                                    .iter()
                                                    .filter(|statement| {
                                                        !is_eval_local_function_declaration_statement(
                                                            statement,
                                                            &eval_local_function_declarations,
                                                        )
                                                    })
                                                    .collect::<Vec<_>>();

                                                for statement in eval_statements {
                                                    compiler.emit_eval_statement_completion_value(
                                                        statement,
                                                        completion_local,
                                                    )?;
                                                }

                                                compiler.push_local_get(completion_local);

                                                Ok(())
                                            })
                                        },
                                    )
                                },
                            )
                        },
                    )
                })?;

                Ok(true)
            }
            _ => {
                match argument {
                    CallArgument::Expression(expression) => {
                        self.emit_numeric_expression(expression)?
                    }
                    CallArgument::Spread(expression) => {
                        self.emit_numeric_expression(expression)?;
                        self.state.emission.output.instructions.push(0x1a);
                        self.push_i32_const(JS_UNDEFINED_TAG);
                    }
                }

                for argument in arguments.iter().skip(1) {
                    emit_argument_discard(self, argument)?;
                }

                Ok(true)
            }
        }
    }
}
