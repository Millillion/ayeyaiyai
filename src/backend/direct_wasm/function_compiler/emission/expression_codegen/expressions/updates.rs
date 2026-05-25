use super::*;

impl<'a> FunctionCompiler<'a> {
    fn update_number_from_ordinary_to_primitive_plan(
        &self,
        plan: &OrdinaryToPrimitivePlan,
    ) -> Option<f64> {
        for step in &plan.steps {
            let StaticEvalOutcome::Value(value) = &step.outcome else {
                return None;
            };
            match self.static_expression_is_non_object_primitive(value) {
                Some(true) => return self.resolve_static_number_value(value),
                Some(false) => continue,
                None => return None,
            }
        }
        None
    }

    fn emit_update_identifier_ordinary_to_primitive(
        &mut self,
        name: &str,
        op: UpdateOp,
        prefix: bool,
    ) -> DirectResult<bool> {
        let target = Expression::Identifier(name.to_string());
        if self.symbol_to_primitive_preempts_ordinary_to_primitive(
            &target,
            self.current_function_name(),
        ) {
            return Ok(false);
        }
        let Some(plan) = self.resolve_ordinary_to_primitive_plan(&target) else {
            return Ok(false);
        };
        let analysis = self.analyze_ordinary_to_primitive_plan(&plan);
        let previous_number = match analysis {
            OrdinaryToPrimitiveAnalysis::Primitive(StaticValueKind::Symbol)
            | OrdinaryToPrimitiveAnalysis::TypeError
            | OrdinaryToPrimitiveAnalysis::Throw => None,
            OrdinaryToPrimitiveAnalysis::Primitive(_) => Some(
                self.update_number_from_ordinary_to_primitive_plan(&plan)
                    .ok_or_else(|| {
                        Unsupported("update ordinary ToPrimitive result is not statically numeric")
                    })?,
            ),
            OrdinaryToPrimitiveAnalysis::Unknown => return Ok(false),
        };

        let result_local = self.allocate_temp_local();
        self.emit_numeric_expression(&target)?;
        self.push_local_set(result_local);

        match self.emit_ordinary_to_primitive_from_plan(&target, &plan, result_local)? {
            SymbolToPrimitiveHandling::AlwaysThrows => {
                self.push_i32_const(JS_UNDEFINED_TAG);
                return Ok(true);
            }
            SymbolToPrimitiveHandling::Handled => {}
            SymbolToPrimitiveHandling::NotHandled => return Ok(false),
        }

        let Some(previous_number) = previous_number else {
            self.emit_named_error_throw("TypeError")?;
            self.push_i32_const(JS_UNDEFINED_TAG);
            return Ok(true);
        };
        let increment = match op {
            UpdateOp::Increment => 1.0,
            UpdateOp::Decrement => -1.0,
        };
        let previous_expression = Expression::Number(previous_number);
        let next_expression = Expression::Number(previous_number + increment);
        let next_local = self.allocate_temp_local();
        self.emit_numeric_expression(&next_expression)?;
        self.push_local_set(next_local);
        self.emit_store_identifier_value_local(name, &next_expression, next_local)?;
        self.note_identifier_numeric_kind(name);
        if prefix {
            self.emit_numeric_expression(&next_expression)?;
        } else {
            self.emit_numeric_expression(&previous_expression)?;
        }
        Ok(true)
    }

    fn emit_update_identifier_boxed_primitive(
        &mut self,
        name: &str,
        op: UpdateOp,
        prefix: bool,
    ) -> DirectResult<bool> {
        let target = Expression::Identifier(name.to_string());
        let Some(primitive) = self.resolve_static_boxed_primitive_value(&target) else {
            return Ok(false);
        };
        let Some(previous_number) = self.resolve_static_number_value(&primitive) else {
            return Ok(false);
        };
        let increment = match op {
            UpdateOp::Increment => 1.0,
            UpdateOp::Decrement => -1.0,
        };
        let previous_expression = Expression::Number(previous_number);
        let next_expression = Expression::Number(previous_number + increment);
        let next_local = self.allocate_temp_local();
        self.emit_numeric_expression(&next_expression)?;
        self.push_local_set(next_local);
        self.emit_store_identifier_value_local(name, &next_expression, next_local)?;
        self.note_identifier_numeric_kind(name);
        if prefix {
            self.emit_numeric_expression(&next_expression)?;
        } else {
            self.emit_numeric_expression(&previous_expression)?;
        }
        Ok(true)
    }

