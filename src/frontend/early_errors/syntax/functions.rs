use super::super::*;
use super::{
    bindings::{
        collect_pattern_binding_names, collect_pattern_binding_names_including_duplicates,
        collect_using_decl_bound_names, collect_var_decl_bound_names,
    },
    blocks::validate_block_statement_early_errors,
    declarations::{
        BindingRestrictions, validate_escaped_identifier_text,
        validate_pattern_syntax_with_restrictions,
    },
    expressions::validate_expression_syntax,
    statements::{
        validate_class_control_flow, validate_statement_syntax,
        validate_statement_syntax_with_restrictions,
    },
};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, PartialEq, Eq)]
enum PrivateNameDeclarationKind {
    Value,
    Getter,
    Setter,
    AccessorPair,
}

#[derive(Clone, Copy)]
struct PrivateNameDeclaration {
    kind: PrivateNameDeclarationKind,
    is_static: bool,
}

fn record_private_name_declaration(
    declarations: &mut HashMap<String, PrivateNameDeclaration>,
    name: &str,
    next_kind: PrivateNameDeclarationKind,
    next_is_static: bool,
) -> Result<()> {
    let updated = match declarations.get(name).copied() {
        None => PrivateNameDeclaration {
            kind: next_kind,
            is_static: next_is_static,
        },
        Some(PrivateNameDeclaration {
            kind: PrivateNameDeclarationKind::Getter,
            is_static,
        }) if is_static == next_is_static && next_kind == PrivateNameDeclarationKind::Setter => {
            PrivateNameDeclaration {
                kind: PrivateNameDeclarationKind::AccessorPair,
                is_static,
            }
        }
        Some(PrivateNameDeclaration {
            kind: PrivateNameDeclarationKind::Setter,
            is_static,
        }) if is_static == next_is_static && next_kind == PrivateNameDeclarationKind::Getter => {
            PrivateNameDeclaration {
                kind: PrivateNameDeclarationKind::AccessorPair,
                is_static,
            }
        }
        _ => bail!("duplicate private name `#{name}` in class body"),
    };
    declarations.insert(name.to_string(), updated);
    Ok(())
}

fn collect_class_private_name_declarations(
    class: &Class,
    file: &swc_common::SourceFile,
) -> Result<HashMap<String, PrivateNameDeclaration>> {
    let mut declarations = HashMap::new();
    for member in &class.body {
        match member {
            ClassMember::PrivateMethod(method) => {
                validate_private_name_syntax(&method.key, file)?;
                record_private_name_declaration(
                    &mut declarations,
                    method.key.name.as_ref(),
                    match method.kind {
                        MethodKind::Getter => PrivateNameDeclarationKind::Getter,
                        MethodKind::Setter => PrivateNameDeclarationKind::Setter,
                        _ => PrivateNameDeclarationKind::Value,
                    },
                    method.is_static,
                )?;
            }
            ClassMember::PrivateProp(property) => {
                validate_private_name_syntax(&property.key, file)?;
                record_private_name_declaration(
                    &mut declarations,
                    property.key.name.as_ref(),
                    PrivateNameDeclarationKind::Value,
                    property.is_static,
                )?;
            }
            ClassMember::AutoAccessor(accessor) => {
                if let Key::Private(private_name) = &accessor.key {
                    validate_private_name_syntax(private_name, file)?;
                    record_private_name_declaration(
                        &mut declarations,
                        private_name.name.as_ref(),
                        PrivateNameDeclarationKind::Value,
                        accessor.is_static,
                    )?;
                }
            }
            _ => {}
        }
    }
    Ok(declarations)
}

fn class_private_name_set(class: &Class) -> HashSet<String> {
    class
        .body
        .iter()
        .filter_map(|member| match member {
            ClassMember::PrivateMethod(method) => Some(method.key.name.to_string()),
            ClassMember::PrivateProp(property) => Some(property.key.name.to_string()),
            ClassMember::AutoAccessor(accessor) => match &accessor.key {
                Key::Private(private_name) => Some(private_name.name.to_string()),
                Key::Public(_) => None,
            },
            _ => None,
        })
        .collect()
}

