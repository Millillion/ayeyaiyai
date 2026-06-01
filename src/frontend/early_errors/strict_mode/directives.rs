use super::super::*;

pub(super) fn is_strict_mode_restricted_identifier(name: &str) -> bool {
    matches!(name, "eval" | "arguments")
}

pub(super) fn is_strict_mode_reserved_identifier(name: &str) -> bool {
    is_strict_mode_reserved_identifier_text(name)
        || decode_identifier_escapes(name)
            .as_deref()
            .is_some_and(is_strict_mode_reserved_identifier_text)
}

fn is_strict_mode_reserved_identifier_text(name: &str) -> bool {
    matches!(
        name,
        "implements"
            | "interface"
            | "let"
            | "package"
            | "private"
            | "protected"
            | "public"
            | "static"
            | "yield"
    )
}

fn decode_identifier_escapes(name: &str) -> Option<String> {
    if !name.contains('\\') {
        return None;
    }

    let mut decoded = String::new();
    let mut characters = name.chars();
    while let Some(character) = characters.next() {
        if character != '\\' {
            decoded.push(character);
            continue;
        }

        if characters.next() != Some('u') {
            return None;
        }

        let code_point = if matches!(characters.clone().next(), Some('{')) {
            characters.next();
            let mut digits = String::new();
            let mut closed = false;
            for character in characters.by_ref() {
                if character == '}' {
                    closed = true;
                    break;
                }
                if !character.is_ascii_hexdigit() {
                    return None;
                }
                digits.push(character);
            }
            if !closed || digits.is_empty() {
                return None;
            }
            u32::from_str_radix(&digits, 16).ok()?
        } else {
            let mut digits = String::new();
            for _ in 0..4 {
                let character = characters.next()?;
                if !character.is_ascii_hexdigit() {
                    return None;
                }
                digits.push(character);
            }
            u32::from_str_radix(&digits, 16).ok()?
        };
        decoded.push(char::from_u32(code_point)?);
    }

    Some(decoded)
}

pub(crate) fn script_has_use_strict_directive(statements: &[Stmt]) -> bool {
    for statement in statements {
        let Stmt::Expr(ExprStmt { expr, .. }) = statement else {
            break;
        };

        let Expr::Lit(Lit::Str(string)) = &**expr else {
            break;
        };

        if is_unescaped_use_strict_directive(string) {
            return true;
        }
    }

    false
}

pub(crate) fn function_has_use_strict_directive(function: &Function) -> bool {
    let Some(body) = &function.body else {
        return false;
    };

    for statement in &body.stmts {
        let Stmt::Expr(ExprStmt { expr, .. }) = statement else {
            break;
        };

        let Expr::Lit(Lit::Str(string)) = &**expr else {
            break;
        };

        if is_unescaped_use_strict_directive(string) {
            return true;
        }
    }

    false
}

fn is_unescaped_use_strict_directive(string: &swc_ecma_ast::Str) -> bool {
    if string.value.as_str() != Some("use strict") {
        return false;
    }

    matches!(
        string.raw.as_ref().map(|raw| raw.as_str()),
        Some("\"use strict\"" | "'use strict'")
    )
}
