use super::*;

impl<'a> FunctionCompiler<'a> {
    pub(in crate::backend::direct_wasm) fn active_loop_numeric_binding_candidates(
        &self,
        name: &str,
    ) -> Option<Vec<i64>> {
        let source_name = scoped_binding_source_name(name).unwrap_or(name);
        self.state
            .emission
            .control_flow
            .loop_stack
            .iter()
            .rev()
            .find_map(|loop_context| {
                loop_context
                    .numeric_binding_candidates
                    .get(name)
                    .or_else(|| {
                        (source_name != name)
                            .then(|| loop_context.numeric_binding_candidates.get(source_name))
                            .flatten()
                    })
                    .cloned()
            })
    }

    fn integer_number_literal(expression: &Expression) -> Option<i64> {
        let Expression::Number(value) = expression else {
            return None;
        };
        (value.is_finite() && value.fract() == 0.0).then_some(*value as i64)
    }

    fn loop_numeric_bound_candidates(&self, expression: &Expression) -> Option<Vec<i64>> {
        match expression {
            Expression::Number(_) => {
                Self::integer_number_literal(expression).map(|value| vec![value])
            }
            Expression::Identifier(name) => self
                .active_loop_numeric_binding_candidates(name)
                .or_else(|| {
                    self.global_value_binding(name)
                        .and_then(Self::integer_number_literal)
                        .map(|value| vec![value])
                })
                .or_else(|| {
                    self.state
                        .speculation
                        .static_semantics
                        .local_value_binding(name)
                        .and_then(Self::integer_number_literal)
                        .map(|value| vec![value])
                }),
            _ => None,
        }
    }

    fn numeric_loop_initializer(init: &[Statement]) -> Option<(String, i64)> {
        init.iter().find_map(|statement| match statement {
            Statement::Var { name, value }
            | Statement::Let { name, value, .. }
            | Statement::Assign { name, value } => {
                Self::integer_number_literal(value).map(|start| (name.clone(), start))
            }
            _ => None,
        })
    }

    fn update_increments_binding(update: Option<&Expression>, name: &str) -> bool {
        match update {
            Some(Expression::Update {
                name: update_name,
                op: UpdateOp::Increment,
                ..
            }) if update_name == name => true,
            Some(Expression::Assign {
                name: update_name,
                value,
            }) if update_name == name => {
                matches!(
                    value.as_ref(),
                    Expression::Binary {
                        op: BinaryOp::Add,
                        left,
                        right,
                    } if matches!(left.as_ref(), Expression::Identifier(left_name) if left_name == name)
                        && matches!(right.as_ref(), Expression::Number(value) if *value == 1.0)
                )
            }
            _ => false,
        }
    }

