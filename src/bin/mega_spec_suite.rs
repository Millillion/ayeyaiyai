use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
    process::{Command, ExitStatus, Stdio},
    thread,
    time::{Duration, Instant},
};

use anyhow::{Context, Result, bail, ensure};
use clap::{ArgAction, Parser};
use serde::Deserialize;
use tempfile::{TempDir, tempdir};
use walkdir::WalkDir;

const ECMA_262_16_URL_PREFIX: &str = "https://262.ecma-international.org/16.0/#";
const DEFAULT_COMPILE_BUDGET_MS: u64 = 10_000;
const DEFAULT_TIMEOUT_SECONDS: u64 = 30;
const RUNTIME_TIMING_RECHECK_RUNS: usize = 2;

#[derive(Debug, Parser)]
#[command(about = "Run the Mega Spec Suite against AyeYaiYai")]
struct Cli {
    #[arg(long, default_value = "tests/mega-spec-suite")]
    root: PathBuf,

    #[arg(long, default_value = "wasm32-wasip2")]
    target: String,

    #[arg(long, action = ArgAction::Append)]
    contains: Vec<String>,

    #[arg(long)]
    limit: Option<usize>,

    #[arg(long)]
    stop_on_fail: bool,

    #[arg(long, default_value_t = DEFAULT_TIMEOUT_SECONDS)]
    timeout_seconds: u64,

    #[arg(long, default_value_t = DEFAULT_COMPILE_BUDGET_MS)]
    compile_budget_ms: u64,
}

#[derive(Debug, Deserialize)]
struct SpecFile {
    subclause: String,
    title: String,
    url: String,
    #[serde(default)]
    behaviors: Vec<String>,
    #[serde(default)]
    behavior_descriptions: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Expected {
    Pass,
    RuntimeError,
    SyntaxError,
    EarlyError,
    ImpossibleWithAot,
}

impl Expected {
    fn parse(value: &str) -> Result<Self> {
        match value {
            "pass" => Ok(Self::Pass),
            "runtime-error" => Ok(Self::RuntimeError),
            "syntax-error" => Ok(Self::SyntaxError),
            "early-error" => Ok(Self::EarlyError),
            "impossible-with-AOT" => Ok(Self::ImpossibleWithAot),
            _ => bail!("unknown expected value `{value}`"),
        }
    }

