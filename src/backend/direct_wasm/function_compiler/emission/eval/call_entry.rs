use super::*;

impl<'a> FunctionCompiler<'a> {
    fn expression_is_lowered_for_in_current_key(&self, expression: &Expression) -> bool {
        let Expression::Member { object, property } = expression else {
            return false;
        };
        matches!(
            (object.as_ref(), property.as_ref()),
            (Expression::Identifier(object_name), Expression::Identifier(property_name))
                if object_name.starts_with("__ayy_for_in_keys_")
                    && property_name.starts_with("__ayy_for_in_index_")
        )
    }

    fn statement_is_lowered_for_in_head_binding(&self, statement: &Statement) -> bool {
        match statement {
            Statement::Assign { value, .. } => self.expression_is_lowered_for_in_current_key(value),
            Statement::AssignMember { value, .. } => {
                self.expression_is_lowered_for_in_current_key(value)
            }
            _ => false,
        }
    }

    fn expression_is_lowered_for_of_step_value(&self, expression: &Expression) -> bool {
        let Expression::Member { object, property } = expression else {
            return false;
        };
        matches!(
            (object.as_ref(), property.as_ref()),
            (Expression::Identifier(object_name), Expression::String(property_name))
                if object_name.starts_with("__ayy_for_of_step_") && property_name == "value"
        )
    }

    fn statement_is_lowered_for_of_head_binding(&self, statement: &Statement) -> bool {
        match statement {
            Statement::Assign { value, .. } => self.expression_is_lowered_for_of_step_value(value),
            Statement::AssignMember { value, .. } => {
                self.expression_is_lowered_for_of_step_value(value)
            }
            _ => false,
        }
    }

    fn expression_is_symbol_dispose_property(expression: &Expression) -> bool {
        matches!(
            expression,
            Expression::Member { object, property }
                if matches!(object.as_ref(), Expression::Identifier(name) if name == "Symbol")
                    && matches!(property.as_ref(), Expression::String(name) if name == "dispose")
        )
    }

    fn statement_is_using_dispose_call(statement: &Statement) -> bool {
        matches!(
            statement,
            Statement::Expression(Expression::Call { callee, .. })
                if matches!(
                    callee.as_ref(),
                    Expression::Member { property, .. }
                        if Self::expression_is_symbol_dispose_property(property)
                )
        )
    }

    fn statement_is_using_finalizer(statement: &Statement) -> bool {
        let Statement::If { then_branch, .. } = statement else {
            return false;
        };
        match then_branch.as_slice() {
            [statement] => Self::statement_is_using_dispose_call(statement),
            [Statement::Assign { name, .. }, statement]
                if name.starts_with("__ayy_using_disposed_") =>
            {
                Self::statement_is_using_finalizer(statement)
            }
            _ => false,
        }
    }

    fn emit_eval_statement_list_completion_value(
        &mut self,
        statements: &[Statement],
        completion_local: u32,
    ) -> DirectResult<()> {
        for statement in statements {
            if Self::statement_is_using_finalizer(statement)
                || self.statement_is_lowered_for_in_head_binding(statement)
                || self.statement_is_lowered_for_of_head_binding(statement)
            {
                self.emit_statement(statement)?;
            } else {
                self.emit_eval_statement_completion_value(statement, completion_local)?;
            }
            if Self::statement_unconditionally_transfers_control(statement) {
                break;
            }
        }
        Ok(())
    }