fn validate_no_forbidden_private_name(
    private_name: &PrivateName,
    forbidden: &HashSet<String>,
) -> Result<()> {
    ensure!(
        !forbidden.contains(private_name.name.as_ref()),
        "private name `#{}` is not in scope while evaluating class heritage",
        private_name.name
    );
    Ok(())
}

fn validate_no_forbidden_private_names_in_member(
    member: &MemberExpr,
    forbidden: &HashSet<String>,
) -> Result<()> {
    validate_no_forbidden_private_names_in_expression(&member.obj, forbidden)?;
    match &member.prop {
        MemberProp::Computed(property) => {
            validate_no_forbidden_private_names_in_expression(&property.expr, forbidden)?;
        }
        MemberProp::PrivateName(private_name) => {
            validate_no_forbidden_private_name(private_name, forbidden)?;
        }
        MemberProp::Ident(_) => {}
    }
    Ok(())
}

fn validate_no_forbidden_private_names_in_property_name(
    name: &PropName,
    forbidden: &HashSet<String>,
) -> Result<()> {
    if let PropName::Computed(computed) = name {
        validate_no_forbidden_private_names_in_expression(&computed.expr, forbidden)?;
    }
    Ok(())
}

fn validate_no_forbidden_private_names_in_pattern(
    pattern: &Pat,
    forbidden: &HashSet<String>,
) -> Result<()> {
    match pattern {
        Pat::Assign(assign) => {
            validate_no_forbidden_private_names_in_pattern(&assign.left, forbidden)?;
            validate_no_forbidden_private_names_in_expression(&assign.right, forbidden)?;
        }
        Pat::Array(array) => {
            for element in array.elems.iter().flatten() {
                validate_no_forbidden_private_names_in_pattern(element, forbidden)?;
            }
        }
        Pat::Object(object) => {
            for property in &object.props {
                match property {
                    ObjectPatProp::KeyValue(property) => {
                        validate_no_forbidden_private_names_in_pattern(&property.value, forbidden)?;
                    }
                    ObjectPatProp::Assign(property) => {
                        if let Some(value) = &property.value {
                            validate_no_forbidden_private_names_in_expression(value, forbidden)?;
                        }
                    }
                    ObjectPatProp::Rest(rest) => {
                        validate_no_forbidden_private_names_in_pattern(&rest.arg, forbidden)?;
                    }
                }
            }
        }
        Pat::Rest(rest) => validate_no_forbidden_private_names_in_pattern(&rest.arg, forbidden)?,
        _ => {}
    }
    Ok(())
}

fn validate_no_forbidden_private_names_in_variable_declaration(
    declaration: &VarDecl,
    forbidden: &HashSet<String>,
) -> Result<()> {
    for declarator in &declaration.decls {
        validate_no_forbidden_private_names_in_pattern(&declarator.name, forbidden)?;
        if let Some(initializer) = &declarator.init {
            validate_no_forbidden_private_names_in_expression(initializer, forbidden)?;
        }
    }
    Ok(())
}

fn validate_no_forbidden_private_names_in_function(
    function: &Function,
    forbidden: &HashSet<String>,
) -> Result<()> {
    for parameter in &function.params {
        validate_no_forbidden_private_names_in_pattern(&parameter.pat, forbidden)?;
    }
    if let Some(body) = &function.body {
        for statement in &body.stmts {
            validate_no_forbidden_private_names_in_statement(statement, forbidden)?;
        }
    }
    Ok(())
}

