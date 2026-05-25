use super::*;

impl<'a> FunctionCompiler<'a> {
    fn expression_contains_super_call(expression: &Expression) -> bool {
        match expression {
            Expression::SuperCall { .. } => true,
            Expression::Array(elements) => elements.iter().any(|element| match element {
                ArrayElement::Expression(expression) | ArrayElement::Spread(expression) => {
                    Self::expression_contains_super_call(expression)
                }
            }),
            Expression::Object(entries) => entries.iter().any(|entry| match entry {
                ObjectEntry::Data { key, value } => {
                    Self::expression_contains_super_call(key)
                        || Self::expression_contains_super_call(value)
                }
                ObjectEntry::Getter { key, getter } => {
                    Self::expression_contains_super_call(key)
                        || Self::expression_contains_super_call(getter)
                }
                ObjectEntry::Setter { key, setter } => {
                    Self::expression_contains_super_call(key)
                        || Self::expression_contains_super_call(setter)
                }
                ObjectEntry::Spread(expression) => Self::expression_contains_super_call(expression),
            }),
            Expression::Member { object, property } => {
                Self::expression_contains_super_call(object)
                    || Self::expression_contains_super_call(property)
            }
            Expression::Assign { value, .. }
            | Expression::Await(value)
            | Expression::EnumerateKeys(value)
            | Expression::GetIterator(value)
            | Expression::IteratorClose(value)
            | Expression::Unary {
                expression: value, ..
            } => Self::expression_contains_super_call(value),
            Expression::AssignMember {
                object,
                property,
                value,
            } => {
                Self::expression_contains_super_call(object)
                    || Self::expression_contains_super_call(property)
                    || Self::expression_contains_super_call(value)
            }
            Expression::SuperMember { property } => Self::expression_contains_super_call(property),
            Expression::AssignSuperMember { property, value } => {
                Self::expression_contains_super_call(property)
                    || Self::expression_contains_super_call(value)
            }
            Expression::Binary { left, right, .. } => {
                Self::expression_contains_super_call(left)
                    || Self::expression_contains_super_call(right)
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                Self::expression_contains_super_call(condition)
                    || Self::expression_contains_super_call(then_expression)
                    || Self::expression_contains_super_call(else_expression)
            }
            Expression::Sequence(expressions) => {
                expressions.iter().any(Self::expression_contains_super_call)
            }
            Expression::Call { callee, arguments } | Expression::New { callee, arguments } => {
                Self::expression_contains_super_call(callee)
                    || arguments
                        .iter()
                        .any(|argument| Self::expression_contains_super_call(argument.expression()))
            }
            Expression::Identifier(_)
            | Expression::Update { .. }
            | Expression::Number(_)
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

    fn statement_contains_super_call(statement: &Statement) -> bool {
        match statement {
            Statement::Declaration { body }
            | Statement::Block { body }
            | Statement::Labeled { body, .. }
            | Statement::With { body, .. }
            | Statement::While { body, .. }
            | Statement::DoWhile { body, .. } => {
                body.iter().any(Self::statement_contains_super_call)
            }
            Statement::Var { value, .. }
            | Statement::Let { value, .. }
            | Statement::Assign { value, .. }
            | Statement::Expression(value)
            | Statement::Throw(value)
            | Statement::Return(value)
            | Statement::Yield { value }
            | Statement::YieldDelegate { value } => Self::expression_contains_super_call(value),
            Statement::AssignMember {
                object,
                property,
                value,
            } => {
                Self::expression_contains_super_call(object)
                    || Self::expression_contains_super_call(property)
                    || Self::expression_contains_super_call(value)
            }
            Statement::Print { values } => values.iter().any(Self::expression_contains_super_call),
            Statement::If {
                condition,
                then_branch,
                else_branch,
            } => {
                Self::expression_contains_super_call(condition)
                    || then_branch.iter().any(Self::statement_contains_super_call)
                    || else_branch.iter().any(Self::statement_contains_super_call)
            }
            Statement::Try {
                body,
                catch_setup,
                catch_body,
                ..
            } => body
                .iter()
                .chain(catch_setup.iter())
                .chain(catch_body.iter())
                .any(Self::statement_contains_super_call),
            Statement::Switch {
                discriminant,
                cases,
                ..
            } => {
                Self::expression_contains_super_call(discriminant)
                    || cases.iter().any(|case| {
                        case.test
                            .as_ref()
                            .is_some_and(Self::expression_contains_super_call)
                            || case.body.iter().any(Self::statement_contains_super_call)
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
                init.iter().any(Self::statement_contains_super_call)
                    || condition
                        .as_ref()
                        .is_some_and(Self::expression_contains_super_call)
                    || update
                        .as_ref()
                        .is_some_and(Self::expression_contains_super_call)
                    || break_hook
                        .as_ref()
                        .is_some_and(Self::expression_contains_super_call)
                    || body.iter().any(Self::statement_contains_super_call)
            }
            Statement::Break { .. } | Statement::Continue { .. } => false,
        }
    }

    fn user_function_body_contains_super_call(&self, user_function: &UserFunction) -> bool {
        self.resolve_registered_function_declaration(&user_function.name)
            .is_some_and(|function| {
                function.params.iter().any(|parameter| {
                    parameter
                        .default
                        .as_ref()
                        .is_some_and(Self::expression_contains_super_call)
                }) || function
                    .body
                    .iter()
                    .any(Self::statement_contains_super_call)
            })
    }

    pub(in crate::backend::direct_wasm) fn emit_user_function_call_with_new_target_and_this_expression_and_bound_captures(
        &mut self,
        user_function: &UserFunction,
        arguments: &[CallArgument],
        new_target_value: i32,
        this_expression: &Expression,
        capture_slots: &BTreeMap<String, String>,
    ) -> DirectResult<()> {
        self.emit_user_function_call_with_new_target_and_this_expression_and_bound_captures_impl(
            user_function,
            arguments,
            new_target_value,
            this_expression,
            capture_slots,
            true,
        )
    }

    pub(in crate::backend::direct_wasm) fn emit_user_function_call_with_new_target_and_this_expression_and_bound_captures_without_static_snapshot(
        &mut self,
        user_function: &UserFunction,
        arguments: &[CallArgument],
        new_target_value: i32,
        this_expression: &Expression,
        capture_slots: &BTreeMap<String, String>,
    ) -> DirectResult<()> {
        self.emit_user_function_call_with_new_target_and_this_expression_and_bound_captures_impl(
            user_function,
            arguments,
            new_target_value,
            this_expression,
            capture_slots,
            false,
        )
    }

    fn emit_user_function_call_with_new_target_and_this_expression_and_bound_captures_impl(
        &mut self,
        user_function: &UserFunction,
        arguments: &[CallArgument],
        new_target_value: i32,
        this_expression: &Expression,
        capture_slots: &BTreeMap<String, String>,
        enable_static_snapshot: bool,
    ) -> DirectResult<()> {
        let contains_super_call = self.user_function_body_contains_super_call(user_function);
        let runtime_only_parameter_iterator_call = contains_super_call
            || user_function.has_lowered_pattern_parameters()
            || !self
                .user_function_parameter_iterator_consumption_indices(user_function)
                .is_empty();
        let has_member_source_capture =
            self.bound_capture_slots_include_member_source(capture_slots);
        let allow_static_snapshot = enable_static_snapshot
            && !self.user_function_mentions_private_member_access(user_function);
        let allow_static_snapshot = allow_static_snapshot && !has_member_source_capture;
        let allow_static_snapshot = allow_static_snapshot && !contains_super_call;
        let allow_static_snapshot =
            allow_static_snapshot && !self.user_function_mentions_direct_eval(user_function);
        let expanded_arguments = self.expand_call_arguments(arguments);
        if self.emit_deferred_generator_call_result(user_function, &expanded_arguments)? {
            return Ok(());
        }
        let (
            prepared_capture_bindings,
            synced_capture_source_bindings,
            saved_new_target_local,
            saved_this_local,
            saved_this_shadow_owner,
        ) = self.prepare_bound_user_function_call_context(
            user_function,
            capture_slots,
            new_target_value,
            this_expression,
        )?;

        let static_result = if !runtime_only_parameter_iterator_call && allow_static_snapshot {
            let capture_snapshot = self
                .snapshot_prepared_bound_user_function_capture_bindings(&prepared_capture_bindings);
            self.resolve_bound_snapshot_user_function_result_with_arguments_and_this(
                &user_function.name,
                &capture_snapshot,
                &expanded_arguments,
                this_expression,
            )
        } else {
            None
        };
        let reliable_updated_bindings = static_result
            .as_ref()
            .map(|(_, updated_bindings)| updated_bindings.clone());
        let existing_snapshot = self
            .state
            .speculation
            .static_semantics
            .last_bound_user_function_call
            .clone()
            .filter(|snapshot| snapshot.function_name == user_function.name);
        self.state
            .speculation
            .static_semantics
            .last_bound_user_function_call =
            if !runtime_only_parameter_iterator_call && allow_static_snapshot {
                Some(BoundUserFunctionCallSnapshot {
                    function_name: user_function.name.clone(),
                    source_expression: None,
                    result_expression: static_result.as_ref().map(|(result, _)| result.clone()),
                    prototype_source_expression: None,
                    updated_bindings: reliable_updated_bindings.clone().unwrap_or_default(),
                })
            } else {
                existing_snapshot
            };
        let mut call_effect_nonlocal_bindings = if runtime_only_parameter_iterator_call {
            HashSet::new()
        } else {
            self.collect_user_function_call_effect_nonlocal_bindings(user_function)
        };
        if !runtime_only_parameter_iterator_call {
            call_effect_nonlocal_bindings.extend(
                self.collect_user_function_argument_call_effect_nonlocal_bindings(
                    user_function,
                    &expanded_arguments,
                ),
            );
        }
        let assigned_nonlocal_binding_results = if runtime_only_parameter_iterator_call {
            None
        } else {
            self.assigned_nonlocal_binding_results(&user_function.name)
                .cloned()
        };
        let member_source_capture_names = prepared_capture_bindings
            .iter()
            .filter_map(|binding| {
                binding
                    .source_binding_name
                    .as_ref()
                    .and_then(|source_name| {
                        Self::capture_slot_member_source_key_parts(source_name)
                            .map(|_| binding.capture_name.clone())
                    })
            })
            .collect::<HashSet<_>>();
        let closure_slot_capture_names = prepared_capture_bindings
            .iter()
            .filter_map(|binding| {
                binding
                    .source_binding_name
                    .as_ref()
                    .is_some_and(|source_name| source_name.starts_with("__ayy_closure_slot_"))
                    .then(|| binding.capture_name.clone())
            })
            .collect::<HashSet<_>>();
        let additional_call_effect_nonlocal_bindings = if runtime_only_parameter_iterator_call {
            HashSet::new()
        } else {
            let mut names = call_effect_nonlocal_bindings
                .iter()
                .filter(|name| {
                    if member_source_capture_names.contains(*name)
                        || closure_slot_capture_names.contains(*name)
                    {
                        return false;
                    }
                    !synced_capture_source_bindings.contains(*name)
                })
                .cloned()
                .collect::<HashSet<_>>();
            names.extend(
                self.collect_snapshot_updated_nonlocal_bindings(
                    user_function,
                    static_result
                        .as_ref()
                        .map(|(_, updated_bindings)| updated_bindings),
                ),
            );
            names.retain(|name| !member_source_capture_names.contains(name));
            names.retain(|name| !closure_slot_capture_names.contains(name));
            names.retain(|name| !synced_capture_source_bindings.contains(name));
            names
        };

        self.emit_prepare_bound_user_function_capture_globals(&prepared_capture_bindings)?;
        let static_argument_member_writebacks = self
            .user_function_static_argument_object_member_writeback_values(
                user_function,
                &expanded_arguments,
            );
        self.predeclare_static_argument_object_member_writeback_properties(
            &static_argument_member_writebacks,
        );
        let parameter_object_shadow_writebacks = self
            .emit_user_function_parameter_object_shadow_setup(user_function, &expanded_arguments)?;

        let visible_param_count = user_function.visible_param_count() as usize;
        let tracked_extra_indices = user_function
            .extra_argument_indices
            .iter()
            .map(|index| *index as usize)
            .collect::<HashSet<_>>();
        let mut argument_locals = HashMap::new();

        for (argument_index, argument) in expanded_arguments.iter().enumerate() {
            if argument_index < visible_param_count
                || tracked_extra_indices.contains(&argument_index)
            {
                let argument_local = self.allocate_temp_local();
                self.emit_numeric_expression(argument)?;
                self.push_local_set(argument_local);
                argument_locals.insert(argument_index, argument_local);
            } else {
                self.emit_numeric_expression(argument)?;
                self.state.emission.output.instructions.push(0x1a);
            }
        }

        for argument_index in 0..visible_param_count {
            if let Some(argument_local) = argument_locals.get(&argument_index).copied() {
                self.push_local_get(argument_local);
            } else {
                self.push_i32_const(JS_UNDEFINED_TAG);
            }
        }

        self.push_i32_const(expanded_arguments.len() as i32);

        for index in &user_function.extra_argument_indices {
            if let Some(argument_local) = argument_locals.get(&(*index as usize)).copied() {
                self.push_local_get(argument_local);
            } else {
                self.push_i32_const(JS_UNDEFINED_TAG);
            }
        }
        self.push_user_function_call(user_function);
        let return_value_local = self.allocate_temp_local();
        self.push_local_set(return_value_local);
        self.emit_user_function_parameter_object_shadow_writeback(
            &parameter_object_shadow_writebacks,
        )?;
        let receiver_updated_via_parameter_writeback = self
            .receiver_shadow_updated_via_parameter_writebacks(
                this_expression,
                &parameter_object_shadow_writebacks,
            );
        let updated_bindings = reliable_updated_bindings;
        self.sync_user_function_parameter_object_shadow_writeback_static_metadata(
            &parameter_object_shadow_writebacks,
            updated_bindings.as_ref(),
        );
        self.sync_static_argument_object_member_writeback_values(
            &static_argument_member_writebacks,
        );

        self.finalize_bound_user_function_call(
            user_function,
            this_expression,
            receiver_updated_via_parameter_writeback,
            &prepared_capture_bindings,
            updated_bindings,
            additional_call_effect_nonlocal_bindings,
            assigned_nonlocal_binding_results,
            saved_new_target_local,
            saved_this_local,
            saved_this_shadow_owner.as_deref(),
            return_value_local,
            &expanded_arguments,
        )?;
        self.sync_static_argument_object_member_writeback_values(
            &static_argument_member_writebacks,
        );
        Ok(())
    }
}
