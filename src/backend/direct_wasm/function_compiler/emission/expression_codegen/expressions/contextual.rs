use super::*;

fn is_internal_assignment_temp(name: &str) -> bool {
    name.starts_with("__ayy_optional_base_")
        || name.starts_with("__ayy_target_object_")
        || name.starts_with("__ayy_target_property_")
        || name.starts_with("__ayy_postfix_previous_")
}

fn is_internal_target_property_temp(name: &str) -> bool {
    name.starts_with("__ayy_target_property_")
}

#[derive(Clone, Copy)]
enum TemplateObjectMemberRead {
    Index(u32),
    Length,
    RawArray,
    AbsentFrozenOwnProperty,
}

impl<'a> FunctionCompiler<'a> {
    fn test262_realm_global_constructor_property_name(property: &Expression) -> Option<&str> {
        let Expression::String(name) = property else {
            return None;
        };
        matches!(
            name.as_str(),
            "Object"
                | "Function"
                | "Array"
                | "ArrayBuffer"
                | "SharedArrayBuffer"
                | "DataView"
                | "Date"
                | "RegExp"
                | "Map"
                | "Set"
                | "WeakMap"
                | "WeakRef"
                | "WeakSet"
                | "Number"
                | "String"
                | "Boolean"
                | "Promise"
                | "Uint8Array"
                | "Int8Array"
                | "Uint16Array"
                | "Int16Array"
                | "Uint32Array"
                | "Int32Array"
                | "Float32Array"
                | "Float64Array"
                | "Uint8ClampedArray"
                | "BigInt64Array"
                | "BigUint64Array"
                | "Error"
                | "EvalError"
                | "RangeError"
                | "ReferenceError"
                | "SyntaxError"
                | "TypeError"
                | "URIError"
                | "AggregateError"
        )
        .then_some(name.as_str())
    }

    fn expression_is_test262_realm_global_constructor_member(
        &self,
        expression: &Expression,
    ) -> bool {
        let Expression::Member { object, property } = expression else {
            return false;
        };
        self.resolve_test262_realm_global_id_from_expression(object)
            .is_some()
            && Self::test262_realm_global_constructor_property_name(property).is_some()
    }

    fn emit_test262_realm_global_constructor_member_value(
        &mut self,
        object: &Expression,
        property: &Expression,
    ) -> DirectResult<bool> {
        if self
            .resolve_test262_realm_global_id_from_expression(object)
            .is_some()
            && Self::test262_realm_global_constructor_property_name(property).is_some()
        {
            self.push_i32_const(JS_TYPEOF_FUNCTION_TAG);
            return Ok(true);
        }

        if matches!(property, Expression::String(name) if name == "prototype")
            && self.expression_is_test262_realm_global_constructor_member(object)
        {
            self.push_i32_const(JS_TYPEOF_OBJECT_TAG);
            return Ok(true);
        }

        Ok(false)
    }

    fn emit_internal_global_read(&mut self, name: &str) -> DirectResult<()> {
        if let Some(global_index) = self.backend.global_binding_index(name) {
            self.push_global_get(global_index);
        } else {
            self.emit_numeric_expression(&Expression::Identifier(name.to_string()))?;
        }
        Ok(())
    }

    fn invalidate_static_module_init_effects(&mut self, user_function: &UserFunction) {
        let mut invalidated = self.collect_user_function_assigned_nonlocal_bindings(user_function);
        invalidated.extend(self.collect_user_function_updated_nonlocal_bindings(user_function));
        invalidated.extend(self.collect_user_function_call_effect_nonlocal_bindings(user_function));
        let runtime_array_effects = invalidated
            .iter()
            .filter(|name| {
                self.global_array_binding(name).is_some()
                    || self.uses_global_runtime_array_state(name)
                    || name.starts_with("__ayy_object_property__")
            })
            .cloned()
            .collect::<Vec<_>>();
        for name in &runtime_array_effects {
            self.backend.mark_global_array_with_runtime_state(name);
            self.backend
                .shared_global_semantics
                .values
                .mark_array_with_runtime_state(name);
        }
        if !invalidated.is_empty() {
            self.invalidate_static_binding_metadata_for_names(&invalidated);
        }
        for name in runtime_array_effects {
            self.backend.mark_global_array_with_runtime_state(&name);
            self.backend
                .shared_global_semantics
                .values
                .mark_array_with_runtime_state(&name);
        }
    }

    fn mark_module_init_runtime_array_effects(&mut self, user_function: &UserFunction) {
        let call_effects = self.collect_user_function_call_effect_nonlocal_bindings(user_function);
        for name in call_effects {
            if self.global_array_binding(&name).is_some()
                || self.uses_global_runtime_array_state(&name)
                || name.starts_with("__ayy_object_property__")
            {
                self.backend.mark_global_array_with_runtime_state(&name);
                self.backend
                    .shared_global_semantics
                    .values
                    .mark_array_with_runtime_state(&name);
            }
        }
    }

    fn sync_static_module_init_effects(&mut self, module_index: usize) {
        let init_name = format!("__ayy_module_init_{module_index}");
        if self.current_function_name() == Some(init_name.as_str()) {
            return;
        }
        let Some(init_function) = self.user_function(&init_name).cloned() else {
            return;
        };
        self.mark_module_init_runtime_array_effects(&init_function);

        let mut init_arguments = vec![CallArgument::Expression(Expression::Identifier(format!(
            "__ayy_module_namespace_{module_index}"
        )))];
        for parameter in init_function.params.iter().skip(1) {
            let Some(dependency_index) = parameter.strip_prefix("__ayy_module_dep_") else {
                continue;
            };
            init_arguments.push(CallArgument::Expression(Expression::Identifier(format!(
                "__ayy_module_namespace_{dependency_index}"
            ))));
        }

        if let Some(mut execution) = self.prepare_static_user_function_execution(
            &init_name,
            &init_function,
            &init_arguments,
            &Expression::Undefined,
            None,
            HashMap::new(),
            |statement| statement,
        ) && self
            .execute_static_statements_with_state(
                &execution.substituted_body,
                &mut execution.environment,
            )
            .is_some()
        {
            self.sync_static_resolution_environment_overrides(&execution.environment);
            return;
        }

        self.invalidate_static_module_init_effects(&init_function);
    }

    pub(in crate::backend::direct_wasm) fn module_index_from_namespace_like_identifier(
        name: &str,
    ) -> Option<usize> {
        let suffix = name
            .strip_prefix("__ayy_module_dep_")
            .or_else(|| name.strip_prefix("__ayy_module_namespace_"))
            .or_else(|| name.strip_prefix("__ayy_module_deferred_namespace_"))
            .or_else(|| {
                name.rsplit_once("__ayy_module_dep_")
                    .map(|(_, suffix)| suffix)
            })
            .or_else(|| {
                name.rsplit_once("__ayy_module_namespace_")
                    .map(|(_, suffix)| suffix)
            })
            .or_else(|| {
                name.rsplit_once("__ayy_module_deferred_namespace_")
                    .map(|(_, suffix)| suffix)
            })?;
        let digit_count = suffix
            .bytes()
            .take_while(|byte| byte.is_ascii_digit())
            .count();
        if digit_count == 0 {
            return None;
        }
        suffix[..digit_count].parse::<usize>().ok()
    }

    fn module_index_from_deferred_namespace_like_identifier(name: &str) -> Option<usize> {
        let suffix = name
            .strip_prefix("__ayy_module_deferred_namespace_")
            .or_else(|| {
                name.rsplit_once("__ayy_module_deferred_namespace_")
                    .map(|(_, suffix)| suffix)
            })?;
        let digit_count = suffix
            .bytes()
            .take_while(|byte| byte.is_ascii_digit())
            .count();
        if digit_count == 0 {
            return None;
        }
        suffix[..digit_count].parse::<usize>().ok()
    }

    fn deferred_module_namespace_candidate_module_index(&self, name: &str) -> Option<usize> {
        let module_index = Self::module_index_from_deferred_namespace_like_identifier(name)
            .or_else(|| {
                self.runtime_object_property_shadow_owner_name_for_identifier(name)
                    .and_then(|owner| {
                        Self::module_index_from_deferred_namespace_like_identifier(&owner)
                    })
            })?;
        if self.current_function_name().is_some_and(|function_name| {
            function_name == format!("__ayy_module_init_{module_index}")
        }) {
            return None;
        }
        Some(module_index)
    }

