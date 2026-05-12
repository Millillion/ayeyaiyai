use super::super::*;
use super::{
    bindings::{
        collect_pattern_binding_names_including_duplicates, collect_using_decl_bound_names,
        collect_var_decl_bound_names,
    },
    blocks::{
        collect_var_declared_names_from_statement, validate_block_statement_early_errors,
        validate_classic_for_header,
    },
    declarations::{
        BindingRestrictions, is_await_like_identifier, is_yield_like_identifier,
        validate_declaration_syntax, validate_for_head_syntax_with_restrictions,
        validate_pattern_syntax_with_restrictions,
        validate_using_declaration_syntax_with_restrictions,
        validate_variable_declaration_syntax_with_restrictions,
    },
    expressions::validate_expression_syntax_with_restrictions,
};

pub(crate) fn validate_statement_syntax(
    statement: &Stmt,
    file: &swc_common::SourceFile,
) -> Result<()> {
    validate_statement_syntax_with_restrictions(statement, file, BindingRestrictions::default())?;
    validate_statement_control_flow(statement)
}

pub(super) fn validate_statement_syntax_with_restrictions(
    statement: &Stmt,
    file: &swc_common::SourceFile,
    restrictions: BindingRestrictions,
) -> Result<()> {
    match statement {
        Stmt::Block(block) => {
            validate_block_statement_early_errors(&block.stmts)?;
            for statement in &block.stmts {
                validate_statement_syntax_with_restrictions(statement, file, restrictions)?;
            }
        }
        Stmt::Decl(declaration) => match declaration {
            Decl::Var(variable_declaration) => {
                validate_variable_declaration_syntax_with_restrictions(
                    variable_declaration,
                    file,
                    restrictions,
                )?
            }
            Decl::Fn(function_declaration) => {
                ensure!(
                    !(restrictions.await_reserved
                        && is_await_like_identifier(function_declaration.ident.sym.as_ref())),
                    "`await` cannot be used as a binding identifier in an async function"
                );
                ensure!(
                    !(restrictions.yield_reserved
                        && is_yield_like_identifier(function_declaration.ident.sym.as_ref())),
                    "`yield` cannot be used as a binding identifier in a generator function"
                );
                validate_declaration_syntax(declaration, file)?
            }
            Decl::Class(class_declaration) => {
                ensure!(
                    !(restrictions.await_reserved
                        && is_await_like_identifier(class_declaration.ident.sym.as_ref())),
                    "`await` cannot be used as a binding identifier in an async function"
                );
                ensure!(
                    !(restrictions.yield_reserved
                        && is_yield_like_identifier(class_declaration.ident.sym.as_ref())),
                    "`yield` cannot be used as a binding identifier in a generator function"
                );
                validate_declaration_syntax(declaration, file)?
            }
            Decl::Using(using_declaration) => validate_using_declaration_syntax_with_restrictions(
                using_declaration,
                file,
                restrictions,
            )?,
            _ => validate_declaration_syntax(declaration, file)?,
        },
        Stmt::Expr(expression) => {
            validate_expression_statement_lookahead(expression, file)?;
            validate_expression_syntax_with_restrictions(&expression.expr, file, restrictions)?
        }
        Stmt::If(statement) => {
            validate_if_branch_statement_position(&statement.cons)?;
            validate_expression_syntax_with_restrictions(&statement.test, file, restrictions)?;
            validate_statement_syntax_with_restrictions(&statement.cons, file, restrictions)?;
            if let Some(alternate) = &statement.alt {
                validate_if_branch_statement_position(alternate)?;
                validate_statement_syntax_with_restrictions(alternate, file, restrictions)?;
            }
        }
        Stmt::While(statement) => {
            validate_iteration_body_statement_position(&statement.body)?;
            validate_expression_syntax_with_restrictions(&statement.test, file, restrictions)?;
            validate_statement_syntax_with_restrictions(&statement.body, file, restrictions)?;
        }
        Stmt::DoWhile(statement) => {
            validate_iteration_body_statement_position(&statement.body)?;
            validate_statement_syntax_with_restrictions(&statement.body, file, restrictions)?;
            validate_expression_syntax_with_restrictions(&statement.test, file, restrictions)?;
        }
        Stmt::For(statement) => {
            validate_classic_for_header(statement, file)?;
            if let Some(init) = &statement.init {
                match init {
                    VarDeclOrExpr::VarDecl(variable_declaration) => {
                        validate_variable_declaration_syntax_with_restrictions(
                            variable_declaration,
                            file,
                            restrictions,
                        )?;
                    }
                    VarDeclOrExpr::Expr(expression) => {
                        validate_expression_syntax_with_restrictions(
                            expression,
                            file,
                            restrictions,
                        )?
                    }
                }
            }
            if let Some(test) = &statement.test {
                validate_expression_syntax_with_restrictions(test, file, restrictions)?;
            }
            if let Some(update) = &statement.update {
                validate_expression_syntax_with_restrictions(update, file, restrictions)?;
            }
            validate_iteration_body_statement_position(&statement.body)?;
            validate_statement_syntax_with_restrictions(&statement.body, file, restrictions)?;
            validate_classic_for_lexical_head_does_not_overlap_body_var_names(statement)?;
        }
        Stmt::ForIn(statement) => {
            validate_for_head_syntax_with_restrictions(&statement.left, file, restrictions)?;
            validate_expression_syntax_with_restrictions(&statement.right, file, restrictions)?;
            validate_iteration_body_statement_position(&statement.body)?;
            validate_statement_syntax_with_restrictions(&statement.body, file, restrictions)?;
            validate_for_in_of_lexical_head_does_not_overlap_body_var_names(
                &statement.left,
                &statement.body,
            )?;
        }
        Stmt::ForOf(statement) => {
            validate_for_of_separator_token(statement, file)?;
            validate_for_head_syntax_with_restrictions(&statement.left, file, restrictions)?;
            validate_expression_syntax_with_restrictions(&statement.right, file, restrictions)?;
            validate_iteration_body_statement_position(&statement.body)?;
            validate_statement_syntax_with_restrictions(&statement.body, file, restrictions)?;
            validate_for_in_of_lexical_head_does_not_overlap_body_var_names(
                &statement.left,
                &statement.body,
            )?;
        }
        Stmt::Switch(statement) => {
            validate_switch_case_block_early_errors(statement)?;
            validate_expression_syntax_with_restrictions(
                &statement.discriminant,
                file,
                restrictions,
            )?;
            for case in &statement.cases {
                if let Some(test) = &case.test {
                    validate_expression_syntax_with_restrictions(test, file, restrictions)?;
                }
                for statement in &case.cons {
                    validate_statement_syntax_with_restrictions(statement, file, restrictions)?;
                }
            }
        }
        Stmt::Try(statement) => {
            validate_block_statement_early_errors(&statement.block.stmts)?;
            for statement in &statement.block.stmts {
                validate_statement_syntax_with_restrictions(statement, file, restrictions)?;
            }
            if let Some(handler) = &statement.handler {
                validate_catch_clause_early_errors(handler)?;
                validate_block_statement_early_errors(&handler.body.stmts)?;
                if let Some(pattern) = &handler.param {
                    validate_pattern_syntax_with_restrictions(pattern, file, restrictions)?;
                }
                for statement in &handler.body.stmts {
                    validate_statement_syntax_with_restrictions(statement, file, restrictions)?;
                }
            }
            if let Some(finalizer) = &statement.finalizer {
                validate_block_statement_early_errors(&finalizer.stmts)?;
                for statement in &finalizer.stmts {
                    validate_statement_syntax_with_restrictions(statement, file, restrictions)?;
                }
            }
        }
        Stmt::With(statement) => {
            validate_with_body_statement_position(&statement.body)?;
            validate_expression_syntax_with_restrictions(&statement.obj, file, restrictions)?;
            validate_statement_syntax_with_restrictions(&statement.body, file, restrictions)?;
        }
        Stmt::Return(statement) => {
            if let Some(argument) = &statement.arg {
                validate_expression_syntax_with_restrictions(argument, file, restrictions)?;
            }
        }
        Stmt::Throw(statement) => {
            validate_expression_syntax_with_restrictions(&statement.arg, file, restrictions)?
        }
        Stmt::Labeled(statement) => {
            ensure!(
                !(restrictions.await_reserved
                    && is_await_like_identifier(statement.label.sym.as_ref())),
                "`await` cannot be used as a label in an async function"
            );
            ensure!(
                !(restrictions.yield_reserved
                    && is_yield_like_identifier(statement.label.sym.as_ref())),
                "`yield` cannot be used as a label in a generator function"
            );
            validate_labeled_body_statement_position(&statement.body)?;
            validate_statement_syntax_with_restrictions(&statement.body, file, restrictions)?
        }
        _ => {}
    }

    Ok(())
}