    fn emit_eval_do_while_completion_value(
        &mut self,
        condition: &Expression,
        break_hook: Option<&Expression>,
        labels: &[String],
        body: &[Statement],
        completion_local: u32,
    ) -> DirectResult<()> {
        let invalidated_bindings = self
            .collect_loop_assigned_binding_names_with_effectful_iterators(
                condition, break_hook, body, None, None,
            );
        let preserved_kinds = self.preserved_binding_kinds_for_loop(
            &invalidated_bindings,
            condition,
            break_hook,
            body,
            None,
        );
        self.invalidate_static_binding_metadata_for_names_with_preserved_kinds(
            &invalidated_bindings,
            &preserved_kinds,
        );

        self.push_i32_const(JS_UNDEFINED_TAG);
        self.push_local_set(completion_local);

        self.state.emission.output.instructions.push(0x02);
        self.state
            .emission
            .output
            .instructions
            .push(EMPTY_BLOCK_TYPE);
        let break_target = self.push_control_frame();

        self.state.emission.output.instructions.push(0x03);
        self.state
            .emission
            .output
            .instructions
            .push(EMPTY_BLOCK_TYPE);
        let loop_target = self.push_control_frame();

        self.state.emission.output.instructions.push(0x02);
        self.state
            .emission
            .output
            .instructions
            .push(EMPTY_BLOCK_TYPE);
        let continue_target = self.push_control_frame();
        self.state
            .emission
            .control_flow
            .loop_stack
            .push(LoopContext {
                break_target,
                continue_target,
                labels: labels.to_vec(),
                assigned_bindings: invalidated_bindings.clone(),
                direct_step_iterators: std::collections::HashSet::new(),
                numeric_binding_candidates: HashMap::new(),
                numeric_spec: None,
            });
        self.state
            .emission
            .control_flow
            .break_stack
            .push(BreakContext {
                break_target,
                labels: labels.to_vec(),
                break_hook: break_hook.cloned(),
            });

        self.emit_eval_statement_list_completion_value(body, completion_local)?;
        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();

        self.emit_truthy_expression(condition)?;
        self.state.emission.output.instructions.push(0x45);
        self.push_br_if(self.relative_depth(break_target));
        self.push_br(self.relative_depth(loop_target));

        self.state.emission.control_flow.loop_stack.pop();
        self.state.emission.control_flow.break_stack.pop();
        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();
        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();
        self.invalidate_static_binding_metadata_for_names_with_preserved_kinds(
            &invalidated_bindings,
            &preserved_kinds,
        );
        Ok(())
    }

    fn emit_eval_while_completion_value(
        &mut self,
        condition: &Expression,
        break_hook: Option<&Expression>,
        labels: &[String],
        body: &[Statement],
        completion_local: u32,
    ) -> DirectResult<()> {
        let invalidated_bindings = self
            .collect_loop_assigned_binding_names_with_effectful_iterators(
                condition, break_hook, body, None, None,
            );
        let preserved_kinds = self.preserved_binding_kinds_for_loop(
            &invalidated_bindings,
            condition,
            break_hook,
            body,
            None,
        );
        self.invalidate_static_binding_metadata_for_names_with_preserved_kinds(
            &invalidated_bindings,
            &preserved_kinds,
        );

        self.push_i32_const(JS_UNDEFINED_TAG);
        self.push_local_set(completion_local);

        self.state.emission.output.instructions.push(0x02);
        self.state
            .emission
            .output
            .instructions
            .push(EMPTY_BLOCK_TYPE);
        let break_target = self.push_control_frame();

        self.state.emission.output.instructions.push(0x03);
        self.state
            .emission
            .output
            .instructions
            .push(EMPTY_BLOCK_TYPE);
        let continue_target = self.push_control_frame();
        self.state
            .emission
            .control_flow
            .loop_stack
            .push(LoopContext {
                break_target,
                continue_target,
                labels: labels.to_vec(),
                assigned_bindings: invalidated_bindings.clone(),
                direct_step_iterators: std::collections::HashSet::new(),
                numeric_binding_candidates: HashMap::new(),
                numeric_spec: None,
            });
        self.state
            .emission
            .control_flow
            .break_stack
            .push(BreakContext {
                break_target,
                labels: labels.to_vec(),
                break_hook: break_hook.cloned(),
            });

        self.emit_truthy_expression(condition)?;
        self.state.emission.output.instructions.push(0x45);
        self.push_br_if(self.relative_depth(break_target));
        self.emit_eval_statement_list_completion_value(body, completion_local)?;
        self.push_br(self.relative_depth(continue_target));

        self.state.emission.control_flow.loop_stack.pop();
        self.state.emission.control_flow.break_stack.pop();
        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();
        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();
        self.invalidate_static_binding_metadata_for_names_with_preserved_kinds(
            &invalidated_bindings,
            &preserved_kinds,
        );
        Ok(())
    }

