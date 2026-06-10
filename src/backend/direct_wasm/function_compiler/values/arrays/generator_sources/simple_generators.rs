use super::*;

mod call_frame_substitution;
mod source_resolution;
mod static_iterables;

type SimpleGeneratorSourceParts = (
    Vec<Statement>,
    Vec<SimpleGeneratorStep>,
    Vec<Statement>,
    Expression,
);

thread_local! {
    static ACTIVE_SIMPLE_GENERATOR_SOURCE_SHAPES: RefCell<HashSet<String>> = RefCell::new(HashSet::new());
    static SIMPLE_GENERATOR_SOURCE_CACHE: RefCell<HashMap<String, Option<SimpleGeneratorSourceParts>>> = RefCell::new(HashMap::new());
}

struct SimpleGeneratorSourceGuard {
    key: String,
    _memo: crate::backend::direct_wasm::memo::ResolutionGuardScope,
}

impl SimpleGeneratorSourceGuard {
    fn enter_key(key: &str) -> Option<Self> {
        let inserted = ACTIVE_SIMPLE_GENERATOR_SOURCE_SHAPES
            .with(|active| active.borrow_mut().insert(key.to_string()));
        if !inserted {
            crate::backend::direct_wasm::memo::note_resolution_guard_block();
        }
        inserted.then_some(Self {
            key: key.to_string(),
            _memo: crate::backend::direct_wasm::memo::ResolutionGuardScope::enter_class(19),
        })
    }
}

impl Drop for SimpleGeneratorSourceGuard {
    fn drop(&mut self) {
        ACTIVE_SIMPLE_GENERATOR_SOURCE_SHAPES.with(|active| {
            active.borrow_mut().remove(&self.key);
        });
    }
}

fn simple_generator_source_cache_key(
    kind: &str,
    function: &FunctionDeclaration,
    expression: &Expression,
    environment_key: &str,
) -> String {
    let expression_hash = crate::backend::direct_wasm::memo::expression_structural_hash(expression);
    format!(
        "{kind}:{expression_hash:032x}:{}:env:{environment_key}",
        function.name
    )
}

fn lookup_simple_generator_source_cache(key: &str) -> Option<Option<SimpleGeneratorSourceParts>> {
    SIMPLE_GENERATOR_SOURCE_CACHE.with(|cache| cache.borrow().get(key).cloned())
}

fn store_simple_generator_source_cache(key: String, value: Option<SimpleGeneratorSourceParts>) {
    SIMPLE_GENERATOR_SOURCE_CACHE.with(|cache| {
        cache.borrow_mut().insert(key, value);
    });
}

pub(super) fn reset_simple_generator_source_caches() {
    ACTIVE_SIMPLE_GENERATOR_SOURCE_SHAPES.with(|active| active.borrow_mut().clear());
    SIMPLE_GENERATOR_SOURCE_CACHE.with(|cache| cache.borrow_mut().clear());
}