fn validate_catch_clause_early_errors(handler: &CatchClause) -> Result<()> {
    let Some(pattern) = &handler.param else {
        return Ok(());
    };

    let mut bound_names = Vec::new();
    collect_pattern_binding_names_including_duplicates(pattern, &mut bound_names)?;

    let mut seen = HashSet::new();
    for name in &bound_names {
        ensure!(
            seen.insert(name.clone()),
            "duplicate catch binding `{name}`"
        );
    }

    let lexical_names = collect_direct_catch_block_lexically_declared_names(&handler.body.stmts)?;
    let lexical_names = lexical_names.into_iter().collect::<HashSet<_>>();
    for name in bound_names {
        ensure!(
            !lexical_names.contains(&name),
            "duplicate lexical name `{name}`"
        );
    }

    Ok(())
}

fn collect_direct_catch_block_lexically_declared_names(statements: &[Stmt]) -> Result<Vec<String>> {
    let mut names = Vec::new();

    for statement in statements {
        match statement {
            Stmt::Decl(Decl::Var(variable_declaration))
                if !matches!(variable_declaration.kind, VarDeclKind::Var) =>
            {
                names.extend(collect_var_decl_bound_names(variable_declaration)?);
            }
            Stmt::Decl(Decl::Using(using_declaration)) => {
                names.extend(collect_using_decl_bound_names(using_declaration)?);
            }
            Stmt::Decl(Decl::Fn(function_declaration)) => {
                names.push(function_declaration.ident.sym.to_string());
            }
            Stmt::Decl(Decl::Class(class_declaration)) => {
                names.push(class_declaration.ident.sym.to_string());
            }
            _ => {}
        }
    }

    Ok(names)
}

