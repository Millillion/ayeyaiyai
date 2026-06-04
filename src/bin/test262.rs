use std::{
    collections::HashSet,
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use anyhow::{Context, Result};
use ayeyaiyai::{CompileOptions, compile_unmodified_file_with_goal_and_strict};
use clap::{ArgAction, Parser};
use tempfile::tempdir;
use walkdir::WalkDir;

#[derive(Debug, Parser)]
#[command(about = "Run the test262 language category against AyeYaiYai")]
struct Cli {
    #[arg(long)]
    test262_dir: PathBuf,

    #[arg(long, default_value = "wasm32-wasip2")]
    target: String,

    #[arg(long = "test", action = ArgAction::Append)]
    tests: Vec<String>,

    #[arg(long = "tests-from", action = ArgAction::Append)]
    tests_from: Vec<PathBuf>,

    #[arg(long, action = ArgAction::Append)]
    contains: Vec<String>,

    #[arg(long)]
    limit: Option<usize>,

    #[arg(long)]
    stop_on_fail: bool,

    #[arg(long, default_value_t = 30)]
    timeout_seconds: u64,
}

#[derive(Debug, Default)]
struct Summary {
    discovered: usize,
    attempted: usize,
    passed: usize,
    compile_failed: usize,
    runtime_failed: usize,
}

#[derive(Debug, Default)]
struct Metadata {
    includes: Vec<String>,
    flags: Vec<String>,
    negative: Option<NegativeExpectation>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct NegativeExpectation {
    phase: Option<String>,
    error_type: Option<String>,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    let mut summary = Summary::default();
    let exact_tests = normalize_requested_tests(&cli.test262_dir, &cli.tests, &cli.tests_from)?;
    let candidate_paths: Box<dyn Iterator<Item = PathBuf>> = if exact_tests.is_empty() {
        Box::new(
            WalkDir::new(cli.test262_dir.join("test/language"))
                .into_iter()
                .filter_map(Result::ok)
                .filter(|entry| entry.file_type().is_file())
                .filter(|entry| entry.path().extension() == Some(OsStr::new("js")))
                .map(|entry| entry.into_path()),
        )
    } else {
        Box::new(
            exact_tests
                .iter()
                .map(|relative| cli.test262_dir.join(relative))
                .collect::<Vec<_>>()
                .into_iter(),
        )
    };

    for path in candidate_paths {
        if cli.limit.is_some_and(|limit| summary.attempted >= limit) {
            break;
        }

        let path = path.as_path();
        let display = path.display().to_string();
        let relative_display = path
            .strip_prefix(&cli.test262_dir)
            .map(normalize_path_display)
            .unwrap_or_else(|_| normalize_path_display(path));

        if !cli.contains.is_empty()
            && !cli
                .contains
                .iter()
                .any(|contains| relative_display.contains(contains))
        {
            continue;
        }

        if should_skip_path(path) {
            continue;
        }

        summary.discovered += 1;

        let source =
            fs::read_to_string(path).with_context(|| format!("failed to read `{display}`"))?;
        let metadata = parse_test262_metadata(&source);

        summary.attempted += 1;

        let is_module = metadata.flags.iter().any(|flag| flag == "module");
        let is_async = metadata.flags.iter().any(|flag| flag == "async");
        let force_strict = metadata.flags.iter().any(|flag| flag == "onlyStrict");

        let outcome = run_single_test(
            path,
            &cli.target,
            cli.timeout_seconds,
            is_module,
            is_async,
            force_strict,
        );

        match apply_negative_expectation(&metadata, outcome) {
            Ok(()) => {
                summary.passed += 1;
                println!("PASS {display}");
            }
            Err(TestFailure::Compile(error)) => {
                summary.compile_failed += 1;
                println!("COMPILE_FAIL {display}\n{error}");
                if cli.stop_on_fail {
                    break;
                }
            }
            Err(TestFailure::Runtime(error)) => {
                summary.runtime_failed += 1;
                println!("RUNTIME_FAIL {display}\n{error}");
                if cli.stop_on_fail {
                    break;
                }
            }
        }
    }

    let compliance_percent = if summary.discovered == 0 {
        0.0
    } else {
        (summary.passed as f64 / summary.discovered as f64) * 100.0
    };
    println!(
        "SUMMARY discovered={} attempted={} passed={} compile_failed={} runtime_failed={} compliance_percent={:.2}",
        summary.discovered,
        summary.attempted,
        summary.passed,
        summary.compile_failed,
        summary.runtime_failed,
        compliance_percent,
    );

    if summary.compile_failed > 0 || summary.runtime_failed > 0 {
        anyhow::bail!(
            "test262 run failed: compile_failed={} runtime_failed={}",
            summary.compile_failed,
            summary.runtime_failed
        );
    }

    Ok(())
}

fn normalize_requested_tests(
    test262_dir: &Path,
    tests: &[String],
    tests_from: &[PathBuf],
) -> Result<Vec<String>> {
    let mut seen = HashSet::new();
    let mut normalized_tests = Vec::new();

    for test in tests {
        let normalized = normalize_requested_test(test262_dir, test)?;
        if seen.insert(normalized.clone()) {
            normalized_tests.push(normalized);
        }
    }
    for list_path in tests_from {
        let source = fs::read_to_string(list_path)
            .with_context(|| format!("failed to read test list `{}`", list_path.display()))?;
        for test in extract_requested_tests_from_list(&source) {
            let normalized = normalize_requested_test(test262_dir, &test)?;
            if seen.insert(normalized.clone()) {
                normalized_tests.push(normalized);
            }
        }
    }

    Ok(normalized_tests)
}

fn extract_requested_tests_from_list(source: &str) -> Vec<String> {
    source
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                return None;
            }
            let start = trimmed.find("test/").unwrap_or(0);
            let candidate = &trimmed[start..];
            let end = candidate.find(".js")? + ".js".len();
            Some(candidate[..end].to_string())
        })
        .collect()
}

