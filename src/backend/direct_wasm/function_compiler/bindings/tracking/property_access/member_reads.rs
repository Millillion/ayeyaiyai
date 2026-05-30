use super::*;

mod getter_calls;
mod runtime_reads;
mod static_reads;

impl<'a> FunctionCompiler<'a> {
    fn resolve_non_thenable_awaited_object_binding(
        &self,
        expression: &Expression,
    ) -> Option<ObjectValueBinding> {
        match expression {
            Expression::Await(value) => {
                let binding = self.resolve_object_binding_from_expression(value)?;
                let then_property = Expression::String("then".to_string());
                (object_binding_lookup_value(&binding, &then_property).is_none()
                    && object_binding_lookup_descriptor(&binding, &then_property).is_none())
                .then_some(binding)
            }
            Expression::Member { object, property }
                if Self::expression_contains_await_for_user_call_runtime(object) =>
            {
                let object_binding = self.resolve_non_thenable_awaited_object_binding(object)?;
                let property = self
                    .resolve_property_key_expression(property)
                    .unwrap_or_else(|| self.materialize_static_expression(property));
                let value =
                    object_binding_lookup_value(&object_binding, &property).or_else(|| {
                        object_binding_lookup_descriptor(&object_binding, &property)
                            .and_then(|descriptor| descriptor.value.as_ref())
                    })?;
                self.resolve_object_binding_from_expression(value)
            }
            _ => None,
        }
    }

    fn internal_temp_static_value(&self, object: &Expression) -> Option<Expression> {
        if !Self::expression_references_internal_assignment_temp(object) {
            return None;
        }
        let Expression::Identifier(name) = object else {
            return None;
        };
        let resolved_name = self
            .resolve_current_local_binding(name)
            .map(|(resolved_name, _)| resolved_name);
        resolved_name
            .as_deref()
            .and_then(|resolved_name| {
                self.state
                    .speculation
                    .static_semantics
                    .local_value_binding(resolved_name)
                    .cloned()
            })
            .or_else(|| {
                self.state
                    .speculation
                    .static_semantics
                    .local_value_binding(name)
                    .cloned()
            })
    }

    fn emit_internal_temp_static_string_member_read(
        &mut self,
        object: &Expression,
        static_property: &Expression,
    ) -> DirectResult<bool> {
        let Some(Expression::String(text)) = self.internal_temp_static_value(object) else {
            return Ok(false);
        };

        if let Some(index) = argument_index_from_expression(static_property) {
            if let Some(character) = text.chars().nth(index as usize) {
                self.emit_static_string_literal(&character.to_string())?;
            } else {
                self.push_i32_const(JS_UNDEFINED_TAG);
            }
            return Ok(true);
        }
        if matches!(static_property, Expression::String(property_name) if property_name == "length")
        {
            self.push_i32_const(text.chars().count() as i32);
            return Ok(true);
        }

        Ok(false)
    }

    fn emit_internal_temp_function_name_member_read(
        &mut self,
        object: &Expression,
        static_property: &Expression,
    ) -> DirectResult<bool> {
        if !Self::expression_references_internal_assignment_temp(object)
            || !matches!(static_property, Expression::String(property_name) if property_name == "name")
        {
            return Ok(false);
        }
        let Expression::Identifier(name) = object else {
            return Ok(false);
        };

        let resolved_name = self
            .resolve_current_local_binding(name)
            .map(|(resolved_name, _)| resolved_name);
        let function_binding = resolved_name
            .as_deref()
            .and_then(|resolved_name| {
                self.state
                    .speculation
                    .static_semantics
                    .local_function_binding(resolved_name)
                    .cloned()
            })
            .or_else(|| {
                self.state
                    .speculation
                    .static_semantics
                    .local_function_binding(name)
                    .cloned()
            });

        let Some(function_binding) = function_binding else {
            return Ok(false);
        };

        match function_binding {
            LocalFunctionBinding::User(function_name) => {
                let Some(user_function) = self.user_function(&function_name).cloned() else {
                    return Ok(false);
                };
                if let Some(Expression::String(display_name)) =
                    self.runtime_user_function_property_value(&user_function, "name")
                {
                    self.emit_static_string_literal(&display_name)?;
                    return Ok(true);
                }
            }
            LocalFunctionBinding::Builtin(function_name) => {
                let display_name = builtin_function_display_name(&function_name).to_string();
                self.emit_static_string_literal(&display_name)?;
                return Ok(true);
            }
        }

        Ok(false)
    }

