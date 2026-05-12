use super::*;

impl<'a> FunctionCompiler<'a> {
    fn resolve_static_class_init_local_identifier(
        &self,
        name: &str,
        local_bindings: &std::collections::HashMap<String, Expression>,
    ) -> Expression {
        let mut current = Expression::Identifier(name.to_string());
        let mut seen = std::collections::HashSet::new();
        while let Expression::Identifier(current_name) = &current {
            if !seen.insert(current_name.clone()) {
                break;
            }
            let Some(next) = local_bindings.get(current_name) else {
                break;
            };
            current = next.clone();
        }
        current
    }

    fn resolve_static_class_init_local_expression(
        &self,
        expression: &Expression,
        local_bindings: &std::collections::HashMap<String, Expression>,
    ) -> Expression {
        match expression {
            Expression::Identifier(name) => {
                self.resolve_static_class_init_local_identifier(name, local_bindings)
            }
            Expression::Member { object, property } => Expression::Member {
                object: Box::new(
                    self.resolve_static_class_init_local_expression(object, local_bindings),
                ),
                property: Box::new(
                    self.resolve_static_class_init_local_expression(property, local_bindings),
                ),
            },
            Expression::Call { callee, arguments } => Expression::Call {
                callee: Box::new(
                    self.resolve_static_class_init_local_expression(callee, local_bindings),
                ),
                arguments: arguments
                    .iter()
                    .map(|argument| match argument {
                        CallArgument::Expression(expression) => CallArgument::Expression(
                            self.resolve_static_class_init_local_expression(
                                expression,
                                local_bindings,
                            ),
                        ),
                        CallArgument::Spread(expression) => {
                            CallArgument::Spread(self.resolve_static_class_init_local_expression(
                                expression,
                                local_bindings,
                            ))
                        }
                    })
                    .collect(),
            },
            Expression::New { callee, arguments } => Expression::New {
                callee: Box::new(
                    self.resolve_static_class_init_local_expression(callee, local_bindings),
                ),
                arguments: arguments
                    .iter()
                    .map(|argument| match argument {
                        CallArgument::Expression(expression) => CallArgument::Expression(
                            self.resolve_static_class_init_local_expression(
                                expression,
                                local_bindings,
                            ),
                        ),
                        CallArgument::Spread(expression) => {
                            CallArgument::Spread(self.resolve_static_class_init_local_expression(
                                expression,
                                local_bindings,
                            ))
                        }
                    })
                    .collect(),
            },
            Expression::Assign { name, value } => Expression::Assign {
                name: name.clone(),
                value: Box::new(
                    self.resolve_static_class_init_local_expression(value, local_bindings),
                ),
            },
            Expression::AssignMember {
                object,
                property,
                value,
            } => Expression::AssignMember {
                object: Box::new(
                    self.resolve_static_class_init_local_expression(object, local_bindings),
                ),
                property: Box::new(
                    self.resolve_static_class_init_local_expression(property, local_bindings),
                ),
                value: Box::new(
                    self.resolve_static_class_init_local_expression(value, local_bindings),
                ),
            },
            Expression::Unary { op, expression } => Expression::Unary {
                op: *op,
                expression: Box::new(
                    self.resolve_static_class_init_local_expression(expression, local_bindings),
                ),
            },
            Expression::Binary { op, left, right } => Expression::Binary {
                op: *op,
                left: Box::new(
                    self.resolve_static_class_init_local_expression(left, local_bindings),
                ),
                right: Box::new(
                    self.resolve_static_class_init_local_expression(right, local_bindings),
                ),
            },
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                Expression::Conditional {
                    condition: Box::new(
                        self.resolve_static_class_init_local_expression(condition, local_bindings),
                    ),
                    then_expression: Box::new(self.resolve_static_class_init_local_expression(
                        then_expression,
                        local_bindings,
                    )),
                    else_expression: Box::new(self.resolve_static_class_init_local_expression(
                        else_expression,
                        local_bindings,
                    )),
                }
            }
            Expression::Sequence(expressions) => Expression::Sequence(
                expressions
                    .iter()
                    .map(|expression| {
                        self.resolve_static_class_init_local_expression(expression, local_bindings)
                    })
                    .collect(),
            ),
            Expression::Array(elements) => Expression::Array(
                elements
                    .iter()
                    .map(|element| match element {
                        ArrayElement::Expression(expression) => ArrayElement::Expression(
                            self.resolve_static_class_init_local_expression(
                                expression,
                                local_bindings,
                            ),
                        ),
                        ArrayElement::Spread(expression) => {
                            ArrayElement::Spread(self.resolve_static_class_init_local_expression(
                                expression,
                                local_bindings,
                            ))
                        }
                    })
                    .collect(),
            ),
            Expression::Object(entries) => Expression::Object(
                entries
                    .iter()
                    .map(|entry| match entry {
                        ObjectEntry::Data { key, value } => ObjectEntry::Data {
                            key: self
                                .resolve_static_class_init_local_expression(key, local_bindings),
                            value: self
                                .resolve_static_class_init_local_expression(value, local_bindings),
                        },
                        ObjectEntry::Getter { key, getter } => ObjectEntry::Getter {
                            key: self
                                .resolve_static_class_init_local_expression(key, local_bindings),
                            getter: self
                                .resolve_static_class_init_local_expression(getter, local_bindings),
                        },
                        ObjectEntry::Setter { key, setter } => ObjectEntry::Setter {
                            key: self
                                .resolve_static_class_init_local_expression(key, local_bindings),
                            setter: self
                                .resolve_static_class_init_local_expression(setter, local_bindings),
                        },
                        ObjectEntry::Spread(expression) => {
                            ObjectEntry::Spread(self.resolve_static_class_init_local_expression(
                                expression,
                                local_bindings,
                            ))
                        }
                    })
                    .collect(),
            ),
            _ => expression.clone(),
        }
    }

    pub(in crate::backend::direct_wasm) fn infer_static_class_init_call_result_expression(
        &self,
        function_name: &str,
    ) -> Option<Expression> {
        if !function_name.starts_with("__ayy_class_init_") {
            return None;
        }
        let function = self.resolve_registered_function_declaration(function_name)?;
        let mut local_bindings = std::collections::HashMap::new();

        for statement in &function.body {
            match statement {
                Statement::Var { name, value } | Statement::Let { name, value, .. } => {
                    local_bindings.insert(
                        name.clone(),
                        self.resolve_static_class_init_local_expression(value, &local_bindings),
                    );
                }
                Statement::Assign { name, value } => {
                    local_bindings.insert(
                        name.clone(),
                        self.resolve_static_class_init_local_expression(value, &local_bindings),
                    );
                }
                Statement::Return(value) => {
                    return Some(
                        self.resolve_static_class_init_local_expression(value, &local_bindings),
                    );
                }
                _ => {}
            }
        }

        None
    }

    pub(in crate::backend::direct_wasm) fn resolve_static_class_init_constructor_alias(
        &self,
        class_binding_name: &str,
    ) -> Option<String> {
        self.backend
            .function_registry
            .catalog
            .registered_function_declarations
            .iter()
            .find(|function| {
                function.name.starts_with("__ayy_class_init_")
                    && function.body.iter().any(|statement| {
                        matches!(
                            statement,
                            Statement::Return(Expression::Identifier(name))
                                if name == class_binding_name
                        )
                    })
            })
            .and_then(|function| {
                self.infer_static_class_init_call_result_expression(&function.name)
            })
            .and_then(|result| match result {
                Expression::Identifier(name) => Some(name),
                _ => None,
            })
    }

    pub(in crate::backend::direct_wasm) fn resolve_static_class_init_local_alias_expression(
        &self,
        alias_name: &str,
    ) -> Option<Expression> {
        for function in &self
            .backend
            .function_registry
            .catalog
            .registered_function_declarations
        {
            if !function.name.starts_with("__ayy_class_init_") {
                continue;
            }

            let mut local_bindings = std::collections::HashMap::new();
            for statement in &function.body {
                match statement {
                    Statement::Var { name, value } | Statement::Let { name, value, .. } => {
                        let resolved =
                            self.resolve_static_class_init_local_expression(value, &local_bindings);
                        if name == alias_name {
                            return Some(resolved);
                        }
                        local_bindings.insert(name.clone(), resolved);
                    }
                    Statement::Assign { name, value } => {
                        let resolved =
                            self.resolve_static_class_init_local_expression(value, &local_bindings);
                        if name == alias_name {
                            return Some(resolved);
                        }
                        local_bindings.insert(name.clone(), resolved);
                    }
                    _ => {}
                }
            }
        }

        None
    }

    pub(in crate::backend::direct_wasm) fn resolve_static_class_init_local_aliases_in_expression(
        &self,
        expression: &Expression,
    ) -> Expression {
        match expression {
            Expression::Identifier(name) => self
                .resolve_static_class_init_local_alias_expression(name)
                .filter(|resolved| !static_expression_matches(resolved, expression))
                .map(|resolved| {
                    self.resolve_static_class_init_local_aliases_in_expression(&resolved)
                })
                .unwrap_or_else(|| expression.clone()),
            Expression::Member { object, property } => Expression::Member {
                object: Box::new(
                    self.resolve_static_class_init_local_aliases_in_expression(object),
                ),
                property: Box::new(
                    self.resolve_static_class_init_local_aliases_in_expression(property),
                ),
            },
            Expression::Sequence(expressions) => Expression::Sequence(
                expressions
                    .iter()
                    .map(|expression| {
                        self.resolve_static_class_init_local_aliases_in_expression(expression)
                    })
                    .collect(),
            ),
            _ => expression.clone(),
        }
    }

    fn resolve_static_class_init_storage_name_from_expression(
        &self,
        expression: &Expression,
    ) -> Option<String> {
        match expression {
            Expression::Identifier(name) => self
                .resolve_static_class_init_local_alias_expression(name)
                .and_then(|resolved| {
                    self.resolve_static_class_init_storage_name_from_expression(&resolved)
                })
                .or_else(|| {
                    self.infer_static_class_init_prototype_object_binding(name)
                        .is_some()
                        .then(|| name.clone())
                })
                .or_else(|| {
                    self.resolve_function_binding_from_expression(expression)
                        .and_then(|binding| self.function_prototype_binding_owner_name(&binding))
                }),
            Expression::Call { callee, .. } => {
                let Expression::Identifier(function_name) = callee.as_ref() else {
                    return None;
                };
                self.infer_static_class_init_call_result_expression(function_name)
                    .and_then(|result| match result {
                        Expression::Identifier(name) => Some(name),
                        _ => None,
                    })
            }
            _ => None,
        }
    }

    fn infer_static_class_init_prototype_object_binding(
        &self,
        constructor_name: &str,
    ) -> Option<ObjectValueBinding> {
        let init_function = self
            .backend
            .function_registry
            .catalog
            .registered_function_declarations
            .iter()
            .find(|function| {
                let Some(Expression::Identifier(returned_name)) =
                    self.infer_static_class_init_call_result_expression(&function.name)
                else {
                    return false;
                };
                returned_name == constructor_name
                    || self
                        .resolve_registered_function_declaration(&returned_name)
                        .and_then(|returned_function| {
                            returned_function
                                .self_binding
                                .as_ref()
                                .or(returned_function.top_level_binding.as_ref())
                        })
                        .is_some_and(|owner_name| owner_name == constructor_name)
            })?;

        let mut local_bindings = std::collections::HashMap::new();
        let mut prototype_binding = empty_object_value_binding();
        let mut found_property = false;

        for statement in &init_function.body {
            match statement {
                Statement::Var { name, value } | Statement::Let { name, value, .. } => {
                    local_bindings.insert(
                        name.clone(),
                        self.resolve_static_class_init_local_expression(value, &local_bindings),
                    );
                }
                Statement::Assign { name, value } => {
                    local_bindings.insert(
                        name.clone(),
                        self.resolve_static_class_init_local_expression(value, &local_bindings),
                    );
                }
                Statement::Expression(Expression::Call { callee, arguments })
                    if matches!(
                        callee.as_ref(),
                        Expression::Member { object, property }
                            if matches!(object.as_ref(), Expression::Identifier(name) if name == "Object")
                                && matches!(property.as_ref(), Expression::String(name) if name == "defineProperty")
                    ) =>
                {
                    let [
                        CallArgument::Expression(target_expression),
                        CallArgument::Expression(property_expression),
                        CallArgument::Expression(descriptor_expression),
                        ..,
                    ] = arguments.as_slice()
                    else {
                        continue;
                    };
                    let resolved_target = self.resolve_static_class_init_local_expression(
                        target_expression,
                        &local_bindings,
                    );
                    let resolved_constructor = self.resolve_static_class_init_local_expression(
                        &Expression::Identifier(constructor_name.to_string()),
                        &local_bindings,
                    );
                    let is_constructor_prototype = matches!(
                        &resolved_target,
                        Expression::Member { object, property }
                            if (matches!(object.as_ref(), Expression::Identifier(name) if name == constructor_name)
                                || static_expression_matches(object.as_ref(), &resolved_constructor))
                                && matches!(property.as_ref(), Expression::String(name) if name == "prototype")
                    );
                    if !is_constructor_prototype {
                        continue;
                    }
                    let Some(descriptor) =
                        resolve_property_descriptor_definition(descriptor_expression)
                    else {
                        continue;
                    };
                    let property = self.resolve_static_class_init_local_expression(
                        property_expression,
                        &local_bindings,
                    );
                    let property = self.canonical_object_property_expression(&property);
                    let property_name = static_property_name_from_expression(&property);
                    let existing_value =
                        object_binding_lookup_value(&prototype_binding, &property).cloned();
                    let existing_descriptor =
                        object_binding_lookup_descriptor(&prototype_binding, &property).cloned();
                    let current_enumerable = property_name.as_ref().is_some_and(|property_name| {
                        !prototype_binding
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
                    let (value, writable, getter, setter, has_get, has_set) =
                        if descriptor.is_accessor() {
                            (
                                None,
                                None,
                                descriptor
                                    .getter
                                    .as_ref()
                                    .map(|value| {
                                        self.resolve_static_class_init_local_expression(
                                            value,
                                            &local_bindings,
                                        )
                                    })
                                    .or_else(|| {
                                        existing_descriptor
                                            .as_ref()
                                            .and_then(|descriptor| descriptor.getter.clone())
                                    }),
                                descriptor
                                    .setter
                                    .as_ref()
                                    .map(|value| {
                                        self.resolve_static_class_init_local_expression(
                                            value,
                                            &local_bindings,
                                        )
                                    })
                                    .or_else(|| {
                                        existing_descriptor
                                            .as_ref()
                                            .and_then(|descriptor| descriptor.setter.clone())
                                    }),
                                descriptor.getter.is_some()
                                    || existing_descriptor
                                        .as_ref()
                                        .is_some_and(|descriptor| descriptor.has_get),
                                descriptor.setter.is_some()
                                    || existing_descriptor
                                        .as_ref()
                                        .is_some_and(|descriptor| descriptor.has_set),
                            )
                        } else {
                            let value = descriptor
                                .value
                                .as_ref()
                                .map(|value| {
                                    self.resolve_static_class_init_local_expression(
                                        value,
                                        &local_bindings,
                                    )
                                })
                                .or_else(|| existing_value.clone())
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
                            (
                                Some(value),
                                Some(writable.unwrap_or(false)),
                                None,
                                None,
                                false,
                                false,
                            )
                        };
                    object_binding_define_property_descriptor(
                        &mut prototype_binding,
                        property,
                        PropertyDescriptorBinding {
                            value,
                            configurable,
                            enumerable,
                            writable,
                            getter,
                            setter,
                            has_get,
                            has_set,
                        },
                    );
                    found_property = true;
                }
                _ => {}
            }
        }

        found_property.then_some(prototype_binding)
    }

    pub(in crate::backend::direct_wasm) fn function_prototype_binding_owner_name(
        &self,
        binding: &LocalFunctionBinding,
    ) -> Option<String> {
        match binding {
            LocalFunctionBinding::User(function_name) => Some(
                self.resolve_registered_function_declaration(function_name)
                    .and_then(|function| {
                        function
                            .self_binding
                            .as_ref()
                            .or(function.top_level_binding.as_ref())
                            .cloned()
                    })
                    .unwrap_or_else(|| function_name.clone()),
            ),
            LocalFunctionBinding::Builtin(function_name) => Some(function_name.clone()),
        }
    }

    pub(in crate::backend::direct_wasm) fn merge_object_binding_properties(
        target: &mut ObjectValueBinding,
        source: &ObjectValueBinding,
    ) {
        for (name, value) in &source.string_properties {
            let enumerable = !source
                .non_enumerable_string_properties
                .iter()
                .any(|hidden_name| hidden_name == name);
            object_binding_define_property(
                target,
                Expression::String(name.clone()),
                value.clone(),
                enumerable,
            );
        }
        for (property, value) in &source.symbol_properties {
            object_binding_define_property(target, property.clone(), value.clone(), true);
        }
        for (property, descriptor) in &source.property_descriptors {
            object_binding_define_property_descriptor(target, property.clone(), descriptor.clone());
        }
    }

    pub(in crate::backend::direct_wasm) fn default_function_prototype_object_binding(
        &self,
        function_binding: &LocalFunctionBinding,
    ) -> Option<ObjectValueBinding> {
        let constructor_expression = match function_binding {
            LocalFunctionBinding::User(function_name) => {
                let user_function = self.user_function(function_name)?;
                if user_function.is_generator() {
                    return Some(empty_object_value_binding());
                }
                if !user_function.is_constructible() {
                    return None;
                }
                Expression::Identifier(function_name.clone())
            }
            LocalFunctionBinding::Builtin(function_name) => {
                if !is_function_constructor_builtin(function_name) {
                    return None;
                }
                Expression::Identifier(function_name.clone())
            }
        };

        let mut object_binding = empty_object_value_binding();
        object_binding_define_property(
            &mut object_binding,
            Expression::String("constructor".to_string()),
            constructor_expression,
            false,
        );
        Some(object_binding)
    }

    pub(in crate::backend::direct_wasm) fn resolve_function_prototype_object_binding(
        &self,
        name: &str,
    ) -> Option<ObjectValueBinding> {
        let resolved_storage_name = self
            .resolve_function_binding_from_expression(&Expression::Identifier(name.to_string()))
            .and_then(|binding| self.function_prototype_binding_owner_name(&binding))
            .or_else(|| self.resolve_static_class_init_constructor_alias(name))
            .or_else(|| {
                self.resolve_static_class_init_local_alias_expression(name)
                    .and_then(|resolved| match resolved {
                        Expression::Identifier(resolved_name) => self
                            .resolve_function_binding_from_expression(&Expression::Identifier(
                                resolved_name.clone(),
                            ))
                            .and_then(|binding| {
                                self.function_prototype_binding_owner_name(&binding)
                            })
                            .or(Some(resolved_name)),
                        _ => None,
                    })
            })
            .or_else(|| {
                self.state
                    .speculation
                    .static_semantics
                    .local_value_binding(name)
                    .or_else(|| self.global_value_binding(name))
                    .and_then(|value| {
                        self.resolve_static_class_init_storage_name_from_expression(value)
                    })
            })
            .unwrap_or_else(|| name.to_string());
        let trace_prototype_bindings = std::env::var_os("AYY_TRACE_PROTOTYPE_BINDINGS").is_some();
        if trace_prototype_bindings {
            eprintln!(
                "prototype_binding:resolve name={name} resolved_storage_name={resolved_storage_name} local_value={:?} global_value={:?}",
                self.state
                    .speculation
                    .static_semantics
                    .local_value_binding(name),
                self.global_value_binding(name),
            );
        }
        let stored_binding = self
            .state
            .speculation
            .static_semantics
            .objects
            .local_prototype_object_bindings
            .get(name)
            .cloned()
            .or_else(|| {
                self.state
                    .speculation
                    .static_semantics
                    .objects
                    .local_prototype_object_bindings
                    .get(&resolved_storage_name)
                    .cloned()
            })
            .or_else(|| {
                self.backend
                    .global_semantics
                    .values
                    .prototype_object_bindings
                    .get(name)
                    .cloned()
            })
            .or_else(|| {
                self.backend
                    .global_semantics
                    .values
                    .prototype_object_bindings
                    .get(&resolved_storage_name)
                    .cloned()
            });
        let inferred_binding = self
            .infer_static_class_init_prototype_object_binding(name)
            .or_else(|| {
                (resolved_storage_name != name).then(|| {
                    self.infer_static_class_init_prototype_object_binding(&resolved_storage_name)
                })?
            });
        if trace_prototype_bindings {
            eprintln!(
                "prototype_binding:bindings name={name} resolved_storage_name={resolved_storage_name} stored_props={:?} inferred_props={:?}",
                stored_binding
                    .as_ref()
                    .map(ordered_object_property_names)
                    .unwrap_or_default(),
                inferred_binding
                    .as_ref()
                    .map(ordered_object_property_names)
                    .unwrap_or_default(),
            );
        }
        let default_binding = self
            .resolve_function_binding_from_expression(&Expression::Identifier(name.to_string()))
            .or_else(|| {
                (resolved_storage_name != name).then(|| {
                    self.resolve_function_binding_from_expression(&Expression::Identifier(
                        resolved_storage_name.clone(),
                    ))
                })?
            })
            .and_then(|binding| self.default_function_prototype_object_binding(&binding));

        match (default_binding, stored_binding, inferred_binding) {
            (Some(mut default_binding), Some(stored_binding), Some(inferred_binding)) => {
                Self::merge_object_binding_properties(&mut default_binding, &stored_binding);
                Self::merge_object_binding_properties(&mut default_binding, &inferred_binding);
                Some(default_binding)
            }
            (Some(mut default_binding), Some(stored_binding), None) => {
                Self::merge_object_binding_properties(&mut default_binding, &stored_binding);
                Some(default_binding)
            }
            (Some(mut default_binding), None, Some(inferred_binding)) => {
                Self::merge_object_binding_properties(&mut default_binding, &inferred_binding);
                Some(default_binding)
            }
            (None, Some(mut stored_binding), Some(inferred_binding)) => {
                Self::merge_object_binding_properties(&mut stored_binding, &inferred_binding);
                Some(stored_binding)
            }
            (Some(default_binding), None, None) => Some(default_binding),
            (None, Some(stored_binding), None) => Some(stored_binding),
            (None, None, Some(inferred_binding)) => Some(inferred_binding),
            (None, None, None) => None,
        }
    }
}
