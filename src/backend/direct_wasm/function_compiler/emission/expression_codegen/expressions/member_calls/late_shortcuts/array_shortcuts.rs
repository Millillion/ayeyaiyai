use super::*;

impl<'a> FunctionCompiler<'a> {
    fn tracked_array_binding_name_for_call(&self, object: &Expression) -> Option<String> {
        let Expression::Identifier(name) = object else {
            return None;
        };
        if self
            .state
            .speculation
            .static_semantics
            .has_local_array_binding(name)
            || self.backend.global_array_binding(name).is_some()
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
            return Some(name.clone());
        }
        self.resolve_user_function_capture_hidden_name(name)
            .filter(|hidden_name| {
                self.state
                    .speculation
                    .static_semantics
                    .has_local_array_binding(hidden_name)
                    || self.backend.global_array_binding(hidden_name).is_some()
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
    }

    fn resolve_static_sort_comparator_order(
        &self,
        comparator: Option<&Expression>,
        left: &Expression,
        right: &Expression,
    ) -> Option<std::cmp::Ordering> {
        let Some(comparator) = comparator else {
            let left =
                self.resolve_static_string_concat_value(left, self.current_function_name())?;
            let right =
                self.resolve_static_string_concat_value(right, self.current_function_name())?;
            return Some(left.cmp(&right));
        };
        let LocalFunctionBinding::User(function_name) =
            self.resolve_function_binding_from_expression(comparator)?
        else {
            return None;
        };
        let result = self.resolve_static_return_expression_from_user_function_call(
            &function_name,
            &[
                CallArgument::Expression(left.clone()),
                CallArgument::Expression(right.clone()),
            ],
            None,
        )?;
        let result = self.resolve_static_number_value(&result)?;
        if result < 0.0 {
            Some(std::cmp::Ordering::Less)
        } else if result > 0.0 {
            Some(std::cmp::Ordering::Greater)
        } else {
            Some(std::cmp::Ordering::Equal)
        }
    }

    fn sorted_static_array_binding(
        &self,
        array_binding: &ArrayValueBinding,
        comparator: Option<&Expression>,
    ) -> Option<ArrayValueBinding> {
        let mut sorted = Vec::<Expression>::new();
        let mut hole_count = 0usize;
        for value in &array_binding.values {
            let Some(value) = value.clone() else {
                hole_count += 1;
                continue;
            };
            let mut position = sorted.len();
            while position > 0 {
                let order = self.resolve_static_sort_comparator_order(
                    comparator,
                    &value,
                    &sorted[position - 1],
                )?;
                if order == std::cmp::Ordering::Less {
                    position -= 1;
                } else {
                    break;
                }
            }
            sorted.insert(position, value);
        }
        let mut values = sorted.into_iter().map(Some).collect::<Vec<_>>();
        values.extend(std::iter::repeat_n(None, hole_count));
        Some(ArrayValueBinding { values })
    }

    fn emit_tracked_array_sort_call(
        &mut self,
        object: &Expression,
        arguments: &[CallArgument],
    ) -> DirectResult<bool> {
        let Some(binding_name) = self.tracked_array_binding_name_for_call(object) else {
            return Ok(false);
        };
        let Some(array_binding) = self
            .state
            .speculation
            .static_semantics
            .local_array_binding(&binding_name)
            .cloned()
            .or_else(|| self.backend.global_array_binding(&binding_name).cloned())
        else {
            return Ok(false);
        };
        let comparator = arguments.first().map(CallArgument::expression);
        let Some(sorted_binding) = self.sorted_static_array_binding(&array_binding, comparator)
        else {
            return Ok(false);
        };

        self.emit_numeric_expression(object)?;
        self.state.emission.output.instructions.push(0x1a);
        for argument in arguments {
            self.emit_numeric_expression(argument.expression())?;
            self.state.emission.output.instructions.push(0x1a);
        }

        if self
            .state
            .speculation
            .static_semantics
            .has_local_array_binding(&binding_name)
        {
            self.state
                .speculation
                .static_semantics
                .set_local_array_binding(&binding_name, sorted_binding.clone());
        }
        if self.backend.global_array_binding(&binding_name).is_some() {
            self.backend
                .sync_global_array_binding(&binding_name, Some(sorted_binding.clone()));
        }

        let use_global_runtime_array = self.is_named_global_array_binding(&binding_name)
            && (!self.state.speculation.execution_context.top_level_function
                || self.uses_global_runtime_array_state(&binding_name));
        if use_global_runtime_array {
            self.emit_sync_global_runtime_array_state_from_binding(&binding_name, &sorted_binding)?;
        } else {
            if let Some(length_local) = self
                .state
                .speculation
                .static_semantics
                .runtime_array_length_local(&binding_name)
            {
                self.push_i32_const(sorted_binding.values.len() as i32);
                self.push_local_set(length_local);
            }
            if self
                .state
                .speculation
                .static_semantics
                .has_runtime_array_slots(&binding_name)
            {
                self.ensure_runtime_array_slots_for_binding(&binding_name, &sorted_binding);
            }
        }

        self.emit_numeric_expression(object)?;
        Ok(true)
    }

    pub(super) fn emit_array_member_call_shortcuts(
        &mut self,
        object: &Expression,
        property: &Expression,
        arguments: &[CallArgument],
    ) -> DirectResult<bool> {
        if matches!(property, Expression::String(property_name) if property_name == "indexOf")
            && let [CallArgument::Expression(search_expression)] = arguments
            && let Some(array_binding) = self.resolve_array_binding_from_expression(object)
        {
            self.emit_numeric_expression(object)?;
            self.state.emission.output.instructions.push(0x1a);
            self.emit_numeric_expression(search_expression)?;
            self.state.emission.output.instructions.push(0x1a);

            let search_value = self.materialize_static_expression(search_expression);
            let found_index = array_binding
                .values
                .iter()
                .enumerate()
                .find_map(|(index, value)| {
                    let value = value.as_ref()?;
                    let value = self.materialize_static_expression(value);
                    static_expression_matches(&value, &search_value).then_some(index as i32)
                })
                .unwrap_or(-1);
            self.push_i32_const(found_index);
            return Ok(true);
        }
        if matches!(property, Expression::String(property_name) if property_name == "push")
            && self.emit_tracked_array_push_call(object, arguments)?
        {
            return Ok(true);
        }
        if matches!(property, Expression::String(property_name) if property_name == "sort")
            && self.emit_tracked_array_sort_call(object, arguments)?
        {
            return Ok(true);
        }
        if matches!(property, Expression::String(property_name) if property_name == "pop")
            && let Expression::Identifier(name) = object
        {
            self.emit_numeric_expression(object)?;
            self.state.emission.output.instructions.push(0x1a);
            let length_local = self
                .state
                .speculation
                .static_semantics
                .runtime_array_length_local(name);
            let use_global_runtime_array = self.is_named_global_array_binding(name)
                && (!self.state.speculation.execution_context.top_level_function
                    || self.uses_global_runtime_array_state(name));
            let mut popped_value = None;
            let mut popped_index = None;
            let mut new_length = None;
            let mut synced_array_binding = None;
            if let Some(array_binding) = self
                .state
                .speculation
                .static_semantics
                .local_array_binding_mut(name)
            {
                popped_index = array_binding
                    .values
                    .len()
                    .checked_sub(1)
                    .map(|index| index as u32);
                popped_value = Some(
                    array_binding
                        .values
                        .pop()
                        .flatten()
                        .unwrap_or(Expression::Undefined),
                );
                synced_array_binding = Some(array_binding.clone());
                new_length = Some(array_binding.values.len() as i32);
            } else if let Some(array_binding) = self
                .backend
                .global_semantics
                .values
                .array_bindings
                .get_mut(name)
            {
                popped_index = array_binding
                    .values
                    .len()
                    .checked_sub(1)
                    .map(|index| index as u32);
                popped_value = Some(
                    array_binding
                        .values
                        .pop()
                        .flatten()
                        .unwrap_or(Expression::Undefined),
                );
                synced_array_binding = Some(array_binding.clone());
                new_length = Some(array_binding.values.len() as i32);
            }
            if self.binding_name_is_global(name) {
                self.backend
                    .sync_global_array_binding(name, synced_array_binding.clone());
            }
            if let Some(popped_index) = popped_index {
                if use_global_runtime_array {
                    self.clear_global_runtime_array_slot(name, popped_index);
                } else {
                    self.clear_runtime_array_slot(name, popped_index);
                }
            }
            if let Some(new_length) = new_length {
                if !use_global_runtime_array && let Some(length_local) = length_local {
                    self.push_i32_const(new_length);
                    self.push_local_set(length_local);
                }
                if use_global_runtime_array {
                    self.emit_global_runtime_array_length_write(name, new_length);
                }
                self.emit_numeric_expression(
                    &popped_value.expect("tracked pop value should exist"),
                )?;
                return Ok(true);
            }
        }
        Ok(false)
    }
}
