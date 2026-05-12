use super::*;

impl<'a> FunctionCompiler<'a> {
    pub(in crate::backend::direct_wasm) fn emit_user_function_call_with_current_new_target_and_this_expression(
        &mut self,
        user_function: &UserFunction,
        arguments: &[CallArgument],
        this_expression: &Expression,
    ) -> DirectResult<()> {
        let expanded_arguments = self.expand_call_arguments(arguments);
        let prepared_capture_bindings =
            self.prepare_user_function_capture_bindings(user_function)?;
        let synced_capture_source_bindings =
            self.synced_prepared_user_function_capture_source_bindings(&prepared_capture_bindings);
        let capture_snapshot =
            self.snapshot_user_function_capture_source_bindings(&prepared_capture_bindings);
        let static_this_expression = self.resolve_static_snapshot_this_expression(this_expression);
        let runtime_only_parameter_iterator_call = user_function.has_lowered_pattern_parameters()
            || !self
                .user_function_parameter_iterator_consumption_indices(user_function)
                .is_empty();
        let updated_bindings = (!runtime_only_parameter_iterator_call
            && !self.user_function_body_contains_new_target(user_function))
        .then(|| {
            self.resolve_bound_snapshot_user_function_result_with_arguments_and_this(
                &user_function.name,
                &capture_snapshot,
                &expanded_arguments,
                &static_this_expression,
            )
        })
        .flatten()
        .map(|(_, updated_bindings)| updated_bindings);
        let assigned_nonlocal_bindings =
            self.collect_user_function_assigned_nonlocal_bindings(user_function);
        let mut call_effect_nonlocal_bindings =
            self.collect_user_function_call_effect_nonlocal_bindings(user_function);
        call_effect_nonlocal_bindings.extend(
            self.collect_user_function_argument_call_effect_nonlocal_bindings(
                user_function,
                &expanded_arguments,
            ),
        );
        let updated_nonlocal_bindings =
            self.collect_user_function_updated_nonlocal_bindings(user_function);
        let assigned_nonlocal_binding_results = if runtime_only_parameter_iterator_call {
            None
        } else {
            self.assigned_nonlocal_binding_results(&user_function.name)
                .cloned()
        };
        let mut additional_call_effect_nonlocal_bindings = call_effect_nonlocal_bindings
            .iter()
            .filter(|name| !synced_capture_source_bindings.contains(*name))
            .cloned()
            .collect::<HashSet<_>>();
        additional_call_effect_nonlocal_bindings.extend(
            self.collect_snapshot_updated_nonlocal_bindings(
                user_function,
                updated_bindings.as_ref(),
            ),
        );

        let saved_this_local = if user_function.lexical_this {
            None
        } else {
            let saved_local = self.allocate_temp_local();
            let this_local = self.allocate_temp_local();
            self.push_global_get(CURRENT_THIS_GLOBAL_INDEX);
            self.push_local_set(saved_local);
            self.emit_numeric_expression(this_expression)?;
            self.push_local_set(this_local);
            self.push_local_get(this_local);
            self.push_global_set(CURRENT_THIS_GLOBAL_INDEX);
            Some(saved_local)
        };
        let saved_this_shadow_owner = if user_function.lexical_this {
            None
        } else {
            self.prepare_user_function_runtime_this_shadow_state(this_expression)?
        };
        let allow_static_this_shadow_commit = self
            .user_function_call_allows_static_this_shadow_commit(user_function, this_expression);

        self.emit_prepare_user_function_capture_globals(&user_function.name)?;
        let parameter_object_shadow_writebacks = self
            .emit_user_function_parameter_object_shadow_setup(user_function, &expanded_arguments)?;

        let visible_param_count = user_function.visible_param_count() as usize;
        let rest_parameter_index = self.user_function_rest_parameter_index(user_function);
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
            if Some(argument_index) == rest_parameter_index {
                self.push_i32_const(JS_TYPEOF_OBJECT_TAG);
            } else if let Some(argument_local) = argument_locals.get(&argument_index).copied() {
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
        self.sync_user_function_parameter_object_shadow_writeback_static_metadata(
            &parameter_object_shadow_writebacks,
            updated_bindings.as_ref(),
        );
        let receiver_updated_via_parameter_writeback = self
            .receiver_shadow_updated_via_parameter_writebacks(
                this_expression,
                &parameter_object_shadow_writebacks,
            );

        self.sync_user_function_capture_source_bindings(
            &prepared_capture_bindings,
            &assigned_nonlocal_bindings,
            &call_effect_nonlocal_bindings,
            &updated_nonlocal_bindings,
            updated_bindings.as_ref(),
            saved_this_shadow_owner.as_deref(),
        )?;
        self.restore_user_function_capture_bindings(&prepared_capture_bindings);
        let additional_call_effect_nonlocal_bindings = self
            .sync_snapshot_user_function_call_effect_bindings(
                &additional_call_effect_nonlocal_bindings,
                updated_bindings.as_ref(),
                updated_bindings
                    .as_ref()
                    .map(|_| assigned_nonlocal_binding_results.as_ref())
                    .flatten(),
            )?;
        if !additional_call_effect_nonlocal_bindings.is_empty() {
            let preserved_kinds = additional_call_effect_nonlocal_bindings
                .iter()
                .filter(|name| !assigned_nonlocal_bindings.contains(*name))
                .filter_map(|name| {
                    self.lookup_identifier_kind(name)
                        .map(|kind| (name.clone(), kind))
                })
                .collect::<HashMap<_, _>>();
            self.invalidate_static_binding_metadata_for_names_with_preserved_kinds(
                &additional_call_effect_nonlocal_bindings,
                &preserved_kinds,
            );
        }
        self.sync_consumed_iterator_bindings_for_user_call(user_function);
        self.sync_argument_iterator_bindings_for_user_call(user_function, &expanded_arguments);
        let receiver_may_require_invalidation = assigned_nonlocal_bindings.contains("this")
            || updated_nonlocal_bindings.contains("this");
        self.finalize_user_function_runtime_this_shadow_state(
            user_function,
            this_expression,
            updated_bindings.as_ref(),
            saved_this_shadow_owner.as_deref(),
            allow_static_this_shadow_commit,
            receiver_updated_via_parameter_writeback,
            receiver_may_require_invalidation,
        )?;

        if let Some(saved_this_local) = saved_this_local {
            self.push_local_get(saved_this_local);
            self.push_global_set(CURRENT_THIS_GLOBAL_INDEX);
        }

        if user_function.is_async() {
            self.push_global_get(THROW_TAG_GLOBAL_INDEX);
            self.push_i32_const(0);
            self.push_binary_op(BinaryOp::NotEqual)?;
            self.state.emission.output.instructions.push(0x04);
            self.state
                .emission
                .output
                .instructions
                .push(EMPTY_BLOCK_TYPE);
            self.push_control_frame();
            self.clear_global_throw_state();
            self.state.emission.output.instructions.push(0x0b);
            self.pop_control_frame();
            self.push_i32_const(JS_TYPEOF_OBJECT_TAG);
            return Ok(());
        }

        self.emit_check_global_throw_for_user_call()?;
        self.push_local_get(return_value_local);
        Ok(())
    }

    fn user_function_body_contains_new_target(&self, user_function: &UserFunction) -> bool {
        self.resolve_registered_function_declaration(&user_function.name)
            .is_some_and(|function| {
                function
                    .body
                    .iter()
                    .any(Self::statement_contains_new_target)
            })
    }

    fn statement_contains_new_target(statement: &Statement) -> bool {
        match statement {
            Statement::Declaration { body }
            | Statement::Block { body }
            | Statement::Labeled { body, .. } => {
                body.iter().any(Self::statement_contains_new_target)
            }
            Statement::Var { value, .. }
            | Statement::Let { value, .. }
            | Statement::Assign { value, .. }
            | Statement::Expression(value)
            | Statement::Throw(value)
            | Statement::Return(value)
            | Statement::Yield { value }
            | Statement::YieldDelegate { value } => Self::expression_contains_new_target(value),
            Statement::AssignMember {
                object,
                property,
                value,
            } => {
                Self::expression_contains_new_target(object)
                    || Self::expression_contains_new_target(property)
                    || Self::expression_contains_new_target(value)
            }
            Statement::Print { values } => values.iter().any(Self::expression_contains_new_target),
            Statement::With { object, body } => {
                Self::expression_contains_new_target(object)
                    || body.iter().any(Self::statement_contains_new_target)
            }
            Statement::If {
                condition,
                then_branch,
                else_branch,
            } => {
                Self::expression_contains_new_target(condition)
                    || then_branch.iter().any(Self::statement_contains_new_target)
                    || else_branch.iter().any(Self::statement_contains_new_target)
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
                .any(Self::statement_contains_new_target),
            Statement::Switch {
                discriminant,
                cases,
                ..
            } => {
                Self::expression_contains_new_target(discriminant)
                    || cases.iter().any(|case| {
                        case.test
                            .as_ref()
                            .is_some_and(Self::expression_contains_new_target)
                            || case.body.iter().any(Self::statement_contains_new_target)
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
                init.iter().any(Self::statement_contains_new_target)
                    || condition
                        .as_ref()
                        .is_some_and(Self::expression_contains_new_target)
                    || update
                        .as_ref()
                        .is_some_and(Self::expression_contains_new_target)
                    || break_hook
                        .as_ref()
                        .is_some_and(Self::expression_contains_new_target)
                    || body.iter().any(Self::statement_contains_new_target)
            }
            Statement::While {
                condition,
                break_hook,
                body,
                ..
            }
            | Statement::DoWhile {
                condition,
                break_hook,
                body,
                ..
            } => {
                Self::expression_contains_new_target(condition)
                    || break_hook
                        .as_ref()
                        .is_some_and(Self::expression_contains_new_target)
                    || body.iter().any(Self::statement_contains_new_target)
            }
            Statement::Break { .. } | Statement::Continue { .. } => false,
        }
    }

    fn expression_contains_new_target(expression: &Expression) -> bool {
        match expression {
            Expression::NewTarget => true,
            Expression::Array(elements) => elements.iter().any(|element| match element {
                ArrayElement::Expression(expression) | ArrayElement::Spread(expression) => {
                    Self::expression_contains_new_target(expression)
                }
            }),
            Expression::Object(entries) => entries.iter().any(|entry| match entry {
                ObjectEntry::Data { key, value } => {
                    Self::expression_contains_new_target(key)
                        || Self::expression_contains_new_target(value)
                }
                ObjectEntry::Getter { key, getter } => {
                    Self::expression_contains_new_target(key)
                        || Self::expression_contains_new_target(getter)
                }
                ObjectEntry::Setter { key, setter } => {
                    Self::expression_contains_new_target(key)
                        || Self::expression_contains_new_target(setter)
                }
                ObjectEntry::Spread(expression) => Self::expression_contains_new_target(expression),
            }),
            Expression::Member { object, property } => {
                Self::expression_contains_new_target(object)
                    || Self::expression_contains_new_target(property)
            }
            Expression::SuperMember { property } => Self::expression_contains_new_target(property),
            Expression::Assign { value, .. }
            | Expression::Await(value)
            | Expression::EnumerateKeys(value)
            | Expression::GetIterator(value)
            | Expression::IteratorClose(value)
            | Expression::Unary {
                expression: value, ..
            } => Self::expression_contains_new_target(value),
            Expression::AssignMember {
                object,
                property,
                value,
            } => {
                Self::expression_contains_new_target(object)
                    || Self::expression_contains_new_target(property)
                    || Self::expression_contains_new_target(value)
            }
            Expression::AssignSuperMember { property, value } => {
                Self::expression_contains_new_target(property)
                    || Self::expression_contains_new_target(value)
            }
            Expression::Binary { left, right, .. } => {
                Self::expression_contains_new_target(left)
                    || Self::expression_contains_new_target(right)
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                Self::expression_contains_new_target(condition)
                    || Self::expression_contains_new_target(then_expression)
                    || Self::expression_contains_new_target(else_expression)
            }
            Expression::Sequence(expressions) => {
                expressions.iter().any(Self::expression_contains_new_target)
            }
            Expression::Call { callee, arguments }
            | Expression::SuperCall { callee, arguments }
            | Expression::New { callee, arguments } => {
                Self::expression_contains_new_target(callee)
                    || arguments.iter().any(|argument| match argument {
                        CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                            Self::expression_contains_new_target(expression)
                        }
                    })
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
            | Expression::Sent => false,
        }
    }
}
