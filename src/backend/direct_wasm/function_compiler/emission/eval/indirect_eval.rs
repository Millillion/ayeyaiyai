use super::*;

impl<'a> FunctionCompiler<'a> {
    fn with_indirect_eval_global_this<T>(
        &mut self,
        callback: impl FnOnce(&mut Self) -> DirectResult<T>,
    ) -> DirectResult<T> {
        let saved_this_local = self.allocate_temp_local();
        self.push_global_get(CURRENT_THIS_GLOBAL_INDEX);
        self.push_local_set(saved_this_local);
        self.push_i32_const(JS_TYPEOF_OBJECT_TAG);
        self.push_global_set(CURRENT_THIS_GLOBAL_INDEX);

        let result = callback(self);

        self.push_local_get(saved_this_local);
        self.push_global_set(CURRENT_THIS_GLOBAL_INDEX);

        result
    }

    pub(in crate::backend::direct_wasm) fn emit_indirect_eval_call(
        &mut self,
        arguments: &[CallArgument],
    ) -> DirectResult<bool> {
        let eval_function_name = self.current_function_name().map(str::to_string);
        self.emit_indirect_eval_call_with_context(arguments, eval_function_name.as_deref())
    }

    pub(in crate::backend::direct_wasm) fn emit_indirect_eval_call_with_context(
        &mut self,
        arguments: &[CallArgument],
        eval_function_name: Option<&str>,
    ) -> DirectResult<bool> {
        self.emit_indirect_eval_call_with_context_and_mode(arguments, eval_function_name, false)
    }

    pub(in crate::backend::direct_wasm) fn emit_test262_eval_script_call(
        &mut self,
        arguments: &[CallArgument],
    ) -> DirectResult<bool> {
        self.emit_indirect_eval_call_with_context_and_mode(arguments, None, true)
    }

    fn emit_indirect_eval_call_with_context_and_mode(
        &mut self,
        arguments: &[CallArgument],
        eval_function_name: Option<&str>,
        script_global_lexical_bindings: bool,
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

        let static_argument_source = self.static_eval_argument_source_from_arguments(arguments);

        if let Some(argument_source) = static_argument_source {
            for argument in arguments.iter().skip(1) {
                emit_argument_discard(self, argument)?;
            }

            let program = if let Ok(program) = frontend::parse_script_goal(&argument_source) {
                program
            } else {
                self.emit_named_error_throw("SyntaxError")?;
                return Ok(true);
            };
            if let Some(realm_id) = eval_function_name.and_then(parse_test262_realm_eval_builtin)
                && let Some((object, property, value)) = match program.statements.as_slice() {
                    [
                        Statement::AssignMember {
                            object,
                            property,
                            value,
                        },
                    ] => Some((object, property, value)),
                    [
                        Statement::Expression(Expression::AssignMember {
                            object,
                            property,
                            value,
                        }),
                    ] => Some((object.as_ref(), property.as_ref(), value.as_ref())),
                    _ => None,
                }
                && self.emit_primitive_prototype_proxy_set_assignment(
                    object,
                    property,
                    value,
                    Some(realm_id),
                )?
            {
                return Ok(true);
            }
            let mut program = lower_eval_static_function_constructors(program);
            namespace_eval_program_internal_function_names(
                &mut program,
                eval_function_name,
                &argument_source,
            );
            if program
                .functions
                .iter()
                .filter(|function| function.register_global)
                .any(|function| is_non_definable_global_name(&function.name))
            {
                self.emit_named_error_throw("TypeError")?;
                return Ok(true);
            }

            self.with_isolated_indirect_eval_state(|compiler| {
                let preexisting_locals = compiler
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
                if script_global_lexical_bindings
                    && compiler
                        .eval_script_global_lexical_declaration_collides_with_existing_global(
                            &program.statements,
                            &eval_local_function_declarations,
                        )
                {
                    compiler.emit_named_error_throw("SyntaxError")?;
                    return Ok(());
                }
                if script_global_lexical_bindings {
                    compiler.prepare_eval_script_global_lexical_bindings(
                        &program.statements,
                        &eval_local_function_declarations,
                    );
                } else {
                    compiler.prepare_eval_lexical_bindings(
                        &mut program.statements,
                        &eval_local_function_declarations,
                    )?;
                }
                compiler.register_bindings_skipping_eval_local_function_declarations(
                    &program.statements,
                    &eval_local_function_declarations,
                )?;
                let var_collision_with_global_lexical = !program.strict
                    && compiler
                        .state
                        .speculation
                        .execution_context
                        .top_level_function
                    && collect_eval_var_names(&program).into_iter().any(|name| {
                        compiler
                            .backend
                            .global_semantics
                            .global_names()
                            .has_exact_lexical_binding(&name)
                            || compiler
                                .backend
                                .shared_global_semantics
                                .global_names()
                                .has_exact_lexical_binding(&name)
                    });
                if var_collision_with_global_lexical {
                    compiler.emit_named_error_throw("SyntaxError")?;
                    return Ok(());
                }
                if compiler.eval_program_declares_non_declarable_global_function(&program) {
                    compiler.emit_named_error_throw("TypeError")?;
                    return Ok(());
                }
                if compiler.eval_program_declares_non_declarable_global_var(
                    &program,
                    script_global_lexical_bindings,
                ) {
                    compiler.emit_named_error_throw("TypeError")?;
                    return Ok(());
                }
                if program.strict {
                    compiler.register_eval_global_function_local_bindings(&program.functions);
                }
                compiler.instantiate_eval_var_bindings(
                    &program,
                    &preexisting_locals,
                    !script_global_lexical_bindings,
                )?;
                if program.strict {
                    let strict_global_function_declarations = program
                        .functions
                        .iter()
                        .filter(|function| function.register_global)
                        .map(|function| (function.name.clone(), function.name.clone()))
                        .collect::<HashMap<_, _>>();
                    compiler
                        .instantiate_eval_local_functions(&strict_global_function_declarations)?;
                } else {
                    compiler.instantiate_eval_global_functions(
                        &program.functions,
                        !script_global_lexical_bindings,
                    )?;
                }
                compiler.instantiate_eval_local_functions(&eval_local_function_declarations)?;

                compiler.with_indirect_eval_global_this(|compiler| {
                    compiler.with_strict_mode(program.strict, |compiler| {
                        let eval_lexical_scope_names = if script_global_lexical_bindings {
                            Vec::new()
                        } else {
                            collect_direct_eval_lexical_binding_names(&program.statements)
                        };
                        compiler.with_active_eval_lexical_scope(
                            eval_lexical_scope_names,
                            |compiler| {
                                compiler.with_eval_template_cache_epoch(|compiler| {
                                    let completion_local = compiler.allocate_temp_local();
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
                    })
                })
            })?;

            Ok(true)
        } else {
            match argument {
                CallArgument::Expression(expression) => self.emit_numeric_expression(expression)?,
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