fn validate_no_forbidden_private_names_in_constructor(
    constructor: &Constructor,
    forbidden: &HashSet<String>,
) -> Result<()> {
    if let Some(body) = &constructor.body {
        for statement in &body.stmts {
            validate_no_forbidden_private_names_in_statement(statement, forbidden)?;
        }
    }
    Ok(())
}

fn validate_no_forbidden_private_names_in_class(
    class: &Class,
    forbidden: &HashSet<String>,
) -> Result<()> {
    let class_private_names = class_private_name_set(class);

    if let Some(super_class) = &class.super_class {
        let mut heritage_forbidden = forbidden.clone();
        heritage_forbidden.extend(class_private_names.iter().cloned());
        validate_no_forbidden_private_names_in_expression(super_class, &heritage_forbidden)?;
    }

    let mut body_forbidden = forbidden.clone();
    for name in &class_private_names {
        body_forbidden.remove(name);
    }

    for member in &class.body {
        match member {
            ClassMember::Constructor(constructor) => {
                validate_no_forbidden_private_names_in_constructor(constructor, &body_forbidden)?;
            }
            ClassMember::Method(method) => {
                validate_no_forbidden_private_names_in_property_name(&method.key, &body_forbidden)?;
                validate_no_forbidden_private_names_in_function(&method.function, &body_forbidden)?;
            }
            ClassMember::ClassProp(property) => {
                validate_no_forbidden_private_names_in_property_name(
                    &property.key,
                    &body_forbidden,
                )?;
                if let Some(value) = &property.value {
                    validate_no_forbidden_private_names_in_expression(value, &body_forbidden)?;
                }
            }
            ClassMember::PrivateMethod(method) => {
                validate_no_forbidden_private_names_in_function(&method.function, &body_forbidden)?;
            }
            ClassMember::PrivateProp(property) => {
                if let Some(value) = &property.value {
                    validate_no_forbidden_private_names_in_expression(value, &body_forbidden)?;
                }
            }
            ClassMember::AutoAccessor(accessor) => {
                if let Key::Public(property_name) = &accessor.key {
                    validate_no_forbidden_private_names_in_property_name(
                        property_name,
                        &body_forbidden,
                    )?;
                }
                if let Some(value) = &accessor.value {
                    validate_no_forbidden_private_names_in_expression(value, &body_forbidden)?;
                }
            }
            ClassMember::StaticBlock(block) => {
                for statement in &block.body.stmts {
                    validate_no_forbidden_private_names_in_statement(statement, &body_forbidden)?;
                }
            }
            _ => {}
        }
    }

    validate_class_control_flow(class)?;
    Ok(())
}

fn validate_no_forbidden_private_names_in_declaration(
    declaration: &Decl,
    forbidden: &HashSet<String>,
) -> Result<()> {
    match declaration {
        Decl::Class(class) => validate_no_forbidden_private_names_in_class(&class.class, forbidden),
        Decl::Fn(function) => {
            validate_no_forbidden_private_names_in_function(&function.function, forbidden)
        }
        Decl::Var(variable_declaration) => {
            validate_no_forbidden_private_names_in_variable_declaration(
                variable_declaration,
                forbidden,
            )
        }
        _ => Ok(()),
    }
}

