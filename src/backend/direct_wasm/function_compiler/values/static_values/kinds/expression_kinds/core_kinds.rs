use super::*;

const INFER_VALUE_KIND_RECURSION_LIMIT: usize = 128;

thread_local! {
    static INFER_VALUE_KIND_DEPTH: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

struct InferValueKindDepthGuard;

impl InferValueKindDepthGuard {
    fn enter() -> Option<Self> {
        INFER_VALUE_KIND_DEPTH.with(|depth| {
            let current = depth.get();
            if current >= INFER_VALUE_KIND_RECURSION_LIMIT {
                return None;
            }
            depth.set(current + 1);
            Some(Self)
        })
    }
}

impl Drop for InferValueKindDepthGuard {
    fn drop(&mut self) {
        INFER_VALUE_KIND_DEPTH.with(|depth| depth.set(depth.get().saturating_sub(1)));
    }
}

#[path = "core_kinds/compound.rs"]
mod compound;
#[path = "core_kinds/primitives.rs"]
mod primitives;

impl<'a> FunctionCompiler<'a> {
    pub(in crate::backend::direct_wasm) fn infer_value_kind(
        &self,
        expression: &Expression,
    ) -> Option<StaticValueKind> {
        let Some(_depth_guard) = InferValueKindDepthGuard::enter() else {
            return Some(StaticValueKind::Unknown);
        };
        match expression {
            Expression::Number(_)
            | Expression::BigInt(_)
            | Expression::String(_)
            | Expression::Bool(_)
            | Expression::Null
            | Expression::Undefined
            | Expression::Identifier(_)
            | Expression::Unary { .. }
            | Expression::Binary { .. }
            | Expression::Conditional { .. } => self.infer_primitive_expression_kind(expression),
            _ => self.infer_compound_expression_kind(expression),
        }
    }
}