fn validate_switch_case_block_early_errors(statement: &SwitchStmt) -> Result<()> {
    let lexical_names = collect_switch_case_block_lexically_declared_names(statement)?;
    let mut seen = HashSet::new();

    for name in &lexical_names {
        ensure!(seen.insert(name.clone()), "duplicate lexical name `{name}`");
    }

    let mut var_names = HashSet::new();
    for case in &statement.cases {
        for statement in &case.cons {
            ensure!(
                !matches!(statement, Stmt::Decl(Decl::Using(_))),
                "using declaration is not allowed directly in switch case statement list"
            );
            var_names.extend(collect_var_declared_names_from_statement(statement, false)?);
        }
    }

    for name in lexical_names {
        ensure!(
            !var_names.contains(&name),
            "duplicate lexical name `{name}`"
        );
    }

    Ok(())
}

fn collect_switch_case_block_lexically_declared_names(
    statement: &SwitchStmt,
) -> Result<Vec<String>> {
    let mut names = Vec::new();

    for case in &statement.cases {
        for statement in &case.cons {
            match statement {
                Stmt::Decl(Decl::Var(variable_declaration))
                    if !matches!(variable_declaration.kind, VarDeclKind::Var) =>
                {
                    names.extend(collect_var_decl_bound_names(variable_declaration)?);
                }
                Stmt::Decl(Decl::Using(using_declaration)) => {
                    names.extend(collect_using_decl_bound_names(using_declaration)?);
                }
                Stmt::Decl(Decl::Fn(function_declaration)) => {
                    names.push(function_declaration.ident.sym.to_string());
                }
                Stmt::Decl(Decl::Class(class_declaration)) => {
                    names.push(class_declaration.ident.sym.to_string());
                }
                _ => {}
            }
        }
    }

    Ok(names)
}