fn validate_no_forbidden_private_names_in_statement(
    statement: &Stmt,
    forbidden: &HashSet<String>,
) -> Result<()> {
    match statement {
        Stmt::Block(block) => {
            for statement in &block.stmts {
                validate_no_forbidden_private_names_in_statement(statement, forbidden)?;
            }
        }
        Stmt::Decl(declaration) => {
            validate_no_forbidden_private_names_in_declaration(declaration, forbidden)?;
        }
        Stmt::Expr(expression) => {
            validate_no_forbidden_private_names_in_expression(&expression.expr, forbidden)?;
        }
        Stmt::If(statement) => {
            validate_no_forbidden_private_names_in_expression(&statement.test, forbidden)?;
            validate_no_forbidden_private_names_in_statement(&statement.cons, forbidden)?;
            if let Some(alternate) = &statement.alt {
                validate_no_forbidden_private_names_in_statement(alternate, forbidden)?;
            }
        }
        Stmt::While(statement) => {
            validate_no_forbidden_private_names_in_expression(&statement.test, forbidden)?;
            validate_no_forbidden_private_names_in_statement(&statement.body, forbidden)?;
        }
        Stmt::DoWhile(statement) => {
            validate_no_forbidden_private_names_in_statement(&statement.body, forbidden)?;
            validate_no_forbidden_private_names_in_expression(&statement.test, forbidden)?;
        }
        Stmt::For(statement) => {
            if let Some(init) = &statement.init {
                match init {
                    VarDeclOrExpr::VarDecl(variable_declaration) => {
                        validate_no_forbidden_private_names_in_variable_declaration(
                            variable_declaration,
                            forbidden,
                        )?;
                    }
                    VarDeclOrExpr::Expr(expression) => {
                        validate_no_forbidden_private_names_in_expression(expression, forbidden)?;
                    }
                }
            }
            if let Some(test) = &statement.test {
                validate_no_forbidden_private_names_in_expression(test, forbidden)?;
            }
            if let Some(update) = &statement.update {
                validate_no_forbidden_private_names_in_expression(update, forbidden)?;
            }
            validate_no_forbidden_private_names_in_statement(&statement.body, forbidden)?;
        }
        Stmt::ForIn(statement) => {
            match &statement.left {
                ForHead::VarDecl(variable_declaration) => {
                    validate_no_forbidden_private_names_in_variable_declaration(
                        variable_declaration,
                        forbidden,
                    )?;
                }
                ForHead::Pat(pattern) => {
                    validate_no_forbidden_private_names_in_pattern(pattern, forbidden)?;
                }
                ForHead::UsingDecl(_) => {}
            }
            validate_no_forbidden_private_names_in_expression(&statement.right, forbidden)?;
            validate_no_forbidden_private_names_in_statement(&statement.body, forbidden)?;
        }
        Stmt::ForOf(statement) => {
            match &statement.left {
                ForHead::VarDecl(variable_declaration) => {
                    validate_no_forbidden_private_names_in_variable_declaration(
                        variable_declaration,
                        forbidden,
                    )?;
                }
                ForHead::Pat(pattern) => {
                    validate_no_forbidden_private_names_in_pattern(pattern, forbidden)?;
                }
                ForHead::UsingDecl(_) => {}
            }
            validate_no_forbidden_private_names_in_expression(&statement.right, forbidden)?;
            validate_no_forbidden_private_names_in_statement(&statement.body, forbidden)?;
        }
        Stmt::Switch(statement) => {
            validate_no_forbidden_private_names_in_expression(&statement.discriminant, forbidden)?;
            for case in &statement.cases {
                if let Some(test) = &case.test {
                    validate_no_forbidden_private_names_in_expression(test, forbidden)?;
                }
                for statement in &case.cons {
                    validate_no_forbidden_private_names_in_statement(statement, forbidden)?;
                }
            }
        }
        Stmt::Try(statement) => {
            for statement in &statement.block.stmts {
                validate_no_forbidden_private_names_in_statement(statement, forbidden)?;
            }
            if let Some(handler) = &statement.handler {
                if let Some(pattern) = &handler.param {
                    validate_no_forbidden_private_names_in_pattern(pattern, forbidden)?;
                }
                for statement in &handler.body.stmts {
                    validate_no_forbidden_private_names_in_statement(statement, forbidden)?;
                }
            }
            if let Some(finalizer) = &statement.finalizer {
                for statement in &finalizer.stmts {
                    validate_no_forbidden_private_names_in_statement(statement, forbidden)?;
                }
            }
        }
        Stmt::With(statement) => {
            validate_no_forbidden_private_names_in_expression(&statement.obj, forbidden)?;
            validate_no_forbidden_private_names_in_statement(&statement.body, forbidden)?;
        }
        Stmt::Return(statement) => {
            if let Some(argument) = &statement.arg {
                validate_no_forbidden_private_names_in_expression(argument, forbidden)?;
            }
        }
        Stmt::Throw(statement) => {
            validate_no_forbidden_private_names_in_expression(&statement.arg, forbidden)?;
        }
        Stmt::Labeled(statement) => {
            validate_no_forbidden_private_names_in_statement(&statement.body, forbidden)?;
        }
        _ => {}
    }

    Ok(())
}

