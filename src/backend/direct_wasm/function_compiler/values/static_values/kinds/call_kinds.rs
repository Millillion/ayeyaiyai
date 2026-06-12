use super::*;

use crate::ir::hir::SPREAD_ITERATE_HELPER_NAME;

fn spread_expression_is_effect_free_to_reemit(expression: &Expression) -> bool {
    matches!(
        expression,
        Expression::Identifier(_)
            | Expression::Number(_)
            | Expression::String(_)
            | Expression::Bool(_)
            | Expression::BigInt(_)
            | Expression::Null
            | Expression::Undefined
    )
}

fn spread_operand_assignment_effects(expression: &Expression) -> bool {
    matches!(
        expression,
        Expression::Assign { .. }
            | Expression::AssignMember { .. }
            | Expression::AssignSuperMember { .. }
    )
}

impl<'a> FunctionCompiler<'a> {
    pub(in crate::backend::direct_wasm) fn spread_iterate_runtime_call(
        expression: &Expression,
    ) -> Expression {
        Expression::Call {
            callee: Box::new(Expression::Identifier(SPREAD_ITERATE_HELPER_NAME.to_string())),
            arguments: vec![CallArgument::Expression(expression.clone())],
        }
    }

    pub(in crate::backend::direct_wasm) fn expand_call_arguments(
        &self,
        arguments: &[CallArgument],
    ) -> Vec<Expression> {
        let mut expanded = Vec::new();
        for argument in arguments {
            match argument {
                CallArgument::Expression(expression) => expanded.push(expression.clone()),
                CallArgument::Spread(expression) => {
                    if let Some(binding) = self.resolve_array_binding_from_expression(expression) {
                        let values = binding
                            .values
                            .into_iter()
                            .map(|value| value.unwrap_or(Expression::Undefined))
                            .collect::<Vec<_>>();
                        // The spread operand must still be evaluated once for
                        // its side effects (e.g. `f(...target = source)`),
                        // even though the resulting argument values are known
                        // statically. Attach the operand evaluation to the
                        // first expanded value when this can be done without
                        // double-evaluating any effectful value expression.
                        if spread_operand_assignment_effects(expression)
                            && let Some((first, rest)) = values.split_first()
                            && values
                                .iter()
                                .all(spread_expression_is_effect_free_to_reemit)
                        {
                            expanded.push(Expression::Sequence(vec![
                                expression.clone(),
                                first.clone(),
                            ]));
                            expanded.extend(rest.iter().cloned());
                        } else {
                            expanded.extend(values);
                        }
                    } else if self.user_function(SPREAD_ITERATE_HELPER_NAME).is_some() {
                        // Not provably a plain array: route the operand
                        // through the runtime iterator protocol so GetMethod
                        // (@@iterator) lookups, `next()` calls, and the
                        // errors they raise are observable.
                        expanded.push(Self::spread_iterate_runtime_call(expression));
                    } else {
                        expanded.push(expression.clone());
                    }
                }
            }
        }
        expanded
    }

    pub(in crate::backend::direct_wasm) fn infer_call_result_kind(
        &self,
        name: &str,
    ) -> Option<StaticValueKind> {
        if let Some(target_name) = parse_bound_function_prototype_call_builtin_name(name) {
            return match target_name {
                "Array.prototype.join" | "Array.prototype.toString" => {
                    Some(StaticValueKind::String)
                }
                "Array.prototype.push" => Some(StaticValueKind::Number),
                "Object.prototype.hasOwnProperty" | "Object.prototype.propertyIsEnumerable" => {
                    Some(StaticValueKind::Bool)
                }
                _ => None,
            };
        }
        match name {
            "Number" => Some(StaticValueKind::Number),
            "String" => Some(StaticValueKind::String),
            "Date" => Some(StaticValueKind::String),
            "Boolean" => Some(StaticValueKind::Bool),
            "isNaN" => Some(StaticValueKind::Bool),
            "Reflect.has" => Some(StaticValueKind::Bool),
            "Reflect.deleteProperty" => Some(StaticValueKind::Bool),
            "Proxy.revocable" => Some(StaticValueKind::Object),
            "Object" | "Array" | "ArrayBuffer" | "SharedArrayBuffer" | "DataView" | "RegExp"
            | "Map" | "Set" | "Error" | "EvalError" | "RangeError" | "ReferenceError"
            | "SyntaxError" | "TypeError" | "URIError" | "AggregateError" | "SuppressedError"
            | "Promise" | "__ayyDynamicImport" | "__ayyImportMeta" | "WeakMap" | "WeakRef"
            | "WeakSet" => Some(StaticValueKind::Object),
            "Uint8Array" | "Int8Array" | "Uint16Array" | "Int16Array" | "Uint32Array"
            | "Int32Array" | "Float32Array" | "Float64Array" | "Uint8ClampedArray"
            | "BigInt64Array" | "BigUint64Array" => Some(StaticValueKind::Object),
            "BigInt" => Some(StaticValueKind::BigInt),
            "Symbol" => Some(StaticValueKind::Symbol),
            "Function" => Some(StaticValueKind::Function),
            _ => None,
        }
    }
}
