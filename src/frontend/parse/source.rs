use std::{borrow::Cow, fs, path::Path};

use anyhow::{Context, Result, bail};
use swc_common::{FileName, SourceMap, sync::Lrc};
use swc_ecma_ast::{Decl, Module, Program as SwcProgram, Stmt};
use swc_ecma_parser::{EsSyntax, Parser, StringInput, Syntax, lexer::Lexer};

use super::{await_rewrite::rewrite_script_await_identifiers, validation::*};

pub(super) fn parse_program_source(source: &str) -> Result<SwcProgram> {
    let file = source_file(FileName::Custom("input.js".into()), source);

    parse_script(&file).or_else(|script_error| {
        parse_module(&file).map_err(|module_error| {
            anyhow::anyhow!(
                "failed to parse JavaScript source as script: {script_error:#}\nfailed to parse JavaScript source as module: {module_error:#}"
            )
        })
    })
}

pub(super) fn parse_script_program_source(source: &str) -> Result<SwcProgram> {
    let file = source_file(FileName::Custom("input.js".into()), source);
    parse_script(&file)
}

pub(super) fn parse_module_program_with_path(path: &Path, source: &str) -> Result<SwcProgram> {
    let file = source_file(FileName::Real(path.to_path_buf()).into(), source);
    parse_module(&file)
}

pub(super) fn validate_script_source(source: &str) -> Result<()> {
    let file = source_file(FileName::Custom("eval.js".into()), source);
    parse_script(&file).map(|_| ())
}

pub(super) fn script_source_has_direct_using_declaration(source: &str) -> bool {
    let file = source_file(FileName::Custom("eval.js".into()), source);
    parse_script_unvalidated(&file).is_ok_and(|script| {
        script
            .body
            .iter()
            .any(|statement| matches!(statement, Stmt::Decl(Decl::Using(_))))
    })
}

pub(crate) fn parse_module_file(path: &Path) -> Result<(Module, String)> {
    let source =
        fs::read_to_string(path).with_context(|| format!("failed to read `{}`", path.display()))?;
    let file = source_file(FileName::Real(path.to_path_buf()).into(), &source);
    let SwcProgram::Module(module) = parse_module(&file)? else {
        unreachable!("parse_module must return a module");
    };
    Ok((module, source))
}

pub(crate) fn parse_script_file(path: &Path) -> Result<(swc_ecma_ast::Script, String)> {
    let source =
        fs::read_to_string(path).with_context(|| format!("failed to read `{}`", path.display()))?;

    parse_script_file_once(path, &source)
        .map(|script| (script, source.clone()))
        .or_else(|parse_error| {
            let Some(rewritten) = rewrite_script_await_identifiers(&source) else {
                return Err(parse_error);
            };
            parse_script_file_once(path, &rewritten)
                .map(|script| (script, rewritten))
                .map_err(|rewrite_error| {
                    anyhow::anyhow!(
                        "{parse_error:#}\nfailed again after rewriting script-goal `await` identifiers: {rewrite_error:#}"
                    )
                })
        })
}

fn source_file(file_name: FileName, source: &str) -> Lrc<swc_common::SourceFile> {
    let normalized = normalize_parser_source(source);
    let source_map: Lrc<SourceMap> = Default::default();
    source_map.new_source_file(file_name.into(), normalized.into_owned())
}

fn normalize_parser_source(source: &str) -> Cow<'_, str> {
    let normalized = normalize_leading_hashbang_comment(source);
    let normalized = normalize_escaped_let_statement_starts(normalized);
    let normalized = normalize_escaped_class_method_names(normalized);
    let normalized = normalize_escaped_object_property_names(normalized);
    let normalized = normalize_escaped_member_property_names(normalized);
    let normalized = normalize_static_constructor_methods(normalized);
    normalize_for_statement_using_declarations(normalized)
}

