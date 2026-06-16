## Compiler

Do not stop until you have successfully implemented a compiler (written in the Rust language) called `AyeYaiYai` that compiles JavaScript into WASI 0.2 (Preview 2). The compiler itself must be written in Rust, but it must directly emit the final Wasm/WASI output itself. You should not just interpret JavaScript in Wasi, like with Boa or QuickJS compiled into Wasi. No, you must literally compile JavaScript into WASI 0.2 (Preview 2).

`AyeYaiYai` must perform ahead-of-time compilation from JavaScript source directly to Wasm/WASI. The compiler must create the Wasm bytes directly itself from Rust. There must be no intermediate generated source language or source-like representation that is then compiled or assembled into Wasm, including generated Rust, C, C++, Zig, WAT, or any other language handed off to `rustc`, `clang`, `zig`, `wat2wasm`, or another compiler toolchain stage. The generated module must not embed a JavaScript engine, bytecode VM, AST interpreter, source parser, `eval` interpreter, or general-purpose JS runtime that executes JS semantics dynamically at runtime. JavaScript constructs must be lowered at compile time into Wasm control flow, data operations, and calls.

For example, here's what the CLI API might look like:

```bash
ayeyaiyai test.js -o test.wasm
wasmtime test.wasm
```

You will know that you have successfully finished creating the compiler when it passes as much of the [test262](https://github.com/tc39/test262) test suite for the `language` category using `wasmtime` as is possible in a true AOT compiler.

## Git

Commit after every commit-worthy milestone of similar work is finished. Skip gpg signing for these commits. Do not push.

## test262 Language Tracking

When a `test/language/...` test is fixed, rerun it and require it to pass through the direct JS->Wasm backend before marking it complete.
Then immediately in `test262-language-progress.md`:

- mark that exact test line with `[x]`
- update the top progress line so it still contains `x/y (z%)`
- immediately below that top progress line, maintain a `Sub-category progress` block at the top of `test262-language-progress.md` that lists every top-level `test/language` sub-category in tracker order, each showing `completed/total (percent%)`
- refresh the overall top progress line and every sub-category progress line every time any checkbox changes, so overall language completion and each sub-category completion are visible at a glance without scrolling
- If a test is impossible to complete due to the fundamental nature of AOT compiling, then leave it unchecked and within parens just after the checkbox add (impossible with AOT)