    pub(in crate::backend::direct_wasm) fn is_private_member_read_property(
        &self,
        property: &Expression,
    ) -> bool {
        matches!(
            self.resolve_property_key_expression(property)
                .unwrap_or_else(|| self.materialize_static_expression(property)),
            Expression::String(name) if name.starts_with("__ayy$private$")
        )
    }

    pub(in crate::backend::direct_wasm) fn emit_member_read_without_prelude(
        &mut self,
        object: &Expression,
        property: &Expression,
    ) -> DirectResult<()> {
        let trace_member_reads = std::env::var_os("AYY_TRACE_MEMBER_READS").is_some();
        if trace_member_reads {
            eprintln!("member_read:start object={object:?} property={property:?}");
        }
        let static_array_property = if inline_summary_side_effect_free_expression(property)
            && !self.expression_depends_on_active_loop_assignment(property)
        {
            self.resolve_property_key_expression(property)
                .unwrap_or_else(|| self.materialize_static_expression(property))
        } else {
            property.clone()
        };

        if trace_member_reads {
            eprintln!(
                "member_read:static_property object={object:?} property={property:?} static={static_array_property:?}"
            );
        }
        let reads_descriptor_member =
            self.expression_reads_local_descriptor_binding_member(&Expression::Member {
                object: Box::new(object.clone()),
                property: Box::new(property.clone()),
            });
        if trace_member_reads {
            eprintln!(
                "member_read:descriptor_check object={object:?} property={property:?} reads={reads_descriptor_member}"
            );
        }
        let descriptor_read_emitted = reads_descriptor_member
            && self.emit_runtime_descriptor_member_read(object, property)?;
        if descriptor_read_emitted {
            if trace_member_reads {
                eprintln!("member_read:descriptor_hit object={object:?} property={property:?}");
            }
            if std::env::var_os("AYY_TRACE_RUNTIME_SHADOWS").is_some() {
                eprintln!(
                    "runtime_shadow_member_dispatch descriptor_early object={object:?} property={property:?}"
                );
            }
            return Ok(());
        }
        let skip_static_special_for_descriptor_member = reads_descriptor_member
            && self
                .resolve_iterator_step_binding_from_expression(object)
                .is_none();
        if trace_member_reads {
            eprintln!(
                "member_read:before_special object={object:?} property={property:?} skip={skip_static_special_for_descriptor_member}"
            );
        }
        let object_uses_internal_assignment_temp =
            Self::expression_references_internal_assignment_temp(object);
        let object_contains_await = Self::expression_contains_await_for_user_call_runtime(object);
        if self.emit_internal_temp_static_string_member_read(object, &static_array_property)? {
            if trace_member_reads {
                eprintln!(
                    "member_read:internal_temp_static_string_hit object={object:?} property={static_array_property:?}"
                );
            }
            return Ok(());
        }
        if self.emit_internal_temp_function_name_member_read(object, &static_array_property)? {
            if trace_member_reads {
                eprintln!("member_read:internal_temp_function_name_hit object={object:?}");
            }
            return Ok(());
        }
        if !skip_static_special_for_descriptor_member
            && !object_contains_await
            && !object_uses_internal_assignment_temp
            && self.emit_special_member_read_without_prelude(
                object,
                property,
                &static_array_property,
            )?
        {
            if trace_member_reads {
                eprintln!("member_read:special_hit object={object:?} property={property:?}");
            }
            if std::env::var_os("AYY_TRACE_RUNTIME_SHADOWS").is_some() {
                eprintln!(
                    "runtime_shadow_member_dispatch special object={object:?} property={property:?}"
                );
            }
            return Ok(());
        }
        if trace_member_reads {
            eprintln!("member_read:before_binding object={object:?} property={property:?}");
        }
        let runtime_array_member_requires_runtime_read = !object_contains_await
            && (argument_index_from_expression(&static_array_property).is_some()
                || matches!(&static_array_property, Expression::String(name) if name == "length"))
            && self
                .runtime_array_binding_name_for_expression(object)
                .is_some_and(|name| {
                    self.runtime_array_binding_has_state(&name)
                        || self.uses_global_runtime_array_state(&name)
                        || self.expression_uses_runtime_array_state(object)
                });
        if runtime_array_member_requires_runtime_read
            && self.emit_runtime_array_member_read(object, &static_array_property)?
        {
            if trace_member_reads {
                eprintln!(
                    "member_read:runtime_array_member_hit object={object:?} property={property:?}"
                );
            }
            return Ok(());
        }
        if matches!(
            object,
            Expression::Identifier(name) if name.starts_with("__ayy_module_deferred_namespace_")
        ) && matches!(&static_array_property, Expression::String(name) if name == "then")
        {
            self.push_i32_const(JS_UNDEFINED_TAG);
            return Ok(());
        }
        let awaited_object_binding = if object_contains_await {
            self.resolve_non_thenable_awaited_object_binding(object)
        } else {
            None
        };
        if let Some(object_binding) = awaited_object_binding.as_ref() {
            if let Some(value) =
                object_binding_lookup_value(&object_binding, &static_array_property)
            {
                self.emit_numeric_expression(value)?;
                return Ok(());
            }
            if let Some(descriptor) =
                object_binding_lookup_descriptor(&object_binding, &static_array_property)
                && let Some(value) = descriptor.value.as_ref()
            {
                self.emit_numeric_expression(value)?;
                return Ok(());
            }
        }
        if !object_uses_internal_assignment_temp
            && !object_contains_await
            && let Some(value) = self
                .resolve_module_namespace_live_binding_member_value(object, &static_array_property)
        {
            if trace_member_reads {
                eprintln!(
                    "member_read:module_namespace_live_binding object={object:?} property={property:?} value={value:?}"
                );
            }
            self.emit_numeric_expression(&value)?;
            return Ok(());
        }
        if !object_uses_internal_assignment_temp
            && !object_contains_await
            && static_property_name_from_expression(&static_array_property).is_some()
            && self
                .direct_module_namespace_object_binding(object)
                .or_else(|| {
                    self.resolve_object_binding_from_expression(object)
                        .filter(Self::object_binding_has_module_namespace_marker)
                })
                .as_ref()
                .is_some_and(Self::object_binding_has_module_namespace_marker)
        {
            self.push_i32_const(JS_UNDEFINED_TAG);
            return Ok(());
        }
        if !object_uses_internal_assignment_temp
            && !object_contains_await
            && self.runtime_object_property_shadow_deletion_may_affect_property(
                object,
                &static_array_property,
            )
            && self.resolve_array_binding_from_expression(object).is_none()
            && self.emit_runtime_object_shadow_member_read(object, &static_array_property)?
        {
            if trace_member_reads {
                eprintln!("member_read:deleted_shadow_hit object={object:?} property={property:?}");
            }
            return Ok(());
        }
        let dynamic_descriptor_member_read = matches!(
            (object, property),
            (Expression::Identifier(name), Expression::String(property_name))
                if matches!(
                    property_name.as_str(),
                    "value" | "configurable" | "enumerable" | "writable" | "get" | "set"
                ) && self.local_binding_is_dynamic_property_descriptor_result(name)
        );
        if trace_member_reads && dynamic_descriptor_member_read {
            eprintln!(
                "member_read:skip_binding_for_dynamic_descriptor object={object:?} property={property:?}"
            );
        }
        let nested_assert_helper_member =
            Self::expression_is_nested_assert_helper_member_parts(object, property);
        if trace_member_reads && nested_assert_helper_member {
            eprintln!(
                "member_read:skip_binding_for_nested_assert_helper object={object:?} property={property:?}"
            );
        }
        if !object_uses_internal_assignment_temp
            && !object_contains_await
            && !dynamic_descriptor_member_read
            && !nested_assert_helper_member
            && self.emit_member_binding_read_without_prelude(object, property)?
        {
            if trace_member_reads {
                eprintln!("member_read:binding_hit object={object:?} property={property:?}");
            }
            if std::env::var_os("AYY_TRACE_RUNTIME_SHADOWS").is_some() {
                eprintln!(
                    "runtime_shadow_member_dispatch binding object={object:?} property={property:?}"
                );
            }
            return Ok(());
        }
        if trace_member_reads {
            eprintln!("member_read:before_runtime object={object:?} property={property:?}");
        }
        if self.emit_runtime_or_object_member_read_without_prelude(
            object,
            property,
            &static_array_property,
        )? {
            if trace_member_reads {
                eprintln!("member_read:runtime_hit object={object:?} property={property:?}");
            }
            if std::env::var_os("AYY_TRACE_RUNTIME_SHADOWS").is_some() {
                eprintln!(
                    "runtime_shadow_member_dispatch runtime object={object:?} property={property:?}"
                );
            }
            return Ok(());
        }
        if self.is_private_member_read_property(property) {
            return self.emit_named_error_throw("TypeError");
        }
        self.push_i32_const(JS_TYPEOF_OBJECT_TAG);
        Ok(())
    }
}
