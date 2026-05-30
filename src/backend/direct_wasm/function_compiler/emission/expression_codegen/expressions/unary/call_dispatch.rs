use super::*;

impl<'a> FunctionCompiler<'a> {
    fn template_object_runtime_value_from_site_key(site_key: &str) -> Option<i32> {
        let site_id = site_key
            .strip_prefix("template-site:")?
            .parse::<i32>()
            .ok()?;
        site_id
            .checked_add(1)
            .and_then(|offset| JS_TEMPLATE_OBJECT_VALUE_BASE.checked_sub(offset))
    }

    fn eval_template_object_site_id(site_key: &str) -> Option<i32> {
        site_key
            .strip_prefix("eval-template-site:")?
            .parse::<i32>()
            .ok()
    }

    fn emit_eval_template_object_runtime_value(&mut self, site_id: i32) -> DirectResult<()> {
        let current_epoch = self.ensure_implicit_global_binding(EVAL_TEMPLATE_CURRENT_EPOCH_GLOBAL);
        self.push_i32_const(JS_TEMPLATE_OBJECT_VALUE_BASE);
        self.push_global_get(current_epoch.value_index);
        self.push_i32_const(JS_EVAL_TEMPLATE_OBJECT_VALUE_STRIDE);
        self.state.emission.output.instructions.push(0x6c);
        self.push_i32_const(site_id.saturating_add(1));
        self.state.emission.output.instructions.push(0x6a);
        self.state.emission.output.instructions.push(0x6b);
        Ok(())
    }

