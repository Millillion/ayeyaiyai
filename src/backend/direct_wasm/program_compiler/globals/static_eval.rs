use super::*;

#[path = "static_eval/eval_bindings.rs"]
mod eval_bindings;
#[path = "static_eval/parsing.rs"]
mod parsing;
#[path = "static_eval/traversal.rs"]
mod traversal;

fn lower_static_eval_function_constructors(program: Program) -> Program {
    let original = program.clone();
    crate::ir::passes::static_function_constructors::lower(program).unwrap_or(original)
}
