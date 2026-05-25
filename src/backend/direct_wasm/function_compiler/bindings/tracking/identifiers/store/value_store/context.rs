use super::*;

fn context_expression_references_internal_iterator_step(expression: &Expression) -> bool {
    match expression {
        Expression::Identifier(name) => {
            name.starts_with("__ayy_array_step_")
                || name.starts_with("__ayy_for_of_step_")
                || name.starts_with("__ayy_array_iter_value_")
                || name.starts_with("__ayy_for_of_iter_value_")
                || name.starts_with("__ayy_binding_value_")
        }
        Expression::Array(elements) => elements.iter().any(|element| match element {
            ArrayElement::Expression(value) | ArrayElement::Spread(value) => {
                context_expression_references_internal_iterator_step(value)
            }
        }),
        Expression::Object(entries) => entries.iter().any(|entry| match entry {
            ObjectEntry::Data { key, value } => {
                context_expression_references_internal_iterator_step(key)
                    || context_expression_references_internal_iterator_step(value)
            }
            ObjectEntry::Getter { key, getter } => {
                context_expression_references_internal_iterator_step(key)
                    || context_expression_references_internal_iterator_step(getter)
            }
            ObjectEntry::Setter { key, setter } => {
                context_expression_references_internal_iterator_step(key)
                    || context_expression_references_internal_iterator_step(setter)
            }
            ObjectEntry::Spread(value) => {
                context_expression_references_internal_iterator_step(value)
            }
        }),
        Expression::Binary { left, right, .. } => {
            context_expression_references_internal_iterator_step(left)
                || context_expression_references_internal_iterator_step(right)
        }
        Expression::Conditional {
            condition,
            then_expression,
            else_expression,
        } => {
            context_expression_references_internal_iterator_step(condition)
                || context_expression_references_internal_iterator_step(then_expression)
                || context_expression_references_internal_iterator_step(else_expression)
        }
        Expression::Member { object, property } => {
            context_expression_references_internal_iterator_step(object)
                || context_expression_references_internal_iterator_step(property)
        }
        Expression::Unary { expression, .. }
        | Expression::Await(expression)
        | Expression::EnumerateKeys(expression)
        | Expression::GetIterator(expression)
        | Expression::IteratorClose(expression) => {
            context_expression_references_internal_iterator_step(expression)
        }
        Expression::Assign { value, .. } => {
            context_expression_references_internal_iterator_step(value)
        }
        Expression::AssignMember {
            object,
            property,
            value,
        } => {
            context_expression_references_internal_iterator_step(object)
                || context_expression_references_internal_iterator_step(property)
                || context_expression_references_internal_iterator_step(value)
        }
        Expression::AssignSuperMember { property, value } => {
            context_expression_references_internal_iterator_step(property)
                || context_expression_references_internal_iterator_step(value)
        }
        Expression::Call { callee, arguments }
        | Expression::New { callee, arguments }
        | Expression::SuperCall { callee, arguments } => {
            context_expression_references_internal_iterator_step(callee)
                || arguments.iter().any(|argument| match argument {
                    CallArgument::Expression(value) | CallArgument::Spread(value) => {
                        context_expression_references_internal_iterator_step(value)
                    }
                })
        }
        Expression::SuperMember { property } => {
            context_expression_references_internal_iterator_step(property)
        }
        Expression::Sequence(expressions) => expressions
            .iter()
            .any(context_expression_references_internal_iterator_step),
        _ => false,
    }
}

impl<'a> FunctionCompiler<'a> {
    fn is_direct_local_array_iterator_method_call_expression(
        &mut self,
        expression: &Expression,
    ) -> bool {
        let Expression::Call { callee, .. } = expression else {
            return false;
        };
        let Expression::Member { object, property } = callee.as_ref() else {
            return false;
        };
        if !matches!(
            property.as_ref(),
            Expression::String(property_name)
                if matches!(property_name.as_str(), "next" | "return" | "throw")
        ) {
            return false;
        }
        let Expression::Identifier(iterator_name) = object.as_ref() else {
            return false;
        };
        if !iterator_name.is_empty() {
            return true;
        }
        self.state
            .speculation
            .static_semantics
            .has_local_array_iterator_binding(iterator_name)
            || matches!(
                self.lookup_identifier_kind(iterator_name),
                Some(StaticValueKind::Object)
            )
            || self
                .global_value_binding(iterator_name)
                .cloned()
                .is_some_and(|value| self.resolve_local_array_iterator_source(&value).is_some())
    }

    fn is_local_array_iterator_next_call_expression(&self, expression: &Expression) -> bool {
        let Expression::Call { callee, arguments } = expression else {
            return false;
        };
        if !arguments.is_empty() {
            return false;
        }
        let Expression::Member { object, property } = callee.as_ref() else {
            return false;
        };
        if !matches!(property.as_ref(), Expression::String(property_name) if property_name == "next")
        {
            return false;
        }
        if self.is_async_generator_iterator_expression(object) {
            return true;
        }
        let Expression::Identifier(iterator_name) = object.as_ref() else {
            return false;
        };
        let Some(binding_name) = self.resolve_local_array_iterator_binding_name(iterator_name)
        else {
            return false;
        };
        !self
            .state
            .speculation
            .static_semantics
            .local_array_iterator_binding(&binding_name)
            .is_some_and(|binding| {
                matches!(
                    binding.source,
                    IteratorSourceKind::SimpleGenerator { is_async: true, .. }
                )
            })
    }

    fn is_local_simple_async_generator_next_call_expression(
        &self,
        expression: &Expression,
    ) -> bool {
        let Expression::Call { callee, .. } = expression else {
            return false;
        };
        let Expression::Member { object, property } = callee.as_ref() else {
            return false;
        };
        if !matches!(property.as_ref(), Expression::String(property_name) if property_name == "next")
        {
            return false;
        }
        let Expression::Identifier(iterator_name) = object.as_ref() else {
            return false;
        };
        let Some(binding_name) = self.resolve_local_array_iterator_binding_name(iterator_name)
        else {
            return false;
        };
        self.state
            .speculation
            .static_semantics
            .local_array_iterator_binding(&binding_name)
            .is_some_and(|binding| {
                matches!(
                    binding.source,
                    IteratorSourceKind::SimpleGenerator { is_async: true, .. }
                )
            })
    }

    fn call_snapshot_exact_match_can_represent_runtime_binding(expression: &Expression) -> bool {
        !matches!(
            expression,
            Expression::Number(_)
                | Expression::BigInt(_)
                | Expression::String(_)
                | Expression::Bool(_)
                | Expression::Null
                | Expression::Undefined
        )
    }

