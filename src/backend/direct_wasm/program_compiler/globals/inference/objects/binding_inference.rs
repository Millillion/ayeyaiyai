use super::*;
use crate::ir::hir::SwitchCase;

impl DirectWasmCompiler {
    const GLOBAL_STATIC_NEW_THIS_BINDING: &str = "__ayy_global_static_new_this";

    fn seed_global_constructed_private_member_markers(
        &self,
        constructor_function_name: &str,
        object_binding: &mut ObjectValueBinding,
    ) {
        let Some(class_name) = self
            .registered_function(constructor_function_name)
            .and_then(|function| function.self_binding.as_deref())
            .map(str::to_string)
            .or_else(|| {
                constructor_function_name
                    .rsplit_once("__name_")
                    .map(|(_, class_name)| class_name.to_string())
            })
        else {
            return;
        };
        let trace_private = std::env::var_os("AYY_TRACE_PRIVATE_MEMBER_LOOKUP").is_some();
        if trace_private {
            eprintln!(
                "private_seed_global constructor={} class={}",
                constructor_function_name, class_name
            );
        }

        for (key, binding) in &self.state.global_semantics.members.member_getter_bindings {
            let MemberFunctionBindingTarget::Prototype(target_name) = &key.target else {
                continue;
            };
            let MemberFunctionBindingProperty::String(property_name) = &key.property else {
                continue;
            };
            if target_name != &class_name || !property_name.starts_with("__ayy$private$") {
                continue;
            }
            let value = match binding {
                LocalFunctionBinding::User(function_name)
                | LocalFunctionBinding::Builtin(function_name) => {
                    Expression::Identifier(function_name.clone())
                }
            };
            object_binding_define_property(
                object_binding,
                Expression::String(property_name.clone()),
                value,
                false,
            );
            if trace_private {
                eprintln!("private_seed_global getter property={property_name}");
            }
        }

        for (key, binding) in &self.state.global_semantics.members.member_function_bindings {
            let MemberFunctionBindingTarget::Prototype(target_name) = &key.target else {
                continue;
            };
            let MemberFunctionBindingProperty::String(property_name) = &key.property else {
                continue;
            };
            if target_name != &class_name || !property_name.starts_with("__ayy$private$") {
                continue;
            }
            let value = match binding {
                LocalFunctionBinding::User(function_name)
                | LocalFunctionBinding::Builtin(function_name) => {
                    Expression::Identifier(function_name.clone())
                }
            };
            object_binding_define_property(
                object_binding,
                Expression::String(property_name.clone()),
                value,
                false,
            );
            if trace_private {
                eprintln!("private_seed_global method property={property_name}");
            }
        }
    }

    pub(in crate::backend::direct_wasm) fn infer_global_object_binding(
        &self,
        expression: &Expression,
    ) -> Option<ObjectValueBinding> {
        self.with_cloned_global_binding_state(|value_bindings, object_bindings| {
            self.infer_global_object_binding_with_state(expression, value_bindings, object_bindings)
        })
    }

