use super::*;

const RUNTIME_STRING_ADDITION_CANDIDATE_LIMIT: usize = 128;

fn binary_expression_references_internal_iterator_step(expression: &Expression) -> bool {
    match expression {
        Expression::Identifier(name) => {
            name.starts_with("__ayy_array_step_")
                || name.starts_with("__ayy_array_iter_value_")
                || name.starts_with("__ayy_array_iter_done_")
                || name.starts_with("__ayy_for_of_step_")
                || name.starts_with("__ayy_for_of_iter_value_")
                || name.starts_with("__ayy_for_of_iter_done_")
                || name.starts_with("__ayy_binding_value_")
        }
        Expression::Array(elements) => elements.iter().any(|element| match element {
            ArrayElement::Expression(value) | ArrayElement::Spread(value) => {
                binary_expression_references_internal_iterator_step(value)
            }
        }),
        Expression::Object(entries) => entries.iter().any(|entry| match entry {
            ObjectEntry::Data { key, value } => {
                binary_expression_references_internal_iterator_step(key)
                    || binary_expression_references_internal_iterator_step(value)
            }
            ObjectEntry::Getter { key, getter } => {
                binary_expression_references_internal_iterator_step(key)
                    || binary_expression_references_internal_iterator_step(getter)
            }
            ObjectEntry::Setter { key, setter } => {
                binary_expression_references_internal_iterator_step(key)
                    || binary_expression_references_internal_iterator_step(setter)
            }
            ObjectEntry::Spread(value) => {
                binary_expression_references_internal_iterator_step(value)
            }
        }),
        Expression::Binary { left, right, .. } => {
            binary_expression_references_internal_iterator_step(left)
                || binary_expression_references_internal_iterator_step(right)
        }
        Expression::Conditional {
            condition,
            then_expression,
            else_expression,
        } => {
            binary_expression_references_internal_iterator_step(condition)
                || binary_expression_references_internal_iterator_step(then_expression)
                || binary_expression_references_internal_iterator_step(else_expression)
        }
        Expression::Member { object, property } => {
            binary_expression_references_internal_iterator_step(object)
                || binary_expression_references_internal_iterator_step(property)
        }
        Expression::Unary { expression, .. }
        | Expression::Await(expression)
        | Expression::EnumerateKeys(expression)
        | Expression::GetIterator(expression)
        | Expression::IteratorClose(expression) => {
            binary_expression_references_internal_iterator_step(expression)
        }
        Expression::Assign { value, .. } => {
            binary_expression_references_internal_iterator_step(value)
        }
        Expression::AssignMember {
            object,
            property,
            value,
        } => {
            binary_expression_references_internal_iterator_step(object)
                || binary_expression_references_internal_iterator_step(property)
                || binary_expression_references_internal_iterator_step(value)
        }
        Expression::AssignSuperMember { property, value } => {
            binary_expression_references_internal_iterator_step(property)
                || binary_expression_references_internal_iterator_step(value)
        }
        Expression::Call { callee, arguments }
        | Expression::New { callee, arguments }
        | Expression::SuperCall { callee, arguments } => {
            binary_expression_references_internal_iterator_step(callee)
                || arguments.iter().any(|argument| match argument {
                    CallArgument::Expression(value) | CallArgument::Spread(value) => {
                        binary_expression_references_internal_iterator_step(value)
                    }
                })
        }
        Expression::SuperMember { property } => {
            binary_expression_references_internal_iterator_step(property)
        }
        _ => false,
    }
}

impl<'a> FunctionCompiler<'a> {
    fn strict_equality_type_mismatch_result(
        left_kind: StaticValueKind,
        right_kind: StaticValueKind,
        op: BinaryOp,
    ) -> Option<bool> {
        if !matches!(op, BinaryOp::Equal | BinaryOp::NotEqual) {
            return None;
        }

        let js_type_class = |kind: StaticValueKind| match kind {
            StaticValueKind::Number => Some(0),
            StaticValueKind::Bool => Some(1),
            StaticValueKind::String => Some(2),
            StaticValueKind::BigInt => Some(3),
            StaticValueKind::Symbol => Some(4),
            StaticValueKind::Null => Some(5),
            StaticValueKind::Undefined => Some(6),
            StaticValueKind::Object | StaticValueKind::Function => Some(7),
            StaticValueKind::Unknown => None,
        };

        let left_type = js_type_class(left_kind)?;
        let right_type = js_type_class(right_kind)?;
        if left_type == right_type {
            return None;
        }

        Some(matches!(op, BinaryOp::NotEqual))
    }

    fn emit_static_strict_type_mismatch_comparison(
        &mut self,
        op: BinaryOp,
        left: &Expression,
        right: &Expression,
    ) -> DirectResult<bool> {
        if self.expression_has_dynamic_member_property_access(left)
            || self.expression_has_dynamic_member_property_access(right)
        {
            return Ok(false);
        }
        let Some(result) = self
            .infer_value_kind(left)
            .zip(self.infer_value_kind(right))
            .and_then(|(left_kind, right_kind)| {
                Self::strict_equality_type_mismatch_result(left_kind, right_kind, op)
            })
        else {
            return Ok(false);
        };

        if !inline_summary_side_effect_free_expression(left) {
            self.emit_numeric_expression(left)?;
            self.state.emission.output.instructions.push(0x1a);
        }
        if !inline_summary_side_effect_free_expression(right) {
            self.emit_numeric_expression(right)?;
            self.state.emission.output.instructions.push(0x1a);
        }
        self.push_i32_const(i32::from(result));
        Ok(true)
    }

