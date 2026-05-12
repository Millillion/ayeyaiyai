use super::*;

#[path = "runtime_reads/array_reads.rs"]
mod array_reads;
#[path = "runtime_reads/descriptor_reads.rs"]
mod descriptor_reads;
#[path = "runtime_reads/object_reads.rs"]
mod object_reads;
#[path = "runtime_reads/shadow_reads.rs"]
mod shadow_reads;
#[path = "runtime_reads/special_reads.rs"]
mod special_reads;

impl<'a> FunctionCompiler<'a> {
    pub(in crate::backend::direct_wasm) fn current_private_brand_binding_name(
        &self,
    ) -> Option<String> {
        self.current_user_function()
            .and_then(|user_function| user_function.private_brand_binding.clone())
    }

    pub(in crate::backend::direct_wasm) fn synthetic_private_brand_runtime_value(
        binding_name: &str,
    ) -> i32 {
        let mut hash = 0x811c9dc5_u32;
        for byte in binding_name.as_bytes() {
            hash ^= u32::from(*byte);
            hash = hash.wrapping_mul(0x01000193);
        }
        (0x5000_0000_u32 | (hash & 0x0fff_ffff)) as i32
    }

    pub(in crate::backend::direct_wasm) fn emit_private_brand_direct_or_synthetic_runtime_value_for_binding_name(
        &mut self,
        binding_name: &str,
    ) -> DirectResult<()> {
        if self.resolve_current_local_binding(binding_name).is_some()
            || self.resolve_global_binding_index(binding_name).is_some()
            || self.hidden_implicit_global_binding(binding_name).is_some()
        {
            if std::env::var_os("AYY_TRACE_PRIVATE_BRAND_COMPILE").is_some() {
                eprintln!(
                    "private_brand_direct_value current_fn={:?} binding={binding_name} source=identifier",
                    self.current_function_name(),
                );
            }
            self.emit_numeric_expression(&Expression::Identifier(binding_name.to_string()))?;
            return Ok(());
        }

        if let Some(private_brand_offset) = binding_name.find("__ayy_class_brand_") {
            if std::env::var_os("AYY_TRACE_PRIVATE_BRAND_COMPILE").is_some() {
                eprintln!(
                    "private_brand_direct_value current_fn={:?} binding={binding_name} source=synthetic suffix={}",
                    self.current_function_name(),
                    &binding_name[private_brand_offset..],
                );
            }
            self.push_i32_const(Self::synthetic_private_brand_runtime_value(
                &binding_name[private_brand_offset..],
            ));
            return Ok(());
        }

        if self.lookup_identifier_kind(binding_name).is_some() {
            if std::env::var_os("AYY_TRACE_PRIVATE_BRAND_COMPILE").is_some() {
                eprintln!(
                    "private_brand_direct_value current_fn={:?} binding={binding_name} source=kind",
                    self.current_function_name(),
                );
            }
            self.emit_numeric_expression(&Expression::Identifier(binding_name.to_string()))?;
            return Ok(());
        }

        self.emit_numeric_expression(&Expression::Identifier(binding_name.to_string()))
    }

    pub(in crate::backend::direct_wasm) fn emit_private_brand_runtime_value_for_binding_name(
        &mut self,
        binding_name: &str,
    ) -> DirectResult<bool> {
        if self.resolve_current_local_binding(binding_name).is_some()
            || self.resolve_global_binding_index(binding_name).is_some()
            || self.hidden_implicit_global_binding(binding_name).is_some()
            || self.lookup_identifier_kind(binding_name).is_some()
        {
            if std::env::var_os("AYY_TRACE_PRIVATE_BRAND_COMPILE").is_some() {
                eprintln!(
                    "private_brand_value_source current_fn={:?} binding={binding_name} source=direct",
                    self.current_function_name(),
                );
            }
            return Ok(false);
        }
        if let Some(hidden_name) = self.resolve_user_function_capture_hidden_name(binding_name) {
            if std::env::var_os("AYY_TRACE_PRIVATE_BRAND_COMPILE").is_some() {
                eprintln!(
                    "private_brand_value_source current_fn={:?} binding={binding_name} source=hidden hidden={hidden_name}",
                    self.current_function_name(),
                );
            }
            let binding = self
                .implicit_global_binding(&hidden_name)
                .unwrap_or_else(|| self.ensure_implicit_global_binding(&hidden_name));
            self.push_global_get(binding.present_index);
            self.state.emission.output.instructions.push(0x04);
            self.state.emission.output.instructions.push(I32_TYPE);
            self.push_control_frame();
            self.push_global_get(binding.value_index);
            self.state.emission.output.instructions.push(0x05);
            self.emit_numeric_expression(&Expression::Identifier(binding_name.to_string()))?;
            self.state.emission.output.instructions.push(0x0b);
            self.pop_control_frame();
            return Ok(true);
        }
        Ok(false)
    }

