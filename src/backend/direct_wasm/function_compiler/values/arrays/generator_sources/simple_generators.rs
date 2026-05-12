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
}

impl SimpleGeneratorSourceGuard {
    fn enter_key(key: &str) -> Option<Self> {
        let inserted = ACTIVE_SIMPLE_GENERATOR_SOURCE_SHAPES
            .with(|active| active.borrow_mut().insert(key.to_string()));
        inserted.then_some(Self {
            key: key.to_string(),
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
) -> String {
    format!("{kind}:{expression:?}:{function:?}")
}

fn lookup_simple_generator_source_cache(key: &str) -> Option<Option<SimpleGeneratorSourceParts>> {
    SIMPLE_GENERATOR_SOURCE_CACHE.with(|cache| cache.borrow().get(key).cloned())
}

fn store_simple_generator_source_cache(key: String, value: Option<SimpleGeneratorSourceParts>) {
    SIMPLE_GENERATOR_SOURCE_CACHE.with(|cache| {
        cache.borrow_mut().insert(key, value);
    });
}
