use std::{fs, process::Command};

use ayeyaiyai::{CompileOptions, compile_file, compile_file_with_goal, compile_source_with_goal};

#[test]
fn compile_options_are_constructible() {
    let options = CompileOptions {
        output: "out.wasm".into(),
        target: "wasm32-wasip2".to_string(),
    };

    assert_eq!(options.target, "wasm32-wasip2");
}

#[test]
fn compile_file_accepts_numeric_functions_on_the_direct_wasm_path() {
    let tempdir = tempfile::tempdir().unwrap();
    let input = tempdir.path().join("numeric-functions.js");
    let output = tempdir.path().join("numeric-functions.wasm");

    fs::write(
        &input,
        r#"
        function sumTo(limit) {
          let total = 0;

          for (let i = 0; i <= limit; i++) {
            total = total + i;
          }

          return total;
        }

        let counter = 1;
        let before = counter++;
        let after = ++counter;

        console.log("sum", sumTo(5), before, after);
        "#,
    )
    .unwrap();

    let options = CompileOptions {
        output: output.clone(),
        target: "wasm32-wasip2".to_string(),
    };

    compile_file(&input, &options).unwrap();
    assert!(output.exists());
}

#[test]
fn compiles_module_goal_sources_on_the_direct_wasm_path() {
    let tempdir = tempfile::tempdir().unwrap();
    let output = tempdir.path().join("module.wasm");
    let options = CompileOptions {
        output: output.clone(),
        target: "wasm32-wasip2".to_string(),
    };

    compile_source_with_goal(
        r#"
        let value = 1;
        console.log(value);
        "#,
        &options,
        true,
    )
    .unwrap();

    let run = Command::new("wasmtime").arg(&output).output().unwrap();
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&run.stdout), "1\n");
}

#[test]
fn compile_file_uses_direct_wasm_backend_for_supported_programs() {
    let tempdir = tempfile::tempdir().unwrap();
    let input = tempdir.path().join("direct-wasm.js");
    let output = tempdir.path().join("direct-wasm.wasm");

    fs::write(
        &input,
        r#"
        let total = 0;
        let i = 1;

        while (i <= 3) {
          total = total + i;
          i = i + 1;
        }

        if (total === 6) {
          console.log("ok");
        } else {
          console.log("bad");
        }
        "#,
    )
    .unwrap();

    let options = CompileOptions {
        output: output.clone(),
        target: "wasm32-wasip2".to_string(),
    };

    compile_file(&input, &options).unwrap();

    let run = Command::new("wasmtime").arg(&output).output().unwrap();
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&run.stdout), "ok\n");
}

#[test]
fn compile_file_throws_on_strict_assignment_to_accessor_without_setter() {
    let tempdir = tempfile::tempdir().unwrap();
    let input = tempdir.path().join("strict-accessor-assignment.js");
    let output = tempdir.path().join("strict-accessor-assignment.wasm");

    fs::write(
        &input,
        r#"
        "use strict";
        var obj = {};
        Object.defineProperty(obj, "prop", {
          get: function() { return 11; },
          set: undefined,
          enumerable: true,
          configurable: true
        });

        try {
          obj.prop = 20;
          console.log("no throw");
        } catch (error) {
          console.log(error.name, obj.prop);
        }
        "#,
    )
    .unwrap();

    let options = CompileOptions {
        output: output.clone(),
        target: "wasm32-wasip2".to_string(),
    };

    compile_file(&input, &options).unwrap();

    let run = Command::new("wasmtime").arg(&output).output().unwrap();
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&run.stdout), "TypeError 11\n");
}

#[test]
fn compile_file_exposes_sloppy_implicit_global_assignment_as_global_property_descriptor() {
    let tempdir = tempfile::tempdir().unwrap();
    let input = tempdir.path().join("implicit-global-descriptor.js");
    let output = tempdir.path().join("implicit-global-descriptor.wasm");

    fs::write(
        &input,
        r#"
        function assignImplicit() {
          __ayy_implicit_descriptor_target__ = 42;
        }
        assignImplicit();

        var desc = Object.getOwnPropertyDescriptor(
          this,
          "__ayy_implicit_descriptor_target__"
        );
        console.log(desc.value, desc.writable, desc.enumerable, desc.configurable);
        "#,
    )
    .unwrap();

    let options = CompileOptions {
        output: output.clone(),
        target: "wasm32-wasip2".to_string(),
    };

    compile_file(&input, &options).unwrap();

    let run = Command::new("wasmtime").arg(&output).output().unwrap();
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&run.stdout), "42 true true true\n");
}

#[test]
fn compile_file_throws_on_strict_assignment_to_readonly_builtin_data_properties() {
    let tempdir = tempfile::tempdir().unwrap();
    let input = tempdir.path().join("readonly-builtin-assignment.js");
    let output = tempdir.path().join("readonly-builtin-assignment.wasm");

    fs::write(
        &input,
        r#"
        "use strict";

        var global = this;
        try {
          Number.MAX_VALUE = 42;
          console.log("number no throw");
        } catch (error) {
          console.log(error.name);
        }
        try {
          Math.PI = 20;
          console.log("math no throw");
        } catch (error) {
          console.log(error.name);
        }
        try {
          (function() { global.undefined = 42; })();
          console.log("global no throw");
        } catch (error) {
          console.log(error.name);
        }
        try {
          Function.length = 42;
          console.log("function no throw");
        } catch (error) {
          console.log(error.name);
        }
        "#,
    )
    .unwrap();

    let options = CompileOptions {
        output: output.clone(),
        target: "wasm32-wasip2".to_string(),
    };

    compile_file(&input, &options).unwrap();

    let run = Command::new("wasmtime").arg(&output).output().unwrap();
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&run.stdout),
        "TypeError\nTypeError\nTypeError\nTypeError\n"
    );
}

