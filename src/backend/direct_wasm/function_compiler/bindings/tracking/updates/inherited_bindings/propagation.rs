use super::*;

fn value_is_function_prototype_bind_call(value: &Expression) -> bool {
    matches!(
        value,
        Expression::Call { callee, .. }
            if matches!(
                callee.as_ref(),
                Expression::Member { property, .. }
                    if matches!(property.as_ref(), Expression::String(name) if name == "bind")
            )
    )
}

impl<'a> FunctionCompiler<'a> {
    fn store_preserves_existing_member_bindings(&self, name: &str, value: &Expression) -> bool {
        let Some(existing_binding) = self
            .resolve_function_binding_from_expression(&Expression::Identifier(name.to_string()))
        else {
            return false;
        };
        let Some(value_binding) = self.resolve_function_binding_from_expression(value) else {
            return false;
        };
        if existing_binding != value_binding {
            return false;
        }
        let Some(owner_name) = self.function_prototype_binding_owner_name(&value_binding) else {
            return false;
        };
        let source_name = scoped_binding_source_name(name).unwrap_or(name);
        owner_name == source_name
    }

    fn inherited_member_binding_target(
        &self,
        name: &str,
        target: ReturnedMemberFunctionBindingTarget,
    ) -> MemberFunctionBindingTarget {
        match target {
            ReturnedMemberFunctionBindingTarget::Value => {
                MemberFunctionBindingTarget::Identifier(name.to_string())
            }
            ReturnedMemberFunctionBindingTarget::Prototype => {
                MemberFunctionBindingTarget::Prototype(name.to_string())
            }
        }
    }

    fn insert_inherited_member_function_binding_for_name(
        &mut self,
        name: &str,
        binding: ReturnedMemberFunctionBinding,
        capture_slots_by_property: &HashMap<String, BTreeMap<String, String>>,
    ) {
        let property_name = binding.property.clone();
        let key = MemberFunctionBindingKey {
            target: self.inherited_member_binding_target(name, binding.target),
            property: MemberFunctionBindingProperty::String(property_name.clone()),
        };
        self.state
            .speculation
            .static_semantics
            .objects
            .member_function_bindings
            .insert(key.clone(), binding.binding.clone());
        if let Some(capture_slots) = capture_slots_by_property.get(&property_name).cloned() {
            self.state
                .speculation
                .static_semantics
                .objects
                .member_function_capture_slots
                .insert(key.clone(), capture_slots.clone());
            if self.binding_name_is_global(name) {
                self.backend
                    .set_global_member_function_capture_slots(key.clone(), capture_slots);
            }
        }
        if self.binding_name_is_global(name) {
            self.backend
                .set_global_member_function_binding(key, binding.binding);
        }
    }

    fn insert_inherited_member_getter_binding_for_name(
        &mut self,
        name: &str,
        binding: ReturnedMemberFunctionBinding,
        capture_slots_by_property: &HashMap<String, BTreeMap<String, String>>,
    ) {
        let property_name = binding.property.clone();
        let key = MemberFunctionBindingKey {
            target: self.inherited_member_binding_target(name, binding.target),
            property: MemberFunctionBindingProperty::String(property_name.clone()),
        };
        self.state
            .speculation
            .static_semantics
            .objects
            .member_getter_bindings
            .insert(key.clone(), binding.binding.clone());
        if let Some(capture_slots) = capture_slots_by_property.get(&property_name).cloned() {
            self.state
                .speculation
                .static_semantics
                .objects
                .member_function_capture_slots
                .insert(key.clone(), capture_slots.clone());
            if self.binding_name_is_global(name) {
                self.backend
                    .set_global_member_function_capture_slots(key.clone(), capture_slots);
            }
        }
        if self.binding_name_is_global(name) {
            self.backend
                .set_global_member_getter_binding(key, binding.binding);
        }
    }

    fn insert_inherited_member_setter_binding_for_name(
        &mut self,
        name: &str,
        binding: ReturnedMemberFunctionBinding,
        capture_slots_by_property: &HashMap<String, BTreeMap<String, String>>,
    ) {
        let property_name = binding.property.clone();
        let key = MemberFunctionBindingKey {
            target: self.inherited_member_binding_target(name, binding.target),
            property: MemberFunctionBindingProperty::String(property_name.clone()),
        };
        self.state
            .speculation
            .static_semantics
            .objects
            .member_setter_bindings
            .insert(key.clone(), binding.binding.clone());
        if let Some(capture_slots) = capture_slots_by_property.get(&property_name).cloned() {
            self.state
                .speculation
                .static_semantics
                .objects
                .member_function_capture_slots
                .insert(key.clone(), capture_slots.clone());
            if self.binding_name_is_global(name) {
                self.backend
                    .set_global_member_function_capture_slots(key.clone(), capture_slots);
            }
        }
        if self.binding_name_is_global(name) {
            self.backend
                .set_global_member_setter_binding(key, binding.binding);
        }
    }

