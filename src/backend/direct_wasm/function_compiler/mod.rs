use super::*;

mod bindings;
mod calls;
mod core;
mod domains;
mod emission;
mod specialization;
mod support;
mod values;

pub(in crate::backend::direct_wasm) use self::support::*;

pub(in crate::backend::direct_wasm) fn reset_function_compiler_thread_locals() {
    values::reset_function_compiler_value_caches();
}

#[cfg(test)]
mod tests;
