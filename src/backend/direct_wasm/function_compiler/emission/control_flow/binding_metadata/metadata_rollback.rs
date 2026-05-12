use super::*;

impl<'a> FunctionCompiler<'a> {
    fn restore_global_metadata_map_entry<T: Clone>(
        map: &mut HashMap<String, T>,
        snapshot: &HashMap<String, T>,
        name: &str,
    ) {
        if let Some(value) = snapshot.get(name) {
            map.insert(name.to_string(), value.clone());
        } else {
            map.remove(name);
        }
    }

    fn restore_global_binding_metadata_for_name_from_snapshot(
        &mut self,
        name: &str,
        snapshot: &GlobalStaticSemanticsSnapshot,
    ) {
        let global_semantics = &mut self.backend.global_semantics;
        Self::restore_global_metadata_map_entry(
            &mut global_semantics.names.kinds,
            &snapshot.names.kinds,
            name,
        );
        Self::restore_global_metadata_map_entry(
            &mut global_semantics.values.value_bindings,
            &snapshot.values.value_bindings,
            name,
        );
        Self::restore_global_metadata_map_entry(
            &mut global_semantics.values.array_bindings,
            &snapshot.values.array_bindings,
            name,
        );
        Self::restore_global_metadata_map_entry(
            &mut global_semantics.values.object_bindings,
            &snapshot.values.object_bindings,
            name,
        );
        Self::restore_global_metadata_map_entry(
            &mut global_semantics.values.property_descriptors,
            &snapshot.values.property_descriptors,
            name,
        );
        Self::restore_global_metadata_map_entry(
            &mut global_semantics.values.object_prototype_bindings,
            &snapshot.values.object_prototype_bindings,
            name,
        );
        Self::restore_global_metadata_map_entry(
            &mut global_semantics.values.runtime_prototype_bindings,
            &snapshot.values.runtime_prototype_bindings,
            name,
        );
        Self::restore_global_metadata_map_entry(
            &mut global_semantics.values.prototype_object_bindings,
            &snapshot.values.prototype_object_bindings,
            name,
        );
        Self::restore_global_metadata_map_entry(
            &mut global_semantics.values.arguments_bindings,
            &snapshot.values.arguments_bindings,
            name,
        );
        Self::restore_global_metadata_map_entry(
            &mut global_semantics.values.proxy_bindings,
            &snapshot.values.proxy_bindings,
            name,
        );
        Self::restore_global_metadata_map_entry(
            &mut global_semantics.functions.function_bindings,
            &snapshot.functions.function_bindings,
            name,
        );
        Self::restore_global_metadata_map_entry(
            &mut global_semantics.functions.specialized_function_values,
            &snapshot.functions.specialized_function_values,
            name,
        );
        if snapshot.values.arrays_with_runtime_state.contains(name) {
            global_semantics
                .values
                .arrays_with_runtime_state
                .insert(name.to_string());
        } else {
            global_semantics
                .values
                .arrays_with_runtime_state
                .remove(name);
        }
    }

    fn local_static_binding_state_from_metadata_snapshot(
        snapshot: &FunctionStaticBindingMetadataSnapshot,
        name: &str,
    ) -> LocalStaticBindingState {
        LocalStaticBindingState {
            value: snapshot.values.local_value_binding(name).cloned(),
            array: snapshot.arrays.local_array_binding(name).cloned(),
            object: snapshot.objects.local_object_binding(name).cloned(),
            kind: snapshot.values.local_kind(name),
        }
    }

    fn merge_branch_array_binding(
        left: &ArrayValueBinding,
        right: &ArrayValueBinding,
    ) -> ArrayValueBinding {
        let len = left.values.len().max(right.values.len());
        let values = (0..len)
            .map(|index| {
                let left_value = left.values.get(index).cloned().flatten();
                let right_value = right.values.get(index).cloned().flatten();
                (left_value == right_value).then_some(left_value).flatten()
            })
            .collect();
        ArrayValueBinding { values }
    }