#[test]
fn compile_file_does_not_create_own_property_over_inherited_non_writable_property() {
    let tempdir = tempfile::tempdir().unwrap();
    let input = tempdir.path().join("inherited-non-writable-assignment.js");
    let output = tempdir
        .path()
        .join("inherited-non-writable-assignment.wasm");

    fs::write(
        &input,
        r#"
        function Foo() {}
        Object.defineProperty(Foo.prototype, "bar", { value: "unwritable" });

        var o = new Foo();
        var result = (o.bar = "overridden");
        console.log(
          result,
          o.hasOwnProperty("bar") ? "own" : "inherited",
          o.bar
        );
        "#,
    )
    .unwrap();

    let options = CompileOptions {
        output: output.clone(),
        target: "wasm32-wasip2".to_string(),
    };

    compile_file(&input, &options).unwrap();

    let run = Command::new("wasmtime").arg(&output).output().unwrap();
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&run.stdout),
        "overridden inherited unwritable\n"
    );
}

#[test]
fn compile_file_preserves_assignment_reference_across_direct_eval_var_declaration() {
    let tempdir = tempfile::tempdir().unwrap();
    let input = tempdir.path().join("direct-eval-assignment-reference.js");
    let output = tempdir.path().join("direct-eval-assignment-reference.wasm");

    fs::write(
        &input,
        r#"
        function testAssignment() {
          var x = 0;
          var innerX = (function() {
            x = (eval("var x;"), 1);
            return x;
          })();

          console.log(innerX, x);
        }

        testAssignment();
        "#,
    )
    .unwrap();

    let options = CompileOptions {
        output: output.clone(),
        target: "wasm32-wasip2".to_string(),
    };

    compile_file(&input, &options).unwrap();

    let run = Command::new("wasmtime").arg(&output).output().unwrap();
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&run.stdout), "undefined 1\n");
}

#[test]
fn compile_file_throws_when_strict_global_capture_assignment_source_is_deleted_by_rhs() {
    let tempdir = tempfile::tempdir().unwrap();
    let input = tempdir
        .path()
        .join("strict-global-capture-putvalue-delete.js");
    let output = tempdir
        .path()
        .join("strict-global-capture-putvalue-delete.wasm");

    fs::write(
        &input,
        r#"
        var count = 0;
        var global = this;

        Object.defineProperty(this, "x", {
          configurable: true,
          value: 1
        });

        (function() {
          "use strict";
          try {
            count++;
            x = (delete global.x, 2);
            count++;
          } catch (error) {
            console.log(error.name);
          }
          count++;
        })();

        console.log(count, 'x' in this, 'x' in global);
        "#,
    )
    .unwrap();

    let options = CompileOptions {
        output: output.clone(),
        target: "wasm32-wasip2".to_string(),
    };

    compile_file(&input, &options).unwrap();

    let run = Command::new("wasmtime").arg(&output).output().unwrap();
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&run.stdout),
        "ReferenceError\n2 false false\n"
    );
}

#[test]
fn compile_file_throws_when_strict_with_capture_assignment_source_is_deleted_by_rhs() {
    let tempdir = tempfile::tempdir().unwrap();
    let input = tempdir
        .path()
        .join("strict-with-capture-putvalue-delete.js");
    let output = tempdir
        .path()
        .join("strict-with-capture-putvalue-delete.wasm");

    fs::write(
        &input,
        r#"
        var count = 0;
        var scope = { x: 1 };

        with (scope) {
          (function() {
            "use strict";
            try {
              count++;
              x = (delete scope.x, 2);
              count++;
            } catch (error) {
              console.log(error.name);
            }
            count++;
          })();
        }

        console.log(count, 'x' in scope);
        "#,
    )
    .unwrap();

    let options = CompileOptions {
        output: output.clone(),
        target: "wasm32-wasip2".to_string(),
    };

    compile_file(&input, &options).unwrap();

    let run = Command::new("wasmtime").arg(&output).output().unwrap();
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&run.stdout),
        "ReferenceError\n2 false\n"
    );
}

#[test]
fn compile_source_preserves_instance_prototype_across_private_field_snapshot_updates() {
    let tempdir = tempfile::tempdir().unwrap();
    let output = tempdir.path().join("private-instance-prototype.wasm");
    let options = CompileOptions {
        output: output.clone(),
        target: "wasm32-wasip2".to_string(),
    };

    compile_source_with_goal(
        r#"
        class C {
          #$_; #__; #o_; #℘_; #ZW_‌_NJ_; #ZW_‍_J_;
          $(v) { this.#$_ = v; return this.#$_; }
          _(v) { this.#__ = v; return this.#__; }
          o(v) { this.#o_ = v; return this.#o_; }
          ℘(v) { this.#℘_ = v; return this.#℘_; }
          ZW_‌_NJ(v) { this.#ZW_‌_NJ_ = v; return this.#ZW_‌_NJ_; }
          ZW_‍_J(v) { this.#ZW_‍_J_ = v; return this.#ZW_‍_J_; }
        }
        const c = new C();
        console.log(c.$(1));
        console.log(c._(1));
        console.log(c.o(1));
        console.log(c.℘(1));
        console.log(c.ZW_‌_NJ(1));
        console.log(c.ZW_‍_J(1));
        "#,
        &options,
        false,
    )
    .unwrap();

    let run = Command::new("wasmtime").arg(&output).output().unwrap();
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&run.stdout), "1\n1\n1\n1\n1\n1\n");
}

#[test]
fn compile_file_executes_for_of_continue_via_direct_wasm_backend() {
    let tempdir = tempfile::tempdir().unwrap();
    let input = tempdir.path().join("for-of-continue.js");
    let output = tempdir.path().join("for-of-continue.wasm");

    fs::write(
        &input,
        r#"
        let sum = 0;
        for (const value of [1, 2, 3]) {
          if (value === 2) {
            continue;
          }
          sum = sum + value;
        }
        console.log(sum);
        "#,
    )
    .unwrap();

    let options = CompileOptions {
        output: output.clone(),
        target: "wasm32-wasip2".to_string(),
    };

    compile_file(&input, &options).unwrap();

    let run = Command::new("wasmtime").arg(&output).output().unwrap();
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&run.stdout), "4\n");
}

#[test]
fn compile_file_executes_nested_for_of_labeled_continue_outer_loop_via_direct_wasm_backend() {
    let tempdir = tempfile::tempdir().unwrap();
    let input = tempdir.path().join("nested-for-of-labeled-continue.js");
    let output = tempdir.path().join("nested-for-of-labeled-continue.wasm");

    fs::write(
        &input,
        r#"
        let sum = 0;
        outer: for (const outer_value of [1, 2, 3]) {
          for (const inner_value of [4, 5, 6]) {
            if (inner_value === 5) {
              continue outer;
            }
            sum = sum + inner_value;
          }
        }
        console.log(sum);
        "#,
    )
    .unwrap();

    let options = CompileOptions {
        output: output.clone(),
        target: "wasm32-wasip2".to_string(),
    };

    compile_file(&input, &options).unwrap();

    let run = Command::new("wasmtime").arg(&output).output().unwrap();
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&run.stdout), "12\n");
}

