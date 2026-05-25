use super::*;

const NULL_SUPER_CONSTRUCTOR_BINDING: &str = "__ayy_null_super_constructor";

impl<'a> FunctionCompiler<'a> {
    fn should_prepare_identifier_function_captures_on_read(&self, name: &str) -> bool {
        !(name.starts_with("__ayy_module_export_getter_")
            && self
                .current_function_name()
                .is_some_and(|function_name| function_name.starts_with("__ayy_module_init_")))
    }

    pub(in crate::backend::direct_wasm) fn emit_declared_global_binding_read(
        &mut self,
        name: &str,
        global_index: u32,
    ) -> DirectResult<()> {
        if let Some(binding) = self.backend.lexical_global_binding(name) {
            self.push_global_get(binding.initialized_index);
            self.state.emission.output.instructions.push(0x04);
            self.state.emission.output.instructions.push(I32_TYPE);
            self.push_control_frame();
            self.push_global_get(global_index);
            self.state.emission.output.instructions.push(0x05);
            self.emit_named_error_throw("ReferenceError")?;
            self.state.emission.output.instructions.push(0x0b);
            self.pop_control_frame();
            return Ok(());
        }

        self.push_global_get(global_index);
        Ok(())
    }

    pub(in crate::backend::direct_wasm) fn emit_plain_identifier_read_fallback(
        &mut self,
        name: &str,
    ) -> DirectResult<()> {
        let trace_identifier_reads = std::env::var_os("AYY_TRACE_IDENTIFIER_READS").is_some();
        if trace_identifier_reads {
            eprintln!(
                "identifier_read:fallback:start current_fn={:?} name={name}",
                self.current_function_name()
            );
        }
        if self.emit_eval_lexical_binding_read(name)? {
            if trace_identifier_reads {
                eprintln!("identifier_read:fallback:path eval_lexical name={name}");
            }
            return Ok(());
        }
        if self.emit_parameter_default_binding_read(name)? {
            if trace_identifier_reads {
                eprintln!("identifier_read:fallback:path parameter_default name={name}");
            }
            return Ok(());
        }
        if let Some(parameter_scope_arguments_local) =
            self.parameter_scope_arguments_local_for(name)
        {
            if trace_identifier_reads {
                eprintln!("identifier_read:fallback:path parameter_scope_arguments name={name}");
            }
            self.push_local_get(parameter_scope_arguments_local);
        } else if parse_test262_realm_identifier(name).is_some()
            || parse_test262_realm_global_identifier(name).is_some()
        {
            if trace_identifier_reads {
                eprintln!("identifier_read:fallback:path test262_realm name={name}");
            }
            self.push_i32_const(JS_TYPEOF_OBJECT_TAG);
        } else if self.is_current_rest_parameter_binding_name(name) {
            if trace_identifier_reads {
                eprintln!("identifier_read:fallback:path current_rest name={name}");
            }
            self.push_i32_const(JS_TYPEOF_OBJECT_TAG);
        } else if let Some((_, local_index)) = self.resolve_current_local_binding(name) {
            if trace_identifier_reads {
                eprintln!(
                    "identifier_read:fallback:path local name={name} local_index={local_index}",
                );
            }
            if let Some(initialized_local) = self.local_lexical_initialized_local(name) {
                self.push_local_get(initialized_local);
                self.state.emission.output.instructions.push(0x04);
                self.state.emission.output.instructions.push(I32_TYPE);
                self.push_control_frame();
                self.push_local_get(local_index);
                self.state.emission.output.instructions.push(0x05);
                self.emit_named_error_throw("ReferenceError")?;
                self.state.emission.output.instructions.push(0x0b);
                self.pop_control_frame();
            } else {
                self.push_local_get(local_index);
            }
        } else if let Some((active_name, global_index)) =
            self.resolve_active_global_lexical_binding(name)
        {
            if trace_identifier_reads {
                eprintln!(
                    "identifier_read:fallback:path active_global_lexical name={name} active_name={active_name} global_index={global_index}",
                );
            }
            self.emit_declared_global_binding_read(&active_name, global_index)?;
        } else if self.current_function_name().is_none()
            && self.backend.lexical_global_binding(name).is_some()
            && let Some(global_index) = self.global_binding_index(name)
        {
            if trace_identifier_reads {
                eprintln!(
                    "identifier_read:fallback:path top_level_global_lexical name={name} global_index={global_index}",
                );
            }
            self.emit_declared_global_binding_read(name, global_index)?;
        } else if self.is_current_arguments_binding_name(name) && self.has_arguments_object() {
            if trace_identifier_reads {
                eprintln!("identifier_read:fallback:path current_arguments name={name}");
            }
            self.push_i32_const(JS_TYPEOF_OBJECT_TAG);
        } else if let Some(function_binding) = self
            .state
            .speculation
            .static_semantics
            .local_function_binding(name)
            .cloned()
        {
            if trace_identifier_reads {
                eprintln!("identifier_read:fallback:path local_function name={name}");
            }
            match function_binding {
                LocalFunctionBinding::User(function_name) => {
                    if let Some(runtime_value) = self.user_function_runtime_value(&function_name) {
                        if self.should_prepare_identifier_function_captures_on_read(&function_name)
                        {
                            self.emit_prepare_user_function_capture_globals(&function_name)?;
                        }
                        self.push_i32_const(runtime_value);
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
        } else if self.emit_user_function_capture_binding_read(name)? {
            if trace_identifier_reads {
                eprintln!("identifier_read:fallback:path user_capture name={name}");
            }
        } else if let Some(global_index) = self.global_binding_index(name) {
            if trace_identifier_reads {
                eprintln!(
                    "identifier_read:fallback:path global name={name} global_index={global_index}",
                );
            }
            self.emit_declared_global_binding_read(name, global_index)?;
        } else if scoped_binding_source_name(name).is_some()
            && let Some(global_index) = self.resolve_global_binding_index(name)
        {
            if trace_identifier_reads {
                eprintln!(
                    "identifier_read:fallback:path scoped_global name={name} global_index={global_index}",
                );
            }
            self.emit_declared_global_binding_read(name, global_index)?;
        } else if let Some(state) = self
            .backend
            .global_property_descriptor(name)
            .or_else(|| {
                self.backend
                    .shared_global_semantics
                    .values
                    .property_descriptor(name)
            })
            .cloned()
        {
            if trace_identifier_reads {
                eprintln!("identifier_read:fallback:path global_property_descriptor name={name}");
            }
            if let Some(getter_binding) = state
                .getter
                .as_ref()
                .and_then(|getter| self.resolve_function_binding_from_expression(getter))
            {
                match getter_binding {
                    LocalFunctionBinding::User(function_name) => {
                        self.emit_member_getter_call_with_bound_this(
                            &function_name,
                            &Expression::This,
                            None,
                        )?;
                    }
                    LocalFunctionBinding::Builtin(function_name) => {
                        let callee = Expression::Identifier(function_name);
                        if !self.emit_arguments_slot_accessor_call(&callee, &[], 0, Some(&[]))? {
                            self.push_i32_const(JS_UNDEFINED_TAG);
                        }
                    }
                }
            } else {
                self.emit_numeric_expression(&state.value)?;
            }
        } else if self.emit_eval_local_function_binding_read(name)? {
            if trace_identifier_reads {
                eprintln!("identifier_read:fallback:path eval_local_function name={name}");
            }
        } else if name == "NaN" && self.is_unshadowed_builtin_identifier(name) {
            if trace_identifier_reads {
                eprintln!("identifier_read:fallback:path nan name={name}");
            }
            self.push_i32_const(JS_NAN_TAG);
        } else if name == "Infinity" && self.is_unshadowed_builtin_identifier(name) {
            if trace_identifier_reads {
                eprintln!("identifier_read:fallback:path infinity name={name}");
            }
            self.emit_numeric_expression(&Expression::Number(f64::INFINITY))?;
        } else if name == "undefined" {
            if trace_identifier_reads {
                eprintln!("identifier_read:fallback:path undefined name={name}");
            }
            self.push_i32_const(JS_UNDEFINED_TAG);
        } else if let Some(runtime_value) = builtin_function_runtime_value(name) {
            if trace_identifier_reads {
                eprintln!("identifier_read:fallback:path builtin_function name={name}");
            }
            self.push_i32_const(runtime_value);
        } else if is_internal_user_function_identifier(name)
            && let Some(runtime_value) = self.user_function_runtime_value(name)
        {
            if trace_identifier_reads {
                eprintln!("identifier_read:fallback:path internal_user_function name={name}");
            }
            if self.should_prepare_identifier_function_captures_on_read(name) {
                self.emit_prepare_user_function_capture_globals(name)?;
            }
            self.push_i32_const(runtime_value);
        } else if let Some(private_brand_offset) = name.find("__ayy_class_brand_") {
            if trace_identifier_reads {
                eprintln!("identifier_read:fallback:path synthetic_private_brand name={name}");
            }
            self.push_i32_const(Self::synthetic_private_brand_runtime_value(
                &name[private_brand_offset..],
            ));
        } else if (name.starts_with("__ayy_")
            || builtin_identifier_kind(name).is_some()
            || self.global_has_binding(name)
            || self.global_has_implicit_binding(name))
            && let Some(kind) = self.lookup_identifier_kind(name)
        {
            if trace_identifier_reads {
                let _ = kind;
                eprintln!("identifier_read:fallback:path inferred_kind name={name}");
            }
            let tag = kind.as_typeof_tag().unwrap_or(JS_UNDEFINED_TAG);
            self.push_i32_const(tag);
        } else if name.starts_with("__ayy_class_super_")
            && let Some(resolved) = self
                .resolve_static_class_init_local_alias_expression(name)
                .filter(|resolved| {
                    !static_expression_matches(resolved, &Expression::Identifier(name.to_string()))
                })
        {
            if trace_identifier_reads {
                eprintln!(
                    "identifier_read:fallback:path class_init_super_alias name={name} resolved={resolved:?}"
                );
            }
            self.emit_numeric_expression(&resolved)?;
        } else if name == NULL_SUPER_CONSTRUCTOR_BINDING {
            if trace_identifier_reads {
                eprintln!("identifier_read:fallback:path null_super_constructor name={name}");
            }
            self.emit_named_error_throw("TypeError")?;
            self.push_i32_const(JS_UNDEFINED_TAG);
        } else {
            if trace_identifier_reads {
                eprintln!("identifier_read:fallback:path missing name={name}");
            }
            self.emit_named_error_throw("ReferenceError")?;
        }
        if trace_identifier_reads {
            eprintln!("identifier_read:fallback:done name={name}");
        }
        Ok(())
    }

    pub(in crate::backend::direct_wasm) fn emit_plain_identifier_read(
        &mut self,
        name: &str,
    ) -> DirectResult<()> {
        let trace_identifier_reads = std::env::var_os("AYY_TRACE_IDENTIFIER_READS").is_some();
        if trace_identifier_reads {
            eprintln!(
                "identifier_read:start current_fn={:?} name={name}",
                self.current_function_name()
            );
        }
        if self.parameter_scope_arguments_local_for(name).is_some()
            || (self.is_current_arguments_binding_name(name) && self.has_arguments_object())
            || self.resolve_current_local_binding(name).is_some()
            || self
                .state
                .speculation
                .static_semantics
                .has_local_function_binding(name)
            || self.resolve_global_binding_index(name).is_some()
            || self
                .resolve_user_function_capture_hidden_name(name)
                .is_some()
            || self.resolve_eval_local_function_hidden_name(name).is_some()
        {
            if trace_identifier_reads {
                eprintln!("identifier_read:path direct_fallback name={name}");
            }
            return self.emit_plain_identifier_read_fallback(name);
        }

        let Some(binding) = self.backend.implicit_global_binding(name) else {
            if trace_identifier_reads {
                eprintln!("identifier_read:path no_implicit_global name={name}");
            }
            return self.emit_plain_identifier_read_fallback(name);
        };

        if trace_identifier_reads {
            eprintln!("identifier_read:path implicit_global name={name}");
        }
        self.push_global_get(binding.present_index);
        self.state.emission.output.instructions.push(0x04);
        self.state.emission.output.instructions.push(I32_TYPE);
        self.push_control_frame();
        self.push_global_get(binding.value_index);
        self.state.emission.output.instructions.push(0x05);
        self.emit_named_error_throw("ReferenceError")?;
        self.push_i32_const(JS_UNDEFINED_TAG);
        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();
        if trace_identifier_reads {
            eprintln!("identifier_read:done name={name}");
        }
        Ok(())
    }
}
