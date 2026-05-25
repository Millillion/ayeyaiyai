use super::*;

const INSTANCE_FIELD_INITIALIZER_LABEL: &str = "__ayy_instance_field_initializers";

fn is_using_completion_binding(name: &str) -> bool {
    name.starts_with("__ayy_using_error_")
}

fn control_if_condition_references_compiler_finally(condition: &Expression) -> bool {
    let mut referenced_names = HashSet::new();
    collect_referenced_binding_names_from_expression(condition, &mut referenced_names);
    referenced_names
        .iter()
        .any(|name| name.starts_with("__ayy_finally_"))
}

fn control_if_condition_references_internal_iterator_temp(condition: &Expression) -> bool {
    let mut referenced_names = HashSet::new();
    collect_referenced_binding_names_from_expression(condition, &mut referenced_names);
    referenced_names.iter().any(|name| {
        name.starts_with("__ayy_binding_value_")
            || name.starts_with("__ayy_array_step_")
            || name.starts_with("__ayy_array_iter_value_")
            || name.starts_with("__ayy_array_iter_done_")
            || name.starts_with("__ayy_for_of_step_")
            || name.starts_with("__ayy_for_of_iter_value_")
            || name.starts_with("__ayy_for_of_iter_done_")
    })
}

#[derive(Clone)]
struct DestructuringDefaultIteratorClosePattern {
    condition: Expression,
    target_name: String,
    then_value: Expression,
    default_value: Expression,
    close_condition: Expression,
    close_target: Expression,
}

impl<'a> FunctionCompiler<'a> {
    fn concrete_static_binding_value_condition_operand(
        &self,
        expression: Expression,
    ) -> Option<Expression> {
        match expression {
            Expression::Identifier(name)
                if name == "undefined" && self.is_unshadowed_builtin_identifier(&name) =>
            {
                Some(Expression::Undefined)
            }
            Expression::Number(_)
            | Expression::BigInt(_)
            | Expression::String(_)
            | Expression::Bool(_)
            | Expression::Null
            | Expression::Undefined => Some(expression),
            Expression::Array(_) | Expression::Object(_) => Some(Expression::Object(Vec::new())),
            _ => None,
        }
    }

    fn identifier_undefined_not_equal_condition_binding(condition: &Expression) -> Option<&str> {
        let Expression::Binary {
            op: BinaryOp::NotEqual,
            left,
            right,
        } = condition
        else {
            return None;
        };
        match (left.as_ref(), right.as_ref()) {
            (Expression::Identifier(name), Expression::Undefined)
            | (Expression::Undefined, Expression::Identifier(name)) => Some(name),
            _ => None,
        }
    }

    fn static_binding_value_condition_operand(
        &self,
        expression: &Expression,
    ) -> Option<Expression> {
        match expression {
            Expression::Identifier(name) if name.starts_with("__ayy_binding_value_") => {
                if let Some(value) = self
                    .state
                    .speculation
                    .static_semantics
                    .local_value_binding(name)
                {
                    let materialized = self.materialize_static_expression(value);
                    if let Some(operand) =
                        self.concrete_static_binding_value_condition_operand(materialized)
                    {
                        return Some(operand);
                    }
                    return None;
                }
                if self
                    .state
                    .speculation
                    .static_semantics
                    .has_local_object_binding(name)
                    || self
                        .state
                        .speculation
                        .static_semantics
                        .has_local_array_binding(name)
                    || self
                        .state
                        .speculation
                        .static_semantics
                        .has_local_function_binding(name)
                {
                    return Some(Expression::Object(Vec::new()));
                }
                match self.state.speculation.static_semantics.local_kind(name) {
                    Some(StaticValueKind::Null) => Some(Expression::Null),
                    Some(StaticValueKind::Undefined) => Some(Expression::Undefined),
                    Some(
                        StaticValueKind::Object
                        | StaticValueKind::Function
                        | StaticValueKind::Symbol,
                    ) => Some(Expression::Object(Vec::new())),
                    _ => None,
                }
            }
            Expression::Null | Expression::Undefined | Expression::Bool(_) => {
                Some(expression.clone())
            }
            _ => None,
        }
    }

    fn resolve_static_binding_value_condition(&self, condition: &Expression) -> Option<bool> {
        let Expression::Binary { op, left, right } = condition else {
            return None;
        };
        match op {
            BinaryOp::LogicalOr => {
                if self.resolve_static_binding_value_condition(left)? {
                    Some(true)
                } else {
                    self.resolve_static_binding_value_condition(right)
                }
            }
            BinaryOp::LogicalAnd => {
                if !self.resolve_static_binding_value_condition(left)? {
                    Some(false)
                } else {
                    self.resolve_static_binding_value_condition(right)
                }
            }
            BinaryOp::Equal
            | BinaryOp::NotEqual
            | BinaryOp::LooseEqual
            | BinaryOp::LooseNotEqual => {
                let left = self.static_binding_value_condition_operand(left)?;
                let right = self.static_binding_value_condition_operand(right)?;
                let loosely_equal_nullish = matches!(
                    (&left, &right),
                    (Expression::Null, Expression::Undefined)
                        | (Expression::Undefined, Expression::Null)
                );
                let equal = static_expression_matches(&left, &right);
                match op {
                    BinaryOp::Equal => Some(equal),
                    BinaryOp::NotEqual => Some(!equal),
                    BinaryOp::LooseEqual => Some(equal || loosely_equal_nullish),
                    BinaryOp::LooseNotEqual => Some(!(equal || loosely_equal_nullish)),
                    _ => unreachable!("filtered by enclosing match arm"),
                }
            }
            _ => None,
        }
    }

    fn iterator_close_expression_from_guard(
        statement: &Statement,
    ) -> Option<(&Expression, &Expression)> {
        let Statement::If {
            condition,
            then_branch,
            else_branch,
        } = statement
        else {
            return None;
        };
        if !else_branch.is_empty() {
            return None;
        }
        let [Statement::Expression(Expression::IteratorClose(close_target))] =
            then_branch.as_slice()
        else {
            return None;
        };
        Some((condition, close_target.as_ref()))
    }

    fn destructuring_default_iterator_close_pattern(
        statement: &Statement,
        next_statement: Option<&Statement>,
    ) -> Option<DestructuringDefaultIteratorClosePattern> {
        let trace = std::env::var_os("AYY_TRACE_DESTRUCTURING_CLOSE").is_some();
        let Statement::If {
            condition,
            then_branch,
            else_branch,
        } = statement
        else {
            return None;
        };
        let Some(binding_name) = Self::identifier_undefined_not_equal_condition_binding(condition)
        else {
            if trace {
                eprintln!("destructuring_close:skip condition={condition:?}");
            }
            return None;
        };
        if !binding_name.starts_with("__ayy_binding_value_") {
            if trace {
                eprintln!("destructuring_close:skip binding_name={binding_name}");
            }
            return None;
        }
        let [
            Statement::Assign {
                name: then_name,
                value: then_value,
            },
        ] = then_branch.as_slice()
        else {
            if trace {
                eprintln!("destructuring_close:skip then_branch={then_branch:?}");
            }
            return None;
        };
        let [
            Statement::Assign {
                name: else_name,
                value: default_value,
            },
        ] = else_branch.as_slice()
        else {
            if trace {
                eprintln!("destructuring_close:skip else_branch={else_branch:?}");
            }
            return None;
        };
        if then_name != else_name {
            if trace {
                eprintln!(
                    "destructuring_close:skip target mismatch then={then_name} else={else_name}"
                );
            }
            return None;
        }
        if !matches!(then_value, Expression::Identifier(name) if name == binding_name) {
            if trace {
                eprintln!(
                    "destructuring_close:skip then_value={then_value:?} binding={binding_name}"
                );
            }
            return None;
        }
        let Some(next_statement) = next_statement else {
            if trace {
                eprintln!("destructuring_close:skip missing next");
            }
            return None;
        };
        let Some((close_condition, close_target)) =
            Self::iterator_close_expression_from_guard(next_statement)
        else {
            if trace {
                eprintln!("destructuring_close:skip next={next_statement:?}");
            }
            return None;
        };
        if trace {
            eprintln!("destructuring_close:match target={then_name} close={close_target:?}");
        }
        Some(DestructuringDefaultIteratorClosePattern {
            condition: condition.clone(),
            target_name: then_name.clone(),
            then_value: then_value.clone(),
            default_value: default_value.clone(),
            close_condition: close_condition.clone(),
            close_target: close_target.clone(),
        })
    }

    fn emit_assignment_store_from_local(
        &mut self,
        scoped_target: Option<Expression>,
        name: &str,
        value_expression: &Expression,
        value_local: u32,
    ) -> DirectResult<()> {
        if let Some(scope_object) = scoped_target {
            self.emit_scoped_property_store_from_local(
                &scope_object,
                name,
                value_local,
                value_expression,
            )?;
            self.state.emission.output.instructions.push(0x1a);
        } else {
            self.emit_store_identifier_value_local(name, value_expression, value_local)?;
        }
        Ok(())
    }

    fn emit_expression_to_local_catching_abrupt_completion(
        &mut self,
        expression: &Expression,
        value_local: u32,
    ) -> DirectResult<()> {
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

        self.emit_numeric_expression(expression)?;
        self.push_local_set(value_local);
        self.clear_local_throw_state();
        self.clear_global_throw_state();

        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();
        self.state.emission.control_flow.try_stack.pop();
        Ok(())
    }

    fn emit_iterator_close_swallowing_abrupt_completion(
        &mut self,
        close_target: &Expression,
    ) -> DirectResult<()> {
        self.emit_statement(&Statement::Try {
            body: vec![Statement::Expression(Expression::IteratorClose(Box::new(
                close_target.clone(),
            )))],
            catch_binding: None,
            catch_setup: Vec::new(),
            catch_body: Vec::new(),
        })
    }

