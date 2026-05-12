use super::*;

fn tracked_array_push_expression_references_internal_iterator_step(
    expression: &Expression,
) -> bool {
    match expression {
        Expression::Identifier(name) => {
            name.starts_with("__ayy_array_step_")
                || name.starts_with("__ayy_array_iter_value_")
                || name.starts_with("__ayy_array_iter_done_")
                || name.starts_with("__ayy_for_of_step_")
                || name.starts_with("__ayy_for_of_iter_value_")
                || name.starts_with("__ayy_for_of_iter_done_")
        }
        Expression::Array(elements) => elements.iter().any(|element| match element {
            ArrayElement::Expression(value) | ArrayElement::Spread(value) => {
                tracked_array_push_expression_references_internal_iterator_step(value)
            }
        }),
        Expression::Object(entries) => entries.iter().any(|entry| match entry {
            ObjectEntry::Data { key, value } => {
                tracked_array_push_expression_references_internal_iterator_step(key)
                    || tracked_array_push_expression_references_internal_iterator_step(value)
            }
            ObjectEntry::Getter { key, getter } => {
                tracked_array_push_expression_references_internal_iterator_step(key)
                    || tracked_array_push_expression_references_internal_iterator_step(getter)
            }
            ObjectEntry::Setter { key, setter } => {
                tracked_array_push_expression_references_internal_iterator_step(key)
                    || tracked_array_push_expression_references_internal_iterator_step(setter)
            }
            ObjectEntry::Spread(value) => {
                tracked_array_push_expression_references_internal_iterator_step(value)
            }
        }),
        Expression::Member { object, property } => {
            tracked_array_push_expression_references_internal_iterator_step(object)
                || tracked_array_push_expression_references_internal_iterator_step(property)
        }
        Expression::SuperMember { property } => {
            tracked_array_push_expression_references_internal_iterator_step(property)
        }
        Expression::Assign { value, .. }
        | Expression::AssignSuperMember { value, .. }
        | Expression::Await(value)
        | Expression::EnumerateKeys(value)
        | Expression::GetIterator(value)
        | Expression::IteratorClose(value)
        | Expression::Unary {
            expression: value, ..
        } => tracked_array_push_expression_references_internal_iterator_step(value),
        Expression::AssignMember {
            object,
            property,
            value,
        } => {
            tracked_array_push_expression_references_internal_iterator_step(object)
                || tracked_array_push_expression_references_internal_iterator_step(property)
                || tracked_array_push_expression_references_internal_iterator_step(value)
        }
        Expression::Binary { left, right, .. } => {
            tracked_array_push_expression_references_internal_iterator_step(left)
                || tracked_array_push_expression_references_internal_iterator_step(right)
        }
        Expression::Conditional {
            condition,
            then_expression,
            else_expression,
        } => {
            tracked_array_push_expression_references_internal_iterator_step(condition)
                || tracked_array_push_expression_references_internal_iterator_step(then_expression)
                || tracked_array_push_expression_references_internal_iterator_step(else_expression)
        }
        Expression::Sequence(expressions) => expressions
            .iter()
            .any(tracked_array_push_expression_references_internal_iterator_step),
        Expression::Call { callee, arguments }
        | Expression::SuperCall { callee, arguments }
        | Expression::New { callee, arguments } => {
            tracked_array_push_expression_references_internal_iterator_step(callee)
                || arguments.iter().any(|argument| {
                    tracked_array_push_expression_references_internal_iterator_step(
                        argument.expression(),
                    )
                })
        }
        Expression::Number(_)
        | Expression::BigInt(_)
        | Expression::String(_)
        | Expression::Bool(_)
        | Expression::Null
        | Expression::Undefined
        | Expression::NewTarget
        | Expression::This
        | Expression::Sent
        | Expression::Update { .. } => false,
    }
}

