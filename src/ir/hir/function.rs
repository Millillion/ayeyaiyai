use super::{Expression, Statement};

/// Name of the synthesized helper that drains an iterable through the
/// iterator protocol at runtime, collecting the produced values into an
/// array. Injected by the frontend lowering whenever a program contains a
/// spread call argument so the backend can route spread operands that are
/// not provably plain arrays through real GetIterator/IteratorStep
/// semantics.
pub const SPREAD_ITERATE_HELPER_NAME: &str = "__ayySpreadIterate";

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionDeclaration {
    pub name: String,
    pub top_level_binding: Option<String>,
    pub params: Vec<Parameter>,
    pub body: Vec<Statement>,
    pub register_global: bool,
    pub kind: FunctionKind,
    pub self_binding: Option<String>,
    pub mapped_arguments: bool,
    pub strict: bool,
    pub lexical_this: bool,
    pub constructible: bool,
    pub derived_constructor: bool,
    pub direct_eval_in_class_field_initializer: bool,
    pub length: usize,
    pub synthetic_capture_bindings: Vec<String>,
    pub immutable_class_bindings: Vec<String>,
    pub private_brand_binding: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Parameter {
    pub name: String,
    pub default: Option<Expression>,
    pub rest: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FunctionKind {
    Ordinary,
    Generator,
    Async,
    AsyncGenerator,
}

impl FunctionKind {
    pub fn from_flags(is_generator: bool, is_async: bool) -> Self {
        match (is_generator, is_async) {
            (false, false) => Self::Ordinary,
            (true, false) => Self::Generator,
            (false, true) => Self::Async,
            (true, true) => Self::AsyncGenerator,
        }
    }

    pub fn is_generator(self) -> bool {
        matches!(self, Self::Generator | Self::AsyncGenerator)
    }

    pub fn is_async(self) -> bool {
        matches!(self, Self::Async | Self::AsyncGenerator)
    }
}

#[cfg(test)]
mod tests {
    use super::FunctionKind;

    #[test]
    fn function_kind_from_flags_preserves_async_generator_shape() {
        let kind = FunctionKind::from_flags(true, true);
        assert_eq!(kind, FunctionKind::AsyncGenerator);
        assert!(kind.is_async());
        assert!(kind.is_generator());
    }
}