    fn emit_iterator_close_then_rethrow_if_local_throw(
        &mut self,
        close_condition: &Expression,
        close_target: &Expression,
    ) -> DirectResult<()> {
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

        let saved_throw_value_local = self.allocate_temp_local();
        let saved_throw_tag_local = self.allocate_temp_local();
        self.push_local_get(self.state.runtime.throws.throw_value_local);
        self.push_local_set(saved_throw_value_local);
        self.push_local_get(self.state.runtime.throws.throw_tag_local);
        self.push_local_set(saved_throw_tag_local);
        self.clear_local_throw_state();
        self.clear_global_throw_state();

        self.emit_truthy_expression(close_condition)?;
        self.state.emission.output.instructions.push(0x04);
        self.state
            .emission
            .output
            .instructions
            .push(EMPTY_BLOCK_TYPE);
        self.push_control_frame();
        self.emit_iterator_close_swallowing_abrupt_completion(close_target)?;
        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();

        self.push_local_get(saved_throw_value_local);
        self.push_local_set(self.state.runtime.throws.throw_value_local);
        self.push_local_get(saved_throw_tag_local);
        self.push_local_set(self.state.runtime.throws.throw_tag_local);
        self.emit_throw_from_locals()?;

        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();
        Ok(())
    }

    fn emit_destructuring_default_then_assignment(
        &mut self,
        name: &str,
        value: &Expression,
    ) -> DirectResult<()> {
        let scoped_target = self.resolve_with_scope_binding(name)?;
        let value_local = self.allocate_temp_local();
        self.emit_numeric_expression(value)?;
        self.push_local_set(value_local);
        self.emit_assignment_store_from_local(scoped_target, name, value, value_local)
    }

    fn emit_destructuring_default_else_assignment_with_close(
        &mut self,
        pattern: &DestructuringDefaultIteratorClosePattern,
    ) -> DirectResult<()> {
        let scoped_target = self.resolve_with_scope_binding(&pattern.target_name)?;
        let value_local = self.allocate_temp_local();
        self.emit_expression_to_local_catching_abrupt_completion(
            &pattern.default_value,
            value_local,
        )?;
        self.emit_iterator_close_then_rethrow_if_local_throw(
            &pattern.close_condition,
            &pattern.close_target,
        )?;
        self.emit_assignment_store_from_local(
            scoped_target,
            &pattern.target_name,
            &pattern.default_value,
            value_local,
        )
    }

    pub(in crate::backend::direct_wasm) fn try_emit_destructuring_default_iterator_close_statement(
        &mut self,
        statement: &Statement,
        next_statement: Option<&Statement>,
    ) -> DirectResult<bool> {
        let Some(pattern) =
            Self::destructuring_default_iterator_close_pattern(statement, next_statement)
        else {
            return Ok(false);
        };

        let mut branch_invalidated_bindings = HashSet::new();
        branch_invalidated_bindings.insert(pattern.target_name.clone());
        collect_assigned_binding_names_from_expression(
            &pattern.default_value,
            &mut branch_invalidated_bindings,
        );
        let mut close_updated_bindings = HashSet::new();
        let close_statement = Statement::Expression(Expression::IteratorClose(Box::new(
            pattern.close_target.clone(),
        )));
        self.collect_iterator_close_updated_binding_names_from_statement(
            &close_statement,
            &mut close_updated_bindings,
        );
        branch_invalidated_bindings.extend(close_updated_bindings.iter().cloned());
        let base_static_metadata = self.state.snapshot_static_binding_metadata();
        let base_global_static_semantics = self.backend.snapshot_global_static_semantics();

        self.emit_truthy_expression(&pattern.condition)?;
        self.state.emission.output.instructions.push(0x04);
        self.state
            .emission
            .output
            .instructions
            .push(EMPTY_BLOCK_TYPE);
        self.push_control_frame();

        let (then_static_metadata, then_global_static_semantics) = self
            .with_restored_static_binding_metadata_snapshots(|compiler| {
                compiler.emit_destructuring_default_then_assignment(
                    &pattern.target_name,
                    &pattern.then_value,
                )
            })?;

        self.state.emission.output.instructions.push(0x05);

        let (else_static_metadata, else_global_static_semantics) = self
            .with_restored_static_binding_metadata_snapshots(|compiler| {
                compiler.emit_destructuring_default_else_assignment_with_close(&pattern)
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
        self.merge_dynamic_branch_global_static_binding_metadata(
            &branch_invalidated_bindings,
            &base_global_static_semantics,
            &then_global_static_semantics,
            Some(&else_global_static_semantics),
        );
        self.invalidate_static_binding_metadata_for_names(&close_updated_bindings);
        Ok(true)
    }

    fn static_if_condition_reads_runtime_nonlocal_binding(&self, condition: &Expression) -> bool {
        if self.current_function_name().is_none() {
            return false;
        }

        let mut referenced_names = HashSet::new();
        collect_referenced_binding_names_from_expression(condition, &mut referenced_names);
        referenced_names.iter().any(|name| {
            let source_name = scoped_binding_source_name(name).unwrap_or(name);
            self.resolve_current_local_binding(source_name).is_none()
                && (self.global_has_binding(source_name)
                    || self.global_has_implicit_binding(source_name)
                    || self
                        .resolve_user_function_capture_hidden_name(source_name)
                        .is_some())
        })
    }

    fn static_if_condition_calls_user_function(&self, expression: &Expression) -> bool {
        match expression {
            Expression::Call { callee, arguments }
            | Expression::SuperCall { callee, arguments }
            | Expression::New { callee, arguments } => {
                matches!(callee.as_ref(), Expression::Identifier(name) if self.contains_user_function(name))
                    || matches!(
                        self.resolve_function_binding_from_expression(callee),
                        Some(LocalFunctionBinding::User(_))
                    )
                    || self.static_if_condition_calls_user_function(callee)
                    || arguments.iter().any(|argument| match argument {
                        CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                            self.static_if_condition_calls_user_function(expression)
                        }
                    })
            }
            Expression::Unary {
                op: UnaryOp::Delete,
                ..
            } => false,
            Expression::Unary { expression, .. }
            | Expression::Await(expression)
            | Expression::EnumerateKeys(expression)
            | Expression::GetIterator(expression)
            | Expression::IteratorClose(expression)
            | Expression::Assign {
                value: expression, ..
            } => self.static_if_condition_calls_user_function(expression),
            Expression::AssignMember {
                object,
                property,
                value,
            } => {
                self.static_if_condition_calls_user_function(object)
                    || self.static_if_condition_calls_user_function(property)
                    || self.static_if_condition_calls_user_function(value)
            }
            Expression::AssignSuperMember { property, value } => {
                self.static_if_condition_calls_user_function(property)
                    || self.static_if_condition_calls_user_function(value)
            }
            Expression::Binary { left, right, .. } => {
                self.static_if_condition_calls_user_function(left)
                    || self.static_if_condition_calls_user_function(right)
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                self.static_if_condition_calls_user_function(condition)
                    || self.static_if_condition_calls_user_function(then_expression)
                    || self.static_if_condition_calls_user_function(else_expression)
            }
            Expression::Sequence(expressions) => expressions
                .iter()
                .any(|expression| self.static_if_condition_calls_user_function(expression)),
            Expression::Array(elements) => elements.iter().any(|element| match element {
                ArrayElement::Expression(expression) | ArrayElement::Spread(expression) => {
                    self.static_if_condition_calls_user_function(expression)
                }
            }),
            Expression::Object(entries) => entries.iter().any(|entry| match entry {
                ObjectEntry::Data { key, value } => {
                    self.static_if_condition_calls_user_function(key)
                        || self.static_if_condition_calls_user_function(value)
                }
                ObjectEntry::Getter { key, getter } => {
                    self.static_if_condition_calls_user_function(key)
                        || self.static_if_condition_calls_user_function(getter)
                }
                ObjectEntry::Setter { key, setter } => {
                    self.static_if_condition_calls_user_function(key)
                        || self.static_if_condition_calls_user_function(setter)
                }
                ObjectEntry::Spread(expression) => {
                    self.static_if_condition_calls_user_function(expression)
                }
            }),
            Expression::Member { object, property } => {
                self.static_if_condition_calls_user_function(object)
                    || self.static_if_condition_calls_user_function(property)
            }
            Expression::SuperMember { property } => {
                self.static_if_condition_calls_user_function(property)
            }
            Expression::Identifier(_)
            | Expression::Update { .. }
            | Expression::Number(_)
            | Expression::BigInt(_)
            | Expression::String(_)
            | Expression::Bool(_)
            | Expression::Null
            | Expression::Undefined
            | Expression::NewTarget
            | Expression::This
            | Expression::Sent => false,
        }
    }

    fn static_if_condition_side_effects_can_be_skipped(&self, expression: &Expression) -> bool {
        if inline_summary_side_effect_free_expression(expression) {
            return true;
        }
        match expression {
            Expression::Call { callee, arguments } => {
                if matches!(callee.as_ref(), Expression::Identifier(name) if name == "__ayyImportMeta")
                    && matches!(
                        arguments.as_slice(),
                        [] | [CallArgument::Expression(Expression::Number(_))]
                            | [CallArgument::Spread(Expression::Number(_))]
                    )
                {
                    return true;
                }
                self.resolve_static_has_own_property_call_result(expression)
                    .is_some()
                    && self.static_if_condition_side_effects_can_be_skipped(callee)
                    && arguments.iter().all(|argument| match argument {
                        CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                            self.static_if_condition_side_effects_can_be_skipped(expression)
                        }
                    })
            }
            Expression::Unary {
                op: UnaryOp::Delete,
                ..
            } => false,
            Expression::Unary { expression, .. }
            | Expression::Await(expression)
            | Expression::EnumerateKeys(expression)
            | Expression::GetIterator(expression)
            | Expression::IteratorClose(expression) => {
                self.static_if_condition_side_effects_can_be_skipped(expression)
            }
            Expression::Binary { left, right, .. } => {
                self.static_if_condition_side_effects_can_be_skipped(left)
                    && self.static_if_condition_side_effects_can_be_skipped(right)
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                self.static_if_condition_side_effects_can_be_skipped(condition)
                    && self.static_if_condition_side_effects_can_be_skipped(then_expression)
                    && self.static_if_condition_side_effects_can_be_skipped(else_expression)
            }
            Expression::Sequence(expressions) => expressions
                .iter()
                .all(|expression| self.static_if_condition_side_effects_can_be_skipped(expression)),
            _ => false,
        }
    }

