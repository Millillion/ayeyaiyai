use super::*;

impl<'a> FunctionCompiler<'a> {
    pub(super) fn infer_object_member_kind(
        &self,
        object: &Expression,
        property: &Expression,
    ) -> Option<StaticValueKind> {
        let materialized_property = self.materialize_static_expression(property);
        if self.runtime_object_property_shadow_deletion_is_statically_present(
            object,
            &materialized_property,
        ) {
            return Some(StaticValueKind::Undefined);
        }
        if self.runtime_object_property_shadow_deletion_may_affect_property(
            object,
            &materialized_property,
        ) {
            return Some(StaticValueKind::Unknown);
        }
        if let Some(array_binding) = self.resolve_array_binding_from_expression(object) {
            if matches!(property, Expression::String(name) if name == "length") {
                return Some(StaticValueKind::Number);
            }
            if let Some(index) = argument_index_from_expression(property) {
                return array_binding
                    .values
                    .get(index as usize)
                    .and_then(|value| value.as_ref())
                    .and_then(|value| self.infer_value_kind(value))
                    .or(Some(StaticValueKind::Undefined));
            }
            if (matches!(object, Expression::Identifier(name) if name.starts_with("__ayy_for_in_keys_"))
                || self.expression_depends_on_active_loop_assignment(property))
                && !array_binding.values.is_empty()
            {
                let mut common_kind = None;
                for value in &array_binding.values {
                    let value_kind = value
                        .as_ref()
                        .and_then(|value| self.infer_value_kind(value))?;
                    if common_kind
                        .replace(value_kind)
                        .is_some_and(|previous_kind| previous_kind != value_kind)
                    {
                        return Some(StaticValueKind::Unknown);
                    }
                }
                if let Some(kind) = common_kind {
                    return Some(kind);
                }
            }
        }
        if let Some(object_binding) = self.resolve_object_binding_from_expression(object) {
            return object_binding_lookup_value(&object_binding, &materialized_property)
                .and_then(|value| self.infer_value_kind(value))
                .or(Some(StaticValueKind::Undefined));
        }
        if let Expression::String(_) = object {
            if matches!(property, Expression::String(name) if name == "length") {
                return Some(StaticValueKind::Number);
            }
            if argument_index_from_expression(property).is_some() {
                return Some(StaticValueKind::String);
            }
        }
        None
    }
}
