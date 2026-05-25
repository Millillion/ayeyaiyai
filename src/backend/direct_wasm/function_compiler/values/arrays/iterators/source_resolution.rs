use super::*;

#[path = "source_resolution/local_source.rs"]
mod local_source;
#[path = "source_resolution/source_kind.rs"]
mod source_kind;

thread_local! {
    static ACTIVE_ITERATOR_SOURCE_SHAPES: RefCell<HashSet<String>> = RefCell::new(HashSet::new());
    static INTERNAL_ITERATOR_VALUE_SOURCE_CACHE: RefCell<HashMap<String, Option<IteratorSourceKind>>> = RefCell::new(HashMap::new());
}

struct IteratorSourceGuard {
    key: String,
}

impl Drop for IteratorSourceGuard {
    fn drop(&mut self) {
        ACTIVE_ITERATOR_SOURCE_SHAPES.with(|active| {
            active.borrow_mut().remove(&self.key);
        });
    }
}

pub(super) fn reset_iterator_source_caches() {
    ACTIVE_ITERATOR_SOURCE_SHAPES.with(|active| active.borrow_mut().clear());
    INTERNAL_ITERATOR_VALUE_SOURCE_CACHE.with(|cache| cache.borrow_mut().clear());
}
