use super::*;

impl<'a> FunctionCompiler<'a> {
    fn statement_is_ignorable_for_cached_iterator_next_step(statement: &Statement) -> bool {
        match statement {
            Statement::Block { body } | Statement::Declaration { body } => body
                .iter()
                .all(Self::statement_is_ignorable_for_cached_iterator_next_step),
            Statement::Expression(Expression::Call { .. }) => true,
            _ => false,
        }
    }

    fn statement_assigns_only_cached_iterator_nonlocals(
        statement: &Statement,
        assigned_nonlocals: &HashSet<String>,
    ) -> bool {
        let mut assigned = HashSet::new();
        collect_assigned_binding_names_from_statement(statement, &mut assigned);
        !assigned.is_empty()
            && assigned.iter().all(|name| {
                assigned_nonlocals.contains(name)
                    || scoped_binding_source_name(name)
                        .is_some_and(|source_name| assigned_nonlocals.contains(source_name))
            })
    }

    fn cached_iterator_branch_return_value<'b>(
        branch: &'b [Statement],
        assigned_nonlocals: &HashSet<String>,
    ) -> Option<&'b Expression> {
        let (last, setup) = branch.split_last()?;
        if !setup.iter().all(|statement| {
            Self::statement_is_ignorable_for_cached_iterator_next_step(statement)
                || Self::statement_assigns_only_cached_iterator_nonlocals(
                    statement,
                    assigned_nonlocals,
                )
        }) {
            return None;
        }
        match last {
            Statement::Return(value) => Some(value),
            Statement::Block { body } | Statement::Declaration { body } => {
                Self::cached_iterator_branch_return_value(body, assigned_nonlocals)
            }
            _ => None,
        }
    }

    fn iterator_result_object_member_value(
        &self,
        result: &Expression,
        property_name: &str,
    ) -> Option<Expression> {
        let property = Expression::String(property_name.to_string());
        if let Expression::Object(entries) = result {
            return entries.iter().find_map(|entry| match entry {
                ObjectEntry::Data { key, value }
                    if self.materialize_static_expression(key) == property =>
                {
                    Some(value.clone())
                }
                _ => None,
            });
        }
        let binding = self.resolve_object_binding_from_expression(result)?;
        if object_binding_lookup_descriptor(&binding, &property)
            .is_some_and(|descriptor| descriptor.has_get || descriptor.getter.is_some())
        {
            return None;
        }
        object_binding_lookup_value(&binding, &property).cloned()
    }

    fn update_static_iterator_result_object_step_binding(
        &mut self,
        name: &str,
        result: &Expression,
    ) -> bool {
        if !Self::is_internal_iterator_step_binding_name(name)
            || Self::expression_is_direct_iterator_next_call(result)
            || self.iterator_next_result_is_static_non_object(result)
        {
            return false;
        }
        let has_iterator_result_member = self
            .iterator_result_object_member_value(result, "done")
            .is_some()
            || self
                .iterator_result_object_member_value(result, "value")
                .is_some();
        if !has_iterator_result_member {
            return false;
        }
        let done = self
            .iterator_result_object_member_value(result, "done")
            .unwrap_or(Expression::Bool(false));
        let value = self
            .iterator_result_object_member_value(result, "value")
            .unwrap_or(Expression::Undefined);
        let (done_local, value_local) = match self
            .state
            .speculation
            .static_semantics
            .local_iterator_step_binding(name)
        {
            Some(IteratorStepBinding::Runtime {
                done_local,
                value_local,
                ..
            }) => (*done_local, *value_local),
            _ => (self.allocate_temp_local(), self.allocate_temp_local()),
        };

        self.emit_truthy_expression(&done)
            .expect("static iterator result done slot must be supported");
        self.push_local_set(done_local);
        self.emit_numeric_expression(&value)
            .expect("static iterator result value slot must be supported");
        self.push_local_set(value_local);

        let static_done = self.resolve_static_boolean_expression(&done);
        let static_value = Some(value.clone());
        let value_candidates = static_value.iter().cloned().collect();
        self.state
            .speculation
            .static_semantics
            .set_local_iterator_step_binding(
                name,
                IteratorStepBinding::Runtime {
                    done_local,
                    value_local,
                    function_binding: static_value
                        .as_ref()
                        .and_then(|value| self.resolve_function_binding_from_expression(value)),
                    static_done,
                    static_value,
                    value_candidates,
                    entry_array: None,
                },
            );
        self.state
            .speculation
            .static_semantics
            .set_local_kind(name, StaticValueKind::Object);
        true
    }

    fn expression_is_direct_iterator_next_call(expression: &Expression) -> bool {
        matches!(
            expression,
            Expression::Call { callee, arguments }
                if arguments.is_empty()
                    && matches!(
                        callee.as_ref(),
                        Expression::Member { property, .. }
                            if matches!(
                                property.as_ref(),
                                Expression::String(property_name) if property_name == "next"
                            )
                    )
        )
    }

    fn cached_iterator_next_step_returns(
        &self,
        binding: &CachedIteratorNextMethodBinding,
        arguments: &[CallArgument],
    ) -> Option<Vec<(Option<Expression>, Expression)>> {
        let trace = std::env::var_os("AYY_TRACE_ITERATOR_STEP").is_some();
        if !arguments.is_empty() {
            if trace {
                eprintln!("iterator_step_cached_returns:reject_args");
            }
            return None;
        }
        let LocalFunctionBinding::User(function_name) = &binding.function_binding else {
            if trace {
                eprintln!("iterator_step_cached_returns:reject_non_user");
            }
            return None;
        };
        let Some(user_function) = self.user_function(function_name).cloned().or_else(|| {
            self.backend
                .function_registry
                .user_function(function_name)
                .cloned()
        }) else {
            if trace {
                eprintln!("iterator_step_cached_returns:reject_user_miss function={function_name}");
            }
            return None;
        };
        let assigned_nonlocals =
            self.collect_user_function_assigned_nonlocal_bindings(&user_function);
        if self.user_function_mentions_direct_eval(&user_function)
            || self.user_function_references_captured_user_function(&user_function)
            || user_function.has_lowered_pattern_parameters()
            || !self
                .user_function_parameter_iterator_consumption_indices(&user_function)
                .is_empty()
        {
            if trace {
                eprintln!(
                    "iterator_step_cached_returns:reject_effects function={function_name} direct_eval={} private={} captured_ref={} lowered={} param_iter={} assigned={} call_effect={}",
                    self.user_function_mentions_direct_eval(&user_function),
                    self.user_function_mentions_private_member_access(&user_function),
                    self.user_function_references_captured_user_function(&user_function),
                    user_function.has_lowered_pattern_parameters(),
                    !self
                        .user_function_parameter_iterator_consumption_indices(&user_function)
                        .is_empty(),
                    !assigned_nonlocals.is_empty(),
                    !self
                        .collect_user_function_call_effect_nonlocal_bindings(&user_function)
                        .is_empty()
                );
            }
            return None;
        }
        let Some(function) = self.resolve_registered_function_declaration(function_name) else {
            if trace {
                eprintln!(
                    "iterator_step_cached_returns:reject_declaration_miss function={function_name}"
                );
            }
            return None;
        };
        let call_this_binding = match &binding.this_expression {
            Expression::Object(_) => self
                .state
                .speculation
                .static_semantics
                .arrays
                .cached_iterator_next_method_bindings
                .iter()
                .find_map(|(name, candidate)| {
                    static_expression_matches(&candidate.this_expression, &binding.this_expression)
                        .then(|| Expression::Identifier(name.clone()))
                })
                .unwrap_or_else(|| binding.this_expression.clone()),
            _ => binding.this_expression.clone(),
        };
        let arguments_binding = Expression::Array(
            arguments
                .iter()
                .map(|argument| match argument {
                    CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                        ArrayElement::Expression(expression.clone())
                    }
                })
                .collect(),
        );
        let mut returns = Vec::new();
        for statement in &function.body {
            match statement {
                Statement::If {
                    condition,
                    then_branch,
                    else_branch,
                } if else_branch.is_empty() => {
                    let Some(return_value) =
                        Self::cached_iterator_branch_return_value(then_branch, &assigned_nonlocals)
                    else {
                        if trace {
                            eprintln!(
                                "iterator_step_cached_returns:reject_then_branch function={function_name} branch={then_branch:?}"
                            );
                        }
                        return None;
                    };
                    let condition = self.substitute_user_function_call_frame_bindings(
                        condition,
                        &user_function,
                        arguments,
                        &call_this_binding,
                        &arguments_binding,
                    );
                    let return_value = self.substitute_user_function_call_frame_bindings(
                        return_value,
                        &user_function,
                        arguments,
                        &call_this_binding,
                        &arguments_binding,
                    );
                    let (condition, return_value) =
                        if let Some(capture_slots) = binding.capture_slots.as_ref() {
                            (
                                self.substitute_capture_slot_bindings(&condition, capture_slots),
                                self.substitute_capture_slot_bindings(&return_value, capture_slots),
                            )
                        } else {
                            (condition, return_value)
                        };
                    returns.push((Some(condition), return_value));
                }
                Statement::Return(return_value) => {
                    let return_value = self.substitute_user_function_call_frame_bindings(
                        return_value,
                        &user_function,
                        arguments,
                        &call_this_binding,
                        &arguments_binding,
                    );
                    let return_value = binding
                        .capture_slots
                        .as_ref()
                        .map(|capture_slots| {
                            self.substitute_capture_slot_bindings(&return_value, capture_slots)
                        })
                        .unwrap_or(return_value);
                    returns.push((None, return_value));
                    return Some(returns);
                }
                statement
                    if Self::statement_is_ignorable_for_cached_iterator_next_step(statement) => {}
                statement
                    if Self::statement_assigns_only_cached_iterator_nonlocals(
                        statement,
                        &assigned_nonlocals,
                    ) => {}
                _ => {
                    if trace {
                        eprintln!(
                            "iterator_step_cached_returns:reject_statement function={function_name} statement={statement:?}"
                        );
                    }
                    return None;
                }
            }
        }
        if trace {
            eprintln!(
                "iterator_step_cached_returns:reject_no_terminal_return function={function_name}"
            );
        }
        None
    }

    fn emit_cached_iterator_next_result_slots(
        &mut self,
        result: &Expression,
        done_local: u32,
        value_local: u32,
    ) -> DirectResult<()> {
        if self.iterator_next_result_is_static_non_object(result) {
            self.emit_named_error_throw("TypeError")?;
            self.push_i32_const(1);
            self.push_local_set(done_local);
            self.push_i32_const(JS_UNDEFINED_TAG);
            self.push_local_set(value_local);
            return Ok(());
        }
        if std::env::var_os("AYY_TRACE_ITERATOR_CACHED_SLOTS").is_some() {
            eprintln!(
                "iterator_cached_slots current_fn={:?} result={result:?} done={:?} value={:?}",
                self.current_function_name(),
                self.iterator_result_object_member_value(result, "done"),
                self.iterator_result_object_member_value(result, "value")
            );
        }
        if std::env::var_os("AYY_TRACE_ITERATOR_STEP").is_some() {
            eprintln!(
                "iterator_step_cached_result_slots result={result:?} done={:?} value={:?}",
                self.iterator_result_object_member_value(result, "done"),
                self.iterator_result_object_member_value(result, "value")
            );
        }
        self.emit_iterator_result_done_slot(result, done_local)?;
        self.emit_iterator_result_value_slot_if_not_done(result, done_local, value_local)?;
        Ok(())
    }

    fn iterator_result_member_expression(result: &Expression, property_name: &str) -> Expression {
        Expression::Member {
            object: Box::new(result.clone()),
            property: Box::new(Expression::String(property_name.to_string())),
        }
    }

    fn emit_iterator_result_done_slot(
        &mut self,
        result: &Expression,
        done_local: u32,
    ) -> DirectResult<()> {
        if let Some(done) = self.iterator_result_object_member_value(result, "done") {
            self.emit_truthy_expression(&done)?;
        } else {
            let done = Self::iterator_result_member_expression(result, "done");
            self.emit_truthy_expression(&done)?;
        }
        self.push_local_set(done_local);
        Ok(())
    }

    fn emit_iterator_result_value_slot(
        &mut self,
        result: &Expression,
        value_local: u32,
    ) -> DirectResult<()> {
        if let Some(value) = self.iterator_result_object_member_value(result, "value") {
            if self.emit_cached_iterator_post_update_value_slot(&value, value_local)? {
                return Ok(());
            }
            self.emit_numeric_expression(&value)?;
        } else {
            let value = Self::iterator_result_member_expression(result, "value");
            self.emit_numeric_expression(&value)?;
        }
        self.push_local_set(value_local);
        Ok(())
    }

    fn emit_cached_iterator_post_update_value_slot(
        &mut self,
        value: &Expression,
        value_local: u32,
    ) -> DirectResult<bool> {
        let Expression::AssignMember {
            object,
            property,
            value: assigned_value,
        } = value
        else {
            return Ok(false);
        };
        let Expression::Binary {
            op: BinaryOp::Add,
            left,
            right,
        } = assigned_value.as_ref()
        else {
            return Ok(false);
        };
        if !matches!(right.as_ref(), Expression::Number(value) if *value == 1.0) {
            return Ok(false);
        }
        let previous_value = Expression::Member {
            object: object.clone(),
            property: property.clone(),
        };
        if !static_expression_matches(left, &previous_value) {
            return Ok(false);
        }

        let old_value_local = self.allocate_temp_local();
        self.emit_numeric_expression(&previous_value)?;
        self.push_local_set(old_value_local);
        self.emit_numeric_expression(value)?;
        self.state.emission.output.instructions.push(0x1a);
        self.push_local_get(old_value_local);
        self.push_local_set(value_local);
        Ok(true)
    }

    fn emit_iterator_result_value_slot_if_not_done(
        &mut self,
        result: &Expression,
        done_local: u32,
        value_local: u32,
    ) -> DirectResult<()> {
        self.push_local_get(done_local);
        self.state.emission.output.instructions.push(0x04);
        self.state
            .emission
            .output
            .instructions
            .push(EMPTY_BLOCK_TYPE);
        self.push_control_frame();
        self.push_i32_const(JS_UNDEFINED_TAG);
        self.push_local_set(value_local);
        self.state.emission.output.instructions.push(0x05);
        self.emit_iterator_result_value_slot(result, value_local)?;
        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();
        Ok(())
    }

    fn emit_cached_iterator_next_step_returns(
        &mut self,
        returns: &[(Option<Expression>, Expression)],
        done_local: u32,
        value_local: u32,
    ) -> DirectResult<()> {
        let Some((head, tail)) = returns.split_first() else {
            self.push_i32_const(1);
            self.push_local_set(done_local);
            self.push_i32_const(JS_UNDEFINED_TAG);
            self.push_local_set(value_local);
            return Ok(());
        };
        let (condition, result) = head;
        let Some(condition) = condition else {
            return self.emit_cached_iterator_next_result_slots(result, done_local, value_local);
        };

        self.emit_truthy_expression(condition)?;
        self.state.emission.output.instructions.push(0x04);
        self.state
            .emission
            .output
            .instructions
            .push(EMPTY_BLOCK_TYPE);
        self.push_control_frame();
        self.emit_cached_iterator_next_result_slots(result, done_local, value_local)?;
        self.state.emission.output.instructions.push(0x05);
        self.emit_cached_iterator_next_step_returns(tail, done_local, value_local)?;
        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();
        Ok(())
    }

    fn is_internal_iterator_step_binding_name(name: &str) -> bool {
        name.starts_with("__ayy_array_step_") || name.starts_with("__ayy_for_of_step_")
    }

    fn captured_iterator_next_call_result_expression(
        &self,
        function_name: &str,
        source_expression: &Expression,
    ) -> Option<Expression> {
        let snapshot = self
            .state
            .speculation
            .static_semantics
            .last_bound_user_function_call
            .as_ref()?;
        if snapshot.function_name != function_name
            || !snapshot
                .source_expression
                .as_ref()
                .is_some_and(|source| static_expression_matches(source, source_expression))
        {
            return None;
        }
        snapshot.result_expression.clone()
    }

    fn iterator_next_result_is_static_non_object(&self, result: &Expression) -> bool {
        let materialized = self.materialize_static_expression(result);
        let result = if static_expression_matches(&materialized, result) {
            result
        } else {
            &materialized
        };
        match self.infer_value_kind(result) {
            Some(StaticValueKind::Object | StaticValueKind::Function) => false,
            Some(
                StaticValueKind::Number
                | StaticValueKind::Bool
                | StaticValueKind::String
                | StaticValueKind::BigInt
                | StaticValueKind::Null
                | StaticValueKind::Undefined
                | StaticValueKind::Symbol,
            ) => true,
            Some(StaticValueKind::Unknown) | None => false,
        }
    }

    fn emit_iterator_next_result_runtime_object_check(
        &mut self,
        result: &Expression,
    ) -> DirectResult<()> {
        let result_value_local = self.allocate_temp_local();
        let result_type_local = self.allocate_temp_local();
        self.emit_numeric_expression(result)?;
        self.push_local_set(result_value_local);
        self.emit_runtime_typeof_tag_from_local(result_value_local)?;
        self.push_local_set(result_type_local);

        self.push_local_get(result_type_local);
        self.push_i32_const(JS_TYPEOF_OBJECT_TAG);
        self.push_binary_op(BinaryOp::NotEqual)?;
        self.push_local_get(result_type_local);
        self.push_i32_const(JS_TYPEOF_FUNCTION_TAG);
        self.push_binary_op(BinaryOp::NotEqual)?;
        self.state.emission.output.instructions.push(0x71);
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

    fn emit_captured_iterator_next_result_step_slots(
        &mut self,
        result: &Expression,
        done_local: u32,
        value_local: u32,
    ) -> DirectResult<()> {
        if self.iterator_next_result_is_static_non_object(result) {
            self.emit_named_error_throw("TypeError")?;
            self.push_i32_const(1);
            self.push_local_set(done_local);
            self.push_i32_const(JS_UNDEFINED_TAG);
            self.push_local_set(value_local);
            return Ok(());
        }
        self.emit_iterator_next_result_runtime_object_check(result)?;
        self.emit_iterator_result_done_slot(result, done_local)?;
        self.emit_iterator_result_value_slot_if_not_done(result, done_local, value_local)?;
        Ok(())
    }

    fn update_captured_iterator_next_step_binding(
        &mut self,
        name: &str,
        source_expression: &Expression,
        object: &Expression,
        property: &Expression,
        arguments: &[CallArgument],
    ) -> bool {
        if !Self::is_internal_iterator_step_binding_name(name) {
            return false;
        }
        let Some(plan) = self.captured_iterator_next_method_plan(object, property, arguments)
        else {
            return false;
        };
        let (done_local, value_local) = match self
            .state
            .speculation
            .static_semantics
            .local_iterator_step_binding(name)
        {
            Some(IteratorStepBinding::Runtime {
                done_local,
                value_local,
                ..
            }) => (*done_local, *value_local),
            _ => (self.allocate_temp_local(), self.allocate_temp_local()),
        };

        let result = self
            .captured_iterator_next_call_result_expression(&plan.function_name, source_expression)
            .unwrap_or_else(|| Expression::Identifier(plan.current_slot.clone()));
        self.emit_captured_iterator_next_result_step_slots(&result, done_local, value_local)
            .expect("captured iterator next result slots must be supported");

        if !matches!(
            result,
            Expression::Identifier(ref result_name) if result_name != &plan.current_slot
        ) {
            self.emit_store_iterator_next_capture_slot(&plan.current_slot, &plan.next_value)
                .expect("captured iterator next capture slot update must be supported");
        }

        self.state
            .speculation
            .static_semantics
            .set_local_iterator_step_binding(
                name,
                IteratorStepBinding::Runtime {
                    done_local,
                    value_local,
                    function_binding: None,
                    static_done: None,
                    static_value: None,
                    value_candidates: Vec::new(),
                    entry_array: None,
                },
            );
        self.state
            .speculation
            .static_semantics
            .set_local_kind(name, StaticValueKind::Object);
        true
    }

    fn update_cached_iterator_next_step_binding(
        &mut self,
        name: &str,
        object: &Expression,
        property: &Expression,
        arguments: &[CallArgument],
    ) -> bool {
        let trace = std::env::var_os("AYY_TRACE_ITERATOR_STEP").is_some();
        let Some(binding) = self.cached_iterator_next_method_binding_for_object(object, property)
        else {
            if trace {
                eprintln!("iterator_step_update:cached_miss name={name} object={object:?}");
            }
            return false;
        };
        if trace {
            eprintln!(
                "iterator_step_update:cached_binding name={name} binding={:?} captures={}",
                binding.function_binding,
                binding
                    .capture_slots
                    .as_ref()
                    .map(|slots| slots.len())
                    .unwrap_or(0)
            );
        }
        let Some(mut returns) = self.cached_iterator_next_step_returns(&binding, arguments) else {
            if trace {
                eprintln!("iterator_step_update:cached_no_returns name={name}");
            }
            return false;
        };
        if matches!(object, Expression::Identifier(iterator_name) if iterator_name.starts_with("__ayy_for_await_iter_"))
        {
            returns = returns
                .into_iter()
                .map(|(condition, result)| {
                    let awaited_result = match self.resolve_static_await_resolution_outcome(&result)
                    {
                        Some(StaticEvalOutcome::Value(value)) => value,
                        _ => result,
                    };
                    (condition, awaited_result)
                })
                .collect();
        }
        if trace {
            eprintln!(
                "iterator_step_update:cached_returns name={name} count={}",
                returns.len()
            );
        }
        let (done_local, value_local) = match self
            .state
            .speculation
            .static_semantics
            .local_iterator_step_binding(name)
        {
            Some(IteratorStepBinding::Runtime {
                done_local,
                value_local,
                ..
            }) => (*done_local, *value_local),
            _ => (self.allocate_temp_local(), self.allocate_temp_local()),
        };

        self.emit_cached_iterator_next_step_returns(&returns, done_local, value_local)
            .expect("cached iterator next step returns must be supported");
        let terminal_value = returns
            .iter()
            .find_map(|(condition, result)| condition.is_none().then_some(result))
            .and_then(|result| self.iterator_result_object_member_value(result, "value"));
        let value_candidates = returns
            .iter()
            .filter_map(|(_, result)| self.iterator_result_object_member_value(result, "value"))
            .collect::<Vec<_>>();

        self.state
            .speculation
            .static_semantics
            .set_local_iterator_step_binding(
                name,
                IteratorStepBinding::Runtime {
                    done_local,
                    value_local,
                    function_binding: terminal_value
                        .as_ref()
                        .and_then(|value| self.resolve_function_binding_from_expression(value)),
                    static_done: None,
                    static_value: terminal_value,
                    value_candidates,
                    entry_array: None,
                },
            );
        self.state
            .speculation
            .static_semantics
            .set_local_kind(name, StaticValueKind::Object);
        true
    }

    pub(in crate::backend::direct_wasm) fn update_local_iterator_step_binding(
        &mut self,
        name: &str,
        value: &Expression,
    ) {
        let trace = std::env::var_os("AYY_TRACE_ITERATOR_STEP").is_some();
        if trace {
            eprintln!("iterator_step_update:start name={name} value={value:?}");
        }
        if self.update_static_iterator_result_object_step_binding(name, value) {
            if trace {
                eprintln!("iterator_step_update:static_object_hit name={name}");
            }
            return;
        }
        let Expression::Call { callee, arguments } = value else {
            if trace {
                eprintln!("iterator_step_update:clear_non_call name={name}");
            }
            self.state
                .speculation
                .static_semantics
                .clear_local_iterator_step_binding(name);
            return;
        };
        if !arguments.is_empty() {
            if trace {
                eprintln!("iterator_step_update:clear_args name={name}");
            }
            self.state
                .speculation
                .static_semantics
                .clear_local_iterator_step_binding(name);
            return;
        }
        let Expression::Member { object, property } = callee.as_ref() else {
            if trace {
                eprintln!("iterator_step_update:clear_non_member name={name}");
            }
            self.state
                .speculation
                .static_semantics
                .clear_local_iterator_step_binding(name);
            return;
        };
        if !matches!(property.as_ref(), Expression::String(property_name) if property_name == "next")
        {
            if trace {
                eprintln!("iterator_step_update:clear_non_next name={name}");
            }
            self.state
                .speculation
                .static_semantics
                .clear_local_iterator_step_binding(name);
            return;
        }
        let object_is_local_array_iterator = matches!(object.as_ref(), Expression::Identifier(iterator_name) if self
                .resolve_local_array_iterator_binding_name(iterator_name)
                .and_then(|binding_name| self
                    .state
                    .speculation
                    .static_semantics
                    .local_array_iterator_binding(&binding_name))
                .is_some());
        if !object_is_local_array_iterator
            && self.update_captured_iterator_next_step_binding(
                name, value, object, property, arguments,
            )
        {
            if trace {
                eprintln!("iterator_step_update:captured_hit name={name}");
            }
            return;
        }
        if !object_is_local_array_iterator
            && self.update_cached_iterator_next_step_binding(name, object, property, arguments)
        {
            if trace {
                eprintln!("iterator_step_update:cached_hit name={name}");
            }
            return;
        }
        let Expression::Identifier(iterator_name) = object.as_ref() else {
            if trace {
                eprintln!("iterator_step_update:clear_non_identifier_object name={name}");
            }
            self.state
                .speculation
                .static_semantics
                .clear_local_iterator_step_binding(name);
            return;
        };
        let iterator_binding_name = self
            .resolve_local_array_iterator_binding_name(iterator_name)
            .unwrap_or_else(|| iterator_name.clone());
        let Some(mut iterator_binding) = self
            .state
            .speculation
            .static_semantics
            .local_array_iterator_binding(&iterator_binding_name)
            .cloned()
        else {
            if trace {
                eprintln!(
                    "iterator_step_update:clear_no_iterator name={name} iterator={iterator_binding_name}"
                );
            }
            self.state
                .speculation
                .static_semantics
                .clear_local_iterator_step_binding(name);
            return;
        };
        if matches!(
            iterator_binding.source,
            IteratorSourceKind::SimpleGenerator { is_async: true, .. }
                | IteratorSourceKind::AsyncYieldDelegateGenerator { .. }
        ) {
            self.state
                .speculation
                .static_semantics
                .clear_local_iterator_step_binding(name);
            return;
        }
        let uses_previous_static_index = self
            .state
            .speculation
            .static_semantics
            .last_bound_user_function_call
            .as_ref()
            .is_some_and(|snapshot| {
                snapshot.function_name == "__ayy_simple_generator_next"
                    && snapshot
                        .source_expression
                        .as_ref()
                        .is_some_and(|source| static_expression_matches(source, value))
            });
        let (done_local, value_local, previous_entry_array) = match self
            .state
            .speculation
            .static_semantics
            .local_iterator_step_binding(name)
        {
            Some(IteratorStepBinding::Runtime {
                done_local,
                value_local,
                entry_array,
                ..
            }) => (*done_local, *value_local, entry_array.clone()),
            _ => (self.allocate_temp_local(), self.allocate_temp_local(), None),
        };
        let mut entry_array = match &iterator_binding.source {
            IteratorSourceKind::StaticArrayEntries { .. }
            | IteratorSourceKind::StaticMapEntries { .. } => Some(
                previous_entry_array.unwrap_or_else(|| IteratorStepEntryArrayBinding {
                    index_local: self.allocate_temp_local(),
                    value_local: self.allocate_temp_local(),
                }),
            ),
            _ => None,
        };
        let function_binding = self.resolve_iterator_step_function_binding(&iterator_binding);
        let value_candidates = iterator_step_value_candidates(&iterator_binding.source);
        let current_static_index = if uses_previous_static_index {
            iterator_binding
                .static_index
                .map(|index| index.saturating_sub(1))
        } else {
            iterator_binding.static_index
        };
        let sent_value = Expression::Undefined;

        if uses_previous_static_index
            && let IteratorSourceKind::SimpleGenerator { .. } = &iterator_binding.source
            && let Some(index) = current_static_index
        {
            let (static_done, static_value) = self.emit_previous_simple_generator_iterator_step(
                &mut iterator_binding,
                index,
                &sent_value,
                done_local,
                value_local,
            );
            self.state
                .speculation
                .static_semantics
                .set_local_array_iterator_binding(&iterator_binding_name, iterator_binding);
            self.state
                .speculation
                .static_semantics
                .set_local_iterator_step_binding(
                    name,
                    IteratorStepBinding::Runtime {
                        done_local,
                        value_local,
                        function_binding,
                        static_done,
                        static_value,
                        value_candidates,
                        entry_array: entry_array.take(),
                    },
                );
            self.state
                .speculation
                .static_semantics
                .set_local_kind(name, StaticValueKind::Object);
            return;
        }

        let (static_done, static_value) = self.resolve_iterator_step_static_outcome(
            &iterator_binding,
            current_static_index,
            &sent_value,
        );
        if self.static_array_iterator_is_exhausted(&iterator_binding.source, current_static_index) {
            if let Some(current_index) = iterator_binding.static_index {
                iterator_binding.static_index = Some(current_index.saturating_add(1));
            }
            self.push_i32_const(1);
            self.push_local_set(done_local);
            self.push_i32_const(JS_UNDEFINED_TAG);
            self.push_local_set(value_local);
            self.state
                .speculation
                .static_semantics
                .set_local_array_iterator_binding(&iterator_binding_name, iterator_binding);
            self.state
                .speculation
                .static_semantics
                .set_local_iterator_step_binding(
                    name,
                    IteratorStepBinding::Runtime {
                        done_local,
                        value_local,
                        function_binding,
                        static_done,
                        static_value,
                        value_candidates,
                        entry_array: entry_array.take(),
                    },
                );
            self.state
                .speculation
                .static_semantics
                .set_local_kind(name, StaticValueKind::Object);
            if trace {
                eprintln!("iterator_step_update:exhausted_static_array name={name}");
            }
            return;
        }

        let current_index_local = self.allocate_temp_local();
        self.push_local_get(iterator_binding.index_local);
        self.push_local_set(current_index_local);

        self.emit_runtime_iterator_step_source_update(
            &mut iterator_binding,
            current_static_index,
            current_index_local,
            &sent_value,
            done_local,
            value_local,
        );
        if let Some(entry_array) = entry_array.as_ref() {
            self.update_runtime_iterator_step_static_array_entry_slots(
                &iterator_binding.source,
                current_index_local,
                done_local,
                entry_array,
            );
        }

        self.state
            .speculation
            .static_semantics
            .set_local_array_iterator_binding(&iterator_binding_name, iterator_binding);
        self.state
            .speculation
            .static_semantics
            .set_local_iterator_step_binding(
                name,
                IteratorStepBinding::Runtime {
                    done_local,
                    value_local,
                    function_binding,
                    static_done,
                    static_value,
                    value_candidates,
                    entry_array,
                },
            );
        self.state
            .speculation
            .static_semantics
            .set_local_kind(name, StaticValueKind::Object);
        if trace {
            eprintln!("iterator_step_update:runtime_source_hit name={name}");
        }
    }

    fn static_array_iterator_is_exhausted(
        &self,
        source: &IteratorSourceKind,
        current_static_index: Option<usize>,
    ) -> bool {
        let Some(current_static_index) = current_static_index else {
            return false;
        };
        match source {
            IteratorSourceKind::StaticArray {
                values,
                length_local,
                runtime_name,
                ..
            } if self
                .static_array_source_exhaustion_length(
                    values.len(),
                    *length_local,
                    runtime_name.as_deref(),
                )
                .is_some_and(|length| current_static_index >= length) =>
            {
                true
            }
            IteratorSourceKind::StaticArrayEntries {
                values,
                length_local,
                runtime_name,
            } if self
                .static_array_source_exhaustion_length(
                    values.len(),
                    *length_local,
                    runtime_name.as_deref(),
                )
                .is_some_and(|length| current_static_index >= length) =>
            {
                true
            }
            IteratorSourceKind::StaticMapEntries {
                values,
                length_local: None,
                key_runtime_name: None,
                value_runtime_name: None,
            } => current_static_index >= values.len(),
            IteratorSourceKind::TypedArrayView { name } => self
                .typed_array_view_binding_for_name(name)
                .as_ref()
                .and_then(|view| view.fixed_length)
                .is_some_and(|length| current_static_index >= length),
            _ => false,
        }
    }

    fn resolve_iterator_step_function_binding(
        &self,
        iterator_binding: &ArrayIteratorBinding,
    ) -> Option<LocalFunctionBinding> {
        match &iterator_binding.source {
            IteratorSourceKind::StaticArray {
                values, keys_only, ..
            } if !keys_only => {
                let bindings = values
                    .iter()
                    .flatten()
                    .map(|value| self.resolve_function_binding_from_expression(value))
                    .collect::<Option<Vec<_>>>();
                bindings.and_then(|bindings| {
                    if bindings.is_empty() {
                        None
                    } else if bindings
                        .iter()
                        .all(|binding| binding == bindings.first().expect("not empty"))
                    {
                        bindings.first().cloned()
                    } else if are_function_constructor_bindings(&bindings) {
                        Some(LocalFunctionBinding::Builtin(
                            FUNCTION_CONSTRUCTOR_FAMILY_BUILTIN.to_string(),
                        ))
                    } else {
                        None
                    }
                })
            }
            _ => None,
        }
    }
}

