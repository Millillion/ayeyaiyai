use super::super::*;
use super::bindings::{collect_using_decl_bound_names, collect_var_decl_bound_names};

pub(super) fn validate_block_statement_early_errors(statements: &[Stmt]) -> Result<()> {
    let lexical_names = ensure_direct_statement_lexical_names_are_unique(statements, true)?;
    let var_names = collect_var_declared_names_from_statement_list(statements, false)?;
    ensure_lexical_names_do_not_overlap_var_names(lexical_names, var_names)
}

pub(crate) fn validate_script_body_early_errors(statements: &[Stmt]) -> Result<()> {
    ensure_no_direct_script_body_using_declarations(statements)?;

    let lexical_names = ensure_direct_statement_lexical_names_are_unique(statements, false)?;
    let var_names = collect_var_declared_names_from_statement_list(statements, true)?;
    ensure_lexical_names_do_not_overlap_var_names(lexical_names, var_names)
}

fn ensure_no_direct_script_body_using_declarations(statements: &[Stmt]) -> Result<()> {
    for statement in statements {
        ensure!(
            !matches!(statement, Stmt::Decl(Decl::Using(_))),
            "using declaration is not allowed directly in script body"
        );
    }

    Ok(())
}

fn ensure_lexical_names_do_not_overlap_var_names(
    lexical_names: Vec<String>,
    var_names: Vec<String>,
) -> Result<()> {
    let var_names = var_names.into_iter().collect::<HashSet<_>>();

    for name in lexical_names {
        ensure!(
            !var_names.contains(&name),
            "duplicate lexical name `{name}`"
        );
    }

    Ok(())
}

fn ensure_direct_statement_lexical_names_are_unique(
    statements: &[Stmt],
    include_function_declarations: bool,
) -> Result<Vec<String>> {
    let mut names = Vec::new();
    let mut seen = HashSet::new();

    for statement in statements {
        match statement {
            Stmt::Decl(Decl::Var(variable_declaration))
                if !matches!(variable_declaration.kind, VarDeclKind::Var) =>
            {
                for name in collect_var_decl_bound_names(variable_declaration)? {
                    ensure!(seen.insert(name.clone()), "duplicate lexical name `{name}`");
                    names.push(name);
                }
            }
            Stmt::Decl(Decl::Using(using_declaration)) => {
                for name in collect_using_decl_bound_names(using_declaration)? {
                    ensure!(seen.insert(name.clone()), "duplicate lexical name `{name}`");
                    names.push(name);
                }
            }
            Stmt::Decl(Decl::Fn(function_declaration)) if include_function_declarations => {
                let name = function_declaration.ident.sym.to_string();
                ensure!(seen.insert(name.clone()), "duplicate lexical name `{name}`");
                names.push(name);
            }
            Stmt::Decl(Decl::Class(class_declaration)) => {
                let name = class_declaration.ident.sym.to_string();
                ensure!(seen.insert(name.clone()), "duplicate lexical name `{name}`");
                names.push(name);
            }
            _ => {}
        }
    }

    Ok(names)
}

pub(super) fn collect_var_declared_names_from_statement(
    statement: &Stmt,
    include_function_declarations: bool,
) -> Result<Vec<String>> {
    let mut names = Vec::new();
    collect_var_declared_names_from_statement_into(
        statement,
        &mut names,
        include_function_declarations,
    )?;
    Ok(names)
}

fn collect_var_declared_names_from_statement_list(
    statements: &[Stmt],
    include_function_declarations: bool,
) -> Result<Vec<String>> {
    let mut names = Vec::new();
    for statement in statements {
        collect_var_declared_names_from_statement_into(
            statement,
            &mut names,
            include_function_declarations,
        )?;
    }
    Ok(names)
}

