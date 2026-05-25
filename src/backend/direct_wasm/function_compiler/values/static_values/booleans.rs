use super::*;

#[path = "booleans/builtin_calls.rs"]
mod builtin_calls;
#[path = "booleans/comparisons.rs"]
mod comparisons;
#[path = "booleans/logical_ops.rs"]
mod logical_ops;

impl<'a> FunctionCompiler<'a> {
    fn resolve_static_delete_expression_result(&self, expression: &Expression) -> Option<bool> {
        match expression {
            Expression::Identifier(name)
                if self.resolve_current_local_binding(name).is_none()
                    && self.backend.global_binding_index(name).is_none()
                    && self.backend.implicit_global_binding(name).is_some() =>
            {
                Some(true)
            }
            Expression::Identifier(name) => Some(!self.is_identifier_bound(name)),
            Expression::Member { object, property } => {
                let resolved_property = self
                    .resolve_property_key_expression(property)
                    .or_else(|| {
                        self.resolve_static_string_value(property)
                            .map(Expression::String)
                    })
                    .unwrap_or_else(|| self.materialize_static_expression(property));
                if self.is_direct_arguments_object(object)
                    && let Some(index) = argument_index_from_expression(&resolved_property)
                {
                    return Some(
                        self.state
                            .parameters
                            .arguments_slots
                            .get(&index)
                            .is_none_or(|slot| slot.state.configurable),
                    );
                }
                if matches!(
                    resolved_property,
                    Expression::String(ref property_name) if property_name == "length"
                ) && self.resolve_array_binding_from_expression(object).is_some()
                {
                    return Some(false);
                }
                if let (Expression::Identifier(object_name), Expression::String(property_name)) = (
                    self.materialize_static_expression(object),
                    resolved_property.clone(),
                ) && self.is_unshadowed_builtin_identifier(&object_name)
                    && builtin_member_delete_returns_false(&object_name, &property_name)
                {
                    return Some(false);
                }
                let materialized_property =
                    self.canonical_object_property_expression(&resolved_property);
                self.resolve_object_binding_from_expression(object)
                    .and_then(|object_binding| {
                        object_binding_lookup_descriptor(&object_binding, &materialized_property)
                            .map(|descriptor| descriptor.configurable)
                    })
                    .or(Some(true))
            }
            _ => Some(true),
        }
    }