    pub(in crate::backend::direct_wasm) fn emit_current_private_brand_value(
        &mut self,
    ) -> DirectResult<bool> {
        let Some(binding_name) = self.current_private_brand_binding_name() else {
            return Ok(false);
        };
        if self.emit_private_brand_runtime_value_for_binding_name(&binding_name)? {
            return Ok(true);
        }
        self.emit_private_brand_direct_or_synthetic_runtime_value_for_binding_name(&binding_name)?;
        Ok(true)
    }

    pub(in crate::backend::direct_wasm) fn emit_private_brand_marker_runtime_value(
        &mut self,
        object: &Expression,
        property: &Expression,
        value: &Expression,
    ) -> DirectResult<bool> {
        if !self.is_private_member_read_property(property) {
            return Ok(false);
        }

        if matches!(object, Expression::This)
            || self
                .resolve_bound_alias_expression(object)
                .is_some_and(|resolved| matches!(resolved, Expression::This))
        {
            if self.emit_current_private_brand_value()? {
                return Ok(true);
            }
        }
        let marker_matches_private_brand = match value {
            Expression::Object(entries) => entries.is_empty(),
            Expression::Identifier(name) => name.starts_with("__ayy_class_brand_"),
            _ => false,
        };
        let capture_marker_binding = match value {
            Expression::Identifier(name) => self
                .resolve_capture_slot_source_binding_name(name)
                .or_else(|| self.resolve_capture_hidden_source_binding_name(name))
                .filter(|source_name| source_name.starts_with("__ayy_class_brand_"))
                .map(|_| name.clone()),
            _ => None,
        };
        if capture_marker_binding.is_some() {
            self.emit_numeric_expression(value)?;
            return Ok(true);
        }
        if marker_matches_private_brand
            && let Some(LocalFunctionBinding::User(function_name)) =
                self.resolve_constructed_object_constructor_binding(object)
            && let Some(hidden_name) = self
                .user_function_capture_bindings(&function_name)
                .and_then(|bindings| {
                    bindings.iter().find_map(|(capture_name, hidden_name)| {
                        capture_name
                            .starts_with("__ayy_class_brand_")
                            .then_some(hidden_name.clone())
                    })
                })
        {
            self.emit_private_brand_direct_or_synthetic_runtime_value_for_binding_name(
                &hidden_name,
            )?;
            return Ok(true);
        }

        let Some(capture_slots) = self.resolve_member_function_capture_slots(object, property)
        else {
            return Ok(false);
        };
        let mut private_brand_slots = capture_slots
            .iter()
            .filter(|(capture_name, _)| capture_name.starts_with("__ayy_class_brand_"));
        let Some((private_brand_binding, slot_name)) = private_brand_slots.next() else {
            return Ok(false);
        };
        if private_brand_slots.next().is_some() {
            return Ok(false);
        }
        let marker_matches_private_brand = marker_matches_private_brand
            || matches!(
                value,
                Expression::Identifier(name)
                    if name == private_brand_binding || name == slot_name
            );
        if !marker_matches_private_brand {
            return Ok(false);
        }

        self.emit_private_brand_direct_or_synthetic_runtime_value_for_binding_name(slot_name)?;
        Ok(true)
    }