    pub(in crate::backend::direct_wasm) fn infer_global_object_binding_with_state(
        &self,
        expression: &Expression,
        value_bindings: &mut HashMap<String, Expression>,
        object_bindings: &mut HashMap<String, ObjectValueBinding>,
    ) -> Option<ObjectValueBinding> {
        if let Some(binding) = self.infer_define_property_result_object_binding_with_state(
            expression,
            value_bindings,
            object_bindings,
        ) {
            return Some(binding);
        }

        match expression {
            Expression::Call { callee, arguments } if matches!(callee.as_ref(), Expression::Identifier(name) if name == "__ayyTemplateObject") =>
            {
                let Some(CallArgument::Expression(raw) | CallArgument::Spread(raw)) =
                    arguments.get(2)
                else {
                    return Some(empty_object_value_binding());
                };
                let raw_array = Self::template_object_array_binding_from_array(raw)
                    .or_else(|| self.infer_global_array_binding(raw))
                    .map(|binding| {
                        Expression::Array(
                            binding
                                .values
                                .into_iter()
                                .map(|value| {
                                    ArrayElement::Expression(value.unwrap_or(Expression::Undefined))
                                })
                                .collect(),
                        )
                    })
                    .unwrap_or_else(|| raw.clone());
                let mut binding = empty_object_value_binding();
                object_binding_set_property(
                    &mut binding,
                    Expression::String("raw".to_string()),
                    raw_array,
                );
                Some(binding)
            }
            Expression::Identifier(name) => {
                if let Some(realm_id) = parse_test262_realm_identifier(name) {
                    let mut realm_binding = empty_object_value_binding();
                    object_binding_set_property(
                        &mut realm_binding,
                        Expression::String("global".to_string()),
                        Expression::Identifier(test262_realm_global_identifier(realm_id)),
                    );
                    return Some(realm_binding);
                }
                if let Some(realm_id) = parse_test262_realm_global_identifier(name) {
                    let mut global_binding = empty_object_value_binding();
                    object_binding_set_property(
                        &mut global_binding,
                        Expression::String("eval".to_string()),
                        Expression::Identifier(test262_realm_eval_builtin_name(realm_id)),
                    );
                    return Some(global_binding);
                }
                object_bindings
                    .get(name)
                    .cloned()
                    .or_else(|| self.global_prototype_object_binding(name).cloned())
                    .or_else(|| {
                        value_bindings
                            .get(name)
                            .cloned()
                            .filter(
                                |value| !matches!(value, Expression::Identifier(alias) if alias == name),
                            )
                            .and_then(|value| {
                                self.infer_global_object_binding_with_state(
                                    &value,
                                    value_bindings,
                                    object_bindings,
                                )
                            })
                    })
            }
            Expression::Member { object, property } if matches!(property.as_ref(), Expression::String(name) if name == "prototype") =>
            {
                let Expression::Identifier(name) = object.as_ref() else {
                    return None;
                };
                self.infer_global_function_prototype_object_binding(name)
            }
            Expression::New { callee, arguments } => self.infer_global_constructed_object_binding(
                callee,
                arguments,
                value_bindings,
                object_bindings,
            ),
            _ => resolve_specialized_object_binding_expression(
                expression,
                &mut (value_bindings, object_bindings),
                |expression, _| self.infer_global_array_binding(expression),
                |entries, (value_bindings, object_bindings)| {
                    let context = self.static_eval_context();
                    resolve_structural_object_binding_in_environment(
                        &context,
                        entries,
                        &mut (value_bindings, object_bindings),
                        &|expression, (value_bindings, object_bindings)| {
                            let local_bindings = HashMap::new();
                            Some(
                                self.materialize_global_expression_with_state(
                                    expression,
                                    &local_bindings,
                                    value_bindings,
                                    object_bindings,
                                )
                                .unwrap_or_else(|| self.materialize_global_expression(expression)),
                            )
                        },
                        &|_, _| false,
                        &|expression, (value_bindings, object_bindings)| {
                            self.infer_global_object_binding_with_state(
                                expression,
                                value_bindings,
                                object_bindings,
                            )
                        },
                        &|object, property, (value_bindings, object_bindings)| {
                            self.infer_global_member_getter_return_value_with_state(
                                object,
                                property,
                                value_bindings,
                                object_bindings,
                            )
                        },
                    )
                },
                |expression, _| {
                    matches!(
                        expression,
                        Expression::Call { callee, .. }
                            if matches!(
                                callee.as_ref(),
                                Expression::Member { object, property }
                                    if matches!(object.as_ref(), Expression::Identifier(name) if name == "Object")
                                        && matches!(property.as_ref(), Expression::String(name) if name == "create")
                            )
                    )
                },
                |_, _| None,
            ),
        }
    }

