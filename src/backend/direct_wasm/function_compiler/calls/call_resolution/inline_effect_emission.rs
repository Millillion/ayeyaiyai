use super::*;

impl<'a> FunctionCompiler<'a> {
    pub(in crate::backend::direct_wasm) fn emit_inline_user_function_effect_statement_with_explicit_call_frame(
        &mut self,
        statement: &Statement,
        user_function: &UserFunction,
        call_arguments: &[CallArgument],
        this_binding: &Expression,
        arguments_binding: &Expression,
        inline_local_bindings: &[String],
    ) -> DirectResult<bool> {
        let mut preserved_descriptor_binding_name = None;
        match statement {
            Statement::Var { name, value } => {
                let substituted_value = self.substitute_user_function_call_frame_bindings(
                    value,
                    user_function,
                    call_arguments,
                    this_binding,
                    arguments_binding,
                );
                if self
                    .resolve_descriptor_binding_from_expression(&substituted_value)
                    .is_some()
                {
                    preserved_descriptor_binding_name = Some(name.clone());
                }
                self.emit_statement(&Statement::Var {
                    name: name.clone(),
                    value: substituted_value,
                })?;
            }
            Statement::Let {
                name,
                mutable,
                value,
            } => {
                let substituted_value = self.substitute_user_function_call_frame_bindings(
                    value,
                    user_function,
                    call_arguments,
                    this_binding,
                    arguments_binding,
                );
                if self
                    .resolve_descriptor_binding_from_expression(&substituted_value)
                    .is_some()
                {
                    preserved_descriptor_binding_name = Some(name.clone());
                }
                self.emit_statement(&Statement::Let {
                    name: name.clone(),
                    mutable: *mutable,
                    value: substituted_value,
                })?;
            }
            Statement::Assign { name, value } => {
                let substituted_value = self.substitute_user_function_call_frame_bindings(
                    value,
                    user_function,
                    call_arguments,
                    this_binding,
                    arguments_binding,
                );
                if self
                    .resolve_descriptor_binding_from_expression(&substituted_value)
                    .is_some()
                {
                    preserved_descriptor_binding_name = Some(name.clone());
                }
                self.emit_statement(&Statement::Assign {
                    name: name.clone(),
                    value: substituted_value,
                })?;
            }
            Statement::AssignMember {
                object,
                property,
                value,
            } => {
                if let Some(parameter_name) =
                    self.call_frame_mapped_arguments_parameter_name(user_function, object, property)
                {
                    self.emit_statement(&Statement::Assign {
                        name: parameter_name.to_string(),
                        value: self.substitute_user_function_call_frame_bindings(
                            value,
                            user_function,
                            call_arguments,
                            this_binding,
                            arguments_binding,
                        ),
                    })?;
                } else {
                    self.emit_statement(&Statement::AssignMember {
                        object: self.substitute_user_function_call_frame_bindings(
                            object,
                            user_function,
                            call_arguments,
                            this_binding,
                            arguments_binding,
                        ),
                        property: self.substitute_user_function_call_frame_bindings(
                            property,
                            user_function,
                            call_arguments,
                            this_binding,
                            arguments_binding,
                        ),
                        value: self.substitute_user_function_call_frame_bindings(
                            value,
                            user_function,
                            call_arguments,
                            this_binding,
                            arguments_binding,
                        ),
                    })?;
                }
            }
            Statement::Expression(Expression::Update { name, op, prefix }) => {
                self.emit_numeric_expression(&Expression::Update {
                    name: name.clone(),
                    op: *op,
                    prefix: *prefix,
                })?;
                self.state.emission.output.instructions.push(0x1a);
            }
            Statement::Print { values } => {
                self.emit_statement(&Statement::Print {
                    values: values
                        .iter()
                        .map(|value| {
                            self.substitute_user_function_call_frame_bindings(
                                value,
                                user_function,
                                call_arguments,
                                this_binding,
                                arguments_binding,
                            )
                        })
                        .collect(),
                })?;
            }
            Statement::With { object, body } => {
                self.emit_statement(&Statement::With {
                    object: self.substitute_user_function_call_frame_bindings(
                        object,
                        user_function,
                        call_arguments,
                        this_binding,
                        arguments_binding,
                    ),
                    body: body
                        .iter()
                        .map(|statement| {
                            self.substitute_statement_call_frame_bindings(
                                statement,
                                user_function,
                                call_arguments,
                                this_binding,
                                arguments_binding,
                            )
                        })
                        .collect(),
                })?;
            }
            Statement::Expression(expression) => {
                let original_assertion_name = if let Expression::Call { callee, .. } = expression
                    && let Expression::Member { object, property } = callee.as_ref()
                    && matches!(object.as_ref(), Expression::Identifier(name) if name == "assert")
                    && let Expression::String(property_name) = property.as_ref()
                {
                    match property_name.as_str() {
                        "sameValue" => Some("__assertSameValue"),
                        "notSameValue" => Some("__assertNotSameValue"),
                        _ => None,
                    }
                } else {
                    None
                };
                let substituted = self.substitute_user_function_call_frame_bindings(
                    expression,
                    user_function,
                    call_arguments,
                    this_binding,
                    arguments_binding,
                );
                if let Some(assertion_name) = original_assertion_name
                    && let Expression::Call { arguments, .. } = &substituted
                    && self.emit_assertion_builtin_call(assertion_name, arguments)?
                {
                    self.state.emission.output.instructions.push(0x1a);
                    return Ok(true);
                }
                if let Expression::Call { callee, arguments } = &substituted
                    && let Expression::Identifier(name) = callee.as_ref()
                {
                    if name == "__ayyAssertCompareArray"
                        && self.emit_assert_compare_array_call(arguments)?
                    {
                        self.state.emission.output.instructions.push(0x1a);
                        return Ok(true);
                    }
                    if name == "compareArray" && self.emit_compare_array_call(arguments)? {
                        self.state.emission.output.instructions.push(0x1a);
                        return Ok(true);
                    }
                    if name == "verifyProperty" && self.emit_verify_property_call(arguments)? {
                        self.state.emission.output.instructions.push(0x1a);
                        return Ok(true);
                    }
                }
                if self
                    .user_function_references_only_direct_async_safe_captured_user_function_calls(
                        user_function,
                    )
                    && let Expression::Call { callee, arguments } = &substituted
                    && arguments.is_empty()
                    && let Some(LocalFunctionBinding::User(function_name)) = self
                        .resolve_function_binding_from_expression_with_context(
                            callee,
                            self.current_function_name(),
                        )
                    && self
                        .backend
                        .function_registry
                        .analysis
                        .user_function_capture_bindings
                        .contains_key(&function_name)
                    && let Some(called_function) = self.user_function(&function_name).cloned()
                    && !called_function.is_async()
                    && !called_function.is_generator()
                    && self.emit_no_arg_captured_user_function_effects_in_current_call_frame(
                        &called_function,
                    )?
                {
                    return Ok(true);
                }
                self.emit_numeric_expression(&substituted)?;
                self.state.emission.output.instructions.push(0x1a);
            }
            Statement::Throw(throw_value) => {
                let substituted = self.substitute_user_function_call_frame_bindings(
                    throw_value,
                    user_function,
                    call_arguments,
                    this_binding,
                    arguments_binding,
                );
                self.emit_statement(&Statement::Throw(substituted))?;
            }
            Statement::If {
                condition,
                then_branch,
                else_branch,
            } => {
                self.emit_statement(&Statement::If {
                    condition: self.substitute_user_function_call_frame_bindings(
                        condition,
                        user_function,
                        call_arguments,
                        this_binding,
                        arguments_binding,
                    ),
                    then_branch: then_branch
                        .iter()
                        .map(|statement| {
                            self.substitute_statement_call_frame_bindings(
                                statement,
                                user_function,
                                call_arguments,
                                this_binding,
                                arguments_binding,
                            )
                        })
                        .collect::<Vec<_>>(),
                    else_branch: else_branch
                        .iter()
                        .map(|statement| {
                            self.substitute_statement_call_frame_bindings(
                                statement,
                                user_function,
                                call_arguments,
                                this_binding,
                                arguments_binding,
                            )
                        })
                        .collect::<Vec<_>>(),
                })?;
            }
            Statement::Block { body } => {
                for statement in body {
                    if !self.emit_inline_user_function_effect_statement_with_explicit_call_frame(
                        statement,
                        user_function,
                        call_arguments,
                        this_binding,
                        arguments_binding,
                        inline_local_bindings,
                    )? {
                        return Ok(false);
                    }
                }
            }
            _ => return Ok(false),
        }
        if Self::statement_contains_runtime_call(statement) {
            self.invalidate_active_inline_local_descriptor_bindings_except(
                inline_local_bindings,
                preserved_descriptor_binding_name.as_deref(),
            );
        }
        Ok(true)
    }

