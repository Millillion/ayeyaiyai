use super::*;

mod bindings;
mod call_entry;
mod direct_eval;
mod indirect_eval;
mod programs;
mod scoped_rewrite;
mod source_patterns;

use programs::eval_program_contains_top_level_return;

fn lower_eval_static_function_constructors(program: Program) -> Program {
    let original = program.clone();
    crate::ir::passes::static_function_constructors::lower(program).unwrap_or(original)
}
