use super::*;

fn simple_return_expression_from_statements(statements: &[Statement]) -> Option<&Expression> {
    let [statement] = statements else {
        return None;
    };
    simple_return_expression_from_statement(statement)
}

fn simple_return_expression_from_statement(statement: &Statement) -> Option<&Expression> {
    match statement {
        Statement::Return(expression) => Some(expression),
        Statement::Declaration { body } | Statement::Block { body } => {
            simple_return_expression_from_statements(body)
        }
        _ => None,
    }
}

impl<'a> FunctionCompiler<'a> {
    pub(in crate::backend::direct_wasm) fn generated_function_statement_source_name(
        function_name: &str,
    ) -> Option<&str> {
        function_name
            .strip_prefix("__ayy_fnstmt_")
            .and_then(|rest| rest.rsplit_once('_'))
            .map(|(source_name, _)| source_name)
    }

    pub(in crate::backend::direct_wasm) fn function_statement_binding_name_for_source(
        &self,
        source_name: &str,
    ) -> Option<String> {
        self.user_functions().into_iter().find_map(|function| {
            (Self::generated_function_statement_source_name(&function.name) == Some(source_name))
                .then_some(function.name)
        })
    }

    pub(in crate::backend::direct_wasm) fn current_function_statement_binding_name_for_source(
        &self,
        source_name: &str,
    ) -> Option<String> {
        let current_function_name = self.current_function_name()?;
        (Self::generated_function_statement_source_name(current_function_name) == Some(source_name))
            .then(|| current_function_name.to_string())
    }

    pub(in crate::backend::direct_wasm) fn simple_zero_arg_function_statement_return_expression(
        &self,
        binding_name: &str,
    ) -> Option<&Expression> {
        let source_name = scoped_binding_source_name(binding_name).unwrap_or(binding_name);
        let function_name = self.function_statement_binding_name_for_source(source_name)?;
        let declaration = self.prepared_function_declaration(&function_name)?;
        if !declaration.params.is_empty() {
            return None;
        }
        simple_return_expression_from_statements(&declaration.body)
    }

    pub(in crate::backend::direct_wasm) fn resolve_user_function_capture_slot_binding_name(
        &self,
        capture_name: &str,
    ) -> Option<String> {
        let capture_source_name = scoped_binding_source_name(capture_name).unwrap_or(capture_name);
        if capture_name == "this" {
            return self
                .resolve_user_function_capture_hidden_name("this")
                .or_else(|| self.resolve_eval_local_function_hidden_name("this"))
                .or_else(|| Some("this".to_string()));
        }

        self.resolve_current_local_binding(capture_name)
            .map(|(resolved_name, _)| resolved_name)
            .or_else(|| {
                self.resolve_current_local_binding(capture_source_name)
                    .map(|(resolved_name, _)| resolved_name)
            })
            .or_else(|| {
                self.current_function_statement_binding_name_for_source(capture_source_name)
            })
            .or_else(|| self.resolve_user_function_capture_hidden_name(capture_name))
            .or_else(|| self.resolve_eval_local_function_hidden_name(capture_name))
            .or_else(|| {
                (self.global_has_binding(capture_source_name)
                    || self.backend.global_has_lexical_binding(capture_source_name)
                    || self
                        .backend
                        .global_function_binding(capture_source_name)
                        .is_some()
                    || self.global_has_implicit_binding(capture_source_name))
                .then_some(capture_source_name.to_string())
            })
    }

    fn resolve_user_function_capture_slots_by_name(
        &self,
        function_name: &str,
        expression: &Expression,
    ) -> Option<BTreeMap<String, String>> {
        let trace_private = std::env::var_os("AYY_TRACE_PRIVATE_MEMBER_LOOKUP").is_some();
        let capture_bindings = self.user_function_capture_bindings(function_name)?;
        if capture_bindings.is_empty() {
            return None;
        }
        let mut capture_names = capture_bindings.keys().cloned().collect::<Vec<_>>();
        capture_names.sort();
        let mut capture_slots = BTreeMap::new();
        for capture_name in capture_names {
            let slot_name = self.resolve_user_function_capture_slot_binding_name(&capture_name);
            let Some(slot_name) = slot_name else {
                if trace_private {
                    eprintln!(
                        "private_lookup capture_slots current_fn={:?} callee={:?} target={} capture_name={} slot=None",
                        self.current_function_name(),
                        expression,
                        function_name,
                        capture_name,
                    );
                }
                return None;
            };
            capture_slots.insert(capture_name, slot_name);
        }
        if trace_private {
            eprintln!(
                "private_lookup capture_slots current_fn={:?} callee={:?} target={} slots={:?}",
                self.current_function_name(),
                expression,
                function_name,
                capture_slots,
            );
        }
        Some(capture_slots)
    }

