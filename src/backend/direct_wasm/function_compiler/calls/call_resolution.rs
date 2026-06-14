use super::*;

const SNAPSHOT_AWAIT_RESOLVE_BINDING: &str = "__ayy_snapshot_await_resolve";
const SNAPSHOT_AWAIT_REJECT_BINDING: &str = "__ayy_snapshot_await_reject";
const SNAPSHOT_AWAIT_RESOLUTION_VALUE: &str = "__ayy_snapshot_await_resolution";
const SNAPSHOT_AWAIT_REJECTION_VALUE: &str = "__ayy_snapshot_await_rejection";

thread_local! {
    static ACTIVE_BOUND_SNAPSHOT_EXPRESSIONS: std::cell::RefCell<std::collections::HashSet<String>> =
        std::cell::RefCell::new(std::collections::HashSet::new());
}

struct BoundSnapshotExpressionGuard {
    key: String,
    _memo: crate::backend::direct_wasm::memo::ResolutionGuardScope,
}

impl BoundSnapshotExpressionGuard {
    fn enter(expression: &Expression, current_function_name: Option<&str>) -> Option<Self> {
        let key = format!("{current_function_name:?}:{expression:?}");
        ACTIVE_BOUND_SNAPSHOT_EXPRESSIONS.with(|active| {
            let mut active = active.borrow_mut();
            if !active.insert(key.clone()) {
                crate::backend::direct_wasm::memo::note_resolution_guard_block();
                return None;
            }
            Some(Self {
                key,
                _memo: crate::backend::direct_wasm::memo::ResolutionGuardScope::enter_class(8),
            })
        })
    }
}

impl Drop for BoundSnapshotExpressionGuard {
    fn drop(&mut self) {
        ACTIVE_BOUND_SNAPSHOT_EXPRESSIONS.with(|active| {
            active.borrow_mut().remove(&self.key);
        });
    }
}

pub(in crate::backend::direct_wasm) enum BoundSnapshotControlFlow {
    None,
    Return(Expression),
    Throw(Expression),
    Break(Option<String>),
}

pub(in crate::backend::direct_wasm) struct PreparedStaticUserFunctionExecution {
    pub(in crate::backend::direct_wasm) substituted_body: Vec<Statement>,
    pub(in crate::backend::direct_wasm) environment: StaticResolutionEnvironment,
}

mod bound_snapshots;
#[path = "call_resolution/inline_effect_emission.rs"]
mod inline_effect_emission;
mod inline_summaries;
mod returned_values;
mod runtime_scans;
#[path = "call_resolution/statement_substitution.rs"]
mod statement_substitution;
mod static_user_functions;
mod substitutions;

impl<'a> FunctionCompiler<'a> {
    pub(in crate::backend::direct_wasm) fn resolve_bound_snapshot_binding_name<'b>(
        &self,
        name: &'b str,
        bindings: &HashMap<String, Expression>,
    ) -> &'b str {
        if bindings.contains_key(name) {
            return name;
        }
        scoped_binding_source_name(name)
            .filter(|source_name| bindings.contains_key(*source_name))
            .unwrap_or(name)
    }

    fn bound_snapshot_name_matches_source(name: &str, source_name: &str) -> bool {
        name == source_name
            || scoped_binding_source_name(name).is_some_and(|candidate| candidate == source_name)
    }

    fn bound_snapshot_current_function_declares_binding_source(
        &self,
        current_function_name: Option<&str>,
        source_name: &str,
    ) -> bool {
        let Some(function_name) = current_function_name else {
            return false;
        };
        if let Some(user_function) = self.user_function(function_name)
            && (user_function
                .params
                .iter()
                .any(|name| Self::bound_snapshot_name_matches_source(name, source_name))
                || user_function
                    .scope_bindings
                    .iter()
                    .any(|name| Self::bound_snapshot_name_matches_source(name, source_name)))
        {
            return true;
        }
        let Some(function) = self.resolve_registered_function_declaration(function_name) else {
            return false;
        };
        if function
            .self_binding
            .as_deref()
            .is_some_and(|name| Self::bound_snapshot_name_matches_source(name, source_name))
        {
            return true;
        }
        collect_declared_bindings_from_statements_recursive(&function.body)
            .iter()
            .any(|name| Self::bound_snapshot_name_matches_source(name, source_name))
    }

    pub(in crate::backend::direct_wasm) fn bound_snapshot_current_function_is_strict(
        &self,
        current_function_name: Option<&str>,
    ) -> bool {
        current_function_name
            .and_then(|function_name| self.user_function(function_name))
            .is_some_and(|function| function.strict)
    }

    pub(in crate::backend::direct_wasm) fn resolve_bound_snapshot_captured_self_binding_name(
        &self,
        name: &str,
        bindings: &HashMap<String, Expression>,
        current_function_name: Option<&str>,
    ) -> Option<String> {
        let source_name = scoped_binding_source_name(name).unwrap_or(name);
        if self.bound_snapshot_current_function_declares_binding_source(
            current_function_name,
            source_name,
        ) {
            return None;
        }
        let mut binding_names = bindings.keys().cloned().collect::<Vec<_>>();
        binding_names.sort();
        binding_names.into_iter().find(|binding_name| {
            self.resolve_registered_function_declaration(binding_name)
                .and_then(|function| function.self_binding.as_deref())
                .is_some_and(|self_binding| {
                    Self::bound_snapshot_name_matches_source(self_binding, source_name)
                })
        })
    }
}