fn normalize_leading_hashbang_comment(source: &str) -> Cow<'_, str> {
    if let Some(rest) = source.strip_prefix("#!") {
        return Cow::Owned(format!("//{rest}"));
    }

    if let Some(rest) = source.strip_prefix("\u{FEFF}#!") {
        return Cow::Owned(format!("\u{FEFF}//{rest}"));
    }

    Cow::Borrowed(source)
}

fn normalize_escaped_let_statement_starts(source: Cow<'_, str>) -> Cow<'_, str> {
    if !source.contains("\\u") {
        return source;
    }

    let bytes = source.as_bytes();
    let mut output = String::new();
    let mut last_copied = 0;
    let mut index = 0;
    let mut statement_start = true;

    while index < bytes.len() {
        match bytes[index] {
            byte if byte.is_ascii_whitespace() => {
                index += 1;
            }
            b'/' if index + 1 < bytes.len() && bytes[index + 1] == b'/' => {
                index = skip_line_comment(bytes, index);
            }
            b'/' if index + 1 < bytes.len() && bytes[index + 1] == b'*' => {
                index = skip_block_comment(bytes, index);
            }
            b'\'' | b'"' => {
                index = skip_quoted_string(bytes, index);
                statement_start = false;
            }
            b'`' => {
                index = skip_template_literal(bytes, index);
                statement_start = false;
            }
            byte if is_identifier_byte(byte) || starts_unicode_escape(bytes, index) => {
                let Some((end, decoded, saw_escape)) =
                    scan_identifier_name_with_unicode_escapes(&source, index)
                else {
                    index += 1;
                    statement_start = false;
                    continue;
                };

                if statement_start
                    && saw_escape
                    && decoded == "let"
                    && trivia_after_token_has_line_terminator(bytes, end)
                {
                    output.push_str(&source[last_copied..index]);
                    output.push_str("void 0");
                    output.extend(std::iter::repeat_n(' ', end - index - "void 0".len()));
                    last_copied = end;
                }

                index = end;
                statement_start = false;
            }
            b';' | b'{' | b'}' => {
                index += 1;
                statement_start = true;
            }
            _ => {
                index += 1;
                statement_start = false;
            }
        }
    }

    if last_copied == 0 {
        return source;
    }

    output.push_str(&source[last_copied..]);
    Cow::Owned(output)
}

#[derive(Clone)]
enum ClassSignificantToken {
    Punct(u8),
    Word(String),
}

fn normalize_escaped_class_method_names(source: Cow<'_, str>) -> Cow<'_, str> {
    if !source.contains("\\u") {
        return source;
    }

    let bytes = source.as_bytes();
    let mut output = String::new();
    let mut last_copied = 0;
    let mut index = 0;
    let mut brace_depth = 0usize;
    let mut pending_class = false;
    let mut class_body_depths = Vec::new();
    let mut last_class_token: Option<ClassSignificantToken> = None;

    while index < bytes.len() {
        match bytes[index] {
            b'\'' | b'"' => {
                index = skip_quoted_string(bytes, index);
            }
            b'`' => {
                index = skip_template_literal(bytes, index);
            }
            b'/' if index + 1 < bytes.len() && bytes[index + 1] == b'/' => {
                index = skip_line_comment(bytes, index);
            }
            b'/' if index + 1 < bytes.len() && bytes[index + 1] == b'*' => {
                index = skip_block_comment(bytes, index);
            }
            b'{' => {
                brace_depth += 1;
                if pending_class {
                    class_body_depths.push(brace_depth);
                    pending_class = false;
                    last_class_token = Some(ClassSignificantToken::Punct(b'{'));
                } else if class_body_depths.last().copied() == Some(brace_depth) {
                    last_class_token = Some(ClassSignificantToken::Punct(b'{'));
                }
                index += 1;
            }
            b'}' => {
                if class_body_depths.last().copied() == Some(brace_depth) {
                    class_body_depths.pop();
                    last_class_token = None;
                }
                brace_depth = brace_depth.saturating_sub(1);
                if class_body_depths.last().copied() == Some(brace_depth) {
                    last_class_token = Some(ClassSignificantToken::Punct(b'}'));
                }
                index += 1;
            }
            byte if byte == b';' || byte == b'*' => {
                if class_body_depths.last().copied() == Some(brace_depth) {
                    last_class_token = Some(ClassSignificantToken::Punct(byte));
                }
                index += 1;
            }
            byte if is_identifier_byte(byte) || starts_unicode_escape(bytes, index) => {
                let Some((end, decoded, saw_escape)) =
                    scan_identifier_name_with_unicode_escapes(&source, index)
                else {
                    index += 1;
                    continue;
                };
                let in_class_body = class_body_depths.last().copied() == Some(brace_depth);
                if in_class_body
                    && saw_escape
                    && class_token_allows_method_key(last_class_token.as_ref())
                    && skip_whitespace_and_comments(bytes, end).and_then(|next| bytes.get(next))
                        == Some(&b'(')
                {
                    output.push_str(&source[last_copied..index]);
                    push_string_literal(&mut output, &decoded);
                    last_copied = end;
                }

                if decoded == "class" {
                    let next = skip_whitespace_and_comments(bytes, end);
                    pending_class = !matches!(next.and_then(|next| bytes.get(next)), Some(b'('));
                } else if in_class_body {
                    last_class_token = Some(ClassSignificantToken::Word(decoded));
                }
                index = end;
            }
            byte => {
                if !byte.is_ascii_whitespace()
                    && class_body_depths.last().copied() == Some(brace_depth)
                {
                    last_class_token = Some(ClassSignificantToken::Punct(byte));
                }
                index += 1;
            }
        }
    }

    if last_copied == 0 {
        return source;
    }

    output.push_str(&source[last_copied..]);
    Cow::Owned(output)
}

fn class_token_allows_method_key(token: Option<&ClassSignificantToken>) -> bool {
    match token {
        Some(ClassSignificantToken::Punct(b'{' | b';' | b'}' | b'*')) => true,
        Some(ClassSignificantToken::Word(word)) => {
            matches!(word.as_str(), "static" | "async" | "get" | "set")
        }
        _ => false,
    }
}

#[derive(Clone)]
enum SignificantToken {
    Punct(u8),
    Word(String),
}

fn token_starts_object_literal_or_pattern(token: Option<&SignificantToken>) -> bool {
    match token {
        Some(SignificantToken::Punct(b'=' | b'(' | b'[' | b',' | b':' | b'?')) => true,
        Some(SignificantToken::Word(word)) => matches!(
            word.as_str(),
            "return" | "throw" | "yield" | "var" | "let" | "const" | "case" | "default"
        ),
        _ => false,
    }
}

fn normalize_escaped_object_property_names(source: Cow<'_, str>) -> Cow<'_, str> {
    if !source.contains("\\u") {
        return source;
    }

    let bytes = source.as_bytes();
    let mut output = String::new();
    let mut last_copied = 0;
    let mut index = 0;
    let mut object_context_stack = Vec::new();
    let mut last_token: Option<SignificantToken> = None;

    while index < bytes.len() {
        match bytes[index] {
            b'\'' | b'"' => {
                index = skip_quoted_string(bytes, index);
                last_token = Some(SignificantToken::Punct(b'"'));
            }
            b'`' => {
                index = skip_template_literal(bytes, index);
                last_token = Some(SignificantToken::Punct(b'`'));
            }
            b'/' if index + 1 < bytes.len() && bytes[index + 1] == b'/' => {
                index = skip_line_comment(bytes, index);
            }
            b'/' if index + 1 < bytes.len() && bytes[index + 1] == b'*' => {
                index = skip_block_comment(bytes, index);
            }
            b'{' => {
                object_context_stack
                    .push(token_starts_object_literal_or_pattern(last_token.as_ref()));
                last_token = Some(SignificantToken::Punct(b'{'));
                index += 1;
            }
            b'}' => {
                object_context_stack.pop();
                last_token = Some(SignificantToken::Punct(b'}'));
                index += 1;
            }
            byte if is_identifier_byte(byte) || starts_unicode_escape(bytes, index) => {
                let Some((end, decoded, saw_escape)) =
                    scan_identifier_name_with_unicode_escapes(&source, index)
                else {
                    index += 1;
                    last_token = Some(SignificantToken::Punct(byte));
                    continue;
                };
                let next = skip_whitespace_and_comments(bytes, end);
                if saw_escape
                    && object_context_stack.last().copied().unwrap_or(false)
                    && next.and_then(|next| bytes.get(next)) == Some(&b':')
                {
                    output.push_str(&source[last_copied..index]);
                    push_string_literal(&mut output, &decoded);
                    last_copied = end;
                }
                last_token = Some(SignificantToken::Word(decoded));
                index = end;
            }
            byte if byte.is_ascii_whitespace() => {
                index += 1;
            }
            byte => {
                last_token = Some(SignificantToken::Punct(byte));
                index += 1;
            }
        }
    }

    if last_copied == 0 {
        return source;
    }

    output.push_str(&source[last_copied..]);
    Cow::Owned(output)
}

fn normalize_escaped_member_property_names(source: Cow<'_, str>) -> Cow<'_, str> {
    if !source.contains("\\u") {
        return source;
    }

    let bytes = source.as_bytes();
    let mut output = String::new();
    let mut last_copied = 0;
    let mut index = 0;

    while index < bytes.len() {
        match bytes[index] {
            b'\'' | b'"' => {
                index = skip_quoted_string(bytes, index);
            }
            b'`' => {
                index = skip_template_literal(bytes, index);
            }
            b'/' if index + 1 < bytes.len() && bytes[index + 1] == b'/' => {
                index = skip_line_comment(bytes, index);
            }
            b'/' if index + 1 < bytes.len() && bytes[index + 1] == b'*' => {
                index = skip_block_comment(bytes, index);
            }
            b'.' => {
                let Some(property_start) = skip_whitespace_and_comments(bytes, index + 1) else {
                    index += 1;
                    continue;
                };
                let Some((property_end, decoded, saw_escape)) =
                    scan_identifier_name_with_unicode_escapes(&source, property_start)
                else {
                    index += 1;
                    continue;
                };

                if saw_escape {
                    let optional_start = index
                        .checked_sub(1)
                        .filter(|previous| bytes.get(*previous) == Some(&b'?'));
                    let replacement_start = optional_start.unwrap_or(index);
                    output.push_str(&source[last_copied..replacement_start]);
                    if optional_start.is_some() {
                        output.push_str("?.[");
                    } else {
                        output.push('[');
                    }
                    push_string_literal(&mut output, &decoded);
                    output.push(']');
                    last_copied = property_end;
                    index = property_end;
                } else {
                    index = property_end;
                }
            }
            _ => {
                index += 1;
            }
        }
    }

    if last_copied == 0 {
        return source;
    }

    output.push_str(&source[last_copied..]);
    Cow::Owned(output)
}

fn trivia_after_token_has_line_terminator(bytes: &[u8], mut index: usize) -> bool {
    let mut saw_line_terminator = false;

    loop {
        while let Some(byte) = bytes.get(index) {
            match *byte {
                b'\n' | b'\r' => {
                    saw_line_terminator = true;
                    index += 1;
                }
                b'\t' | b'\x0b' | b'\x0c' | b' ' => {
                    index += 1;
                }
                _ => break,
            }
        }

        if bytes.get(index) == Some(&b'/') && bytes.get(index + 1) == Some(&b'/') {
            index += 2;
            while let Some(byte) = bytes.get(index) {
                if matches!(*byte, b'\n' | b'\r') {
                    saw_line_terminator = true;
                    index += 1;
                    break;
                }
                index += 1;
            }
            continue;
        }

        if bytes.get(index) == Some(&b'/') && bytes.get(index + 1) == Some(&b'*') {
            index += 2;
            while index + 1 < bytes.len() {
                if matches!(bytes[index], b'\n' | b'\r') {
                    saw_line_terminator = true;
                }
                if bytes[index] == b'*' && bytes[index + 1] == b'/' {
                    index += 2;
                    break;
                }
                index += 1;
            }
            continue;
        }

        return saw_line_terminator;
    }
}

fn starts_unicode_escape(bytes: &[u8], index: usize) -> bool {
    bytes.get(index) == Some(&b'\\') && bytes.get(index + 1) == Some(&b'u')
}

fn scan_identifier_name_with_unicode_escapes(
    source: &str,
    mut index: usize,
) -> Option<(usize, String, bool)> {
    let bytes = source.as_bytes();
    let mut decoded = String::new();
    let mut saw_escape = false;
    let mut consumed = false;

    while index < bytes.len() {
        let byte = bytes[index];
        if is_identifier_byte(byte) {
            decoded.push(byte as char);
            index += 1;
            consumed = true;
            continue;
        }

        if let Some((next, character)) = decode_unicode_escape(source, index) {
            decoded.push(character);
            index = next;
            saw_escape = true;
            consumed = true;
            continue;
        }

        break;
    }

    consumed.then_some((index, decoded, saw_escape))
}

fn decode_unicode_escape(source: &str, index: usize) -> Option<(usize, char)> {
    let bytes = source.as_bytes();
    if !starts_unicode_escape(bytes, index) {
        return None;
    }

    if bytes.get(index + 2) == Some(&b'{') {
        let mut end = index + 3;
        while end < bytes.len() && bytes[end] != b'}' {
            end += 1;
        }
        if end >= bytes.len() || end == index + 3 {
            return None;
        }
        let value = u32::from_str_radix(&source[index + 3..end], 16).ok()?;
        return char::from_u32(value).map(|character| (end + 1, character));
    }

    let end = index + 6;
    if end > bytes.len() {
        return None;
    }
    let value = u32::from_str_radix(&source[index + 2..end], 16).ok()?;
    char::from_u32(value).map(|character| (end, character))
}

fn push_string_literal(output: &mut String, value: &str) {
    output.push('"');
    for character in value.chars() {
        match character {
            '\\' => output.push_str("\\\\"),
            '"' => output.push_str("\\\""),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            _ => output.push(character),
        }
    }
    output.push('"');
}

fn normalize_static_constructor_methods(source: Cow<'_, str>) -> Cow<'_, str> {
    if !source.contains("static") || !source.contains("constructor") {
        return source;
    }

    let mut output = String::new();
    let mut last_copied = 0;
    let mut index = 0;
    let bytes = source.as_bytes();

    while index < bytes.len() {
        match bytes[index] {
            b'\'' | b'"' => {
                index = skip_quoted_string(bytes, index);
            }
            b'`' => {
                index = skip_template_literal(bytes, index);
            }
            b'/' if index + 1 < bytes.len() && bytes[index + 1] == b'/' => {
                index = skip_line_comment(bytes, index);
            }
            b'/' if index + 1 < bytes.len() && bytes[index + 1] == b'*' => {
                index = skip_block_comment(bytes, index);
            }
            _ if identifier_at(&source, index, "static") => {
                if let Some((constructor_start, constructor_end)) =
                    static_constructor_method_name_range(&source, index)
                {
                    output.push_str(&source[last_copied..constructor_start]);
                    output.push_str("[\"constructor\"]");
                    last_copied = constructor_end;
                    index = constructor_end;
                } else {
                    index += 1;
                }
            }
            _ => {
                index += 1;
            }
        }
    }

    if last_copied == 0 {
        return source;
    }

    output.push_str(&source[last_copied..]);
    Cow::Owned(output)
}

fn normalize_for_statement_using_declarations(source: Cow<'_, str>) -> Cow<'_, str> {
    if !source.contains("for") || !source.contains("using") {
        return source;
    }

    let bytes = source.as_bytes();
    let mut output = String::new();
    let mut last_copied = 0;
    let mut index = 0;

    while index < bytes.len() {
        match bytes[index] {
            b'\'' | b'"' => {
                index = skip_quoted_string(bytes, index);
            }
            b'`' => {
                index = skip_template_literal(bytes, index);
            }
            b'/' if index + 1 < bytes.len() && bytes[index + 1] == b'/' => {
                index = skip_line_comment(bytes, index);
            }
            b'/' if index + 1 < bytes.len() && bytes[index + 1] == b'*' => {
                index = skip_block_comment(bytes, index);
            }
            _ if identifier_at(&source, index, "for") => {
                let Some(open_paren) = skip_whitespace_and_comments(bytes, index + "for".len())
                else {
                    index += 1;
                    continue;
                };
                if bytes.get(open_paren) != Some(&b'(') {
                    index += 1;
                    continue;
                }
                let Some(head_start) = skip_whitespace_and_comments(bytes, open_paren + 1) else {
                    index += 1;
                    continue;
                };
                if !identifier_at(&source, head_start, "using") {
                    index += 1;
                    continue;
                }
                let Some(close_paren) = find_matching_delimiter(bytes, open_paren, b'(', b')')
                else {
                    index += 1;
                    continue;
                };
                let Some(first_semicolon) =
                    find_top_level_semicolon(bytes, head_start, close_paren)
                else {
                    index = close_paren + 1;
                    continue;
                };
                let Some(body_start) = skip_whitespace_and_comments(bytes, close_paren + 1) else {
                    index = close_paren + 1;
                    continue;
                };
                let Some(body_end) = (if bytes.get(body_start) == Some(&b'{') {
                    find_matching_delimiter(bytes, body_start, b'{', b'}').map(|end| end + 1)
                } else {
                    find_single_statement_end(bytes, body_start)
                }) else {
                    index = close_paren + 1;
                    continue;
                };

                output.push_str(&source[last_copied..index]);
                output.push_str("{ ");
                output.push_str(&source[head_start..first_semicolon]);
                output.push_str("; for (;");
                output.push_str(&source[first_semicolon + 1..close_paren]);
                output.push(')');
                output.push_str(&source[close_paren + 1..body_end]);
                output.push_str(" }");
                last_copied = body_end;
                index = body_end;
            }
            _ => {
                index += 1;
            }
        }
    }

    if last_copied == 0 {
        return source;
    }

    output.push_str(&source[last_copied..]);
    Cow::Owned(output)
}

fn find_matching_delimiter(bytes: &[u8], start: usize, open: u8, close: u8) -> Option<usize> {
    let mut index = start;
    let mut depth = 0usize;

    while index < bytes.len() {
        match bytes[index] {
            b'\'' | b'"' => {
                index = skip_quoted_string(bytes, index);
                continue;
            }
            b'`' => {
                index = skip_template_literal(bytes, index);
                continue;
            }
            b'/' if index + 1 < bytes.len() && bytes[index + 1] == b'/' => {
                index = skip_line_comment(bytes, index);
                continue;
            }
            b'/' if index + 1 < bytes.len() && bytes[index + 1] == b'*' => {
                index = skip_block_comment(bytes, index);
                continue;
            }
            byte if byte == open => {
                depth += 1;
            }
            byte if byte == close => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
        index += 1;
    }

    None
}

fn find_single_statement_end(bytes: &[u8], start: usize) -> Option<usize> {
    let mut index = start;
    let mut paren_depth = 0usize;
    let mut bracket_depth = 0usize;
    let mut brace_depth = 0usize;

    while index < bytes.len() {
        match bytes[index] {
            b'\'' | b'"' => {
                index = skip_quoted_string(bytes, index);
                continue;
            }
            b'`' => {
                index = skip_template_literal(bytes, index);
                continue;
            }
            b'/' if index + 1 < bytes.len() && bytes[index + 1] == b'/' => {
                index = skip_line_comment(bytes, index);
                continue;
            }
            b'/' if index + 1 < bytes.len() && bytes[index + 1] == b'*' => {
                index = skip_block_comment(bytes, index);
                continue;
            }
            b'(' => paren_depth += 1,
            b')' => paren_depth = paren_depth.saturating_sub(1),
            b'[' => bracket_depth += 1,
            b']' => bracket_depth = bracket_depth.saturating_sub(1),
            b'{' => brace_depth += 1,
            b'}' => {
                if paren_depth == 0 && bracket_depth == 0 && brace_depth == 0 {
                    return Some(index);
                }
                brace_depth = brace_depth.saturating_sub(1);
            }
            b';' if paren_depth == 0 && bracket_depth == 0 && brace_depth == 0 => {
                return Some(index + 1);
            }
            _ => {}
        }
        index += 1;
    }

    None
}

fn find_top_level_semicolon(bytes: &[u8], start: usize, end: usize) -> Option<usize> {
    let mut index = start;
    let mut paren_depth = 0usize;
    let mut bracket_depth = 0usize;
    let mut brace_depth = 0usize;

    while index < end {
        match bytes[index] {
            b'\'' | b'"' => {
                index = skip_quoted_string(bytes, index);
                continue;
            }
            b'`' => {
                index = skip_template_literal(bytes, index);
                continue;
            }
            b'/' if index + 1 < bytes.len() && bytes[index + 1] == b'/' => {
                index = skip_line_comment(bytes, index);
                continue;
            }
            b'/' if index + 1 < bytes.len() && bytes[index + 1] == b'*' => {
                index = skip_block_comment(bytes, index);
                continue;
            }
            b'(' => paren_depth += 1,
            b')' => paren_depth = paren_depth.saturating_sub(1),
            b'[' => bracket_depth += 1,
            b']' => bracket_depth = bracket_depth.saturating_sub(1),
            b'{' => brace_depth += 1,
            b'}' => brace_depth = brace_depth.saturating_sub(1),
            b';' if paren_depth == 0 && bracket_depth == 0 && brace_depth == 0 => {
                return Some(index);
            }
            _ => {}
        }
        index += 1;
    }

    None
}

fn static_constructor_method_name_range(
    source: &str,
    static_start: usize,
) -> Option<(usize, usize)> {
    let bytes = source.as_bytes();
    let mut index = skip_whitespace_and_comments(bytes, static_start + "static".len())?;

    if identifier_at(source, index, "get") || identifier_at(source, index, "set") {
        index = skip_whitespace_and_comments(bytes, index + 3)?;
        return constructor_method_name_range_after_prefix(source, index);
    }

    if identifier_at(source, index, "async") {
        index = skip_whitespace_and_comments(bytes, index + "async".len())?;
        if bytes.get(index) == Some(&b'*') {
            index = skip_whitespace_and_comments(bytes, index + 1)?;
        }
        return constructor_method_name_range_after_prefix(source, index);
    }

    if bytes.get(index) == Some(&b'*') {
        index = skip_whitespace_and_comments(bytes, index + 1)?;
        return constructor_method_name_range_after_prefix(source, index);
    }

    constructor_method_name_range_after_prefix(source, index)
}

fn constructor_method_name_range_after_prefix(
    source: &str,
    index: usize,
) -> Option<(usize, usize)> {
    if !identifier_at(source, index, "constructor") {
        return None;
    }

    let constructor_end = index + "constructor".len();
    let next = skip_whitespace_and_comments(source.as_bytes(), constructor_end)?;
    if source.as_bytes().get(next) == Some(&b'(') {
        Some((index, constructor_end))
    } else {
        None
    }
}

fn identifier_at(source: &str, index: usize, expected: &str) -> bool {
    if !source.as_bytes()[index..].starts_with(expected.as_bytes()) {
        return false;
    }

    let end = index + expected.len();
    let before = index
        .checked_sub(1)
        .and_then(|previous| source.as_bytes().get(previous));
    let after = source.as_bytes().get(end);
    !before.is_some_and(|byte| is_identifier_byte(*byte))
        && !after.is_some_and(|byte| is_identifier_byte(*byte))
}

fn is_identifier_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'$')
}

fn skip_whitespace_and_comments(bytes: &[u8], mut index: usize) -> Option<usize> {
    loop {
        while bytes
            .get(index)
            .is_some_and(|byte| byte.is_ascii_whitespace())
        {
            index += 1;
        }

        if bytes.get(index) == Some(&b'/') && bytes.get(index + 1) == Some(&b'/') {
            index = skip_line_comment(bytes, index);
            continue;
        }

        if bytes.get(index) == Some(&b'/') && bytes.get(index + 1) == Some(&b'*') {
            let next = skip_block_comment(bytes, index);
            if next == bytes.len() {
                return None;
            }
            index = next;
            continue;
        }

        return Some(index);
    }
}

fn skip_quoted_string(bytes: &[u8], mut index: usize) -> usize {
    let quote = bytes[index];
    index += 1;
    while index < bytes.len() {
        match bytes[index] {
            b'\\' => {
                index = (index + 2).min(bytes.len());
            }
            byte if byte == quote => {
                return index + 1;
            }
            _ => {
                index += 1;
            }
        }
    }
    index
}

fn skip_template_literal(bytes: &[u8], mut index: usize) -> usize {
    index += 1;
    while index < bytes.len() {
        match bytes[index] {
            b'\\' => {
                index = (index + 2).min(bytes.len());
            }
            b'`' => {
                return index + 1;
            }
            _ => {
                index += 1;
            }
        }
    }
    index
}

fn skip_line_comment(bytes: &[u8], mut index: usize) -> usize {
    index += 2;
    while index < bytes.len() && !matches!(bytes[index], b'\n' | b'\r') {
        index += 1;
    }
    index
}

fn skip_block_comment(bytes: &[u8], mut index: usize) -> usize {
    index += 2;
    while index + 1 < bytes.len() {
        if bytes[index] == b'*' && bytes[index + 1] == b'/' {
            return index + 2;
        }
        index += 1;
    }
    bytes.len()
}

fn parse_script(file: &swc_common::SourceFile) -> Result<SwcProgram> {
    let script = parse_script_unvalidated(file)?;
    validate_script_ast(&script, file)?;
    Ok(SwcProgram::Script(script))
}

fn parse_script_unvalidated(file: &swc_common::SourceFile) -> Result<swc_ecma_ast::Script> {
    let lexer = Lexer::new(
        script_syntax(),
        Default::default(),
        StringInput::from(file),
        None,
    );
    let mut parser = Parser::new_from(lexer);
    let script = parser
        .parse_script()
        .map_err(|error| anyhow::anyhow!("{error:?}"))?;
    if let Some(error) = parser.take_errors().into_iter().next() {
        bail!("{error:?}");
    }
    Ok(script)
}

fn parse_module(file: &swc_common::SourceFile) -> Result<SwcProgram> {
    let lexer = Lexer::new(
        script_syntax(),
        Default::default(),
        StringInput::from(file),
        None,
    );
    let mut parser = Parser::new_from(lexer);
    let module = parser
        .parse_module()
        .map_err(|error| anyhow::anyhow!("{error:?}"))?;
    if let Some(error) = parser.take_errors().into_iter().next() {
        bail!("{error:?}");
    }
    validate_module_ast(&module, file)?;
    Ok(SwcProgram::Module(module))
}

fn parse_script_file_once(path: &Path, source: &str) -> Result<swc_ecma_ast::Script> {
    let file = source_file(FileName::Real(path.to_path_buf()).into(), source);
    let SwcProgram::Script(script) = parse_script(&file)? else {
        unreachable!("parse_script must return a script");
    };
    Ok(script)
}

fn script_syntax() -> Syntax {
    Syntax::Es(EsSyntax {
        decorators: true,
        decorators_before_export: true,
        auto_accessors: true,
        explicit_resource_management: true,
        ..Default::default()
    })
}
