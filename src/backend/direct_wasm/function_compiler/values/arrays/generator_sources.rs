use super::*;

mod delegate_sources;
mod simple_generators;
mod substitution;

pub(super) fn reset_generator_source_caches() {
    simple_generators::reset_simple_generator_source_caches();
}
