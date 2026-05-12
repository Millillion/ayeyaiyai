use super::*;

impl<'a> FunctionCompiler<'a> {
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
        if self.emit_direct_iterator_step_member_read(object, property)? {
            if trace_member_reads {
                eprintln!("member_expr:direct_iterator object={object:?} property={property:?}");
            }
            return Ok(());
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
        let effective_property = resolved_property.as_ref().unwrap_or(property);
        let result = self.emit_member_read_without_prelude(object, effective_property);
        if trace_member_reads {
            eprintln!(
                "member_expr:done object={object:?} property={effective_property:?} ok={}",
                result.is_ok()
            );
        }
        result
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
            self.emit_numeric_expression(property)?;
            self.state.emission.output.instructions.push(0x1a);
            let callee = match function_binding {
                LocalFunctionBinding::User(function_name)
                | LocalFunctionBinding::Builtin(function_name) => {
                    Expression::Identifier(function_name)
                }
            };
            if !self.emit_arguments_slot_accessor_call(&callee, &[], 0, Some(&[]))? {
                self.push_i32_const(JS_UNDEFINED_TAG);
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
