use super::*;

thread_local! {
    static FUNCTION_BINDING_RESOLUTION_DEPTH: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static ACTIVE_FUNCTION_BINDING_RESOLUTION_SHAPES: RefCell<HashMap<String, u64>> = RefCell::new(HashMap::new());
}

pub(super) struct FunctionBindingResolutionGuard {
    _memo: crate::backend::direct_wasm::memo::ResolutionGuardScope,
}

impl FunctionBindingResolutionGuard {
    pub(super) fn enter(
        expression: &Expression,
        current_function_name: Option<&str>,
    ) -> Option<Self> {
        FUNCTION_BINDING_RESOLUTION_DEPTH.with(|depth| {
            let next = depth.get() + 1;
            if next > 256 {
                crate::backend::direct_wasm::memo::note_resolution_guard_block();
                if crate::ayy_env_flag!("AYY_TRACE_FUNCTION_BINDINGS") {
                    eprintln!(
                        "function_binding_resolution:depth_limit current_function={current_function_name:?} expression={expression:?}"
                    );
                }
                return None;
            }
            depth.set(next);
            Some(Self {
                _memo: crate::backend::direct_wasm::memo::ResolutionGuardScope::enter_class(2),
            })
        })
    }
}

pub(super) fn function_binding_resolution_is_active() -> bool {
    FUNCTION_BINDING_RESOLUTION_DEPTH.with(|depth| depth.get() > 0)
}

impl Drop for FunctionBindingResolutionGuard {
    fn drop(&mut self) {
        FUNCTION_BINDING_RESOLUTION_DEPTH.with(|depth| depth.set(depth.get().saturating_sub(1)));
    }
}

pub(super) struct FunctionBindingResolutionShapeGuard {
    key: String,
    _memo: crate::backend::direct_wasm::memo::ResolutionGuardScope,
}

impl FunctionBindingResolutionShapeGuard {
    pub(super) fn enter(
        expression: &Expression,
        current_function_name: Option<&str>,
    ) -> Option<Self> {
        let key = format!("{current_function_name:?}:{expression:?}");
        let conflict = ACTIVE_FUNCTION_BINDING_RESOLUTION_SHAPES.with(|active| {
            let mut active = active.borrow_mut();
            match active.get(&key) {
                Some(serial) => Some(*serial),
                None => {
                    active.insert(
                        key.clone(),
                        crate::backend::direct_wasm::memo::next_guard_serial(),
                    );
                    None
                }
            }
        });
        if let Some(serial) = conflict {
            crate::backend::direct_wasm::memo::note_resolution_guard_block_conflict(serial);
            return None;
        }
        Some(Self {
            key,
            _memo: crate::backend::direct_wasm::memo::ResolutionGuardScope::enter_class(2),
        })
    }
}

impl Drop for FunctionBindingResolutionShapeGuard {
    fn drop(&mut self) {
        ACTIVE_FUNCTION_BINDING_RESOLUTION_SHAPES.with(|active| {
            active.borrow_mut().remove(&self.key);
        });
    }
}
