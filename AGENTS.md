## Compiler

You are implementing a compiler (written in the Rust language) called `AyeYaiYai` that compiles JavaScript into WASI 0.2 (Preview 2). The compiler itself must be written in Rust, but it must directly emit the final Wasm/WASI output itself. You should not just interpret JavaScript in Wasi, like with Boa or QuickJS compiled into Wasi. No, you must literally compile JavaScript into WASI 0.2 (Preview 2).

`AyeYaiYai` must perform ahead-of-time compilation from JavaScript source directly to Wasm/WASI. The compiler must create the Wasm bytes directly itself from Rust. There must be no intermediate generated source language or source-like representation that is then compiled or assembled into Wasm, including generated Rust, C, C++, Zig, WAT, or any other language handed off to `rustc`, `clang`, `zig`, `wat2wasm`, or another compiler toolchain stage. The generated module must not embed a JavaScript engine, bytecode VM, AST interpreter, source parser, `eval` interpreter, or general-purpose JS runtime that executes JS semantics dynamically at runtime. JavaScript constructs must be lowered at compile time into Wasm control flow, data operations, and calls.

For example, here's what the CLI API might look like:

```bash
ayeyaiyai test.js -o test.wasm
wasmtime test.wasm
```

## Git

Commit after every commit-worthy milestone of similar work is finished. Skip gpg signing for these commits. Do not push.

## test262 Language Tracking

If you are asked to work on passing the test262 language category:

When a `test/language/...` test is fixed, rerun it and require it to pass through the direct JS->Wasm backend before marking it complete.
Then immediately in `test262-language-progress.md`:

- mark that exact test line with `[x]`
- update the top progress line so it still contains `x/y (z%)`
- immediately below that top progress line, maintain a `Sub-category progress` block at the top of `test262-language-progress.md` that lists every top-level `test/language` sub-category in tracker order, each showing `completed/total (percent%)`
- refresh the overall top progress line and every sub-category progress line every time any checkbox changes, so overall language completion and each sub-category completion are visible at a glance without scrolling
- If a test is impossible to complete due to the fundamental nature of AOT compiling, then leave it unchecked and within parens just after the checkbox add (impossible with AOT)

## Mega Spec Suite

Build `tests/mega-spec-suite` as a spec-mapped ECMAScript conformance suite for AyeYaiYai.

