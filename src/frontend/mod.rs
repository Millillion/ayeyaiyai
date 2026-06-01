mod early_errors;
mod lowering;
mod modules;
mod parse;

pub use modules::{
    bundle_module_entry, bundle_module_entry_unmodified, bundle_script_entry,
    bundle_script_entry_with_strict, bundle_script_entry_with_strict_unmodified,
};
pub use parse::{
    parse, parse_module_goal, parse_module_goal_with_path, parse_script_goal,
    script_goal_has_direct_using_declaration, validate_script_goal,
    validate_script_goal_with_forced_strict,
};

#[cfg(test)]
mod parse_tests;