    fn restore_try_metadata_map_entry<T: Clone>(
        target: &mut HashMap<String, T>,
        source: &HashMap<String, T>,
        name: &str,
    ) {
        if let Some(value) = source.get(name).cloned() {
            target.insert(name.to_string(), value);
        } else {
            target.remove(name);
        }
    }

    fn restore_local_try_body_binding_metadata_for_name(
        &mut self,
        name: &str,
        snapshot: &FunctionStaticBindingMetadataSnapshot,
    ) {
        let static_semantics = &mut self.state.speculation.static_semantics;
        Self::restore_try_metadata_map_entry(
            &mut static_semantics.values.local_kinds,
            &snapshot.values.local_kinds,
            name,
        );
        Self::restore_try_metadata_map_entry(
            &mut static_semantics.values.local_value_bindings,
            &snapshot.values.local_value_bindings,
            name,
        );
        Self::restore_try_metadata_map_entry(
            &mut static_semantics.values.local_function_bindings,
            &snapshot.values.local_function_bindings,
            name,
        );
        Self::restore_try_metadata_map_entry(
            &mut static_semantics.values.local_specialized_function_values,
            &snapshot.values.local_specialized_function_values,
            name,
        );
        Self::restore_try_metadata_map_entry(
            &mut static_semantics.values.local_proxy_bindings,
            &snapshot.values.local_proxy_bindings,
            name,
        );
        Self::restore_try_metadata_map_entry(
            &mut static_semantics.objects.local_object_bindings,
            &snapshot.objects.local_object_bindings,
            name,
        );
        Self::restore_try_metadata_map_entry(
            &mut static_semantics.objects.local_prototype_object_bindings,
            &snapshot.objects.local_prototype_object_bindings,
            name,
        );
        Self::restore_try_metadata_map_entry(
            &mut static_semantics.objects.local_descriptor_bindings,
            &snapshot.objects.local_descriptor_bindings,
            name,
        );
        Self::restore_try_metadata_map_entry(
            &mut static_semantics.arrays.local_array_bindings,
            &snapshot.arrays.local_array_bindings,
            name,
        );
        Self::restore_try_metadata_map_entry(
            &mut static_semantics
                .arrays
                .local_resizable_array_buffer_bindings,
            &snapshot.arrays.local_resizable_array_buffer_bindings,
            name,
        );
        Self::restore_try_metadata_map_entry(
            &mut static_semantics.arrays.local_typed_array_view_bindings,
            &snapshot.arrays.local_typed_array_view_bindings,
            name,
        );
        Self::restore_try_metadata_map_entry(
            &mut static_semantics.arrays.local_array_iterator_bindings,
            &snapshot.arrays.local_array_iterator_bindings,
            name,
        );
        Self::restore_try_metadata_map_entry(
            &mut static_semantics.arrays.cached_iterator_next_method_bindings,
            &snapshot.arrays.cached_iterator_next_method_bindings,
            name,
        );
        Self::restore_try_metadata_map_entry(
            &mut static_semantics.arrays.local_iterator_step_bindings,
            &snapshot.arrays.local_iterator_step_bindings,
            name,
        );
        Self::restore_try_metadata_map_entry(
            &mut static_semantics.arrays.runtime_array_length_locals,
            &snapshot.arrays.runtime_array_length_locals,
            name,
        );
    }

    fn restore_global_try_body_binding_metadata_for_name(
        &mut self,
        name: &str,
        snapshot: &GlobalStaticSemanticsSnapshot,
    ) {
        let global_semantics = &mut self.backend.global_semantics;
        Self::restore_try_metadata_map_entry(
            &mut global_semantics.names.kinds,
            &snapshot.names.kinds,
            name,
        );
        Self::restore_try_metadata_map_entry(
            &mut global_semantics.values.value_bindings,
            &snapshot.values.value_bindings,
            name,
        );
        Self::restore_try_metadata_map_entry(
            &mut global_semantics.values.array_bindings,
            &snapshot.values.array_bindings,
            name,
        );
        Self::restore_try_metadata_map_entry(
            &mut global_semantics.values.object_bindings,
            &snapshot.values.object_bindings,
            name,
        );
        Self::restore_try_metadata_map_entry(
            &mut global_semantics.values.property_descriptors,
            &snapshot.values.property_descriptors,
            name,
        );
        Self::restore_try_metadata_map_entry(
            &mut global_semantics.values.object_prototype_bindings,
            &snapshot.values.object_prototype_bindings,
            name,
        );
        Self::restore_try_metadata_map_entry(
            &mut global_semantics.values.runtime_prototype_bindings,
            &snapshot.values.runtime_prototype_bindings,
            name,
        );
        Self::restore_try_metadata_map_entry(
            &mut global_semantics.values.prototype_object_bindings,
            &snapshot.values.prototype_object_bindings,
            name,
        );
        Self::restore_try_metadata_map_entry(
            &mut global_semantics.values.arguments_bindings,
            &snapshot.values.arguments_bindings,
            name,
        );
        Self::restore_try_metadata_map_entry(
            &mut global_semantics.values.proxy_bindings,
            &snapshot.values.proxy_bindings,
            name,
        );
        Self::restore_try_metadata_map_entry(
            &mut global_semantics.functions.function_bindings,
            &snapshot.functions.function_bindings,
            name,
        );
        Self::restore_try_metadata_map_entry(
            &mut global_semantics.functions.specialized_function_values,
            &snapshot.functions.specialized_function_values,
            name,
        );
        if snapshot.values.arrays_with_runtime_state.contains(name) {
            global_semantics
                .values
                .arrays_with_runtime_state
                .insert(name.to_string());
        } else {
            global_semantics
                .values
                .arrays_with_runtime_state
                .remove(name);
        }
    }

    fn restore_try_body_binding_metadata_for_names(
        &mut self,
        names: &HashSet<String>,
        local_snapshot: &FunctionStaticBindingMetadataSnapshot,
        global_snapshot: &GlobalStaticSemanticsSnapshot,
    ) {
        for name in names {
            self.restore_local_try_body_binding_metadata_for_name(name, local_snapshot);
            self.restore_global_try_body_binding_metadata_for_name(name, global_snapshot);
        }
    }

    pub(in crate::backend::direct_wasm) fn statement_preserves_try_metadata_before_terminal_throw(
        &self,
        statement: &Statement,
    ) -> bool {
        match statement {
            Statement::Declaration { body }
            | Statement::Block { body }
            | Statement::Labeled { body, .. } => body.iter().all(|statement| {
                self.statement_preserves_try_metadata_before_terminal_throw(statement)
            }),
            Statement::Var { value, .. }
            | Statement::Let { value, .. }
            | Statement::Assign { value, .. }
            | Statement::Expression(value) => {
                self.expression_preserves_try_metadata_before_terminal_throw(value)
            }
            Statement::With { object, body } => {
                inline_summary_side_effect_free_expression(object)
                    && body.iter().all(|statement| {
                        self.statement_preserves_try_metadata_before_terminal_throw(statement)
                    })
            }
            Statement::Try {
                body,
                catch_binding: None,
                catch_setup,
                catch_body,
            } if catch_setup.is_empty() && catch_body.is_empty() => body.iter().all(|statement| {
                self.statement_preserves_try_metadata_before_terminal_throw(statement)
            }),
            _ => false,
        }
    }

    fn expression_preserves_try_metadata_before_terminal_throw(
        &self,
        expression: &Expression,
    ) -> bool {
        inline_summary_side_effect_free_expression(expression)
            && self
                .resolve_terminal_expression_throw_value(expression)
                .is_none()
            && self.expression_has_no_uncertain_try_prefix_coercion(expression)
    }