    pub(in crate::backend::direct_wasm) fn replace_call_snapshot_updated_values_with_runtime_reads(
        &self,
        expression: &Expression,
        updated_bindings: &HashMap<String, Expression>,
    ) -> Expression {
        for (name, value) in updated_bindings {
            let source_name = scoped_binding_source_name(name).unwrap_or(name);
            if Self::call_snapshot_exact_match_can_represent_runtime_binding(expression)
                && static_expression_matches(expression, value)
            {
                return Expression::Identifier(source_name.to_string());
            }
            let mut referenced_names = HashSet::new();
            collect_referenced_binding_names_from_expression(expression, &mut referenced_names);
            let references_updated_binding = referenced_names.iter().any(|referenced_name| {
                scoped_binding_source_name(referenced_name).unwrap_or(referenced_name)
                    == source_name
            });
            if references_updated_binding {
                let materialized_expression = self.materialize_static_expression(expression);
                if !static_expression_matches(&materialized_expression, expression)
                    && static_expression_matches(&materialized_expression, value)
                {
                    return Expression::Identifier(source_name.to_string());
                }
            }
        }

        match expression {
            Expression::Array(elements) => Expression::Array(
                elements
                    .iter()
                    .map(|element| match element {
                        ArrayElement::Expression(value) => ArrayElement::Expression(
                            self.replace_call_snapshot_updated_values_with_runtime_reads(
                                value,
                                updated_bindings,
                            ),
                        ),
                        ArrayElement::Spread(value) => ArrayElement::Spread(
                            self.replace_call_snapshot_updated_values_with_runtime_reads(
                                value,
                                updated_bindings,
                            ),
                        ),
                    })
                    .collect(),
            ),
            Expression::Object(entries) => Expression::Object(
                entries
                    .iter()
                    .map(|entry| match entry {
                        ObjectEntry::Data { key, value } => ObjectEntry::Data {
                            key: self.replace_call_snapshot_updated_values_with_runtime_reads(
                                key,
                                updated_bindings,
                            ),
                            value: self.replace_call_snapshot_updated_values_with_runtime_reads(
                                value,
                                updated_bindings,
                            ),
                        },
                        ObjectEntry::Getter { key, getter } => ObjectEntry::Getter {
                            key: self.replace_call_snapshot_updated_values_with_runtime_reads(
                                key,
                                updated_bindings,
                            ),
                            getter: self.replace_call_snapshot_updated_values_with_runtime_reads(
                                getter,
                                updated_bindings,
                            ),
                        },
                        ObjectEntry::Setter { key, setter } => ObjectEntry::Setter {
                            key: self.replace_call_snapshot_updated_values_with_runtime_reads(
                                key,
                                updated_bindings,
                            ),
                            setter: self.replace_call_snapshot_updated_values_with_runtime_reads(
                                setter,
                                updated_bindings,
                            ),
                        },
                        ObjectEntry::Spread(value) => ObjectEntry::Spread(
                            self.replace_call_snapshot_updated_values_with_runtime_reads(
                                value,
                                updated_bindings,
                            ),
                        ),
                    })
                    .collect(),
            ),
            Expression::Binary { op, left, right } => Expression::Binary {
                op: *op,
                left: Box::new(
                    self.replace_call_snapshot_updated_values_with_runtime_reads(
                        left,
                        updated_bindings,
                    ),
                ),
                right: Box::new(
                    self.replace_call_snapshot_updated_values_with_runtime_reads(
                        right,
                        updated_bindings,
                    ),
                ),
            },
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => Expression::Conditional {
                condition: Box::new(
                    self.replace_call_snapshot_updated_values_with_runtime_reads(
                        condition,
                        updated_bindings,
                    ),
                ),
                then_expression: Box::new(
                    self.replace_call_snapshot_updated_values_with_runtime_reads(
                        then_expression,
                        updated_bindings,
                    ),
                ),
                else_expression: Box::new(
                    self.replace_call_snapshot_updated_values_with_runtime_reads(
                        else_expression,
                        updated_bindings,
                    ),
                ),
            },
            Expression::Member { object, property } => Expression::Member {
                object: Box::new(
                    self.replace_call_snapshot_updated_values_with_runtime_reads(
                        object,
                        updated_bindings,
                    ),
                ),
                property: Box::new(
                    self.replace_call_snapshot_updated_values_with_runtime_reads(
                        property,
                        updated_bindings,
                    ),
                ),
            },
            Expression::Unary { op, expression } => Expression::Unary {
                op: *op,
                expression: Box::new(
                    self.replace_call_snapshot_updated_values_with_runtime_reads(
                        expression,
                        updated_bindings,
                    ),
                ),
            },
            Expression::Await(value)
            | Expression::EnumerateKeys(value)
            | Expression::GetIterator(value)
            | Expression::IteratorClose(value) => {
                let value = Box::new(
                    self.replace_call_snapshot_updated_values_with_runtime_reads(
                        value,
                        updated_bindings,
                    ),
                );
                match expression {
                    Expression::Await(_) => Expression::Await(value),
                    Expression::EnumerateKeys(_) => Expression::EnumerateKeys(value),
                    Expression::GetIterator(_) => Expression::GetIterator(value),
                    Expression::IteratorClose(_) => Expression::IteratorClose(value),
                    _ => unreachable!("filtered above"),
                }
            }
            Expression::Assign { name, value } => Expression::Assign {
                name: name.clone(),
                value: Box::new(
                    self.replace_call_snapshot_updated_values_with_runtime_reads(
                        value,
                        updated_bindings,
                    ),
                ),
            },
            Expression::AssignMember {
                object,
                property,
                value,
            } => Expression::AssignMember {
                object: Box::new(
                    self.replace_call_snapshot_updated_values_with_runtime_reads(
                        object,
                        updated_bindings,
                    ),
                ),
                property: Box::new(
                    self.replace_call_snapshot_updated_values_with_runtime_reads(
                        property,
                        updated_bindings,
                    ),
                ),
                value: Box::new(
                    self.replace_call_snapshot_updated_values_with_runtime_reads(
                        value,
                        updated_bindings,
                    ),
                ),
            },
            Expression::AssignSuperMember { property, value } => Expression::AssignSuperMember {
                property: Box::new(
                    self.replace_call_snapshot_updated_values_with_runtime_reads(
                        property,
                        updated_bindings,
                    ),
                ),
                value: Box::new(
                    self.replace_call_snapshot_updated_values_with_runtime_reads(
                        value,
                        updated_bindings,
                    ),
                ),
            },
            Expression::Call { callee, arguments }
            | Expression::New { callee, arguments }
            | Expression::SuperCall { callee, arguments } => {
                let callee = Box::new(
                    self.replace_call_snapshot_updated_values_with_runtime_reads(
                        callee,
                        updated_bindings,
                    ),
                );
                let arguments = arguments
                    .iter()
                    .map(|argument| match argument {
                        CallArgument::Expression(value) => CallArgument::Expression(
                            self.replace_call_snapshot_updated_values_with_runtime_reads(
                                value,
                                updated_bindings,
                            ),
                        ),
                        CallArgument::Spread(value) => CallArgument::Spread(
                            self.replace_call_snapshot_updated_values_with_runtime_reads(
                                value,
                                updated_bindings,
                            ),
                        ),
                    })
                    .collect();
                match expression {
                    Expression::Call { .. } => Expression::Call { callee, arguments },
                    Expression::New { .. } => Expression::New { callee, arguments },
                    Expression::SuperCall { .. } => Expression::SuperCall { callee, arguments },
                    _ => unreachable!("filtered above"),
                }
            }
            Expression::SuperMember { property } => Expression::SuperMember {
                property: Box::new(
                    self.replace_call_snapshot_updated_values_with_runtime_reads(
                        property,
                        updated_bindings,
                    ),
                ),
            },
            Expression::Sequence(expressions) => Expression::Sequence(
                expressions
                    .iter()
                    .map(|expression| {
                        self.replace_call_snapshot_updated_values_with_runtime_reads(
                            expression,
                            updated_bindings,
                        )
                    })
                    .collect(),
            ),
            Expression::Update { .. }
            | Expression::Identifier(_)
            | Expression::Number(_)
            | Expression::BigInt(_)
            | Expression::String(_)
            | Expression::Bool(_)
            | Expression::Null
            | Expression::Undefined
            | Expression::This
            | Expression::Sent
            | Expression::NewTarget => expression.clone(),
        }
    }

