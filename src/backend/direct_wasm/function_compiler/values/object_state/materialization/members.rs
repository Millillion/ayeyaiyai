use super::*;

impl<'a> FunctionCompiler<'a> {
    pub(super) fn materialize_get_prototype_of_constructor_member(
        &self,
        object: &Expression,
        property: &Expression,
    ) -> Option<Expression> {
        if !matches!(property, Expression::String(name) if name == "constructor") {
            return None;
        }

        let Expression::Call { callee, arguments } = object else {
            return None;
        };
        if !matches!(
            callee.as_ref(),
            Expression::Member { object, property }
                if matches!(object.as_ref(), Expression::Identifier(name) if name == "Object")
                    && matches!(property.as_ref(), Expression::String(name) if name == "getPrototypeOf")
        ) {
            return None;
        }

        let Some(CallArgument::Expression(target) | CallArgument::Spread(target)) =
            arguments.first()
        else {
            return None;
        };
        let binding = self.resolve_function_binding_from_expression(target)?;
        let constructor_name = match binding {
            LocalFunctionBinding::User(function_name) => {
                let function = self.user_function(&function_name)?;
                match function.kind {
                    FunctionKind::Ordinary => "Function",
                    FunctionKind::Generator => "GeneratorFunction",
                    FunctionKind::Async => "AsyncFunction",
                    FunctionKind::AsyncGenerator => "AsyncGeneratorFunction",
                }
            }
            LocalFunctionBinding::Builtin(_) => "Function",
        };
        Some(Expression::Identifier(constructor_name.to_string()))
    }

    pub(in crate::backend::direct_wasm) fn iterator_step_static_value_requires_runtime_read(
        &self,
        value: &Expression,
    ) -> bool {
        let mut referenced_names = HashSet::new();
        collect_referenced_binding_names_from_expression(value, &mut referenced_names);
        referenced_names.iter().any(|name| {
            let source_name = scoped_binding_source_name(name).unwrap_or(name);
            self.resolve_current_local_binding(source_name).is_none()
                && (self.global_has_binding(source_name)
                    || self.global_has_implicit_binding(source_name)
                    || self
                        .resolve_user_function_capture_hidden_name(source_name)
                        .is_some())
        })
    }

