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
        self.reset_compilation_state();
        if crate::ayy_env_flag!("AYY_TRACE_PROGRAM_COMPILE") {
            eprintln!("program_compile=prepare");
        }
        let prepared_program = self.prepare_program(program)?;
        if crate::ayy_env_flag!("AYY_TRACE_PROGRAM_COMPILE") {
            eprintln!("program_compile=emit");
        }
        let result = self.emit_program(prepared_program)?.assemble();
        crate::backend::direct_wasm::memo::dump_memo_stats("program");
        Ok(result)
    }

    fn reset_compilation_state(&mut self) {
        reset_function_compiler_thread_locals();
        crate::backend::direct_wasm::memo::reset_memo_state();
        self.compiler.reset_for_program_compilation();
    }
}
