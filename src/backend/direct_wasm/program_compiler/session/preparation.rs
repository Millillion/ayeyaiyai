use super::*;

impl<'a> ProgramCompilationSession<'a> {
    pub(super) fn prepare_program(
        &mut self,
        program: &Program,
    ) -> DirectResult<PreparedBackendProgram> {
        let trace_timing = crate::ayy_env_flag!("AYY_TRACE_COMPILE_TIMING");
        let timing_start = trace_timing.then(std::time::Instant::now);
        let mut timing_last = timing_start;
        let mut trace_step = |step: &str| {
            if let Some(previous) = timing_last {
                let now = std::time::Instant::now();
                let total_ms = timing_start
                    .map(|start| now.duration_since(start).as_millis())
                    .unwrap_or(0);
                eprintln!(
                    "prepare_program_timing step={step} elapsed_ms={} total_ms={total_ms}",
                    now.duration_since(previous).as_millis()
                );
                timing_last = Some(now);
            }
        };
        self.run_function_discovery_phase(program)?;
        trace_step("function_discovery");
        self.run_global_binding_phase(program);
        trace_step("global_binding");
        // Runtime reservation re-registers class member bindings (it can
        // resolve class-expression aliases such as `var C = class { ... }`
        // once constructor metadata exists), so parameter analysis runs after
        // it to see prototype method bindings for those classes.
        self.run_runtime_reservation_phase(program)?;
        trace_step("runtime_reservation");
        self.run_parameter_analysis_phase(program);
        trace_step("parameter_analysis");
        let global_binding_environment = self.compiler.snapshot_global_binding_environment();
        let global_static_semantics = self.compiler.snapshot_global_static_semantics();
        trace_step("snapshots");

        let start = self.prepare_start_function(program, &global_binding_environment)?;
        trace_step("prepare_start");
        let (user_functions, analysis) = self.prepare_user_function_compilations(
            program,
            &global_binding_environment,
            global_static_semantics,
        )?;
        trace_step("prepare_user_functions");

        Ok(PreparedBackendProgram {
            start,
            analysis,
            user_functions,
            module_layout: self.capture_module_layout(),
        })
    }

    pub(super) fn prepare_start_function(
        &mut self,
        program: &Program,
        global_binding_environment: &GlobalBindingEnvironment,
    ) -> DirectResult<PreparedStartFunction> {
        Ok(PreparedStartFunction {
            statements: self.compiler.prepare_start_statements(program),
            entry_state: FunctionCompiler::prepare_top_level_entry_state(
                self.compiler,
                program.strict,
                global_binding_environment,
            )?,
            initial_named_error: self
                .compiler
                .global_declaration_instantiation_named_error(program),
        })
    }

    pub(super) fn prepare_user_function_compilations(
        &mut self,
        _program: &Program,
        global_binding_environment: &GlobalBindingEnvironment,
        global_static_semantics: GlobalStaticSemanticsSnapshot,
    ) -> DirectResult<(
        Vec<PreparedUserFunctionCompilation>,
        PreparedProgramAnalysis,
    )> {
        let mut user_functions = Vec::new();
        let mut ordered_user_function_names = Vec::new();
        let mut assigned_nonlocal_bindings = HashMap::new();
        let mut assigned_nonlocal_binding_results = HashMap::new();
        let mut user_function_metadata = HashMap::new();
        let registered_declarations = self
            .compiler
            .state
            .user_functions()
            .iter()
            .filter_map(|function| self.compiler.registered_function(&function.name).cloned())
            .collect::<Vec<_>>();
        for declaration in registered_declarations {
            let Some((prepared_function, prepared_results)) =
                self.prepare_user_function_compilation(&declaration, global_binding_environment)?
            else {
                continue;
            };
            ordered_user_function_names.push(prepared_function.metadata.name.clone());
            user_function_metadata.insert(
                prepared_function.metadata.name.clone(),
                prepared_function.metadata.clone(),
            );
            if !prepared_results.is_empty() {
                assigned_nonlocal_binding_results
                    .insert(prepared_function.metadata.name.clone(), prepared_results);
            }
            if !prepared_function
                .analysis
                .assigned_nonlocal_bindings
                .is_empty()
            {
                assigned_nonlocal_bindings.insert(
                    prepared_function.metadata.name.clone(),
                    prepared_function
                        .analysis
                        .assigned_nonlocal_bindings
                        .clone(),
                );
            }
            user_functions.push(prepared_function);
        }
        let eval_local_function_bindings = self.compiler.prepared_eval_local_function_bindings();
        let user_function_capture_bindings =
            self.compiler.prepared_user_function_capture_bindings();
        Ok((
            user_functions,
            PreparedProgramAnalysis::new(
                assigned_nonlocal_bindings,
                assigned_nonlocal_binding_results,
                user_function_metadata,
                ordered_user_function_names,
                eval_local_function_bindings,
                user_function_capture_bindings,
                global_binding_environment.clone(),
                global_static_semantics,
            ),
        ))
    }

    pub(super) fn prepare_user_function_compilation(
        &mut self,
        declaration: &FunctionDeclaration,
        global_binding_environment: &GlobalBindingEnvironment,
    ) -> DirectResult<Option<(PreparedUserFunctionCompilation, HashMap<String, Expression>)>> {
        let Some(user_function) = self.compiler.prepared_user_function(&declaration.name) else {
            return Ok(None);
        };

        let parameter_bindings = self
            .compiler
            .prepared_user_function_parameter_bindings(&declaration.name);
        let entry_state = FunctionCompiler::prepare_user_function_entry_state(
            self.compiler,
            declaration,
            &user_function,
            &parameter_bindings.function_bindings,
            &parameter_bindings.value_bindings,
            &parameter_bindings.array_bindings,
            &parameter_bindings.object_bindings,
            global_binding_environment,
        )?;
        let (analysis, assigned_nonlocal_binding_results) =
            self.prepare_user_function_analysis(&user_function);

        Ok(Some((
            PreparedUserFunctionCompilation {
                metadata: PreparedFunctionMetadata {
                    name: declaration.name.clone(),
                    declaration: declaration.clone(),
                    user_function: user_function.clone(),
                },
                analysis,
                entry_state,
            },
            assigned_nonlocal_binding_results,
        )))
    }

    pub(super) fn prepare_user_function_analysis(
        &mut self,
        user_function: &UserFunction,
    ) -> (PreparedUserFunctionAnalysis, HashMap<String, Expression>) {
        let assigned_nonlocal_bindings = self
            .compiler
            .collect_user_function_assigned_nonlocal_bindings(user_function);
        let assigned_nonlocal_binding_results = self
            .compiler
            .capture_assigned_nonlocal_binding_results(&assigned_nonlocal_bindings);
        (
            PreparedUserFunctionAnalysis {
                assigned_nonlocal_bindings,
            },
            assigned_nonlocal_binding_results,
        )
    }
}
