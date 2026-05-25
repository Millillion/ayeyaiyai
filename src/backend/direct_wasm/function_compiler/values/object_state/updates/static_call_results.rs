use super::*;

#[path = "static_call_results/binding_results.rs"]
mod binding_results;
#[path = "static_call_results/member_builtins.rs"]
mod member_builtins;
#[path = "static_call_results/specialized_results.rs"]
mod specialized_results;

thread_local! {
    static ACTIVE_STATIC_CALL_RESULT_SHAPES: std::cell::RefCell<HashSet<String>> =
        std::cell::RefCell::new(HashSet::new());
}

struct StaticCallResultResolutionShapeGuard {
    key: String,
}

impl StaticCallResultResolutionShapeGuard {
    fn enter(
        callee: &Expression,
        arguments: &[CallArgument],
        current_function_name: Option<&str>,
    ) -> Option<Self> {
        let key = format!("{current_function_name:?}:{callee:?}:{arguments:?}");
        let inserted =
            ACTIVE_STATIC_CALL_RESULT_SHAPES.with(|active| active.borrow_mut().insert(key.clone()));
        inserted.then_some(Self { key })
    }
}

impl Drop for StaticCallResultResolutionShapeGuard {
    fn drop(&mut self) {
        ACTIVE_STATIC_CALL_RESULT_SHAPES.with(|active| {
            active.borrow_mut().remove(&self.key);
        });
    }
}

impl<'a> FunctionCompiler<'a> {
    fn snapshot_live_capture_source_expression(&self, source_name: &str) -> Option<Expression> {
        let identifier = Expression::Identifier(source_name.to_string());
        if let Some(value) = self
            .state
            .speculation
            .static_semantics
            .local_value_binding(source_name)
            .cloned()
            .or_else(|| self.global_value_binding(source_name).cloned())
        {
            if !static_expression_matches(&value, &identifier) {
                return Some(self.materialize_static_expression(&value));
            }
        }
        if let Some(array_binding) = self.resolve_array_binding_from_expression(&identifier) {
            return Some(Expression::Array(
                array_binding
                    .values
                    .iter()
                    .map(|value| {
                        ArrayElement::Expression(value.clone().unwrap_or(Expression::Undefined))
                    })
                    .collect(),
            ));
        }
        if let Some(object_binding) = self.resolve_object_binding_from_expression(&identifier) {
            if object_binding.property_descriptors.is_empty() {
                return Some(object_binding_to_expression(&object_binding));
            }
            return Some(identifier);
        }
        self.resolve_bound_alias_expression(&identifier)
            .filter(|value| !static_expression_matches(value, &identifier))
            .map(|value| self.materialize_static_expression(&value))
    }

    pub(in crate::backend::direct_wasm) fn resolve_static_call_result_expression(
        &self,
        callee: &Expression,
        arguments: &[CallArgument],
    ) -> Option<Expression> {
        self.resolve_static_call_result_expression_with_context(
            callee,
            arguments,
            self.current_function_name(),
        )
        .map(|(value, _)| value)
    }

    pub(in crate::backend::direct_wasm) fn resolve_static_call_result_expression_with_context(
        &self,
        callee: &Expression,
        arguments: &[CallArgument],
        current_function_name: Option<&str>,
    ) -> Option<(Expression, Option<String>)> {
        let _guard =
            StaticCallResultResolutionShapeGuard::enter(callee, arguments, current_function_name)?;
        if let Some(result) = self.resolve_static_member_builtin_call_result_with_context(
            callee,
            arguments,
            current_function_name,
        ) {
            return Some(result);
        }
        if self
            .resolve_function_expression_capture_slots(callee)
            .is_some_and(|capture_slots| capture_slots.contains_key("new.target"))
        {
            return None;
        }
        if let Some(result) = self.resolve_static_captured_user_function_call_result_with_context(
            callee,
            arguments,
            current_function_name,
        ) {
            return Some(result);
        }
        if self
            .resolve_function_expression_capture_slots(callee)
            .is_some()
        {
            return None;
        }
        if self.static_call_targets_self_recursive_user_function(callee, current_function_name) {
            return None;
        }
        self.resolve_specialized_static_call_result_with_context(callee, arguments)
            .or_else(|| {
                self.resolve_static_binding_call_result_with_context(
                    callee,
                    arguments,
                    current_function_name,
                )
            })
    }

