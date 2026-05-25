use super::*;

pub(in crate::backend::direct_wasm) fn eval_program_declares_var_arguments(
    program: &Program,
) -> bool {
    eval_statements_declare_var_arguments(&program.statements)
}

pub(in crate::backend::direct_wasm) fn collect_direct_eval_lexical_binding_names(
    statements: &[Statement],
) -> Vec<String> {
    fn should_collect_eval_lexical_name(name: &str) -> bool {
        !name.starts_with("__ayy_") || scoped_binding_source_name(name).is_some()
    }

    fn collect_direct_statement_name(statement: &Statement, bindings: &mut Vec<String>) {
        match statement {
            Statement::Let { name, .. } if should_collect_eval_lexical_name(name) => {
                bindings.push(name.clone());
            }
            Statement::Declaration { body } => {
                for statement in body {
                    if let Statement::Let { name, .. } = statement
                        && should_collect_eval_lexical_name(name)
                    {
                        bindings.push(name.clone());
                    }
                }
            }
            _ => {}
        }
    }

    let mut bindings = Vec::new();
    let mut seen = HashSet::new();
    for statement in statements {
        let mut direct_names = Vec::new();
        collect_direct_statement_name(statement, &mut direct_names);
        for name in direct_names {
            if seen.insert(name.clone()) {
                bindings.push(name);
            }
        }
    }
    bindings
}