    fn local_function_binding_runtime_value(&self, binding: &LocalFunctionBinding) -> Option<i32> {
        match binding {
            LocalFunctionBinding::User(function_name) => self
                .user_function(function_name)
                .map(user_function_runtime_value),
            LocalFunctionBinding::Builtin(function_name) => {
                builtin_function_runtime_value(function_name)
            }
        }
    }

    fn emit_function_binding_private_brand_runtime_value(
        &mut self,
        binding: &LocalFunctionBinding,
    ) -> DirectResult<bool> {
        let LocalFunctionBinding::User(function_name) = binding else {
            return Ok(false);
        };
        let Some(private_brand_binding) = self
            .user_function(function_name)
            .and_then(|function| function.private_brand_binding.clone())
        else {
            return Ok(false);
        };
        if self.emit_private_brand_runtime_value_for_binding_name(&private_brand_binding)? {
            return Ok(true);
        }
        self.emit_private_brand_direct_or_synthetic_runtime_value_for_binding_name(
            &private_brand_binding,
        )?;
        Ok(true)
    }

    pub(in crate::backend::direct_wasm) fn emit_private_member_binding_match_from_local(
        &mut self,
        binding: &LocalFunctionBinding,
        value_local: u32,
    ) -> DirectResult<()> {
        let matches_local = self.allocate_temp_local();
        self.push_i32_const(0);
        self.push_local_set(matches_local);

        if let Some(expected_value) = self.local_function_binding_runtime_value(binding) {
            self.push_local_get(value_local);
            self.push_i32_const(expected_value);
            self.push_binary_op(BinaryOp::Equal)?;
            self.state.emission.output.instructions.push(0x04);
            self.state.emission.output.instructions.push(I32_TYPE);
            self.push_control_frame();
            self.push_i32_const(1);
            self.state.emission.output.instructions.push(0x05);
            self.push_local_get(matches_local);
            self.state.emission.output.instructions.push(0x0b);
            self.pop_control_frame();
            self.push_local_set(matches_local);
        }

        if self.emit_current_private_brand_value()? {
            self.push_local_get(value_local);
            self.push_binary_op(BinaryOp::Equal)?;
            self.state.emission.output.instructions.push(0x04);
            self.state.emission.output.instructions.push(I32_TYPE);
            self.push_control_frame();
            self.push_i32_const(1);
            self.state.emission.output.instructions.push(0x05);
            self.push_local_get(matches_local);
            self.state.emission.output.instructions.push(0x0b);
            self.pop_control_frame();
            self.push_local_set(matches_local);
        }

        if self.emit_function_binding_private_brand_runtime_value(binding)? {
            self.push_local_get(value_local);
            self.push_binary_op(BinaryOp::Equal)?;
            self.state.emission.output.instructions.push(0x04);
            self.state.emission.output.instructions.push(I32_TYPE);
            self.push_control_frame();
            self.push_i32_const(1);
            self.state.emission.output.instructions.push(0x05);
            self.push_local_get(matches_local);
            self.state.emission.output.instructions.push(0x0b);
            self.pop_control_frame();
            self.push_local_set(matches_local);
        }

        self.push_local_get(matches_local);
        Ok(())
    }

    fn emit_private_function_binding_match_from_local(
        &mut self,
        binding: &LocalFunctionBinding,
        value_local: u32,
    ) -> DirectResult<()> {
        self.emit_private_member_binding_match_from_local(binding, value_local)
    }

    fn resolve_current_private_getter_binding(
        &self,
        property: &Expression,
    ) -> Option<(LocalFunctionBinding, Option<BTreeMap<String, String>>)> {
        if !self.is_private_member_read_property(property) {
            return None;
        }
        self.resolve_member_getter_binding(&Expression::This, property)
            .map(|binding| {
                let capture_slots =
                    self.resolve_member_function_capture_slots(&Expression::This, property);
                (binding, capture_slots)
            })
    }