    fn boolean_expression_reads_runtime_nonlocal_binding(&self, expression: &Expression) -> bool {
        if self.current_function_name().is_none() {
            return false;
        }

        let mut referenced_names = HashSet::new();
        collect_referenced_binding_names_from_expression(expression, &mut referenced_names);
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

    pub(in crate::backend::direct_wasm) fn resolve_static_boolean_expression(
        &self,
        expression: &Expression,
    ) -> Option<bool> {
        if self.boolean_expression_reads_runtime_nonlocal_binding(expression) {
            return None;
        }

        let materialized = self.materialize_static_expression(expression);
        match materialized {
            Expression::Bool(value) => Some(value),
            Expression::Null | Expression::Undefined => Some(false),
            Expression::String(text) => Some(!text.is_empty()),
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                let branch = if self.resolve_static_if_condition_value(&condition)? {
                    &then_expression
                } else {
                    &else_expression
                };
                self.resolve_static_boolean_expression(branch)
            }
            Expression::Array(_)
            | Expression::Object(_)
            | Expression::New { .. }
            | Expression::This => Some(true),
            Expression::Identifier(name) => match name.as_str() {
                "undefined" => Some(false),
                "NaN" if self.is_unshadowed_builtin_identifier(name.as_str()) => Some(false),
                "Infinity" if self.is_unshadowed_builtin_identifier(name.as_str()) => Some(true),
                _ => {
                    let identifier = Expression::Identifier(name.clone());
                    if self
                        .resolve_object_binding_from_expression(&identifier)
                        .is_some()
                        || self
                            .resolve_array_binding_from_expression(&identifier)
                            .is_some()
                        || self
                            .resolve_arguments_binding_from_expression(&identifier)
                            .is_some()
                        || self
                            .resolve_proxy_binding_from_expression(&identifier)
                            .is_some()
                        || self
                            .resolve_function_binding_from_expression(&identifier)
                            .is_some()
                    {
                        Some(true)
                    } else {
                        match self.lookup_identifier_kind(&name) {
                            Some(StaticValueKind::Object)
                            | Some(StaticValueKind::Function)
                            | Some(StaticValueKind::Symbol) => Some(true),
                            Some(StaticValueKind::String) => self
                                .resolve_static_string_value(&identifier)
                                .map(|text| !text.is_empty()),
                            Some(StaticValueKind::Number) => self
                                .resolve_static_number_value(&identifier)
                                .map(|number| number != 0.0 && !number.is_nan()),
                            Some(StaticValueKind::Bool) => self
                                .resolve_bound_alias_expression(&identifier)
                                .or_else(|| self.resolve_global_value_expression(&identifier))
                                .and_then(|value| self.resolve_static_boolean_expression(&value)),
                            Some(StaticValueKind::Null) | Some(StaticValueKind::Undefined) => {
                                Some(false)
                            }
                            _ => None,
                        }
                    }
                }
            },
            Expression::Unary {
                op: UnaryOp::Not,
                expression,
            } => Some(!self.resolve_static_boolean_expression(&expression)?),
            Expression::Unary {
                op: UnaryOp::Delete,
                expression,
            } => self.resolve_static_delete_expression_result(&expression),
            Expression::Binary { op, left, right } => match op {
                BinaryOp::LogicalAnd | BinaryOp::LogicalOr | BinaryOp::NullishCoalescing => self
                    .resolve_static_logical_result_expression(op, &left, &right)
                    .and_then(|value| self.resolve_static_boolean_expression(&value)),
                BinaryOp::Equal
                | BinaryOp::LooseEqual
                | BinaryOp::NotEqual
                | BinaryOp::LooseNotEqual
                | BinaryOp::LessThan
                | BinaryOp::LessThanOrEqual
                | BinaryOp::GreaterThan
                | BinaryOp::GreaterThanOrEqual => {
                    self.resolve_static_binary_boolean_result(&op, &left, &right)
                }
                BinaryOp::In => self.resolve_static_in_expression_result(&left, &right),
                _ => None,
            },
            Expression::Unary {
                op: UnaryOp::Plus,
                expression,
            }
            | Expression::Unary {
                op: UnaryOp::Negate,
                expression,
            } => {
                let number = self.resolve_static_number_value(&expression)?;
                Some(number != 0.0 && !number.is_nan())
            }
            Expression::Number(value) => Some(value != 0.0 && !value.is_nan()),
            Expression::Member { object, property } => {
                if let Expression::Identifier(object_name) = object.as_ref()
                    && self.is_unshadowed_builtin_identifier(object_name)
                    && let Expression::String(property_name) = property.as_ref()
                    && let Some(number) = builtin_member_number_value(object_name, property_name)
                {
                    Some(number != 0.0 && !number.is_nan())
                } else {
                    None
                }
            }
            Expression::Call { .. } => self
                .resolve_static_has_own_property_call_result(expression)
                .or_else(|| self.resolve_static_reflect_has_call_result(expression))
                .or_else(|| self.resolve_static_private_in_predicate_call_result(expression))
                .or_else(|| self.resolve_static_is_nan_call_result(expression))
                .or_else(|| self.resolve_static_object_is_call_result(expression))
                .or_else(|| self.resolve_static_array_is_array_call_result(expression)),
            Expression::Assign { value, .. } => self.resolve_static_boolean_expression(&value),
            _ => None,
        }
    }