    pub(in crate::backend::direct_wasm) fn binary_expression_calls_user_function(
        &self,
        expression: &Expression,
    ) -> bool {
        match expression {
            Expression::Call { callee, arguments }
            | Expression::SuperCall { callee, arguments }
            | Expression::New { callee, arguments } => {
                matches!(callee.as_ref(), Expression::Identifier(name) if self.contains_user_function(name))
                    || matches!(
                        self.resolve_function_binding_from_expression(callee),
                        Some(LocalFunctionBinding::User(_))
                    )
                    || self.binary_expression_calls_user_function(callee)
                    || arguments.iter().any(|argument| match argument {
                        CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                            self.binary_expression_calls_user_function(expression)
                        }
                    })
            }
            Expression::Unary { expression, .. }
            | Expression::Await(expression)
            | Expression::EnumerateKeys(expression)
            | Expression::GetIterator(expression)
            | Expression::IteratorClose(expression)
            | Expression::Assign {
                value: expression, ..
            } => self.binary_expression_calls_user_function(expression),
            Expression::AssignMember {
                object,
                property,
                value,
            } => {
                self.binary_expression_calls_user_function(object)
                    || self.binary_expression_calls_user_function(property)
                    || self.binary_expression_calls_user_function(value)
            }
            Expression::AssignSuperMember { property, value } => {
                self.binary_expression_calls_user_function(property)
                    || self.binary_expression_calls_user_function(value)
            }
            Expression::Binary { left, right, .. } => {
                self.binary_expression_calls_user_function(left)
                    || self.binary_expression_calls_user_function(right)
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                self.binary_expression_calls_user_function(condition)
                    || self.binary_expression_calls_user_function(then_expression)
                    || self.binary_expression_calls_user_function(else_expression)
            }
            Expression::Sequence(expressions) => expressions
                .iter()
                .any(|expression| self.binary_expression_calls_user_function(expression)),
            Expression::Array(elements) => elements.iter().any(|element| match element {
                ArrayElement::Expression(expression) | ArrayElement::Spread(expression) => {
                    self.binary_expression_calls_user_function(expression)
                }
            }),
            Expression::Object(entries) => entries.iter().any(|entry| match entry {
                ObjectEntry::Data { key, value } => {
                    self.binary_expression_calls_user_function(key)
                        || self.binary_expression_calls_user_function(value)
                }
                ObjectEntry::Getter { key, getter } => {
                    self.binary_expression_calls_user_function(key)
                        || self.binary_expression_calls_user_function(getter)
                }
                ObjectEntry::Setter { key, setter } => {
                    self.binary_expression_calls_user_function(key)
                        || self.binary_expression_calls_user_function(setter)
                }
                ObjectEntry::Spread(expression) => {
                    self.binary_expression_calls_user_function(expression)
                }
            }),
            Expression::Member { object, property } => {
                self.binary_expression_calls_user_function(object)
                    || self.binary_expression_calls_user_function(property)
            }
            Expression::SuperMember { property } => {
                self.binary_expression_calls_user_function(property)
            }
            Expression::Identifier(_)
            | Expression::Update { .. }
            | Expression::Number(_)
            | Expression::BigInt(_)
            | Expression::String(_)
            | Expression::Bool(_)
            | Expression::Null
            | Expression::Undefined
            | Expression::NewTarget
            | Expression::This
            | Expression::Sent => false,
        }
    }

    fn static_boxed_primitive_constructor_call_is_side_effect_free(
        &self,
        expression: &Expression,
    ) -> bool {
        if self
            .resolve_static_boxed_primitive_value(expression)
            .is_none()
        {
            return false;
        }
        let (callee, arguments) = match expression {
            Expression::Call { callee, arguments } | Expression::New { callee, arguments } => {
                (callee.as_ref(), arguments.as_slice())
            }
            Expression::Sequence(expressions) => {
                let Some((last, prefix)) = expressions.split_last() else {
                    return false;
                };
                return prefix
                    .iter()
                    .all(inline_summary_side_effect_free_expression)
                    && self.static_boxed_primitive_constructor_call_is_side_effect_free(last);
            }
            Expression::Identifier(_) => return true,
            _ => return false,
        };
        if !matches!(
            callee,
            Expression::Identifier(name)
                if matches!(name.as_str(), "Boolean" | "Number" | "String" | "Object")
                    && self.is_unshadowed_builtin_identifier(name)
        ) {
            return false;
        }
        arguments.iter().all(|argument| match argument {
            CallArgument::Expression(value) => inline_summary_side_effect_free_expression(value),
            CallArgument::Spread(_) => false,
        })
    }

    fn numeric_static_outcome_operand_is_side_effect_free(&self, expression: &Expression) -> bool {
        if self.binary_expression_calls_user_function(expression) {
            return false;
        }
        inline_summary_side_effect_free_expression(expression)
            || self.static_boxed_primitive_constructor_call_is_side_effect_free(expression)
    }

    fn binary_expression_reads_runtime_nonlocal_binding(&self, expression: &Expression) -> bool {
        if self.current_function_name().is_none() {
            return false;
        }

        let mut referenced_names = HashSet::new();
        collect_referenced_binding_names_from_expression(expression, &mut referenced_names);
        referenced_names.iter().any(|name| {
            let source_name = scoped_binding_source_name(name).unwrap_or(name);
            self.resolve_current_local_binding(source_name).is_none()
                && (self.global_has_binding(source_name)
                    || self.global_has_implicit_binding(source_name)
                    || self
                        .resolve_user_function_capture_hidden_name(source_name)
                        .is_some())
        })
    }

    fn push_unique_runtime_string_value_candidate(
        candidates: &mut Vec<(Expression, String)>,
        value: Expression,
        text: String,
    ) -> bool {
        if candidates.iter().any(|(candidate_value, candidate_text)| {
            static_expression_matches(candidate_value, &value) && candidate_text == &text
        }) {
            return false;
        }
        candidates.push((value, text));
        true
    }

    fn intern_runtime_string_candidate_text(&mut self, text: &str) {
        self.intern_string(text.as_bytes().to_vec());
    }

    fn active_loop_integer_literal(expression: &Expression) -> Option<i64> {
        let Expression::Number(value) = expression else {
            return None;
        };
        (value.is_finite() && value.fract() == 0.0).then_some(*value as i64)
    }

    fn active_loop_static_array_length_member(&self, expression: &Expression) -> Option<i64> {
        let Expression::Member { object, property } = expression else {
            return None;
        };
        if !matches!(property.as_ref(), Expression::String(name) if name == "length") {
            return None;
        }
        i64::try_from(
            self.resolve_array_binding_from_expression(object)?
                .values
                .len(),
        )
        .ok()
    }

    fn expression_is_active_loop_indexed_static_array_member(
        &self,
        expression: &Expression,
    ) -> bool {
        let Expression::Member { object, property } = expression else {
            return false;
        };
        self.resolve_array_binding_from_expression(object).is_some()
            && self.expression_depends_on_active_loop_assignment(property)
    }

    pub(in crate::backend::direct_wasm) fn expression_contains_assignment_or_update(
        expression: &Expression,
    ) -> bool {
        match expression {
            Expression::Assign { .. }
            | Expression::AssignMember { .. }
            | Expression::AssignSuperMember { .. }
            | Expression::Update { .. } => true,
            Expression::Unary { expression, .. }
            | Expression::Await(expression)
            | Expression::EnumerateKeys(expression)
            | Expression::GetIterator(expression)
            | Expression::IteratorClose(expression) => {
                Self::expression_contains_assignment_or_update(expression)
            }
            Expression::Binary { left, right, .. } => {
                Self::expression_contains_assignment_or_update(left)
                    || Self::expression_contains_assignment_or_update(right)
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                Self::expression_contains_assignment_or_update(condition)
                    || Self::expression_contains_assignment_or_update(then_expression)
                    || Self::expression_contains_assignment_or_update(else_expression)
            }
            Expression::Sequence(expressions) => expressions
                .iter()
                .any(Self::expression_contains_assignment_or_update),
            Expression::Array(elements) => elements.iter().any(|element| match element {
                crate::ir::hir::ArrayElement::Expression(expression)
                | crate::ir::hir::ArrayElement::Spread(expression) => {
                    Self::expression_contains_assignment_or_update(expression)
                }
            }),
            Expression::Object(entries) => entries.iter().any(|entry| match entry {
                crate::ir::hir::ObjectEntry::Data { key, value } => {
                    Self::expression_contains_assignment_or_update(key)
                        || Self::expression_contains_assignment_or_update(value)
                }
                crate::ir::hir::ObjectEntry::Getter { key, getter } => {
                    Self::expression_contains_assignment_or_update(key)
                        || Self::expression_contains_assignment_or_update(getter)
                }
                crate::ir::hir::ObjectEntry::Setter { key, setter } => {
                    Self::expression_contains_assignment_or_update(key)
                        || Self::expression_contains_assignment_or_update(setter)
                }
                crate::ir::hir::ObjectEntry::Spread(expression) => {
                    Self::expression_contains_assignment_or_update(expression)
                }
            }),
            Expression::Member { object, property } => {
                Self::expression_contains_assignment_or_update(object)
                    || Self::expression_contains_assignment_or_update(property)
            }
            Expression::SuperMember { property } => {
                Self::expression_contains_assignment_or_update(property)
            }
            Expression::Call { callee, arguments }
            | Expression::SuperCall { callee, arguments }
            | Expression::New { callee, arguments } => {
                Self::expression_contains_assignment_or_update(callee)
                    || arguments.iter().any(|argument| match argument {
                        CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                            Self::expression_contains_assignment_or_update(expression)
                        }
                    })
            }
            Expression::Number(_)
            | Expression::BigInt(_)
            | Expression::String(_)
            | Expression::Bool(_)
            | Expression::Null
            | Expression::Undefined
            | Expression::Identifier(_)
            | Expression::This
            | Expression::NewTarget
            | Expression::Sent => false,
        }
    }

    pub(in crate::backend::direct_wasm) fn expression_references_internal_assignment_temp(
        expression: &Expression,
    ) -> bool {
        match expression {
            Expression::Identifier(name) => {
                name.starts_with("__ayy_optional_base_")
                    || name.starts_with("__ayy_target_object_")
                    || name.starts_with("__ayy_target_property_")
                    || name.starts_with("__ayy_postfix_previous_")
            }
            Expression::Unary { expression, .. }
            | Expression::Await(expression)
            | Expression::EnumerateKeys(expression)
            | Expression::GetIterator(expression)
            | Expression::IteratorClose(expression) => {
                Self::expression_references_internal_assignment_temp(expression)
            }
            Expression::Binary { left, right, .. } => {
                Self::expression_references_internal_assignment_temp(left)
                    || Self::expression_references_internal_assignment_temp(right)
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                Self::expression_references_internal_assignment_temp(condition)
                    || Self::expression_references_internal_assignment_temp(then_expression)
                    || Self::expression_references_internal_assignment_temp(else_expression)
            }
            Expression::Sequence(expressions) => expressions
                .iter()
                .any(Self::expression_references_internal_assignment_temp),
            Expression::Array(elements) => elements.iter().any(|element| match element {
                crate::ir::hir::ArrayElement::Expression(expression)
                | crate::ir::hir::ArrayElement::Spread(expression) => {
                    Self::expression_references_internal_assignment_temp(expression)
                }
            }),
            Expression::Object(entries) => entries.iter().any(|entry| match entry {
                crate::ir::hir::ObjectEntry::Data { key, value } => {
                    Self::expression_references_internal_assignment_temp(key)
                        || Self::expression_references_internal_assignment_temp(value)
                }
                crate::ir::hir::ObjectEntry::Getter { key, getter } => {
                    Self::expression_references_internal_assignment_temp(key)
                        || Self::expression_references_internal_assignment_temp(getter)
                }
                crate::ir::hir::ObjectEntry::Setter { key, setter } => {
                    Self::expression_references_internal_assignment_temp(key)
                        || Self::expression_references_internal_assignment_temp(setter)
                }
                crate::ir::hir::ObjectEntry::Spread(expression) => {
                    Self::expression_references_internal_assignment_temp(expression)
                }
            }),
            Expression::Member { object, property } => {
                Self::expression_references_internal_assignment_temp(object)
                    || Self::expression_references_internal_assignment_temp(property)
            }
            Expression::SuperMember { property } => {
                Self::expression_references_internal_assignment_temp(property)
            }
            Expression::Assign { value, .. } => {
                Self::expression_references_internal_assignment_temp(value)
            }
            Expression::Update { name, .. } => {
                name.starts_with("__ayy_optional_base_")
                    || name.starts_with("__ayy_target_object_")
                    || name.starts_with("__ayy_target_property_")
                    || name.starts_with("__ayy_postfix_previous_")
            }
            Expression::AssignMember {
                object,
                property,
                value,
            } => {
                Self::expression_references_internal_assignment_temp(object)
                    || Self::expression_references_internal_assignment_temp(property)
                    || Self::expression_references_internal_assignment_temp(value)
            }
            Expression::AssignSuperMember { property, value } => {
                Self::expression_references_internal_assignment_temp(property)
                    || Self::expression_references_internal_assignment_temp(value)
            }
            Expression::Call { callee, arguments }
            | Expression::SuperCall { callee, arguments }
            | Expression::New { callee, arguments } => {
                Self::expression_references_internal_assignment_temp(callee)
                    || arguments.iter().any(|argument| match argument {
                        CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                            Self::expression_references_internal_assignment_temp(expression)
                        }
                    })
            }
            Expression::Number(_)
            | Expression::BigInt(_)
            | Expression::String(_)
            | Expression::Bool(_)
            | Expression::Null
            | Expression::Undefined
            | Expression::This
            | Expression::NewTarget
            | Expression::Sent => false,
        }
    }

    fn expression_is_relational_binary(expression: &Expression) -> bool {
        matches!(
            expression,
            Expression::Binary {
                op: BinaryOp::LessThan
                    | BinaryOp::LessThanOrEqual
                    | BinaryOp::GreaterThan
                    | BinaryOp::GreaterThanOrEqual,
                ..
            }
        )
    }

    fn flatten_addition_parts<'expr>(
        expression: &'expr Expression,
        parts: &mut Vec<&'expr Expression>,
    ) {
        if let Expression::Binary {
            op: BinaryOp::Add,
            left,
            right,
        } = expression
        {
            Self::flatten_addition_parts(left, parts);
            Self::flatten_addition_parts(right, parts);
        } else {
            parts.push(expression);
        }
    }

    fn static_template_number_to_string(value: f64) -> String {
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

    fn emit_static_template_update_addition(
        &mut self,
        expression: &Expression,
    ) -> DirectResult<bool> {
        let mut parts = Vec::new();
        Self::flatten_addition_parts(expression, &mut parts);
        if parts.len() < 3 {
            return Ok(false);
        }

        let mut saw_string = false;
        let mut saw_update = false;
        let mut simulated_numbers = HashMap::new();
        let mut updates = Vec::new();
        let mut text = String::new();

        for part in parts {
            match part {
                Expression::String(fragment) => {
                    saw_string = true;
                    text.push_str(fragment);
                }
                Expression::Update { name, op, prefix } => {
                    saw_update = true;
                    let previous_number = if let Some(value) = simulated_numbers.get(name).copied()
                    {
                        value
                    } else {
                        let target = Expression::Identifier(name.clone());
                        let Some(value) = self.resolve_static_number_value(&target) else {
                            return Ok(false);
                        };
                        simulated_numbers.insert(name.clone(), value);
                        value
                    };
                    let increment = match op {
                        UpdateOp::Increment => 1.0,
                        UpdateOp::Decrement => -1.0,
                    };
                    let next_number = previous_number + increment;
                    let yielded_number = if *prefix {
                        next_number
                    } else {
                        previous_number
                    };
                    text.push_str(&Self::static_template_number_to_string(yielded_number));
                    simulated_numbers.insert(name.clone(), next_number);
                    updates.push((name.clone(), next_number));
                }
                _ => return Ok(false),
            }
        }

        if !saw_string || !saw_update {
            return Ok(false);
        }

        for (name, next_number) in updates {
            let next_expression = Expression::Number(next_number);
            let next_local = self.allocate_temp_local();
            self.emit_numeric_expression(&next_expression)?;
            self.push_local_set(next_local);
            self.emit_store_identifier_value_local(&name, &next_expression, next_local)?;
            self.note_identifier_numeric_kind(&name);
        }
        self.emit_static_string_literal(&text)?;
        Ok(true)
    }

    pub(in crate::backend::direct_wasm) fn active_loop_integer_value(
        &mut self,
        expression: &Expression,
        environment: &HashMap<String, i64>,
    ) -> Option<i64> {
        match expression {
            Expression::Number(_) => Self::active_loop_integer_literal(expression),
            Expression::Identifier(name) => environment
                .get(name)
                .copied()
                .or_else(|| {
                    scoped_binding_source_name(name)
                        .and_then(|source_name| environment.get(source_name).copied())
                })
                .or_else(|| {
                    self.global_value_binding(name)
                        .and_then(Self::active_loop_integer_literal)
                })
                .or_else(|| {
                    self.state
                        .speculation
                        .static_semantics
                        .local_value_binding(name)
                        .and_then(Self::active_loop_integer_literal)
                }),
            Expression::Unary {
                op: UnaryOp::Plus,
                expression,
            } => self.active_loop_integer_value(expression, environment),
            Expression::Unary {
                op: UnaryOp::Negate,
                expression,
            } => self
                .active_loop_integer_value(expression, environment)
                .map(|value| -value),
            Expression::Binary {
                op: BinaryOp::Add,
                left,
                right,
            } => Some(
                self.active_loop_integer_value(left, environment)?
                    + self.active_loop_integer_value(right, environment)?,
            ),
            Expression::Binary {
                op: BinaryOp::Subtract,
                left,
                right,
            } => Some(
                self.active_loop_integer_value(left, environment)?
                    - self.active_loop_integer_value(right, environment)?,
            ),
            Expression::Member { .. } => self.active_loop_static_array_length_member(expression),
            _ => None,
        }
    }

    fn active_numeric_loop_specs(&self) -> Vec<NumericLoopSpec> {
        self.state
            .emission
            .control_flow
            .loop_stack
            .iter()
            .filter_map(|loop_context| loop_context.numeric_spec.clone())
            .collect::<Vec<_>>()
    }

    pub(in crate::backend::direct_wasm) fn active_numeric_loop_environments(
        &mut self,
    ) -> Option<Vec<HashMap<String, i64>>> {
        let specs = self.active_numeric_loop_specs();
        if specs.is_empty() {
            return None;
        }

        let mut environments = vec![HashMap::new()];
        for spec in specs {
            let mut next_environments = Vec::new();
            for environment in &environments {
                let bound = self.active_loop_integer_value(&spec.bound, environment)?;
                let end = if spec.inclusive {
                    bound
                } else {
                    bound.saturating_sub(1)
                };
                if end < spec.start {
                    continue;
                }
                if end - spec.start > RUNTIME_STRING_ADDITION_CANDIDATE_LIMIT as i64 {
                    return None;
                }
                for value in spec.start..=end {
                    let mut next_environment = environment.clone();
                    next_environment.insert(spec.binding.clone(), value);
                    next_environments.push(next_environment);
                    if next_environments.len() > RUNTIME_STRING_ADDITION_CANDIDATE_LIMIT {
                        return None;
                    }
                }
            }
            environments = next_environments;
            if environments.is_empty() {
                return Some(environments);
            }
        }
        Some(environments)
    }

    fn active_loop_stringified_expression_value(
        &mut self,
        expression: &Expression,
        environment: &HashMap<String, i64>,
    ) -> Option<String> {
        match expression {
            Expression::String(text) => Some(text.clone()),
            Expression::Number(_) => {
                Self::active_loop_integer_literal(expression).map(|value| value.to_string())
            }
            Expression::Bool(value) => Some(value.to_string()),
            Expression::Null => Some("null".to_string()),
            Expression::Undefined => Some("undefined".to_string()),
            Expression::Identifier(name) => environment
                .get(name)
                .copied()
                .or_else(|| {
                    scoped_binding_source_name(name)
                        .and_then(|source_name| environment.get(source_name).copied())
                })
                .map(|value| value.to_string())
                .or_else(|| self.resolve_static_string_value(expression))
                .or_else(|| {
                    let value = self
                        .state
                        .speculation
                        .static_semantics
                        .local_value_binding(name)
                        .cloned()?;
                    (!static_expression_matches(&value, expression))
                        .then(|| self.active_loop_stringified_expression_value(&value, environment))
                        .flatten()
                })
                .or_else(|| {
                    let value = self.global_value_binding(name).cloned()?;
                    (!static_expression_matches(&value, expression))
                        .then(|| self.active_loop_stringified_expression_value(&value, environment))
                        .flatten()
                })
                .or_else(|| {
                    self.global_value_binding(name)
                        .and_then(Self::active_loop_integer_literal)
                        .map(|value| value.to_string())
                })
                .or_else(|| {
                    self.state
                        .speculation
                        .static_semantics
                        .local_value_binding(name)
                        .and_then(Self::active_loop_integer_literal)
                        .map(|value| value.to_string())
                }),
            Expression::Unary {
                op: UnaryOp::Plus | UnaryOp::Negate,
                ..
            }
            | Expression::Binary {
                op: BinaryOp::Subtract | BinaryOp::Multiply | BinaryOp::Divide | BinaryOp::Modulo,
                ..
            } => self
                .active_loop_integer_value(expression, environment)
                .map(|value| value.to_string()),
            Expression::Binary {
                op: BinaryOp::Add,
                left,
                right,
            } => Some(format!(
                "{}{}",
                self.active_loop_stringified_expression_value(left, environment)?,
                self.active_loop_stringified_expression_value(right, environment)?
            )),
            Expression::Member { object, property } => {
                let value = self.active_loop_indexed_static_array_member_value(
                    object,
                    property,
                    environment,
                )?;
                self.active_loop_stringified_expression_value(&value, environment)
                    .or_else(|| {
                        self.resolve_static_string_concat_value(
                            &value,
                            self.current_function_name(),
                        )
                    })
            }
            _ if !self.expression_depends_on_active_loop_assignment(expression) => {
                self.resolve_static_string_concat_value(expression, self.current_function_name())
            }
            _ => None,
        }
    }

    fn active_loop_indexed_static_array_member_value(
        &mut self,
        object: &Expression,
        property: &Expression,
        environment: &HashMap<String, i64>,
    ) -> Option<Expression> {
        let index = self.active_loop_integer_value(property, environment)?;
        if index < 0 {
            return None;
        }
        let array_binding = self.resolve_array_binding_from_expression(object)?;
        Some(
            array_binding
                .values
                .get(index as usize)
                .cloned()
                .flatten()
                .unwrap_or(Expression::Undefined),
        )
    }

    fn active_loop_candidate_value_expression(
        &mut self,
        expression: &Expression,
        environment: &HashMap<String, i64>,
        text: &str,
    ) -> Expression {
        if self.infer_value_kind(expression) == Some(StaticValueKind::String) {
            return Expression::String(text.to_string());
        }
        if let Some(value) = self.active_loop_integer_value(expression, environment) {
            return Expression::Number(value as f64);
        }
        match expression {
            Expression::Bool(value) => Expression::Bool(*value),
            Expression::Null => Expression::Null,
            Expression::Undefined => Expression::Undefined,
            _ => Expression::String(text.to_string()),
        }
    }

    fn active_loop_stringified_candidate_environment_sequence(
        &mut self,
        expression: &Expression,
        require_string_kind: bool,
    ) -> Option<Vec<(HashMap<String, i64>, Expression, String)>> {
        if self.expression_has_dynamic_member_property_access(expression)
            && !self.expression_is_active_loop_indexed_static_array_member(expression)
        {
            return None;
        }
        if !self.expression_depends_on_active_loop_assignment(expression) {
            return None;
        }
        if require_string_kind && self.infer_value_kind(expression) != Some(StaticValueKind::String)
        {
            return None;
        }
        let environments = self.active_numeric_loop_environments()?;
        if environments.is_empty() {
            return Some(Vec::new());
        }
        let mut sequence = Vec::new();
        for environment in environments {
            let text = self.active_loop_stringified_expression_value(expression, &environment)?;
            let value =
                self.active_loop_candidate_value_expression(expression, &environment, &text);
            sequence.push((environment, value, text));
        }
        Some(sequence)
    }

    fn active_loop_string_expression_environment_sequence(
        &mut self,
        expression: &Expression,
    ) -> Option<Vec<(HashMap<String, i64>, String)>> {
        self.active_loop_stringified_candidate_environment_sequence(expression, true)
            .map(|sequence| {
                sequence
                    .into_iter()
                    .map(|(environment, _, text)| (environment, text))
                    .collect::<Vec<_>>()
            })
    }

    fn active_loop_string_expression_sequence(
        &mut self,
        expression: &Expression,
    ) -> Option<Vec<String>> {
        self.active_loop_string_expression_environment_sequence(expression)
            .map(|sequence| {
                sequence
                    .into_iter()
                    .map(|(_, text)| text)
                    .collect::<Vec<_>>()
            })
    }

    fn active_loop_stringified_candidate_sequence(
        &mut self,
        expression: &Expression,
    ) -> Option<Vec<(Expression, String)>> {
        self.active_loop_stringified_candidate_environment_sequence(expression, false)
            .map(|sequence| {
                sequence
                    .into_iter()
                    .map(|(_, value, text)| (value, text))
                    .collect::<Vec<_>>()
            })
    }

    fn push_active_loop_string_expression_candidates(
        &mut self,
        expression: &Expression,
        candidates: &mut Vec<(Expression, String)>,
    ) -> bool {
        let Some(sequence) = self.active_loop_string_expression_sequence(expression) else {
            return false;
        };
        for text in sequence {
            if !candidates
                .iter()
                .any(|(existing_value, existing_text): &(Expression, String)| {
                    matches!(existing_value, Expression::String(existing) if existing == &text)
                        && existing_text == &text
                })
            {
                candidates.push((Expression::String(text.clone()), text));
                if candidates.len() >= RUNTIME_STRING_ADDITION_CANDIDATE_LIMIT {
                    break;
                }
            }
        }
        !candidates.is_empty()
    }

    fn emit_active_loop_string_expression_from_sequence(
        &mut self,
        expression: &Expression,
    ) -> DirectResult<bool> {
        if let Expression::Binary {
            op: BinaryOp::Add,
            left,
            ..
        } = expression
            && let Expression::Identifier(name) = left.as_ref()
            && self.expression_depends_on_active_loop_assignment(left)
            && self.active_loop_numeric_binding_candidates(name).is_none()
        {
            return Ok(false);
        }

        let Some(sequence) = self.active_loop_string_expression_environment_sequence(expression)
        else {
            return Ok(false);
        };
        if sequence.is_empty() {
            return Ok(false);
        }
        let specs = self.active_numeric_loop_specs();
        if specs.is_empty() {
            return Ok(false);
        }

        let result_local = self.allocate_temp_local();
        self.emit_static_string_literal(&sequence[0].1)?;
        self.push_local_set(result_local);

        for (environment, text) in sequence {
            let mut emitted_condition = false;
            for spec in &specs {
                let Some(expected) = environment.get(&spec.binding).copied() else {
                    return Ok(false);
                };
                self.emit_numeric_expression(&Expression::Identifier(spec.binding.clone()))?;
                self.push_i32_const(expected as i32);
                self.push_binary_op(BinaryOp::Equal)?;
                if emitted_condition {
                    self.state.emission.output.instructions.push(0x71);
                }
                emitted_condition = true;
            }
            if !emitted_condition {
                return Ok(false);
            }
            self.state.emission.output.instructions.push(0x04);
            self.state
                .emission
                .output
                .instructions
                .push(EMPTY_BLOCK_TYPE);
            self.push_control_frame();
            self.emit_static_string_literal(&text)?;
            self.push_local_set(result_local);
            self.state.emission.output.instructions.push(0x0b);
            self.pop_control_frame();
        }

        self.push_local_get(result_local);
        Ok(true)
    }

    fn for_in_key_array_member_name(&self, expression: &Expression) -> Option<String> {
        if let Expression::Identifier(name) = expression
            && let Some(value) = self
                .state
                .speculation
                .static_semantics
                .local_value_binding(name)
        {
            return self.for_in_key_array_member_name(value);
        }
        if let Expression::Identifier(name) = expression
            && let Some(value) = self.global_value_binding(name)
        {
            return self.for_in_key_array_member_name(value);
        }
        let Expression::Member { object, .. } = expression else {
            return None;
        };
        match object.as_ref() {
            Expression::Identifier(name) if name.starts_with("__ayy_for_in_keys_") => {
                Some(name.clone())
            }
            _ => None,
        }
    }

    fn runtime_string_correlated_for_in_member_candidates(
        &mut self,
        key_expression: &Expression,
        value_expression: &Expression,
    ) -> Vec<(Expression, String)> {
        let Some(key_array_name) = self.for_in_key_array_member_name(key_expression) else {
            return Vec::new();
        };
        let Some(key_array_binding) = self
            .state
            .speculation
            .static_semantics
            .local_array_binding(&key_array_name)
            .cloned()
        else {
            return Vec::new();
        };
        let Expression::Member { object, property } = value_expression else {
            return Vec::new();
        };
        if !self.expression_depends_on_active_loop_assignment(property) {
            return Vec::new();
        }
        let Some(object_binding) = self.resolve_object_binding_from_expression(object) else {
            return Vec::new();
        };

        let mut candidates = Vec::new();
        for key in key_array_binding.values.iter().flatten() {
            let Expression::String(property_name) = key else {
                continue;
            };
            let property = Expression::String(property_name.clone());
            if self.runtime_object_property_shadow_deletion_is_statically_present(object, &property)
            {
                continue;
            }
            let Some(value) = self.resolve_object_binding_property_value_with_inherited(
                object,
                &object_binding,
                &property,
            ) else {
                continue;
            };
            let text = match value {
                Expression::String(text) => text.clone(),
                Expression::Number(number) if number.fract() == 0.0 => (number as i64).to_string(),
                _ if inline_summary_side_effect_free_expression(&value) => {
                    let Some(text) = self
                        .resolve_static_string_concat_value(&value, self.current_function_name())
                    else {
                        continue;
                    };
                    text
                }
                _ => continue,
            };
            let candidate_text = format!("{property_name}{text}");
            candidates.push((Expression::String(candidate_text.clone()), candidate_text));
            if candidates.len() >= RUNTIME_STRING_ADDITION_CANDIDATE_LIMIT {
                break;
            }
        }
        candidates
    }

    fn runtime_string_iterator_step_value_candidates(
        &mut self,
        expression: &Expression,
    ) -> Vec<(Expression, String)> {
        let Expression::Member { object, property } = expression else {
            return Vec::new();
        };
        let property = self.materialize_static_expression(property);
        if !matches!(property, Expression::String(ref property_name) if property_name == "value") {
            return Vec::new();
        }
        let Some(IteratorStepBinding::Runtime {
            value_candidates, ..
        }) = self.resolve_iterator_step_binding_from_expression(object)
        else {
            return Vec::new();
        };

        let mut candidates = Vec::new();
        for value in value_candidates {
            let candidate_value = self.materialize_static_expression(&value);
            let Some(candidate_text) = self
                .resolve_static_string_concat_value(&candidate_value, self.current_function_name())
            else {
                continue;
            };
            if candidates
                .iter()
                .any(|(existing_value, existing_text): &(Expression, String)| {
                    static_expression_matches(existing_value, &candidate_value)
                        && existing_text == &candidate_text
                })
            {
                continue;
            }
            candidates.push((candidate_value, candidate_text));
            if candidates.len() >= RUNTIME_STRING_ADDITION_CANDIDATE_LIMIT {
                break;
            }
        }
        candidates
    }

    pub(in crate::backend::direct_wasm) fn runtime_string_addition_right_candidates(
        &mut self,
        expression: &Expression,
    ) -> Vec<(Expression, String)> {
        if self.expression_has_dynamic_member_property_access(expression)
            && !self.expression_is_active_loop_indexed_static_array_member(expression)
        {
            return Vec::new();
        }
        let mut candidates = Vec::new();
        if self.push_active_loop_string_expression_candidates(expression, &mut candidates) {
            return candidates;
        }
        if let Expression::Identifier(name) = expression
            && let Some(numeric_candidates) = self.active_loop_numeric_binding_candidates(name)
        {
            for value in numeric_candidates {
                let text = value.to_string();
                candidates.push((Expression::Number(value as f64), text));
                if candidates.len() >= RUNTIME_STRING_ADDITION_CANDIDATE_LIMIT {
                    return candidates;
                }
            }
        }
        if inline_summary_side_effect_free_expression(expression)
            && let Some(text) =
                self.resolve_static_string_concat_value(expression, self.current_function_name())
        {
            candidates.push((self.materialize_static_expression(expression), text));
        }

        for (candidate_value, candidate_text) in
            self.runtime_string_iterator_step_value_candidates(expression)
        {
            if !candidates
                .iter()
                .any(|(existing_value, existing_text): &(Expression, String)| {
                    static_expression_matches(existing_value, &candidate_value)
                        && existing_text == &candidate_text
                })
            {
                candidates.push((candidate_value, candidate_text));
            }
            if candidates.len() >= RUNTIME_STRING_ADDITION_CANDIDATE_LIMIT {
                break;
            }
        }

        let may_materialize = !self.expression_depends_on_active_loop_assignment(expression)
            || matches!(expression, Expression::Identifier(_));
        if may_materialize {
            let materialized_expression = self.materialize_static_expression(expression);
            if !static_expression_matches(&materialized_expression, expression) {
                for (candidate_value, candidate_text) in
                    self.runtime_string_addition_right_candidates(&materialized_expression)
                {
                    if !candidates.iter().any(
                        |(existing_value, existing_text): &(Expression, String)| {
                            static_expression_matches(existing_value, &candidate_value)
                                && existing_text == &candidate_text
                        },
                    ) {
                        candidates.push((candidate_value, candidate_text));
                    }
                    if candidates.len() >= RUNTIME_STRING_ADDITION_CANDIDATE_LIMIT {
                        break;
                    }
                }
            }
        }

        if let Expression::Binary {
            op: BinaryOp::Add,
            left,
            right,
        } = expression
        {
            let correlated_candidates =
                self.runtime_string_correlated_for_in_member_candidates(left, right);
            if !correlated_candidates.is_empty() {
                for (candidate_value, candidate_text) in correlated_candidates {
                    if !candidates.iter().any(
                        |(existing_value, existing_text): &(Expression, String)| {
                            static_expression_matches(existing_value, &candidate_value)
                                && existing_text == &candidate_text
                        },
                    ) {
                        candidates.push((candidate_value, candidate_text));
                    }
                    if candidates.len() >= RUNTIME_STRING_ADDITION_CANDIDATE_LIMIT {
                        break;
                    }
                }
                candidates.truncate(RUNTIME_STRING_ADDITION_CANDIDATE_LIMIT);
                return candidates;
            }
            let left_candidates = self.runtime_string_addition_right_candidates(left);
            let right_candidates = self.runtime_string_addition_right_candidates(right);
            for (_, left_text) in &left_candidates {
                for (_, right_text) in &right_candidates {
                    let text = format!("{left_text}{right_text}");
                    if !candidates.iter().any(
                        |(existing_value, existing_text): &(Expression, String)| {
                            matches!(existing_value, Expression::String(existing) if existing == &text)
                                && existing_text == &text
                        },
                    ) {
                        candidates.push((Expression::String(text.clone()), text));
                    }
                    if candidates.len() >= RUNTIME_STRING_ADDITION_CANDIDATE_LIMIT {
                        break;
                    }
                }
                if candidates.len() >= RUNTIME_STRING_ADDITION_CANDIDATE_LIMIT {
                    break;
                }
            }
        }

        if let Expression::Member { object, .. } = expression
            && let Some(array_binding) = self.resolve_array_binding_from_expression(object)
        {
            for value in array_binding.values.iter().flatten() {
                if !inline_summary_side_effect_free_expression(value) {
                    continue;
                }
                let Some(text) =
                    self.resolve_static_string_concat_value(value, self.current_function_name())
                else {
                    continue;
                };
                let candidate_value = self.materialize_static_expression(value);
                if !candidates.iter().any(
                    |(existing_value, existing_text): &(Expression, String)| {
                        static_expression_matches(existing_value, &candidate_value)
                            && existing_text == &text
                    },
                ) {
                    candidates.push((candidate_value, text));
                }
            }
        }
        if let Expression::Member { object, .. } = expression
            && let Some(object_binding) = self.resolve_object_binding_from_expression(object)
        {
            for (_, value) in
                self.object_binding_string_property_values_with_inherited(object, &object_binding)
            {
                let text = if self.expression_depends_on_active_loop_assignment(expression) {
                    match &value {
                        Expression::String(text) => Some(text.clone()),
                        Expression::Number(number) if number.fract() == 0.0 => {
                            Some((*number as i64).to_string())
                        }
                        _ => None,
                    }
                } else if inline_summary_side_effect_free_expression(&value) {
                    self.resolve_static_string_concat_value(&value, self.current_function_name())
                } else {
                    None
                };
                let Some(text) = text else {
                    continue;
                };
                let candidate_value = match &value {
                    Expression::String(_) | Expression::Number(_) => value,
                    _ => self.materialize_static_expression(&value),
                };
                if !candidates.iter().any(
                    |(existing_value, existing_text): &(Expression, String)| {
                        static_expression_matches(existing_value, &candidate_value)
                            && existing_text == &text
                    },
                ) {
                    candidates.push((candidate_value, text));
                }
                if candidates.len() >= RUNTIME_STRING_ADDITION_CANDIDATE_LIMIT {
                    break;
                }
            }
        }

        if candidates.is_empty()
            && self.infer_value_kind(expression) == Some(StaticValueKind::String)
        {
            for (_, text) in self.runtime_string_print_candidates(expression) {
                let candidate_value = Expression::String(text.clone());
                if !candidates.iter().any(
                    |(existing_value, existing_text): &(Expression, String)| {
                        static_expression_matches(existing_value, &candidate_value)
                            && existing_text == &text
                    },
                ) {
                    candidates.push((candidate_value, text));
                }
                if candidates.len() >= RUNTIME_STRING_ADDITION_CANDIDATE_LIMIT {
                    break;
                }
            }
        }

        candidates.truncate(RUNTIME_STRING_ADDITION_CANDIDATE_LIMIT);
        for (_, text) in &candidates {
            self.intern_runtime_string_candidate_text(text);
        }
        candidates
    }

    fn runtime_string_addition_left_candidates(
        &mut self,
        left: &Expression,
        right_candidates: &[(Expression, String)],
    ) -> Vec<(Expression, String)> {
        let mut candidates = Vec::new();
        let left_kind = self.infer_value_kind(left);
        let left_is_definitely_string = left_kind == Some(StaticValueKind::String);
        if left_is_definitely_string && let Some(text) = self.resolve_static_string_value(left) {
            Self::push_unique_runtime_string_value_candidate(
                &mut candidates,
                Expression::String(text.clone()),
                text,
            );
        }
        if left_is_definitely_string
            && inline_summary_side_effect_free_expression(left)
            && let Some(text) =
                self.resolve_static_string_concat_value(left, self.current_function_name())
        {
            let value = self.materialize_static_expression(left);
            if !static_expression_matches(&value, left) {
                Self::push_unique_runtime_string_value_candidate(&mut candidates, value, text);
            }
        }
        match left_kind {
            Some(StaticValueKind::Undefined) | Some(StaticValueKind::Unknown) | None => {
                Self::push_unique_runtime_string_value_candidate(
                    &mut candidates,
                    Expression::Undefined,
                    "undefined".to_string(),
                );
            }
            _ => {}
        }
        match left_kind {
            Some(StaticValueKind::Null) | Some(StaticValueKind::Unknown) | None => {
                Self::push_unique_runtime_string_value_candidate(
                    &mut candidates,
                    Expression::Null,
                    "null".to_string(),
                );
            }
            _ => {}
        }
        if left_kind == Some(StaticValueKind::Bool) {
            Self::push_unique_runtime_string_value_candidate(
                &mut candidates,
                Expression::Bool(false),
                "false".to_string(),
            );
            Self::push_unique_runtime_string_value_candidate(
                &mut candidates,
                Expression::Bool(true),
                "true".to_string(),
            );
        }
        if self.expression_depends_on_active_loop_assignment(left) && left_is_definitely_string {
            Self::push_unique_runtime_string_value_candidate(
                &mut candidates,
                Expression::String(String::new()),
                String::new(),
            );
        }
        if self.expression_depends_on_active_loop_assignment(left) {
            for (_, text) in self.runtime_string_addition_right_candidates(left) {
                Self::push_unique_runtime_string_value_candidate(
                    &mut candidates,
                    Expression::String(text.clone()),
                    text,
                );
                if candidates.len() >= 256 {
                    break;
                }
            }
        }
        if left_is_definitely_string {
            let string_data = self.backend.module_artifacts.string_data.clone();
            let mut fragments = Vec::new();
            for (_, bytes) in string_data {
                let Ok(text) = String::from_utf8(bytes) else {
                    continue;
                };
                if text.len() == 1
                    && text
                        .as_bytes()
                        .first()
                        .is_some_and(|byte| byte.is_ascii_alphanumeric())
                    && !fragments.iter().any(|existing| existing == &text)
                {
                    fragments.push(text);
                }
                if fragments.len() >= 8 {
                    break;
                }
            }
            let mut prefix = String::new();
            for fragment in &fragments {
                prefix.push_str(fragment);
                Self::push_unique_runtime_string_value_candidate(
                    &mut candidates,
                    Expression::String(prefix.clone()),
                    prefix.clone(),
                );
                if candidates.len() >= 256 {
                    break;
                }
            }
        }
        if left_is_definitely_string {
            let string_data = self.backend.module_artifacts.string_data.clone();
            for (_, bytes) in string_data {
                let Ok(text) = String::from_utf8(bytes) else {
                    continue;
                };
                Self::push_unique_runtime_string_value_candidate(
                    &mut candidates,
                    Expression::String(text.clone()),
                    text,
                );
                if candidates.len() >= 256 {
                    break;
                }
            }
        }
        let mut frontier = candidates
            .iter()
            .map(|(_, text)| text.clone())
            .collect::<Vec<_>>();
        let depth = right_candidates.len().clamp(1, 6);
        for _ in 0..depth {
            if candidates.len() >= 256 || frontier.is_empty() {
                break;
            }
            let mut next_frontier = Vec::new();
            for prefix in frontier {
                for (_, suffix) in right_candidates {
                    let combined = format!("{prefix}{suffix}");
                    if Self::push_unique_runtime_string_value_candidate(
                        &mut candidates,
                        Expression::String(combined.clone()),
                        combined.clone(),
                    ) {
                        next_frontier.push(combined);
                        if candidates.len() >= 256 {
                            break;
                        }
                    }
                }
                if candidates.len() >= 256 {
                    break;
                }
            }
            frontier = next_frontier;
        }

        for (_, text) in &candidates {
            self.intern_runtime_string_candidate_text(text);
        }
        candidates
    }

    fn emit_runtime_string_candidate_value(&mut self, value: &Expression) -> DirectResult<()> {
        self.emit_numeric_expression(value)
    }

    fn emit_string_append_transition(
        &mut self,
        left_local: u32,
        right_local: u32,
        result_local: u32,
        prefix: &str,
        right_value: &Expression,
        suffix: &str,
    ) -> DirectResult<String> {
        self.push_local_get(left_local);
        self.emit_numeric_expression(&Expression::String(prefix.to_string()))?;
        self.push_binary_op(BinaryOp::Equal)?;

        self.push_local_get(right_local);
        self.emit_runtime_string_candidate_value(right_value)?;
        self.push_binary_op(BinaryOp::Equal)?;
        self.state.emission.output.instructions.push(0x71);

        self.state.emission.output.instructions.push(0x04);
        self.state
            .emission
            .output
            .instructions
            .push(EMPTY_BLOCK_TYPE);
        self.push_control_frame();
        let next_prefix = format!("{prefix}{suffix}");
        self.emit_numeric_expression(&Expression::String(next_prefix.clone()))?;
        self.push_local_set(result_local);
        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();
        Ok(next_prefix)
    }

    fn emit_active_loop_string_append_sequence(
        &mut self,
        left: &Expression,
        right: &Expression,
        sequence: &[(Expression, String)],
    ) -> DirectResult<bool> {
        if sequence.is_empty()
            || !matches!(left, Expression::Identifier(_))
            || !self.expression_depends_on_active_loop_assignment(left)
        {
            return Ok(false);
        }

        let left_local = self.allocate_temp_local();
        let right_local = self.allocate_temp_local();
        let result_local = self.allocate_temp_local();

        self.emit_numeric_expression(left)?;
        self.push_local_set(left_local);
        self.emit_numeric_expression(right)?;
        self.push_local_set(right_local);

        self.push_local_get(left_local);
        self.push_local_get(right_local);
        self.push_binary_op(BinaryOp::Add)?;
        self.push_local_set(result_local);

        let mut emitted = HashSet::new();
        for start in 0..sequence.len() {
            let mut prefix = self.resolve_static_string_value(left).unwrap_or_default();
            for (right_value, suffix) in &sequence[start..] {
                if emitted.insert((prefix.clone(), suffix.clone())) {
                    prefix = self.emit_string_append_transition(
                        left_local,
                        right_local,
                        result_local,
                        &prefix,
                        right_value,
                        suffix,
                    )?;
                } else {
                    prefix.push_str(suffix);
                }
            }
        }

        const SKIPPED_APPEND_PREFIX_LIMIT: usize = 4096;
        let mut prefixes = vec![self.resolve_static_string_value(left).unwrap_or_default()];
        for (right_value, suffix) in sequence {
            let current_prefixes = prefixes.clone();
            for prefix in current_prefixes {
                let next_prefix = if emitted.insert((prefix.clone(), suffix.clone())) {
                    self.emit_string_append_transition(
                        left_local,
                        right_local,
                        result_local,
                        &prefix,
                        right_value,
                        suffix,
                    )?
                } else {
                    format!("{prefix}{suffix}")
                };
                if !prefixes.iter().any(|existing| existing == &next_prefix) {
                    prefixes.push(next_prefix);
                    if prefixes.len() >= SKIPPED_APPEND_PREFIX_LIMIT {
                        break;
                    }
                }
            }
            if prefixes.len() >= SKIPPED_APPEND_PREFIX_LIMIT {
                break;
            }
        }

        self.push_local_get(result_local);
        Ok(true)
    }

    fn emit_runtime_string_addition_from_candidates(
        &mut self,
        left: &Expression,
        right: &Expression,
    ) -> DirectResult<bool> {
        if (self.expression_has_dynamic_member_property_access(left)
            && !self.expression_is_active_loop_indexed_static_array_member(left))
            || (self.expression_has_dynamic_member_property_access(right)
                && !self.expression_is_active_loop_indexed_static_array_member(right))
        {
            return Ok(false);
        }
        if let Some(sequence) = self.active_loop_stringified_candidate_sequence(right)
            && self.emit_active_loop_string_append_sequence(left, right, &sequence)?
        {
            return Ok(true);
        }

        let right_candidates = self.runtime_string_addition_right_candidates(right);
        if right_candidates.is_empty() {
            return Ok(false);
        }
        let left_candidates = self.runtime_string_addition_left_candidates(left, &right_candidates);
        if left_candidates.is_empty() {
            return Ok(false);
        }

        let left_local = self.allocate_temp_local();
        let right_local = self.allocate_temp_local();
        let result_local = self.allocate_temp_local();
        let handled_local = self.allocate_temp_local();

        self.emit_numeric_expression(left)?;
        self.push_local_set(left_local);
        self.emit_numeric_expression(right)?;
        self.push_local_set(right_local);
        self.push_i32_const(0);
        self.push_local_set(handled_local);

        self.push_local_get(left_local);
        self.push_local_get(right_local);
        self.push_binary_op(BinaryOp::Add)?;
        self.push_local_set(result_local);

        for (left_value, left_text) in left_candidates {
            for (right_value, right_text) in &right_candidates {
                self.push_local_get(left_local);
                self.emit_runtime_string_candidate_value(&left_value)?;
                self.push_binary_op(BinaryOp::Equal)?;

                self.push_local_get(right_local);
                self.emit_runtime_string_candidate_value(right_value)?;
                self.push_binary_op(BinaryOp::Equal)?;
                self.state.emission.output.instructions.push(0x71);

                self.state.emission.output.instructions.push(0x04);
                self.state
                    .emission
                    .output
                    .instructions
                    .push(EMPTY_BLOCK_TYPE);
                self.push_control_frame();
                self.emit_numeric_expression(&Expression::String(format!(
                    "{left_text}{right_text}"
                )))?;
                self.push_local_set(result_local);
                self.push_i32_const(1);
                self.push_local_set(handled_local);
                self.state.emission.output.instructions.push(0x0b);
                self.pop_control_frame();
            }
        }

        self.push_local_get(result_local);
        Ok(true)
    }

    fn emit_stringified_division_split_length_comparison(
        &mut self,
        op: BinaryOp,
        left: &Expression,
        right: &Expression,
    ) -> DirectResult<bool> {
        if op != BinaryOp::GreaterThan
            || !matches!(right, Expression::Number(value) if *value == 1.0)
        {
            return Ok(false);
        }

        let Expression::Member {
            object: split_call,
            property: length_property,
        } = left
        else {
            return Ok(false);
        };
        if !matches!(length_property.as_ref(), Expression::String(name) if name == "length") {
            return Ok(false);
        }

        let Expression::Call { callee, arguments } = split_call.as_ref() else {
            return Ok(false);
        };
        let [
            CallArgument::Expression(Expression::String(separator))
            | CallArgument::Spread(Expression::String(separator)),
        ] = arguments.as_slice()
        else {
            return Ok(false);
        };
        if separator != "." {
            return Ok(false);
        }

        let Expression::Member {
            object: split_receiver,
            property: split_property,
        } = callee.as_ref()
        else {
            return Ok(false);
        };
        if !matches!(split_property.as_ref(), Expression::String(name) if name == "split") {
            return Ok(false);
        }

        let Expression::Binary {
            op: BinaryOp::Add,
            left: concat_left,
            right: concat_right,
        } = split_receiver.as_ref()
        else {
            return Ok(false);
        };
        if !matches!(concat_left.as_ref(), Expression::String(text) if text.is_empty()) {
            return Ok(false);
        }

        let Expression::Binary {
            op: BinaryOp::Divide,
            left: dividend,
            right: divisor,
        } = concat_right.as_ref()
        else {
            return Ok(false);
        };

        let dividend_local = self.allocate_temp_local();
        let divisor_local = self.allocate_temp_local();
        self.emit_numeric_expression(dividend)?;
        self.push_local_set(dividend_local);
        self.emit_check_global_throw_for_user_call()?;
        self.emit_conditionally_reachable_numeric_expression_to_local(divisor, divisor_local)?;
        self.emit_check_global_throw_for_user_call()?;
        self.push_local_get(dividend_local);
        self.push_local_get(divisor_local);
        self.push_binary_op(BinaryOp::Modulo)?;
        self.push_i32_const(0);
        self.push_binary_op(BinaryOp::NotEqual)?;
        Ok(true)
    }

    fn emit_safe_integer_division_or_modulo(
        &mut self,
        op: BinaryOp,
        left: &Expression,
        right: &Expression,
    ) -> DirectResult<()> {
        let left_local = self.allocate_temp_local();
        let right_local = self.allocate_temp_local();
        self.emit_numeric_expression(left)?;
        self.push_local_set(left_local);
        self.emit_check_global_throw_for_user_call()?;
        self.emit_conditionally_reachable_numeric_expression_to_local(right, right_local)?;
        self.emit_check_global_throw_for_user_call()?;

        self.push_local_get(right_local);
        self.state.emission.output.instructions.push(0x45);
        self.state.emission.output.instructions.push(0x04);
        self.state.emission.output.instructions.push(I32_TYPE);
        self.push_control_frame();
        self.push_i32_const(0);
        self.state.emission.output.instructions.push(0x05);
        self.push_local_get(left_local);
        self.push_local_get(right_local);
        self.push_binary_op(op)?;
        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();
        Ok(())
    }

    fn finite_fractional_integer_floor(value: f64) -> Option<i32> {
        if !value.is_finite() || value.fract() == 0.0 {
            return None;
        }
        let floor = value.floor();
        if floor < i32::MIN as f64 || floor > i32::MAX as f64 {
            return None;
        }
        Some(floor as i32)
    }

    fn finite_fractional_integer_ceil(value: f64) -> Option<i32> {
        if !value.is_finite() || value.fract() == 0.0 {
            return None;
        }
        let ceil = value.ceil();
        if ceil < i32::MIN as f64 || ceil > i32::MAX as f64 {
            return None;
        }
        Some(ceil as i32)
    }

    fn emit_fractional_static_relational_comparison(
        &mut self,
        op: BinaryOp,
        left: &Expression,
        right: &Expression,
    ) -> DirectResult<bool> {
        if !matches!(
            op,
            BinaryOp::LessThan
                | BinaryOp::LessThanOrEqual
                | BinaryOp::GreaterThan
                | BinaryOp::GreaterThanOrEqual
        ) {
            return Ok(false);
        }

        if inline_summary_side_effect_free_expression(right)
            && let Some(right_number) = self.resolve_static_number_value(right)
            && let Some(right_floor) = Self::finite_fractional_integer_floor(right_number)
        {
            self.emit_numeric_expression(left)?;
            self.push_i32_const(right_floor);
            let adjusted_op = match op {
                BinaryOp::LessThan | BinaryOp::LessThanOrEqual => BinaryOp::LessThanOrEqual,
                BinaryOp::GreaterThan | BinaryOp::GreaterThanOrEqual => BinaryOp::GreaterThan,
                _ => unreachable!("filtered above"),
            };
            self.push_binary_op(adjusted_op)?;
            return Ok(true);
        }

        if inline_summary_side_effect_free_expression(left)
            && let Some(left_number) = self.resolve_static_number_value(left)
        {
            match op {
                BinaryOp::LessThan | BinaryOp::LessThanOrEqual => {
                    let Some(left_ceil) = Self::finite_fractional_integer_ceil(left_number) else {
                        return Ok(false);
                    };
                    self.emit_numeric_expression(right)?;
                    self.push_i32_const(left_ceil);
                    self.push_binary_op(BinaryOp::GreaterThanOrEqual)?;
                    return Ok(true);
                }
                BinaryOp::GreaterThan | BinaryOp::GreaterThanOrEqual => {
                    let Some(left_floor) = Self::finite_fractional_integer_floor(left_number)
                    else {
                        return Ok(false);
                    };
                    self.emit_numeric_expression(right)?;
                    self.push_i32_const(left_floor);
                    self.push_binary_op(BinaryOp::LessThanOrEqual)?;
                    return Ok(true);
                }
                _ => {}
            }
        }

        Ok(false)
    }

    fn emit_static_relational_operand_effects(
        &mut self,
        expression: &Expression,
    ) -> DirectResult<bool> {
        if self
            .resolve_static_primitive_expression_with_context(
                expression,
                self.current_function_name(),
            )
            .is_some()
            || self
                .resolve_static_boxed_primitive_value(expression)
                .is_some()
        {
            return Ok(true);
        }
        if let Some(plan) = self.resolve_ordinary_to_primitive_plan(expression) {
            let result_local = self.allocate_temp_local();
            match self.emit_ordinary_to_primitive_effects_from_plan(
                expression,
                &plan,
                result_local,
            )? {
                SymbolToPrimitiveHandling::Handled => return Ok(true),
                SymbolToPrimitiveHandling::AlwaysThrows => return Ok(false),
                SymbolToPrimitiveHandling::NotHandled => {
                    if !inline_summary_side_effect_free_expression(expression) {
                        self.emit_numeric_expression(expression)?;
                        self.state.emission.output.instructions.push(0x1a);
                    }
                    return Ok(true);
                }
            }
        }

        if !inline_summary_side_effect_free_expression(expression) {
            self.emit_numeric_expression(expression)?;
            self.state.emission.output.instructions.push(0x1a);
        }

        Ok(true)
    }

    fn emit_ordinary_to_primitive_effects_from_plan(
        &mut self,
        expression: &Expression,
        plan: &OrdinaryToPrimitivePlan,
        result_local: u32,
    ) -> DirectResult<SymbolToPrimitiveHandling> {
        for step in &plan.steps {
            if matches!(step.binding, LocalFunctionBinding::Builtin(_)) {
                match &step.outcome {
                    StaticEvalOutcome::Throw(_) => {
                        self.emit_static_eval_outcome(&step.outcome)?;
                        return Ok(SymbolToPrimitiveHandling::AlwaysThrows);
                    }
                    StaticEvalOutcome::Value(value) => {
                        match self.static_expression_is_non_object_primitive(value) {
                            Some(true) => return Ok(SymbolToPrimitiveHandling::Handled),
                            Some(false) => continue,
                            None => return Ok(SymbolToPrimitiveHandling::NotHandled),
                        }
                    }
                }
            }

            if !self.emit_binding_call_result_to_local_with_explicit_this(
                &step.binding,
                &[],
                expression,
                JS_TYPEOF_OBJECT_TAG,
                result_local,
            )? {
                return Ok(SymbolToPrimitiveHandling::NotHandled);
            }
            match &step.outcome {
                StaticEvalOutcome::Throw(_) => {
                    self.emit_check_global_throw_for_user_call()?;
                    return Ok(SymbolToPrimitiveHandling::AlwaysThrows);
                }
                StaticEvalOutcome::Value(value) => {
                    match self.static_expression_is_non_object_primitive(value) {
                        Some(true) => return Ok(SymbolToPrimitiveHandling::Handled),
                        Some(false) => continue,
                        None => return Ok(SymbolToPrimitiveHandling::NotHandled),
                    }
                }
            }
        }
        self.emit_named_error_throw("TypeError")?;
        Ok(SymbolToPrimitiveHandling::AlwaysThrows)
    }

    fn emit_static_relational_outcome(
        &mut self,
        left: &Expression,
        right: &Expression,
        outcome: &StaticEvalOutcome,
    ) -> DirectResult<()> {
        if matches!(outcome, StaticEvalOutcome::Value(_)) {
            if !self.emit_static_relational_operand_effects(left)?
                || !self.emit_static_relational_operand_effects(right)?
            {
                return Ok(());
            }
        }
        self.emit_static_eval_outcome(outcome)
    }

    fn resolve_static_relational_primitive_after_effects(
        &self,
        expression: &Expression,
    ) -> Option<(Option<OrdinaryToPrimitivePlan>, Expression)> {
        let current_function_name = self.current_function_name();
        if let Some(primitive) =
            self.resolve_static_primitive_expression_with_context(expression, current_function_name)
        {
            return Some((None, primitive));
        }
        if let Some(primitive) = self.resolve_static_boxed_primitive_value(expression) {
            return Some((None, primitive));
        }

        let plan = self.resolve_ordinary_to_primitive_plan(expression)?;
        for step in &plan.steps {
            match &step.outcome {
                StaticEvalOutcome::Throw(_) => return None,
                StaticEvalOutcome::Value(value) => {
                    if let Some(primitive) = self.resolve_static_primitive_expression_with_context(
                        value,
                        current_function_name,
                    ) {
                        return Some((Some(plan), primitive));
                    }
                }
            }
        }
        None
    }

    fn emit_static_relational_operand_plan_effects(
        &mut self,
        expression: &Expression,
        plan: Option<&OrdinaryToPrimitivePlan>,
    ) -> DirectResult<bool> {
        let Some(plan) = plan else {
            return Ok(true);
        };

        let result_local = self.allocate_temp_local();
        match self.emit_ordinary_to_primitive_effects_from_plan(expression, plan, result_local)? {
            SymbolToPrimitiveHandling::Handled => Ok(true),
            SymbolToPrimitiveHandling::AlwaysThrows | SymbolToPrimitiveHandling::NotHandled => {
                Ok(false)
            }
        }
    }

    fn emit_effectful_static_relational_comparison(
        &mut self,
        op: BinaryOp,
        left: &Expression,
        right: &Expression,
    ) -> DirectResult<bool> {
        if !matches!(
            op,
            BinaryOp::LessThan
                | BinaryOp::LessThanOrEqual
                | BinaryOp::GreaterThan
                | BinaryOp::GreaterThanOrEqual
        ) || Self::expression_contains_assignment_or_update(left)
            || Self::expression_contains_assignment_or_update(right)
        {
            return Ok(false);
        }

        let Some((left_plan, left_value)) =
            self.resolve_static_relational_primitive_after_effects(left)
        else {
            return Ok(false);
        };
        let Some((right_plan, right_value)) =
            self.resolve_static_relational_primitive_after_effects(right)
        else {
            return Ok(false);
        };
        let Some(outcome) = self.resolve_static_relational_outcome_with_context(
            op,
            &left_value,
            &right_value,
            self.current_function_name(),
        ) else {
            return Ok(false);
        };
        if !matches!(outcome, StaticEvalOutcome::Value(_)) {
            return Ok(false);
        }

        if !self.emit_static_relational_operand_plan_effects(left, left_plan.as_ref())?
            || !self.emit_static_relational_operand_plan_effects(right, right_plan.as_ref())?
        {
            return Ok(false);
        }
        self.emit_static_eval_outcome(&outcome)?;
        Ok(true)
    }

    fn emit_effectful_static_numeric_binary_outcome(
        &mut self,
        op: BinaryOp,
        left: &Expression,
        right: &Expression,
    ) -> DirectResult<bool> {
        if !matches!(
            op,
            BinaryOp::Subtract
                | BinaryOp::Multiply
                | BinaryOp::Divide
                | BinaryOp::Modulo
                | BinaryOp::BitwiseAnd
                | BinaryOp::BitwiseOr
                | BinaryOp::BitwiseXor
                | BinaryOp::LeftShift
                | BinaryOp::RightShift
                | BinaryOp::UnsignedRightShift
        ) || Self::expression_contains_assignment_or_update(left)
            || Self::expression_contains_assignment_or_update(right)
            || self.expression_depends_on_active_loop_assignment(left)
            || self.expression_depends_on_active_loop_assignment(right)
        {
            return Ok(false);
        }

        let current_function_name = self.current_function_name();
        let left_plan = if self
            .symbol_to_primitive_preempts_ordinary_to_primitive(left, current_function_name)
        {
            None
        } else {
            self.resolve_ordinary_to_primitive_plan(left)
        };
        let right_plan = if self
            .symbol_to_primitive_preempts_ordinary_to_primitive(right, current_function_name)
        {
            None
        } else {
            self.resolve_ordinary_to_primitive_plan(right)
        };
        if left_plan.is_none() && right_plan.is_none() {
            return Ok(false);
        }

        let Some(outcome) = self.resolve_static_numeric_binary_outcome_with_context(
            op,
            left,
            right,
            current_function_name,
        ) else {
            return Ok(false);
        };

        let left_local = self.allocate_temp_local();
        self.emit_numeric_expression(left)?;
        self.push_local_set(left_local);
        self.emit_check_global_throw_for_user_call()?;
        let right_local = self.allocate_temp_local();
        self.emit_conditionally_reachable_numeric_expression_to_local(right, right_local)?;
        self.emit_check_global_throw_for_user_call()?;

        if let Some(plan) = left_plan.as_ref() {
            match self.emit_ordinary_to_primitive_from_plan(left, plan, left_local)? {
                SymbolToPrimitiveHandling::AlwaysThrows => {
                    self.push_i32_const(JS_UNDEFINED_TAG);
                    return Ok(true);
                }
                SymbolToPrimitiveHandling::Handled => {}
                SymbolToPrimitiveHandling::NotHandled => return Ok(false),
            }
        }

        if let Some(plan) = right_plan.as_ref() {
            match self.emit_ordinary_to_primitive_from_plan(right, plan, right_local)? {
                SymbolToPrimitiveHandling::AlwaysThrows => {
                    self.push_i32_const(JS_UNDEFINED_TAG);
                    return Ok(true);
                }
                SymbolToPrimitiveHandling::Handled => {}
                SymbolToPrimitiveHandling::NotHandled => return Ok(false),
            }
        }

        self.emit_static_eval_outcome(&outcome)?;
        Ok(true)
    }

    fn emit_throw_aware_numeric_binary_op(
        &mut self,
        op: BinaryOp,
        left: &Expression,
        right: &Expression,
    ) -> DirectResult<()> {
        let left_local = self.allocate_temp_local();
        let right_local = self.allocate_temp_local();
        self.emit_numeric_expression(left)?;
        self.push_local_set(left_local);
        self.emit_check_global_throw_for_user_call()?;
        self.emit_conditionally_reachable_numeric_expression_to_local(right, right_local)?;
        self.emit_check_global_throw_for_user_call()?;
        if matches!(op, BinaryOp::Add | BinaryOp::Subtract) {
            return self.emit_nan_tag_aware_numeric_binary_op(op, left_local, right_local);
        }
        self.push_local_get(left_local);
        self.push_local_get(right_local);
        self.push_binary_op(op)
    }

    fn emit_nan_tag_aware_numeric_binary_op(
        &mut self,
        op: BinaryOp,
        left_local: u32,
        right_local: u32,
    ) -> DirectResult<()> {
        self.push_local_get(left_local);
        self.push_i32_const(JS_NAN_TAG);
        self.push_binary_op(BinaryOp::Equal)?;
        self.state.emission.output.instructions.push(0x04);
        self.state.emission.output.instructions.push(I32_TYPE);
        self.push_control_frame();
        self.push_i32_const(JS_NAN_TAG);
        self.state.emission.output.instructions.push(0x05);
        self.push_local_get(right_local);
        self.push_i32_const(JS_NAN_TAG);
        self.push_binary_op(BinaryOp::Equal)?;
        self.state.emission.output.instructions.push(0x04);
        self.state.emission.output.instructions.push(I32_TYPE);
        self.push_control_frame();
        self.push_i32_const(JS_NAN_TAG);
        self.state.emission.output.instructions.push(0x05);
        self.push_local_get(left_local);
        self.push_local_get(right_local);
        self.push_binary_op(op)?;
        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();
        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();
        Ok(())
    }

    fn conditional_operand_effect_bindings(&self, expression: &Expression) -> HashSet<String> {
        let mut names = HashSet::new();
        collect_assigned_binding_names_from_expression(expression, &mut names);
        let mut visited = HashSet::new();
        self.collect_expression_call_effect_nonlocal_bindings(
            expression,
            self.current_function_name(),
            &mut names,
            &mut visited,
        );
        self.collect_direct_user_function_call_effect_bindings(
            expression,
            &mut names,
            &mut visited,
        );
        names
    }

    fn collect_direct_user_function_call_effect_bindings(
        &self,
        expression: &Expression,
        names: &mut HashSet<String>,
        visited: &mut HashSet<String>,
    ) {
        match expression {
            Expression::Call { callee, arguments }
            | Expression::New { callee, arguments }
            | Expression::SuperCall { callee, arguments } => {
                if let Expression::Identifier(function_name) = callee.as_ref()
                    && self.user_function(function_name).is_some()
                {
                    names.extend(
                        self.collect_user_function_call_effect_nonlocal_bindings_for_name(
                            function_name,
                            visited,
                        ),
                    );
                }
                self.collect_direct_user_function_call_effect_bindings(callee, names, visited);
                for argument in arguments {
                    match argument {
                        CallArgument::Expression(argument) | CallArgument::Spread(argument) => self
                            .collect_direct_user_function_call_effect_bindings(
                                argument, names, visited,
                            ),
                    }
                }
            }
            Expression::Binary { left, right, .. } => {
                self.collect_direct_user_function_call_effect_bindings(left, names, visited);
                self.collect_direct_user_function_call_effect_bindings(right, names, visited);
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                self.collect_direct_user_function_call_effect_bindings(condition, names, visited);
                self.collect_direct_user_function_call_effect_bindings(
                    then_expression,
                    names,
                    visited,
                );
                self.collect_direct_user_function_call_effect_bindings(
                    else_expression,
                    names,
                    visited,
                );
            }
            Expression::Sequence(expressions) => {
                for expression in expressions {
                    self.collect_direct_user_function_call_effect_bindings(
                        expression, names, visited,
                    );
                }
            }
            Expression::Assign { value, .. }
            | Expression::Await(value)
            | Expression::EnumerateKeys(value)
            | Expression::GetIterator(value)
            | Expression::IteratorClose(value)
            | Expression::Unary {
                expression: value, ..
            } => self.collect_direct_user_function_call_effect_bindings(value, names, visited),
            Expression::AssignMember {
                object,
                property,
                value,
            } => {
                self.collect_direct_user_function_call_effect_bindings(object, names, visited);
                self.collect_direct_user_function_call_effect_bindings(property, names, visited);
                self.collect_direct_user_function_call_effect_bindings(value, names, visited);
            }
            Expression::AssignSuperMember { property, value } => {
                self.collect_direct_user_function_call_effect_bindings(property, names, visited);
                self.collect_direct_user_function_call_effect_bindings(value, names, visited);
            }
            Expression::Member { object, property } => {
                self.collect_direct_user_function_call_effect_bindings(object, names, visited);
                self.collect_direct_user_function_call_effect_bindings(property, names, visited);
            }
            Expression::SuperMember { property } => {
                self.collect_direct_user_function_call_effect_bindings(property, names, visited);
            }
            Expression::Array(elements) => {
                for element in elements {
                    match element {
                        ArrayElement::Expression(element) | ArrayElement::Spread(element) => self
                            .collect_direct_user_function_call_effect_bindings(
                                element, names, visited,
                            ),
                    }
                }
            }
            Expression::Object(entries) => {
                for entry in entries {
                    match entry {
                        ObjectEntry::Data { key, value } => {
                            self.collect_direct_user_function_call_effect_bindings(
                                key, names, visited,
                            );
                            self.collect_direct_user_function_call_effect_bindings(
                                value, names, visited,
                            );
                        }
                        ObjectEntry::Getter { key, getter } => {
                            self.collect_direct_user_function_call_effect_bindings(
                                key, names, visited,
                            );
                            self.collect_direct_user_function_call_effect_bindings(
                                getter, names, visited,
                            );
                        }
                        ObjectEntry::Setter { key, setter } => {
                            self.collect_direct_user_function_call_effect_bindings(
                                key, names, visited,
                            );
                            self.collect_direct_user_function_call_effect_bindings(
                                setter, names, visited,
                            );
                        }
                        ObjectEntry::Spread(value) => {
                            self.collect_direct_user_function_call_effect_bindings(
                                value, names, visited,
                            );
                        }
                    }
                }
            }
            Expression::Identifier(_)
            | Expression::Update { .. }
            | Expression::Number(_)
            | Expression::BigInt(_)
            | Expression::String(_)
            | Expression::Bool(_)
            | Expression::Null
            | Expression::Undefined
            | Expression::NewTarget
            | Expression::Sent
            | Expression::This => {}
        }
    }

    pub(in crate::backend::direct_wasm) fn emit_conditionally_reachable_numeric_expression_to_local(
        &mut self,
        expression: &Expression,
        target_local: u32,
    ) -> DirectResult<()> {
        let invalidated_bindings = self.conditional_operand_effect_bindings(expression);
        self.with_restored_static_binding_metadata(|compiler| {
            compiler.emit_numeric_expression(expression)?;
            compiler.push_local_set(target_local);
            Ok(())
        })?;
        if !invalidated_bindings.is_empty() {
            self.invalidate_static_binding_metadata_for_names(&invalidated_bindings);
        }
        Ok(())
    }

    pub(in crate::backend::direct_wasm) fn emit_binary_expression_value(
        &mut self,
        expression: &Expression,
        op: BinaryOp,
        left: &Expression,
        right: &Expression,
    ) -> DirectResult<()> {
        let arithmetic_requires_runtime_value = self.has_current_user_function()
            && (self.addition_operand_requires_runtime_value(left)
                || self.addition_operand_requires_runtime_value(right));
        let numeric_static_outcome_operands_are_side_effect_free = self
            .numeric_static_outcome_operand_is_side_effect_free(left)
            && self.numeric_static_outcome_operand_is_side_effect_free(right);
        if self.emit_effectful_static_numeric_binary_outcome(op, left, right)? {
            return Ok(());
        }
        if matches!(
            op,
            BinaryOp::Subtract
                | BinaryOp::Multiply
                | BinaryOp::Divide
                | BinaryOp::Modulo
                | BinaryOp::Exponentiate
                | BinaryOp::BitwiseAnd
                | BinaryOp::BitwiseOr
                | BinaryOp::BitwiseXor
                | BinaryOp::LeftShift
                | BinaryOp::RightShift
                | BinaryOp::UnsignedRightShift
        ) && self.emit_effectful_ordinary_to_primitive_numeric(left, right)?
        {
            return Ok(());
        }
        if (!arithmetic_requires_runtime_value
            || numeric_static_outcome_operands_are_side_effect_free)
            && !Self::expression_contains_assignment_or_update(left)
            && !Self::expression_contains_assignment_or_update(right)
            && !self.binary_expression_calls_user_function(left)
            && !self.binary_expression_calls_user_function(right)
            && matches!(
                op,
                BinaryOp::Subtract
                    | BinaryOp::Multiply
                    | BinaryOp::Divide
                    | BinaryOp::Modulo
                    | BinaryOp::Exponentiate
                    | BinaryOp::BitwiseAnd
                    | BinaryOp::BitwiseOr
                    | BinaryOp::BitwiseXor
                    | BinaryOp::LeftShift
                    | BinaryOp::RightShift
                    | BinaryOp::UnsignedRightShift
            )
            && let Some(outcome) = self.resolve_static_numeric_binary_outcome_with_context(
                op,
                left,
                right,
                self.current_function_name(),
            )
        {
            return self.emit_static_eval_outcome(&outcome);
        }
        if !arithmetic_requires_runtime_value
            && !Self::expression_contains_assignment_or_update(left)
            && !Self::expression_contains_assignment_or_update(right)
            && !self.binary_expression_calls_user_function(expression)
            && matches!(
                op,
                BinaryOp::Subtract
                    | BinaryOp::Multiply
                    | BinaryOp::Divide
                    | BinaryOp::Modulo
                    | BinaryOp::Exponentiate
                    | BinaryOp::BitwiseAnd
                    | BinaryOp::BitwiseOr
                    | BinaryOp::BitwiseXor
                    | BinaryOp::LeftShift
                    | BinaryOp::RightShift
                    | BinaryOp::UnsignedRightShift
            )
            && let Some(number) = self.resolve_static_number_value(expression)
        {
            return self.emit_numeric_expression(&Expression::Number(number));
        }
        let equality_depends_on_active_loop_assignment =
            !self.state.emission.control_flow.loop_stack.is_empty()
                || self.expression_depends_on_active_loop_assignment(left)
                || self.expression_depends_on_active_loop_assignment(right);
        let equality_references_internal_iterator_step =
            binary_expression_references_internal_iterator_step(left)
                || binary_expression_references_internal_iterator_step(right);
        let equality_reads_runtime_nonlocal_binding = self
            .binary_expression_reads_runtime_nonlocal_binding(left)
            || self.binary_expression_reads_runtime_nonlocal_binding(right);
        if equality_references_internal_iterator_step
            && matches!(op, BinaryOp::Equal | BinaryOp::NotEqual)
        {
            return self.emit_throw_aware_numeric_binary_op(op, left, right);
        }
        if matches!(op, BinaryOp::Equal | BinaryOp::NotEqual)
            && (Self::expression_references_internal_assignment_temp(left)
                || Self::expression_references_internal_assignment_temp(right))
        {
            return self.emit_throw_aware_numeric_binary_op(op, left, right);
        }
        if !equality_depends_on_active_loop_assignment
            && !equality_references_internal_iterator_step
            && !equality_reads_runtime_nonlocal_binding
            && !Self::expression_references_internal_assignment_temp(left)
            && !Self::expression_references_internal_assignment_temp(right)
            && !Self::expression_contains_assignment_or_update(left)
            && !Self::expression_contains_assignment_or_update(right)
            && !self.binary_expression_calls_user_function(left)
            && !self.binary_expression_calls_user_function(right)
            && matches!(op, BinaryOp::Equal | BinaryOp::NotEqual)
            && !Self::expression_is_relational_binary(left)
            && !Self::expression_is_relational_binary(right)
            && !self.expression_has_dynamic_member_property_access(left)
            && !self.expression_has_dynamic_member_property_access(right)
            && let Some(value) = self.resolve_static_binary_boolean_result(&op, left, right)
        {
            if !inline_summary_side_effect_free_expression(left) {
                self.emit_numeric_expression(left)?;
                self.state.emission.output.instructions.push(0x1a);
            }
            if !inline_summary_side_effect_free_expression(right) {
                self.emit_numeric_expression(right)?;
                self.state.emission.output.instructions.push(0x1a);
            }
            return self.emit_literal_expression(&Expression::Bool(value));
        }
        if self.emit_static_strict_type_mismatch_comparison(op, left, right)? {
            return Ok(());
        }
        if self.emit_stringified_division_split_length_comparison(op, left, right)? {
            return Ok(());
        }
        if matches!(
            op,
            BinaryOp::LessThan
                | BinaryOp::LessThanOrEqual
                | BinaryOp::GreaterThan
                | BinaryOp::GreaterThanOrEqual
        ) && !Self::expression_contains_assignment_or_update(left)
            && !Self::expression_contains_assignment_or_update(right)
            && !self.binary_expression_calls_user_function(left)
            && !self.binary_expression_calls_user_function(right)
            && let Some(outcome) = self.resolve_static_relational_outcome_with_context(
                op,
                left,
                right,
                self.current_function_name(),
            )
        {
            return self.emit_static_relational_outcome(left, right, &outcome);
        }
        if self.emit_effectful_static_relational_comparison(op, left, right)? {
            return Ok(());
        }
        if self.emit_fractional_static_relational_comparison(op, left, right)? {
            return Ok(());
        }
        if matches!(op, BinaryOp::LooseEqual | BinaryOp::LooseNotEqual)
            && self.emit_effectful_ordinary_to_primitive_numeric(left, right)?
        {
            return Ok(());
        }
        if matches!(op, BinaryOp::Divide | BinaryOp::Modulo) {
            return self.emit_safe_integer_division_or_modulo(op, left, right);
        }
        match op {
            BinaryOp::Add => {
                if self.emit_static_template_update_addition(expression)? {
                    return Ok(());
                }
                let addition_depends_on_active_loop_assignment = self
                    .expression_depends_on_active_loop_assignment(left)
                    || self.expression_depends_on_active_loop_assignment(right);
                let addition_contains_assignment_or_update =
                    Self::expression_contains_assignment_or_update(left)
                        || Self::expression_contains_assignment_or_update(right);
                let addition_calls_user_function = self.binary_expression_calls_user_function(left)
                    || self.binary_expression_calls_user_function(right);
                let addition_operand_side_effect_free = |operand: &Expression| {
                    !self.binary_expression_calls_user_function(operand)
                        && (inline_summary_side_effect_free_expression(operand)
                            || self.resolve_static_boxed_primitive_value(operand).is_some())
                };
                let addition_operands_side_effect_free = addition_operand_side_effect_free(left)
                    && addition_operand_side_effect_free(right);
                let addition_requires_runtime_value = self.has_current_user_function()
                    && (self.addition_operand_requires_runtime_value(left)
                        || self.addition_operand_requires_runtime_value(right));
                let allow_static_addition = !addition_requires_runtime_value
                    && !addition_depends_on_active_loop_assignment
                    && !addition_contains_assignment_or_update
                    && !addition_calls_user_function
                    && addition_operands_side_effect_free;
                if allow_static_addition
                    && let Some(outcome) = self.resolve_static_addition_outcome_with_context(
                        left,
                        right,
                        self.current_function_name(),
                    )
                {
                    return self.emit_static_eval_outcome(&outcome);
                }
                if !addition_depends_on_active_loop_assignment
                    && !addition_contains_assignment_or_update
                    && !addition_calls_user_function
                    && addition_operands_side_effect_free
                    && !addition_requires_runtime_value
                    && let Some(text) = self.resolve_static_string_addition_value_with_context(
                        left,
                        right,
                        self.current_function_name(),
                    )
                {
                    self.emit_static_string_literal(&text)?;
                    return Ok(());
                }
                if self.emit_effectful_symbol_to_primitive_addition(left, right)? {
                    return Ok(());
                }
                if self.emit_effectful_ordinary_to_primitive_addition(left, right)? {
                    return Ok(());
                }
                if self.emit_active_loop_string_expression_from_sequence(expression)? {
                    return Ok(());
                }
                let addition_operands_are_definitely_numeric = self.infer_value_kind(left)
                    == Some(StaticValueKind::Number)
                    && self.infer_value_kind(right) == Some(StaticValueKind::Number);
                if !addition_operands_are_definitely_numeric
                    && self.emit_runtime_string_addition_from_candidates(left, right)?
                {
                    return Ok(());
                }
                self.emit_throw_aware_numeric_binary_op(op, left, right)
            }
            BinaryOp::LogicalAnd => self.emit_logical_and(left, right),
            BinaryOp::LogicalOr => self.emit_logical_or(left, right),
            BinaryOp::NullishCoalescing => self.emit_nullish_coalescing(left, right),
            BinaryOp::Exponentiate => self.emit_exponentiate(left, right),
            BinaryOp::Equal | BinaryOp::NotEqual
                if self.expression_has_dynamic_member_property_access(left)
                    || self.expression_has_dynamic_member_property_access(right) =>
            {
                self.emit_throw_aware_numeric_binary_op(op, left, right)
            }
            BinaryOp::Equal | BinaryOp::NotEqual
                if self.emit_static_string_equality_comparison(left, right, op)? =>
            {
                Ok(())
            }
            BinaryOp::Equal | BinaryOp::NotEqual
                if self.emit_typeof_string_comparison(left, right, op)? =>
            {
                Ok(())
            }
            BinaryOp::Equal | BinaryOp::NotEqual
                if self.emit_runtime_typeof_tag_string_comparison(left, right, op)? =>
            {
                Ok(())
            }
            BinaryOp::Equal | BinaryOp::NotEqual
                if self.emit_runtime_static_string_equality_comparison(left, right, op)? =>
            {
                Ok(())
            }
            BinaryOp::Equal | BinaryOp::NotEqual
                if self.emit_hex_quad_string_comparison(left, right, op)? =>
            {
                Ok(())
            }
            BinaryOp::LooseEqual | BinaryOp::LooseNotEqual
                if self.emit_static_string_equality_comparison(left, right, op)? =>
            {
                Ok(())
            }
            BinaryOp::LooseEqual | BinaryOp::LooseNotEqual
                if self.emit_typeof_string_comparison(left, right, op)? =>
            {
                Ok(())
            }
            BinaryOp::LooseEqual | BinaryOp::LooseNotEqual
                if self.emit_runtime_typeof_tag_string_comparison(left, right, op)? =>
            {
                Ok(())
            }
            BinaryOp::LooseEqual | BinaryOp::LooseNotEqual
                if self.emit_runtime_static_string_equality_comparison(left, right, op)? =>
            {
                Ok(())
            }
            BinaryOp::LooseEqual | BinaryOp::LooseNotEqual
                if self.emit_hex_quad_string_comparison(left, right, op)? =>
            {
                Ok(())
            }
            BinaryOp::LooseEqual | BinaryOp::LooseNotEqual
                if self.emit_static_bigint_non_finite_loose_equality(op, left, right)? =>
            {
                Ok(())
            }
            BinaryOp::LooseEqual | BinaryOp::LooseNotEqual
                if self.emit_effectful_symbol_to_primitive_loose_equality(op, left, right)? =>
            {
                Ok(())
            }
            BinaryOp::LooseEqual => {
                self.emit_loose_comparison(left, right)?;
                self.state.emission.output.instructions.push(0x46);
                Ok(())
            }
            BinaryOp::LooseNotEqual => {
                self.emit_loose_comparison(left, right)?;
                self.state.emission.output.instructions.push(0x47);
                Ok(())
            }
            BinaryOp::In => {
                self.emit_in_expression(left, right)?;
                Ok(())
            }
            BinaryOp::InstanceOf => {
                self.emit_instanceof_expression(left, right)?;
                Ok(())
            }
            _ => self.emit_throw_aware_numeric_binary_op(op, left, right),
        }
    }

    pub(in crate::backend::direct_wasm) fn emit_conditional_expression_value(
        &mut self,
        condition: &Expression,
        then_expression: &Expression,
        else_expression: &Expression,
    ) -> DirectResult<()> {
        let trace_conditional = std::env::var_os("AYY_TRACE_CONDITIONAL").is_some();
        if trace_conditional {
            eprintln!(
                "conditional_emit:start condition={condition:?} then={then_expression:?} else={else_expression:?}"
            );
        }
        if trace_conditional {
            eprintln!("conditional_emit:condition:start");
        }
        self.emit_truthy_expression(condition)?;
        if trace_conditional {
            eprintln!("conditional_emit:condition:done");
        }
        self.state.emission.output.instructions.push(0x04);
        self.state.emission.output.instructions.push(I32_TYPE);
        self.push_control_frame();
        if trace_conditional {
            eprintln!("conditional_emit:then:start");
        }
        self.emit_numeric_expression(then_expression)?;
        if trace_conditional {
            eprintln!("conditional_emit:then:done");
        }
        self.state.emission.output.instructions.push(0x05);
        if trace_conditional {
            eprintln!("conditional_emit:else:start");
        }
        self.emit_numeric_expression(else_expression)?;
        if trace_conditional {
            eprintln!("conditional_emit:else:done");
        }
        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();
        if trace_conditional {
            eprintln!("conditional_emit:done");
        }
        Ok(())
    }

    fn sequence_mutated_name_matches_source(mutated_names: &HashSet<String>, source: &str) -> bool {
        mutated_names.contains(source)
            || scoped_binding_source_name(source)
                .is_some_and(|source_name| mutated_names.contains(source_name))
            || mutated_names.iter().any(|name| {
                scoped_binding_source_name(name).is_some_and(|source_name| source_name == source)
            })
    }

    fn sequence_runtime_array_length_local_for_target(&self, target_name: &str) -> Option<u32> {
        self.state
            .speculation
            .static_semantics
            .runtime_array_length_local(target_name)
            .or_else(|| {
                self.resolve_runtime_array_binding_name(target_name)
                    .and_then(|binding_name| {
                        self.state
                            .speculation
                            .static_semantics
                            .runtime_array_length_local(&binding_name)
                    })
            })
    }

    fn sequence_created_capture_slot_syncs(
        &self,
        sequence_initial_capture_slots: &HashSet<String>,
        mutated_names: &HashSet<String>,
    ) -> Vec<(String, String, u32, u32)> {
        if mutated_names.is_empty() {
            return Vec::new();
        }

        let mut syncs = Vec::new();
        let capture_slot_sources = &self
            .state
            .speculation
            .static_semantics
            .capture_slot_source_bindings;
        for (key, capture_slots) in &self
            .state
            .speculation
            .static_semantics
            .objects
            .member_function_capture_slots
        {
            let MemberFunctionBindingTarget::Identifier(target_name) = &key.target else {
                continue;
            };
            let MemberFunctionBindingProperty::String(property_name) = &key.property else {
                continue;
            };
            let Ok(index) = property_name.parse::<u32>() else {
                continue;
            };
            let Some(length_local) =
                self.sequence_runtime_array_length_local_for_target(target_name)
            else {
                continue;
            };

            for slot_name in capture_slots.values() {
                if sequence_initial_capture_slots.contains(slot_name)
                    || self.state.runtime.locals.get(slot_name).is_none()
                    || syncs
                        .iter()
                        .any(|(existing_slot, _, _, _)| existing_slot == slot_name)
                {
                    continue;
                }
                let Some(source_name) = capture_slot_sources.get(slot_name) else {
                    continue;
                };
                if !Self::sequence_mutated_name_matches_source(mutated_names, source_name) {
                    continue;
                }
                syncs.push((
                    slot_name.clone(),
                    source_name.clone(),
                    length_local,
                    index.saturating_add(1),
                ));
            }
        }
        syncs
    }

    fn emit_sequence_created_capture_slot_syncs(
        &mut self,
        syncs: &[(String, String, u32, u32)],
    ) -> DirectResult<()> {
        for (slot_name, source_name, length_local, expected_length) in syncs {
            let Some(slot_local) = self.state.runtime.locals.get(slot_name).copied() else {
                continue;
            };
            let source_expression = Expression::Identifier(source_name.clone());
            self.push_local_get(*length_local);
            self.push_i32_const(*expected_length as i32);
            self.push_binary_op(BinaryOp::Equal)?;
            self.state.emission.output.instructions.push(0x04);
            self.state
                .emission
                .output
                .instructions
                .push(EMPTY_BLOCK_TYPE);
            self.push_control_frame();
            self.emit_numeric_expression(&source_expression)?;
            self.push_local_set(slot_local);
            self.update_capture_slot_binding_from_expression(slot_name, &source_expression)?;
            self.sync_capture_slot_runtime_object_shadows_from_expression(
                slot_name,
                &source_expression,
            )?;
            self.state
                .speculation
                .static_semantics
                .capture_slot_source_bindings
                .insert(slot_name.clone(), source_name.clone());
            self.state.emission.output.instructions.push(0x0b);
            self.pop_control_frame();
        }
        Ok(())
    }

    fn sequence_created_capture_slot_syncs_for_expression(
        &self,
        sequence_initial_capture_slots: &HashSet<String>,
        expression: &Expression,
    ) -> Vec<(String, String, u32, u32)> {
        let mut mutated_names = HashSet::new();
        collect_assigned_binding_names_from_expression(expression, &mut mutated_names);
        self.sequence_created_capture_slot_syncs(sequence_initial_capture_slots, &mutated_names)
    }

    pub(in crate::backend::direct_wasm) fn emit_sequence_expression_value(
        &mut self,
        expressions: &[Expression],
    ) -> DirectResult<()> {
        let Some((last, rest)) = expressions.split_last() else {
            self.push_i32_const(JS_UNDEFINED_TAG);
            return Ok(());
        };
        let sequence_initial_capture_slots = self
            .state
            .speculation
            .static_semantics
            .capture_slot_source_bindings
            .keys()
            .cloned()
            .collect::<HashSet<_>>();
        for expression in rest {
            let syncs = self.sequence_created_capture_slot_syncs_for_expression(
                &sequence_initial_capture_slots,
                expression,
            );
            self.emit_numeric_expression(expression)?;
            self.state.emission.output.instructions.push(0x1a);
            self.emit_sequence_created_capture_slot_syncs(&syncs)?;
        }
        let syncs = self.sequence_created_capture_slot_syncs_for_expression(
            &sequence_initial_capture_slots,
            last,
        );
        self.emit_numeric_expression(last)?;
        if !syncs.is_empty() {
            let result_local = self.allocate_temp_local();
            self.push_local_set(result_local);
            self.emit_sequence_created_capture_slot_syncs(&syncs)?;
            self.push_local_get(result_local);
        }
        Ok(())
    }
}