    fn resolve_current_private_method_binding(
        &self,
        property: &Expression,
    ) -> Option<LocalFunctionBinding> {
        self.is_private_member_read_property(property)
            .then(|| self.resolve_member_function_binding(&Expression::This, property))?
    }

    fn resolve_current_private_setter_binding(
        &self,
        property: &Expression,
    ) -> Option<LocalFunctionBinding> {
        self.is_private_member_read_property(property)
            .then(|| self.resolve_member_setter_binding(&Expression::This, property))?
    }

    fn emit_private_data_field_brand_check_against_local(
        &mut self,
        marker_local: u32,
        field_presence_fallback: Option<ImplicitGlobalBinding>,
    ) -> DirectResult<()> {
        let expected_local = self.allocate_temp_local();
        if !self.emit_current_private_brand_value()? {
            if std::env::var_os("AYY_TRACE_PRIVATE_MEMBER_VALUES").is_some() {
                self.emit_runtime_shadow_debug_print_local(
                    "private_brand_check_marker_without_expected",
                    marker_local,
                )?;
            }
            return Ok(());
        }
        self.push_local_set(expected_local);
        if std::env::var_os("AYY_TRACE_PRIVATE_MEMBER_VALUES").is_some() {
            self.emit_runtime_shadow_debug_print_local("private_brand_check_marker", marker_local)?;
            self.emit_runtime_shadow_debug_print_local(
                "private_brand_check_expected",
                expected_local,
            )?;
        }
        self.push_local_get(marker_local);
        self.push_local_get(expected_local);
        self.push_binary_op(BinaryOp::Equal)?;
        self.push_i32_const(0);
        self.push_binary_op(BinaryOp::Equal)?;
        self.state.emission.output.instructions.push(0x04);
        self.state
            .emission
            .output
            .instructions
            .push(EMPTY_BLOCK_TYPE);
        self.push_control_frame();
        if std::env::var_os("AYY_TRACE_PRIVATE_BRAND_COMPILE").is_some() {
            eprintln!(
                "private_brand_check_throw current_fn={:?} instruction={} marker_local={marker_local}",
                self.current_function_name(),
                self.state.emission.output.instructions.len(),
            );
        }
        if let Some(field_presence_fallback) = field_presence_fallback {
            self.push_global_get(field_presence_fallback.present_index);
            self.state.emission.output.instructions.push(0x04);
            self.state
                .emission
                .output
                .instructions
                .push(EMPTY_BLOCK_TYPE);
            self.push_control_frame();
            self.state.emission.output.instructions.push(0x05);
            self.emit_named_error_throw("TypeError")?;
            self.state.emission.output.instructions.push(0x0b);
            self.pop_control_frame();
        } else {
            self.emit_named_error_throw("TypeError")?;
        }
        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();
        Ok(())
    }

    fn emit_private_data_field_brand_check_from_marker_expression(
        &mut self,
        marker_value: &Expression,
    ) -> DirectResult<()> {
        let marker_local = self.allocate_temp_local();
        self.emit_numeric_expression(marker_value)?;
        self.push_local_set(marker_local);
        self.emit_private_data_field_brand_check_against_local(marker_local, None)
    }

    pub(in crate::backend::direct_wasm) fn emit_private_member_assignment_target_base_or_throw(
        &mut self,
        object: &Expression,
    ) -> DirectResult<()> {
        if matches!(object, Expression::This) {
            self.emit_numeric_expression(object)?;
            self.state.emission.output.instructions.push(0x1a);
        }
        Ok(())
    }

