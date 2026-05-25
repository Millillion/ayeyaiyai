use super::*;

impl<'a> FunctionCompiler<'a> {
    fn is_tracked_array_step_binding_name(name: &str) -> bool {
        name.starts_with("__ayy_array_step_") || name.starts_with("__ayy_for_of_step_")
    }

    fn is_private_brand_binding_initializer(&self, name: &str, value: &Expression) -> bool {
        name.starts_with("__ayy_class_brand_")
            && matches!(value, Expression::Object(entries) if entries.is_empty())
    }

    fn emit_fresh_private_brand_value(&mut self) -> DirectResult<()> {
        let brand_local = self.allocate_temp_local();
        self.push_global_get(NEXT_PRIVATE_BRAND_GLOBAL_INDEX);
        self.push_local_set(brand_local);
        self.push_local_get(brand_local);
        self.push_i32_const(1);
        self.push_binary_op(BinaryOp::Add)?;
        self.push_global_set(NEXT_PRIVATE_BRAND_GLOBAL_INDEX);
        self.push_local_get(brand_local);
        Ok(())
    }

    fn tracked_array_step_initializer_parts<'b>(
        &self,
        name: &str,
        value: &'b Expression,
    ) -> Option<(
        &'b Expression,
        &'b Expression,
        &'b Expression,
        &'b [CallArgument],
    )> {
        if !Self::is_tracked_array_step_binding_name(name) {
            return None;
        }
        let Expression::Call { callee, arguments } = value else {
            return None;
        };
        if !arguments.is_empty() {
            return None;
        }
        let Expression::Member { object, property } = callee.as_ref() else {
            return None;
        };
        if !matches!(property.as_ref(), Expression::String(property_name) if property_name == "next")
        {
            return None;
        }
        let Expression::Identifier(iterator_name) = object.as_ref() else {
            return None;
        };
        let iterator_binding_name = self
            .resolve_local_array_iterator_binding_name(iterator_name)
            .unwrap_or_else(|| iterator_name.clone());
        self.state
            .speculation
            .static_semantics
            .local_array_iterator_binding(&iterator_binding_name)
            .is_some()
            .then_some((
                callee.as_ref(),
                object.as_ref(),
                property.as_ref(),
                arguments,
            ))
    }

    fn has_static_tracked_array_step_initializer(&self, name: &str, value: &Expression) -> bool {
        let Some((_, object, _, _)) = self.tracked_array_step_initializer_parts(name, value) else {
            return false;
        };
        let Expression::Identifier(iterator_name) = object else {
            return false;
        };
        let iterator_binding_name = self
            .resolve_local_array_iterator_binding_name(iterator_name)
            .unwrap_or_else(|| iterator_name.clone());
        if self
            .state
            .emission
            .control_flow
            .loop_stack
            .iter()
            .rev()
            .any(|loop_context| {
                loop_context.direct_step_iterators.contains(iterator_name)
                    || loop_context
                        .direct_step_iterators
                        .contains(&iterator_binding_name)
            })
        {
            return false;
        }
        let Some(iterator_binding) = self
            .state
            .speculation
            .static_semantics
            .local_array_iterator_binding(&iterator_binding_name)
        else {
            return false;
        };
        matches!(
            iterator_binding.source,
            IteratorSourceKind::StaticArray {
                length_local: None,
                runtime_name: None,
                ..
            } | IteratorSourceKind::SimpleGenerator { .. }
        )
    }

    fn emit_static_tracked_array_step_binding_if_possible(
        &mut self,
        statement: &Statement,
    ) -> DirectResult<bool> {
        let static_step = match statement {
            Statement::Let { name, value, .. }
            | Statement::Var { name, value }
            | Statement::Assign { name, value } => {
                self.has_static_tracked_array_step_initializer(name, value)
                    && self
                        .tracked_array_step_initializer_parts(name, value)
                        .and_then(|(_, object, _, _)| match object {
                            Expression::Identifier(iterator_name) => {
                                let iterator_binding_name = self
                                    .resolve_local_array_iterator_binding_name(iterator_name)
                                    .unwrap_or_else(|| iterator_name.clone());
                                self.state
                                    .speculation
                                    .static_semantics
                                    .local_array_iterator_binding(&iterator_binding_name)
                            }
                            _ => None,
                        })
                        .is_some_and(|iterator_binding| {
                            matches!(
                                iterator_binding.source,
                                IteratorSourceKind::SimpleGenerator { .. }
                            )
                        })
            }
            _ => false,
        };
        if !static_step {
            return Ok(false);
        }
        self.try_emit_static_simple_generator_binding_effect(statement, &[])
    }

    fn direct_static_class_constructor_new_expression(
        &self,
        expression: &Expression,
        current_function_name: Option<&str>,
    ) -> Option<Expression> {
        let Expression::New { callee, arguments } = expression else {
            return None;
        };
        if !arguments.is_empty() {
            return None;
        }

        let constructor_name = match callee.as_ref() {
            Expression::Identifier(function_name)
                if function_name.starts_with("__ayy_class_ctor_") =>
            {
                Some(function_name.clone())
            }
            _ => match self.resolve_function_binding_from_expression_with_context(
                callee,
                current_function_name,
            ) {
                Some(LocalFunctionBinding::User(function_name))
                    if function_name.starts_with("__ayy_class_ctor_") =>
                {
                    Some(function_name)
                }
                _ => None,
            },
        }?;

        Some(Expression::New {
            callee: Box::new(Expression::Identifier(constructor_name)),
            arguments: Vec::new(),
        })
    }

    fn is_single_return_new_function_body(body: &[Statement]) -> bool {
        matches!(body, [Statement::Return(Expression::New { .. })])
    }

    fn static_class_constructor_call_initializer_result(
        &self,
        value: &Expression,
    ) -> Option<Expression> {
        let Expression::Call { callee, arguments } = value else {
            return None;
        };
        let LocalFunctionBinding::User(function_name) =
            self.resolve_function_binding_from_expression(callee)?
        else {
            return None;
        };
        let declaration = self.prepared_function_declaration(&function_name)?;
        if !Self::is_single_return_new_function_body(&declaration.body) {
            return None;
        }
        let (result, result_function_name) = self
            .resolve_static_call_result_expression_with_context(
                callee,
                arguments,
                self.current_function_name(),
            )?;
        self.direct_static_class_constructor_new_expression(
            &result,
            result_function_name
                .as_deref()
                .or_else(|| self.current_function_name()),
        )
    }

    fn lowered_optional_member_non_nullish_target(
        &self,
        expression: &Expression,
    ) -> Option<Expression> {
        let Expression::Sequence(expressions) = expression else {
            return None;
        };
        let Expression::Conditional {
            then_expression,
            else_expression,
            ..
        } = expressions.last()?
        else {
            return None;
        };
        if !matches!(then_expression.as_ref(), Expression::Undefined) {
            return None;
        }
        let Expression::Member { object, property } = else_expression.as_ref() else {
            return None;
        };
        let Expression::Identifier(temp_name) = object.as_ref() else {
            return None;
        };
        let base = expressions
            .iter()
            .rev()
            .find_map(|expression| match expression {
                Expression::Assign { name, value } if name == temp_name => {
                    Some(value.as_ref().clone())
                }
                _ => None,
            })?;
        Some(Expression::Member {
            object: Box::new(base),
            property: property.clone(),
        })
    }

    fn for_await_iterator_initializer_source(
        &self,
        name: &str,
        value: &Expression,
    ) -> Option<Expression> {
        if !name.starts_with("__ayy_for_await_iter_") {
            return None;
        }
        let Expression::GetIterator(source) = value else {
            return None;
        };
        Some(
            self.lowered_optional_member_non_nullish_target(source)
                .unwrap_or_else(|| source.as_ref().clone()),
        )
    }

    fn has_static_async_iterator_property(
        &self,
        source: &Expression,
        async_iterator_property: &Expression,
    ) -> bool {
        self.resolve_member_getter_binding(source, async_iterator_property)
            .is_some()
            || self
                .resolve_member_function_binding(source, async_iterator_property)
                .is_some()
            || self
                .resolve_object_binding_from_expression(source)
                .and_then(|binding| {
                    object_binding_lookup_value(&binding, async_iterator_property).cloned()
                })
                .is_some_and(|value| !matches!(value, Expression::Undefined | Expression::Null))
    }

    fn static_for_await_iterator_initializer_result(
        &self,
        name: &str,
        value: &Expression,
    ) -> Option<Expression> {
        let source = self.for_await_iterator_initializer_source(name, value)?;
        let async_iterator_property = self.materialize_static_expression(&Expression::Member {
            object: Box::new(Expression::Identifier("Symbol".to_string())),
            property: Box::new(Expression::String("asyncIterator".to_string())),
        });
        if !self.has_static_async_iterator_property(&source, &async_iterator_property) {
            return None;
        }
        Some(Expression::Call {
            callee: Box::new(Expression::Member {
                object: Box::new(source),
                property: Box::new(async_iterator_property),
            }),
            arguments: Vec::new(),
        })
    }

    fn emit_binding_initializer_value(
        &mut self,
        name: &str,
        value: &Expression,
    ) -> DirectResult<()> {
        if self.is_private_brand_binding_initializer(name, value) {
            return self.emit_fresh_private_brand_value();
        }
        if !matches!(
            value,
            Expression::Call { .. } | Expression::New { .. } | Expression::SuperCall { .. }
        ) && !Self::expression_contains_assignment_or_update(value)
            && inline_summary_side_effect_free_expression(value)
            && let Some(LocalFunctionBinding::User(function_name)) =
                self.resolve_function_binding_from_expression(value)
            && let Some(runtime_value) = self.user_function_runtime_value(&function_name)
        {
            self.push_i32_const(runtime_value);
            return Ok(());
        }
        if let Some(resolved_value) = self.static_for_await_iterator_initializer_result(name, value)
        {
            return self.emit_numeric_expression(&resolved_value);
        }
        if let Some(resolved_value) = self.static_class_constructor_call_initializer_result(value) {
            return self.emit_numeric_expression(&resolved_value);
        }
        if let Some((_, object, property, arguments)) =
            self.tracked_array_step_initializer_parts(name, value)
            && self
                .state
                .speculation
                .static_semantics
                .local_iterator_step_binding(name)
                .is_some()
            && self.has_static_tracked_array_step_initializer(name, value)
            && self
                .captured_iterator_next_method_plan(object, property, arguments)
                .is_none()
        {
            self.push_i32_const(JS_TYPEOF_OBJECT_TAG);
            return Ok(());
        }
        self.emit_numeric_expression(value)
    }

    fn emit_scoped_compound_assignment_value(
        &mut self,
        scope_object: &Expression,
        name: &str,
        value: &Expression,
    ) -> DirectResult<Option<Expression>> {
        let Expression::Binary { op, left, right } = value else {
            return Ok(None);
        };
        if !matches!(left.as_ref(), Expression::Identifier(left_name) if left_name == name) {
            return Ok(None);
        }

        let property = Expression::String(name.to_string());
        let previous_value =
            self.resolve_scoped_compound_assignment_previous_value(scope_object, &property);
        self.emit_scoped_property_read(scope_object, name)?;
        self.emit_numeric_expression(right)?;
        self.push_binary_op(*op)?;
        Ok(Some(
            previous_value
                .map(|previous_value| {
                    self.scoped_compound_assignment_static_store_value(*op, previous_value, right)
                })
                .unwrap_or_else(|| value.clone()),
        ))
    }

    fn resolve_scoped_compound_assignment_previous_value(
        &self,
        scope_object: &Expression,
        property: &Expression,
    ) -> Option<Expression> {
        if let Some(getter_binding) = self.resolve_member_getter_binding(scope_object, property)
            && let Some(value) = self.resolve_static_getter_value_from_binding_with_context(
                &getter_binding,
                scope_object,
                self.current_function_name(),
            )
        {
            return Some(value);
        }

        if let Some(object_binding) = self.resolve_object_binding_from_expression(scope_object)
            && let Some(value) = object_binding_lookup_value(&object_binding, property)
        {
            return Some(value.clone());
        }

        let member = Expression::Member {
            object: Box::new(scope_object.clone()),
            property: Box::new(property.clone()),
        };
        let materialized = self.materialize_static_expression(&member);
        (!static_expression_matches(&materialized, &member)).then_some(materialized)
    }

    fn emit_identifier_reference_target_read(
        &mut self,
        name: &str,
        resolved_local_binding: Option<&(String, u32)>,
        capture_binding: bool,
        declared_global_index: Option<u32>,
        eval_local_binding: bool,
        implicit_global_binding: Option<ImplicitGlobalBinding>,
        unresolvable_reference: bool,
    ) -> DirectResult<()> {
        if let Some((resolved_name, local_index)) = resolved_local_binding {
            if let Some(initialized_local) = self.local_lexical_initialized_local(resolved_name) {
                self.push_local_get(initialized_local);
                self.state.emission.output.instructions.push(0x04);
                self.state.emission.output.instructions.push(I32_TYPE);
                self.push_control_frame();
                self.push_local_get(*local_index);
                self.state.emission.output.instructions.push(0x05);
                self.emit_named_error_throw("ReferenceError")?;
                self.state.emission.output.instructions.push(0x0b);
                self.pop_control_frame();
            } else {
                self.push_local_get(*local_index);
            }
            return Ok(());
        }

        if capture_binding {
            if !self.emit_user_function_capture_binding_read(name)? {
                self.emit_named_error_throw("ReferenceError")?;
            }
            return Ok(());
        }

        if let Some(global_index) = declared_global_index {
            return self.emit_declared_global_binding_read(name, global_index);
        }

        if eval_local_binding {
            if !self.emit_eval_local_function_binding_read(name)? {
                self.emit_named_error_throw("ReferenceError")?;
            }
            return Ok(());
        }

        if let Some(binding) = implicit_global_binding {
            self.push_global_get(binding.present_index);
            self.state.emission.output.instructions.push(0x04);
            self.state.emission.output.instructions.push(I32_TYPE);
            self.push_control_frame();
            self.push_global_get(binding.value_index);
            self.state.emission.output.instructions.push(0x05);
            self.emit_named_error_throw("ReferenceError")?;
            self.state.emission.output.instructions.push(0x0b);
            self.pop_control_frame();
            return Ok(());
        }

        if unresolvable_reference {
            self.emit_named_error_throw("ReferenceError")?;
            return Ok(());
        }

        self.emit_plain_identifier_read(name)
    }

    fn emit_identifier_compound_assignment_value(
        &mut self,
        name: &str,
        value: &Expression,
        resolved_local_binding: Option<&(String, u32)>,
        capture_binding: bool,
        declared_global_index: Option<u32>,
        eval_local_binding: bool,
        implicit_global_binding: Option<ImplicitGlobalBinding>,
        unresolvable_reference: bool,
    ) -> DirectResult<Option<Expression>> {
        let Expression::Binary { op, left, right } = value else {
            return Ok(None);
        };
        if !matches!(left.as_ref(), Expression::Identifier(left_name) if left_name == name) {
            return Ok(None);
        }
        if !self.assignment_value_declares_static_direct_eval_var_binding(name, value) {
            return Ok(None);
        }

        self.emit_identifier_reference_target_read(
            name,
            resolved_local_binding,
            capture_binding,
            declared_global_index,
            eval_local_binding,
            implicit_global_binding,
            unresolvable_reference,
        )?;
        self.emit_numeric_expression(right)?;
        self.push_binary_op(*op)?;
        Ok(Some(value.clone()))
    }

    fn scoped_compound_assignment_static_store_value(
        &self,
        op: BinaryOp,
        previous_value: Expression,
        right: &Expression,
    ) -> Expression {
        if op == BinaryOp::Add
            && let Some(StaticEvalOutcome::Value(value)) = self
                .resolve_static_addition_outcome_with_context(
                    &previous_value,
                    right,
                    self.current_function_name(),
                )
        {
            return self.materialize_static_expression(&value);
        }

        let computed = Expression::Binary {
            op,
            left: Box::new(previous_value),
            right: Box::new(right.clone()),
        };
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
        ) && let Some(value) = self.resolve_static_number_value(&computed)
        {
            return Expression::Number(value);
        }
        self.materialize_static_expression(&computed)
    }

    pub(super) fn emit_binding_statement(&mut self, statement: &Statement) -> DirectResult<()> {
        if self.emit_static_tracked_array_step_binding_if_possible(statement)? {
            return Ok(());
        }
        match statement {
            Statement::Var { name, value } => {
                if matches!(value, Expression::Undefined) {
                    if !self.state.emission.emitted_value_bindings.contains(name) {
                        if self.global_has_binding(name) || self.global_has_implicit_binding(name) {
                            self.update_static_global_assignment_metadata(
                                name,
                                &Expression::Undefined,
                            );
                        } else if let Some((resolved_name, local_index)) =
                            self.resolve_current_local_binding(name)
                        {
                            let preserves_existing_local = local_index
                                < self.state.parameters.param_count
                                || self
                                    .state
                                    .speculation
                                    .static_semantics
                                    .local_value_binding(&resolved_name)
                                    .is_some();
                            if preserves_existing_local {
                                return Ok(());
                            }
                            self.update_local_value_binding(&resolved_name, &Expression::Undefined);
                            self.state
                                .speculation
                                .static_semantics
                                .set_local_kind(&resolved_name, StaticValueKind::Undefined);
                        } else {
                            self.update_local_value_binding(name, &Expression::Undefined);
                            self.state
                                .speculation
                                .static_semantics
                                .set_local_kind(name, StaticValueKind::Undefined);
                        }
                    }
                    return Ok(());
                }
                let trace = std::env::var_os("AYY_TRACE_FUNCTION_COMPILE").is_some();
                let value_local = self.allocate_temp_local();
                let resolved_store_value = self
                    .static_for_await_iterator_initializer_result(name, value)
                    .or_else(|| self.static_class_constructor_call_initializer_result(value));
                let store_value = resolved_store_value.as_ref().unwrap_or(value);
                let scoped_target = self.resolve_with_scope_binding(name)?;
                if trace {
                    eprintln!("binding_statement:var:start name={name}");
                }
                self.emit_binding_initializer_value(name, value)?;
                if trace {
                    eprintln!("binding_statement:var:after_emit name={name}");
                }
                self.push_local_set(value_local);
                if let Some(scope_object) = scoped_target {
                    self.emit_scoped_property_store_from_local(
                        &scope_object,
                        name,
                        value_local,
                        value,
                    )?;
                    self.state.emission.output.instructions.push(0x1a);
                } else {
                    if trace {
                        eprintln!("binding_statement:var:before_store name={name}");
                    }
                    self.emit_store_identifier_value_local(name, store_value, value_local)?;
                    if trace {
                        eprintln!("binding_statement:var:after_store name={name}");
                    }
                }
                self.update_member_function_binding_from_expression(store_value);
                if trace {
                    eprintln!("binding_statement:var:after_member name={name}");
                }
                self.update_object_binding_from_expression(store_value);
                if trace {
                    eprintln!("binding_statement:var:done name={name}");
                }
                Ok(())
            }
            Statement::Let { name, value, .. } => {
                let trace = std::env::var_os("AYY_TRACE_FUNCTION_COMPILE").is_some();
                let value_local = self.allocate_temp_local();
                let resolved_store_value = self
                    .static_for_await_iterator_initializer_result(name, value)
                    .or_else(|| self.static_class_constructor_call_initializer_result(value));
                let store_value = resolved_store_value.as_ref().unwrap_or(value);
                if trace {
                    eprintln!("binding_statement:let:start name={name} value={value:?}");
                }
                self.emit_binding_initializer_value(name, value)?;
                if trace {
                    eprintln!("binding_statement:let:after_emit name={name}");
                }
                self.push_local_set(value_local);
                if let Some(initialized_local) = self
                    .state
                    .speculation
                    .static_semantics
                    .local_lexical_initialized_locals
                    .get(name)
                    .copied()
                    .or_else(|| {
                        self.state
                            .speculation
                            .static_semantics
                            .eval_lexical_initialized_locals
                            .get(name)
                            .copied()
                    })
                {
                    self.push_i32_const(1);
                    self.push_local_set(initialized_local);
                }
                if trace {
                    eprintln!("binding_statement:let:before_initialize name={name}");
                }
                self.emit_initialize_identifier_value_local(name, store_value, value_local)?;
                if trace {
                    eprintln!("binding_statement:let:after_initialize name={name}");
                }
                if trace {
                    eprintln!("binding_statement:let:before_member name={name}");
                }
                self.update_member_function_binding_from_expression(store_value);
                if trace {
                    eprintln!("binding_statement:let:after_member name={name}");
                }
                self.update_object_binding_from_expression(store_value);
                if trace {
                    eprintln!("binding_statement:let:done name={name}");
                }
                Ok(())
            }
            Statement::Assign { name, value } => {
                let trace = std::env::var_os("AYY_TRACE_FUNCTION_COMPILE").is_some();
                if trace {
                    eprintln!("binding_statement:assign:start name={name}");
                }
                if self.try_emit_destructuring_default_assign_statement(name, value)? {
                    return Ok(());
                }
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
                if let Some(scope_object) = scoped_target {
                    let value_local = self.allocate_temp_local();
                    let scoped_store_value =
                        self.emit_scoped_compound_assignment_value(&scope_object, name, value)?;
                    if scoped_store_value.is_none() {
                        self.emit_binding_initializer_value(name, value)?;
                    }
                    if trace {
                        eprintln!("binding_statement:assign:after_emit name={name}");
                    }
                    self.push_local_set(value_local);
                    let store_value = scoped_store_value.as_ref().unwrap_or(value);
                    self.emit_scoped_property_store_from_local(
                        &scope_object,
                        name,
                        value_local,
                        store_value,
                    )?;
                    self.state.emission.output.instructions.push(0x1a);
                } else {
                    let compound_store_value = self.emit_identifier_compound_assignment_value(
                        name,
                        value,
                        resolved_reference_local.as_ref(),
                        reference_targets_capture,
                        reference_global_index,
                        reference_targets_eval_local,
                        reference_implicit_global,
                        reference_is_unresolvable,
                    )?;
                    if compound_store_value.is_none() {
                        self.emit_binding_initializer_value(name, value)?;
                    }
                    if trace {
                        eprintln!("binding_statement:assign:after_emit name={name}");
                    }
                    let value_local = self.allocate_temp_local();
                    self.push_local_set(value_local);
                    if trace {
                        eprintln!("binding_statement:assign:before_store name={name}");
                    }
                    let store_value = compound_store_value.as_ref().unwrap_or(value);
                    self.emit_store_identifier_value_local_with_reference_target(
                        name,
                        store_value,
                        value_local,
                        resolved_reference_local,
                        reference_targets_capture,
                        reference_global_index,
                        reference_targets_eval_local,
                        reference_implicit_global,
                        reference_is_unresolvable,
                    )?;
                    if trace {
                        eprintln!("binding_statement:assign:after_store name={name}");
                    }
                }
                if trace {
                    eprintln!("binding_statement:assign:before_member name={name}");
                }
                self.update_member_function_binding_from_expression(value);
                if trace {
                    eprintln!("binding_statement:assign:after_member name={name}");
                }
                self.update_object_binding_from_expression(value);
                if trace {
                    eprintln!("binding_statement:assign:done name={name}");
                }
                Ok(())
            }
            Statement::AssignMember {
                object,
                property,
                value,
            } => {
                let expression = Expression::AssignMember {
                    object: Box::new(object.clone()),
                    property: Box::new(property.clone()),
                    value: Box::new(value.clone()),
                };
                self.with_class_field_initializer_eval_scope(
                    self.statement_uses_class_field_initializer_eval_rules(statement),
                    |compiler| compiler.emit_numeric_expression(&expression),
                )?;
                self.state.emission.output.instructions.push(0x1a);
                Ok(())
            }
            _ => unreachable!("emit_binding_statement called with non-binding statement"),
        }
    }
}