fn validate_for_in_of_lexical_head_does_not_overlap_body_var_names(
    head: &ForHead,
    body: &Stmt,
) -> Result<()> {
    let lexical_names = match head {
        ForHead::VarDecl(variable_declaration)
            if !matches!(variable_declaration.kind, VarDeclKind::Var) =>
        {
            collect_var_decl_bound_names(variable_declaration)?
        }
        ForHead::UsingDecl(using_declaration) => collect_using_decl_bound_names(using_declaration)?,
        _ => return Ok(()),
    };
    let var_names = collect_var_declared_names_from_statement(body, true)?
        .into_iter()
        .collect::<HashSet<_>>();
    for name in lexical_names {
        ensure!(
            !var_names.contains(&name),
            "duplicate lexical name `{name}`"
        );
    }

    Ok(())
}

fn validate_classic_for_lexical_head_does_not_overlap_body_var_names(
    statement: &swc_ecma_ast::ForStmt,
) -> Result<()> {
    let Some(VarDeclOrExpr::VarDecl(variable_declaration)) = &statement.init else {
        return Ok(());
    };
    if matches!(variable_declaration.kind, VarDeclKind::Var) {
        return Ok(());
    }

    let var_names = collect_var_declared_names_from_statement(&statement.body, true)?
        .into_iter()
        .collect::<HashSet<_>>();
    for name in collect_var_decl_bound_names(variable_declaration)? {
        ensure!(
            !var_names.contains(&name),
            "duplicate lexical name `{name}`"
        );
    }

    Ok(())
}

#[derive(Clone, Default)]
struct ControlFlowContext {
    iteration_depth: usize,
    continue_labels: Vec<String>,
}

fn is_iteration_statement(statement: &Stmt) -> bool {
    matches!(
        statement,
        Stmt::While(_) | Stmt::DoWhile(_) | Stmt::For(_) | Stmt::ForIn(_) | Stmt::ForOf(_)
    )
}

fn validate_expression_statement_lookahead(
    statement: &ExprStmt,
    file: &swc_common::SourceFile,
) -> Result<()> {
    let raw = source_slice_for_span(file, statement.span)?;
    let bytes = raw.as_bytes();
    let start = skip_js_trivia(bytes, 0);
    if !bytes[start..].starts_with(b"let") {
        return Ok(());
    }
    let after_let = start + 3;
    if bytes
        .get(after_let)
        .is_some_and(|byte| is_ascii_identifier_continue(*byte))
    {
        return Ok(());
    }
    let after_trivia = skip_js_trivia(bytes, after_let);
    ensure!(
        !bytes.get(after_trivia).is_some_and(|byte| *byte == b'['),
        "expression statement cannot start with `let [`"
    );
    Ok(())
}

fn skip_js_trivia(bytes: &[u8], mut index: usize) -> usize {
    loop {
        while bytes
            .get(index)
            .is_some_and(|byte| matches!(*byte, b'\t' | b'\n' | b'\x0b' | b'\x0c' | b'\r' | b' '))
        {
            index += 1;
        }

        if bytes.get(index..index + 2) == Some(b"//") {
            index += 2;
            while bytes
                .get(index)
                .is_some_and(|byte| !matches!(*byte, b'\n' | b'\r'))
            {
                index += 1;
            }
            continue;
        }

        if bytes.get(index..index + 2) == Some(b"/*") {
            index += 2;
            while index + 1 < bytes.len() && bytes.get(index..index + 2) != Some(b"*/") {
                index += 1;
            }
            index = (index + 2).min(bytes.len());
            continue;
        }

        return index;
    }
}