    pub(in crate::backend::direct_wasm) fn materialize_member_expression(
        &self,
        object: &Expression,
        property: &Expression,
    ) -> Expression {
        if let Some(constructor) =
            self.materialize_get_prototype_of_constructor_member(object, property)
        {
            return constructor;
        }

        let resolved_object = self
            .resolve_bound_alias_expression(object)
            .filter(|resolved| !static_expression_matches(resolved, object));
        let resolved_property = self.resolve_property_key_expression(property).or_else(|| {
            self.resolve_bound_alias_expression(property)
                .filter(|resolved| !static_expression_matches(resolved, property))
        });
        if matches!(property, Expression::String(name) if name == "prototype") {
            let mut materialized_object = self.materialize_static_expression(object);
            if let Some(LocalFunctionBinding::Builtin(function_name)) =
                self.resolve_function_binding_from_expression(&materialized_object)
                && is_function_constructor_builtin(&function_name)
                && function_name != FUNCTION_CONSTRUCTOR_FAMILY_BUILTIN
            {
                materialized_object = Expression::Identifier(function_name);
            }
            if matches!(
                materialized_object,
                Expression::Identifier(_) | Expression::New { .. }
            ) {
                return Expression::Member {
                    object: Box::new(materialized_object),
                    property: Box::new(Expression::String("prototype".to_string())),
                };
            }
        }
        if !self.expression_depends_on_active_loop_assignment(object)
            && let Some(step_binding) = self.resolve_iterator_step_binding_from_expression(object)
            && let Expression::String(property_name) = property
        {
            match (property_name.as_str(), step_binding) {
                (
                    "done",
                    IteratorStepBinding::Runtime {
                        static_done: Some(done),
                        ..
                    },
                ) => return Expression::Bool(done),
                (
                    "value",
                    IteratorStepBinding::Runtime {
                        static_value: Some(value),
                        ..
                    },
                ) if !self.iterator_step_static_value_requires_runtime_read(&value) => {
                    return self.materialize_static_expression(&value);
                }
                _ => {}
            }
        }
        if self.expression_uses_runtime_dynamic_binding(object) {
            if std::env::var_os("AYY_TRACE_THIS_FLOW").is_some()
                && matches!(object, Expression::This)
            {
                eprintln!(
                    "this_flow materialize_member runtime-dynamic fn={:?} expr={:?}",
                    self.current_function_name(),
                    Expression::Member {
                        object: Box::new(object.clone()),
                        property: Box::new(property.clone()),
                    }
                );
            }
            return Expression::Member {
                object: Box::new(self.materialize_static_expression(object)),
                property: Box::new(self.materialize_static_expression(property)),
            };
        }
        let materialized_object = self.materialize_static_expression(object);
        let materialized_property = self.materialize_static_expression(property);
        if self
            .runtime_object_property_shadow_binding_name_for_expression(object, property)
            .is_some_and(|shadow_binding_name| {
                self.runtime_object_property_shadow_binding_should_defer_static_resolution(
                    &shadow_binding_name,
                )
            })
            || self
                .runtime_object_property_shadow_binding_name_for_expression(
                    &materialized_object,
                    &materialized_property,
                )
                .is_some_and(|shadow_binding_name| {
                    self.runtime_object_property_shadow_binding_should_defer_static_resolution(
                        &shadow_binding_name,
                    )
                })
        {
            if std::env::var_os("AYY_TRACE_RUNTIME_SHADOWS").is_some() {
                eprintln!(
                    "runtime_shadow_materialize_preserve object={object:?} property={property:?} materialized_object={materialized_object:?} materialized_property={materialized_property:?}"
                );
            }
            return Expression::Member {
                object: Box::new(materialized_object),
                property: Box::new(materialized_property),
            };
        }
        if self.current_function_requires_runtime_public_this_resolution()
            && self.expression_is_current_this_reference(object)
            && !is_private_property_name_expression(&materialized_property)
        {
            return Expression::Member {
                object: Box::new(materialized_object),
                property: Box::new(materialized_property),
            };
        }
        if matches!(materialized_object, Expression::This)
            && let Expression::String(property_name) = &materialized_property
        {
            if self.global_has_binding(property_name)
                || self.global_has_implicit_binding(property_name)
            {
                return Expression::Identifier(property_name.clone());
            }
            if let Some(descriptor) =
                self.resolve_top_level_global_property_descriptor_binding(property_name)
                && let Some(value) = descriptor.value
            {
                return self.materialize_static_expression(&value);
            }
        }
        if let Some(function_name) = self
            .resolve_function_name_value(object, property)
            .or_else(|| {
                resolved_object.as_ref().and_then(|resolved_object| {
                    self.resolve_function_name_value(resolved_object, property)
                })
            })
            .or_else(|| {
                resolved_property.as_ref().and_then(|resolved_property| {
                    self.resolve_function_name_value(object, resolved_property)
                })
            })
            .or_else(|| {
                resolved_object.as_ref().and_then(|resolved_object| {
                    resolved_property.as_ref().and_then(|resolved_property| {
                        self.resolve_function_name_value(resolved_object, resolved_property)
                    })
                })
            })
            .or_else(|| {
                (!static_expression_matches(&materialized_object, object)
                    || !static_expression_matches(&materialized_property, property))
                .then(|| {
                    self.resolve_function_name_value(&materialized_object, &materialized_property)
                })?
            })
        {
            return Expression::String(function_name);
        }
        if matches!(&materialized_property, Expression::String(name) if name == "constructor")
            && let Some(binding) = self.resolve_function_binding_from_expression(object)
        {
            let constructor_name = match binding {
                LocalFunctionBinding::User(function_name) => self
                    .user_function(&function_name)
                    .map(|function| match function.kind {
                        FunctionKind::Ordinary => "Function",
                        FunctionKind::Generator => "GeneratorFunction",
                        FunctionKind::Async => "AsyncFunction",
                        FunctionKind::AsyncGenerator => "AsyncGeneratorFunction",
                    })
                    .unwrap_or("Function"),
                LocalFunctionBinding::Builtin(_) => "Function",
            };
            return Expression::Identifier(constructor_name.to_string());
        }
        if let Some(getter_binding) = self
            .resolve_member_getter_binding_shallow(object, &materialized_property)
            .or_else(|| {
                resolved_object.as_ref().and_then(|resolved_object| {
                    self.resolve_member_getter_binding_shallow(
                        resolved_object,
                        &materialized_property,
                    )
                })
            })
            .or_else(|| {
                resolved_property.as_ref().and_then(|resolved_property| {
                    self.resolve_member_getter_binding_shallow(object, resolved_property)
                })
            })
            .or_else(|| {
                resolved_object.as_ref().and_then(|resolved_object| {
                    resolved_property.as_ref().and_then(|resolved_property| {
                        self.resolve_member_getter_binding_shallow(
                            resolved_object,
                            resolved_property,
                        )
                    })
                })
            })
            .or_else(|| {
                self.resolve_member_getter_binding_shallow(
                    &materialized_object,
                    &materialized_property,
                )
            })
        {
            if let Some(value) = self.resolve_static_getter_value_from_binding_with_context(
                &getter_binding,
                object,
                self.current_function_name(),
            ) {
                return self.materialize_static_expression(&value);
            }
            return Expression::Member {
                object: Box::new(materialized_object),
                property: Box::new(materialized_property),
            };
        }
        if let Some(array_binding) = self.resolve_array_binding_from_expression(object) {
            if matches!(property, Expression::String(text) if text == "length") {
                let has_runtime_array_state = self
                    .runtime_array_length_local_for_expression(object)
                    .is_some()
                    || matches!(
                        object,
                        Expression::Identifier(name)
                            if self.is_named_global_array_binding(name)
                                && self.uses_global_runtime_array_state(name)
                    );
                if has_runtime_array_state {
                    return Expression::Member {
                        object: Box::new(self.materialize_static_expression(object)),
                        property: Box::new(self.materialize_static_expression(property)),
                    };
                }
                return Expression::Number(array_binding.values.len() as f64);
            }
            if let Some(index) = argument_index_from_expression(property) {
                if let Some(Some(value)) = array_binding.values.get(index as usize) {
                    return self.materialize_static_expression(value);
                }
                let has_runtime_array_state = self
                    .runtime_array_length_local_for_expression(object)
                    .is_some()
                    || matches!(
                        object,
                        Expression::Identifier(name)
                            if self.is_named_global_array_binding(name)
                                && self.uses_global_runtime_array_state(name)
                    );
                if has_runtime_array_state {
                    return Expression::Member {
                        object: Box::new(self.materialize_static_expression(object)),
                        property: Box::new(self.materialize_static_expression(property)),
                    };
                }
                return Expression::Undefined;
            }
        }
        if let Expression::String(text) = &materialized_object {
            if let Some(index) = argument_index_from_expression(&materialized_property) {
                return text
                    .chars()
                    .nth(index as usize)
                    .map(|character| Expression::String(character.to_string()))
                    .unwrap_or(Expression::Undefined);
            }
            if matches!(&materialized_property, Expression::String(name) if name == "length") {
                return Expression::Number(text.chars().count() as f64);
            }
        }
        if matches!(&materialized_property, Expression::String(name) if name == "length") {
            let object_candidates = [
                Some(object),
                resolved_object.as_ref(),
                (!static_expression_matches(&materialized_object, object))
                    .then_some(&materialized_object),
            ];
            for object_candidate in object_candidates.into_iter().flatten() {
                if let Some(Expression::String(text)) =
                    self.resolve_static_boxed_primitive_value(object_candidate)
                {
                    return Expression::Number(text.chars().count() as f64);
                }
            }
        }
        if let Some(function_length) = self
            .resolve_user_function_length(object, property)
            .or_else(|| {
                resolved_object.as_ref().and_then(|resolved_object| {
                    self.resolve_user_function_length(resolved_object, property)
                })
            })
            .or_else(|| {
                resolved_property.as_ref().and_then(|resolved_property| {
                    self.resolve_user_function_length(object, resolved_property)
                })
            })
            .or_else(|| {
                resolved_object.as_ref().and_then(|resolved_object| {
                    resolved_property.as_ref().and_then(|resolved_property| {
                        self.resolve_user_function_length(resolved_object, resolved_property)
                    })
                })
            })
            .or_else(|| {
                (!static_expression_matches(&materialized_object, object)
                    || !static_expression_matches(&materialized_property, property))
                .then(|| {
                    self.resolve_user_function_length(&materialized_object, &materialized_property)
                })?
            })
        {
            return Expression::Number(function_length as f64);
        }
        if matches!(&materialized_property, Expression::String(name) if name == "buffer")
            && let Some(buffer_expression) = self
                .resolve_static_viewed_array_buffer_expression(object)
                .or_else(|| {
                    resolved_object.as_ref().and_then(|resolved| {
                        self.resolve_static_viewed_array_buffer_expression(resolved)
                    })
                })
                .or_else(|| {
                    (!static_expression_matches(&materialized_object, object))
                        .then(|| {
                            self.resolve_static_viewed_array_buffer_expression(&materialized_object)
                        })
                        .flatten()
                })
        {
            return self.materialize_static_expression(&buffer_expression);
        }
        let object_binding = self
            .resolve_object_binding_from_expression(object)
            .or_else(|| {
                resolved_object.as_ref().and_then(|resolved_object| {
                    self.resolve_object_binding_from_expression(resolved_object)
                })
            })
            .or_else(|| {
                (!static_expression_matches(&materialized_object, object))
                    .then(|| self.resolve_object_binding_from_expression(&materialized_object))
                    .flatten()
            });
        if let Some(object_binding) = object_binding {
            if std::env::var_os("AYY_TRACE_THIS_FLOW").is_some()
                && matches!(object, Expression::This)
            {
                eprintln!(
                    "this_flow materialize_member static-binding fn={:?} expr={:?} properties={:?}",
                    self.current_function_name(),
                    Expression::Member {
                        object: Box::new(object.clone()),
                        property: Box::new(property.clone()),
                    },
                    object_binding
                        .string_properties
                        .iter()
                        .map(|(name, _)| name.clone())
                        .collect::<Vec<_>>()
                );
            }
            if let Some(getter_binding) = self
                .resolve_member_getter_binding(object, &materialized_property)
                .or_else(|| {
                    resolved_object.as_ref().and_then(|resolved_object| {
                        self.resolve_member_getter_binding(resolved_object, &materialized_property)
                    })
                })
                .or_else(|| {
                    resolved_property.as_ref().and_then(|resolved_property| {
                        self.resolve_member_getter_binding(object, resolved_property)
                    })
                })
                .or_else(|| {
                    resolved_object.as_ref().and_then(|resolved_object| {
                        resolved_property.as_ref().and_then(|resolved_property| {
                            self.resolve_member_getter_binding(resolved_object, resolved_property)
                        })
                    })
                })
                .or_else(|| {
                    self.resolve_member_getter_binding(&materialized_object, &materialized_property)
                })
            {
                if let Some(value) = self.resolve_static_getter_value_from_binding_with_context(
                    &getter_binding,
                    object,
                    self.current_function_name(),
                ) {
                    return self.materialize_static_expression(&value);
                }
                return Expression::Member {
                    object: Box::new(materialized_object),
                    property: Box::new(materialized_property),
                };
            }
            if let Some(value) =
                object_binding_lookup_value(&object_binding, &materialized_property)
            {
                if self.resolve_iterator_source_kind(value).is_some() {
                    return value.clone();
                }
                if argument_index_from_expression(&materialized_property).is_some()
                    && self
                        .static_typed_array_name_from_binding(&object_binding)
                        .is_some()
                    && let Some(value) = self.static_typed_array_member_value_from_binding(
                        &object_binding,
                        &materialized_property,
                    )
                {
                    return self.materialize_static_expression(&value);
                }
                return self.materialize_static_expression(value);
            }
            if let Some(value) = self.static_typed_array_member_value_from_binding(
                &object_binding,
                &materialized_property,
            ) {
                return self.materialize_static_expression(&value);
            }
            if matches!(&materialized_property, Expression::String(name) if name == "size")
                && self.object_binding_is_static_map(&object_binding)
                && let Some(size) = self.static_map_size_from_binding(&object_binding)
            {
                return Expression::Number(size);
            }
            if let Some(value) = self
                .resolve_inherited_object_property_value(object, &materialized_property)
                .or_else(|| {
                    (!static_expression_matches(&materialized_object, object))
                        .then(|| {
                            self.resolve_inherited_object_property_value(
                                &materialized_object,
                                &materialized_property,
                            )
                        })
                        .flatten()
                })
            {
                return self.materialize_static_expression(&value);
            }
            if self
                .resolve_member_function_binding(object, &materialized_property)
                .or_else(|| {
                    self.resolve_member_function_binding(
                        &materialized_object,
                        &materialized_property,
                    )
                })
                .is_some()
            {
                return Expression::Member {
                    object: Box::new(materialized_object),
                    property: Box::new(materialized_property),
                };
            }
            if static_property_name_from_expression(&materialized_property).is_some()
                || object_binding_has_property(&object_binding, &materialized_property)
            {
                if is_private_property_name_expression(&materialized_property) {
                    return Expression::Member {
                        object: Box::new(materialized_object),
                        property: Box::new(materialized_property),
                    };
                }
                return Expression::Undefined;
            }
        }
        Expression::Member {
            object: Box::new(materialized_object),
            property: Box::new(materialized_property),
        }
    }
}
