use super::*;

impl<'a> FunctionCompiler<'a> {
    pub(in crate::backend::direct_wasm) fn prototype_member_expression(name: &str) -> Expression {
        Expression::Member {
            object: Box::new(Expression::Identifier(name.to_string())),
            property: Box::new(Expression::String("prototype".to_string())),
        }
    }

    pub(in crate::backend::direct_wasm) fn generator_intrinsic_default_prototype_expression(
        kind: FunctionKind,
    ) -> Option<Expression> {
        let constructor_name = match kind {
            FunctionKind::Generator => "GeneratorFunction",
            FunctionKind::AsyncGenerator => "AsyncGeneratorFunction",
            _ => return None,
        };
        Some(Expression::Member {
            object: Box::new(Self::prototype_member_expression(constructor_name)),
            property: Box::new(Expression::String("prototype".to_string())),
        })
    }

    pub(in crate::backend::direct_wasm) fn builtin_constructor_object_prototype_expression(
        name: &str,
    ) -> Option<Expression> {
        if matches!(
            name,
            "AggregateError"
                | "SuppressedError"
                | "EvalError"
                | "RangeError"
                | "ReferenceError"
                | "SyntaxError"
                | "TypeError"
                | "URIError"
        ) {
            return Some(Expression::Identifier("Error".to_string()));
        }
        if builtin_identifier_kind(name) == Some(StaticValueKind::Function)
            || infer_call_result_kind(name).is_some()
        {
            return Some(Self::prototype_member_expression("Function"));
        }
        None
    }

    pub(in crate::backend::direct_wasm) fn builtin_prototype_object_prototype_expression(
        name: &str,
    ) -> Option<Expression> {
        if name == "Object" {
            return Some(Expression::Null);
        }
        if matches!(
            name,
            "AggregateError"
                | "SuppressedError"
                | "EvalError"
                | "RangeError"
                | "ReferenceError"
                | "SyntaxError"
                | "TypeError"
                | "URIError"
        ) {
            return Some(Self::prototype_member_expression("Error"));
        }
        if matches!(
            name,
            "AsyncFunction" | "GeneratorFunction" | "AsyncGeneratorFunction"
        ) {
            return Some(Self::prototype_member_expression("Function"));
        }
        if name == "Error"
            || builtin_identifier_kind(name) == Some(StaticValueKind::Function)
            || infer_call_result_kind(name).is_some()
        {
            return Some(Self::prototype_member_expression("Object"));
        }
        None
    }
}