    fn emit_eval_for_completion_value(
        &mut self,
        labels: &[String],
        init: &[Statement],
        per_iteration_bindings: &[String],
        condition: Option<&Expression>,
        update: Option<&Expression>,
        break_hook: Option<&Expression>,
        body: &[Statement],
        completion_local: u32,
    ) -> DirectResult<()> {
        let fallback_condition = Expression::Bool(true);
        let invalidated_bindings = self
            .collect_loop_assigned_binding_names_with_effectful_iterators(
                condition.unwrap_or(&fallback_condition),
                break_hook,
                body,
                Some(init),
                update,
            );
        self.with_active_eval_lexical_scope(per_iteration_bindings.to_vec(), |compiler| {
            compiler.emit_statements(init)?;
            let preserved_kinds = compiler.preserved_binding_kinds_for_loop(
                &invalidated_bindings,
                condition.unwrap_or(&fallback_condition),
                break_hook,
                body,
                update,
            );
            compiler.invalidate_static_binding_metadata_for_names_with_preserved_kinds(
                &invalidated_bindings,
                &preserved_kinds,
            );

            compiler.push_i32_const(JS_UNDEFINED_TAG);
            compiler.push_local_set(completion_local);

            compiler.state.emission.output.instructions.push(0x02);
            compiler
                .state
                .emission
                .output
                .instructions
                .push(EMPTY_BLOCK_TYPE);
            let break_target = compiler.push_control_frame();

            compiler.state.emission.output.instructions.push(0x03);
            compiler
                .state
                .emission
                .output
                .instructions
                .push(EMPTY_BLOCK_TYPE);
            let loop_target = compiler.push_control_frame();

            if let Some(condition) = condition {
                compiler.emit_truthy_expression(condition)?;
                compiler.state.emission.output.instructions.push(0x45);
                compiler.push_br_if(compiler.relative_depth(break_target));
            }

            compiler.state.emission.output.instructions.push(0x02);
            compiler
                .state
                .emission
                .output
                .instructions
                .push(EMPTY_BLOCK_TYPE);
            let continue_target = compiler.push_control_frame();
            compiler
                .state
                .emission
                .control_flow
                .loop_stack
                .push(LoopContext {
                    break_target,
                    continue_target,
                    labels: labels.to_vec(),
                    assigned_bindings: invalidated_bindings.clone(),
                    direct_step_iterators: std::collections::HashSet::new(),
                    numeric_binding_candidates: HashMap::new(),
                    numeric_spec: None,
                });
            compiler
                .state
                .emission
                .control_flow
                .break_stack
                .push(BreakContext {
                    break_target,
                    labels: labels.to_vec(),
                    break_hook: break_hook.cloned(),
                });

            compiler.emit_eval_statement_list_completion_value(body, completion_local)?;
            compiler.state.emission.output.instructions.push(0x0b);
            compiler.pop_control_frame();

            if let Some(update) = update {
                compiler.emit_numeric_expression(update)?;
                compiler.state.emission.output.instructions.push(0x1a);
            }
            compiler.push_br(compiler.relative_depth(loop_target));

            compiler.state.emission.control_flow.loop_stack.pop();
            compiler.state.emission.control_flow.break_stack.pop();
            compiler.state.emission.output.instructions.push(0x0b);
            compiler.pop_control_frame();
            compiler.state.emission.output.instructions.push(0x0b);
            compiler.pop_control_frame();
            compiler.invalidate_static_binding_metadata_for_names_with_preserved_kinds(
                &invalidated_bindings,
                &preserved_kinds,
            );
            Ok(())
        })
    }

