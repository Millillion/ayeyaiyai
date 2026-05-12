use super::*;

impl<'a> ProgramCompilationSession<'a> {
    pub(super) fn capture_module_layout(&self) -> PreparedModuleLayout {
        self.compiler.capture_prepared_module_layout()
    }

    pub(super) fn compile_registered_functions(
        &mut self,
        prepared_program: &PreparedBackendProgram,
    ) -> DirectResult<Vec<CompiledFunction>> {
        let trace = std::env::var_os("AYY_TRACE_PROGRAM_COMPILE").is_some();
        prepared_program
            .user_functions
            .iter()
            .map(|function| {
                if trace {
                    eprintln!("program_compile=user_function:{}", function.metadata.name);
                }
                self.compiler.compile_user_function(
                    function,
                    prepared_program.analysis.function_compiler_inputs(),
                )
            })
            .collect()
    }

    pub(super) fn compile_runtime_called_registered_functions(
        &mut self,
        prepared_program: &PreparedBackendProgram,
    ) -> DirectResult<Vec<CompiledFunction>> {
        let trace = std::env::var_os("AYY_TRACE_PROGRAM_COMPILE").is_some();
        let mut compiled_functions = HashMap::new();

        loop {
            let called_function_names = self
                .compiler
                .state
                .function_registry
                .runtime_called_user_function_names();
            let mut made_progress = false;

            for function in &prepared_program.user_functions {
                if !called_function_names.contains(&function.metadata.name)
                    || compiled_functions.contains_key(&function.metadata.name)
                {
                    continue;
                }

                if trace {
                    eprintln!("program_compile=user_function:{}", function.metadata.name);
                }
                let compiled_function = self.compiler.compile_user_function(
                    function,
                    prepared_program.analysis.function_compiler_inputs(),
                )?;
                compiled_functions.insert(function.metadata.name.clone(), compiled_function);
                made_progress = true;
            }

            if !made_progress {
                break;
            }
        }

        Ok(prepared_program
            .user_functions
            .iter()
            .map(|function| {
                compiled_functions
                    .remove(&function.metadata.name)
                    .unwrap_or_else(stub_compiled_user_function)
            })
            .collect())
    }

    pub(super) fn emit_program(
        &mut self,
        prepared_program: PreparedBackendProgram,
    ) -> DirectResult<EmittedBackendProgram> {
        if std::env::var_os("AYY_TRACE_PROGRAM_COMPILE").is_some() {
            eprintln!("program_compile=start");
        }
        let compiled_start = self.compiler.compile_start(
            &prepared_program.start,
            prepared_program.analysis.function_compiler_inputs(),
        )?;
        let compiled_functions =
            self.compile_runtime_called_registered_functions(&prepared_program)?;
        if std::env::var_os("AYY_TRACE_PROGRAM_COMPILE").is_some() {
            eprintln!("program_compile=layout");
        }
        // Start/function lowering can still reserve implicit globals and related runtime slots,
        // so the final module layout must be captured after compilation completes.
        let module_layout = self.capture_module_layout();
        let (int_min_ptr, int_min_len) = self.compiler.intern_string(b"-2147483648".to_vec());
        let (string_data, next_data_offset) = self.compiler.snapshot_module_data();

        Ok(EmittedBackendProgram {
            compiled_start,
            compiled_functions,
            module_layout,
            artifacts: EmittedModuleArtifacts {
                string_data,
                next_data_offset,
                int_min_ptr,
                int_min_len,
            },
        })
    }
}

fn stub_compiled_user_function() -> CompiledFunction {
    let mut instructions = Vec::new();
    instructions.push(0x41);
    push_i32(&mut instructions, JS_UNDEFINED_TAG);
    CompiledFunction {
        local_count: 0,
        instructions,
    }
}
