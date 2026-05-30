use std::collections::HashSet;

use anyhow::{Context, Result, ensure};
use swc_common::{Spanned, source_map::SmallPos};
use swc_ecma_ast::{
    Decl, DefaultDecl, ExportDefaultDecl, ExportSpecifier, Module, ModuleDecl, ModuleExportName,
    ModuleItem, NamedExport,
};

use crate::frontend::early_errors::{
    collect_var_decl_bound_names, script_has_use_strict_directive, validate_class_syntax,
    validate_declaration_syntax, validate_expression_syntax, validate_function_syntax,
    validate_import_attributes, validate_script_body_early_errors, validate_statement_syntax,
    validate_strict_mode_early_errors_in_module_items,
    validate_strict_mode_early_errors_in_statements,
};

pub(super) fn validate_script_ast(
    script: &swc_ecma_ast::Script,
    file: &swc_common::SourceFile,
) -> Result<()> {
    validate_script_body_early_errors(&script.body)?;

    for statement in &script.body {
        validate_statement_syntax(statement, file)?;
    }

    validate_strict_mode_early_errors_in_statements(
        &script.body,
        script_has_use_strict_directive(&script.body),
    )?;

    Ok(())
}

pub(super) fn validate_module_ast(module: &Module, file: &swc_common::SourceFile) -> Result<()> {
    validate_module_exported_names_are_unique(module)?;
    validate_named_export_statement_boundaries(module, file)?;

    for item in &module.body {
        match item {
            ModuleItem::Stmt(statement) => validate_statement_syntax(statement, file)?,
            ModuleItem::ModuleDecl(module_declaration) => match module_declaration {
                ModuleDecl::Import(import) => {
                    validate_import_attributes(import.with.as_deref())?;
                }
                ModuleDecl::ExportNamed(export) => {
                    validate_import_attributes(export.with.as_deref())?;
                }
                ModuleDecl::ExportAll(export) => {
                    validate_import_attributes(export.with.as_deref())?;
                }
                ModuleDecl::ExportDecl(export) => validate_declaration_syntax(&export.decl, file)?,
                ModuleDecl::ExportDefaultDecl(ExportDefaultDecl { decl, .. }) => match decl {
                    DefaultDecl::Fn(function) => {
                        validate_function_syntax(&function.function, file)?
                    }
                    DefaultDecl::Class(class) => validate_class_syntax(&class.class, file)?,
                    _ => {}
                },
                ModuleDecl::ExportDefaultExpr(export) => {
                    validate_expression_syntax(&export.expr, file)?;
                }
                _ => {}
            },
        }
    }

    validate_strict_mode_early_errors_in_module_items(&module.body, true)?;

    Ok(())
}

fn validate_named_export_statement_boundaries(
    module: &Module,
    file: &swc_common::SourceFile,
) -> Result<()> {
    for item in &module.body {
        let ModuleItem::ModuleDecl(ModuleDecl::ExportNamed(export_named)) = item else {
            continue;
        };

        ensure!(
            named_export_span_has_statement_boundary(export_named, file),
            "`export` declaration must be followed by a semicolon or line terminator"
        );
    }

    for window in module.body.windows(2) {
        let ModuleItem::ModuleDecl(ModuleDecl::ExportNamed(export_named)) = &window[0] else {
            continue;
        };

        ensure!(
            module_decl_has_statement_boundary(file, export_named.span.hi, window[1].span().lo()),
            "`export` declaration must be followed by a semicolon or line terminator"
        );
    }

    Ok(())
}

fn named_export_span_has_statement_boundary(
    export_named: &NamedExport,
    file: &swc_common::SourceFile,
) -> bool {
    let source: &str = file.src.as_ref();
    let file_start = file.start_pos.to_usize();
    let start = export_named.span.lo.to_usize().saturating_sub(file_start);
    let Some(raw) = source.get(start.min(source.len())..) else {
        return false;
    };
    named_export_raw_has_statement_boundary(raw)
}

fn named_export_raw_has_statement_boundary(raw: &str) -> bool {
    let bytes = raw.as_bytes();
    let Some(export_start) = find_keyword_token(bytes, 0, b"export") else {
        return true;
    };

    let mut index = export_start + "export".len();
    let (next, _) = skip_export_boundary_trivia(bytes, index);
    index = next;

    if bytes.get(index) == Some(&b'{') {
        let Some(close_brace) = find_matching_export_delimiter(bytes, index, b'{', b'}') else {
            return true;
        };
        index = close_brace + 1;
        return named_export_tail_has_statement_boundary(bytes, index);
    }

    if bytes.get(index) == Some(&b'*') {
        if let Some(from_index) = find_keyword_token(bytes, index + 1, b"from") {
            return named_export_tail_has_statement_boundary(bytes, from_index);
        }
    }

    true
}

