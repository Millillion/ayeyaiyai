use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;

use ayeyaiyai::{CompileOptions, compile_file};

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

    compile_file(&cli.input, &options)
}