    fn module_init_dependency_indices(&self, module_index: usize) -> Option<Vec<usize>> {
        let init_name = format!("__ayy_module_init_{module_index}");
        let init_function = self.user_function(&init_name)?;
        Some(
            init_function
                .params
                .iter()
                .skip(1)
                .filter_map(|parameter| {
                    parameter
                        .strip_prefix("__ayy_module_dep_")
                        .and_then(|index| index.parse::<usize>().ok())
                })
                .filter(|dependency_index| {
                    self.backend
                        .global_binding_index(&format!(
                            "__ayy_module_eager_dependency_{module_index}_{dependency_index}"
                        ))
                        .is_some()
                })
                .collect(),
        )
    }

    fn module_requested_dependency_indices(&self, module_index: usize) -> Option<Vec<usize>> {
        let init_name = format!("__ayy_module_init_{module_index}");
        let init_function = self.user_function(&init_name)?;
        Some(
            init_function
                .params
                .iter()
                .skip(1)
                .filter_map(|parameter| {
                    parameter
                        .strip_prefix("__ayy_module_dep_")
                        .and_then(|index| index.parse::<usize>().ok())
                })
                .collect(),
        )
    }

    fn module_init_is_sync(&self, module_index: usize) -> bool {
        let init_name = format!("__ayy_module_init_{module_index}");
        self.user_function(&init_name)
            .is_some_and(|init_function| !init_function.kind.is_async())
    }

    fn emit_module_ready_for_sync_execution(
        &mut self,
        module_index: usize,
        seen: &mut HashSet<usize>,
    ) -> DirectResult<()> {
        if !seen.insert(module_index) {
            self.push_i32_const(1);
            return Ok(());
        }

        let status_name = format!("__ayy_module_status_{module_index}");
        self.emit_internal_global_read(&status_name)?;
        self.push_i32_const(2);
        self.push_binary_op(BinaryOp::Equal)?;
        self.state.emission.output.instructions.push(0x04);
        self.state.emission.output.instructions.push(I32_TYPE);
        self.push_control_frame();
        self.push_i32_const(1);
        self.state.emission.output.instructions.push(0x05);
        self.emit_internal_global_read(&status_name)?;
        self.push_i32_const(0);
        self.push_binary_op(BinaryOp::Equal)?;
        self.state.emission.output.instructions.push(0x04);
        self.state.emission.output.instructions.push(I32_TYPE);
        self.push_control_frame();
        if self.module_init_is_sync(module_index) {
            let dependency_indices = self
                .module_requested_dependency_indices(module_index)
                .unwrap_or_default();
            let ready_local = self.allocate_temp_local();
            self.push_i32_const(1);
            self.push_local_set(ready_local);
            for dependency_index in dependency_indices {
                self.push_local_get(ready_local);
                self.emit_module_ready_for_sync_execution(dependency_index, seen)?;
                self.push_binary_op(BinaryOp::BitwiseAnd)?;
                self.push_local_set(ready_local);
            }
            self.push_local_get(ready_local);
        } else {
            self.push_i32_const(0);
        }
        self.state.emission.output.instructions.push(0x05);
        self.push_i32_const(0);
        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();
        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();
        Ok(())
    }

    fn emit_module_cached_error_throw(
        &mut self,
        module_index: usize,
        branch_value: Option<i32>,
    ) -> DirectResult<()> {
        self.emit_internal_global_read(&format!("__ayy_module_error_{module_index}"))?;
        self.push_local_set(self.state.runtime.throws.throw_value_local);
        self.push_i32_const(1);
        self.push_local_set(self.state.runtime.throws.throw_tag_local);
        if branch_value.is_some() && !self.state.emission.control_flow.try_stack.is_empty() {
            self.push_local_get(self.state.runtime.throws.throw_value_local);
            self.push_global_set(THROW_VALUE_GLOBAL_INDEX);
            self.push_local_get(self.state.runtime.throws.throw_tag_local);
            self.push_global_set(THROW_TAG_GLOBAL_INDEX);
            if let Some(value) = branch_value {
                self.push_i32_const(value);
            }
            let catch_target = self
                .state
                .emission
                .control_flow
                .try_stack
                .last()
                .map(|try_context| try_context.catch_target)
                .unwrap_or_default();
            self.push_br(self.relative_depth(catch_target));
            Ok(())
        } else {
            self.emit_throw_from_locals()
        }
    }

    pub(in crate::backend::direct_wasm) fn emit_sync_module_init_if_needed(
        &mut self,
        module_index: usize,
        seen: &mut HashSet<usize>,
    ) -> DirectResult<()> {
        if !seen.insert(module_index) {
            return Ok(());
        }

        let status_name = format!("__ayy_module_status_{module_index}");
        self.emit_internal_global_read(&status_name)?;
        self.push_i32_const(2);
        self.push_binary_op(BinaryOp::Equal)?;
        self.state.emission.output.instructions.push(0x04);
        self.state
            .emission
            .output
            .instructions
            .push(EMPTY_BLOCK_TYPE);
        self.push_control_frame();
        self.state.emission.output.instructions.push(0x05);
        self.emit_internal_global_read(&status_name)?;
        self.push_i32_const(0);
        self.push_binary_op(BinaryOp::Equal)?;
        self.state.emission.output.instructions.push(0x04);
        self.state
            .emission
            .output
            .instructions
            .push(EMPTY_BLOCK_TYPE);
        self.push_control_frame();

        let init_name = format!("__ayy_module_init_{module_index}");
        if let Some(init_function) = self.user_function(&init_name).cloned()
            && !init_function.kind.is_async()
        {
            self.emit_module_ready_for_sync_execution(module_index, &mut HashSet::new())?;
            self.push_i32_const(0);
            self.push_binary_op(BinaryOp::Equal)?;
            self.state.emission.output.instructions.push(0x04);
            self.state
                .emission
                .output
                .instructions
                .push(EMPTY_BLOCK_TYPE);
            self.push_control_frame();
            self.emit_named_error_throw("TypeError")?;
            self.state.emission.output.instructions.push(0x0b);
            self.pop_control_frame();

            let dependency_indices = self
                .module_init_dependency_indices(module_index)
                .unwrap_or_default();
            for dependency_index in dependency_indices {
                self.emit_sync_module_init_if_needed(dependency_index, seen)?;
            }
            if let Some(init_declaration) = self
                .resolve_registered_function_declaration(&init_name)
                .cloned()
            {
                self.emit_sync_module_live_binding_initializers(&init_declaration)?;
            }

            let mut init_arguments = vec![CallArgument::Expression(Expression::Identifier(
                format!("__ayy_module_namespace_{module_index}"),
            ))];
            for parameter in init_function.params.iter().skip(1) {
                let Some(dependency_index) = parameter.strip_prefix("__ayy_module_dep_") else {
                    continue;
                };
                init_arguments.push(CallArgument::Expression(Expression::Identifier(format!(
                    "__ayy_module_namespace_{dependency_index}"
                ))));
            }
            self.emit_numeric_expression(&Expression::Call {
                callee: Box::new(Expression::Identifier(init_name)),
                arguments: init_arguments,
            })?;
            self.state.emission.output.instructions.push(0x1a);
            self.sync_static_module_init_effects(module_index);
        } else {
            self.emit_named_error_throw("TypeError")?;
        }

        self.state.emission.output.instructions.push(0x05);
        self.emit_internal_global_read(&status_name)?;
        self.push_i32_const(3);
        self.push_binary_op(BinaryOp::Equal)?;
        self.state.emission.output.instructions.push(0x04);
        self.state
            .emission
            .output
            .instructions
            .push(EMPTY_BLOCK_TYPE);
        self.push_control_frame();
        self.emit_module_cached_error_throw(module_index, None)?;
        self.state.emission.output.instructions.push(0x05);
        self.emit_named_error_throw("TypeError")?;
        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();
        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();
        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();
        Ok(())
    }

