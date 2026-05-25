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
        if std::env::var_os("AYY_TRACE_PROGRAM_COMPILE").is_some() {
            eprintln!("program_compile=prepare");
        }
        let prepared_program = self.prepare_program(program)?;
        if std::env::var_os("AYY_TRACE_PROGRAM_COMPILE").is_some() {
            eprintln!("program_compile=emit");
        }
        Ok(self.emit_program(prepared_program)?.assemble())
    }

    fn reset_compilation_state(&mut self) {
        reset_function_compiler_thread_locals();
        self.compiler.reset_for_program_compilation();
    }
}