fn validate_no_forbidden_private_names_in_expression(
    expression: &Expr,
    forbidden: &HashSet<String>,
) -> Result<()> {
    if forbidden.is_empty() {
        return Ok(());
    }

    match expression {
        Expr::Call(call) => {
            if let Callee::Expr(callee) = &call.callee {
                validate_no_forbidden_private_names_in_expression(callee, forbidden)?;
            }
            for argument in &call.args {
                validate_no_forbidden_private_names_in_expression(&argument.expr, forbidden)?;
            }
        }
        Expr::New(new_expression) => {
            validate_no_forbidden_private_names_in_expression(&new_expression.callee, forbidden)?;
            for argument in new_expression.args.iter().flatten() {
                validate_no_forbidden_private_names_in_expression(&argument.expr, forbidden)?;
            }
        }
        Expr::Await(await_expression) => {
            validate_no_forbidden_private_names_in_expression(&await_expression.arg, forbidden)?;
        }
        Expr::Yield(yield_expression) => {
            if let Some(argument) = &yield_expression.arg {
                validate_no_forbidden_private_names_in_expression(argument, forbidden)?;
            }
        }
        Expr::Paren(parenthesized) => {
            validate_no_forbidden_private_names_in_expression(&parenthesized.expr, forbidden)?;
        }
        Expr::Array(array) => {
            for element in array.elems.iter().flatten() {
                validate_no_forbidden_private_names_in_expression(&element.expr, forbidden)?;
            }
        }
        Expr::Object(object) => {
            for property in &object.props {
                match property {
                    PropOrSpread::Spread(spread) => {
                        validate_no_forbidden_private_names_in_expression(&spread.expr, forbidden)?;
                    }
                    PropOrSpread::Prop(property) => match &**property {
                        Prop::Shorthand(_) => {}
                        Prop::KeyValue(property) => {
                            validate_no_forbidden_private_names_in_property_name(
                                &property.key,
                                forbidden,
                            )?;
                            validate_no_forbidden_private_names_in_expression(
                                &property.value,
                                forbidden,
                            )?;
                        }
                        Prop::Getter(property) => {
                            validate_no_forbidden_private_names_in_property_name(
                                &property.key,
                                forbidden,
                            )?;
                            if let Some(body) = &property.body {
                                for statement in &body.stmts {
                                    validate_no_forbidden_private_names_in_statement(
                                        statement, forbidden,
                                    )?;
                                }
                            }
                        }
                        Prop::Setter(property) => {
                            validate_no_forbidden_private_names_in_property_name(
                                &property.key,
                                forbidden,
                            )?;
                            validate_no_forbidden_private_names_in_pattern(
                                &property.param,
                                forbidden,
                            )?;
                            if let Some(body) = &property.body {
                                for statement in &body.stmts {
                                    validate_no_forbidden_private_names_in_statement(
                                        statement, forbidden,
                                    )?;
                                }
                            }
                        }
                        Prop::Method(property) => {
                            validate_no_forbidden_private_names_in_property_name(
                                &property.key,
                                forbidden,
                            )?;
                            validate_no_forbidden_private_names_in_function(
                                &property.function,
                                forbidden,
                            )?;
                        }
                        Prop::Assign(property) => {
                            validate_no_forbidden_private_names_in_expression(
                                &property.value,
                                forbidden,
                            )?;
                        }
                    },
                }
            }
        }
        Expr::OptChain(optional_chain) => match optional_chain.base.as_ref() {
            OptChainBase::Member(member) => {
                validate_no_forbidden_private_names_in_member(member, forbidden)?;
            }
            OptChainBase::Call(call) => {
                validate_no_forbidden_private_names_in_expression(&call.callee, forbidden)?;
                for argument in &call.args {
                    validate_no_forbidden_private_names_in_expression(&argument.expr, forbidden)?;
                }
            }
        },
        Expr::Member(member) => {
            validate_no_forbidden_private_names_in_member(member, forbidden)?;
        }
        Expr::Unary(unary) => {
            validate_no_forbidden_private_names_in_expression(&unary.arg, forbidden)?;
        }
        Expr::Update(update) => {
            validate_no_forbidden_private_names_in_expression(&update.arg, forbidden)?;
        }
        Expr::Bin(binary) => {
            validate_no_forbidden_private_names_in_expression(&binary.left, forbidden)?;
            validate_no_forbidden_private_names_in_expression(&binary.right, forbidden)?;
        }
        Expr::Assign(assignment) => {
            match &assignment.left {
                AssignTarget::Simple(SimpleAssignTarget::Member(member)) => {
                    validate_no_forbidden_private_names_in_member(member, forbidden)?;
                }
                AssignTarget::Simple(_) | AssignTarget::Pat(_) => {}
            }
            validate_no_forbidden_private_names_in_expression(&assignment.right, forbidden)?;
        }
        Expr::Cond(conditional) => {
            validate_no_forbidden_private_names_in_expression(&conditional.test, forbidden)?;
            validate_no_forbidden_private_names_in_expression(&conditional.cons, forbidden)?;
            validate_no_forbidden_private_names_in_expression(&conditional.alt, forbidden)?;
        }
        Expr::Seq(sequence) => {
            for expression in &sequence.exprs {
                validate_no_forbidden_private_names_in_expression(expression, forbidden)?;
            }
        }
        Expr::Fn(function) => {
            validate_no_forbidden_private_names_in_function(&function.function, forbidden)?;
        }
        Expr::Arrow(arrow) => {
            for parameter in &arrow.params {
                validate_no_forbidden_private_names_in_pattern(parameter, forbidden)?;
            }
            match &*arrow.body {
                BlockStmtOrExpr::BlockStmt(block) => {
                    for statement in &block.stmts {
                        validate_no_forbidden_private_names_in_statement(statement, forbidden)?;
                    }
                }
                BlockStmtOrExpr::Expr(expression) => {
                    validate_no_forbidden_private_names_in_expression(expression, forbidden)?;
                }
            }
        }
        Expr::Class(class) => {
            validate_no_forbidden_private_names_in_class(&class.class, forbidden)?;
        }
        Expr::Tpl(template) => {
            for expression in &template.exprs {
                validate_no_forbidden_private_names_in_expression(expression, forbidden)?;
            }
        }
        Expr::TaggedTpl(tagged) => {
            validate_no_forbidden_private_names_in_expression(&tagged.tag, forbidden)?;
            for expression in &tagged.tpl.exprs {
                validate_no_forbidden_private_names_in_expression(expression, forbidden)?;
            }
        }
        _ => {}
    }

    Ok(())
}