fn normalize_requested_test(test262_dir: &Path, test: &str) -> Result<String> {
    let requested = Path::new(test);
    let candidate = if requested.is_absolute() {
        requested.to_path_buf()
    } else if requested
        .components()
        .next()
        .is_some_and(|component| component.as_os_str() == "test")
    {
        test262_dir.join(requested)
    } else {
        test262_dir.join("test").join(requested)
    };

    if !candidate.is_file() {
        anyhow::bail!(
            "exact test `{test}` was not found under `{}`",
            test262_dir.display()
        );
    }

    let relative = candidate.strip_prefix(test262_dir).with_context(|| {
        format!(
            "exact test `{}` must live under `{}`",
            candidate.display(),
            test262_dir.display()
        )
    })?;

    Ok(normalize_path_display(relative))
}

fn normalize_path_display(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

enum TestFailure {
    Compile(String),
    Runtime(String),
}

fn run_single_test(
    source_path: &Path,
    target: &str,
    timeout_seconds: u64,
    module: bool,
    async_test: bool,
    force_strict: bool,
) -> Result<(), TestFailure> {
    let tempdir = tempdir().map_err(|error| TestFailure::Compile(error.to_string()))?;
    let trace_timing = std::env::var_os("AYY_TRACE_TEST262_TIMING").is_some();
    let run_started = Instant::now();
    if trace_timing {
        eprintln!("test262 timing run_start {}", source_path.display());
    }

    let wasm_path = tempdir.path().join("test.wasm");
    let options = CompileOptions {
        output: wasm_path.clone(),
        target: target.to_string(),
    };

    let keep_tempdir = std::env::var_os("AYY_KEEP_TEST262_TEMP").is_some()
        || std::env::var_os("AYY_KEEP_TEST262_TEMP_PRECOMPILE").is_some();

    let compile_result =
        compile_unmodified_file_with_goal_and_strict(source_path, &options, module, force_strict);

    compile_result.map_err(|error| TestFailure::Compile(format!("{error:#}")))?;
    if trace_timing {
        eprintln!(
            "test262 timing compile_done {} elapsed_ms={}",
            source_path.display(),
            run_started.elapsed().as_millis()
        );
    }

    if std::env::var_os("AYY_COMPILE_ONLY").is_some() {
        if keep_tempdir {
            eprintln!("kept test262 tempdir: {}", tempdir.path().display());
            std::mem::forget(tempdir);
        }
        return Ok(());
    }

    let mut child = Command::new("wasmtime")
        .arg(&wasm_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| TestFailure::Runtime(error.to_string()))?;

    let timeout = Duration::from_secs(timeout_seconds);
    let started = Instant::now();

    loop {
        if child
            .try_wait()
            .map_err(|error| TestFailure::Runtime(error.to_string()))?
            .is_some()
        {
            break;
        }

        if started.elapsed() >= timeout {
            child
                .kill()
                .map_err(|error| TestFailure::Runtime(error.to_string()))?;
            let output = child
                .wait_with_output()
                .map_err(|error| TestFailure::Runtime(error.to_string()))?;
            return Err(TestFailure::Runtime(format!(
                "timed out after {}s\nstdout:\n{}\nstderr:\n{}",
                timeout_seconds,
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr),
            )));
        }

        thread::sleep(Duration::from_millis(10));
    }

    let output = child
        .wait_with_output()
        .map_err(|error| TestFailure::Runtime(error.to_string()))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    let result = if output.status.success() {
        if async_test && stdout.contains("Test262:AsyncTestFailure:") {
            Err(TestFailure::Runtime(format!(
                "async test reported failure\nstdout:\n{stdout}\nstderr:\n{stderr}",
            )))
        } else if async_test && !stdout.contains("Test262:AsyncTestComplete") {
            Err(TestFailure::Runtime(format!(
                "async test did not report completion\nstdout:\n{stdout}\nstderr:\n{stderr}",
            )))
        } else {
            Ok(())
        }
    } else {
        Err(TestFailure::Runtime(format!(
            "stdout:\n{}\nstderr:\n{}",
            stdout, stderr,
        )))
    };
    if keep_tempdir {
        eprintln!("kept test262 tempdir: {}", tempdir.path().display());
        std::mem::forget(tempdir);
    }
    result
}