    pub(in crate::backend::direct_wasm) fn emit_private_data_field_brand_check_after_base_or_throw(
        &mut self,
        object: &Expression,
        property: &Expression,
    ) -> DirectResult<()> {
        let Some(marker_property) = private_brand_marker_property_expression(property) else {
            return self.emit_named_error_throw("TypeError");
        };
        let static_marker = self
            .resolve_object_binding_from_expression(object)
            .and_then(|object_binding| {
                self.resolve_object_binding_property_value(&object_binding, &marker_property)
            });
        if std::env::var_os("AYY_TRACE_PRIVATE_MEMBER_VALUES").is_some() {
            eprintln!(
                "private_brand_check_setup current_fn={:?} object={object:?} property={property:?} marker_property={marker_property:?} static_marker={static_marker:?}",
                self.current_function_name(),
            );
        }
        let runtime_binding =
            self.resolve_runtime_object_property_shadow_binding(object, &marker_property);
        let deleted_binding =
            self.resolve_runtime_object_property_shadow_deleted_binding(object, &marker_property);
        let private_field_runtime_binding =
            self.resolve_runtime_object_property_shadow_binding(object, property);

        if let Some(deleted_binding) = deleted_binding {
            self.push_global_get(deleted_binding.present_index);
            self.state.emission.output.instructions.push(0x04);
            self.state
                .emission
                .output
                .instructions
                .push(EMPTY_BLOCK_TYPE);
            self.push_control_frame();
            if std::env::var_os("AYY_TRACE_PRIVATE_MEMBER_VALUES").is_some() {
                self.emit_print(&[Expression::String(
                    "private_brand_check_deleted_marker".to_string(),
                )])?;
            }
            self.emit_named_error_throw("TypeError")?;
            self.state.emission.output.instructions.push(0x0b);
            self.pop_control_frame();
        }

        let emit_static_or_throw = |compiler: &mut Self| -> DirectResult<()> {
            if let Some(static_marker) = static_marker.as_ref() {
                if std::env::var_os("AYY_TRACE_PRIVATE_BRAND_COMPILE").is_some() {
                    eprintln!(
                        "private_brand_check_static_marker current_fn={:?} property={property:?} instruction={}",
                        compiler.current_function_name(),
                        compiler.state.emission.output.instructions.len(),
                    );
                }
                compiler.emit_private_data_field_brand_check_from_marker_expression(static_marker)
            } else {
                if std::env::var_os("AYY_TRACE_PRIVATE_MEMBER_VALUES").is_some() {
                    compiler.emit_print(&[Expression::String(
                        "private_brand_check_missing_marker".to_string(),
                    )])?;
                }
                compiler.emit_named_error_throw("TypeError")
            }
        };

        let emit_field_presence_or_static_or_throw = |compiler: &mut Self| -> DirectResult<()> {
            let Some(private_field_runtime_binding) = private_field_runtime_binding else {
                return emit_static_or_throw(compiler);
            };

            compiler.push_global_get(private_field_runtime_binding.present_index);
            compiler.state.emission.output.instructions.push(0x04);
            compiler
                .state
                .emission
                .output
                .instructions
                .push(EMPTY_BLOCK_TYPE);
            compiler.push_control_frame();
            if std::env::var_os("AYY_TRACE_PRIVATE_MEMBER_VALUES").is_some() {
                compiler.emit_print(&[Expression::String(
                    "private_brand_check_field_shadow_present".to_string(),
                )])?;
            }
            compiler.state.emission.output.instructions.push(0x05);
            emit_static_or_throw(compiler)?;
            compiler.state.emission.output.instructions.push(0x0b);
            compiler.pop_control_frame();
            Ok(())
        };

        let Some(runtime_binding) = runtime_binding else {
            return emit_field_presence_or_static_or_throw(self);
        };

        self.push_global_get(runtime_binding.present_index);
        self.state.emission.output.instructions.push(0x04);
        self.state
            .emission
            .output
            .instructions
            .push(EMPTY_BLOCK_TYPE);
        self.push_control_frame();
        let marker_local = self.allocate_temp_local();
        self.push_global_get(runtime_binding.value_index);
        self.push_local_set(marker_local);
        if std::env::var_os("AYY_TRACE_PRIVATE_BRAND_COMPILE").is_some() {
            eprintln!(
                "private_brand_check_runtime_marker current_fn={:?} property={property:?} instruction={} marker_local={marker_local}",
                self.current_function_name(),
                self.state.emission.output.instructions.len(),
            );
        }
        self.emit_private_data_field_brand_check_against_local(
            marker_local,
            private_field_runtime_binding,
        )?;
        self.state.emission.output.instructions.push(0x05);
        emit_field_presence_or_static_or_throw(self)?;
        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();
        Ok(())
    }