    fn emit_update_identifier_static_number(
        &mut self,
        name: &str,
        op: UpdateOp,
        prefix: bool,
    ) -> DirectResult<bool> {
        let target = Expression::Identifier(name.to_string());
        let Some(previous_number) = self.resolve_static_number_value(&target) else {
            return Ok(false);
        };
        if std::env::var_os("AYY_TRACE_UPDATES").is_some() {
            eprintln!(
                "update:static_number current_fn={:?} name={name} previous={previous_number:?}",
                self.current_function_name()
            );
        }
        let increment = match op {
            UpdateOp::Increment => 1.0,
            UpdateOp::Decrement => -1.0,
        };
        let previous_expression = Expression::Number(previous_number);
        let next_expression = Expression::Number(previous_number + increment);
        let next_local = self.allocate_temp_local();
        self.emit_numeric_expression(&next_expression)?;
        self.push_local_set(next_local);
        self.emit_store_identifier_value_local(name, &next_expression, next_local)?;
        self.note_identifier_numeric_kind(name);
        if prefix {
            self.emit_numeric_expression(&next_expression)?;
        } else {
            self.emit_numeric_expression(&previous_expression)?;
        }
        Ok(true)
    }

    fn emit_update_identifier_static_bigint(
        &mut self,
        name: &str,
        op: UpdateOp,
        prefix: bool,
    ) -> DirectResult<bool> {
        let target = Expression::Identifier(name.to_string());
        let Some(previous_bigint) = self.resolve_static_bigint_value(&target) else {
            return Ok(false);
        };
        let unit = StaticBigInt::from(1);
        let next_bigint = match op {
            UpdateOp::Increment => &previous_bigint + &unit,
            UpdateOp::Decrement => &previous_bigint - &unit,
        };
        let previous_expression = Expression::BigInt(previous_bigint.to_string());
        let next_expression = Expression::BigInt(next_bigint.to_string());
        let next_local = self.allocate_temp_local();
        self.emit_numeric_expression(&next_expression)?;
        self.push_local_set(next_local);
        self.emit_store_identifier_value_local(name, &next_expression, next_local)?;
        if prefix {
            self.emit_numeric_expression(&next_expression)?;
        } else {
            self.emit_numeric_expression(&previous_expression)?;
        }
        Ok(true)
    }

    fn static_update_kind_is_stale_declared_global(
        &self,
        name: &str,
        previous_kind: StaticValueKind,
    ) -> bool {
        self.current_function_name().is_some()
            && previous_kind == StaticValueKind::Undefined
            && self.global_binding_index(name).is_some()
            && self.global_value_binding(name).is_none()
    }

    fn emit_runtime_update_coerced_previous_local(
        &mut self,
        value_local: u32,
        numeric_previous_local: u32,
    ) -> DirectResult<()> {
        self.push_local_get(value_local);
        self.push_i32_const(JS_UNDEFINED_TAG);
        self.push_binary_op(BinaryOp::Equal)?;
        self.state.emission.output.instructions.push(0x04);
        self.state.emission.output.instructions.push(I32_TYPE);
        self.push_control_frame();
        self.push_i32_const(JS_NAN_TAG);
        self.state.emission.output.instructions.push(0x05);

        self.push_local_get(value_local);
        self.push_i32_const(JS_NAN_TAG);
        self.push_binary_op(BinaryOp::Equal)?;
        self.state.emission.output.instructions.push(0x04);
        self.state.emission.output.instructions.push(I32_TYPE);
        self.push_control_frame();
        self.push_i32_const(JS_NAN_TAG);
        self.state.emission.output.instructions.push(0x05);

        self.push_local_get(value_local);
        self.push_i32_const(JS_NULL_TAG);
        self.push_binary_op(BinaryOp::Equal)?;
        self.state.emission.output.instructions.push(0x04);
        self.state.emission.output.instructions.push(I32_TYPE);
        self.push_control_frame();
        self.push_i32_const(0);
        self.state.emission.output.instructions.push(0x05);
        self.push_local_get(value_local);
        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();

        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();

        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();
        self.push_local_set(numeric_previous_local);
        Ok(())
    }