fn is_ascii_identifier_continue(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'$')
}

fn validate_for_of_separator_token(
    statement: &ForOfStmt,
    file: &swc_common::SourceFile,
) -> Result<()> {
    let raw_statement = source_slice_for_span(file, statement.span)?;
    let start = for_head_end(&statement.left)
        .to_usize()
        .saturating_sub(statement.span.lo.to_usize());
    let bytes = raw_statement.as_bytes();
    let token_start = skip_js_trivia(bytes, start.min(bytes.len()));
    let token_end = identifier_token_end(bytes, token_start);
    let separator = raw_statement
        .get(token_start..token_end)
        .unwrap_or_default();

    ensure!(
        separator == "of",
        "the `of` contextual keyword in a for-of statement cannot contain escapes"
    );

    Ok(())
}

fn for_head_end(head: &ForHead) -> swc_common::BytePos {
    match head {
        ForHead::VarDecl(declaration) => declaration.span.hi,
        ForHead::UsingDecl(declaration) => declaration.span.hi,
        ForHead::Pat(pattern) => pattern_end(pattern),
    }
}

fn pattern_end(pattern: &Pat) -> swc_common::BytePos {
    match pattern {
        Pat::Ident(identifier) => identifier.id.span.hi,
        Pat::Array(pattern) => pattern.span.hi,
        Pat::Rest(pattern) => pattern.span.hi,
        Pat::Object(pattern) => pattern.span.hi,
        Pat::Assign(pattern) => pattern.span.hi,
        Pat::Invalid(pattern) => pattern.span.hi,
        Pat::Expr(expression) => expression_end(expression),
    }
}

fn expression_end(expression: &Expr) -> swc_common::BytePos {
    match expression {
        Expr::This(expression) => expression.span.hi,
        Expr::Array(expression) => expression.span.hi,
        Expr::Object(expression) => expression.span.hi,
        Expr::Fn(expression) => expression.function.span.hi,
        Expr::Unary(expression) => expression.span.hi,
        Expr::Update(expression) => expression.span.hi,
        Expr::Bin(expression) => expression.span.hi,
        Expr::Assign(expression) => expression.span.hi,
        Expr::Member(expression) => expression.span.hi,
        Expr::SuperProp(expression) => expression.span.hi,
        Expr::Cond(expression) => expression.span.hi,
        Expr::Call(expression) => expression.span.hi,
        Expr::New(expression) => expression.span.hi,
        Expr::Seq(expression) => expression.span.hi,
        Expr::Ident(expression) => expression.span.hi,
        Expr::Lit(expression) => literal_end(expression),
        Expr::Tpl(expression) => expression.span.hi,
        Expr::TaggedTpl(expression) => expression.span.hi,
        Expr::Arrow(expression) => expression.span.hi,
        Expr::Class(expression) => expression.class.span.hi,
        Expr::Yield(expression) => expression.span.hi,
        Expr::MetaProp(expression) => expression.span.hi,
        Expr::Await(expression) => expression.span.hi,
        Expr::Paren(expression) => expression.span.hi,
        Expr::JSXMember(expression) => expression.span.hi,
        Expr::JSXNamespacedName(expression) => expression.span.hi,
        Expr::JSXEmpty(expression) => expression.span.hi,
        Expr::JSXElement(expression) => expression.span.hi,
        Expr::JSXFragment(expression) => expression.span.hi,
        Expr::TsTypeAssertion(expression) => expression.span.hi,
        Expr::TsConstAssertion(expression) => expression.span.hi,
        Expr::TsNonNull(expression) => expression.span.hi,
        Expr::TsAs(expression) => expression.span.hi,
        Expr::TsInstantiation(expression) => expression.span.hi,
        Expr::TsSatisfies(expression) => expression.span.hi,
        Expr::PrivateName(expression) => expression.span.hi,
        Expr::OptChain(expression) => expression.span.hi,
        Expr::Invalid(expression) => expression.span.hi,
    }
}