    fn infer_define_property_result_object_binding_with_state(
        &self,
        expression: &Expression,
        value_bindings: &mut HashMap<String, Expression>,
        object_bindings: &mut HashMap<String, ObjectValueBinding>,
    ) -> Option<ObjectValueBinding> {
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
        let property = self
            .materialize_global_expression_with_state(
                property,
                &HashMap::new(),
                value_bindings,
                object_bindings,
            )
            .unwrap_or_else(|| self.materialize_global_expression(property));
        let materialized_target = self
            .materialize_global_expression_with_state(
                target,
                &HashMap::new(),
                value_bindings,
                object_bindings,
            )
            .unwrap_or_else(|| self.materialize_global_expression(target));
        let mut target_value_bindings = value_bindings.clone();
        let mut target_object_bindings = object_bindings.clone();
        let mut object_binding = self
            .infer_global_object_binding_with_state(
                &materialized_target,
                &mut target_value_bindings,
                &mut target_object_bindings,
            )
            .or_else(|| {
                self.infer_global_object_binding_with_state(
                    target,
                    &mut target_value_bindings,
                    &mut target_object_bindings,
                )
            })
            .unwrap_or_else(empty_object_value_binding);
        let descriptor_binding = self.global_property_descriptor_binding_with_state(
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

    fn global_property_descriptor_binding_with_state(
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
        let materialize_descriptor_value = |expression: &Expression| {
            self.materialize_global_expression_with_state(
                expression,
                &HashMap::new(),
                value_bindings,
                object_bindings,
            )
            .unwrap_or_else(|| self.materialize_global_expression(expression))
        };
        if descriptor.is_accessor() {
            return PropertyDescriptorBinding {
                value: None,
                configurable,
                enumerable,
                writable: None,
                getter: descriptor.getter.as_ref().map(materialize_descriptor_value),
                setter: descriptor.setter.as_ref().map(materialize_descriptor_value),
                has_get: descriptor.getter.is_some(),
                has_set: descriptor.setter.is_some(),
            };
        }

        let value = descriptor
            .value
            .as_ref()
            .map(materialize_descriptor_value)
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

    fn merge_inferred_object_binding_properties(
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
    }

    fn default_global_function_prototype_object_binding(
        &self,
        name: &str,
    ) -> Option<ObjectValueBinding> {
        let constructor_expression =
            match self.infer_global_function_binding(&Expression::Identifier(name.to_string()))? {
                LocalFunctionBinding::User(function_name) => {
                    let user_function = self.user_function(&function_name)?;
                    if !user_function.is_constructible() {
                        return None;
                    }
                    Expression::Identifier(function_name)
                }
                LocalFunctionBinding::Builtin(function_name) => {
                    if !is_function_constructor_builtin(&function_name) {
                        return None;
                    }
                    Expression::Identifier(function_name)
                }
            };

        let mut binding = empty_object_value_binding();
        object_binding_define_property(
            &mut binding,
            Expression::String("constructor".to_string()),
            constructor_expression,
            false,
        );
        Some(binding)
    }

    fn infer_static_class_init_prototype_object_binding(
        &self,
        constructor_name: &str,
    ) -> Option<ObjectValueBinding> {
        let init_function = self
            .state
            .function_registry
            .catalog
            .registered_function_declarations
            .iter()
            .find(|function| {
                matches!(
                    self.infer_static_class_init_call_result_expression(&function.name),
                    Some(Expression::Identifier(ref returned_name)) if returned_name == constructor_name
                )
            })?;

        let mut local_bindings = HashMap::new();
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
                    let is_constructor_prototype = matches!(
                        &resolved_target,
                        Expression::Member { object, property }
                            if matches!(object.as_ref(), Expression::Identifier(name) if name == constructor_name)
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
                    let Some(value) = descriptor.value.as_ref() else {
                        continue;
                    };
                    let property = self.resolve_static_class_init_local_expression(
                        property_expression,
                        &local_bindings,
                    );
                    let property = self.canonical_global_object_property_expression(&property);
                    let value =
                        self.resolve_static_class_init_local_expression(value, &local_bindings);
                    object_binding_define_property(
                        &mut prototype_binding,
                        property,
                        value,
                        descriptor.enumerable.unwrap_or(true),
                    );
                    found_property = true;
                }
                _ => {}
            }
        }

        found_property.then_some(prototype_binding)
    }

    fn infer_global_function_prototype_object_binding(
        &self,
        name: &str,
    ) -> Option<ObjectValueBinding> {
        let stored_binding = self.global_prototype_object_binding(name).cloned();
        let inferred_binding = self.infer_static_class_init_prototype_object_binding(name);
        let default_binding = self.default_global_function_prototype_object_binding(name);

        match (default_binding, stored_binding, inferred_binding) {
            (Some(mut default_binding), Some(stored_binding), Some(inferred_binding)) => {
                Self::merge_inferred_object_binding_properties(
                    &mut default_binding,
                    &stored_binding,
                );
                Self::merge_inferred_object_binding_properties(
                    &mut default_binding,
                    &inferred_binding,
                );
                Some(default_binding)
            }
            (Some(mut default_binding), Some(stored_binding), None) => {
                Self::merge_inferred_object_binding_properties(
                    &mut default_binding,
                    &stored_binding,
                );
                Some(default_binding)
            }
            (Some(mut default_binding), None, Some(inferred_binding)) => {
                Self::merge_inferred_object_binding_properties(
                    &mut default_binding,
                    &inferred_binding,
                );
                Some(default_binding)
            }
            (None, Some(mut stored_binding), Some(inferred_binding)) => {
                Self::merge_inferred_object_binding_properties(
                    &mut stored_binding,
                    &inferred_binding,
                );
                Some(stored_binding)
            }
            (Some(default_binding), None, None) => Some(default_binding),
            (None, Some(stored_binding), None) => Some(stored_binding),
            (None, None, Some(inferred_binding)) => Some(inferred_binding),
            (None, None, None) => None,
        }
    }

    fn infer_global_constructed_object_binding(
        &self,
        callee: &Expression,
        arguments: &[CallArgument],
        value_bindings: &mut HashMap<String, Expression>,
        object_bindings: &mut HashMap<String, ObjectValueBinding>,
    ) -> Option<ObjectValueBinding> {
        let LocalFunctionBinding::User(function_name) =
            self.infer_global_function_binding(callee)?
        else {
            return None;
        };
        let user_function = self.user_function(&function_name)?;
        if !user_function.is_constructible() {
            return None;
        }
        let function = self.registered_function(&function_name)?;
        let rewritten_body = function
            .body
            .iter()
            .map(|statement| {
                self.substitute_global_static_constructor_statement(
                    statement,
                    user_function,
                    arguments,
                    Self::GLOBAL_STATIC_NEW_THIS_BINDING,
                )
            })
            .collect::<Vec<_>>();
        let mut environment = GlobalStaticEvaluationEnvironment::from_snapshots(
            HashMap::new(),
            value_bindings.clone(),
            object_bindings.clone(),
        );
        environment.set_local_binding(
            Self::GLOBAL_STATIC_NEW_THIS_BINDING.to_string(),
            Expression::Identifier(Self::GLOBAL_STATIC_NEW_THIS_BINDING.to_string()),
        );
        let mut this_binding = empty_object_value_binding();
        self.seed_global_constructed_private_member_markers(&function_name, &mut this_binding);
        environment.set_object_binding(
            Self::GLOBAL_STATIC_NEW_THIS_BINDING.to_string(),
            this_binding,
        );
        self.apply_global_static_constructor_statement_updates(
            &rewritten_body,
            &mut environment,
            Self::GLOBAL_STATIC_NEW_THIS_BINDING,
        );
        environment
            .object_binding(Self::GLOBAL_STATIC_NEW_THIS_BINDING)
            .cloned()
    }

    fn substitute_global_static_constructor_statement(
        &self,
        statement: &Statement,
        user_function: &UserFunction,
        arguments: &[CallArgument],
        this_name: &str,
    ) -> Statement {
        match statement {
            Statement::Declaration { body } => Statement::Declaration {
                body: body
                    .iter()
                    .map(|statement| {
                        self.substitute_global_static_constructor_statement(
                            statement,
                            user_function,
                            arguments,
                            this_name,
                        )
                    })
                    .collect(),
            },
            Statement::Block { body } => Statement::Block {
                body: body
                    .iter()
                    .map(|statement| {
                        self.substitute_global_static_constructor_statement(
                            statement,
                            user_function,
                            arguments,
                            this_name,
                        )
                    })
                    .collect(),
            },
            Statement::Labeled { labels, body } => Statement::Labeled {
                labels: labels.clone(),
                body: body
                    .iter()
                    .map(|statement| {
                        self.substitute_global_static_constructor_statement(
                            statement,
                            user_function,
                            arguments,
                            this_name,
                        )
                    })
                    .collect(),
            },
            Statement::Var { name, value } => Statement::Var {
                name: name.clone(),
                value: self.substitute_global_static_constructor_expression(
                    value,
                    user_function,
                    arguments,
                    this_name,
                ),
            },
            Statement::Let {
                name,
                mutable,
                value,
            } => Statement::Let {
                name: name.clone(),
                mutable: *mutable,
                value: self.substitute_global_static_constructor_expression(
                    value,
                    user_function,
                    arguments,
                    this_name,
                ),
            },
            Statement::Assign { name, value } => Statement::Assign {
                name: name.clone(),
                value: self.substitute_global_static_constructor_expression(
                    value,
                    user_function,
                    arguments,
                    this_name,
                ),
            },
            Statement::AssignMember {
                object,
                property,
                value,
            } => Statement::AssignMember {
                object: self.substitute_global_static_constructor_expression(
                    object,
                    user_function,
                    arguments,
                    this_name,
                ),
                property: self.substitute_global_static_constructor_expression(
                    property,
                    user_function,
                    arguments,
                    this_name,
                ),
                value: self.substitute_global_static_constructor_expression(
                    value,
                    user_function,
                    arguments,
                    this_name,
                ),
            },
            Statement::Print { values } => Statement::Print {
                values: values
                    .iter()
                    .map(|value| {
                        self.substitute_global_static_constructor_expression(
                            value,
                            user_function,
                            arguments,
                            this_name,
                        )
                    })
                    .collect(),
            },
            Statement::Expression(expression) => {
                Statement::Expression(self.substitute_global_static_constructor_expression(
                    expression,
                    user_function,
                    arguments,
                    this_name,
                ))
            }
            Statement::Throw(expression) => {
                Statement::Throw(self.substitute_global_static_constructor_expression(
                    expression,
                    user_function,
                    arguments,
                    this_name,
                ))
            }
            Statement::Return(expression) => {
                Statement::Return(self.substitute_global_static_constructor_expression(
                    expression,
                    user_function,
                    arguments,
                    this_name,
                ))
            }
            Statement::With { object, body } => Statement::With {
                object: self.substitute_global_static_constructor_expression(
                    object,
                    user_function,
                    arguments,
                    this_name,
                ),
                body: body
                    .iter()
                    .map(|statement| {
                        self.substitute_global_static_constructor_statement(
                            statement,
                            user_function,
                            arguments,
                            this_name,
                        )
                    })
                    .collect(),
            },
            Statement::If {
                condition,
                then_branch,
                else_branch,
            } => Statement::If {
                condition: self.substitute_global_static_constructor_expression(
                    condition,
                    user_function,
                    arguments,
                    this_name,
                ),
                then_branch: then_branch
                    .iter()
                    .map(|statement| {
                        self.substitute_global_static_constructor_statement(
                            statement,
                            user_function,
                            arguments,
                            this_name,
                        )
                    })
                    .collect(),
                else_branch: else_branch
                    .iter()
                    .map(|statement| {
                        self.substitute_global_static_constructor_statement(
                            statement,
                            user_function,
                            arguments,
                            this_name,
                        )
                    })
                    .collect(),
            },
            Statement::Try {
                body,
                catch_binding,
                catch_setup,
                catch_body,
            } => Statement::Try {
                body: body
                    .iter()
                    .map(|statement| {
                        self.substitute_global_static_constructor_statement(
                            statement,
                            user_function,
                            arguments,
                            this_name,
                        )
                    })
                    .collect(),
                catch_binding: catch_binding.clone(),
                catch_setup: catch_setup
                    .iter()
                    .map(|statement| {
                        self.substitute_global_static_constructor_statement(
                            statement,
                            user_function,
                            arguments,
                            this_name,
                        )
                    })
                    .collect(),
                catch_body: catch_body
                    .iter()
                    .map(|statement| {
                        self.substitute_global_static_constructor_statement(
                            statement,
                            user_function,
                            arguments,
                            this_name,
                        )
                    })
                    .collect(),
            },
            Statement::Switch {
                labels,
                bindings,
                discriminant,
                cases,
            } => Statement::Switch {
                labels: labels.clone(),
                bindings: bindings.clone(),
                discriminant: self.substitute_global_static_constructor_expression(
                    discriminant,
                    user_function,
                    arguments,
                    this_name,
                ),
                cases: cases
                    .iter()
                    .map(|case| SwitchCase {
                        test: case.test.as_ref().map(|test| {
                            self.substitute_global_static_constructor_expression(
                                test,
                                user_function,
                                arguments,
                                this_name,
                            )
                        }),
                        body: case
                            .body
                            .iter()
                            .map(|statement| {
                                self.substitute_global_static_constructor_statement(
                                    statement,
                                    user_function,
                                    arguments,
                                    this_name,
                                )
                            })
                            .collect(),
                    })
                    .collect(),
            },
            Statement::For {
                labels,
                init,
                per_iteration_bindings,
                condition,
                update,
                break_hook,
                body,
            } => Statement::For {
                labels: labels.clone(),
                init: init
                    .iter()
                    .map(|statement| {
                        self.substitute_global_static_constructor_statement(
                            statement,
                            user_function,
                            arguments,
                            this_name,
                        )
                    })
                    .collect(),
                per_iteration_bindings: per_iteration_bindings.clone(),
                condition: condition.as_ref().map(|condition| {
                    self.substitute_global_static_constructor_expression(
                        condition,
                        user_function,
                        arguments,
                        this_name,
                    )
                }),
                update: update.as_ref().map(|update| {
                    self.substitute_global_static_constructor_expression(
                        update,
                        user_function,
                        arguments,
                        this_name,
                    )
                }),
                break_hook: break_hook.as_ref().map(|break_hook| {
                    self.substitute_global_static_constructor_expression(
                        break_hook,
                        user_function,
                        arguments,
                        this_name,
                    )
                }),
                body: body
                    .iter()
                    .map(|statement| {
                        self.substitute_global_static_constructor_statement(
                            statement,
                            user_function,
                            arguments,
                            this_name,
                        )
                    })
                    .collect(),
            },
            Statement::While {
                labels,
                condition,
                break_hook,
                body,
            } => Statement::While {
                labels: labels.clone(),
                condition: self.substitute_global_static_constructor_expression(
                    condition,
                    user_function,
                    arguments,
                    this_name,
                ),
                break_hook: break_hook.as_ref().map(|break_hook| {
                    self.substitute_global_static_constructor_expression(
                        break_hook,
                        user_function,
                        arguments,
                        this_name,
                    )
                }),
                body: body
                    .iter()
                    .map(|statement| {
                        self.substitute_global_static_constructor_statement(
                            statement,
                            user_function,
                            arguments,
                            this_name,
                        )
                    })
                    .collect(),
            },
            Statement::DoWhile {
                labels,
                condition,
                break_hook,
                body,
            } => Statement::DoWhile {
                labels: labels.clone(),
                condition: self.substitute_global_static_constructor_expression(
                    condition,
                    user_function,
                    arguments,
                    this_name,
                ),
                break_hook: break_hook.as_ref().map(|break_hook| {
                    self.substitute_global_static_constructor_expression(
                        break_hook,
                        user_function,
                        arguments,
                        this_name,
                    )
                }),
                body: body
                    .iter()
                    .map(|statement| {
                        self.substitute_global_static_constructor_statement(
                            statement,
                            user_function,
                            arguments,
                            this_name,
                        )
                    })
                    .collect(),
            },
            Statement::Yield { value } => Statement::Yield {
                value: self.substitute_global_static_constructor_expression(
                    value,
                    user_function,
                    arguments,
                    this_name,
                ),
            },
            Statement::YieldDelegate { value } => Statement::YieldDelegate {
                value: self.substitute_global_static_constructor_expression(
                    value,
                    user_function,
                    arguments,
                    this_name,
                ),
            },
            Statement::Break { label } => Statement::Break {
                label: label.clone(),
            },
            Statement::Continue { label } => Statement::Continue {
                label: label.clone(),
            },
        }
    }

    fn substitute_global_static_constructor_expression(
        &self,
        expression: &Expression,
        user_function: &UserFunction,
        arguments: &[CallArgument],
        this_name: &str,
    ) -> Expression {
        let expression = self.substitute_global_user_function_argument_bindings(
            expression,
            user_function,
            arguments,
        );
        Self::rewrite_global_static_constructor_expression(&expression, this_name)
    }

    fn rewrite_global_static_constructor_expression(
        expression: &Expression,
        this_name: &str,
    ) -> Expression {
        match expression {
            Expression::Member { object, property } => Expression::Member {
                object: Box::new(Self::rewrite_global_static_constructor_expression(
                    object, this_name,
                )),
                property: Box::new(Self::rewrite_global_static_constructor_expression(
                    property, this_name,
                )),
            },
            Expression::SuperMember { property } => Expression::SuperMember {
                property: Box::new(Self::rewrite_global_static_constructor_expression(
                    property, this_name,
                )),
            },
            Expression::Assign { name, value } => Expression::Assign {
                name: name.clone(),
                value: Box::new(Self::rewrite_global_static_constructor_expression(
                    value, this_name,
                )),
            },
            Expression::AssignMember {
                object,
                property,
                value,
            } => Expression::AssignMember {
                object: Box::new(Self::rewrite_global_static_constructor_expression(
                    object, this_name,
                )),
                property: Box::new(Self::rewrite_global_static_constructor_expression(
                    property, this_name,
                )),
                value: Box::new(Self::rewrite_global_static_constructor_expression(
                    value, this_name,
                )),
            },
            Expression::AssignSuperMember { property, value } => Expression::AssignSuperMember {
                property: Box::new(Self::rewrite_global_static_constructor_expression(
                    property, this_name,
                )),
                value: Box::new(Self::rewrite_global_static_constructor_expression(
                    value, this_name,
                )),
            },
            Expression::Await(value) => Expression::Await(Box::new(
                Self::rewrite_global_static_constructor_expression(value, this_name),
            )),
            Expression::EnumerateKeys(value) => Expression::EnumerateKeys(Box::new(
                Self::rewrite_global_static_constructor_expression(value, this_name),
            )),
            Expression::GetIterator(value) => Expression::GetIterator(Box::new(
                Self::rewrite_global_static_constructor_expression(value, this_name),
            )),
            Expression::IteratorClose(value) => Expression::IteratorClose(Box::new(
                Self::rewrite_global_static_constructor_expression(value, this_name),
            )),
            Expression::Unary { op, expression } => Expression::Unary {
                op: *op,
                expression: Box::new(Self::rewrite_global_static_constructor_expression(
                    expression, this_name,
                )),
            },
            Expression::Binary { op, left, right } => Expression::Binary {
                op: *op,
                left: Box::new(Self::rewrite_global_static_constructor_expression(
                    left, this_name,
                )),
                right: Box::new(Self::rewrite_global_static_constructor_expression(
                    right, this_name,
                )),
            },
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => Expression::Conditional {
                condition: Box::new(Self::rewrite_global_static_constructor_expression(
                    condition, this_name,
                )),
                then_expression: Box::new(Self::rewrite_global_static_constructor_expression(
                    then_expression,
                    this_name,
                )),
                else_expression: Box::new(Self::rewrite_global_static_constructor_expression(
                    else_expression,
                    this_name,
                )),
            },
            Expression::Sequence(expressions) => Expression::Sequence(
                expressions
                    .iter()
                    .map(|expression| {
                        Self::rewrite_global_static_constructor_expression(expression, this_name)
                    })
                    .collect(),
            ),
            Expression::Call { callee, arguments } => Expression::Call {
                callee: Box::new(Self::rewrite_global_static_constructor_expression(
                    callee, this_name,
                )),
                arguments: arguments
                    .iter()
                    .map(|argument| match argument {
                        CallArgument::Expression(expression) => CallArgument::Expression(
                            Self::rewrite_global_static_constructor_expression(
                                expression, this_name,
                            ),
                        ),
                        CallArgument::Spread(expression) => CallArgument::Spread(
                            Self::rewrite_global_static_constructor_expression(
                                expression, this_name,
                            ),
                        ),
                    })
                    .collect(),
            },
            Expression::SuperCall { callee, arguments } => Expression::SuperCall {
                callee: Box::new(Self::rewrite_global_static_constructor_expression(
                    callee, this_name,
                )),
                arguments: arguments
                    .iter()
                    .map(|argument| match argument {
                        CallArgument::Expression(expression) => CallArgument::Expression(
                            Self::rewrite_global_static_constructor_expression(
                                expression, this_name,
                            ),
                        ),
                        CallArgument::Spread(expression) => CallArgument::Spread(
                            Self::rewrite_global_static_constructor_expression(
                                expression, this_name,
                            ),
                        ),
                    })
                    .collect(),
            },
            Expression::New { callee, arguments } => Expression::New {
                callee: Box::new(Self::rewrite_global_static_constructor_expression(
                    callee, this_name,
                )),
                arguments: arguments
                    .iter()
                    .map(|argument| match argument {
                        CallArgument::Expression(expression) => CallArgument::Expression(
                            Self::rewrite_global_static_constructor_expression(
                                expression, this_name,
                            ),
                        ),
                        CallArgument::Spread(expression) => CallArgument::Spread(
                            Self::rewrite_global_static_constructor_expression(
                                expression, this_name,
                            ),
                        ),
                    })
                    .collect(),
            },
            Expression::Array(elements) => Expression::Array(
                elements
                    .iter()
                    .map(|element| match element {
                        ArrayElement::Expression(expression) => ArrayElement::Expression(
                            Self::rewrite_global_static_constructor_expression(
                                expression, this_name,
                            ),
                        ),
                        ArrayElement::Spread(expression) => ArrayElement::Spread(
                            Self::rewrite_global_static_constructor_expression(
                                expression, this_name,
                            ),
                        ),
                    })
                    .collect(),
            ),
            Expression::Object(entries) => Expression::Object(
                entries
                    .iter()
                    .map(|entry| match entry {
                        ObjectEntry::Data { key, value } => ObjectEntry::Data {
                            key: Self::rewrite_global_static_constructor_expression(key, this_name),
                            value: Self::rewrite_global_static_constructor_expression(
                                value, this_name,
                            ),
                        },
                        ObjectEntry::Getter { key, getter } => ObjectEntry::Getter {
                            key: Self::rewrite_global_static_constructor_expression(key, this_name),
                            getter: Self::rewrite_global_static_constructor_expression(
                                getter, this_name,
                            ),
                        },
                        ObjectEntry::Setter { key, setter } => ObjectEntry::Setter {
                            key: Self::rewrite_global_static_constructor_expression(key, this_name),
                            setter: Self::rewrite_global_static_constructor_expression(
                                setter, this_name,
                            ),
                        },
                        ObjectEntry::Spread(expression) => {
                            ObjectEntry::Spread(Self::rewrite_global_static_constructor_expression(
                                expression, this_name,
                            ))
                        }
                    })
                    .collect(),
            ),
            Expression::This => Expression::Identifier(this_name.to_string()),
            Expression::NewTarget => Expression::Bool(true),
            _ => expression.clone(),
        }
    }

    fn apply_global_static_constructor_statement_updates(
        &self,
        statements: &[Statement],
        environment: &mut GlobalStaticEvaluationEnvironment,
        this_name: &str,
    ) {
        for statement in statements {
            match statement {
                Statement::Var { name, value } | Statement::Let { name, value, .. } => {
                    let value =
                        self.resolve_global_static_constructor_binding_value(value, environment);
                    environment.set_local_binding(name.clone(), value);
                }
                Statement::Assign { name, value } => {
                    let value =
                        self.resolve_global_static_constructor_binding_value(value, environment);
                    environment.assign_binding_value(name.clone(), value);
                }
                Statement::AssignMember {
                    object,
                    property,
                    value,
                } => {
                    let Expression::Identifier(target_name) = object else {
                        continue;
                    };
                    let property = self
                        .evaluate_static_expression_with_state(property, environment)
                        .or_else(|| {
                            self.materialize_global_expression_with_state(
                                property,
                                &environment.local_bindings,
                                &environment.value_bindings,
                                &environment.object_bindings,
                            )
                        })
                        .unwrap_or_else(|| property.clone());
                    let value =
                        self.resolve_global_static_constructor_binding_value(value, environment);
                    if let Some(binding) = environment.object_binding_mut(target_name) {
                        object_binding_set_property(binding, property, value);
                    }
                }
                Statement::Declaration { body }
                | Statement::Block { body }
                | Statement::Labeled { body, .. }
                | Statement::With { body, .. } => {
                    self.apply_global_static_constructor_statement_updates(
                        body,
                        environment,
                        this_name,
                    );
                }
                Statement::If {
                    then_branch,
                    else_branch,
                    ..
                } => {
                    self.apply_global_static_constructor_statement_updates(
                        then_branch,
                        environment,
                        this_name,
                    );
                    self.apply_global_static_constructor_statement_updates(
                        else_branch,
                        environment,
                        this_name,
                    );
                }
                Statement::While { body, .. }
                | Statement::DoWhile { body, .. }
                | Statement::For { body, .. }
                | Statement::Try { body, .. } => {
                    self.apply_global_static_constructor_statement_updates(
                        body,
                        environment,
                        this_name,
                    );
                }
                Statement::Switch { cases, .. } => {
                    for case in cases {
                        self.apply_global_static_constructor_statement_updates(
                            &case.body,
                            environment,
                            this_name,
                        );
                    }
                }
                Statement::Expression(Expression::Call { callee, arguments })
                | Statement::Return(Expression::Call { callee, arguments })
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
                    let target_name = match target_expression {
                        Expression::Identifier(target_name) => target_name.as_str(),
                        _ => continue,
                    };
                    if target_name != this_name {
                        continue;
                    }
                    let Some(descriptor) =
                        resolve_property_descriptor_definition(descriptor_expression)
                    else {
                        continue;
                    };
                    let property = self
                        .evaluate_static_expression_with_state(property_expression, environment)
                        .or_else(|| {
                            self.materialize_global_expression_with_state(
                                property_expression,
                                &environment.local_bindings,
                                &environment.value_bindings,
                                &environment.object_bindings,
                            )
                        })
                        .unwrap_or_else(|| property_expression.clone());
                    let value = descriptor
                        .value
                        .as_ref()
                        .map(|value| {
                            self.resolve_global_static_constructor_binding_value(value, environment)
                        })
                        .unwrap_or(Expression::Undefined);
                    if let Some(binding) = environment.object_binding_mut(this_name) {
                        object_binding_define_property(
                            binding,
                            property,
                            value,
                            descriptor.enumerable.unwrap_or(true),
                        );
                    }
                }
                _ => {}
            }
        }
    }

    fn resolve_global_static_constructor_binding_value(
        &self,
        expression: &Expression,
        environment: &mut GlobalStaticEvaluationEnvironment,
    ) -> Expression {
        if let Expression::Call { callee, arguments } = expression
            && let Some(value) = self.infer_static_call_result_expression(callee, arguments)
        {
            return value;
        }
        self.evaluate_static_expression_with_state(expression, environment)
            .or_else(|| {
                self.materialize_global_expression_with_state(
                    expression,
                    &environment.local_bindings,
                    &environment.value_bindings,
                    &environment.object_bindings,
                )
            })
            .unwrap_or_else(|| expression.clone())
    }
}
