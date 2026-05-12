use super::*;

impl<'a> FunctionCompiler<'a> {
    fn call_expression_static_number_shortcut_requires_runtime(
        &self,
        expression: &Expression,
    ) -> bool {
        let Expression::Call { callee, arguments } = expression else {
            return false;
        };
        if !inline_summary_side_effect_free_expression(callee)
            || arguments
                .iter()
                .any(|argument| !inline_summary_side_effect_free_expression(argument.expression()))
        {
            return true;
        }
        if matches!(
            callee.as_ref(),
            Expression::Member { property, .. }
                if matches!(property.as_ref(), Expression::String(name) if matches!(name.as_str(), "then" | "catch" | "finally"))
        ) {
            return false;
        }
        let Some(user_function) = self.resolve_user_function_from_expression(callee) else {
            return false;
        };

        if self
            .resolve_function_expression_capture_slots(callee)
            .is_some()
        {
            return true;
        }

        user_function.has_parameter_defaults()
            || self.user_function_mentions_direct_eval(user_function)
            || user_function
                .inline_summary
                .as_ref()
                .is_some_and(|summary| !summary.effects.is_empty())
            || !self
                .collect_user_function_assigned_nonlocal_bindings(user_function)
                .is_empty()
            || !self
                .collect_user_function_call_effect_nonlocal_bindings(user_function)
                .is_empty()
    }

    fn call_expression_static_number_shortcut_value(&self, expression: &Expression) -> Option<f64> {
        let materialized = self.materialize_static_expression(expression);
        if self.infer_value_kind(&materialized) != Some(StaticValueKind::Number) {
            return None;
        }
        self.resolve_static_number_value(&materialized)
            .or_else(|| self.resolve_static_number_value(expression))
    }

    fn call_expression_static_nullish_shortcut_value(
        &self,
        expression: &Expression,
    ) -> Option<Expression> {
        let materialized = self.materialize_static_expression(expression);
        match materialized {
            Expression::Null | Expression::Undefined => Some(materialized),
            _ => None,
        }
    }