    pub(in crate::backend::direct_wasm) fn with_restored_static_binding_metadata_snapshot(
        &mut self,
        callback: impl FnOnce(&mut Self) -> DirectResult<()>,
    ) -> DirectResult<FunctionStaticBindingMetadataSnapshot> {
        let transaction = StaticBindingMetadataTransaction::capture(self);

        let result = callback(self);
        let branch_snapshot = self.state.snapshot_static_binding_metadata();

        transaction.restore(self);

        result?;
        Ok(branch_snapshot)
    }

    pub(in crate::backend::direct_wasm) fn with_restored_static_binding_metadata_snapshots(
        &mut self,
        callback: impl FnOnce(&mut Self) -> DirectResult<()>,
    ) -> DirectResult<(
        FunctionStaticBindingMetadataSnapshot,
        GlobalStaticSemanticsSnapshot,
    )> {
        let transaction = StaticBindingMetadataTransaction::capture(self);

        let result = callback(self);
        let local_snapshot = self.state.snapshot_static_binding_metadata();
        let global_snapshot = self.backend.snapshot_global_static_semantics();

        transaction.restore(self);

        result?;
        Ok((local_snapshot, global_snapshot))
    }

    pub(in crate::backend::direct_wasm) fn seed_runtime_array_metadata_for_names_from_snapshot(
        &mut self,
        snapshot: &FunctionStaticBindingMetadataSnapshot,
        names: &HashSet<String>,
    ) {
        for name in names {
            if let Some(length_local) = snapshot.arrays.runtime_array_length_local(name) {
                self.state
                    .speculation
                    .static_semantics
                    .set_runtime_array_length_local(name, length_local);
            }
            if let Some(slots) = snapshot.arrays.runtime_array_slots(name) {
                self.state
                    .speculation
                    .static_semantics
                    .set_runtime_array_slots(name, slots);
            }
            if let Some(values) = snapshot
                .arrays
                .tracked_array_specialized_function_values(name)
            {
                self.state
                    .speculation
                    .static_semantics
                    .set_tracked_array_specialized_function_values(name, values);
            }
        }
    }

    fn restore_runtime_array_metadata_for_name_from_snapshot(
        &mut self,
        snapshot: &FunctionStaticBindingMetadataSnapshot,
        name: &str,
    ) {
        if let Some(length_local) = snapshot.arrays.runtime_array_length_local(name) {
            self.state
                .speculation
                .static_semantics
                .set_runtime_array_length_local(name, length_local);
        } else {
            self.state
                .speculation
                .static_semantics
                .clear_runtime_array_length_local(name);
        }

        if let Some(slots) = snapshot.arrays.runtime_array_slots(name) {
            self.state
                .speculation
                .static_semantics
                .set_runtime_array_slots(name, slots);
        } else {
            self.state
                .speculation
                .static_semantics
                .clear_runtime_array_slots(name);
        }

        if let Some(values) = snapshot
            .arrays
            .tracked_array_specialized_function_values(name)
        {
            self.state
                .speculation
                .static_semantics
                .set_tracked_array_specialized_function_values(name, values);
        } else {
            self.state
                .speculation
                .static_semantics
                .clear_tracked_array_specialized_function_values(name);
        }
    }

    pub(in crate::backend::direct_wasm) fn merge_dynamic_branch_static_binding_metadata(
        &mut self,
        invalidated_bindings: &HashSet<String>,
        base_snapshot: &FunctionStaticBindingMetadataSnapshot,
        then_snapshot: &FunctionStaticBindingMetadataSnapshot,
        else_snapshot: Option<&FunctionStaticBindingMetadataSnapshot>,
    ) {
        for name in invalidated_bindings {
            let then_state =
                Self::local_static_binding_state_from_metadata_snapshot(then_snapshot, name);
            let else_state = else_snapshot
                .map(|snapshot| {
                    Self::local_static_binding_state_from_metadata_snapshot(snapshot, name)
                })
                .unwrap_or_else(|| {
                    Self::local_static_binding_state_from_metadata_snapshot(base_snapshot, name)
                });

            match (then_state.array.as_ref(), else_state.array.as_ref()) {
                (Some(then_array), Some(else_array)) => {
                    self.seed_runtime_array_metadata_for_names_from_snapshot(
                        then_snapshot,
                        &HashSet::from([name.clone()]),
                    );
                    let array_binding = Self::merge_branch_array_binding(then_array, else_array);
                    self.state
                        .speculation
                        .static_semantics
                        .set_local_array_binding(name, array_binding);
                    self.state
                        .speculation
                        .static_semantics
                        .set_local_kind(name, StaticValueKind::Object);
                }
                _ => {
                    self.restore_runtime_array_metadata_for_name_from_snapshot(base_snapshot, name);
                    if then_state.kind == else_state.kind
                        && let Some(kind) = then_state
                            .kind
                            .filter(|kind| *kind != StaticValueKind::Unknown)
                    {
                        self.state
                            .speculation
                            .static_semantics
                            .set_local_kind(name, kind);
                    }
                }
            }

            if then_state.value == else_state.value
                && let Some(value) = then_state.value
            {
                self.state
                    .speculation
                    .static_semantics
                    .set_local_value_binding(name, value);
            }
            if then_state.object == else_state.object
                && let Some(object) = then_state.object
            {
                self.state
                    .speculation
                    .static_semantics
                    .set_local_object_binding(name, object);
            }
        }
    }

