mod bindings;
mod blocks;
mod declarations;
mod expressions;
mod functions;
mod statements;

pub(crate) use self::bindings::{
    collect_module_declared_names, collect_pattern_binding_names, collect_var_decl_bound_names,
    ensure_module_lexical_names_are_unique,
};
pub(crate) use self::blocks::validate_script_body_early_errors;
pub(crate) use self::declarations::{
    BindingRestrictions, validate_declaration_syntax, validate_declaration_syntax_with_restrictions,
};
pub(crate) use self::expressions::{
    validate_expression_syntax, validate_expression_syntax_with_restrictions,
};
pub(crate) use self::functions::{
    validate_class_syntax, validate_class_syntax_with_restrictions, validate_function_syntax,
    validate_function_syntax_with_restrictions,
};
pub(crate) use self::statements::{
    validate_statement_syntax, validate_statement_syntax_with_restrictions,
};