    fn symbol_member_capture_slot_property_name(
        property: &MemberFunctionBindingProperty,
    ) -> String {
        match property {
            MemberFunctionBindingProperty::String(name) => name.clone(),
            MemberFunctionBindingProperty::Symbol(name) => format!("__ayy_symbol::{name}"),
            MemberFunctionBindingProperty::SymbolExpression(name) => {
                format!("__ayy_symbol_expr::{name}")
            }
        }
    }

    fn inherited_symbol_member_function_bindings(
        &self,
        value: &Expression,
    ) -> Vec<(MemberFunctionBindingProperty, LocalFunctionBinding)> {
        let Some(object_binding) = self.resolve_object_binding_from_expression(value) else {
            return Vec::new();
        };
        object_binding
            .symbol_properties
            .iter()
            .filter_map(|(property, value)| {
                let property = self.member_function_binding_property(property)?;
                let binding = self.resolve_function_binding_from_expression(value)?;
                Some((property, binding))
            })
            .collect()
    }

    fn insert_inherited_symbol_member_function_bindings_for_name(
        &mut self,
        name: &str,
        value: &Expression,
        value_local: u32,
    ) -> DirectResult<()> {
        for (property, binding) in self.inherited_symbol_member_function_bindings(value) {
            let capture_property_name = Self::symbol_member_capture_slot_property_name(&property);
            let returned_binding = ReturnedMemberFunctionBinding {
                target: ReturnedMemberFunctionBindingTarget::Value,
                property: capture_property_name.clone(),
                binding: binding.clone(),
            };
            let capture_slots_by_property = self
                .initialize_returned_member_capture_slots_for_bindings(
                    name,
                    value,
                    value_local,
                    std::slice::from_ref(&returned_binding),
                )?;
            let key = MemberFunctionBindingKey {
                target: MemberFunctionBindingTarget::Identifier(name.to_string()),
                property,
            };
            self.state
                .speculation
                .static_semantics
                .objects
                .member_function_bindings
                .insert(key.clone(), binding.clone());
            if let Some(capture_slots) = capture_slots_by_property
                .get(&capture_property_name)
                .cloned()
            {
                self.state
                    .speculation
                    .static_semantics
                    .objects
                    .member_function_capture_slots
                    .insert(key.clone(), capture_slots.clone());
                if self.binding_name_is_global(name) {
                    self.backend
                        .set_global_member_function_capture_slots(key.clone(), capture_slots);
                }
            }
            if self.binding_name_is_global(name) {
                self.backend
                    .set_global_member_function_binding(key, binding);
            }
        }
        Ok(())
    }