    fn expression_has_no_uncertain_try_prefix_coercion(&self, expression: &Expression) -> bool {
        match expression {
            Expression::Unary { op, expression } => {
                self.expression_has_no_uncertain_try_prefix_coercion(expression)
                    && self
                        .unary_operand_preserves_try_metadata_before_terminal_throw(*op, expression)
            }
            Expression::Binary { left, right, .. } => {
                self.expression_has_no_uncertain_try_prefix_coercion(left)
                    && self.expression_has_no_uncertain_try_prefix_coercion(right)
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                self.expression_has_no_uncertain_try_prefix_coercion(condition)
                    && self.expression_has_no_uncertain_try_prefix_coercion(then_expression)
                    && self.expression_has_no_uncertain_try_prefix_coercion(else_expression)
            }
            Expression::Sequence(expressions) => expressions
                .iter()
                .all(|expression| self.expression_has_no_uncertain_try_prefix_coercion(expression)),
            Expression::Array(elements) => elements.iter().all(|element| match element {
                ArrayElement::Expression(expression) | ArrayElement::Spread(expression) => {
                    self.expression_has_no_uncertain_try_prefix_coercion(expression)
                }
            }),
            Expression::Object(entries) => entries.iter().all(|entry| match entry {
                ObjectEntry::Data { key, value } => {
                    self.expression_has_no_uncertain_try_prefix_coercion(key)
                        && self.expression_has_no_uncertain_try_prefix_coercion(value)
                }
                ObjectEntry::Getter { key, getter } => {
                    self.expression_has_no_uncertain_try_prefix_coercion(key)
                        && self.expression_has_no_uncertain_try_prefix_coercion(getter)
                }
                ObjectEntry::Setter { key, setter } => {
                    self.expression_has_no_uncertain_try_prefix_coercion(key)
                        && self.expression_has_no_uncertain_try_prefix_coercion(setter)
                }
                ObjectEntry::Spread(expression) => {
                    self.expression_has_no_uncertain_try_prefix_coercion(expression)
                }
            }),
            Expression::Member { object, property } => {
                self.expression_has_no_uncertain_try_prefix_coercion(object)
                    && self.expression_has_no_uncertain_try_prefix_coercion(property)
            }
            Expression::SuperMember { property } => {
                self.expression_has_no_uncertain_try_prefix_coercion(property)
            }
            Expression::Await(expression)
            | Expression::EnumerateKeys(expression)
            | Expression::GetIterator(expression)
            | Expression::IteratorClose(expression) => {
                self.expression_has_no_uncertain_try_prefix_coercion(expression)
            }
            Expression::Number(_)
            | Expression::BigInt(_)
            | Expression::String(_)
            | Expression::Bool(_)
            | Expression::Null
            | Expression::Undefined
            | Expression::Identifier(_)
            | Expression::This
            | Expression::NewTarget
            | Expression::Sent => true,
            Expression::Assign { .. }
            | Expression::AssignMember { .. }
            | Expression::AssignSuperMember { .. }
            | Expression::Call { .. }
            | Expression::SuperCall { .. }
            | Expression::New { .. }
            | Expression::Update { .. } => false,
        }
    }

    fn unary_operand_preserves_try_metadata_before_terminal_throw(
        &self,
        op: UnaryOp,
        expression: &Expression,
    ) -> bool {
        match op {
            UnaryOp::Plus => matches!(
                self.infer_value_kind(expression),
                Some(
                    StaticValueKind::Number
                        | StaticValueKind::String
                        | StaticValueKind::Bool
                        | StaticValueKind::Null
                        | StaticValueKind::Undefined
                )
            ),
            UnaryOp::Negate | UnaryOp::BitwiseNot => matches!(
                self.infer_value_kind(expression),
                Some(
                    StaticValueKind::Number
                        | StaticValueKind::BigInt
                        | StaticValueKind::String
                        | StaticValueKind::Bool
                        | StaticValueKind::Null
                        | StaticValueKind::Undefined
                )
            ),
            UnaryOp::Not | UnaryOp::TypeOf | UnaryOp::Void | UnaryOp::Delete => true,
        }
    }

    pub(in crate::backend::direct_wasm) fn statement_has_deterministic_terminal_throw(
        &self,
        statement: &Statement,
    ) -> bool {
        match statement {
            Statement::Declaration { body }
            | Statement::Block { body }
            | Statement::Labeled { body, .. } => {
                self.statements_have_deterministic_terminal_throw(body)
            }
            Statement::With { object, body } => {
                inline_summary_side_effect_free_expression(object)
                    && self.statements_have_deterministic_terminal_throw(body)
            }
            Statement::Throw(expression) => inline_summary_side_effect_free_expression(expression),
            Statement::Expression(expression) => self
                .resolve_terminal_expression_throw_value(expression)
                .is_some(),
            _ => false,
        }
    }

    fn statements_have_deterministic_terminal_throw(&self, statements: &[Statement]) -> bool {
        let Some((last, prefix)) = statements.split_last() else {
            return false;
        };
        prefix
            .iter()
            .all(|statement| self.statement_preserves_try_metadata_before_terminal_throw(statement))
            && self.statement_has_deterministic_terminal_throw(last)
    }