impl<'a> FunctionCompiler<'a> {
    pub(in crate::backend::direct_wasm) fn emit_tracked_array_push_call(
        &mut self,
        object: &Expression,
        arguments: &[CallArgument],
    ) -> DirectResult<bool> {
        let trace = std::env::var_os("AYY_TRACE_TRACKED_ARRAY_PUSH").is_some();
        let Expression::Identifier(name) = object else {
            if trace {
                eprintln!("tracked_array_push:skip non_identifier object={object:?}");
            }
            return Ok(false);
        };
        let binding_name = if self
            .state
            .speculation
            .static_semantics
            .has_local_array_binding(name)
            || self
                .backend
                .global_semantics
                .values
                .array_bindings
                .contains_key(name)
            || self
                .state
                .speculation
                .static_semantics
                .runtime_array_length_local(name)
                .is_some()
            || self
                .state
                .speculation
                .static_semantics
                .has_runtime_array_slots(name)
        {
            name.clone()
        } else if let Some(hidden_name) = self
            .resolve_user_function_capture_hidden_name(name)
            .filter(|hidden_name| {
                self.state
                    .speculation
                    .static_semantics
                    .has_local_array_binding(hidden_name)
                    || self
                        .backend
                        .global_semantics
                        .values
                        .array_bindings
                        .contains_key(hidden_name)
                    || self
                        .state
                        .speculation
                        .static_semantics
                        .runtime_array_length_local(hidden_name)
                        .is_some()
                    || self
                        .state
                        .speculation
                        .static_semantics
                        .has_runtime_array_slots(hidden_name)
            })
        {
            hidden_name
        } else {
            name.clone()
        };
        if trace {
            eprintln!(
                "tracked_array_push:start name={name} binding={binding_name} args={arguments:?}"
            );
        }
        if !self
            .state
            .speculation
            .static_semantics
            .has_local_array_binding(&binding_name)
            && !self
                .backend
                .global_semantics
                .values
                .array_bindings
                .contains_key(&binding_name)
        {
            if trace {
                eprintln!("tracked_array_push:skip untracked binding={binding_name}");
            }
            return Ok(false);
        }

        let expanded_arguments = self.expand_call_arguments(arguments);
        let materialized_arguments = expanded_arguments
            .iter()
            .map(|argument| self.materialize_static_expression(argument))
            .collect::<Vec<_>>();
        let use_global_runtime_array = self.is_named_global_array_binding(&binding_name)
            && (!self.state.speculation.execution_context.top_level_function
                || self.uses_global_runtime_array_state(&binding_name));
        self.emit_numeric_expression(object)?;
        self.state.emission.output.instructions.push(0x1a);
        let argument_locals = expanded_arguments
            .iter()
            .zip(materialized_arguments.iter())
            .map(|(argument, materialized_argument)| {
                let local = self.allocate_temp_local();
                let argument_is_runtime_iterator_step_member =
                    if let Expression::Member { object, property } = argument {
                        matches!(property.as_ref(), Expression::String(property_name) if property_name == "value")
                            && matches!(
                                self.resolve_iterator_step_binding_from_expression(object),
                                Some(IteratorStepBinding::Runtime { .. })
                            )
                    } else {
                        false
                    };
                let static_step_argument =
                    if binding_name.starts_with("__ayy_array_rest_")
                        && argument_is_runtime_iterator_step_member
                    {
                        None
                    } else {
                        self.resolve_static_iterator_step_assignment_value(argument)
                    };
                let emission_argument = if let Some(static_step_argument) =
                    static_step_argument.as_ref()
                {
                    static_step_argument
                } else if tracked_array_push_expression_references_internal_iterator_step(argument)
                    && !tracked_array_push_expression_references_internal_iterator_step(
                        materialized_argument,
                    )
                {
                    materialized_argument
                } else {
                    argument
                };
                if trace {
                    eprintln!(
                        "tracked_array_push:argument original={argument:?} materialized={materialized_argument:?} emission={emission_argument:?}"
                    );
                }
                self.emit_numeric_expression(emission_argument)?;
                self.push_local_set(local);
                Ok(local)
            })
            .collect::<DirectResult<Vec<_>>>()?;
        for argument_local in &argument_locals {
            self.push_local_get(*argument_local);
            self.state.emission.output.instructions.push(0x1a);
        }
        let mut old_length = None;
        let mut new_length = None;
        let mut synced_array_binding = None;
        if let Some(array_binding) = self
            .state
            .speculation
            .static_semantics
            .local_array_binding_mut(&binding_name)
        {
            old_length = Some(array_binding.values.len() as u32);
            array_binding
                .values
                .extend(materialized_arguments.into_iter().map(Some));
            synced_array_binding = Some(array_binding.clone());
            new_length = Some(array_binding.values.len() as i32);
        } else if let Some(array_binding) = self
            .backend
            .global_semantics
            .values
            .array_bindings
            .get_mut(&binding_name)
        {
            old_length = Some(array_binding.values.len() as u32);
            array_binding
                .values
                .extend(materialized_arguments.into_iter().map(Some));
            synced_array_binding = Some(array_binding.clone());
            new_length = Some(array_binding.values.len() as i32);
        }
        if self.binding_name_is_global(&binding_name) {
            self.backend
                .sync_global_array_binding(&binding_name, synced_array_binding.clone());
        }
        let mut used_runtime_push = false;
        if let Some(old_length) = old_length {
            for (offset, argument_local) in argument_locals.iter().enumerate() {
                if use_global_runtime_array
                    && self
                        .emit_global_runtime_array_push_from_local(&binding_name, *argument_local)?
                {
                    self.update_tracked_array_specialized_function_value(
                        &binding_name,
                        old_length + offset as u32,
                        &expanded_arguments[offset],
                    )?;
                    used_runtime_push = true;
                    if offset + 1 < argument_locals.len() {
                        self.state.emission.output.instructions.push(0x1a);
                    }
                    continue;
                }
                if !use_global_runtime_array
                    && self.emit_runtime_array_push_from_local(
                        &binding_name,
                        *argument_local,
                        &expanded_arguments[offset],
                    )?
                {
                    self.update_tracked_array_specialized_function_value(
                        &binding_name,
                        old_length + offset as u32,
                        &expanded_arguments[offset],
                    )?;
                    used_runtime_push = true;
                    if offset + 1 < argument_locals.len() {
                        self.state.emission.output.instructions.push(0x1a);
                    }
                    continue;
                }
                self.update_tracked_array_specialized_function_value(
                    &binding_name,
                    old_length + offset as u32,
                    &expanded_arguments[offset],
                )?;
                if use_global_runtime_array {
                    if self.emit_global_runtime_array_slot_write_from_local(
                        &binding_name,
                        old_length + offset as u32,
                        *argument_local,
                    )? {
                        self.state.emission.output.instructions.push(0x1a);
                    }
                } else if self.emit_runtime_array_slot_write_from_local(
                    &binding_name,
                    old_length + offset as u32,
                    *argument_local,
                )? {
                    self.state.emission.output.instructions.push(0x1a);
                }
            }
        }
        if used_runtime_push {
            return Ok(true);
        }
        let new_length = new_length.expect("tracked push length should exist");
        if !use_global_runtime_array
            && let Some(length_local) = self
                .state
                .speculation
                .static_semantics
                .runtime_array_length_local(&binding_name)
        {
            self.push_i32_const(new_length);
            self.push_local_set(length_local);
        }
        if use_global_runtime_array {
            self.emit_global_runtime_array_length_write(&binding_name, new_length);
        }
        self.push_i32_const(new_length);
        Ok(true)
    }
}