#[test]
fn compile_file_executes_nested_for_of_continue_outer_loop_closes_inner_only() {
    let tempdir = tempfile::tempdir().unwrap();
    let input = tempdir
        .path()
        .join("nested-for-of-labeled-continue-inner-close.js");
    let output = tempdir
        .path()
        .join("nested-for-of-labeled-continue-inner-close.wasm");

    fs::write(
        &input,
        r#"
        function makeIterable(values, tracker) {
          let index = 0;
          const iterator = {
            next: function () {
              if (index >= values.length) {
                return { done: true };
              }
              return { value: values[index++], done: false };
            },
            return: function () {
              tracker.value = tracker.value + 1;
              return { done: true };
            },
          };

          iterator[Symbol.iterator] = function () {
            return iterator;
          };

          return iterator;
        }

        let outerClosed = { value: 0 };
        let innerClosed = { value: 0 };

        outer: for (const outerValue of makeIterable([1, 2], outerClosed)) {
          for (const innerValue of makeIterable([4, 5], innerClosed)) {
            if (innerValue === 5) {
              continue outer;
            }
            let skip = outerValue + innerValue;
          }
        }

        console.log(outerClosed.value, innerClosed.value);
        "#,
    )
    .unwrap();

    let options = CompileOptions {
        output: output.clone(),
        target: "wasm32-wasip2".to_string(),
    };

    compile_file(&input, &options).unwrap();

    let run = Command::new("wasmtime").arg(&output).output().unwrap();
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&run.stdout), "0 2\n");
}

#[test]
fn compile_file_executes_generator_parameter_array_rest_default_from_global_array() {
    let tempdir = tempfile::tempdir().unwrap();
    let input = tempdir
        .path()
        .join("generator-parameter-array-rest-default.js");
    let output = tempdir
        .path()
        .join("generator-parameter-array-rest-default.wasm");

    fs::write(
        &input,
        r#"
        var values = [2, 1, 3];
        class C {
          *method([[...x] = values]) {
            console.log(x[0], x[1], x[2], x.length, Array.isArray(x));
          }
        }

        new C().method([]).next();
        "#,
    )
    .unwrap();

    let options = CompileOptions {
        output: output.clone(),
        target: "wasm32-wasip2".to_string(),
    };

    compile_file(&input, &options).unwrap();

    let run = Command::new("wasmtime").arg(&output).output().unwrap();
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&run.stdout), "2 1 3 3 true\n");
}

#[test]
fn compile_file_executes_async_generator_private_method_yield_star_async_return_chain() {
    let tempdir = tempfile::tempdir().unwrap();
    let input = tempdir.path().join("async-private-yield-star-return.js");
    let output = tempdir.path().join("async-private-yield-star-return.wasm");

    fs::write(
        &input,
        r#"
        var obj = {
          [Symbol.asyncIterator]() {
            var returnCount = 0;
            return {
              next() {
                return { value: "next-value-1", done: false };
              },
              return() {
                returnCount++;
                if (returnCount === 1) {
                  return {
                    then(resolve) {
                      resolve({
                        get value() { return "return-value-1"; },
                        get done() { return false; }
                      });
                    }
                  };
                }
                return {
                  then(resolve) {
                    resolve({
                      get value() { return "return-value-2"; },
                      get done() { return true; }
                    });
                  }
                };
              }
            };
          }
        };

        class C {
          async *#gen() {
            yield* obj;
          }
          get gen() { return this.#gen; }
        }

        let iter = new C().gen();
        iter.next().then(function(v) {
          iter.return("return-arg-1").then(function(v2) {
            iter.return("return-arg-2").then(function(v3) {
              console.log(v.value, v.done, v2.value, v2.done, v3.value, v3.done);
            });
          });
        });
        "#,
    )
    .unwrap();

    let options = CompileOptions {
        output: output.clone(),
        target: "wasm32-wasip2".to_string(),
    };

    compile_file(&input, &options).unwrap();

    let run = Command::new("wasmtime").arg(&output).output().unwrap();
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&run.stdout),
        "next-value-1 false return-value-1 false return-value-2 true\n"
    );
}

#[test]
fn compile_file_executes_async_generator_private_method_non_callable_async_iterator_rejection_constructor_chain()
 {
    let tempdir = tempfile::tempdir().unwrap();
    let input = tempdir
        .path()
        .join("async-private-yield-star-non-callable-async-iterator.js");
    let output = tempdir
        .path()
        .join("async-private-yield-star-non-callable-async-iterator.wasm");

    fs::write(
        &input,
        r#"
        var obj = {
          get [Symbol.iterator]() {
            throw new Error("it should not get Symbol.iterator");
          },
          [Symbol.asyncIterator]: 0
        };

        var callCount = 0;

        class C {
          async *#gen() {
            callCount += 1;
            yield* obj;
          }
          get gen() { return this.#gen; }
        }

        var iter = new C().gen();
        iter.next().then(function() {
          console.log("fulfilled-first");
        }, function(v) {
          console.log(v.constructor === TypeError, callCount);
          iter.next().then(function(v2) {
            console.log(v2.done, v2.value);
          });
        });
        "#,
    )
    .unwrap();

    let options = CompileOptions {
        output: output.clone(),
        target: "wasm32-wasip2".to_string(),
    };

    compile_file(&input, &options).unwrap();

    let run = Command::new("wasmtime").arg(&output).output().unwrap();
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&run.stdout),
        "true 1\ntrue undefined\n"
    );
}