    fn is_rejection(self) -> bool {
        matches!(self, Self::SyntaxError | Self::EarlyError)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Goal {
    Script,
    Module,
}

impl Goal {
    fn parse(value: &str) -> Result<Self> {
        match value {
            "script" => Ok(Self::Script),
            "module" => Ok(Self::Module),
            _ => bail!("unknown goal `{value}`"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Size {
    Standard,
    Large,
    Stress,
}

impl Size {
    fn parse(value: &str) -> Result<Self> {
        match value {
            "standard" => Ok(Self::Standard),
            "large" => Ok(Self::Large),
            "stress" => Ok(Self::Stress),
            _ => bail!("unknown size `{value}`"),
        }
    }

    fn all() -> [Self; 3] {
        [Self::Standard, Self::Large, Self::Stress]
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Standard => "standard",
            Self::Large => "large",
            Self::Stress => "stress",
        }
    }

    fn minimum_meaningful_lines(self) -> usize {
        match self {
            Self::Standard => 100,
            Self::Large => 500,
            Self::Stress => 1000,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Variant {
    ScriptSloppy,
    ScriptStrict,
    Module,
}

impl Variant {
    fn parse(value: &str) -> Result<Self> {
        match value {
            "script.sloppy" => Ok(Self::ScriptSloppy),
            "script.strict" => Ok(Self::ScriptStrict),
            "module" => Ok(Self::Module),
            "module.strict" => {
                bail!("use `module`, not `module.strict`; modules are already strict")
            }
            _ => bail!("unknown variant `{value}`"),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::ScriptSloppy => "script.sloppy",
            Self::ScriptStrict => "script.strict",
            Self::Module => "module",
        }
    }

    fn goal(self) -> Goal {
        match self {
            Self::ScriptSloppy | Self::ScriptStrict => Goal::Script,
            Self::Module => Goal::Module,
        }
    }
}

#[derive(Debug)]
struct TestHeader {
    behavior: String,
    expected: Expected,
    goal: Goal,
    size: Size,
    variant: Variant,
}

#[derive(Debug)]
struct SuiteTest {
    path: PathBuf,
    subclause: String,
    behavior: String,
    header: TestHeader,
    meaningful_lines: usize,
}

#[derive(Debug, Default)]
struct Summary {
    specs: usize,
    tests: usize,
    passed: usize,
    skipped_impossible: usize,
    failed: usize,
}

#[derive(Debug)]
struct CommandOutput {
    status: ExitStatus,
    stdout: String,
    stderr: String,
    elapsed: Duration,
}

fn main() {
    let worker = std::thread::Builder::new()
        .name("ayy-mega-spec-suite".to_string())
        .stack_size(512 * 1024 * 1024)
        .spawn(|| {
            if let Err(error) = run() {
                eprintln!("error: {error:#}");
                std::process::exit(1);
            }
        })
        .expect("spawn mega suite runner thread");
    if worker.join().is_err() {
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    let (spec_count, mut tests) = discover_suite_tests(&cli.root)?;
    tests.retain(|test| {
        cli.contains.is_empty()
            || cli.contains.iter().any(|needle| {
                test.path.to_string_lossy().contains(needle)
                    || test.subclause.contains(needle)
                    || test.behavior.contains(needle)
            })
    });
    if let Some(limit) = cli.limit {
        tests.truncate(limit);
    }

    let mut summary = Summary {
        specs: spec_count,
        tests: tests.len(),
        ..Summary::default()
    };

    for test in &tests {
        match run_suite_test(test, &cli) {
            Ok(TestRunStatus::Passed) => {
                summary.passed += 1;
                println!("PASS {}", test.path.display());
            }
            Ok(TestRunStatus::SkippedImpossible) => {
                summary.skipped_impossible += 1;
                println!("IMPOSSIBLE_WITH_AOT {}", test.path.display());
            }
            Err(error) => {
                summary.failed += 1;
                eprintln!("FAIL {}\n{error:#}", test.path.display());
                if cli.stop_on_fail {
                    break;
                }
            }
        }
    }

    println!(
        "SUMMARY specs={} tests={} passed={} skipped_impossible={} failed={}",
        summary.specs, summary.tests, summary.passed, summary.skipped_impossible, summary.failed
    );

    ensure!(summary.failed == 0, "mega spec suite failed");
    Ok(())
}

fn discover_suite_tests(root: &Path) -> Result<(usize, Vec<SuiteTest>)> {
    let mut specs = Vec::new();
    for entry in WalkDir::new(root)
        .into_iter()
        .filter_map(std::result::Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .filter(|entry| entry.file_name() == OsStr::new("spec.yaml"))
    {
        specs.push(entry.into_path());
    }
    specs.sort();

    let mut tests = Vec::new();
    for spec_path in &specs {
        let spec = read_spec_file(spec_path)?;
        validate_spec_file(spec_path, &spec)?;
        let spec_dir = spec_path
            .parent()
            .with_context(|| format!("`{}` has no parent directory", spec_path.display()))?;
        validate_behavior_descriptions(spec_path, &spec)?;

        for behavior in &spec.behaviors {
            let behavior_dir = spec_dir.join("behaviors").join(behavior);
            ensure!(
                behavior_dir.is_dir(),
                "{}: missing behavior directory `{}`",
                spec_path.display(),
                behavior_dir.display()
            );
            let behavior_tests = discover_behavior_tests(&spec, behavior, &behavior_dir)?;
            validate_behavior_coverage(behavior, &behavior_tests)?;
            tests.extend(behavior_tests);
        }
    }

    tests.sort_by(|left, right| left.path.cmp(&right.path));
    Ok((specs.len(), tests))
}

fn read_spec_file(path: &Path) -> Result<SpecFile> {
    let source =
        fs::read_to_string(path).with_context(|| format!("failed to read `{}`", path.display()))?;
    serde_yaml::from_str(&source).with_context(|| format!("failed to parse `{}`", path.display()))
}

fn validate_spec_file(path: &Path, spec: &SpecFile) -> Result<()> {
    ensure!(
        spec.url.starts_with(ECMA_262_16_URL_PREFIX),
        "{}: spec URL must use pinned ECMA-262 16.0 URL",
        path.display()
    );
    let dir_name = path
        .parent()
        .and_then(Path::file_name)
        .and_then(OsStr::to_str)
        .unwrap_or_default();
    ensure!(
        dir_name == spec.subclause,
        "{}: subclause `{}` must match directory `{}`",
        path.display(),
        spec.subclause,
        dir_name
    );
    ensure!(
        !spec.title.trim().is_empty(),
        "{}: missing title",
        path.display()
    );
    Ok(())
}

fn validate_behavior_descriptions(path: &Path, spec: &SpecFile) -> Result<()> {
    let behavior_set = spec.behaviors.iter().collect::<BTreeSet<_>>();
    let description_set = spec.behavior_descriptions.keys().collect::<BTreeSet<_>>();
    let missing = behavior_set
        .difference(&description_set)
        .map(|value| value.as_str())
        .collect::<Vec<_>>();
    let extra = description_set
        .difference(&behavior_set)
        .map(|value| value.as_str())
        .collect::<Vec<_>>();

    ensure!(
        missing.is_empty() && extra.is_empty(),
        "{}: behavior description mismatch missing={missing:?} extra={extra:?}",
        path.display()
    );
    for (behavior, description) in &spec.behavior_descriptions {
        ensure!(
            !description.trim().is_empty(),
            "{}: empty behavior description for `{behavior}`",
            path.display()
        );
    }
    Ok(())
}

fn discover_behavior_tests(
    spec: &SpecFile,
    behavior: &str,
    behavior_dir: &Path,
) -> Result<Vec<SuiteTest>> {
    let mut tests = Vec::new();
    for entry in WalkDir::new(behavior_dir)
        .min_depth(1)
        .max_depth(1)
        .into_iter()
        .filter_map(std::result::Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .filter(|entry| entry.path().extension() == Some(OsStr::new("js")))
    {
        let path = entry.into_path();
        let source = fs::read_to_string(&path)
            .with_context(|| format!("failed to read `{}`", path.display()))?;
        let header = parse_test_header(&path, &source)?;
        ensure!(
            header.behavior == behavior,
            "{}: header behavior `{}` does not match directory behavior `{behavior}`",
            path.display(),
            header.behavior
        );
        ensure!(
            header.goal == header.variant.goal(),
            "{}: goal does not match variant `{}`",
            path.display(),
            header.variant.as_str()
        );
        validate_filename_matches_header(&path, &header)?;

        let meaningful_lines = count_meaningful_lines(&source);
        if header.expected != Expected::ImpossibleWithAot {
            ensure!(
                meaningful_lines >= header.size.minimum_meaningful_lines(),
                "{}: `{}` requires at least {} meaningful lines, found {meaningful_lines}",
                path.display(),
                header.size.as_str(),
                header.size.minimum_meaningful_lines()
            );
        }
        tests.push(SuiteTest {
            path,
            subclause: spec.subclause.clone(),
            behavior: behavior.to_string(),
            header,
            meaningful_lines,
        });
    }

    ensure!(
        !tests.is_empty(),
        "behavior `{behavior}` has no .js tests in `{}`",
        behavior_dir.display()
    );
    tests.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(tests)
}

fn validate_filename_matches_header(path: &Path, header: &TestHeader) -> Result<()> {
    let stem = path
        .file_stem()
        .and_then(OsStr::to_str)
        .with_context(|| format!("invalid test filename `{}`", path.display()))?;
    let pieces = stem.split('.').collect::<Vec<_>>();
    let actual_size = pieces
        .last()
        .with_context(|| format!("invalid test filename `{}`", path.display()))?;
    ensure!(
        *actual_size == header.size.as_str(),
        "{}: filename size `{actual_size}` does not match header size `{}`",
        path.display(),
        header.size.as_str()
    );

    if pieces.len() > 1 {
        let filename_variant = pieces[..pieces.len() - 1].join(".");
        ensure!(
            filename_variant == header.variant.as_str(),
            "{}: filename variant `{filename_variant}` does not match header variant `{}`",
            path.display(),
            header.variant.as_str()
        );
    }
    Ok(())
}

fn validate_behavior_coverage(behavior: &str, tests: &[SuiteTest]) -> Result<()> {
    let non_impossible = tests
        .iter()
        .filter(|test| test.header.expected != Expected::ImpossibleWithAot)
        .collect::<Vec<_>>();
    if non_impossible.is_empty() {
        return Ok(());
    }

    let mut coverage: BTreeMap<Variant, BTreeSet<Size>> = BTreeMap::new();
    for test in non_impossible {
        coverage
            .entry(test.header.variant)
            .or_default()
            .insert(test.header.size);
    }

    for (variant, sizes) in coverage {
        for size in Size::all() {
            ensure!(
                sizes.contains(&size),
                "behavior `{behavior}` variant `{}` is missing `{}` coverage",
                variant.as_str(),
                size.as_str()
            );
        }
    }
    Ok(())
}

fn parse_test_header(path: &Path, source: &str) -> Result<TestHeader> {
    let mut values = BTreeMap::new();
    let mut saw_header = false;

    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if saw_header {
                break;
            }
            continue;
        }

        let Some(comment) = trimmed.strip_prefix("//") else {
            break;
        };
        saw_header = true;
        let Some((key, value)) = comment.trim().split_once(':') else {
            continue;
        };
        values.insert(key.trim().to_string(), value.trim().to_string());
    }

    let behavior = take_header_value(path, &mut values, "behavior")?;
    let expected = Expected::parse(&take_header_value(path, &mut values, "expected")?)?;
    let goal = Goal::parse(&take_header_value(path, &mut values, "goal")?)?;
    let size = Size::parse(&take_header_value(path, &mut values, "size")?)?;
    let variant = Variant::parse(&take_header_value(path, &mut values, "variant")?)?;
    let impossible_reason = values.remove("impossible_reason");

    ensure!(
        expected != Expected::ImpossibleWithAot
            || impossible_reason
                .as_deref()
                .is_some_and(|reason| !reason.trim().is_empty()),
        "{}: impossible-with-AOT tests require impossible_reason",
        path.display()
    );

    Ok(TestHeader {
        behavior,
        expected,
        goal,
        size,
        variant,
    })
}

fn take_header_value(
    path: &Path,
    values: &mut BTreeMap<String, String>,
    key: &str,
) -> Result<String> {
    values
        .remove(key)
        .with_context(|| format!("{}: missing header field `{key}`", path.display()))
}

fn count_meaningful_lines(source: &str) -> usize {
    source
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            !trimmed.is_empty()
                && !trimmed.starts_with("//")
                && !trimmed.starts_with("/*")
                && !trimmed.starts_with('*')
                && !trimmed.starts_with("*/")
        })
        .count()
}

enum TestRunStatus {
    Passed,
    SkippedImpossible,
}

fn run_suite_test(test: &SuiteTest, cli: &Cli) -> Result<TestRunStatus> {
    if test.header.expected == Expected::ImpossibleWithAot {
        return Ok(TestRunStatus::SkippedImpossible);
    }

    let tempdir = tempdir().context("failed to create tempdir")?;
    let source_path = materialize_source_for_variant(test, &tempdir)?;

    if test.header.expected.is_rejection() {
        let node = run_node(&source_path, test.header.goal, cli.timeout_seconds)?;
        ensure!(
            !node.status.success(),
            "{}: Node accepted expected rejection\nstdout:\n{}\nstderr:\n{}",
            test.path.display(),
            node.stdout,
            node.stderr
        );
        let compile = compile_with_ayeyaiyai(
            &source_path,
            test.header.goal,
            test.header.variant,
            &cli.target,
            cli.timeout_seconds,
            &tempdir,
        )?;
        ensure!(
            !compile.status.success(),
            "{}: AyeYaiYai accepted expected rejection\nstdout:\n{}\nstderr:\n{}",
            test.path.display(),
            compile.stdout,
            compile.stderr
        );
        ensure!(
            compile.elapsed <= Duration::from_millis(cli.compile_budget_ms),
            "{}: rejection compile took {}ms, budget is {}ms",
            test.path.display(),
            compile.elapsed.as_millis(),
            cli.compile_budget_ms
        );
        return Ok(TestRunStatus::Passed);
    }

    let node = run_node(&source_path, test.header.goal, cli.timeout_seconds)?;
    match test.header.expected {
        Expected::Pass => ensure!(
            node.status.success(),
            "{}: Node failed pass test\nstdout:\n{}\nstderr:\n{}",
            test.path.display(),
            node.stdout,
            node.stderr
        ),
        Expected::RuntimeError => ensure!(
            !node.status.success(),
            "{}: Node succeeded runtime-error test",
            test.path.display()
        ),
        _ => unreachable!("handled non-executable expectations above"),
    }

    let compile = compile_with_ayeyaiyai(
        &source_path,
        test.header.goal,
        test.header.variant,
        &cli.target,
        cli.timeout_seconds,
        &tempdir,
    )?;
    ensure!(
        compile.status.success(),
        "{}: AyeYaiYai compile failed\nstdout:\n{}\nstderr:\n{}",
        test.path.display(),
        compile.stdout,
        compile.stderr
    );
    ensure!(
        compile.elapsed <= Duration::from_millis(cli.compile_budget_ms),
        "{}: compile took {}ms, budget is {}ms",
        test.path.display(),
        compile.elapsed.as_millis(),
        cli.compile_budget_ms
    );

    let wasm_path = tempdir.path().join("test.wasm");
    let wasmtime = run_wasmtime(&wasm_path, cli.timeout_seconds)?;
    match test.header.expected {
        Expected::Pass => {
            ensure!(
                wasmtime.status.success(),
                "{}: wasmtime failed pass test\nstdout:\n{}\nstderr:\n{}",
                test.path.display(),
                wasmtime.stdout,
                wasmtime.stderr
            );
            ensure!(
                node.stdout == wasmtime.stdout,
                "{}: stdout mismatch\nNode:\n{}\nwasmtime:\n{}",
                test.path.display(),
                node.stdout,
                wasmtime.stdout
            );
        }
        Expected::RuntimeError => {
            ensure!(
                !wasmtime.status.success(),
                "{}: wasmtime succeeded runtime-error test",
                test.path.display()
            );
            let node_error = extract_error_name(&node.stderr);
            let wasm_error = extract_error_name(&wasmtime.stderr);
            if let (Some(node_error), Some(wasm_error)) = (node_error, wasm_error) {
                ensure!(
                    node_error == wasm_error,
                    "{}: runtime error mismatch Node={node_error} wasmtime={wasm_error}",
                    test.path.display()
                );
            }
        }
        _ => unreachable!("handled non-executable expectations above"),
    }

    let (node_elapsed, wasmtime_elapsed) =
        fair_runtime_elapsed_pair(test, &source_path, &wasm_path, &node, &wasmtime, cli)?;
    validate_runtime_ratio(test, node_elapsed, wasmtime_elapsed)?;
    println!(
        "TIMING {} meaningful_lines={} node_ms={} compile_ms={} wasmtime_ms={}",
        test.path.display(),
        test.meaningful_lines,
        node_elapsed.as_millis(),
        compile.elapsed.as_millis(),
        wasmtime_elapsed.as_millis()
    );

    Ok(TestRunStatus::Passed)
}

fn materialize_source_for_variant(test: &SuiteTest, tempdir: &TempDir) -> Result<PathBuf> {
    let source = fs::read_to_string(&test.path)
        .with_context(|| format!("failed to read `{}`", test.path.display()))?;
    let mut materialized = String::new();
    if test.header.variant == Variant::ScriptStrict {
        materialized.push_str("\"use strict\";\n");
    }
    materialized.push_str(&source);

    let extension = if test.header.goal == Goal::Module {
        "mjs"
    } else {
        "js"
    };
    let path = tempdir.path().join(format!("source.{extension}"));
    fs::write(&path, materialized)
        .with_context(|| format!("failed to write `{}`", path.display()))?;
    Ok(path)
}

fn run_node(path: &Path, goal: Goal, timeout_seconds: u64) -> Result<CommandOutput> {
    run_command_with_timeout(node_command(path, goal), timeout_seconds)
}

fn node_command(path: &Path, goal: Goal) -> Command {
    let mut command = Command::new("node");
    if goal == Goal::Module {
        command.arg(path);
    } else {
        command.arg(path);
    }
    command
}

fn compile_with_ayeyaiyai(
    source_path: &Path,
    goal: Goal,
    variant: Variant,
    target: &str,
    timeout_seconds: u64,
    tempdir: &TempDir,
) -> Result<CommandOutput> {
    let compiler = ayeyaiyai_binary()?;
    let wasm_path = tempdir.path().join("test.wasm");
    let mut command = Command::new(compiler);
    command
        .arg(source_path)
        .arg("-o")
        .arg(&wasm_path)
        .arg("--target")
        .arg(target);
    if goal == Goal::Module {
        command.arg("--module");
    }
    if variant == Variant::ScriptStrict {
        command.arg("--force-strict");
    }
    run_command_with_timeout(command, timeout_seconds)
}

fn ayeyaiyai_binary() -> Result<PathBuf> {
    let sibling = std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(|dir| dir.join("ayeyaiyai")))
        .filter(|path| path.is_file());
    sibling.ok_or_else(|| {
        anyhow::anyhow!(
            "sibling ayeyaiyai binary not found; build with `cargo build --bin ayeyaiyai --bin mega_spec_suite`"
        )
    })
}

fn run_wasmtime(wasm_path: &Path, timeout_seconds: u64) -> Result<CommandOutput> {
    run_command_with_timeout(wasmtime_command(wasm_path), timeout_seconds)
}

fn wasmtime_command(wasm_path: &Path) -> Command {
    let mut command = Command::new("wasmtime");
    command.arg(wasm_path);
    command
}

fn run_command_with_timeout(mut command: Command, timeout_seconds: u64) -> Result<CommandOutput> {
    let started = Instant::now();
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("failed to spawn `{command:?}`"))?;
    let timeout = Duration::from_secs(timeout_seconds);

    loop {
        if child.try_wait()?.is_some() {
            break;
        }
        if started.elapsed() >= timeout {
            child.kill()?;
            let output = child.wait_with_output()?;
            return Ok(CommandOutput {
                status: output.status,
                stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
                stderr: format!(
                    "timed out after {timeout_seconds}s\n{}",
                    String::from_utf8_lossy(&output.stderr)
                ),
                elapsed: started.elapsed(),
            });
        }
        thread::sleep(Duration::from_millis(10));
    }

    let output = child.wait_with_output()?;
    Ok(CommandOutput {
        status: output.status,
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        elapsed: started.elapsed(),
    })
}

fn fair_runtime_elapsed_pair(
    test: &SuiteTest,
    source_path: &Path,
    wasm_path: &Path,
    node: &CommandOutput,
    wasmtime: &CommandOutput,
    cli: &Cli,
) -> Result<(Duration, Duration)> {
    let mut node_elapsed = node.elapsed;
    let mut wasmtime_elapsed = wasmtime.elapsed;
    if !runtime_ratio_exceeded(node_elapsed, wasmtime_elapsed) {
        return Ok((node_elapsed, wasmtime_elapsed));
    }

    node_elapsed = best_rechecked_runtime_elapsed(
        node_elapsed,
        RUNTIME_TIMING_RECHECK_RUNS,
        || node_command(source_path, test.header.goal),
        cli.timeout_seconds,
        |output| validate_rechecked_node_output(test, node, output),
    )?;
    wasmtime_elapsed = best_rechecked_runtime_elapsed(
        wasmtime_elapsed,
        RUNTIME_TIMING_RECHECK_RUNS,
        || wasmtime_command(wasm_path),
        cli.timeout_seconds,
        |output| validate_rechecked_wasmtime_output(test, node, wasmtime, output),
    )?;
    Ok((node_elapsed, wasmtime_elapsed))
}

fn best_rechecked_runtime_elapsed<F, V>(
    initial_elapsed: Duration,
    runs: usize,
    mut make_command: F,
    timeout_seconds: u64,
    mut validate_output: V,
) -> Result<Duration>
where
    F: FnMut() -> Command,
    V: FnMut(&CommandOutput) -> Result<()>,
{
    let mut best_elapsed = initial_elapsed;
    for _ in 0..runs {
        let output = run_command_with_timeout(make_command(), timeout_seconds)?;
        validate_output(&output)?;
        best_elapsed = best_elapsed.min(output.elapsed);
    }
    Ok(best_elapsed)
}

fn validate_rechecked_node_output(
    test: &SuiteTest,
    reference: &CommandOutput,
    output: &CommandOutput,
) -> Result<()> {
    match test.header.expected {
        Expected::Pass => {
            ensure!(
                output.status.success(),
                "{}: Node failed timing recheck\nstdout:\n{}\nstderr:\n{}",
                test.path.display(),
                output.stdout,
                output.stderr
            );
            ensure!(
                output.stdout == reference.stdout,
                "{}: Node timing recheck stdout mismatch\nfirst run:\n{}\nrecheck:\n{}",
                test.path.display(),
                reference.stdout,
                output.stdout
            );
        }
        Expected::RuntimeError => {
            ensure!(
                !output.status.success(),
                "{}: Node succeeded runtime-error timing recheck",
                test.path.display()
            );
            let reference_error = extract_error_name(&reference.stderr);
            let output_error = extract_error_name(&output.stderr);
            if let (Some(reference_error), Some(output_error)) = (reference_error, output_error) {
                ensure!(
                    reference_error == output_error,
                    "{}: Node timing recheck runtime error mismatch first={reference_error} recheck={output_error}",
                    test.path.display()
                );
            }
        }
        _ => unreachable!("runtime timing is only checked for executable tests"),
    }
    Ok(())
}

fn validate_rechecked_wasmtime_output(
    test: &SuiteTest,
    node: &CommandOutput,
    reference: &CommandOutput,
    output: &CommandOutput,
) -> Result<()> {
    match test.header.expected {
        Expected::Pass => {
            ensure!(
                output.status.success(),
                "{}: wasmtime failed timing recheck\nstdout:\n{}\nstderr:\n{}",
                test.path.display(),
                output.stdout,
                output.stderr
            );
            ensure!(
                output.stdout == reference.stdout,
                "{}: wasmtime timing recheck stdout mismatch\nfirst run:\n{}\nrecheck:\n{}",
                test.path.display(),
                reference.stdout,
                output.stdout
            );
        }
        Expected::RuntimeError => {
            ensure!(
                !output.status.success(),
                "{}: wasmtime succeeded runtime-error timing recheck",
                test.path.display()
            );
            let node_error = extract_error_name(&node.stderr);
            let output_error = extract_error_name(&output.stderr);
            if let (Some(node_error), Some(output_error)) = (node_error, output_error) {
                ensure!(
                    node_error == output_error,
                    "{}: wasmtime timing recheck runtime error mismatch Node={node_error} wasmtime={output_error}",
                    test.path.display()
                );
            }
        }
        _ => unreachable!("runtime timing is only checked for executable tests"),
    }
    Ok(())
}

fn runtime_ratio_exceeded(node_elapsed: Duration, wasm_elapsed: Duration) -> bool {
    let strict_limit = node_elapsed.saturating_mul(10);
    let noise_floor = node_elapsed + Duration::from_millis(100);
    let allowed = strict_limit.max(noise_floor);
    wasm_elapsed > allowed
}

fn validate_runtime_ratio(
    test: &SuiteTest,
    node_elapsed: Duration,
    wasm_elapsed: Duration,
) -> Result<()> {
    ensure!(
        !runtime_ratio_exceeded(node_elapsed, wasm_elapsed),
        "{}: wasmtime runtime {}ms exceeds Node runtime {}ms by more than 10x with timing floor",
        test.path.display(),
        wasm_elapsed.as_millis(),
        node_elapsed.as_millis()
    );
    Ok(())
}

fn extract_error_name(stderr: &str) -> Option<&'static str> {
    for name in [
        "SyntaxError",
        "ReferenceError",
        "TypeError",
        "RangeError",
        "Error",
    ] {
        if stderr.contains(name) {
            return Some(name);
        }
    }
    None
}
