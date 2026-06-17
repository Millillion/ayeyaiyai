use super::*;

const LOCAL_STATIC_VALUE_NODE_LIMIT: usize = 32;

fn expression_is_promise_resolve_undefined_call(expression: &Expression) -> bool {
    let Expression::Call { callee, arguments } = expression else {
        return false;
    };
    let Expression::Member { object, property } = callee.as_ref() else {
        return false;
    };
    matches!(object.as_ref(), Expression::Identifier(name) if name == "Promise")
        && matches!(property.as_ref(), Expression::String(name) if name == "resolve")
        && matches!(
            arguments.as_slice(),
            [] | [CallArgument::Expression(Expression::Undefined)]
        )
}

fn expression_is_static_promise_with_resolvers_record(expression: &Expression) -> bool {
    let Expression::Object(entries) = expression else {
        return false;
    };
    let mut has_promise = false;
    let mut has_resolve = false;
    let mut has_reject = false;
    for entry in entries {
        let ObjectEntry::Data {
            key: Expression::String(key),
            value,
        } = entry
        else {
            continue;
        };
        match key.as_str() {
            "promise" => has_promise = expression_is_promise_resolve_undefined_call(value),
            "resolve" => {
                has_resolve = matches!(
                    value,
                    Expression::Identifier(name)
                        if name == "__ayy_promise_with_resolvers_resolve"
                );
            }
            "reject" => {
                has_reject = matches!(
                    value,
                    Expression::Identifier(name)
                        if name == "__ayy_promise_with_resolvers_reject"
                );
            }
            _ => {}
        }
    }
    has_promise && has_resolve && has_reject
}

fn static_expression_exceeds_node_limit(expression: &Expression, limit: usize) -> bool {
    fn visit(expression: &Expression, remaining: &mut usize) -> bool {
        if *remaining == 0 {
            return true;
        }
        *remaining -= 1;
        match expression {
            Expression::Array(elements) => elements.iter().any(|element| match element {
                ArrayElement::Expression(value) | ArrayElement::Spread(value) => {
                    visit(value, remaining)
                }
            }),
            Expression::Object(entries) => entries.iter().any(|entry| match entry {
                ObjectEntry::Data { key, value } => {
                    visit(key, remaining) || visit(value, remaining)
                }
                ObjectEntry::Getter { key, getter } => {
                    visit(key, remaining) || visit(getter, remaining)
                }
                ObjectEntry::Setter { key, setter } => {
                    visit(key, remaining) || visit(setter, remaining)
                }
                ObjectEntry::Spread(value) => visit(value, remaining),
            }),
            Expression::Member { object, property } => {
                visit(object, remaining) || visit(property, remaining)
            }
            Expression::SuperMember { property } => visit(property, remaining),
            Expression::Assign { value, .. }
            | Expression::AssignSuperMember { value, .. }
            | Expression::Await(value)
            | Expression::EnumerateKeys(value)
            | Expression::GetIterator(value)
            | Expression::IteratorClose(value)
            | Expression::Unary {
                expression: value, ..
            } => visit(value, remaining),
            Expression::AssignMember {
                object,
                property,
                value,
            } => visit(object, remaining) || visit(property, remaining) || visit(value, remaining),
            Expression::Binary { left, right, .. } => {
                visit(left, remaining) || visit(right, remaining)
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                visit(condition, remaining)
                    || visit(then_expression, remaining)
                    || visit(else_expression, remaining)
            }
            Expression::Sequence(expressions) => expressions
                .iter()
                .any(|expression| visit(expression, remaining)),
            Expression::Call { callee, arguments }
            | Expression::SuperCall { callee, arguments }
            | Expression::New { callee, arguments } => {
                visit(callee, remaining)
                    || arguments
                        .iter()
                        .any(|argument| visit(argument.expression(), remaining))
            }
            Expression::Number(_)
            | Expression::BigInt(_)
            | Expression::String(_)
            | Expression::Bool(_)
            | Expression::Null
            | Expression::Undefined
            | Expression::NewTarget
            | Expression::Identifier(_)
            | Expression::This
            | Expression::Sent
            | Expression::Update { .. } => false,
        }
    }

    let mut remaining = limit;
    visit(expression, &mut remaining)
}

