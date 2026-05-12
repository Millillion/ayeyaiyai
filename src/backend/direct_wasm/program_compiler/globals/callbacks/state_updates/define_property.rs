use super::*;

impl DirectWasmCompiler {
    pub(super) fn define_property_result_object_binding_for_parameter_state(
        &self,
        expression: &Expression,
        value_bindings: &HashMap<String, Expression>,
        object_bindings: &HashMap<String, ObjectValueBinding>,
    ) -> Option<ObjectValueBinding> {
        let (target, property, descriptor) = self.define_property_call_parts(expression)?;
        let property =
            self.materialize_callback_state_expression(property, value_bindings, object_bindings);
        let mut value_state = value_bindings.clone();
        let mut object_state = object_bindings.clone();
        let materialized_target =
            self.materialize_callback_state_expression(target, value_bindings, object_bindings);
        let mut object_binding = self
            .infer_global_object_binding_with_state(
                &materialized_target,
                &mut value_state,
                &mut object_state,
            )
            .or_else(|| {
                self.infer_global_object_binding_with_state(
                    target,
                    &mut value_state,
                    &mut object_state,
                )
            })
            .unwrap_or_else(empty_object_value_binding);
        let descriptor_binding = self.property_descriptor_binding_for_parameter_state(
            &object_binding,
            &property,
            &descriptor,
            value_bindings,
            object_bindings,
        );
        object_binding_define_property_descriptor(
            &mut object_binding,
            property,
            descriptor_binding,
        );
        Some(object_binding)
    }

    pub(super) fn update_parameter_binding_state_from_define_property_call(
        &self,
        expression: &Expression,
        value_bindings: &mut HashMap<String, Expression>,
        object_bindings: &mut HashMap<String, ObjectValueBinding>,
    ) {
        let Some((target, property, descriptor)) = self.define_property_call_parts(expression)
        else {
            return;
        };
        let Expression::Identifier(name) = target else {
            return;
        };

        let property =
            self.materialize_callback_state_expression(property, value_bindings, object_bindings);
        let mut object_binding = object_bindings
            .get(name)
            .cloned()
            .unwrap_or_else(empty_object_value_binding);
        let descriptor_binding = self.property_descriptor_binding_for_parameter_state(
            &object_binding,
            &property,
            &descriptor,
            value_bindings,
            object_bindings,
        );
        object_binding_define_property_descriptor(
            &mut object_binding,
            property,
            descriptor_binding,
        );
        object_bindings.insert(name.clone(), object_binding);
    }

    fn define_property_call_parts<'a>(
        &self,
        expression: &'a Expression,
    ) -> Option<(&'a Expression, &'a Expression, PropertyDescriptorDefinition)> {
        let Expression::Call { callee, arguments } = expression else {
            return None;
        };
        let Expression::Member { object, property } = callee.as_ref() else {
            return None;
        };
        if !matches!(object.as_ref(), Expression::Identifier(name) if name == "Object")
            || !matches!(property.as_ref(), Expression::String(name) if name == "defineProperty")
        {
            return None;
        }
        let [
            CallArgument::Expression(target),
            CallArgument::Expression(property),
            CallArgument::Expression(descriptor_expression),
            ..,
        ] = arguments.as_slice()
        else {
            return None;
        };
        let descriptor = resolve_property_descriptor_definition(descriptor_expression)?;
        Some((target, property, descriptor))
    }

    fn property_descriptor_binding_for_parameter_state(
        &self,
        object_binding: &ObjectValueBinding,
        property: &Expression,
        descriptor: &PropertyDescriptorDefinition,
        value_bindings: &HashMap<String, Expression>,
        object_bindings: &HashMap<String, ObjectValueBinding>,
    ) -> PropertyDescriptorBinding {
        let property_name = static_property_name_from_expression(property);
        let existing_value = object_binding_lookup_value(object_binding, property).cloned();
        let existing_descriptor =
            object_binding_lookup_descriptor(object_binding, property).cloned();
        let current_enumerable = property_name.as_ref().is_some_and(|property_name| {
            !object_binding
                .non_enumerable_string_properties
                .iter()
                .any(|hidden_name| hidden_name == property_name)
        });
        let enumerable = descriptor.enumerable.unwrap_or_else(|| {
            existing_descriptor
                .as_ref()
                .map(|descriptor| descriptor.enumerable)
                .unwrap_or(current_enumerable)
        });
        let configurable = descriptor.configurable.unwrap_or_else(|| {
            existing_descriptor
                .as_ref()
                .map(|descriptor| descriptor.configurable)
                .unwrap_or(false)
        });
        if descriptor.is_accessor() {
            return PropertyDescriptorBinding {
                value: None,
                configurable,
                enumerable,
                writable: None,
                getter: descriptor.getter.as_ref().map(|expression| {
                    self.materialize_callback_state_expression(
                        expression,
                        value_bindings,
                        object_bindings,
                    )
                }),
                setter: descriptor.setter.as_ref().map(|expression| {
                    self.materialize_callback_state_expression(
                        expression,
                        value_bindings,
                        object_bindings,
                    )
                }),
                has_get: descriptor.getter.is_some(),
                has_set: descriptor.setter.is_some(),
            };
        }

        let value = descriptor
            .value
            .as_ref()
            .map(|expression| {
                self.materialize_callback_state_expression(
                    expression,
                    value_bindings,
                    object_bindings,
                )
            })
            .or(existing_value)
            .or_else(|| {
                existing_descriptor
                    .as_ref()
                    .and_then(|descriptor| descriptor.value.clone())
            })
            .unwrap_or(Expression::Undefined);
        let writable = descriptor.writable.or_else(|| {
            existing_descriptor
                .as_ref()
                .and_then(|descriptor| descriptor.writable)
        });

        PropertyDescriptorBinding {
            value: Some(value),
            configurable,
            enumerable,
            writable: Some(writable.unwrap_or(false)),
            getter: None,
            setter: None,
            has_get: false,
            has_set: false,
        }
    }
}
