use super::*;

impl<'a> FunctionCompiler<'a> {
    fn private_member_class_name_matches(candidate: &str, class_name: &str) -> bool {
        candidate == class_name
            || candidate
                .strip_prefix(class_name)
                .is_some_and(|suffix| suffix.starts_with("____evalctx_"))
            || class_name
                .strip_prefix(candidate)
                .is_some_and(|suffix| suffix.starts_with("____evalctx_"))
    }

    fn private_member_target_matches_class_name(
        target: &MemberFunctionBindingTarget,
        class_name: &str,
    ) -> bool {
        match target {
            MemberFunctionBindingTarget::Identifier(name)
            | MemberFunctionBindingTarget::Prototype(name) => {
                Self::private_member_class_name_matches(name, class_name)
            }
        }
    }

    fn push_unique_private_member_binding_target(
        targets: &mut Vec<MemberFunctionBindingTarget>,
        candidate: &MemberFunctionBindingTarget,
    ) {
        if !targets.contains(candidate) {
            targets.push(candidate.clone());
        }
    }

    fn collect_private_member_binding_targets(
        &self,
        property_name: &str,
    ) -> Vec<MemberFunctionBindingTarget> {
        let expected_property = MemberFunctionBindingProperty::String(property_name.to_string());
        let mut targets = Vec::new();
        let global_member_function_keys = self
            .backend
            .global_member_function_binding_entries()
            .into_iter()
            .map(|(key, _)| key)
            .collect::<Vec<_>>();
        let global_member_getter_keys = self
            .backend
            .global_member_getter_binding_entries()
            .into_iter()
            .map(|(key, _)| key)
            .collect::<Vec<_>>();
        let global_member_setter_keys = self
            .backend
            .global_member_setter_binding_entries()
            .into_iter()
            .map(|(key, _)| key)
            .collect::<Vec<_>>();

        for key in self
            .state
            .speculation
            .static_semantics
            .objects
            .member_function_bindings
            .keys()
            .chain(
                self.state
                    .speculation
                    .static_semantics
                    .objects
                    .member_getter_bindings
                    .keys(),
            )
            .chain(
                self.state
                    .speculation
                    .static_semantics
                    .objects
                    .member_setter_bindings
                    .keys(),
            )
            .chain(global_member_function_keys.iter())
            .chain(global_member_getter_keys.iter())
            .chain(global_member_setter_keys.iter())
        {
            if key.property == expected_property {
                Self::push_unique_private_member_binding_target(&mut targets, &key.target);
            }
        }

        targets
    }