#[test]
fn compile_file_executes_async_generator_yield_star_sync_next_chain() {
    let tempdir = tempfile::tempdir().unwrap();
    let input = tempdir.path().join("async-yield-star-sync-next.js");
    let output = tempdir.path().join("async-yield-star-sync-next.wasm");

    fs::write(
        &input,
        r#"
        var log = [];
        var obj = {
          get [Symbol.iterator]() {
            log.push({ name: "get [Symbol.iterator]", thisValue: this });
            return function() {
              log.push({ name: "call [Symbol.iterator]", thisValue: this, args: [...arguments] });
              var nextCount = 0;
              return {
                name: "syncIterator",
                get next() {
                  log.push({ name: "get next", thisValue: this });
                  return function() {
                    log.push({ name: "call next", thisValue: this, args: [...arguments] });
                    nextCount++;
                    if (nextCount === 1) {
                      return {
                        name: "next-result-1",
                        get value() {
                          log.push({ name: "get next value (1)", thisValue: this });
                          return "next-value-1";
                        },
                        get done() {
                          log.push({ name: "get next done (1)", thisValue: this });
                          return false;
                        }
                      };
                    }
                    return {
                      name: "next-result-2",
                      get value() {
                        log.push({ name: "get next value (2)", thisValue: this });
                        return "next-value-2";
                      },
                      get done() {
                        log.push({ name: "get next done (2)", thisValue: this });
                        return true;
                      }
                    };
                  };
                }
              };
            };
          },
          get [Symbol.asyncIterator]() {
            log.push({ name: "get [Symbol.asyncIterator]" });
            return null;
          }
        };

        async function *gen() {
          log.push({ name: "before yield*" });
          var value = yield* obj;
          log.push({ name: "after yield*", value: value });
          return "return-value";
        }

        let iter = gen();
        iter.next("next-arg-1").then(function(v) {
          console.log(
            "after-first",
            v.value,
            v.done,
            log.length,
            log[0].name,
            log[1].name,
            log[2].name
          );
          iter.next("next-arg-2").then(function(v2) {
            console.log(
              "after-second",
              v2.value,
              v2.done,
              log.length,
              log[11].name,
              log[11].value
            );
          });
        });
        "#,
    )
    .unwrap();

    let options = CompileOptions {
        output: output.clone(),
        target: "wasm32-wasip2".to_string(),
    };

    compile_file(&input, &options).unwrap();

    let run = Command::new("wasmtime").arg(&output).output().unwrap();
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&run.stdout),
        "after-first next-value-1 false 8 before yield* get [Symbol.asyncIterator] get [Symbol.iterator]\nafter-second return-value true 12 after yield* next-value-2\n"
    );
}

#[test]
fn compile_file_executes_async_generator_yield_star_sync_next_arrow_chain() {
    let tempdir = tempfile::tempdir().unwrap();
    let input = tempdir.path().join("async-yield-star-sync-next-arrow.js");
    let output = tempdir.path().join("async-yield-star-sync-next-arrow.wasm");

    fs::write(
        &input,
        r#"
        var log = [];
        var obj = {
          get [Symbol.iterator]() {
            log.push({ name: "get [Symbol.iterator]", thisValue: this });
            return function() {
              log.push({ name: "call [Symbol.iterator]", thisValue: this, args: [...arguments] });
              var nextCount = 0;
              return {
                name: "syncIterator",
                get next() {
                  log.push({ name: "get next", thisValue: this });
                  return function() {
                    log.push({ name: "call next", thisValue: this, args: [...arguments] });
                    nextCount++;
                    if (nextCount === 1) {
                      return {
                        name: "next-result-1",
                        get value() {
                          log.push({ name: "get next value (1)", thisValue: this });
                          return "next-value-1";
                        },
                        get done() {
                          log.push({ name: "get next done (1)", thisValue: this });
                          return false;
                        }
                      };
                    }
                    return {
                      name: "next-result-2",
                      get value() {
                        log.push({ name: "get next value (2)", thisValue: this });
                        return "next-value-2";
                      },
                      get done() {
                        log.push({ name: "get next done (2)", thisValue: this });
                        return true;
                      }
                    };
                  };
                }
              };
            };
          },
          get [Symbol.asyncIterator]() {
            log.push({ name: "get [Symbol.asyncIterator]" });
            return null;
          }
        };

        async function *gen() {
          log.push({ name: "before yield*" });
          var value = yield* obj;
          log.push({ name: "after yield*", value: value });
          return "return-value";
        }

        let iter = gen();
        iter.next("next-arg-1").then(v => {
          iter.next("next-arg-2").then(v2 => {
            console.log(
              "after-second",
              v.value,
              v.done,
              v2.value,
              v2.done,
              log.length,
              log[9] && log[9].name,
              log[9] && log[9].thisValue && log[9].thisValue.name,
              log[10] && log[10].name,
              log[10] && log[10].thisValue && log[10].thisValue.name,
              log[11] && log[11].name,
              log[11] && log[11].value
            );
          });
        });
        "#,
    )
    .unwrap();

    let options = CompileOptions {
        output: output.clone(),
        target: "wasm32-wasip2".to_string(),
    };

    compile_file(&input, &options).unwrap();

    let run = Command::new("wasmtime").arg(&output).output().unwrap();
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    println!("{}", String::from_utf8_lossy(&run.stdout));
}