    pub(in crate::backend::direct_wasm) fn emit_call_expression_dispatch(
        &mut self,
        expression: &Expression,
        callee: &Expression,
        arguments: &[CallArgument],
    ) -> DirectResult<()> {
        let trace_call_dispatch = std::env::var_os("AYY_TRACE_CALL_DISPATCH").is_some();
        if trace_call_dispatch {
            eprintln!(
                "call_dispatch:start function={:?} expr={:?}",
                self.current_function_name(),
                expression
            );
        }
        self.state
            .speculation
            .static_semantics
            .last_bound_user_function_call = None;
        let callee_requires_runtime_private_brand_check = match callee {
            Expression::Member { object, property } => {
                self.private_member_call_requires_runtime_brand_check(object, property)
            }
            _ => false,
        };
        if arguments.is_empty()
            && let Expression::Member { object, property } = callee
            && is_symbol_iterator_expression(property)
            && self
                .resolve_member_function_binding(object, property)
                .is_none()
            && self
                .resolve_member_getter_binding(object, property)
                .is_none()
            && (self.resolve_iterator_source_kind(object).is_some()
                || self
                    .resolve_for_await_step_value_iterator_source_kind(object)
                    .is_some())
        {
            self.emit_numeric_expression(object)?;
            self.state.emission.output.instructions.push(0x1a);
            self.push_i32_const(JS_TYPEOF_OBJECT_TAG);
            return Ok(());
        }
        if let Expression::Member { object, property } = callee
            && matches!(object.as_ref(), Expression::Identifier(name) if name == "assert")
            && matches!(property.as_ref(), Expression::String(name) if name == "throws")
            && self.emit_assert_throws_call(arguments)?
        {
            return Ok(());
        }
        let local_iterator_next_binding_name = match callee {
            Expression::Member { object, property }
                if matches!(property.as_ref(), Expression::String(property_name) if property_name == "next") =>
            {
                match object.as_ref() {
                    Expression::Identifier(iterator_name) => {
                        self.resolve_local_array_iterator_binding_name(iterator_name)
                    }
                    _ => None,
                }
            }
            _ => None,
        };
        let known_local_iterator_next_call =
            local_iterator_next_binding_name
                .as_ref()
                .is_some_and(|iterator_binding_name| {
                    arguments.is_empty()
                        || self
                            .state
                            .speculation
                            .static_semantics
                            .local_array_iterator_binding(iterator_binding_name)
                            .is_some_and(|binding| {
                                matches!(binding.source, IteratorSourceKind::SimpleGenerator { .. })
                            })
                });
        let promise_chain_call = matches!(
            callee,
            Expression::Member { property, .. }
                if matches!(property.as_ref(), Expression::String(name) if matches!(name.as_str(), "then" | "catch" | "finally"))
        );
        let reads_descriptor_member =
            self.expression_reads_local_descriptor_binding_member(expression);
        if trace_call_dispatch {
            eprintln!(
                "call_dispatch:before_static_number function={:?} reads_descriptor_member={} try_depth={} private_brand={}",
                self.current_function_name(),
                reads_descriptor_member,
                self.state.emission.control_flow.try_stack.len(),
                callee_requires_runtime_private_brand_check
            );
        }
        if self.state.emission.control_flow.try_stack.is_empty()
            && !callee_requires_runtime_private_brand_check
            && !known_local_iterator_next_call
            && !promise_chain_call
            && !reads_descriptor_member
            && !self.call_expression_static_number_shortcut_requires_runtime(expression)
            && let Some(number) = self.call_expression_static_number_shortcut_value(expression)
        {
            if trace_call_dispatch {
                eprintln!(
                    "call_dispatch:static_number function={:?} number={:?}",
                    self.current_function_name(),
                    number
                );
            }
            return self.emit_numeric_expression(&Expression::Number(number));
        }
        if self.state.emission.control_flow.try_stack.is_empty()
            && !callee_requires_runtime_private_brand_check
            && !known_local_iterator_next_call
            && !promise_chain_call
            && !reads_descriptor_member
            && !self.call_expression_static_number_shortcut_requires_runtime(expression)
            && let Some(value) = self.call_expression_static_nullish_shortcut_value(expression)
        {
            if trace_call_dispatch {
                eprintln!(
                    "call_dispatch:static_nullish function={:?} value={:?}",
                    self.current_function_name(),
                    value
                );
            }
            return self.emit_numeric_expression(&value);
        }
        if trace_call_dispatch {
            eprintln!(
                "call_dispatch:after_static_number function={:?} expr={:?}",
                self.current_function_name(),
                expression
            );
        }
        if known_local_iterator_next_call
            && let Expression::Member { object, .. } = callee
            && let Expression::Identifier(iterator_name) = object.as_ref()
            && let Some(iterator_binding_name) =
                self.resolve_local_array_iterator_binding_name(iterator_name)
            && self
                .state
                .speculation
                .static_semantics
                .local_array_iterator_binding(&iterator_binding_name)
                .is_some_and(|binding| {
                    !matches!(binding.source, IteratorSourceKind::SimpleGenerator { .. })
                })
        {
            self.emit_numeric_expression(object)?;
            self.state.emission.output.instructions.push(0x1a);
            self.push_i32_const(JS_TYPEOF_OBJECT_TAG);
            return Ok(());
        }
        if let Expression::Member { object, property } = callee
            && matches!(property.as_ref(), Expression::String(property_name) if property_name == "next")
            && self.emit_fresh_simple_generator_next_call(object, arguments)?
        {
            return Ok(());
        }
        if let Expression::Member { object, property } = callee
            && matches!(property.as_ref(), Expression::String(property_name) if property_name == "return")
            && self.emit_fresh_simple_generator_return_call(object, arguments)?
        {
            return Ok(());
        }
        if let Expression::Member { object, property } = callee
            && self
                .emit_captured_iterator_next_method_call(expression, object, property, arguments)?
        {
            return Ok(());
        }
        if let Expression::Member { object, property } = callee
            && matches!(
                property.as_ref(),
                Expression::String(property_name) if property_name == "then" || property_name == "catch"
            )
            && Self::call_is_promise_like_chain(object)
            && self.emit_early_member_call_shortcuts(object, property, arguments)?
        {
            return Ok(());
        }
        if !callee_requires_runtime_private_brand_check
            && !known_local_iterator_next_call
            && arguments.is_empty()
            && let Expression::Member { object, property } = callee
            && let Expression::String(property_name) = property.as_ref()
            && let Some(outcome) = self.resolve_static_member_call_outcome_with_context(
                object,
                property_name,
                self.current_function_name(),
            )
        {
            return self.emit_static_eval_outcome(&outcome);
        }
        if let Expression::Member { object, property } = callee
            && self.emit_cached_iterator_next_method_call(object, property, arguments)?
        {
            return Ok(());
        }
        if let Expression::Member { object, property } = callee
            && self.emit_member_getter_returned_user_function_call(object, property, arguments)?
        {
            return Ok(());
        }
        if self.emit_specialized_callee_call(callee, arguments)? {
            return Ok(());
        }
        if self.emit_static_weakref_deref_call(callee, arguments)? {
            return Ok(());
        }
        if self.emit_function_prototype_bind_call(callee, arguments)? {
            return Ok(());
        }
        if let Expression::Member { object, property } = callee
            && matches!(
                property.as_ref(),
                Expression::String(property_name) if property_name == "push" || property_name == "pop"
            )
            && self
                .emit_late_member_call_shortcuts(expression, callee, object, property, arguments)?
        {
            return Ok(());
        }
        if let Expression::Member { object, property } = callee
            && self.emit_early_member_call_shortcuts(object, property, arguments)?
        {
            return Ok(());
        }
        if self.constructed_function_call_creates_generator_iterator(callee) {
            self.push_i32_const(JS_TYPEOF_OBJECT_TAG);
            return Ok(());
        }
        if let Some(result) =
            self.resolve_static_constructed_function_call_result(callee, arguments)
        {
            return self.emit_numeric_expression(&result);
        }
        if let Expression::Identifier(name) = callee {
            if trace_call_dispatch {
                eprintln!(
                    "call_dispatch:identifier function={:?} name={}",
                    self.current_function_name(),
                    name
                );
            }
            return self.emit_identifier_call_expression(expression, callee, name, arguments);
        }
        if self.emit_resolved_function_binding_call_expression(expression, callee, arguments)? {
            return Ok(());
        }
        if !matches!(callee, Expression::Member { .. })
            && self.emit_dynamic_user_function_call(callee, arguments)?
        {
            return Ok(());
        }
        if let Expression::Member { object, property } = callee
            && self
                .emit_late_member_call_shortcuts(expression, callee, object, property, arguments)?
        {
            return Ok(());
        }

        self.emit_numeric_expression(callee)?;
        self.state.emission.output.instructions.push(0x1a);
        for argument in arguments {
            match argument {
                CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                    self.emit_numeric_expression(expression)?;
                    self.state.emission.output.instructions.push(0x1a);
                }
            }
        }
        self.push_i32_const(JS_UNDEFINED_TAG);
        Ok(())
    }
}