fn parse_test262_metadata(source: &str) -> Metadata {
    let Some(start) = source.find("/*---") else {
        return Metadata::default();
    };
    let rest = &source[start + "/*---".len()..];
    let Some((frontmatter, _body)) = rest.split_once("---*/") else {
        return Metadata::default();
    };

    parse_frontmatter(frontmatter)
}

fn parse_frontmatter(frontmatter: &str) -> Metadata {
    let mut metadata = Metadata::default();
    let mut active_list: Option<&str> = None;
    let mut in_negative = false;

    for raw_line in frontmatter.lines() {
        let line = raw_line.trim_end();
        let trimmed = line.trim();

        if trimmed.is_empty() {
            continue;
        }

        if let Some(item) = trimmed.strip_prefix("- ") {
            match active_list {
                Some("flags") => metadata.flags.push(item.trim().to_string()),
                Some("includes") => metadata.includes.push(item.trim().to_string()),
                _ => {}
            }
            continue;
        }

        active_list = None;
        if !raw_line.starts_with(' ') && !raw_line.starts_with('\t') {
            in_negative = false;
        }

        if trimmed.starts_with("negative:") {
            metadata.negative = Some(NegativeExpectation::default());
            in_negative = true;
        } else if in_negative {
            if let Some(phase) = trimmed.strip_prefix("phase:") {
                metadata.negative.get_or_insert_with(Default::default).phase =
                    Some(phase.trim().to_string());
                continue;
            }
            if let Some(error_type) = trimmed.strip_prefix("type:") {
                metadata
                    .negative
                    .get_or_insert_with(Default::default)
                    .error_type = Some(error_type.trim().to_string());
                continue;
            }
        } else if let Some(values) = parse_inline_list(trimmed, "flags:") {
            metadata.flags.extend(values);
        } else if trimmed == "flags:" {
            active_list = Some("flags");
        } else if let Some(values) = parse_inline_list(trimmed, "includes:") {
            metadata.includes.extend(values);
        } else if trimmed == "includes:" {
            active_list = Some("includes");
        }
    }

    metadata
}

