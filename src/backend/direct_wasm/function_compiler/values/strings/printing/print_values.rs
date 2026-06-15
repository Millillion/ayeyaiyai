use super::*;

fn format_static_number(value: f64) -> String {
    js_console_number_to_string(value)
}

fn is_direct_eval_call_expression(value: &Expression) -> bool {
    matches!(
        value,
        Expression::Call { callee, .. }
            if matches!(callee.as_ref(), Expression::Identifier(name) if name == "eval")
    )
}

impl<'a> FunctionCompiler<'a> {
    fn print_expression_has_no_runtime_side_effects(value: &Expression) -> bool {
        match value {
            Expression::Assign { .. }
            | Expression::AssignMember { .. }
            | Expression::AssignSuperMember { .. }
            | Expression::Await(_)
            | Expression::EnumerateKeys(_)
            | Expression::GetIterator(_)
            | Expression::IteratorClose(_)
            | Expression::Call { .. }
            | Expression::SuperCall { .. }
            | Expression::New { .. }
            | Expression::Update { .. } => false,
            Expression::Member { object, property } => {
                Self::print_expression_has_no_runtime_side_effects(object)
                    && Self::print_expression_has_no_runtime_side_effects(property)
            }
            Expression::SuperMember { .. } => false,
            Expression::Unary { expression, .. } => {
                Self::print_expression_has_no_runtime_side_effects(expression)
            }
            Expression::Binary { left, right, .. } => {
                Self::print_expression_has_no_runtime_side_effects(left)
                    && Self::print_expression_has_no_runtime_side_effects(right)
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                Self::print_expression_has_no_runtime_side_effects(condition)
                    && Self::print_expression_has_no_runtime_side_effects(then_expression)
                    && Self::print_expression_has_no_runtime_side_effects(else_expression)
            }
            Expression::Sequence(expressions) => expressions
                .iter()
                .all(Self::print_expression_has_no_runtime_side_effects),
            Expression::Array(elements) => elements.iter().all(|element| match element {
                ArrayElement::Expression(expression) | ArrayElement::Spread(expression) => {
                    Self::print_expression_has_no_runtime_side_effects(expression)
                }
            }),
            Expression::Object(entries) => entries.iter().all(|entry| match entry {
                ObjectEntry::Data { key, value } => {
                    Self::print_expression_has_no_runtime_side_effects(key)
                        && Self::print_expression_has_no_runtime_side_effects(value)
                }
                ObjectEntry::Getter { key, .. } | ObjectEntry::Setter { key, .. } => {
                    Self::print_expression_has_no_runtime_side_effects(key)
                }
                ObjectEntry::Spread(expression) => {
                    Self::print_expression_has_no_runtime_side_effects(expression)
                }
            }),
            Expression::Number(_)
            | Expression::BigInt(_)
            | Expression::String(_)
            | Expression::Bool(_)
            | Expression::Null
            | Expression::Undefined
            | Expression::NewTarget
            | Expression::Identifier(_)
            | Expression::This
            | Expression::Sent => true,
        }
    }

    fn print_expression_is_definitely_string_concat_result(value: &Expression) -> bool {
        match value {
            Expression::String(_) => true,
            Expression::Binary {
                op: BinaryOp::Add,
                left,
                right,
            } => Self::print_binary_addition_is_definitely_string_concat(left, right),
            _ => false,
        }
    }

    fn print_binary_addition_is_definitely_string_concat(
        left: &Expression,
        right: &Expression,
    ) -> bool {
        Self::print_expression_is_definitely_string_concat_result(left)
            || Self::print_expression_is_definitely_string_concat_result(right)
    }

    fn print_expression_is_streamable_string_concat(value: &Expression) -> bool {
        let Expression::Binary {
            op: BinaryOp::Add,
            left,
            right,
        } = value
        else {
            return false;
        };
        Self::print_binary_addition_is_definitely_string_concat(left, right)
            && Self::print_expression_has_no_runtime_side_effects(value)
    }

    fn emit_print_boolean_runtime_value(&mut self, value: &Expression) -> DirectResult<()> {
        let bool_local = self.allocate_temp_local();
        self.emit_numeric_expression(value)?;
        self.push_local_set(bool_local);
        self.push_local_get(bool_local);
        self.state.emission.output.instructions.push(0x45);
        self.state.emission.output.instructions.push(0x04);
        self.state
            .emission
            .output
            .instructions
            .push(EMPTY_BLOCK_TYPE);
        self.push_control_frame();
        self.emit_print_string("false")?;
        self.state.emission.output.instructions.push(0x05);
        self.emit_print_string("true")?;
        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();
        Ok(())
    }