    fn restorable_user_function_assignment_value(&self, value: &Expression) -> Option<String> {
        match value {
            Expression::Identifier(name)
                if is_internal_user_function_identifier(name)
                    && self.contains_user_function(name) =>
            {
                Some(name.clone())
            }
            Expression::Assign { value, .. } => {
                self.restorable_user_function_assignment_value(value)
            }
            Expression::Sequence(expressions) => expressions
                .last()
                .and_then(|value| self.restorable_user_function_assignment_value(value)),
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                let branch = if self.resolve_loop_expression_truthy(condition)? {
                    then_expression
                } else {
                    else_expression
                };
                self.restorable_user_function_assignment_value(branch)
            }
            Expression::Binary { op, left, right }
                if matches!(
                    op,
                    BinaryOp::LogicalAnd | BinaryOp::LogicalOr | BinaryOp::NullishCoalescing
                ) =>
            {
                self.resolve_static_logical_result_expression(*op, left, right)
                    .and_then(|value| self.restorable_user_function_assignment_value(&value))
            }
            _ => None,
        }
    }

    fn resolve_loop_expression_truthy(&self, expression: &Expression) -> Option<bool> {
        match expression {
            Expression::Sequence(expressions) => expressions
                .last()
                .and_then(|value| self.resolve_loop_expression_truthy(value)),
            Expression::Assign { value, .. } => self.resolve_loop_expression_truthy(value),
            Expression::Binary {
                op: BinaryOp::LogicalAnd,
                left,
                right,
            } => {
                if self.resolve_loop_expression_truthy(left)? {
                    self.resolve_loop_expression_truthy(right)
                } else {
                    Some(false)
                }
            }
            Expression::Binary {
                op: BinaryOp::LogicalOr,
                left,
                right,
            } => {
                if self.resolve_loop_expression_truthy(left)? {
                    Some(true)
                } else {
                    self.resolve_loop_expression_truthy(right)
                }
            }
            Expression::Binary {
                op: BinaryOp::NullishCoalescing,
                left,
                right,
            } => {
                let left_value = self.materialize_static_expression(left);
                if matches!(left_value, Expression::Null | Expression::Undefined) {
                    self.resolve_loop_expression_truthy(right)
                } else {
                    self.resolve_loop_expression_truthy(&left_value)
                }
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                let branch = if self.resolve_loop_expression_truthy(condition)? {
                    then_expression
                } else {
                    else_expression
                };
                self.resolve_loop_expression_truthy(branch)
            }
            _ => self.resolve_static_boolean_expression(expression),
        }
    }

    fn merge_loop_function_assignment_candidate(
        &self,
        invalidated_bindings: &HashSet<String>,
        candidates: &mut HashMap<String, Option<String>>,
        name: &str,
        value: &Expression,
    ) {
        if !invalidated_bindings.contains(name) {
            return;
        }
        let candidate = self.restorable_user_function_assignment_value(value);
        match candidates.get_mut(name) {
            Some(existing) if *existing != candidate => {
                *existing = None;
            }
            Some(_) => {}
            None => {
                candidates.insert(name.to_string(), candidate);
            }
        }
    }

    fn collect_executed_loop_function_assignments_from_expression(
        &self,
        invalidated_bindings: &HashSet<String>,
        candidates: &mut HashMap<String, Option<String>>,
        expression: &Expression,
    ) {
        match expression {
            Expression::Identifier(_)
            | Expression::Number(_)
            | Expression::BigInt(_)
            | Expression::String(_)
            | Expression::Bool(_)
            | Expression::Null
            | Expression::Undefined
            | Expression::This
            | Expression::NewTarget
            | Expression::Sent => {}
            Expression::Update { name, .. } => {
                if invalidated_bindings.contains(name) {
                    candidates.insert(name.clone(), None);
                }
            }
            Expression::Member { object, property } => {
                self.collect_executed_loop_function_assignments_from_expression(
                    invalidated_bindings,
                    candidates,
                    object,
                );
                self.collect_executed_loop_function_assignments_from_expression(
                    invalidated_bindings,
                    candidates,
                    property,
                );
            }
            Expression::SuperMember { property } => {
                self.collect_executed_loop_function_assignments_from_expression(
                    invalidated_bindings,
                    candidates,
                    property,
                );
            }
            Expression::Assign { name, value } => {
                self.merge_loop_function_assignment_candidate(
                    invalidated_bindings,
                    candidates,
                    name,
                    value,
                );
                self.collect_executed_loop_function_assignments_from_expression(
                    invalidated_bindings,
                    candidates,
                    value,
                );
            }
            Expression::AssignMember {
                object,
                property,
                value,
            } => {
                self.collect_executed_loop_function_assignments_from_expression(
                    invalidated_bindings,
                    candidates,
                    object,
                );
                self.collect_executed_loop_function_assignments_from_expression(
                    invalidated_bindings,
                    candidates,
                    property,
                );
                self.collect_executed_loop_function_assignments_from_expression(
                    invalidated_bindings,
                    candidates,
                    value,
                );
            }
            Expression::AssignSuperMember { property, value } => {
                self.collect_executed_loop_function_assignments_from_expression(
                    invalidated_bindings,
                    candidates,
                    property,
                );
                self.collect_executed_loop_function_assignments_from_expression(
                    invalidated_bindings,
                    candidates,
                    value,
                );
            }
            Expression::Await(value)
            | Expression::EnumerateKeys(value)
            | Expression::GetIterator(value)
            | Expression::IteratorClose(value)
            | Expression::Unary {
                expression: value, ..
            } => self.collect_executed_loop_function_assignments_from_expression(
                invalidated_bindings,
                candidates,
                value,
            ),
            Expression::Binary { op, left, right } => {
                self.collect_executed_loop_function_assignments_from_expression(
                    invalidated_bindings,
                    candidates,
                    left,
                );
                match op {
                    BinaryOp::LogicalAnd => {
                        if self.resolve_loop_expression_truthy(left) == Some(true) {
                            self.collect_executed_loop_function_assignments_from_expression(
                                invalidated_bindings,
                                candidates,
                                right,
                            );
                        }
                    }
                    BinaryOp::LogicalOr => {
                        if self.resolve_loop_expression_truthy(left) == Some(false) {
                            self.collect_executed_loop_function_assignments_from_expression(
                                invalidated_bindings,
                                candidates,
                                right,
                            );
                        }
                    }
                    BinaryOp::NullishCoalescing => {
                        let left_value = self.materialize_static_expression(left);
                        if matches!(left_value, Expression::Null | Expression::Undefined) {
                            self.collect_executed_loop_function_assignments_from_expression(
                                invalidated_bindings,
                                candidates,
                                right,
                            );
                        }
                    }
                    _ => self.collect_executed_loop_function_assignments_from_expression(
                        invalidated_bindings,
                        candidates,
                        right,
                    ),
                }
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                self.collect_executed_loop_function_assignments_from_expression(
                    invalidated_bindings,
                    candidates,
                    condition,
                );
                if let Some(condition_value) = self.resolve_loop_expression_truthy(condition) {
                    let branch = if condition_value {
                        then_expression
                    } else {
                        else_expression
                    };
                    self.collect_executed_loop_function_assignments_from_expression(
                        invalidated_bindings,
                        candidates,
                        branch,
                    );
                }
            }
            Expression::Sequence(expressions) => {
                for expression in expressions {
                    self.collect_executed_loop_function_assignments_from_expression(
                        invalidated_bindings,
                        candidates,
                        expression,
                    );
                }
            }
            Expression::Call { callee, arguments }
            | Expression::SuperCall { callee, arguments }
            | Expression::New { callee, arguments } => {
                self.collect_executed_loop_function_assignments_from_expression(
                    invalidated_bindings,
                    candidates,
                    callee,
                );
                for argument in arguments {
                    match argument {
                        CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                            self.collect_executed_loop_function_assignments_from_expression(
                                invalidated_bindings,
                                candidates,
                                expression,
                            );
                        }
                    }
                }
            }
            Expression::Array(elements) => {
                for element in elements {
                    match element {
                        ArrayElement::Expression(expression) | ArrayElement::Spread(expression) => {
                            self.collect_executed_loop_function_assignments_from_expression(
                                invalidated_bindings,
                                candidates,
                                expression,
                            );
                        }
                    }
                }
            }
            Expression::Object(entries) => {
                for entry in entries {
                    match entry {
                        ObjectEntry::Data { key, value } => {
                            self.collect_executed_loop_function_assignments_from_expression(
                                invalidated_bindings,
                                candidates,
                                key,
                            );
                            self.collect_executed_loop_function_assignments_from_expression(
                                invalidated_bindings,
                                candidates,
                                value,
                            );
                        }
                        ObjectEntry::Getter { key, getter } => {
                            self.collect_executed_loop_function_assignments_from_expression(
                                invalidated_bindings,
                                candidates,
                                key,
                            );
                            self.collect_executed_loop_function_assignments_from_expression(
                                invalidated_bindings,
                                candidates,
                                getter,
                            );
                        }
                        ObjectEntry::Setter { key, setter } => {
                            self.collect_executed_loop_function_assignments_from_expression(
                                invalidated_bindings,
                                candidates,
                                key,
                            );
                            self.collect_executed_loop_function_assignments_from_expression(
                                invalidated_bindings,
                                candidates,
                                setter,
                            );
                        }
                        ObjectEntry::Spread(expression) => {
                            self.collect_executed_loop_function_assignments_from_expression(
                                invalidated_bindings,
                                candidates,
                                expression,
                            );
                        }
                    }
                }
            }
        }
    }

    fn statement_may_prevent_for_update(statement: &Statement) -> bool {
        match statement {
            Statement::Break { .. }
            | Statement::Continue { .. }
            | Statement::Return(_)
            | Statement::Throw(_) => true,
            Statement::Declaration { body }
            | Statement::Block { body }
            | Statement::Labeled { body, .. }
            | Statement::With { body, .. } => {
                body.iter().any(Self::statement_may_prevent_for_update)
            }
            Statement::If {
                then_branch,
                else_branch,
                ..
            } => {
                then_branch
                    .iter()
                    .any(Self::statement_may_prevent_for_update)
                    || else_branch
                        .iter()
                        .any(Self::statement_may_prevent_for_update)
            }
            Statement::Try {
                body,
                catch_setup,
                catch_body,
                ..
            } => {
                body.iter().any(Self::statement_may_prevent_for_update)
                    || catch_setup
                        .iter()
                        .any(Self::statement_may_prevent_for_update)
                    || catch_body
                        .iter()
                        .any(Self::statement_may_prevent_for_update)
            }
            Statement::Switch { cases, .. } => cases
                .iter()
                .any(|case| case.body.iter().any(Self::statement_may_prevent_for_update)),
            Statement::For { .. } | Statement::While { .. } | Statement::DoWhile { .. } => true,
            _ => false,
        }
    }

    fn statement_may_prevent_terminal_break_prefix(statement: &Statement) -> bool {
        match statement {
            Statement::Break { .. }
            | Statement::Continue { .. }
            | Statement::Return(_)
            | Statement::Throw(_) => true,
            Statement::Declaration { body }
            | Statement::Block { body }
            | Statement::Labeled { body, .. }
            | Statement::With { body, .. } => body
                .iter()
                .any(Self::statement_may_prevent_terminal_break_prefix),
            Statement::If { .. }
            | Statement::Try { .. }
            | Statement::Switch { .. }
            | Statement::For { .. }
            | Statement::While { .. }
            | Statement::DoWhile { .. } => true,
            _ => false,
        }
    }

    fn loop_body_has_deterministic_terminal_unlabeled_break(body: &[Statement]) -> bool {
        let Some((terminal, prefix)) = body.split_last() else {
            return false;
        };
        Self::statement_has_deterministic_terminal_unlabeled_break(terminal)
            && !prefix
                .iter()
                .any(Self::statement_may_prevent_terminal_break_prefix)
    }

    fn statement_has_deterministic_terminal_unlabeled_break(statement: &Statement) -> bool {
        match statement {
            Statement::Break { label: None } => true,
            Statement::Declaration { body }
            | Statement::Block { body }
            | Statement::Labeled { body, .. } => {
                Self::loop_body_has_deterministic_terminal_unlabeled_break(body)
            }
            _ => false,
        }
    }

    fn collect_executed_loop_function_assignments_from_statement(
        &self,
        invalidated_bindings: &HashSet<String>,
        candidates: &mut HashMap<String, Option<String>>,
        statement: &Statement,
    ) {
        match statement {
            Statement::Declaration { body }
            | Statement::Block { body }
            | Statement::Labeled { body, .. }
            | Statement::With { body, .. } => {
                for statement in body {
                    self.collect_executed_loop_function_assignments_from_statement(
                        invalidated_bindings,
                        candidates,
                        statement,
                    );
                }
            }
            Statement::Var { name, value } | Statement::Let { name, value, .. } => {
                self.merge_loop_function_assignment_candidate(
                    invalidated_bindings,
                    candidates,
                    name,
                    value,
                );
                self.collect_executed_loop_function_assignments_from_expression(
                    invalidated_bindings,
                    candidates,
                    value,
                );
            }
            Statement::Assign { name, value } => {
                self.merge_loop_function_assignment_candidate(
                    invalidated_bindings,
                    candidates,
                    name,
                    value,
                );
                self.collect_executed_loop_function_assignments_from_expression(
                    invalidated_bindings,
                    candidates,
                    value,
                );
            }
            Statement::AssignMember {
                object,
                property,
                value,
            } => {
                self.collect_executed_loop_function_assignments_from_expression(
                    invalidated_bindings,
                    candidates,
                    object,
                );
                self.collect_executed_loop_function_assignments_from_expression(
                    invalidated_bindings,
                    candidates,
                    property,
                );
                self.collect_executed_loop_function_assignments_from_expression(
                    invalidated_bindings,
                    candidates,
                    value,
                );
            }
            Statement::Expression(expression)
            | Statement::Throw(expression)
            | Statement::Return(expression)
            | Statement::Yield { value: expression }
            | Statement::YieldDelegate { value: expression } => {
                self.collect_executed_loop_function_assignments_from_expression(
                    invalidated_bindings,
                    candidates,
                    expression,
                );
            }
            Statement::Print { values } => {
                for value in values {
                    self.collect_executed_loop_function_assignments_from_expression(
                        invalidated_bindings,
                        candidates,
                        value,
                    );
                }
            }
            Statement::If {
                condition,
                then_branch,
                else_branch,
            } => {
                self.collect_executed_loop_function_assignments_from_expression(
                    invalidated_bindings,
                    candidates,
                    condition,
                );
                if let Some(condition_value) = self.resolve_loop_expression_truthy(condition) {
                    let branch = if condition_value {
                        then_branch
                    } else {
                        else_branch
                    };
                    for statement in branch {
                        self.collect_executed_loop_function_assignments_from_statement(
                            invalidated_bindings,
                            candidates,
                            statement,
                        );
                    }
                }
            }
            Statement::Try { body, .. } => {
                for statement in body {
                    self.collect_executed_loop_function_assignments_from_statement(
                        invalidated_bindings,
                        candidates,
                        statement,
                    );
                }
            }
            Statement::Switch { discriminant, .. } => {
                self.collect_executed_loop_function_assignments_from_expression(
                    invalidated_bindings,
                    candidates,
                    discriminant,
                );
            }
            Statement::For { init, .. } => {
                for statement in init {
                    self.collect_executed_loop_function_assignments_from_statement(
                        invalidated_bindings,
                        candidates,
                        statement,
                    );
                }
            }
            Statement::While { condition, .. } | Statement::DoWhile { condition, .. } => {
                self.collect_executed_loop_function_assignments_from_expression(
                    invalidated_bindings,
                    candidates,
                    condition,
                );
            }
            Statement::Break { .. } | Statement::Continue { .. } => {}
        }
    }

    fn restorable_loop_function_assignments(
        &self,
        invalidated_bindings: &HashSet<String>,
        condition: Option<&Expression>,
        update: Option<&Expression>,
        body: &[Statement],
    ) -> HashMap<String, String> {
        let mut candidates = HashMap::new();
        if let Some(condition) = condition {
            self.collect_executed_loop_function_assignments_from_expression(
                invalidated_bindings,
                &mut candidates,
                condition,
            );
        }

        let body_and_update_are_guaranteed = condition
            .map(|condition| self.resolve_loop_expression_truthy(condition) == Some(true))
            .unwrap_or(true)
            && !body.iter().any(Self::statement_may_prevent_for_update);
        if body_and_update_are_guaranteed {
            for statement in body {
                self.collect_executed_loop_function_assignments_from_statement(
                    invalidated_bindings,
                    &mut candidates,
                    statement,
                );
            }
            if let Some(update) = update {
                self.collect_executed_loop_function_assignments_from_expression(
                    invalidated_bindings,
                    &mut candidates,
                    update,
                );
            }
        }

        candidates
            .into_iter()
            .filter_map(|(name, function_name)| {
                function_name.map(|function_name| (name, function_name))
            })
            .collect()
    }

    fn restore_loop_function_assignment_metadata(&mut self, assignments: &HashMap<String, String>) {
        for (name, function_name) in assignments {
            let value = Expression::Identifier(function_name.clone());
            if let Some((resolved_name, _)) = self.resolve_current_local_binding(name) {
                self.state
                    .speculation
                    .static_semantics
                    .set_local_static_binding(
                        &resolved_name,
                        value,
                        None,
                        None,
                        Some(StaticValueKind::Function),
                    );
                self.state
                    .speculation
                    .static_semantics
                    .set_local_function_binding(
                        &resolved_name,
                        LocalFunctionBinding::User(function_name.clone()),
                    );
            } else if self.state.runtime.locals.bindings.contains_key(name)
                || self.parameter_scope_arguments_local_for(name).is_some()
            {
                self.state
                    .speculation
                    .static_semantics
                    .set_local_static_binding(
                        name,
                        value,
                        None,
                        None,
                        Some(StaticValueKind::Function),
                    );
                self.state
                    .speculation
                    .static_semantics
                    .set_local_function_binding(
                        name,
                        LocalFunctionBinding::User(function_name.clone()),
                    );
            } else if let Some(hidden_name) = self.resolve_user_function_capture_hidden_name(name) {
                self.update_static_global_assignment_metadata(&hidden_name, &value);
            } else if self.binding_name_is_global(name) || self.backend.global_has_binding(name) {
                self.update_static_global_assignment_metadata(name, &value);
            } else {
                self.state
                    .speculation
                    .static_semantics
                    .set_local_static_binding(
                        name,
                        value,
                        None,
                        None,
                        Some(StaticValueKind::Function),
                    );
                self.state
                    .speculation
                    .static_semantics
                    .set_local_function_binding(
                        name,
                        LocalFunctionBinding::User(function_name.clone()),
                    );
            }
        }
    }

    fn numeric_loop_binding_candidates(
        &self,
        init: &[Statement],
        condition: Option<&Expression>,
        update: Option<&Expression>,
    ) -> HashMap<String, Vec<i64>> {
        let mut candidates = HashMap::new();
        let Some((name, start)) = Self::numeric_loop_initializer(init) else {
            return candidates;
        };
        if !Self::update_increments_binding(update, &name) {
            return candidates;
        }
        let Some(Expression::Binary { op, left, right }) = condition else {
            return candidates;
        };
        if !matches!(left.as_ref(), Expression::Identifier(left_name) if left_name == &name) {
            return candidates;
        }
        let Some(bound_candidates) = self.loop_numeric_bound_candidates(right) else {
            return candidates;
        };
        let Some(bound) = bound_candidates.into_iter().max() else {
            return candidates;
        };
        let end = match op {
            BinaryOp::LessThanOrEqual => bound,
            BinaryOp::LessThan => bound.saturating_sub(1),
            _ => return candidates,
        };
        if end < start || end - start > 128 {
            return candidates;
        }
        candidates.insert(name, (start..=end).collect());
        candidates
    }

    fn numeric_loop_spec(
        &self,
        init: &[Statement],
        condition: Option<&Expression>,
        update: Option<&Expression>,
    ) -> Option<NumericLoopSpec> {
        let (name, start) = Self::numeric_loop_initializer(init)?;
        if !Self::update_increments_binding(update, &name) {
            return None;
        }
        let Expression::Binary { op, left, right } = condition? else {
            return None;
        };
        if !matches!(left.as_ref(), Expression::Identifier(left_name) if left_name == &name) {
            return None;
        }
        let inclusive = match op {
            BinaryOp::LessThanOrEqual => true,
            BinaryOp::LessThan => false,
            _ => return None,
        };
        Some(NumericLoopSpec {
            binding: name,
            start,
            bound: right.as_ref().clone(),
            inclusive,
        })
    }

    fn direct_loop_step_iterators(body: &[Statement]) -> std::collections::HashSet<String> {
        let mut iterators = std::collections::HashSet::new();
        for statement in body {
            let expression = match statement {
                Statement::Let { value, .. }
                | Statement::Var { value, .. }
                | Statement::Assign { value, .. }
                | Statement::Expression(value) => value,
                _ => continue,
            };
            let Expression::Call { callee, arguments } = expression else {
                continue;
            };
            if !arguments.is_empty() {
                continue;
            }
            let Expression::Member { object, property } = callee.as_ref() else {
                continue;
            };
            if !matches!(property.as_ref(), Expression::String(name) if name == "next") {
                continue;
            }
            if let Expression::Identifier(iterator_name) = object.as_ref() {
                iterators.insert(iterator_name.clone());
            }
        }
        iterators
    }

    fn seed_loop_assigned_runtime_array_state(
        &mut self,
        names: &HashSet<String>,
    ) -> DirectResult<()> {
        for name in names {
            let expression = Expression::Identifier(name.clone());
            let Some(array_binding) = self.resolve_array_binding_from_expression(&expression)
            else {
                continue;
            };

            if self.emit_sync_global_runtime_array_state_from_binding(name, &array_binding)? {
                continue;
            }

            let length_local = self.ensure_runtime_array_length_local(name);
            self.push_i32_const(array_binding.values.len() as i32);
            self.push_local_set(length_local);
            self.ensure_runtime_array_slots_for_binding(name, &array_binding);
        }
        Ok(())
    }

    pub(in crate::backend::direct_wasm) fn mark_runtime_with_scope_shadow_dynamic(
        &mut self,
        object: &Expression,
        name: &str,
    ) {
        if !self.scope_object_has_binding_property(object, name) {
            return;
        }
        let property = Expression::String(name.to_string());
        let _ = self.resolve_runtime_object_property_shadow_binding(object, &property);
        let Some(shadow_binding_name) =
            self.runtime_object_property_shadow_binding_name_for_expression(object, &property)
        else {
            return;
        };
        let dynamic_value = Expression::Member {
            object: Box::new(object.clone()),
            property: Box::new(property),
        };
        self.backend
            .global_semantics
            .values
            .set_value_binding(shadow_binding_name.clone(), dynamic_value.clone());
        self.backend
            .shared_global_semantics
            .values
            .set_value_binding(shadow_binding_name, dynamic_value);
    }

    pub(in crate::backend::direct_wasm) fn mark_loop_with_scope_shadow_dynamics_from_expression(
        &mut self,
        expression: &Expression,
        active_with_object: Option<&Expression>,
    ) {
        if let Some(object) = active_with_object {
            match expression {
                Expression::Assign { name, .. } | Expression::Update { name, .. } => {
                    self.mark_runtime_with_scope_shadow_dynamic(object, name);
                }
                _ => {}
            }
        }
        match expression {
            Expression::Assign { value, .. }
            | Expression::AssignSuperMember { value, .. }
            | Expression::Await(value)
            | Expression::EnumerateKeys(value)
            | Expression::GetIterator(value)
            | Expression::IteratorClose(value)
            | Expression::Unary {
                expression: value, ..
            } => {
                self.mark_loop_with_scope_shadow_dynamics_from_expression(value, active_with_object)
            }
            Expression::Member { object, property }
            | Expression::AssignMember {
                object, property, ..
            } => {
                self.mark_loop_with_scope_shadow_dynamics_from_expression(
                    object,
                    active_with_object,
                );
                self.mark_loop_with_scope_shadow_dynamics_from_expression(
                    property,
                    active_with_object,
                );
                if let Expression::AssignMember { value, .. } = expression {
                    self.mark_loop_with_scope_shadow_dynamics_from_expression(
                        value,
                        active_with_object,
                    );
                }
            }
            Expression::SuperMember { property } => self
                .mark_loop_with_scope_shadow_dynamics_from_expression(property, active_with_object),
            Expression::Binary { left, right, .. } => {
                self.mark_loop_with_scope_shadow_dynamics_from_expression(left, active_with_object);
                self.mark_loop_with_scope_shadow_dynamics_from_expression(
                    right,
                    active_with_object,
                );
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                self.mark_loop_with_scope_shadow_dynamics_from_expression(
                    condition,
                    active_with_object,
                );
                self.mark_loop_with_scope_shadow_dynamics_from_expression(
                    then_expression,
                    active_with_object,
                );
                self.mark_loop_with_scope_shadow_dynamics_from_expression(
                    else_expression,
                    active_with_object,
                );
            }
            Expression::Sequence(expressions) => {
                for expression in expressions {
                    self.mark_loop_with_scope_shadow_dynamics_from_expression(
                        expression,
                        active_with_object,
                    );
                }
            }
            Expression::Call { callee, arguments }
            | Expression::SuperCall { callee, arguments }
            | Expression::New { callee, arguments } => {
                self.mark_loop_with_scope_shadow_dynamics_from_expression(
                    callee,
                    active_with_object,
                );
                for argument in arguments {
                    self.mark_loop_with_scope_shadow_dynamics_from_expression(
                        argument.expression(),
                        active_with_object,
                    );
                }
            }
            Expression::Object(entries) => {
                for entry in entries {
                    match entry {
                        ObjectEntry::Data { key, value } => {
                            self.mark_loop_with_scope_shadow_dynamics_from_expression(
                                key,
                                active_with_object,
                            );
                            self.mark_loop_with_scope_shadow_dynamics_from_expression(
                                value,
                                active_with_object,
                            );
                        }
                        ObjectEntry::Getter { key, getter }
                        | ObjectEntry::Setter {
                            key,
                            setter: getter,
                        } => {
                            self.mark_loop_with_scope_shadow_dynamics_from_expression(
                                key,
                                active_with_object,
                            );
                            self.mark_loop_with_scope_shadow_dynamics_from_expression(
                                getter,
                                active_with_object,
                            );
                        }
                        ObjectEntry::Spread(value) => self
                            .mark_loop_with_scope_shadow_dynamics_from_expression(
                                value,
                                active_with_object,
                            ),
                    }
                }
            }
            Expression::Array(elements) => {
                for element in elements {
                    match element {
                        ArrayElement::Expression(value) | ArrayElement::Spread(value) => self
                            .mark_loop_with_scope_shadow_dynamics_from_expression(
                                value,
                                active_with_object,
                            ),
                    }
                }
            }
            _ => {}
        }
    }

    pub(in crate::backend::direct_wasm) fn mark_loop_with_scope_shadow_dynamics_from_statement(
        &mut self,
        statement: &Statement,
        active_with_object: Option<&Expression>,
    ) {
        if let Some(object) = active_with_object {
            match statement {
                Statement::Assign { name, .. } | Statement::Var { name, .. } => {
                    self.mark_runtime_with_scope_shadow_dynamic(object, name);
                }
                _ => {}
            }
        }
        match statement {
            Statement::Declaration { body }
            | Statement::Block { body }
            | Statement::Labeled { body, .. } => {
                self.mark_loop_with_scope_shadow_dynamics_from_statements(body, active_with_object);
            }
            Statement::With { object, body } => {
                self.mark_loop_with_scope_shadow_dynamics_from_statements(body, Some(object));
            }
            Statement::If {
                condition,
                then_branch,
                else_branch,
            } => {
                self.mark_loop_with_scope_shadow_dynamics_from_expression(
                    condition,
                    active_with_object,
                );
                self.mark_loop_with_scope_shadow_dynamics_from_statements(
                    then_branch,
                    active_with_object,
                );
                self.mark_loop_with_scope_shadow_dynamics_from_statements(
                    else_branch,
                    active_with_object,
                );
            }
            Statement::Try {
                body,
                catch_setup,
                catch_body,
                ..
            } => {
                self.mark_loop_with_scope_shadow_dynamics_from_statements(body, active_with_object);
                self.mark_loop_with_scope_shadow_dynamics_from_statements(
                    catch_setup,
                    active_with_object,
                );
                self.mark_loop_with_scope_shadow_dynamics_from_statements(
                    catch_body,
                    active_with_object,
                );
            }
            Statement::Switch {
                discriminant,
                cases,
                ..
            } => {
                self.mark_loop_with_scope_shadow_dynamics_from_expression(
                    discriminant,
                    active_with_object,
                );
                for case in cases {
                    if let Some(test) = &case.test {
                        self.mark_loop_with_scope_shadow_dynamics_from_expression(
                            test,
                            active_with_object,
                        );
                    }
                    self.mark_loop_with_scope_shadow_dynamics_from_statements(
                        &case.body,
                        active_with_object,
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
                self.mark_loop_with_scope_shadow_dynamics_from_statements(init, active_with_object);
                if let Some(condition) = condition {
                    self.mark_loop_with_scope_shadow_dynamics_from_expression(
                        condition,
                        active_with_object,
                    );
                }
                if let Some(update) = update {
                    self.mark_loop_with_scope_shadow_dynamics_from_expression(
                        update,
                        active_with_object,
                    );
                }
                if let Some(break_hook) = break_hook {
                    self.mark_loop_with_scope_shadow_dynamics_from_expression(
                        break_hook,
                        active_with_object,
                    );
                }
                self.mark_loop_with_scope_shadow_dynamics_from_statements(body, active_with_object);
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
                self.mark_loop_with_scope_shadow_dynamics_from_expression(
                    condition,
                    active_with_object,
                );
                if let Some(break_hook) = break_hook {
                    self.mark_loop_with_scope_shadow_dynamics_from_expression(
                        break_hook,
                        active_with_object,
                    );
                }
                self.mark_loop_with_scope_shadow_dynamics_from_statements(body, active_with_object);
            }
            Statement::Expression(expression)
            | Statement::Throw(expression)
            | Statement::Return(expression)
            | Statement::Yield { value: expression }
            | Statement::YieldDelegate { value: expression } => {
                self.mark_loop_with_scope_shadow_dynamics_from_expression(
                    expression,
                    active_with_object,
                );
            }
            Statement::Assign { value, .. }
            | Statement::Var { value, .. }
            | Statement::Let { value, .. } => {
                self.mark_loop_with_scope_shadow_dynamics_from_expression(
                    value,
                    active_with_object,
                );
            }
            Statement::AssignMember {
                object,
                property,
                value,
            } => {
                self.mark_loop_with_scope_shadow_dynamics_from_expression(
                    object,
                    active_with_object,
                );
                self.mark_loop_with_scope_shadow_dynamics_from_expression(
                    property,
                    active_with_object,
                );
                self.mark_loop_with_scope_shadow_dynamics_from_expression(
                    value,
                    active_with_object,
                );
            }
            Statement::Print { values } => {
                for value in values {
                    self.mark_loop_with_scope_shadow_dynamics_from_expression(
                        value,
                        active_with_object,
                    );
                }
            }
            Statement::Break { .. } | Statement::Continue { .. } => {}
        }
    }

    pub(in crate::backend::direct_wasm) fn mark_loop_with_scope_shadow_dynamics_from_statements(
        &mut self,
        statements: &[Statement],
        active_with_object: Option<&Expression>,
    ) {
        for statement in statements {
            self.mark_loop_with_scope_shadow_dynamics_from_statement(statement, active_with_object);
        }
    }

    pub(in crate::backend::direct_wasm) fn emit_while(
        &mut self,
        condition: &Expression,
        break_hook: Option<&Expression>,
        labels: &[String],
        body: &[Statement],
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
        let restores_terminal_break_metadata = self.resolve_loop_expression_truthy(condition)
            == Some(true)
            && Self::loop_body_has_deterministic_terminal_unlabeled_break(body);
        let numeric_binding_candidates = HashMap::new();
        let numeric_spec = None;
        self.seed_loop_assigned_runtime_array_state(&invalidated_bindings)?;
        self.invalidate_static_binding_metadata_for_names_with_preserved_kinds(
            &invalidated_bindings,
            &preserved_kinds,
        );
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
                direct_step_iterators: Self::direct_loop_step_iterators(body),
                numeric_binding_candidates,
                numeric_spec,
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
        self.emit_statements(body)?;
        let terminal_break_local_static_metadata =
            restores_terminal_break_metadata.then(|| self.state.snapshot_static_binding_metadata());
        let terminal_break_global_static_semantics = restores_terminal_break_metadata
            .then(|| self.backend.snapshot_global_static_semantics());
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
        let active_with_object = self
            .state
            .emission
            .lexical_scopes
            .with_scopes
            .last()
            .cloned();
        self.mark_loop_with_scope_shadow_dynamics_from_statements(
            body,
            active_with_object.as_ref(),
        );
        if let (Some(local_snapshot), Some(global_snapshot)) = (
            terminal_break_local_static_metadata,
            terminal_break_global_static_semantics,
        ) {
            self.state.restore_static_binding_metadata(local_snapshot);
            self.backend
                .restore_global_static_semantics(global_snapshot);
        }
        Ok(())
    }

    pub(in crate::backend::direct_wasm) fn emit_do_while(
        &mut self,
        condition: &Expression,
        break_hook: Option<&Expression>,
        labels: &[String],
        body: &[Statement],
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
        let restores_single_iteration_metadata =
            Self::loop_body_has_deterministic_terminal_unlabeled_break(body)
                || (self.resolve_loop_expression_truthy(condition) == Some(false)
                    && !body.iter().any(Self::statement_may_prevent_for_update));
        let numeric_binding_candidates = HashMap::new();
        let numeric_spec = None;
        self.seed_loop_assigned_runtime_array_state(&invalidated_bindings)?;
        self.invalidate_static_binding_metadata_for_names_with_preserved_kinds(
            &invalidated_bindings,
            &preserved_kinds,
        );
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
                direct_step_iterators: Self::direct_loop_step_iterators(body),
                numeric_binding_candidates,
                numeric_spec,
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

        self.emit_statements(body)?;
        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();

        self.emit_truthy_expression(condition)?;
        let single_iteration_local_static_metadata = restores_single_iteration_metadata
            .then(|| self.state.snapshot_static_binding_metadata());
        let single_iteration_global_static_semantics = restores_single_iteration_metadata
            .then(|| self.backend.snapshot_global_static_semantics());
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
        let active_with_object = self
            .state
            .emission
            .lexical_scopes
            .with_scopes
            .last()
            .cloned();
        self.mark_loop_with_scope_shadow_dynamics_from_statements(
            body,
            active_with_object.as_ref(),
        );
        if let (Some(local_snapshot), Some(global_snapshot)) = (
            single_iteration_local_static_metadata,
            single_iteration_global_static_semantics,
        ) {
            self.state.restore_static_binding_metadata(local_snapshot);
            self.backend
                .restore_global_static_semantics(global_snapshot);
        }
        Ok(())
    }

    pub(in crate::backend::direct_wasm) fn emit_for(
        &mut self,
        labels: &[String],
        init: &[Statement],
        per_iteration_bindings: &[String],
        condition: Option<&Expression>,
        update: Option<&Expression>,
        break_hook: Option<&Expression>,
        body: &[Statement],
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
        if std::env::var_os("AYY_TRACE_LOOP_INVALIDATION").is_some() {
            eprintln!("loop_invalidated for={invalidated_bindings:?}");
        }
        self.with_active_eval_lexical_scope(per_iteration_bindings.to_vec(), |compiler| {
            compiler.emit_statements(init)?;
            let preserved_kinds = compiler.preserved_binding_kinds_for_loop(
                &invalidated_bindings,
                condition.unwrap_or(&fallback_condition),
                break_hook,
                body,
                update,
            );
            let restorable_function_assignments = compiler.restorable_loop_function_assignments(
                &invalidated_bindings,
                condition,
                update,
                body,
            );
            let numeric_binding_candidates =
                compiler.numeric_loop_binding_candidates(init, condition, update);
            let numeric_spec = compiler.numeric_loop_spec(init, condition, update);
            compiler.seed_loop_assigned_runtime_array_state(&invalidated_bindings)?;
            compiler.invalidate_static_binding_metadata_for_names_with_preserved_kinds(
                &invalidated_bindings,
                &preserved_kinds,
            );

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
                    direct_step_iterators: Self::direct_loop_step_iterators(body),
                    numeric_binding_candidates,
                    numeric_spec,
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

            compiler.emit_statements(body)?;
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
            let active_with_object = compiler
                .state
                .emission
                .lexical_scopes
                .with_scopes
                .last()
                .cloned();
            compiler.mark_loop_with_scope_shadow_dynamics_from_statements(
                body,
                active_with_object.as_ref(),
            );
            compiler.restore_loop_function_assignment_metadata(&restorable_function_assignments);
            Ok(())
        })
    }

    pub(in crate::backend::direct_wasm) fn emit_switch_case_match_probe(
        &mut self,
        case: &crate::ir::hir::SwitchCase,
        case_index: usize,
        start_case_local: u32,
        discriminant_local: u32,
    ) -> DirectResult<()> {
        let Some(test) = &case.test else {
            return Ok(());
        };

        self.push_local_get(start_case_local);
        self.push_i32_const(-1);
        self.state.emission.output.instructions.push(0x46);
        self.state.emission.output.instructions.push(0x04);
        self.state
            .emission
            .output
            .instructions
            .push(EMPTY_BLOCK_TYPE);
        self.push_control_frame();

        self.push_local_get(discriminant_local);
        self.emit_numeric_expression(test)?;
        self.state.emission.output.instructions.push(0x46);
        self.state.emission.output.instructions.push(0x04);
        self.state
            .emission
            .output
            .instructions
            .push(EMPTY_BLOCK_TYPE);
        self.push_control_frame();

        self.push_i32_const(case_index as i32);
        self.push_local_set(start_case_local);

        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();
        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();
        Ok(())
    }

    pub(in crate::backend::direct_wasm) fn emit_switch_post_default_match_scan(
        &mut self,
        cases: &[crate::ir::hir::SwitchCase],
        first_case_index: usize,
        start_case_local: u32,
        discriminant_local: u32,
    ) -> DirectResult<()> {
        self.push_local_get(start_case_local);
        self.push_i32_const(-1);
        self.state.emission.output.instructions.push(0x46);
        self.state.emission.output.instructions.push(0x04);
        self.state
            .emission
            .output
            .instructions
            .push(EMPTY_BLOCK_TYPE);
        self.push_control_frame();

        for (offset, case) in cases.iter().enumerate() {
            self.emit_switch_case_match_probe(
                case,
                first_case_index + offset,
                start_case_local,
                discriminant_local,
            )?;
        }

        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();
        Ok(())
    }

    pub(in crate::backend::direct_wasm) fn emit_switch_default_fallback(
        &mut self,
        default_index: usize,
        start_case_local: u32,
    ) -> DirectResult<()> {
        self.push_local_get(start_case_local);
        self.push_i32_const(-1);
        self.state.emission.output.instructions.push(0x46);
        self.state.emission.output.instructions.push(0x04);
        self.state
            .emission
            .output
            .instructions
            .push(EMPTY_BLOCK_TYPE);
        self.push_control_frame();

        self.push_i32_const(default_index as i32);
        self.push_local_set(start_case_local);

        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();
        Ok(())
    }

    pub(in crate::backend::direct_wasm) fn emit_switch_case(
        &mut self,
        case: &crate::ir::hir::SwitchCase,
        case_index: usize,
        active_local: u32,
        start_case_local: u32,
    ) -> DirectResult<()> {
        self.push_local_get(active_local);
        self.push_i32_const(1);
        self.state.emission.output.instructions.push(0x46);
        self.push_local_get(start_case_local);
        self.push_i32_const(case_index as i32);
        self.state.emission.output.instructions.push(0x46);
        self.state.emission.output.instructions.push(0x72);

        self.state.emission.output.instructions.push(0x04);
        self.state
            .emission
            .output
            .instructions
            .push(EMPTY_BLOCK_TYPE);
        self.push_control_frame();

        self.push_i32_const(1);
        self.push_local_set(active_local);
        for case_statement in &case.body {
            self.emit_statement(case_statement)?;
        }

        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();
        Ok(())
    }
}