    pub(in crate::backend::direct_wasm) fn expand_apply_call_arguments_from_expression(
        &self,
        expression: &Expression,
    ) -> Option<Vec<CallArgument>> {
        let materialized = self.materialize_static_expression(expression);
        match &materialized {
            Expression::Undefined | Expression::Null => Some(Vec::new()),
            _ => {
                if let Some(array_binding) =
                    self.resolve_array_binding_from_expression(&materialized)
                {
                    return Some(
                        array_binding
                            .values
                            .into_iter()
                            .map(|value| {
                                CallArgument::Expression(value.unwrap_or(Expression::Undefined))
                            })
                            .collect(),
                    );
                }
                self.resolve_arguments_binding_from_expression(&materialized)
                    .map(|binding| {
                        binding
                            .values
                            .into_iter()
                            .map(CallArgument::Expression)
                            .collect()
                    })
            }
        }
    }

    pub(in crate::backend::direct_wasm) fn resolve_function_expression_capture_slots(
        &self,
        expression: &Expression,
    ) -> Option<BTreeMap<String, String>> {
        if let Expression::Identifier(name) = expression
            && let Some(capture_slots) = self.resolve_identifier_function_value_capture_slots(name)
        {
            return Some(capture_slots);
        }
        if let Expression::Member { object, property } = expression
            && let Some(capture_slots) =
                self.resolve_member_function_capture_slots(object, property)
        {
            return Some(capture_slots);
        }
        if let Some(resolved) = self
            .resolve_bound_alias_expression(expression)
            .filter(|resolved| !static_expression_matches(resolved, expression))
            && let Some(capture_slots) = self.resolve_function_expression_capture_slots(&resolved)
        {
            return Some(capture_slots);
        }
        if let Some(user_function) = self.resolve_user_function_from_expression(expression)
            && let Some(capture_slots) =
                self.resolve_user_function_capture_slots_by_name(&user_function.name, expression)
        {
            return Some(capture_slots);
        }
        None
    }

    pub(in crate::backend::direct_wasm) fn should_box_sloppy_function_this(
        &self,
        user_function: &UserFunction,
        this_expression: &Expression,
    ) -> bool {
        if user_function.strict || user_function.lexical_this {
            return false;
        }
        matches!(
            self.infer_value_kind(this_expression),
            Some(
                StaticValueKind::Undefined
                    | StaticValueKind::Null
                    | StaticValueKind::Number
                    | StaticValueKind::BigInt
                    | StaticValueKind::String
                    | StaticValueKind::Bool
                    | StaticValueKind::Symbol
            )
        )
    }

    pub(in crate::backend::direct_wasm) fn static_sloppy_function_this_binding(
        &self,
        user_function: &UserFunction,
        this_expression: &Expression,
    ) -> Option<Expression> {
        if user_function.strict || user_function.lexical_this {
            return None;
        }
        let primitive = self
            .resolve_static_primitive_expression_with_context(
                this_expression,
                self.current_function_name(),
            )
            .or_else(|| self.resolve_static_boxed_primitive_value(this_expression))?;
        let constructor_name = match primitive {
            Expression::Number(_) => "Number",
            Expression::String(_) => "String",
            Expression::Bool(_) => "Boolean",
            Expression::BigInt(_) => "Object",
            Expression::Undefined | Expression::Null => return Some(Expression::This),
            _ if self.infer_value_kind(&primitive) == Some(StaticValueKind::Symbol) => "Object",
            _ => return None,
        };
        Some(Expression::New {
            callee: Box::new(Expression::Identifier(constructor_name.to_string())),
            arguments: vec![CallArgument::Expression(primitive)],
        })
    }

