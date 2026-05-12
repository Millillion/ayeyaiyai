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

    fn emit_binding_initializer_value(
        &mut self,
        name: &str,
        value: &Expression,
    ) -> DirectResult<()> {
        if self.is_private_brand_binding_initializer(name, value) {
            return self.emit_fresh_private_brand_value();
        }
        if let Some(LocalFunctionBinding::User(function_name)) =
            self.resolve_function_binding_from_expression(value)
            && let Some(runtime_value) = self.user_function_runtime_value(&function_name)
        {
            self.push_i32_const(runtime_value);
            return Ok(());
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
    ) -> DirectResult<bool> {
        let Expression::Binary { op, left, right } = value else {
            return Ok(false);
        };
        if !matches!(left.as_ref(), Expression::Identifier(left_name) if left_name == name) {
            return Ok(false);
        }

        self.emit_scoped_property_read(scope_object, name)?;
        self.emit_numeric_expression(right)?;
        self.push_binary_op(*op)?;
        Ok(true)
    }

    pub(super) fn emit_binding_statement(&mut self, statement: &Statement) -> DirectResult<()> {
        if self.emit_static_tracked_array_step_binding_if_possible(statement)? {
            return Ok(());
        }
        match statement {
            Statement::Var { name, value } => {
                if matches!(value, Expression::Undefined) {
                    return Ok(());
                }
                let trace = std::env::var_os("AYY_TRACE_FUNCTION_COMPILE").is_some();
                let value_local = self.allocate_temp_local();
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
                    self.emit_store_identifier_value_local(name, value, value_local)?;
                    if trace {
                        eprintln!("binding_statement:var:after_store name={name}");
                    }
                }
                self.update_member_function_binding_from_expression(value);
                if trace {
                    eprintln!("binding_statement:var:after_member name={name}");
                }
                self.update_object_binding_from_expression(value);
                if trace {
                    eprintln!("binding_statement:var:done name={name}");
                }
                Ok(())
            }
            Statement::Let { name, value, .. } => {
                let trace = std::env::var_os("AYY_TRACE_FUNCTION_COMPILE").is_some();
                let value_local = self.allocate_temp_local();
                if trace {
                    eprintln!("binding_statement:let:start name={name} value={value:?}");
                }
                self.emit_binding_initializer_value(name, value)?;
                if trace {
                    eprintln!("binding_statement:let:after_emit name={name}");
                }
                self.push_local_set(value_local);
                if trace {
                    eprintln!("binding_statement:let:before_initialize name={name}");
                }
                self.emit_initialize_identifier_value_local(name, value, value_local)?;
                if trace {
                    eprintln!("binding_statement:let:after_initialize name={name}");
                }
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
                    eprintln!("binding_statement:let:before_member name={name}");
                }
                self.update_member_function_binding_from_expression(value);
                if trace {
                    eprintln!("binding_statement:let:after_member name={name}");
                }
                self.update_object_binding_from_expression(value);
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
                    if !self.emit_scoped_compound_assignment_value(&scope_object, name, value)? {
                        self.emit_binding_initializer_value(name, value)?;
                    }
                    if trace {
                        eprintln!("binding_statement:assign:after_emit name={name}");
                    }
                    self.push_local_set(value_local);
                    self.emit_scoped_property_store_from_local(
                        &scope_object,
                        name,
                        value_local,
                        value,
                    )?;
                    self.state.emission.output.instructions.push(0x1a);
                } else {
                    self.emit_binding_initializer_value(name, value)?;
                    if trace {
                        eprintln!("binding_statement:assign:after_emit name={name}");
                    }
                    let value_local = self.allocate_temp_local();
                    self.push_local_set(value_local);
                    if trace {
                        eprintln!("binding_statement:assign:before_store name={name}");
                    }
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
