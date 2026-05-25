use super::*;

impl DirectWasmCompiler {
    fn resolve_global_class_prototype_parent_expression(
        &self,
        expression: &Expression,
    ) -> Expression {
        match expression {
            Expression::Identifier(alias) => self
                .resolve_static_class_init_local_alias_expression(alias)
                .or_else(|| {
                    self.global_value_binding(alias)
                        .filter(|value| !static_expression_matches(value, expression))
                        .cloned()
                })
                .map(|resolved| self.resolve_global_class_prototype_parent_expression(&resolved))
                .unwrap_or_else(|| expression.clone()),
            Expression::Member { object, property } => Expression::Member {
                object: Box::new(self.resolve_global_class_prototype_parent_expression(object)),
                property: Box::new(self.materialize_global_expression(property)),
            },
            Expression::Sequence(expressions) => expressions
                .last()
                .map(|expression| self.resolve_global_class_prototype_parent_expression(expression))
                .unwrap_or(Expression::Undefined),
            _ => self.materialize_global_expression(expression),
        }
    }

    pub(in crate::backend::direct_wasm) fn update_global_object_prototype_binding(
        &mut self,
        name: &str,
        prototype: &Expression,
    ) {
        let prototype = match prototype {
            Expression::Identifier(alias) => self
                .resolve_static_class_init_local_alias_expression(alias)
                .unwrap_or_else(|| prototype.clone()),
            Expression::Member { property, .. } if matches!(property.as_ref(), Expression::String(property_name) if property_name == "prototype") => {
                prototype.clone()
            }
            _ => self.materialize_global_expression(prototype),
        };
        self.update_global_object_prototype_expression(name, prototype);
    }

    pub(in crate::backend::direct_wasm) fn update_global_object_prototype_binding_from_value(
        &mut self,
        name: &str,
        value: &Expression,
    ) {
        let object_prototype = || Expression::Member {
            object: Box::new(Expression::Identifier("Object".to_string())),
            property: Box::new(Expression::String("prototype".to_string())),
        };
        let prototype_from_value = |_compiler: &Self, expression: &Expression| {
            if matches!(expression, Expression::Object(_)) {
                return Some(
                    object_literal_prototype_expression(expression)
                        .unwrap_or_else(object_prototype),
                );
            }
            let Expression::New { callee, .. } = expression else {
                return None;
            };
            let Expression::Identifier(constructor_name) = callee.as_ref() else {
                return None;
            };
            Some(Expression::Member {
                object: Box::new(Expression::Identifier(constructor_name.clone())),
                property: Box::new(Expression::String("prototype".to_string())),
            })
        };
        let prototype = prototype_from_value(self, value).or_else(|| {
            let materialized = self.materialize_global_expression(value);
            if static_expression_matches(&materialized, value) {
                return None;
            }
            prototype_from_value(self, &materialized)
        });
        if let Some(prototype) = prototype {
            self.update_global_object_prototype_binding(name, &prototype);
        }
    }

    pub(in crate::backend::direct_wasm) fn record_global_runtime_prototype_variant(
        &mut self,
        name: &str,
        prototype: Option<&Expression>,
    ) {
        let prototype = prototype.map(|expression| match expression {
            Expression::Identifier(alias) => self
                .resolve_static_class_init_local_alias_expression(alias)
                .map(|resolved| self.materialize_global_expression(&resolved))
                .unwrap_or_else(|| self.materialize_global_expression(expression)),
            _ => self.materialize_global_expression(expression),
        });
        self.record_runtime_prototype_variant(name, prototype);
    }