    pub(in crate::backend::direct_wasm) fn merge_dynamic_branch_global_static_binding_metadata(
        &mut self,
        invalidated_bindings: &HashSet<String>,
        base_snapshot: &GlobalStaticSemanticsSnapshot,
        then_snapshot: &GlobalStaticSemanticsSnapshot,
        else_snapshot: Option<&GlobalStaticSemanticsSnapshot>,
    ) {
        let trace_static_if = std::env::var_os("AYY_TRACE_STATIC_IF").is_some();
        for name in invalidated_bindings {
            let else_snapshot = else_snapshot.unwrap_or(base_snapshot);
            let then_runtime_array = then_snapshot
                .values
                .arrays_with_runtime_state
                .contains(name);
            let else_runtime_array = else_snapshot
                .values
                .arrays_with_runtime_state
                .contains(name);
            if trace_static_if {
                eprintln!(
                    "dynamic_if:global_merge name={name} then_runtime_array={then_runtime_array} else_runtime_array={else_runtime_array} then_array={} else_array={}",
                    then_snapshot.values.array_bindings.contains_key(name),
                    else_snapshot.values.array_bindings.contains_key(name)
                );
            }
            if then_runtime_array && else_runtime_array {
                self.backend.mark_global_array_with_runtime_state(name);
            }

            if let (Some(then_array), Some(else_array)) = (
                then_snapshot.values.array_bindings.get(name),
                else_snapshot.values.array_bindings.get(name),
            ) {
                let array_binding = Self::merge_branch_array_binding(then_array, else_array);
                self.backend
                    .sync_global_array_binding(name, Some(array_binding));
            }
        }
    }

    pub(in crate::backend::direct_wasm) fn conditional_defined_binding_narrowing(
        &self,
        condition: &Expression,
        then_branch: bool,
    ) -> Option<(String, Expression)> {
        let (name, defined_when_condition_true) = match condition {
            Expression::Binary {
                op: BinaryOp::NotEqual,
                left,
                right,
            } if matches!(right.as_ref(), Expression::Undefined) => {
                let Expression::Identifier(name) = left.as_ref() else {
                    return None;
                };
                (name.clone(), true)
            }
            Expression::Binary {
                op: BinaryOp::NotEqual,
                left,
                right,
            } if matches!(left.as_ref(), Expression::Undefined) => {
                let Expression::Identifier(name) = right.as_ref() else {
                    return None;
                };
                (name.clone(), true)
            }
            Expression::Binary {
                op: BinaryOp::Equal,
                left,
                right,
            } if matches!(right.as_ref(), Expression::Undefined) => {
                let Expression::Identifier(name) = left.as_ref() else {
                    return None;
                };
                (name.clone(), false)
            }
            Expression::Binary {
                op: BinaryOp::Equal,
                left,
                right,
            } if matches!(left.as_ref(), Expression::Undefined) => {
                let Expression::Identifier(name) = right.as_ref() else {
                    return None;
                };
                (name.clone(), false)
            }
            _ => return None,
        };

        let Expression::Conditional {
            then_expression,
            else_expression,
            ..
        } = self
            .state
            .speculation
            .static_semantics
            .local_value_binding(&name)
            .or_else(|| self.global_value_binding(&name))?
        else {
            return None;
        };

        let then_is_undefined = matches!(then_expression.as_ref(), Expression::Undefined);
        let else_is_undefined = matches!(else_expression.as_ref(), Expression::Undefined);
        if then_is_undefined == else_is_undefined {
            return None;
        }

        let defined_expression = if !then_is_undefined {
            then_expression.as_ref().clone()
        } else {
            else_expression.as_ref().clone()
        };
        let branch_expression = if then_branch == defined_when_condition_true {
            defined_expression
        } else {
            Expression::Undefined
        };
        Some((name, branch_expression))
    }