    pub(in crate::backend::direct_wasm) fn resolve_static_in_expression_result(
        &self,
        left: &Expression,
        right: &Expression,
    ) -> Option<bool> {
        if let Some(array_binding) = self.resolve_array_binding_from_expression(right) {
            if matches!(left, Expression::String(property_name) if property_name == "length") {
                return Some(true);
            }
            let materialized_left = self.materialize_static_expression(left);
            if let Some(index) = argument_index_from_expression(left)
                .or_else(|| argument_index_from_expression(&materialized_left))
            {
                return Some(
                    array_binding
                        .values
                        .get(index as usize)
                        .is_some_and(|value| value.is_some()),
                );
            }
            if let Expression::Member { object, .. } = left
                && let Some(key_binding) = self.resolve_array_binding_from_expression(object)
                && !key_binding.values.is_empty()
                && key_binding.values.iter().all(|value| {
                    matches!(
                        value,
                        Some(Expression::String(property_name))
                            if argument_index_from_expression(&Expression::String(property_name.clone()))
                                .is_some_and(|index| {
                                    array_binding
                                        .values
                                        .get(index as usize)
                                        .is_some_and(|value| value.is_some())
                                })
                    )
                })
            {
                return Some(true);
            }
        }

        if let Some(object_name) =
            self.static_builtin_object_name_for_in_result_after_left(left, right)
            && let Some(property_name) = self.static_property_name_for_in_result(left)
        {
            return Some(Self::static_builtin_object_has_in_property_for_in_result(
                &object_name,
                &property_name,
            ));
        }

        let materialized_right = self.materialize_static_expression(right);
        let object_binding = if static_expression_matches(&materialized_right, right) {
            self.resolve_object_binding_from_expression(right)
        } else {
            self.resolve_object_binding_from_expression(&materialized_right)
                .or_else(|| self.resolve_object_binding_from_expression(right))
        };
        if let Some(object_binding) = object_binding {
            if let Expression::Member { object, .. } = left
                && let Some(key_binding) = self.resolve_array_binding_from_expression(object)
                && !key_binding.values.is_empty()
                && key_binding.values.iter().all(|value| {
                    matches!(
                        value,
                        Some(Expression::String(property_name))
                            if object_binding_has_property(
                                &object_binding,
                                &Expression::String(property_name.clone())
                            ) || self
                                .resolve_object_binding_has_property_with_inherited(
                                    right,
                                    &object_binding,
                                    &Expression::String(property_name.clone()),
                                )
                    )
                })
            {
                return Some(true);
            }
            let materialized_left = self.materialize_static_expression(left);
            let shadow_object = if static_expression_matches(&materialized_right, right) {
                right
            } else {
                &materialized_right
            };
            if self.runtime_object_property_shadow_deletion_may_affect_property(
                shadow_object,
                &materialized_left,
            ) {
                return None;
            }
            return Some(self.resolve_object_binding_has_property_with_inherited(
                right,
                &object_binding,
                &materialized_left,
            ));
        }

        None
    }

    pub(in crate::backend::direct_wasm) fn resolve_static_private_in_predicate_call_result(
        &self,
        expression: &Expression,
    ) -> Option<bool> {
        let Expression::Call { callee, arguments } = expression else {
            return None;
        };
        let Expression::Member { object, property } = callee.as_ref() else {
            return None;
        };
        let [CallArgument::Expression(argument)] = arguments.as_slice() else {
            return None;
        };
        let LocalFunctionBinding::User(function_name) =
            self.resolve_private_in_predicate_member_function_binding(object, property)?
        else {
            return None;
        };
        let (private_property, parameter_name) =
            self.static_private_in_predicate_return(&function_name)?;
        if !self.private_in_predicate_argument_matches_parameter(&parameter_name, argument) {
            return None;
        }
        self.resolve_static_private_property_presence_for_expression(&private_property, argument)
    }

