use super::*;

impl<'a> FunctionCompiler<'a> {
    pub(in crate::backend::direct_wasm) fn preserved_expression_kind(
        &self,
        preserved_kinds: &HashMap<String, StaticValueKind>,
        expression: &Expression,
    ) -> Option<StaticValueKind> {
        match expression {
            Expression::Identifier(name) => preserved_kinds
                .get(name)
                .copied()
                .or_else(|| self.infer_value_kind(expression)),
            Expression::Member { object, .. } if matches!(object.as_ref(), Expression::Identifier(name) if name.starts_with("__ayy_for_in_keys_")) => {
                Some(StaticValueKind::String)
            }
            Expression::Binary {
                op: BinaryOp::Add,
                left,
                right,
            } => {
                let left_kind = self.preserved_expression_kind(preserved_kinds, left);
                let right_kind = self.preserved_expression_kind(preserved_kinds, right);
                if left_kind == Some(StaticValueKind::String)
                    || right_kind == Some(StaticValueKind::String)
                {
                    Some(StaticValueKind::String)
                } else {
                    self.infer_value_kind(expression)
                }
            }
            _ => self.infer_value_kind(expression),
        }
    }

    pub(in crate::backend::direct_wasm) fn current_binding_kind_for_preservation(
        &self,
        name: &str,
    ) -> Option<StaticValueKind> {
        self.resolve_current_local_binding(name)
            .and_then(|(resolved_name, _)| {
                self.state
                    .speculation
                    .static_semantics
                    .local_kind(&resolved_name)
            })
            .or_else(|| self.state.speculation.static_semantics.local_kind(name))
            .or_else(|| {
                self.resolve_user_function_capture_hidden_name(name)
                    .and_then(|hidden_name| self.global_binding_kind(&hidden_name))
            })
            .or_else(|| self.global_binding_kind(name))
            .filter(|kind| *kind != StaticValueKind::Unknown)
    }

    pub(in crate::backend::direct_wasm) fn merge_preserved_binding_kind(
        &self,
        invalidated_bindings: &HashSet<String>,
        preserved_kinds: &mut HashMap<String, StaticValueKind>,
        blocked_bindings: &mut HashSet<String>,
        name: &str,
        candidate: Option<StaticValueKind>,
    ) {
        if !invalidated_bindings.contains(name) || blocked_bindings.contains(name) {
            return;
        }
        let Some(candidate) = candidate.filter(|kind| *kind != StaticValueKind::Unknown) else {
            preserved_kinds.remove(name);
            blocked_bindings.insert(name.to_string());
            return;
        };
        match preserved_kinds.get(name).copied() {
            Some(StaticValueKind::Undefined) if candidate != StaticValueKind::Undefined => {
                preserved_kinds.insert(name.to_string(), candidate);
            }
            Some(existing_kind) if existing_kind != candidate => {
                preserved_kinds.remove(name);
                blocked_bindings.insert(name.to_string());
            }
            Some(_) => {}
            None => {
                preserved_kinds.insert(name.to_string(), candidate);
            }
        }
    }
}
