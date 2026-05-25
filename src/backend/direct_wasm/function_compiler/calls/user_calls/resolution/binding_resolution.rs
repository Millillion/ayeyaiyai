use super::guard::{FunctionBindingResolutionGuard, FunctionBindingResolutionShapeGuard};
use super::*;

impl<'a> FunctionCompiler<'a> {
    fn resolve_scoped_function_declaration_alias_binding(
        &self,
        name: &str,
        current_function_name: Option<&str>,
    ) -> Option<LocalFunctionBinding> {
        fn statements_alias_target(statements: &[Statement], name: &str) -> Option<String> {
            statements
                .iter()
                .find_map(|statement| statement_alias_target(statement, name))
        }

        fn statement_alias_target(statement: &Statement, name: &str) -> Option<String> {
            match statement {
                Statement::Var {
                    name: target,
                    value,
                }
                | Statement::Let {
                    name: target,
                    value,
                    ..
                }
                | Statement::Assign {
                    name: target,
                    value,
                } if target == name => match value {
                    Expression::Identifier(function_name)
                        if is_internal_user_function_identifier(function_name) =>
                    {
                        Some(function_name.clone())
                    }
                    _ => None,
                },
                Statement::Declaration { body }
                | Statement::Block { body }
                | Statement::Labeled { body, .. }
                | Statement::With { body, .. }
                | Statement::While { body, .. }
                | Statement::DoWhile { body, .. } => statements_alias_target(body, name),
                Statement::If {
                    then_branch,
                    else_branch,
                    ..
                } => statements_alias_target(then_branch, name)
                    .or_else(|| statements_alias_target(else_branch, name)),
                Statement::Try {
                    body,
                    catch_setup,
                    catch_body,
                    ..
                } => statements_alias_target(body, name)
                    .or_else(|| statements_alias_target(catch_setup, name))
                    .or_else(|| statements_alias_target(catch_body, name)),
                Statement::Switch { cases, .. } => cases
                    .iter()
                    .find_map(|case| statements_alias_target(&case.body, name)),
                Statement::For { init, body, .. } => statements_alias_target(init, name)
                    .or_else(|| statements_alias_target(body, name)),
                _ => None,
            }
        }

        let declaration = current_function_name.and_then(|function_name| {
            self.resolve_registered_function_declaration(function_name)
        })?;
        let function_name = statements_alias_target(&declaration.body, name)?;
        self.contains_user_function(&function_name)
            .then_some(LocalFunctionBinding::User(function_name))
    }

    fn resolve_static_default_condition_value(
        &self,
        condition: &Expression,
        then_expression: &Expression,
        current_function_name: Option<&str>,
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
        if !static_expression_matches(compared_value, then_expression) {
            let materialized_compared = self.materialize_static_expression(compared_value);
            let materialized_then = self.materialize_static_expression(then_expression);
            if !static_expression_matches(&materialized_compared, &materialized_then) {
                return None;
            }
        }
        let is_undefined =
            self.expression_resolves_to_static_undefined(compared_value, current_function_name)?;
        Some(is_undefined ^ is_not_equal)
    }

    fn expression_resolves_to_static_undefined(
        &self,
        expression: &Expression,
        current_function_name: Option<&str>,
    ) -> Option<bool> {
        if matches!(expression, Expression::Undefined) {
            return Some(true);
        }
        if matches!(expression, Expression::Identifier(name) if name == "undefined" && self.is_unshadowed_builtin_identifier(name))
        {
            return Some(true);
        }
        if let Some(primitive) =
            self.resolve_static_primitive_expression_with_context(expression, current_function_name)
        {
            return Some(matches!(primitive, Expression::Undefined));
        }
        if let Some(StaticEvalOutcome::Value(value)) =
            self.resolve_static_await_resolution_outcome(expression)
        {
            if static_expression_matches(&value, expression) {
                return None;
            }
            return self.expression_resolves_to_static_undefined(&value, current_function_name);
        }
        let materialized = self.materialize_static_expression(expression);
        if !static_expression_matches(&materialized, expression) {
            return self
                .expression_resolves_to_static_undefined(&materialized, current_function_name);
        }
        if let Expression::Member { object, property } = expression {
            let property = self.materialize_static_expression(property);
            if let Some(StaticEvalOutcome::Value(value)) =
                self.resolve_static_property_get_outcome(object, &property)
            {
                return self.expression_resolves_to_static_undefined(&value, current_function_name);
            }
            let materialized_object = self.materialize_static_expression(object);
            if !static_expression_matches(&materialized_object, object)
                && let Some(StaticEvalOutcome::Value(value)) =
                    self.resolve_static_property_get_outcome(&materialized_object, &property)
            {
                return self.expression_resolves_to_static_undefined(&value, current_function_name);
            }
        }
        None
    }