    pub(in crate::backend::direct_wasm) fn emit_user_function_call_with_function_this_binding(
        &mut self,
        user_function: &UserFunction,
        arguments: &[CallArgument],
        this_expression: &Expression,
        capture_slots: Option<&BTreeMap<String, String>>,
    ) -> DirectResult<()> {
        let trace_private = std::env::var_os("AYY_TRACE_PRIVATE_MEMBER_LOOKUP").is_some();
        let expanded_arguments = self.expand_call_arguments(arguments);
        let simple_generator_call = Expression::Call {
            callee: Box::new(Expression::Identifier(user_function.name.clone())),
            arguments: expanded_arguments
                .iter()
                .cloned()
                .map(CallArgument::Expression)
                .collect(),
        };
        if user_function.is_generator()
            && self
                .resolve_simple_generator_source(&simple_generator_call)
                .is_some()
        {
            self.emit_simple_generator_call_time_prefix_effects(&simple_generator_call)?;
            self.push_i32_const(JS_TYPEOF_OBJECT_TAG);
            return Ok(());
        }
        if capture_slots.is_none()
            && !self.should_box_sloppy_function_this(user_function, this_expression)
        {
            let materialized_this_expression = self
                .with_suspended_with_scopes_if_active_scope_object(this_expression, |compiler| {
                    Ok(compiler.materialize_static_expression(this_expression))
                })?;
            let materialized_call_arguments = expanded_arguments
                .iter()
                .map(|argument| self.materialize_static_expression(argument))
                .collect::<Vec<_>>();
            if trace_private {
                eprintln!(
                    "private_lookup user_call current_fn={:?} target={} capture_slots=None inlineable={}",
                    self.current_function_name(),
                    user_function.name,
                    self.can_inline_user_function_call_with_explicit_call_frame(
                        user_function,
                        &materialized_call_arguments,
                        &materialized_this_expression,
                    ),
                );
            }
            if self.can_inline_user_function_call_with_explicit_call_frame(
                user_function,
                &materialized_call_arguments,
                &materialized_this_expression,
            ) {
                let result_local = self.allocate_temp_local();
                if self.emit_inline_user_function_summary_with_explicit_call_frame(
                    user_function,
                    &expanded_arguments,
                    &materialized_this_expression,
                    result_local,
                )? {
                    self.push_local_get(result_local);
                    return Ok(());
                }
            }
        }
        if trace_private && capture_slots.is_some() {
            eprintln!(
                "private_lookup user_call current_fn={:?} target={} capture_slots={:?}",
                self.current_function_name(),
                user_function.name,
                capture_slots,
            );
        }
        let class_field_initializer_function = self
            .resolve_registered_function_declaration(&user_function.name)
            .is_some_and(|function| function.direct_eval_in_class_field_initializer);
        let adjusted_capture_slots = capture_slots.and_then(|capture_slots| {
            if !user_function.lexical_this
                || !class_field_initializer_function
                || !capture_slots
                    .get("this")
                    .is_some_and(|slot_name| slot_name == "this")
            {
                return None;
            }
            let receiver_owner =
                self.resolve_user_function_call_receiver_shadow_owner(this_expression)?;
            if receiver_owner == "this" {
                return None;
            }
            let mut adjusted = capture_slots.clone();
            adjusted.insert("this".to_string(), receiver_owner);
            Some(adjusted)
        });
        if let Some(capture_slots) = capture_slots {
            let capture_slots = adjusted_capture_slots.as_ref().unwrap_or(capture_slots);
            let effective_this_expression =
                if self.should_box_sloppy_function_this(user_function, this_expression) {
                    Expression::This
                } else {
                    this_expression.clone()
                };
            return self
                .emit_user_function_call_with_new_target_and_this_expression_and_bound_captures(
                    user_function,
                    arguments,
                    JS_UNDEFINED_TAG,
                    &effective_this_expression,
                    capture_slots,
                );
        }
        if self.should_box_sloppy_function_this(user_function, this_expression) {
            return self.emit_user_function_call_with_new_target_and_this(
                user_function,
                arguments,
                JS_UNDEFINED_TAG,
                JS_TYPEOF_OBJECT_TAG,
            );
        }
        self.emit_user_function_call_with_new_target_and_this_expression(
            user_function,
            arguments,
            JS_UNDEFINED_TAG,
            this_expression,
        )
    }

    pub(in crate::backend::direct_wasm) fn emit_function_binding_call_with_function_this_binding_from_argument_locals(
        &mut self,
        function_binding: &LocalFunctionBinding,
        argument_locals: &[u32],
        argument_count: usize,
        this_expression: &Expression,
    ) -> DirectResult<bool> {
        let LocalFunctionBinding::User(function_name) = function_binding else {
            return Ok(false);
        };
        let Some(user_function) = self.user_function(function_name).cloned() else {
            return Ok(false);
        };
        let callee_expression = Expression::Identifier(user_function.name.clone());
        let capture_slots = self
            .resolve_user_function_capture_slots_by_name(&user_function.name, &callee_expression);
        if let Some(capture_slots) = capture_slots.as_ref() {
            self.emit_user_function_call_with_new_target_and_this_expression_and_bound_captures_from_argument_locals(
                &user_function,
                argument_locals,
                argument_count,
                JS_UNDEFINED_TAG,
                this_expression,
                capture_slots,
            )?;
            return Ok(true);
        }
        self.emit_user_function_call_with_new_target_and_this_expression_from_argument_locals(
            &user_function,
            argument_locals,
            argument_count,
            JS_UNDEFINED_TAG,
            this_expression,
        )?;
        Ok(true)
    }
}