    fn emit_sync_module_live_binding_initializers(
        &mut self,
        init_function: &FunctionDeclaration,
    ) -> DirectResult<()> {
        let live_initializers = self.static_dynamic_import_live_binding_initializers(init_function);
        for (hidden_name, initial_value) in live_initializers {
            let binding = self.ensure_implicit_global_binding(&hidden_name);
            self.push_global_get(binding.present_index);
            self.state.emission.output.instructions.push(0x45);
            self.state.emission.output.instructions.push(0x04);
            self.state
                .emission
                .output
                .instructions
                .push(EMPTY_BLOCK_TYPE);
            self.push_control_frame();
            self.emit_numeric_expression(&initial_value)?;
            self.push_global_set(binding.value_index);
            self.push_i32_const(1);
            self.push_global_set(binding.present_index);
            self.state.emission.output.instructions.push(0x0b);
            self.pop_control_frame();
            self.update_static_global_assignment_metadata(&hidden_name, &initial_value);
            self.update_global_specialized_function_value(&hidden_name, &initial_value)?;
        }
        Ok(())
    }

    fn object_has_static_own_property_or_descriptor(
        &self,
        object: &Expression,
        property: &Expression,
    ) -> bool {
        let canonical_property = self.canonical_object_property_expression(property);
        self.resolve_object_binding_from_expression(object)
            .is_some_and(|binding| {
                object_binding_lookup_value(&binding, &canonical_property).is_some()
                    || object_binding_lookup_value(&binding, property).is_some()
                    || object_binding_lookup_descriptor(&binding, &canonical_property).is_some()
                    || object_binding_lookup_descriptor(&binding, property).is_some()
            })
    }

    fn deferred_module_namespace_get_property_key(
        &self,
        property: &Expression,
    ) -> Option<Expression> {
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
        if is_symbol_to_string_tag_expression(&property_key) {
            return None;
        }
        let property_name = static_property_name_from_expression(&property_key)?;
        if property_name == "then" || property_name.starts_with("__ayy$") {
            return None;
        }
        Some(property_key)
    }

    pub(in crate::backend::direct_wasm) fn deferred_module_namespace_materialized_object_module_index(
        &self,
        object: &Expression,
    ) -> Option<usize> {
        if let Expression::Member {
            object: base,
            property,
        } = object
        {
            for value in [
                self.resolve_module_namespace_live_binding_member_raw_value(base, property),
                self.resolve_module_namespace_live_binding_member_value(base, property),
            ]
            .into_iter()
            .flatten()
            {
                let materialized_value = self.materialize_static_expression(&value);
                for candidate in [&value, &materialized_value] {
                    if let Expression::Identifier(name) = candidate
                        && let Some(module_index) =
                            self.deferred_module_namespace_candidate_module_index(name)
                    {
                        return Some(module_index);
                    }
                }
            }
        }

        let materialized_object = self.materialize_static_expression(object);
        for candidate in [object, &materialized_object] {
            let Expression::Identifier(name) = candidate else {
                continue;
            };
            let Some(module_index) = self.deferred_module_namespace_candidate_module_index(name)
            else {
                continue;
            };
            return Some(module_index);
        }
        None
    }

    pub(in crate::backend::direct_wasm) fn deferred_module_namespace_materialized_member_access(
        &self,
        object: &Expression,
        property: &Expression,
    ) -> Option<(usize, Expression)> {
        let property_key = self.deferred_module_namespace_get_property_key(property)?;
        let module_index =
            self.deferred_module_namespace_materialized_object_module_index(object)?;
        Some((module_index, property_key))
    }

    pub(in crate::backend::direct_wasm) fn module_namespace_live_value_is_readable_in_current_context(
        &self,
        value: &Expression,
    ) -> bool {
        let Expression::Identifier(name) = value else {
            return true;
        };
        let accessible = self.resolve_current_local_binding(name).is_some()
            || self.backend.global_binding_index(name).is_some()
            || self.backend.global_has_implicit_binding(name)
            || self.hidden_implicit_global_binding(name).is_some();
        if name.starts_with("__ayy_capture_binding__") {
            return accessible;
        }
        accessible || self.lookup_identifier_kind(name).is_some()
    }

    fn emit_deferred_module_namespace_member_value_after_eval(
        &mut self,
        module_index: usize,
        property: &Expression,
    ) -> DirectResult<()> {
        let live_value = self.resolve_static_dynamic_import_namespace_live_binding_member_value(
            module_index,
            property,
        );
        let initializer = self
            .resolve_static_dynamic_import_namespace_live_binding_member_initializer_value(
                module_index,
                property,
            );

        if let Some(Expression::Identifier(name)) = live_value.as_ref()
            && name.starts_with("__ayy_capture_binding__")
            && let Some(initializer) = initializer.as_ref()
            && let Some(binding) = self
                .backend
                .implicit_global_binding(name)
                .or_else(|| self.hidden_implicit_global_binding(name))
        {
            self.push_global_get(binding.present_index);
            self.state.emission.output.instructions.push(0x04);
            self.state.emission.output.instructions.push(I32_TYPE);
            self.push_control_frame();
            self.push_global_get(binding.value_index);
            self.state.emission.output.instructions.push(0x05);
            self.emit_numeric_expression(initializer)?;
            self.state.emission.output.instructions.push(0x0b);
            self.pop_control_frame();
            return Ok(());
        }

        if let Some(live_value) = live_value.as_ref()
            && self.module_namespace_live_value_is_readable_in_current_context(live_value)
        {
            self.emit_numeric_expression(live_value)?;
            return Ok(());
        }

        if let Some(initializer) = initializer.as_ref() {
            self.emit_numeric_expression(initializer)?;
        } else if let Some(live_value) = live_value.as_ref() {
            self.emit_numeric_expression(live_value)?;
        } else {
            self.push_i32_const(JS_UNDEFINED_TAG);
        }
        Ok(())
    }

    fn deferred_module_namespace_prototype_get_module_index_and_property(
        &self,
        object: &Expression,
        property: &Expression,
    ) -> Option<(usize, Expression)> {
        if Self::expression_contains_await_for_user_call_runtime(object) {
            return None;
        }
        if let Some(access) =
            self.deferred_module_namespace_materialized_member_access(object, property)
        {
            return Some(access);
        }
        let property_key = self.deferred_module_namespace_get_property_key(property)?;
        if self.object_has_static_own_property_or_descriptor(object, &property_key) {
            return None;
        }

        let mut prototype = self.resolve_static_object_prototype_expression(object)?;
        for _ in 0..32 {
            let materialized_prototype = self.materialize_static_expression(&prototype);
            for candidate in [&prototype, &materialized_prototype] {
                if let Expression::Identifier(name) = candidate
                    && name.starts_with("__ayy_module_deferred_namespace_")
                {
                    let module_index = Self::module_index_from_namespace_like_identifier(name)?;
                    if self.current_function_name().is_some_and(|function_name| {
                        function_name == format!("__ayy_module_init_{module_index}")
                    }) {
                        return None;
                    }
                    return Some((module_index, property_key));
                }
                if self.object_has_static_own_property_or_descriptor(candidate, &property_key) {
                    return None;
                }
            }
            if matches!(materialized_prototype, Expression::Null) {
                return None;
            }

            let next_prototype = self
                .resolve_static_object_prototype_expression(&materialized_prototype)
                .or_else(|| self.resolve_static_object_prototype_expression(&prototype))?;
            if static_expression_matches(&next_prototype, &prototype)
                || static_expression_matches(&next_prototype, &materialized_prototype)
            {
                return None;
            }
            prototype = next_prototype;
        }
        None
    }

    fn emit_deferred_module_namespace_prototype_member_read(
        &mut self,
        object: &Expression,
        property: &Expression,
    ) -> DirectResult<bool> {
        let Some((module_index, property_key)) = self
            .deferred_module_namespace_prototype_get_module_index_and_property(object, property)
        else {
            return Ok(false);
        };

        self.emit_sync_module_init_if_needed(module_index, &mut HashSet::new())?;
        self.emit_deferred_module_namespace_member_value_after_eval(module_index, &property_key)?;
        Ok(true)
    }