    fn emit_print_scalar_value_without_concat_expansion(
        &mut self,
        value: &Expression,
    ) -> DirectResult<()> {
        match value {
            Expression::Number(number) => self.emit_print_string(&format_static_number(*number)),
            Expression::String(text) => self.emit_print_string(text),
            Expression::Bool(true) => self.emit_print_string("true"),
            Expression::Bool(false) => self.emit_print_string("false"),
            Expression::Null => self.emit_print_string("null"),
            Expression::Undefined => self.emit_print_string("undefined"),
            Expression::Unary {
                op: UnaryOp::TypeOf,
                expression,
            } => self.emit_typeof_print(expression),
            Expression::Unary {
                op: UnaryOp::Void, ..
            } => self.emit_print_string("undefined"),
            _ => {
                let depends_on_active_loop_assignment =
                    self.expression_depends_on_active_loop_assignment(value);
                if let Some(primitive) = self.resolve_print_static_member_primitive(value) {
                    return self.emit_print_scalar_value_without_concat_expansion(&primitive);
                }
                if self.print_value_kind(value) == Some(StaticValueKind::Bool) {
                    return self.emit_print_boolean_runtime_value(value);
                }
                if !depends_on_active_loop_assignment
                    && inline_summary_side_effect_free_expression(value)
                    && let Some(primitive) = self.resolve_static_primitive_expression_with_context(
                        value,
                        self.current_function_name(),
                    )
                    && !static_expression_matches(&primitive, value)
                {
                    return self.emit_print_scalar_value_without_concat_expansion(&primitive);
                }
                if self.emit_print_object_value(value)? {
                    return Ok(());
                }
                if self.emit_runtime_print_known_string_value(value)? {
                    return Ok(());
                }
                self.emit_runtime_print_numeric_value(value)
            }
        }
    }

    fn emit_print_string_concat_operand(&mut self, value: &Expression) -> DirectResult<()> {
        if let Expression::Binary {
            op: BinaryOp::Add,
            left,
            right,
        } = value
            && Self::print_binary_addition_is_definitely_string_concat(left, right)
        {
            self.emit_print_string_concat_operand(left)?;
            self.emit_print_string_concat_operand(right)?;
            return Ok(());
        }
        self.emit_print_scalar_value_without_concat_expansion(value)
    }

    fn emit_print_streamable_string_concat(&mut self, value: &Expression) -> DirectResult<bool> {
        if !Self::print_expression_is_streamable_string_concat(value) {
            return Ok(false);
        }
        self.emit_print_string_concat_operand(value)?;
        Ok(true)
    }

    fn resolve_print_static_member_primitive(&self, value: &Expression) -> Option<Expression> {
        if self.expression_depends_on_active_loop_assignment(value) {
            return None;
        }
        let Expression::Member { object, property } = value else {
            return None;
        };
        let property = self.materialize_static_expression(property);
        let object_binding = self.resolve_object_binding_from_expression(object)?;
        let property_value =
            self.resolve_object_binding_property_value(&object_binding, &property)?;
        if self.infer_value_kind(&property_value) == Some(StaticValueKind::Bool) {
            return self
                .resolve_static_boolean_expression(&property_value)
                .map(Expression::Bool);
        }
        let primitive = self
            .resolve_static_primitive_expression_with_context(
                &property_value,
                self.current_function_name(),
            )
            .or_else(|| {
                let materialized = self.materialize_static_expression(&property_value);
                (!static_expression_matches(&materialized, &property_value)).then(|| {
                    self.resolve_static_primitive_expression_with_context(
                        &materialized,
                        self.current_function_name(),
                    )
                    .unwrap_or(materialized)
                })
            })
            .unwrap_or(property_value);
        match primitive {
            Expression::Number(_)
            | Expression::String(_)
            | Expression::Bool(_)
            | Expression::Null
            | Expression::Undefined
            | Expression::BigInt(_) => Some(primitive),
            _ => None,
        }
    }

    pub(in crate::backend::direct_wasm) fn print_runtime_shadow_static_value_kind(
        &self,
        value: &Expression,
    ) -> Option<StaticValueKind> {
        let Expression::Member { object, property } = value else {
            return None;
        };
        let property = self.materialize_static_expression(property);
        let shadow_binding_name =
            self.runtime_object_property_shadow_binding_name_for_expression(object, &property)?;
        if !self.runtime_object_property_shadow_binding_should_defer_static_resolution(
            &shadow_binding_name,
        ) {
            return None;
        }
        self.global_value_binding(&shadow_binding_name)
            .and_then(|value| self.infer_value_kind(value))
            .or_else(|| self.global_binding_kind(&shadow_binding_name))
    }

