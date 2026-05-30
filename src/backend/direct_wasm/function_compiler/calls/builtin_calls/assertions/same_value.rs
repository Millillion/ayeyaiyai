use super::*;

impl<'a> FunctionCompiler<'a> {
    fn emit_same_value_operand(&mut self, expression: &Expression) -> DirectResult<()> {
        self.emit_numeric_expression(expression)
    }

    fn same_value_static_number_to_string(value: f64) -> String {
        if value.is_nan() {
            "NaN".to_string()
        } else if value == f64::INFINITY {
            "Infinity".to_string()
        } else if value == f64::NEG_INFINITY {
            "-Infinity".to_string()
        } else if value == 0.0 {
            "0".to_string()
        } else if value.is_finite() && value.fract() == 0.0 {
            (value as i64).to_string()
        } else {
            value.to_string()
        }
    }

    fn same_value_static_primitive_to_string(expression: &Expression) -> Option<String> {
        match expression {
            Expression::String(value) => Some(value.clone()),
            Expression::Number(value) => Some(Self::same_value_static_number_to_string(*value)),
            Expression::Bool(value) => Some(value.to_string()),
            Expression::Null => Some("null".to_string()),
            Expression::Undefined => Some("undefined".to_string()),
            Expression::BigInt(value) => Some(value.trim_end_matches('n').to_string()),
            _ => None,
        }
    }

    fn same_value_static_primitive_to_number(expression: &Expression) -> Option<f64> {
        match expression {
            Expression::Number(value) => Some(*value),
            Expression::Bool(value) => Some(if *value { 1.0 } else { 0.0 }),
            Expression::Null => Some(0.0),
            Expression::Undefined => Some(f64::NAN),
            _ => None,
        }
    }

    fn same_value_static_binary_expression_value(
        op: BinaryOp,
        left: &Expression,
        right: &Expression,
    ) -> Option<Expression> {
        match op {
            BinaryOp::Add => {
                if matches!(left, Expression::String(_)) || matches!(right, Expression::String(_)) {
                    let left = Self::same_value_static_primitive_to_string(left)?;
                    let right = Self::same_value_static_primitive_to_string(right)?;
                    Some(Expression::String(format!("{left}{right}")))
                } else {
                    Some(Expression::Number(
                        Self::same_value_static_primitive_to_number(left)?
                            + Self::same_value_static_primitive_to_number(right)?,
                    ))
                }
            }
            _ => None,
        }
    }

