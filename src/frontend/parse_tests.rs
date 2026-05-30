use super::parse;
use crate::{
    frontend,
    ir::hir::{BinaryOp, Expression, Statement, UnaryOp, UpdateOp},
};

#[test]
fn parses_asi_prefix_increment_as_separate_expression_statements() {
    let program = frontend::parse(
        r#"
        var x = 0;
        var y = 0;
        x
        ++y
        "#,
    )
    .unwrap();

    assert!(
        matches!(
            program.statements.as_slice(),
            [
                Statement::Var {
                    name: first_name,
                    value: Expression::Number(first_value),
                },
                Statement::Var {
                    name: second_name,
                    value: Expression::Number(second_value),
                },
                Statement::Expression(Expression::Identifier(name)),
                Statement::Expression(Expression::Update {
                    name: update_name,
                    op: UpdateOp::Increment,
                    prefix: true,
                }),
            ] if first_name == "x"
                && *first_value == 0.0
                && second_name == "y"
                && *second_value == 0.0
                && name == "x"
                && update_name == "y"
        ),
        "{:#?}",
        program.statements
    );
}

#[test]
fn parses_asi_prefix_decrement_as_separate_expression_statements() {
    let program = frontend::parse(
        r#"
        var x = 1;
        var y = 1;
        x
        --y
        "#,
    )
    .unwrap();

    assert!(
        matches!(
            program.statements.as_slice(),
            [
                Statement::Var {
                    name: first_name,
                    value: Expression::Number(first_value),
                },
                Statement::Var {
                    name: second_name,
                    value: Expression::Number(second_value),
                },
                Statement::Expression(Expression::Identifier(name)),
                Statement::Expression(Expression::Update {
                    name: update_name,
                    op: UpdateOp::Decrement,
                    prefix: true,
                }),
            ] if first_name == "x"
                && *first_value == 1.0
                && second_name == "y"
                && *second_value == 1.0
                && name == "x"
                && update_name == "y"
        ),
        "{:#?}",
        program.statements
    );
}

#[test]
fn rejects_classic_for_headers_with_only_one_semicolon() {
    let invalid_sources = [
        "for(false;false\n) { break; }",
        "for(false;\nfalse\n) { break; }",
        "for(false\n    ;\n) { break; }",
        "for(false\n    ;false\n) { break; }",
        "for(\n;false) { break; }",
    ];

    for source in invalid_sources {
        assert!(
            frontend::validate_script_goal(source).is_err(),
            "source should fail to parse:\n{source}"
        );
    }
}

#[test]
fn accepts_classic_for_headers_with_two_semicolons_across_newlines() {
    let source = r#"
    for(false
        ;false
        ;
    ) {
      break;
    }
    "#;

    frontend::validate_script_goal(source).expect("source should parse");
}

#[test]
fn parse_module_goal_rejects_named_export_without_statement_boundary() {
    let source = "export {} null;";

    assert!(
        frontend::parse_module_goal(source).is_err(),
        "source should fail to parse:\n{source}"
    );
}

#[test]
fn parse_module_goal_accepts_named_export_with_asi_boundary() {
    for source in ["export {}\nnull;", "export {}; null;"] {
        frontend::parse_module_goal(source).expect("source should parse");
    }
}

#[test]
fn parses_top_level_global_this_update_as_binding_update() {
    let program = frontend::parse(
        r#"
        var y;
        this.y++;
        "#,
    )
    .unwrap();

    assert!(
        matches!(
            program.statements.as_slice(),
            [
                Statement::Var { name, value },
                Statement::Expression(Expression::AssignMember {
                    object,
                    property,
                    value: assign_value,
                }),
            ] if name == "y"
                && matches!(value, Expression::Undefined)
                && matches!(object.as_ref(), Expression::This)
                && matches!(property.as_ref(), Expression::String(property_name) if property_name == "y")
                && matches!(
                    assign_value.as_ref(),
                    Expression::Binary {
                        op: BinaryOp::Add,
                        left,
                        right,
                    } if matches!(
                        left.as_ref(),
                        Expression::Unary {
                            op: UnaryOp::Plus,
                            expression,
                        } if matches!(
                            expression.as_ref(),
                            Expression::Member {
                                object: member_object,
                                property: member_property,
                            } if matches!(member_object.as_ref(), Expression::This)
                                && matches!(member_property.as_ref(), Expression::String(member_property_name) if member_property_name == "y")
                        )
                    ) && matches!(right.as_ref(), Expression::Number(1.0))
                )
        ),
        "{:#?}",
        program.statements
    );
}

