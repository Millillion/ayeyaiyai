use super::*;

#[path = "assertions/array_compare.rs"]
mod array_compare;
#[path = "assertions/same_value.rs"]
mod same_value;
#[path = "assertions/throws.rs"]
mod throws;
#[path = "assertions/try_scan.rs"]
mod try_scan;

impl<'a> FunctionCompiler<'a> {
    fn assertion_static_message_argument_effect_free(&self, expression: &Expression) -> bool {
        if inline_summary_side_effect_free_expression(expression) {
            return true;
        }
        match expression {
            Expression::Binary {
                op: BinaryOp::Add,
                left,
                right,
            } => {
                self.assertion_static_message_argument_effect_free(left)
                    && self.assertion_static_message_argument_effect_free(right)
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                self.assertion_static_message_argument_effect_free(condition)
                    && self.assertion_static_message_argument_effect_free(then_expression)
                    && self.assertion_static_message_argument_effect_free(else_expression)
            }
            Expression::Call { callee, arguments } => {
                if let Expression::Member { object, property } = callee.as_ref()
                    && arguments.is_empty()
                    && matches!(property.as_ref(), Expression::String(name) if name == "toString")
                {
                    return self.assertion_static_message_argument_effect_free(object);
                }
                if matches!(callee.as_ref(), Expression::Identifier(name) if name == "String")
                    && self.is_unshadowed_builtin_identifier("String")
                    && arguments.iter().all(|argument| match argument {
                        CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                            self.assertion_static_message_argument_effect_free(expression)
                        }
                    })
                {
                    return true;
                }
                false
            }
            Expression::Sequence(expressions) => expressions
                .iter()
                .all(|expression| self.assertion_static_message_argument_effect_free(expression)),
            _ => false,
        }
    }

    fn assertion_static_message_arguments_effect_free(&self, arguments: &[CallArgument]) -> bool {
        arguments.iter().all(|argument| match argument {
            CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                self.assertion_static_message_argument_effect_free(expression)
            }
        })
    }

    fn assertion_module_namespace_object_binding(
        &self,
        target: &Expression,
    ) -> Option<ObjectValueBinding> {
        self.resolve_object_binding_from_expression(target)
            .or_else(|| match target {
                Expression::Identifier(name) => self
                    .resolve_identifier_object_binding_fallback(name)
                    .or_else(|| self.resolve_runtime_shadow_object_binding(name)),
                Expression::This => self.resolve_runtime_shadow_object_binding("this"),
                _ => None,
            })
            .filter(Self::object_binding_has_module_namespace_marker)
    }

    fn assertion_define_property_descriptor_is_empty(descriptor_expression: &Expression) -> bool {
        resolve_property_descriptor_definition(descriptor_expression).is_some_and(|descriptor| {
            descriptor.value.is_none()
                && descriptor.writable.is_none()
                && descriptor.enumerable.is_none()
                && descriptor.configurable.is_none()
                && descriptor.getter.is_none()
                && descriptor.setter.is_none()
        })
    }

    fn emit_assertion_module_namespace_property_presence(
        &mut self,
        object_binding: &ObjectValueBinding,
        property: &Expression,
    ) -> DirectResult<bool> {
        if let Some(property_name) = static_property_name_from_expression(property) {
            let present = object_binding_lookup_value(
                object_binding,
                &Expression::String(property_name.clone()),
            )
            .is_some()
                && !property_name.starts_with("__ayy$")
                && property_name != "then";
            self.push_i32_const(present as i32);
            return Ok(true);
        }
        if is_symbol_to_string_tag_expression(property) {
            self.push_i32_const(1);
            return Ok(true);
        }
        if !inline_summary_side_effect_free_expression(property) {
            return Ok(false);
        }

        let property_names = ordered_object_property_names(object_binding)
            .into_iter()
            .filter(|name| !name.starts_with("__ayy$") && name != "then")
            .collect::<Vec<_>>();
        if property_names.is_empty() {
            self.push_i32_const(0);
            return Ok(true);
        }

        let property_local = self.allocate_temp_local();
        let result_local = self.allocate_temp_local();
        self.emit_numeric_expression(property)?;
        self.push_local_set(property_local);
        self.push_i32_const(0);
        self.push_local_set(result_local);

        for property_name in property_names {
            self.push_local_get(property_local);
            self.emit_static_string_literal(&property_name)?;
            self.push_binary_op(BinaryOp::Equal)?;
            self.state.emission.output.instructions.push(0x04);
            self.state
                .emission
                .output
                .instructions
                .push(EMPTY_BLOCK_TYPE);
            self.push_control_frame();
            self.push_i32_const(1);
            self.push_local_set(result_local);
            self.state.emission.output.instructions.push(0x0b);
            self.pop_control_frame();
        }

        self.push_local_get(property_local);
        self.emit_numeric_expression(&Expression::Member {
            object: Box::new(Expression::Identifier("Symbol".to_string())),
            property: Box::new(Expression::String("toStringTag".to_string())),
        })?;
        self.push_binary_op(BinaryOp::Equal)?;
        self.state.emission.output.instructions.push(0x04);
        self.state
            .emission
            .output
            .instructions
            .push(EMPTY_BLOCK_TYPE);
        self.push_control_frame();
        self.push_i32_const(1);
        self.push_local_set(result_local);
        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();

        self.push_local_get(result_local);
        Ok(true)
    }

    fn emit_assertion_fail_if_false(&mut self, condition_local: u32) -> DirectResult<()> {
        self.push_local_get(condition_local);
        self.state.emission.output.instructions.push(0x45);
        self.state.emission.output.instructions.push(0x04);
        self.state
            .emission
            .output
            .instructions
            .push(EMPTY_BLOCK_TYPE);
        self.push_control_frame();
        self.emit_error_throw()?;
        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();
        Ok(())
    }

    fn assertion_global_runtime_array_length_name(
        &self,
        expression: &Expression,
    ) -> Option<String> {
        let Expression::Member { object, property } = expression else {
            return None;
        };
        let property = self
            .resolve_property_key_expression(property)
            .unwrap_or_else(|| self.materialize_static_expression(property));
        if !matches!(property, Expression::String(property_name) if property_name == "length") {
            return None;
        }
        let binding_name = self.runtime_array_binding_name_for_expression(object)?;
        (self.backend.global_array_binding(&binding_name).is_some()
            || self.uses_global_runtime_array_state(&binding_name)
            || self
                .backend
                .shared_global_semantics
                .values
                .array_bindings
                .contains_key(&binding_name))
        .then_some(binding_name)
    }

    fn emit_assertion_global_runtime_array_length_read(&mut self, name: &str) -> bool {
        let trace = std::env::var_os("AYY_TRACE_ASSERTIONS").is_some();
        let initial_length = self
            .backend
            .global_array_binding(name)
            .or_else(|| {
                self.backend
                    .shared_global_semantics
                    .values
                    .array_bindings
                    .get(name)
            })
            .map(|binding| binding.values.len() as i32)
            .unwrap_or(0);
        if self.backend.global_array_binding(name).is_none()
            && !self.uses_global_runtime_array_state(name)
            && !self
                .backend
                .shared_global_semantics
                .values
                .array_bindings
                .contains_key(name)
        {
            if trace {
                eprintln!(
                    "assertion_runtime_length:skip_read name={name} global_array=false uses_global={} shared_array={}",
                    self.uses_global_runtime_array_state(name),
                    self.backend
                        .shared_global_semantics
                        .values
                        .array_bindings
                        .contains_key(name)
                );
            }
            return false;
        }
        if trace {
            eprintln!(
                "assertion_runtime_length:emit_read name={name} initial_length={initial_length} global_array={} uses_global={} shared_array={}",
                self.backend.global_array_binding(name).is_some(),
                self.uses_global_runtime_array_state(name),
                self.backend
                    .shared_global_semantics
                    .values
                    .array_bindings
                    .contains_key(name)
            );
        }
        self.backend.mark_global_array_with_runtime_state(name);
        self.backend
            .shared_global_semantics
            .values
            .mark_array_with_runtime_state(name);

        let binding = self.global_runtime_array_length_binding(name);
        self.push_global_get(binding.present_index);
        self.state.emission.output.instructions.push(0x04);
        self.state.emission.output.instructions.push(I32_TYPE);
        self.push_control_frame();
        self.push_global_get(binding.value_index);
        self.state.emission.output.instructions.push(0x05);
        self.push_i32_const(initial_length);
        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();
        true
    }

    fn assertion_static_values_match(&self, left: &Expression, right: &Expression) -> bool {
        if static_expression_matches(left, right) {
            return true;
        }
        let left_materialized = self.materialize_static_expression(left);
        let right_materialized = self.materialize_static_expression(right);
        static_expression_matches(&left_materialized, &right_materialized)
            || (is_symbol_to_string_tag_expression(left)
                || is_symbol_to_string_tag_expression(&left_materialized))
                && (is_symbol_to_string_tag_expression(right)
                    || is_symbol_to_string_tag_expression(&right_materialized))
    }

    fn assertion_static_array_binding(
        &self,
        expression: &Expression,
        depth: usize,
    ) -> Option<ArrayValueBinding> {
        if depth == 0 {
            return None;
        }
        match expression {
            Expression::Identifier(name) => {
                let existing = self
                    .state
                    .speculation
                    .static_semantics
                    .local_array_binding(name)
                    .cloned()
                    .or_else(|| self.backend.global_array_binding(name).cloned());
                let derived = self
                    .state
                    .speculation
                    .static_semantics
                    .local_value_binding(name)
                    .or_else(|| self.global_value_binding(name))
                    .filter(|value| !static_expression_matches(value, expression))
                    .and_then(|value| self.assertion_static_array_binding(value, depth - 1));
                match (existing, derived) {
                    (Some(existing), Some(derived)) => {
                        if derived.values.len() > existing.values.len() {
                            Some(derived)
                        } else {
                            Some(existing)
                        }
                    }
                    (Some(existing), None) => Some(existing),
                    (None, Some(derived)) => Some(derived),
                    (None, None) => None,
                }
            }
            Expression::Call { callee, arguments } => self
                .static_builtin_object_array_call_binding(callee, arguments)
                .or_else(|| self.resolve_array_binding_from_expression(expression)),
            Expression::Array(elements) => Some(ArrayValueBinding {
                values: elements
                    .iter()
                    .map(|element| match element {
                        ArrayElement::Expression(expression) => Some(expression.clone()),
                        ArrayElement::Spread(expression) => self
                            .assertion_static_array_binding(expression, depth - 1)
                            .and_then(|binding| {
                                (binding.values.len() == 1).then(|| binding.values[0].clone())
                            })
                            .flatten(),
                    })
                    .collect(),
            }),
            _ => self.resolve_array_binding_from_expression(expression),
        }
    }

    fn assertion_static_array_index_of(
        &self,
        object: &Expression,
        arguments: &[CallArgument],
        depth: usize,
    ) -> Option<f64> {
        if depth == 0 {
            return None;
        }
        let search_expression = match arguments.first() {
            Some(CallArgument::Expression(expression) | CallArgument::Spread(expression)) => {
                expression
            }
            None => return Some(-1.0),
        };
        let array_binding = self.assertion_static_array_binding(object, depth - 1)?;
        let found_index = array_binding
            .values
            .iter()
            .enumerate()
            .find_map(|(index, value)| {
                let value = value.as_ref()?;
                self.assertion_static_values_match(value, search_expression)
                    .then_some(index as f64)
            })
            .unwrap_or(-1.0);
        Some(found_index)
    }

    fn assertion_static_number_value(&self, expression: &Expression, depth: usize) -> Option<f64> {
        if depth == 0 {
            return None;
        }
        match expression {
            Expression::Number(value) => Some(*value),
            Expression::Bool(value) => Some(if *value { 1.0 } else { 0.0 }),
            Expression::Unary {
                op: UnaryOp::Negate,
                expression,
            } => Some(-self.assertion_static_number_value(expression, depth - 1)?),
            Expression::Member { object, property }
                if static_property_name_from_expression(property).as_deref() == Some("length") =>
            {
                self.assertion_static_array_binding(object, depth - 1)
                    .map(|binding| binding.values.len() as f64)
            }
            Expression::Call { callee, arguments } => {
                let Expression::Member { object, property } = callee.as_ref() else {
                    return self.resolve_static_number_value(expression);
                };
                if matches!(property.as_ref(), Expression::String(name) if name == "indexOf") {
                    return self.assertion_static_array_index_of(object, arguments, depth - 1);
                }
                self.resolve_static_number_value(expression)
            }
            Expression::Identifier(name) => self
                .state
                .speculation
                .static_semantics
                .local_value_binding(name)
                .or_else(|| self.global_value_binding(name))
                .filter(|value| !static_expression_matches(value, expression))
                .and_then(|value| self.assertion_static_number_value(value, depth - 1))
                .or_else(|| self.resolve_static_number_value(expression)),
            _ => self.resolve_static_number_value(expression),
        }
    }

    fn assertion_static_boolean_condition(
        &self,
        condition: &Expression,
        depth: usize,
    ) -> Option<bool> {
        if depth == 0 {
            return None;
        }
        match condition {
            Expression::Bool(value) => Some(*value),
            Expression::Unary {
                op: UnaryOp::Not,
                expression,
            } => Some(!self.assertion_static_boolean_condition(expression, depth - 1)?),
            Expression::Binary { op, left, right }
                if matches!(
                    op,
                    BinaryOp::LessThan
                        | BinaryOp::LessThanOrEqual
                        | BinaryOp::GreaterThan
                        | BinaryOp::GreaterThanOrEqual
                ) =>
            {
                let left_number = self.assertion_static_number_value(left, depth - 1)?;
                let right_number = self.assertion_static_number_value(right, depth - 1)?;
                Some(match op {
                    BinaryOp::LessThan => left_number < right_number,
                    BinaryOp::LessThanOrEqual => left_number <= right_number,
                    BinaryOp::GreaterThan => left_number > right_number,
                    BinaryOp::GreaterThanOrEqual => left_number >= right_number,
                    _ => unreachable!("filtered above"),
                })
            }
            Expression::Binary {
                op: BinaryOp::LogicalAnd,
                left,
                right,
            } => {
                let left_value = self.assertion_static_boolean_condition(left, depth - 1)?;
                if !left_value {
                    Some(false)
                } else {
                    self.assertion_static_boolean_condition(right, depth - 1)
                }
            }
            Expression::Binary {
                op: BinaryOp::LogicalOr,
                left,
                right,
            } => {
                let left_value = self.assertion_static_boolean_condition(left, depth - 1)?;
                if left_value {
                    Some(true)
                } else {
                    self.assertion_static_boolean_condition(right, depth - 1)
                }
            }
            _ => self.resolve_static_boolean_expression(condition),
        }
    }

    fn emit_runtime_array_length_assertion_condition(
        &mut self,
        condition: &Expression,
    ) -> DirectResult<bool> {
        if !self
            .current_function_name()
            .is_some_and(|name| name.starts_with("__ayy_module_init_"))
        {
            return Ok(false);
        }
        let Expression::Binary { op, left, right } = condition else {
            return Ok(false);
        };
        if !matches!(
            op,
            BinaryOp::LessThan
                | BinaryOp::LessThanOrEqual
                | BinaryOp::GreaterThan
                | BinaryOp::GreaterThanOrEqual
        ) {
            return Ok(false);
        }
        let trace = std::env::var_os("AYY_TRACE_ASSERTIONS").is_some();
        if trace {
            eprintln!(
                "assertion_runtime_length:condition fn={:?} condition={condition:?} left_binding={:?} right_binding={:?}",
                self.current_function_name(),
                self.assertion_global_runtime_array_length_name(left),
                self.assertion_global_runtime_array_length_name(right)
            );
        }

        if let (Some(binding_name), Expression::Number(number)) = (
            self.assertion_global_runtime_array_length_name(left),
            right.as_ref(),
        ) && number.is_finite()
            && number.fract() == 0.0
            && *number >= i32::MIN as f64
            && *number <= i32::MAX as f64
            && self.emit_assertion_global_runtime_array_length_read(&binding_name)
        {
            self.push_i32_const(*number as i32);
            self.push_binary_op(*op)?;
            return Ok(true);
        }

        if let (Expression::Number(number), Some(binding_name)) = (
            left.as_ref(),
            self.assertion_global_runtime_array_length_name(right),
        ) && number.is_finite()
            && number.fract() == 0.0
            && *number >= i32::MIN as f64
            && *number <= i32::MAX as f64
        {
            self.push_i32_const(*number as i32);
            if self.emit_assertion_global_runtime_array_length_read(&binding_name) {
                self.push_binary_op(*op)?;
                return Ok(true);
            }
            self.state.emission.output.instructions.push(0x1a);
        }

        Ok(false)
    }

    pub(in crate::backend::direct_wasm) fn emit_assertion_builtin_call(
        &mut self,
        name: &str,
        arguments: &[CallArgument],
    ) -> DirectResult<bool> {
        match name {
            "__assert" => {
                let Some(CallArgument::Expression(condition)) = arguments.first() else {
                    self.push_i32_const(JS_UNDEFINED_TAG);
                    return Ok(true);
                };
                let condition_local = self.allocate_temp_local();
                if let Some(static_condition) =
                    self.assertion_static_boolean_condition(condition, 6)
                {
                    self.push_i32_const(static_condition as i32);
                } else if !self.emit_runtime_array_length_assertion_condition(condition)? {
                    self.emit_numeric_expression(condition)?;
                }
                self.push_local_set(condition_local);
                for argument in arguments.iter().skip(1) {
                    match argument {
                        CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                            self.emit_numeric_expression(expression)?;
                            self.state.emission.output.instructions.push(0x1a);
                        }
                    }
                }
                self.push_local_get(condition_local);
                self.state.emission.output.instructions.push(0x45);
                self.state.emission.output.instructions.push(0x04);
                self.state
                    .emission
                    .output
                    .instructions
                    .push(EMPTY_BLOCK_TYPE);
                self.push_control_frame();
                if std::env::var_os("AYY_TRACE_ASSERTIONS").is_some() {
                    self.emit_print(&[Expression::String(format!(
                        "assertion_fail name=__assert condition={condition:?} fn={:?}",
                        self.current_function_name()
                    ))])?;
                }
                self.emit_error_throw()?;
                self.state.emission.output.instructions.push(0x0b);
                self.pop_control_frame();
                self.push_i32_const(JS_UNDEFINED_TAG);
                Ok(true)
            }
            "__assertSameValue" | "__assertNotSameValue" => {
                self.emit_same_value_assertion(name, arguments)
            }
            _ => Ok(false),
        }
    }
}
