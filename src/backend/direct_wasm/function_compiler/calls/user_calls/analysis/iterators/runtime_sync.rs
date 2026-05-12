use super::*;

impl<'a> FunctionCompiler<'a> {
    pub(in crate::backend::direct_wasm) fn close_local_iterator_binding(&mut self, name: &str) {
        let Some(mut binding) = self
            .state
            .speculation
            .static_semantics
            .local_array_iterator_binding(name)
            .cloned()
        else {
            return;
        };
        let (closed_state, closed_static_index) = match &binding.source {
            IteratorSourceKind::StaticArray {
                values,
                length_local,
                runtime_name,
                ..
            }
            | IteratorSourceKind::StaticArrayEntries {
                values,
                length_local,
                runtime_name,
            } => {
                let closed_static_index = if length_local.is_none() && runtime_name.is_none() {
                    Some(values.len().saturating_add(1))
                } else {
                    None
                };
                (i32::MAX, closed_static_index)
            }
            IteratorSourceKind::StaticMapEntries {
                values,
                length_local,
                key_runtime_name,
                value_runtime_name,
            } => {
                let closed_static_index = if length_local.is_none()
                    && key_runtime_name.is_none()
                    && value_runtime_name.is_none()
                {
                    Some(values.len().saturating_add(1))
                } else {
                    None
                };
                (i32::MAX, closed_static_index)
            }
            IteratorSourceKind::SimpleGenerator { steps, .. } => {
                let closed_index = steps.len().saturating_add(1);
                (closed_index as i32, Some(closed_index))
            }
            IteratorSourceKind::AsyncYieldDelegateGenerator { .. } => (2, None),
            IteratorSourceKind::TypedArrayView { .. }
            | IteratorSourceKind::DirectArguments { .. } => (i32::MAX, None),
        };
        self.push_i32_const(closed_state);
        self.push_local_set(binding.index_local);
        binding.static_index = closed_static_index;
        self.state
            .speculation
            .static_semantics
            .set_local_array_iterator_binding(name, binding);
    }

    pub(in crate::backend::direct_wasm) fn sync_argument_iterator_bindings_for_user_call(
        &mut self,
        user_function: &UserFunction,
        arguments: &[Expression],
    ) {
        let consumed_indices =
            self.user_function_parameter_iterator_consumption_indices(user_function);
        if consumed_indices.is_empty() {
            return;
        }
        for (index, argument) in arguments.iter().enumerate() {
            if !consumed_indices.contains(&index) {
                continue;
            }
            self.sync_iterator_binding_from_expression(argument);
        }

        for index in consumed_indices {
            let default_used = match arguments.get(index) {
                None => true,
                Some(argument) => {
                    matches!(
                        self.materialize_static_expression(argument),
                        Expression::Undefined
                    )
                }
            };
            if !default_used {
                continue;
            }
            let Some(default_expression) = user_function
                .parameter_defaults
                .get(index)
                .and_then(|default| default.as_ref())
            else {
                continue;
            };
            self.sync_iterator_binding_from_expression(default_expression);
        }
    }

    pub(in crate::backend::direct_wasm) fn sync_consumed_iterator_bindings_for_user_call(
        &mut self,
        user_function: &UserFunction,
    ) {
        let Some(function) = self.resolve_registered_function_declaration(&user_function.name)
        else {
            return;
        };
        let body = function.body.clone();
        let mut iterator_names = HashSet::new();
        Self::collect_consumed_iterator_names_from_statements(&body, &mut iterator_names);
        for iterator_name in iterator_names {
            let source = Self::find_iterator_source_expression_in_statements(&body, &iterator_name)
                .unwrap_or_else(|| Expression::Identifier(iterator_name));
            let source = Self::resolve_iterator_source_alias_in_statements(
                &body,
                source,
                &mut HashSet::new(),
            );
            self.sync_iterator_binding_from_expression(&source);
        }
    }

    fn collect_consumed_iterator_names_from_statements(
        statements: &[Statement],
        names: &mut HashSet<String>,
    ) {
        for statement in statements {
            Self::collect_consumed_iterator_names_from_statement(statement, names);
        }
    }