    fn emit_eval_branch_completion_value(
        &mut self,
        body: &[Statement],
        completion_local: u32,
    ) -> DirectResult<()> {
        self.push_i32_const(JS_UNDEFINED_TAG);
        self.push_local_set(completion_local);
        self.emit_eval_statement_list_completion_value(body, completion_local)
    }

    fn emit_eval_with_completion_value(
        &mut self,
        object: &Expression,
        body: &[Statement],
        completion_local: u32,
    ) -> DirectResult<()> {
        let object_kind = self.infer_value_kind(object);
        self.emit_numeric_expression(object)?;
        self.state.emission.output.instructions.push(0x1a);
        if matches!(
            object_kind,
            Some(StaticValueKind::Null | StaticValueKind::Undefined)
        ) {
            self.emit_named_error_throw("TypeError")?;
            return Ok(());
        }

        let with_scope = self.canonicalize_with_scope_expression(object);
        self.state.push_with_scope(with_scope);
        self.push_i32_const(JS_UNDEFINED_TAG);
        self.push_local_set(completion_local);
        let result = self.emit_eval_statement_list_completion_value(body, completion_local);
        self.state.pop_with_scope();
        result
    }

    fn eval_if_condition_requires_runtime_check(condition: &Expression) -> bool {
        let mut referenced_names = HashSet::new();
        collect_referenced_binding_names_from_expression(condition, &mut referenced_names);
        referenced_names
            .iter()
            .any(|name| name.starts_with("__ayy_finally_"))
    }

    fn emit_eval_if_completion_value(
        &mut self,
        condition: &Expression,
        then_branch: &[Statement],
        else_branch: &[Statement],
        completion_local: u32,
    ) -> DirectResult<()> {
        if !Self::eval_if_condition_requires_runtime_check(condition)
            && let Some(condition_value) = self.resolve_static_if_condition_value(condition)
        {
            self.emit_numeric_expression(condition)?;
            self.state.emission.output.instructions.push(0x1a);
            let branch = if condition_value {
                then_branch
            } else {
                else_branch
            };
            return self.emit_eval_branch_completion_value(branch, completion_local);
        }

        let mut branch_invalidated_bindings = HashSet::new();
        for statement in then_branch {
            collect_assigned_binding_names_from_statement(
                statement,
                &mut branch_invalidated_bindings,
            );
        }
        for statement in else_branch {
            collect_assigned_binding_names_from_statement(
                statement,
                &mut branch_invalidated_bindings,
            );
        }
        let base_static_metadata = self.state.snapshot_static_binding_metadata();

        self.emit_truthy_expression(condition)?;
        self.state.emission.output.instructions.push(0x04);
        self.state
            .emission
            .output
            .instructions
            .push(EMPTY_BLOCK_TYPE);
        self.push_control_frame();

        let then_static_metadata =
            self.with_restored_static_binding_metadata_snapshot(|compiler| {
                compiler.push_i32_const(JS_UNDEFINED_TAG);
                compiler.push_local_set(completion_local);
                if let Some((name, narrowed_expression)) =
                    compiler.conditional_defined_binding_narrowing(condition, true)
                {
                    compiler.with_narrowed_local_binding_metadata(
                        &name,
                        &narrowed_expression,
                        |compiler| {
                            compiler.emit_eval_statement_list_completion_value(
                                then_branch,
                                completion_local,
                            )
                        },
                    )
                } else {
                    compiler
                        .emit_eval_statement_list_completion_value(then_branch, completion_local)
                }
            })?;

        if !else_branch.is_empty() {
            self.seed_runtime_array_metadata_for_names_from_snapshot(
                &then_static_metadata,
                &branch_invalidated_bindings,
            );
        }
        self.state.emission.output.instructions.push(0x05);

        let else_static_metadata =
            self.with_restored_static_binding_metadata_snapshot(|compiler| {
                compiler.push_i32_const(JS_UNDEFINED_TAG);
                compiler.push_local_set(completion_local);
                if let Some((name, narrowed_expression)) =
                    compiler.conditional_defined_binding_narrowing(condition, false)
                {
                    compiler.with_narrowed_local_binding_metadata(
                        &name,
                        &narrowed_expression,
                        |compiler| {
                            compiler.emit_eval_statement_list_completion_value(
                                else_branch,
                                completion_local,
                            )
                        },
                    )
                } else {
                    compiler
                        .emit_eval_statement_list_completion_value(else_branch, completion_local)
                }
            })?;

        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();
        self.invalidate_static_binding_metadata_for_names(&branch_invalidated_bindings);
        self.merge_dynamic_branch_static_binding_metadata(
            &branch_invalidated_bindings,
            &base_static_metadata,
            &then_static_metadata,
            Some(&else_static_metadata),
        );
        Ok(())
    }

