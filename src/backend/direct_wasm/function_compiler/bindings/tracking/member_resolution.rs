use super::*;

thread_local! {
    static MEMBER_FUNCTION_BINDING_RESOLUTION_DEPTH: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static ACTIVE_MEMBER_BINDING_RESOLUTION_SHAPES: RefCell<HashSet<String>> = RefCell::new(HashSet::new());
}

struct MemberFunctionBindingResolutionGuard;

impl MemberFunctionBindingResolutionGuard {
    fn enter(object: &Expression, property: &Expression) -> Self {
        MEMBER_FUNCTION_BINDING_RESOLUTION_DEPTH.with(|depth| {
            let next = depth.get() + 1;
            if next > 256 {
                panic!(
                    "member function binding resolution recursion overflow: object={object:?}, property={property:?}"
                );
            }
            depth.set(next);
        });
        Self
    }
}

impl Drop for MemberFunctionBindingResolutionGuard {
    fn drop(&mut self) {
        MEMBER_FUNCTION_BINDING_RESOLUTION_DEPTH
            .with(|depth| depth.set(depth.get().saturating_sub(1)));
    }
}

struct MemberBindingResolutionShapeGuard {
    key: String,
}

impl MemberBindingResolutionShapeGuard {
    fn enter(kind: &str, object: &Expression, property: &Expression) -> Option<Self> {
        let key = format!("{kind}:{object:?}:{property:?}");
        let inserted = ACTIVE_MEMBER_BINDING_RESOLUTION_SHAPES
            .with(|active| active.borrow_mut().insert(key.clone()));
        inserted.then_some(Self { key })
    }
}

impl Drop for MemberBindingResolutionShapeGuard {
    fn drop(&mut self) {
        ACTIVE_MEMBER_BINDING_RESOLUTION_SHAPES.with(|active| {
            active.borrow_mut().remove(&self.key);
        });
    }
}

impl<'a> FunctionCompiler<'a> {
    pub(in crate::backend::direct_wasm) fn iterator_step_member_static_value_binding_candidates(
        &self,
        expression: &Expression,
    ) -> Vec<Expression> {
        let Expression::Member { object, property } = expression else {
            return Vec::new();
        };
        let property = self
            .resolve_property_key_expression(property)
            .unwrap_or_else(|| self.materialize_static_expression(property));
        if !matches!(property, Expression::String(ref name) if name == "value") {
            return Vec::new();
        }
        let Some(IteratorStepBinding::Runtime {
            static_value,
            value_candidates,
            ..
        }) = self.resolve_iterator_step_binding_from_expression(object)
        else {
            return Vec::new();
        };

        let mut candidates = Vec::new();
        let mut push_candidate = |candidate: Expression| {
            if static_expression_matches(&candidate, expression)
                || candidates
                    .iter()
                    .any(|existing| static_expression_matches(existing, &candidate))
            {
                return;
            }
            candidates.push(candidate);
        };

        if let Some(value) = static_value {
            push_candidate(value.clone());
            push_candidate(self.materialize_static_expression(&value));
        }
        if let [candidate] = value_candidates.as_slice() {
            push_candidate(candidate.clone());
            push_candidate(self.materialize_static_expression(candidate));
        }
        candidates
    }
}

mod accessor_bindings;
mod binding_entries;
mod function_bindings;
mod iterator_reads;
mod proxy_bindings;
mod scope_helpers;