fn parse_inline_list(line: &str, key: &str) -> Option<Vec<String>> {
    let remainder = line.strip_prefix(key)?.trim();
    let inner = remainder.strip_prefix('[')?.strip_suffix(']')?;

    if inner.trim().is_empty() {
        return Some(Vec::new());
    }

    Some(
        inner
            .split(',')
            .map(|item| item.trim().trim_matches('"').trim_matches('\'').to_string())
            .collect(),
    )
}

fn apply_negative_expectation(
    metadata: &Metadata,
    outcome: Result<(), TestFailure>,
) -> Result<(), TestFailure> {
    let Some(negative) = metadata.negative.as_ref() else {
        return outcome;
    };

    match negative.phase.as_deref() {
        Some("parse" | "resolution") => match outcome {
            Err(TestFailure::Compile(_)) => Ok(()),
            Ok(()) => Err(TestFailure::Runtime(format!(
                "expected {} failure, but test succeeded",
                negative.phase.as_deref().unwrap_or("negative")
            ))),
            Err(TestFailure::Runtime(error)) => Err(TestFailure::Runtime(format!(
                "expected compile-time {} failure, but execution failed at runtime:\n{error}",
                negative.phase.as_deref().unwrap_or("negative")
            ))),
        },
        Some("runtime") => match outcome {
            Err(TestFailure::Runtime(error))
                if negative
                    .error_type
                    .as_deref()
                    .is_none_or(|expected| error.contains(expected)) =>
            {
                Ok(())
            }
            Err(TestFailure::Runtime(error)) => Err(TestFailure::Runtime(format!(
                "runtime failure did not match expected {:?}:\n{error}",
                negative.error_type
            ))),
            Ok(()) => Err(TestFailure::Runtime(format!(
                "expected runtime {:?} failure, but test succeeded",
                negative.error_type
            ))),
            Err(TestFailure::Compile(error)) => Err(TestFailure::Compile(format!(
                "expected runtime {:?} failure, but compilation failed:\n{error}",
                negative.error_type
            ))),
        },
        _ => outcome,
    }
}