    fn resolve_private_in_predicate_member_function_binding(
        &self,
        object: &Expression,
        property: &Expression,
    ) -> Option<LocalFunctionBinding> {
        self.resolve_member_function_binding(object, property)
            .or_else(|| {
                self.resolve_member_function_binding_from_deferred_class_assignment(
                    object, property,
                )
            })
    }

    fn resolve_member_function_binding_from_deferred_class_assignment(
        &self,
        object: &Expression,
        property: &Expression,
    ) -> Option<LocalFunctionBinding> {
        let Expression::Identifier(object_name) = object else {
            return None;
        };
        let property_name = self.static_property_name_for_in_result(property)?;
        let class_init_name = self.deferred_class_init_assigned_to_identifier(object_name)?;
        let method_name =
            self.deferred_class_init_static_method_name(&class_init_name, &property_name)?;
        Some(LocalFunctionBinding::User(method_name))
    }

    fn deferred_class_init_assigned_to_identifier(&self, target_name: &str) -> Option<String> {
        self.user_functions()
            .iter()
            .filter_map(|function| self.resolve_registered_function_declaration(&function.name))
            .find_map(|function| {
                function.body.iter().find_map(|statement| match statement {
                    Statement::Assign {
                        name,
                        value: Expression::Call { callee, arguments },
                    } if name == target_name
                        && arguments.is_empty()
                        && matches!(callee.as_ref(), Expression::Identifier(_)) =>
                    {
                        let Expression::Identifier(class_init_name) = callee.as_ref() else {
                            return None;
                        };
                        Some(class_init_name.clone())
                    }
                    _ => None,
                })
            })
    }

    fn deferred_class_init_return_identifier(&self, class_init_name: &str) -> Option<String> {
        self.resolve_registered_function_declaration(class_init_name)?
            .body
            .iter()
            .find_map(|statement| match statement {
                Statement::Return(Expression::Identifier(class_name)) => Some(class_name.clone()),
                _ => None,
            })
    }

    fn deferred_class_init_static_method_name(
        &self,
        class_init_name: &str,
        property_name: &str,
    ) -> Option<String> {
        let class_name = self.deferred_class_init_return_identifier(class_init_name)?;
        self.resolve_registered_function_declaration(class_init_name)?
            .body
            .iter()
            .find_map(|statement| {
                deferred_define_property_value(statement, &class_name, property_name).and_then(
                    |value| match value {
                        Expression::Identifier(function_name)
                            if self
                                .resolve_registered_function_declaration(function_name)
                                .is_some() =>
                        {
                            Some(function_name.clone())
                        }
                        _ => None,
                    },
                )
            })
    }

    fn static_private_in_predicate_return(&self, function_name: &str) -> Option<(String, String)> {
        let function = self.resolve_registered_function_declaration(function_name)?;
        let parameter_name = function.params.first()?.name.clone();
        let [
            Statement::Return(Expression::Binary {
                op: BinaryOp::In,
                left,
                right,
            }),
        ] = function.body.as_slice()
        else {
            return None;
        };
        let Expression::String(private_property) = left.as_ref() else {
            return None;
        };
        if !private_property.starts_with("__ayy$private$") {
            return None;
        }
        let Expression::Identifier(returned_name) = right.as_ref() else {
            return None;
        };
        (returned_name == &parameter_name).then(|| (private_property.clone(), parameter_name))
    }

    fn private_in_predicate_argument_matches_parameter(
        &self,
        parameter_name: &str,
        argument: &Expression,
    ) -> bool {
        let materialized = self.materialize_static_expression(argument);
        !matches!(argument, Expression::Identifier(name) if name == parameter_name)
            || !static_expression_matches(&materialized, argument)
    }