    fn resolve_static_captured_user_function_call_result_with_context(
        &self,
        callee: &Expression,
        arguments: &[CallArgument],
        current_function_name: Option<&str>,
    ) -> Option<(Expression, Option<String>)> {
        let LocalFunctionBinding::User(function_name) = self
            .resolve_function_binding_from_expression_with_context(callee, current_function_name)?
        else {
            return None;
        };
        let user_function = self.user_function(&function_name)?;
        if self.user_function_mentions_private_member_access(user_function)
            || self.user_function_mentions_direct_eval(user_function)
            || user_function.has_lowered_pattern_parameters()
            || !self
                .user_function_parameter_iterator_consumption_indices(user_function)
                .is_empty()
        {
            return None;
        }
        let capture_bindings = self
            .user_function_capture_bindings(&function_name)
            .filter(|captures| !captures.is_empty())?;
        let capture_source_bindings = self
            .resolve_function_expression_capture_slots(callee)
            .map(|capture_slots| {
                capture_slots
                    .into_iter()
                    .map(|(capture_name, slot_name)| {
                        let slot_expression = Expression::Identifier(slot_name.clone());
                        let source_expression = if capture_name == slot_name {
                            self.snapshot_live_capture_source_expression(&slot_name)
                                .unwrap_or(slot_expression)
                        } else if self
                            .resolve_function_binding_from_expression(&slot_expression)
                            .is_some()
                        {
                            slot_expression
                        } else {
                            self.resolve_capture_slot_static_source_expression(&slot_name)
                                .unwrap_or_else(|| {
                                    self.snapshot_bound_capture_slot_expression(&slot_name)
                                })
                        };
                        (capture_name, source_expression)
                    })
                    .collect::<HashMap<_, _>>()
            })
            .or_else(|| self.resolve_constructor_capture_source_bindings_from_expression(callee));
        let capture_source_bindings = capture_source_bindings.filter(|sources| {
            capture_bindings.keys().any(|capture_name| {
                sources.get(capture_name).is_some_and(|source| {
                    !matches!(source, Expression::Identifier(name) if name == capture_name)
                })
            })
        });
        if let Some(capture_source_bindings) = capture_source_bindings.as_ref()
            && !capture_bindings
                .keys()
                .all(|capture_name| capture_source_bindings.contains_key(capture_name))
        {
            return None;
        }
        let value = self.resolve_static_return_expression_from_user_function_call(
            &function_name,
            arguments,
            capture_source_bindings.as_ref(),
        )?;
        let value = self
            .resolve_static_primitive_expression_with_context(
                &value,
                Some(function_name.as_str()).or(current_function_name),
            )
            .unwrap_or(value);
        Some((value, Some(function_name)))
    }

    fn static_call_targets_self_recursive_user_function(
        &self,
        callee: &Expression,
        current_function_name: Option<&str>,
    ) -> bool {
        let Some(LocalFunctionBinding::User(function_name)) = self
            .resolve_function_binding_from_expression_with_context(callee, current_function_name)
        else {
            return false;
        };
        self.user_function_contains_self_callee_reference(&function_name)
    }

    pub(in crate::backend::direct_wasm) fn user_function_contains_self_callee_reference(
        &self,
        function_name: &str,
    ) -> bool {
        let Some(function) = self.resolve_registered_function_declaration(function_name) else {
            return false;
        };
        let mut callee_names = HashSet::new();
        callee_names.insert(function.name.clone());
        if let Some(self_binding) = &function.self_binding {
            callee_names.insert(self_binding.clone());
        }
        if let Some(top_level_binding) = &function.top_level_binding {
            callee_names.insert(top_level_binding.clone());
        }
        if let Some(global_binding) = self
            .backend
            .find_global_user_function_binding_name(function_name)
        {
            callee_names.insert(global_binding);
        }
        function
            .body
            .iter()
            .any(|statement| statement_contains_callee_name(statement, &callee_names))
    }
}

fn identifier_matches_callee_name(name: &str, callee_names: &HashSet<String>) -> bool {
    callee_names.contains(name)
        || scoped_binding_source_name(name).is_some_and(|source_name| {
            callee_names
                .iter()
                .any(|callee_name| callee_name.as_str() == source_name)
        })
}

