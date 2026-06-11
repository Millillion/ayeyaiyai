use super::*;

impl<'a> FunctionCompiler<'a> {
    pub(super) fn emit_runtime_native_error_member_read(
        &mut self,
        object: &Expression,
        property: &Expression,
    ) -> DirectResult<bool> {
        if !matches!(property, Expression::String(name) if name == "name" || name == "constructor")
            || !inline_summary_side_effect_free_expression(object)
        {
            return Ok(false);
        }
        let property_name = match property {
            Expression::String(name) => name.as_str(),
            _ => return Ok(false),
        };

        let mut open_frames = 0;
        for name in NATIVE_ERROR_NAMES {
            let Some(value) = native_error_runtime_value(name) else {
                continue;
            };
            self.emit_numeric_expression(object)?;
            self.push_i32_const(value);
            self.push_binary_op(BinaryOp::Equal)?;
            self.state.emission.output.instructions.push(0x04);
            self.state.emission.output.instructions.push(I32_TYPE);
            self.push_control_frame();
            match property_name {
                "name" => self.emit_static_string_literal(name)?,
                "constructor" => self.push_i32_const(
                    builtin_function_runtime_value(name).unwrap_or(JS_TYPEOF_FUNCTION_TAG),
                ),
                _ => unreachable!("native error member read prefilter limits properties"),
            }
            self.state.emission.output.instructions.push(0x05);
            open_frames += 1;
        }

        if open_frames == 0 {
            return Ok(false);
        }

        if !self.emit_runtime_user_function_property_read(object, property)? {
            let fallback = if property_name == "constructor" {
                JS_TYPEOF_FUNCTION_TAG
            } else {
                JS_TYPEOF_OBJECT_TAG
            };
            self.push_i32_const(fallback);
        }
        for _ in 0..open_frames {
            self.state.emission.output.instructions.push(0x0b);
            self.pop_control_frame();
        }
        Ok(true)
    }

    pub(super) fn emit_runtime_string_member_read(
        &mut self,
        object: &Expression,
        property: &Expression,
    ) -> DirectResult<bool> {
        let resolved_text = match object {
            Expression::String(text) => Some(text.clone()),
            _ => self.resolve_static_string_value(object),
        };
        let Some(text) = resolved_text else {
            return Ok(false);
        };
        if let Some(index) = argument_index_from_expression(property) {
            if let Some(character) = text.chars().nth(index as usize) {
                self.emit_numeric_expression(&Expression::String(character.to_string()))?;
            } else {
                self.push_i32_const(JS_UNDEFINED_TAG);
            }
            return Ok(true);
        }
        if matches!(property, Expression::String(name) if name == "length") {
            self.push_i32_const(text.encode_utf16().count() as i32);
            return Ok(true);
        }
        Ok(false)
    }

    pub(super) fn emit_runtime_arguments_member_read(
        &mut self,
        object: &Expression,
        property: &Expression,
    ) -> DirectResult<bool> {
        if let Some(arguments_binding) = self.resolve_arguments_binding_from_expression(object) {
            if matches!(property, Expression::String(text) if text == "length") {
                if !arguments_binding.length_present {
                    self.push_i32_const(JS_UNDEFINED_TAG);
                } else {
                    self.emit_numeric_expression(&arguments_binding.length_value)?;
                }
                return Ok(true);
            }
            if matches!(property, Expression::String(property_name) if property_name == "callee") {
                if arguments_binding.strict {
                    return self.emit_error_throw().map(|()| true);
                }
                if !arguments_binding.callee_present {
                    self.push_i32_const(JS_UNDEFINED_TAG);
                } else if let Some(value) = arguments_binding.callee_value.as_ref() {
                    self.emit_numeric_expression(value)?;
                } else {
                    self.push_i32_const(JS_UNDEFINED_TAG);
                }
                return Ok(true);
            }
            if let Some(index) = argument_index_from_expression(property) {
                if let Some(value) = arguments_binding.values.get(index as usize) {
                    self.emit_numeric_expression(value)?;
                } else {
                    self.push_i32_const(JS_UNDEFINED_TAG);
                }
                return Ok(true);
            }
            self.emit_dynamic_arguments_binding_property_read(&arguments_binding, property)?;
            return Ok(true);
        }

        if self.is_direct_arguments_object(object) {
            if matches!(property, Expression::String(text) if text == "length") {
                self.emit_direct_arguments_length()?;
                return Ok(true);
            }
            if matches!(property, Expression::String(text) if text == "callee") {
                self.emit_direct_arguments_callee()?;
                return Ok(true);
            }
            if let Some(index) = argument_index_from_expression(property) {
                self.emit_arguments_slot_read(index)?;
                return Ok(true);
            }
            self.emit_dynamic_direct_arguments_property_read(property)?;
            return Ok(true);
        }

        Ok(false)
    }

