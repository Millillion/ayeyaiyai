use super::*;

use crate::ir::visit::{Visitor, walk_expression, walk_statement};

/// Returns true when the function body contains a `++`/`--` update targeting
/// one of the function's own bindings (a parameter or a body-declared
/// var/let). Such updates cannot be replayed at an inline call site: the
/// binding has no storage in the caller, so the update emission would treat
/// it as an unresolvable global and raise a spurious ReferenceError (e.g.
/// `function f(a) { return ++a; }`).
pub(in crate::backend::direct_wasm) fn function_body_updates_own_binding(
    function: &FunctionDeclaration,
) -> bool {
    struct DeclaredNameCollector {
        names: HashSet<String>,
    }

    impl Visitor for DeclaredNameCollector {
        fn visit_statement(&mut self, statement: &Statement) {
            match statement {
                Statement::Var { name, .. } | Statement::Let { name, .. } => {
                    self.names.insert(name.clone());
                }
                _ => {}
            }
            walk_statement(self, statement);
        }
    }

    struct UpdateFinder<'a> {
        names: &'a HashSet<String>,
        found: bool,
    }

    impl Visitor for UpdateFinder<'_> {
        fn visit_expression(&mut self, expression: &Expression) {
            if let Expression::Update { name, .. } = expression
                && self.names.contains(name)
            {
                self.found = true;
            }
            walk_expression(self, expression);
        }
    }

    let mut declared = DeclaredNameCollector {
        names: function
            .params
            .iter()
            .map(|param| param.name.clone())
            .collect(),
    };
    for statement in &function.body {
        declared.visit_statement(statement);
    }

    let mut finder = UpdateFinder {
        names: &declared.names,
        found: false,
    };
    for statement in &function.body {
        finder.visit_statement(statement);
        if finder.found {
            return true;
        }
    }
    false
}

pub(in crate::backend::direct_wasm) fn collect_inline_function_summary(
    function: &FunctionDeclaration,
) -> Option<InlineFunctionSummary> {
    if function_body_updates_own_binding(function) {
        return None;
    }
    let mut summary = InlineFunctionSummary::default();
    let parameter_names = function
        .params
        .iter()
        .map(|param| param.name.clone())
        .collect::<HashSet<_>>();
    let mut local_bindings = HashMap::new();
    for statement in &function.body {
        match statement {
            Statement::Var { name, value } | Statement::Let { name, value, .. } => {
                if parameter_names.contains(name) {
                    return None;
                }
                local_bindings.insert(
                    name.clone(),
                    substitute_inline_summary_bindings(value, &local_bindings),
                );
            }
            Statement::Assign { name, value } => {
                if parameter_names.contains(name) {
                    return None;
                }
                if local_bindings.contains_key(name) {
                    return None;
                }
                summary.effects.push(InlineFunctionEffect::Assign {
                    name: name.clone(),
                    value: substitute_inline_summary_bindings(value, &local_bindings),
                });
            }
            Statement::AssignMember {
                object,
                property,
                value,
            } => {
                let object = substitute_inline_summary_bindings(object, &local_bindings);
                let property = substitute_inline_summary_bindings(property, &local_bindings);
                let value = substitute_inline_summary_bindings(value, &local_bindings);
                if !function.mapped_arguments
                    && matches!(&object, Expression::Identifier(name) if name == "arguments")
                    && inline_summary_side_effect_free_expression(&property)
                    && inline_summary_side_effect_free_expression(&value)
                {
                    continue;
                }
                summary
                    .effects
                    .push(InlineFunctionEffect::Expression(Expression::AssignMember {
                        object: Box::new(object),
                        property: Box::new(property),
                        value: Box::new(value),
                    }));
            }
            Statement::Expression(Expression::Update { name, op, prefix }) => {
                if function.params.iter().any(|param| param.name == *name)
                    || local_bindings.contains_key(name)
                {
                    return None;
                }
                summary.effects.push(InlineFunctionEffect::Update {
                    name: name.clone(),
                    op: *op,
                    prefix: *prefix,
                });
            }
            Statement::Expression(expression) => {
                summary.effects.push(InlineFunctionEffect::Expression(
                    substitute_inline_summary_bindings(expression, &local_bindings),
                ))
            }
            Statement::Return(value) => {
                if summary.return_value.is_some() {
                    return None;
                }
                summary.return_value =
                    Some(substitute_inline_summary_bindings(value, &local_bindings));
                return Some(summary);
            }
            Statement::Block { body } if body.is_empty() => {}
            _ => return None,
        }
    }

    Some(summary)
}