    fn emit_runtime_update_next_from_numeric_previous_local(
        &mut self,
        numeric_previous_local: u32,
        next_local: u32,
        opcode: u8,
    ) -> DirectResult<()> {
        self.push_local_get(numeric_previous_local);
        self.push_i32_const(JS_NAN_TAG);
        self.push_binary_op(BinaryOp::Equal)?;
        self.state.emission.output.instructions.push(0x04);
        self.state.emission.output.instructions.push(I32_TYPE);
        self.push_control_frame();
        self.push_i32_const(JS_NAN_TAG);
        self.state.emission.output.instructions.push(0x05);
        self.push_local_get(numeric_previous_local);
        self.push_i32_const(1);
        self.state.emission.output.instructions.push(opcode);
        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();
        self.push_local_set(next_local);
        Ok(())
    }

    fn emit_runtime_update_from_previous_local(
        &mut self,
        previous_local: u32,
        next_local: u32,
        opcode: u8,
    ) -> DirectResult<u32> {
        let numeric_previous_local = self.allocate_temp_local();
        self.emit_runtime_update_coerced_previous_local(previous_local, numeric_previous_local)?;
        self.emit_runtime_update_next_from_numeric_previous_local(
            numeric_previous_local,
            next_local,
            opcode,
        )?;
        Ok(numeric_previous_local)
    }

    fn sync_updated_identifier_capture_from_local(
        &mut self,
        name: &str,
        value_local: u32,
    ) -> DirectResult<()> {
        if self
            .resolve_user_function_capture_hidden_name(name)
            .is_some()
        {
            self.emit_store_user_function_capture_binding_from_local(name, value_local)?;
        }
        Ok(())
    }