    pub(in crate::backend::direct_wasm) fn emit_private_data_field_brand_check_or_throw(
        &mut self,
        object: &Expression,
        property: &Expression,
    ) -> DirectResult<()> {
        self.emit_private_member_assignment_target_base_or_throw(object)?;
        self.emit_private_data_field_brand_check_after_base_or_throw(object, property)
    }

    fn emit_private_member_binding_value_from_local(
        &mut self,
        object: &Expression,
        property: &Expression,
        value_local: u32,
    ) -> DirectResult<()> {
        if std::env::var_os("AYY_TRACE_PRIVATE_MEMBER_LOOKUP").is_some() {
            eprintln!(
                "private_runtime_local_read current_fn={:?} object={object:?} property={property:?} getter={:?} method={:?}",
                self.current_function_name(),
                self.resolve_current_private_getter_binding(property)
                    .as_ref()
                    .map(|(binding, _)| binding),
                self.resolve_current_private_method_binding(property),
            );
        }
        if let Some((binding, capture_slots)) =
            self.resolve_current_private_getter_binding(property)
        {
            self.emit_private_member_binding_match_from_local(&binding, value_local)?;
            self.state.emission.output.instructions.push(0x04);
            self.state.emission.output.instructions.push(I32_TYPE);
            self.push_control_frame();
            match binding {
                LocalFunctionBinding::User(function_name) => {
                    self.emit_member_getter_call_with_bound_this(
                        &function_name,
                        object,
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
            self.state.emission.output.instructions.push(0x05);
            self.emit_named_error_throw("TypeError")?;
            self.state.emission.output.instructions.push(0x0b);
            self.pop_control_frame();
            return Ok(());
        }

        if let Some(binding) = self.resolve_current_private_method_binding(property) {
            self.emit_private_function_binding_match_from_local(&binding, value_local)?;
            self.state.emission.output.instructions.push(0x04);
            self.state.emission.output.instructions.push(I32_TYPE);
            self.push_control_frame();
            if let Some(runtime_value) = self.local_function_binding_runtime_value(&binding) {
                self.push_i32_const(runtime_value);
            } else {
                self.push_local_get(value_local);
            }
            self.state.emission.output.instructions.push(0x05);
            self.emit_named_error_throw("TypeError")?;
            self.state.emission.output.instructions.push(0x0b);
            self.pop_control_frame();
            return Ok(());
        }

        if self
            .resolve_current_private_setter_binding(property)
            .is_some()
        {
            return self.emit_named_error_throw("TypeError");
        }

        self.emit_private_data_field_brand_check_or_throw(object, property)?;
        self.push_local_get(value_local);
        Ok(())
    }

    fn emit_private_member_fallback_function_binding_read(
        &mut self,
        object: &Expression,
        property: &Expression,
        function_binding: &LocalFunctionBinding,
        capture_slots: Option<&BTreeMap<String, String>>,
    ) -> DirectResult<()> {
        if std::env::var_os("AYY_TRACE_PRIVATE_MEMBER_LOOKUP").is_some() {
            eprintln!(
                "private_runtime_fallback_read current_fn={:?} object={object:?} property={property:?} binding={function_binding:?} getter={:?} method={:?}",
                self.current_function_name(),
                self.resolve_current_private_getter_binding(property)
                    .as_ref()
                    .map(|(binding, _)| binding),
                self.resolve_current_private_method_binding(property),
            );
        }
        if let Some((expected_binding, expected_capture_slots)) =
            self.resolve_current_private_getter_binding(property)
        {
            if expected_binding != *function_binding {
                return self.emit_named_error_throw("TypeError");
            }
            match expected_binding {
                LocalFunctionBinding::User(function_name) => self
                    .emit_member_getter_call_with_bound_this(
                        &function_name,
                        object,
                        expected_capture_slots.as_ref().or(capture_slots),
                    ),
                LocalFunctionBinding::Builtin(function_name) => {
                    let callee = Expression::Identifier(function_name);
                    if !self.emit_arguments_slot_accessor_call(&callee, &[], 0, Some(&[]))? {
                        self.push_i32_const(JS_UNDEFINED_TAG);
                    }
                    Ok(())
                }
            }?;
            return Ok(());
        }

        if let Some(expected_binding) = self.resolve_current_private_method_binding(property) {
            if expected_binding != *function_binding {
                return self.emit_named_error_throw("TypeError");
            }
            if let Some(runtime_value) =
                self.local_function_binding_runtime_value(&expected_binding)
            {
                self.push_i32_const(runtime_value);
                return Ok(());
            }
            return self.emit_named_error_throw("TypeError");
        }

        self.emit_named_error_throw("TypeError")
    }

    pub(super) fn emit_runtime_or_object_member_read_without_prelude(
        &mut self,
        object: &Expression,
        property: &Expression,
        static_array_property: &Expression,
    ) -> DirectResult<bool> {
        if self.emit_runtime_descriptor_member_read(object, property)? {
            if std::env::var_os("AYY_TRACE_RUNTIME_SHADOWS").is_some() {
                eprintln!(
                    "runtime_shadow_member_branch descriptor object={object:?} property={property:?}"
                );
            }
            return Ok(true);
        }
        if self.emit_runtime_array_member_read(object, static_array_property)? {
            if std::env::var_os("AYY_TRACE_RUNTIME_SHADOWS").is_some() {
                eprintln!(
                    "runtime_shadow_member_branch array object={object:?} property={property:?}"
                );
            }
            return Ok(true);
        }
        if self.emit_runtime_object_shadow_member_read(object, property)? {
            if std::env::var_os("AYY_TRACE_RUNTIME_SHADOWS").is_some() {
                eprintln!(
                    "runtime_shadow_member_branch shadow object={object:?} property={property:?}"
                );
            }
            return Ok(true);
        }
        if self.emit_runtime_object_binding_member_read(object, property)? {
            if std::env::var_os("AYY_TRACE_RUNTIME_SHADOWS").is_some() {
                eprintln!(
                    "runtime_shadow_member_branch object object={object:?} property={property:?}"
                );
            }
            return Ok(true);
        }
        if self.emit_runtime_string_member_read(object, property)? {
            if std::env::var_os("AYY_TRACE_RUNTIME_SHADOWS").is_some() {
                eprintln!(
                    "runtime_shadow_member_branch string object={object:?} property={property:?}"
                );
            }
            return Ok(true);
        }
        if self.emit_runtime_arguments_member_read(object, property)? {
            if std::env::var_os("AYY_TRACE_RUNTIME_SHADOWS").is_some() {
                eprintln!(
                    "runtime_shadow_member_branch arguments object={object:?} property={property:?}"
                );
            }
            return Ok(true);
        }
        if self.emit_runtime_native_error_member_read(object, property)? {
            if std::env::var_os("AYY_TRACE_RUNTIME_SHADOWS").is_some() {
                eprintln!(
                    "runtime_shadow_member_branch native_error object={object:?} property={property:?}"
                );
            }
            return Ok(true);
        }
        if self.emit_runtime_returned_or_function_member_read(object, property)? {
            if std::env::var_os("AYY_TRACE_RUNTIME_SHADOWS").is_some() {
                eprintln!(
                    "runtime_shadow_member_branch returned_or_function object={object:?} property={property:?}"
                );
            }
            return Ok(true);
        }
        if self.resolve_array_binding_from_expression(object).is_some() {
            if std::env::var_os("AYY_TRACE_RUNTIME_SHADOWS").is_some() {
                eprintln!(
                    "runtime_shadow_member_branch array_undefined object={object:?} property={property:?}"
                );
            }
            self.push_i32_const(JS_UNDEFINED_TAG);
            return Ok(true);
        }
        Ok(false)
    }
}
