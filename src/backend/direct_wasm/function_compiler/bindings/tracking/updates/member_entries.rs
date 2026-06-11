use super::*;

impl<'a> FunctionCompiler<'a> {
    fn set_member_function_binding_entry(
        &mut self,
        key: &MemberFunctionBindingKey,
        binding: LocalFunctionBinding,
    ) {
        crate::backend::direct_wasm::memo::bump_static_state_generation();
        self.state
            .speculation
            .static_semantics
            .objects
            .member_function_bindings
            .insert(key.clone(), binding.clone());
        if self.binding_key_is_global(key) {
            self.backend
                .set_global_member_function_binding(key.clone(), binding);
        }
    }

    fn clear_member_function_binding_entry(&mut self, key: &MemberFunctionBindingKey) {
        crate::backend::direct_wasm::memo::bump_static_state_generation();
        self.state
            .speculation
            .static_semantics
            .objects
            .member_function_bindings
            .remove(key);
        if self.binding_key_is_global(key) {
            self.backend.clear_global_member_function_binding(key);
        }
    }

    fn set_member_getter_binding_entry(
        &mut self,
        key: &MemberFunctionBindingKey,
        binding: LocalFunctionBinding,
    ) {
        crate::backend::direct_wasm::memo::bump_static_state_generation();
        self.state
            .speculation
            .static_semantics
            .objects
            .member_getter_bindings
            .insert(key.clone(), binding.clone());
        if self.binding_key_is_global(key) {
            self.backend
                .set_global_member_getter_binding(key.clone(), binding);
        }
    }

    fn clear_member_getter_binding_entry(&mut self, key: &MemberFunctionBindingKey) {
        crate::backend::direct_wasm::memo::bump_static_state_generation();
        self.state
            .speculation
            .static_semantics
            .objects
            .member_getter_bindings
            .remove(key);
        if self.binding_key_is_global(key) {
            self.backend.clear_global_member_getter_binding(key);
        }
    }

    fn set_member_setter_binding_entry(
        &mut self,
        key: &MemberFunctionBindingKey,
        binding: LocalFunctionBinding,
    ) {
        crate::backend::direct_wasm::memo::bump_static_state_generation();
        self.state
            .speculation
            .static_semantics
            .objects
            .member_setter_bindings
            .insert(key.clone(), binding.clone());
        if self.binding_key_is_global(key) {
            self.backend
                .set_global_member_setter_binding(key.clone(), binding);
        }
    }

    fn clear_member_setter_binding_entry(&mut self, key: &MemberFunctionBindingKey) {
        crate::backend::direct_wasm::memo::bump_static_state_generation();
        self.state
            .speculation
            .static_semantics
            .objects
            .member_setter_bindings
            .remove(key);
        if self.binding_key_is_global(key) {
            self.backend.clear_global_member_setter_binding(key);
        }
    }

    pub(in crate::backend::direct_wasm) fn update_member_function_binding_from_expression(
        &mut self,
        expression: &Expression,
    ) {
        let trace_iterator_close_updates = crate::ayy_env_flag!("AYY_TRACE_ITERATOR_CLOSE_UPDATES");
        if trace_iterator_close_updates && matches!(expression, Expression::IteratorClose(_)) {
            eprintln!("iterator_close_updates:member:start expr={expression:?}");
        }
        match expression {
            Expression::Member { object, property } => {
                self.update_member_function_binding_from_expression(object);
                self.update_member_function_binding_from_expression(property);
            }
            Expression::SuperMember { property } => {
                self.update_member_function_binding_from_expression(property);
            }
            Expression::Assign { value, .. }
            | Expression::Await(value)
            | Expression::EnumerateKeys(value)
            | Expression::GetIterator(value)
            | Expression::IteratorClose(value)
            | Expression::Unary {
                expression: value, ..
            } => self.update_member_function_binding_from_expression(value),
            Expression::AssignMember {
                object,
                property,
                value,
            } => {
                self.update_member_function_binding_from_expression(object);
                self.update_member_function_binding_from_expression(property);
                self.update_member_function_binding_from_expression(value);
            }
            Expression::AssignSuperMember { property, value } => {
                self.update_member_function_binding_from_expression(property);
                self.update_member_function_binding_from_expression(value);
            }
            Expression::Binary { left, right, .. } => {
                self.update_member_function_binding_from_expression(left);
                self.update_member_function_binding_from_expression(right);
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                self.update_member_function_binding_from_expression(condition);
                self.update_member_function_binding_from_expression(then_expression);
                self.update_member_function_binding_from_expression(else_expression);
            }
            Expression::Sequence(expressions) => {
                for expression in expressions {
                    self.update_member_function_binding_from_expression(expression);
                }
            }
            Expression::Call { callee, arguments }
            | Expression::SuperCall { callee, arguments }
            | Expression::New { callee, arguments } => {
                self.update_member_function_binding_from_expression(callee);
                for argument in arguments {
                    match argument {
                        CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                            self.update_member_function_binding_from_expression(expression);
                        }
                    }
                }
            }
            Expression::Array(elements) => {
                for element in elements {
                    match element {
                        crate::ir::hir::ArrayElement::Expression(expression)
                        | crate::ir::hir::ArrayElement::Spread(expression) => {
                            self.update_member_function_binding_from_expression(expression);
                        }
                    }
                }
            }
            Expression::Object(entries) => {
                for entry in entries {
                    match entry {
                        crate::ir::hir::ObjectEntry::Data { key, value } => {
                            self.update_member_function_binding_from_expression(key);
                            self.update_member_function_binding_from_expression(value);
                        }
                        crate::ir::hir::ObjectEntry::Getter { key, getter } => {
                            self.update_member_function_binding_from_expression(key);
                            self.update_member_function_binding_from_expression(getter);
                        }
                        crate::ir::hir::ObjectEntry::Setter { key, setter } => {
                            self.update_member_function_binding_from_expression(key);
                            self.update_member_function_binding_from_expression(setter);
                        }
                        crate::ir::hir::ObjectEntry::Spread(expression) => {
                            self.update_member_function_binding_from_expression(expression);
                        }
                    }
                }
            }
            _ => {}
        }
        if trace_iterator_close_updates && matches!(expression, Expression::IteratorClose(_)) {
            eprintln!("iterator_close_updates:member:after_walk expr={expression:?}");
        }
        let Expression::Call { callee, arguments } = expression else {
            if trace_iterator_close_updates && matches!(expression, Expression::IteratorClose(_)) {
                eprintln!("iterator_close_updates:member:done expr={expression:?}");
            }
            return;
        };
        let Expression::Member { object, property } = callee.as_ref() else {
            return;
        };
        if !matches!(object.as_ref(), Expression::Identifier(name) if name == "Object" || name == "Reflect")
        {
            return;
        }
        if !matches!(property.as_ref(), Expression::String(name) if name == "defineProperty") {
            return;
        }