    fn deferred_module_namespace_super_get_property_key(
        &self,
        property: &Expression,
    ) -> Option<Option<Expression>> {
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
            return Some(None);
        };
        if property_name == "then" || property_name.starts_with("__ayy$") {
            return None;
        }
        Some(Some(property_key))
    }

    fn emit_deferred_module_namespace_super_member_read(
        &mut self,
        super_base: Option<&Expression>,
        property: &Expression,
    ) -> DirectResult<bool> {
        let Some(super_base) = super_base else {
            return Ok(false);
        };
        let mut candidate_bases = Vec::new();
        candidate_bases.push(super_base.clone());
        let materialized_base = self.materialize_static_expression(super_base);
        if !static_expression_matches(&materialized_base, super_base) {
            candidate_bases.push(materialized_base);
        }
        let Some(module_index) = candidate_bases.into_iter().find_map(|candidate| {
            let Expression::Identifier(name) = candidate else {
                return None;
            };
            Self::module_index_from_namespace_like_identifier(&name)
        }) else {
            return Ok(false);
        };
        if self.current_function_name().is_some_and(|function_name| {
            function_name == format!("__ayy_module_init_{module_index}")
        }) {
            return Ok(false);
        }
        let Some(property_key) = self.deferred_module_namespace_super_get_property_key(property)
        else {
            return Ok(false);
        };

        self.emit_sync_module_init_if_needed(module_index, &mut HashSet::new())?;
        if let Some(property_key) = property_key {
            self.emit_deferred_module_namespace_member_value_after_eval(
                module_index,
                &property_key,
            )?;
        } else {
            self.push_i32_const(JS_UNDEFINED_TAG);
        }
        Ok(true)
    }

    pub(in crate::backend::direct_wasm) fn template_object_absent_static_own_property(
        property: &Expression,
    ) -> bool {
        let Some(property_name) = static_property_name_from_expression(property) else {
            return false;
        };
        if argument_index_from_expression(property).is_some() {
            return false;
        }
        !matches!(
            property_name.as_str(),
            "length"
                | "raw"
                | "__proto__"
                | "constructor"
                | "toString"
                | "toLocaleString"
                | "valueOf"
                | "hasOwnProperty"
                | "isPrototypeOf"
                | "propertyIsEnumerable"
                | "at"
                | "concat"
                | "copyWithin"
                | "entries"
                | "every"
                | "fill"
                | "filter"
                | "find"
                | "findIndex"
                | "findLast"
                | "findLastIndex"
                | "flat"
                | "flatMap"
                | "forEach"
                | "includes"
                | "indexOf"
                | "join"
                | "keys"
                | "lastIndexOf"
                | "map"
                | "pop"
                | "push"
                | "reduce"
                | "reduceRight"
                | "reverse"
                | "shift"
                | "slice"
                | "some"
                | "sort"
                | "splice"
                | "toReversed"
                | "toSorted"
                | "toSpliced"
                | "unshift"
                | "values"
                | "with"
        )
    }

    pub(in crate::backend::direct_wasm) fn current_function_declares_non_eval_binding_source(
        &self,
        name: &str,
    ) -> bool {
        let Some(function) = self.current_user_function_declaration() else {
            return false;
        };
        let source_name = scoped_binding_source_name(name).unwrap_or(name);
        collect_declared_bindings_from_statements_recursive(&function.body)
            .into_iter()
            .any(|binding| scoped_binding_source_name(&binding).unwrap_or(&binding) == source_name)
            || function.params.iter().any(|parameter| {
                scoped_binding_source_name(&parameter.name).unwrap_or(&parameter.name)
                    == source_name
            })
            || function.self_binding.as_ref().is_some_and(|self_binding| {
                scoped_binding_source_name(self_binding).unwrap_or(self_binding) == source_name
            })
            || source_name == "arguments"
    }

    pub(in crate::backend::direct_wasm) fn assignment_value_declares_static_direct_eval_var_binding(
        &self,
        name: &str,
        value: &Expression,
    ) -> bool {
        let mut bindings = HashSet::new();
        let caller_strict = self
            .current_user_function_declaration()
            .map(|function| function.strict)
            .unwrap_or(self.state.speculation.execution_context.strict_mode);
        collect_static_direct_eval_var_bindings_from_expression(
            value,
            caller_strict,
            &mut bindings,
        );
        let source_name = scoped_binding_source_name(name).unwrap_or(name);
        bindings
            .into_iter()
            .any(|binding| scoped_binding_source_name(&binding).unwrap_or(&binding) == source_name)
    }

    pub(in crate::backend::direct_wasm) fn emit_identifier_expression_value(
        &mut self,
        name: &str,
    ) -> DirectResult<()> {
        let trace_identifier_dispatch = std::env::var_os("AYY_TRACE_IDENTIFIER_DISPATCH").is_some();
        if trace_identifier_dispatch {
            eprintln!("identifier_dispatch:start name={name}");
        }
        if let Some(scope_object) = self.resolve_with_scope_binding(name)? {
            if trace_identifier_dispatch {
                eprintln!("identifier_dispatch:path scoped name={name}");
            }
            self.emit_scoped_property_read(&scope_object, name)?;
        } else {
            if trace_identifier_dispatch {
                eprintln!("identifier_dispatch:path plain name={name}");
            }
            self.with_suspended_with_scopes(|compiler| compiler.emit_plain_identifier_read(name))?;
        }
        if trace_identifier_dispatch {
            eprintln!("identifier_dispatch:done name={name}");
        }
        Ok(())
    }

    pub(in crate::backend::direct_wasm) fn emit_assign_expression_value(
        &mut self,
        name: &str,
        value: &Expression,
    ) -> DirectResult<()> {
        let scoped_target = self.resolve_with_scope_binding(name)?;
        let resolved_reference_local = scoped_target
            .is_none()
            .then(|| self.resolve_current_local_binding(name))
            .flatten();
        let resolved_reference_local = if resolved_reference_local.is_some()
            && self.assignment_value_declares_static_direct_eval_var_binding(name, value)
            && !self.current_function_declares_non_eval_binding_source(name)
        {
            None
        } else {
            resolved_reference_local
        };
        let reference_targets_capture = scoped_target.is_none()
            && resolved_reference_local.is_none()
            && !is_internal_assignment_temp(name)
            && self
                .resolve_user_function_capture_hidden_name(name)
                .is_some();
        let reference_global_index = (scoped_target.is_none()
            && resolved_reference_local.is_none()
            && !reference_targets_capture)
            .then(|| self.resolve_global_binding_index(name))
            .flatten();
        let reference_targets_eval_local = scoped_target.is_none()
            && resolved_reference_local.is_none()
            && !reference_targets_capture
            && reference_global_index.is_none()
            && self.resolve_eval_local_function_hidden_name(name).is_some();
        let reference_implicit_global = (scoped_target.is_none()
            && resolved_reference_local.is_none()
            && !reference_targets_capture
            && reference_global_index.is_none()
            && !reference_targets_eval_local)
            .then(|| self.backend.implicit_global_binding(name))
            .flatten();
        let reference_is_unresolvable = scoped_target.is_none()
            && resolved_reference_local.is_none()
            && !reference_targets_capture
            && reference_global_index.is_none()
            && !reference_targets_eval_local
            && reference_implicit_global.is_none();
        self.emit_numeric_expression(value)?;
        if let Some(scope_object) = scoped_target {
            let value_local = self.allocate_temp_local();
            self.push_local_set(value_local);
            self.emit_scoped_property_store_from_local(&scope_object, name, value_local, value)?;
        } else {
            let value_local = self.allocate_temp_local();
            self.push_local_set(value_local);
            self.emit_store_identifier_value_local_with_reference_target(
                name,
                value,
                value_local,
                resolved_reference_local,
                reference_targets_capture,
                reference_global_index,
                reference_targets_eval_local,
                reference_implicit_global,
                reference_is_unresolvable,
            )?;
            self.push_local_get(value_local);
        }
        Ok(())
    }

    fn template_object_member_read_kind(
        &self,
        property: &Expression,
    ) -> Option<TemplateObjectMemberRead> {
        if let Some(index) = argument_index_from_expression(property) {
            return Some(TemplateObjectMemberRead::Index(index));
        }
        match property {
            Expression::String(name) if name == "length" => Some(TemplateObjectMemberRead::Length),
            Expression::String(name) if name == "raw" => Some(TemplateObjectMemberRead::RawArray),
            _ if Self::template_object_absent_static_own_property(property) => {
                Some(TemplateObjectMemberRead::AbsentFrozenOwnProperty)
            }
            _ => None,
        }
    }

    fn emit_template_object_member_value(
        &mut self,
        binding: &ArrayValueBinding,
        read_kind: TemplateObjectMemberRead,
    ) -> DirectResult<()> {
        match read_kind {
            TemplateObjectMemberRead::Index(index) => {
                if let Some(Some(value)) = binding.values.get(index as usize) {
                    self.emit_numeric_expression(value)?;
                } else {
                    self.push_i32_const(JS_UNDEFINED_TAG);
                }
            }
            TemplateObjectMemberRead::Length => {
                self.push_i32_const(binding.values.len() as i32);
            }
            TemplateObjectMemberRead::RawArray => {
                self.push_i32_const(JS_TYPEOF_OBJECT_TAG);
            }
            TemplateObjectMemberRead::AbsentFrozenOwnProperty => {
                self.push_i32_const(JS_UNDEFINED_TAG);
            }
        }
        Ok(())
    }

    fn emit_template_object_member_read_from_local(
        &mut self,
        object_value_local: u32,
        fallback_object: &Expression,
        property: &Expression,
    ) -> DirectResult<bool> {
        let Some(read_kind) = self.template_object_member_read_kind(property) else {
            return Ok(false);
        };
        let mut templates = self
            .backend
            .template_object_array_bindings
            .iter()
            .map(|(runtime_value, binding)| (*runtime_value, binding.clone()))
            .collect::<Vec<_>>();
        let trace_template_objects = std::env::var_os("AYY_TRACE_TEMPLATE_OBJECTS").is_some();
        if trace_template_objects {
            eprintln!(
                "template_member:property={property:?} entries={}",
                templates.len()
            );
        }
        if templates.is_empty() {
            return Ok(false);
        }
        templates.sort_by_key(|(runtime_value, _)| *runtime_value);

        fn emit_branch<'a>(
            compiler: &mut FunctionCompiler<'a>,
            object_value_local: u32,
            fallback_object: &Expression,
            property: &Expression,
            templates: &[(i32, ArrayValueBinding)],
            read_kind: TemplateObjectMemberRead,
            index: usize,
        ) -> DirectResult<()> {
            let Some((runtime_value, binding)) = templates.get(index) else {
                return compiler.emit_member_read_without_prelude(fallback_object, property);
            };
            compiler.push_local_get(object_value_local);
            compiler.push_i32_const(*runtime_value);
            compiler.push_binary_op(BinaryOp::Equal)?;
            compiler.state.emission.output.instructions.push(0x04);
            compiler.state.emission.output.instructions.push(I32_TYPE);
            compiler.push_control_frame();
            compiler.emit_template_object_member_value(binding, read_kind)?;
            compiler.state.emission.output.instructions.push(0x05);
            emit_branch(
                compiler,
                object_value_local,
                fallback_object,
                property,
                templates,
                read_kind,
                index.saturating_add(1),
            )?;
            compiler.state.emission.output.instructions.push(0x0b);
            compiler.pop_control_frame();
            Ok(())
        }

        emit_branch(
            self,
            object_value_local,
            fallback_object,
            property,
            &templates,
            read_kind,
            0,
        )?;
        Ok(true)
    }

    fn emit_template_object_raw_array_member_read_from_local(
        &mut self,
        object_value_local: u32,
    ) -> DirectResult<bool> {
        let mut templates = self
            .backend
            .template_object_raw_array_bindings
            .iter()
            .map(|(runtime_value, binding)| (*runtime_value, binding.clone()))
            .collect::<Vec<_>>();
        if templates.is_empty() {
            return Ok(false);
        }
        templates.sort_by_key(|(runtime_value, _)| *runtime_value);

        fn emit_branch<'a>(
            compiler: &mut FunctionCompiler<'a>,
            object_value_local: u32,
            templates: &[(i32, ArrayValueBinding)],
            index: usize,
        ) -> DirectResult<()> {
            let Some((runtime_value, binding)) = templates.get(index) else {
                compiler.push_i32_const(JS_TYPEOF_OBJECT_TAG);
                return Ok(());
            };
            compiler.push_local_get(object_value_local);
            compiler.push_i32_const(*runtime_value);
            compiler.push_binary_op(BinaryOp::Equal)?;
            compiler.state.emission.output.instructions.push(0x04);
            compiler.state.emission.output.instructions.push(I32_TYPE);
            compiler.push_control_frame();
            let array = Expression::Array(
                binding
                    .values
                    .iter()
                    .map(|value| {
                        ArrayElement::Expression(value.clone().unwrap_or(Expression::Undefined))
                    })
                    .collect(),
            );
            compiler.emit_numeric_expression(&array)?;
            compiler.state.emission.output.instructions.push(0x05);
            emit_branch(
                compiler,
                object_value_local,
                templates,
                index.saturating_add(1),
            )?;
            compiler.state.emission.output.instructions.push(0x0b);
            compiler.pop_control_frame();
            Ok(())
        }

        emit_branch(self, object_value_local, &templates, 0)?;
        Ok(true)
    }

    fn emit_template_object_raw_array_absent_member_read(
        &mut self,
        object: &Expression,
        property: &Expression,
    ) -> DirectResult<bool> {
        if !Self::template_object_absent_static_own_property(property) {
            return Ok(false);
        }
        let Expression::Member {
            object: base_object,
            property: raw_property,
        } = object
        else {
            return Ok(false);
        };
        if !matches!(raw_property.as_ref(), Expression::String(name) if name == "raw") {
            return Ok(false);
        }
        if !inline_summary_side_effect_free_expression(base_object) {
            return Ok(false);
        }

        let mut runtime_values = self
            .backend
            .template_object_raw_array_bindings
            .keys()
            .copied()
            .collect::<Vec<_>>();
        if runtime_values.is_empty() {
            return Ok(false);
        }
        runtime_values.sort_unstable();

        let base_object_local = self.allocate_temp_local();
        self.emit_numeric_expression(base_object)?;
        self.push_local_set(base_object_local);

        fn emit_branch<'a>(
            compiler: &mut FunctionCompiler<'a>,
            base_object_local: u32,
            object: &Expression,
            property: &Expression,
            runtime_values: &[i32],
            index: usize,
        ) -> DirectResult<()> {
            let Some(runtime_value) = runtime_values.get(index) else {
                return compiler.emit_member_read_without_prelude(object, property);
            };
            compiler.push_local_get(base_object_local);
            compiler.push_i32_const(*runtime_value);
            compiler.push_binary_op(BinaryOp::Equal)?;
            compiler.state.emission.output.instructions.push(0x04);
            compiler.state.emission.output.instructions.push(I32_TYPE);
            compiler.push_control_frame();
            compiler.push_i32_const(JS_UNDEFINED_TAG);
            compiler.state.emission.output.instructions.push(0x05);
            emit_branch(
                compiler,
                base_object_local,
                object,
                property,
                runtime_values,
                index.saturating_add(1),
            )?;
            compiler.state.emission.output.instructions.push(0x0b);
            compiler.pop_control_frame();
            Ok(())
        }

        emit_branch(
            self,
            base_object_local,
            object,
            property,
            &runtime_values,
            0,
        )?;
        Ok(true)
    }

    pub(in crate::backend::direct_wasm) fn emit_member_expression_value(
        &mut self,
        object: &Expression,
        property: &Expression,
    ) -> DirectResult<()> {
        let trace_member_reads = std::env::var_os("AYY_TRACE_MEMBER_READS").is_some();
        if trace_member_reads {
            eprintln!(
                "member_expr:start current_fn={:?} object={object:?} property={property:?}",
                self.current_function_name(),
            );
        }
        if std::env::var_os("AYY_TRACE_PRIVATE_MEMBER_LOOKUP").is_some()
            && matches!(property, Expression::String(name) if name.starts_with("__ayy$private$"))
        {
            eprintln!(
                "private_emit_member current_fn={:?} object={object:?} property={property:?}",
                self.current_function_name(),
            );
        }
        let object_is_internal_assignment_temp =
            matches!(object, Expression::Identifier(name) if is_internal_assignment_temp(name));
        let object_contains_await = Self::expression_contains_await_for_user_call_runtime(object);
        let nested_assert_helper_member =
            Self::expression_is_nested_assert_helper_member_parts(object, property);
        let assert_helper_member = nested_assert_helper_member
            || matches!(object, Expression::Identifier(name) if name == "assert");
        if !object_is_internal_assignment_temp
            && !object_contains_await
            && self.emit_direct_iterator_step_member_read(object, property)?
        {
            if trace_member_reads {
                eprintln!("member_expr:direct_iterator object={object:?} property={property:?}");
            }
            return Ok(());
        }
        let original_member = Expression::Member {
            object: Box::new(object.clone()),
            property: Box::new(property.clone()),
        };
        let static_array_property = self
            .resolve_property_key_expression(property)
            .unwrap_or_else(|| self.materialize_static_expression(property));
        if !object_is_internal_assignment_temp
            && !object_contains_await
            && self.emit_test262_realm_global_constructor_member_value(
                object,
                &static_array_property,
            )?
        {
            return Ok(());
        }
        let tracked_array_property = argument_index_from_expression(&static_array_property)
            .is_some()
            || matches!(&static_array_property, Expression::String(name) if name == "length");
        if !object_is_internal_assignment_temp
            && matches!(object, Expression::Identifier(_))
            && tracked_array_property
            && self
                .runtime_array_binding_name_for_expression(object)
                .is_some_and(|name| {
                    self.runtime_array_binding_has_state(&name)
                        || self.uses_global_runtime_array_state(&name)
                        || self.expression_uses_runtime_array_state(object)
                })
            && self.emit_runtime_array_member_read(object, &static_array_property)?
        {
            if trace_member_reads {
                eprintln!(
                    "member_expr:runtime_array_early object={object:?} property={property:?}"
                );
            }
            return Ok(());
        }
        if let Expression::Identifier(name) = object
            && name.starts_with("__ayy_module_deferred_namespace_")
            && matches!(
                self.resolve_property_key_expression(property)
                    .unwrap_or_else(|| self.materialize_static_expression(property)),
                Expression::String(property_name) if property_name == "then"
            )
        {
            self.push_i32_const(JS_UNDEFINED_TAG);
            return Ok(());
        }
        if !object_is_internal_assignment_temp
            && !object_contains_await
            && matches!(object, Expression::Member { .. })
            && self.emit_deferred_module_namespace_prototype_member_read(object, property)?
        {
            if trace_member_reads {
                eprintln!(
                    "member_expr:deferred_namespace_materialized object={object:?} property={property:?}"
                );
            }
            return Ok(());
        }
        if let Expression::Identifier(name) = object
            && name.starts_with("__ayy_module_deferred_namespace_")
            && let Some(property_key) = self.deferred_module_namespace_get_property_key(property)
        {
            let Some(module_index) = Self::module_index_from_namespace_like_identifier(name) else {
                self.emit_named_error_throw("TypeError")?;
                self.push_i32_const(JS_UNDEFINED_TAG);
                return Ok(());
            };
            let status_name = format!("__ayy_module_status_{module_index}");
            let result_local = self.allocate_temp_local();
            self.push_i32_const(JS_UNDEFINED_TAG);
            self.push_local_set(result_local);

            self.emit_internal_global_read(&status_name)?;
            self.push_i32_const(2);
            self.push_binary_op(BinaryOp::Equal)?;
            self.state.emission.output.instructions.push(0x04);
            self.state
                .emission
                .output
                .instructions
                .push(EMPTY_BLOCK_TYPE);
            self.push_control_frame();
            self.emit_deferred_module_namespace_member_value_after_eval(
                module_index,
                &property_key,
            )?;
            self.push_local_set(result_local);
            self.state.emission.output.instructions.push(0x05);
            self.emit_internal_global_read(&status_name)?;
            self.push_i32_const(0);
            self.push_binary_op(BinaryOp::Equal)?;
            self.state.emission.output.instructions.push(0x04);
            self.state
                .emission
                .output
                .instructions
                .push(EMPTY_BLOCK_TYPE);
            self.push_control_frame();
            self.emit_sync_module_init_if_needed(module_index, &mut HashSet::new())?;
            self.emit_deferred_module_namespace_member_value_after_eval(
                module_index,
                &property_key,
            )?;
            self.push_local_set(result_local);
            self.state.emission.output.instructions.push(0x05);
            self.emit_internal_global_read(&status_name)?;
            self.push_i32_const(3);
            self.push_binary_op(BinaryOp::Equal)?;
            self.state.emission.output.instructions.push(0x04);
            self.state
                .emission
                .output
                .instructions
                .push(EMPTY_BLOCK_TYPE);
            self.push_control_frame();
            self.emit_module_cached_error_throw(module_index, None)?;
            self.state.emission.output.instructions.push(0x05);
            self.emit_named_error_throw("TypeError")?;
            self.state.emission.output.instructions.push(0x0b);
            self.pop_control_frame();
            self.state.emission.output.instructions.push(0x0b);
            self.pop_control_frame();
            self.state.emission.output.instructions.push(0x0b);
            self.pop_control_frame();
            self.push_local_get(result_local);
            return Ok(());
        }
        if !object_is_internal_assignment_temp
            && !object_contains_await
            && self
                .deferred_module_namespace_materialized_member_access(object, property)
                .is_none()
            && let Some(value) =
                self.resolve_module_namespace_live_binding_member_raw_value(object, property)
            && self.module_namespace_live_binding_value_is_capture_slot(&value)
        {
            self.emit_numeric_expression(&value)?;
            return Ok(());
        }
        if !object_is_internal_assignment_temp
            && !object_contains_await
            && self
                .deferred_module_namespace_materialized_member_access(object, property)
                .is_none()
            && let Some(value) =
                self.resolve_module_namespace_live_binding_member_value(object, property)
        {
            self.emit_numeric_expression(&value)?;
            return Ok(());
        }
        if !object_is_internal_assignment_temp
            && !object_contains_await
            && !nested_assert_helper_member
            && !matches!(property, Expression::Member { .. })
            && let Some(function_binding) =
                self.resolve_function_binding_from_expression(&original_member)
        {
            match function_binding {
                LocalFunctionBinding::User(function_name) => {
                    if let Some(runtime_value) = self.user_function_runtime_value(&function_name) {
                        if trace_member_reads {
                            eprintln!("member_expr:user_function_value member={original_member:?}");
                        }
                        self.push_i32_const(runtime_value);
                        return Ok(());
                    }
                }
                LocalFunctionBinding::Builtin(function_name) => {
                    if trace_member_reads {
                        eprintln!("member_expr:builtin_function_value member={original_member:?}");
                    }
                    self.push_i32_const(
                        builtin_function_runtime_value(&function_name)
                            .unwrap_or(JS_TYPEOF_FUNCTION_TAG),
                    );
                    return Ok(());
                }
            }
        }
        if !object_is_internal_assignment_temp
            && !object_contains_await
            && matches!(property, Expression::String(property_name) if property_name == "prototype")
        {
            let resolved_object = self
                .resolve_bound_alias_expression(object)
                .filter(|resolved| !static_expression_matches(resolved, object));
            let materialized_object = self.materialize_static_expression(object);
            if let Some(descriptor) = self.resolve_function_property_descriptor_binding(
                object,
                resolved_object.as_ref(),
                &materialized_object,
                "prototype",
            ) {
                let original_member = Expression::Member {
                    object: Box::new(object.clone()),
                    property: Box::new(Expression::String("prototype".to_string())),
                };
                match descriptor.value {
                    Some(value) if static_expression_matches(&value, &original_member) => {
                        self.push_i32_const(JS_TYPEOF_OBJECT_TAG);
                    }
                    Some(value) => {
                        self.emit_numeric_expression(&value)?;
                    }
                    None => {
                        self.push_i32_const(JS_UNDEFINED_TAG);
                    }
                }
                return Ok(());
            }
        }
        let object_value_local = self.allocate_temp_local();
        self.emit_numeric_expression(object)?;
        self.push_local_set(object_value_local);
        self.emit_throw_if_member_base_nullish_local(object_value_local)?;
        self.push_local_get(object_value_local);
        if trace_member_reads {
            eprintln!("member_expr:object_done object={object:?} property={property:?}");
        }
        self.state.emission.output.instructions.push(0x1a);
        let resolved_property = self.emit_property_key_expression_effects(property)?;
        if trace_member_reads {
            eprintln!(
                "member_expr:property_done object={object:?} property={property:?} resolved={resolved_property:?}"
            );
        }
        if let Some(resolved_property) = resolved_property.as_ref()
            && let Expression::Identifier(property_name) = property
            && is_internal_target_property_temp(property_name)
        {
            let property_key_local = self.allocate_temp_local();
            self.emit_numeric_expression(resolved_property)?;
            self.push_local_set(property_key_local);
            self.emit_store_identifier_value_local(
                property_name,
                resolved_property,
                property_key_local,
            )?;
        }
        let effective_property = resolved_property.as_ref().unwrap_or(property);
        let read_object = self
            .private_member_read_receiver_after_evaluation(object, effective_property)
            .unwrap_or_else(|| object.clone());
        if self.emit_deferred_module_namespace_prototype_member_read(
            &read_object,
            effective_property,
        )? {
            return Ok(());
        }
        if !object_contains_await && !assert_helper_member {
            if matches!(effective_property, Expression::String(name) if name == "raw")
                && self.emit_template_object_raw_array_member_read_from_local(object_value_local)?
            {
                if trace_member_reads {
                    eprintln!("member_expr:template_object_raw object={read_object:?}");
                }
                return Ok(());
            }
            if self.emit_template_object_raw_array_absent_member_read(
                &read_object,
                effective_property,
            )? {
                if trace_member_reads {
                    eprintln!(
                        "member_expr:template_object_raw_absent object={read_object:?} property={effective_property:?}"
                    );
                }
                return Ok(());
            }
            if self.emit_template_object_member_read_from_local(
                object_value_local,
                &read_object,
                effective_property,
            )? {
                if trace_member_reads {
                    eprintln!(
                        "member_expr:template_object object={read_object:?} property={effective_property:?}"
                    );
                }
                return Ok(());
            }
        }
        let result = self.emit_member_read_without_prelude(&read_object, effective_property);
        if trace_member_reads {
            eprintln!(
                "member_expr:done object={read_object:?} property={effective_property:?} ok={}",
                result.is_ok()
            );
        }
        result
    }

    pub(in crate::backend::direct_wasm) fn expression_is_nested_assert_helper_member_parts(
        object: &Expression,
        _property: &Expression,
    ) -> bool {
        matches!(
            object,
            Expression::Member { object: root, .. }
                if matches!(root.as_ref(), Expression::Identifier(name) if name == "assert")
        )
    }

    pub(in crate::backend::direct_wasm) fn expression_is_nested_assert_helper_member_expression(
        expression: &Expression,
    ) -> bool {
        matches!(
            expression,
            Expression::Member { object, property }
                if Self::expression_is_nested_assert_helper_member_parts(object, property)
        )
    }

    pub(in crate::backend::direct_wasm) fn resolve_module_namespace_live_binding_member_value(
        &self,
        object: &Expression,
        property: &Expression,
    ) -> Option<Expression> {
        let property = self
            .resolve_property_key_expression(property)
            .unwrap_or_else(|| self.materialize_static_expression(property));
        let property = static_property_name_from_expression(&property)
            .map(Expression::String)
            .unwrap_or(property);
        if let Expression::Identifier(name) = object
            && let Some(module_index) = Self::module_index_from_namespace_like_identifier(name)
        {
            let live_value = self
                .resolve_static_dynamic_import_namespace_live_binding_member_value(
                    module_index,
                    &property,
                );
            let initializer = self
                .resolve_static_dynamic_import_namespace_live_binding_member_initializer_value(
                    module_index,
                    &property,
                );
            return Self::module_namespace_member_value_with_initializer_fallback(
                live_value,
                initializer,
            );
        }
        let object_binding = self
            .direct_module_namespace_object_binding(object)
            .or_else(|| {
                self.resolve_object_binding_from_expression(object)
                    .filter(Self::object_binding_has_module_namespace_marker)
            })?;
        let namespace_marker = object_binding_lookup_value(
            &object_binding,
            &Expression::String("__ayy$module$namespace".to_string()),
        )?;
        if !matches!(namespace_marker, Expression::Bool(true)) {
            return None;
        }
        if let Some(descriptor) = object_binding_lookup_descriptor(&object_binding, &property)
            && let Some(value) = descriptor.value.clone()
        {
            return Some(value);
        }
        let value = object_binding_lookup_value(&object_binding, &property)?.clone();
        match &value {
            Expression::Identifier(name)
                if name.starts_with("__ayy_capture_binding__")
                    && self
                        .implicit_global_binding(name)
                        .or_else(|| self.hidden_implicit_global_binding(name))
                        .is_some() =>
            {
                let module_index = object_binding_lookup_value(
                    &object_binding,
                    &Expression::String("__ayy$module$namespace$moduleIndex".to_string()),
                )
                .and_then(|value| match value {
                    Expression::Number(index)
                        if index.is_finite() && *index >= 0.0 && index.fract() == 0.0 =>
                    {
                        Some(*index as usize)
                    }
                    _ => None,
                });
                let initializer = module_index.and_then(|module_index| {
                    self.resolve_static_dynamic_import_namespace_live_binding_member_initializer_value(
                        module_index,
                        &property,
                    )
                });
                Self::module_namespace_member_value_with_initializer_fallback(
                    Some(value),
                    initializer,
                )
            }
            _ => {
                let module_index = object_binding_lookup_value(
                    &object_binding,
                    &Expression::String("__ayy$module$namespace$moduleIndex".to_string()),
                )
                .and_then(|value| match value {
                    Expression::Number(index)
                        if index.is_finite() && *index >= 0.0 && index.fract() == 0.0 =>
                    {
                        Some(*index as usize)
                    }
                    _ => None,
                })?;
                self.resolve_static_dynamic_import_namespace_live_binding_member_value(
                    module_index,
                    &property,
                )
            }
        }
    }

    pub(in crate::backend::direct_wasm) fn resolve_module_namespace_live_binding_member_raw_value(
        &self,
        object: &Expression,
        property: &Expression,
    ) -> Option<Expression> {
        let property = self
            .resolve_property_key_expression(property)
            .unwrap_or_else(|| self.materialize_static_expression(property));
        let property = static_property_name_from_expression(&property)
            .map(Expression::String)
            .unwrap_or(property);
        if let Expression::Identifier(name) = object
            && let Some(module_index) = Self::module_index_from_namespace_like_identifier(name)
        {
            return self.resolve_static_dynamic_import_namespace_live_binding_member_value(
                module_index,
                &property,
            );
        }

        let object_binding = self
            .direct_module_namespace_object_binding(object)
            .or_else(|| {
                self.resolve_object_binding_from_expression(object)
                    .filter(Self::object_binding_has_module_namespace_marker)
            })?;
        let namespace_marker = object_binding_lookup_value(
            &object_binding,
            &Expression::String("__ayy$module$namespace".to_string()),
        )?;
        if !matches!(namespace_marker, Expression::Bool(true)) {
            return None;
        }
        if let Some(descriptor) = object_binding_lookup_descriptor(&object_binding, &property)
            && let Some(value) = descriptor.value.clone()
        {
            return Some(value);
        }
        let value = object_binding_lookup_value(&object_binding, &property)?.clone();
        if self.module_namespace_live_binding_value_is_capture_slot(&value) {
            return Some(value);
        }
        let module_index = object_binding_lookup_value(
            &object_binding,
            &Expression::String("__ayy$module$namespace$moduleIndex".to_string()),
        )
        .and_then(|value| match value {
            Expression::Number(index)
                if index.is_finite() && *index >= 0.0 && index.fract() == 0.0 =>
            {
                Some(*index as usize)
            }
            _ => None,
        })?;
        self.resolve_static_dynamic_import_namespace_live_binding_member_value(
            module_index,
            &property,
        )
    }

    pub(in crate::backend::direct_wasm) fn module_namespace_live_binding_value_is_capture_slot(
        &self,
        value: &Expression,
    ) -> bool {
        matches!(
            value,
            Expression::Identifier(name)
                if name.starts_with("__ayy_capture_binding__")
                    && self.hidden_implicit_global_binding(name).is_some()
        )
    }

    pub(in crate::backend::direct_wasm) fn module_namespace_member_value_with_initializer_fallback(
        live_value: Option<Expression>,
        initializer: Option<Expression>,
    ) -> Option<Expression> {
        match (live_value, initializer) {
            (Some(Expression::Identifier(name)), Some(initializer))
                if name.starts_with("__ayy_capture_binding__") =>
            {
                Some(initializer)
            }
            (Some(value), _) => Some(value),
            (None, Some(initializer)) => Some(initializer),
            (None, None) => None,
        }
    }

    pub(in crate::backend::direct_wasm) fn direct_module_namespace_object_binding(
        &self,
        object: &Expression,
    ) -> Option<ObjectValueBinding> {
        let Expression::Identifier(name) = object else {
            return None;
        };
        let resolved_local_name = self
            .resolve_current_local_binding(name)
            .map(|(resolved_name, _)| resolved_name);
        let mut candidate_names = Vec::new();
        if let Some(resolved_name) = resolved_local_name.as_ref() {
            candidate_names.push(resolved_name.as_str());
        }
        candidate_names.push(name.as_str());
        candidate_names.sort_unstable();
        candidate_names.dedup();

        for candidate_name in &candidate_names {
            if let Some(binding) = self
                .state
                .speculation
                .static_semantics
                .local_object_binding(candidate_name)
                .filter(|binding| Self::object_binding_has_module_namespace_marker(binding))
            {
                return Some(binding.clone());
            }
        }
        if resolved_local_name.is_some() {
            return None;
        }
        for candidate_name in candidate_names {
            if let Some(binding) = self
                .global_object_binding(candidate_name)
                .filter(|binding| Self::object_binding_has_module_namespace_marker(binding))
            {
                return Some(binding.clone());
            }
        }
        None
    }

    fn private_member_read_receiver_after_evaluation(
        &self,
        object: &Expression,
        property: &Expression,
    ) -> Option<Expression> {
        if !self.is_private_member_read_property(property) {
            return None;
        }
        let Expression::Call { callee, arguments } = object else {
            return None;
        };
        if !arguments.is_empty() {
            return None;
        }
        let LocalFunctionBinding::User(function_name) =
            self.resolve_function_binding_from_expression(callee)?
        else {
            return None;
        };
        let user_function = self.user_function(&function_name)?;
        if !user_function.lexical_this {
            return None;
        }
        let function = self.resolve_registered_function_declaration(&function_name)?;
        matches!(
            function.body.as_slice(),
            [Statement::Return(Expression::This)]
        )
        .then_some(Expression::This)
    }

    pub(in crate::backend::direct_wasm) fn emit_throw_if_member_base_nullish_local(
        &mut self,
        object_value_local: u32,
    ) -> DirectResult<()> {
        self.push_local_get(object_value_local);
        self.push_i32_const(JS_NULL_TAG);
        self.push_binary_op(BinaryOp::Equal)?;

        self.push_local_get(object_value_local);
        self.push_i32_const(JS_UNDEFINED_TAG);
        self.push_binary_op(BinaryOp::Equal)?;

        self.state.emission.output.instructions.push(0x72);
        self.state.emission.output.instructions.push(0x04);
        self.state
            .emission
            .output
            .instructions
            .push(EMPTY_BLOCK_TYPE);
        self.push_control_frame();
        self.emit_named_error_throw("TypeError")?;
        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();
        Ok(())
    }

    pub(in crate::backend::direct_wasm) fn emit_super_member_expression_value(
        &mut self,
        property: &Expression,
    ) -> DirectResult<()> {
        if std::env::var_os("AYY_TRACE_SUPER_RESOLUTION").is_some() {
            eprintln!(
                "super_resolution:emit_member current={:?} property={property:?}",
                self.current_function_name()
            );
        }
        let resolved_property = self.resolve_property_key_expression_with_coercion(property);
        self.emit_numeric_expression(property)?;
        self.state.emission.output.instructions.push(0x1a);
        if let Some(coercion) = resolved_property
            .as_ref()
            .and_then(|resolved| resolved.coercion.clone())
            .or_else(|| self.resolve_property_key_coercion_binding(property))
        {
            self.emit_super_property_key_coercion_effect(&coercion)?;
        }
        let property = resolved_property
            .as_ref()
            .map(|resolved| &resolved.key)
            .unwrap_or(property);

        if self.current_function_is_derived_constructor() {
            self.push_global_get(CURRENT_THIS_GLOBAL_INDEX);
            self.push_i32_const(JS_UNDEFINED_TAG);
            self.push_binary_op(BinaryOp::Equal)?;
            self.state.emission.output.instructions.push(0x04);
            self.state
                .emission
                .output
                .instructions
                .push(EMPTY_BLOCK_TYPE);
            self.push_control_frame();
            self.emit_named_error_throw("ReferenceError")?;
            self.state.emission.output.instructions.push(0x0b);
            self.pop_control_frame();
        }

        let super_base =
            self.resolve_super_base_expression_with_context(self.current_function_name());
        if self.super_base_is_statically_nullish(super_base.as_ref()) {
            self.emit_named_error_throw("TypeError")?;
            return Ok(());
        }
        if let Some(function_binding) = self.resolve_super_function_binding(property) {
            match function_binding {
                LocalFunctionBinding::User(function_name) => {
                    if let Some(user_function) = self.user_function(&function_name) {
                        self.push_i32_const(user_function_runtime_value(user_function));
                    } else {
                        self.push_i32_const(JS_UNDEFINED_TAG);
                    }
                }
                LocalFunctionBinding::Builtin(_) => {
                    self.push_i32_const(JS_TYPEOF_FUNCTION_TAG);
                }
            }
            return Ok(());
        }
        if let Some(function_binding) = self.resolve_super_getter_binding(property) {
            match function_binding {
                LocalFunctionBinding::User(function_name) => {
                    let capture_slots = self
                        .resolve_super_base_expression_with_context(self.current_function_name())
                        .and_then(|base| {
                            self.resolve_member_function_capture_slots(&base, property)
                        });
                    self.emit_member_getter_call_with_bound_this(
                        &function_name,
                        &Expression::This,
                        capture_slots.as_ref(),
                    )?;
                }
                LocalFunctionBinding::Builtin(function_name) => {
                    let callee = Expression::Identifier(function_name);
                    if !self.emit_arguments_slot_accessor_call(&callee, &[], 0, Some(&[]))? {
                        self.push_i32_const(JS_UNDEFINED_TAG);
                    }
                }
            }
            return Ok(());
        }
        if let Some(value) = self.resolve_super_value_expression(property) {
            self.emit_numeric_expression(&value)?;
            return Ok(());
        }
        if self.emit_super_member_read_via_runtime_prototype_binding(property)? {
            return Ok(());
        }
        self.push_i32_const(JS_UNDEFINED_TAG);
        Ok(())
    }

    pub(in crate::backend::direct_wasm) fn emit_this_expression_value(
        &mut self,
    ) -> DirectResult<()> {
        if self.current_function_is_derived_constructor() {
            self.push_global_get(CURRENT_THIS_GLOBAL_INDEX);
            self.push_i32_const(JS_UNDEFINED_TAG);
            self.push_binary_op(BinaryOp::Equal)?;
            self.state.emission.output.instructions.push(0x04);
            self.state.emission.output.instructions.push(I32_TYPE);
            self.push_control_frame();
            self.emit_named_error_throw("ReferenceError")?;
            self.push_i32_const(JS_UNDEFINED_TAG);
            self.state.emission.output.instructions.push(0x05);
            self.push_global_get(CURRENT_THIS_GLOBAL_INDEX);
            self.state.emission.output.instructions.push(0x0b);
            self.pop_control_frame();
            return Ok(());
        }
        if self
            .current_user_function()
            .is_some_and(|function| function.lexical_this)
            && let Some(hidden_name) = self.resolve_user_function_capture_hidden_name("this")
        {
            let binding = self
                .implicit_global_binding(&hidden_name)
                .unwrap_or_else(|| self.ensure_implicit_global_binding(&hidden_name));
            self.push_global_get(binding.value_index);
            return Ok(());
        }
        self.push_global_get(CURRENT_THIS_GLOBAL_INDEX);
        Ok(())
    }
}