fn named_export_tail_has_statement_boundary(bytes: &[u8], index: usize) -> bool {
    let (mut index, saw_line_terminator) = skip_export_boundary_trivia(bytes, index);
    let Some(token) = bytes.get(index).copied() else {
        return true;
    };
    if saw_line_terminator || token == b';' {
        return true;
    }

    if !keyword_token_starts_at(bytes, index, b"from") {
        return false;
    }

    index += "from".len();
    let (next, _) = skip_export_boundary_trivia(bytes, index);
    index = next;
    let Some(after_source) = skip_export_source_literal(bytes, index) else {
        return true;
    };
    index = after_source;

    let (next, saw_line_terminator) = skip_export_boundary_trivia(bytes, index);
    index = next;
    if saw_line_terminator {
        return true;
    }

    if keyword_token_starts_at(bytes, index, b"with") {
        index += "with".len();
        let (next, _) = skip_export_boundary_trivia(bytes, index);
        index = next;
        if bytes.get(index) == Some(&b'{') {
            let Some(close_brace) = find_matching_export_delimiter(bytes, index, b'{', b'}') else {
                return true;
            };
            index = close_brace + 1;
        }
    }

    let (index, saw_line_terminator) = skip_export_boundary_trivia(bytes, index);
    saw_line_terminator || bytes.get(index).is_none_or(|byte| *byte == b';')
}

fn module_decl_has_statement_boundary(
    file: &swc_common::SourceFile,
    declaration_end: swc_common::BytePos,
    next_item_start: swc_common::BytePos,
) -> bool {
    let source: &str = file.src.as_ref();
    let file_start = file.start_pos.to_usize();
    let declaration_end = declaration_end.to_usize().saturating_sub(file_start);
    let next_item_start = next_item_start.to_usize().saturating_sub(file_start);

    if declaration_end > 0 && source.as_bytes().get(declaration_end - 1) == Some(&b';') {
        return true;
    }

    let Some(gap) =
        source.get(declaration_end.min(source.len())..next_item_start.min(source.len()))
    else {
        return false;
    };

    let (saw_line_terminator, next_token) = scan_export_boundary_gap(gap.as_bytes());
    saw_line_terminator || next_token.is_none() || next_token == Some(b';')
}

fn skip_export_boundary_trivia(bytes: &[u8], mut index: usize) -> (usize, bool) {
    let mut saw_line_terminator = false;

    while index < bytes.len() {
        match bytes[index] {
            b'\n' | b'\r' => {
                saw_line_terminator = true;
                index += 1;
            }
            b'\t' | b'\x0b' | b'\x0c' | b' ' => {
                index += 1;
            }
            b'/' if bytes.get(index + 1) == Some(&b'/') => {
                index += 2;
                while bytes
                    .get(index)
                    .is_some_and(|byte| !matches!(*byte, b'\n' | b'\r'))
                {
                    index += 1;
                }
            }
            b'/' if bytes.get(index + 1) == Some(&b'*') => {
                index += 2;
                while index + 1 < bytes.len() && bytes.get(index..index + 2) != Some(b"*/") {
                    if matches!(bytes[index], b'\n' | b'\r') {
                        saw_line_terminator = true;
                    }
                    index += 1;
                }
                index = (index + 2).min(bytes.len());
            }
            _ => return (index, saw_line_terminator),
        }
    }

    (index, saw_line_terminator)
}

fn skip_export_source_literal(bytes: &[u8], index: usize) -> Option<usize> {
    let quote = *bytes.get(index)?;
    if !matches!(quote, b'\'' | b'"') {
        return None;
    }
    Some(skip_quoted_literal(bytes, index, quote))
}

fn skip_quoted_literal(bytes: &[u8], mut index: usize, quote: u8) -> usize {
    index += 1;
    while index < bytes.len() {
        match bytes[index] {
            b'\\' => index = (index + 2).min(bytes.len()),
            byte if byte == quote => return index + 1,
            _ => index += 1,
        }
    }
    bytes.len()
}

fn find_matching_export_delimiter(
    bytes: &[u8],
    open_index: usize,
    open: u8,
    close: u8,
) -> Option<usize> {
    let mut index = open_index + 1;
    let mut depth = 1usize;

    while index < bytes.len() {
        match bytes[index] {
            b'\'' | b'"' => {
                index = skip_quoted_literal(bytes, index, bytes[index]);
            }
            b'/' if bytes.get(index + 1) == Some(&b'/') => {
                index += 2;
                while bytes
                    .get(index)
                    .is_some_and(|byte| !matches!(*byte, b'\n' | b'\r'))
                {
                    index += 1;
                }
            }
            b'/' if bytes.get(index + 1) == Some(&b'*') => {
                index += 2;
                while index + 1 < bytes.len() && bytes.get(index..index + 2) != Some(b"*/") {
                    index += 1;
                }
                index = (index + 2).min(bytes.len());
            }
            byte if byte == open => {
                depth += 1;
                index += 1;
            }
            byte if byte == close => {
                depth -= 1;
                if depth == 0 {
                    return Some(index);
                }
                index += 1;
            }
            _ => index += 1,
        }
    }

    None
}

