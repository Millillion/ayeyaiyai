use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;

use ayeyaiyai::{
    CompileOptions, compile_file_with_goal_and_strict, compile_unmodified_file_with_goal_and_strict,
};

#[derive(Debug, Parser)]
#[command(
    author,
    version,
    about = "Compile JavaScript directly to WASI Preview 2"
)]
struct Cli {
    input: PathBuf,

    #[arg(short, long)]
    output: PathBuf,

    #[arg(long, default_value = "wasm32-wasip2")]
    target: String,

    /// Parse as a module (test262 runner support).
    #[arg(long, hide = true)]
    module: bool,

    /// Force strict mode (test262 runner support).
    #[arg(long, hide = true)]
    force_strict: bool,

    /// Use the rewrite-free parse path (test262 runner support).
    #[arg(long, hide = true)]
    unmodified: bool,
}

fn main() {
    // Static resolution recurses with expression depth; give the compiler a
    // large dedicated stack instead of relying on the platform default.
    let worker = std::thread::Builder::new()
        .name("ayy-compile".to_string())
        .stack_size(512 * 1024 * 1024)
        .spawn(|| {
            if let Err(error) = run() {
                eprintln!("error: {error:#}");
                std::process::exit(1);
            }
        })
        .expect("spawn compile thread");
    if worker.join().is_err() {
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    let options = CompileOptions {
        output: cli.output,
        target: cli.target,
    };

    if cli.unmodified {
        return compile_unmodified_file_with_goal_and_strict(
            &cli.input,
            &options,
            cli.module,
            cli.force_strict,
        );
    }
    compile_file_with_goal_and_strict(&cli.input, &options, cli.module, cli.force_strict)
}