        let [
            CallArgument::Expression(target),
            CallArgument::Expression(property),
            CallArgument::Expression(descriptor),
            ..,
        ] = arguments.as_slice()
        else {
            return;
        };

        let Some(key) = self.member_function_binding_key(target, property) else {
            return;
        };
        let has_value_field = self.descriptor_expression_has_named_field(descriptor, "value");
        let has_get_field = self.descriptor_expression_has_named_field(descriptor, "get");
        let has_set_field = self.descriptor_expression_has_named_field(descriptor, "set");
        let value_binding = self.resolve_function_binding_from_descriptor_expression(descriptor);
        let getter_binding = self.resolve_getter_binding_from_descriptor_expression(descriptor);
        let setter_binding = self.resolve_setter_binding_from_descriptor_expression(descriptor);

        if let Some(binding) = value_binding {
            self.set_member_function_binding_entry(&key, binding);
        } else if has_value_field {
            self.clear_member_function_binding_entry(&key);
        }

        if let Some(binding) = getter_binding {
            self.record_receiver_delete_shadows_from_accessor_binding(target, &binding);
            self.set_member_getter_binding_entry(&key, binding);
        } else if has_get_field {
            self.clear_member_getter_binding_entry(&key);
        }

        if let Some(binding) = setter_binding {
            self.record_receiver_delete_shadows_from_accessor_binding(target, &binding);
            self.set_member_setter_binding_entry(&key, binding);
        } else if has_set_field {
            self.clear_member_setter_binding_entry(&key);
        }
        if trace_iterator_close_updates && matches!(expression, Expression::IteratorClose(_)) {
            eprintln!("iterator_close_updates:member:done expr={expression:?}");
        }
    }

    /// Records prospective delete shadows for properties that an accessor
    /// deletes from its receiver (`get x() { delete this.x; ... }`). The
    /// accessor body may be compiled after statements that statically fold
    /// property presence, so the emitted-delete registry must learn about the
    /// deletion when the accessor is bound, not when its body is compiled.
    pub(in crate::backend::direct_wasm) fn record_receiver_delete_shadows_from_accessor_binding(
        &mut self,
        target: &Expression,
        binding: &LocalFunctionBinding,
    ) {
        let LocalFunctionBinding::User(function_name) = binding else {
            return;
        };
        let Some(summary) = self
            .user_function(function_name)
            .and_then(|user_function| user_function.inline_summary.clone())
        else {
            return;
        };
        let owner_name = match target {
            Expression::Identifier(name) => {
                self.runtime_object_property_shadow_owner_name_for_identifier(name)
            }
            Expression::This => self.runtime_object_property_shadow_owner_name_for_identifier("this"),
            _ => None,
        };
        let Some(owner_name) = owner_name else {
            return;
        };
        for effect in &summary.effects {
            let InlineFunctionEffect::Expression(Expression::Unary {
                op: UnaryOp::Delete,
                expression,
            }) = effect
            else {
                continue;
            };
            let Expression::Member { object, property } = expression.as_ref() else {
                continue;
            };
            if !matches!(object.as_ref(), Expression::This) {
                continue;
            }
            let Some(property_name) = static_property_name_from_expression(property) else {
                continue;
            };
            if crate::ayy_env_flag!("AYY_TRACE_RUNTIME_SHADOWS") {
                eprintln!(
                    "record_accessor_delete_shadow owner={owner_name} property={property_name} accessor={function_name}"
                );
            }
            self.record_emitted_delete_shadow_for(
                &owner_name,
                &Expression::String(property_name),
            );
        }
    }

    /// Applies the receiver-delete effects of a global accessor when an
    /// unresolvable identifier reference to it is evaluated (for example a
    /// strict closure performing `x++` against `Object.defineProperty(this,
    /// "x", { get() { delete this.x; ... } })`). The spec evaluates GetValue
    /// (running the getter and its deletes) before PutValue throws, so the
    /// deletion must be reflected in the runtime shadow state and static
    /// metadata even though the reference itself is compiled to a throw.
    pub(in crate::backend::direct_wasm) fn emit_unresolvable_global_accessor_reference_read_effects(
        &mut self,
        name: &str,
    ) -> DirectResult<()> {
        let Some(getter) = self
            .backend
            .global_property_descriptor(name)
            .and_then(|descriptor| descriptor.getter.clone())
        else {
            return Ok(());
        };
        let Some(LocalFunctionBinding::User(function_name)) =
            self.resolve_function_binding_from_expression(&getter)
        else {
            return Ok(());
        };
        let Some(summary) = self
            .user_function(&function_name)
            .and_then(|user_function| user_function.inline_summary.clone())
        else {
            return Ok(());
        };
        for effect in &summary.effects {
            let InlineFunctionEffect::Expression(Expression::Unary {
                op: UnaryOp::Delete,
                expression,
            }) = effect
            else {
                continue;
            };
            let Expression::Member { object, property } = expression.as_ref() else {
                continue;
            };
            if !matches!(object.as_ref(), Expression::This) {
                continue;
            }
            let Some(property_name) = static_property_name_from_expression(property) else {
                continue;
            };
            let property_expression = Expression::String(property_name);
            self.mark_runtime_object_property_shadow_deleted_binding(
                &Expression::This,
                &property_expression,
            );
            self.clear_member_function_bindings_for_deleted_property(
                &Expression::This,
                &property_expression,
            );
        }
        Ok(())
    }

    /// Clears the tracked data/getter/setter function bindings for a member
    /// that has been deleted at runtime (`delete obj.x`), so stale accessor
    /// metadata cannot resurface after the deletion.
    pub(in crate::backend::direct_wasm) fn clear_member_function_bindings_for_deleted_property(
        &mut self,
        object: &Expression,
        property: &Expression,
    ) {
        let Some(key) = self.member_function_binding_key(object, property) else {
            return;
        };
        self.clear_member_function_binding_entry(&key);
        self.clear_member_getter_binding_entry(&key);
        self.clear_member_setter_binding_entry(&key);
    }

    pub(in crate::backend::direct_wasm) fn update_member_function_assignment_binding(
        &mut self,
        object: &Expression,
        property: &Expression,
        value: &Expression,
    ) {
        let Some(key) = self.member_function_binding_key(object, property) else {
            return;
        };
        let value_binding = self.resolve_function_binding_from_expression(value);

        if let Some(binding) = value_binding {
            self.set_member_function_binding_entry(&key, binding);
        } else {
            self.clear_member_function_binding_entry(&key);
        }

        self.clear_member_getter_binding_entry(&key);
        self.clear_member_setter_binding_entry(&key);
    }

    pub(in crate::backend::direct_wasm) fn update_local_function_binding(
        &mut self,
        name: &str,
        value: &Expression,
    ) {
        let iterator_step_value = match value {
            Expression::Await(value) => value.as_ref(),
            _ => value,
        };
        if let Expression::Member { object, property } = iterator_step_value
            && matches!(property.as_ref(), Expression::String(property_name) if property_name == "value")
            && let Some(IteratorStepBinding::Runtime {
                function_binding,
                static_value,
                ..
            }) = self.resolve_iterator_step_binding_from_expression(object)
        {
            if let Some(function_binding) = function_binding {
                self.state
                    .speculation
                    .static_semantics
                    .set_local_function_binding(name, function_binding);
                return;
            }
            if let Some(static_value) = static_value.as_ref()
                && let Some(function_binding) =
                    self.resolve_function_binding_from_expression(static_value)
            {
                self.state
                    .speculation
                    .static_semantics
                    .set_local_function_binding(name, function_binding);
                return;
            }
            self.state
                .speculation
                .static_semantics
                .clear_local_function_binding(name);
            return;
        }
        if self.expression_depends_on_active_loop_assignment(value) {
            self.state
                .speculation
                .static_semantics
                .clear_local_function_binding(name);
            return;
        }
        let Some(function_name) = self.resolve_function_binding_from_expression(value) else {
            self.state
                .speculation
                .static_semantics
                .clear_local_function_binding(name);
            return;
        };
        self.state
            .speculation
            .static_semantics
            .set_local_function_binding(name, function_name);
    }
}