    fn resolve_static_private_property_presence_for_expression(
        &self,
        private_property: &str,
        expression: &Expression,
    ) -> Option<bool> {
        let private_property_expression = Expression::String(private_property.to_string());
        let materialized = self.materialize_static_expression(expression);
        let object_binding = self
            .resolve_object_binding_from_expression(expression)
            .or_else(|| {
                (!static_expression_matches(&materialized, expression))
                    .then(|| self.resolve_object_binding_from_expression(&materialized))?
            });
        if let Some(object_binding) = object_binding {
            return Some(self.resolve_object_binding_has_property_with_inherited(
                expression,
                &object_binding,
                &private_property_expression,
            ));
        }

        let private_owner = private_property_declaring_class_name(private_property)?;
        if let Some(constructed_owner) =
            self.resolve_constructed_static_private_brand_owner(expression)
        {
            return Some(constructed_owner == private_owner);
        }
        if !static_expression_matches(&materialized, expression)
            && let Some(constructed_owner) =
                self.resolve_constructed_static_private_brand_owner(&materialized)
        {
            return Some(constructed_owner == private_owner);
        }
        None
    }

    fn resolve_constructed_static_private_brand_owner(
        &self,
        expression: &Expression,
    ) -> Option<String> {
        match expression {
            Expression::New { callee, .. } => {
                if let Some(LocalFunctionBinding::User(function_name)) =
                    self.resolve_function_binding_from_expression(callee)
                {
                    return self.static_private_brand_owner_for_constructor(&function_name);
                }
                let Expression::Identifier(name) = callee.as_ref() else {
                    return None;
                };
                self.deferred_class_init_assigned_to_identifier(name)
                    .and_then(|class_init_name| {
                        self.deferred_class_init_return_identifier(&class_init_name)
                    })
            }
            Expression::Identifier(name) => {
                let local_value = self
                    .resolve_current_local_binding(name)
                    .and_then(|(resolved_name, _)| {
                        self.state
                            .speculation
                            .static_semantics
                            .local_value_binding(&resolved_name)
                    })
                    .or_else(|| {
                        self.state
                            .speculation
                            .static_semantics
                            .local_value_binding(name)
                    })
                    .cloned();
                if let Some(value) = local_value {
                    return self.resolve_constructed_static_private_brand_owner(&value);
                }
                self.global_value_binding(name)
                    .cloned()
                    .and_then(|value| self.resolve_constructed_static_private_brand_owner(&value))
            }
            Expression::Assign { value, .. } => {
                self.resolve_constructed_static_private_brand_owner(value)
            }
            Expression::Sequence(expressions) => expressions
                .last()
                .and_then(|last| self.resolve_constructed_static_private_brand_owner(last)),
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                let branch = if self.resolve_static_if_condition_value(condition)? {
                    then_expression
                } else {
                    else_expression
                };
                self.resolve_constructed_static_private_brand_owner(branch)
            }
            _ => None,
        }
    }

    fn static_private_brand_owner_for_constructor(&self, function_name: &str) -> Option<String> {
        let function = self.resolve_registered_function_declaration(function_name);
        function
            .and_then(|function| function.self_binding.clone())
            .or_else(|| function.and_then(|function| function.top_level_binding.clone()))
            .or_else(|| {
                function_name
                    .rsplit_once("__name_")
                    .map(|(_, class_name)| class_name.to_string())
            })
    }

    fn static_property_name_for_in_result(&self, property: &Expression) -> Option<String> {
        if let Expression::Sequence(expressions) = property
            && let Some(last) = expressions.last()
        {
            return self.static_property_name_for_in_result(last);
        }
        let resolved = self.resolve_property_key_expression(property).or_else(|| {
            self.resolve_bound_alias_expression(property)
                .filter(|resolved| !static_expression_matches(resolved, property))
        });
        for candidate in [resolved.as_ref(), Some(property)] {
            if let Some(property_name) = candidate.and_then(static_property_name_from_expression) {
                return Some(property_name);
            }
        }
        let materialized = self.materialize_static_expression(property);
        static_property_name_from_expression(&materialized)
    }

    fn static_builtin_object_name_for_in_result(&self, object: &Expression) -> Option<String> {
        if let Expression::Sequence(expressions) = object
            && let Some(last) = expressions.last()
        {
            return self.static_builtin_object_name_for_in_result(last);
        }
        if let Expression::Identifier(name) = object
            && name == "Number"
            && self.is_unshadowed_builtin_identifier(name)
        {
            return Some(name.clone());
        }
        if let Expression::Identifier(name) = object
            && let Some(Expression::Identifier(alias)) = self
                .state
                .speculation
                .static_semantics
                .local_value_binding(name)
                .or_else(|| self.global_value_binding(name))
            && alias == "Number"
            && self.is_unshadowed_builtin_identifier(alias)
        {
            return Some(alias.clone());
        }

        let resolved = self
            .resolve_bound_alias_expression(object)
            .filter(|resolved| !static_expression_matches(resolved, object));
        if let Some(Expression::Identifier(name)) = resolved.as_ref()
            && name == "Number"
            && self.is_unshadowed_builtin_identifier(name)
        {
            return Some(name.clone());
        }

        let materialized = self.materialize_static_expression(object);
        if let Expression::Identifier(name) = materialized
            && name == "Number"
            && self.is_unshadowed_builtin_identifier(&name)
        {
            return Some(name);
        }

        None
    }

    fn last_assignment_value_to_identifier_for_in_result<'b>(
        expression: &'b Expression,
        target_name: &str,
    ) -> Option<&'b Expression> {
        match expression {
            Expression::Assign { name, value } if name == target_name => Some(value),
            Expression::Sequence(expressions) => {
                expressions.iter().fold(None, |last, expression| {
                    Self::last_assignment_value_to_identifier_for_in_result(expression, target_name)
                        .or(last)
                })
            }
            _ => None,
        }
    }

    fn static_builtin_object_name_for_in_result_after_left(
        &self,
        left: &Expression,
        right: &Expression,
    ) -> Option<String> {
        self.static_builtin_object_name_for_in_result(right)
            .or_else(|| {
                let Expression::Identifier(name) = right else {
                    return None;
                };
                let assigned_value =
                    Self::last_assignment_value_to_identifier_for_in_result(left, name)?;
                self.static_builtin_object_name_for_in_result(assigned_value)
            })
    }

    fn static_builtin_object_has_in_property_for_in_result(
        object_name: &str,
        property_name: &str,
    ) -> bool {
        match object_name {
            "Number" => matches!(
                property_name,
                "MAX_VALUE" | "MIN_VALUE" | "NaN" | "POSITIVE_INFINITY" | "NEGATIVE_INFINITY"
            ),
            _ => false,
        }
    }
}