    pub(super) fn emit_update_expression(
        &mut self,
        name: &str,
        op: UpdateOp,
        prefix: bool,
    ) -> DirectResult<()> {
        if let Some(scope_object) = self.resolve_with_scope_binding(name)? {
            self.emit_scoped_property_update(&scope_object, name, op, prefix)?;
            return Ok(());
        }

        let opcode = match op {
            UpdateOp::Increment => 0x6a,
            UpdateOp::Decrement => 0x6b,
        };

        let mut previous_kind = self
            .lookup_identifier_kind(name)
            .unwrap_or(StaticValueKind::Unknown);
        if self.static_update_kind_is_stale_declared_global(name, previous_kind) {
            previous_kind = StaticValueKind::Unknown;
        }
        if std::env::var_os("AYY_TRACE_UPDATES").is_some() {
            eprintln!(
                "update:start current_fn={:?} name={name} kind={previous_kind:?} global_kind={:?} global_value={:?} capture={:?}",
                self.current_function_name(),
                self.global_binding_kind(name),
                self.global_value_binding(name),
                self.resolve_user_function_capture_hidden_name(name)
            );
        }

        if matches!(
            previous_kind,
            StaticValueKind::Object | StaticValueKind::Function
        ) && self.emit_update_identifier_ordinary_to_primitive(name, op, prefix)?
        {
            return Ok(());
        }
        if matches!(
            previous_kind,
            StaticValueKind::Object | StaticValueKind::Function
        ) && self.emit_update_identifier_boxed_primitive(name, op, prefix)?
        {
            return Ok(());
        }
        if matches!(
            previous_kind,
            StaticValueKind::Number
                | StaticValueKind::String
                | StaticValueKind::Bool
                | StaticValueKind::Null
                | StaticValueKind::Undefined
        ) && self.emit_update_identifier_static_number(name, op, prefix)?
        {
            return Ok(());
        }
        if previous_kind == StaticValueKind::BigInt
            && self.emit_update_identifier_static_bigint(name, op, prefix)?
        {
            return Ok(());
        }

        match previous_kind {
            StaticValueKind::Undefined
            | StaticValueKind::String
            | StaticValueKind::Object
            | StaticValueKind::Function
            | StaticValueKind::Symbol => {
                let nan_local = self.allocate_temp_local();
                self.push_i32_const(JS_NAN_TAG);
                self.push_local_set(nan_local);
                self.emit_store_identifier_from_local(name, nan_local)?;
                self.sync_updated_identifier_capture_from_local(name, nan_local)?;
                self.note_identifier_numeric_kind(name);
                self.push_local_get(nan_local);
                return Ok(());
            }
            StaticValueKind::Null => {
                let previous_local = self.allocate_temp_local();
                let next_local = self.allocate_temp_local();
                self.push_i32_const(0);
                self.push_local_set(previous_local);
                self.push_i32_const(match op {
                    UpdateOp::Increment => 1,
                    UpdateOp::Decrement => -1,
                });
                self.push_local_set(next_local);
                self.emit_store_identifier_from_local(name, next_local)?;
                self.sync_updated_identifier_capture_from_local(name, next_local)?;
                self.note_identifier_numeric_kind(name);
                if prefix {
                    self.push_local_get(next_local);
                } else {
                    self.push_local_get(previous_local);
                }
                return Ok(());
            }
            _ => {}
        }

        let mut emitted_runtime_write = false;
        if let Some((resolved_name, local_index)) = self.resolve_current_local_binding(name) {
            if let Some(initialized_local) = self.local_lexical_initialized_local(&resolved_name) {
                let previous_local = self.allocate_temp_local();
                let next_local = self.allocate_temp_local();
                let result_local = self.allocate_temp_local();
                self.state
                    .clear_local_static_binding_metadata(&resolved_name);
                self.push_local_get(initialized_local);
                self.state.emission.output.instructions.push(0x04);
                self.state
                    .emission
                    .output
                    .instructions
                    .push(EMPTY_BLOCK_TYPE);
                self.push_control_frame();
                if self.local_binding_is_immutable(&resolved_name) {
                    self.emit_named_error_throw("TypeError")?;
                } else {
                    self.push_local_get(local_index);
                    self.push_local_tee(previous_local);
                    let numeric_previous_local = self.emit_runtime_update_from_previous_local(
                        previous_local,
                        next_local,
                        opcode,
                    )?;
                    self.push_local_get(next_local);
                    self.push_local_set(local_index);
                    if prefix {
                        self.push_local_get(next_local);
                    } else {
                        self.push_local_get(numeric_previous_local);
                    }
                    self.push_local_set(result_local);
                }
                self.state.emission.output.instructions.push(0x05);
                self.emit_named_error_throw("ReferenceError")?;
                self.state.emission.output.instructions.push(0x0b);
                self.pop_control_frame();
                self.push_local_get(result_local);
                emitted_runtime_write = true;
            } else {
                let previous_local = self.allocate_temp_local();
                let next_local = self.allocate_temp_local();
                self.push_local_get(local_index);
                self.push_local_set(previous_local);
                let numeric_previous_local = self.emit_runtime_update_from_previous_local(
                    previous_local,
                    next_local,
                    opcode,
                )?;
                self.push_local_get(next_local);
                self.push_local_set(local_index);
                if prefix {
                    self.push_local_get(next_local);
                } else {
                    self.push_local_get(numeric_previous_local);
                }
                emitted_runtime_write = true;
            }
        } else if let Some(global_index) = self
            .backend
            .global_semantics
            .names
            .bindings
            .get(name)
            .copied()
        {
            if let Some(binding) = self.backend.lexical_global_binding(name) {
                let previous_local = self.allocate_temp_local();
                let next_local = self.allocate_temp_local();
                let result_local = self.allocate_temp_local();
                self.clear_static_identifier_binding_metadata(name);
                self.push_global_get(binding.initialized_index);
                self.state.emission.output.instructions.push(0x04);
                self.state
                    .emission
                    .output
                    .instructions
                    .push(EMPTY_BLOCK_TYPE);
                self.push_control_frame();
                if binding.mutable {
                    self.push_global_get(global_index);
                    self.push_local_tee(previous_local);
                    let numeric_previous_local = self.emit_runtime_update_from_previous_local(
                        previous_local,
                        next_local,
                        opcode,
                    )?;
                    self.push_local_get(next_local);
                    self.push_global_set(global_index);
                    if prefix {
                        self.push_local_get(next_local);
                    } else {
                        self.push_local_get(numeric_previous_local);
                    }
                    self.push_local_set(result_local);
                } else {
                    self.emit_named_error_throw("TypeError")?;
                }
                self.sync_updated_identifier_capture_from_local(name, next_local)?;
                self.state.emission.output.instructions.push(0x05);
                self.emit_named_error_throw("ReferenceError")?;
                self.state.emission.output.instructions.push(0x0b);
                self.pop_control_frame();
                self.push_local_get(result_local);
                emitted_runtime_write = true;
            } else {
                let previous_local = self.allocate_temp_local();
                let next_local = self.allocate_temp_local();
                self.push_global_get(global_index);
                self.push_local_set(previous_local);
                let numeric_previous_local = self.emit_runtime_update_from_previous_local(
                    previous_local,
                    next_local,
                    opcode,
                )?;
                self.push_local_get(next_local);
                self.push_global_set(global_index);
                self.sync_updated_identifier_capture_from_local(name, next_local)?;
                if prefix {
                    self.push_local_get(next_local);
                } else {
                    self.push_local_get(numeric_previous_local);
                }
                emitted_runtime_write = true;
            }
        } else if let Some(hidden_name) = self.resolve_user_function_capture_hidden_name(name)
            && let Some(binding) = self.backend.implicit_global_binding(&hidden_name)
        {
            let previous_local = self.allocate_temp_local();
            let next_local = self.allocate_temp_local();
            self.push_global_get(binding.present_index);
            self.state.emission.output.instructions.push(0x04);
            self.state
                .emission
                .output
                .instructions
                .push(EMPTY_BLOCK_TYPE);
            self.push_control_frame();
            self.push_global_get(binding.value_index);
            self.push_local_tee(previous_local);
            let numeric_previous_local =
                self.emit_runtime_update_from_previous_local(previous_local, next_local, opcode)?;
            self.push_local_get(next_local);
            self.push_global_set(binding.value_index);
            self.sync_updated_identifier_capture_from_local(name, next_local)?;
            self.state.emission.output.instructions.push(0x05);
            self.emit_named_error_throw("ReferenceError")?;
            self.state.emission.output.instructions.push(0x0b);
            self.pop_control_frame();
            if prefix {
                self.push_local_get(next_local);
            } else {
                self.push_local_get(numeric_previous_local);
            }
            emitted_runtime_write = true;
        } else if let Some(binding) = self.backend.implicit_global_binding(name) {
            let previous_local = self.allocate_temp_local();
            let next_local = self.allocate_temp_local();
            self.push_global_get(binding.present_index);
            self.state.emission.output.instructions.push(0x04);
            self.state
                .emission
                .output
                .instructions
                .push(EMPTY_BLOCK_TYPE);
            self.push_control_frame();
            self.push_global_get(binding.value_index);
            self.push_local_tee(previous_local);
            let numeric_previous_local =
                self.emit_runtime_update_from_previous_local(previous_local, next_local, opcode)?;
            self.push_local_get(next_local);
            self.push_global_set(binding.value_index);
            self.sync_updated_identifier_capture_from_local(name, next_local)?;
            self.state.emission.output.instructions.push(0x05);
            self.emit_named_error_throw("ReferenceError")?;
            self.state.emission.output.instructions.push(0x0b);
            self.pop_control_frame();
            if prefix {
                self.push_local_get(next_local);
            } else {
                self.push_local_get(numeric_previous_local);
            }
            emitted_runtime_write = true;
        } else {
            self.emit_named_error_throw("ReferenceError")?;
        }
        if emitted_runtime_write {
            self.state
                .emission
                .emitted_value_bindings
                .insert(name.to_string());
        }
        if self.backend.lexical_global_binding(name).is_none()
            && self.local_lexical_initialized_local(name).is_none()
        {
            if previous_kind == StaticValueKind::BigInt {
                if let Some((resolved_name, _)) = self.resolve_current_local_binding(name) {
                    self.state
                        .speculation
                        .static_semantics
                        .set_local_kind(&resolved_name, StaticValueKind::BigInt);
                } else {
                    self.backend
                        .set_global_binding_kind(name, StaticValueKind::BigInt);
                }
            } else {
                self.note_identifier_numeric_kind(name);
            }
        }
        Ok(())
    }
}