    fn same_value_define_property_call_parts<'b>(
        expression: &'b Expression,
    ) -> Option<(bool, &'b Expression, &'b Expression, &'b Expression)> {
        let Expression::Call { callee, arguments } = expression else {
            return None;
        };
        let Expression::Member { object, property } = callee.as_ref() else {
            return None;
        };
        let reflect_call = matches!(object.as_ref(), Expression::Identifier(name) if name == "Reflect")
            && matches!(property.as_ref(), Expression::String(name) if name == "defineProperty");
        let object_call = matches!(object.as_ref(), Expression::Identifier(name) if name == "Object")
            && matches!(property.as_ref(), Expression::String(name) if name == "defineProperty");
        if !reflect_call && !object_call {
            return None;
        }
        let [
            CallArgument::Expression(target),
            CallArgument::Expression(property),
            CallArgument::Expression(descriptor),
            ..,
        ] = arguments.as_slice()
        else {
            return None;
        };
        Some((reflect_call, target, property, descriptor))
    }

    fn same_value_expected_is_define_property_target(
        &self,
        target: &Expression,
        expected: &Expression,
    ) -> bool {
        static_expression_matches(target, expected)
            || static_expression_matches(
                &self.materialize_static_expression(target),
                &self.materialize_static_expression(expected),
            )
            || self
                .resolve_static_reference_identity_key(target)
                .zip(self.resolve_static_reference_identity_key(expected))
                .is_some_and(|(target_key, expected_key)| target_key == expected_key)
    }

    fn emit_same_value_module_namespace_define_property_assertion(
        &mut self,
        actual: &Expression,
        expected: &Expression,
        assertion_failure: BinaryOp,
    ) -> DirectResult<bool> {
        let Some((reflect_call, target, property, descriptor)) =
            Self::same_value_define_property_call_parts(actual)
        else {
            return Ok(false);
        };
        if assertion_failure != BinaryOp::NotEqual {
            return Ok(false);
        }
        if !inline_summary_side_effect_free_expression(target)
            || !inline_summary_side_effect_free_expression(property)
            || !inline_summary_side_effect_free_expression(descriptor)
        {
            return Ok(false);
        }

        if let Some(accepted) =
            self.static_define_property_accepts_without_mutation(target, property, descriptor)
        {
            if reflect_call {
                if matches!(expected, Expression::Bool(expected) if *expected == accepted) {
                    self.push_i32_const(JS_UNDEFINED_TAG);
                    return Ok(true);
                }
            } else if accepted
                && self.same_value_expected_is_define_property_target(target, expected)
            {
                self.push_i32_const(JS_UNDEFINED_TAG);
                return Ok(true);
            }
            return Ok(false);
        }

        let Some(object_binding) = self.assertion_module_namespace_object_binding(target) else {
            return Ok(false);
        };
        if !Self::assertion_define_property_descriptor_is_empty(descriptor) {
            return Ok(false);
        }

        if reflect_call {
            let Expression::Bool(expected_result) = expected else {
                return Ok(false);
            };
            let pass_local = self.allocate_temp_local();
            if !self.emit_assertion_module_namespace_property_presence(&object_binding, property)? {
                return Ok(false);
            }
            if !*expected_result {
                self.state.emission.output.instructions.push(0x45);
            }
            self.push_local_set(pass_local);
            self.emit_assertion_fail_if_false(pass_local)?;
            self.push_i32_const(JS_UNDEFINED_TAG);
            return Ok(true);
        }

        if !self.same_value_expected_is_define_property_target(target, expected) {
            return Ok(false);
        }
        let accepted_local = self.allocate_temp_local();
        if !self.emit_assertion_module_namespace_property_presence(&object_binding, property)? {
            return Ok(false);
        }
        self.push_local_set(accepted_local);
        self.push_local_get(accepted_local);
        self.state.emission.output.instructions.push(0x45);
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
        self.push_i32_const(JS_UNDEFINED_TAG);
        Ok(true)
    }

    fn same_value_operand_contains_member_access(expression: &Expression) -> bool {
        match expression {
            Expression::Member { .. } | Expression::SuperMember { .. } => true,
            Expression::Assign { value, .. }
            | Expression::Await(value)
            | Expression::EnumerateKeys(value)
            | Expression::GetIterator(value)
            | Expression::IteratorClose(value)
            | Expression::Unary {
                expression: value, ..
            } => Self::same_value_operand_contains_member_access(value),
            Expression::AssignMember {
                object,
                property,
                value,
            } => {
                Self::same_value_operand_contains_member_access(object)
                    || Self::same_value_operand_contains_member_access(property)
                    || Self::same_value_operand_contains_member_access(value)
            }
            Expression::AssignSuperMember { property, value } => {
                Self::same_value_operand_contains_member_access(property)
                    || Self::same_value_operand_contains_member_access(value)
            }
            Expression::Binary { left, right, .. } => {
                Self::same_value_operand_contains_member_access(left)
                    || Self::same_value_operand_contains_member_access(right)
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                Self::same_value_operand_contains_member_access(condition)
                    || Self::same_value_operand_contains_member_access(then_expression)
                    || Self::same_value_operand_contains_member_access(else_expression)
            }
            Expression::Sequence(expressions) => expressions
                .iter()
                .any(Self::same_value_operand_contains_member_access),
            Expression::Call { callee, arguments }
            | Expression::SuperCall { callee, arguments }
            | Expression::New { callee, arguments } => {
                Self::same_value_operand_contains_member_access(callee)
                    || arguments.iter().any(|argument| match argument {
                        CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                            Self::same_value_operand_contains_member_access(expression)
                        }
                    })
            }
            Expression::Array(elements) => elements.iter().any(|element| match element {
                ArrayElement::Expression(expression) | ArrayElement::Spread(expression) => {
                    Self::same_value_operand_contains_member_access(expression)
                }
            }),
            Expression::Object(entries) => entries.iter().any(|entry| match entry {
                ObjectEntry::Data { key, value }
                | ObjectEntry::Getter { key, getter: value }
                | ObjectEntry::Setter { key, setter: value } => {
                    Self::same_value_operand_contains_member_access(key)
                        || Self::same_value_operand_contains_member_access(value)
                }
                ObjectEntry::Spread(expression) => {
                    Self::same_value_operand_contains_member_access(expression)
                }
            }),
            Expression::Identifier(_)
            | Expression::Number(_)
            | Expression::BigInt(_)
            | Expression::String(_)
            | Expression::Bool(_)
            | Expression::Null
            | Expression::Undefined
            | Expression::This
            | Expression::NewTarget
            | Expression::Sent
            | Expression::Update { .. } => false,
        }
    }

    fn same_value_operand_static_evaluation_safe(&self, expression: &Expression) -> bool {
        if matches!(
            expression,
            Expression::Binary {
                op: BinaryOp::LessThan
                    | BinaryOp::LessThanOrEqual
                    | BinaryOp::GreaterThan
                    | BinaryOp::GreaterThanOrEqual,
                ..
            }
        ) {
            return false;
        }
        if inline_summary_side_effect_free_expression(expression) {
            return true;
        }
        if matches!(
            expression,
            Expression::Identifier(name)
                if FunctionCompiler::module_index_from_namespace_like_identifier(name).is_some()
        ) {
            return true;
        }
        match expression {
            Expression::Unary { expression, .. }
            | Expression::Await(expression)
            | Expression::EnumerateKeys(expression)
            | Expression::GetIterator(expression)
            | Expression::IteratorClose(expression) => {
                self.same_value_operand_static_evaluation_safe(expression)
            }
            Expression::Binary { left, right, .. } => {
                self.same_value_operand_static_evaluation_safe(left)
                    && self.same_value_operand_static_evaluation_safe(right)
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                self.same_value_operand_static_evaluation_safe(condition)
                    && self.same_value_operand_static_evaluation_safe(then_expression)
                    && self.same_value_operand_static_evaluation_safe(else_expression)
            }
            Expression::Sequence(expressions) => expressions
                .iter()
                .all(|expression| self.same_value_operand_static_evaluation_safe(expression)),
            Expression::Call { callee, arguments } => {
                if let Expression::Member { object, property } = callee.as_ref()
                    && matches!(object.as_ref(), Expression::Identifier(name) if name == "Object")
                    && matches!(
                        property.as_ref(),
                        Expression::String(name) if matches!(name.as_str(), "getOwnPropertyNames" | "keys")
                    )
                    && let [CallArgument::Expression(target)] = arguments.as_slice()
                {
                    return self.resolve_array_binding_from_expression(target).is_some()
                        || self
                            .resolve_object_binding_from_expression(target)
                            .is_some();
                }
                if let Expression::Member { object, property } = callee.as_ref()
                    && matches!(object.as_ref(), Expression::Identifier(name) if name == "Object" || name == "Reflect")
                    && matches!(property.as_ref(), Expression::String(name) if name == "getPrototypeOf")
                    && let [CallArgument::Expression(target)] = arguments.as_slice()
                {
                    return self
                        .resolve_static_object_prototype_expression(target)
                        .is_some()
                        && self.same_value_operand_static_evaluation_safe(target);
                }
                matches!(
                    callee.as_ref(),
                    Expression::Identifier(name)
                        if matches!(name.as_str(), "Object" | "Boolean" | "Number" | "String")
                            && self.is_unshadowed_builtin_identifier(name)
                ) && arguments.iter().all(|argument| match argument {
                    CallArgument::Expression(expression) => {
                        self.same_value_operand_static_evaluation_safe(expression)
                    }
                    CallArgument::Spread(_) => false,
                }) || (inline_summary_side_effect_free_expression(callee)
                    && arguments.iter().all(|argument| match argument {
                        CallArgument::Expression(expression) => {
                            self.same_value_operand_static_evaluation_safe(expression)
                        }
                        CallArgument::Spread(_) => false,
                    })
                    && self
                        .resolve_function_binding_from_expression_with_context(
                            callee,
                            self.current_function_name(),
                        )
                        .and_then(|binding| match binding {
                            LocalFunctionBinding::User(function_name) => {
                                self.user_function(&function_name)
                            }
                            LocalFunctionBinding::Builtin(_) => None,
                        })
                        .is_some_and(|user_function| {
                            self.collect_user_function_assigned_nonlocal_bindings(user_function)
                                .is_empty()
                                && self
                                    .collect_user_function_call_effect_nonlocal_bindings(
                                        user_function,
                                    )
                                    .is_empty()
                        })
                    && self
                        .resolve_static_call_result_expression_with_context(
                            callee,
                            arguments,
                            self.current_function_name(),
                        )
                        .is_some())
            }
            Expression::New { callee, arguments } => {
                inline_summary_side_effect_free_expression(callee)
                    && arguments.iter().all(|argument| match argument {
                        CallArgument::Expression(expression) => {
                            self.same_value_operand_static_evaluation_safe(expression)
                        }
                        CallArgument::Spread(_) => false,
                    })
                    && self
                        .resolve_function_binding_from_expression_with_context(
                            callee,
                            self.current_function_name(),
                        )
                        .and_then(|binding| match binding {
                            LocalFunctionBinding::User(function_name) => {
                                self.user_function(&function_name)
                            }
                            LocalFunctionBinding::Builtin(_) => None,
                        })
                        .is_some_and(|user_function| {
                            user_function.is_constructible()
                                && self
                                    .collect_user_function_assigned_nonlocal_bindings(user_function)
                                    .is_empty()
                                && self
                                    .collect_user_function_call_effect_nonlocal_bindings(
                                        user_function,
                                    )
                                    .is_empty()
                        })
            }
            Expression::Member { object, property } => {
                if matches!(
                    object.as_ref(),
                    Expression::Call { callee, .. }
                        if !matches!(
                            callee.as_ref(),
                            Expression::Member { object, property }
                                if matches!(object.as_ref(), Expression::Identifier(name) if name == "Object")
                                    && matches!(
                                        property.as_ref(),
                                        Expression::String(name)
                                            if matches!(
                                                name.as_str(),
                                                "getOwnPropertyNames" | "keys" | "getPrototypeOf"
                                            )
                                    )
                        )
                ) {
                    return false;
                }
                if self
                    .same_value_assertion_direct_static_value(expression, 4)
                    .as_ref()
                    .is_some_and(|value| {
                        self.same_value_assertion_is_primitive_literal_operand(value)
                    })
                {
                    return true;
                }
                if matches!(
                    object.as_ref(),
                    Expression::Call { callee, arguments }
                        if matches!(
                            callee.as_ref(),
                            Expression::Member { object, property }
                                if matches!(object.as_ref(), Expression::Identifier(name) if name == "Object")
                                    && matches!(
                                        property.as_ref(),
                                        Expression::String(name) if matches!(name.as_str(), "getOwnPropertyNames" | "keys")
                                    )
                        ) && matches!(
                            arguments.as_slice(),
                            [CallArgument::Expression(target)]
                                if self.resolve_array_binding_from_expression(target).is_some()
                                    || self.resolve_object_binding_from_expression(target).is_some()
                        )
                ) && self.same_value_operand_static_evaluation_safe(property)
                {
                    return true;
                }
                if matches!(object.as_ref(), Expression::Call { callee, arguments }
                if matches!(
                    callee.as_ref(),
                    Expression::Member { object, property }
                        if matches!(object.as_ref(), Expression::Identifier(name) if name == "Object")
                            && matches!(property.as_ref(), Expression::String(name) if name == "getPrototypeOf")
                ) && matches!(
                    arguments.as_slice(),
                    [CallArgument::Expression(target)]
                        if self.same_value_operand_static_evaluation_safe(target)
                )) && self.same_value_operand_static_evaluation_safe(property)
                {
                    let materialized = self.materialize_static_expression(expression);
                    if !static_expression_matches(&materialized, expression)
                        && self.same_value_assertion_is_primitive_literal_operand(&materialized)
                    {
                        return true;
                    }
                }
                matches!(object.as_ref(), Expression::Call { callee, arguments }
                if matches!(
                    callee.as_ref(),
                    Expression::Member { object, property }
                        if matches!(object.as_ref(), Expression::Identifier(name) if name == "Object")
                            && matches!(property.as_ref(), Expression::String(name) if name == "getPrototypeOf")
                ) && matches!(
                    arguments.as_slice(),
                    [CallArgument::Expression(target)]
                        if self.resolve_static_object_prototype_expression(target).is_some()
                            && self.same_value_operand_static_evaluation_safe(target)
                )) && self.same_value_operand_static_evaluation_safe(property)
            }
            _ => false,
        }
    }

    fn same_value_assertion_primitive_result(
        &self,
        actual: &Expression,
        expected: &Expression,
    ) -> Option<bool> {
        match (actual, expected) {
            (Expression::Number(actual), Expression::Number(expected)) => {
                if actual.is_nan() && expected.is_nan() {
                    Some(true)
                } else if *actual == 0.0 && *expected == 0.0 {
                    Some(actual.is_sign_negative() == expected.is_sign_negative())
                } else {
                    Some(actual == expected)
                }
            }
            (Expression::BigInt(actual), Expression::BigInt(expected)) => {
                Some(parse_static_bigint_literal(actual)? == parse_static_bigint_literal(expected)?)
            }
            (Expression::String(actual), Expression::String(expected)) => Some(actual == expected),
            (Expression::Bool(actual), Expression::Bool(expected)) => Some(actual == expected),
            (Expression::Null, Expression::Null)
            | (Expression::Undefined, Expression::Undefined) => Some(true),
            _ => None,
        }
    }

    fn same_value_property_can_be_typed_array_or_array_buffer_member(
        property: &Expression,
    ) -> bool {
        matches!(
            static_property_name_from_expression(property).as_deref(),
            Some("length" | "byteLength" | "buffer" | "immutable")
        ) || argument_index_from_expression(property).is_some()
    }

    fn same_value_resolved_property_key(&self, property: &Expression) -> Expression {
        if let Some(property_name) = static_property_name_from_expression(property) {
            return Expression::String(property_name);
        }
        self.resolve_property_key_expression(property)
            .unwrap_or_else(|| self.materialize_static_expression(property))
    }

    fn same_value_assertion_descriptor_member_value(
        &self,
        object: &Expression,
        property: &Expression,
    ) -> Option<Expression> {
        let Expression::Identifier(name) = object else {
            return None;
        };
        let property_name = static_property_name_from_expression(property)?;
        if matches!(
            property_name.as_str(),
            "value" | "configurable" | "enumerable" | "writable" | "get" | "set"
        ) && self.local_binding_is_dynamic_property_descriptor_result(name)
        {
            return None;
        }
        let descriptor = self
            .resolve_identifier_descriptor_binding(name)
            .or_else(|| {
                self.state
                    .speculation
                    .static_semantics
                    .local_value_binding(name)
                    .or_else(|| self.global_value_binding(name))
                    .and_then(|value| self.resolve_descriptor_binding_from_expression(value))
            })?;
        match property_name.as_str() {
            "value" => descriptor.value,
            "configurable" => Some(Expression::Bool(descriptor.configurable)),
            "enumerable" => Some(Expression::Bool(descriptor.enumerable)),
            "writable" => descriptor.writable.map(Expression::Bool),
            "get" if descriptor.has_get => descriptor.getter.or(Some(Expression::Undefined)),
            "set" if descriptor.has_set => descriptor.setter.or(Some(Expression::Undefined)),
            _ => None,
        }
    }

    fn same_value_assertion_fast_object_property(
        &self,
        object: &Expression,
        property: &Expression,
        depth: usize,
    ) -> Option<Expression> {
        if depth == 0 {
            return None;
        }
        if let Some(value) = self.resolve_primitive_prototype_property_value(object, property) {
            return Some(value);
        }
        match object {
            Expression::Object(entries) => entries.iter().rev().find_map(|entry| {
                let ObjectEntry::Data { key, value } = entry else {
                    return None;
                };
                (static_property_name_from_expression(key).as_deref()
                    == static_property_name_from_expression(property).as_deref())
                .then(|| value.clone())
            }),
            Expression::Identifier(name) => {
                if let Some(value) = self
                    .state
                    .speculation
                    .static_semantics
                    .local_value_binding(name)
                    .or_else(|| self.global_value_binding(name))
                    && !static_expression_matches(value, object)
                    && let Some(resolved) =
                        self.same_value_assertion_fast_object_property(value, property, depth - 1)
                {
                    return Some(resolved);
                }
                self.state
                    .speculation
                    .static_semantics
                    .local_object_binding(name)
                    .or_else(|| self.backend.global_object_binding(name))
                    .and_then(|binding| object_binding_lookup_value(binding, property).cloned())
            }
            _ => None,
        }
    }

    fn same_value_assertion_fast_primitive_value(
        &self,
        expression: &Expression,
        depth: usize,
    ) -> Option<Expression> {
        if depth == 0 {
            return None;
        }
        match expression {
            Expression::Number(_)
            | Expression::BigInt(_)
            | Expression::String(_)
            | Expression::Bool(_)
            | Expression::Null
            | Expression::Undefined => Some(expression.clone()),
            Expression::Identifier(name)
                if matches!(name.as_str(), "undefined" | "NaN" | "Infinity")
                    && self.is_unshadowed_builtin_identifier(name) =>
            {
                match name.as_str() {
                    "undefined" => Some(Expression::Undefined),
                    "NaN" => Some(Expression::Number(f64::NAN)),
                    "Infinity" => Some(Expression::Number(f64::INFINITY)),
                    _ => None,
                }
            }
            Expression::Identifier(name) => self
                .state
                .speculation
                .static_semantics
                .local_value_binding(name)
                .or_else(|| self.global_value_binding(name))
                .filter(|value| !static_expression_matches(value, expression))
                .and_then(|value| self.same_value_assertion_fast_primitive_value(value, depth - 1)),
            Expression::Member { object, property } => {
                let resolved_property = self.same_value_resolved_property_key(property);
                if (is_symbol_to_string_tag_expression(property)
                    || is_symbol_to_string_tag_expression(&resolved_property))
                    && self
                        .deferred_module_namespace_materialized_object_module_index(object)
                        .is_some()
                {
                    return Some(Expression::String("Deferred Module".to_string()));
                }
                if matches!(
                    static_property_name_from_expression(&resolved_property).as_deref(),
                    Some("length")
                ) && !self.expression_uses_runtime_array_state(object)
                    && let Some(array_binding) =
                        self.same_value_assertion_direct_array_binding(object, depth - 1)
                {
                    return Some(Expression::Number(array_binding.values.len() as f64));
                }
                if let Some(index) = argument_index_from_expression(&resolved_property)
                    && let Some(array_binding) =
                        self.same_value_assertion_direct_array_binding(object, depth - 1)
                    && let Some(Some(value)) = array_binding.values.get(index as usize)
                {
                    return self
                        .same_value_assertion_fast_primitive_value(value, depth - 1)
                        .or_else(|| {
                            self.same_value_assertion_direct_static_value(value, depth - 1)
                        });
                }
                if let Some(value) =
                    self.same_value_assertion_descriptor_member_value(object, &resolved_property)
                {
                    return self
                        .same_value_assertion_fast_primitive_value(&value, depth - 1)
                        .or_else(|| {
                            self.same_value_assertion_direct_static_value(&value, depth - 1)
                        });
                }
                if Self::same_value_property_can_be_typed_array_or_array_buffer_member(
                    &resolved_property,
                ) && let Some(value) = self
                    .resolve_static_typed_array_or_array_buffer_member_value(
                        object,
                        &resolved_property,
                    )
                {
                    if let Some(value) =
                        self.same_value_assertion_fast_primitive_value(&value, depth - 1)
                    {
                        return Some(value);
                    }
                }
                if matches!(object.as_ref(), Expression::Identifier(_))
                    && let Some(value) = self.resolve_module_namespace_live_binding_member_value(
                        object,
                        &resolved_property,
                    )
                {
                    if let Some(value) =
                        self.same_value_assertion_fast_primitive_value(&value, depth - 1)
                    {
                        return Some(value);
                    }
                }
                if let Expression::Identifier(name) = object.as_ref()
                    && let Some(module_index) = Self::module_index_from_namespace_identifier(name)
                        .and_then(|index| index.parse::<usize>().ok())
                    && let Some(initializer) = self
                        .resolve_static_dynamic_import_namespace_live_binding_member_initializer_value(
                            module_index,
                            &resolved_property,
                        )
                {
                    return self
                        .same_value_assertion_fast_primitive_value(&initializer, depth - 1)
                        .or_else(|| {
                        self.same_value_assertion_direct_static_value(&initializer, depth - 1)
                    });
                }
                if matches!(object.as_ref(), Expression::Call { .. }) {
                    return None;
                }
                let property_name = static_property_name_from_expression(&resolved_property)?;
                let property = Expression::String(property_name);
                let value =
                    self.same_value_assertion_fast_object_property(object, &property, depth - 1)?;
                self.same_value_assertion_fast_primitive_value(&value, depth - 1)
            }
            Expression::Binary { op, left, right } => {
                let left_value = self.same_value_assertion_fast_primitive_value(left, depth - 1)?;
                let right_value =
                    self.same_value_assertion_fast_primitive_value(right, depth - 1)?;
                Self::same_value_static_binary_expression_value(*op, &left_value, &right_value)
            }
            Expression::Call { callee, arguments } => self
                .same_value_assertion_fast_static_call_primitive_value(
                    callee,
                    arguments,
                    depth - 1,
                ),
            _ => None,
        }
    }

    fn same_value_assertion_fast_static_call_primitive_value(
        &self,
        callee: &Expression,
        arguments: &[CallArgument],
        depth: usize,
    ) -> Option<Expression> {
        if depth == 0
            || !arguments.iter().all(|argument| match argument {
                CallArgument::Expression(expression) => {
                    self.same_value_operand_static_evaluation_safe(expression)
                }
                CallArgument::Spread(_) => false,
            })
        {
            return None;
        }

        let call_expression = Expression::Call {
            callee: Box::new(callee.clone()),
            arguments: arguments.to_vec(),
        };
        if let Some(result) = self.resolve_static_private_in_predicate_call_result(&call_expression)
        {
            return Some(Expression::Bool(result));
        }

        if matches!(callee, Expression::Identifier(name) if name == "eval")
            && let Some(argument_source) =
                self.static_eval_argument_source_from_arguments(arguments)
            && let Ok(program) = crate::frontend::parse_script_goal(&argument_source)
            && program.functions.is_empty()
            && let [Statement::Expression(expression)] = program.statements.as_slice()
        {
            return self
                .same_value_assertion_fast_primitive_value(expression, depth - 1)
                .or_else(|| self.same_value_assertion_direct_static_value(expression, depth - 1));
        }

        if let Some((value, result_function_name)) = self
            .resolve_static_member_builtin_call_result_with_context(
                callee,
                arguments,
                self.current_function_name(),
            )
        {
            let result_context = result_function_name
                .as_deref()
                .or_else(|| self.current_function_name());
            return self
                .resolve_static_primitive_expression_with_context(&value, result_context)
                .or_else(|| {
                    let materialized = self.materialize_static_expression(&value);
                    self.resolve_static_primitive_expression_with_context(
                        &materialized,
                        result_context,
                    )
                    .or_else(|| {
                        self.same_value_assertion_fast_primitive_value(&materialized, depth - 1)
                    })
                });
        }

        let LocalFunctionBinding::User(function_name) = self
            .resolve_function_binding_from_expression_with_context(
                callee,
                self.current_function_name(),
            )?
        else {
            return None;
        };
        let user_function = self.user_function(&function_name)?;
        if !self
            .collect_user_function_assigned_nonlocal_bindings(user_function)
            .is_empty()
            || !self
                .collect_user_function_call_effect_nonlocal_bindings(user_function)
                .is_empty()
        {
            return None;
        }

        let (value, result_function_name) = self
            .resolve_static_call_result_expression_with_context(
                callee,
                arguments,
                self.current_function_name(),
            )?;
        let result_context = result_function_name
            .as_deref()
            .or(Some(function_name.as_str()))
            .or_else(|| self.current_function_name());
        self.resolve_static_primitive_expression_with_context(&value, result_context)
            .or_else(|| {
                let materialized = self.materialize_static_expression(&value);
                self.resolve_static_primitive_expression_with_context(&materialized, result_context)
                    .or_else(|| {
                        self.same_value_assertion_fast_primitive_value(&materialized, depth - 1)
                    })
            })
    }

    fn same_value_assertion_fast_primitive_result(
        &self,
        actual: &Expression,
        expected: &Expression,
    ) -> Option<bool> {
        let actual_value = self.same_value_assertion_fast_primitive_value(actual, 6)?;
        let expected_value = self.same_value_assertion_fast_primitive_value(expected, 6)?;
        self.same_value_assertion_primitive_result(&actual_value, &expected_value)
    }

    fn same_value_assertion_bytes_import_primitive_value(
        &self,
        expression: &Expression,
        depth: usize,
    ) -> Option<Expression> {
        if depth == 0 {
            return None;
        }
        match expression {
            Expression::Number(_)
            | Expression::BigInt(_)
            | Expression::String(_)
            | Expression::Bool(_)
            | Expression::Null
            | Expression::Undefined => Some(expression.clone()),
            Expression::Identifier(name)
                if matches!(name.as_str(), "undefined" | "NaN" | "Infinity")
                    && self.is_unshadowed_builtin_identifier(name) =>
            {
                match name.as_str() {
                    "undefined" => Some(Expression::Undefined),
                    "NaN" => Some(Expression::Number(f64::NAN)),
                    "Infinity" => Some(Expression::Number(f64::INFINITY)),
                    _ => None,
                }
            }
            Expression::Member { object, property } => {
                let resolved_property = self.same_value_resolved_property_key(property);
                if !Self::same_value_property_can_be_typed_array_or_array_buffer_member(
                    &resolved_property,
                ) {
                    return None;
                }
                let value = self.resolve_static_typed_array_or_array_buffer_member_value(
                    object,
                    &resolved_property,
                )?;
                self.same_value_assertion_bytes_import_primitive_value(&value, depth - 1)
            }
            _ => None,
        }
    }

    fn same_value_assertion_bytes_import_primitive_result(
        &self,
        actual: &Expression,
        expected: &Expression,
    ) -> Option<bool> {
        let actual_value = self.same_value_assertion_bytes_import_primitive_value(actual, 4)?;
        let expected_value = self.same_value_assertion_bytes_import_primitive_value(expected, 4)?;
        self.same_value_assertion_primitive_result(&actual_value, &expected_value)
    }

    fn same_value_assertion_namespace_reflect_call_primitive_value(
        &self,
        callee: &Expression,
        arguments: &[CallArgument],
    ) -> Option<Expression> {
        let Expression::Member { object, property } = callee else {
            return None;
        };
        let Expression::Identifier(object_name) = object.as_ref() else {
            return None;
        };
        if !matches!(object_name.as_str(), "Object" | "Reflect") {
            return None;
        }
        let property_name = static_property_name_from_expression(property)?;
        match property_name.as_str() {
            "getPrototypeOf" => {
                let [CallArgument::Expression(target), ..] = arguments else {
                    return None;
                };
                FunctionCompiler::module_index_from_namespace_like_identifier(match target {
                    Expression::Identifier(name) => name,
                    _ => return None,
                })
                .map(|_| Expression::Null)
            }
            "isExtensible" => {
                let [CallArgument::Expression(target), ..] = arguments else {
                    return None;
                };
                FunctionCompiler::module_index_from_namespace_like_identifier(match target {
                    Expression::Identifier(name) => name,
                    _ => return None,
                })
                .map(|_| Expression::Bool(false))
            }
            "preventExtensions" if object_name == "Reflect" => {
                let [CallArgument::Expression(target), ..] = arguments else {
                    return None;
                };
                FunctionCompiler::module_index_from_namespace_like_identifier(match target {
                    Expression::Identifier(name) => name,
                    _ => return None,
                })
                .map(|_| Expression::Bool(true))
            }
            "setPrototypeOf" if object_name == "Reflect" => {
                let [
                    CallArgument::Expression(target),
                    CallArgument::Expression(prototype),
                    ..,
                ] = arguments
                else {
                    return None;
                };
                FunctionCompiler::module_index_from_namespace_like_identifier(match target {
                    Expression::Identifier(name) => name,
                    _ => return None,
                })?;
                Some(Expression::Bool(matches!(prototype, Expression::Null)))
            }
            "getOwnPropertyDescriptor" => {
                let [
                    CallArgument::Expression(target),
                    CallArgument::Expression(property),
                    ..,
                ] = arguments
                else {
                    return None;
                };
                FunctionCompiler::module_index_from_namespace_like_identifier(match target {
                    Expression::Identifier(name) => name,
                    _ => return None,
                })?;
                let resolved_property = self.same_value_resolved_property_key(property);
                if static_property_name_from_expression(&resolved_property).is_some()
                    && self
                        .resolve_call_descriptor_binding(callee, arguments)
                        .is_none()
                {
                    return Some(Expression::Undefined);
                }
                None
            }
            _ => None,
        }
    }

    fn same_value_assertion_namespace_descriptor_primitive_value(
        &self,
        expression: &Expression,
        depth: usize,
    ) -> Option<Expression> {
        if depth == 0 {
            return None;
        }
        match expression {
            Expression::Number(_)
            | Expression::BigInt(_)
            | Expression::String(_)
            | Expression::Bool(_)
            | Expression::Null
            | Expression::Undefined => Some(expression.clone()),
            Expression::Identifier(name)
                if matches!(name.as_str(), "undefined" | "NaN" | "Infinity")
                    && self.is_unshadowed_builtin_identifier(name) =>
            {
                match name.as_str() {
                    "undefined" => Some(Expression::Undefined),
                    "NaN" => Some(Expression::Number(f64::NAN)),
                    "Infinity" => Some(Expression::Number(f64::INFINITY)),
                    _ => None,
                }
            }
            Expression::Identifier(name) => self
                .state
                .speculation
                .static_semantics
                .local_value_binding(name)
                .or_else(|| self.global_value_binding(name))
                .filter(|value| !static_expression_matches(value, expression))
                .and_then(|value| {
                    self.same_value_assertion_namespace_descriptor_primitive_value(value, depth - 1)
                }),
            Expression::Unary {
                op: UnaryOp::TypeOf,
                expression,
            } if matches!(
                expression.as_ref(),
                Expression::Identifier(name)
                    if FunctionCompiler::module_index_from_namespace_like_identifier(name)
                        .is_some()
            ) =>
            {
                Some(Expression::String("object".to_string()))
            }
            Expression::Member { object, property } => {
                let resolved_property = self.same_value_resolved_property_key(property);
                let value =
                    self.same_value_assertion_descriptor_member_value(object, &resolved_property)?;
                self.same_value_assertion_namespace_descriptor_primitive_value(&value, depth - 1)
            }
            Expression::Call { callee, arguments } => self
                .same_value_assertion_namespace_reflect_call_primitive_value(callee, arguments)
                .and_then(|value| {
                    self.same_value_assertion_namespace_descriptor_primitive_value(
                        &value,
                        depth - 1,
                    )
                }),
            _ => None,
        }
    }

    fn same_value_assertion_namespace_descriptor_result(
        &self,
        actual: &Expression,
        expected: &Expression,
    ) -> Option<bool> {
        let actual_value =
            self.same_value_assertion_namespace_descriptor_primitive_value(actual, 6)?;
        let expected_value =
            self.same_value_assertion_namespace_descriptor_primitive_value(expected, 6)?;
        self.same_value_assertion_primitive_result(&actual_value, &expected_value)
    }

    fn same_value_assertion_object_names_length_value(
        &self,
        expression: &Expression,
    ) -> Option<Expression> {
        let Expression::Member { object, property } = expression else {
            return None;
        };
        if static_property_name_from_expression(property).as_deref() != Some("length") {
            return None;
        }
        if let Some(array_binding) = self
            .same_value_assertion_direct_array_binding(object, 4)
            .or_else(|| self.resolve_array_binding_from_expression(object))
        {
            return Some(Expression::Number(array_binding.values.len() as f64));
        }
        let Expression::Call { callee, arguments } = object.as_ref() else {
            return None;
        };
        let Expression::Member {
            object: callee_object,
            property: callee_property,
        } = callee.as_ref()
        else {
            return None;
        };
        let Expression::Identifier(callee_object_name) = callee_object.as_ref() else {
            return None;
        };
        if callee_object_name != "Object" {
            return None;
        }
        let Expression::String(callee_property_name) = callee_property.as_ref() else {
            return None;
        };
        let [CallArgument::Expression(target)] = arguments.as_slice() else {
            return None;
        };
        let array_binding = self.resolve_array_binding_from_expression(target);
        let object_binding = self.resolve_object_binding_from_expression(target);
        let property_names = match callee_property_name.as_str() {
            "getOwnPropertyNames" => array_binding
                .map(|binding| own_property_names_from_array_binding(&binding))
                .or_else(|| {
                    object_binding.map(|binding| own_property_names_from_object_binding(&binding))
                }),
            "keys" => array_binding
                .map(|binding| enumerated_keys_from_array_binding(&binding))
                .or_else(|| {
                    object_binding.map(|binding| enumerated_keys_from_object_binding(&binding))
                }),
            _ => None,
        }?;
        Some(Expression::Number(property_names.values.len() as f64))
    }

    fn same_value_assertion_fast_object_names_length_result(
        &self,
        actual: &Expression,
        expected: &Expression,
    ) -> Option<bool> {
        if let Some(actual_value) = self.same_value_assertion_object_names_length_value(actual) {
            let expected_value = self
                .same_value_assertion_fast_primitive_value(expected, 4)
                .or_else(|| self.same_value_assertion_direct_static_value(expected, 4))?;
            return self.same_value_assertion_primitive_result(&actual_value, &expected_value);
        }
        if let Some(expected_value) = self.same_value_assertion_object_names_length_value(expected)
        {
            let actual_value = self
                .same_value_assertion_fast_primitive_value(actual, 4)
                .or_else(|| self.same_value_assertion_direct_static_value(actual, 4))?;
            return self.same_value_assertion_primitive_result(&actual_value, &expected_value);
        }
        None
    }

    fn same_value_assertion_fast_reference_identity_key(
        &self,
        expression: &Expression,
        depth: usize,
    ) -> Option<String> {
        if depth == 0 {
            return None;
        }
        if let Some(key) = Self::same_value_import_meta_identity_key(expression) {
            return Some(key);
        }
        if let Expression::Identifier(name) = expression
            && name.starts_with("__ayy_capture_binding__")
        {
            return Some(format!("module-live-binding:{name}"));
        }
        if let Expression::Member { object, property } = expression {
            let resolved_property = self.same_value_resolved_property_key(property);
            if let Some(shadow_binding_name) = self
                .runtime_object_property_shadow_binding_name_for_expression(
                    object,
                    &resolved_property,
                )
                && let Some(value) = self
                    .global_value_binding(&shadow_binding_name)
                    .cloned()
                    .or_else(|| {
                        self.backend
                            .shared_global_semantics
                            .values
                            .value_bindings
                            .get(&shadow_binding_name)
                            .cloned()
                    })
                && let Some(key) =
                    self.same_value_assertion_fast_reference_identity_key(&value, depth - 1)
            {
                return Some(key);
            }
            if let Expression::Identifier(name) = object.as_ref()
                && let Some(module_index) = Self::module_index_from_namespace_identifier(name)
                    .and_then(|index| index.parse::<usize>().ok())
                && let Some(value) = self
                    .resolve_static_dynamic_import_namespace_live_binding_member_value(
                        module_index,
                        &resolved_property,
                    )
                && let Some(key) =
                    self.same_value_assertion_fast_reference_identity_key(&value, depth - 1)
            {
                return Some(key);
            }
            if let Some(value) =
                self.resolve_module_namespace_live_binding_member_value(object, &resolved_property)
            {
                let materialized = self.materialize_static_expression(&value);
                if let Some(key) =
                    self.same_value_assertion_fast_reference_identity_key(&materialized, depth - 1)
                {
                    return Some(key);
                }
                if let Some(key) =
                    self.same_value_assertion_fast_reference_identity_key(&value, depth - 1)
                {
                    return Some(key);
                }
                if let Expression::Identifier(name) = materialized {
                    return Some(format!("module-live-binding:{name}"));
                }
            }
            if let Some(object_binding) = self.resolve_object_binding_from_expression(object)
                && let Some(value) =
                    self.resolve_object_binding_property_value(&object_binding, &resolved_property)
                && let Some(key) =
                    self.same_value_assertion_fast_reference_identity_key(&value, depth - 1)
            {
                return Some(key);
            }
        }
        self.resolve_static_reference_identity_key(expression)
    }

    fn same_value_assertion_fast_reference_result(
        &self,
        actual: &Expression,
        expected: &Expression,
    ) -> Option<bool> {
        if self.same_value_assertion_is_primitive_literal_operand(actual)
            || self.same_value_assertion_is_primitive_literal_operand(expected)
        {
            return None;
        }
        let actual_key = self.same_value_assertion_fast_reference_identity_key(actual, 4)?;
        let expected_key = self.same_value_assertion_fast_reference_identity_key(expected, 4)?;
        Some(actual_key == expected_key)
    }

    fn same_value_assertion_direct_array_binding(
        &self,
        expression: &Expression,
        depth: usize,
    ) -> Option<ArrayValueBinding> {
        if depth == 0 {
            return None;
        }
        let array_binding_from_value = |value: Expression| {
            let Expression::Array(elements) = value else {
                return None;
            };
            let mut values = Vec::new();
            for element in elements {
                let ArrayElement::Expression(value) = element else {
                    return None;
                };
                values.push(Some(value));
            }
            Some(ArrayValueBinding { values })
        };
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
                    .and_then(|value| {
                        self.same_value_assertion_direct_call_array_binding(value, depth - 1)
                            .or_else(|| {
                                self.same_value_assertion_direct_static_value(value, depth - 1)
                                    .or_else(|| Some(value.clone()))
                                    .and_then(|value| array_binding_from_value(value))
                            })
                    });
                match (existing, derived) {
                    (Some(existing), Some(derived)) => {
                        let existing_defined = existing
                            .values
                            .iter()
                            .filter(|value| value.is_some())
                            .count();
                        let derived_defined = derived
                            .values
                            .iter()
                            .filter(|value| value.is_some())
                            .count();
                        if derived.values.len() > existing.values.len()
                            || derived_defined > existing_defined
                        {
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
            Expression::Call { .. } => {
                self.same_value_assertion_direct_call_array_binding(expression, depth - 1)
            }
            Expression::Member { object, property } => {
                let resolved_property = self.same_value_resolved_property_key(property);
                if let Expression::Identifier(name) = object.as_ref()
                    && let Some(module_index) =
                        Self::module_index_from_namespace_like_identifier(name)
                    && let Some(value) = self
                        .resolve_static_dynamic_import_namespace_live_binding_member_value(
                            module_index,
                            &resolved_property,
                        )
                        .or_else(|| {
                            self.resolve_static_dynamic_import_namespace_live_binding_member_value(
                                module_index,
                                property,
                            )
                        })
                    && !static_expression_matches(&value, expression)
                {
                    return self
                        .same_value_assertion_direct_array_binding(&value, depth - 1)
                        .or_else(|| {
                            self.same_value_assertion_direct_static_value(&value, depth - 1)
                                .or_else(|| Some(value))
                                .and_then(array_binding_from_value)
                        });
                }
                if let Some(value) = self
                    .resolve_module_namespace_live_binding_member_value(object, &resolved_property)
                {
                    return self
                        .same_value_assertion_direct_array_binding(&value, depth - 1)
                        .or_else(|| {
                            self.same_value_assertion_direct_static_value(&value, depth - 1)
                                .or_else(|| Some(value))
                                .and_then(array_binding_from_value)
                        });
                }
                if let Some(value) =
                    self.same_value_assertion_direct_static_value(expression, depth - 1)
                {
                    return self
                        .same_value_assertion_direct_array_binding(&value, depth - 1)
                        .or_else(|| array_binding_from_value(value));
                }
                None
            }
            Expression::Array(elements) => {
                let mut values = Vec::new();
                for element in elements {
                    let ArrayElement::Expression(value) = element else {
                        return None;
                    };
                    values.push(Some(value.clone()));
                }
                Some(ArrayValueBinding { values })
            }
            _ => match self.same_value_assertion_direct_static_value(expression, depth - 1)? {
                Expression::Array(elements) => {
                    array_binding_from_value(Expression::Array(elements))
                }
                Expression::Identifier(name) => self
                    .state
                    .speculation
                    .static_semantics
                    .local_array_binding(&name)
                    .cloned()
                    .or_else(|| self.backend.global_array_binding(&name).cloned()),
                _ => None,
            },
        }
    }

    fn same_value_assertion_binding_seed_value(
        &self,
        name: &str,
        bindings: &std::collections::HashMap<String, Expression>,
    ) -> Option<Expression> {
        bindings
            .get(name)
            .cloned()
            .or_else(|| {
                self.state
                    .speculation
                    .static_semantics
                    .local_value_binding(name)
                    .cloned()
            })
            .or_else(|| self.global_value_binding(name).cloned())
            .or_else(|| {
                self.prepared_program
                    .required_global_static_semantics()
                    .values
                    .value_binding(name)
                    .cloned()
            })
    }

    fn same_value_assertion_evaluate_simple_static_expression(
        &self,
        expression: &Expression,
        bindings: &mut std::collections::HashMap<String, Expression>,
        depth: usize,
    ) -> Option<Expression> {
        if depth == 0 {
            return None;
        }
        match expression {
            Expression::Number(_)
            | Expression::BigInt(_)
            | Expression::String(_)
            | Expression::Bool(_)
            | Expression::Null
            | Expression::Undefined => Some(expression.clone()),
            Expression::Identifier(name) => {
                self.same_value_assertion_binding_seed_value(name, bindings)
            }
            Expression::Update { name, op, prefix } => {
                let current = self.same_value_assertion_binding_seed_value(name, bindings)?;
                let current_number = match current {
                    Expression::Number(value) => value,
                    Expression::Bool(true) => 1.0,
                    Expression::Bool(false) | Expression::Null => 0.0,
                    Expression::Undefined => f64::NAN,
                    _ => return None,
                };
                let next_number = match op {
                    UpdateOp::Increment => current_number + 1.0,
                    UpdateOp::Decrement => current_number - 1.0,
                };
                let next = Expression::Number(next_number);
                bindings.insert(name.clone(), next.clone());
                if *prefix {
                    Some(next)
                } else {
                    Some(Expression::Number(current_number))
                }
            }
            Expression::Sequence(expressions) => {
                let mut value = Expression::Undefined;
                for expression in expressions {
                    value = self
                        .same_value_assertion_evaluate_simple_static_expression(
                            expression,
                            bindings,
                            depth - 1,
                        )
                        .or_else(|| {
                            self.same_value_assertion_direct_static_value(expression, depth - 1)
                        })?;
                }
                Some(value)
            }
            Expression::Unary {
                op: UnaryOp::Plus,
                expression,
            } => {
                let value = self.same_value_assertion_evaluate_simple_static_expression(
                    expression,
                    bindings,
                    depth - 1,
                )?;
                static_numeric_property_name_value(&value).map(Expression::Number)
            }
            Expression::Unary {
                op: UnaryOp::Negate,
                expression,
            } => {
                let value = self.same_value_assertion_evaluate_simple_static_expression(
                    expression,
                    bindings,
                    depth - 1,
                )?;
                static_numeric_property_name_value(&value).map(|number| Expression::Number(-number))
            }
            Expression::Binary { op, left, right } => {
                let left_value = self
                    .same_value_assertion_evaluate_simple_static_expression(
                        left,
                        bindings,
                        depth - 1,
                    )
                    .or_else(|| self.same_value_assertion_direct_static_value(left, depth - 1))?;
                let right_value = self
                    .same_value_assertion_evaluate_simple_static_expression(
                        right,
                        bindings,
                        depth - 1,
                    )
                    .or_else(|| self.same_value_assertion_direct_static_value(right, depth - 1))?;
                Self::same_value_static_binary_expression_value(*op, &left_value, &right_value)
            }
            _ => self.same_value_assertion_direct_static_value(expression, depth - 1),
        }
    }

    fn same_value_assertion_evaluate_object_literal_key(
        &self,
        key: &Expression,
        bindings: &mut std::collections::HashMap<String, Expression>,
        depth: usize,
    ) -> Option<Expression> {
        if depth == 0 {
            return None;
        }
        let evaluated =
            self.same_value_assertion_evaluate_simple_static_expression(key, bindings, depth - 1);
        if let Some(evaluated) = evaluated.as_ref() {
            if let Some(property_name) = static_property_name_from_expression(evaluated) {
                return Some(Expression::String(property_name));
            }
            if let Some(resolved) = self.resolve_property_key_expression(evaluated) {
                return Some(resolved);
            }
        }
        self.resolve_property_key_expression(key)
            .or(evaluated)
            .and_then(|resolved| {
                static_property_name_from_expression(&resolved)
                    .map(Expression::String)
                    .or(Some(resolved))
            })
    }

    fn same_value_assertion_direct_object_literal_binding(
        &self,
        entries: &[ObjectEntry],
        depth: usize,
    ) -> Option<ObjectValueBinding> {
        if depth == 0 {
            return None;
        }
        let mut object_binding = empty_object_value_binding();
        let mut bindings = std::collections::HashMap::new();
        for entry in entries {
            let ObjectEntry::Data { key, value } = entry else {
                return None;
            };
            if object_entry_is_literal_proto_setter(entry) {
                continue;
            }
            let key =
                self.same_value_assertion_evaluate_object_literal_key(key, &mut bindings, depth)?;
            let value = self
                .same_value_assertion_evaluate_simple_static_expression(
                    value,
                    &mut bindings,
                    depth - 1,
                )
                .or_else(|| self.same_value_assertion_direct_static_value(value, depth - 1))?;
            object_binding_set_property(&mut object_binding, key, value);
        }
        Some(object_binding)
    }

    fn same_value_assertion_direct_object_binding(
        &self,
        expression: &Expression,
        depth: usize,
    ) -> Option<ObjectValueBinding> {
        if depth == 0 {
            return None;
        }
        match expression {
            Expression::Object(entries) => {
                self.same_value_assertion_direct_object_literal_binding(entries, depth - 1)
            }
            Expression::Identifier(name) => {
                let value_binding = self
                    .state
                    .speculation
                    .static_semantics
                    .local_value_binding(name)
                    .or_else(|| self.global_value_binding(name));
                let existing = self
                    .state
                    .speculation
                    .static_semantics
                    .local_object_binding(name)
                    .cloned()
                    .or_else(|| self.backend.global_object_binding(name).cloned());
                let derived = value_binding.and_then(|value| {
                    self.same_value_assertion_direct_object_binding(value, depth - 1)
                });
                if existing
                    .as_ref()
                    .is_none_or(|binding| ordered_object_property_names(binding).is_empty())
                    && derived.is_some()
                {
                    derived
                } else {
                    existing.or(derived)
                }
            }
            _ => match self.same_value_assertion_direct_static_value(expression, depth - 1)? {
                Expression::Object(entries) => {
                    self.same_value_assertion_direct_object_literal_binding(&entries, depth - 1)
                }
                Expression::Identifier(name) => self.same_value_assertion_direct_object_binding(
                    &Expression::Identifier(name),
                    depth - 1,
                ),
                _ => None,
            },
        }
    }

    fn same_value_assertion_direct_call_array_binding(
        &self,
        expression: &Expression,
        depth: usize,
    ) -> Option<ArrayValueBinding> {
        if depth == 0 {
            return None;
        }
        let Expression::Call { callee, arguments } = expression else {
            return None;
        };
        let Expression::Member { object, property } = callee.as_ref() else {
            return None;
        };
        let Some(target) = arguments.first().and_then(|argument| match argument {
            CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                Some(expression)
            }
        }) else {
            return None;
        };
        if let Some(binding) = self.static_builtin_object_array_call_binding(callee, arguments) {
            return Some(binding);
        }
        match (object.as_ref(), property.as_ref()) {
            (Expression::Identifier(name), Expression::String(property))
                if name == "Object" && property == "getOwnPropertyNames" =>
            {
                if let Some(array_binding) =
                    self.same_value_assertion_direct_array_binding(target, depth - 1)
                {
                    return Some(own_property_names_from_array_binding(&array_binding));
                }
                let object_binding =
                    self.same_value_assertion_direct_object_binding(target, depth - 1)?;
                Some(own_property_names_from_object_binding(&object_binding))
            }
            (Expression::Identifier(name), Expression::String(property))
                if name == "Object" && property == "keys" =>
            {
                if let Some(array_binding) =
                    self.same_value_assertion_direct_array_binding(target, depth - 1)
                {
                    return Some(enumerated_keys_from_array_binding(&array_binding));
                }
                let object_binding =
                    self.same_value_assertion_direct_object_binding(target, depth - 1)?;
                Some(enumerated_keys_from_object_binding(&object_binding))
            }
            _ => None,
        }
    }

    fn same_value_assertion_direct_object_property(
        &self,
        object: &Expression,
        property: &Expression,
        depth: usize,
    ) -> Option<Expression> {
        if depth == 0 {
            return None;
        }
        if let Some(value) = self.resolve_primitive_prototype_property_value(object, property) {
            return Some(value);
        }
        if let Some(value) = self
            .same_value_assertion_direct_object_binding(object, depth - 1)
            .and_then(|binding| object_binding_lookup_value(&binding, property).cloned())
        {
            return Some(value);
        }
        let lookup_direct_object = |entries: &[ObjectEntry]| {
            let property_name = static_property_name_from_expression(property)?;
            entries.iter().rev().find_map(|entry| {
                let ObjectEntry::Data { key, value } = entry else {
                    return None;
                };
                (static_property_name_from_expression(key).as_deref()
                    == Some(property_name.as_str()))
                .then(|| value.clone())
            })
        };
        match object {
            Expression::Object(entries) => lookup_direct_object(entries),
            Expression::Identifier(name) => self
                .state
                .speculation
                .static_semantics
                .local_object_binding(name)
                .or_else(|| self.backend.global_object_binding(name))
                .and_then(|binding| object_binding_lookup_value(binding, property).cloned())
                .or_else(|| {
                    self.state
                        .speculation
                        .static_semantics
                        .local_value_binding(name)
                        .or_else(|| self.global_value_binding(name))
                        .and_then(|value| {
                            self.same_value_assertion_direct_static_value(value, depth - 1)
                                .or_else(|| Some(value.clone()))
                        })
                        .and_then(|value| match value {
                            Expression::Object(entries) => lookup_direct_object(&entries),
                            _ => None,
                        })
                }),
            _ => match self.same_value_assertion_direct_static_value(object, depth - 1)? {
                Expression::Object(entries) => lookup_direct_object(&entries),
                Expression::Identifier(name) => self
                    .state
                    .speculation
                    .static_semantics
                    .local_object_binding(&name)
                    .or_else(|| self.backend.global_object_binding(&name))
                    .and_then(|binding| object_binding_lookup_value(binding, property).cloned()),
                _ => None,
            },
        }
    }

    fn same_value_assertion_direct_static_value(
        &self,
        expression: &Expression,
        depth: usize,
    ) -> Option<Expression> {
        if depth == 0 {
            return None;
        }
        match expression {
            Expression::Number(_)
            | Expression::BigInt(_)
            | Expression::String(_)
            | Expression::Bool(_)
            | Expression::Null
            | Expression::Undefined
            | Expression::Array(_)
            | Expression::Object(_) => Some(expression.clone()),
            Expression::Identifier(name)
                if matches!(name.as_str(), "undefined" | "NaN" | "Infinity")
                    && self.is_unshadowed_builtin_identifier(name) =>
            {
                match name.as_str() {
                    "undefined" => Some(Expression::Undefined),
                    "NaN" => Some(Expression::Number(f64::NAN)),
                    "Infinity" => Some(Expression::Number(f64::INFINITY)),
                    _ => None,
                }
            }
            Expression::Identifier(name) => self
                .state
                .speculation
                .static_semantics
                .local_value_binding(name)
                .or_else(|| self.global_value_binding(name))
                .and_then(|value| {
                    self.same_value_assertion_direct_static_value(value, depth - 1)
                        .or_else(|| Some(value.clone()))
                }),
            Expression::Member { object, property } => {
                if matches!(
                    object.as_ref(),
                    Expression::Call { callee, .. }
                        if !matches!(
                            callee.as_ref(),
                            Expression::Member { object, property }
                                if matches!(object.as_ref(), Expression::Identifier(name) if name == "Object")
                                    && matches!(
                                        property.as_ref(),
                                        Expression::String(name)
                                            if matches!(
                                                name.as_str(),
                                                "getOwnPropertyNames" | "keys" | "getPrototypeOf"
                                            )
                                    )
                        )
                ) {
                    return None;
                }
                if let Expression::Call { callee, arguments } = object.as_ref()
                    && matches!(
                        callee.as_ref(),
                        Expression::Member { object, property }
                            if matches!(object.as_ref(), Expression::Identifier(name) if name == "Object")
                                && matches!(property.as_ref(), Expression::String(name) if name == "getPrototypeOf")
                    )
                    && let Some(CallArgument::Expression(target) | CallArgument::Spread(target)) =
                        arguments.first()
                    && let Some(prototype) = self.resolve_static_object_prototype_expression(target)
                {
                    let prototype_member = Expression::Member {
                        object: Box::new(prototype),
                        property: property.clone(),
                    };
                    if let Some(value) =
                        self.same_value_assertion_direct_static_value(&prototype_member, depth - 1)
                    {
                        return Some(value);
                    }
                    let materialized = self.materialize_static_expression(&prototype_member);
                    if self.same_value_assertion_is_primitive_literal_operand(&materialized) {
                        return Some(materialized);
                    }
                }
                let resolved_property = if self.same_value_operand_static_evaluation_safe(property)
                {
                    self.same_value_assertion_direct_static_value(property, depth - 1)
                        .filter(|value| static_property_name_from_expression(value).is_some())
                } else {
                    None
                };
                let original_property = property.as_ref();
                let property = resolved_property.as_ref().unwrap_or(original_property);
                if (is_symbol_to_string_tag_expression(original_property)
                    || is_symbol_to_string_tag_expression(property))
                    && self
                        .deferred_module_namespace_materialized_object_module_index(object)
                        .is_some()
                {
                    return Some(Expression::String("Deferred Module".to_string()));
                }
                if let Some(value) =
                    self.same_value_assertion_descriptor_member_value(object, property)
                {
                    return self
                        .same_value_assertion_direct_static_value(&value, depth - 1)
                        .or(Some(value));
                }
                if Self::same_value_property_can_be_typed_array_or_array_buffer_member(property)
                    && let Some(value) = self
                        .resolve_static_typed_array_or_array_buffer_member_value(object, property)
                {
                    if let Some(value) =
                        self.same_value_assertion_direct_static_value(&value, depth - 1)
                    {
                        return Some(value);
                    }
                }
                if matches!(object.as_ref(), Expression::Identifier(_))
                    && let Some(value) =
                        self.resolve_module_namespace_live_binding_member_value(object, property)
                {
                    if let Some(value) =
                        self.same_value_assertion_direct_static_value(&value, depth - 1)
                    {
                        return Some(value);
                    }
                }
                if let Expression::Identifier(name) = object.as_ref()
                    && let Some(module_index) = Self::module_index_from_namespace_identifier(name)
                        .and_then(|index| index.parse::<usize>().ok())
                    && let Some(initializer) = self
                        .resolve_static_dynamic_import_namespace_live_binding_member_initializer_value(
                            module_index,
                            property,
                        )
                {
                    return self
                        .same_value_assertion_direct_static_value(&initializer, depth - 1)
                        .or(Some(initializer));
                }
                if let Some(getter_binding) = self.resolve_member_getter_binding(object, property)
                    && let Some(value) = self.resolve_static_getter_value_from_binding_with_context(
                        &getter_binding,
                        object,
                        self.current_function_name(),
                    )
                {
                    if let Some(primitive) = self.resolve_static_primitive_expression_with_context(
                        &value,
                        self.current_function_name(),
                    ) {
                        return Some(primitive);
                    }
                    let materialized = self.materialize_static_expression(&value);
                    if let Some(primitive) = self.resolve_static_primitive_expression_with_context(
                        &materialized,
                        self.current_function_name(),
                    ) {
                        return Some(primitive);
                    }
                    return self
                        .same_value_assertion_direct_static_value(&value, depth - 1)
                        .or(Some(value));
                }
                if matches!(
                    static_property_name_from_expression(property).as_deref(),
                    Some("length")
                ) && !self.expression_uses_runtime_array_state(object)
                    && let Some(array_binding) =
                        self.same_value_assertion_direct_array_binding(object, depth - 1)
                {
                    return Some(Expression::Number(array_binding.values.len() as f64));
                }
                if let Some(index) = argument_index_from_expression(property)
                    && let Some(array_binding) =
                        self.same_value_assertion_direct_array_binding(object, depth - 1)
                {
                    if let Some(Some(value)) = array_binding.values.get(index as usize) {
                        return self
                            .same_value_assertion_direct_static_value(value, depth - 1)
                            .or_else(|| Some(value.clone()));
                    }
                    return Some(Expression::Undefined);
                }
                if let Some(value) =
                    self.same_value_assertion_direct_object_property(object, property, depth - 1)
                {
                    return self
                        .same_value_assertion_direct_static_value(&value, depth - 1)
                        .or(Some(value));
                }
                None
            }
            Expression::Binary { op, left, right } => {
                let left_value = self.same_value_assertion_direct_static_value(left, depth - 1)?;
                let right_value =
                    self.same_value_assertion_direct_static_value(right, depth - 1)?;
                Self::same_value_static_binary_expression_value(*op, &left_value, &right_value)
            }
            Expression::Unary {
                op: UnaryOp::TypeOf,
                expression,
            } => self
                .same_value_assertion_direct_typeof_string(expression, depth - 1)
                .map(Expression::String),
            _ => None,
        }
    }

    fn same_value_assertion_direct_typeof_string(
        &self,
        expression: &Expression,
        depth: usize,
    ) -> Option<String> {
        if depth == 0 {
            return None;
        }
        let value = self
            .same_value_assertion_direct_static_value(expression, depth - 1)
            .unwrap_or_else(|| expression.clone());
        if self.same_value_assertion_direct_value_is_function(&value) {
            return Some("function".to_string());
        }
        match value {
            Expression::Number(_) => Some("number".to_string()),
            Expression::BigInt(_) => Some("bigint".to_string()),
            Expression::String(_) => Some("string".to_string()),
            Expression::Bool(_) => Some("boolean".to_string()),
            Expression::Null => Some("object".to_string()),
            Expression::Undefined => Some("undefined".to_string()),
            Expression::Array(_) | Expression::Object(_) | Expression::This => {
                Some("object".to_string())
            }
            Expression::Identifier(ref name) => self
                .backend
                .global_property_descriptor(name)
                .or_else(|| {
                    self.backend
                        .shared_global_semantics
                        .values
                        .property_descriptor(name)
                })
                .is_some_and(|state| state.has_get || state.getter.is_some())
                .then_some(None)
                .unwrap_or_else(|| {
                    self.state
                        .speculation
                        .static_semantics
                        .local_kind(name)
                        .or_else(|| self.global_binding_kind(name))
                        .or_else(|| builtin_identifier_kind(name))
                        .and_then(StaticValueKind::as_typeof_str)
                        .map(str::to_string)
                }),
            _ => None,
        }
    }

    fn same_value_assertion_direct_value_is_function(&self, expression: &Expression) -> bool {
        match expression {
            Expression::Identifier(name) => {
                self.state
                    .speculation
                    .static_semantics
                    .local_function_binding(name)
                    .is_some()
                    || self.backend.global_function_binding(name).is_some()
                    || builtin_identifier_kind(name) == Some(StaticValueKind::Function)
                    || self
                        .backend
                        .function_registry
                        .catalog
                        .user_function(name)
                        .is_some()
            }
            _ => false,
        }
    }

    fn same_value_assertion_direct_static_result(
        &self,
        actual: &Expression,
        expected: &Expression,
    ) -> Option<bool> {
        let actual_value = self
            .same_value_assertion_direct_static_value(actual, 10)
            .unwrap_or_else(|| actual.clone());
        let expected_value = self
            .same_value_assertion_direct_static_value(expected, 10)
            .unwrap_or_else(|| expected.clone());
        self.same_value_assertion_primitive_result(&actual_value, &expected_value)
    }

    fn same_value_assertion_is_primitive_literal_operand(&self, expression: &Expression) -> bool {
        matches!(
            expression,
            Expression::Number(_)
                | Expression::BigInt(_)
                | Expression::String(_)
                | Expression::Bool(_)
                | Expression::Null
                | Expression::Undefined
        ) || matches!(
            expression,
            Expression::Identifier(name)
                if matches!(name.as_str(), "undefined" | "NaN" | "Infinity")
                && self.is_unshadowed_builtin_identifier(name)
        )
    }

    fn same_value_assertion_is_tracked_array_index_member(&self, expression: &Expression) -> bool {
        let Expression::Member { object, property } = expression else {
            return false;
        };
        if argument_index_from_expression(property).is_none() {
            return false;
        }
        let Expression::Identifier(name) = object.as_ref() else {
            return false;
        };
        self.runtime_array_binding_name_for_expression(&Expression::Identifier(name.clone()))
            .is_some_and(|binding_name| {
                self.runtime_array_binding_has_state(&binding_name)
                    || self.backend.global_array_binding(&binding_name).is_some()
                    || self
                        .backend
                        .shared_global_semantics
                        .values
                        .array_bindings
                        .contains_key(&binding_name)
            })
    }

    fn same_value_assertion_has_tracked_array_element_base(&self, expression: &Expression) -> bool {
        match expression {
            Expression::Member { object, .. } => {
                self.same_value_assertion_is_tracked_array_index_member(object)
                    || self.same_value_assertion_has_tracked_array_element_base(object)
            }
            Expression::Unary {
                op: UnaryOp::TypeOf,
                expression,
            } => self.same_value_assertion_has_tracked_array_element_base(expression),
            _ => false,
        }
    }

    fn same_value_assertion_tracked_array_snapshot_binding(
        &self,
        name: &str,
    ) -> Option<ArrayValueBinding> {
        self.state
            .speculation
            .static_semantics
            .local_array_binding(name)
            .cloned()
            .or_else(|| self.backend.global_array_binding(name).cloned())
            .or_else(|| {
                self.backend
                    .shared_global_semantics
                    .values
                    .array_binding(name)
                    .cloned()
            })
    }

    fn same_value_assertion_tracked_array_snapshot_index_value(
        &self,
        object: &Expression,
        property: &Expression,
    ) -> Option<Expression> {
        let index = argument_index_from_expression(property)?;
        let Expression::Identifier(name) = object else {
            return None;
        };
        let binding_name = self
            .runtime_array_binding_name_for_expression(object)
            .unwrap_or_else(|| name.clone());
        self.same_value_assertion_tracked_array_snapshot_binding(&binding_name)
            .or_else(|| self.same_value_assertion_tracked_array_snapshot_binding(name))?
            .values
            .get(index as usize)
            .cloned()
            .flatten()
    }

    fn same_value_assertion_tracked_array_snapshot_length(
        &self,
        object: &Expression,
    ) -> Option<Expression> {
        let Expression::Identifier(name) = object else {
            return None;
        };
        let binding_name = self
            .runtime_array_binding_name_for_expression(object)
            .unwrap_or_else(|| name.clone());
        let length = self
            .same_value_assertion_tracked_array_snapshot_binding(&binding_name)
            .or_else(|| self.same_value_assertion_tracked_array_snapshot_binding(name))?
            .values
            .len();
        Some(Expression::Number(length as f64))
    }

    fn same_value_assertion_object_literal_data_member(
        &self,
        entries: &[ObjectEntry],
        property: &Expression,
    ) -> Option<Expression> {
        let property_name = static_property_name_from_expression(property)?;
        entries.iter().rev().find_map(|entry| {
            let ObjectEntry::Data { key, value } = entry else {
                return None;
            };
            let key = self
                .resolve_property_key_expression(key)
                .unwrap_or_else(|| self.materialize_static_expression(key));
            (static_property_name_from_expression(&key).as_deref() == Some(property_name.as_str()))
                .then(|| value.clone())
        })
    }

    fn same_value_assertion_tracked_array_snapshot_typeof(
        &self,
        expression: &Expression,
        depth: usize,
    ) -> Option<Expression> {
        let value = self.same_value_assertion_tracked_array_snapshot_value(expression, depth)?;
        if self.same_value_assertion_direct_value_is_function(&value) {
            return Some(Expression::String("function".to_string()));
        }
        let type_name = match value {
            Expression::Number(_) => "number",
            Expression::BigInt(_) => "bigint",
            Expression::String(_) => "string",
            Expression::Bool(_) => "boolean",
            Expression::Null => "object",
            Expression::Undefined => "undefined",
            Expression::Array(_) | Expression::Object(_) | Expression::This => "object",
            Expression::Identifier(ref name) => self
                .state
                .speculation
                .static_semantics
                .local_kind(name)
                .or_else(|| self.global_binding_kind(name))
                .or_else(|| builtin_identifier_kind(name))
                .and_then(StaticValueKind::as_typeof_str)?,
            _ => return None,
        };
        Some(Expression::String(type_name.to_string()))
    }

    fn same_value_assertion_tracked_array_snapshot_value(
        &self,
        expression: &Expression,
        depth: usize,
    ) -> Option<Expression> {
        if depth == 0 {
            return None;
        }
        match expression {
            Expression::Number(_)
            | Expression::BigInt(_)
            | Expression::String(_)
            | Expression::Bool(_)
            | Expression::Null
            | Expression::Undefined
            | Expression::Array(_)
            | Expression::Object(_)
            | Expression::This => Some(expression.clone()),
            Expression::Identifier(name)
                if matches!(name.as_str(), "undefined" | "NaN" | "Infinity")
                    && self.is_unshadowed_builtin_identifier(name) =>
            {
                match name.as_str() {
                    "undefined" => Some(Expression::Undefined),
                    "NaN" => Some(Expression::Number(f64::NAN)),
                    "Infinity" => Some(Expression::Number(f64::INFINITY)),
                    _ => None,
                }
            }
            Expression::Identifier(name) => self
                .state
                .speculation
                .static_semantics
                .local_value_binding(name)
                .or_else(|| self.global_value_binding(name))
                .and_then(|value| match value {
                    Expression::Number(_)
                    | Expression::BigInt(_)
                    | Expression::String(_)
                    | Expression::Bool(_)
                    | Expression::Null
                    | Expression::Undefined
                    | Expression::Array(_)
                    | Expression::Object(_) => Some(value.clone()),
                    _ => None,
                })
                .or_else(|| Some(expression.clone())),
            Expression::Unary {
                op: UnaryOp::TypeOf,
                expression,
            } => self.same_value_assertion_tracked_array_snapshot_typeof(expression, depth - 1),
            Expression::Member { object, property } => {
                let property = self
                    .resolve_property_key_expression(property)
                    .unwrap_or_else(|| self.materialize_static_expression(property));
                if matches!(property, Expression::String(ref name) if name == "length")
                    && let Some(length) =
                        self.same_value_assertion_tracked_array_snapshot_length(object)
                {
                    return Some(length);
                }
                if let Some(value) =
                    self.same_value_assertion_tracked_array_snapshot_index_value(object, &property)
                {
                    return Some(value);
                }
                let object_value =
                    self.same_value_assertion_tracked_array_snapshot_value(object, depth - 1)?;
                match object_value {
                    Expression::Array(elements) => {
                        if matches!(property, Expression::String(ref name) if name == "length") {
                            return Some(Expression::Number(elements.len() as f64));
                        }
                        let index = argument_index_from_expression(&property)? as usize;
                        let Some(ArrayElement::Expression(value)) = elements.get(index) else {
                            return Some(Expression::Undefined);
                        };
                        Some(value.clone())
                    }
                    Expression::Object(entries) => {
                        self.same_value_assertion_object_literal_data_member(&entries, &property)
                    }
                    Expression::Identifier(name) => self
                        .state
                        .speculation
                        .static_semantics
                        .local_object_binding(&name)
                        .or_else(|| self.backend.global_object_binding(&name))
                        .and_then(|binding| {
                            self.resolve_object_binding_property_value(binding, &property)
                        }),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    fn same_value_assertion_tracked_array_snapshot_result(
        &self,
        actual: &Expression,
        expected: &Expression,
    ) -> Option<bool> {
        if !self.same_value_assertion_has_tracked_array_element_base(actual)
            && !self.same_value_assertion_has_tracked_array_element_base(expected)
        {
            return None;
        }
        let actual_value = self.same_value_assertion_tracked_array_snapshot_value(actual, 10)?;
        let expected_value =
            self.same_value_assertion_tracked_array_snapshot_value(expected, 10)?;
        self.same_value_assertion_primitive_result(&actual_value, &expected_value)
    }

    fn emit_same_value_assertion_runtime_compare(
        &mut self,
        arguments: &[CallArgument],
        actual: &Expression,
        expected: &Expression,
        assertion_failure: BinaryOp,
    ) -> DirectResult<()> {
        let actual_local = self.allocate_temp_local();
        let expected_local = self.allocate_temp_local();
        self.emit_same_value_operand(actual)?;
        self.push_local_set(actual_local);
        self.emit_same_value_operand(expected)?;
        self.push_local_set(expected_local);
        self.emit_same_value_result_from_locals(actual_local, expected_local, actual_local)?;
        if assertion_failure == BinaryOp::NotEqual {
            self.push_local_get(actual_local);
            self.state.emission.output.instructions.push(0x45);
            self.push_local_set(actual_local);
        }
        for argument in arguments.iter().skip(2) {
            match argument {
                CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                    self.emit_numeric_expression(expression)?;
                    self.state.emission.output.instructions.push(0x1a);
                }
            }
        }
        self.push_local_get(actual_local);
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
        self.push_i32_const(JS_UNDEFINED_TAG);
        Ok(())
    }

    fn same_value_assertion_member_chain_depth(expression: &Expression) -> usize {
        match expression {
            Expression::Member { object, .. } => {
                1 + Self::same_value_assertion_member_chain_depth(object)
            }
            Expression::Unary {
                op: UnaryOp::TypeOf,
                expression,
            } => Self::same_value_assertion_member_chain_depth(expression),
            _ => 0,
        }
    }

    fn same_value_assertion_contains_symbol_to_primitive_reference(
        expression: &Expression,
    ) -> bool {
        match expression {
            Expression::Member { object, property }
                if matches!(object.as_ref(), Expression::Identifier(name) if name == "Symbol")
                    && matches!(property.as_ref(), Expression::String(name) if name == "toPrimitive") =>
            {
                true
            }
            Expression::Member { object, property } => {
                Self::same_value_assertion_contains_symbol_to_primitive_reference(object)
                    || Self::same_value_assertion_contains_symbol_to_primitive_reference(property)
            }
            Expression::SuperMember { property } => {
                Self::same_value_assertion_contains_symbol_to_primitive_reference(property)
            }
            Expression::Unary { expression, .. }
            | Expression::Await(expression)
            | Expression::EnumerateKeys(expression)
            | Expression::GetIterator(expression)
            | Expression::IteratorClose(expression)
            | Expression::Assign {
                value: expression, ..
            } => Self::same_value_assertion_contains_symbol_to_primitive_reference(expression),
            Expression::AssignMember {
                object,
                property,
                value,
            } => {
                Self::same_value_assertion_contains_symbol_to_primitive_reference(object)
                    || Self::same_value_assertion_contains_symbol_to_primitive_reference(property)
                    || Self::same_value_assertion_contains_symbol_to_primitive_reference(value)
            }
            Expression::AssignSuperMember { property, value } => {
                Self::same_value_assertion_contains_symbol_to_primitive_reference(property)
                    || Self::same_value_assertion_contains_symbol_to_primitive_reference(value)
            }
            Expression::Binary { left, right, .. } => {
                Self::same_value_assertion_contains_symbol_to_primitive_reference(left)
                    || Self::same_value_assertion_contains_symbol_to_primitive_reference(right)
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                Self::same_value_assertion_contains_symbol_to_primitive_reference(condition)
                    || Self::same_value_assertion_contains_symbol_to_primitive_reference(
                        then_expression,
                    )
                    || Self::same_value_assertion_contains_symbol_to_primitive_reference(
                        else_expression,
                    )
            }
            Expression::Sequence(expressions) => expressions
                .iter()
                .any(Self::same_value_assertion_contains_symbol_to_primitive_reference),
            Expression::Call { callee, arguments }
            | Expression::SuperCall { callee, arguments }
            | Expression::New { callee, arguments } => {
                Self::same_value_assertion_contains_symbol_to_primitive_reference(callee)
                    || arguments.iter().any(|argument| match argument {
                        CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                            Self::same_value_assertion_contains_symbol_to_primitive_reference(
                                expression,
                            )
                        }
                    })
            }
            Expression::Array(elements) => elements.iter().any(|element| match element {
                ArrayElement::Expression(expression) | ArrayElement::Spread(expression) => {
                    Self::same_value_assertion_contains_symbol_to_primitive_reference(expression)
                }
            }),
            Expression::Object(entries) => entries.iter().any(|entry| match entry {
                ObjectEntry::Data { key, value }
                | ObjectEntry::Getter { key, getter: value }
                | ObjectEntry::Setter { key, setter: value } => {
                    Self::same_value_assertion_contains_symbol_to_primitive_reference(key)
                        || Self::same_value_assertion_contains_symbol_to_primitive_reference(value)
                }
                ObjectEntry::Spread(expression) => {
                    Self::same_value_assertion_contains_symbol_to_primitive_reference(expression)
                }
            }),
            Expression::Identifier(_)
            | Expression::Number(_)
            | Expression::BigInt(_)
            | Expression::String(_)
            | Expression::Bool(_)
            | Expression::Null
            | Expression::Undefined
            | Expression::This
            | Expression::NewTarget
            | Expression::Sent
            | Expression::Update { .. } => false,
        }
    }

    fn same_value_assertion_is_typeof_string_operand_pair(
        actual: &Expression,
        expected: &Expression,
    ) -> bool {
        matches!(
            (actual, expected),
            (
                Expression::Unary {
                    op: UnaryOp::TypeOf,
                    ..
                },
                Expression::String(_)
            ) | (
                Expression::String(_),
                Expression::Unary {
                    op: UnaryOp::TypeOf,
                    ..
                }
            )
        )
    }

    fn same_value_assertion_should_skip_broad_static_result(
        &self,
        actual: &Expression,
        expected: &Expression,
        direct_static_result: Option<bool>,
    ) -> bool {
        if direct_static_result == Some(false) {
            return true;
        }
        if Self::same_value_assertion_contains_symbol_to_primitive_reference(actual)
            || Self::same_value_assertion_contains_symbol_to_primitive_reference(expected)
        {
            return true;
        }
        (self.same_value_assertion_is_primitive_literal_operand(actual)
            && Self::same_value_assertion_member_chain_depth(expected) > 0)
            || (self.same_value_assertion_is_primitive_literal_operand(expected)
                && Self::same_value_assertion_member_chain_depth(actual) > 0)
    }

    fn same_value_expression_contains_dynamic_descriptor_member(
        &self,
        expression: &Expression,
    ) -> bool {
        match expression {
            Expression::Member { object, property } => {
                let is_dynamic_descriptor_member = matches!(
                    (object.as_ref(), property.as_ref()),
                    (Expression::Identifier(name), Expression::String(property_name))
                        if matches!(
                            property_name.as_str(),
                            "value" | "configurable" | "enumerable" | "writable" | "get" | "set"
                        ) && self.local_binding_is_dynamic_property_descriptor_result(name)
                );
                is_dynamic_descriptor_member
                    || self.same_value_expression_contains_dynamic_descriptor_member(object)
                    || self.same_value_expression_contains_dynamic_descriptor_member(property)
            }
            Expression::Unary { expression, .. }
            | Expression::Await(expression)
            | Expression::EnumerateKeys(expression)
            | Expression::GetIterator(expression)
            | Expression::IteratorClose(expression)
            | Expression::Assign {
                value: expression, ..
            } => self.same_value_expression_contains_dynamic_descriptor_member(expression),
            Expression::AssignMember {
                object,
                property,
                value,
            } => {
                self.same_value_expression_contains_dynamic_descriptor_member(object)
                    || self.same_value_expression_contains_dynamic_descriptor_member(property)
                    || self.same_value_expression_contains_dynamic_descriptor_member(value)
            }
            Expression::AssignSuperMember { property, value } => {
                self.same_value_expression_contains_dynamic_descriptor_member(property)
                    || self.same_value_expression_contains_dynamic_descriptor_member(value)
            }
            Expression::Binary { left, right, .. } => {
                self.same_value_expression_contains_dynamic_descriptor_member(left)
                    || self.same_value_expression_contains_dynamic_descriptor_member(right)
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                self.same_value_expression_contains_dynamic_descriptor_member(condition)
                    || self
                        .same_value_expression_contains_dynamic_descriptor_member(then_expression)
                    || self
                        .same_value_expression_contains_dynamic_descriptor_member(else_expression)
            }
            Expression::Sequence(expressions) => expressions.iter().any(|expression| {
                self.same_value_expression_contains_dynamic_descriptor_member(expression)
            }),
            Expression::Call { callee, arguments }
            | Expression::SuperCall { callee, arguments }
            | Expression::New { callee, arguments } => {
                self.same_value_expression_contains_dynamic_descriptor_member(callee)
                    || arguments.iter().any(|argument| {
                        self.same_value_expression_contains_dynamic_descriptor_member(
                            argument.expression(),
                        )
                    })
            }
            Expression::Array(elements) => elements.iter().any(|element| match element {
                ArrayElement::Expression(expression) | ArrayElement::Spread(expression) => {
                    self.same_value_expression_contains_dynamic_descriptor_member(expression)
                }
            }),
            Expression::Object(entries) => entries.iter().any(|entry| match entry {
                ObjectEntry::Data { key, value }
                | ObjectEntry::Getter { key, getter: value }
                | ObjectEntry::Setter { key, setter: value } => {
                    self.same_value_expression_contains_dynamic_descriptor_member(key)
                        || self.same_value_expression_contains_dynamic_descriptor_member(value)
                }
                ObjectEntry::Spread(expression) => {
                    self.same_value_expression_contains_dynamic_descriptor_member(expression)
                }
            }),
            Expression::SuperMember { property } => {
                self.same_value_expression_contains_dynamic_descriptor_member(property)
            }
            Expression::Identifier(_)
            | Expression::Number(_)
            | Expression::BigInt(_)
            | Expression::String(_)
            | Expression::Bool(_)
            | Expression::Null
            | Expression::Undefined
            | Expression::This
            | Expression::NewTarget
            | Expression::Sent
            | Expression::Update { .. } => false,
        }
    }

    fn same_value_expression_contains_deferred_namespace_eval_member(
        &self,
        expression: &Expression,
    ) -> bool {
        match expression {
            Expression::Member { object, property } => {
                self.deferred_module_namespace_materialized_member_access(object, property)
                    .is_some()
                    || self.same_value_expression_contains_deferred_namespace_eval_member(object)
                    || self.same_value_expression_contains_deferred_namespace_eval_member(property)
            }
            Expression::Unary { expression, .. }
            | Expression::Await(expression)
            | Expression::EnumerateKeys(expression)
            | Expression::GetIterator(expression)
            | Expression::IteratorClose(expression)
            | Expression::Assign {
                value: expression, ..
            } => self.same_value_expression_contains_deferred_namespace_eval_member(expression),
            Expression::AssignMember {
                object,
                property,
                value,
            } => {
                self.same_value_expression_contains_deferred_namespace_eval_member(object)
                    || self.same_value_expression_contains_deferred_namespace_eval_member(property)
                    || self.same_value_expression_contains_deferred_namespace_eval_member(value)
            }
            Expression::AssignSuperMember { property, value } => {
                self.same_value_expression_contains_deferred_namespace_eval_member(property)
                    || self.same_value_expression_contains_deferred_namespace_eval_member(value)
            }
            Expression::Binary { left, right, .. } => {
                self.same_value_expression_contains_deferred_namespace_eval_member(left)
                    || self.same_value_expression_contains_deferred_namespace_eval_member(right)
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                self.same_value_expression_contains_deferred_namespace_eval_member(condition)
                    || self.same_value_expression_contains_deferred_namespace_eval_member(
                        then_expression,
                    )
                    || self.same_value_expression_contains_deferred_namespace_eval_member(
                        else_expression,
                    )
            }
            Expression::Sequence(expressions) => expressions.iter().any(|expression| {
                self.same_value_expression_contains_deferred_namespace_eval_member(expression)
            }),
            Expression::Call { callee, arguments }
            | Expression::SuperCall { callee, arguments }
            | Expression::New { callee, arguments } => {
                self.same_value_expression_contains_deferred_namespace_eval_member(callee)
                    || arguments.iter().any(|argument| {
                        self.same_value_expression_contains_deferred_namespace_eval_member(
                            argument.expression(),
                        )
                    })
            }
            Expression::Array(elements) => elements.iter().any(|element| match element {
                ArrayElement::Expression(expression) | ArrayElement::Spread(expression) => {
                    self.same_value_expression_contains_deferred_namespace_eval_member(expression)
                }
            }),
            Expression::Object(entries) => entries.iter().any(|entry| match entry {
                ObjectEntry::Data { key, value }
                | ObjectEntry::Getter { key, getter: value }
                | ObjectEntry::Setter { key, setter: value } => {
                    self.same_value_expression_contains_deferred_namespace_eval_member(key)
                        || self.same_value_expression_contains_deferred_namespace_eval_member(value)
                }
                ObjectEntry::Spread(expression) => {
                    self.same_value_expression_contains_deferred_namespace_eval_member(expression)
                }
            }),
            Expression::SuperMember { property } => {
                self.same_value_expression_contains_deferred_namespace_eval_member(property)
            }
            Expression::Identifier(_)
            | Expression::Number(_)
            | Expression::BigInt(_)
            | Expression::String(_)
            | Expression::Bool(_)
            | Expression::Null
            | Expression::Undefined
            | Expression::This
            | Expression::NewTarget
            | Expression::Sent
            | Expression::Update { .. } => false,
        }
    }

    fn same_value_dynamic_descriptor_undefined_accessor_result(
        &self,
        actual: &Expression,
        expected: &Expression,
    ) -> Option<bool> {
        let (member, other) = match (actual, expected) {
            (Expression::Member { object, property }, other) => ((object, property), other),
            (other, Expression::Member { object, property }) => ((object, property), other),
            _ => return None,
        };
        let other_is_undefined = matches!(other, Expression::Undefined)
            || matches!(
                other,
                Expression::Identifier(name)
                    if name == "undefined" && self.is_unshadowed_builtin_identifier(name)
            );
        if !other_is_undefined {
            return None;
        }
        let (object, property) = member;
        matches!(
            (object.as_ref(), property.as_ref()),
            (Expression::Identifier(name), Expression::String(property_name))
                if matches!(property_name.as_str(), "get" | "set")
                    && self.local_binding_is_dynamic_property_descriptor_result(name)
        )
        .then_some(true)
    }

    fn same_value_assertion_needs_runtime_identifier_check(
        &self,
        actual: &Expression,
        expected: &Expression,
    ) -> bool {
        fn is_syntactic_primitive(
            compiler: &FunctionCompiler<'_>,
            expression: &Expression,
        ) -> bool {
            matches!(
                expression,
                Expression::Number(_)
                    | Expression::BigInt(_)
                    | Expression::String(_)
                    | Expression::Bool(_)
                    | Expression::Null
                    | Expression::Undefined
            ) || matches!(
                expression,
                Expression::Identifier(name)
                    if matches!(name.as_str(), "undefined" | "NaN")
                        && compiler.is_unshadowed_builtin_identifier(name)
            )
        }

        matches!(actual, Expression::Identifier(_)) && is_syntactic_primitive(self, expected)
            || matches!(expected, Expression::Identifier(_)) && is_syntactic_primitive(self, actual)
    }

    fn import_meta_identifier_module_index(name: &str) -> Option<&str> {
        let suffix = name.strip_prefix("__ayy_import_meta_").or_else(|| {
            name.rsplit_once("__ayy_import_meta_")
                .map(|(_, suffix)| suffix)
        })?;
        let digit_count = suffix
            .bytes()
            .take_while(|byte| byte.is_ascii_digit())
            .count();
        (digit_count > 0 && digit_count == suffix.len()).then_some(suffix)
    }

    fn same_value_import_meta_identity_probe_safe(expression: &Expression) -> bool {
        match expression {
            Expression::Identifier(name) => {
                Self::import_meta_identifier_module_index(name).is_some()
            }
            Expression::Call { callee, arguments } => {
                if matches!(callee.as_ref(), Expression::Identifier(name) if name == "__ayyImportMeta")
                    && matches!(
                        arguments.as_slice(),
                        [] | [CallArgument::Expression(Expression::Number(_))]
                            | [CallArgument::Spread(Expression::Number(_))]
                    )
                {
                    return true;
                }
                matches!(
                    callee.as_ref(),
                    Expression::Member { object, .. }
                        if matches!(
                            object.as_ref(),
                            Expression::Identifier(name) if name.starts_with("__ayy_module_dep_")
                        )
                ) && arguments.is_empty()
            }
            Expression::Member { object, .. } => {
                matches!(
                    object.as_ref(),
                    Expression::Identifier(name) if name.starts_with("__ayy_module_dep_")
                )
            }
            _ => false,
        }
    }

    fn module_index_from_namespace_identifier(name: &str) -> Option<&str> {
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
        (digit_count > 0).then_some(&suffix[..digit_count])
    }

    fn same_value_expression_has_module_namespace_base(expression: &Expression) -> bool {
        match expression {
            Expression::Identifier(name) => {
                Self::module_index_from_namespace_identifier(name).is_some()
            }
            Expression::Member { object, .. } | Expression::Call { callee: object, .. } => {
                Self::same_value_expression_has_module_namespace_base(object)
            }
            _ => false,
        }
    }

    fn same_value_import_meta_identity_key(expression: &Expression) -> Option<String> {
        match expression {
            Expression::Identifier(name) => Self::import_meta_identifier_module_index(name)
                .map(|module_index| format!("import-meta:{module_index}")),
            Expression::Call { callee, arguments } => {
                if matches!(callee.as_ref(), Expression::Identifier(name) if name == "__ayyImportMeta")
                {
                    let module_key = match arguments.as_slice() {
                        [] => "global".to_string(),
                        [CallArgument::Expression(Expression::Number(index))]
                        | [CallArgument::Spread(Expression::Number(index))]
                            if index.is_finite() =>
                        {
                            let integer = index.trunc();
                            if integer != *index || integer < 0.0 {
                                return None;
                            }
                            format!("{integer:.0}")
                        }
                        _ => return None,
                    };
                    return Some(format!("import-meta:{module_key}"));
                }
                if let Expression::Member { object, property } = callee.as_ref()
                    && arguments.is_empty()
                    && matches!(property.as_ref(), Expression::String(name) if name == "getMeta")
                    && let Expression::Identifier(namespace) = object.as_ref()
                    && let Some(module_index) =
                        Self::module_index_from_namespace_identifier(namespace)
                {
                    return Some(format!("import-meta:{module_index}"));
                }
                None
            }
            Expression::Member { object, property } if matches!(property.as_ref(), Expression::String(name) if name == "meta") =>
            {
                let Expression::Identifier(namespace) = object.as_ref() else {
                    return None;
                };
                Self::module_index_from_namespace_identifier(namespace)
                    .map(|module_index| format!("import-meta:{module_index}"))
            }
            _ => None,
        }
    }

    pub(in crate::backend::direct_wasm) fn emit_same_value_assertion(
        &mut self,
        name: &str,
        arguments: &[CallArgument],
    ) -> DirectResult<bool> {
        let [
            CallArgument::Expression(actual),
            CallArgument::Expression(expected),
            ..,
        ] = arguments
        else {
            return Ok(false);
        };
        let assertion_failure = match name {
            "__assertSameValue" => BinaryOp::NotEqual,
            "__assertNotSameValue" => BinaryOp::Equal,
            _ => return Ok(false),
        };
        let trace_assertions = std::env::var_os("AYY_TRACE_ASSERTIONS").is_some();
        if trace_assertions {
            eprintln!(
                "same_value_assertion:start name={name} actual={actual:?} expected={expected:?} fn={:?}",
                self.current_function_name()
            );
        }
        if Self::expression_contains_await_for_user_call_runtime(actual)
            || Self::expression_contains_await_for_user_call_runtime(expected)
            || self.same_value_expression_contains_deferred_namespace_eval_member(actual)
            || self.same_value_expression_contains_deferred_namespace_eval_member(expected)
        {
            self.emit_same_value_assertion_runtime_compare(
                arguments,
                actual,
                expected,
                assertion_failure,
            )?;
            return Ok(true);
        }
        let fast_extra_arguments_side_effect_free =
            arguments.iter().skip(2).all(|argument| match argument {
                CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                    self.assertion_static_message_argument_effect_free(expression)
                }
            });
        let operands_contain_dynamic_descriptor_member = self
            .same_value_expression_contains_dynamic_descriptor_member(actual)
            || self.same_value_expression_contains_dynamic_descriptor_member(expected);
        if fast_extra_arguments_side_effect_free
            && operands_contain_dynamic_descriptor_member
            && let Some(static_result) =
                self.same_value_dynamic_descriptor_undefined_accessor_result(actual, expected)
        {
            let assertion_passes = match assertion_failure {
                BinaryOp::NotEqual => static_result,
                BinaryOp::Equal => !static_result,
                _ => false,
            };
            if assertion_passes {
                self.push_i32_const(JS_UNDEFINED_TAG);
                return Ok(true);
            }
        }
        if let Some(true) = (fast_extra_arguments_side_effect_free
            && !operands_contain_dynamic_descriptor_member
            && !self.assertion_requires_runtime_same_value_fallback())
        .then(|| {
            self.emit_same_value_module_namespace_define_property_assertion(
                actual,
                expected,
                assertion_failure,
            )
        })
        .transpose()?
        {
            if trace_assertions {
                eprintln!(
                    "same_value_assertion:module_namespace_define_property_success name={name} actual={actual:?} expected={expected:?} fn={:?}",
                    self.current_function_name()
                );
            }
            return Ok(true);
        }
        if fast_extra_arguments_side_effect_free
            && !operands_contain_dynamic_descriptor_member
            && let Some(static_result) =
                self.same_value_assertion_bytes_import_primitive_result(actual, expected)
        {
            let assertion_passes = match assertion_failure {
                BinaryOp::NotEqual => static_result,
                BinaryOp::Equal => !static_result,
                _ => false,
            };
            if assertion_passes {
                if trace_assertions {
                    eprintln!(
                        "same_value_assertion:bytes_import_static_success name={name} actual={actual:?} expected={expected:?} fn={:?}",
                        self.current_function_name()
                    );
                }
                self.push_i32_const(JS_UNDEFINED_TAG);
                return Ok(true);
            }
        }
        if fast_extra_arguments_side_effect_free
            && !operands_contain_dynamic_descriptor_member
            && let Some(static_result) =
                self.same_value_assertion_namespace_descriptor_result(actual, expected)
        {
            let assertion_passes = match assertion_failure {
                BinaryOp::NotEqual => static_result,
                BinaryOp::Equal => !static_result,
                _ => false,
            };
            if assertion_passes {
                if trace_assertions {
                    eprintln!(
                        "same_value_assertion:namespace_descriptor_static_success name={name} actual={actual:?} expected={expected:?} fn={:?}",
                        self.current_function_name()
                    );
                }
                self.push_i32_const(JS_UNDEFINED_TAG);
                return Ok(true);
            }
        }
        if fast_extra_arguments_side_effect_free
            && !operands_contain_dynamic_descriptor_member
            && let Some(static_result) =
                self.same_value_assertion_fast_object_names_length_result(actual, expected)
        {
            let assertion_passes = match assertion_failure {
                BinaryOp::NotEqual => static_result,
                BinaryOp::Equal => !static_result,
                _ => false,
            };
            if assertion_passes {
                self.push_i32_const(JS_UNDEFINED_TAG);
                return Ok(true);
            }
        }
        if fast_extra_arguments_side_effect_free
            && !operands_contain_dynamic_descriptor_member
            && !self.assertion_requires_runtime_same_value_fallback()
            && let Some(static_result) =
                self.same_value_assertion_fast_reference_result(actual, expected)
        {
            let assertion_passes = match assertion_failure {
                BinaryOp::NotEqual => static_result,
                BinaryOp::Equal => !static_result,
                _ => false,
            };
            if assertion_passes {
                if trace_assertions {
                    eprintln!(
                        "same_value_assertion:fast_reference_success name={name} actual={actual:?} expected={expected:?} fn={:?}",
                        self.current_function_name()
                    );
                }
                self.push_i32_const(JS_UNDEFINED_TAG);
                return Ok(true);
            }
        }
        if fast_extra_arguments_side_effect_free
            && !operands_contain_dynamic_descriptor_member
            && !self.assertion_requires_runtime_same_value_fallback()
            && let Some(static_result) =
                self.same_value_assertion_fast_primitive_result(actual, expected)
        {
            let assertion_passes = match assertion_failure {
                BinaryOp::NotEqual => static_result,
                BinaryOp::Equal => !static_result,
                _ => false,
            };
            if assertion_passes {
                if trace_assertions {
                    eprintln!(
                        "same_value_assertion:fast_static_success name={name} actual={actual:?} expected={expected:?} fn={:?}",
                        self.current_function_name()
                    );
                }
                self.push_i32_const(JS_UNDEFINED_TAG);
                return Ok(true);
            }
        }
        if fast_extra_arguments_side_effect_free
            && !operands_contain_dynamic_descriptor_member
            && let Some(static_result) =
                self.same_value_assertion_tracked_array_snapshot_result(actual, expected)
        {
            let assertion_passes = match assertion_failure {
                BinaryOp::NotEqual => static_result,
                BinaryOp::Equal => !static_result,
                _ => false,
            };
            if assertion_passes {
                if trace_assertions {
                    eprintln!(
                        "same_value_assertion:tracked_array_snapshot_success name={name} actual={actual:?} expected={expected:?} fn={:?}",
                        self.current_function_name()
                    );
                }
                self.push_i32_const(JS_UNDEFINED_TAG);
                return Ok(true);
            }
        }
        let actual_local = self.allocate_temp_local();
        let expected_local = self.allocate_temp_local();
        let operands_side_effect_free = inline_summary_side_effect_free_expression(actual)
            && inline_summary_side_effect_free_expression(expected);
        let operands_static_evaluation_safe = self
            .same_value_operand_static_evaluation_safe(actual)
            && self.same_value_operand_static_evaluation_safe(expected);
        let direct_static_result = if operands_contain_dynamic_descriptor_member {
            None
        } else {
            self.same_value_assertion_direct_static_result(actual, expected)
        };
        let handled_as_typeof =
            Self::same_value_assertion_is_typeof_string_operand_pair(actual, expected);
        let skip_broad_static_result = operands_contain_dynamic_descriptor_member
            || self.same_value_assertion_should_skip_broad_static_result(
                actual,
                expected,
                direct_static_result,
            );
        let reference_identity_can_affect_result = !self
            .same_value_assertion_is_primitive_literal_operand(actual)
            && !self.same_value_assertion_is_primitive_literal_operand(expected);
        let import_meta_identity_probe_safe =
            Self::same_value_import_meta_identity_probe_safe(actual)
                && Self::same_value_import_meta_identity_probe_safe(expected);
        let (actual_reference_identity, expected_reference_identity) =
            if !operands_contain_dynamic_descriptor_member
                && reference_identity_can_affect_result
                && (operands_side_effect_free || import_meta_identity_probe_safe)
            {
                (
                    Self::same_value_import_meta_identity_key(actual)
                        .or_else(|| self.resolve_static_reference_identity_key(actual)),
                    Self::same_value_import_meta_identity_key(expected)
                        .or_else(|| self.resolve_static_reference_identity_key(expected)),
                )
            } else {
                (None, None)
            };
        if trace_assertions {
            eprintln!(
                "same_value_assertion:identity actual={actual_reference_identity:?} expected={expected_reference_identity:?} fn={:?}",
                self.current_function_name()
            );
        }
        let has_static_reference_identity =
            actual_reference_identity.is_some() && expected_reference_identity.is_some();
        let needs_runtime_identifier_check =
            self.same_value_assertion_needs_runtime_identifier_check(actual, expected);
        let identifier_operands_are_unshadowed_primitives =
            [actual, expected].iter().all(|expression| {
                !matches!(expression, Expression::Identifier(_))
                    || matches!(
                        expression,
                        Expression::Identifier(name)
                            if matches!(name.as_str(), "undefined" | "NaN" | "Infinity")
                                && self.is_unshadowed_builtin_identifier(name)
                    )
            });
        let extra_arguments_side_effect_free = fast_extra_arguments_side_effect_free;
        let materialized_skip_static_result = if skip_broad_static_result
            && !operands_contain_dynamic_descriptor_member
            && operands_static_evaluation_safe
            && extra_arguments_side_effect_free
            && !needs_runtime_identifier_check
            && !self.assertion_requires_runtime_same_value_fallback()
            && !handled_as_typeof
            && !Self::same_value_expression_has_module_namespace_base(actual)
            && !Self::same_value_expression_has_module_namespace_base(expected)
        {
            let actual_materialized = self.materialize_static_expression(actual);
            let expected_materialized = self.materialize_static_expression(expected);
            let actual_value = self
                .resolve_static_primitive_expression_with_context(
                    &actual_materialized,
                    self.current_function_name(),
                )
                .unwrap_or(actual_materialized);
            let expected_value = self
                .resolve_static_primitive_expression_with_context(
                    &expected_materialized,
                    self.current_function_name(),
                )
                .unwrap_or(expected_materialized);
            self.same_value_assertion_primitive_result(&actual_value, &expected_value)
        } else {
            None
        };
        if extra_arguments_side_effect_free
            && let (Some(actual_key), Some(expected_key)) = (
                actual_reference_identity.as_ref(),
                expected_reference_identity.as_ref(),
            )
        {
            let static_result = actual_key == expected_key;
            let assertion_passes = match assertion_failure {
                BinaryOp::NotEqual => static_result,
                BinaryOp::Equal => !static_result,
                _ => false,
            };
            if assertion_passes {
                self.push_i32_const(JS_UNDEFINED_TAG);
                return Ok(true);
            }
        }
        let operands_contain_member_access =
            Self::same_value_operand_contains_member_access(actual)
                || Self::same_value_operand_contains_member_access(expected);
        let operands_use_runtime_array_state = !handled_as_typeof
            && (self.expression_uses_runtime_array_state(actual)
                || self.expression_uses_runtime_array_state(expected));
        if handled_as_typeof
            && extra_arguments_side_effect_free
            && !needs_runtime_identifier_check
            && !self.assertion_requires_runtime_same_value_fallback()
            && let Some(static_result) = direct_static_result
        {
            let assertion_passes = match assertion_failure {
                BinaryOp::NotEqual => static_result,
                BinaryOp::Equal => !static_result,
                _ => false,
            };
            if assertion_passes {
                if trace_assertions {
                    eprintln!(
                        "same_value_assertion:typeof_static_success name={name} actual={actual:?} expected={expected:?} fn={:?}",
                        self.current_function_name()
                    );
                }
                self.push_i32_const(JS_UNDEFINED_TAG);
                return Ok(true);
            }
        }
        if operands_static_evaluation_safe
            && extra_arguments_side_effect_free
            && !needs_runtime_identifier_check
            && !self.assertion_requires_runtime_same_value_fallback()
            && !handled_as_typeof
            && let Some(static_result) = direct_static_result.or_else(|| {
                if skip_broad_static_result {
                    None
                } else {
                    self.resolve_static_same_value_result_with_context(
                        actual,
                        expected,
                        self.current_function_name(),
                    )
                }
            })
        {
            let assertion_passes = match assertion_failure {
                BinaryOp::NotEqual => static_result,
                BinaryOp::Equal => !static_result,
                _ => false,
            };
            if assertion_passes {
                if trace_assertions {
                    eprintln!(
                        "same_value_assertion:static_success name={name} actual={actual:?} expected={expected:?} fn={:?}",
                        self.current_function_name()
                    );
                }
                self.push_i32_const(JS_UNDEFINED_TAG);
                return Ok(true);
            }
        }
        let can_use_effectful_static_boolean_call_result = extra_arguments_side_effect_free
            && !needs_runtime_identifier_check
            && !operands_side_effect_free
            && (matches!(actual, Expression::Call { .. })
                && matches!(expected, Expression::Bool(_))
                || matches!(expected, Expression::Call { .. })
                    && matches!(actual, Expression::Bool(_)));
        if can_use_effectful_static_boolean_call_result
            && let Some(static_result) = self.resolve_static_same_value_result_with_context(
                actual,
                expected,
                self.current_function_name(),
            )
        {
            let assertion_passes = match assertion_failure {
                BinaryOp::NotEqual => static_result,
                BinaryOp::Equal => !static_result,
                _ => false,
            };
            if assertion_passes {
                if !inline_summary_side_effect_free_expression(actual) {
                    self.emit_same_value_operand(actual)?;
                    self.state.emission.output.instructions.push(0x1a);
                }
                if !inline_summary_side_effect_free_expression(expected) {
                    self.emit_same_value_operand(expected)?;
                    self.state.emission.output.instructions.push(0x1a);
                }
                self.push_i32_const(JS_UNDEFINED_TAG);
                return Ok(true);
            }
        }
        if operands_side_effect_free
            && !operands_contain_member_access
            && !needs_runtime_identifier_check
            && !handled_as_typeof
            && let (Some(actual_text), Some(expected_text)) = (
                self.resolve_static_string_value(actual),
                self.resolve_static_string_value(expected),
            )
        {
            self.push_i32_const((actual_text == expected_text) as i32);
            self.push_local_set(actual_local);
            if assertion_failure == BinaryOp::NotEqual {
                self.push_local_get(actual_local);
                self.state.emission.output.instructions.push(0x45);
                self.push_local_set(actual_local);
            }
        } else if operands_static_evaluation_safe
            && extra_arguments_side_effect_free
            && !needs_runtime_identifier_check
            && !handled_as_typeof
            && !operands_use_runtime_array_state
            && (matches!(actual, Expression::String(_))
                || matches!(expected, Expression::String(_)))
            && let (Some(actual_text), Some(expected_text)) = (
                self.resolve_static_string_value_with_context(actual, self.current_function_name()),
                self.resolve_static_string_value_with_context(
                    expected,
                    self.current_function_name(),
                ),
            )
        {
            self.push_i32_const((actual_text == expected_text) as i32);
            self.push_local_set(actual_local);
            if assertion_failure == BinaryOp::NotEqual {
                self.push_local_get(actual_local);
                self.state.emission.output.instructions.push(0x45);
                self.push_local_set(actual_local);
            }
        } else if skip_broad_static_result
            && !operands_contain_dynamic_descriptor_member
            && operands_static_evaluation_safe
            && extra_arguments_side_effect_free
            && !needs_runtime_identifier_check
            && !self.assertion_requires_runtime_same_value_fallback()
            && !handled_as_typeof
            && let Some(static_result) = self.resolve_static_same_value_result_with_context(
                actual,
                expected,
                self.current_function_name(),
            )
        {
            self.push_i32_const(static_result as i32);
            self.push_local_set(actual_local);
            if assertion_failure == BinaryOp::NotEqual {
                self.push_local_get(actual_local);
                self.state.emission.output.instructions.push(0x45);
                self.push_local_set(actual_local);
            }
        } else if let Some(static_result) = materialized_skip_static_result {
            self.push_i32_const(static_result as i32);
            self.push_local_set(actual_local);
            if assertion_failure == BinaryOp::NotEqual {
                self.push_local_get(actual_local);
                self.state.emission.output.instructions.push(0x45);
                self.push_local_set(actual_local);
            }
        } else {
            if handled_as_typeof {
                if trace_assertions {
                    eprintln!(
                        "same_value_assertion:typeof_compare start actual={actual:?} expected={expected:?} fn={:?}",
                        self.current_function_name()
                    );
                }
                let handled_by_typeof =
                    self.emit_typeof_string_comparison(actual, expected, assertion_failure)?;
                if trace_assertions {
                    eprintln!(
                        "same_value_assertion:typeof_compare primary={handled_by_typeof} actual={actual:?} expected={expected:?} fn={:?}",
                        self.current_function_name()
                    );
                }
                let handled_by_runtime_typeof = if handled_by_typeof {
                    false
                } else {
                    self.emit_runtime_typeof_tag_string_comparison(
                        actual,
                        expected,
                        assertion_failure,
                    )?
                };
                if trace_assertions {
                    eprintln!(
                        "same_value_assertion:typeof_compare runtime={handled_by_runtime_typeof} actual={actual:?} expected={expected:?} fn={:?}",
                        self.current_function_name()
                    );
                }
                if handled_by_typeof || handled_by_runtime_typeof {
                    self.push_local_set(actual_local);
                } else {
                    self.push_i32_const(0);
                    self.push_local_set(actual_local);
                }
            } else if !needs_runtime_identifier_check
                && (matches!(actual, Expression::String(_))
                    || matches!(expected, Expression::String(_)))
                && (matches!(actual, Expression::Call { .. })
                    || matches!(expected, Expression::Call { .. }))
                && let Some(static_result) = self.resolve_static_same_value_result_with_context(
                    actual,
                    expected,
                    self.current_function_name(),
                )
            {
                if !inline_summary_side_effect_free_expression(actual) {
                    self.emit_same_value_operand(actual)?;
                    self.state.emission.output.instructions.push(0x1a);
                }
                if !inline_summary_side_effect_free_expression(expected) {
                    self.emit_same_value_operand(expected)?;
                    self.state.emission.output.instructions.push(0x1a);
                }
                let assertion_fails = match assertion_failure {
                    BinaryOp::NotEqual => !static_result,
                    BinaryOp::Equal => static_result,
                    _ => false,
                };
                self.push_i32_const(assertion_fails as i32);
                self.push_local_set(actual_local);
            } else if !needs_runtime_identifier_check
                && (matches!(actual, Expression::String(_))
                    || matches!(expected, Expression::String(_)))
                && !operands_use_runtime_array_state
            {
                self.emit_numeric_expression(&Expression::Binary {
                    op: BinaryOp::Equal,
                    left: Box::new(actual.clone()),
                    right: Box::new(expected.clone()),
                })?;
                self.push_local_set(actual_local);
                if assertion_failure == BinaryOp::NotEqual {
                    self.push_local_get(actual_local);
                    self.state.emission.output.instructions.push(0x45);
                    self.push_local_set(actual_local);
                }
            } else if !needs_runtime_identifier_check
                && self.emit_runtime_static_string_equality_comparison(
                    actual,
                    expected,
                    BinaryOp::Equal,
                )?
            {
                self.push_local_set(actual_local);
                if assertion_failure == BinaryOp::NotEqual {
                    self.push_local_get(actual_local);
                    self.state.emission.output.instructions.push(0x45);
                    self.push_local_set(actual_local);
                }
            } else {
                let can_use_static_same_value_result = !skip_broad_static_result
                    && !needs_runtime_identifier_check
                    && (!self.assertion_requires_runtime_same_value_fallback()
                        || has_static_reference_identity)
                    && operands_static_evaluation_safe
                    && (matches!(actual, Expression::This)
                        || matches!(expected, Expression::This)
                        || self.resolve_array_binding_from_expression(actual).is_some()
                        || self
                            .resolve_array_binding_from_expression(expected)
                            .is_some()
                        || self
                            .resolve_object_binding_from_expression(actual)
                            .is_some()
                        || self
                            .resolve_object_binding_from_expression(expected)
                            .is_some()
                        || self.resolve_user_function_from_expression(actual).is_some()
                        || self
                            .resolve_user_function_from_expression(expected)
                            .is_some()
                        || has_static_reference_identity
                        || identifier_operands_are_unshadowed_primitives
                        || (!matches!(actual, Expression::Identifier(_))
                            && !matches!(expected, Expression::Identifier(_))));
                let static_same_value_result = if can_use_static_same_value_result {
                    if trace_assertions {
                        eprintln!(
                            "same_value_assertion:resolve_static start actual={actual:?} expected={expected:?} fn={:?}",
                            self.current_function_name()
                        );
                    }
                    let result = self.resolve_static_same_value_result_with_context(
                        actual,
                        expected,
                        self.current_function_name(),
                    );
                    if trace_assertions {
                        eprintln!(
                            "same_value_assertion:resolve_static result={result:?} actual={actual:?} expected={expected:?} fn={:?}",
                            self.current_function_name()
                        );
                    }
                    result
                } else {
                    if trace_assertions {
                        eprintln!(
                            "same_value_assertion:resolve_static skipped actual={actual:?} expected={expected:?} side_effect_free={operands_side_effect_free} fn={:?}",
                            self.current_function_name()
                        );
                    }
                    None
                };
                if let Some(result) = static_same_value_result {
                    self.push_i32_const(result as i32);
                    self.push_local_set(actual_local);
                    if assertion_failure == BinaryOp::NotEqual {
                        self.push_local_get(actual_local);
                        self.state.emission.output.instructions.push(0x45);
                        self.push_local_set(actual_local);
                    }
                } else {
                    if self.same_value_assertion_has_tracked_array_element_base(actual)
                        || self.same_value_assertion_has_tracked_array_element_base(expected)
                    {
                        self.emit_same_value_assertion_runtime_compare(
                            arguments,
                            actual,
                            expected,
                            assertion_failure,
                        )?;
                        return Ok(true);
                    }
                    if trace_assertions {
                        eprintln!(
                            "same_value_assertion:emit_operands start actual={actual:?} expected={expected:?} fn={:?}",
                            self.current_function_name()
                        );
                    }
                    self.emit_same_value_operand(actual)?;
                    self.push_local_set(actual_local);
                    self.emit_same_value_operand(expected)?;
                    self.push_local_set(expected_local);
                    self.emit_same_value_result_from_locals(
                        actual_local,
                        expected_local,
                        actual_local,
                    )?;
                    if assertion_failure == BinaryOp::NotEqual {
                        self.push_local_get(actual_local);
                        self.state.emission.output.instructions.push(0x45);
                        self.push_local_set(actual_local);
                    }
                    if trace_assertions {
                        eprintln!(
                            "same_value_assertion:emit_operands done actual={actual:?} expected={expected:?} fn={:?}",
                            self.current_function_name()
                        );
                    }
                }
            }
        }
        for argument in arguments.iter().skip(2) {
            match argument {
                CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                    self.emit_numeric_expression(expression)?;
                    self.state.emission.output.instructions.push(0x1a);
                }
            }
        }
        self.push_local_get(actual_local);
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
        self.push_i32_const(JS_UNDEFINED_TAG);
        Ok(true)
    }
}