fn large_function_should_preserve_exact_local_value(value: &Expression) -> bool {
    matches!(value, Expression::Identifier(_) | Expression::This)
}

impl<'a> FunctionCompiler<'a> {
    pub(in crate::backend::direct_wasm) fn update_local_descriptor_binding(
        &mut self,
        name: &str,
        value: &Expression,
    ) {
        let Some(descriptor_binding) = self.resolve_descriptor_binding_from_expression(value)
        else {
            self.state
                .speculation
                .static_semantics
                .objects
                .local_descriptor_bindings
                .remove(name);
            return;
        };
        self.state
            .speculation
            .static_semantics
            .objects
            .local_descriptor_bindings
            .insert(name.to_string(), descriptor_binding);
        self.state
            .speculation
            .static_semantics
            .set_local_kind(name, StaticValueKind::Object);
    }

    /// Self-referential descriptor values (for example `x + 1` for `x` after
    /// an unresolvable `x += 1`) are replaced with a plain self-identifier so
    /// later kind/value resolution treats the binding as runtime-valued
    /// instead of recursively re-expanding the expression.
    fn neutralize_self_referential_descriptor_value(
        name: &str,
        materialized: Expression,
    ) -> Expression {
        let mut referenced_names = HashSet::new();
        collect_referenced_binding_names_from_expression(&materialized, &mut referenced_names);
        if referenced_names.contains(name) {
            Expression::Identifier(name.to_string())
        } else {
            materialized
        }
    }

    pub(in crate::backend::direct_wasm) fn update_global_property_descriptor_value(
        &mut self,
        name: &str,
        value_expression: &Expression,
    ) {
        let materialized = self
            .backend
            .global_value_binding(name)
            .cloned()
            .unwrap_or_else(|| self.materialize_static_expression(value_expression));
        let materialized = Self::neutralize_self_referential_descriptor_value(name, materialized);
        if let Some(mut state) = self.backend.global_property_descriptor(name).cloned() {
            state.value = materialized;
            self.backend
                .upsert_global_property_descriptor(name.to_string(), state);
        }
    }

    pub(in crate::backend::direct_wasm) fn ensure_global_property_descriptor_value(
        &mut self,
        name: &str,
        value_expression: &Expression,
        configurable: bool,
    ) {
        let materialized = self
            .backend
            .global_value_binding(name)
            .cloned()
            .unwrap_or_else(|| self.materialize_static_expression(value_expression));
        let materialized = Self::neutralize_self_referential_descriptor_value(name, materialized);
        let next_state = self
            .backend
            .global_property_descriptor(name)
            .cloned()
            .map(|mut state| {
                state.value = materialized.clone();
                state
            })
            .unwrap_or(GlobalPropertyDescriptorState {
                value: materialized,
                writable: Some(true),
                enumerable: true,
                configurable,
                getter: None,
                setter: None,
                has_get: false,
                has_set: false,
            });
        self.backend
            .upsert_global_property_descriptor(name.to_string(), next_state);
    }

    pub(in crate::backend::direct_wasm) fn instantiate_eval_global_function_property_descriptor(
        &mut self,
        name: &str,
        configurable: bool,
    ) {
        let value = Expression::Identifier(name.to_string());
        let next_state = match self.backend.global_property_descriptor(name).cloned() {
            Some(mut state) if !state.configurable => {
                state.value = value;
                state
            }
            Some(_) | None => GlobalPropertyDescriptorState {
                value,
                writable: Some(true),
                enumerable: true,
                configurable,
                getter: None,
                setter: None,
                has_get: false,
                has_set: false,
            },
        };
        self.backend
            .upsert_global_property_descriptor(name.to_string(), next_state);
    }