#[test]
fn compile_file_executes_async_generator_private_method_yield_star_sync_next_arrow_chain() {
    let tempdir = tempfile::tempdir().unwrap();
    let input = tempdir
        .path()
        .join("async-private-yield-star-sync-next-arrow.js");
    let output = tempdir
        .path()
        .join("async-private-yield-star-sync-next-arrow.wasm");

    fs::write(
        &input,
        r#"
        var log = [];
        var obj = {
          get [Symbol.iterator]() {
            log.push({ name: "get [Symbol.iterator]", thisValue: this });
            return function() {
              log.push({ name: "call [Symbol.iterator]", thisValue: this, args: [...arguments] });
              var nextCount = 0;
              return {
                name: "syncIterator",
                get next() {
                  log.push({ name: "get next", thisValue: this });
                  return function() {
                    log.push({ name: "call next", thisValue: this, args: [...arguments] });
                    nextCount++;
                    if (nextCount === 1) {
                      return {
                        name: "next-result-1",
                        get value() {
                          log.push({ name: "get next value (1)", thisValue: this });
                          return "next-value-1";
                        },
                        get done() {
                          log.push({ name: "get next done (1)", thisValue: this });
                          return false;
                        }
                      };
                    }
                    return {
                      name: "next-result-2",
                      get value() {
                        log.push({ name: "get next value (2)", thisValue: this });
                        return "next-value-2";
                      },
                      get done() {
                        log.push({ name: "get next done (2)", thisValue: this });
                        return true;
                      }
                    };
                  };
                }
              };
            };
          },
          get [Symbol.asyncIterator]() {
            log.push({ name: "get [Symbol.asyncIterator]" });
            return null;
          }
        };

        class C {
          async *#gen() {
            log.push({ name: "before yield*" });
            var v = yield* obj;
            log.push({ name: "after yield*", value: v });
            return "return-value";
          }
          get gen() { return this.#gen; }
        }

        var iter = new C().gen();
        iter.next("next-arg-1").then(v => {
          console.log(
            "next1",
            v.value,
            v.done,
            log.length,
            log[0] && log[0].name,
            log[1] && log[1].name,
            log[2] && log[2].name,
            log[2] && log[2].thisValue === obj,
            log[3] && log[3].name,
            log[3] && log[3].thisValue === obj,
            log[4] && log[4].name,
            log[4] && log[4].thisValue && log[4].thisValue.name,
            log[5] && log[5].name,
            log[5] && log[5].thisValue && log[5].thisValue.name,
            log[6] && log[6].name,
            log[6] && log[6].thisValue && log[6].thisValue.name,
            log[7] && log[7].name,
            log[7] && log[7].thisValue && log[7].thisValue.name
          );
          iter.next("next-arg-2").then(v2 => {
            console.log(
              "next2",
              v2.value,
              v2.done,
              log.length,
              log[8] && log[8].name,
              log[8] && log[8].thisValue && log[8].thisValue.name,
              log[9] && log[9].name,
              log[9] && log[9].thisValue && log[9].thisValue.name,
              log[10] && log[10].name,
              log[10] && log[10].thisValue && log[10].thisValue.name,
              log[11] && log[11].name,
              log[11] && log[11].value
            );
          });
        });
        "#,
    )
    .unwrap();

    let options = CompileOptions {
        output: output.clone(),
        target: "wasm32-wasip2".to_string(),
    };

    compile_file(&input, &options).unwrap();

    let run = Command::new("wasmtime").arg(&output).output().unwrap();
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    println!("{}", String::from_utf8_lossy(&run.stdout));
}

#[test]
fn compile_file_executes_public_class_field_define_property_on_proxy_receiver() {
    let tempdir = tempfile::tempdir().unwrap();
    let input = tempdir.path().join("class-field-define-property-proxy.js");
    let output = tempdir
        .path()
        .join("class-field-define-property-proxy.wasm");

    fs::write(
        &input,
        r#"
        let arr = [];
        let expectedTarget = null;
        function ProxyBase() {
          expectedTarget = this;
          return new Proxy(this, {
            defineProperty: function (target, key, descriptor) {
              arr.push(String(key));
              arr.push(String(descriptor.value));
              arr.push(String(target === expectedTarget));
              return Reflect.defineProperty(target, key, descriptor);
            }
          });
        }

        class Test extends ProxyBase {
          f = 3;
          g = "Test262";
        }

        let t = new Test();
        console.log(t.f, t.g, arr.join("|"));
        "#,
    )
    .unwrap();

    let options = CompileOptions {
        output: output.clone(),
        target: "wasm32-wasip2".to_string(),
    };

    compile_file(&input, &options).unwrap();

    let run = Command::new("wasmtime").arg(&output).output().unwrap();
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&run.stdout),
        "3 Test262 f|3|true|g|Test262|true\n"
    );
}

#[test]
fn compile_file_executes_constructor_read_of_initialized_public_field_into_global() {
    let tempdir = tempfile::tempdir().unwrap();
    let input = tempdir.path().join("constructor-field-read-into-global.js");
    let output = tempdir
        .path()
        .join("constructor-field-read-into-global.wasm");

    fs::write(
        &input,
        r#"
        var ctor;

        class C {
          constructor() {
            var tmp = this.foo;
            ctor = tmp;
          }

          foo = 42;
        }

        new C();
        console.log(ctor, ctor === 42);
        "#,
    )
    .unwrap();

    let options = CompileOptions {
        output: output.clone(),
        target: "wasm32-wasip2".to_string(),
    };

    compile_file(&input, &options).unwrap();

    let run = Command::new("wasmtime").arg(&output).output().unwrap();
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&run.stdout), "42 true\n");
}