fn find_keyword_token(bytes: &[u8], mut index: usize, keyword: &[u8]) -> Option<usize> {
    while index + keyword.len() <= bytes.len() {
        if keyword_token_starts_at(bytes, index, keyword) {
            return Some(index);
        }
        index += 1;
    }
    None
}

fn keyword_token_starts_at(bytes: &[u8], index: usize, keyword: &[u8]) -> bool {
    bytes.get(index..index + keyword.len()) == Some(keyword)
        && !bytes
            .get(index.wrapping_sub(1))
            .is_some_and(|byte| is_ascii_identifier_continue(*byte))
        && !bytes
            .get(index + keyword.len())
            .is_some_and(|byte| is_ascii_identifier_continue(*byte))
}

fn is_ascii_identifier_continue(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'$')
}

fn scan_export_boundary_gap(bytes: &[u8]) -> (bool, Option<u8>) {
    let mut index = 0;
    let mut saw_line_terminator = false;

    while index < bytes.len() {
        match bytes[index] {
            b'\n' | b'\r' => {
                saw_line_terminator = true;
                index += 1;
            }
            b'\t' | b'\x0b' | b'\x0c' | b' ' => {
                index += 1;
            }
            b'/' if bytes.get(index + 1) == Some(&b'/') => {
                index += 2;
                while bytes
                    .get(index)
                    .is_some_and(|byte| !matches!(*byte, b'\n' | b'\r'))
                {
                    index += 1;
                }
            }
            b'/' if bytes.get(index + 1) == Some(&b'*') => {
                index += 2;
                while index + 1 < bytes.len() && bytes.get(index..index + 2) != Some(b"*/") {
                    if matches!(bytes[index], b'\n' | b'\r') {
                        saw_line_terminator = true;
                    }
                    index += 1;
                }
                index = (index + 2).min(bytes.len());
            }
            byte => return (saw_line_terminator, Some(byte)),
        }
    }

    (saw_line_terminator, None)
}

fn validate_module_exported_names_are_unique(module: &Module) -> Result<()> {
    let mut exported_names = HashSet::new();

    for item in &module.body {
        let ModuleItem::ModuleDecl(module_declaration) = item else {
            continue;
        };

        match module_declaration {
            ModuleDecl::ExportDecl(export) => match &export.decl {
                Decl::Fn(function) => {
                    insert_exported_name(&mut exported_names, function.ident.sym.to_string())?;
                }
                Decl::Class(class) => {
                    insert_exported_name(&mut exported_names, class.ident.sym.to_string())?;
                }
                Decl::Var(variable) => {
                    for name in collect_var_decl_bound_names(variable)? {
                        insert_exported_name(&mut exported_names, name)?;
                    }
                }
                _ => {}
            },
            ModuleDecl::ExportDefaultDecl(_) | ModuleDecl::ExportDefaultExpr(_) => {
                insert_exported_name(&mut exported_names, "default".to_string())?;
            }
            ModuleDecl::ExportNamed(export_named) => {
                for specifier in &export_named.specifiers {
                    let export_name = match specifier {
                        ExportSpecifier::Named(named) => named
                            .exported
                            .as_ref()
                            .map(module_export_name_to_string)
                            .transpose()?
                            .unwrap_or(module_export_name_to_string(&named.orig)?),
                        ExportSpecifier::Namespace(namespace) => {
                            module_export_name_to_string(&namespace.name)?
                        }
                        ExportSpecifier::Default(default) => default.exported.sym.to_string(),
                    };
                    insert_exported_name(&mut exported_names, export_name)?;
                }
            }
            ModuleDecl::ExportAll(_) => {}
            _ => {}
        }
    }

    Ok(())
}

fn insert_exported_name(exported_names: &mut HashSet<String>, export_name: String) -> Result<()> {
    ensure!(
        exported_names.insert(export_name.clone()),
        "duplicate export name `{export_name}`"
    );
    Ok(())
}

fn module_export_name_to_string(name: &ModuleExportName) -> Result<String> {
    match name {
        ModuleExportName::Ident(identifier) => Ok(identifier.sym.to_string()),
        ModuleExportName::Str(string) => string
            .value
            .as_str()
            .map(str::to_string)
            .context("malformed module export name"),
    }
}
