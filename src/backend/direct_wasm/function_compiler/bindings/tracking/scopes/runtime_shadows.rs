use super::*;

thread_local! {
    static ACTIVE_RUNTIME_SHADOW_FALLBACKS: std::cell::RefCell<std::collections::HashSet<String>> =
        std::cell::RefCell::new(std::collections::HashSet::new());
}

struct RuntimeShadowFallbackGuard {
    key: String,
}

impl RuntimeShadowFallbackGuard {
    fn enter(fallback_value: &Expression) -> Option<Self> {
        let key = format!("{fallback_value:?}");
        let inserted =
            ACTIVE_RUNTIME_SHADOW_FALLBACKS.with(|active| active.borrow_mut().insert(key.clone()));
        inserted.then_some(Self { key })
    }
}

impl Drop for RuntimeShadowFallbackGuard {
    fn drop(&mut self) {
        ACTIVE_RUNTIME_SHADOW_FALLBACKS.with(|active| {
            active.borrow_mut().remove(&self.key);
        });
    }
}

impl<'a> FunctionCompiler<'a> {
    pub(in crate::backend::direct_wasm) fn reference_preserving_static_value_expression(
        &self,
        value: &Expression,
    ) -> Expression {
        let preserve_reference_alias =
            matches!(value, Expression::Identifier(_) | Expression::This)
                && (self
                    .runtime_array_binding_name_for_expression(value)
                    .is_some()
                    || self.resolve_array_binding_from_expression(value).is_some()
                    || self.resolve_object_binding_from_expression(value).is_some()
                    || self
                        .resolve_function_binding_from_expression(value)
                        .is_some());
        if preserve_reference_alias {
            value.clone()
        } else {
            self.materialize_static_expression(value)
        }
    }

    pub(in crate::backend::direct_wasm) fn emit_runtime_shadow_debug_print_local(
        &mut self,
        label: &str,
        value_local: u32,
    ) -> DirectResult<()> {
        let hidden_name =
            self.allocate_named_hidden_local("runtime_shadow_debug", StaticValueKind::Unknown);
        let hidden_local = self
            .state
            .runtime
            .locals
            .get(&hidden_name)
            .copied()
            .expect("fresh runtime shadow debug local must exist");
        self.push_local_get(value_local);
        self.push_local_set(hidden_local);
        self.emit_print(&[
            Expression::String(label.to_string()),
            Expression::Identifier(hidden_name),
        ])
    }

    pub(in crate::backend::direct_wasm) fn resolve_identifier_object_binding_fallback(
        &self,
        name: &str,
    ) -> Option<ObjectValueBinding> {
        self.current_function_name()
            .and_then(|function_name| {
                self.backend
                    .function_registry
                    .parameter_bindings_for(function_name)
                    .object_bindings
                    .get(name)
                    .cloned()
                    .flatten()
            })
            .or_else(|| self.global_object_binding(name).cloned())
    }

    fn runtime_object_property_shadow_fragment(text: &str) -> String {
        text.as_bytes()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    }

    fn runtime_object_property_shadow_key(property: &Expression) -> String {
        if let Some(property_name) = static_property_name_from_expression(property) {
            return format!(
                "str__{}",
                Self::runtime_object_property_shadow_fragment(&property_name)
            );
        }

        format!(
            "expr__{}",
            Self::runtime_object_property_shadow_fragment(&format!("{property:?}"))
        )
    }

    fn runtime_object_property_deleted_shadow_name(
        owner_name: &str,
        property: &Expression,
    ) -> String {
        format!(
            "__ayy_object_property_deleted__{owner_name}__{}",
            Self::runtime_object_property_shadow_key(property)
        )
    }

    fn runtime_object_property_shadow_owner_has_bindings(&self, owner_name: &str) -> bool {
        let property_prefix = format!("__ayy_object_property__{owner_name}__");
        let deleted_prefix = format!("__ayy_object_property_deleted__{owner_name}__");
        self.backend
            .global_semantics
            .global_names()
            .implicit_bindings
            .keys()
            .any(|name| name.starts_with(&property_prefix) || name.starts_with(&deleted_prefix))
    }

    pub(in crate::backend::direct_wasm) fn user_function_arguments_slot_object_shadow_owner_name(
        function_name: &str,
        index: u32,
    ) -> String {
        format!("__ayy_arguments_object_slot_{function_name}_{index}")
    }

    fn direct_arguments_slot_member_assignment_property(
        object: &Expression,
        property: &Expression,
        index: u32,
    ) -> Option<String> {
        let Expression::Member {
            object: base_object,
            property: base_property,
        } = object
        else {
            return None;
        };
        let Expression::Identifier(base_name) = base_object.as_ref() else {
            return None;
        };
        if scoped_binding_source_name(base_name).unwrap_or(base_name) != "arguments" {
            return None;
        }
        (argument_index_from_expression(base_property) == Some(index))
            .then(|| static_property_name_from_expression(property))
            .flatten()
    }