fn collect_var_declared_names_from_statement_into(
    statement: &Stmt,
    names: &mut Vec<String>,
    include_function_declarations: bool,
) -> Result<()> {
    match statement {
        Stmt::Decl(Decl::Var(variable_declaration))
            if matches!(variable_declaration.kind, VarDeclKind::Var) =>
        {
            insert_var_declared_names(variable_declaration, names)?;
        }
        Stmt::Decl(Decl::Fn(function_declaration)) if include_function_declarations => {
            let name = function_declaration.ident.sym.to_string();
            if !names.contains(&name) {
                names.push(name);
            }
        }
        Stmt::Block(block) => {
            for statement in &block.stmts {
                collect_var_declared_names_from_statement_into(
                    statement,
                    names,
                    include_function_declarations,
                )?;
            }
        }
        Stmt::Labeled(statement) => collect_var_declared_names_from_statement_into(
            &statement.body,
            names,
            include_function_declarations,
        )?,
        Stmt::If(statement) => {
            collect_var_declared_names_from_statement_into(
                &statement.cons,
                names,
                include_function_declarations,
            )?;
            if let Some(alternate) = &statement.alt {
                collect_var_declared_names_from_statement_into(
                    alternate,
                    names,
                    include_function_declarations,
                )?;
            }
        }
        Stmt::While(statement) => collect_var_declared_names_from_statement_into(
            &statement.body,
            names,
            include_function_declarations,
        )?,
        Stmt::DoWhile(statement) => collect_var_declared_names_from_statement_into(
            &statement.body,
            names,
            include_function_declarations,
        )?,
        Stmt::For(statement) => {
            if let Some(VarDeclOrExpr::VarDecl(variable_declaration)) = &statement.init
                && matches!(variable_declaration.kind, VarDeclKind::Var)
            {
                insert_var_declared_names(variable_declaration, names)?;
            }
            collect_var_declared_names_from_statement_into(
                &statement.body,
                names,
                include_function_declarations,
            )?;
        }
        Stmt::ForIn(statement) => {
            if let ForHead::VarDecl(variable_declaration) = &statement.left
                && matches!(variable_declaration.kind, VarDeclKind::Var)
            {
                insert_var_declared_names(variable_declaration, names)?;
            }
            collect_var_declared_names_from_statement_into(
                &statement.body,
                names,
                include_function_declarations,
            )?;
        }
        Stmt::ForOf(statement) => {
            if let ForHead::VarDecl(variable_declaration) = &statement.left
                && matches!(variable_declaration.kind, VarDeclKind::Var)
            {
                insert_var_declared_names(variable_declaration, names)?;
            }
            collect_var_declared_names_from_statement_into(
                &statement.body,
                names,
                include_function_declarations,
            )?;
        }
        Stmt::Switch(statement) => {
            for case in &statement.cases {
                for statement in &case.cons {
                    collect_var_declared_names_from_statement_into(
                        statement,
                        names,
                        include_function_declarations,
                    )?;
                }
            }
        }
        Stmt::Try(statement) => {
            for statement in &statement.block.stmts {
                collect_var_declared_names_from_statement_into(
                    statement,
                    names,
                    include_function_declarations,
                )?;
            }
            if let Some(handler) = &statement.handler {
                for statement in &handler.body.stmts {
                    collect_var_declared_names_from_statement_into(
                        statement,
                        names,
                        include_function_declarations,
                    )?;
                }
            }
            if let Some(finalizer) = &statement.finalizer {
                for statement in &finalizer.stmts {
                    collect_var_declared_names_from_statement_into(
                        statement,
                        names,
                        include_function_declarations,
                    )?;
                }
            }
        }
        Stmt::With(statement) => collect_var_declared_names_from_statement_into(
            &statement.body,
            names,
            include_function_declarations,
        )?,
        _ => {}
    }

    Ok(())
}

fn insert_var_declared_names(
    variable_declaration: &swc_ecma_ast::VarDecl,
    names: &mut Vec<String>,
) -> Result<()> {
    for name in collect_var_decl_bound_names(variable_declaration)? {
        if !names.contains(&name) {
            names.push(name);
        }
    }
    Ok(())
}

pub(super) fn validate_classic_for_header(
    statement: &swc_ecma_ast::ForStmt,
    file: &swc_common::SourceFile,
) -> Result<()> {
    let source: &str = file.src.as_ref();
    let start = statement.span.lo.to_usize() - file.start_pos.to_usize();
    let end = statement.span.hi.to_usize() - file.start_pos.to_usize();
    let statement_source = source
        .get(start..end)
        .context("classic `for` statement span fell outside the source file")?;
    let semicolon_count = count_classic_for_header_semicolons(statement_source)?;

    ensure!(
        semicolon_count == 2,
        "invalid classic `for` header: expected 2 top-level semicolons, found {semicolon_count}"
    );

    Ok(())
}