    pub(in crate::backend::direct_wasm) fn update_member_function_bindings_for_value(
        &mut self,
        name: &str,
        value: &Expression,
        value_local: u32,
    ) -> DirectResult<()> {
        if matches!(
            value,
            Expression::Number(_)
                | Expression::BigInt(_)
                | Expression::String(_)
                | Expression::Bool(_)
                | Expression::Null
                | Expression::Undefined
        ) {
            self.clear_member_function_bindings_for_name(name);
            return Ok(());
        }
        if value_is_function_prototype_bind_call(value) {
            self.clear_member_function_bindings_for_name(name);
            return Ok(());
        }
        if matches!(
            value,
            Expression::Identifier(source_name)
                if parse_test262_realm_eval_builtin(source_name).is_some()
        ) {
            self.clear_member_function_bindings_for_name(name);
            return Ok(());
        }
        let value_is_static_class_instance_new = matches!(
            value,
            Expression::New { callee, .. } if matches!(
                callee.as_ref(),
                Expression::Identifier(function_name) if function_name.starts_with("__ayy_class_ctor_")
            ) || matches!(
                callee.as_ref(),
                Expression::Call {
                    callee: init_callee,
                    arguments: init_arguments,
                } if init_arguments.is_empty()
                    && matches!(
                        init_callee.as_ref(),
                        Expression::Identifier(function_name)
                            if function_name.starts_with("__ayy_class_init_")
                    )
            )
        );
        if !value_is_static_class_instance_new
            && self.store_preserves_existing_member_bindings(name, value)
        {
            return Ok(());
        }
        self.clear_member_function_bindings_for_name(name);
        if let Expression::Identifier(source_name) = value {
            if matches!(
                self.infer_value_kind(value),
                Some(
                    StaticValueKind::Number
                        | StaticValueKind::BigInt
                        | StaticValueKind::String
                        | StaticValueKind::Bool
                        | StaticValueKind::Null
                        | StaticValueKind::Undefined
                        | StaticValueKind::Symbol
                )
            ) {
                return Ok(());
            }
            self.copy_member_bindings_for_alias(name, source_name);
            return Ok(());
        }

        let inherited_source = self
            .direct_iterator_binding_source_expression(value)
            .unwrap_or(value);
        if let Expression::Identifier(source_name) = inherited_source {
            if matches!(
                self.infer_value_kind(inherited_source),
                Some(
                    StaticValueKind::Number
                        | StaticValueKind::BigInt
                        | StaticValueKind::String
                        | StaticValueKind::Bool
                        | StaticValueKind::Null
                        | StaticValueKind::Undefined
                        | StaticValueKind::Symbol
                )
            ) {
                return Ok(());
            }
            self.copy_member_bindings_for_alias(name, source_name);
            return Ok(());
        }
        let inherited_function_bindings = self.inherited_member_function_bindings(inherited_source);
        let capture_slots_by_property = self
            .initialize_returned_member_capture_slots_for_bindings(
                name,
                inherited_source,
                value_local,
                &inherited_function_bindings,
            )?;
        for binding in inherited_function_bindings {
            self.insert_inherited_member_function_binding_for_name(
                name,
                binding,
                &capture_slots_by_property,
            );
        }
        self.insert_inherited_symbol_member_function_bindings_for_name(
            name,
            inherited_source,
            value_local,
        )?;
        let inherited_getter_bindings = self.inherited_member_getter_bindings(inherited_source);
        let getter_capture_slots_by_property = self
            .initialize_returned_member_capture_slots_for_bindings(
                name,
                inherited_source,
                value_local,
                &inherited_getter_bindings,
            )?;
        for binding in inherited_getter_bindings {
            self.insert_inherited_member_getter_binding_for_name(
                name,
                binding,
                &getter_capture_slots_by_property,
            );
        }
        let inherited_setter_bindings = self.inherited_member_setter_bindings(inherited_source);
        let setter_capture_slots_by_property = self
            .initialize_returned_member_capture_slots_for_bindings(
                name,
                inherited_source,
                value_local,
                &inherited_setter_bindings,
            )?;
        for binding in inherited_setter_bindings {
            self.insert_inherited_member_setter_binding_for_name(
                name,
                binding,
                &setter_capture_slots_by_property,
            );
        }
        if let Expression::GetIterator(iterated) = value {
            let iterator_call = Expression::Call {
                callee: Box::new(Expression::Member {
                    object: Box::new((**iterated).clone()),
                    property: Box::new(symbol_iterator_expression()),
                }),
                arguments: Vec::new(),
            };
            let iterator_function_bindings =
                self.inherited_member_function_bindings(&iterator_call);
            let iterator_capture_slots_by_property = self
                .initialize_returned_member_capture_slots_for_bindings(
                    name,
                    &iterator_call,
                    value_local,
                    &iterator_function_bindings,
                )?;
            for binding in iterator_function_bindings {
                self.insert_inherited_member_function_binding_for_name(
                    name,
                    binding,
                    &iterator_capture_slots_by_property,
                );
            }
            let iterator_getter_bindings = self.inherited_member_getter_bindings(&iterator_call);
            let iterator_getter_capture_slots_by_property = self
                .initialize_returned_member_capture_slots_for_bindings(
                    name,
                    &iterator_call,
                    value_local,
                    &iterator_getter_bindings,
                )?;
            for binding in iterator_getter_bindings {
                self.insert_inherited_member_getter_binding_for_name(
                    name,
                    binding,
                    &iterator_getter_capture_slots_by_property,
                );
            }
            let iterator_setter_bindings = self.inherited_member_setter_bindings(&iterator_call);
            let iterator_setter_capture_slots_by_property = self
                .initialize_returned_member_capture_slots_for_bindings(
                    name,
                    &iterator_call,
                    value_local,
                    &iterator_setter_bindings,
                )?;
            for binding in iterator_setter_bindings {
                self.insert_inherited_member_setter_binding_for_name(
                    name,
                    binding,
                    &iterator_setter_capture_slots_by_property,
                );
            }
        }
        Ok(())
    }
}