    fn collect_direct_arguments_slot_member_assignment_properties_from_expression(
        expression: &Expression,
        index: u32,
        properties: &mut BTreeSet<String>,
    ) {
        match expression {
            Expression::AssignMember {
                object,
                property,
                value,
            } => {
                if let Some(property_name) =
                    Self::direct_arguments_slot_member_assignment_property(object, property, index)
                {
                    properties.insert(property_name);
                }
                Self::collect_direct_arguments_slot_member_assignment_properties_from_expression(
                    object, index, properties,
                );
                Self::collect_direct_arguments_slot_member_assignment_properties_from_expression(
                    property, index, properties,
                );
                Self::collect_direct_arguments_slot_member_assignment_properties_from_expression(
                    value, index, properties,
                );
            }
            Expression::Member { object, property } => {
                Self::collect_direct_arguments_slot_member_assignment_properties_from_expression(
                    object, index, properties,
                );
                Self::collect_direct_arguments_slot_member_assignment_properties_from_expression(
                    property, index, properties,
                );
            }
            Expression::Assign { value, .. }
            | Expression::AssignSuperMember { value, .. }
            | Expression::Await(value)
            | Expression::EnumerateKeys(value)
            | Expression::GetIterator(value)
            | Expression::IteratorClose(value)
            | Expression::Unary {
                expression: value, ..
            } => {
                Self::collect_direct_arguments_slot_member_assignment_properties_from_expression(
                    value, index, properties,
                );
            }
            Expression::SuperMember { property } => {
                Self::collect_direct_arguments_slot_member_assignment_properties_from_expression(
                    property, index, properties,
                );
            }
            Expression::Binary { left, right, .. } => {
                Self::collect_direct_arguments_slot_member_assignment_properties_from_expression(
                    left, index, properties,
                );
                Self::collect_direct_arguments_slot_member_assignment_properties_from_expression(
                    right, index, properties,
                );
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                Self::collect_direct_arguments_slot_member_assignment_properties_from_expression(
                    condition, index, properties,
                );
                Self::collect_direct_arguments_slot_member_assignment_properties_from_expression(
                    then_expression,
                    index,
                    properties,
                );
                Self::collect_direct_arguments_slot_member_assignment_properties_from_expression(
                    else_expression,
                    index,
                    properties,
                );
            }
            Expression::Sequence(expressions) => {
                for expression in expressions {
                    Self::collect_direct_arguments_slot_member_assignment_properties_from_expression(
                        expression, index, properties,
                    );
                }
            }
            Expression::Call { callee, arguments }
            | Expression::SuperCall { callee, arguments }
            | Expression::New { callee, arguments } => {
                Self::collect_direct_arguments_slot_member_assignment_properties_from_expression(
                    callee, index, properties,
                );
                for argument in arguments {
                    Self::collect_direct_arguments_slot_member_assignment_properties_from_expression(
                        argument.expression(),
                        index,
                        properties,
                    );
                }
            }
            Expression::Array(elements) => {
                for element in elements {
                    match element {
                        ArrayElement::Expression(expression) | ArrayElement::Spread(expression) => {
                            Self::collect_direct_arguments_slot_member_assignment_properties_from_expression(
                                expression, index, properties,
                            );
                        }
                    }
                }
            }
            Expression::Object(entries) => {
                for entry in entries {
                    match entry {
                        ObjectEntry::Data { key, value } => {
                            Self::collect_direct_arguments_slot_member_assignment_properties_from_expression(
                                key, index, properties,
                            );
                            Self::collect_direct_arguments_slot_member_assignment_properties_from_expression(
                                value, index, properties,
                            );
                        }
                        ObjectEntry::Getter { key, getter } => {
                            Self::collect_direct_arguments_slot_member_assignment_properties_from_expression(
                                key, index, properties,
                            );
                            Self::collect_direct_arguments_slot_member_assignment_properties_from_expression(
                                getter, index, properties,
                            );
                        }
                        ObjectEntry::Setter { key, setter } => {
                            Self::collect_direct_arguments_slot_member_assignment_properties_from_expression(
                                key, index, properties,
                            );
                            Self::collect_direct_arguments_slot_member_assignment_properties_from_expression(
                                setter, index, properties,
                            );
                        }
                        ObjectEntry::Spread(expression) => {
                            Self::collect_direct_arguments_slot_member_assignment_properties_from_expression(
                                expression, index, properties,
                            );
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn direct_arguments_slot_assignment_properties(
        user_function: &UserFunction,
        index: u32,
    ) -> Vec<String> {
        let mut properties = BTreeSet::new();
        if let Some(summary) = user_function.inline_summary.as_ref() {
            for effect in &summary.effects {
                match effect {
                    InlineFunctionEffect::Assign { value, .. }
                    | InlineFunctionEffect::Expression(value) => {
                        Self::collect_direct_arguments_slot_member_assignment_properties_from_expression(
                            value,
                            index,
                            &mut properties,
                        );
                    }
                    InlineFunctionEffect::Update { .. } => {}
                }
            }
            if let Some(return_value) = summary.return_value.as_ref() {
                Self::collect_direct_arguments_slot_member_assignment_properties_from_expression(
                    return_value,
                    index,
                    &mut properties,
                );
            }
        }
        properties.into_iter().collect()
    }

    fn predeclare_runtime_shadow_property(&mut self, owner_name: &str, property_name: &str) {
        let property = Expression::String(property_name.to_string());
        self.runtime_object_property_shadow_binding_by_property(owner_name, &property);
        self.runtime_object_property_shadow_deleted_binding_by_property(owner_name, &property);
        let mut object_binding = self
            .global_object_binding(owner_name)
            .cloned()
            .unwrap_or_else(empty_object_value_binding);
        object_binding_set_property(&mut object_binding, property, Expression::Undefined);
        self.sync_runtime_object_shadow_owner_static_metadata_from_binding(
            owner_name,
            &object_binding,
        );
    }

    fn runtime_object_property_name_from_shadow_suffix(suffix: &str) -> Option<String> {
        let hex = suffix.strip_prefix("str__")?;
        if hex.len() % 2 != 0 {
            return None;
        }
        let bytes = (0..hex.len())
            .step_by(2)
            .map(|index| u8::from_str_radix(&hex[index..index + 2], 16).ok())
            .collect::<Option<Vec<_>>>()?;
        String::from_utf8(bytes).ok()
    }

    fn object_runtime_shadow_entries_from_binding(
        &self,
        object_binding: &ObjectValueBinding,
    ) -> Vec<(Expression, Expression)> {
        let mut entries = ordered_object_property_names(object_binding)
            .into_iter()
            .filter_map(|property_name| {
                object_binding_lookup_value(
                    object_binding,
                    &Expression::String(property_name.clone()),
                )
                .cloned()
                .map(|value| (Expression::String(property_name), value))
            })
            .collect::<Vec<_>>();
        entries.extend(
            object_binding
                .symbol_properties
                .iter()
                .map(|(property, value)| (property.clone(), value.clone())),
        );
        entries
    }

    fn runtime_shadow_property_is_private(property: &Expression) -> bool {
        matches!(
            property,
            Expression::String(property_name)
                if property_name.starts_with("__ayy$private$")
                    || property_name.starts_with("__ayy$private_brand$")
        )
    }

    fn runtime_shadow_owner_resolves_to_proxy(&self, owner_name: &str) -> bool {
        let expression = if owner_name == "this" {
            Expression::This
        } else {
            Expression::Identifier(owner_name.to_string())
        };
        self.resolve_proxy_binding_from_expression(&expression)
            .is_some()
    }

    fn runtime_shadow_owner_has_proxy_target_only_private_metadata(
        &self,
        owner_name: &str,
    ) -> bool {
        self.runtime_shadow_owner_resolves_to_proxy(owner_name)
    }

    fn filter_proxy_private_runtime_shadow_entries(
        &self,
        owner_name: &str,
        entries: &mut Vec<(Expression, Expression)>,
    ) {
        if self.runtime_shadow_owner_has_proxy_target_only_private_metadata(owner_name) {
            entries.retain(|(property, _)| !Self::runtime_shadow_property_is_private(property));
        }
    }

    fn filter_proxy_private_object_binding_entries(
        &self,
        owner_name: &str,
        object_binding: &mut ObjectValueBinding,
    ) {
        if !self.runtime_shadow_owner_has_proxy_target_only_private_metadata(owner_name) {
            return;
        }
        object_binding
            .string_properties
            .retain(|(property_name, _)| {
                !property_name.starts_with("__ayy$private$")
                    && !property_name.starts_with("__ayy$private_brand$")
            });
        object_binding
            .non_enumerable_string_properties
            .retain(|property_name| {
                !property_name.starts_with("__ayy$private$")
                    && !property_name.starts_with("__ayy$private_brand$")
            });
    }

    fn private_runtime_shadow_entries_for_owner(
        &self,
        source_owner: &str,
    ) -> Vec<(Expression, Expression)> {
        if self.should_suppress_private_runtime_shadow_fallbacks(source_owner) {
            return Vec::new();
        }
        if self.runtime_shadow_owner_has_proxy_target_only_private_metadata(source_owner) {
            return Vec::new();
        }
        let object_binding = if source_owner == "this" {
            self.resolve_home_object_this_binding()
                .or_else(|| self.resolve_object_binding_from_expression(&Expression::This))
        } else {
            self.resolve_object_binding_from_expression(&Expression::Identifier(
                source_owner.to_string(),
            ))
        };
        let Some(object_binding) = object_binding else {
            return Vec::new();
        };

        self.object_runtime_shadow_entries_from_binding(&object_binding)
            .into_iter()
            .filter(|(property, _)| {
                matches!(property, Expression::String(property_name) if property_name.starts_with("__ayy$private$"))
            })
            .collect()
    }

    fn private_runtime_shadow_marker_fallback(
        &self,
        source_owner: &str,
        property: &Expression,
        fallback_value: Expression,
    ) -> Expression {
        if !matches!(property, Expression::String(property_name) if property_name.starts_with("__ayy$private$"))
            || !matches!(fallback_value, Expression::Undefined)
        {
            return fallback_value;
        }

        let source_expression = if source_owner == "this" {
            Expression::This
        } else {
            Expression::Identifier(source_owner.to_string())
        };
        self.resolve_member_getter_binding(&source_expression, property)
            .or_else(|| self.resolve_member_setter_binding(&source_expression, property))
            .or_else(|| self.resolve_member_function_binding(&source_expression, property))
            .map(|binding| match binding {
                LocalFunctionBinding::User(function_name)
                | LocalFunctionBinding::Builtin(function_name) => {
                    Expression::Identifier(function_name)
                }
            })
            .unwrap_or(fallback_value)
    }

    fn runtime_object_property_shadow_copy_entries(
        &self,
        source_owner: &str,
    ) -> Vec<(Expression, Option<Expression>)> {
        let suppress_private_fallbacks =
            self.should_suppress_private_runtime_shadow_fallbacks(source_owner);
        let mut entries = self
            .object_runtime_shadow_properties(source_owner)
            .into_iter()
            .filter(|(property, _)| {
                !suppress_private_fallbacks || !Self::runtime_shadow_property_is_private(property)
            })
            .map(|(property, fallback_value)| {
                let fallback_value = self.private_runtime_shadow_marker_fallback(
                    source_owner,
                    &property,
                    fallback_value,
                );
                (property, Some(fallback_value))
            })
            .collect::<Vec<_>>();

        let mut known_private_properties = entries
            .iter()
            .filter_map(|(property, _)| match property {
                Expression::String(property_name) => Some(property_name.clone()),
                _ => None,
            })
            .collect::<HashSet<_>>();
        for (property, fallback_value) in
            self.private_runtime_shadow_entries_for_owner(source_owner)
        {
            let Expression::String(property_name) = &property else {
                continue;
            };
            if known_private_properties.insert(property_name.clone()) {
                let fallback_value = self.private_runtime_shadow_marker_fallback(
                    source_owner,
                    &property,
                    fallback_value,
                );
                entries.push((property, Some(fallback_value)));
            }
        }

        entries
    }

    fn append_target_private_runtime_shadow_copy_entries(
        &self,
        source_owner: &str,
        target_owner: &str,
        entries: &mut Vec<(Expression, Option<Expression>)>,
    ) {
        let mut known_suffixes = entries
            .iter()
            .map(|(property, _)| Self::runtime_object_property_shadow_key(property))
            .collect::<HashSet<_>>();
        let mut private_properties = BTreeSet::new();
        let target_prefix = format!("__ayy_object_property__{target_owner}__");
        for name in self
            .backend
            .global_semantics
            .global_names()
            .implicit_bindings
            .keys()
        {
            if let Some((suffix, property_name)) =
                name.strip_prefix(&target_prefix).and_then(|suffix| {
                    let property_name =
                        Self::runtime_object_property_name_from_shadow_suffix(suffix)?;
                    (property_name.starts_with("__ayy$private$")
                        || property_name.starts_with("__ayy$private_brand$"))
                    .then_some((suffix.to_string(), property_name))
                })
            {
                private_properties.insert((suffix, property_name));
            }

            if self.should_suppress_private_runtime_shadow_fallbacks(source_owner)
                && name.starts_with("__ayy_object_property__")
            {
                for (index, _) in name.match_indices("__str__") {
                    let suffix = &name[index + 2..];
                    let Some(property_name) =
                        Self::runtime_object_property_name_from_shadow_suffix(suffix)
                    else {
                        continue;
                    };
                    if property_name.starts_with("__ayy$private$")
                        || property_name.starts_with("__ayy$private_brand$")
                    {
                        private_properties.insert((suffix.to_string(), property_name));
                    }
                }
            }
        }

        for (suffix, property_name) in private_properties {
            if known_suffixes.insert(suffix) {
                entries.push((Expression::String(property_name), None));
            }
        }
    }

    fn should_suppress_private_runtime_shadow_fallbacks(&self, source_owner: &str) -> bool {
        source_owner == "this"
    }

    pub(in crate::backend::direct_wasm) fn resolve_static_symbol_property_shadow_entry(
        &self,
        object_binding: &ObjectValueBinding,
        property: &Expression,
    ) -> Option<(Expression, Expression)> {
        let canonical_property = self.canonical_object_property_expression(property);
        let requested_symbol = self
            .resolve_symbol_identity_expression(&canonical_property)
            .or_else(|| self.resolve_symbol_identity_expression(property));

        object_binding
            .symbol_properties
            .iter()
            .find_map(|(existing_key, fallback_value)| {
                let canonical_existing = self.canonical_object_property_expression(existing_key);
                if static_expression_matches(&canonical_existing, &canonical_property)
                    || static_expression_matches(existing_key, property)
                {
                    return Some((existing_key.clone(), fallback_value.clone()));
                }

                let requested_symbol = requested_symbol.as_ref()?;
                let existing_symbol = self
                    .resolve_symbol_identity_expression(&canonical_existing)
                    .or_else(|| self.resolve_symbol_identity_expression(existing_key))?;
                static_expression_matches(&existing_symbol, requested_symbol)
                    .then_some((existing_key.clone(), fallback_value.clone()))
            })
    }

    pub(in crate::backend::direct_wasm) fn emit_user_function_parameter_object_shadow_setup(
        &mut self,
        user_function: &UserFunction,
        argument_expressions: &[Expression],
    ) -> DirectResult<Vec<(String, String, Option<ObjectValueBinding>)>> {
        let parameter_bindings = self
            .backend
            .function_registry
            .parameter_bindings_for(&user_function.name);
        let mut writebacks = Vec::new();

        for (index, param_name) in user_function.params.iter().enumerate() {
            let Some(argument_expression) = argument_expressions.get(index) else {
                continue;
            };
            let argument_requires_current_object_binding = matches!(argument_expression, Expression::Object(entries) if entries.iter().any(|entry| matches!(entry, ObjectEntry::Spread(_))));
            let argument_reads_descriptor_member =
                self.expression_reads_local_descriptor_binding_member(argument_expression);
            let parameter_object_binding =
                if argument_reads_descriptor_member || argument_requires_current_object_binding {
                    None
                } else {
                    parameter_bindings
                        .object_bindings
                        .get(param_name)
                        .and_then(|binding| binding.as_ref())
                };

            let source_owner = match argument_expression {
                Expression::Identifier(name) => {
                    self.runtime_object_property_shadow_owner_name_for_identifier(name)
                }
                _ => None,
            };
            let argument_object_binding = if argument_reads_descriptor_member {
                None
            } else {
                self.resolve_object_binding_from_expression(argument_expression)
                    .map(|binding| {
                        self.object_binding_with_constructed_constructor_shadow(
                            binding,
                            argument_expression,
                        )
                    })
                    .or_else(|| self.function_argument_metadata_object_binding(argument_expression))
            };
            if std::env::var_os("AYY_TRACE_PRIVATE_MEMBER_LOOKUP").is_some() {
                eprintln!(
                    "private_param_shadow_setup fn={} param={} arg={argument_expression:?} descriptor_arg={} param_binding={} arg_binding={} source_owner={source_owner:?}",
                    user_function.name,
                    param_name,
                    argument_reads_descriptor_member,
                    parameter_object_binding.is_some(),
                    argument_object_binding.is_some(),
                );
            }
            if parameter_object_binding.is_none()
                && argument_object_binding.is_none()
                && source_owner.is_none()
            {
                continue;
            }
            if source_owner.as_deref() == Some(param_name.as_str()) {
                continue;
            }
            self.clear_runtime_object_property_shadow_prefix(param_name);
            if let Some(source_owner) = source_owner.as_ref() {
                let source_owner_has_bindings =
                    self.runtime_object_property_shadow_owner_has_bindings(source_owner);
                self.emit_runtime_object_property_shadow_copy(source_owner, param_name)?;
                if !source_owner_has_bindings
                    && let Some(argument_object_binding) = argument_object_binding
                        .as_ref()
                        .or(parameter_object_binding)
                {
                    self.emit_runtime_object_property_shadow_seed_from_binding(
                        param_name,
                        argument_object_binding,
                    )?;
                    self.sync_runtime_object_property_shadow_static_metadata_from_binding(
                        param_name,
                        argument_object_binding,
                    );
                }
                writebacks.push((
                    param_name.clone(),
                    source_owner.clone(),
                    argument_object_binding.clone(),
                ));
                continue;
            }

            if let Some(argument_object_binding) = argument_object_binding
                .as_ref()
                .or(parameter_object_binding)
            {
                self.emit_runtime_object_property_shadow_seed_from_binding(
                    param_name,
                    argument_object_binding,
                )?;
                self.sync_runtime_object_property_shadow_static_metadata_from_binding(
                    param_name,
                    argument_object_binding,
                );
            }
        }

        for index in &user_function.extra_argument_indices {
            let Some(argument_expression) = argument_expressions.get(*index as usize) else {
                continue;
            };
            let owner_name = Self::user_function_arguments_slot_object_shadow_owner_name(
                &user_function.name,
                *index,
            );
            let argument_reads_descriptor_member =
                self.expression_reads_local_descriptor_binding_member(argument_expression);
            let argument_requires_current_object_binding = matches!(argument_expression, Expression::Object(entries) if entries.iter().any(|entry| matches!(entry, ObjectEntry::Spread(_))));
            let source_owner = match argument_expression {
                Expression::Identifier(name) => {
                    self.runtime_object_property_shadow_owner_name_for_identifier(name)
                }
                Expression::This => {
                    self.runtime_object_property_shadow_owner_name_for_identifier("this")
                }
                _ => None,
            };
            let argument_object_binding = if argument_reads_descriptor_member {
                None
            } else {
                self.resolve_object_binding_from_expression(argument_expression)
                    .map(|binding| {
                        self.object_binding_with_constructed_constructor_shadow(
                            binding,
                            argument_expression,
                        )
                    })
                    .or_else(|| {
                        (!argument_requires_current_object_binding)
                            .then(|| {
                                self.function_argument_metadata_object_binding(argument_expression)
                            })
                            .flatten()
                    })
            };

            if argument_object_binding.is_none() && source_owner.is_none() {
                continue;
            }
            if source_owner.as_deref() == Some(owner_name.as_str()) {
                continue;
            }
            self.clear_runtime_object_property_shadow_prefix(&owner_name);
            for property_name in
                Self::direct_arguments_slot_assignment_properties(user_function, *index)
            {
                self.predeclare_runtime_shadow_property(&owner_name, &property_name);
            }
            if let Some(source_owner) = source_owner.as_ref() {
                let source_owner_has_bindings =
                    self.runtime_object_property_shadow_owner_has_bindings(source_owner);
                self.emit_runtime_object_property_shadow_copy(source_owner, &owner_name)?;
                if !source_owner_has_bindings
                    && let Some(argument_object_binding) = argument_object_binding.as_ref()
                {
                    self.emit_runtime_object_property_shadow_seed_from_binding(
                        &owner_name,
                        argument_object_binding,
                    )?;
                    self.sync_runtime_object_property_shadow_static_metadata_from_binding(
                        &owner_name,
                        argument_object_binding,
                    );
                }
                writebacks.push((
                    owner_name,
                    source_owner.clone(),
                    argument_object_binding.clone(),
                ));
                continue;
            }

            if let Some(argument_object_binding) = argument_object_binding.as_ref() {
                self.emit_runtime_object_property_shadow_seed_from_binding(
                    &owner_name,
                    argument_object_binding,
                )?;
                self.sync_runtime_object_property_shadow_static_metadata_from_binding(
                    &owner_name,
                    argument_object_binding,
                );
            }
        }

        Ok(writebacks)
    }

    fn function_argument_metadata_object_binding(
        &self,
        argument_expression: &Expression,
    ) -> Option<ObjectValueBinding> {
        let function_binding = self.resolve_function_binding_from_expression(argument_expression);
        let mut object_binding = empty_object_value_binding();
        let (name_value, length_value) = match function_binding {
            Some(LocalFunctionBinding::User(function_name)) => {
                let user_function = self.user_function(&function_name)?;
                (
                    self.runtime_user_function_property_value(user_function, "name"),
                    self.runtime_user_function_property_value(user_function, "length"),
                )
            }
            Some(LocalFunctionBinding::Builtin(function_name)) => (
                Some(Expression::String(
                    builtin_function_display_name(&function_name).to_string(),
                )),
                builtin_function_length(&function_name)
                    .map(|length| Expression::Number(length as f64)),
            ),
            None => {
                let name_member = Expression::Member {
                    object: Box::new(argument_expression.clone()),
                    property: Box::new(Expression::String("name".to_string())),
                };
                let length_member = Expression::Member {
                    object: Box::new(argument_expression.clone()),
                    property: Box::new(Expression::String("length".to_string())),
                };
                let hinted_user_function =
                    if let Expression::Identifier(argument_name) = argument_expression {
                        let source_name =
                            scoped_binding_source_name(argument_name).unwrap_or(argument_name);
                        let matches = self
                            .user_functions()
                            .into_iter()
                            .filter(|function| {
                                self.resolve_user_function_display_name(&function.name)
                                    .as_deref()
                                    == Some(source_name)
                            })
                            .collect::<Vec<_>>();
                        match matches.as_slice() {
                            [function] => Some(function.clone()),
                            _ => None,
                        }
                    } else {
                        None
                    };
                (
                    self.resolve_static_string_value(&name_member)
                        .map(Expression::String)
                        .or_else(|| {
                            hinted_user_function.as_ref().and_then(|function| {
                                self.runtime_user_function_property_value(function, "name")
                            })
                        }),
                    self.resolve_static_number_value(&length_member)
                        .map(Expression::Number)
                        .or_else(|| {
                            hinted_user_function.as_ref().and_then(|function| {
                                self.runtime_user_function_property_value(function, "length")
                            })
                        }),
                )
            }
        };
        if let Some(name_value) = name_value {
            object_binding_define_property_descriptor(
                &mut object_binding,
                Expression::String("name".to_string()),
                PropertyDescriptorBinding {
                    value: Some(name_value),
                    configurable: true,
                    enumerable: false,
                    writable: Some(false),
                    getter: None,
                    setter: None,
                    has_get: false,
                    has_set: false,
                },
            );
        }
        if let Some(length_value) = length_value {
            object_binding_define_property_descriptor(
                &mut object_binding,
                Expression::String("length".to_string()),
                PropertyDescriptorBinding {
                    value: Some(length_value),
                    configurable: true,
                    enumerable: false,
                    writable: Some(false),
                    getter: None,
                    setter: None,
                    has_get: false,
                    has_set: false,
                },
            );
        }
        (!object_binding.string_properties.is_empty()
            || !object_binding.property_descriptors.is_empty())
        .then_some(object_binding)
    }

    pub(in crate::backend::direct_wasm) fn object_binding_with_constructed_constructor_shadow(
        &self,
        mut object_binding: ObjectValueBinding,
        argument_expression: &Expression,
    ) -> ObjectValueBinding {
        if object_binding
            .string_properties
            .iter()
            .any(|(property, _)| property == "constructor")
        {
            return object_binding;
        }
        let Some(constructor_binding) =
            self.constructed_object_constructor_binding_for_shadow_argument(argument_expression)
        else {
            return object_binding;
        };
        let constructor_expression = match constructor_binding {
            LocalFunctionBinding::User(function_name)
            | LocalFunctionBinding::Builtin(function_name) => Expression::Identifier(function_name),
        };
        object_binding
            .string_properties
            .push(("constructor".to_string(), constructor_expression));
        if !object_binding
            .non_enumerable_string_properties
            .iter()
            .any(|property| property == "constructor")
        {
            object_binding
                .non_enumerable_string_properties
                .push("constructor".to_string());
        }
        object_binding
    }

    pub(in crate::backend::direct_wasm) fn constructed_object_constructor_binding_for_shadow_argument(
        &self,
        argument_expression: &Expression,
    ) -> Option<LocalFunctionBinding> {
        self.resolve_constructed_object_constructor_binding(argument_expression)
            .or_else(|| {
                let Expression::Identifier(name) = argument_expression else {
                    return None;
                };
                let active_name = self
                    .resolve_current_local_binding(name)
                    .map(|(resolved_name, _)| resolved_name)
                    .unwrap_or_else(|| name.clone());
                let value = self
                    .state
                    .speculation
                    .static_semantics
                    .local_value_binding(&active_name)?;
                self.resolve_constructed_object_constructor_binding(value)
            })
    }

    pub(in crate::backend::direct_wasm) fn emit_user_function_parameter_object_shadow_writeback(
        &mut self,
        writebacks: &[(String, String, Option<ObjectValueBinding>)],
    ) -> DirectResult<()> {
        for (param_name, source_owner, _) in writebacks {
            self.emit_runtime_object_property_shadow_copy(param_name, source_owner)?;
        }
        Ok(())
    }

    fn clear_runtime_object_property_shadows_for_owner(
        &mut self,
        owner_name: &str,
        object_binding: &ObjectValueBinding,
    ) {
        for (property, _) in self.object_runtime_shadow_entries_from_binding(object_binding) {
            let binding =
                self.runtime_object_property_shadow_binding_by_property(owner_name, &property);
            let deleted_binding = self
                .runtime_object_property_shadow_deleted_binding_by_property(owner_name, &property);
            self.push_i32_const(JS_UNDEFINED_TAG);
            self.push_global_set(binding.value_index);
            self.push_i32_const(0);
            self.push_global_set(binding.present_index);
            self.push_i32_const(JS_UNDEFINED_TAG);
            self.push_global_set(deleted_binding.value_index);
            self.push_i32_const(0);
            self.push_global_set(deleted_binding.present_index);
        }
    }

    pub(in crate::backend::direct_wasm) fn clear_runtime_object_property_shadow_binding(
        &mut self,
        object: &Expression,
        property: &Expression,
    ) -> bool {
        let binding = self.resolve_runtime_object_property_shadow_binding(object, property);
        let Some(deleted_binding) =
            self.resolve_runtime_object_property_shadow_deleted_binding(object, property)
        else {
            return false;
        };
        if let Some(binding) = binding {
            self.push_i32_const(JS_UNDEFINED_TAG);
            self.push_global_set(binding.value_index);
            self.push_i32_const(0);
            self.push_global_set(binding.present_index);
        }
        self.push_i32_const(JS_UNDEFINED_TAG);
        self.push_global_set(deleted_binding.value_index);
        self.push_i32_const(0);
        self.push_global_set(deleted_binding.present_index);
        true
    }

    pub(in crate::backend::direct_wasm) fn clear_runtime_object_property_shadow_deleted_binding(
        &mut self,
        object: &Expression,
        property: &Expression,
    ) -> bool {
        let Some(binding) =
            self.resolve_runtime_object_property_shadow_deleted_binding(object, property)
        else {
            return false;
        };
        self.push_i32_const(JS_UNDEFINED_TAG);
        self.push_global_set(binding.value_index);
        self.push_i32_const(0);
        self.push_global_set(binding.present_index);
        true
    }

    pub(in crate::backend::direct_wasm) fn mark_runtime_object_property_shadow_deleted_binding(
        &mut self,
        object: &Expression,
        property: &Expression,
    ) -> bool {
        let canonical_property = self.canonical_object_property_expression(property);
        let shadow_binding_name =
            self.runtime_object_property_shadow_binding_name_for_expression(object, property);
        let deleted_shadow_name = self
            .runtime_object_property_shadow_owner_name_for_expression(object)
            .map(|owner_name| {
                Self::runtime_object_property_deleted_shadow_name(&owner_name, &canonical_property)
            });
        let binding = self.resolve_runtime_object_property_shadow_binding(object, property);
        let Some(deleted_binding) =
            self.resolve_runtime_object_property_shadow_deleted_binding(object, property)
        else {
            return false;
        };
        if let Some(binding) = binding {
            self.push_i32_const(JS_UNDEFINED_TAG);
            self.push_global_set(binding.value_index);
            self.push_i32_const(0);
            self.push_global_set(binding.present_index);
        }
        self.push_i32_const(JS_UNDEFINED_TAG);
        self.push_global_set(deleted_binding.value_index);
        self.push_i32_const(1);
        self.push_global_set(deleted_binding.present_index);
        if let Expression::String(property_name) = &canonical_property
            && let Some(function_name) = self.current_function_name()
            && !self.assigned_user_function_capture_originates_in_enclosing_local(
                function_name,
                property_name,
            )
            && let Some(hidden_name) = self.resolve_user_function_capture_hidden_name(property_name)
        {
            let deleted_marker_name =
                Self::capture_slot_member_source_deleted_binding_name(&hidden_name);
            let deleted_marker = self.ensure_implicit_global_binding(&deleted_marker_name);
            self.push_i32_const(JS_UNDEFINED_TAG);
            self.push_global_set(deleted_marker.value_index);
            self.push_i32_const(1);
            self.push_global_set(deleted_marker.present_index);
        }
        if let Some(shadow_binding_name) = shadow_binding_name {
            self.update_static_global_assignment_metadata(
                &shadow_binding_name,
                &Expression::Undefined,
            );
        }
        if let Some(deleted_shadow_name) = deleted_shadow_name {
            self.update_static_global_assignment_metadata(
                &deleted_shadow_name,
                &Expression::Undefined,
            );
        }
        if let Expression::Identifier(name) = object {
            let source_name = self
                .resolve_user_function_capture_hidden_name(name)
                .or_else(|| self.resolve_eval_local_function_hidden_name(name))
                .and_then(|hidden_name| self.resolve_capture_slot_source_binding_name(&hidden_name))
                .filter(|source_name| {
                    Self::capture_slot_member_source_key_parts(source_name).is_none()
                });
            if let Some(source_name) = source_name {
                let source_object = Expression::Identifier(source_name.clone());
                if let Some(source_binding) = self.resolve_runtime_object_property_shadow_binding(
                    &source_object,
                    &canonical_property,
                ) {
                    self.push_i32_const(JS_UNDEFINED_TAG);
                    self.push_global_set(source_binding.value_index);
                    self.push_i32_const(0);
                    self.push_global_set(source_binding.present_index);
                }
                if let Some(source_deleted_binding) = self
                    .resolve_runtime_object_property_shadow_deleted_binding(
                        &source_object,
                        &canonical_property,
                    )
                {
                    self.push_i32_const(JS_UNDEFINED_TAG);
                    self.push_global_set(source_deleted_binding.value_index);
                    self.push_i32_const(1);
                    self.push_global_set(source_deleted_binding.present_index);
                }
                if let Some(source_shadow_name) = self
                    .runtime_object_property_shadow_binding_name_for_expression(
                        &source_object,
                        &canonical_property,
                    )
                {
                    self.update_static_global_assignment_metadata(
                        &source_shadow_name,
                        &Expression::Undefined,
                    );
                }
                if let Some(source_owner_name) =
                    self.runtime_object_property_shadow_owner_name_for_expression(&source_object)
                {
                    let source_deleted_name = Self::runtime_object_property_deleted_shadow_name(
                        &source_owner_name,
                        &canonical_property,
                    );
                    self.update_static_global_assignment_metadata(
                        &source_deleted_name,
                        &Expression::Undefined,
                    );
                }
            }
        }
        true
    }

    pub(in crate::backend::direct_wasm) fn runtime_object_property_shadow_binding_name(
        owner_name: &str,
        property_name: &str,
    ) -> String {
        format!(
            "__ayy_object_property__{owner_name}__str__{}",
            Self::runtime_object_property_shadow_fragment(property_name)
        )
    }

    pub(in crate::backend::direct_wasm) fn runtime_object_property_shadow_owner_name_for_identifier(
        &self,
        name: &str,
    ) -> Option<String> {
        let identifier_expression = Expression::Identifier(name.to_string());
        let trace_runtime_shadows = std::env::var_os("AYY_TRACE_RUNTIME_SHADOWS").is_some();
        if name == "this" {
            if self
                .current_user_function()
                .is_some_and(|function| function.lexical_this)
                && let Some(hidden_name) = self.resolve_user_function_capture_hidden_name("this")
            {
                return Some(hidden_name);
            }
            return Some("this".to_string());
        }
        if let Some(source_name) = scoped_binding_source_name(name)
            && (self.runtime_object_property_shadow_owner_has_bindings(source_name)
                || self.backend.global_has_binding(source_name)
                || self.backend.global_has_lexical_binding(source_name)
                || self.backend.global_has_implicit_binding(source_name)
                || self.global_value_binding(source_name).is_some()
                || self.contains_user_function(source_name))
        {
            return Some(source_name.to_string());
        }
        if name.starts_with("__ayy_for_in_target_")
            && let Some(Expression::Identifier(source_name)) = self
                .state
                .speculation
                .static_semantics
                .local_value_binding(name)
            && self
                .runtime_object_property_shadow_owner_name_for_identifier(source_name)
                .is_some()
        {
            return Some(source_name.clone());
        }
        if let Some(hidden_name) = self.resolve_user_function_capture_hidden_name(name) {
            return Some(hidden_name);
        }
        if let Some(hidden_name) = self.resolve_eval_local_function_hidden_name(name) {
            return Some(hidden_name);
        }
        if self.hidden_implicit_global_binding(name).is_some()
            && self.runtime_object_property_shadow_owner_has_bindings(name)
        {
            return Some(name.to_string());
        }
        if self.resolve_current_local_binding(name).is_some() {
            return Some(name.to_string());
        }
        if let Some(Expression::Identifier(source_name)) = self
            .state
            .speculation
            .static_semantics
            .local_value_binding(name)
            .or_else(|| self.global_value_binding(name))
            && source_name != name
            && let Some(source_owner) =
                self.runtime_object_property_shadow_owner_name_for_identifier(source_name)
        {
            return Some(source_owner);
        }
        if (self.backend.global_has_binding(name)
            || self.backend.global_has_lexical_binding(name)
            || self.backend.global_has_implicit_binding(name))
            && self.runtime_object_property_shadow_owner_has_bindings(name)
        {
            return Some(name.to_string());
        }
        if let Some((resolved_name, _)) = self.resolve_current_local_binding(name)
            && self
                .state
                .speculation
                .static_semantics
                .has_local_object_binding(&resolved_name)
        {
            return Some(resolved_name);
        }
        if self
            .state
            .speculation
            .static_semantics
            .has_local_object_binding(name)
        {
            return Some(name.to_string());
        }
        if self.current_function_name().is_some_and(|function_name| {
            self.backend
                .function_registry
                .parameter_bindings_for(function_name)
                .object_bindings
                .contains_key(name)
                || self
                    .user_function(function_name)
                    .is_some_and(|function| function.params.iter().any(|param| param == name))
        }) {
            return Some(name.to_string());
        }
        let resolved_owner = ((self.backend.global_has_binding(name)
            || self.backend.global_has_lexical_binding(name)
            || self.backend.global_has_implicit_binding(name))
            && self.backend.global_object_binding(name).is_some())
        .then(|| name.to_string())
        .or_else(|| {
            (self.backend.global_has_implicit_binding(name)
                && self.backend.global_object_binding(name).is_some())
            .then(|| name.to_string())
        })
        .or_else(|| {
            self.resolve_bound_alias_expression(&identifier_expression)
                .filter(|resolved| !static_expression_matches(resolved, &identifier_expression))
                .or_else(|| {
                    self.state
                        .speculation
                        .static_semantics
                        .local_value_binding(name)
                        .cloned()
                })
                .or_else(|| self.global_value_binding(name).cloned())
                .filter(|resolved| !static_expression_matches(resolved, &identifier_expression))
                .and_then(|resolved| {
                    self.runtime_object_property_shadow_owner_name_for_expression(&resolved)
                        .or_else(|| {
                            (matches!(
                                self.infer_value_kind(&resolved),
                                Some(StaticValueKind::Object | StaticValueKind::Function)
                            ) || self
                                .resolve_object_binding_from_expression(&resolved)
                                .is_some())
                            .then(|| name.to_string())
                        })
                })
        });
        if trace_runtime_shadows {
            eprintln!(
                "runtime_shadow_owner identifier={name} fn={:?} local_object={} local_value={:?} global_object={} global_value={:?} alias={:?} resolved_owner={resolved_owner:?}",
                self.current_function_name(),
                self.state
                    .speculation
                    .static_semantics
                    .has_local_object_binding(name),
                self.state
                    .speculation
                    .static_semantics
                    .local_value_binding(name)
                    .cloned(),
                self.backend.global_object_binding(name).is_some(),
                self.global_value_binding(name).cloned(),
                self.resolve_bound_alias_expression(&identifier_expression)
                    .filter(|resolved| !static_expression_matches(
                        resolved,
                        &identifier_expression
                    )),
            );
        }
        resolved_owner
    }

    fn runtime_object_property_shadow_owner_name_for_expression(
        &self,
        expression: &Expression,
    ) -> Option<String> {
        match expression {
            Expression::Identifier(name) => {
                self.runtime_object_property_shadow_owner_name_for_identifier(name)
            }
            Expression::This => {
                self.runtime_object_property_shadow_owner_name_for_identifier("this")
            }
            Expression::Member { object, property }
                if self.is_direct_arguments_object(object)
                    && argument_index_from_expression(
                        &self.canonical_object_property_expression(property),
                    )
                    .is_some() =>
            {
                let index = argument_index_from_expression(
                    &self.canonical_object_property_expression(property),
                )?;
                let function_name = self.current_function_name()?;
                Some(Self::user_function_arguments_slot_object_shadow_owner_name(
                    function_name,
                    index,
                ))
            }
            _ => None,
        }
    }

    pub(in crate::backend::direct_wasm) fn runtime_object_property_shadow_binding_name_for_expression(
        &self,
        object: &Expression,
        property: &Expression,
    ) -> Option<String> {
        let owner_name = self.runtime_object_property_shadow_owner_name_for_expression(object)?;
        let property_name = static_property_name_from_expression(
            &self.canonical_object_property_expression(property),
        )?;
        Some(Self::runtime_object_property_shadow_binding_name(
            &owner_name,
            &property_name,
        ))
    }

    pub(in crate::backend::direct_wasm) fn runtime_object_property_shadow_binding_has_static_metadata(
        &self,
        shadow_binding_name: &str,
    ) -> bool {
        self.global_value_binding(shadow_binding_name).is_some()
            || self
                .backend
                .shared_global_semantics
                .values
                .value_bindings
                .contains_key(shadow_binding_name)
            || self.global_binding_kind(shadow_binding_name).is_some()
    }

    pub(in crate::backend::direct_wasm) fn runtime_object_property_shadow_binding_should_defer_static_resolution(
        &self,
        shadow_binding_name: &str,
    ) -> bool {
        self.global_has_implicit_binding(shadow_binding_name)
            && self.runtime_object_property_shadow_binding_has_static_metadata(shadow_binding_name)
    }

    pub(in crate::backend::direct_wasm) fn runtime_object_property_shadow_deletion_may_hide_static_property(
        &self,
        object: &Expression,
        property: &Expression,
    ) -> bool {
        let property = self.canonical_object_property_expression(property);
        let Some(owner_name) =
            self.runtime_object_property_shadow_owner_name_for_expression(object)
        else {
            return false;
        };
        let deleted_shadow_name =
            Self::runtime_object_property_deleted_shadow_name(&owner_name, &property);
        if !self.global_has_implicit_binding(&deleted_shadow_name) {
            return false;
        }
        let object_binding = self
            .resolve_object_binding_from_expression(object)
            .or_else(|| match object {
                Expression::Identifier(name) => {
                    self.resolve_identifier_object_binding_fallback(name)
                }
                _ => None,
            });
        !object_binding
            .as_ref()
            .is_some_and(|binding| object_binding_has_property(binding, &property))
    }

    pub(in crate::backend::direct_wasm) fn runtime_object_property_shadow_deletion_may_affect_property(
        &self,
        object: &Expression,
        property: &Expression,
    ) -> bool {
        let property = self.canonical_object_property_expression(property);
        let Some(owner_name) =
            self.runtime_object_property_shadow_owner_name_for_expression(object)
        else {
            return false;
        };
        let deleted_shadow_name =
            Self::runtime_object_property_deleted_shadow_name(&owner_name, &property);
        self.global_has_implicit_binding(&deleted_shadow_name)
    }

    pub(in crate::backend::direct_wasm) fn runtime_object_property_shadow_deletion_is_statically_present(
        &self,
        object: &Expression,
        property: &Expression,
    ) -> bool {
        let property = self.canonical_object_property_expression(property);
        let Some(owner_name) =
            self.runtime_object_property_shadow_owner_name_for_expression(object)
        else {
            return false;
        };
        let deleted_shadow_name =
            Self::runtime_object_property_deleted_shadow_name(&owner_name, &property);
        let deleted_value_is_static = match self.global_value_binding(&deleted_shadow_name) {
            Some(Expression::Undefined) => true,
            Some(Expression::Number(number)) => *number == JS_UNDEFINED_TAG as f64,
            _ => false,
        };
        deleted_value_is_static
            || matches!(
                self.global_binding_kind(&deleted_shadow_name),
                Some(StaticValueKind::Undefined)
            )
    }

    pub(in crate::backend::direct_wasm) fn resolve_runtime_object_property_shadow_binding(
        &mut self,
        object: &Expression,
        property: &Expression,
    ) -> Option<ImplicitGlobalBinding> {
        let property = self.canonical_object_property_expression(property);
        let owner_name = self.runtime_object_property_shadow_owner_name_for_expression(object)?;
        if std::env::var_os("AYY_TRACE_RUNTIME_SHADOWS").is_some() {
            eprintln!(
                "runtime_shadow_binding object={object:?} property={property:?} owner={owner_name}"
            );
        }
        if let Expression::String(property_name) = property {
            return Some(self.ensure_implicit_global_binding(
                &Self::runtime_object_property_shadow_binding_name(&owner_name, &property_name),
            ));
        }
        let object_binding = self
            .resolve_object_binding_from_expression(object)
            .or_else(|| match object {
                Expression::Identifier(name) => {
                    self.resolve_identifier_object_binding_fallback(name)
                }
                _ => None,
            })?;
        object_binding_has_property(&object_binding, &property).then(|| {
            self.runtime_object_property_shadow_binding_by_property(&owner_name, &property)
        })
    }

    pub(in crate::backend::direct_wasm) fn resolve_runtime_object_property_shadow_deleted_binding(
        &mut self,
        object: &Expression,
        property: &Expression,
    ) -> Option<ImplicitGlobalBinding> {
        let property = self.canonical_object_property_expression(property);
        let owner_name = self.runtime_object_property_shadow_owner_name_for_expression(object)?;
        if let Expression::String(property_name) = &property {
            return Some(self.ensure_implicit_global_binding(
                &Self::runtime_object_property_deleted_shadow_name(
                    &owner_name,
                    &Expression::String(property_name.clone()),
                ),
            ));
        }
        let object_binding = self
            .resolve_object_binding_from_expression(object)
            .or_else(|| match object {
                Expression::Identifier(name) => {
                    self.resolve_identifier_object_binding_fallback(name)
                }
                _ => None,
            })?;
        object_binding_has_property(&object_binding, &property).then(|| {
            self.runtime_object_property_shadow_deleted_binding_by_property(&owner_name, &property)
        })
    }

    pub(in crate::backend::direct_wasm) fn runtime_object_property_shadow_binding_by_property(
        &mut self,
        owner_name: &str,
        property: &Expression,
    ) -> ImplicitGlobalBinding {
        self.ensure_implicit_global_binding(&format!(
            "__ayy_object_property__{owner_name}__{}",
            Self::runtime_object_property_shadow_key(property)
        ))
    }

    pub(in crate::backend::direct_wasm) fn runtime_object_property_shadow_deleted_binding_by_property(
        &mut self,
        owner_name: &str,
        property: &Expression,
    ) -> ImplicitGlobalBinding {
        self.ensure_implicit_global_binding(&Self::runtime_object_property_deleted_shadow_name(
            owner_name, property,
        ))
    }

    pub(in crate::backend::direct_wasm) fn runtime_object_property_shadow_binding_by_names(
        &mut self,
        owner_name: &str,
        property_name: &str,
    ) -> ImplicitGlobalBinding {
        self.ensure_implicit_global_binding(&Self::runtime_object_property_shadow_binding_name(
            owner_name,
            property_name,
        ))
    }

    pub(in crate::backend::direct_wasm) fn object_runtime_shadow_properties(
        &self,
        owner_name: &str,
    ) -> Vec<(Expression, Expression)> {
        let object_expression = Expression::Identifier(owner_name.to_string());
        let Some(object_binding) = self.resolve_object_binding_from_expression(&object_expression)
        else {
            return Vec::new();
        };
        let mut entries = self.object_runtime_shadow_entries_from_binding(&object_binding);
        self.filter_proxy_private_runtime_shadow_entries(owner_name, &mut entries);
        entries
    }

    pub(in crate::backend::direct_wasm) fn resolve_runtime_shadow_object_binding(
        &self,
        owner_name: &str,
    ) -> Option<ObjectValueBinding> {
        let prefix = format!("__ayy_object_property__{owner_name}__");
        let deleted_prefix = format!("__ayy_object_property_deleted__{owner_name}__");
        let static_object_binding = self.resolve_object_binding_from_expression(
            &Expression::Identifier(owner_name.to_string()),
        );
        let had_static_object_binding = static_object_binding.is_some();
        let mut object_binding = static_object_binding.unwrap_or_else(empty_object_value_binding);
        self.filter_proxy_private_object_binding_entries(owner_name, &mut object_binding);
        let mut found_shadow_entry = false;
        for name in self
            .backend
            .global_semantics
            .global_names()
            .implicit_bindings
            .keys()
            .filter(|name| name.starts_with(&prefix))
        {
            let Some(property_name) =
                Self::runtime_object_property_name_from_shadow_suffix(&name[prefix.len()..])
            else {
                continue;
            };
            let Some(value) = self.global_value_binding(name).cloned().or_else(|| {
                self.backend
                    .shared_global_semantics
                    .values
                    .value_bindings
                    .get(name)
                    .cloned()
            }) else {
                continue;
            };
            let property = Expression::String(property_name);
            if let Some(descriptor) = self.backend.global_property_descriptor(name).or_else(|| {
                self.backend
                    .shared_global_semantics
                    .values
                    .property_descriptor(name)
            }) {
                object_binding_define_property_descriptor(
                    &mut object_binding,
                    property,
                    PropertyDescriptorBinding {
                        value: Some(descriptor.value.clone()),
                        configurable: descriptor.configurable,
                        enumerable: descriptor.enumerable,
                        writable: descriptor.writable,
                        getter: None,
                        setter: None,
                        has_get: false,
                        has_set: false,
                    },
                );
            } else {
                object_binding_set_property(&mut object_binding, property, value);
            }
            found_shadow_entry = true;
        }
        let deleted_shadow_names = self
            .backend
            .global_semantics
            .global_names()
            .implicit_bindings
            .keys()
            .chain(
                self.backend
                    .shared_global_semantics
                    .global_names()
                    .implicit_bindings
                    .keys(),
            )
            .filter(|name| name.starts_with(&deleted_prefix))
            .cloned()
            .collect::<Vec<_>>();
        for name in deleted_shadow_names {
            let Some(property_name) = Self::runtime_object_property_name_from_shadow_suffix(
                &name[deleted_prefix.len()..],
            ) else {
                continue;
            };
            let deleted_is_static = self.global_value_binding(&name).is_some()
                || self
                    .backend
                    .shared_global_semantics
                    .values
                    .value_bindings
                    .contains_key(&name);
            if deleted_is_static {
                object_binding_remove_property(
                    &mut object_binding,
                    &Expression::String(property_name),
                );
            }
        }

        (found_shadow_entry
            || had_static_object_binding
            || !object_binding.string_properties.is_empty()
            || !object_binding.symbol_properties.is_empty())
        .then_some(object_binding)
    }

    pub(in crate::backend::direct_wasm) fn emit_runtime_shadow_fallback_value(
        &mut self,
        fallback_value: &Expression,
    ) -> DirectResult<()> {
        if std::env::var_os("AYY_TRACE_RUNTIME_SHADOWS").is_some() {
            eprintln!("runtime_shadow_fallback value={fallback_value:?}");
        }
        let Some(_fallback_guard) = RuntimeShadowFallbackGuard::enter(fallback_value) else {
            self.push_i32_const(JS_UNDEFINED_TAG);
            return Ok(());
        };
        if !self.runtime_shadow_fallback_references_readable_bindings(fallback_value) {
            self.push_i32_const(self.runtime_shadow_fallback_type_tag(fallback_value));
            return Ok(());
        }
        if !inline_summary_side_effect_free_expression(fallback_value) {
            self.push_i32_const(self.runtime_shadow_fallback_type_tag(fallback_value));
            return Ok(());
        }
        if let Expression::Identifier(name) = fallback_value
            && name.starts_with("__ayy_closure_slot_")
            && let Some(private_brand_offset) = name.find("__ayy_class_brand_")
        {
            let private_brand_name = &name[private_brand_offset..];
            let emit_private_brand = |compiler: &mut Self| -> DirectResult<()> {
                if !compiler
                    .emit_private_brand_runtime_value_for_binding_name(private_brand_name)?
                {
                    compiler
                        .emit_private_brand_direct_or_synthetic_runtime_value_for_binding_name(
                            private_brand_name,
                        )?;
                }
                Ok(())
            };
            if let Some(hidden_binding) = self.hidden_implicit_global_binding(name) {
                self.push_global_get(hidden_binding.present_index);
                self.state.emission.output.instructions.push(0x04);
                self.state.emission.output.instructions.push(I32_TYPE);
                self.push_control_frame();
                self.push_global_get(hidden_binding.value_index);
                self.state.emission.output.instructions.push(0x05);
                emit_private_brand(self)?;
                self.state.emission.output.instructions.push(0x0b);
                self.pop_control_frame();
            } else {
                emit_private_brand(self)?;
            }
            return Ok(());
        }
        if let Some(function_binding) =
            self.resolve_function_binding_from_expression(fallback_value)
        {
            match function_binding {
                LocalFunctionBinding::User(function_name) => {
                    if let Some(user_function) = self.user_function(&function_name) {
                        self.push_i32_const(user_function_runtime_value(user_function));
                    } else {
                        self.push_i32_const(JS_UNDEFINED_TAG);
                    }
                }
                LocalFunctionBinding::Builtin(function_name) => {
                    self.push_i32_const(
                        builtin_function_runtime_value(&function_name)
                            .unwrap_or(JS_TYPEOF_FUNCTION_TAG),
                    );
                }
            }
            return Ok(());
        }

        if let Expression::Identifier(name) = fallback_value
            && (self.resolve_current_local_binding(name).is_some()
                || self.resolve_global_binding_index(name).is_some()
                || self
                    .resolve_user_function_capture_hidden_name(name)
                    .is_some()
                || self.resolve_eval_local_function_hidden_name(name).is_some()
                || self.hidden_implicit_global_binding(name).is_some())
        {
            self.emit_numeric_expression(fallback_value)?;
            return Ok(());
        }

        if self
            .resolve_array_binding_from_expression(fallback_value)
            .is_some()
            || self
                .resolve_object_binding_from_expression(fallback_value)
                .is_some()
            || self
                .resolve_arguments_binding_from_expression(fallback_value)
                .is_some()
            || self
                .resolve_proxy_binding_from_expression(fallback_value)
                .is_some()
        {
            self.push_i32_const(JS_TYPEOF_OBJECT_TAG);
            return Ok(());
        }

        self.emit_numeric_expression(fallback_value)
    }

    fn runtime_shadow_fallback_type_tag(&self, fallback_value: &Expression) -> i32 {
        match self.infer_value_kind(fallback_value) {
            Some(StaticValueKind::Null) => JS_NULL_TAG,
            Some(StaticValueKind::Undefined) => JS_UNDEFINED_TAG,
            Some(kind) => kind.as_typeof_tag().unwrap_or(JS_UNDEFINED_TAG),
            None => JS_UNDEFINED_TAG,
        }
    }

    fn runtime_shadow_fallback_identifier_is_readable(&self, name: &str) -> bool {
        self.parameter_scope_arguments_local_for(name).is_some()
            || (self.is_current_arguments_binding_name(name) && self.has_arguments_object())
            || self.resolve_current_local_binding(name).is_some()
            || self
                .state
                .speculation
                .static_semantics
                .has_local_function_binding(name)
            || self.resolve_global_binding_index(name).is_some()
            || self.backend.implicit_global_binding(name).is_some()
            || self
                .resolve_user_function_capture_hidden_name(name)
                .is_some()
            || self.resolve_eval_local_function_hidden_name(name).is_some()
            || parse_test262_realm_identifier(name).is_some()
            || parse_test262_realm_global_identifier(name).is_some()
            || (name == "NaN" && self.is_unshadowed_builtin_identifier(name))
            || (name == "Infinity" && self.is_unshadowed_builtin_identifier(name))
            || name == "undefined"
            || builtin_function_runtime_value(name).is_some()
            || (is_internal_user_function_identifier(name)
                && self.user_function_runtime_value(name).is_some())
            || self.lookup_identifier_kind(name).is_some()
            || name.find("__ayy_class_brand_").is_some()
            || (name.starts_with("__ayy_class_super_")
                && self
                    .resolve_static_class_init_local_alias_expression(name)
                    .filter(|resolved| {
                        !static_expression_matches(
                            resolved,
                            &Expression::Identifier(name.to_string()),
                        )
                    })
                    .is_some())
            || name == "__ayy_null_super_constructor"
    }

    fn runtime_shadow_fallback_references_readable_bindings(
        &self,
        fallback_value: &Expression,
    ) -> bool {
        let mut referenced_names = HashSet::new();
        collect_referenced_binding_names_from_expression(fallback_value, &mut referenced_names);
        referenced_names
            .iter()
            .all(|name| self.runtime_shadow_fallback_identifier_is_readable(name))
    }

    pub(in crate::backend::direct_wasm) fn sync_runtime_object_property_shadow_static_metadata_from_binding(
        &mut self,
        target_owner: &str,
        object_binding: &ObjectValueBinding,
    ) {
        for (property, fallback_value) in
            self.object_runtime_shadow_entries_from_binding(object_binding)
        {
            let fallback_value =
                self.rewrite_static_new_this_expression_for_owner(&fallback_value, target_owner);
            let shadow_binding_name = format!(
                "__ayy_object_property__{target_owner}__{}",
                Self::runtime_object_property_shadow_key(&property)
            );
            self.ensure_implicit_global_binding(&shadow_binding_name);
            self.ensure_implicit_global_binding(
                &Self::runtime_object_property_deleted_shadow_name(target_owner, &property),
            );
            let materialized_value =
                self.reference_preserving_static_value_expression(&fallback_value);
            if std::env::var_os("AYY_TRACE_RUNTIME_SHADOWS").is_some() {
                eprintln!(
                    "runtime_shadow_static_sync target={target_owner} property={property:?} fallback={fallback_value:?} materialized={materialized_value:?}"
                );
            }
            self.update_static_global_assignment_metadata(
                &shadow_binding_name,
                &materialized_value,
            );
            if let Some(descriptor) = object_binding_lookup_descriptor(object_binding, &property) {
                let descriptor_state = GlobalPropertyDescriptorState {
                    value: descriptor
                        .value
                        .as_ref()
                        .map(|value| self.materialize_static_expression(value))
                        .unwrap_or(Expression::Undefined),
                    writable: descriptor.writable,
                    enumerable: descriptor.enumerable,
                    configurable: descriptor.configurable,
                };
                self.backend.upsert_global_property_descriptor(
                    shadow_binding_name.clone(),
                    descriptor_state.clone(),
                );
                self.backend
                    .shared_global_semantics
                    .values
                    .property_descriptors
                    .insert(shadow_binding_name.clone(), descriptor_state);
            }
            self.backend
                .shared_global_semantics
                .values
                .set_value_binding(shadow_binding_name.clone(), materialized_value.clone());
            if let Some(kind) = self.infer_value_kind(&materialized_value) {
                self.backend
                    .shared_global_semantics
                    .set_global_binding_kind(&shadow_binding_name, kind);
            }
        }
    }

    pub(in crate::backend::direct_wasm) fn emit_runtime_object_property_shadow_seed_from_binding(
        &mut self,
        target_owner: &str,
        object_binding: &ObjectValueBinding,
    ) -> DirectResult<()> {
        let target_expression = Expression::Identifier(target_owner.to_string());
        for (property, fallback_value) in
            self.object_runtime_shadow_entries_from_binding(object_binding)
        {
            let fallback_value =
                self.rewrite_static_new_this_expression_for_owner(&fallback_value, target_owner);
            let target_binding =
                self.runtime_object_property_shadow_binding_by_property(target_owner, &property);
            let target_deleted = self.runtime_object_property_shadow_deleted_binding_by_property(
                target_owner,
                &property,
            );
            self.push_i32_const(JS_UNDEFINED_TAG);
            self.push_global_set(target_deleted.value_index);
            self.push_i32_const(0);
            self.push_global_set(target_deleted.present_index);
            if !self.emit_private_brand_marker_runtime_value(
                &target_expression,
                &property,
                &fallback_value,
            )? {
                self.emit_runtime_shadow_fallback_value(&fallback_value)?;
            }
            self.push_global_set(target_binding.value_index);
            self.push_i32_const(1);
            self.push_global_set(target_binding.present_index);
        }
        Ok(())
    }

    fn emit_runtime_object_property_shadow_prefix_copy(
        &mut self,
        source_owner: &str,
        target_owner: &str,
    ) -> DirectResult<()> {
        let handled_suffixes = self
            .object_runtime_shadow_properties(source_owner)
            .into_iter()
            .map(|(property, _)| Self::runtime_object_property_shadow_key(&property))
            .collect::<HashSet<_>>();
        let source_prefix = format!("__ayy_object_property__{source_owner}__");
        let source_deleted_prefix = format!("__ayy_object_property_deleted__{source_owner}__");
        let implicit_bindings = self
            .backend
            .global_semantics
            .global_names()
            .implicit_bindings
            .iter()
            .map(|(name, binding)| (name.clone(), *binding))
            .collect::<Vec<_>>();
        let mut suffix_bindings: BTreeMap<
            String,
            (Option<ImplicitGlobalBinding>, Option<ImplicitGlobalBinding>),
        > = BTreeMap::new();

        for (name, binding) in implicit_bindings {
            if let Some(suffix) = name.strip_prefix(&source_prefix) {
                if handled_suffixes.contains(suffix) {
                    continue;
                }
                suffix_bindings.entry(suffix.to_string()).or_default().0 = Some(binding);
                continue;
            }

            let Some(suffix) = name.strip_prefix(&source_deleted_prefix) else {
                continue;
            };
            if handled_suffixes.contains(suffix) {
                continue;
            }
            suffix_bindings.entry(suffix.to_string()).or_default().1 = Some(binding);
        }

        for (suffix, (source_binding, source_deleted)) in suffix_bindings {
            let private_shadow_property_name =
                Self::runtime_object_property_name_from_shadow_suffix(&suffix).filter(
                    |property_name| {
                        property_name.starts_with("__ayy$private$")
                            || property_name.starts_with("__ayy$private_brand$")
                    },
                );
            if std::env::var_os("AYY_TRACE_RUNTIME_SHADOWS").is_some() {
                eprintln!(
                    "runtime_shadow_prefix_copy {source_owner}->{target_owner} suffix={suffix} source_binding={} source_deleted={}",
                    source_binding.is_some(),
                    source_deleted.is_some()
                );
            }
            let target_binding = self.ensure_implicit_global_binding(&format!(
                "__ayy_object_property__{target_owner}__{suffix}"
            ));
            let target_deleted = self.ensure_implicit_global_binding(&format!(
                "__ayy_object_property_deleted__{target_owner}__{suffix}"
            ));

            if let Some(source_deleted) = source_deleted {
                self.push_global_get(source_deleted.present_index);
                self.state.emission.output.instructions.push(0x04);
                self.state
                    .emission
                    .output
                    .instructions
                    .push(EMPTY_BLOCK_TYPE);
                self.push_control_frame();
                self.push_i32_const(JS_UNDEFINED_TAG);
                self.push_global_set(target_binding.value_index);
                self.push_i32_const(0);
                self.push_global_set(target_binding.present_index);
                self.push_i32_const(JS_UNDEFINED_TAG);
                self.push_global_set(target_deleted.value_index);
                self.push_i32_const(1);
                self.push_global_set(target_deleted.present_index);
                self.state.emission.output.instructions.push(0x05);
                if let Some(source_binding) = source_binding {
                    self.push_global_get(source_binding.present_index);
                    self.state.emission.output.instructions.push(0x04);
                    self.state
                        .emission
                        .output
                        .instructions
                        .push(EMPTY_BLOCK_TYPE);
                    self.push_control_frame();
                    self.push_i32_const(JS_UNDEFINED_TAG);
                    self.push_global_set(target_deleted.value_index);
                    self.push_i32_const(0);
                    self.push_global_set(target_deleted.present_index);
                    if std::env::var_os("AYY_TRACE_PRIVATE_MEMBER_VALUES").is_some()
                        && private_shadow_property_name.is_some()
                    {
                        let copied_value_local = self.allocate_temp_local();
                        self.push_global_get(source_binding.value_index);
                        self.push_local_set(copied_value_local);
                        self.emit_runtime_shadow_debug_print_local(
                            &format!(
                                "private_shadow_prefix_copy {source_owner}->{target_owner} {}",
                                private_shadow_property_name
                                    .as_deref()
                                    .unwrap_or(suffix.as_str())
                            ),
                            copied_value_local,
                        )?;
                        self.push_local_get(copied_value_local);
                    } else {
                        self.push_global_get(source_binding.value_index);
                    }
                    self.push_global_set(target_binding.value_index);
                    self.push_i32_const(1);
                    self.push_global_set(target_binding.present_index);
                    self.state.emission.output.instructions.push(0x05);
                    self.push_i32_const(JS_UNDEFINED_TAG);
                    self.push_global_set(target_binding.value_index);
                    self.push_i32_const(0);
                    self.push_global_set(target_binding.present_index);
                    self.push_i32_const(JS_UNDEFINED_TAG);
                    self.push_global_set(target_deleted.value_index);
                    self.push_i32_const(0);
                    self.push_global_set(target_deleted.present_index);
                    self.state.emission.output.instructions.push(0x0b);
                    self.pop_control_frame();
                } else {
                    self.push_i32_const(JS_UNDEFINED_TAG);
                    self.push_global_set(target_binding.value_index);
                    self.push_i32_const(0);
                    self.push_global_set(target_binding.present_index);
                    self.push_i32_const(JS_UNDEFINED_TAG);
                    self.push_global_set(target_deleted.value_index);
                    self.push_i32_const(0);
                    self.push_global_set(target_deleted.present_index);
                }
                self.state.emission.output.instructions.push(0x0b);
                self.pop_control_frame();
                continue;
            }

            if let Some(source_binding) = source_binding {
                if std::env::var_os("AYY_TRACE_PRIVATE_MEMBER_VALUES").is_some()
                    && private_shadow_property_name.is_some()
                {
                    let copied_value_local = self.allocate_temp_local();
                    self.push_global_get(source_binding.value_index);
                    self.push_local_set(copied_value_local);
                    self.emit_runtime_shadow_debug_print_local(
                        &format!(
                            "private_shadow_prefix_copy {source_owner}->{target_owner} {}",
                            private_shadow_property_name
                                .as_deref()
                                .unwrap_or(suffix.as_str())
                        ),
                        copied_value_local,
                    )?;
                    self.push_local_get(copied_value_local);
                } else {
                    self.push_global_get(source_binding.value_index);
                }
                self.push_global_set(target_binding.value_index);
                self.push_global_get(source_binding.present_index);
                self.push_global_set(target_binding.present_index);
                self.push_i32_const(JS_UNDEFINED_TAG);
                self.push_global_set(target_deleted.value_index);
                self.push_i32_const(0);
                self.push_global_set(target_deleted.present_index);
                continue;
            }

            self.push_i32_const(JS_UNDEFINED_TAG);
            self.push_global_set(target_binding.value_index);
            self.push_i32_const(0);
            self.push_global_set(target_binding.present_index);
            self.push_i32_const(JS_UNDEFINED_TAG);
            self.push_global_set(target_deleted.value_index);
            self.push_i32_const(0);
            self.push_global_set(target_deleted.present_index);
        }
        Ok(())
    }

    pub(in crate::backend::direct_wasm) fn clear_runtime_object_property_shadow_prefix(
        &mut self,
        owner_name: &str,
    ) {
        let property_prefix = format!("__ayy_object_property__{owner_name}__");
        let deleted_prefix = format!("__ayy_object_property_deleted__{owner_name}__");
        let implicit_bindings = self
            .backend
            .global_semantics
            .global_names()
            .implicit_bindings
            .iter()
            .map(|(name, binding)| (name.clone(), *binding))
            .collect::<Vec<_>>();

        for (name, binding) in implicit_bindings {
            if name.starts_with(&property_prefix) || name.starts_with(&deleted_prefix) {
                self.push_i32_const(JS_UNDEFINED_TAG);
                self.push_global_set(binding.value_index);
                self.push_i32_const(0);
                self.push_global_set(binding.present_index);
            }
        }
    }

    pub(in crate::backend::direct_wasm) fn clear_runtime_object_property_shadow_static_metadata_prefix(
        &mut self,
        owner_name: &str,
    ) {
        let property_prefix = format!("__ayy_object_property__{owner_name}__");
        let deleted_prefix = format!("__ayy_object_property_deleted__{owner_name}__");
        let names = self
            .backend
            .global_semantics
            .global_names()
            .implicit_bindings
            .keys()
            .filter(|name| name.starts_with(&property_prefix) || name.starts_with(&deleted_prefix))
            .cloned()
            .collect::<Vec<_>>();

        for name in names {
            self.clear_global_binding_state(&name);
        }
    }

    pub(in crate::backend::direct_wasm) fn sync_runtime_object_shadow_owner_static_metadata_from_expression(
        &mut self,
        owner_name: &str,
        updated_expression: &Expression,
    ) {
        let updated_expression = self.materialize_static_expression(updated_expression);
        let Some(updated_object_binding) =
            self.resolve_object_binding_from_expression(&updated_expression)
        else {
            return;
        };

        self.clear_runtime_object_property_shadow_static_metadata_prefix(owner_name);
        self.sync_runtime_object_property_shadow_static_metadata_from_binding(
            owner_name,
            &updated_object_binding,
        );

        let resolved_identifier_name = self
            .resolve_current_local_binding(owner_name)
            .map(|(resolved_name, _)| resolved_name)
            .filter(|resolved_name| resolved_name != owner_name);
        if let Some(resolved_name) = resolved_identifier_name.as_deref() {
            self.update_local_object_binding(resolved_name, &updated_expression);
        }
        self.update_local_object_binding(owner_name, &updated_expression);
    }

    fn sync_runtime_object_shadow_owner_static_metadata_from_binding(
        &mut self,
        owner_name: &str,
        updated_object_binding: &ObjectValueBinding,
    ) {
        self.clear_runtime_object_property_shadow_static_metadata_prefix(owner_name);
        self.sync_runtime_object_property_shadow_static_metadata_from_binding(
            owner_name,
            updated_object_binding,
        );

        let resolved_identifier_name = self
            .resolve_current_local_binding(owner_name)
            .map(|(resolved_name, _)| resolved_name)
            .filter(|resolved_name| resolved_name != owner_name);
        if let Some(resolved_name) = resolved_identifier_name.as_deref() {
            self.state
                .speculation
                .static_semantics
                .set_local_object_binding(resolved_name, updated_object_binding.clone());
            self.state
                .speculation
                .static_semantics
                .set_local_kind(resolved_name, StaticValueKind::Object);
        }
        self.state
            .speculation
            .static_semantics
            .set_local_object_binding(owner_name, updated_object_binding.clone());
        self.state
            .speculation
            .static_semantics
            .set_local_kind(owner_name, StaticValueKind::Object);
        if self.binding_name_is_global(owner_name)
            || self.backend.global_has_binding(owner_name)
            || self.backend.global_has_lexical_binding(owner_name)
            || self.global_has_implicit_binding(owner_name)
        {
            self.backend
                .sync_global_object_binding(owner_name, Some(updated_object_binding.clone()));
            self.backend
                .set_global_binding_kind(owner_name, StaticValueKind::Object);
        }
    }

    pub(in crate::backend::direct_wasm) fn sync_user_function_parameter_object_shadow_writeback_static_metadata(
        &mut self,
        writebacks: &[(String, String, Option<ObjectValueBinding>)],
        updated_bindings: Option<&HashMap<String, Expression>>,
    ) {
        let Some(updated_bindings) = updated_bindings else {
            for (param_name, source_owner, source_object_binding) in writebacks {
                let Some(updated_object_binding) = self
                    .resolve_runtime_shadow_object_binding(param_name)
                    .or_else(|| source_object_binding.as_ref().cloned())
                else {
                    continue;
                };
                self.sync_runtime_object_shadow_owner_static_metadata_from_binding(
                    param_name,
                    &updated_object_binding,
                );
                self.sync_runtime_object_shadow_owner_static_metadata_from_binding(
                    source_owner,
                    &updated_object_binding,
                );
            }
            return;
        };

        for (param_name, source_owner, _) in writebacks {
            if std::env::var_os("AYY_TRACE_RUNTIME_SHADOWS").is_some() {
                eprintln!(
                    "runtime_shadow_param_writeback_sync param={param_name} source_owner={source_owner} updated_binding={:?}",
                    updated_bindings.get(param_name),
                    param_name = param_name,
                    source_owner = source_owner,
                );
            }
            let Some(updated_expression) = updated_bindings.get(param_name) else {
                let Some(updated_expression) = updated_bindings.get(source_owner) else {
                    continue;
                };
                self.sync_runtime_object_shadow_owner_static_metadata_from_expression(
                    param_name,
                    updated_expression,
                );
                self.sync_runtime_object_shadow_owner_static_metadata_from_expression(
                    source_owner,
                    updated_expression,
                );
                continue;
            };
            if std::env::var_os("AYY_TRACE_RUNTIME_SHADOWS").is_some() {
                eprintln!(
                    "runtime_shadow_param_writeback_sync_commit param={param_name} source_owner={source_owner} updated_expression={updated_expression:?}",
                    param_name = param_name,
                    source_owner = source_owner,
                );
            }
            self.sync_runtime_object_shadow_owner_static_metadata_from_expression(
                param_name,
                updated_expression,
            );
            self.sync_runtime_object_shadow_owner_static_metadata_from_expression(
                source_owner,
                updated_expression,
            );
        }
    }

    pub(in crate::backend::direct_wasm) fn emit_runtime_object_property_shadow_copy(
        &mut self,
        source_owner: &str,
        target_owner: &str,
    ) -> DirectResult<()> {
        if source_owner == target_owner {
            return Ok(());
        }
        if std::env::var_os("AYY_TRACE_RUNTIME_SHADOWS").is_some() {
            eprintln!("runtime_shadow_copy {source_owner} -> {target_owner}");
        }
        if std::env::var_os("AYY_TRACE_RUNTIME_SHADOWS").is_some() {
            let entry_count = self
                .runtime_object_property_shadow_copy_entries(source_owner)
                .len();
            eprintln!(
                "runtime_shadow_copy_entries {source_owner}->{target_owner} count={entry_count}"
            );
        }
        let mut copy_entries = self.runtime_object_property_shadow_copy_entries(source_owner);
        self.append_target_private_runtime_shadow_copy_entries(
            source_owner,
            target_owner,
            &mut copy_entries,
        );
        for (property, fallback_value) in copy_entries {
            let is_private_property = matches!(
                &property,
                Expression::String(property_name) if property_name.starts_with("__ayy$private$")
            );
            let is_private_shadow_property = matches!(
                &property,
                Expression::String(property_name)
                    if property_name.starts_with("__ayy$private$")
                        || property_name.starts_with("__ayy$private_brand$")
            );
            if std::env::var_os("AYY_TRACE_RUNTIME_SHADOWS").is_some() {
                eprintln!(
                    "runtime_shadow_copy_entry {source_owner}->{target_owner} property={property:?} fallback={fallback_value:?} private={is_private_property}",
                );
            }
            let source_binding =
                self.runtime_object_property_shadow_binding_by_property(source_owner, &property);
            let target_binding =
                self.runtime_object_property_shadow_binding_by_property(target_owner, &property);
            let source_deleted = self.runtime_object_property_shadow_deleted_binding_by_property(
                source_owner,
                &property,
            );
            let target_deleted = self.runtime_object_property_shadow_deleted_binding_by_property(
                target_owner,
                &property,
            );
            self.push_global_get(source_deleted.present_index);
            self.state.emission.output.instructions.push(0x04);
            self.state
                .emission
                .output
                .instructions
                .push(EMPTY_BLOCK_TYPE);
            self.push_control_frame();
            self.push_i32_const(JS_UNDEFINED_TAG);
            self.push_global_set(target_binding.value_index);
            self.push_i32_const(0);
            self.push_global_set(target_binding.present_index);
            self.push_i32_const(JS_UNDEFINED_TAG);
            self.push_global_set(target_deleted.value_index);
            self.push_i32_const(1);
            self.push_global_set(target_deleted.present_index);
            self.state.emission.output.instructions.push(0x05);
            self.push_global_get(source_binding.present_index);
            self.state.emission.output.instructions.push(0x04);
            self.state
                .emission
                .output
                .instructions
                .push(EMPTY_BLOCK_TYPE);
            self.push_control_frame();
            self.push_i32_const(JS_UNDEFINED_TAG);
            self.push_global_set(target_deleted.value_index);
            self.push_i32_const(0);
            self.push_global_set(target_deleted.present_index);
            if std::env::var_os("AYY_TRACE_PRIVATE_MEMBER_VALUES").is_some()
                && is_private_shadow_property
            {
                let copied_value_local = self.allocate_temp_local();
                self.push_global_get(source_binding.value_index);
                self.push_local_set(copied_value_local);
                self.emit_runtime_shadow_debug_print_local(
                    &format!(
                        "private_shadow_copy {source_owner}->{target_owner} {}",
                        static_property_name_from_expression(&property)
                            .unwrap_or_else(|| format!("{property:?}"))
                    ),
                    copied_value_local,
                )?;
                self.push_local_get(copied_value_local);
            } else {
                self.push_global_get(source_binding.value_index);
            }
            self.push_global_set(target_binding.value_index);
            self.push_i32_const(1);
            self.push_global_set(target_binding.present_index);
            self.state.emission.output.instructions.push(0x05);
            self.push_i32_const(JS_UNDEFINED_TAG);
            self.push_global_set(target_deleted.value_index);
            self.push_i32_const(0);
            self.push_global_set(target_deleted.present_index);
            self.push_i32_const(JS_UNDEFINED_TAG);
            self.push_global_set(target_binding.value_index);
            if let Some(fallback_value) = fallback_value.as_ref() {
                if is_private_property
                    && !self.emit_private_brand_marker_runtime_value(
                        &Expression::Identifier(target_owner.to_string()),
                        &property,
                        fallback_value,
                    )?
                {
                    self.emit_runtime_shadow_fallback_value(fallback_value)?;
                } else if !is_private_property {
                    self.emit_runtime_shadow_fallback_value(fallback_value)?;
                }
                if std::env::var_os("AYY_TRACE_PRIVATE_MEMBER_VALUES").is_some()
                    && is_private_shadow_property
                {
                    let copied_value_local = self.allocate_temp_local();
                    self.push_local_set(copied_value_local);
                    self.emit_runtime_shadow_debug_print_local(
                        &format!(
                            "private_shadow_seed {source_owner}->{target_owner} {}",
                            static_property_name_from_expression(&property)
                                .unwrap_or_else(|| format!("{property:?}"))
                        ),
                        copied_value_local,
                    )?;
                    self.push_local_get(copied_value_local);
                }
                self.push_global_set(target_binding.value_index);
                self.push_i32_const(1);
                self.push_global_set(target_binding.present_index);
            } else {
                self.push_i32_const(0);
                self.push_global_set(target_binding.present_index);
            }
            self.state.emission.output.instructions.push(0x0b);
            self.pop_control_frame();
            self.state.emission.output.instructions.push(0x0b);
            self.pop_control_frame();
        }
        self.emit_runtime_object_property_shadow_prefix_copy(source_owner, target_owner)?;
        Ok(())
    }

    pub(in crate::backend::direct_wasm) fn emit_object_spread_copy_data_properties_effects(
        &mut self,
        expression: &Expression,
    ) -> DirectResult<()> {
        if !inline_summary_side_effect_free_expression(expression) {
            return Ok(());
        }
        let Some(object_binding) = self.resolve_object_binding_from_expression(expression) else {
            return Ok(());
        };

        for property_name in ordered_object_property_names(&object_binding) {
            if object_binding
                .non_enumerable_string_properties
                .iter()
                .any(|hidden_name| hidden_name == &property_name)
            {
                continue;
            }
            self.emit_member_read_without_prelude(expression, &Expression::String(property_name))?;
            self.state.emission.output.instructions.push(0x1a);
        }
        for (property, _) in &object_binding.symbol_properties {
            self.emit_member_read_without_prelude(expression, property)?;
            self.state.emission.output.instructions.push(0x1a);
        }

        Ok(())
    }
}