#[test]
fn parses_hashbang_comments_terminated_by_carriage_return() {
    parse("#! comment\r{}\n").expect("carriage-return-terminated hashbang should parse");
}

#[test]
fn parses_hashbang_comments_terminated_by_line_separator() {
    parse("#! comment\u{2028}{}\n").expect("line-separator-terminated hashbang should parse");
}

#[test]
fn parses_hashbang_comments_terminated_by_paragraph_separator() {
    parse("#! comment\u{2029}{}\n").expect("paragraph-separator-terminated hashbang should parse");
}

#[test]
fn rejects_invalid_numeric_separator_placements() {
    let invalid_sources = [
        "0b_1", "0x_FF", "1__0", "1_.0", "1._0", "1e_1", "1e+_1", "0_1", "0_1.5",
    ];

    for source in invalid_sources {
        assert!(
            frontend::validate_script_goal(source).is_err(),
            "source should fail to parse:\n{source}"
        );
    }
}

#[test]
fn accepts_valid_numeric_separator_placements() {
    let valid_sources = ["0b1_0", "0xA_B", "1_0", "1_0.5_0", "1.0_5e+1_0"];

    for source in valid_sources {
        frontend::validate_script_goal(source).expect("source should parse");
    }
}

#[test]
fn rejects_escaped_reserved_words_in_binding_identifiers() {
    let invalid_sources = [
        "var \\u{65}lse = 123;",
        "var \\u0065lse = 123;",
        "var \\u{64}elete = 123;",
        "var \\u0064elete = 123;",
        "var \\u{65}\\u{6e}\\u{75}\\u{6d} = 123;",
        "var \\u0065\\u006e\\u0075\\u006d = 123;",
    ];

    for source in invalid_sources {
        assert!(
            frontend::validate_script_goal(source).is_err(),
            "source should fail to parse:\n{source}"
        );
    }
}

#[test]
fn rejects_reserved_object_pattern_shorthand_bindings() {
    let invalid_sources = [
        "var x = ({ default }) => {};",
        "var x = ({ if }) => {};",
        "var x = ({ default = 1 }) => {};",
    ];

    for source in invalid_sources {
        assert!(
            frontend::validate_script_goal(source).is_err(),
            "source should fail to parse:\n{source}"
        );
    }
}

#[test]
fn rejects_strict_reserved_object_pattern_shorthand_bindings() {
    let invalid_sources = [
        "\"use strict\"; var x = ({ implements }) => {};",
        "\"use strict\"; var x = ({ \\u0069mplements }) => {};",
        "\"use strict\"; var x = ({ package = 1 }) => {};",
        "\"use strict\"; var x = ({ static }) => {};",
    ];

    for source in invalid_sources {
        assert!(
            frontend::validate_script_goal(source).is_err(),
            "source should fail to parse:\n{source}"
        );
    }
}

#[test]
fn parse_script_goal_rejects_escaped_await_binding_in_async_generator_method() {
    let source = r#"
    class C { async *gen() {
        var \u0061wait;
    }}
    "#;

    assert!(
        frontend::parse_script_goal(source).is_err(),
        "source should fail to parse:\n{source}"
    );
}

#[test]
fn parse_script_goal_rejects_await_binding_in_async_generator_method() {
    let source = r#"
    class C { async *gen() {
        var await;
    }}
    "#;

    assert!(
        frontend::parse_script_goal(source).is_err(),
        "source should fail to parse:\n{source}"
    );
}

#[test]
fn parse_script_goal_rejects_await_identifier_reference_in_async_generator_method() {
    let source = r#"
    class C { async *gen() {
        await;
    }}
    "#;

    assert!(
        frontend::parse_script_goal(source).is_err(),
        "source should fail to parse:\n{source}"
    );
}

#[test]
fn parse_script_goal_rejects_await_label_in_async_generator_method() {
    let source = r#"
    class C { async *gen() {
        await: 1;
    }}
    "#;

    assert!(
        frontend::parse_script_goal(source).is_err(),
        "source should fail to parse:\n{source}"
    );
}

#[test]
fn parse_script_goal_rejects_yield_assignment_pattern_shorthand_in_generator() {
    let source = r#"
    (function*() {
        0, { yield } = {};
    });
    "#;

    assert!(
        frontend::parse_script_goal(source).is_err(),
        "source should fail to parse:\n{source}"
    );
}

#[test]
fn validate_script_goal_rejects_yield_assignment_pattern_shorthand_in_strict_mode() {
    let source = r#"
    "use strict";
    0, { yield } = {};
    "#;

    assert!(
        frontend::validate_script_goal(source).is_err(),
        "source should fail to parse:\n{source}"
    );
}