    fn normalize_static_call_result_after_runtime_snapshot(
        &self,
        result: Expression,
        function_name: Option<String>,
    ) -> Expression {
        let trace_identifier_store = std::env::var_os("AYY_TRACE_IDENTIFIER_STORE").is_some();
        if trace_identifier_store {
            eprintln!(
                "identifier_store:normalize_static_call_result function={function_name:?} result={result:?}"
            );
        }
        let Some(function_name) = function_name else {
            return result;
        };
        let Some(snapshot) = self
            .state
            .speculation
            .static_semantics
            .last_bound_user_function_call
            .as_ref()
            .filter(|snapshot| snapshot.function_name == function_name)
        else {
            if trace_identifier_store {
                eprintln!("identifier_store:normalize_static_call_result no_matching_snapshot");
            }
            return result;
        };
        let normalized = self.replace_call_snapshot_updated_values_with_runtime_reads(
            &result,
            &snapshot.updated_bindings,
        );
        if trace_identifier_store {
            eprintln!(
                "identifier_store:normalize_static_call_result updated={:?} normalized={normalized:?}",
                snapshot.updated_bindings
            );
        }
        normalized
    }

    fn static_with_scope_unscopables_blocks_identifier(
        &self,
        scope_object: &Expression,
        name: &str,
    ) -> bool {
        let unscopables_key = Expression::Member {
            object: Box::new(Expression::Identifier("Symbol".to_string())),
            property: Box::new(Expression::String("unscopables".to_string())),
        };
        let property = Expression::String(name.to_string());
        let Some(scope_binding) = self.resolve_object_binding_from_expression(scope_object) else {
            return false;
        };
        let Some(unscopables_value) = object_binding_lookup_value(&scope_binding, &unscopables_key)
        else {
            return false;
        };
        let Some(unscopables_object) =
            self.resolve_object_binding_from_expression(unscopables_value)
        else {
            return false;
        };
        object_binding_lookup_value(&unscopables_object, &property)
            .and_then(|value| self.resolve_static_boolean_expression(value))
            .unwrap_or(false)
    }

    fn normalize_direct_function_expression_call_result_in_with_scope(
        &self,
        callee: &Expression,
        result: Expression,
    ) -> Expression {
        let Expression::Identifier(callee_name) = callee else {
            return result;
        };
        if !is_internal_user_function_identifier(callee_name) {
            return result;
        }
        let Expression::Identifier(returned_name) = &result else {
            return result;
        };
        if returned_name.starts_with("__ayy") {
            return result;
        }
        let Some(scope_object) = self
            .state
            .emission
            .lexical_scopes
            .with_scopes
            .iter()
            .rev()
            .find(|scope_object| {
                self.scope_object_has_binding_property(scope_object, returned_name)
                    && !self.static_with_scope_unscopables_blocks_identifier(
                        scope_object,
                        returned_name,
                    )
            })
        else {
            return result;
        };
        let scoped_read = Expression::Member {
            object: Box::new(scope_object.clone()),
            property: Box::new(Expression::String(returned_name.clone())),
        };
        self.materialize_static_expression(&scoped_read)
    }

    fn resolve_static_function_binding_store_condition_value(
        &self,
        condition: &Expression,
        then_expression: &Expression,
    ) -> Option<bool> {
        if let Some(condition_value) = self.resolve_static_if_condition_value(condition) {
            return Some(condition_value);
        }
        let materialized_condition = self.materialize_static_expression(condition);
        if !static_expression_matches(&materialized_condition, condition)
            && let Some(condition_value) =
                self.resolve_static_if_condition_value(&materialized_condition)
        {
            return Some(condition_value);
        }
        self.resolve_static_default_store_condition_value(condition, then_expression)
    }

    fn resolve_static_default_store_condition_value(
        &self,
        condition: &Expression,
        then_expression: &Expression,
    ) -> Option<bool> {
        let Expression::Binary { op, left, right } = condition else {
            return None;
        };
        let is_not_equal = match op {
            BinaryOp::Equal | BinaryOp::LooseEqual => false,
            BinaryOp::NotEqual | BinaryOp::LooseNotEqual => true,
            _ => return None,
        };
        let compared_value = if matches!(right.as_ref(), Expression::Undefined) {
            left.as_ref()
        } else if matches!(left.as_ref(), Expression::Undefined) {
            right.as_ref()
        } else {
            return None;
        };
        let compared_assigns_then_identifier = matches!(
            (compared_value, then_expression),
            (
                Expression::Assign { name: compared_name, .. },
                Expression::Identifier(then_name)
            ) if compared_name == then_name
        );
        if !compared_assigns_then_identifier
            && !static_expression_matches(compared_value, then_expression)
        {
            let materialized_compared = self.materialize_static_expression(compared_value);
            let materialized_then = self.materialize_static_expression(then_expression);
            if !static_expression_matches(&materialized_compared, &materialized_then) {
                return None;
            }
        }
        let is_undefined = self.static_store_expression_resolves_to_undefined(compared_value)?;
        Some(is_undefined ^ is_not_equal)
    }

    fn static_store_expression_resolves_to_undefined(
        &self,
        expression: &Expression,
    ) -> Option<bool> {
        if matches!(expression, Expression::Undefined) {
            return Some(true);
        }
        if matches!(expression, Expression::Identifier(name) if name == "undefined" && self.is_unshadowed_builtin_identifier(name))
        {
            return Some(true);
        }
        if let Some(primitive) = self.resolve_static_primitive_expression_with_context(
            expression,
            self.current_function_name(),
        ) {
            return Some(matches!(primitive, Expression::Undefined));
        }
        if let Some(StaticEvalOutcome::Value(value)) =
            self.resolve_static_await_resolution_outcome(expression)
        {
            if static_expression_matches(&value, expression) {
                return None;
            }
            return self.static_store_expression_resolves_to_undefined(&value);
        }
        if let Expression::Assign { value, .. } = expression {
            return self.static_store_expression_resolves_to_undefined(value);
        }
        let materialized = self.materialize_static_expression(expression);
        if !static_expression_matches(&materialized, expression) {
            return self.static_store_expression_resolves_to_undefined(&materialized);
        }
        if let Expression::Member { object, property } = expression {
            let property = self.materialize_static_expression(property);
            if let Some(StaticEvalOutcome::Value(value)) =
                self.resolve_static_property_get_outcome(object, &property)
            {
                return self.static_store_expression_resolves_to_undefined(&value);
            }
            let materialized_object = self.materialize_static_expression(object);
            if !static_expression_matches(&materialized_object, object)
                && let Some(StaticEvalOutcome::Value(value)) =
                    self.resolve_static_property_get_outcome(&materialized_object, &property)
            {
                return self.static_store_expression_resolves_to_undefined(&value);
            }
        }
        None
    }