fn iterator_step_value_candidates(source: &IteratorSourceKind) -> Vec<Expression> {
    match source {
        IteratorSourceKind::StaticArray {
            values, keys_only, ..
        } => values
            .iter()
            .enumerate()
            .map(|(index, value)| {
                if *keys_only {
                    Expression::Number(index as f64)
                } else {
                    value.clone().unwrap_or(Expression::Undefined)
                }
            })
            .collect(),
        IteratorSourceKind::StaticArrayEntries { values, .. } => values
            .iter()
            .enumerate()
            .map(|(index, value)| {
                Expression::Array(vec![
                    ArrayElement::Expression(Expression::Number(index as f64)),
                    ArrayElement::Expression(value.clone().unwrap_or(Expression::Undefined)),
                ])
            })
            .collect(),
        IteratorSourceKind::StaticMapEntries { values, .. } => values
            .iter()
            .map(|value| value.clone().unwrap_or(Expression::Undefined))
            .collect(),
        IteratorSourceKind::SimpleGenerator { steps, .. } => steps
            .iter()
            .filter_map(|step| match &step.outcome {
                SimpleGeneratorStepOutcome::Yield(value) => Some(value.clone()),
                SimpleGeneratorStepOutcome::YieldResult(result) => match result {
                    Expression::Object(entries) => Some(
                        entries
                            .iter()
                            .find_map(|entry| match entry {
                                ObjectEntry::Data {
                                    key: Expression::String(name),
                                    value,
                                } if name == "value" => Some(value.clone()),
                                _ => None,
                            })
                            .unwrap_or(Expression::Undefined),
                    ),
                    _ => None,
                },
                SimpleGeneratorStepOutcome::Throw(_) => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}
