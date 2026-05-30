use super::*;

impl<'a> FunctionCompiler<'a> {
    fn collect_module_init_call_effect_nonlocal_bindings_for_module_index(
        &self,
        module_index: usize,
        names: &mut HashSet<String>,
        visited: &mut HashSet<String>,
        seen_modules: &mut HashSet<usize>,
    ) {
        if !seen_modules.insert(module_index) {
            return;
        }

        let init_name = format!("__ayy_module_init_{module_index}");
        if let Some(init_function) = self.user_function(&init_name) {
            for parameter in init_function.params.iter().skip(1) {
                let Some(dependency_index) = parameter
                    .strip_prefix("__ayy_module_dep_")
                    .and_then(|index| index.parse::<usize>().ok())
                else {
                    continue;
                };
                if self
                    .backend
                    .global_binding_index(&format!(
                        "__ayy_module_eager_dependency_{module_index}_{dependency_index}"
                    ))
                    .is_some()
                {
                    self.collect_module_init_call_effect_nonlocal_bindings_for_module_index(
                        dependency_index,
                        names,
                        visited,
                        seen_modules,
                    );
                }
            }
        }

        names.extend(
            self.collect_user_function_call_effect_nonlocal_bindings_for_name(&init_name, visited),
        );
    }

    fn deferred_module_namespace_super_call_effect_property_may_trigger(
        &self,
        property: &Expression,
    ) -> bool {
        let property_key = self
            .resolve_property_key_expression(property)
            .or_else(|| {
                if let Expression::Identifier(name) = property {
                    self.state
                        .speculation
                        .static_semantics
                        .local_value_binding(name)
                        .or_else(|| self.global_value_binding(name))
                        .cloned()
                        .and_then(|value| self.resolve_property_key_expression(&value))
                } else {
                    None
                }
            })
            .unwrap_or_else(|| self.canonical_object_property_expression(property));
        let Some(property_name) = static_property_name_from_expression(&property_key) else {
            return true;
        };
        property_name != "then" && !property_name.starts_with("__ayy$")
    }

    fn collect_deferred_module_namespace_super_member_call_effects(
        &self,
        property: &Expression,
        current_function_name: Option<&str>,
        names: &mut HashSet<String>,
        visited: &mut HashSet<String>,
    ) {
        if !self.deferred_module_namespace_super_call_effect_property_may_trigger(property) {
            return;
        }
        let Some(super_base) =
            self.resolve_super_base_expression_with_context(current_function_name)
        else {
            return;
        };

        let mut candidate_bases = Vec::new();
        candidate_bases.push(super_base.clone());
        let materialized_base = self.materialize_static_expression(&super_base);
        if !static_expression_matches(&materialized_base, &super_base) {
            candidate_bases.push(materialized_base);
        }

        let Some(module_index) = candidate_bases.into_iter().find_map(|candidate| {
            let Expression::Identifier(name) = candidate else {
                return None;
            };
            Self::module_index_from_namespace_like_identifier(&name)
        }) else {
            return;
        };
        if current_function_name.is_some_and(|function_name| {
            function_name == format!("__ayy_module_init_{module_index}")
        }) {
            return;
        }

        self.collect_module_init_call_effect_nonlocal_bindings_for_module_index(
            module_index,
            names,
            visited,
            &mut HashSet::new(),
        );
    }

    fn collect_member_assignment_call_effect_target(
        &self,
        object: &Expression,
        names: &mut HashSet<String>,
    ) {
        match object {
            Expression::Identifier(name) => {
                names.insert(name.clone());
            }
            Expression::This => {
                names.insert("this".to_string());
            }
            Expression::Member { object, property } => {
                let canonical_property = self.canonical_object_property_expression(property);
                if let Some(shadow_binding_name) = self
                    .runtime_object_property_shadow_binding_name_for_expression(
                        object,
                        &canonical_property,
                    )
                {
                    names.insert(shadow_binding_name);
                }
                self.collect_member_assignment_call_effect_target(object, names);
            }
            _ => {}
        }
    }