#[test]
fn compile_file_throws_syntax_error_for_direct_eval_arguments_in_public_field_initializer() {
    let tempdir = tempfile::tempdir().unwrap();
    let input = tempdir.path().join("field-direct-eval-arguments.js");
    let output = tempdir.path().join("field-direct-eval-arguments.wasm");

    fs::write(
        &input,
        r#"
        var executed = false;
        var threw = false;

        class C {
          x = eval('executed = true; arguments;');
        }

        try {
          new C();
        } catch (error) {
          threw = error instanceof SyntaxError;
        }

        console.log(threw, executed);
        "#,
    )
    .unwrap();

    let options = CompileOptions {
        output: output.clone(),
        target: "wasm32-wasip2".to_string(),
    };

    compile_file(&input, &options).unwrap();

    let run = Command::new("wasmtime").arg(&output).output().unwrap();
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&run.stdout), "true false\n");
}

#[test]
fn compile_file_allows_direct_eval_new_target_in_public_field_initializer() {
    let tempdir = tempfile::tempdir().unwrap();
    let input = tempdir.path().join("field-direct-eval-new-target.js");
    let output = tempdir.path().join("field-direct-eval-new-target.wasm");

    fs::write(
        &input,
        r#"
        var executed = false;

        class C {
          x = eval('executed = true; new.target;');
        }

        var c = new C();
        console.log(executed, c.x === undefined);
        "#,
    )
    .unwrap();

    let options = CompileOptions {
        output: output.clone(),
        target: "wasm32-wasip2".to_string(),
    };

    compile_file(&input, &options).unwrap();

    let run = Command::new("wasmtime").arg(&output).output().unwrap();
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&run.stdout), "true true\n");
}

#[test]
fn compile_file_executes_symbol_has_own_checks_after_dead_for_in_branch() {
    let tempdir = tempfile::tempdir().unwrap();
    let input = tempdir.path().join("symbol-has-own-after-dead-for-in.js");
    let output = tempdir.path().join("symbol-has-own-after-dead-for-in.wasm");

    fs::write(
        &input,
        r#"
        var __hasOwnProperty = Function.prototype.call.bind(Object.prototype.hasOwnProperty);
        var __propertyIsEnumerable =
          Function.prototype.call.bind(Object.prototype.propertyIsEnumerable);
        var s = Symbol();

        class C {
          [s] = 42;
        }

        function probe(obj, name) {
          if (typeof name === "string") {
            for (var x in obj) {
              if (x === name) {
                break;
              }
            }
          }

          console.log(__hasOwnProperty(obj, name), __propertyIsEnumerable(obj, name));
        }

        probe(new C(), s);
        "#,
    )
    .unwrap();

    let options = CompileOptions {
        output: output.clone(),
        target: "wasm32-wasip2".to_string(),
    };

    compile_file(&input, &options).unwrap();

    let run = Command::new("wasmtime").arg(&output).output().unwrap();
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&run.stdout), "true true\n");
}

#[test]
fn compile_file_executes_bound_symbol_has_own_after_delete() {
    let tempdir = tempfile::tempdir().unwrap();
    let input = tempdir.path().join("bound-symbol-has-own-after-delete.js");
    let output = tempdir
        .path()
        .join("bound-symbol-has-own-after-delete.wasm");

    fs::write(
        &input,
        r#"
        var __hasOwnProperty = Function.prototype.call.bind(Object.prototype.hasOwnProperty);
        var s = Symbol();

        class C {
          [s] = 42;
        }

        function probe(obj, name) {
          console.log(__hasOwnProperty(obj, name));
          delete obj[name];
          console.log(__hasOwnProperty(obj, name));
        }

        probe(new C(), s);
        "#,
    )
    .unwrap();

    let options = CompileOptions {
        output: output.clone(),
        target: "wasm32-wasip2".to_string(),
    };

    compile_file(&input, &options).unwrap();

    let run = Command::new("wasmtime").arg(&output).output().unwrap();
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&run.stdout), "true\nfalse\n");
}

#[test]
fn compile_file_executes_symbol_has_own_after_descriptor_lookup_and_stringify() {
    let tempdir = tempfile::tempdir().unwrap();
    let input = tempdir
        .path()
        .join("symbol-has-own-after-descriptor-and-stringify.js");
    let output = tempdir
        .path()
        .join("symbol-has-own-after-descriptor-and-stringify.wasm");

    fs::write(
        &input,
        r#"
        var __getOwnPropertyDescriptor = Object.getOwnPropertyDescriptor;
        var __hasOwnProperty = Function.prototype.call.bind(Object.prototype.hasOwnProperty);
        var s = Symbol("field");

        class C {
          [s] = 42;
        }

        function probe(obj, name) {
          var originalDesc = __getOwnPropertyDescriptor(obj, name);
          var nameStr = String(name);
          console.log(typeof originalDesc, typeof originalDesc.value, nameStr);
          console.log(Object.prototype.hasOwnProperty.call(obj, name));
          console.log(__hasOwnProperty(obj, name));
        }

        probe(new C(), s);
        "#,
    )
    .unwrap();

    let options = CompileOptions {
        output: output.clone(),
        target: "wasm32-wasip2".to_string(),
    };

    compile_file(&input, &options).unwrap();

    let run = Command::new("wasmtime").arg(&output).output().unwrap();
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&run.stdout),
        "object number Symbol(field)\ntrue\ntrue\n"
    );
}

#[test]
fn compile_file_executes_dynamic_index_reads_from_get_own_property_names() {
    let tempdir = tempfile::tempdir().unwrap();
    let input = tempdir.path().join("dynamic-own-property-names.js");
    let output = tempdir.path().join("dynamic-own-property-names.wasm");

    fs::write(
        &input,
        r#"
        var __getOwnPropertyNames = Object.getOwnPropertyNames;
        var desc = {
          value: 42,
          enumerable: true,
          writable: true,
          configurable: true
        };
        var expected = ["value", "enumerable", "writable", "configurable"];
        var names = __getOwnPropertyNames(desc);
        for (var i = 0; i < names.length; i++) {
          console.log(names[i] === expected[i]);
        }
        "#,
    )
    .unwrap();

    let options = CompileOptions {
        output: output.clone(),
        target: "wasm32-wasip2".to_string(),
    };

    compile_file(&input, &options).unwrap();

    let run = Command::new("wasmtime").arg(&output).output().unwrap();
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&run.stdout),
        "true\ntrue\ntrue\ntrue\n"
    );
}