pub(crate) fn validate_function_syntax(
    function: &Function,
    file: &swc_common::SourceFile,
) -> Result<()> {
    validate_function_syntax_with_explicit_strictness(
        function,
        file,
        function_has_use_strict_directive(function),
    )
}

fn validate_function_syntax_with_explicit_strictness(
    function: &Function,
    file: &swc_common::SourceFile,
    strict: bool,
) -> Result<()> {
    let restrictions = BindingRestrictions {
        await_reserved: function.is_async,
        yield_reserved: function.is_generator,
        await_expression_forbidden: false,
    };
    ensure_parameter_names_are_valid(
        function.params.iter().map(|parameter| &parameter.pat),
        function
            .params
            .iter()
            .all(|parameter| matches!(parameter.pat, Pat::Ident(_))),
        strict,
    )?;
    for parameter in &function.params {
        validate_pattern_syntax_with_restrictions(&parameter.pat, file, restrictions)?;
    }
    if let Some(body) = &function.body {
        validate_function_parameters_do_not_overlap_body_lexical_names(
            function.params.iter().map(|parameter| &parameter.pat),
            body,
        )?;
        for statement in &body.stmts {
            validate_statement_syntax_with_restrictions(statement, file, restrictions)?;
        }
    }

    Ok(())
}