#[test]
fn parse_script_goal_accepts_yield_assignment_pattern_shorthand_outside_strict_and_generator() {
    let source = r#"
    var yield;
    0, { yield } = { yield: 3 };
    "#;

    frontend::parse_script_goal(source).expect("source should parse");
}

#[test]
fn parse_script_goal_rejects_escaped_reserved_assignment_pattern_shorthand() {
    let invalid_sources = [
        r#"0, { \u0063onst } = { const: 1 };"#,
        r#"0, { \u0063ontinue } = { continue: 1 };"#,
        r#"0, { \u0064ebugger } = { debugger: 1 };"#,
    ];

    for source in invalid_sources {
        assert!(
            frontend::parse_script_goal(source).is_err(),
            "source should fail to parse:\n{source}"
        );
    }
}

#[test]
fn parse_script_goal_rejects_await_arrow_binding_in_static_block() {
    let source = r#"
    class C {
        static {
        (await => 0);
      }
    }
    "#;

    assert!(
        frontend::parse_script_goal(source).is_err(),
        "source should fail to parse:\n{source}"
    );
}

#[test]
fn parse_script_goal_rejects_await_arrow_parameter_default_in_static_block() {
    let source = r#"
    class C {
      static {
        ((x = await) => 0);
      }
    }
    "#;

    assert!(
        frontend::parse_script_goal(source).is_err(),
        "source should fail to parse:\n{source}"
    );
}

#[test]
fn parse_script_goal_rejects_await_object_shorthand_in_static_block() {
    let source = r#"
    class C {
      static {
        ({ await });
      }
    }
    "#;

    assert!(
        frontend::parse_script_goal(source).is_err(),
        "source should fail to parse:\n{source}"
    );
}

#[test]
fn parse_script_goal_rejects_await_arrow_parameter_default_in_async_arrow_body() {
    let source = r#"
    async() => { (a = await/r/g) => {} };
    "#;

    assert!(
        frontend::parse_script_goal(source).is_err(),
        "source should fail to parse:\n{source}"
    );
}

#[test]
fn parse_script_goal_rejects_arrow_parameter_body_lexical_duplicate() {
    let source = r#"
    async(bar) => { let bar; }
    "#;

    assert!(
        frontend::parse_script_goal(source).is_err(),
        "source should fail to parse:\n{source}"
    );
}

#[test]
fn parse_script_goal_rejects_function_parameter_body_lexical_duplicate() {
    let source = r#"
    (async function foo(bar) { let bar; });
    "#;

    assert!(
        frontend::parse_script_goal(source).is_err(),
        "source should fail to parse:\n{source}"
    );
}

#[test]
fn parse_script_goal_accepts_decorator_member_private_identifier_in_static_block() {
    let source = r#"
    class C {
      static #yield() {}
      static #await() {}
      static {
        @C.#yield
        @C.#await
        class D {}
      }
    }
    "#;

    frontend::parse_script_goal(source).expect("source should parse");
}

#[test]
fn parse_script_goal_accepts_class_expression_decorator_member_expression() {
    let source = r#"
    var ns;
    var C = @ns.$
    @ns._
    @ns.\u{6F}
    @ns.\u2118
    @ns.ZW_\u200C_NJ
    @ns.ZW_\u200D_J
    @ns.yield
    @ns.await class {};
    "#;

    frontend::parse_script_goal(source).expect("source should parse");
}

#[test]
fn parse_script_goal_accepts_nested_yield_spread_in_async_generator_method() {
    let valid_sources = [
        r#"
        class C { async *gen() {
            yield [...yield yield];
        }}
        "#,
        r#"
        class C { async *gen() {
            yield [...yield];
        }}
        "#,
        r#"
        class C { async *gen() {
            yield {...yield};
        }}
        "#,
    ];

    for source in valid_sources {
        assert!(
            frontend::parse_script_goal(source).is_ok(),
            "source should parse:\n{source}"
        );
    }
}

#[test]
fn accepts_escaped_non_reserved_binding_identifiers() {
    let valid_sources = [
        "var \\u{65}lsewhere = 123;",
        "var $\\u200D = 2;",
        "var $\\u200C = 3;",
    ];

    for source in valid_sources {
        frontend::validate_script_goal(source)
            .expect("non-reserved escaped identifier should parse");
    }
}