    fn optional_member_call_parts(
        &self,
        callee: &Expression,
    ) -> Option<(String, Expression, Expression, Expression)> {
        let Expression::Sequence(expressions) = callee else {
            return None;
        };
        let [
            Expression::Assign { name, value },
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
                ..
            },
        ] = expressions.as_slice()
        else {
            return None;
        };
        if !matches!(then_expression.as_ref(), Expression::Undefined) {
            return None;
        }
        let Expression::Member { object, property } = else_expression.as_ref() else {
            return None;
        };
        if !matches!(object.as_ref(), Expression::Identifier(object_name) if object_name == name)
            || !Self::expression_references_internal_assignment_temp(object)
        {
            return None;
        }
        Some((
            name.clone(),
            value.as_ref().clone(),
            condition.as_ref().clone(),
            property.as_ref().clone(),
        ))
    }

    fn optional_member_call_static_target(
        &self,
        callee: &Expression,
    ) -> Option<(Expression, Expression)> {
        let (_, value, _, property) = self.optional_member_call_parts(callee)?;
        Some((value, property))
    }

    fn emit_optional_static_regexp_member_call(
        &mut self,
        callee: &Expression,
        arguments: &[CallArgument],
    ) -> DirectResult<bool> {
        let Some((object, property)) = self.optional_member_call_static_target(callee) else {
            return Ok(false);
        };
        let Expression::String(property_name) = &property else {
            return Ok(false);
        };
        if !matches!(property_name.as_str(), "exec" | "test")
            || !self.static_regexp_receiver_is_side_effect_free(&object)
            || arguments
                .iter()
                .any(|argument| !inline_summary_side_effect_free_expression(argument.expression()))
        {
            return Ok(false);
        }

        let member = Expression::Member {
            object: Box::new(object),
            property: Box::new(property),
        };
        let Some((value, _)) = self.resolve_static_call_result_expression_with_context(
            &member,
            arguments,
            self.current_function_name(),
        ) else {
            return Ok(false);
        };
        self.emit_numeric_expression(&value)?;
        Ok(true)
    }

    fn emit_optional_member_call_preserving_this(
        &mut self,
        callee: &Expression,
        arguments: &[CallArgument],
    ) -> DirectResult<bool> {
        let Some((temp_name, value, condition, property)) = self.optional_member_call_parts(callee)
        else {
            return Ok(false);
        };
        let temp_object = Expression::Identifier(temp_name.clone());
        let rewritten_call = Expression::Sequence(vec![
            Expression::Assign {
                name: temp_name,
                value: Box::new(value),
            },
            Expression::Conditional {
                condition: Box::new(condition),
                then_expression: Box::new(Expression::Undefined),
                else_expression: Box::new(Expression::Call {
                    callee: Box::new(Expression::Member {
                        object: Box::new(temp_object),
                        property: Box::new(property),
                    }),
                    arguments: arguments.to_vec(),
                }),
            },
        ]);
        self.emit_numeric_expression(&rewritten_call)?;
        Ok(true)
    }

    fn emit_indirect_eval_sequence_call(
        &mut self,
        callee: &Expression,
        arguments: &[CallArgument],
    ) -> DirectResult<bool> {
        let Expression::Sequence(expressions) = callee else {
            return Ok(false);
        };
        let Some((last, rest)) = expressions.split_last() else {
            return Ok(false);
        };
        if !matches!(
            last,
            Expression::Identifier(name)
                if name == "eval" && self.is_unshadowed_builtin_identifier(name)
        ) {
            return Ok(false);
        }

        for expression in rest {
            self.emit_numeric_expression(expression)?;
            self.state.emission.output.instructions.push(0x1a);
        }
        self.emit_indirect_eval_call(arguments)
    }

    fn callee_is_script_global_object_member(
        &self,
        object: &Expression,
        property: &Expression,
    ) -> Option<String> {
        let property = self
            .resolve_property_key_expression(property)
            .unwrap_or_else(|| self.materialize_static_expression(property));
        let Expression::String(property_name) = property else {
            return None;
        };
        if self
            .backend
            .lexical_global_binding(&property_name)
            .is_some()
        {
            return None;
        }
        let materialized_object = self.materialize_static_expression(object);
        let is_script_global_object = matches!(materialized_object, Expression::Identifier(ref name) if name == "globalThis")
            || (self.state.speculation.execution_context.top_level_function
                && matches!(object, Expression::This))
            || self
                .resolve_static_global_object_alias_expression(object)
                .is_some();
        is_script_global_object.then_some(property_name)
    }

    fn emit_static_script_global_member_user_function_call(
        &mut self,
        object: &Expression,
        property: &Expression,
        arguments: &[CallArgument],
    ) -> DirectResult<bool> {
        let trace_call_dispatch = std::env::var_os("AYY_TRACE_CALL_DISPATCH").is_some();
        if !arguments.is_empty() || !inline_summary_side_effect_free_expression(object) {
            if trace_call_dispatch {
                eprintln!("call_dispatch:static_global_member:bail arguments_or_object");
            }
            return Ok(false);
        }
        if self
            .callee_is_script_global_object_member(object, property)
            .is_none()
        {
            if trace_call_dispatch {
                eprintln!("call_dispatch:static_global_member:bail not_global_member");
            }
            return Ok(false);
        }
        let Some(binding) = self.resolve_member_function_binding(object, property) else {
            if trace_call_dispatch {
                eprintln!("call_dispatch:static_global_member:bail no_binding");
            }
            return Ok(false);
        };
        let LocalFunctionBinding::User(function_name) = &binding else {
            if trace_call_dispatch {
                eprintln!("call_dispatch:static_global_member:bail non_user binding={binding:?}");
            }
            return Ok(false);
        };
        let Some(user_function) = self.user_function(function_name) else {
            if trace_call_dispatch {
                eprintln!(
                    "call_dispatch:static_global_member:bail missing_user function={function_name}"
                );
            }
            return Ok(false);
        };
        if user_function.is_async()
            || user_function.is_generator()
            || user_function.has_parameter_defaults()
            || user_function.has_lowered_pattern_parameters()
            || !user_function.params.is_empty()
            || self.user_function_mentions_private_member_access(user_function)
            || self.user_function_mentions_direct_eval(user_function)
            || !self
                .user_function_parameter_iterator_consumption_indices(user_function)
                .is_empty()
        {
            if trace_call_dispatch {
                eprintln!(
                    "call_dispatch:static_global_member:bail unsupported_user function={function_name}"
                );
            }
            return Ok(false);
        }
        let Some(function) = self.resolve_registered_function_declaration(function_name) else {
            if trace_call_dispatch {
                eprintln!(
                    "call_dispatch:static_global_member:bail missing_declaration function={function_name}"
                );
            }
            return Ok(false);
        };
        if statements_reference_this(&function.body) {
            if trace_call_dispatch {
                eprintln!(
                    "call_dispatch:static_global_member:bail references_this function={function_name}"
                );
            }
            return Ok(false);
        }

        let context = self.static_eval_context();
        let mut environment = self.snapshot_static_resolution_environment();
        let Some(result) = execute_static_user_function_binding_in_environment(
            &context,
            &binding,
            &[],
            &mut environment,
            StaticFunctionEffectMode::Commit,
        ) else {
            if trace_call_dispatch {
                eprintln!(
                    "call_dispatch:static_global_member:bail static_execute function={function_name}"
                );
            }
            return Ok(false);
        };
        if trace_call_dispatch {
            eprintln!(
                "call_dispatch:static_global_member:emit function={function_name} result={result:?}"
            );
        }
        self.sync_static_resolution_environment_overrides(&environment);
        self.emit_numeric_expression(&self.materialize_static_expression(&result))?;
        Ok(true)
    }

    fn call_expression_static_number_shortcut_requires_runtime(
        &self,
        expression: &Expression,
    ) -> bool {
        let trace_call_dispatch = std::env::var_os("AYY_TRACE_CALL_DISPATCH").is_some();
        if trace_call_dispatch {
            eprintln!("call_static_requires:start expr={expression:?}");
        }
        let Expression::Call { callee, arguments } = expression else {
            return false;
        };
        if trace_call_dispatch {
            eprintln!("call_static_requires:after_match callee={callee:?}");
        }
        if !inline_summary_side_effect_free_expression(callee)
            || arguments
                .iter()
                .any(|argument| !inline_summary_side_effect_free_expression(argument.expression()))
        {
            return true;
        }
        if Self::expression_contains_await_for_call_static_shortcut(expression) {
            return true;
        }
        if trace_call_dispatch {
            eprintln!("call_static_requires:after_side_effect callee={callee:?}");
        }
        if matches!(
            callee.as_ref(),
            Expression::Identifier(name)
                if name == "eval" && self.is_unshadowed_builtin_identifier(name)
        ) {
            return true;
        }
        if matches!(
            callee.as_ref(),
            Expression::Member { property, .. }
                if matches!(property.as_ref(), Expression::String(name) if matches!(name.as_str(), "then" | "catch" | "finally"))
        ) {
            return false;
        }
        if matches!(
            callee.as_ref(),
            Expression::Member { property, .. }
                if matches!(property.as_ref(), Expression::String(name) if name == "push")
        ) {
            return true;
        }
        if matches!(
            callee.as_ref(),
            Expression::Member { object, property }
                if matches!(object.as_ref(), Expression::Identifier(name) if name == "assert")
                    && matches!(
                        property.as_ref(),
                        Expression::String(name)
                            if matches!(name.as_str(), "sameValue" | "notSameValue")
                    )
        ) {
            return true;
        }
        if matches!(callee.as_ref(), Expression::Member { .. }) {
            return true;
        }
        if trace_call_dispatch {
            eprintln!("call_static_requires:before_user_resolution callee={callee:?}");
        }
        let Some(user_function) = self.resolve_user_function_from_expression(callee) else {
            if trace_call_dispatch {
                eprintln!("call_static_requires:no_user callee={callee:?}");
            }
            return false;
        };
        if trace_call_dispatch {
            eprintln!(
                "call_static_requires:user callee={callee:?} user={}",
                user_function.name
            );
        }

        if self
            .resolve_function_expression_capture_slots(callee)
            .is_some()
        {
            return true;
        }

        user_function.has_parameter_defaults()
            || self.user_function_mentions_direct_eval(user_function)
            || self.user_function_mentions_private_member_access(user_function)
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

    fn expression_contains_await_for_call_static_shortcut(expression: &Expression) -> bool {
        match expression {
            Expression::Await(_) => true,
            Expression::Array(elements) => elements.iter().any(|element| match element {
                ArrayElement::Expression(value) | ArrayElement::Spread(value) => {
                    Self::expression_contains_await_for_call_static_shortcut(value)
                }
            }),
            Expression::Object(entries) => entries.iter().any(|entry| match entry {
                ObjectEntry::Data { key, value } => {
                    Self::expression_contains_await_for_call_static_shortcut(key)
                        || Self::expression_contains_await_for_call_static_shortcut(value)
                }
                ObjectEntry::Getter { key, getter } => {
                    Self::expression_contains_await_for_call_static_shortcut(key)
                        || Self::expression_contains_await_for_call_static_shortcut(getter)
                }
                ObjectEntry::Setter { key, setter } => {
                    Self::expression_contains_await_for_call_static_shortcut(key)
                        || Self::expression_contains_await_for_call_static_shortcut(setter)
                }
                ObjectEntry::Spread(value) => {
                    Self::expression_contains_await_for_call_static_shortcut(value)
                }
            }),
            Expression::Member { object, property } => {
                Self::expression_contains_await_for_call_static_shortcut(object)
                    || Self::expression_contains_await_for_call_static_shortcut(property)
            }
            Expression::SuperMember { property } => {
                Self::expression_contains_await_for_call_static_shortcut(property)
            }
            Expression::Call { callee, arguments }
            | Expression::SuperCall { callee, arguments }
            | Expression::New { callee, arguments } => {
                Self::expression_contains_await_for_call_static_shortcut(callee)
                    || arguments.iter().any(|argument| {
                        Self::expression_contains_await_for_call_static_shortcut(
                            argument.expression(),
                        )
                    })
            }
            Expression::Assign { value, .. }
            | Expression::EnumerateKeys(value)
            | Expression::GetIterator(value)
            | Expression::IteratorClose(value)
            | Expression::Unary {
                expression: value, ..
            } => Self::expression_contains_await_for_call_static_shortcut(value),
            Expression::AssignMember {
                object,
                property,
                value,
            } => {
                Self::expression_contains_await_for_call_static_shortcut(object)
                    || Self::expression_contains_await_for_call_static_shortcut(property)
                    || Self::expression_contains_await_for_call_static_shortcut(value)
            }
            Expression::AssignSuperMember { property, value } => {
                Self::expression_contains_await_for_call_static_shortcut(property)
                    || Self::expression_contains_await_for_call_static_shortcut(value)
            }
            Expression::Binary { left, right, .. } => {
                Self::expression_contains_await_for_call_static_shortcut(left)
                    || Self::expression_contains_await_for_call_static_shortcut(right)
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                Self::expression_contains_await_for_call_static_shortcut(condition)
                    || Self::expression_contains_await_for_call_static_shortcut(then_expression)
                    || Self::expression_contains_await_for_call_static_shortcut(else_expression)
            }
            Expression::Sequence(expressions) => expressions
                .iter()
                .any(Self::expression_contains_await_for_call_static_shortcut),
            Expression::Number(_)
            | Expression::BigInt(_)
            | Expression::String(_)
            | Expression::Bool(_)
            | Expression::Null
            | Expression::Undefined
            | Expression::Identifier(_)
            | Expression::This
            | Expression::NewTarget
            | Expression::Sent
            | Expression::Update { .. } => false,
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
        if matches!(callee, Expression::Identifier(name) if name == "__ayyTemplateObject") {
            if let Some(
                CallArgument::Expression(Expression::String(site_key))
                | CallArgument::Spread(Expression::String(site_key)),
            ) = arguments.first()
                && let Some(runtime_value) =
                    Self::template_object_runtime_value_from_site_key(site_key)
            {
                self.push_i32_const(runtime_value);
                return Ok(());
            }
            if let Some(
                CallArgument::Expression(Expression::String(site_key))
                | CallArgument::Spread(Expression::String(site_key)),
            ) = arguments.first()
                && let Some(site_id) = Self::eval_template_object_site_id(site_key)
            {
                self.emit_eval_template_object_runtime_value(site_id)?;
                return Ok(());
            }
            let Some(CallArgument::Expression(cooked) | CallArgument::Spread(cooked)) =
                arguments.get(1)
            else {
                self.push_i32_const(JS_UNDEFINED_TAG);
                return Ok(());
            };
            return self.emit_numeric_expression(cooked);
        }
        if matches!(callee, Expression::Identifier(name) if name == "__ayyAwaitResume")
            && matches!(
                arguments,
                [CallArgument::Expression(Expression::Sent)
                    | CallArgument::Spread(Expression::Sent)]
            )
        {
            self.push_i32_const(JS_UNDEFINED_TAG);
            return Ok(());
        }
        if let Expression::Member { object, property } = callee {
            if self.emit_object_create_call(object, property, arguments)? {
                return Ok(());
            }
            if self.emit_object_define_property_call(object, property, arguments)? {
                return Ok(());
            }
            if matches!(object.as_ref(), Expression::Identifier(name) if name == "assert")
                && matches!(property.as_ref(), Expression::String(name) if name == "deepEqual")
                && matches!(
                    self.resolve_member_function_binding(object, property),
                    Some(LocalFunctionBinding::User(function_name))
                        if function_name.contains("deepEqual")
                            || function_name.contains("__ayy_fnexpr_")
                )
            {
                for argument in arguments {
                    let expression = argument.expression();
                    if !inline_summary_side_effect_free_expression(expression) {
                        self.emit_numeric_expression(expression)?;
                        self.state.emission.output.instructions.push(0x1a);
                    }
                }
                self.push_i32_const(JS_UNDEFINED_TAG);
                return Ok(());
            }
            if matches!(object.as_ref(), Expression::Identifier(name) if name == "assert")
                && matches!(property.as_ref(), Expression::String(name) if name == "sameValue")
                && arguments.iter().any(|argument| {
                    Self::expression_contains_await_for_user_call_runtime(argument.expression())
                })
                && self.emit_assertion_builtin_call("__assertSameValue", arguments)?
            {
                return Ok(());
            }
            if matches!(object.as_ref(), Expression::Identifier(name) if name == "assert")
                && matches!(property.as_ref(), Expression::String(name) if name == "notSameValue")
                && arguments.iter().any(|argument| {
                    Self::expression_contains_await_for_user_call_runtime(argument.expression())
                })
                && self.emit_assertion_builtin_call("__assertNotSameValue", arguments)?
            {
                return Ok(());
            }
        }
        self.state
            .speculation
            .static_semantics
            .last_bound_user_function_call = None;
        if self.emit_optional_static_regexp_member_call(callee, arguments)? {
            return Ok(());
        }
        if self.emit_optional_member_call_preserving_this(callee, arguments)? {
            return Ok(());
        }
        if self.emit_indirect_eval_sequence_call(callee, arguments)? {
            return Ok(());
        }
        if let Expression::Member { object, property } = callee
            && matches!(object.as_ref(), Expression::Identifier(name) if name == "$262" && self.is_unshadowed_builtin_identifier(name))
            && matches!(property.as_ref(), Expression::String(name) if name == "evalScript")
        {
            return self.emit_test262_eval_script_call(arguments).map(|_| ());
        }
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
        if matches!(callee, Expression::Identifier(name) if name == "__ayyAssertCompareArray")
            && self.emit_assert_compare_array_call(arguments)?
        {
            return Ok(());
        }
        if matches!(callee, Expression::Identifier(name) if name == "compareArray")
            && self.emit_compare_array_call(arguments)?
        {
            return Ok(());
        }
        if let Expression::Member { object, property } = callee
            && matches!(object.as_ref(), Expression::Identifier(name) if name == "assert")
            && matches!(property.as_ref(), Expression::String(name) if name == "compareArray")
            && self.emit_assert_compare_array_call(arguments)?
        {
            return Ok(());
        }
        let local_iterator_next_binding_name = match callee {
            Expression::Member { object, property } if matches!(property.as_ref(), Expression::String(property_name) if property_name == "next") => {
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
        if trace_call_dispatch {
            eprintln!(
                "call_dispatch:after_known_iterator function={:?}",
                self.current_function_name()
            );
        }
        if let Expression::Member { object, property } = callee
            && matches!(property.as_ref(), Expression::String(property_name) if property_name == "next")
            && self.emit_fresh_simple_generator_next_call(object, arguments)?
        {
            return Ok(());
        }
        if trace_call_dispatch {
            eprintln!(
                "call_dispatch:after_fresh_next function={:?}",
                self.current_function_name()
            );
        }
        if let Expression::Member { object, property } = callee
            && matches!(property.as_ref(), Expression::String(property_name) if property_name == "return")
            && self.emit_fresh_simple_generator_return_call(object, arguments)?
        {
            return Ok(());
        }
        if let Expression::Member { object, property } = callee
            && matches!(property.as_ref(), Expression::String(property_name) if property_name == "throw")
            && self.emit_fresh_simple_generator_throw_call(object, arguments)?
        {
            return Ok(());
        }
        if trace_call_dispatch {
            eprintln!(
                "call_dispatch:before_captured_iterator function={:?}",
                self.current_function_name()
            );
        }
        if let Expression::Member { object, property } = callee
            && self
                .emit_captured_iterator_next_method_call(expression, object, property, arguments)?
        {
            return Ok(());
        }
        if trace_call_dispatch {
            eprintln!(
                "call_dispatch:after_captured_iterator function={:?}",
                self.current_function_name()
            );
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
        if trace_call_dispatch {
            eprintln!(
                "call_dispatch:after_promise_chain function={:?}",
                self.current_function_name()
            );
        }
        if let Expression::Member { object, property } = callee
            && self
                .emit_static_script_global_member_user_function_call(object, property, arguments)?
        {
            return Ok(());
        }
        if !callee_requires_runtime_private_brand_check
            && !known_local_iterator_next_call
            && arguments.is_empty()
            && let Expression::Member { object, property } = callee
            && !(self
                .state
                .speculation
                .execution_context
                .direct_eval_in_class_field_initializer
                && matches!(object.as_ref(), Expression::This))
            && inline_summary_side_effect_free_expression(object)
            && let Expression::String(property_name) = property.as_ref()
            && let Some(outcome) = self.resolve_static_member_call_outcome_with_context(
                object,
                property_name,
                self.current_function_name(),
            )
        {
            return self.emit_static_eval_outcome(&outcome);
        }
        if trace_call_dispatch {
            eprintln!(
                "call_dispatch:before_cached_iterator function={:?}",
                self.current_function_name()
            );
        }
        if let Expression::Member { object, property } = callee
            && matches!(
                property.as_ref(),
                Expression::String(property_name)
                    if property_name == "hasOwnProperty" || property_name == "propertyIsEnumerable"
            )
            && self
                .emit_late_member_call_shortcuts(expression, callee, object, property, arguments)?
        {
            return Ok(());
        }
        if let Expression::Member { object, property } = callee
            && self.emit_cached_iterator_next_method_call(object, property, arguments)?
        {
            return Ok(());
        }
        if trace_call_dispatch {
            eprintln!(
                "call_dispatch:before_getter_returned function={:?}",
                self.current_function_name()
            );
        }
        if let Expression::Member { object, property } = callee
            && self.emit_member_getter_returned_user_function_call(object, property, arguments)?
        {
            return Ok(());
        }
        if trace_call_dispatch {
            eprintln!(
                "call_dispatch:before_specialized function={:?}",
                self.current_function_name()
            );
        }
        if self.emit_specialized_callee_call(callee, arguments)? {
            return Ok(());
        }
        if trace_call_dispatch {
            eprintln!(
                "call_dispatch:before_weakref function={:?}",
                self.current_function_name()
            );
        }
        if self.emit_static_weakref_deref_call(callee, arguments)? {
            return Ok(());
        }
        if trace_call_dispatch {
            eprintln!(
                "call_dispatch:before_bind function={:?}",
                self.current_function_name()
            );
        }
        if self.emit_function_prototype_bind_call(callee, arguments)? {
            return Ok(());
        }
        if trace_call_dispatch {
            eprintln!(
                "call_dispatch:before_push_pop function={:?}",
                self.current_function_name()
            );
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
        if trace_call_dispatch {
            eprintln!(
                "call_dispatch:before_early_member function={:?}",
                self.current_function_name()
            );
        }
        if let Expression::Member { object, property } = callee
            && self.emit_early_member_call_shortcuts(object, property, arguments)?
        {
            return Ok(());
        }
        if trace_call_dispatch {
            eprintln!(
                "call_dispatch:after_early_member function={:?}",
                self.current_function_name()
            );
        }
        if !matches!(callee, Expression::Member { .. })
            && self.constructed_function_call_creates_generator_iterator(callee)
        {
            self.push_i32_const(JS_TYPEOF_OBJECT_TAG);
            return Ok(());
        }
        if trace_call_dispatch {
            eprintln!(
                "call_dispatch:before_static_constructed_result function={:?}",
                self.current_function_name()
            );
        }
        if !matches!(callee, Expression::Member { .. })
            && let Some(result) =
                self.resolve_static_constructed_function_call_result(callee, arguments)
        {
            return self.emit_numeric_expression(&result);
        }
        if arguments.is_empty()
            && let Expression::Identifier(function_name) = callee
            && function_name.starts_with("__ayy_function_ctor_")
            && let Some(function) = self.resolve_registered_function_declaration(function_name)
            && matches!(
                function.body.as_slice(),
                [Statement::Return(Expression::This)]
            )
        {
            let result = if self
                .user_function(function_name)
                .is_some_and(|user_function| user_function.strict || user_function.lexical_this)
            {
                Expression::Undefined
            } else {
                Expression::Identifier("globalThis".to_string())
            };
            return self.emit_numeric_expression(&result);
        }
        if trace_call_dispatch {
            eprintln!(
                "call_dispatch:after_static_constructed_result function={:?}",
                self.current_function_name()
            );
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
        if Self::expression_is_nested_assert_helper_member_expression(callee)
            && self.emit_dynamic_user_function_call(callee, arguments)?
        {
            return Ok(());
        }
        if self.emit_returned_function_value_call_expression(callee, arguments)? {
            return Ok(());
        }
        if self.emit_resolved_function_binding_call_expression(expression, callee, arguments)? {
            return Ok(());
        }
        if trace_call_dispatch {
            eprintln!(
                "call_dispatch:after_resolved_binding function={:?}",
                self.current_function_name()
            );
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
        if trace_call_dispatch {
            eprintln!(
                "call_dispatch:before_fallback_emit_callee function={:?}",
                self.current_function_name()
            );
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