fn private_property_declaring_class_name(property_name: &str) -> Option<String> {
    let remainder = property_name.strip_prefix("__ayy$private$")?;
    let (class_name, _) = remainder.rsplit_once('$')?;
    Some(class_name.to_string())
}

fn deferred_define_property_value<'a>(
    statement: &'a Statement,
    object_name: &str,
    property_name: &str,
) -> Option<&'a Expression> {
    let Statement::Expression(Expression::Call { callee, arguments }) = statement else {
        return None;
    };
    let Expression::Member { object, property } = callee.as_ref() else {
        return None;
    };
    if !matches!(object.as_ref(), Expression::Identifier(name) if name == "Object")
        || !matches!(property.as_ref(), Expression::String(name) if name == "defineProperty")
    {
        return None;
    }
    let [
        CallArgument::Expression(Expression::Identifier(target_object)),
        CallArgument::Expression(Expression::String(target_property)),
        CallArgument::Expression(Expression::Object(descriptor_entries)),
    ] = arguments.as_slice()
    else {
        return None;
    };
    if target_object != object_name || target_property != property_name {
        return None;
    }
    descriptor_entries.iter().find_map(|entry| match entry {
        ObjectEntry::Data {
            key: Expression::String(key),
            value,
        } if key == "value" => Some(value),
        _ => None,
    })
}
