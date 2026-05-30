use super::*;

impl<'a> FunctionCompiler<'a> {
    fn merge_dynamic_member_candidate_object_binding(
        target: &mut ObjectValueBinding,
        source: &ObjectValueBinding,
    ) {
        for (name, value) in &source.string_properties {
            object_binding_set_property(target, Expression::String(name.clone()), value.clone());
            object_binding_set_string_property_enumerable(
                target,
                name,
                !source
                    .non_enumerable_string_properties
                    .iter()
                    .any(|hidden_name| hidden_name == name),
            );
        }
        for (property, value) in &source.symbol_properties {
            object_binding_set_property(target, property.clone(), value.clone());
        }
        for (property, descriptor) in &source.property_descriptors {
            if let Some((_, existing)) = target
                .property_descriptors
                .iter_mut()
                .find(|(existing_property, _)| existing_property == property)
            {
                *existing = descriptor.clone();
            } else {
                target
                    .property_descriptors
                    .push((property.clone(), descriptor.clone()));
            }
        }
        target.runtime_symbol_properties |= source.runtime_symbol_properties;
        target.extensible |= source.extensible;
    }

    fn resolve_dynamic_member_object_binding_from_candidates(
        &self,
        object_binding: &ObjectValueBinding,
        property: &Expression,
    ) -> Option<ObjectValueBinding> {
        if static_property_name_from_expression(property).is_some()
            || argument_index_from_expression(property).is_some()
        {
            return None;
        }

        let mut merged = empty_object_value_binding();
        let mut found_candidate = false;
        for (_, value) in &object_binding.string_properties {
            let Some(candidate_binding) = self.resolve_object_binding_from_expression(value) else {
                continue;
            };
            Self::merge_dynamic_member_candidate_object_binding(&mut merged, &candidate_binding);
            found_candidate = true;
        }
        found_candidate.then_some(merged)
    }

    pub(super) fn resolve_member_object_binding(
        &self,
        expression: &Expression,
    ) -> Option<ObjectValueBinding> {
        let Expression::Member { object, property } = expression else {
            return None;
        };
        if matches!(property.as_ref(), Expression::String(name) if name == "global") {
            let realm_id = self.resolve_test262_realm_id_from_expression(object)?;
            return self.test262_realm_global_object_binding(realm_id);
        }
        if matches!(property.as_ref(), Expression::String(name) if name == "prototype") {
            if let Expression::Identifier(name) = object.as_ref()
                && let Some(prototype_binding) =
                    self.resolve_function_prototype_object_binding(name)
            {
                return Some(prototype_binding);
            }
        }

        let property = self
            .resolve_property_key_expression(property)
            .unwrap_or_else(|| self.materialize_static_expression(property));
        if let Some(value) =
            self.resolve_module_namespace_live_binding_member_value(object, &property)
        {
            if let Some(binding) = self.resolve_object_binding_from_expression(&value) {
                return Some(binding);
            }
        }
        if let Expression::Identifier(name) = object.as_ref()
            && let Some(module_index) = Self::module_index_from_namespace_like_identifier(name)
            && let Some(initializer) = self
                .resolve_static_dynamic_import_namespace_live_binding_member_initializer_value(
                    module_index,
                    &property,
                )
            && let Some(binding) = self.resolve_object_binding_from_expression(&initializer)
        {
            return Some(binding);
        }
        if let Some(IteratorStepBinding::Runtime {
            static_value,
            value_candidates,
            ..
        }) = self.resolve_iterator_step_binding_from_expression(object)
            && matches!(property, Expression::String(ref name) if name == "value")
        {
            if let Some(value) = static_value {
                return self.resolve_object_binding_from_expression(&value);
            }
            if let [candidate] = value_candidates.as_slice() {
                let materialized = self.materialize_static_expression(candidate);
                return self.resolve_object_binding_from_expression(&materialized);
            }
        }
        if let Some(index) = argument_index_from_expression(&property)
            && let Some(array_binding) = self.resolve_array_binding_from_expression(object)
            && let Some(Some(value)) = array_binding.values.get(index as usize)
        {
            return self.resolve_object_binding_from_expression(value);
        }
        if !self.runtime_object_property_shadow_deletion_is_statically_present(object, &property)
            && let Some(shadow_binding_name) =
                self.runtime_object_property_shadow_binding_name_for_expression(object, &property)
            && let Some(shadow_value) = self
                .global_value_binding(&shadow_binding_name)
                .cloned()
                .or_else(|| {
                    self.backend
                        .shared_global_semantics
                        .values
                        .value_bindings
                        .get(&shadow_binding_name)
                        .cloned()
                })
            && let Some(shadow_object_binding) =
                self.resolve_object_binding_from_expression(&shadow_value)
        {
            return Some(shadow_object_binding);
        }
        if let Some(object_binding) = self.resolve_object_binding_from_expression(object)
            && let Some(value) =
                self.resolve_object_binding_property_value(&object_binding, &property)
        {
            return self.resolve_object_binding_from_expression(&value);
        }
        if let Some(object_binding) = self.resolve_object_binding_from_expression(object)
            && let Some(dynamic_binding) = self
                .resolve_dynamic_member_object_binding_from_candidates(&object_binding, &property)
        {
            return Some(dynamic_binding);
        }
        None
    }
}