#[test]
fn parse_script_goal_accepts_escaped_let_expression_statement_with_asi() {
    let source = r#"
    this.let = 0;
    l\u0065t // ASI
    a;
    var a;
    "#;

    frontend::parse_script_goal(source)
        .expect("escaped let at statement start should not parse as a lexical declaration");
}

#[test]
fn parse_script_goal_accepts_escaped_await_class_name_identifier() {
    let source = r#"
    class aw\u0061it {}
    "#;

    frontend::parse_script_goal(source)
        .expect("escaped await class name should parse in script goal");
}

#[test]
fn parse_script_goal_accepts_escaped_reserved_class_method_name() {
    let source = r#"
    class C {
        bre\u0061k() { return 42; }
    }
    "#;

    frontend::parse_script_goal(source).expect("escaped reserved class method name should parse");
}

#[test]
fn parse_script_goal_accepts_escaped_reserved_object_assignment_property_name() {
    let source = r#"
    var y = { bre\u0061k: x } = { break: 42 };
    "#;

    frontend::parse_script_goal(source)
        .expect("escaped reserved assignment property name should parse");
}

#[test]
fn parse_script_goal_accepts_escaped_reserved_object_method_name() {
    let source = r#"
    var obj = {
      bre\u0061k() { return 42; }
    };
    "#;

    frontend::parse_script_goal(source).expect("escaped reserved object method name should parse");
}

#[test]
fn parse_script_goal_accepts_bigint_literal_property_names() {
    let source = r#"
    let o = { 999999999999999999n: true, 1n() { return 42; } };
    class C { 1n() { return 42; } }
    let { 1n: value } = { "1": 42 };
    "#;

    frontend::parse_script_goal(source).expect("BigInt property names should lower");
}

#[test]
fn parse_script_goal_accepts_escaped_reserved_member_property_name() {
    let source = r#"
    var obj = {};
    obj.bre\u0061k = 42;
    obj?.def\u0061ult;
    "#;

    frontend::parse_script_goal(source)
        .expect("escaped reserved member property name should parse");
}

#[test]
fn parse_script_goal_rejects_duplicate_parameters_in_async_class_method() {
    let source = r#"
    class Foo {
      async foo(a, a) {}
    }
    "#;

    assert!(
        frontend::validate_script_goal(source).is_err(),
        "duplicate parameters in async class methods should be rejected"
    );
}

#[test]
fn parse_script_goal_rejects_duplicate_parameters_in_object_methods() {
    for source in ["({ foo(a, a) { } });", "({ async foo(a, a) { } });"] {
        assert!(
            frontend::validate_script_goal(source).is_err(),
            "duplicate parameters in object methods should be rejected"
        );
    }
}

#[test]
fn parse_script_goal_rejects_line_terminator_in_async_object_method_head() {
    let source = "({ async\nfoo() { } });";

    assert!(
        frontend::validate_script_goal(source).is_err(),
        "line terminators between async and object method names should be rejected"
    );
}

#[test]
fn parse_script_goal_rejects_escaped_object_accessor_keywords() {
    for source in [
        r"({ g\u0065t m() {} });",
        r"({ \u0067et m() {} });",
        r"({ ge\u0074 m() {} });",
        r"({ \u0067\u0065\u0074 m() {} });",
        r"({ s\u0065t m(v) {} });",
        r"({ \u0073et m(v) {} });",
        r"({ se\u0074 m(v) {} });",
        r"({ \u0073\u0065\u0074 m(v) {} });",
    ] {
        assert!(
            frontend::validate_script_goal(source).is_err(),
            "escaped object accessor keywords should be rejected"
        );
    }
}

#[test]
fn parse_script_goal_rejects_duplicate_arrow_parameters() {
    let source = "0, (a, a) => { };";

    assert!(
        frontend::validate_script_goal(source).is_err(),
        "duplicate arrow parameters should be rejected"
    );
}

#[test]
fn parse_script_goal_rejects_new_import_phase_calls() {
    for source in [
        "let f = () => new import('./empty_FIXTURE.js');",
        "let f = () => new import.defer('./empty_FIXTURE.js');",
        "let f = () => new import.source('./empty_FIXTURE.js');",
    ] {
        assert!(
            frontend::validate_script_goal(source).is_err(),
            "dynamic import constructor form should be rejected:\n{source}"
        );
    }
}

#[test]
fn rejects_invalid_escaped_identifier_starts_and_code_points() {
    let invalid_sources = ["var \\u200D;", "var \\u200C;", "var \\u{00_76} = 1;"];

    for source in invalid_sources {
        assert!(
            frontend::validate_script_goal(source).is_err(),
            "source should fail to parse:\n{source}"
        );
    }
}