    fn emit_eval_labeled_completion_value(
        &mut self,
        labels: &[String],
        body: &[Statement],
        completion_local: u32,
    ) -> DirectResult<()> {
        self.state.emission.output.instructions.push(0x02);
        self.state
            .emission
            .output
            .instructions
            .push(EMPTY_BLOCK_TYPE);
        let break_target = self.push_control_frame();
        self.state
            .emission
            .control_flow
            .break_stack
            .push(BreakContext {
                break_target,
                labels: labels.to_vec(),
                break_hook: None,
            });

        self.emit_eval_statement_list_completion_value(body, completion_local)?;

        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();
        self.state.emission.control_flow.break_stack.pop();
        Ok(())
    }

    fn emit_eval_switch_match_start_update(
        &mut self,
        case_index: usize,
        test: &Expression,
        start_local: u32,
        discriminant_local: u32,
    ) -> DirectResult<()> {
        self.push_local_get(start_local);
        self.push_i32_const(-1);
        self.push_binary_op(BinaryOp::Equal)?;
        self.push_local_get(discriminant_local);
        self.emit_numeric_expression(test)?;
        self.push_binary_op(BinaryOp::Equal)?;
        self.state.emission.output.instructions.push(0x71);

        self.state.emission.output.instructions.push(0x04);
        self.state
            .emission
            .output
            .instructions
            .push(EMPTY_BLOCK_TYPE);
        self.push_control_frame();

        self.push_i32_const(case_index as i32);
        self.push_local_set(start_local);

        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();
        Ok(())
    }

    fn emit_eval_switch_default_start_update(
        &mut self,
        default_index: usize,
        start_local: u32,
    ) -> DirectResult<()> {
        self.push_local_get(start_local);
        self.push_i32_const(-1);
        self.push_binary_op(BinaryOp::Equal)?;

        self.state.emission.output.instructions.push(0x04);
        self.state
            .emission
            .output
            .instructions
            .push(EMPTY_BLOCK_TYPE);
        self.push_control_frame();

        self.push_i32_const(default_index as i32);
        self.push_local_set(start_local);

        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();
        Ok(())
    }

    fn emit_eval_switch_case_body_completion_value(
        &mut self,
        case_index: usize,
        case: &crate::ir::hir::SwitchCase,
        start_local: u32,
        completion_local: u32,
    ) -> DirectResult<()> {
        self.push_local_get(start_local);
        self.push_i32_const(-1);
        self.push_binary_op(BinaryOp::NotEqual)?;
        self.push_local_get(start_local);
        self.push_i32_const(case_index as i32);
        self.push_binary_op(BinaryOp::LessThanOrEqual)?;
        self.state.emission.output.instructions.push(0x71);

        self.state.emission.output.instructions.push(0x04);
        self.state
            .emission
            .output
            .instructions
            .push(EMPTY_BLOCK_TYPE);
        self.push_control_frame();

        for case_statement in &case.body {
            self.emit_eval_statement_completion_value(case_statement, completion_local)?;
        }

        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();
        Ok(())
    }