    /// `args = arguments;` inside a function stores the live arguments object
    /// into a global; the paired store path materializes length and indexed
    /// slots into the target's global runtime array state channel. Serve
    /// later reads of those own properties from that channel. The channel
    /// globals are keyed by name, so this stays correct regardless of the
    /// order the store and read sites are compiled in.
    pub(super) fn emit_stored_arguments_global_member_read(
        &mut self,
        object: &Expression,
        property: &Expression,
    ) -> DirectResult<bool> {
        let Expression::Identifier(name) = object else {
            return Ok(false);
        };
        let is_length = matches!(property, Expression::String(text) if text == "length");
        let index = argument_index_from_expression(property);
        if !is_length && index.is_none() {
            return Ok(false);
        }
        if !self.global_binding_may_hold_stored_arguments_object(name) {
            return Ok(false);
        }
        if crate::ayy_env_flag!("AYY_TRACE_RUNTIME_SHADOWS")
            || crate::ayy_env_flag!("AYY_TRACE_MEMBER_READS")
        {
            eprintln!(
                "runtime_shadow_member_branch stored_arguments object={object:?} property={property:?}"
            );
        }
        let binding = if is_length {
            self.global_runtime_array_length_binding(name)
        } else {
            self.global_runtime_array_slot_binding(name, index.unwrap_or(0))
        };
        self.push_global_get(binding.present_index);
        self.state.emission.output.instructions.push(0x04);
        self.state.emission.output.instructions.push(I32_TYPE);
        self.push_control_frame();
        self.push_global_get(binding.value_index);
        self.state.emission.output.instructions.push(0x05);
        self.push_i32_const(JS_UNDEFINED_TAG);
        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();
        Ok(true)
    }

    fn global_binding_may_hold_stored_arguments_object(&self, name: &str) -> bool {
        if self.resolve_current_local_binding(name).is_some() {
            return false;
        }
        if self.backend.global_binding_index(name).is_none()
            && !self.backend.global_has_lexical_binding(name)
            && !self.backend.global_has_implicit_binding(name)
        {
            return false;
        }
        self.backend
            .function_registry
            .user_functions()
            .iter()
            .any(|user_function| {
                if user_function.params.iter().any(|param| {
                    param == "arguments"
                        || scoped_binding_source_name(param)
                            .is_some_and(|source| source == "arguments")
                }) || user_function.body_declares_arguments_binding
                {
                    return false;
                }
                self.backend
                    .function_registry
                    .registered_function(&user_function.name)
                    .is_some_and(|declaration| {
                        statements_assign_arguments_to_binding(&declaration.body, name)
                    })
            })
    }

    pub(super) fn emit_runtime_returned_or_function_member_read(
        &mut self,
        object: &Expression,
        property: &Expression,
    ) -> DirectResult<bool> {
        if let Some(returned_value) =
            self.resolve_returned_member_value_from_expression(object, property)
        {
            self.emit_numeric_expression(&returned_value)?;
            return Ok(true);
        }
        if self.emit_runtime_user_function_property_read(object, property)? {
            return Ok(true);
        }
        if matches!(property, Expression::String(text) if text == "constructor") {
            if let Some(binding) = self.resolve_constructed_object_constructor_binding(object) {
                match binding {
                    LocalFunctionBinding::User(function_name) => {
                        if let Some(user_function) = self.user_function(&function_name) {
                            self.push_i32_const(user_function_runtime_value(user_function));
                        } else {
                            self.push_i32_const(JS_TYPEOF_FUNCTION_TAG);
                        }
                    }
                    LocalFunctionBinding::Builtin(function_name) => {
                        self.push_i32_const(
                            builtin_function_runtime_value(&function_name)
                                .unwrap_or(JS_TYPEOF_FUNCTION_TAG),
                        );
                    }
                }
            } else {
                self.push_i32_const(JS_TYPEOF_FUNCTION_TAG);
            }
            return Ok(true);
        }
        Ok(false)
    }
}

fn statements_assign_arguments_to_binding(statements: &[Statement], name: &str) -> bool {
    struct ArgumentsStoreScan<'a> {
        name: &'a str,
        found: bool,
    }

    impl<'a> crate::ir::visit::Visitor for ArgumentsStoreScan<'a> {
        fn visit_statement(&mut self, statement: &Statement) {
            if self.found {
                return;
            }
            if let Statement::Assign { name, value } = statement
                && (name == self.name
                    || scoped_binding_source_name(name).is_some_and(|source| source == self.name))
                && matches!(value, Expression::Identifier(source) if source == "arguments")
            {
                self.found = true;
                return;
            }
            crate::ir::visit::walk_statement(self, statement);
        }

        fn visit_expression(&mut self, expression: &Expression) {
            if self.found {
                return;
            }
            if let Expression::Assign { name, value } = expression
                && (name == self.name
                    || scoped_binding_source_name(name).is_some_and(|source| source == self.name))
                && matches!(value.as_ref(), Expression::Identifier(source) if source == "arguments")
            {
                self.found = true;
                return;
            }
            crate::ir::visit::walk_expression(self, expression);
        }
    }

    let mut scan = ArgumentsStoreScan { name, found: false };
    for statement in statements {
        crate::ir::visit::Visitor::visit_statement(&mut scan, statement);
        if scan.found {
            return true;
        }
    }
    false
}