    fn collect_consumed_iterator_names_from_statement(
        statement: &Statement,
        names: &mut HashSet<String>,
    ) {
        match statement {
            Statement::Expression(expression)
            | Statement::Return(expression)
            | Statement::Throw(expression)
            | Statement::Yield { value: expression }
            | Statement::YieldDelegate { value: expression }
            | Statement::Var {
                value: expression, ..
            }
            | Statement::Let {
                value: expression, ..
            }
            | Statement::Assign {
                value: expression, ..
            } => Self::collect_consumed_iterator_names_from_expression(expression, names),
            Statement::AssignMember {
                object,
                property,
                value,
            } => {
                Self::collect_consumed_iterator_names_from_expression(object, names);
                Self::collect_consumed_iterator_names_from_expression(property, names);
                Self::collect_consumed_iterator_names_from_expression(value, names);
            }
            Statement::Print { values } => {
                for value in values {
                    Self::collect_consumed_iterator_names_from_expression(value, names);
                }
            }
            Statement::Block { body }
            | Statement::Declaration { body }
            | Statement::Labeled { body, .. }
            | Statement::With { body, .. } => {
                Self::collect_consumed_iterator_names_from_statements(body, names);
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
                Self::collect_consumed_iterator_names_from_expression(condition, names);
                if let Some(break_hook) = break_hook {
                    Self::collect_consumed_iterator_names_from_expression(break_hook, names);
                }
                Self::collect_consumed_iterator_names_from_statements(body, names);
            }
            Statement::If {
                condition,
                then_branch,
                else_branch,
            } => {
                Self::collect_consumed_iterator_names_from_expression(condition, names);
                Self::collect_consumed_iterator_names_from_statements(then_branch, names);
                Self::collect_consumed_iterator_names_from_statements(else_branch, names);
            }
            Statement::Switch {
                discriminant,
                cases,
                ..
            } => {
                Self::collect_consumed_iterator_names_from_expression(discriminant, names);
                for case in cases {
                    if let Some(test) = &case.test {
                        Self::collect_consumed_iterator_names_from_expression(test, names);
                    }
                    Self::collect_consumed_iterator_names_from_statements(&case.body, names);
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
                Self::collect_consumed_iterator_names_from_statements(init, names);
                if let Some(condition) = condition {
                    Self::collect_consumed_iterator_names_from_expression(condition, names);
                }
                if let Some(update) = update {
                    Self::collect_consumed_iterator_names_from_expression(update, names);
                }
                if let Some(break_hook) = break_hook {
                    Self::collect_consumed_iterator_names_from_expression(break_hook, names);
                }
                Self::collect_consumed_iterator_names_from_statements(body, names);
            }
            Statement::Try {
                body,
                catch_setup,
                catch_body,
                ..
            } => {
                Self::collect_consumed_iterator_names_from_statements(body, names);
                Self::collect_consumed_iterator_names_from_statements(catch_setup, names);
                Self::collect_consumed_iterator_names_from_statements(catch_body, names);
            }
            Statement::Break { .. } | Statement::Continue { .. } => {}
        }
    }

    fn collect_consumed_iterator_names_from_expression(
        expression: &Expression,
        names: &mut HashSet<String>,
    ) {
        if let Expression::Call { callee, arguments } = expression
            && arguments.is_empty()
            && let Expression::Member { object, property } = callee.as_ref()
            && matches!(property.as_ref(), Expression::String(name) if matches!(name.as_str(), "next" | "return" | "throw"))
            && let Expression::Identifier(name) = object.as_ref()
        {
            names.insert(name.clone());
        }
        match expression {
            Expression::Member { object, property } => {
                Self::collect_consumed_iterator_names_from_expression(object, names);
                Self::collect_consumed_iterator_names_from_expression(property, names);
            }
            Expression::SuperMember { property } => {
                Self::collect_consumed_iterator_names_from_expression(property, names);
            }
            Expression::Assign { value, .. }
            | Expression::Await(value)
            | Expression::EnumerateKeys(value)
            | Expression::GetIterator(value)
            | Expression::IteratorClose(value)
            | Expression::Unary {
                expression: value, ..
            } => Self::collect_consumed_iterator_names_from_expression(value, names),
            Expression::AssignMember {
                object,
                property,
                value,
            } => {
                Self::collect_consumed_iterator_names_from_expression(object, names);
                Self::collect_consumed_iterator_names_from_expression(property, names);
                Self::collect_consumed_iterator_names_from_expression(value, names);
            }
            Expression::AssignSuperMember { property, value } => {
                Self::collect_consumed_iterator_names_from_expression(property, names);
                Self::collect_consumed_iterator_names_from_expression(value, names);
            }
            Expression::Binary { left, right, .. } => {
                Self::collect_consumed_iterator_names_from_expression(left, names);
                Self::collect_consumed_iterator_names_from_expression(right, names);
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                Self::collect_consumed_iterator_names_from_expression(condition, names);
                Self::collect_consumed_iterator_names_from_expression(then_expression, names);
                Self::collect_consumed_iterator_names_from_expression(else_expression, names);
            }
            Expression::Sequence(expressions) => {
                for expression in expressions {
                    Self::collect_consumed_iterator_names_from_expression(expression, names);
                }
            }
            Expression::Call { callee, arguments }
            | Expression::SuperCall { callee, arguments }
            | Expression::New { callee, arguments } => {
                Self::collect_consumed_iterator_names_from_expression(callee, names);
                for argument in arguments {
                    match argument {
                        CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                            Self::collect_consumed_iterator_names_from_expression(
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
                            Self::collect_consumed_iterator_names_from_expression(
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
                            Self::collect_consumed_iterator_names_from_expression(key, names);
                            Self::collect_consumed_iterator_names_from_expression(value, names);
                        }
                        ObjectEntry::Getter { key, getter } => {
                            Self::collect_consumed_iterator_names_from_expression(key, names);
                            Self::collect_consumed_iterator_names_from_expression(getter, names);
                        }
                        ObjectEntry::Setter { key, setter } => {
                            Self::collect_consumed_iterator_names_from_expression(key, names);
                            Self::collect_consumed_iterator_names_from_expression(setter, names);
                        }
                        ObjectEntry::Spread(expression) => {
                            Self::collect_consumed_iterator_names_from_expression(
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

    fn resolve_iterator_source_alias_in_statements(
        statements: &[Statement],
        expression: Expression,
        seen: &mut HashSet<String>,
    ) -> Expression {
        match expression {
            Expression::Identifier(name) => {
                if !seen.insert(name.clone()) {
                    return Expression::Identifier(name);
                }
                let Some(value) =
                    Self::find_binding_source_expression_in_statements(statements, &name)
                else {
                    return Expression::Identifier(name);
                };
                Self::resolve_iterator_source_alias_in_statements(statements, value, seen)
            }
            Expression::GetIterator(iterated) => {
                Self::resolve_iterator_source_alias_in_statements(statements, *iterated, seen)
            }
            Expression::Member { object, property } if matches!(property.as_ref(), Expression::String(name) if name == "value") =>
            {
                if let Some(value) = Self::resolve_iterator_step_value_source_in_statements(
                    statements, &object, seen,
                ) {
                    return Self::resolve_iterator_source_alias_in_statements(
                        statements, value, seen,
                    );
                }
                Expression::Member { object, property }
            }
            other => other,
        }
    }

    fn resolve_iterator_step_value_source_in_statements(
        statements: &[Statement],
        step_object: &Expression,
        seen: &mut HashSet<String>,
    ) -> Option<Expression> {
        let Expression::Identifier(step_name) = step_object else {
            return None;
        };
        let Expression::Call { callee, arguments } =
            Self::find_binding_source_expression_in_statements(statements, step_name)?
        else {
            return None;
        };
        if !arguments.is_empty() {
            return None;
        }
        let Expression::Member { object, property } = callee.as_ref() else {
            return None;
        };
        if !matches!(property.as_ref(), Expression::String(name) if name == "next") {
            return None;
        }
        let iterator_source = Self::resolve_iterator_source_alias_in_statements(
            statements,
            object.as_ref().clone(),
            seen,
        );
        Self::single_static_iterator_value_expression(iterator_source)
    }

    fn single_static_iterator_value_expression(expression: Expression) -> Option<Expression> {
        let Expression::Array(elements) = expression else {
            return None;
        };
        let mut values = elements.into_iter().filter_map(|element| match element {
            ArrayElement::Expression(expression) => Some(expression),
            ArrayElement::Spread(_) => None,
        });
        let value = values.next()?;
        values.next().is_none().then_some(value)
    }

    fn find_binding_source_expression_in_statements(
        statements: &[Statement],
        binding_name: &str,
    ) -> Option<Expression> {
        for statement in statements {
            match statement {
                Statement::Var { name, value }
                | Statement::Let { name, value, .. }
                | Statement::Assign { name, value }
                    if name == binding_name =>
                {
                    return Some(value.clone());
                }
                Statement::Declaration { body }
                | Statement::Block { body }
                | Statement::Labeled { body, .. }
                | Statement::With { body, .. } => {
                    if let Some(value) =
                        Self::find_binding_source_expression_in_statements(body, binding_name)
                    {
                        return Some(value);
                    }
                }
                Statement::If {
                    then_branch,
                    else_branch,
                    ..
                } => {
                    if let Some(value) = Self::find_binding_source_expression_in_statements(
                        then_branch,
                        binding_name,
                    ) {
                        return Some(value);
                    }
                    if let Some(value) = Self::find_binding_source_expression_in_statements(
                        else_branch,
                        binding_name,
                    ) {
                        return Some(value);
                    }
                }
                Statement::Try {
                    body,
                    catch_binding,
                    catch_setup,
                    catch_body,
                } => {
                    if catch_binding.as_deref() == Some(binding_name)
                        && let Some(Statement::Throw(value)) = body
                            .iter()
                            .find(|statement| matches!(statement, Statement::Throw(_)))
                    {
                        return Some(value.clone());
                    }
                    if let Some(value) =
                        Self::find_binding_source_expression_in_statements(body, binding_name)
                    {
                        return Some(value);
                    }
                    if let Some(value) = Self::find_binding_source_expression_in_statements(
                        catch_setup,
                        binding_name,
                    ) {
                        return Some(value);
                    }
                    if let Some(value) =
                        Self::find_binding_source_expression_in_statements(catch_body, binding_name)
                    {
                        return Some(value);
                    }
                }
                Statement::Switch { cases, .. } => {
                    for case in cases {
                        if let Some(value) = Self::find_binding_source_expression_in_statements(
                            &case.body,
                            binding_name,
                        ) {
                            return Some(value);
                        }
                    }
                }
                Statement::For { init, body, .. } => {
                    if let Some(value) =
                        Self::find_binding_source_expression_in_statements(init, binding_name)
                    {
                        return Some(value);
                    }
                    if let Some(value) =
                        Self::find_binding_source_expression_in_statements(body, binding_name)
                    {
                        return Some(value);
                    }
                }
                Statement::While { body, .. } | Statement::DoWhile { body, .. } => {
                    if let Some(value) =
                        Self::find_binding_source_expression_in_statements(body, binding_name)
                    {
                        return Some(value);
                    }
                }
                _ => {}
            }
        }
        None
    }

    fn sync_iterator_binding_from_expression(&mut self, expression: &Expression) {
        let Some(name) = self.iterator_binding_source_name_from_expression(expression) else {
            return;
        };
        let Some(binding_name) = self.resolve_local_array_iterator_binding_name(&name) else {
            return;
        };
        self.close_local_iterator_binding(&binding_name);
    }

    fn iterator_binding_source_name_from_expression(
        &self,
        expression: &Expression,
    ) -> Option<String> {
        match expression {
            Expression::Identifier(name) => Some(name.clone()),
            _ => {
                if let Some(Expression::Identifier(name)) =
                    self.resolve_bound_alias_expression(expression)
                {
                    return Some(name);
                }
                let materialized = self.materialize_static_expression(expression);
                if !static_expression_matches(&materialized, expression) {
                    return self.iterator_binding_source_name_from_expression(&materialized);
                }
                None
            }
        }
    }
}
