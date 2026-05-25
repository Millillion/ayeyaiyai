mod await_rewrite;
mod entrypoints;
mod source;
mod validation;

pub use self::entrypoints::{
    parse, parse_module_goal, parse_module_goal_with_path, parse_script_goal,
    script_goal_has_direct_using_declaration, validate_script_goal,
};
pub(crate) use self::source::{parse_module_file, parse_script_file, parse_script_program_source};
