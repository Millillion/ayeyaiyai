use super::*;

const NULL_SUPER_CONSTRUCTOR_BINDING: &str = "__ayy_null_super_constructor";

impl<'a> FunctionCompiler<'a> {
    fn emit_user_function_capture_slot_source_value(
        &mut self,
        capture_name: &str,
        source_expression: &Expression,
    ) -> DirectResult<()> {
        if capture_name == "this" && matches!(source_expression, Expression::This) {
            self.push_global_get(CURRENT_THIS_GLOBAL_INDEX);
            return Ok(());
        }
        self.emit_capture_source_expression_value(capture_name, source_expression)
    }

    fn null_super_constructor_statement_arguments<'b>(
        statement: &'b Statement,
    ) -> Option<&'b [CallArgument]> {
        match statement {
            Statement::Expression(Expression::SuperCall { callee, arguments })
            | Statement::Var {
                value: Expression::SuperCall { callee, arguments },
                ..
            }
            | Statement::Let {
                value: Expression::SuperCall { callee, arguments },
                ..
            }
            | Statement::Assign {
                value: Expression::SuperCall { callee, arguments },
                ..
            }
            | Statement::Return(Expression::SuperCall { callee, arguments }) => {
                if matches!(callee.as_ref(), Expression::Identifier(name) if name == NULL_SUPER_CONSTRUCTOR_BINDING)
                {
                    Some(arguments.as_slice())
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn statement_contains_explicit_return(statement: &Statement) -> bool {
        match statement {
            Statement::Return(_) => true,
            Statement::Declaration { body }
            | Statement::Block { body }
            | Statement::Labeled { body, .. }
            | Statement::With { body, .. }
            | Statement::While { body, .. }
            | Statement::DoWhile { body, .. } => {
                body.iter().any(Self::statement_contains_explicit_return)
            }
            Statement::If {
                then_branch,
                else_branch,
                ..
            } => then_branch
                .iter()
                .chain(else_branch)
                .any(Self::statement_contains_explicit_return),
            Statement::Try {
                body,
                catch_setup,
                catch_body,
                ..
            } => body
                .iter()
                .chain(catch_setup)
                .chain(catch_body)
                .any(Self::statement_contains_explicit_return),
            Statement::Switch { cases, .. } => cases.iter().any(|case| {
                case.body
                    .iter()
                    .any(Self::statement_contains_explicit_return)
            }),
            Statement::For { init, body, .. } => init
                .iter()
                .chain(body)
                .any(Self::statement_contains_explicit_return),
            _ => false,
        }
    }

    fn user_function_has_explicit_return(&self, user_function: &UserFunction) -> bool {
        self.resolve_registered_function_declaration(&user_function.name)
            .is_some_and(|function| {
                function
                    .body
                    .iter()
                    .any(Self::statement_contains_explicit_return)
            })
    }

    fn member_call_object_for_property<'b>(
        expression: &'b Expression,
        expected_property: &str,
    ) -> Option<&'b Expression> {
        let Expression::Call { callee, .. } = expression else {
            return None;
        };
        let Expression::Member { object, property } = callee.as_ref() else {
            return None;
        };
        if matches!(property.as_ref(), Expression::String(property) if property == expected_property)
        {
            Some(object.as_ref())
        } else {
            None
        }
    }

    fn expression_is_async_generator_call_candidate(&self, expression: &Expression) -> bool {
        let Expression::Call { callee, .. } = expression else {
            return false;
        };
        if self
            .resolve_function_binding_from_expression(callee)
            .and_then(|binding| match binding {
                LocalFunctionBinding::User(function_name) => self.user_function(&function_name),
                LocalFunctionBinding::Builtin(_) => None,
            })
            .is_some_and(|function| matches!(function.kind, FunctionKind::AsyncGenerator))
        {
            return true;
        }
        matches!(callee.as_ref(), Expression::Member { .. })
    }

    fn expression_contains_async_generator_next_then_chain(&self, expression: &Expression) -> bool {
        if let Some(next_call) = Self::member_call_object_for_property(expression, "then")
            && let Some(generator_call) = Self::member_call_object_for_property(next_call, "next")
            && self.expression_is_async_generator_call_candidate(generator_call)
        {
            return true;
        }
        match expression {
            Expression::Member { object, property } => {
                self.expression_contains_async_generator_next_then_chain(object)
                    || self.expression_contains_async_generator_next_then_chain(property)
            }
            Expression::SuperMember { property } => {
                self.expression_contains_async_generator_next_then_chain(property)
            }
            Expression::Assign { value, .. }
            | Expression::Await(value)
            | Expression::EnumerateKeys(value)
            | Expression::GetIterator(value)
            | Expression::IteratorClose(value) => {
                self.expression_contains_async_generator_next_then_chain(value)
            }
            Expression::AssignMember {
                object,
                property,
                value,
            } => {
                self.expression_contains_async_generator_next_then_chain(object)
                    || self.expression_contains_async_generator_next_then_chain(property)
                    || self.expression_contains_async_generator_next_then_chain(value)
            }
            Expression::AssignSuperMember { property, value } => {
                self.expression_contains_async_generator_next_then_chain(property)
                    || self.expression_contains_async_generator_next_then_chain(value)
            }
            Expression::Unary { expression, .. } => {
                self.expression_contains_async_generator_next_then_chain(expression)
            }
            Expression::Binary { left, right, .. } => {
                self.expression_contains_async_generator_next_then_chain(left)
                    || self.expression_contains_async_generator_next_then_chain(right)
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                self.expression_contains_async_generator_next_then_chain(condition)
                    || self.expression_contains_async_generator_next_then_chain(then_expression)
                    || self.expression_contains_async_generator_next_then_chain(else_expression)
            }
            Expression::Sequence(expressions) => expressions.iter().any(|expression| {
                self.expression_contains_async_generator_next_then_chain(expression)
            }),
            Expression::Call { callee, arguments }
            | Expression::SuperCall { callee, arguments }
            | Expression::New { callee, arguments } => {
                self.expression_contains_async_generator_next_then_chain(callee)
                    || arguments.iter().any(|argument| {
                        self.expression_contains_async_generator_next_then_chain(
                            argument.expression(),
                        )
                    })
            }
            Expression::Array(elements) => elements.iter().any(|element| match element {
                ArrayElement::Expression(value) | ArrayElement::Spread(value) => {
                    self.expression_contains_async_generator_next_then_chain(value)
                }
            }),
            Expression::Object(entries) => entries.iter().any(|entry| match entry {
                ObjectEntry::Data { key, value } => {
                    self.expression_contains_async_generator_next_then_chain(key)
                        || self.expression_contains_async_generator_next_then_chain(value)
                }
                ObjectEntry::Getter { key, getter } => {
                    self.expression_contains_async_generator_next_then_chain(key)
                        || self.expression_contains_async_generator_next_then_chain(getter)
                }
                ObjectEntry::Setter { key, setter } => {
                    self.expression_contains_async_generator_next_then_chain(key)
                        || self.expression_contains_async_generator_next_then_chain(setter)
                }
                ObjectEntry::Spread(value) => {
                    self.expression_contains_async_generator_next_then_chain(value)
                }
            }),
            _ => false,
        }
    }

    fn statement_contains_async_generator_next_then_chain(&self, statement: &Statement) -> bool {
        match statement {
            Statement::Var { value, .. }
            | Statement::Let { value, .. }
            | Statement::Assign { value, .. }
            | Statement::Expression(value)
            | Statement::Throw(value)
            | Statement::Return(value)
            | Statement::Yield { value }
            | Statement::YieldDelegate { value } => {
                self.expression_contains_async_generator_next_then_chain(value)
            }
            Statement::AssignMember {
                object,
                property,
                value,
            } => {
                self.expression_contains_async_generator_next_then_chain(object)
                    || self.expression_contains_async_generator_next_then_chain(property)
                    || self.expression_contains_async_generator_next_then_chain(value)
            }
            Statement::Print { values } => values
                .iter()
                .any(|value| self.expression_contains_async_generator_next_then_chain(value)),
            Statement::Declaration { body }
            | Statement::Block { body }
            | Statement::Labeled { body, .. }
            | Statement::With { body, .. }
            | Statement::While { body, .. }
            | Statement::DoWhile { body, .. } => body.iter().any(|statement| {
                self.statement_contains_async_generator_next_then_chain(statement)
            }),
            Statement::If {
                condition,
                then_branch,
                else_branch,
            } => {
                self.expression_contains_async_generator_next_then_chain(condition)
                    || then_branch.iter().any(|statement| {
                        self.statement_contains_async_generator_next_then_chain(statement)
                    })
                    || else_branch.iter().any(|statement| {
                        self.statement_contains_async_generator_next_then_chain(statement)
                    })
            }
            Statement::Try {
                body,
                catch_setup,
                catch_body,
                ..
            } => body
                .iter()
                .chain(catch_setup)
                .chain(catch_body)
                .any(|statement| {
                    self.statement_contains_async_generator_next_then_chain(statement)
                }),
            Statement::Switch {
                discriminant,
                cases,
                ..
            } => {
                self.expression_contains_async_generator_next_then_chain(discriminant)
                    || cases.iter().any(|case| {
                        case.body.iter().any(|statement| {
                            self.statement_contains_async_generator_next_then_chain(statement)
                        })
                    })
            }
            Statement::For {
                init,
                condition,
                update,
                break_hook,
                body,
                ..
            } => {
                init.iter().any(|statement| {
                    self.statement_contains_async_generator_next_then_chain(statement)
                }) || condition.as_ref().is_some_and(|expression| {
                    self.expression_contains_async_generator_next_then_chain(expression)
                }) || update.as_ref().is_some_and(|expression| {
                    self.expression_contains_async_generator_next_then_chain(expression)
                }) || break_hook.as_ref().is_some_and(|expression| {
                    self.expression_contains_async_generator_next_then_chain(expression)
                }) || body.iter().any(|statement| {
                    self.statement_contains_async_generator_next_then_chain(statement)
                })
            }
            Statement::Break { .. } | Statement::Continue { .. } => false,
        }
    }

    fn user_function_contains_async_generator_next_then_chain(
        &self,
        user_function: &UserFunction,
    ) -> bool {
        self.resolve_registered_function_declaration(&user_function.name)
            .is_some_and(|function| {
                function.body.iter().any(|statement| {
                    self.statement_contains_async_generator_next_then_chain(statement)
                })
            })
    }

    fn expression_is_deferred_module_namespace_reference(expression: &Expression) -> bool {
        matches!(
            expression,
            Expression::Identifier(name) if name.starts_with("__ayy_module_deferred_namespace_")
        )
    }

    fn derived_constructor_super_returns_deferred_module_namespace(
        &self,
        user_function: &UserFunction,
        arguments: &[CallArgument],
    ) -> bool {
        self.user_function_is_derived_constructor(user_function)
            && self
                .resolve_derived_constructor_super_call_replacement_this_expression(
                    user_function,
                    arguments,
                )
                .is_some_and(|expression| {
                    Self::expression_is_deferred_module_namespace_reference(&expression)
                })
    }

    fn emit_null_super_constructor_construct(
        &mut self,
        user_function: &UserFunction,
    ) -> DirectResult<bool> {
        let Some(function) = self.resolve_registered_function_declaration(&user_function.name)
        else {
            return Ok(false);
        };
        let Some((super_index, super_arguments)) =
            function
                .body
                .iter()
                .enumerate()
                .find_map(|(index, statement)| {
                    Self::null_super_constructor_statement_arguments(statement)
                        .map(|arguments| (index, arguments.to_vec()))
                })
        else {
            return Ok(false);
        };
        let prefix = function.body[..super_index].to_vec();
        let saved_new_target_local = self.allocate_temp_local();
        self.push_global_get(CURRENT_NEW_TARGET_GLOBAL_INDEX);
        self.push_local_set(saved_new_target_local);
        let saved_this_local = self.allocate_temp_local();
        self.push_global_get(CURRENT_THIS_GLOBAL_INDEX);
        self.push_local_set(saved_this_local);
        self.push_i32_const(user_function_runtime_value(user_function));
        self.push_global_set(CURRENT_NEW_TARGET_GLOBAL_INDEX);
        self.push_i32_const(JS_UNDEFINED_TAG);
        self.push_global_set(CURRENT_THIS_GLOBAL_INDEX);
        for statement in &prefix {
            self.emit_statement(statement)?;
        }
        self.push_local_get(saved_new_target_local);
        self.push_global_set(CURRENT_NEW_TARGET_GLOBAL_INDEX);
        self.push_local_get(saved_this_local);
        self.push_global_set(CURRENT_THIS_GLOBAL_INDEX);
        self.emit_null_super_constructor_call(&super_arguments)?;
        Ok(true)
    }

    fn default_derived_constructor_builtin_super_name(
        &self,
        callee: &Expression,
        user_function: &UserFunction,
    ) -> Option<String> {
        if !self.user_function_is_derived_constructor(user_function) {
            return None;
        }
        let declaration = self.resolve_registered_function_declaration(&user_function.name)?;
        if declaration.direct_eval_in_class_field_initializer {
            return None;
        }
        let [rest_parameter] = declaration.params.as_slice() else {
            return None;
        };
        if !rest_parameter.rest {
            return None;
        }
        let (super_callee, super_arguments) =
            self.resolve_derived_constructor_super_call(user_function)?;
        let [CallArgument::Spread(Expression::Identifier(spread_name))] = super_arguments else {
            return None;
        };
        if spread_name != &rest_parameter.name {
            return None;
        }

        let resolved_super = match super_callee {
            Expression::Identifier(name) => self
                .resolve_constructor_capture_source_bindings_from_expression(callee)
                .and_then(|bindings| bindings.get(name).cloned())
                .unwrap_or_else(|| super_callee.clone()),
            _ => super_callee.clone(),
        };
        let LocalFunctionBinding::Builtin(function_name) =
            self.resolve_function_binding_from_expression(&resolved_super)?
        else {
            return None;
        };
        Some(function_name)
    }

    fn emit_default_derived_builtin_construct(
        &mut self,
        callee: &Expression,
        user_function: &UserFunction,
        arguments: &[CallArgument],
    ) -> DirectResult<bool> {
        let Some(function_name) =
            self.default_derived_constructor_builtin_super_name(callee, user_function)
        else {
            return Ok(false);
        };
        if std::env::var_os("AYY_TRACE_CONSTRUCT_CALLS").is_some() {
            eprintln!(
                "construct_call:default_derived_builtin callee={callee:?} super={function_name}"
            );
        }
        self.state
            .speculation
            .static_semantics
            .last_bound_user_function_call = Some(BoundUserFunctionCallSnapshot {
            function_name: user_function.name.clone(),
            source_expression: Some(Expression::New {
                callee: Box::new(callee.clone()),
                arguments: arguments.to_vec(),
            }),
            result_expression: Some(Expression::New {
                callee: Box::new(Expression::Identifier(function_name.clone())),
                arguments: arguments.to_vec(),
            }),
            prototype_source_expression: None,
            updated_bindings: self
                .resolve_constructor_capture_source_bindings_from_expression(callee)
                .unwrap_or_default(),
        });
        self.emit_builtin_call_for_callee(callee, &function_name, arguments, true)
    }

    pub(in crate::backend::direct_wasm) fn prepare_constructor_runtime_argument_bindings(
        &mut self,
        arguments: &[CallArgument],
    ) -> DirectResult<(Vec<CallArgument>, Vec<(String, String)>)> {
        let mut runtime_arguments = Vec::new();
        let mut shadow_writebacks = Vec::new();
        let mut argument_index = 0usize;

        for argument in arguments {
            match argument {
                CallArgument::Expression(expression) => {
                    self.emit_constructor_runtime_argument_binding(
                        argument_index,
                        expression,
                        &mut runtime_arguments,
                        &mut shadow_writebacks,
                    )?;
                    argument_index += 1;
                }
                CallArgument::Spread(expression) => {
                    if let Some(array_binding) =
                        self.resolve_array_binding_from_expression(expression)
                    {
                        self.emit_constructor_spread_static_source_side_effect(expression)?;
                        for value in array_binding.values {
                            let value = value.unwrap_or(Expression::Undefined);
                            self.emit_constructor_runtime_argument_binding(
                                argument_index,
                                &value,
                                &mut runtime_arguments,
                                &mut shadow_writebacks,
                            )?;
                            argument_index += 1;
                        }
                        continue;
                    }

                    if let Some(iterable_binding) =
                        self.resolve_constructor_spread_static_iterable_binding(expression)
                    {
                        self.emit_constructor_spread_static_source_side_effect(expression)?;
                        for value in iterable_binding.values {
                            let value = value.unwrap_or(Expression::Undefined);
                            self.emit_constructor_runtime_argument_binding(
                                argument_index,
                                &value,
                                &mut runtime_arguments,
                                &mut shadow_writebacks,
                            )?;
                            argument_index += 1;
                        }
                        continue;
                    }

                    if let Some((_, steps, completion_effects, _)) =
                        self.simple_generator_source_metadata(expression)
                    {
                        for step in steps {
                            for effect in step.effects {
                                self.emit_statement(&effect)?;
                            }
                            match step.outcome {
                                SimpleGeneratorStepOutcome::Yield(value) => {
                                    self.emit_constructor_runtime_argument_binding(
                                        argument_index,
                                        &value,
                                        &mut runtime_arguments,
                                        &mut shadow_writebacks,
                                    )?;
                                    argument_index += 1;
                                }
                                SimpleGeneratorStepOutcome::YieldResult(result) => {
                                    let value = self.simple_generator_yield_result_value(
                                        &result,
                                        &Expression::Undefined,
                                    );
                                    self.emit_constructor_runtime_argument_binding(
                                        argument_index,
                                        &value,
                                        &mut runtime_arguments,
                                        &mut shadow_writebacks,
                                    )?;
                                    argument_index += 1;
                                }
                                SimpleGeneratorStepOutcome::Throw(value) => {
                                    self.emit_static_throw_value(&StaticThrowValue::Value(value))?;
                                    return Ok((runtime_arguments, shadow_writebacks));
                                }
                            }
                        }
                        for effect in completion_effects {
                            self.emit_statement(&effect)?;
                        }
                        continue;
                    }

                    if self.emit_constructor_spread_static_iterator_throw(expression)? {
                        return Ok((runtime_arguments, shadow_writebacks));
                    }

                    self.emit_constructor_runtime_argument_binding(
                        argument_index,
                        expression,
                        &mut runtime_arguments,
                        &mut shadow_writebacks,
                    )?;
                    argument_index += 1;
                }
            }
        }

        Ok((runtime_arguments, shadow_writebacks))
    }

    fn emit_constructor_runtime_argument_binding(
        &mut self,
        index: usize,
        argument: &Expression,
        runtime_arguments: &mut Vec<CallArgument>,
        shadow_writebacks: &mut Vec<(String, String)>,
    ) -> DirectResult<()> {
        let precomputed_object_binding = if matches!(
            argument,
            Expression::Object(entries)
                if entries.iter().any(|entry| matches!(entry, ObjectEntry::Spread(_)))
        ) {
            self.resolve_object_binding_from_expression(argument)
        } else {
            None
        };
        if std::env::var_os("AYY_TRACE_CONSTRUCT_CALLS").is_some()
            && let Some(object_binding) = &precomputed_object_binding
        {
            let property_summary = ordered_object_property_names(object_binding)
                .into_iter()
                .map(|name| {
                    let property = Expression::String(name.clone());
                    let enumerable = object_binding_lookup_descriptor(object_binding, &property)
                        .map(|descriptor| descriptor.enumerable);
                    let hidden = object_binding
                        .non_enumerable_string_properties
                        .iter()
                        .any(|hidden_name| hidden_name == &name);
                    format!("{name}:enumerable={enumerable:?}:hidden={hidden}")
                })
                .collect::<Vec<_>>();
            eprintln!(
                "construct_call:precomputed_argument index={index} argument={argument:?} properties={property_summary:?}"
            );
        }
        let argument_needs_runtime_metadata = self
            .expression_depends_on_active_loop_assignment(argument)
            || self.expression_has_dynamic_member_property_access(argument);
        let argument_kind = if argument_needs_runtime_metadata {
            StaticValueKind::Unknown
        } else {
            self.infer_value_kind(argument)
                .unwrap_or(StaticValueKind::Unknown)
        };
        let hidden_name =
            self.allocate_named_hidden_local(&format!("construct_arg_{index}"), argument_kind);
        let hidden_local = self
            .state
            .runtime
            .locals
            .get(&hidden_name)
            .copied()
            .expect("fresh constructor argument local must exist");
        let source_owner = match argument {
            Expression::Identifier(name) => {
                self.runtime_object_property_shadow_owner_name_for_identifier(name)
            }
            Expression::This => {
                self.runtime_object_property_shadow_owner_name_for_identifier("this")
            }
            _ => None,
        };

        self.emit_numeric_expression(argument)?;
        self.push_local_set(hidden_local);
        if !argument_needs_runtime_metadata {
            self.update_capture_slot_binding_from_expression(&hidden_name, argument)?;
            if let Some(object_binding) = precomputed_object_binding {
                let object_binding = self
                    .object_binding_with_constructed_constructor_shadow(object_binding, argument);
                self.state
                    .speculation
                    .static_semantics
                    .set_local_object_binding(&hidden_name, object_binding.clone());
                self.clear_runtime_object_property_shadow_prefix(&hidden_name);
                self.emit_runtime_object_property_shadow_seed_from_binding(
                    &hidden_name,
                    &object_binding,
                )?;
                self.sync_runtime_object_property_shadow_static_metadata_from_binding(
                    &hidden_name,
                    &object_binding,
                );
            } else {
                self.sync_capture_slot_runtime_object_shadows_from_expression(
                    &hidden_name,
                    argument,
                )?;
            }
        }

        if let Some(source_owner) = source_owner
            && source_owner != hidden_name
        {
            shadow_writebacks.push((hidden_name.clone(), source_owner));
        }

        runtime_arguments.push(CallArgument::Expression(Expression::Identifier(
            hidden_name,
        )));
        Ok(())
    }

    fn emit_constructor_spread_static_source_side_effect(
        &mut self,
        expression: &Expression,
    ) -> DirectResult<()> {
        if matches!(
            expression,
            Expression::Assign { .. }
                | Expression::AssignMember { .. }
                | Expression::AssignSuperMember { .. }
                | Expression::Update { .. }
                | Expression::Call { .. }
                | Expression::New { .. }
        ) {
            self.emit_numeric_expression(expression)?;
            self.state.emission.output.instructions.push(0x1a);
        }
        Ok(())
    }

    fn evaluate_constructor_spread_static_iterator_field(
        &self,
        expression: Expression,
        bindings: &HashMap<String, Expression>,
        current_function_name: Option<&str>,
    ) -> Expression {
        self.evaluate_bound_snapshot_expression(
            &expression,
            &mut bindings.clone(),
            current_function_name,
        )
        .or_else(|| self.evaluate_simple_static_expression_with_bindings(&expression, bindings))
        .unwrap_or(expression)
    }

    fn resolve_constructor_spread_static_iterable_binding(
        &self,
        expression: &Expression,
    ) -> Option<ArrayValueBinding> {
        if let Some(binding) = self.resolve_static_iterable_binding_from_expression(expression) {
            return Some(binding);
        }

        let object_binding = self.resolve_object_binding_from_expression(expression)?;
        let symbol_iterator = self.materialize_static_expression(&Expression::Member {
            object: Box::new(Expression::Identifier("Symbol".to_string())),
            property: Box::new(Expression::String("iterator".to_string())),
        });
        let iterator_method = object_binding_lookup_value(&object_binding, &symbol_iterator)?;
        let LocalFunctionBinding::User(iterator_function_name) =
            self.resolve_function_binding_from_expression(iterator_method)?
        else {
            return None;
        };
        let (iterator_result, iterator_bindings) = self
            .execute_simple_static_user_function_with_bindings(
                &iterator_function_name,
                &HashMap::new(),
            )
            .or_else(|| {
                self.resolve_bound_snapshot_user_function_result(
                    &iterator_function_name,
                    &HashMap::new(),
                )
            })?;
        let iterator_result_binding =
            self.resolve_object_binding_from_expression(&iterator_result)?;
        let next_value = object_binding_lookup_value(
            &iterator_result_binding,
            &Expression::String("next".to_string()),
        )?;
        let LocalFunctionBinding::User(next_function_name) =
            self.resolve_function_binding_from_expression(next_value)?
        else {
            return None;
        };

        let mut step_bindings = iterator_bindings;
        let mut values = Vec::new();
        for _ in 0..256 {
            let (step_result, updated_bindings) = self
                .execute_simple_static_user_function_with_bindings(
                    &next_function_name,
                    &step_bindings,
                )
                .or_else(|| {
                    self.resolve_bound_snapshot_user_function_result(
                        &next_function_name,
                        &step_bindings,
                    )
                })?;
            step_bindings = updated_bindings;
            let step_object_binding = self.resolve_object_binding_from_expression(&step_result)?;
            let done = object_binding_lookup_value(
                &step_object_binding,
                &Expression::String("done".to_string()),
            )
            .cloned()
            .unwrap_or(Expression::Bool(false));
            let value = object_binding_lookup_value(
                &step_object_binding,
                &Expression::String("value".to_string()),
            )
            .cloned()
            .unwrap_or(Expression::Undefined);
            let done = self.evaluate_constructor_spread_static_iterator_field(
                done,
                &step_bindings,
                Some(&next_function_name),
            );
            let value = self.evaluate_constructor_spread_static_iterator_field(
                value,
                &step_bindings,
                Some(&next_function_name),
            );
            match done {
                Expression::Bool(true) => return Some(ArrayValueBinding { values }),
                Expression::Bool(false) => values.push(Some(value)),
                _ => return None,
            }
        }

        None
    }

    fn emit_constructor_spread_static_iterator_throw(
        &mut self,
        expression: &Expression,
    ) -> DirectResult<bool> {
        if let Some(throw_value) = self.resolve_static_get_iterator_throw_value(expression, &[]) {
            self.emit_static_throw_value(&throw_value)?;
            return Ok(true);
        }

        let Some(iterator_target) = self.resolve_static_get_iterator_value(expression, &[]) else {
            return Ok(false);
        };
        if matches!(
            self.infer_value_kind(&iterator_target),
            Some(
                StaticValueKind::Undefined
                    | StaticValueKind::Null
                    | StaticValueKind::Bool
                    | StaticValueKind::Number
                    | StaticValueKind::String
                    | StaticValueKind::BigInt
                    | StaticValueKind::Symbol
            )
        ) {
            self.emit_named_error_throw("TypeError")?;
            return Ok(true);
        }
        let next_property = Expression::String("next".to_string());
        let Some(next_binding) =
            self.resolve_member_function_binding(&iterator_target, &next_property)
        else {
            return Ok(false);
        };
        let Some(next_outcome) = self.resolve_static_function_outcome_from_binding_with_context(
            &next_binding,
            &[],
            self.current_function_name(),
        ) else {
            return Ok(false);
        };
        let step_result = match next_outcome {
            StaticEvalOutcome::Throw(throw_value) => {
                self.emit_static_throw_value(&throw_value)?;
                return Ok(true);
            }
            StaticEvalOutcome::Value(value) => self.materialize_static_expression(&value),
        };
        let Some(step_binding) = self.resolve_object_binding_from_expression(&step_result) else {
            if matches!(
                self.infer_value_kind(&step_result),
                Some(
                    StaticValueKind::Undefined
                        | StaticValueKind::Null
                        | StaticValueKind::Bool
                        | StaticValueKind::Number
                        | StaticValueKind::String
                        | StaticValueKind::BigInt
                        | StaticValueKind::Symbol
                )
            ) {
                self.emit_named_error_throw("TypeError")?;
                return Ok(true);
            }
            return Ok(false);
        };

        let done_property = Expression::String("done".to_string());
        if let Some(getter_binding) =
            self.resolve_member_getter_binding(&step_result, &done_property)
        {
            let Some(done_outcome) = self
                .resolve_static_function_outcome_from_binding_with_context(
                    &getter_binding,
                    &[],
                    self.current_function_name(),
                )
            else {
                return Ok(false);
            };
            match done_outcome {
                StaticEvalOutcome::Throw(throw_value) => {
                    self.emit_static_throw_value(&throw_value)?;
                    return Ok(true);
                }
                StaticEvalOutcome::Value(value)
                    if self.resolve_static_boolean_expression(&value) == Some(true) =>
                {
                    return Ok(false);
                }
                StaticEvalOutcome::Value(_) => {}
            }
        }
        if let Some(descriptor) = object_binding_lookup_descriptor(&step_binding, &done_property)
            && let Some(getter) = &descriptor.getter
        {
            let Some(getter_binding) = self.resolve_function_binding_from_expression(getter) else {
                return Ok(false);
            };
            let Some(done_outcome) = self
                .resolve_static_function_outcome_from_binding_with_context(
                    &getter_binding,
                    &[],
                    self.current_function_name(),
                )
            else {
                return Ok(false);
            };
            match done_outcome {
                StaticEvalOutcome::Throw(throw_value) => {
                    self.emit_static_throw_value(&throw_value)?;
                    return Ok(true);
                }
                StaticEvalOutcome::Value(value)
                    if self.resolve_static_boolean_expression(&value) == Some(true) =>
                {
                    return Ok(false);
                }
                StaticEvalOutcome::Value(_) => {}
            }
        }
        let done_value = object_binding_lookup_value(&step_binding, &done_property)
            .cloned()
            .unwrap_or(Expression::Bool(false));
        if self.resolve_static_boolean_expression(&done_value) == Some(true) {
            return Ok(false);
        }

        let value_property = Expression::String("value".to_string());
        if let Some(getter_binding) =
            self.resolve_member_getter_binding(&step_result, &value_property)
        {
            let Some(value_outcome) = self
                .resolve_static_function_outcome_from_binding_with_context(
                    &getter_binding,
                    &[],
                    self.current_function_name(),
                )
            else {
                return Ok(false);
            };
            if let StaticEvalOutcome::Throw(throw_value) = value_outcome {
                self.emit_static_throw_value(&throw_value)?;
                return Ok(true);
            }
        }
        if let Some(descriptor) = object_binding_lookup_descriptor(&step_binding, &value_property)
            && let Some(getter) = &descriptor.getter
        {
            let Some(getter_binding) = self.resolve_function_binding_from_expression(getter) else {
                return Ok(false);
            };
            let Some(value_outcome) = self
                .resolve_static_function_outcome_from_binding_with_context(
                    &getter_binding,
                    &[],
                    self.current_function_name(),
                )
            else {
                return Ok(false);
            };
            if let StaticEvalOutcome::Throw(throw_value) = value_outcome {
                self.emit_static_throw_value(&throw_value)?;
                return Ok(true);
            }
        }

        Ok(false)
    }

    pub(in crate::backend::direct_wasm) fn initialize_user_function_capture_slots_from_expression(
        &mut self,
        expression: &Expression,
        user_function: &UserFunction,
    ) -> DirectResult<Option<BTreeMap<String, String>>> {
        if user_function.lexical_this
            && let Some((target, _, LocalFunctionBinding::User(function_name))) =
                self.resolve_function_prototype_bind_call(expression, self.current_function_name())
            && function_name == user_function.name
            && let Some(capture_slots) = self.resolve_function_expression_capture_slots(&target)
        {
            return Ok(Some(capture_slots));
        }
        if let Some(capture_slots) = self.resolve_function_expression_capture_slots(expression) {
            return Ok(Some(capture_slots));
        }
        let mut capture_bindings = self
            .backend
            .function_registry
            .analysis
            .user_function_capture_bindings
            .get(&user_function.name)
            .cloned()
            .unwrap_or_default();
        self.add_active_with_scope_function_capture_bindings(
            &user_function.name,
            &mut capture_bindings,
        )?;
        if capture_bindings.is_empty() {
            return Ok(None);
        }
        let Some(capture_source_bindings) =
            self.resolve_constructor_capture_source_bindings_from_expression(expression)
        else {
            return Ok(None);
        };

        let mut capture_slots = BTreeMap::new();
        for capture_name in capture_bindings.keys() {
            let capture_identifier = Expression::Identifier(capture_name.clone());
            let scoped_source_object =
                if self.expression_is_active_with_scope_object(&capture_identifier) {
                    None
                } else {
                    self.resolve_with_scope_binding(capture_name)?
                };
            if !self.user_function_capture_source_is_locally_bound(capture_name)
                && scoped_source_object.is_none()
            {
                continue;
            }
            let source_expression = if let Some(scope_object) = scoped_source_object.as_ref() {
                Expression::Member {
                    object: Box::new(scope_object.clone()),
                    property: Box::new(Expression::String(capture_name.clone())),
                }
            } else {
                let Some(source_expression) = capture_source_bindings.get(capture_name).cloned()
                else {
                    return Ok(None);
                };
                source_expression
            };
            let source_expression = if matches!(
                &source_expression,
                Expression::Identifier(name) if name == capture_name
            ) {
                self.resolve_user_function_capture_hidden_name(capture_name)
                    .map(Expression::Identifier)
                    .unwrap_or(source_expression)
            } else {
                source_expression
            };
            let hidden_name = self.allocate_named_hidden_local(
                &format!("closure_slot_{}_{}", user_function.name, capture_name),
                self.infer_value_kind(&source_expression)
                    .unwrap_or(StaticValueKind::Unknown),
            );
            let hidden_local = self
                .state
                .runtime
                .locals
                .get(&hidden_name)
                .copied()
                .expect("fresh returned function capture slot local must exist");
            self.emit_user_function_capture_slot_source_value(capture_name, &source_expression)?;
            self.push_local_set(hidden_local);
            self.update_capture_slot_binding_from_expression(&hidden_name, &source_expression)?;
            self.sync_capture_slot_runtime_object_shadows_from_expression(
                &hidden_name,
                &source_expression,
            )?;
            if let Expression::Identifier(source_binding_name) = &source_expression {
                self.state
                    .speculation
                    .static_semantics
                    .capture_slot_source_bindings
                    .insert(
                        hidden_name.clone(),
                        self.capture_slot_live_source_binding_name(source_binding_name),
                    );
            } else if matches!(source_expression, Expression::This | Expression::NewTarget) {
                self.state
                    .speculation
                    .static_semantics
                    .capture_slot_source_bindings
                    .insert(hidden_name.clone(), capture_name.clone());
            } else if let Expression::Member { object, property } = &source_expression
                && let Some(source_key) = Self::capture_slot_member_source_key(object, property)
            {
                self.state
                    .speculation
                    .static_semantics
                    .capture_slot_source_bindings
                    .insert(hidden_name.clone(), source_key);
            }
            capture_slots.insert(capture_name.clone(), hidden_name);
        }

        if capture_slots.is_empty() {
            return Ok(None);
        }
        Ok(Some(capture_slots))
    }

    pub(in crate::backend::direct_wasm) fn emit_user_function_construct(
        &mut self,
        callee: &Expression,
        user_function: &UserFunction,
        arguments: &[CallArgument],
    ) -> DirectResult<bool> {
        if !user_function.is_constructible() {
            return Ok(false);
        }
        if self.emit_null_super_constructor_construct(user_function)? {
            return Ok(true);
        }
        if self.emit_default_derived_builtin_construct(callee, user_function, arguments)? {
            return Ok(true);
        }

        let ordinary_this_expression = Expression::Object(Vec::new());
        let construct_this_expression = if self.user_function_is_derived_constructor(user_function)
        {
            &Expression::Undefined
        } else {
            &ordinary_this_expression
        };
        let capture_slots =
            self.initialize_user_function_capture_slots_from_expression(callee, user_function)?;
        let capture_source_bindings = self
            .resolve_constructor_capture_source_bindings_from_expression(callee)
            .or_else(|| {
                capture_slots.as_ref().and_then(|slots| {
                    let bindings = slots
                        .iter()
                        .map(|(capture_name, slot_name)| {
                            (
                                capture_name.clone(),
                                self.snapshot_bound_capture_slot_expression(slot_name),
                            )
                        })
                        .collect::<HashMap<_, _>>();
                    (!bindings.is_empty()).then_some(bindings)
                })
            });
        let constructor_ordinary_direct_eval = self
            .user_function_mentions_direct_eval(user_function)
            && !self
                .resolve_registered_function_declaration(&user_function.name)
                .is_some_and(|declaration| declaration.direct_eval_in_class_field_initializer);
        let constructor_static_resolution_allowed = arguments.iter().all(|argument| {
            let expression = match argument {
                CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                    expression
                }
            };
            !self.expression_depends_on_active_loop_assignment(expression)
                && !self.expression_has_dynamic_member_property_access(expression)
        }) && !self
            .derived_constructor_super_returns_deferred_module_namespace(user_function, arguments)
            && !self.registered_function_body_mentions_promise_like_chain(&user_function.name)
            && !self.user_function_contains_async_generator_next_then_chain(user_function);
        let constructor_result_outcome = (!constructor_ordinary_direct_eval
            && constructor_static_resolution_allowed)
            .then(|| {
                self.resolve_user_constructor_object_binding_outcome_for_function(
                    user_function,
                    arguments,
                    capture_source_bindings.as_ref(),
                )
            })
            .flatten();
        if let Some(Err(throw_value)) = constructor_result_outcome.as_ref() {
            self.emit_static_throw_value(throw_value)?;
            return Ok(true);
        }
        let constructor_return_resolution = (!constructor_ordinary_direct_eval
            && constructor_static_resolution_allowed
            && self.user_function_has_explicit_return(user_function))
        .then(|| {
            self.resolve_user_constructor_return_expression_with_explicit_status_for_function(
                user_function,
                arguments,
                capture_source_bindings.as_ref(),
            )
        })
        .flatten()
        .filter(|(expression, _)| {
            self.resolve_object_binding_from_expression(expression)
                .is_some()
                || self
                    .resolve_array_binding_from_expression(expression)
                    .is_some()
                || self
                    .resolve_function_binding_from_expression(expression)
                    .is_some()
        });
        let constructor_return_expression = constructor_return_resolution
            .as_ref()
            .map(|(expression, _)| expression.clone());
        let constructor_source_expression = Expression::New {
            callee: Box::new(callee.clone()),
            arguments: arguments.to_vec(),
        };
        let constructor_prototype_source_expression = constructor_return_resolution
            .as_ref()
            .and_then(|(expression, explicit)| {
                if !explicit {
                    return None;
                }
                if matches!(expression, Expression::This)
                    || matches!(
                        expression,
                        Expression::Identifier(name) if name == Self::STATIC_NEW_THIS_BINDING
                    )
                {
                    return Some(constructor_source_expression.clone());
                }
                Some(expression.clone())
            });
        let constructor_result_expression = constructor_return_expression.clone().or_else(|| {
            constructor_result_outcome
                .as_ref()
                .and_then(|outcome| outcome.as_ref().ok())
                .map(|binding| object_binding_to_expression(binding))
        });
        let constructor_updated_bindings = (!constructor_ordinary_direct_eval
            && constructor_static_resolution_allowed
            && capture_slots.is_none())
        .then(|| {
            self.resolve_user_constructor_updated_bindings_for_function(
                user_function,
                arguments,
                capture_source_bindings.as_ref(),
            )
        })
        .flatten();
        if std::env::var_os("AYY_TRACE_CONSTRUCT_CALLS").is_some() {
            eprintln!(
                "construct_call:static_updates function={} direct_eval={} static_allowed={} updated={constructor_updated_bindings:?}",
                user_function.name,
                constructor_ordinary_direct_eval,
                constructor_static_resolution_allowed
            );
        }
        let constructor_snapshot_updated_bindings =
            constructor_updated_bindings.clone().unwrap_or_default();
        let constructor_updated_bindings_for_sync = constructor_updated_bindings.clone();
        self.state
            .speculation
            .static_semantics
            .last_bound_user_function_call = Some(BoundUserFunctionCallSnapshot {
            function_name: user_function.name.clone(),
            source_expression: Some(constructor_source_expression),
            result_expression: constructor_result_expression,
            prototype_source_expression: constructor_prototype_source_expression,
            updated_bindings: constructor_snapshot_updated_bindings,
        });

        let (runtime_arguments, argument_shadow_writebacks) =
            self.prepare_constructor_runtime_argument_bindings(arguments)?;
        if let Some(capture_slots) = capture_slots.as_ref() {
            self.emit_user_function_call_with_new_target_and_this_expression_and_bound_captures_without_static_snapshot(
                user_function,
                &runtime_arguments,
                user_function_runtime_value(user_function),
                construct_this_expression,
                capture_slots,
            )?;
        } else {
            self.emit_user_function_call_with_new_target_and_this_expression_without_static_snapshot(
                user_function,
                &runtime_arguments,
                user_function_runtime_value(user_function),
                construct_this_expression,
            )?;
        }
        let constructor_return_local = self.allocate_temp_local();
        self.push_local_set(constructor_return_local);
        for (hidden_name, source_owner) in argument_shadow_writebacks {
            self.emit_runtime_object_property_shadow_copy(&hidden_name, &source_owner)?;
        }
        self.invalidate_raw_assigned_global_metadata_after_user_call(user_function);
        if let Some(updated_bindings) = constructor_updated_bindings_for_sync.as_ref() {
            let mut updated_names =
                self.collect_user_function_call_effect_nonlocal_bindings(user_function);
            updated_names.extend(
                self.collect_snapshot_updated_nonlocal_bindings(
                    user_function,
                    Some(updated_bindings),
                ),
            );
            let unresolved = self.sync_snapshot_user_function_call_effect_bindings(
                &updated_names,
                Some(updated_bindings),
                None,
            )?;
            if !unresolved.is_empty() {
                let preserved_kinds = unresolved
                    .iter()
                    .filter_map(|name| {
                        self.lookup_identifier_kind(name)
                            .map(|kind| (name.clone(), kind))
                    })
                    .collect::<HashMap<_, _>>();
                self.invalidate_static_binding_metadata_for_names_with_preserved_kinds(
                    &unresolved,
                    &preserved_kinds,
                );
            }
        }
        self.push_local_get(constructor_return_local);
        self.state.emission.output.instructions.push(0x1a);
        self.push_i32_const(JS_TYPEOF_OBJECT_TAG);
        Ok(true)
    }
}