    pub(in crate::backend::direct_wasm) fn with_restored_static_binding_metadata<T>(
        &mut self,
        callback: impl FnOnce(&mut Self) -> DirectResult<T>,
    ) -> DirectResult<T> {
        let transaction = StaticBindingMetadataTransaction::capture(self);

        let result = callback(self);

        transaction.restore(self);

        result
    }

    pub(in crate::backend::direct_wasm) fn with_restored_function_static_binding_metadata<T>(
        &mut self,
        callback: impl FnOnce(&mut Self) -> DirectResult<T>,
    ) -> DirectResult<T> {
        let transaction = FunctionStaticBindingMetadataTransaction::capture(&self.state);

        let result = callback(self);

        transaction.restore(&mut self.state);

        result
    }

    pub(in crate::backend::direct_wasm) fn with_narrowed_local_binding_metadata<T>(
        &mut self,
        name: &str,
        expression: &Expression,
        callback: impl FnOnce(&mut Self) -> DirectResult<T>,
    ) -> DirectResult<T> {
        if self.resolve_current_local_binding(name).is_none()
            && (self.global_value_binding(name).is_some()
                || self.global_has_binding(name)
                || self.global_has_implicit_binding(name))
        {
            let saved_global_semantics = self.backend.snapshot_global_static_semantics();
            self.update_static_global_assignment_metadata(name, expression);

            let result = callback(self);

            self.restore_global_binding_metadata_for_name_from_snapshot(
                name,
                &saved_global_semantics,
            );

            return result;
        }

        let saved_binding = self.state.snapshot_local_static_binding(name);
        let array_binding = self.resolve_array_binding_from_expression(expression);
        let object_binding = self.resolve_object_binding_from_expression(expression);
        let kind = self.infer_value_kind(expression);
        self.state.set_local_static_binding(
            name,
            expression.clone(),
            array_binding,
            object_binding,
            kind,
        );

        let result = callback(self);

        self.state.restore_local_static_binding(saved_binding);

        result
    }

    pub(in crate::backend::direct_wasm) fn invalidate_static_binding_metadata_for_names(
        &mut self,
        names: &HashSet<String>,
    ) {
        for name in names {
            self.clear_static_identifier_binding_metadata(name);
        }
    }

    pub(in crate::backend::direct_wasm) fn invalidate_static_binding_metadata_for_names_with_preserved_kinds(
        &mut self,
        names: &HashSet<String>,
        preserved_kinds: &HashMap<String, StaticValueKind>,
    ) {
        self.invalidate_static_binding_metadata_for_names(names);
        for (name, kind) in preserved_kinds {
            if let Some((resolved_name, _)) = self.resolve_current_local_binding(name) {
                self.state
                    .speculation
                    .static_semantics
                    .set_local_kind(&resolved_name, *kind);
            } else if self.state.runtime.locals.bindings.contains_key(name)
                || self.parameter_scope_arguments_local_for(name).is_some()
            {
                self.state
                    .speculation
                    .static_semantics
                    .set_local_kind(name, *kind);
            } else if let Some(hidden_name) = self.resolve_user_function_capture_hidden_name(name) {
                self.backend.set_global_binding_kind(&hidden_name, *kind);
            } else if self.binding_name_is_global(name) || self.backend.global_has_binding(name) {
                self.backend.set_global_binding_kind(name, *kind);
            } else {
                self.state
                    .speculation
                    .static_semantics
                    .set_local_kind(name, *kind);
            }
        }
    }
}
