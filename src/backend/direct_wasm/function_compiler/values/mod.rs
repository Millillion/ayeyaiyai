use super::*;

mod arrays;
mod object_state;
mod static_values;
mod strings;
mod typed_arrays;

pub(super) fn reset_function_compiler_value_caches() {
    arrays::reset_array_value_caches();
}