fn literal_end(literal: &Lit) -> swc_common::BytePos {
    match literal {
        Lit::Str(literal) => literal.span.hi,
        Lit::Bool(literal) => literal.span.hi,
        Lit::Null(literal) => literal.span.hi,
        Lit::Num(literal) => literal.span.hi,
        Lit::BigInt(literal) => literal.span.hi,
        Lit::Regex(literal) => literal.span.hi,
        Lit::JSXText(literal) => literal.span.hi,
    }
}

fn identifier_token_end(bytes: &[u8], mut index: usize) -> usize {
    while bytes.get(index).is_some_and(|byte| {
        byte.is_ascii_alphanumeric() || matches!(*byte, b'_' | b'$' | b'\\' | b'{' | b'}')
    }) {
        index += 1;
    }
    index
}

fn validate_iteration_body_statement_position(statement: &Stmt) -> Result<()> {
    match statement {
        Stmt::Decl(Decl::Fn(_) | Decl::Class(_)) => {
            bail!("declaration is not allowed directly in iteration statement position")
        }
        Stmt::Decl(Decl::Var(variable_declaration))
            if !matches!(variable_declaration.kind, VarDeclKind::Var) =>
        {
            bail!("lexical declaration is not allowed directly in iteration statement position")
        }
        _ if is_labelled_function_statement(statement) => {
            bail!("labelled function is not allowed directly in iteration statement position")
        }
        _ => Ok(()),
    }
}

fn validate_with_body_statement_position(statement: &Stmt) -> Result<()> {
    match statement {
        Stmt::Decl(Decl::Fn(_) | Decl::Class(_)) => {
            bail!("declaration is not allowed directly in with statement position")
        }
        Stmt::Decl(Decl::Var(variable_declaration))
            if !matches!(variable_declaration.kind, VarDeclKind::Var) =>
        {
            bail!("lexical declaration is not allowed directly in with statement position")
        }
        _ if is_labelled_function_statement(statement) => {
            bail!("labelled function is not allowed directly in with statement position")
        }
        _ => Ok(()),
    }
}

fn validate_if_branch_statement_position(statement: &Stmt) -> Result<()> {
    match statement {
        Stmt::Decl(Decl::Fn(function_declaration))
            if function_declaration.function.is_async
                || function_declaration.function.is_generator =>
        {
            bail!(
                "async and generator declarations are not allowed directly in if statement position"
            )
        }
        Stmt::Decl(Decl::Class(_)) => {
            bail!("class declaration is not allowed directly in if statement position")
        }
        Stmt::Decl(Decl::Var(variable_declaration))
            if !matches!(variable_declaration.kind, VarDeclKind::Var) =>
        {
            bail!("lexical declaration is not allowed directly in if statement position")
        }
        _ if is_labelled_function_statement(statement) => {
            bail!("labelled function is not allowed directly in if statement position")
        }
        _ => Ok(()),
    }
}

fn validate_labeled_body_statement_position(statement: &Stmt) -> Result<()> {
    match statement {
        Stmt::Decl(Decl::Fn(function_declaration))
            if function_declaration.function.is_async
                || function_declaration.function.is_generator =>
        {
            bail!("async and generator declarations are not allowed as labeled statement bodies")
        }
        Stmt::Decl(Decl::Class(_)) => {
            bail!("class declaration is not allowed as a labeled statement body")
        }
        Stmt::Decl(Decl::Var(variable_declaration))
            if !matches!(variable_declaration.kind, VarDeclKind::Var) =>
        {
            bail!("lexical declaration is not allowed as a labeled statement body")
        }
        Stmt::Labeled(labelled) => validate_labeled_body_statement_position(&labelled.body),
        _ => Ok(()),
    }
}

fn is_labelled_function_statement(statement: &Stmt) -> bool {
    match statement {
        Stmt::Labeled(labelled) => {
            matches!(&*labelled.body, Stmt::Decl(Decl::Fn(_)))
                || is_labelled_function_statement(&labelled.body)
        }
        _ => false,
    }
}