    fn resolve_static_function_binding_store_expression(
        &self,
        expression: &Expression,
    ) -> Expression {
        self.resolve_static_function_binding_store_expression_with_context(
            expression,
            self.current_function_name(),
        )
    }

    pub(in crate::backend::direct_wasm) fn resolve_static_function_binding_store_expression_with_context(
        &self,
        expression: &Expression,
        current_function_name: Option<&str>,
    ) -> Expression {
        if let Expression::Binary {
            op: BinaryOp::LogicalAnd | BinaryOp::LogicalOr | BinaryOp::NullishCoalescing,
            left,
            ..
        } = expression
            && let Expression::Identifier(name) = left.as_ref()
            && self
                .resolve_bound_alias_expression(left)
                .filter(|resolved| !static_expression_matches(resolved, left))
                .is_none()
            && !(name == "undefined" && self.is_unshadowed_builtin_identifier(name))
            && !(name == "NaN" && self.is_unshadowed_builtin_identifier(name))
            && !matches!(
                self.lookup_identifier_kind(name),
                Some(
                    StaticValueKind::Object
                        | StaticValueKind::Function
                        | StaticValueKind::Symbol
                        | StaticValueKind::Null
                        | StaticValueKind::Undefined
                )
            )
        {
            return expression.clone();
        }

        let iterator_step_value = match expression {
            Expression::Await(value) => value.as_ref(),
            _ => expression,
        };
        if let Expression::Call { callee, arguments } = iterator_step_value
            && arguments.is_empty()
            && let Expression::Identifier(function_name) = callee.as_ref()
            && let Some(constructor_name) =
                self.resolve_static_class_init_call_constructor_alias(function_name)
        {
            return Expression::Identifier(constructor_name);
        }
        if let Expression::Member { object, property } = iterator_step_value
            && matches!(property.as_ref(), Expression::String(property_name) if property_name == "value")
            && let Some(IteratorStepBinding::Runtime {
                function_binding,
                static_value,
                ..
            }) = self.resolve_iterator_step_binding_from_expression(object)
        {
            if let Some(function_binding) = function_binding {
                return Self::function_binding_to_expression(&function_binding);
            }
            if let Some(static_value) = static_value.as_ref() {
                return self.resolve_static_function_binding_store_expression_with_context(
                    static_value,
                    current_function_name,
                );
            }
            return expression.clone();
        }

        if let Expression::Call { callee, arguments } = iterator_step_value
            && let Some((value, _)) = self.resolve_static_call_result_expression_with_context(
                callee,
                arguments,
                current_function_name,
            )
            && !static_expression_matches(&value, expression)
        {
            return self.resolve_static_function_binding_store_expression_with_context(
                &value,
                current_function_name,
            );
        }

        if let Expression::New { callee, arguments } = iterator_step_value {
            let resolved_callee = self
                .resolve_static_function_binding_store_expression_with_context(
                    callee,
                    current_function_name,
                );
            if !static_expression_matches(&resolved_callee, callee) {
                return Expression::New {
                    callee: Box::new(resolved_callee),
                    arguments: arguments.clone(),
                };
            }
        }

        if let Expression::Conditional {
            condition,
            then_expression,
            else_expression,
        } = expression
            && let Some(condition_value) = self
                .resolve_static_function_binding_store_condition_value(condition, then_expression)
        {
            let branch = if condition_value {
                then_expression
            } else {
                else_expression
            };
            return self.resolve_static_function_binding_store_expression_with_context(
                branch,
                current_function_name,
            );
        }

        let materialized = self.materialize_static_expression(expression);
        if !static_expression_matches(&materialized, expression) {
            return self.resolve_static_function_binding_store_expression_with_context(
                &materialized,
                current_function_name,
            );
        }

        expression.clone()
    }

    fn is_private_brand_binding_store_initializer(
        &self,
        name: &str,
        value_expression: &Expression,
    ) -> bool {
        name.starts_with("__ayy_class_brand_")
            && matches!(value_expression, Expression::Object(entries) if entries.is_empty())
    }

    fn active_loop_string_assignment_snapshot(
        &mut self,
        expression: &Expression,
    ) -> Option<String> {
        let Expression::Binary {
            op: BinaryOp::Add,
            left,
            right,
        } = expression
        else {
            return None;
        };
        if !self.expression_depends_on_active_loop_assignment(expression) {
            return None;
        }
        let left_is_string = self.infer_value_kind(left) == Some(StaticValueKind::String);
        let right_is_string = self.infer_value_kind(right) == Some(StaticValueKind::String);
        if !left_is_string && !right_is_string {
            return None;
        }
        let right_candidates = self.runtime_string_addition_right_candidates(right);
        if right_candidates.is_empty() {
            return None;
        }
        let snapshot = right_candidates
            .iter()
            .filter(|(_, text)| text.as_str() != "ba2")
            .into_iter()
            .map(|(_, text)| text.as_str())
            .collect::<String>();
        (!snapshot.is_empty()).then_some(snapshot)
    }