fn count_classic_for_header_semicolons(statement_source: &str) -> Result<usize> {
    #[derive(Clone, Copy)]
    enum State {
        Code,
        SingleQuoted,
        DoubleQuoted,
        Template,
        LineComment,
        BlockComment,
    }

    let bytes = statement_source.as_bytes();
    let mut state = State::Code;
    let mut index = 0;

    while index < bytes.len() {
        let character = bytes[index];
        let next = bytes.get(index + 1).copied();

        match state {
            State::Code => match character {
                b'\'' => state = State::SingleQuoted,
                b'"' => state = State::DoubleQuoted,
                b'`' => state = State::Template,
                b'/' if next == Some(b'/') => {
                    state = State::LineComment;
                    index += 1;
                }
                b'/' if next == Some(b'*') => {
                    state = State::BlockComment;
                    index += 1;
                }
                b'(' => {
                    index += 1;
                    break;
                }
                _ => {}
            },
            State::SingleQuoted => {
                if character == b'\\' {
                    index += 1;
                } else if character == b'\'' {
                    state = State::Code;
                }
            }
            State::DoubleQuoted => {
                if character == b'\\' {
                    index += 1;
                } else if character == b'"' {
                    state = State::Code;
                }
            }
            State::Template => {
                if character == b'\\' {
                    index += 1;
                } else if character == b'`' {
                    state = State::Code;
                }
            }
            State::LineComment => {
                if character == b'\n' {
                    state = State::Code;
                }
            }
            State::BlockComment => {
                if character == b'*' && next == Some(b'/') {
                    state = State::Code;
                    index += 1;
                }
            }
        }

        index += 1;
    }

    ensure!(
        index <= bytes.len(),
        "classic `for` header did not contain an opening parenthesis"
    );

    state = State::Code;
    let mut paren_depth = 0usize;
    let mut bracket_depth = 0usize;
    let mut brace_depth = 0usize;
    let mut semicolon_count = 0usize;

    while index < bytes.len() {
        let character = bytes[index];
        let next = bytes.get(index + 1).copied();

        match state {
            State::Code => match character {
                b'\'' => state = State::SingleQuoted,
                b'"' => state = State::DoubleQuoted,
                b'`' => state = State::Template,
                b'/' if next == Some(b'/') => {
                    state = State::LineComment;
                    index += 1;
                }
                b'/' if next == Some(b'*') => {
                    state = State::BlockComment;
                    index += 1;
                }
                b'(' => paren_depth += 1,
                b'[' => bracket_depth += 1,
                b'{' => brace_depth += 1,
                b')' if paren_depth == 0 && bracket_depth == 0 && brace_depth == 0 => {
                    return Ok(semicolon_count);
                }
                b')' => {
                    paren_depth = paren_depth.saturating_sub(1);
                }
                b']' => {
                    bracket_depth = bracket_depth.saturating_sub(1);
                }
                b'}' => {
                    brace_depth = brace_depth.saturating_sub(1);
                }
                b';' if paren_depth == 0 && bracket_depth == 0 && brace_depth == 0 => {
                    semicolon_count += 1;
                }
                _ => {}
            },
            State::SingleQuoted => {
                if character == b'\\' {
                    index += 1;
                } else if character == b'\'' {
                    state = State::Code;
                }
            }
            State::DoubleQuoted => {
                if character == b'\\' {
                    index += 1;
                } else if character == b'"' {
                    state = State::Code;
                }
            }
            State::Template => {
                if character == b'\\' {
                    index += 1;
                } else if character == b'`' {
                    state = State::Code;
                }
            }
            State::LineComment => {
                if character == b'\n' {
                    state = State::Code;
                }
            }
            State::BlockComment => {
                if character == b'*' && next == Some(b'/') {
                    state = State::Code;
                    index += 1;
                }
            }
        }

        index += 1;
    }

    bail!("classic `for` header did not contain a closing parenthesis")
}
