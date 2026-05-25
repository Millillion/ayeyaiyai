use super::*;

impl<'a> FunctionCompiler<'a> {
    pub(in crate::backend::direct_wasm) fn resolve_static_primitive_expression_with_context(
        &self,
        expression: &Expression,
        current_function_name: Option<&str>,
    ) -> Option<Expression> {
        let materialized = self.materialize_static_expression(expression);
        if !static_expression_matches(&materialized, expression) {
            if self.expression_is_static_boxed_primitive_object(&materialized) {
                return None;
            }
            return self.resolve_static_primitive_expression_with_context(
                &materialized,
                current_function_name,
            );
        }

        match expression {
            Expression::Number(_)
            | Expression::BigInt(_)
            | Expression::String(_)
            | Expression::Bool(_)
            | Expression::Null
            | Expression::Undefined => Some(expression.clone()),
            _ if self
                .resolve_static_symbol_to_string_value_with_context(
                    expression,
                    current_function_name,
                )
                .is_some() =>
            {
                Some(expression.clone())
            }
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
                self.resolve_static_primitive_expression_with_context(branch, current_function_name)
            }
            Expression::Sequence(expressions) => expressions.last().and_then(|last| {
                self.resolve_static_primitive_expression_with_context(last, current_function_name)
            }),
            Expression::Assign { value, .. }
            | Expression::AssignMember { value, .. }
            | Expression::AssignSuperMember { value, .. } => {
                self.resolve_static_primitive_expression_with_context(value, current_function_name)
            }
            Expression::Await(value) => {
                self.resolve_static_primitive_expression_with_context(value, current_function_name)
            }
            Expression::Unary {
                op: UnaryOp::Void, ..
            } => Some(Expression::Undefined),
            Expression::Identifier(name)
                if name == "undefined" && self.is_unshadowed_builtin_identifier(name) =>
            {
                Some(Expression::Undefined)
            }
            Expression::Identifier(name)
                if name == "NaN" && self.is_unshadowed_builtin_identifier(name) =>
            {
                Some(Expression::Number(f64::NAN))
            }
            Expression::Identifier(name)
                if name == "Infinity" && self.is_unshadowed_builtin_identifier(name) =>
            {
                Some(Expression::Number(f64::INFINITY))
            }
            Expression::Unary {
                op: UnaryOp::Plus,
                expression,
            } if matches!(expression.as_ref(), Expression::Identifier(name) if name == "Infinity" && self.is_unshadowed_builtin_identifier(name)) => {
                Some(Expression::Number(f64::INFINITY))
            }
            Expression::Unary {
                op: UnaryOp::Negate,
                expression,
            } if matches!(expression.as_ref(), Expression::Identifier(name) if name == "Infinity" && self.is_unshadowed_builtin_identifier(name)) => {
                Some(Expression::Number(f64::NEG_INFINITY))
            }
            Expression::Identifier(_) => self
                .resolve_static_string_value_with_context(expression, current_function_name)
                .map(Expression::String),
            Expression::Member { object, property } => {
                if std::env::var_os("AYY_TRACE_THIS_FLOW").is_some()
                    && matches!(object.as_ref(), Expression::This)
                {
                    eprintln!(
                        "this_flow primitive_resolution fn={:?} expr={:?} runtime_dynamic_this={}",
                        current_function_name,
                        expression,
                        self.state
                            .runtime
                            .locals
                            .runtime_dynamic_bindings
                            .contains("this")
                    );
                }
                if let Some(shadow_binding_name) = self
                    .runtime_object_property_shadow_binding_name_for_expression(object, property)
                    .filter(|shadow_binding_name| {
                        self.runtime_object_property_shadow_binding_should_defer_static_resolution(
                            shadow_binding_name,
                        )
                    })
                {
                    if std::env::var_os("AYY_TRACE_RUNTIME_SHADOWS").is_some() {
                        eprintln!(
                            "runtime_shadow_primitive_defer object={object:?} property={property:?} shadow_name={shadow_binding_name}"
                        );
                    }
                    return None;
                }
                if self.expression_uses_runtime_dynamic_binding(object)
                    || self.expression_uses_runtime_dynamic_binding(property)
                {
                    if std::env::var_os("AYY_TRACE_RUNTIME_SHADOWS").is_some() {
                        eprintln!(
                            "runtime_shadow_primitive_member_dynamic object={object:?} property={property:?}"
                        );
                    }
                    return None;
                }
                let materialized_property = self.materialize_static_expression(property);
                if self.runtime_object_property_shadow_deletion_is_statically_present(
                    object,
                    &materialized_property,
                ) {
                    return Some(Expression::Undefined);
                }
                if self.runtime_object_property_shadow_deletion_may_affect_property(
                    object,
                    &materialized_property,
                ) {
                    return None;
                }
                if matches!(&materialized_property, Expression::String(name) if name == "prototype")
                    && self
                        .resolve_function_binding_from_expression(object)
                        .is_some()
                {
                    return None;
                }
                if matches!(&materialized_property, Expression::String(name) if name == "length")
                    && self
                        .resolve_typed_array_view_binding_from_expression(object)
                        .is_some()
                    && self
                        .runtime_array_length_local_for_expression(object)
                        .is_some()
                {
                    return None;
                }
                let reads_runtime_array_member = matches!(&materialized_property, Expression::String(name) if name == "length")
                    && self
                        .runtime_array_binding_name_for_expression(object)
                        .is_some()
                    || argument_index_from_expression(&materialized_property).is_some()
                        && self
                            .runtime_array_binding_name_for_expression(object)
                            .is_some();
                if reads_runtime_array_member {
                    return None;
                }
                if let Some(function_name) = self.resolve_function_name_value(object, property) {
                    if std::env::var_os("AYY_TRACE_RUNTIME_SHADOWS").is_some() {
                        eprintln!(
                            "runtime_shadow_primitive_member_function_name object={object:?} property={property:?} function_name={function_name:?}"
                        );
                    }
                    return Some(Expression::String(function_name));
                }
                if let Some(value) = self
                    .resolve_static_member_getter_value_with_context(
                        object,
                        property,
                        current_function_name,
                    )
                    .filter(|value| !static_expression_matches(value, expression))
                {
                    if std::env::var_os("AYY_TRACE_RUNTIME_SHADOWS").is_some() {
                        eprintln!(
                            "runtime_shadow_primitive_member_getter object={object:?} property={property:?} value={value:?}"
                        );
                    }
                    if self.expression_is_static_boxed_primitive_object(&value) {
                        return None;
                    }
                    return self.resolve_static_primitive_expression_with_context(
                        &value,
                        current_function_name,
                    );
                }
                if !self.function_object_has_explicit_own_property(object, property)
                    && let Some(number) = self.resolve_static_number_value(expression)
                {
                    if std::env::var_os("AYY_TRACE_RUNTIME_SHADOWS").is_some() {
                        eprintln!(
                            "runtime_shadow_primitive_member_number object={object:?} property={property:?} number={number:?}"
                        );
                    }
                    return Some(Expression::Number(number));
                }
                let materialized_object = self.materialize_static_expression(object);
                if let Some(value) =
                    self.resolve_primitive_prototype_property_value(object, &materialized_property)
                {
                    return self.resolve_static_primitive_expression_with_context(
                        &value,
                        current_function_name,
                    );
                }
                let object_binding =
                    self.resolve_object_binding_from_expression(object)
                        .or_else(|| {
                            (!static_expression_matches(&materialized_object, object))
                                .then(|| {
                                    self.resolve_object_binding_from_expression(
                                        &materialized_object,
                                    )
                                })
                                .flatten()
                        });
                if let Some(object_binding) = object_binding {
                    if let Some(value) = self.resolve_object_binding_property_value(
                        &object_binding,
                        &materialized_property,
                    ) {
                        return self.resolve_static_primitive_expression_with_context(
                            &value,
                            current_function_name,
                        );
                    }
                    if let Some(value) = self.static_typed_array_member_value_from_binding(
                        &object_binding,
                        &materialized_property,
                    ) {
                        return self.resolve_static_primitive_expression_with_context(
                            &value,
                            current_function_name,
                        );
                    }
                    if let Some(value) = self
                        .resolve_inherited_object_property_value(object, &materialized_property)
                        .or_else(|| {
                            (!static_expression_matches(&materialized_object, object))
                                .then(|| {
                                    self.resolve_inherited_object_property_value(
                                        &materialized_object,
                                        &materialized_property,
                                    )
                                })
                                .flatten()
                        })
                    {
                        return self.resolve_static_primitive_expression_with_context(
                            &value,
                            current_function_name,
                        );
                    }
                    if static_property_name_from_expression(&materialized_property).is_some() {
                        return Some(Expression::Undefined);
                    }
                }
                if std::env::var_os("AYY_TRACE_RUNTIME_SHADOWS").is_some() {
                    eprintln!(
                        "runtime_shadow_primitive_member_none object={object:?} property={property:?}"
                    );
                }
                None
            }
            Expression::Unary {
                op: UnaryOp::TypeOf,
                expression,
            } => Some(Expression::String(
                self.infer_typeof_operand_kind(expression)?
                    .as_typeof_str()?
                    .to_string(),
            )),
            Expression::Unary {
                op: UnaryOp::Negate,
                expression,
            } if self.infer_value_kind(expression) == Some(StaticValueKind::BigInt) => Some(
                Expression::BigInt((-self.resolve_static_bigint_value(expression)?).to_string()),
            ),
            Expression::Binary {
                op:
                    op @ (BinaryOp::Subtract
                    | BinaryOp::Multiply
                    | BinaryOp::Divide
                    | BinaryOp::Modulo
                    | BinaryOp::Exponentiate),
                left,
                right,
            } => match self.resolve_static_numeric_binary_outcome_with_context(
                *op,
                left,
                right,
                current_function_name,
            )? {
                StaticEvalOutcome::Value(value) => Some(value),
                StaticEvalOutcome::Throw(_) => None,
            },
            Expression::Binary {
                op:
                    BinaryOp::Add
                    | BinaryOp::BitwiseAnd
                    | BinaryOp::BitwiseOr
                    | BinaryOp::BitwiseXor
                    | BinaryOp::LeftShift
                    | BinaryOp::RightShift,
                left,
                right,
            } if self.infer_value_kind(left) == Some(StaticValueKind::BigInt)
                && self.infer_value_kind(right) == Some(StaticValueKind::BigInt) =>
            {
                Some(Expression::BigInt(
                    self.resolve_static_bigint_value(expression)?.to_string(),
                ))
            }
            Expression::Unary {
                op: UnaryOp::Plus | UnaryOp::Negate | UnaryOp::BitwiseNot,
                ..
            }
            | Expression::Binary {
                op:
                    BinaryOp::BitwiseAnd
                    | BinaryOp::BitwiseOr
                    | BinaryOp::BitwiseXor
                    | BinaryOp::LeftShift
                    | BinaryOp::RightShift
                    | BinaryOp::UnsignedRightShift,
                ..
            } => Some(Expression::Number(
                self.resolve_static_number_value(expression)?,
            )),
            Expression::Binary {
                op: op @ (BinaryOp::LogicalAnd | BinaryOp::LogicalOr | BinaryOp::NullishCoalescing),
                left,
                right,
            } => {
                let value = self.resolve_static_logical_result_expression(*op, left, right)?;
                self.resolve_static_primitive_expression_with_context(&value, current_function_name)
            }
            Expression::Binary {
                op:
                    BinaryOp::LessThan
                    | BinaryOp::LessThanOrEqual
                    | BinaryOp::GreaterThan
                    | BinaryOp::GreaterThanOrEqual
                    | BinaryOp::Equal
                    | BinaryOp::NotEqual
                    | BinaryOp::LooseEqual
                    | BinaryOp::LooseNotEqual
                    | BinaryOp::In
                    | BinaryOp::InstanceOf,
                ..
            }
            | Expression::Unary {
                op: UnaryOp::Not | UnaryOp::Delete,
                ..
            } => Some(Expression::Bool(
                self.resolve_static_boolean_expression(expression)?,
            )),
            Expression::Binary {
                op: BinaryOp::Add,
                left,
                right,
            } => match self.resolve_static_addition_outcome_with_context(
                left,
                right,
                current_function_name,
            )? {
                StaticEvalOutcome::Value(value) => self
                    .resolve_static_primitive_expression_with_context(
                        &value,
                        current_function_name,
                    ),
                StaticEvalOutcome::Throw(_) => None,
            },
            Expression::Call { callee, arguments } => {
                if let Some(value) = self
                    .resolve_static_has_own_property_call_result(expression)
                    .map(Expression::Bool)
                    .or_else(|| {
                        self.resolve_static_is_nan_call_result(expression)
                            .map(Expression::Bool)
                    })
                    .or_else(|| {
                        self.resolve_static_private_in_predicate_call_result(expression)
                            .map(Expression::Bool)
                    })
                    .or_else(|| {
                        self.resolve_static_object_is_call_result(expression)
                            .map(Expression::Bool)
                    })
                    .or_else(|| {
                        self.resolve_static_array_is_array_call_result(expression)
                            .map(Expression::Bool)
                    })
                {
                    return Some(value);
                }
                if arguments.is_empty()
                    && let Expression::Member { object, property } = callee.as_ref()
                    && let Expression::String(property_name) = property.as_ref()
                    && let Some(StaticEvalOutcome::Value(value)) = self
                        .resolve_static_member_call_outcome_with_context(
                            object,
                            property_name,
                            current_function_name,
                        )
                {
                    return self.resolve_static_primitive_expression_with_context(
                        &value,
                        current_function_name,
                    );
                }
                let (value, callee_function_name) = self
                    .resolve_static_call_result_expression_with_context(
                        callee,
                        arguments,
                        current_function_name,
                    )?;
                self.resolve_static_primitive_expression_with_context(
                    &value,
                    callee_function_name.as_deref().or(current_function_name),
                )
            }
            _ => None,
        }
    }
}
