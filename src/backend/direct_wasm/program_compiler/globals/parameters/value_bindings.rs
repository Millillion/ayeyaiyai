use super::*;

#[path = "value_bindings/call_registration.rs"]
mod call_registration;
#[path = "value_bindings/expression_traversal.rs"]
mod expression_traversal;
#[path = "value_bindings/rest_array_aliases.rs"]
pub(in crate::backend::direct_wasm) mod rest_array_aliases;
#[path = "value_bindings/statement_traversal.rs"]
mod statement_traversal;