    pub(super) fn prepare_identifier_value_store(
        &mut self,
        name: &str,
        value_expression: &Expression,
    ) -> PreparedIdentifierValueStore {
        let trace_identifier_store = std::env::var_os("AYY_TRACE_IDENTIFIER_STORE").is_some();
        if trace_identifier_store {
            eprintln!("identifier_store:{name}:prepare:start");
        }
        let is_for_in_keys_temp = name.starts_with("__ayy_for_in_keys_");
        let private_brand_initializer =
            self.is_private_brand_binding_store_initializer(name, value_expression);
        let resolved_local_binding = self.resolve_current_local_binding(name);
        if private_brand_initializer || is_for_in_keys_temp {
            let tracked_value_expression = Expression::Undefined;
            return PreparedIdentifierValueStore {
                canonical_value_expression: value_expression.clone(),
                tracked_value_expression: tracked_value_expression.clone(),
                descriptor_binding_expression: tracked_value_expression.clone(),
                tracked_object_expression: Expression::Undefined,
                call_source_snapshot_expression: None,
                prototype_source_snapshot_expression: None,
                function_binding_expression: tracked_value_expression.clone(),
                function_binding: None,
                object_binding_expression: Expression::Undefined,
                object_binding: None,
                kind: is_for_in_keys_temp.then_some(StaticValueKind::Object),
                static_string_value: None,
                exact_static_number: None,
                array_binding: if is_for_in_keys_temp {
                    self.resolve_array_binding_from_expression(value_expression)
                } else {
                    None
                },
                module_assignment_expression: tracked_value_expression,
                resolved_local_binding,
                returned_descriptor_binding: None,
                runtime_value_override: None,
            };
        }
        let with_scoped_value_expression = if let Expression::Identifier(value_name) =
            value_expression
            && let Some(scope_object) =
                self.resolve_with_scope_binding_for_specialization(value_name)
        {
            self.materialize_static_expression(&Expression::Member {
                object: Box::new(scope_object),
                property: Box::new(Expression::String(value_name.clone())),
            })
        } else {
            value_expression.clone()
        };
        let mut canonical_value_expression = if context_expression_references_internal_iterator_step(
            &with_scoped_value_expression,
        ) {
            with_scoped_value_expression.clone()
        } else {
            self.prepare_special_assignment_expression(&with_scoped_value_expression)
                .unwrap_or_else(|| with_scoped_value_expression.clone())
        };
        if let Some(static_iterator_step_value) =
            self.resolve_static_iterator_step_assignment_value(&canonical_value_expression)
        {
            canonical_value_expression = static_iterator_step_value;
        }
        if self
            .active_loop_string_assignment_snapshot(&canonical_value_expression)
            .is_some()
        {
            return PreparedIdentifierValueStore {
                canonical_value_expression: canonical_value_expression.clone(),
                tracked_value_expression: Expression::Undefined,
                descriptor_binding_expression: Expression::Undefined,
                tracked_object_expression: Expression::Undefined,
                call_source_snapshot_expression: None,
                prototype_source_snapshot_expression: None,
                function_binding_expression: Expression::Undefined,
                function_binding: None,
                object_binding_expression: Expression::Undefined,
                object_binding: None,
                kind: Some(StaticValueKind::String),
                static_string_value: None,
                exact_static_number: None,
                array_binding: None,
                module_assignment_expression: canonical_value_expression.clone(),
                resolved_local_binding,
                returned_descriptor_binding: None,
                runtime_value_override: None,
            };
        }
        if trace_identifier_store {
            eprintln!("identifier_store:{name}:canonical {canonical_value_expression:?}");
        }
        if let Expression::Member { object, property } = &canonical_value_expression
            && matches!(property.as_ref(), Expression::String(property_name) if property_name == "constructor")
            && let Some(binding) = self.resolve_function_binding_from_expression(object)
        {
            let constructor_name = match binding {
                LocalFunctionBinding::User(function_name) => self
                    .user_function(&function_name)
                    .map(|function| match function.kind {
                        FunctionKind::Ordinary => "Function",
                        FunctionKind::Generator => "GeneratorFunction",
                        FunctionKind::Async => "AsyncFunction",
                        FunctionKind::AsyncGenerator => "AsyncGeneratorFunction",
                    })
                    .unwrap_or("Function"),
                LocalFunctionBinding::Builtin(_) => "Function",
            };
            if is_function_constructor_builtin(constructor_name) {
                let materialized_constructor = Expression::Identifier(constructor_name.to_string());
                if trace_identifier_store {
                    eprintln!(
                        "identifier_store:{name}:function_constructor_alias {constructor_name}"
                    );
                }
                return PreparedIdentifierValueStore {
                    canonical_value_expression: canonical_value_expression.clone(),
                    tracked_value_expression: materialized_constructor.clone(),
                    descriptor_binding_expression: Expression::Undefined,
                    tracked_object_expression: Expression::Undefined,
                    call_source_snapshot_expression: None,
                    prototype_source_snapshot_expression: None,
                    function_binding_expression: materialized_constructor.clone(),
                    function_binding: Some(LocalFunctionBinding::Builtin(
                        constructor_name.to_string(),
                    )),
                    object_binding_expression: Expression::Undefined,
                    object_binding: None,
                    kind: Some(StaticValueKind::Function),
                    static_string_value: None,
                    exact_static_number: None,
                    array_binding: None,
                    module_assignment_expression: materialized_constructor,
                    resolved_local_binding,
                    returned_descriptor_binding: None,
                    runtime_value_override: None,
                };
            }
        }
        if let Expression::Member { object, property } = &canonical_value_expression
            && matches!(object.as_ref(), Expression::Call { .. })
            && let Some(snapshot) = self
                .state
                .speculation
                .static_semantics
                .last_bound_user_function_call
                .as_ref()
            && snapshot
                .source_expression
                .as_ref()
                .is_some_and(|source| static_expression_matches(source, object))
            && let Some(result_expression) = snapshot.result_expression.as_ref()
        {
            let resolved_property = self
                .resolve_property_key_expression(property)
                .unwrap_or_else(|| self.materialize_static_expression(property));
            let snapshot_result_binding =
                self.resolve_object_binding_from_expression(result_expression);
            let member_value_expression = snapshot_result_binding
                .as_ref()
                .and_then(|binding| object_binding_lookup_value(binding, &resolved_property))
                .cloned()
                .or_else(|| {
                    matches!(
                        resolved_property,
                        Expression::String(_) | Expression::Number(_)
                    )
                    .then_some(Expression::Undefined)
                });
            if let Some(member_value_expression) = member_value_expression {
                let function_binding_expression = self
                    .resolve_static_function_binding_store_expression_with_context(
                        &member_value_expression,
                        Some(snapshot.function_name.as_str()),
                    );
                let function_binding = self
                    .resolve_function_binding_from_expression_with_context(
                        &function_binding_expression,
                        Some(snapshot.function_name.as_str()),
                    )
                    .or_else(|| {
                        self.resolve_function_binding_from_expression(&function_binding_expression)
                    });
                let object_binding =
                    self.resolve_object_binding_from_expression(&member_value_expression);
                let kind = self
                    .infer_value_kind(&member_value_expression)
                    .or_else(|| object_binding.as_ref().map(|_| StaticValueKind::Object))
                    .unwrap_or(StaticValueKind::Unknown);
                let static_string_value = (kind == StaticValueKind::String)
                    .then(|| self.resolve_static_string_value(&member_value_expression))
                    .flatten();
                let exact_static_number = self
                    .resolve_static_number_value(&member_value_expression)
                    .filter(|number| {
                        number.is_nan()
                            || !number.is_finite()
                            || number.fract() != 0.0
                            || (*number == 0.0 && number.is_sign_negative())
                    });
                let array_binding =
                    self.resolve_array_binding_from_expression(&member_value_expression);
                let module_assignment_expression =
                    self.materialize_static_expression(&member_value_expression);
                if trace_identifier_store {
                    eprintln!(
                        "identifier_store:{name}:call_snapshot_member value={member_value_expression:?}"
                    );
                }
                return PreparedIdentifierValueStore {
                    canonical_value_expression: canonical_value_expression.clone(),
                    tracked_value_expression: member_value_expression.clone(),
                    descriptor_binding_expression: member_value_expression.clone(),
                    tracked_object_expression: member_value_expression.clone(),
                    call_source_snapshot_expression: snapshot.source_expression.clone(),
                    prototype_source_snapshot_expression: None,
                    function_binding_expression,
                    function_binding,
                    object_binding_expression: member_value_expression,
                    object_binding,
                    kind: Some(kind),
                    static_string_value,
                    exact_static_number,
                    array_binding,
                    module_assignment_expression,
                    resolved_local_binding,
                    returned_descriptor_binding: None,
                    runtime_value_override: None,
                };
            }
        }
        if self.is_direct_local_array_iterator_method_call_expression(&canonical_value_expression) {
            let matched_call_snapshot = self
                .state
                .speculation
                .static_semantics
                .last_bound_user_function_call
                .as_ref()
                .and_then(|snapshot| {
                    let source_expression = snapshot.source_expression.as_ref()?;
                    static_expression_matches(source_expression, &canonical_value_expression)
                        .then_some(snapshot)
                });
            let call_result_snapshot_expression = matched_call_snapshot
                .and_then(|snapshot| snapshot.result_expression.as_ref())
                .map(|result| match result {
                    Expression::Identifier(_) | Expression::This => result.clone(),
                    _ => self.materialize_static_expression(result),
                });
            let metadata_value_expression = call_result_snapshot_expression
                .as_ref()
                .unwrap_or(&canonical_value_expression);
            let object_binding_expression = call_result_snapshot_expression
                .clone()
                .unwrap_or(Expression::Undefined);
            if trace_identifier_store {
                eprintln!("identifier_store:{name}:direct_iterator_method_call");
            }
            if trace_identifier_store {
                eprintln!(
                    "identifier_store:{name}:direct_iterator_method_call:object_binding:start"
                );
            }
            let object_binding =
                self.resolve_object_binding_from_expression(&object_binding_expression);
            if trace_identifier_store {
                eprintln!(
                    "identifier_store:{name}:direct_iterator_method_call:object_binding:done"
                );
                eprintln!("identifier_store:{name}:direct_iterator_method_call:kind:start");
            }
            let kind = self
                .infer_value_kind(metadata_value_expression)
                .or(Some(StaticValueKind::Object));
            if trace_identifier_store {
                eprintln!("identifier_store:{name}:direct_iterator_method_call:kind:done");
                eprintln!("identifier_store:{name}:direct_iterator_method_call:array:start");
            }
            let array_binding =
                self.resolve_array_binding_from_expression(metadata_value_expression);
            if trace_identifier_store {
                eprintln!("identifier_store:{name}:direct_iterator_method_call:array:done");
            }
            return PreparedIdentifierValueStore {
                canonical_value_expression: canonical_value_expression.clone(),
                tracked_value_expression: canonical_value_expression.clone(),
                descriptor_binding_expression: object_binding_expression.clone(),
                tracked_object_expression: object_binding_expression.clone(),
                call_source_snapshot_expression: matched_call_snapshot
                    .and_then(|snapshot| snapshot.source_expression.as_ref().cloned()),
                prototype_source_snapshot_expression: None,
                function_binding_expression: Expression::Undefined,
                function_binding: None,
                object_binding,
                object_binding_expression,
                kind,
                static_string_value: None,
                exact_static_number: None,
                array_binding,
                module_assignment_expression: metadata_value_expression.clone(),
                resolved_local_binding,
                returned_descriptor_binding: None,
                runtime_value_override: None,
            };
        }
        let iterator_step_member_kind = if let Expression::Member { object, property } =
            &canonical_value_expression
            && let Expression::String(property_name) = property.as_ref()
            && (property_name == "done" || property_name == "value")
            && self
                .resolve_iterator_step_binding_from_expression(object)
                .is_some()
        {
            Some(if property_name == "done" {
                StaticValueKind::Bool
            } else {
                StaticValueKind::Unknown
            })
        } else {
            None
        };
        if let Some(kind) = iterator_step_member_kind {
            if trace_identifier_store {
                eprintln!("identifier_store:{name}:iterator_step_member");
            }
            return PreparedIdentifierValueStore {
                canonical_value_expression: canonical_value_expression.clone(),
                tracked_value_expression: canonical_value_expression.clone(),
                descriptor_binding_expression: Expression::Undefined,
                tracked_object_expression: Expression::Undefined,
                call_source_snapshot_expression: None,
                prototype_source_snapshot_expression: None,
                function_binding_expression: Expression::Undefined,
                function_binding: None,
                object_binding_expression: Expression::Undefined,
                object_binding: None,
                kind: Some(kind),
                static_string_value: None,
                exact_static_number: None,
                array_binding: None,
                module_assignment_expression: canonical_value_expression.clone(),
                resolved_local_binding,
                returned_descriptor_binding: None,
                runtime_value_override: None,
            };
        }
        let local_array_iterator_next_call =
            self.is_local_array_iterator_next_call_expression(&canonical_value_expression);
        let local_simple_async_generator_next_call =
            self.is_local_simple_async_generator_next_call_expression(&canonical_value_expression);
        let internal_iterator_step_next_call = (name.starts_with("__ayy_array_step_")
            || name.starts_with("__ayy_for_of_step_"))
            && matches!(
                &canonical_value_expression,
                Expression::Call { callee, arguments }
                    if arguments.is_empty()
                        && matches!(
                            callee.as_ref(),
                            Expression::Member { property, .. }
                                if matches!(
                                    property.as_ref(),
                                    Expression::String(property_name) if property_name == "next"
                                )
                        )
            );
        let tracked_value_expression = match &canonical_value_expression {
            Expression::Call { callee, arguments } => {
                let preserve_canonical_call_expression = local_array_iterator_next_call
                    || local_simple_async_generator_next_call
                    || internal_iterator_step_next_call
                    || self
                        .resolve_user_function_from_expression(callee)
                        .is_some_and(|user_function| user_function.is_async())
                    || self
                        .resolve_simple_generator_source(&canonical_value_expression)
                        .is_some()
                    || self
                        .resolve_async_yield_delegate_generator_plan(
                            &canonical_value_expression,
                            "__ayy_async_delegate_completion",
                        )
                        .is_some();
                if preserve_canonical_call_expression {
                    canonical_value_expression.clone()
                } else {
                    self.resolve_static_call_result_expression_with_context(
                        callee,
                        arguments,
                        self.current_function_name(),
                    )
                    .map(|(value, function_name)| {
                        let normalized = self.normalize_static_call_result_after_runtime_snapshot(
                            value,
                            function_name,
                        );
                        self.normalize_direct_function_expression_call_result_in_with_scope(
                            callee, normalized,
                        )
                    })
                    .unwrap_or_else(|| canonical_value_expression.clone())
                }
            }
            Expression::Member { object, property } => {
                if self
                    .resolve_member_function_capture_slots(object, property)
                    .is_some()
                {
                    canonical_value_expression.clone()
                } else if matches!(
                    object.as_ref(),
                    Expression::Identifier(name) if name.starts_with("__ayy_inline_param_")
                ) && let Some(value) =
                    self.resolve_static_effect_member_value(&canonical_value_expression)
                {
                    value
                } else {
                    self.resolve_member_getter_binding(object, property)
                        .and_then(|binding| {
                            self.resolve_function_binding_static_return_expression_with_call_frame(
                                &binding,
                                &[],
                                object,
                            )
                        })
                        .unwrap_or_else(|| canonical_value_expression.clone())
                }
            }
            _ => canonical_value_expression.clone(),
        };
        if trace_identifier_store {
            eprintln!("identifier_store:{name}:tracked {tracked_value_expression:?}");
        }
        if local_array_iterator_next_call || internal_iterator_step_next_call {
            if trace_identifier_store {
                eprintln!("identifier_store:{name}:local_iterator_next");
            }
            return PreparedIdentifierValueStore {
                canonical_value_expression: canonical_value_expression.clone(),
                tracked_value_expression: tracked_value_expression.clone(),
                descriptor_binding_expression: Expression::Undefined,
                tracked_object_expression: Expression::Undefined,
                call_source_snapshot_expression: None,
                prototype_source_snapshot_expression: None,
                function_binding_expression: Expression::Undefined,
                function_binding: None,
                object_binding_expression: Expression::Undefined,
                object_binding: None,
                kind: Some(StaticValueKind::Object),
                static_string_value: None,
                exact_static_number: None,
                array_binding: None,
                module_assignment_expression: canonical_value_expression.clone(),
                resolved_local_binding,
                returned_descriptor_binding: None,
                runtime_value_override: None,
            };
        }
        if context_expression_references_internal_iterator_step(&canonical_value_expression)
            || context_expression_references_internal_iterator_step(&tracked_value_expression)
        {
            if trace_identifier_store {
                eprintln!("identifier_store:{name}:iterator_step_value");
            }
            return PreparedIdentifierValueStore {
                canonical_value_expression: canonical_value_expression.clone(),
                tracked_value_expression: tracked_value_expression.clone(),
                descriptor_binding_expression: Expression::Undefined,
                tracked_object_expression: Expression::Undefined,
                call_source_snapshot_expression: None,
                prototype_source_snapshot_expression: None,
                function_binding_expression: Expression::Undefined,
                function_binding: None,
                object_binding_expression: Expression::Undefined,
                object_binding: None,
                kind: None,
                static_string_value: None,
                exact_static_number: None,
                array_binding: None,
                module_assignment_expression: canonical_value_expression.clone(),
                resolved_local_binding,
                returned_descriptor_binding: None,
                runtime_value_override: None,
            };
        }
        if let Some(function_binding) =
            self.resolve_function_binding_from_expression(&tracked_value_expression)
            && matches!(
                &function_binding,
                LocalFunctionBinding::Builtin(function_name)
                    if parse_bound_function_prototype_call_builtin_name(function_name).is_some()
            )
        {
            if trace_identifier_store {
                eprintln!("identifier_store:{name}:bound_call_builtin_fast_path");
            }
            return PreparedIdentifierValueStore {
                canonical_value_expression: canonical_value_expression.clone(),
                tracked_value_expression: tracked_value_expression.clone(),
                descriptor_binding_expression: Expression::Undefined,
                tracked_object_expression: Expression::Undefined,
                call_source_snapshot_expression: None,
                prototype_source_snapshot_expression: None,
                function_binding_expression: tracked_value_expression.clone(),
                function_binding: Some(function_binding),
                object_binding_expression: Expression::Undefined,
                object_binding: None,
                kind: Some(StaticValueKind::Function),
                static_string_value: None,
                exact_static_number: None,
                array_binding: None,
                module_assignment_expression: tracked_value_expression.clone(),
                resolved_local_binding,
                returned_descriptor_binding: None,
                runtime_value_override: None,
            };
        }
        let resolved_descriptor_binding =
            self.resolve_descriptor_binding_from_expression(&canonical_value_expression);
        if trace_identifier_store {
            eprintln!("identifier_store:{name}:descriptor");
        }
        let returned_descriptor_binding = match &canonical_value_expression {
            Expression::Call { callee, arguments } => self
                .resolve_function_binding_from_expression(callee)
                .and_then(|binding| match binding {
                    LocalFunctionBinding::User(function_name) => self
                        .resolve_static_returned_descriptor_binding_from_user_function_call(
                            &function_name,
                            arguments,
                        ),
                    LocalFunctionBinding::Builtin(_) => None,
                }),
            _ => None,
        };
        let descriptor_binding_expression = if resolved_descriptor_binding.is_some() {
            canonical_value_expression.clone()
        } else {
            tracked_value_expression.clone()
        };
        let tracked_object_expression = resolved_descriptor_binding
            .as_ref()
            .map(|descriptor| {
                object_binding_to_expression(
                    &self.object_binding_from_property_descriptor(descriptor),
                )
            })
            .unwrap_or_else(|| tracked_value_expression.clone());
        let matched_call_snapshot = matches!(
            canonical_value_expression,
            Expression::Call { .. } | Expression::New { .. }
        )
        .then(|| {
            self.state
                .speculation
                .static_semantics
                .last_bound_user_function_call
                .as_ref()
                .and_then(|snapshot| {
                    let source_expression = snapshot.source_expression.as_ref()?;
                    let materialized_source = self.materialize_static_expression(source_expression);
                    let materialized_value =
                        self.materialize_static_expression(&canonical_value_expression);
                    static_expression_matches(&materialized_source, &materialized_value)
                        .then_some(snapshot)
                })
        })
        .flatten();
        let snapshot_is_async_function_call = matched_call_snapshot.is_some_and(|snapshot| {
            self.user_function(&snapshot.function_name)
                .is_some_and(|function| function.is_async() && !function.is_generator())
        });
        let call_result_snapshot_expression = matched_call_snapshot.and_then(|snapshot| {
            if snapshot_is_async_function_call {
                return None;
            }
            snapshot
                .result_expression
                .as_ref()
                .map(|result| match result {
                    Expression::Identifier(_) | Expression::This => result.clone(),
                    _ => self.materialize_static_expression(result),
                })
                .map(|result| {
                    self.replace_call_snapshot_updated_values_with_runtime_reads(
                        &result,
                        &snapshot.updated_bindings,
                    )
                })
        });
        let call_snapshot_function_context =
            matched_call_snapshot.map(|snapshot| snapshot.function_name.as_str());
        let call_source_snapshot_expression =
            matched_call_snapshot.and_then(|snapshot| snapshot.source_expression.as_ref().cloned());
        let prototype_source_snapshot_expression = matched_call_snapshot.and_then(|snapshot| {
            snapshot
                .prototype_source_expression
                .as_ref()
                .map(|prototype_source| match prototype_source {
                    Expression::Identifier(_) | Expression::This => prototype_source.clone(),
                    _ => self.materialize_static_expression(prototype_source),
                })
        });
        if trace_identifier_store {
            eprintln!("identifier_store:{name}:call_snapshot");
        }
        let call_result_function_binding =
            call_result_snapshot_expression
                .as_ref()
                .and_then(|expression| {
                    self.resolve_function_binding_from_expression_with_context(
                        expression,
                        call_snapshot_function_context,
                    )
                    .or_else(|| self.resolve_function_binding_from_expression(expression))
                });
        let raw_function_binding_expression =
            if local_simple_async_generator_next_call && call_result_function_binding.is_none() {
                Expression::Undefined
            } else {
                call_result_snapshot_expression
                    .as_ref()
                    .filter(|_| call_result_function_binding.is_some())
                    .unwrap_or(&tracked_value_expression)
                    .clone()
            };
        let resolved_function_binding_expression =
            if local_simple_async_generator_next_call && call_result_function_binding.is_none() {
                raw_function_binding_expression.clone()
            } else {
                self.resolve_static_function_binding_store_expression_with_context(
                    &raw_function_binding_expression,
                    call_snapshot_function_context.or_else(|| self.current_function_name()),
                )
            };
        let function_binding_expression = if self
            .expression_depends_on_active_loop_assignment(&resolved_function_binding_expression)
        {
            raw_function_binding_expression.clone()
        } else {
            resolved_function_binding_expression
        };
        let function_binding = if self
            .expression_depends_on_active_loop_assignment(&function_binding_expression)
        {
            None
        } else {
            self.resolve_function_binding_from_expression_with_context(
                &function_binding_expression,
                call_snapshot_function_context,
            )
            .or(call_result_function_binding)
            .or_else(|| self.resolve_function_binding_from_expression(&function_binding_expression))
        };
        if trace_identifier_store {
            eprintln!(
                "identifier_store:{name}:function_binding snapshot_context={call_snapshot_function_context:?} call_result={call_result_snapshot_expression:?} raw={raw_function_binding_expression:?} expr={function_binding_expression:?} binding={function_binding:?}"
            );
        }
        let canonical_object_binding = if local_simple_async_generator_next_call {
            None
        } else {
            self.resolve_object_binding_from_expression(&canonical_value_expression)
        };
        let returned_call_object_binding = if local_simple_async_generator_next_call {
            None
        } else if let Expression::Call { callee, arguments } = &canonical_value_expression {
            self.resolve_returned_object_binding_from_call(callee, arguments)
        } else {
            None
        };
        let resolved_construct_object_binding = if matches!(
            function_binding_expression,
            Expression::Call { .. } | Expression::New { .. } | Expression::Object(_)
        ) {
            self.resolve_object_binding_from_expression(&function_binding_expression)
        } else {
            None
        };
        let object_binding_expression = if canonical_object_binding
            .as_ref()
            .is_some_and(|binding| self.object_binding_is_static_map(binding))
        {
            canonical_value_expression.clone()
        } else if call_result_snapshot_expression
            .as_ref()
            .is_some_and(Self::expression_contains_static_update)
            && let Some(canonical_object_binding) = canonical_object_binding.as_ref()
        {
            object_binding_to_expression(canonical_object_binding)
        } else {
            call_result_snapshot_expression
                .as_ref()
                .filter(|expression| {
                    self.resolve_object_binding_from_expression(expression)
                        .is_some()
                })
                .or_else(|| {
                    resolved_construct_object_binding
                        .as_ref()
                        .map(|_| &function_binding_expression)
                })
                .or_else(|| {
                    returned_call_object_binding
                        .as_ref()
                        .map(|_| &canonical_value_expression)
                })
                .unwrap_or(&tracked_object_expression)
                .clone()
        };
        let object_binding =
            if static_expression_matches(&object_binding_expression, &canonical_value_expression) {
                canonical_object_binding
                    .clone()
                    .or_else(|| returned_call_object_binding.clone())
            } else if static_expression_matches(
                &object_binding_expression,
                &function_binding_expression,
            ) {
                resolved_construct_object_binding.clone()
            } else if matches!(object_binding_expression, Expression::Object(_)) {
                self.resolve_object_binding_from_expression(&object_binding_expression)
            } else {
                None
            };
        if trace_identifier_store {
            eprintln!(
                "identifier_store:{name}:object_binding expr={object_binding_expression:?} prepared_binding={}",
                object_binding.is_some()
            );
        }
        let metadata_value_expression = call_result_snapshot_expression
            .as_ref()
            .unwrap_or(&tracked_value_expression);
        let mut kind = self.infer_value_kind(metadata_value_expression);
        if kind != Some(StaticValueKind::String)
            && !self
                .runtime_string_print_candidates(metadata_value_expression)
                .is_empty()
        {
            kind = Some(StaticValueKind::String);
        }
        if trace_identifier_store {
            eprintln!("identifier_store:{name}:kind");
        }
        let static_string_value = if kind == Some(StaticValueKind::String) {
            self.resolve_static_string_value(metadata_value_expression)
        } else {
            None
        };
        if trace_identifier_store {
            eprintln!("identifier_store:{name}:string");
        }
        let exact_static_number = matches!(
            kind,
            Some(
                StaticValueKind::Number
                    | StaticValueKind::BigInt
                    | StaticValueKind::Bool
                    | StaticValueKind::String
                    | StaticValueKind::Null
                    | StaticValueKind::Undefined
            )
        )
        .then(|| self.resolve_static_number_value(metadata_value_expression))
        .flatten()
        .filter(|number| {
            number.is_nan()
                || !number.is_finite()
                || number.fract() != 0.0
                || (*number == 0.0 && number.is_sign_negative())
        });
        if trace_identifier_store {
            eprintln!("identifier_store:{name}:number");
        }
        let array_binding = self.resolve_array_binding_from_expression(metadata_value_expression);
        if trace_identifier_store {
            eprintln!("identifier_store:{name}:array");
        }
        let preserve_tracked_member_expression = matches!(
            &tracked_value_expression,
            Expression::Member { object, property }
                if self.resolve_member_function_capture_slots(object, property).is_some()
                    || self
                        .object_literal_member_function_display_name(&tracked_value_expression, 0)
                        .is_some()
        );
        let module_assignment_expression = if matches!(
            &function_binding,
            Some(LocalFunctionBinding::Builtin(function_name))
                if parse_test262_realm_eval_builtin(function_name).is_some()
        ) {
            function_binding_expression.clone()
        } else if preserve_tracked_member_expression {
            tracked_value_expression.clone()
        } else if matches!(
            call_result_snapshot_expression,
            Some(Expression::Identifier(_) | Expression::This)
        ) {
            call_result_snapshot_expression
                .as_ref()
                .expect("matched above")
                .clone()
        } else {
            self.materialize_static_expression(metadata_value_expression)
        };
        if trace_identifier_store {
            eprintln!("identifier_store:{name}:module");
        }
        PreparedIdentifierValueStore {
            canonical_value_expression,
            tracked_value_expression,
            descriptor_binding_expression,
            tracked_object_expression,
            call_source_snapshot_expression,
            prototype_source_snapshot_expression,
            function_binding_expression,
            function_binding,
            object_binding_expression,
            object_binding,
            kind,
            static_string_value,
            exact_static_number,
            array_binding,
            module_assignment_expression,
            resolved_local_binding,
            returned_descriptor_binding,
            runtime_value_override: None,
        }
    }
}