#[test]
fn compile_file_executes_incremental_public_class_field_initializers() {
    let tempdir = tempfile::tempdir().unwrap();
    let input = tempdir.path().join("incremental-public-class-fields.js");
    let output = tempdir.path().join("incremental-public-class-fields.wasm");

    fs::write(
        &input,
        r#"
        var x = 1;

        class C {
          [x++] = x++;
          [x++] = x++;
        }

        var c1 = new C();
        var c2 = new C();
        console.log(c1[1], c1[2], c2[1], c2[2], x);
        "#,
    )
    .unwrap();

    let options = CompileOptions {
        output: output.clone(),
        target: "wasm32-wasip2".to_string(),
    };

    compile_file(&input, &options).unwrap();

    let run = Command::new("wasmtime").arg(&output).output().unwrap();
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&run.stdout), "3 4 5 6 7\n");
}

#[test]
fn compile_file_executes_intercalated_static_and_instance_computed_fields_in_order() {
    let tempdir = tempfile::tempdir().unwrap();
    let input = tempdir
        .path()
        .join("intercalated-static-instance-computed-fields.js");
    let output = tempdir
        .path()
        .join("intercalated-static-instance-computed-fields.wasm");

    fs::write(
        &input,
        r#"
        let i = 0;

        class C {
          [i++] = i++;
          static [i++] = i++;
          [i++] = i++;
        }

        let c = new C();
        console.log(c[0], c[2], C[1], i);
        "#,
    )
    .unwrap();

    let options = CompileOptions {
        output: output.clone(),
        target: "wasm32-wasip2".to_string(),
    };

    compile_file(&input, &options).unwrap();

    let run = Command::new("wasmtime").arg(&output).output().unwrap();
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&run.stdout), "4 5 3 6\n");
}

#[test]
fn compile_file_executes_symbol_assignment_to_computed_class_field() {
    let tempdir = tempfile::tempdir().unwrap();
    let input = tempdir
        .path()
        .join("symbol-assignment-to-computed-class-field.js");
    let output = tempdir
        .path()
        .join("symbol-assignment-to-computed-class-field.wasm");

    fs::write(
        &input,
        r#"
        var s = Symbol();

        class C {
          [s] = 42;
        }

        var c = new C();
        c[s] = "unlikelyValue";
        console.log(String(c[s]));
        "#,
    )
    .unwrap();

    let options = CompileOptions {
        output: output.clone(),
        target: "wasm32-wasip2".to_string(),
    };

    compile_file(&input, &options).unwrap();

    let run = Command::new("wasmtime").arg(&output).output().unwrap();
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&run.stdout), "unlikelyValue\n");
}

#[test]
fn compile_file_keeps_computed_constructor_class_field_on_instance_only() {
    let tempdir = tempfile::tempdir().unwrap();
    let input = tempdir
        .path()
        .join("computed-constructor-class-field-instance-only.js");
    let output = tempdir
        .path()
        .join("computed-constructor-class-field-instance-only.wasm");

    fs::write(
        &input,
        r#"
        var x = "constructor";

        class C1 {
          [x];
        }

        var c1 = new C1();
        console.log(String(c1.hasOwnProperty("constructor")));
        console.log(String(C1.hasOwnProperty("constructor")));
        "#,
    )
    .unwrap();

    let options = CompileOptions {
        output: output.clone(),
        target: "wasm32-wasip2".to_string(),
    };

    compile_file(&input, &options).unwrap();

    let run = Command::new("wasmtime").arg(&output).output().unwrap();
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&run.stdout), "true\nfalse\n");
}

#[test]
fn compile_file_executes_for_of_over_custom_iterator_breaks_and_closes() {
    let tempdir = tempfile::tempdir().unwrap();
    let input = tempdir.path().join("for-of-custom-iterator-break-close.js");
    let output = tempdir
        .path()
        .join("for-of-custom-iterator-break-close.wasm");

    fs::write(
        &input,
        r#"
        let closed = 0;

        function makeIterable(values) {
          let index = 0;
          const iterator = {
            next: function () {
              if (index >= values.length) {
                return { done: true };
              }
              return { value: values[index++], done: false };
            },
            return: function () {
              closed = closed + 1;
              return { done: true };
            },
          };

          iterator[Symbol.iterator] = function () {
            return iterator;
          };

          return iterator;
        }

        let count = 0;
        for (const value of makeIterable([4, 5])) {
          count = count + 1;
          if (value === 5) {
            break;
          }
        }

        console.log(count, closed);
        "#,
    )
    .unwrap();

    let options = CompileOptions {
        output: output.clone(),
        target: "wasm32-wasip2".to_string(),
    };

    compile_file(&input, &options).unwrap();

    let run = Command::new("wasmtime").arg(&output).output().unwrap();
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&run.stdout), "2 1\n");
}

#[test]
fn compile_file_executes_nested_for_of_labeled_break_outer_loop_via_direct_wasm_backend() {
    let tempdir = tempfile::tempdir().unwrap();
    let input = tempdir.path().join("nested-for-of-labeled-break.js");
    let output = tempdir.path().join("nested-for-of-labeled-break.wasm");

    fs::write(
        &input,
        r#"
        let sum = 0;
        outer: for (const outer_value of [1, 2, 3]) {
          for (const inner_value of [4, 5, 6]) {
            if (inner_value === 5) {
              break outer;
            }
            sum = sum + inner_value;
          }
        }
        console.log(sum);
        "#,
    )
    .unwrap();

    let options = CompileOptions {
        output: output.clone(),
        target: "wasm32-wasip2".to_string(),
    };

    compile_file(&input, &options).unwrap();

    let run = Command::new("wasmtime").arg(&output).output().unwrap();
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&run.stdout), "4\n");
}