- The purpose is to implement the ECMAScript specification, not only to write tests.
- For each subclause, implement the spec in AyeYaiYai, create the detailed mega tests, run them, then keep fixing the implementation until the subclause is truly handled.
- A subclause is not done until all non-impossible tests pass, each test file compiles in 10 seconds or less, runtime is within 10x of Node.js when compared fairly, and you believe the subclause is implemented correctly.
- Passing tests is not enough by itself. Before calling a subclause done, read the pinned spec text and audit every grammar production, static semantic rule, runtime semantic step, and normative note with observable behavior.
- Faithful means every observable requirement in the subclause is implemented and tested, unless it is marked impossible with AOT for a real AOT reason.
- Do not stop at "probably covered". If a rule can change parse success, early errors, runtime result, thrown error, binding resolution, evaluation order, completion, timing budget, or emitted output, add behavior coverage for it.
- If a rule depends on another clause, test the interaction enough to prove this subclause is using that rule correctly, then leave the deeper rule to its own subclause.
- Treat valid tests as requirements.
- Do not change a valid test to make it fit correctness, compile-time, or runtime requirements.
- A valid large or stress test is not invalid because it is slow, hard to compile, or exposes an unrelated compiler bug.
- If a valid test exposes a correctness bug, compile-time budget failure, runtime budget failure, or seemingly unrelated compiler issue, fix AyeYaiYai itself. Fix the suite runner only when the runner is measuring or executing the test incorrectly. Do not weaken, delete, split, or reshape the valid test just to make it pass.
- Change a test only when the test is invalid, does not match the pinned spec, does not test the stated behavior, or violates the suite rules.
- Do not call a subclause complete until the audit finds no missing observable behavior, the implementation matches the spec, and all non-impossible tests pass within the compile and runtime budgets.
- Maintain nested directories named by ECMA-262 subclause, each with one `spec.yaml`.
- Use pinned ECMA-262 16th edition URLs: `https://262.ecma-international.org/16.0/#...`.
- In each `spec.yaml`, list observable behavior names under `behaviors`, then document each under `behavior_descriptions` with matching hyphenated keys and multiline descriptions.
- Observable behaviors must cover every externally detectable rule needed for spec compliance, including accepted syntax, rejected syntax, early errors, runtime results, thrown errors, evaluation order, strict/sloppy/module differences, edge cases, and boundary cases.
- If a subclause rule can affect program output, completion, thrown error, parse success, or compile-time rejection, it must be represented by one or more behaviors.
- Validate every `behaviors` entry has a matching `behavior_descriptions` entry and a `behaviors/<behavior-key>/` directory.
- Put test files under `behaviors/<behavior-key>/` beside that subclause's `spec.yaml`.
- Every non-impossible behavior must have `standard.js`, `large.js`, and `stress.js`.
- `standard.js` must have at least 100 meaningful lines, `large.js` at least 500, and `stress.js` at least 1000.
- `standard` should cover the behavior in a clear non-trivial setting; `large` should add many meaningful variations and surrounding language constructs; `stress` should heavily exercise the behavior across deep, broad, or repeated contexts.
- Larger sizes must increase semantic variety and compiler work, not repeat the same tiny case.
- Meaningful lines are executable or syntactically relevant; blank lines, comments, and repeated no-op padding do not count.
- If a behavior needs variants, name files `<variant>.<size>.js`.
- Valid variants are `script.sloppy`, `script.strict`, and `module`; do not write `module.strict` because modules are already strict.
- Each required variant must have standard, large, and stress coverage.
- Each `.js` test must start with this header format:
  - `// behavior: <behavior-key>`
  - `// expected: pass|runtime-error|syntax-error|early-error|impossible-with-AOT`
  - `// goal: script|module`
  - `// size: standard|large|stress`
  - `// variant: script.sloppy|script.strict|module`
  - `// impossible_reason: <reason>` only when `expected` is `impossible-with-AOT`
- Valid `expected` values are `pass`, `runtime-error`, `syntax-error`, `early-error`, and `impossible-with-AOT`.
- `pass` means Node and `ayeyaiyai -> wasm -> wasmtime` both complete successfully.
- `runtime-error` means both compile, both run, and both throw at runtime.
- `syntax-error` means Node and AyeYaiYai both reject the file during parsing.
- `early-error` means the file parses, but Node and AyeYaiYai both reject it before execution due to static semantics.
- `impossible-with-AOT` means the behavior is fundamentally incompatible with true AOT compilation; AyeYaiYai is not expected to pass it.
- Do not add an expected value for unimplemented valid JavaScript. Implement the missing compiler behavior instead.
- For `pass`, make the file run assertions to completion.
- For `runtime-error`, put meaningful setup and assertions before the expected throw.
- For `syntax-error`, put the syntax error after meaningful preceding syntax when possible.
- For `early-error`, keep the file parseable and put the static-semantics violation after meaningful preceding syntax when possible.
- For `impossible-with-AOT`, document why in the test header and do not count AyeYaiYai failure as a test failure.
- Use Node.js as the semantic oracle. For executable tests, compare against `ayeyaiyai -> wasm -> wasmtime`; for rejection tests, compare Node's rejection against AyeYaiYai's compile-time rejection.
- Track compile time for every AyeYaiYai test; all test files should compile in under ten seconds.
- Track execution time for every AyeYaiYai test; all test files should execute within 10x of the equivalent Node execution time.
