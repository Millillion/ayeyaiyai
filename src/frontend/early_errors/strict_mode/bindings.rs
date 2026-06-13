use super::super::*;
use super::{
    directives::{is_strict_mode_reserved_identifier, is_strict_mode_restricted_identifier},
    expressions::validate_strict_mode_early_errors_in_expression,
};

pub(super) fn is_strict_mode_forbidden_binding_identifier(name: &str) -> bool {
    is_strict_mode_restricted_identifier(name) || is_strict_mode_reserved_identifier(name)
}

pub(super) fn validate_strict_mode_early_errors_in_variable_declaration(
    declaration: &swc_ecma_ast::VarDecl,
    strict: bool,
) -> Result<()> {
    for declarator in &declaration.decls {
        validate_strict_mode_early_errors_in_pattern(&declarator.name, strict)?;
        if let Some(initializer) = &declarator.init {
            validate_strict_mode_early_errors_in_expression(initializer, strict)?;
        }
    }

    Ok(())
}

pub(super) fn validate_strict_mode_early_errors_in_for_head(
    head: &ForHead,
    strict: bool,
) -> Result<()> {
    match head {
        ForHead::VarDecl(variable_declaration) => {
            validate_strict_mode_early_errors_in_variable_declaration(
                variable_declaration,
                strict,
            )?;
        }
        ForHead::Pat(pattern) => validate_strict_mode_early_errors_in_pattern(pattern, strict)?,
        ForHead::UsingDecl(_) => {}
    }

    Ok(())
}

pub(super) fn validate_strict_mode_early_errors_in_pattern(
    pattern: &Pat,
    strict: bool,
) -> Result<()> {
    match pattern {
        Pat::Ident(identifier) => {
            ensure!(
                !strict || !is_strict_mode_forbidden_binding_identifier(identifier.id.sym.as_ref()),
                "strict mode forbids binding `{}`",
                identifier.id.sym
            );
        }
        Pat::Assign(assign) => {
            validate_strict_mode_early_errors_in_pattern(&assign.left, strict)?;
            validate_strict_mode_early_errors_in_expression(&assign.right, strict)?;
        }
        Pat::Array(array) => {
            for element in array.elems.iter().flatten() {
                validate_strict_mode_early_errors_in_pattern(element, strict)?;
            }
        }
        Pat::Object(object) => {
            for property in &object.props {
                match property {
                    ObjectPatProp::KeyValue(property) => {
                        validate_property_name_strict_mode_early_errors(&property.key, strict)?;
                        validate_strict_mode_early_errors_in_pattern(&property.value, strict)?;
                    }
                    ObjectPatProp::Assign(property) => {
                        ensure!(
                            !strict
                                || !is_strict_mode_forbidden_binding_identifier(
                                    property.key.sym.as_ref()
                                ),
                            "strict mode forbids binding `{}`",
                            property.key.sym
                        );
                        if let Some(value) = &property.value {
                            validate_strict_mode_early_errors_in_expression(value, strict)?;
                        }
                    }
                    ObjectPatProp::Rest(rest) => {
                        validate_strict_mode_early_errors_in_pattern(&rest.arg, strict)?;
                    }
                }
            }
        }
        Pat::Rest(rest) => validate_strict_mode_early_errors_in_pattern(&rest.arg, strict)?,
        Pat::Expr(expression) => {
            validate_strict_mode_early_errors_in_expression(expression, strict)?
        }
        _ => {}
    }

    Ok(())
}

pub(super) fn validate_property_name_strict_mode_early_errors(
    name: &PropName,
    strict: bool,
) -> Result<()> {
    if let PropName::Computed(computed) = name {
        validate_strict_mode_early_errors_in_expression(&computed.expr, strict)?;
    }

    Ok(())
}

fn validate_strict_mode_assignment_identifier(identifier: &Ident, strict: bool) -> Result<()> {
    ensure!(
        !strict || !is_strict_mode_forbidden_binding_identifier(identifier.sym.as_ref()),
        "strict mode forbids assigning to `{}`",
        identifier.sym
    );

    Ok(())
}

fn validate_strict_mode_member_assignment_target(member: &MemberExpr, strict: bool) -> Result<()> {
    validate_strict_mode_early_errors_in_expression(&member.obj, strict)?;
    if let MemberProp::Computed(property) = &member.prop {
        validate_strict_mode_early_errors_in_expression(&property.expr, strict)?;
    }

    Ok(())
}

fn validate_strict_mode_assignment_target_expression(
    expression: &Expr,
    strict: bool,
) -> Result<()> {
    match expression {
        Expr::Ident(identifier) => validate_strict_mode_assignment_identifier(identifier, strict)?,
        Expr::Paren(parenthesized) => {
            validate_strict_mode_assignment_target_expression(&parenthesized.expr, strict)?
        }
        Expr::Member(member) => validate_strict_mode_member_assignment_target(member, strict)?,
        Expr::SuperProp(super_property) => {
            if let SuperProp::Computed(property) = &super_property.prop {
                validate_strict_mode_early_errors_in_expression(&property.expr, strict)?;
            }
        }
        _ => validate_strict_mode_early_errors_in_expression(expression, strict)?,
    }

    Ok(())
}

pub(super) fn validate_strict_mode_assignment_target(
    target: &AssignTarget,
    strict: bool,
) -> Result<()> {
    match target {
        AssignTarget::Simple(SimpleAssignTarget::Ident(identifier)) => {
            validate_strict_mode_assignment_identifier(&identifier.id, strict)?;
        }
        AssignTarget::Simple(SimpleAssignTarget::Member(member)) => {
            validate_strict_mode_member_assignment_target(member, strict)?;
        }
        AssignTarget::Simple(SimpleAssignTarget::SuperProp(super_property)) => {
            if let SuperProp::Computed(property) = &super_property.prop {
                validate_strict_mode_early_errors_in_expression(&property.expr, strict)?;
            }
        }
        AssignTarget::Simple(SimpleAssignTarget::Paren(parenthesized)) => {
            validate_strict_mode_assignment_target_expression(&parenthesized.expr, strict)?;
        }
        AssignTarget::Pat(pattern) => {
            let pattern: Pat = pattern.clone().into();
            validate_strict_mode_early_errors_in_pattern(&pattern, strict)?;
        }
        AssignTarget::Simple(_) => {}
    }

    Ok(())
}