    pub(in crate::backend::direct_wasm) fn collect_expression_call_effect_nonlocal_bindings(
        &self,
        expression: &Expression,
        current_function_name: Option<&str>,
        names: &mut HashSet<String>,
        visited: &mut HashSet<String>,
    ) {
        if let Expression::Call { callee, arguments }
        | Expression::SuperCall { callee, arguments }
        | Expression::New { callee, arguments } = expression
            && let Expression::Member { object, property } = callee.as_ref()
            && let Expression::String(property_name) = property.as_ref()
        {
            let object_is_promise_like_chain = Self::call_is_promise_like_chain(object);
            let object_is_async_user_call = matches!(property_name.as_str(), "then" | "catch")
                && !object_is_promise_like_chain
                && if let Expression::Call { callee, .. } = object.as_ref() {
                    self.resolve_function_binding_from_expression_with_context(
                        callee,
                        current_function_name,
                    )
                    .is_some_and(|binding| {
                        let LocalFunctionBinding::User(function_name) = binding else {
                            return false;
                        };
                        self.user_function(&function_name)
                            .is_some_and(|function| function.is_async())
                    })
                } else {
                    false
                };
            let is_promise_protocol_call = matches!(property_name.as_str(), "then" | "catch")
                && (object_is_promise_like_chain || object_is_async_user_call);
            let object_is_async_generator_iterator =
                matches!(property_name.as_str(), "next" | "return" | "throw")
                    && self.is_async_generator_iterator_expression(object);
            let object_has_simple_generator_metadata =
                matches!(property_name.as_str(), "next" | "return" | "throw")
                    && self.simple_generator_source_metadata(object).is_some();
            let is_generator_protocol_call =
                matches!(property_name.as_str(), "next" | "return" | "throw")
                    && (object_is_async_generator_iterator || object_has_simple_generator_metadata);
            let is_function_meta_call = matches!(property_name.as_str(), "call" | "apply");
            let is_tracked_mutating_member_call = matches!(property_name.as_str(), "push");
            if is_tracked_mutating_member_call {
                self.collect_member_assignment_call_effect_target(object, names);
            }
            if property_name == "replace"
                && let [
                    CallArgument::Expression(_search_expression),
                    CallArgument::Expression(replacement_expression),
                    ..,
                ] = arguments.as_slice()
                && let Some(LocalFunctionBinding::User(function_name)) = self
                    .resolve_function_binding_from_expression_with_context(
                        replacement_expression,
                        current_function_name,
                    )
            {
                names.extend(
                    self.collect_user_function_call_effect_nonlocal_bindings_for_name(
                        &function_name,
                        visited,
                    ),
                );
            }
            if is_promise_protocol_call || is_generator_protocol_call || is_function_meta_call {
                self.collect_expression_call_effect_nonlocal_bindings(
                    object,
                    current_function_name,
                    names,
                    visited,
                );
                for argument in arguments {
                    match argument {
                        CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                            if is_promise_protocol_call
                                && matches!(property_name.as_str(), "then" | "catch" | "finally")
                                && let Some(LocalFunctionBinding::User(function_name)) = self
                                    .resolve_function_binding_from_expression_with_context(
                                        expression,
                                        current_function_name,
                                    )
                            {
                                names.extend(
                                    self.collect_user_function_call_effect_nonlocal_bindings_for_name(
                                        &function_name,
                                        visited,
                                    ),
                                );
                            }
                            self.collect_expression_call_effect_nonlocal_bindings(
                                expression,
                                current_function_name,
                                names,
                                visited,
                            );
                        }
                    }
                }
                return;
            }
        }
        match expression {
            Expression::Call { callee, arguments }
            | Expression::SuperCall { callee, arguments }
            | Expression::New { callee, arguments } => {
                if matches!(callee.as_ref(), Expression::Identifier(name) if name == "__ayyDynamicImport")
                {
                    for argument in arguments {
                        match argument {
                            CallArgument::Expression(expression)
                            | CallArgument::Spread(expression) => {
                                self.collect_expression_call_effect_nonlocal_bindings(
                                    expression,
                                    current_function_name,
                                    names,
                                    visited,
                                );
                            }
                        }
                    }
                    return;
                }
                if let Some(LocalFunctionBinding::User(function_name)) = self
                    .resolve_function_binding_from_expression_with_context(
                        callee,
                        current_function_name,
                    )
                {
                    names.extend(
                        self.collect_user_function_call_effect_nonlocal_bindings_for_name(
                            &function_name,
                            visited,
                        ),
                    );
                }
                self.collect_expression_call_effect_nonlocal_bindings(
                    callee,
                    current_function_name,
                    names,
                    visited,
                );
                for argument in arguments {
                    match argument {
                        CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                            self.collect_expression_call_effect_nonlocal_bindings(
                                expression,
                                current_function_name,
                                names,
                                visited,
                            );
                        }
                    }
                }
            }
            Expression::AssignMember {
                object,
                property,
                value,
            } => {
                self.collect_member_assignment_call_effect_target(object, names);
                if let Some(LocalFunctionBinding::User(function_name)) =
                    self.resolve_member_setter_binding(object, property)
                {
                    names.extend(
                        self.collect_user_function_call_effect_nonlocal_bindings_for_name(
                            &function_name,
                            visited,
                        ),
                    );
                }
                self.collect_expression_call_effect_nonlocal_bindings(
                    object,
                    current_function_name,
                    names,
                    visited,
                );
                self.collect_expression_call_effect_nonlocal_bindings(
                    property,
                    current_function_name,
                    names,
                    visited,
                );
                self.collect_expression_call_effect_nonlocal_bindings(
                    value,
                    current_function_name,
                    names,
                    visited,
                );
            }
            Expression::AssignSuperMember { property, value } => {
                names.insert("this".to_string());
                if let Some(effective_property) = self.resolve_property_key_expression(property) {
                    if let Some((_, binding)) = self
                        .resolve_super_runtime_prototype_binding_with_context(current_function_name)
                    {
                        if let Some(variants) =
                            self.resolve_user_super_setter_variants(&binding, &effective_property)
                        {
                            for (user_function, _) in variants {
                                names.extend(
                                    self.collect_user_function_call_effect_nonlocal_bindings_for_name(
                                        &user_function.name,
                                        visited,
                                    ),
                                );
                            }
                        }
                    } else if let Some(super_base) =
                        self.resolve_super_base_expression_with_context(current_function_name)
                        && let Some(LocalFunctionBinding::User(function_name)) =
                            self.resolve_member_setter_binding(&super_base, &effective_property)
                    {
                        names.extend(
                            self.collect_user_function_call_effect_nonlocal_bindings_for_name(
                                &function_name,
                                visited,
                            ),
                        );
                    }
                }
                self.collect_expression_call_effect_nonlocal_bindings(
                    property,
                    current_function_name,
                    names,
                    visited,
                );
                self.collect_expression_call_effect_nonlocal_bindings(
                    value,
                    current_function_name,
                    names,
                    visited,
                );
            }
            Expression::Member { object, property } => {
                self.collect_expression_call_effect_nonlocal_bindings(
                    object,
                    current_function_name,
                    names,
                    visited,
                );
                self.collect_expression_call_effect_nonlocal_bindings(
                    property,
                    current_function_name,
                    names,
                    visited,
                );
            }
            Expression::SuperMember { property } => {
                self.collect_deferred_module_namespace_super_member_call_effects(
                    property,
                    current_function_name,
                    names,
                    visited,
                );
                self.collect_expression_call_effect_nonlocal_bindings(
                    property,
                    current_function_name,
                    names,
                    visited,
                );
            }
            Expression::IteratorClose(value) => {
                let return_property = Expression::String("return".to_string());
                if let Some(LocalFunctionBinding::User(function_name)) = self
                    .resolve_member_function_binding(value, &return_property)
                    .or_else(|| {
                        let Expression::Identifier(iterator_name) = value.as_ref() else {
                            return None;
                        };
                        self.resolve_iterator_close_return_binding_in_function(
                            iterator_name,
                            current_function_name,
                        )
                    })
                {
                    names.extend(
                        self.collect_user_function_call_effect_nonlocal_bindings_for_name(
                            &function_name,
                            visited,
                        ),
                    );
                }
                self.collect_expression_call_effect_nonlocal_bindings(
                    value,
                    current_function_name,
                    names,
                    visited,
                );
            }
            Expression::Assign { value, .. }
            | Expression::Await(value)
            | Expression::EnumerateKeys(value)
            | Expression::GetIterator(value)
            | Expression::Unary {
                expression: value, ..
            } => self.collect_expression_call_effect_nonlocal_bindings(
                value,
                current_function_name,
                names,
                visited,
            ),
            Expression::Binary { left, right, .. } => {
                self.collect_expression_call_effect_nonlocal_bindings(
                    left,
                    current_function_name,
                    names,
                    visited,
                );
                self.collect_expression_call_effect_nonlocal_bindings(
                    right,
                    current_function_name,
                    names,
                    visited,
                );
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                self.collect_expression_call_effect_nonlocal_bindings(
                    condition,
                    current_function_name,
                    names,
                    visited,
                );
                self.collect_expression_call_effect_nonlocal_bindings(
                    then_expression,
                    current_function_name,
                    names,
                    visited,
                );
                self.collect_expression_call_effect_nonlocal_bindings(
                    else_expression,
                    current_function_name,
                    names,
                    visited,
                );
            }
            Expression::Sequence(expressions) => {
                for expression in expressions {
                    self.collect_expression_call_effect_nonlocal_bindings(
                        expression,
                        current_function_name,
                        names,
                        visited,
                    );
                }
            }
            Expression::Array(elements) => {
                for element in elements {
                    match element {
                        ArrayElement::Expression(expression) | ArrayElement::Spread(expression) => {
                            self.collect_expression_call_effect_nonlocal_bindings(
                                expression,
                                current_function_name,
                                names,
                                visited,
                            );
                        }
                    }
                }
            }
            Expression::Object(entries) => {
                for entry in entries {
                    match entry {
                        ObjectEntry::Data { key, value } => {
                            self.collect_expression_call_effect_nonlocal_bindings(
                                key,
                                current_function_name,
                                names,
                                visited,
                            );
                            self.collect_expression_call_effect_nonlocal_bindings(
                                value,
                                current_function_name,
                                names,
                                visited,
                            );
                        }
                        ObjectEntry::Getter { key, .. } | ObjectEntry::Setter { key, .. } => {
                            self.collect_expression_call_effect_nonlocal_bindings(
                                key,
                                current_function_name,
                                names,
                                visited,
                            );
                        }
                        ObjectEntry::Spread(expression) => {
                            self.collect_expression_call_effect_nonlocal_bindings(
                                expression,
                                current_function_name,
                                names,
                                visited,
                            );
                        }
                    }
                }
            }
            Expression::Identifier(_)
            | Expression::Update { .. }
            | Expression::Number(_)
            | Expression::BigInt(_)
            | Expression::String(_)
            | Expression::Bool(_)
            | Expression::Null
            | Expression::Undefined
            | Expression::NewTarget
            | Expression::Sent
            | Expression::This => {}
        }
    }
}