    pub(in crate::backend::direct_wasm) fn update_local_value_binding(
        &mut self,
        name: &str,
        value: &Expression,
    ) {
        if Self::expression_contains_await_for_user_call_runtime(value) {
            self.state
                .speculation
                .static_semantics
                .clear_local_value_binding(name);
            return;
        }
        if self.current_function_exceeds_static_analysis_budget()
            && !large_function_should_preserve_exact_local_value(value)
        {
            let kind = self
                .infer_value_kind(value)
                .unwrap_or(StaticValueKind::Unknown);
            self.state
                .speculation
                .static_semantics
                .clear_local_value_binding(name);
            self.state
                .speculation
                .static_semantics
                .set_local_kind(name, kind);
            return;
        }
        let snapshot_value = self
            .state
            .speculation
            .static_semantics
            .local_value_binding(name)
            .or_else(|| self.global_value_binding(name))
            .map(|snapshot| substitute_self_referential_binding_snapshot(value, name, snapshot))
            .unwrap_or_else(|| value.clone());
        if static_expression_exceeds_node_limit(&snapshot_value, LOCAL_STATIC_VALUE_NODE_LIMIT) {
            self.state
                .speculation
                .static_semantics
                .clear_local_value_binding(name);
            return;
        }
        let mut referenced_names = HashSet::new();
        collect_referenced_binding_names_from_expression(&snapshot_value, &mut referenced_names);
        if referenced_names.contains(name) {
            self.state
                .speculation
                .static_semantics
                .clear_local_value_binding(name);
            return;
        }
        let template_object_identity_value =
            self.resolve_template_object_reference_identity_expression(&snapshot_value);
        let metadata_source_value = template_object_identity_value
            .as_ref()
            .unwrap_or(&snapshot_value);
        let module_error_identity_value = match metadata_source_value {
            Expression::Identifier(alias) if alias.starts_with("__ayy_module_error_") => {
                Some(metadata_source_value.clone())
            }
            Expression::Identifier(alias) => self
                .state
                .speculation
                .static_semantics
                .local_value_binding(alias)
                .or_else(|| self.global_value_binding(alias))
                .and_then(|value| match value {
                    Expression::Identifier(module_error)
                        if module_error.starts_with("__ayy_module_error_") =>
                    {
                        Some(value.clone())
                    }
                    _ => None,
                }),
            _ => None,
        };
        let preserve_reference_alias =
            matches!(snapshot_value, Expression::Identifier(_) | Expression::This)
                && (self
                    .resolve_object_binding_from_expression(metadata_source_value)
                    .is_some()
                    || self
                        .resolve_array_binding_from_expression(metadata_source_value)
                        .is_some()
                    || self
                        .resolve_function_binding_from_expression(metadata_source_value)
                        .is_some());
        let preserve_object_literal_member_function_alias = self
            .object_literal_member_function_display_name(metadata_source_value, 0)
            .is_some()
            && self
                .resolve_function_binding_from_expression(metadata_source_value)
                .is_some();
        let materialized_value =
            if let Some(template_object_identity_value) = template_object_identity_value {
                template_object_identity_value
            } else if let Some(module_error_identity_value) = module_error_identity_value {
                module_error_identity_value
            } else if preserve_reference_alias || preserve_object_literal_member_function_alias {
                snapshot_value.clone()
            } else if matches!(
                metadata_source_value,
                Expression::Call { callee, .. }
                    if matches!(callee.as_ref(), Expression::Identifier(name)
                        if name == "__ayyDynamicImport")
            ) {
                snapshot_value.clone()
            } else if expression_is_static_promise_with_resolvers_record(metadata_source_value) {
                snapshot_value.clone()
            } else if matches!(
                metadata_source_value,
                Expression::Call { callee, .. }
                    if matches!(callee.as_ref(), Expression::Identifier(name)
                        if name == "Symbol" && self.is_unshadowed_builtin_identifier(name))
            ) {
                snapshot_value.clone()
            } else if let Some(bigint) = self.resolve_static_bigint_value(metadata_source_value) {
                Expression::BigInt(bigint.to_string())
            } else {
                self.resolve_static_string_value(metadata_source_value)
                    .map(Expression::String)
                    .unwrap_or_else(|| self.materialize_static_expression(metadata_source_value))
            };
        self.state
            .speculation
            .static_semantics
            .set_local_value_binding(name, materialized_value);
    }
}