    fn resolve_static_conditional_binding_branch<'e>(
        &self,
        condition: &Expression,
        then_expression: &'e Expression,
        else_expression: &'e Expression,
        current_function_name: Option<&str>,
    ) -> Option<&'e Expression> {
        if let Some(condition_value) = self.resolve_static_if_condition_value(condition) {
            return Some(if condition_value {
                then_expression
            } else {
                else_expression
            });
        }
        let materialized_condition = self.materialize_static_expression(condition);
        if !static_expression_matches(&materialized_condition, condition)
            && let Some(condition_value) =
                self.resolve_static_if_condition_value(&materialized_condition)
        {
            return Some(if condition_value {
                then_expression
            } else {
                else_expression
            });
        }
        self.resolve_static_default_condition_value(
            condition,
            then_expression,
            current_function_name,
        )
        .map(|condition_value| {
            if condition_value {
                then_expression
            } else {
                else_expression
            }
        })
    }

    fn resolve_bound_builtin_function_binding_from_call(
        &self,
        callee: &Expression,
        arguments: &[CallArgument],
        current_function_name: Option<&str>,
    ) -> Option<LocalFunctionBinding> {
        let Expression::Member { object, property } = callee else {
            return None;
        };
        if !matches!(property.as_ref(), Expression::String(name) if name == "bind") {
            return None;
        }
        let LocalFunctionBinding::Builtin(function_name) = self
            .resolve_function_binding_from_expression_with_context(object, current_function_name)?
        else {
            return None;
        };
        if function_name != "Function.prototype.call" {
            return None;
        }
        let [
            CallArgument::Expression(target) | CallArgument::Spread(target),
            ..,
        ] = arguments
        else {
            return None;
        };
        let LocalFunctionBinding::Builtin(target_name) = self
            .resolve_function_binding_from_expression_with_context(target, current_function_name)?
        else {
            return None;
        };
        Some(LocalFunctionBinding::Builtin(
            bound_function_prototype_call_builtin_name(&target_name),
        ))
    }

    pub(in crate::backend::direct_wasm) fn resolve_function_binding_from_expression(
        &self,
        expression: &Expression,
    ) -> Option<LocalFunctionBinding> {
        self.resolve_function_binding_from_expression_with_context(
            expression,
            self.current_function_name(),
        )
    }

    pub(in crate::backend::direct_wasm) fn function_binding_resolution_is_active(&self) -> bool {
        super::guard::function_binding_resolution_is_active()
    }

    pub(in crate::backend::direct_wasm) fn is_restricted_function_property(
        &self,
        object: &Expression,
        property: &Expression,
    ) -> bool {
        if !matches!(
            property,
            Expression::String(property_name)
                if property_name == "caller" || property_name == "arguments"
        ) {
            return false;
        }

        if self
            .resolve_function_prototype_bind_call(object, self.current_function_name())
            .is_some()
        {
            return true;
        }

        self.resolve_user_function_from_expression(object)
            .is_some_and(|user_function| {
                user_function.is_arrow() || user_function.is_generator() || user_function.strict
            })
    }

    pub(in crate::backend::direct_wasm) fn resolve_function_binding_from_expression_with_context(
        &self,
        expression: &Expression,
        current_function_name: Option<&str>,
    ) -> Option<LocalFunctionBinding> {
        let _guard = FunctionBindingResolutionGuard::enter(expression, current_function_name)?;
        let _shape_guard =
            FunctionBindingResolutionShapeGuard::enter(expression, current_function_name)?;
        if let Expression::Identifier(name) = expression {
            if let Some((resolved_name, _)) = self.resolve_current_local_binding(name) {
                if let Some(function_binding) = self
                    .state
                    .speculation
                    .static_semantics
                    .local_function_binding(&resolved_name)
                    .cloned()
                {
                    return Some(function_binding);
                }
                if resolved_name.as_str() != name.as_str()
                    && let Some(function_binding) = self
                        .state
                        .speculation
                        .static_semantics
                        .local_function_binding(name)
                        .cloned()
                {
                    return Some(function_binding);
                }
            } else if let Some(function_binding) = self
                .state
                .speculation
                .static_semantics
                .local_function_binding(name)
                .cloned()
            {
                return Some(function_binding);
            }
        }
        if let Some(resolved) = self
            .resolve_bound_alias_expression(expression)
            .filter(|resolved| !static_expression_matches(resolved, expression))
        {
            if let Some(binding) = self.resolve_function_binding_from_expression_with_context(
                &resolved,
                current_function_name,
            ) {
                return Some(binding);
            }
        }
        let binding = match expression {
            Expression::Identifier(name) => {
                if let Some((resolved_name, _)) = self.resolve_current_local_binding(name) {
                    self.state
                        .speculation
                        .static_semantics
                        .local_function_binding(&resolved_name)
                        .cloned()
                        .or_else(|| {
                            self.resolve_scoped_function_declaration_alias_binding(
                                &resolved_name,
                                current_function_name,
                            )
                        })
                        .or_else(|| {
                            if resolved_name.as_str() != name.as_str() {
                                self.resolve_scoped_function_declaration_alias_binding(
                                    name,
                                    current_function_name,
                                )
                            } else {
                                None
                            }
                        })
                        .or_else(|| {
                            self.state
                                .speculation
                                .execution_context
                                .top_level_function
                                .then(|| {
                                    self.backend.global_function_binding(name).cloned().or_else(
                                        || {
                                            self.backend
                                                .global_function_binding(&resolved_name)
                                                .cloned()
                                        },
                                    )
                                })
                                .flatten()
                        })
                } else if let Some(function_binding) = self
                    .state
                    .speculation
                    .static_semantics
                    .local_function_binding(name)
                    .cloned()
                {
                    Some(function_binding)
                } else if self.resolve_eval_local_function_hidden_name(name).is_some() {
                    self.state
                        .speculation
                        .static_semantics
                        .local_function_binding(name)
                        .cloned()
                } else if let Some(function_binding) = self
                    .resolve_scoped_function_declaration_alias_binding(name, current_function_name)
                {
                    Some(function_binding)
                } else if builtin_function_runtime_value(name).is_some() {
                    Some(LocalFunctionBinding::Builtin(name.clone()))
                } else if let Some(function_binding) = self
                    .backend
                    .global_semantics
                    .functions
                    .function_binding(name)
                {
                    Some(function_binding.clone())
                } else if let Some(function_binding) =
                    current_function_name.and_then(|function_name| {
                        self.backend
                            .function_registry
                            .parameter_bindings_for(function_name)
                            .function_bindings
                            .get(name)
                            .cloned()
                            .flatten()
                    })
                {
                    Some(function_binding)
                } else if is_internal_user_function_identifier(name)
                    && self.contains_user_function(name)
                {
                    Some(LocalFunctionBinding::User(name.clone()))
                } else if name == "eval" || self.infer_call_result_kind(name).is_some() {
                    Some(LocalFunctionBinding::Builtin(name.clone()))
                } else {
                    None
                }
            }
            Expression::Assign { value, .. } => self
                .resolve_function_binding_from_expression_with_context(
                    value,
                    current_function_name,
                ),
            Expression::Call { callee, arguments } | Expression::New { callee, arguments } => {
                if let Expression::Call { .. } = expression
                    && let Some(binding) = self.resolve_bound_builtin_function_binding_from_call(
                        callee,
                        arguments,
                        current_function_name,
                    )
                {
                    return Some(binding);
                }
                if let Expression::Call { .. } = expression
                    && let Some((_, _, binding)) =
                        self.resolve_function_prototype_bind_call(expression, current_function_name)
                {
                    return Some(binding);
                }
                self.resolve_static_call_result_expression_with_context(
                    callee,
                    arguments,
                    current_function_name,
                )
                .and_then(|(value, callee_function_name)| {
                    self.resolve_function_binding_from_expression_with_context(
                        &value,
                        callee_function_name.as_deref().or(current_function_name),
                    )
                })
                .or_else(|| self.resolve_returned_function_binding_from_call(callee, arguments))
            }
            Expression::Binary { op, left, right }
                if matches!(
                    op,
                    BinaryOp::LogicalAnd | BinaryOp::LogicalOr | BinaryOp::NullishCoalescing
                ) =>
            {
                self.resolve_static_logical_result_expression(*op, left, right)
                    .and_then(|resolved| {
                        self.resolve_function_binding_from_expression_with_context(
                            &resolved,
                            current_function_name,
                        )
                    })
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                if let Some(branch) = self.resolve_static_conditional_binding_branch(
                    condition,
                    then_expression,
                    else_expression,
                    current_function_name,
                ) {
                    self.resolve_function_binding_from_expression_with_context(
                        branch,
                        current_function_name,
                    )
                } else {
                    let then_binding = self.resolve_function_binding_from_expression_with_context(
                        then_expression,
                        current_function_name,
                    );
                    let else_binding = self.resolve_function_binding_from_expression_with_context(
                        else_expression,
                        current_function_name,
                    );
                    match (then_binding, else_binding) {
                        (Some(then_binding), Some(else_binding))
                            if then_binding == else_binding =>
                        {
                            Some(then_binding)
                        }
                        _ => None,
                    }
                }
            }
            Expression::Sequence(expressions) => expressions.last().and_then(|expression| {
                self.resolve_function_binding_from_expression_with_context(
                    expression,
                    current_function_name,
                )
            }),
            Expression::Member { object, property } => {
                if matches!(property.as_ref(), Expression::String(name) if name == "constructor")
                    && let Expression::Call { callee, arguments } = object.as_ref()
                    && matches!(
                        callee.as_ref(),
                        Expression::Member { object, property }
                            if matches!(object.as_ref(), Expression::Identifier(name) if name == "Object")
                                && matches!(property.as_ref(), Expression::String(name) if name == "getPrototypeOf")
                    )
                    && let Some(CallArgument::Expression(target) | CallArgument::Spread(target)) =
                        arguments.first()
                    && let Some(target_binding) = self
                        .resolve_function_binding_from_expression_with_context(
                            target,
                            current_function_name,
                        )
                {
                    let constructor_name = match target_binding {
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
                    return Some(LocalFunctionBinding::Builtin(constructor_name.to_string()));
                }
                if matches!(property.as_ref(), Expression::String(name) if name == "constructor")
                    && let Some(function_binding) =
                        self.resolve_function_binding_from_expression(object)
                {
                    let constructor_name = match function_binding {
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
                    return Some(LocalFunctionBinding::Builtin(constructor_name.to_string()));
                }
                if matches!(property.as_ref(), Expression::String(name) if name == "value") {
                    if let Some(IteratorStepBinding::Runtime {
                        function_binding: Some(function_binding),
                        ..
                    }) = self.resolve_iterator_step_binding_from_expression(object)
                    {
                        return Some(function_binding);
                    }
                }
                if let Some(value) =
                    self.resolve_returned_member_value_from_expression(object, property)
                {
                    self.resolve_function_binding_from_expression(&value)
                } else if let Some(getter_binding) =
                    self.resolve_member_getter_binding(object, property)
                    && let Some(value) = self
                        .resolve_function_binding_static_return_expression_with_call_frame(
                            &getter_binding,
                            &[],
                            object,
                        )
                {
                    self.resolve_function_binding_from_expression(&value)
                } else {
                    self.resolve_member_function_binding(object, property)
                }
            }
            Expression::SuperMember { property } => {
                self.resolve_super_function_binding_with_context(property, current_function_name)
            }
            _ => None,
        };
        if binding.is_some() {
            return binding;
        }

        let materialized = self.materialize_static_expression(expression);
        if !static_expression_matches(&materialized, expression) {
            return self.resolve_function_binding_from_expression_with_context(
                &materialized,
                current_function_name,
            );
        }
        None
    }
}
