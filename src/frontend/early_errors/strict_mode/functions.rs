use super::super::*;
use super::{
    bindings::{
        validate_property_name_strict_mode_early_errors,
        validate_strict_mode_early_errors_in_pattern,
        validate_strict_mode_early_errors_in_variable_declaration,
    },
    directives::function_has_use_strict_directive,
    expressions::validate_strict_mode_early_errors_in_expression,
    statements::validate_strict_mode_early_errors_in_statements,
};

pub(super) fn validate_strict_mode_early_errors_in_declaration(
    declaration: &Decl,
    strict: bool,
) -> Result<()> {
    match declaration {
        Decl::Fn(function) => {
            validate_strict_mode_function_binding_identifier(
                function.ident.sym.as_ref(),
                strict || function_has_use_strict_directive(&function.function),
            )?;
            validate_strict_mode_early_errors_in_function(&function.function, strict)?;
        }
        Decl::Class(class) => validate_strict_mode_early_errors_in_class(&class.class, strict)?,
        Decl::Var(variable_declaration) => {
            validate_strict_mode_early_errors_in_variable_declaration(
                variable_declaration,
                strict,
            )?;
        }
        _ => {}
    }

    Ok(())
}

pub(super) fn validate_strict_mode_early_errors_in_function(
    function: &Function,
    strict: bool,
) -> Result<()> {
    let function_strict = strict || function_has_use_strict_directive(function);

    ensure_no_duplicate_parameter_names(
        function.params.iter().map(|parameter| &parameter.pat),
        function_strict,
    )?;
    for parameter in &function.params {
        validate_strict_mode_early_errors_in_pattern(&parameter.pat, function_strict)?;
    }

    if let Some(body) = &function.body {
        validate_strict_mode_early_errors_in_statements(&body.stmts, function_strict)?;
    }

    Ok(())
}

pub(super) fn validate_strict_mode_function_binding_identifier(
    name: &str,
    strict: bool,
) -> Result<()> {
    ensure!(
        !strict || !super::bindings::is_strict_mode_forbidden_binding_identifier(name),
        "strict mode forbids binding `{name}`"
    );

    Ok(())
}

pub(super) fn validate_strict_mode_early_errors_in_class(
    class: &Class,
    strict: bool,
) -> Result<()> {
    if let Some(super_class) = &class.super_class {
        validate_strict_mode_early_errors_in_expression(super_class, strict)?;
    }

    for member in &class.body {
        match member {
            ClassMember::Constructor(constructor) => {
                validate_strict_mode_early_errors_in_constructor(constructor, true)?;
            }
            ClassMember::Method(method) => {
                validate_property_name_strict_mode_early_errors(&method.key, true)?;
                validate_strict_mode_early_errors_in_function(&method.function, true)?;
            }
            ClassMember::ClassProp(property) => {
                validate_property_name_strict_mode_early_errors(&property.key, true)?;
                if let Some(value) = &property.value {
                    validate_strict_mode_early_errors_in_expression(value, true)?;
                }
            }
            ClassMember::PrivateMethod(method) => {
                validate_strict_mode_early_errors_in_function(&method.function, true)?;
            }
            ClassMember::PrivateProp(property) => {
                if let Some(value) = &property.value {
                    validate_strict_mode_early_errors_in_expression(value, true)?;
                }
            }
            ClassMember::StaticBlock(block) => {
                validate_strict_mode_early_errors_in_statements(&block.body.stmts, true)?;
            }
            _ => {}
        }
    }

    Ok(())
}

fn validate_strict_mode_early_errors_in_constructor(
    constructor: &Constructor,
    strict: bool,
) -> Result<()> {
    ensure_no_duplicate_parameter_names(
        constructor
            .params
            .iter()
            .filter_map(|parameter| match parameter {
                ParamOrTsParamProp::Param(parameter) => Some(&parameter.pat),
                ParamOrTsParamProp::TsParamProp(_) => None,
            }),
        strict,
    )?;
    for parameter in &constructor.params {
        match parameter {
            ParamOrTsParamProp::Param(parameter) => {
                validate_strict_mode_early_errors_in_pattern(&parameter.pat, strict)?;
            }
            ParamOrTsParamProp::TsParamProp(_) => {}
        }
    }

    if let Some(body) = &constructor.body {
        validate_strict_mode_early_errors_in_statements(&body.stmts, strict)?;
    }

    Ok(())
}

pub(super) fn ensure_no_duplicate_parameter_names<'a>(
    parameters: impl IntoIterator<Item = &'a Pat>,
    strict: bool,
) -> Result<()> {
    if !strict {
        return Ok(());
    }

    let mut seen = HashSet::new();
    for parameter in parameters {
        let mut names = Vec::new();
        collect_parameter_binding_names_including_duplicates(parameter, &mut names)?;
        for name in names {
            ensure!(
                seen.insert(name.clone()),
                "duplicate parameter name `{name}`"
            );
        }
    }

    Ok(())
}

fn collect_parameter_binding_names_including_duplicates(
    pattern: &Pat,
    names: &mut Vec<String>,
) -> Result<()> {
    match pattern {
        Pat::Ident(identifier) => {
            names.push(identifier.id.sym.to_string());
        }
        Pat::Assign(assign) => {
            collect_parameter_binding_names_including_duplicates(&assign.left, names)?;
        }
        Pat::Array(array) => {
            for element in array.elems.iter().flatten() {
                collect_parameter_binding_names_including_duplicates(element, names)?;
            }
        }
        Pat::Object(object) => {
            for property in &object.props {
                match property {
                    ObjectPatProp::KeyValue(property) => {
                        collect_parameter_binding_names_including_duplicates(
                            &property.value,
                            names,
                        )?;
                    }
                    ObjectPatProp::Assign(property) => {
                        names.push(property.key.id.sym.to_string());
                    }
                    ObjectPatProp::Rest(rest) => {
                        collect_parameter_binding_names_including_duplicates(&rest.arg, names)?;
                    }
                }
            }
        }
        Pat::Rest(rest) => collect_parameter_binding_names_including_duplicates(&rest.arg, names)?,
        Pat::Expr(_) | Pat::Invalid(_) => bail!("unsupported binding pattern"),
    }

    Ok(())
}