    fn emit_eval_switch_completion_value(
        &mut self,
        labels: &[String],
        bindings: &[String],
        discriminant: &Expression,
        cases: &[crate::ir::hir::SwitchCase],
        completion_local: u32,
    ) -> DirectResult<()> {
        let mut invalidated_bindings = HashSet::new();
        collect_assigned_binding_names_from_expression(discriminant, &mut invalidated_bindings);
        for case in cases {
            if let Some(test) = &case.test {
                collect_assigned_binding_names_from_expression(test, &mut invalidated_bindings);
            }
            for statement in &case.body {
                collect_assigned_binding_names_from_statement(statement, &mut invalidated_bindings);
            }
        }
        self.invalidate_static_binding_metadata_for_names(&invalidated_bindings);

        self.push_i32_const(JS_UNDEFINED_TAG);
        self.push_local_set(completion_local);

        self.state.emission.output.instructions.push(0x02);
        self.state
            .emission
            .output
            .instructions
            .push(EMPTY_BLOCK_TYPE);
        let break_target = self.push_control_frame();
        self.state
            .emission
            .control_flow
            .break_stack
            .push(BreakContext {
                break_target,
                labels: labels.to_vec(),
                break_hook: None,
            });

        let discriminant_local = self.allocate_temp_local();
        let start_local = self.allocate_temp_local();

        self.emit_numeric_expression(discriminant)?;
        self.push_local_set(discriminant_local);
        self.push_i32_const(-1);
        self.push_local_set(start_local);

        self.with_active_eval_lexical_scope(bindings.to_vec(), |compiler| {
            let default_index = cases.iter().position(|case| case.test.is_none());
            let split_index = default_index.unwrap_or(cases.len());

            for (case_index, case) in cases.iter().enumerate().take(split_index) {
                if let Some(test) = &case.test {
                    compiler.emit_eval_switch_match_start_update(
                        case_index,
                        test,
                        start_local,
                        discriminant_local,
                    )?;
                }
            }

            if let Some(default_index) = default_index {
                for (case_index, case) in cases.iter().enumerate().skip(default_index + 1) {
                    if let Some(test) = &case.test {
                        compiler.emit_eval_switch_match_start_update(
                            case_index,
                            test,
                            start_local,
                            discriminant_local,
                        )?;
                    }
                }
                compiler.emit_eval_switch_default_start_update(default_index, start_local)?;
            } else {
                for (case_index, case) in cases.iter().enumerate().skip(split_index) {
                    if let Some(test) = &case.test {
                        compiler.emit_eval_switch_match_start_update(
                            case_index,
                            test,
                            start_local,
                            discriminant_local,
                        )?;
                    }
                }
            }

            for (case_index, case) in cases.iter().enumerate() {
                compiler.emit_eval_switch_case_body_completion_value(
                    case_index,
                    case,
                    start_local,
                    completion_local,
                )?;
            }
            Ok(())
        })?;

        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();
        self.state.emission.control_flow.break_stack.pop();
        self.invalidate_static_binding_metadata_for_names(&invalidated_bindings);
        Ok(())
    }