fn collect_direct_function_body_lexically_declared_names(
    statements: &[Stmt],
) -> Result<Vec<String>> {
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

fn validate_function_parameters_do_not_overlap_body_lexical_names<'a>(
    parameters: impl IntoIterator<Item = &'a Pat>,
    body: &BlockStmt,
) -> Result<()> {
    let lexical_names = collect_direct_function_body_lexically_declared_names(&body.stmts)?
        .into_iter()
        .collect::<HashSet<_>>();

    if lexical_names.is_empty() {
        return Ok(());
    }

    for parameter in parameters {
        let mut parameter_names = Vec::new();
        collect_pattern_binding_names(parameter, &mut parameter_names)?;
        for name in parameter_names {
            ensure!(
                !lexical_names.contains(&name),
                "function parameter name `{name}` conflicts with lexical declaration in body"
            );
        }
    }

    Ok(())
}

pub(super) fn ensure_parameter_names_are_valid<'a>(
    parameters: impl IntoIterator<Item = &'a Pat>,
    has_simple_parameter_list: bool,
    strict: bool,
) -> Result<()> {
    let mut seen = HashSet::new();
    let mut duplicate = None;

    for parameter in parameters {
        let mut names = Vec::new();
        collect_pattern_binding_names_including_duplicates(parameter, &mut names)?;
        for name in names {
            if !seen.insert(name.clone()) && duplicate.is_none() {
                duplicate = Some(name);
            }
        }
    }

    if let Some(name) = duplicate {
        ensure!(
            has_simple_parameter_list && !strict,
            "duplicate parameter name `{name}`"
        );
    }

    Ok(())
}