    fn private_member_declaring_class_name<'b>(&self, property_name: &'b str) -> Option<&'b str> {
        let remainder = property_name.strip_prefix("__ayy$private$")?;
        let (class_name, _) = remainder.rsplit_once('$')?;
        Some(class_name)
    }

    fn private_member_binding_target_with_context(
        &self,
        property: &Expression,
        current_function_name: Option<&str>,
    ) -> Option<MemberFunctionBindingTarget> {
        let Expression::String(name) = property else {
            return None;
        };
        if !name.starts_with("__ayy$private$") {
            return None;
        }

        let declaring_class_name = self.private_member_declaring_class_name(name);
        let mut targets = self.collect_private_member_binding_targets(name);
        if let Some(class_name) = declaring_class_name {
            let filtered_targets = targets
                .iter()
                .filter(|target| Self::private_member_target_matches_class_name(target, class_name))
                .cloned()
                .collect::<Vec<_>>();
            if !filtered_targets.is_empty() {
                targets = filtered_targets;
            }
        }
        if std::env::var_os("AYY_TRACE_PRIVATE_MEMBER_LOOKUP").is_some() {
            eprintln!(
                "private_binding_targets property={name} current_fn={current_function_name:?} declaring={declaring_class_name:?} targets={targets:?}"
            );
        }
        if let Some(current_function_name) = current_function_name
            && let Some(home_object_name) =
                self.resolve_home_object_name_for_function(current_function_name)
        {
            let home_object_target =
                if let Some(class_name) = home_object_name.strip_suffix(".prototype") {
                    MemberFunctionBindingTarget::Prototype(class_name.to_string())
                } else {
                    MemberFunctionBindingTarget::Identifier(home_object_name)
                };

            if targets.contains(&home_object_target) {
                return Some(home_object_target);
            }

            if declaring_class_name.is_none_or(|class_name| {
                Self::private_member_target_matches_class_name(&home_object_target, class_name)
            }) {
                if !targets.contains(&home_object_target)
                    && let [target] = targets.as_slice()
                {
                    return Some(target.clone());
                }
                return Some(home_object_target);
            }
        }

        match targets.as_slice() {
            [target] => Some(target.clone()),
            _ => None,
        }
    }

    fn normalize_member_function_binding_identifier_target(&self, name: &str) -> String {
        self.resolve_registered_function_declaration(name)
            .and_then(|function| function.self_binding.as_ref())
            .or_else(|| {
                self.resolve_registered_function_declaration(name)
                    .and_then(|function| function.top_level_binding.as_ref())
            })
            .cloned()
            .or_else(|| scoped_binding_source_name(name).map(str::to_string))
            .unwrap_or_else(|| name.to_string())
    }

    fn resolve_member_function_binding_identifier_source(&self, name: &str) -> Option<Expression> {
        let source_name = self.resolve_capture_slot_source_binding_name(name)?;
        let source_expression = Expression::Identifier(source_name);
        self.resolve_bound_alias_expression(&source_expression)
            .filter(|resolved| !static_expression_matches(resolved, &source_expression))
            .or(Some(source_expression))
    }

    fn member_function_binding_prototype_target(
        &self,
        expression: &Expression,
    ) -> Option<MemberFunctionBindingTarget> {
        match expression {
            Expression::Member { object, property } if matches!(property.as_ref(), Expression::String(name) if name == "prototype") =>
            {
                let resolved_object = self
                    .resolve_bound_alias_expression(object)
                    .filter(|resolved| !static_expression_matches(resolved, object))
                    .unwrap_or_else(|| object.as_ref().clone());
                let Expression::Identifier(name) = resolved_object else {
                    return None;
                };
                Some(MemberFunctionBindingTarget::Prototype(
                    self.normalize_member_function_binding_identifier_target(&name),
                ))
            }
            Expression::New { callee, .. } => {
                let resolved_callee = self
                    .resolve_bound_alias_expression(callee)
                    .filter(|resolved| !static_expression_matches(resolved, callee))
                    .unwrap_or_else(|| callee.as_ref().clone());
                let Expression::Identifier(name) = resolved_callee else {
                    return None;
                };
                Some(MemberFunctionBindingTarget::Prototype(
                    self.normalize_member_function_binding_identifier_target(&name),
                ))
            }
            _ => None,
        }
    }

    pub(in crate::backend::direct_wasm) fn member_function_binding_key(
        &self,
        object: &Expression,
        property: &Expression,
    ) -> Option<MemberFunctionBindingKey> {
        self.member_function_binding_key_with_context(
            object,
            property,
            self.current_function_name(),
        )
    }

    pub(in crate::backend::direct_wasm) fn member_function_binding_key_without_runtime_public_this_guard(
        &self,
        object: &Expression,
        property: &Expression,
    ) -> Option<MemberFunctionBindingKey> {
        self.member_function_binding_key_with_context_and_runtime_public_this_guard(
            object,
            property,
            self.current_function_name(),
            false,
        )
    }

    pub(in crate::backend::direct_wasm) fn member_function_binding_key_with_context(
        &self,
        object: &Expression,
        property: &Expression,
        current_function_name: Option<&str>,
    ) -> Option<MemberFunctionBindingKey> {
        self.member_function_binding_key_with_context_and_runtime_public_this_guard(
            object,
            property,
            current_function_name,
            true,
        )
    }

    fn member_function_binding_key_with_context_and_runtime_public_this_guard(
        &self,
        object: &Expression,
        property: &Expression,
        current_function_name: Option<&str>,
        guard_runtime_public_this_resolution: bool,
    ) -> Option<MemberFunctionBindingKey> {
        let target = if let Some(target) =
            self.private_member_binding_target_with_context(property, current_function_name)
        {
            target
        } else {
            match object {
                Expression::This => {
                    if guard_runtime_public_this_resolution
                        && self.current_function_requires_runtime_public_this_resolution()
                    {
                        return None;
                    }
                    let current_function_name = current_function_name?;
                    let home_object_name =
                        self.resolve_home_object_name_for_function(current_function_name)?;
                    if let Some(class_name) = home_object_name.strip_suffix(".prototype") {
                        MemberFunctionBindingTarget::Prototype(class_name.to_string())
                    } else {
                        MemberFunctionBindingTarget::Identifier(home_object_name)
                    }
                }
                Expression::Identifier(name) => self
                    .resolve_member_function_binding_identifier_source(name)
                    .or_else(|| {
                        self.resolve_bound_alias_expression(object)
                            .filter(|resolved| !static_expression_matches(resolved, object))
                    })
                    .and_then(|resolved| {
                        if matches!(resolved, Expression::This) {
                            self.private_member_binding_target_with_context(
                                property,
                                current_function_name,
                            )
                        } else {
                            self.member_function_binding_prototype_target(&resolved)
                                .or_else(|| {
                                    match resolved {
                                Expression::Identifier(resolved_name) => {
                                    Some(MemberFunctionBindingTarget::Identifier(
                                        self.normalize_member_function_binding_identifier_target(
                                            &resolved_name,
                                        ),
                                    ))
                                }
                                _ => None,
                            }
                                })
                        }
                    })
                    .unwrap_or_else(|| {
                        MemberFunctionBindingTarget::Identifier(
                            self.normalize_member_function_binding_identifier_target(name),
                        )
                    }),
                _ => self.member_function_binding_prototype_target(object)?,
            }
        };

        let property = self.member_function_binding_property(property)?;

        Some(MemberFunctionBindingKey { target, property })
    }

    pub(in crate::backend::direct_wasm) fn identifier_member_function_binding_fallback_keys(
        &self,
        name: &str,
        property: &Expression,
    ) -> Vec<MemberFunctionBindingKey> {
        let Some(property) = self.member_function_binding_property(property) else {
            return Vec::new();
        };
        let mut names = Vec::new();
        let mut push_name = |candidate: String| {
            if !names.iter().any(|name| name == &candidate) {
                names.push(candidate);
            }
        };
        push_name(name.to_string());
        push_name(self.normalize_member_function_binding_identifier_target(name));
        if let Some(source_name) = self.resolve_capture_slot_source_binding_name(name) {
            push_name(source_name);
        }
        let mut static_alias_names = Vec::new();
        if let Some((resolved_name, _)) = self.resolve_current_local_binding(name) {
            static_alias_names.push(resolved_name.clone());
            if let Some(Expression::Identifier(alias_name)) = self
                .state
                .speculation
                .static_semantics
                .local_value_binding(&resolved_name)
            {
                static_alias_names.push(alias_name.clone());
                static_alias_names
                    .push(self.normalize_member_function_binding_identifier_target(alias_name));
            }
        }
        if let Some(Expression::Identifier(alias_name)) = self
            .state
            .speculation
            .static_semantics
            .local_value_binding(name)
            .or_else(|| self.backend.global_value_binding(name))
        {
            static_alias_names.push(alias_name.clone());
            static_alias_names
                .push(self.normalize_member_function_binding_identifier_target(alias_name));
        }
        for alias_name in static_alias_names {
            push_name(alias_name);
        }
        let source_expression = Expression::Identifier(name.to_string());
        if let Some(Expression::Identifier(alias_name)) = self
            .resolve_bound_alias_expression(&source_expression)
            .filter(|resolved| !static_expression_matches(resolved, &source_expression))
        {
            push_name(alias_name.clone());
            push_name(self.normalize_member_function_binding_identifier_target(&alias_name));
        }

        names
            .into_iter()
            .map(|name| MemberFunctionBindingKey {
                target: MemberFunctionBindingTarget::Identifier(name),
                property: property.clone(),
            })
            .collect()
    }
}
