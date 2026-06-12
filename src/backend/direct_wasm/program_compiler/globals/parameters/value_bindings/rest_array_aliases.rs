//! Program-level tracking of bindings whose only assigned value is a
//! destructuring rest-array temporary (`__ayy_array_rest_*`). Rest arrays are
//! fresh allocations minted per destructuring evaluation, so a binding that is
//! only ever assigned one such temp can use the temp name as a static identity
//! alias for strict-equality resolution inside callees (the parameter value
//! binding channel). Bindings that are reassigned, shadowed by a function
//! parameter, or bound by catch/switch/loop headers are poisoned.

use std::cell::RefCell;
use std::collections::HashMap;

use crate::ir::hir::{Expression, Parameter, Program, Statement};
use crate::ir::visit::{Visitor, walk_expression, walk_parameter, walk_statement};

thread_local! {
    static TOP_LEVEL_REST_ARRAY_ALIASES: RefCell<HashMap<String, Option<String>>> =
        RefCell::new(HashMap::new());
}

#[derive(Default)]
struct RestArrayAliasCollector {
    aliases: HashMap<String, Option<String>>,
}

impl RestArrayAliasCollector {
    fn record(&mut self, name: &str, value: &Expression) {
        // Hoisted `var` declarations initialize with Undefined before the
        // real assignment; they carry no identity and must not poison.
        if matches!(value, Expression::Undefined) {
            return;
        }
        let rest_value = match value {
            Expression::Identifier(alias) if alias.contains("__ayy_array_rest_") => {
                Some(alias.clone())
            }
            _ => None,
        };
        match (self.aliases.get(name), rest_value) {
            (Some(Some(existing)), Some(rest)) if *existing == rest => {}
            (None, rest) => {
                self.aliases.insert(name.to_string(), rest);
            }
            _ => {
                self.aliases.insert(name.to_string(), None);
            }
        }
    }

    fn poison(&mut self, name: &str) {
        self.aliases.insert(name.to_string(), None);
    }
}

impl Visitor for RestArrayAliasCollector {
    fn visit_parameter(&mut self, parameter: &Parameter) {
        self.poison(&parameter.name);
        walk_parameter(self, parameter);
    }

    fn visit_statement(&mut self, statement: &Statement) {
        match statement {
            Statement::Var { name, value }
            | Statement::Let { name, value, .. }
            | Statement::Assign { name, value } => {
                self.record(name, value);
            }
            Statement::Try {
                catch_binding: Some(catch_binding),
                ..
            } => {
                self.poison(catch_binding);
            }
            Statement::Switch { bindings, .. } => {
                for binding in bindings {
                    self.poison(binding);
                }
            }
            _ => {}
        }
        walk_statement(self, statement);
    }

    fn visit_expression(&mut self, expression: &Expression) {
        if let Expression::Assign { name, value } = expression {
            self.record(name, value);
        }
        walk_expression(self, expression);
    }
}

pub(in crate::backend::direct_wasm) fn collect_stable_rest_array_aliases(program: &Program) {
    let mut collector = RestArrayAliasCollector::default();
    collector.visit_program(program);
    TOP_LEVEL_REST_ARRAY_ALIASES.with(|aliases| {
        *aliases.borrow_mut() = collector.aliases;
    });
}

pub(in crate::backend::direct_wasm) fn stable_rest_array_alias(name: &str) -> Option<String> {
    TOP_LEVEL_REST_ARRAY_ALIASES.with(|aliases| aliases.borrow().get(name).cloned().flatten())
}
