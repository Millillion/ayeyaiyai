use anyhow::Result;

use crate::ir::{hir::Program, passes};

pub fn validate(program: &Program) -> Result<()> {
    passes::refined_aot::validate(program)
}

pub fn prepare(program: Program) -> Result<Program> {
    let program = passes::static_function_constructors::lower(program)?;
    validate(&program)?;
    Ok(program)
}