fn expression_contains_callee_name(
    expression: &Expression,
    callee_names: &HashSet<String>,
) -> bool {
    match expression {
        Expression::Call { callee, arguments }
        | Expression::SuperCall { callee, arguments }
        | Expression::New { callee, arguments } => {
            matches!(
                callee.as_ref(),
                Expression::Identifier(name) if identifier_matches_callee_name(name, callee_names)
            ) || matches!(
                callee.as_ref(),
                Expression::Member { object, property }
                    if matches!(
                        object.as_ref(),
                        Expression::Identifier(name)
                            if identifier_matches_callee_name(name, callee_names)
                    ) && matches!(
                        property.as_ref(),
                        Expression::String(name) if name == "call" || name == "apply"
                    )
            ) || expression_contains_callee_name(callee, callee_names)
                || arguments.iter().any(|argument| {
                    expression_contains_callee_name(argument.expression(), callee_names)
                })
        }
        Expression::Member { object, property } => {
            expression_contains_callee_name(object, callee_names)
                || expression_contains_callee_name(property, callee_names)
        }
        Expression::SuperMember { property } => {
            expression_contains_callee_name(property, callee_names)
        }
        Expression::Assign { value, .. }
        | Expression::Await(value)
        | Expression::EnumerateKeys(value)
        | Expression::GetIterator(value)
        | Expression::IteratorClose(value)
        | Expression::Unary {
            expression: value, ..
        } => expression_contains_callee_name(value, callee_names),
        Expression::AssignMember {
            object,
            property,
            value,
        } => {
            expression_contains_callee_name(object, callee_names)
                || expression_contains_callee_name(property, callee_names)
                || expression_contains_callee_name(value, callee_names)
        }
        Expression::AssignSuperMember { property, value } => {
            expression_contains_callee_name(property, callee_names)
                || expression_contains_callee_name(value, callee_names)
        }
        Expression::Binary { left, right, .. } => {
            expression_contains_callee_name(left, callee_names)
                || expression_contains_callee_name(right, callee_names)
        }
        Expression::Conditional {
            condition,
            then_expression,
            else_expression,
        } => {
            expression_contains_callee_name(condition, callee_names)
                || expression_contains_callee_name(then_expression, callee_names)
                || expression_contains_callee_name(else_expression, callee_names)
        }
        Expression::Sequence(expressions) => expressions
            .iter()
            .any(|expression| expression_contains_callee_name(expression, callee_names)),
        Expression::Array(elements) => elements.iter().any(|element| match element {
            ArrayElement::Expression(expression) | ArrayElement::Spread(expression) => {
                expression_contains_callee_name(expression, callee_names)
            }
        }),
        Expression::Object(entries) => entries.iter().any(|entry| match entry {
            ObjectEntry::Data { key, value } => {
                expression_contains_callee_name(key, callee_names)
                    || expression_contains_callee_name(value, callee_names)
            }
            ObjectEntry::Getter { key, getter } => {
                expression_contains_callee_name(key, callee_names)
                    || expression_contains_callee_name(getter, callee_names)
            }
            ObjectEntry::Setter { key, setter } => {
                expression_contains_callee_name(key, callee_names)
                    || expression_contains_callee_name(setter, callee_names)
            }
            ObjectEntry::Spread(expression) => {
                expression_contains_callee_name(expression, callee_names)
            }
        }),
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

fn statement_contains_callee_name(statement: &Statement, callee_names: &HashSet<String>) -> bool {
    match statement {
        Statement::Declaration { body }
        | Statement::Block { body }
        | Statement::Labeled { body, .. } => body
            .iter()
            .any(|statement| statement_contains_callee_name(statement, callee_names)),
        Statement::Var { value, .. }
        | Statement::Let { value, .. }
        | Statement::Expression(value)
        | Statement::Throw(value)
        | Statement::Return(value)
        | Statement::Yield { value }
        | Statement::YieldDelegate { value }
        | Statement::Assign { value, .. } => expression_contains_callee_name(value, callee_names),
        Statement::AssignMember {
            object,
            property,
            value,
        } => {
            expression_contains_callee_name(object, callee_names)
                || expression_contains_callee_name(property, callee_names)
                || expression_contains_callee_name(value, callee_names)
        }
        Statement::Print { values } => values
            .iter()
            .any(|value| expression_contains_callee_name(value, callee_names)),
        Statement::With { object, body } => {
            expression_contains_callee_name(object, callee_names)
                || body
                    .iter()
                    .any(|statement| statement_contains_callee_name(statement, callee_names))
        }
        Statement::If {
            condition,
            then_branch,
            else_branch,
        } => {
            expression_contains_callee_name(condition, callee_names)
                || then_branch
                    .iter()
                    .any(|statement| statement_contains_callee_name(statement, callee_names))
                || else_branch
                    .iter()
                    .any(|statement| statement_contains_callee_name(statement, callee_names))
        }
        Statement::Try {
            body,
            catch_setup,
            catch_body,
            ..
        } => {
            body.iter()
                .any(|statement| statement_contains_callee_name(statement, callee_names))
                || catch_setup
                    .iter()
                    .any(|statement| statement_contains_callee_name(statement, callee_names))
                || catch_body
                    .iter()
                    .any(|statement| statement_contains_callee_name(statement, callee_names))
        }
        Statement::Switch {
            discriminant,
            cases,
            ..
        } => {
            expression_contains_callee_name(discriminant, callee_names)
                || cases.iter().any(|case| {
                    case.test
                        .as_ref()
                        .is_some_and(|test| expression_contains_callee_name(test, callee_names))
                        || case.body.iter().any(|statement| {
                            statement_contains_callee_name(statement, callee_names)
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
            init.iter()
                .any(|statement| statement_contains_callee_name(statement, callee_names))
                || condition.as_ref().is_some_and(|condition| {
                    expression_contains_callee_name(condition, callee_names)
                })
                || update
                    .as_ref()
                    .is_some_and(|update| expression_contains_callee_name(update, callee_names))
                || break_hook.as_ref().is_some_and(|break_hook| {
                    expression_contains_callee_name(break_hook, callee_names)
                })
                || body
                    .iter()
                    .any(|statement| statement_contains_callee_name(statement, callee_names))
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
            expression_contains_callee_name(condition, callee_names)
                || break_hook.as_ref().is_some_and(|break_hook| {
                    expression_contains_callee_name(break_hook, callee_names)
                })
                || body
                    .iter()
                    .any(|statement| statement_contains_callee_name(statement, callee_names))
        }
        Statement::Break { .. } | Statement::Continue { .. } => false,
    }
}