pub(super) fn validate_static_block_control_flow(statements: &[Stmt]) -> Result<()> {
    let context = ControlFlowContext::default();
    for statement in statements {
        validate_statement_control_flow_with_context(statement, &context)?;
    }
    Ok(())
}

fn validate_statement_control_flow(statement: &Stmt) -> Result<()> {
    validate_statement_control_flow_with_context(statement, &ControlFlowContext::default())
}

fn validate_statement_control_flow_with_context(
    statement: &Stmt,
    context: &ControlFlowContext,
) -> Result<()> {
    match statement {
        Stmt::Block(block) => {
            for statement in &block.stmts {
                validate_statement_control_flow_with_context(statement, context)?;
            }
        }
        Stmt::If(statement) => {
            validate_statement_control_flow_with_context(&statement.cons, context)?;
            if let Some(alternate) = &statement.alt {
                validate_statement_control_flow_with_context(alternate, context)?;
            }
        }
        Stmt::While(statement) => {
            let mut loop_context = context.clone();
            loop_context.iteration_depth += 1;
            validate_statement_control_flow_with_context(&statement.body, &loop_context)?;
        }
        Stmt::DoWhile(statement) => {
            let mut loop_context = context.clone();
            loop_context.iteration_depth += 1;
            validate_statement_control_flow_with_context(&statement.body, &loop_context)?;
        }
        Stmt::For(statement) => {
            let mut loop_context = context.clone();
            loop_context.iteration_depth += 1;
            validate_statement_control_flow_with_context(&statement.body, &loop_context)?;
        }
        Stmt::ForIn(statement) => {
            let mut loop_context = context.clone();
            loop_context.iteration_depth += 1;
            validate_statement_control_flow_with_context(&statement.body, &loop_context)?;
        }
        Stmt::ForOf(statement) => {
            let mut loop_context = context.clone();
            loop_context.iteration_depth += 1;
            validate_statement_control_flow_with_context(&statement.body, &loop_context)?;
        }
        Stmt::Switch(statement) => {
            for case in &statement.cases {
                for statement in &case.cons {
                    validate_statement_control_flow_with_context(statement, context)?;
                }
            }
        }
        Stmt::Try(statement) => {
            for statement in &statement.block.stmts {
                validate_statement_control_flow_with_context(statement, context)?;
            }
            if let Some(handler) = &statement.handler {
                for statement in &handler.body.stmts {
                    validate_statement_control_flow_with_context(statement, context)?;
                }
            }
            if let Some(finalizer) = &statement.finalizer {
                for statement in &finalizer.stmts {
                    validate_statement_control_flow_with_context(statement, context)?;
                }
            }
        }
        Stmt::With(statement) => {
            validate_statement_control_flow_with_context(&statement.body, context)?;
        }
        Stmt::Labeled(statement) => {
            let mut labeled_context = context.clone();
            if is_iteration_statement(&statement.body) {
                labeled_context
                    .continue_labels
                    .push(statement.label.sym.to_string());
            }
            validate_statement_control_flow_with_context(&statement.body, &labeled_context)?;
        }
        Stmt::Continue(statement) => {
            if let Some(label) = &statement.label {
                ensure!(
                    context
                        .continue_labels
                        .iter()
                        .any(|candidate| candidate == label.sym.as_ref()),
                    "continue label does not target an iteration statement"
                );
            } else {
                ensure!(
                    context.iteration_depth > 0,
                    "continue is not nested within an iteration statement"
                );
            }
        }
        Stmt::Decl(Decl::Class(class_declaration)) => {
            validate_class_control_flow(&class_declaration.class)?;
        }
        _ => {}
    }

    Ok(())
}

pub(super) fn validate_class_control_flow(class: &Class) -> Result<()> {
    for member in &class.body {
        if let ClassMember::StaticBlock(block) = member {
            validate_static_block_control_flow(&block.body.stmts)?;
        }
    }
    Ok(())
}