    fn emit_no_arg_captured_user_function_effects_in_current_call_frame(
        &mut self,
        called_function: &UserFunction,
    ) -> DirectResult<bool> {
        if called_function.visible_param_count() != 0
            || !called_function.extra_argument_indices.is_empty()
            || called_function.has_parameter_defaults()
            || called_function.has_lowered_pattern_parameters()
            || called_function.is_async()
            || called_function.is_generator()
            || self.user_function_deletes_call_frame_arguments_member(called_function)
            || !self.user_function_has_explicit_call_frame_inlineable_terminal_body(called_function)
        {
            return Ok(false);
        }
        let Some(function) = self
            .resolve_registered_function_declaration(&called_function.name)
            .cloned()
        else {
            return Ok(false);
        };

        let inline_local_bindings =
            collect_declared_bindings_from_statements_recursive(&function.body)
                .into_iter()
                .filter(|name| name != "arguments")
                .collect::<Vec<_>>();
        let inline_local_scope_names =
            self.prepare_inline_summary_local_bindings(&inline_local_bindings);
        let call_arguments = Vec::new();
        let this_binding = Expression::Undefined;
        let arguments_binding = Expression::Array(Vec::new());

        self.with_scoped_lexical_bindings_cleanup(inline_local_scope_names, |compiler| {
            let Some((terminal_statement, effect_statements)) = function.body.split_last() else {
                return Ok(true);
            };
            for statement in effect_statements {
                if !compiler.emit_inline_user_function_effect_statement_with_explicit_call_frame(
                    statement,
                    called_function,
                    &call_arguments,
                    &this_binding,
                    &arguments_binding,
                    &inline_local_bindings,
                )? {
                    return Ok(false);
                }
            }
            match terminal_statement {
                Statement::Return(return_value) => {
                    let substituted = compiler.substitute_user_function_call_frame_bindings(
                        return_value,
                        called_function,
                        &call_arguments,
                        &this_binding,
                        &arguments_binding,
                    );
                    compiler.emit_numeric_expression(&substituted)?;
                    compiler.state.emission.output.instructions.push(0x1a);
                    Ok(true)
                }
                _ => compiler.emit_inline_user_function_effect_statement_with_explicit_call_frame(
                    terminal_statement,
                    called_function,
                    &call_arguments,
                    &this_binding,
                    &arguments_binding,
                    &inline_local_bindings,
                ),
            }
        })
    }
}
