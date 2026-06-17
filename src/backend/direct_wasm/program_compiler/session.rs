use super::*;

#[path = "session/emission.rs"]
mod emission;
#[path = "session/phases.rs"]
mod phases;
#[path = "session/preparation.rs"]
mod preparation;

pub(in crate::backend::direct_wasm) struct ProgramCompilationSession<'a> {
    compiler: &'a mut DirectWasmCompiler,
}

impl<'a> ProgramCompilationSession<'a> {
    pub(in crate::backend::direct_wasm) fn new(
        compiler: &'a mut DirectWasmCompiler,
    ) -> ProgramCompilationSession<'a> {
        Self { compiler }
    }

    pub(in crate::backend::direct_wasm) fn compile(
        mut self,
        program: &Program,
    ) -> DirectResult<Vec<u8>> {
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
                    "program_compile_timing step={step} elapsed_ms={} total_ms={total_ms}",
                    now.duration_since(previous).as_millis()
                );
                timing_last = Some(now);
            }
        };
        self.reset_compilation_state();
        trace_step("reset");
        if crate::ayy_env_flag!("AYY_TRACE_PROGRAM_COMPILE") {
            eprintln!("program_compile=prepare");
        }
        let prepared_program = self.prepare_program(program)?;
        trace_step("prepare");
        if crate::ayy_env_flag!("AYY_TRACE_PROGRAM_COMPILE") {
            eprintln!("program_compile=emit");
        }
        let emitted_program = self.emit_program(prepared_program)?;
        trace_step("emit");
        let result = emitted_program.assemble();
        trace_step("assemble");
        crate::backend::direct_wasm::memo::dump_memo_stats("program");
        Ok(result)
    }

    fn reset_compilation_state(&mut self) {
        reset_function_compiler_thread_locals();
        crate::backend::direct_wasm::memo::reset_memo_state();
        self.compiler.reset_for_program_compilation();
    }
}