    fn emit_eval_try_completion_value(
        &mut self,
        body: &[Statement],
        catch_binding: Option<&String>,
        catch_setup: &[Statement],
        catch_body: &[Statement],
        completion_local: u32,
    ) -> DirectResult<()> {
        let static_catch_value =
            catch_binding.and_then(|_| self.resolve_terminal_throw_value_from_try_body(body));

        self.push_i32_const(JS_UNDEFINED_TAG);
        self.push_local_set(completion_local);

        self.state.emission.output.instructions.push(0x02);
        self.state
            .emission
            .output
            .instructions
            .push(EMPTY_BLOCK_TYPE);
        let catch_target = self.push_control_frame();
        self.state
            .emission
            .control_flow
            .try_stack
            .push(TryContext { catch_target });

        self.emit_eval_statement_list_completion_value(body, completion_local)?;

        self.clear_local_throw_state();
        self.clear_global_throw_state();

        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();
        self.state.emission.control_flow.try_stack.pop();

        self.push_local_get(self.state.runtime.throws.throw_tag_local);
        self.push_i32_const(0);
        self.push_binary_op(BinaryOp::NotEqual)?;
        self.state.emission.output.instructions.push(0x04);
        self.state
            .emission
            .output
            .instructions
            .push(EMPTY_BLOCK_TYPE);
        self.push_control_frame();

        if let Some(catch_binding) = catch_binding {
            let catch_local = self.lookup_local(catch_binding)?;
            self.push_local_get(self.state.runtime.throws.throw_value_local);
            self.push_local_set(catch_local);
            let mut invalidated_bindings = HashSet::new();
            invalidated_bindings.insert(catch_binding.clone());
            self.invalidate_static_binding_metadata_for_names(&invalidated_bindings);
            if let Some(static_catch_value) = static_catch_value.as_ref() {
                self.update_capture_slot_binding_from_expression(
                    catch_binding,
                    static_catch_value,
                )?;
            }
        } else {
            self.push_i32_const(JS_UNDEFINED_TAG);
            self.push_local_set(self.state.runtime.throws.throw_value_local);
        }

        self.clear_local_throw_state();
        self.clear_global_throw_state();

        let mut catch_scope_bindings = collect_direct_eval_lexical_binding_names(catch_setup);
        catch_scope_bindings.extend(collect_direct_eval_lexical_binding_names(catch_body));
        if let Some(catch_binding) = catch_binding {
            catch_scope_bindings.push(catch_binding.clone());
        }
        self.with_active_eval_lexical_scope(catch_scope_bindings, |compiler| {
            compiler.push_i32_const(JS_UNDEFINED_TAG);
            compiler.push_local_set(completion_local);
            if !catch_setup.is_empty() {
                compiler.emit_statements(catch_setup)?;
            }
            if !catch_body.is_empty() {
                compiler.emit_eval_statement_list_completion_value(catch_body, completion_local)?;
            }
            Ok(())
        })?;

        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();
        Ok(())
    }

    fn object_prototype_to_string_result_for_callee(&self, callee: &Expression) -> Expression {
        let tag = match callee {
            Expression::Member { object, .. } => {
                let materialized = self.materialize_static_expression(object);
                if matches!(materialized, Expression::Undefined) {
                    "Undefined"
                } else if matches!(materialized, Expression::Null) {
                    "Null"
                } else if matches!(materialized, Expression::Array(_))
                    || self.resolve_array_binding_from_expression(object).is_some()
                {
                    "Array"
                } else if self
                    .resolve_function_binding_from_expression(object)
                    .is_some()
                {
                    "Function"
                } else if matches!(materialized, Expression::String(_)) {
                    "String"
                } else if matches!(materialized, Expression::Bool(_)) {
                    "Boolean"
                } else if matches!(materialized, Expression::Number(_)) {
                    "Number"
                } else if matches!(materialized, Expression::BigInt(_)) {
                    "BigInt"
                } else {
                    "Object"
                }
            }
            Expression::SuperMember { .. } => "Object",
            _ => "Undefined",
        };
        Expression::String(format!("[object {tag}]"))
    }