    fn collect_iterator_close_updated_binding_names_from_expression(
        &mut self,
        expression: &Expression,
        names: &mut HashSet<String>,
    ) {
        match expression {
            Expression::IteratorClose(value) => {
                let return_property = Expression::String("return".to_string());
                if let Some(updated_bindings) =
                    self.resolve_iterator_close_updated_bindings(value, &return_property)
                {
                    for (name, updated_value) in &updated_bindings {
                        let source_name =
                            scoped_binding_source_name(name).unwrap_or(name).to_string();
                        if self.should_sync_async_delegate_snapshot_binding(&source_name) {
                            names.insert(source_name.clone());
                            if self.resolve_current_local_binding(&source_name).is_none()
                                && (self.global_has_binding(&source_name)
                                    || self.global_has_implicit_binding(&source_name))
                                && self
                                    .resolve_array_binding_from_expression(updated_value)
                                    .is_some()
                            {
                                self.backend
                                    .mark_global_array_with_runtime_state(&source_name);
                            }
                        }
                    }
                }
                self.collect_iterator_close_updated_binding_names_from_expression(value, names);
            }
            Expression::Member { object, property } => {
                self.collect_iterator_close_updated_binding_names_from_expression(object, names);
                self.collect_iterator_close_updated_binding_names_from_expression(property, names);
            }
            Expression::SuperMember { property } => {
                self.collect_iterator_close_updated_binding_names_from_expression(property, names);
            }
            Expression::Assign { value, .. }
            | Expression::Await(value)
            | Expression::EnumerateKeys(value)
            | Expression::GetIterator(value)
            | Expression::Unary {
                expression: value, ..
            } => self.collect_iterator_close_updated_binding_names_from_expression(value, names),
            Expression::AssignMember {
                object,
                property,
                value,
            } => {
                self.collect_iterator_close_updated_binding_names_from_expression(object, names);
                self.collect_iterator_close_updated_binding_names_from_expression(property, names);
                self.collect_iterator_close_updated_binding_names_from_expression(value, names);
            }
            Expression::AssignSuperMember { property, value } => {
                self.collect_iterator_close_updated_binding_names_from_expression(property, names);
                self.collect_iterator_close_updated_binding_names_from_expression(value, names);
            }
            Expression::Binary { left, right, .. } => {
                self.collect_iterator_close_updated_binding_names_from_expression(left, names);
                self.collect_iterator_close_updated_binding_names_from_expression(right, names);
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                self.collect_iterator_close_updated_binding_names_from_expression(condition, names);
                self.collect_iterator_close_updated_binding_names_from_expression(
                    then_expression,
                    names,
                );
                self.collect_iterator_close_updated_binding_names_from_expression(
                    else_expression,
                    names,
                );
            }
            Expression::Sequence(expressions) => {
                for expression in expressions {
                    self.collect_iterator_close_updated_binding_names_from_expression(
                        expression, names,
                    );
                }
            }
            Expression::Call { callee, arguments }
            | Expression::SuperCall { callee, arguments }
            | Expression::New { callee, arguments } => {
                self.collect_iterator_close_updated_binding_names_from_expression(callee, names);
                for argument in arguments {
                    match argument {
                        CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                            self.collect_iterator_close_updated_binding_names_from_expression(
                                expression, names,
                            );
                        }
                    }
                }
            }
            Expression::Array(elements) => {
                for element in elements {
                    match element {
                        ArrayElement::Expression(expression) | ArrayElement::Spread(expression) => {
                            self.collect_iterator_close_updated_binding_names_from_expression(
                                expression, names,
                            );
                        }
                    }
                }
            }
            Expression::Object(entries) => {
                for entry in entries {
                    match entry {
                        ObjectEntry::Data { key, value } => {
                            self.collect_iterator_close_updated_binding_names_from_expression(
                                key, names,
                            );
                            self.collect_iterator_close_updated_binding_names_from_expression(
                                value, names,
                            );
                        }
                        ObjectEntry::Getter { key, getter } => {
                            self.collect_iterator_close_updated_binding_names_from_expression(
                                key, names,
                            );
                            self.collect_iterator_close_updated_binding_names_from_expression(
                                getter, names,
                            );
                        }
                        ObjectEntry::Setter { key, setter } => {
                            self.collect_iterator_close_updated_binding_names_from_expression(
                                key, names,
                            );
                            self.collect_iterator_close_updated_binding_names_from_expression(
                                setter, names,
                            );
                        }
                        ObjectEntry::Spread(expression) => {
                            self.collect_iterator_close_updated_binding_names_from_expression(
                                expression, names,
                            );
                        }
                    }
                }
            }
            Expression::Identifier(_)
            | Expression::Number(_)
            | Expression::BigInt(_)
            | Expression::String(_)
            | Expression::Bool(_)
            | Expression::Null
            | Expression::Undefined
            | Expression::This
            | Expression::NewTarget
            | Expression::Sent
            | Expression::Update { .. } => {}
        }
    }

    fn collect_simple_generator_source_effect_binding_names(
        &self,
        source: &Expression,
        names: &mut HashSet<String>,
    ) {
        if let Some(prefix_effects) = self.simple_generator_call_time_prefix_effects(source) {
            for effect in prefix_effects {
                collect_assigned_binding_names_from_statement(&effect, names);
            }
        }
        let Some((steps, completion_effects, _)) = self.resolve_simple_generator_source(source)
        else {
            return;
        };
        for step in steps {
            for effect in step.effects.iter().chain(step.close_effects.iter()) {
                collect_assigned_binding_names_from_statement(effect, names);
            }
        }
        for effect in completion_effects {
            collect_assigned_binding_names_from_statement(&effect, names);
        }
    }

    fn simple_generator_call_expression_for_value(&self, value: &Expression) -> Option<Expression> {
        let materialized = self.materialize_static_expression(value);
        let candidate = if static_expression_matches(&materialized, value) {
            value
        } else {
            &materialized
        };
        let Expression::Call { callee, .. } = candidate else {
            return None;
        };
        let Some(LocalFunctionBinding::User(function_name)) =
            self.resolve_function_binding_from_expression(callee)
        else {
            return None;
        };
        self.user_function(&function_name)
            .is_some_and(|function| function.is_generator())
            .then(|| candidate.clone())
    }

    fn collect_hidden_simple_generator_step_binding_names_from_expression(
        &self,
        expression: &Expression,
        generator_values: &HashMap<String, Expression>,
        iterator_sources: &HashMap<String, Expression>,
        names: &mut HashSet<String>,
    ) {
        match expression {
            Expression::Call { callee, arguments } => {
                if let Some(LocalFunctionBinding::User(function_name)) =
                    self.resolve_function_binding_from_expression(callee)
                    && let Some(user_function) = self.user_function(&function_name)
                    && !user_function.is_generator()
                {
                    names.extend(
                        self.collect_user_function_call_effect_nonlocal_bindings(user_function),
                    );
                }
                if arguments.is_empty()
                    && let Expression::Member { object, property } = callee.as_ref()
                    && matches!(property.as_ref(), Expression::String(name) if name == "next")
                    && let Expression::Identifier(iterator_name) = object.as_ref()
                    && let Some(source) = iterator_sources
                        .get(iterator_name)
                        .or_else(|| generator_values.get(iterator_name))
                {
                    self.collect_simple_generator_source_effect_binding_names(source, names);
                }
                self.collect_hidden_simple_generator_step_binding_names_from_expression(
                    callee,
                    generator_values,
                    iterator_sources,
                    names,
                );
                for argument in arguments {
                    self.collect_hidden_simple_generator_step_binding_names_from_expression(
                        argument.expression(),
                        generator_values,
                        iterator_sources,
                        names,
                    );
                }
            }
            Expression::SuperCall { callee, arguments } | Expression::New { callee, arguments } => {
                if let Some(LocalFunctionBinding::User(function_name)) =
                    self.resolve_function_binding_from_expression(callee)
                    && let Some(user_function) = self.user_function(&function_name)
                    && !user_function.is_generator()
                {
                    names.extend(
                        self.collect_user_function_call_effect_nonlocal_bindings(user_function),
                    );
                }
                self.collect_hidden_simple_generator_step_binding_names_from_expression(
                    callee,
                    generator_values,
                    iterator_sources,
                    names,
                );
                for argument in arguments {
                    self.collect_hidden_simple_generator_step_binding_names_from_expression(
                        argument.expression(),
                        generator_values,
                        iterator_sources,
                        names,
                    );
                }
            }
            Expression::Member { object, property } => {
                self.collect_hidden_simple_generator_step_binding_names_from_expression(
                    object,
                    generator_values,
                    iterator_sources,
                    names,
                );
                self.collect_hidden_simple_generator_step_binding_names_from_expression(
                    property,
                    generator_values,
                    iterator_sources,
                    names,
                );
            }
            Expression::SuperMember { property } => {
                self.collect_hidden_simple_generator_step_binding_names_from_expression(
                    property,
                    generator_values,
                    iterator_sources,
                    names,
                );
            }
            Expression::Assign { value, .. }
            | Expression::Await(value)
            | Expression::EnumerateKeys(value)
            | Expression::GetIterator(value)
            | Expression::IteratorClose(value)
            | Expression::Unary {
                expression: value, ..
            } => self.collect_hidden_simple_generator_step_binding_names_from_expression(
                value,
                generator_values,
                iterator_sources,
                names,
            ),
            Expression::AssignMember {
                object,
                property,
                value,
            } => {
                self.collect_hidden_simple_generator_step_binding_names_from_expression(
                    object,
                    generator_values,
                    iterator_sources,
                    names,
                );
                self.collect_hidden_simple_generator_step_binding_names_from_expression(
                    property,
                    generator_values,
                    iterator_sources,
                    names,
                );
                self.collect_hidden_simple_generator_step_binding_names_from_expression(
                    value,
                    generator_values,
                    iterator_sources,
                    names,
                );
            }
            Expression::AssignSuperMember { property, value } => {
                self.collect_hidden_simple_generator_step_binding_names_from_expression(
                    property,
                    generator_values,
                    iterator_sources,
                    names,
                );
                self.collect_hidden_simple_generator_step_binding_names_from_expression(
                    value,
                    generator_values,
                    iterator_sources,
                    names,
                );
            }
            Expression::Binary { left, right, .. } => {
                self.collect_hidden_simple_generator_step_binding_names_from_expression(
                    left,
                    generator_values,
                    iterator_sources,
                    names,
                );
                self.collect_hidden_simple_generator_step_binding_names_from_expression(
                    right,
                    generator_values,
                    iterator_sources,
                    names,
                );
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                self.collect_hidden_simple_generator_step_binding_names_from_expression(
                    condition,
                    generator_values,
                    iterator_sources,
                    names,
                );
                self.collect_hidden_simple_generator_step_binding_names_from_expression(
                    then_expression,
                    generator_values,
                    iterator_sources,
                    names,
                );
                self.collect_hidden_simple_generator_step_binding_names_from_expression(
                    else_expression,
                    generator_values,
                    iterator_sources,
                    names,
                );
            }
            Expression::Sequence(expressions) => {
                for expression in expressions {
                    self.collect_hidden_simple_generator_step_binding_names_from_expression(
                        expression,
                        generator_values,
                        iterator_sources,
                        names,
                    );
                }
            }
            Expression::Array(elements) => {
                for element in elements {
                    match element {
                        ArrayElement::Expression(expression) | ArrayElement::Spread(expression) => {
                            self.collect_hidden_simple_generator_step_binding_names_from_expression(
                                expression,
                                generator_values,
                                iterator_sources,
                                names,
                            );
                        }
                    }
                }
            }
            Expression::Object(entries) => {
                for entry in entries {
                    match entry {
                        ObjectEntry::Data { key, value } => {
                            self.collect_hidden_simple_generator_step_binding_names_from_expression(
                                key,
                                generator_values,
                                iterator_sources,
                                names,
                            );
                            self.collect_hidden_simple_generator_step_binding_names_from_expression(
                                value,
                                generator_values,
                                iterator_sources,
                                names,
                            );
                        }
                        ObjectEntry::Getter { key, getter } => {
                            self.collect_hidden_simple_generator_step_binding_names_from_expression(
                                key,
                                generator_values,
                                iterator_sources,
                                names,
                            );
                            self.collect_hidden_simple_generator_step_binding_names_from_expression(
                                getter,
                                generator_values,
                                iterator_sources,
                                names,
                            );
                        }
                        ObjectEntry::Setter { key, setter } => {
                            self.collect_hidden_simple_generator_step_binding_names_from_expression(
                                key,
                                generator_values,
                                iterator_sources,
                                names,
                            );
                            self.collect_hidden_simple_generator_step_binding_names_from_expression(
                                setter,
                                generator_values,
                                iterator_sources,
                                names,
                            );
                        }
                        ObjectEntry::Spread(expression) => {
                            self.collect_hidden_simple_generator_step_binding_names_from_expression(
                                expression,
                                generator_values,
                                iterator_sources,
                                names,
                            );
                        }
                    }
                }
            }
            Expression::Identifier(_)
            | Expression::Number(_)
            | Expression::BigInt(_)
            | Expression::String(_)
            | Expression::Bool(_)
            | Expression::Null
            | Expression::Undefined
            | Expression::This
            | Expression::NewTarget
            | Expression::Sent
            | Expression::Update { .. } => {}
        }
    }

    fn collect_hidden_simple_generator_step_binding_names_from_statement(
        &self,
        statement: &Statement,
        generator_values: &mut HashMap<String, Expression>,
        iterator_sources: &mut HashMap<String, Expression>,
        names: &mut HashSet<String>,
    ) {
        match statement {
            Statement::Var { name, value }
            | Statement::Let { name, value, .. }
            | Statement::Assign { name, value } => {
                self.collect_hidden_simple_generator_step_binding_names_from_expression(
                    value,
                    generator_values,
                    iterator_sources,
                    names,
                );
                if let Some(source) = self.simple_generator_call_expression_for_value(value) {
                    self.collect_simple_generator_source_effect_binding_names(&source, names);
                    generator_values.insert(name.clone(), source);
                } else if let Expression::GetIterator(source) = value
                    && let Expression::Identifier(source_name) = source.as_ref()
                    && let Some(generator_source) = generator_values.get(source_name).cloned()
                {
                    iterator_sources.insert(name.clone(), generator_source);
                }
            }
            Statement::Expression(value)
            | Statement::Throw(value)
            | Statement::Return(value)
            | Statement::Yield { value }
            | Statement::YieldDelegate { value } => {
                self.collect_hidden_simple_generator_step_binding_names_from_expression(
                    value,
                    generator_values,
                    iterator_sources,
                    names,
                );
            }
            Statement::AssignMember {
                object,
                property,
                value,
            } => {
                self.collect_hidden_simple_generator_step_binding_names_from_expression(
                    object,
                    generator_values,
                    iterator_sources,
                    names,
                );
                self.collect_hidden_simple_generator_step_binding_names_from_expression(
                    property,
                    generator_values,
                    iterator_sources,
                    names,
                );
                self.collect_hidden_simple_generator_step_binding_names_from_expression(
                    value,
                    generator_values,
                    iterator_sources,
                    names,
                );
            }
            Statement::Declaration { body }
            | Statement::Block { body }
            | Statement::Labeled { body, .. } => {
                let mut nested_generator_values = generator_values.clone();
                let mut nested_iterator_sources = iterator_sources.clone();
                self.collect_hidden_simple_generator_step_binding_names_from_statements_with_state(
                    body,
                    &mut nested_generator_values,
                    &mut nested_iterator_sources,
                    names,
                );
            }
            Statement::If {
                condition,
                then_branch,
                else_branch,
            } => {
                self.collect_hidden_simple_generator_step_binding_names_from_expression(
                    condition,
                    generator_values,
                    iterator_sources,
                    names,
                );
                let mut then_generator_values = generator_values.clone();
                let mut then_iterator_sources = iterator_sources.clone();
                self.collect_hidden_simple_generator_step_binding_names_from_statements_with_state(
                    then_branch,
                    &mut then_generator_values,
                    &mut then_iterator_sources,
                    names,
                );
                let mut else_generator_values = generator_values.clone();
                let mut else_iterator_sources = iterator_sources.clone();
                self.collect_hidden_simple_generator_step_binding_names_from_statements_with_state(
                    else_branch,
                    &mut else_generator_values,
                    &mut else_iterator_sources,
                    names,
                );
            }
            Statement::Try {
                body,
                catch_setup,
                catch_body,
                ..
            } => {
                let mut body_generator_values = generator_values.clone();
                let mut body_iterator_sources = iterator_sources.clone();
                self.collect_hidden_simple_generator_step_binding_names_from_statements_with_state(
                    body,
                    &mut body_generator_values,
                    &mut body_iterator_sources,
                    names,
                );
                let mut catch_generator_values = generator_values.clone();
                let mut catch_iterator_sources = iterator_sources.clone();
                self.collect_hidden_simple_generator_step_binding_names_from_statements_with_state(
                    catch_setup,
                    &mut catch_generator_values,
                    &mut catch_iterator_sources,
                    names,
                );
                self.collect_hidden_simple_generator_step_binding_names_from_statements_with_state(
                    catch_body,
                    &mut catch_generator_values,
                    &mut catch_iterator_sources,
                    names,
                );
            }
            Statement::Print { values } => {
                for value in values {
                    self.collect_hidden_simple_generator_step_binding_names_from_expression(
                        value,
                        generator_values,
                        iterator_sources,
                        names,
                    );
                }
            }
            Statement::With { object, body } => {
                self.collect_hidden_simple_generator_step_binding_names_from_expression(
                    object,
                    generator_values,
                    iterator_sources,
                    names,
                );
                let mut nested_generator_values = generator_values.clone();
                let mut nested_iterator_sources = iterator_sources.clone();
                self.collect_hidden_simple_generator_step_binding_names_from_statements_with_state(
                    body,
                    &mut nested_generator_values,
                    &mut nested_iterator_sources,
                    names,
                );
            }
            Statement::Switch {
                discriminant,
                cases,
                ..
            } => {
                self.collect_hidden_simple_generator_step_binding_names_from_expression(
                    discriminant,
                    generator_values,
                    iterator_sources,
                    names,
                );
                for case in cases {
                    let mut case_generator_values = generator_values.clone();
                    let mut case_iterator_sources = iterator_sources.clone();
                    if let Some(test) = &case.test {
                        self.collect_hidden_simple_generator_step_binding_names_from_expression(
                            test,
                            &case_generator_values,
                            &case_iterator_sources,
                            names,
                        );
                    }
                    self.collect_hidden_simple_generator_step_binding_names_from_statements_with_state(
                        &case.body,
                        &mut case_generator_values,
                        &mut case_iterator_sources,
                        names,
                    );
                }
            }
            Statement::For {
                init,
                condition,
                update,
                body,
                ..
            } => {
                let mut loop_generator_values = generator_values.clone();
                let mut loop_iterator_sources = iterator_sources.clone();
                self.collect_hidden_simple_generator_step_binding_names_from_statements_with_state(
                    init,
                    &mut loop_generator_values,
                    &mut loop_iterator_sources,
                    names,
                );
                if let Some(condition) = condition {
                    self.collect_hidden_simple_generator_step_binding_names_from_expression(
                        condition,
                        &loop_generator_values,
                        &loop_iterator_sources,
                        names,
                    );
                }
                if let Some(update) = update {
                    self.collect_hidden_simple_generator_step_binding_names_from_expression(
                        update,
                        &loop_generator_values,
                        &loop_iterator_sources,
                        names,
                    );
                }
                self.collect_hidden_simple_generator_step_binding_names_from_statements_with_state(
                    body,
                    &mut loop_generator_values,
                    &mut loop_iterator_sources,
                    names,
                );
            }
            Statement::While {
                condition, body, ..
            }
            | Statement::DoWhile {
                condition, body, ..
            } => {
                self.collect_hidden_simple_generator_step_binding_names_from_expression(
                    condition,
                    generator_values,
                    iterator_sources,
                    names,
                );
                let mut nested_generator_values = generator_values.clone();
                let mut nested_iterator_sources = iterator_sources.clone();
                self.collect_hidden_simple_generator_step_binding_names_from_statements_with_state(
                    body,
                    &mut nested_generator_values,
                    &mut nested_iterator_sources,
                    names,
                );
            }
            Statement::Break { .. } | Statement::Continue { .. } => {}
        }
    }

    fn collect_hidden_simple_generator_step_binding_names_from_statements_with_state(
        &self,
        statements: &[Statement],
        generator_values: &mut HashMap<String, Expression>,
        iterator_sources: &mut HashMap<String, Expression>,
        names: &mut HashSet<String>,
    ) {
        for statement in statements {
            self.collect_hidden_simple_generator_step_binding_names_from_statement(
                statement,
                generator_values,
                iterator_sources,
                names,
            );
        }
    }

    fn collect_iterator_close_updated_binding_names_from_statement(
        &mut self,
        statement: &Statement,
        names: &mut HashSet<String>,
    ) {
        match statement {
            Statement::Declaration { body }
            | Statement::Block { body }
            | Statement::Labeled { body, .. } => {
                self.collect_iterator_close_updated_binding_names_from_statements(body, names);
            }
            Statement::Var { value, .. }
            | Statement::Let { value, .. }
            | Statement::Assign { value, .. }
            | Statement::Expression(value)
            | Statement::Throw(value)
            | Statement::Return(value)
            | Statement::Yield { value }
            | Statement::YieldDelegate { value } => {
                self.collect_iterator_close_updated_binding_names_from_expression(value, names);
            }
            Statement::AssignMember {
                object,
                property,
                value,
            } => {
                self.collect_iterator_close_updated_binding_names_from_expression(object, names);
                self.collect_iterator_close_updated_binding_names_from_expression(property, names);
                self.collect_iterator_close_updated_binding_names_from_expression(value, names);
            }
            Statement::Print { values } => {
                for value in values {
                    self.collect_iterator_close_updated_binding_names_from_expression(value, names);
                }
            }
            Statement::With { object, body } => {
                self.collect_iterator_close_updated_binding_names_from_expression(object, names);
                self.collect_iterator_close_updated_binding_names_from_statements(body, names);
            }
            Statement::If {
                condition,
                then_branch,
                else_branch,
            } => {
                self.collect_iterator_close_updated_binding_names_from_expression(condition, names);
                self.collect_iterator_close_updated_binding_names_from_statements(
                    then_branch,
                    names,
                );
                self.collect_iterator_close_updated_binding_names_from_statements(
                    else_branch,
                    names,
                );
            }
            Statement::Try {
                body,
                catch_setup,
                catch_body,
                ..
            } => {
                self.collect_iterator_close_updated_binding_names_from_statements(body, names);
                self.collect_iterator_close_updated_binding_names_from_statements(
                    catch_setup,
                    names,
                );
                self.collect_iterator_close_updated_binding_names_from_statements(
                    catch_body, names,
                );
            }
            Statement::Switch {
                discriminant,
                cases,
                ..
            } => {
                self.collect_iterator_close_updated_binding_names_from_expression(
                    discriminant,
                    names,
                );
                for case in cases {
                    if let Some(test) = &case.test {
                        self.collect_iterator_close_updated_binding_names_from_expression(
                            test, names,
                        );
                    }
                    self.collect_iterator_close_updated_binding_names_from_statements(
                        &case.body, names,
                    );
                }
            }
            Statement::For {
                init,
                condition,
                update,
                break_hook,
                body,
                ..
            } => {
                self.collect_iterator_close_updated_binding_names_from_statements(init, names);
                if let Some(condition) = condition {
                    self.collect_iterator_close_updated_binding_names_from_expression(
                        condition, names,
                    );
                }
                if let Some(update) = update {
                    self.collect_iterator_close_updated_binding_names_from_expression(
                        update, names,
                    );
                }
                if let Some(break_hook) = break_hook {
                    self.collect_iterator_close_updated_binding_names_from_expression(
                        break_hook, names,
                    );
                }
                self.collect_iterator_close_updated_binding_names_from_statements(body, names);
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
                self.collect_iterator_close_updated_binding_names_from_expression(condition, names);
                if let Some(break_hook) = break_hook {
                    self.collect_iterator_close_updated_binding_names_from_expression(
                        break_hook, names,
                    );
                }
                self.collect_iterator_close_updated_binding_names_from_statements(body, names);
            }
            Statement::Break { .. } | Statement::Continue { .. } => {}
        }
    }

    pub(in crate::backend::direct_wasm) fn collect_iterator_close_updated_binding_names_from_statements(
        &mut self,
        statements: &[Statement],
        names: &mut HashSet<String>,
    ) {
        let mut generator_values = HashMap::new();
        let mut iterator_sources = HashMap::new();
        self.collect_hidden_simple_generator_step_binding_names_from_statements_with_state(
            statements,
            &mut generator_values,
            &mut iterator_sources,
            names,
        );
        for statement in statements {
            self.collect_iterator_close_updated_binding_names_from_statement(statement, names);
        }
    }

    pub(super) fn emit_structured_statement(&mut self, statement: &Statement) -> DirectResult<()> {
        let trace_static_if = std::env::var_os("AYY_TRACE_STATIC_IF").is_some();
        match statement {
            Statement::Declaration { body } => {
                let global_static_semantics = self.backend.snapshot_global_static_semantics();
                let local_static_metadata = self.state.snapshot_static_binding_metadata();
                self.with_active_eval_lexical_scope(
                    collect_direct_eval_lexical_binding_names(body),
                    |compiler| compiler.emit_statements(body),
                )?;
                self.backend
                    .restore_global_static_semantics(global_static_semantics);
                self.state
                    .restore_static_binding_metadata(local_static_metadata);
                self.sync_static_statement_tracking_effects(statement);
                Ok(())
            }
            Statement::Block { body } => self.emit_statements_in_direct_lexical_scope(body),
            Statement::Labeled { labels, body } => self.with_private_field_initializer_block(
                labels
                    .iter()
                    .any(|label| label == INSTANCE_FIELD_INITIALIZER_LABEL),
                |compiler| {
                    compiler.with_active_eval_lexical_scope(
                        collect_direct_eval_lexical_binding_names(body),
                        |compiler| compiler.emit_labeled_block(labels, body),
                    )
                },
            ),
            Statement::With { object, body } => {
                let object_kind = self.infer_value_kind(object);
                let object_is_statically_nullish = matches!(
                    object,
                    Expression::Null | Expression::Undefined
                ) || matches!(
                    object,
                        Expression::Identifier(name)
                            if name == "undefined" && self.is_unshadowed_builtin_identifier(name)
                );
                self.emit_numeric_expression(object)?;
                if matches!(
                    object_kind,
                    Some(StaticValueKind::Null | StaticValueKind::Undefined)
                ) && object_is_statically_nullish
                {
                    self.state.emission.output.instructions.push(0x1a);
                    self.emit_named_error_throw("TypeError")?;
                    return Ok(());
                } else if matches!(
                    object_kind,
                    Some(StaticValueKind::Null | StaticValueKind::Undefined)
                ) || matches!(object_kind, None | Some(StaticValueKind::Unknown))
                {
                    let object_local = self.allocate_temp_local();
                    self.push_local_set(object_local);

                    self.push_local_get(object_local);
                    self.push_i32_const(JS_NULL_TAG);
                    self.push_binary_op(BinaryOp::Equal)?;
                    self.push_local_get(object_local);
                    self.push_i32_const(JS_UNDEFINED_TAG);
                    self.push_binary_op(BinaryOp::Equal)?;
                    self.state.emission.output.instructions.push(0x72);

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
                } else {
                    self.state.emission.output.instructions.push(0x1a);
                }
                let with_scope = self.canonicalize_with_scope_expression(object);
                self.state.push_with_scope(with_scope);
                let result = self.emit_statements(body);
                self.state.pop_with_scope();
                result
            }
            Statement::If {
                condition,
                then_branch,
                else_branch,
            } => {
                let references_finally =
                    control_if_condition_references_compiler_finally(condition);
                let contains_assignment_or_update =
                    Self::expression_contains_assignment_or_update(condition);
                let calls_user_function = self.static_if_condition_calls_user_function(condition);
                let reads_runtime_nonlocal =
                    self.static_if_condition_reads_runtime_nonlocal_binding(condition);
                let static_condition_value = if references_finally
                    || contains_assignment_or_update
                    || calls_user_function
                    || reads_runtime_nonlocal
                {
                    None
                } else if control_if_condition_references_internal_iterator_temp(condition) {
                    if self.if_condition_depends_on_active_loop_assignment(condition) {
                        None
                    } else {
                        self.resolve_static_iterator_step_condition_value(condition)
                            .or_else(|| self.resolve_static_binding_value_condition(condition))
                    }
                } else {
                    let condition_depends_on_active_loop =
                        self.if_condition_depends_on_active_loop_assignment(condition);
                    let condition_depends_on_active_iterator_loop =
                        self.if_condition_depends_on_active_iterator_loop_assignment(condition);
                    if self.expression_has_dynamic_member_property_access(condition) {
                        if condition_depends_on_active_loop {
                            self.resolve_active_loop_indexed_member_if_condition_value(condition)
                        } else {
                            None
                        }
                    } else if condition_depends_on_active_iterator_loop {
                        self.resolve_static_loop_dependent_if_condition_value(condition)
                    } else if condition_depends_on_active_loop {
                        None
                    } else {
                        self.resolve_static_if_condition_value(condition)
                    }
                };
                if let Some(condition_value) = static_condition_value {
                    if trace_static_if {
                        eprintln!(
                            "static_if:start condition_value={condition_value} statement={statement:?}"
                        );
                    }
                    if !self.static_if_condition_side_effects_can_be_skipped(condition) {
                        self.emit_numeric_expression(condition)?;
                        self.state.emission.output.instructions.push(0x1a);
                    }
                    if condition_value {
                        if trace_static_if {
                            eprintln!("static_if:emit_then:start");
                        }
                        self.emit_statements(then_branch)?;
                        if trace_static_if {
                            eprintln!("static_if:emit_then:done");
                        }
                    } else {
                        if trace_static_if {
                            eprintln!("static_if:emit_else:start");
                        }
                        self.emit_statements(else_branch)?;
                        if trace_static_if {
                            eprintln!("static_if:emit_else:done");
                        }
                    }
                    if trace_static_if {
                        eprintln!("static_if:done");
                    }
                    return Ok(());
                }
                self.emit_truthy_expression(condition)?;
                self.state.emission.output.instructions.push(0x04);
                self.state
                    .emission
                    .output
                    .instructions
                    .push(EMPTY_BLOCK_TYPE);
                self.push_control_frame();
                let mut branch_invalidated_bindings = HashSet::new();
                for statement in then_branch {
                    collect_assigned_binding_names_from_statement(
                        statement,
                        &mut branch_invalidated_bindings,
                    );
                }
                self.collect_iterator_close_updated_binding_names_from_statements(
                    then_branch,
                    &mut branch_invalidated_bindings,
                );
                for statement in else_branch {
                    collect_assigned_binding_names_from_statement(
                        statement,
                        &mut branch_invalidated_bindings,
                    );
                }
                self.collect_iterator_close_updated_binding_names_from_statements(
                    else_branch,
                    &mut branch_invalidated_bindings,
                );
                let base_static_metadata = self.state.snapshot_static_binding_metadata();
                let base_global_static_semantics = self.backend.snapshot_global_static_semantics();
                if let Some((name, narrowed_expression)) =
                    self.conditional_defined_binding_narrowing(condition, true)
                {
                    if trace_static_if {
                        eprintln!("dynamic_if:then:narrowed:start name={name}");
                    }
                    let (then_static_metadata, then_global_static_semantics) = self
                        .with_restored_static_binding_metadata_snapshots(|compiler| {
                            compiler.with_narrowed_local_binding_metadata(
                                &name,
                                &narrowed_expression,
                                |compiler| compiler.emit_statements(then_branch),
                            )
                        })?;
                    if trace_static_if {
                        eprintln!("dynamic_if:then:narrowed:done name={name}");
                    }
                    if !else_branch.is_empty() {
                        self.seed_runtime_array_metadata_for_names_from_snapshot(
                            &then_static_metadata,
                            &branch_invalidated_bindings,
                        );
                    }
                    self.state.emission.output.instructions.push(0x05);
                    let (else_static_metadata, else_global_static_semantics) =
                        if let Some((name, narrowed_expression)) =
                            self.conditional_defined_binding_narrowing(condition, false)
                        {
                            if trace_static_if {
                                eprintln!("dynamic_if:else:narrowed:start name={name}");
                            }
                            let (local_snapshot, global_snapshot) = self
                                .with_restored_static_binding_metadata_snapshots(|compiler| {
                                    compiler.with_narrowed_local_binding_metadata(
                                        &name,
                                        &narrowed_expression,
                                        |compiler| compiler.emit_statements(else_branch),
                                    )
                                })?;
                            if trace_static_if {
                                eprintln!("dynamic_if:else:narrowed:done name={name}");
                            }
                            (Some(local_snapshot), Some(global_snapshot))
                        } else {
                            if trace_static_if {
                                eprintln!("dynamic_if:else:start");
                            }
                            let (local_snapshot, global_snapshot) = self
                                .with_restored_static_binding_metadata_snapshots(|compiler| {
                                    compiler.emit_statements(else_branch)
                                })?;
                            if trace_static_if {
                                eprintln!("dynamic_if:else:done");
                            }
                            (Some(local_snapshot), Some(global_snapshot))
                        };
                    self.state.emission.output.instructions.push(0x0b);
                    self.pop_control_frame();
                    if trace_static_if {
                        eprintln!(
                            "dynamic_if:invalidate:start names={branch_invalidated_bindings:?}"
                        );
                    }
                    self.invalidate_static_binding_metadata_for_names(&branch_invalidated_bindings);
                    self.merge_dynamic_branch_static_binding_metadata(
                        &branch_invalidated_bindings,
                        &base_static_metadata,
                        &then_static_metadata,
                        else_static_metadata.as_ref(),
                    );
                    self.merge_dynamic_branch_global_static_binding_metadata(
                        &branch_invalidated_bindings,
                        &base_global_static_semantics,
                        &then_global_static_semantics,
                        else_global_static_semantics.as_ref(),
                    );
                    let active_with_object = self
                        .state
                        .emission
                        .lexical_scopes
                        .with_scopes
                        .last()
                        .cloned();
                    self.mark_loop_with_scope_shadow_dynamics_from_statements(
                        then_branch,
                        active_with_object.as_ref(),
                    );
                    if !else_branch.is_empty() {
                        self.mark_loop_with_scope_shadow_dynamics_from_statements(
                            else_branch,
                            active_with_object.as_ref(),
                        );
                    }
                    if trace_static_if {
                        eprintln!("dynamic_if:invalidate:done");
                    }
                    return Ok(());
                } else {
                    if trace_static_if {
                        eprintln!("dynamic_if:then:start");
                    }
                    let (then_static_metadata, then_global_static_semantics) = self
                        .with_restored_static_binding_metadata_snapshots(|compiler| {
                            compiler.emit_statements(then_branch)
                        })?;
                    if trace_static_if {
                        eprintln!("dynamic_if:then:done");
                    }
                    if !else_branch.is_empty() {
                        self.seed_runtime_array_metadata_for_names_from_snapshot(
                            &then_static_metadata,
                            &branch_invalidated_bindings,
                        );
                    }
                    if !else_branch.is_empty() {
                        self.state.emission.output.instructions.push(0x05);
                    }
                    let (else_static_metadata, else_global_static_semantics) = if !else_branch
                        .is_empty()
                    {
                        if let Some((name, narrowed_expression)) =
                            self.conditional_defined_binding_narrowing(condition, false)
                        {
                            if trace_static_if {
                                eprintln!("dynamic_if:else:narrowed:start name={name}");
                            }
                            let (local_snapshot, global_snapshot) = self
                                .with_restored_static_binding_metadata_snapshots(|compiler| {
                                    compiler.with_narrowed_local_binding_metadata(
                                        &name,
                                        &narrowed_expression,
                                        |compiler| compiler.emit_statements(else_branch),
                                    )
                                })?;
                            if trace_static_if {
                                eprintln!("dynamic_if:else:narrowed:done name={name}");
                            }
                            (Some(local_snapshot), Some(global_snapshot))
                        } else {
                            if trace_static_if {
                                eprintln!("dynamic_if:else:start");
                            }
                            let (local_snapshot, global_snapshot) = self
                                .with_restored_static_binding_metadata_snapshots(|compiler| {
                                    compiler.emit_statements(else_branch)
                                })?;
                            if trace_static_if {
                                eprintln!("dynamic_if:else:done");
                            }
                            (Some(local_snapshot), Some(global_snapshot))
                        }
                    } else {
                        (None, None)
                    };
                    self.state.emission.output.instructions.push(0x0b);
                    self.pop_control_frame();
                    if trace_static_if {
                        eprintln!(
                            "dynamic_if:invalidate:start names={branch_invalidated_bindings:?}"
                        );
                    }
                    self.invalidate_static_binding_metadata_for_names(&branch_invalidated_bindings);
                    self.merge_dynamic_branch_static_binding_metadata(
                        &branch_invalidated_bindings,
                        &base_static_metadata,
                        &then_static_metadata,
                        else_static_metadata.as_ref(),
                    );
                    self.merge_dynamic_branch_global_static_binding_metadata(
                        &branch_invalidated_bindings,
                        &base_global_static_semantics,
                        &then_global_static_semantics,
                        else_global_static_semantics.as_ref(),
                    );
                    let active_with_object = self
                        .state
                        .emission
                        .lexical_scopes
                        .with_scopes
                        .last()
                        .cloned();
                    self.mark_loop_with_scope_shadow_dynamics_from_statements(
                        then_branch,
                        active_with_object.as_ref(),
                    );
                    if !else_branch.is_empty() {
                        self.mark_loop_with_scope_shadow_dynamics_from_statements(
                            else_branch,
                            active_with_object.as_ref(),
                        );
                    }
                    if trace_static_if {
                        eprintln!("dynamic_if:invalidate:done");
                    }
                    return Ok(());
                }
            }
            Statement::Try {
                body,
                catch_binding,
                catch_setup,
                catch_body,
            } => {
                let mut try_body_assigned_bindings = HashSet::new();
                for statement in body {
                    collect_assigned_binding_names_from_statement(
                        statement,
                        &mut try_body_assigned_bindings,
                    );
                }
                let mut catch_assigned_bindings = HashSet::new();
                for statement in catch_setup {
                    collect_assigned_binding_names_from_statement(
                        statement,
                        &mut catch_assigned_bindings,
                    );
                }
                for statement in catch_body {
                    collect_assigned_binding_names_from_statement(
                        statement,
                        &mut catch_assigned_bindings,
                    );
                }
                if let Some(catch_binding) = catch_binding {
                    catch_assigned_bindings.insert(catch_binding.clone());
                }
                let catch_body_has_deterministic_terminal_throw =
                    self.statements_have_deterministic_terminal_throw(catch_body);
                let try_body_has_deterministic_terminal_throw =
                    !catch_body_has_deterministic_terminal_throw
                        && self.statements_have_deterministic_terminal_throw(body);
                let try_body_metadata_survives_catch = catch_binding.is_some()
                    && (catch_body_has_deterministic_terminal_throw
                        || try_body_has_deterministic_terminal_throw);
                let static_catch_value = catch_binding.as_ref().and_then(|_| {
                    try_body_has_deterministic_terminal_throw
                        .then(|| self.resolve_static_catch_value_from_try_body(body))
                        .flatten()
                });
                let assert_throws_single_call_try = catch_binding.is_none()
                    && catch_setup.is_empty()
                    && matches!(
                        catch_body.as_slice(),
                        [Statement::Assign {
                            value: Expression::Bool(true),
                            ..
                        }]
                    )
                    && matches!(
                        body.as_slice(),
                        [Statement::Expression(Expression::Call { .. })]
                    );
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

                self.with_active_eval_lexical_scope(
                    collect_direct_eval_lexical_binding_names(body),
                    |compiler| compiler.emit_statements(body),
                )?;
                let try_body_local_static_metadata = try_body_metadata_survives_catch
                    .then(|| self.state.snapshot_static_binding_metadata());
                let try_body_global_static_metadata = try_body_metadata_survives_catch
                    .then(|| self.backend.snapshot_global_static_semantics());

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
                        self.update_local_value_binding(catch_binding, static_catch_value);
                        if let Some(object_binding) =
                            self.resolve_object_binding_from_expression(static_catch_value)
                        {
                            let object_binding = self
                                .object_binding_with_constructed_constructor_shadow(
                                    object_binding,
                                    static_catch_value,
                                );
                            self.update_local_object_binding_from_resolved(
                                catch_binding,
                                static_catch_value,
                                object_binding,
                            );
                        }
                        self.state.speculation.static_semantics.set_local_kind(
                            catch_binding,
                            self.infer_value_kind(static_catch_value)
                                .unwrap_or(StaticValueKind::Unknown),
                        );
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

                let mut catch_scope_bindings =
                    collect_direct_eval_lexical_binding_names(catch_setup);
                catch_scope_bindings.extend(collect_direct_eval_lexical_binding_names(catch_body));
                if let Some(catch_binding) = catch_binding {
                    catch_scope_bindings.push(catch_binding.clone());
                }
                self.with_active_eval_lexical_scope(catch_scope_bindings, |compiler| {
                    if !catch_setup.is_empty() {
                        compiler.emit_statements(catch_setup)?;
                    }
                    if !catch_body.is_empty() {
                        compiler.emit_statements(catch_body)?;
                    }
                    Ok(())
                })?;

                self.state.emission.output.instructions.push(0x0b);
                self.pop_control_frame();
                self.invalidate_static_binding_metadata_for_names(&try_body_assigned_bindings);
                let catch_assigned_bindings_to_invalidate = catch_assigned_bindings
                    .iter()
                    .filter(|name| !is_using_completion_binding(name))
                    .cloned()
                    .collect::<HashSet<_>>();
                self.invalidate_static_binding_metadata_for_names(
                    &catch_assigned_bindings_to_invalidate,
                );
                if let (Some(local_snapshot), Some(global_snapshot)) = (
                    try_body_local_static_metadata.as_ref(),
                    try_body_global_static_metadata.as_ref(),
                ) {
                    let preserved_try_body_bindings = try_body_assigned_bindings
                        .difference(&catch_assigned_bindings)
                        .cloned()
                        .collect::<HashSet<_>>();
                    self.restore_try_body_binding_metadata_for_names(
                        &preserved_try_body_bindings,
                        local_snapshot,
                        global_snapshot,
                    );
                }
                if assert_throws_single_call_try {
                    self.sync_assert_throws_iterator_bindings_for_body(body);
                }
                Ok(())
            }
            Statement::Switch {
                labels,
                bindings,
                discriminant,
                cases,
            } => {
                let mut invalidated_bindings = HashSet::new();
                collect_assigned_binding_names_from_expression(
                    discriminant,
                    &mut invalidated_bindings,
                );
                for case in cases {
                    if let Some(test) = &case.test {
                        collect_assigned_binding_names_from_expression(
                            test,
                            &mut invalidated_bindings,
                        );
                    }
                    for statement in &case.body {
                        collect_assigned_binding_names_from_statement(
                            statement,
                            &mut invalidated_bindings,
                        );
                    }
                }
                self.invalidate_static_binding_metadata_for_names(&invalidated_bindings);
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
                let start_case_local = self.allocate_temp_local();
                let active_local = self.allocate_temp_local();

                self.emit_numeric_expression(discriminant)?;
                self.push_local_set(discriminant_local);
                self.push_i32_const(-1);
                self.push_local_set(start_case_local);
                self.push_i32_const(0);
                self.push_local_set(active_local);

                self.with_active_eval_lexical_scope(bindings.to_vec(), |compiler| {
                    if let Some(default_index) = cases.iter().position(|case| case.test.is_none()) {
                        for (case_index, case) in cases.iter().enumerate().take(default_index) {
                            compiler.emit_switch_case_match_probe(
                                case,
                                case_index,
                                start_case_local,
                                discriminant_local,
                            )?;
                        }
                        compiler.emit_switch_post_default_match_scan(
                            &cases[default_index + 1..],
                            default_index + 1,
                            start_case_local,
                            discriminant_local,
                        )?;
                        compiler.emit_switch_default_fallback(default_index, start_case_local)?;
                    } else {
                        for (case_index, case) in cases.iter().enumerate() {
                            compiler.emit_switch_case_match_probe(
                                case,
                                case_index,
                                start_case_local,
                                discriminant_local,
                            )?;
                        }
                    }

                    for (case_index, case) in cases.iter().enumerate() {
                        compiler.emit_switch_case(
                            case,
                            case_index,
                            active_local,
                            start_case_local,
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
            _ => unreachable!("emit_structured_statement called with non-structured statement"),
        }
    }
}