    fn print_value_kind(&self, value: &Expression) -> Option<StaticValueKind> {
        match self.infer_value_kind(value) {
            Some(StaticValueKind::Unknown) => self
                .print_runtime_shadow_static_value_kind(value)
                .or(Some(StaticValueKind::Unknown)),
            Some(kind) => Some(kind),
            None => self
                .print_runtime_shadow_static_value_kind(value)
                .or_else(|| {
                    let Expression::Member { object, property } = value else {
                        return None;
                    };
                    let property = self.materialize_static_expression(property);
                    self.resolve_object_binding_from_expression(object)
                        .and_then(|binding| {
                            self.resolve_object_binding_property_value(&binding, &property)
                        })
                        .and_then(|property_value| self.infer_value_kind(&property_value))
                }),
        }
    }

    fn emit_print_static_property_name(&mut self, name: &str) -> DirectResult<()> {
        self.emit_print_string(name)
    }

    fn emit_print_object_entries(
        &mut self,
        entries: Vec<(String, Expression)>,
    ) -> DirectResult<bool> {
        self.emit_print_string("{ ")?;
        for (index, (name, value)) in entries.iter().enumerate() {
            if index > 0 {
                self.emit_print_string(", ")?;
            }
            self.emit_print_static_property_name(name)?;
            self.emit_print_string(": ")?;
            self.emit_print_scalar_value_without_concat_expansion(value)?;
        }
        self.emit_print_string(" }")?;
        Ok(true)
    }

    fn object_literal_print_entries(
        &self,
        entries: &[ObjectEntry],
    ) -> Option<Vec<(String, Expression)>> {
        let mut printable_entries = Vec::new();
        for entry in entries {
            let ObjectEntry::Data { key, value } = entry else {
                return None;
            };
            let key = match key {
                Expression::String(name) => name.clone(),
                Expression::Number(number) if number.fract() == 0.0 => (*number as i64).to_string(),
                _ => return None,
            };
            printable_entries.push((key, value.clone()));
        }
        Some(printable_entries)
    }

    fn emit_print_object_value(&mut self, value: &Expression) -> DirectResult<bool> {
        if let Expression::Object(entries) = value
            && let Some(printable_entries) = self.object_literal_print_entries(entries)
        {
            return self.emit_print_object_entries(printable_entries);
        }

        let Some(object_binding) = self.resolve_object_binding_from_expression(value) else {
            return Ok(false);
        };
        let entries = object_binding
            .string_properties
            .iter()
            .filter(|(name, _)| {
                !object_binding
                    .non_enumerable_string_properties
                    .iter()
                    .any(|hidden_name| hidden_name == name)
            })
            .map(|(name, _)| {
                (
                    name.clone(),
                    Expression::Member {
                        object: Box::new(value.clone()),
                        property: Box::new(Expression::String(name.clone())),
                    },
                )
            })
            .collect::<Vec<_>>();
        if entries.is_empty() {
            return Ok(false);
        }
        self.emit_print_object_entries(entries)
    }

    pub(in crate::backend::direct_wasm) fn emit_print(
        &mut self,
        values: &[Expression],
    ) -> DirectResult<()> {
        let (space_ptr, space_len) = self.intern_string(b" ".to_vec());
        let (newline_ptr, newline_len) = self.intern_string(b"\n".to_vec());

        for (index, value) in values.iter().enumerate() {
            if index > 0 {
                self.push_i32_const(space_ptr as i32);
                self.push_i32_const(space_len as i32);
                self.push_call(WRITE_BYTES_FUNCTION_INDEX);
            }
            self.emit_print_value(value)?;
        }

        self.push_i32_const(newline_ptr as i32);
        self.push_i32_const(newline_len as i32);
        self.push_call(WRITE_BYTES_FUNCTION_INDEX);
        Ok(())
    }