#[test]
fn compile_file_executes_labeled_for_of_current_loop_continue_closes_iterator() {
    let tempdir = tempfile::tempdir().unwrap();
    let input = tempdir.path().join("for-of-continue-current-loop.js");
    let output = tempdir.path().join("for-of-continue-current-loop.wasm");

    fs::write(
        &input,
        r#"
        let sum = 0;
        let closed = false;
        const iterator = [1, 2, 3][Symbol.iterator]();

        iterator.return = function () {
          closed = true;
          return { done: true };
        };

        outer: for (const value of iterator) {
          if (value === 2) {
            continue outer;
          }
          sum = sum + value;
        }

        console.log(sum, closed ? 1 : 0);
        "#,
    )
    .unwrap();

    let options = CompileOptions {
        output: output.clone(),
        target: "wasm32-wasip2".to_string(),
    };

    compile_file(&input, &options).unwrap();

    let run = Command::new("wasmtime").arg(&output).output().unwrap();
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&run.stdout), "4 1\n");
}

#[test]
fn compile_file_closes_throwing_iterator_return_getter_without_dynamic_call_fanout() {
    let tempdir = tempfile::tempdir().unwrap();
    let input = tempdir.path().join("destructuring-close-return-getter.js");
    let output = tempdir
        .path()
        .join("destructuring-close-return-getter.wasm");

    fs::write(
        &input,
        r#"
        function Test262Error(message) {
          this.name = "Test262Error";
          this.message = message ?? "";
        }

        function __sameValue(left, right) {
          if (left === right) {
            return left !== 0 || 1 / left === 1 / right;
          }
          return left !== left && right !== right;
        }

        function __assertToString(value) {
          return "" + value;
        }

        function __assertSameValue(actual, expected, message) {
          if (__sameValue(actual, expected)) {
            return;
          }
          message = "Expected " + __assertToString(actual) + " to match " + __assertToString(expected);
          throw new Test262Error(message);
        }

        function __assertNotSameValue(actual, expected, message) {
          if (!__sameValue(actual, expected)) {
            return;
          }
          throw new Test262Error("Expected values to differ");
        }

        function assert(mustBeTrue, message) {
          if (mustBeTrue === true) {
            return;
          }
          throw new Test262Error(message);
        }

        function __ayyAssertThrows(expectedErrorConstructor, func, message) {
          try {
            func();
          } catch (thrown) {
            return;
          }
          throw new Test262Error("Expected " + expectedErrorConstructor.name + " to be thrown");
        }

        function compareArray(actual, expected) {
          if (actual.length !== expected.length) {
            return false;
          }
          for (var i = 0; i < actual.length; i += 1) {
            if (!__sameValue(actual[i], expected[i])) {
              return false;
            }
          }
          return true;
        }

        globalThis.assert = assert;
        assert._isSameValue = __sameValue;
        assert._toString = __assertToString;
        assert.sameValue = __assertSameValue;
        assert.notSameValue = __assertNotSameValue;
        assert.throws = __ayyAssertThrows;
        assert.compareArray = compareArray;

        function MyError() {}

        function thrower() {
          throw new MyError();
        }

        var returnGetterCalled = 0;

        var iterator = {
          [Symbol.iterator]() {
            return this;
          },
          next() {
            return {done: false};
          },
          get return() {
            returnGetterCalled += 1;
            throw "bad";
          }
        };

        __ayyAssertThrows(MyError, function() {
          var a;
          ([a = thrower()] = iterator);
        });

        for (var returnMethod of [0, 0n, true, "string", {}, Symbol()]) {
          var iterable = {
            [Symbol.iterator]() {
              return this;
            },
            next() {
              return {done: false};
            },
            return: returnMethod,
          };

          __ayyAssertThrows(MyError, function() {
            var a;
            ([a = thrower()] = iterable);
          });
        }

        console.log(eval("returnGetterCalled"));
        console.log("done");
        "#,
    )
    .unwrap();

    let options = CompileOptions {
        output: output.clone(),
        target: "wasm32-wasip2".to_string(),
    };

    compile_file(&input, &options).unwrap();
    assert!(fs::metadata(&output).unwrap().len() < 200_000);

    let run = Command::new("wasmtime").arg(&output).output().unwrap();
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&run.stdout), "1\ndone\n");
}

#[test]
fn compiles_module_goal_files_with_real_paths() {
    let tempdir = tempfile::tempdir().unwrap();
    let input = tempdir.path().join("module.js");
    let output = tempdir.path().join("module.wasm");
    fs::write(
        &input,
        r#"
        export const answer = 42;
        console.log(answer);
        "#,
    )
    .unwrap();
    let options = CompileOptions {
        output: output.clone(),
        target: "wasm32-wasip2".to_string(),
    };

    compile_file_with_goal(&input, &options, true).unwrap();

    let run = Command::new("wasmtime").arg(&output).output().unwrap();
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&run.stdout), "42\n");
}

#[test]
fn rejects_named_and_namespace_module_imports() {
    let tempdir = tempfile::tempdir().unwrap();
    let dep = tempdir.path().join("dep.js");
    let reexport = tempdir.path().join("reexport.js");
    let entry = tempdir.path().join("entry.js");
    let output = tempdir.path().join("entry.wasm");

    fs::write(
        &dep,
        r#"
        export const value = 7;
        export default function named() { return value; }
        "#,
    )
    .unwrap();
    fs::write(
        &reexport,
        r#"
        export * from "./dep.js";
        "#,
    )
    .unwrap();
    fs::write(
        &entry,
        r#"
        import named, { value } from "./dep.js";
        import * as ns from "./reexport.js";
        console.log(named(), value, ns.value, ns[Symbol.toStringTag]);
        "#,
    )
    .unwrap();

    let options = CompileOptions {
        output,
        target: "wasm32-wasip2".to_string(),
    };

    compile_file_with_goal(&entry, &options, true)
        .expect_err("module imports are not yet supported by direct wasm backend");
}