    pub(in crate::backend::direct_wasm) fn update_global_expression_metadata(
        &mut self,
        expression: &Expression,
    ) {
        match expression {
            Expression::AssignMember {
                object,
                property,
                value,
            } => {
                self.update_global_member_assignment_metadata(object, property, value);
            }
            Expression::Sequence(expressions) => {
                for expression in expressions {
                    self.update_global_expression_metadata(expression);
                }
            }
            Expression::Call { callee, arguments } => {
                if matches!(callee.as_ref(), Expression::Identifier(name) if name == "__ayyClassPrototypeInit")
                {
                    let [
                        CallArgument::Expression(Expression::Identifier(target_name)),
                        CallArgument::Expression(prototype_parent),
                        ..,
                    ] = arguments.as_slice()
                    else {
                        return;
                    };
                    let prototype_parent =
                        self.resolve_global_class_prototype_parent_expression(prototype_parent);
                    let mut target_names = vec![target_name.clone()];
                    if let Some(Expression::Identifier(alias)) =
                        self.resolve_static_class_init_local_alias_expression(target_name)
                        && !target_names.contains(&alias)
                    {
                        target_names.push(alias);
                    }
                    if let Some(Expression::Identifier(alias)) =
                        self.global_value_binding(target_name)
                        && !target_names.contains(alias)
                    {
                        target_names.push(alias.clone());
                    }
                    for target_name in target_names {
                        self.update_global_object_prototype_expression(
                            &format!("{target_name}.prototype"),
                            prototype_parent.clone(),
                        );
                    }
                    return;
                }
                let Expression::Member { object, property } = callee.as_ref() else {
                    return;
                };
                if !matches!(object.as_ref(), Expression::Identifier(name) if name == "Object")
                    || !matches!(
                        property.as_ref(),
                        Expression::String(name)
                            if name == "setPrototypeOf" || name == "defineProperty"
                    )
                {
                    return;
                }
                if matches!(property.as_ref(), Expression::String(name) if name == "defineProperty")
                {
                    let [
                        CallArgument::Expression(target),
                        CallArgument::Expression(property),
                        CallArgument::Expression(descriptor_expression),
                        ..,
                    ] = arguments.as_slice()
                    else {
                        return;
                    };
                    let Some(descriptor) =
                        resolve_property_descriptor_definition(descriptor_expression)
                    else {
                        return;
                    };
                    let target_name = match target {
                        Expression::Identifier(name) => Some((name.as_str(), false)),
                        Expression::Member { object, property } if matches!(property.as_ref(), Expression::String(name) if name == "prototype") =>
                        {
                            let Expression::Identifier(name) = object.as_ref() else {
                                return;
                            };
                            Some((name.as_str(), true))
                        }
                        _ => None,
                    };
                    let Some((target_name, is_prototype)) = target_name else {
                        return;
                    };
                    let home_object_name = if is_prototype {
                        format!("{target_name}.prototype")
                    } else {
                        target_name.to_string()
                    };
                    let property = self.canonical_global_object_property_expression(property);
                    if static_property_name_from_expression(&property).is_none()
                        && self
                            .global_member_function_binding_property(&property)
                            .is_none()
                    {
                        return;
                    }
                    let property_name = static_property_name_from_expression(&property);
                    let existing_value = if is_prototype {
                        self.global_prototype_object_binding(target_name)
                            .and_then(|object_binding| {
                                object_binding_lookup_value(object_binding, &property)
                            })
                            .cloned()
                    } else {
                        self.global_object_binding(target_name)
                            .and_then(|object_binding| {
                                object_binding_lookup_value(object_binding, &property)
                            })
                            .cloned()
                    };
                    let current_enumerable = property_name.as_ref().is_some_and(|property_name| {
                        let binding = if is_prototype {
                            self.global_prototype_object_binding(target_name)
                        } else {
                            self.global_object_binding(target_name)
                        };
                        binding
                            .map(|object_binding| {
                                !object_binding
                                    .non_enumerable_string_properties
                                    .iter()
                                    .any(|hidden_name| hidden_name == property_name)
                            })
                            .unwrap_or(false)
                    });
                    let enumerable = descriptor.enumerable.unwrap_or(current_enumerable);
                    let value = if descriptor.is_accessor() {
                        Expression::Undefined
                    } else {
                        descriptor
                            .value
                            .as_ref()
                            .map(|expression| self.materialize_global_expression(expression))
                            .or(existing_value)
                            .unwrap_or(Expression::Undefined)
                    };
                    if is_prototype {
                        self.define_global_prototype_object_property(
                            target_name,
                            property.clone(),
                            value,
                            enumerable,
                        );
                    } else {
                        self.define_global_object_property(
                            target_name,
                            property.clone(),
                            value,
                            enumerable,
                        );
                    }

                    let Some(key) = self.global_member_function_binding_key(target, &property)
                    else {
                        return;
                    };
                    let has_value_field = descriptor.value.is_some();
                    let has_get_field = descriptor.getter.is_some();
                    let has_set_field = descriptor.setter.is_some();
                    if let Some(binding) = descriptor
                        .value
                        .as_ref()
                        .and_then(|expression| self.infer_global_function_binding(expression))
                    {
                        self.update_user_function_home_object_binding(
                            binding.clone(),
                            &home_object_name,
                        );
                        self.set_global_member_function_binding(key.clone(), binding);
                    } else if has_value_field {
                        self.clear_global_member_function_binding(&key);
                    }
                    if let Some(binding) = descriptor
                        .getter
                        .as_ref()
                        .and_then(|expression| self.infer_global_function_binding(expression))
                    {
                        self.update_user_function_home_object_binding(
                            binding.clone(),
                            &home_object_name,
                        );
                        self.set_global_member_getter_binding(key.clone(), binding);
                    } else if has_get_field {
                        self.clear_global_member_getter_binding(&key);
                    }
                    if let Some(binding) = descriptor
                        .setter
                        .as_ref()
                        .and_then(|expression| self.infer_global_function_binding(expression))
                    {
                        self.update_user_function_home_object_binding(
                            binding.clone(),
                            &home_object_name,
                        );
                        self.set_global_member_setter_binding(key, binding);
                    } else if has_set_field {
                        self.clear_global_member_setter_binding(&key);
                    }
                    return;
                }
                let [
                    CallArgument::Expression(Expression::Identifier(target_name)),
                    CallArgument::Expression(prototype),
                    ..,
                ] = arguments.as_slice()
                else {
                    return;
                };
                let mut target_names = vec![target_name.clone()];
                if let Some(Expression::Identifier(alias)) =
                    self.global_value_binding(target_name).cloned()
                    && !target_names.contains(&alias)
                {
                    target_names.push(alias);
                }
                if let Some(Expression::Identifier(alias)) =
                    self.resolve_static_class_init_local_alias_expression(target_name)
                    && !target_names.contains(&alias)
                {
                    target_names.push(alias);
                }
                for target_name in target_names {
                    let has_runtime_prototype = self
                        .state
                        .global_semantics
                        .values
                        .runtime_prototype_binding(&target_name)
                        .is_some();
                    if !self.global_has_binding(&target_name)
                        && !has_runtime_prototype
                        && self
                            .global_object_prototype_expression(&target_name)
                            .is_none()
                    {
                        continue;
                    }
                    let runtime_prototype = if matches!(prototype, Expression::Null) {
                        None
                    } else {
                        Some(prototype)
                    };
                    self.record_global_runtime_prototype_variant(&target_name, runtime_prototype);
                    self.update_global_object_prototype_binding(&target_name, prototype);
                }
            }
            _ => {}
        }
    }
}