    pub(in crate::backend::direct_wasm) fn emit_eval_statement_completion_value(
        &mut self,
        statement: &Statement,
        completion_local: u32,
    ) -> DirectResult<()> {
        match statement {
            Statement::Expression(expression) => {
                self.emit_numeric_expression(expression)?;
                self.push_local_tee(completion_local);
                self.state.emission.output.instructions.push(0x1a);
            }
            Statement::Assign { name, value } => {
                if name.starts_with("__ayy_") {
                    self.emit_statement(statement)?;
                    return Ok(());
                }
                self.emit_numeric_expression(&Expression::Assign {
                    name: name.clone(),
                    value: Box::new(value.clone()),
                })?;
                self.push_local_tee(completion_local);
                self.state.emission.output.instructions.push(0x1a);
            }
            Statement::AssignMember {
                object,
                property,
                value,
            } => {
                self.emit_numeric_expression(&Expression::AssignMember {
                    object: Box::new(object.clone()),
                    property: Box::new(property.clone()),
                    value: Box::new(value.clone()),
                })?;
                self.push_local_tee(completion_local);
                self.state.emission.output.instructions.push(0x1a);
            }
            Statement::Block { body } | Statement::Declaration { body } => {
                self.emit_eval_statement_list_completion_value(body, completion_local)?;
            }
            Statement::Labeled { labels, body } => {
                self.emit_eval_labeled_completion_value(labels, body, completion_local)?;
            }
            Statement::With { object, body } => {
                self.emit_eval_with_completion_value(object, body, completion_local)?;
            }
            Statement::If {
                condition,
                then_branch,
                else_branch,
            } => {
                if Self::eval_if_condition_requires_runtime_check(condition) {
                    self.emit_statement(statement)?;
                } else {
                    self.emit_eval_if_completion_value(
                        condition,
                        then_branch,
                        else_branch,
                        completion_local,
                    )?;
                }
            }
            Statement::DoWhile {
                condition,
                body,
                break_hook,
                labels,
            } => {
                self.emit_eval_do_while_completion_value(
                    condition,
                    break_hook.as_ref(),
                    labels,
                    body,
                    completion_local,
                )?;
            }
            Statement::While {
                condition,
                body,
                break_hook,
                labels,
            } => {
                self.emit_eval_while_completion_value(
                    condition,
                    break_hook.as_ref(),
                    labels,
                    body,
                    completion_local,
                )?;
            }
            Statement::For {
                labels,
                init,
                per_iteration_bindings,
                condition,
                update,
                break_hook,
                body,
            } => {
                self.emit_eval_for_completion_value(
                    labels,
                    init,
                    per_iteration_bindings,
                    condition.as_ref(),
                    update.as_ref(),
                    break_hook.as_ref(),
                    body,
                    completion_local,
                )?;
            }
            Statement::Try {
                body,
                catch_binding,
                catch_setup,
                catch_body,
            } => {
                self.emit_eval_try_completion_value(
                    body,
                    catch_binding.as_ref(),
                    catch_setup,
                    catch_body,
                    completion_local,
                )?;
            }
            Statement::Switch {
                labels,
                bindings,
                discriminant,
                cases,
            } => {
                self.emit_eval_switch_completion_value(
                    labels,
                    bindings,
                    discriminant,
                    cases,
                    completion_local,
                )?;
            }
            _ => self.emit_statement(statement)?,
        }

        Ok(())
    }

    pub(in crate::backend::direct_wasm) fn emit_builtin_call_for_callee(
        &mut self,
        callee: &Expression,
        name: &str,
        arguments: &[CallArgument],
        construct: bool,
    ) -> DirectResult<bool> {
        if !construct && name == "Object.prototype.toString" {
            self.emit_ignored_call_arguments(arguments)?;
            let value = self.object_prototype_to_string_result_for_callee(callee);
            self.emit_numeric_expression(&value)?;
            return Ok(true);
        }

        if name == "eval" {
            if matches!(callee, Expression::Identifier(identifier) if identifier == "eval") {
                return self.emit_eval_call(arguments);
            }
            if let Expression::Member { object, property } = callee
                && matches!(property.as_ref(), Expression::String(property_name) if property_name == "eval")
                && let Some(realm_id) = self.resolve_test262_realm_global_id_from_expression(object)
            {
                let realm_eval_name = test262_realm_eval_builtin_name(realm_id);
                return self
                    .emit_indirect_eval_call_with_context(arguments, Some(&realm_eval_name));
            }
            return self.emit_indirect_eval_call(arguments);
        }

        if !construct
            && let Some(value) = self.resolve_static_builtin_primitive_call_value(
                name,
                arguments,
                self.current_function_name(),
            )
        {
            for argument in arguments {
                match argument {
                    CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                        self.emit_numeric_expression(expression)?;
                        self.state.emission.output.instructions.push(0x1a);
                    }
                }
            }
            self.emit_numeric_expression(&value)?;
            return Ok(true);
        }

        self.emit_builtin_call(name, arguments)
    }
}