    pub(in crate::backend::direct_wasm) fn emit_print_value(
        &mut self,
        value: &Expression,
    ) -> DirectResult<()> {
        if crate::ayy_env_flag!("AYY_TRACE_RUNTIME_SHADOWS") {
            eprintln!("runtime_shadow_print_value value={value:?}");
        }
        match value {
            Expression::Number(number) => self.emit_print_string(&format_static_number(*number)),
            Expression::String(text) => {
                let (ptr, len) = self.intern_string(text.as_bytes().to_vec());
                self.push_i32_const(ptr as i32);
                self.push_i32_const(len as i32);
                self.push_call(WRITE_BYTES_FUNCTION_INDEX);
                Ok(())
            }
            Expression::Bool(true) => {
                let (ptr, len) = self.intern_string(b"true".to_vec());
                self.push_i32_const(ptr as i32);
                self.push_i32_const(len as i32);
                self.push_call(WRITE_BYTES_FUNCTION_INDEX);
                Ok(())
            }
            Expression::Bool(false) => {
                let (ptr, len) = self.intern_string(b"false".to_vec());
                self.push_i32_const(ptr as i32);
                self.push_i32_const(len as i32);
                self.push_call(WRITE_BYTES_FUNCTION_INDEX);
                Ok(())
            }
            Expression::Null => {
                let (ptr, len) = self.intern_string(b"null".to_vec());
                self.push_i32_const(ptr as i32);
                self.push_i32_const(len as i32);
                self.push_call(WRITE_BYTES_FUNCTION_INDEX);
                Ok(())
            }
            Expression::Undefined => {
                let (ptr, len) = self.intern_string(b"undefined".to_vec());
                self.push_i32_const(ptr as i32);
                self.push_i32_const(len as i32);
                self.push_call(WRITE_BYTES_FUNCTION_INDEX);
                Ok(())
            }
            Expression::Unary {
                op: UnaryOp::TypeOf,
                expression,
            } => self.emit_typeof_print(expression),
            Expression::Unary {
                op: UnaryOp::Void, ..
            } => {
                let (ptr, len) = self.intern_string(b"undefined".to_vec());
                self.push_i32_const(ptr as i32);
                self.push_i32_const(len as i32);
                self.push_call(WRITE_BYTES_FUNCTION_INDEX);
                Ok(())
            }
            Expression::Unary {
                op: UnaryOp::Delete,
                ..
            } => {
                let bool_local = self.allocate_temp_local();
                self.emit_numeric_expression(value)?;
                self.push_local_set(bool_local);
                self.push_local_get(bool_local);
                self.state.emission.output.instructions.push(0x45);
                self.state.emission.output.instructions.push(0x04);
                self.state
                    .emission
                    .output
                    .instructions
                    .push(EMPTY_BLOCK_TYPE);
                self.push_control_frame();
                self.emit_print_string("false")?;
                self.state.emission.output.instructions.push(0x05);
                self.emit_print_string("true")?;
                self.state.emission.output.instructions.push(0x0b);
                self.pop_control_frame();
                Ok(())
            }
            _ => {
                let depends_on_active_loop_assignment =
                    self.expression_depends_on_active_loop_assignment(value);
                let direct_eval_call = is_direct_eval_call_expression(value);
                if !direct_eval_call && self.emit_print_streamable_string_concat(value)? {
                    return Ok(());
                }
                if let Some(primitive) = self.resolve_print_static_member_primitive(value) {
                    return self.emit_print_value(&primitive);
                }
                if self.emit_print_object_value(value)? {
                    return Ok(());
                }
                if self.print_value_kind(value) == Some(StaticValueKind::Bool) {
                    return self.emit_print_boolean_runtime_value(value);
                }
                if !depends_on_active_loop_assignment
                    && !direct_eval_call
                    && let Some(primitive) = self.resolve_static_primitive_expression_with_context(
                        value,
                        self.current_function_name(),
                    )
                    && !static_expression_matches(&primitive, value)
                {
                    if !inline_summary_side_effect_free_expression(value) {
                        self.emit_numeric_expression(value)?;
                        self.state.emission.output.instructions.push(0x1a);
                    }
                    return self.emit_print_value(&primitive);
                }
                if !matches!(
                    value,
                    Expression::Member { .. } | Expression::SuperMember { .. }
                ) && !direct_eval_call
                    && let Some(number) = self.resolve_static_number_value(value)
                    && (number.is_nan()
                        || !number.is_finite()
                        || number.fract() != 0.0
                        || (number == 0.0 && number.is_sign_negative()))
                {
                    return self.emit_print_value(&Expression::Number(number));
                }
                if !depends_on_active_loop_assignment
                    && !direct_eval_call
                    && let Some(text) = self.resolve_static_string_value(value)
                {
                    self.emit_print_string(&text)?;
                    return Ok(());
                }
                if self.emit_runtime_print_known_string_value(value)? {
                    return Ok(());
                }
                self.emit_runtime_print_numeric_value(value)
            }
        }
    }
}
