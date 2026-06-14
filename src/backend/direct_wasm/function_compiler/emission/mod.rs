use super::*;

mod assignments;
mod control_flow;
mod eval;
mod expression_codegen;
mod process_argv;

pub(in crate::backend::direct_wasm) use eval::eval_statement_contains_return;
