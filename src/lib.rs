pub mod backend;
mod compile_options;
pub mod frontend;
pub mod ir;

/// Caches an `AYY_*` env-flag lookup per call site: profiling showed the
/// repeated `getenv` calls from trace guards consuming ~12% of compile time
/// on hot paths. Flags are read once per process; the compiler never sets
/// env vars at runtime.
#[macro_export]
macro_rules! ayy_env_flag {
    ($name:literal) => {{
        static FLAG: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
        *FLAG.get_or_init(|| std::env::var_os($name).is_some())
    }};
}

use std::path::Path;

use anyhow::{Result, bail};

pub use backend::{emit_wasm, emit_wasm_with_reason};
pub use compile_options::CompileOptions;

pub fn compile_file(path: &Path, options: &CompileOptions) -> Result<()> {
    compile_file_with_goal_and_strict(path, options, false, false)
}

pub fn compile_file_with_goal(path: &Path, options: &CompileOptions, module: bool) -> Result<()> {
    compile_file_with_goal_and_strict(path, options, module, false)
}

pub fn compile_file_with_goal_and_strict(
    path: &Path,
    options: &CompileOptions,
    module: bool,
    force_strict: bool,
) -> Result<()> {
    let trace_timing = ayy_env_flag!("AYY_TRACE_COMPILE_TIMING");
    let timing_start = trace_timing.then(std::time::Instant::now);
    let mut timing_last = timing_start;
    let mut trace_step = |step: &str| {
        if let Some(previous) = timing_last {
            let now = std::time::Instant::now();
            let total_ms = timing_start
                .map(|start| now.duration_since(start).as_millis())
                .unwrap_or(0);
            eprintln!(
                "compile_file_timing step={step} elapsed_ms={} total_ms={total_ms}",
                now.duration_since(previous).as_millis()
            );
            timing_last = Some(now);
        }
    };
    let program = if module {
        frontend::bundle_module_entry(path)?
    } else {
        frontend::bundle_script_entry_with_strict(path, force_strict)?
    };
    trace_step("parse_bundle");
    let program = ir::pipeline::prepare(program)?;
    trace_step("ir_prepare");
    if backend::compile_if_supported(&program, options)? {
        trace_step("backend_compile_write");
        return Ok(());
    }

    bail!("program uses JavaScript features that are not yet supported by the direct wasm backend")
}

pub fn compile_unmodified_file_with_goal_and_strict(
    path: &Path,
    options: &CompileOptions,
    module: bool,
    force_strict: bool,
) -> Result<()> {
    let program = if module {
        frontend::bundle_module_entry_unmodified(path)?
    } else {
        frontend::bundle_script_entry_with_strict_unmodified(path, force_strict)?
    };
    let program = ir::pipeline::prepare(program)?;
    if backend::compile_if_supported(&program, options)? {
        return Ok(());
    }

    bail!("program uses JavaScript features that are not yet supported by the direct wasm backend")
}

pub fn compile_source(source: &str, options: &CompileOptions) -> Result<()> {
    compile_source_with_goal(source, options, false)
}

pub fn compile_source_with_goal(
    source: &str,
    options: &CompileOptions,
    module: bool,
) -> Result<()> {
    let program = if module {
        frontend::parse_module_goal(source)?
    } else {
        frontend::parse(source)?
    };
    let program = ir::pipeline::prepare(program)?;
    if backend::compile_if_supported(&program, options)? {
        return Ok(());
    }
    bail!("program uses JavaScript features that are not yet supported by the direct wasm backend")
}

pub fn compile_source_with_reason(source: &str) -> std::result::Result<(), String> {
    let program = frontend::parse(source).map_err(|_| "source failed to parse".to_string())?;
    let program = ir::passes::static_function_constructors::lower(program)
        .map_err(|_| "static function constructor lowering failed".to_string())?;
    ir::pipeline::validate(&program).map_err(|_| "refined aot validation failed".to_string())?;
    match backend::emit_wasm_with_reason(&program) {
        Ok(_) => Ok(()),
        Err(message) => Err(message.to_string()),
    }
}