fn should_skip_path(path: &std::path::Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.ends_with("_FIXTURE.js"))
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use super::{
        Metadata, NegativeExpectation, TestFailure, apply_negative_expectation,
        extract_requested_tests_from_list, normalize_requested_test, parse_frontmatter,
        parse_test262_metadata, run_single_test, should_skip_path,
    };

    #[test]
    fn parses_metadata_without_rewriting_test_source() {
        let source = r#"#\041

/*---
flags: [raw]
negative:
  phase: parse
  type: SyntaxError
---*/

throw "unreachable";
"#;
        let metadata = parse_test262_metadata(source);

        assert_eq!(metadata.flags, vec!["raw".to_string()]);
        assert_eq!(
            metadata.negative,
            Some(NegativeExpectation {
                phase: Some("parse".to_string()),
                error_type: Some("SyntaxError".to_string()),
            })
        );
    }

    #[test]
    fn parses_negative_phase_and_type() {
        let metadata = parse_frontmatter(
            r#"
negative:
  phase: parse
  type: SyntaxError
"#,
        );

        assert_eq!(
            metadata.negative,
            Some(NegativeExpectation {
                phase: Some("parse".to_string()),
                error_type: Some("SyntaxError".to_string()),
            })
        );
    }

    #[test]
    fn parses_inline_flags_and_includes_from_metadata() {
        let metadata = parse_test262_metadata(
            r#"
/*---
flags: [module, async]
includes: [propertyHelper.js, compareArray.js]
---*/

assert.sameValue(1, 1);
"#,
        );

        assert_eq!(
            metadata.flags,
            vec!["module".to_string(), "async".to_string()]
        );
        assert_eq!(
            metadata.includes,
            vec![
                "propertyHelper.js".to_string(),
                "compareArray.js".to_string()
            ]
        );
    }

    #[test]
    fn parse_negative_tests_pass_on_compile_failure() {
        let metadata = Metadata {
            negative: Some(NegativeExpectation {
                phase: Some("parse".to_string()),
                error_type: Some("SyntaxError".to_string()),
            }),
            ..Metadata::default()
        };

        assert!(
            apply_negative_expectation(&metadata, Err(TestFailure::Compile("syntax".to_string())))
                .is_ok()
        );
    }

    #[test]
    fn runtime_negative_tests_require_matching_error_type() {
        let metadata = Metadata {
            negative: Some(NegativeExpectation {
                phase: Some("runtime".to_string()),
                error_type: Some("TypeError".to_string()),
            }),
            ..Metadata::default()
        };

        assert!(
            apply_negative_expectation(
                &metadata,
                Err(TestFailure::Runtime("TypeError: boom".to_string()))
            )
            .is_ok()
        );
        assert!(
            apply_negative_expectation(
                &metadata,
                Err(TestFailure::Runtime("ReferenceError: boom".to_string()))
            )
            .is_err()
        );
    }

    #[test]
    fn skips_fixture_files() {
        assert!(should_skip_path(Path::new(
            "/tmp/test262/test/language/module-code/eval-rqstd-order-1_FIXTURE.js"
        )));
        assert!(!should_skip_path(Path::new(
            "/tmp/test262/test/language/expressions/yield/rhs-iter.js"
        )));
    }

    #[test]
    fn normalizes_exact_test_paths_from_test_prefix() {
        let tempdir = tempfile::tempdir().unwrap();
        let path = tempdir
            .path()
            .join("test/language/expressions/yield/rhs-iter.js");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "").unwrap();

        let normalized = normalize_requested_test(
            tempdir.path(),
            "test/language/expressions/yield/rhs-iter.js",
        )
        .unwrap();

        assert_eq!(normalized, "test/language/expressions/yield/rhs-iter.js");
    }

    #[test]
    fn normalizes_exact_test_paths_from_language_prefix() {
        let tempdir = tempfile::tempdir().unwrap();
        let path = tempdir
            .path()
            .join("test/language/expressions/yield/rhs-iter.js");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "").unwrap();

        let normalized =
            normalize_requested_test(tempdir.path(), "language/expressions/yield/rhs-iter.js")
                .unwrap();

        assert_eq!(normalized, "test/language/expressions/yield/rhs-iter.js");
    }

    #[test]
    fn extracts_test_paths_from_progress_tracker() {
        let tests = extract_requested_tests_from_list(
            r#"
0/2 (0.00%)
- [ ] comments
  - [ ] test/language/comments/S7.4_A1_T1.js
  - [ ] test/language/comments/S7.4_A1_T2.js (impossible with AOT)
"#,
        );

        assert_eq!(
            tests,
            vec![
                "test/language/comments/S7.4_A1_T1.js".to_string(),
                "test/language/comments/S7.4_A1_T2.js".to_string()
            ]
        );
    }

    #[test]
    fn runner_compiles_test262_source_without_await_rewrite_fallback() {
        let tempdir = tempfile::tempdir().unwrap();
        let path = tempdir.path().join("escaped-await-async-generator.js");
        fs::write(&path, "class C { async *gen() { var \\u0061wait; } }\n").unwrap();

        let outcome = run_single_test(&path, "wasm32-wasip2", 30, false, false, false);

        match outcome {
            Err(TestFailure::Compile(error)) => {
                assert!(
                    error.contains("await"),
                    "expected an unmodified parse error for escaped await, got:\n{error}"
                );
            }
            Ok(()) => panic!("test262 runner unexpectedly accepted rewritten source"),
            Err(TestFailure::Runtime(_)) => {
                panic!("invalid escaped await should fail during unmodified parsing")
            }
        }
    }
}