pub(crate) fn validate_class_syntax(class: &Class, file: &swc_common::SourceFile) -> Result<()> {
    let private_name_declarations = collect_class_private_name_declarations(class, file)?;
    let class_private_names = private_name_declarations
        .keys()
        .cloned()
        .collect::<HashSet<_>>();

    if let Some(super_class) = &class.super_class {
        validate_expression_syntax(super_class, file)?;
        validate_no_forbidden_private_names_in_expression(super_class, &class_private_names)?;
    }

    for member in &class.body {
        match member {
            ClassMember::Constructor(constructor) => {
                validate_constructor_syntax(constructor, file, true)?;
            }
            ClassMember::Method(method) => {
                if let Some(property_name) = static_prop_name(&method.key) {
                    ensure!(
                        !(method.is_static && property_name == "prototype"),
                        "static class method name `prototype` is not allowed"
                    );
                }
                validate_property_name_syntax(&method.key, file)?;
                validate_function_syntax_with_explicit_strictness(&method.function, file, true)?;
            }
            ClassMember::ClassProp(property) => {
                if let Some(property_name) = static_prop_name(&property.key) {
                    ensure!(
                        property_name != "constructor" || property.is_static,
                        "class field name `constructor` is not allowed"
                    );
                    ensure!(
                        !(property.is_static
                            && matches!(property_name, "prototype" | "constructor")),
                        "static class field name `{property_name}` is not allowed"
                    );
                }
                validate_property_name_syntax(&property.key, file)?;
                if let Some(value) = &property.value {
                    validate_expression_syntax(value, file)?;
                }
            }
            ClassMember::PrivateMethod(method) => {
                validate_function_syntax_with_explicit_strictness(&method.function, file, true)?;
            }
            ClassMember::PrivateProp(property) => {
                if let Some(value) = &property.value {
                    validate_expression_syntax(value, file)?;
                }
            }
            ClassMember::AutoAccessor(accessor) => {
                match &accessor.key {
                    Key::Public(property_key) => {
                        if let Some(property_name) = static_prop_name(property_key) {
                            ensure!(
                                property_name != "constructor" || accessor.is_static,
                                "class accessor field name `constructor` is not allowed"
                            );
                            ensure!(
                                !(accessor.is_static
                                    && matches!(property_name, "prototype" | "constructor")),
                                "static class accessor field name `{property_name}` is not allowed"
                            );
                        }
                        validate_property_name_syntax(property_key, file)?;
                    }
                    Key::Private(_) => {}
                }
                if let Some(value) = &accessor.value {
                    validate_expression_syntax(value, file)?;
                }
            }
            ClassMember::StaticBlock(block) => {
                validate_block_statement_early_errors(&block.body.stmts)?;
                let restrictions = BindingRestrictions {
                    await_reserved: true,
                    yield_reserved: false,
                    await_expression_forbidden: true,
                };
                for statement in &block.body.stmts {
                    validate_statement_syntax_with_restrictions(statement, file, restrictions)?;
                }
            }
            _ => {}
        }
    }

    Ok(())
}

fn validate_constructor_syntax(
    constructor: &Constructor,
    file: &swc_common::SourceFile,
    strict: bool,
) -> Result<()> {
    ensure_parameter_names_are_valid(
        constructor.params.iter().filter_map(|parameter| match parameter {
            ParamOrTsParamProp::Param(parameter) => Some(&parameter.pat),
            ParamOrTsParamProp::TsParamProp(_) => None,
        }),
        constructor
            .params
            .iter()
            .all(|parameter| matches!(parameter, ParamOrTsParamProp::Param(parameter) if matches!(parameter.pat, Pat::Ident(_)))),
        strict,
    )?;
    for parameter in &constructor.params {
        match parameter {
            ParamOrTsParamProp::Param(parameter) => validate_pattern_syntax_with_restrictions(
                &parameter.pat,
                file,
                BindingRestrictions::default(),
            )?,
            ParamOrTsParamProp::TsParamProp(_) => {}
        }
    }
    if let Some(body) = &constructor.body {
        for statement in &body.stmts {
            validate_statement_syntax(statement, file)?;
        }
    }

    Ok(())
}

fn static_prop_name(name: &PropName) -> Option<&str> {
    match name {
        PropName::Ident(identifier) => Some(identifier.sym.as_ref()),
        PropName::Str(string) => string.value.as_str(),
        PropName::Computed(_) => None,
        _ => None,
    }
}

pub(super) fn validate_property_name_syntax(
    name: &PropName,
    file: &swc_common::SourceFile,
) -> Result<()> {
    match name {
        PropName::Ident(identifier) => {
            let raw = source_slice_for_span(file, identifier.span)?;
            if raw.contains('\\') {
                validate_escaped_identifier_text(raw)?;
            }
        }
        PropName::Computed(computed) => {
            validate_expression_syntax(&computed.expr, file)?;
        }
        _ => {}
    }

    Ok(())
}

fn validate_private_name_syntax(
    private_name: &PrivateName,
    file: &swc_common::SourceFile,
) -> Result<()> {
    let raw = source_slice_for_span(file, private_name.span)?;
    let raw_identifier = raw.strip_prefix('#').unwrap_or(raw);
    if raw_identifier.contains('\\') {
        validate_escaped_identifier_text(raw_identifier)?;
    }
    Ok(())
}
