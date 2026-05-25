use super::*;

thread_local! {
    static BOUND_ALIAS_RESOLUTION_DEPTH: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

struct BoundAliasResolutionGuard;

impl BoundAliasResolutionGuard {
    fn enter(expression: &Expression) -> Self {
        BOUND_ALIAS_RESOLUTION_DEPTH.with(|depth| {
            let next = depth.get() + 1;
            if next > 256 {
                panic!("bound alias resolution recursion overflow: expression={expression:?}");
            }
            depth.set(next);
        });
        Self
    }
}

impl Drop for BoundAliasResolutionGuard {
    fn drop(&mut self) {
        BOUND_ALIAS_RESOLUTION_DEPTH.with(|depth| depth.set(depth.get().saturating_sub(1)));
    }
}

impl<'a> FunctionCompiler<'a> {
    fn function_references_nested_function(
        &self,
        function: &FunctionDeclaration,
        nested_function_name: &str,
    ) -> bool {
        if collect_referenced_binding_names_from_statements(&function.body)
            .contains(nested_function_name)
        {
            return true;
        }
        function.params.iter().any(|parameter| {
            parameter.default.as_ref().is_some_and(|default| {
                let mut referenced = HashSet::new();
                collect_referenced_binding_names_from_expression(default, &mut referenced);
                referenced.contains(nested_function_name)
            })
        })
    }

    fn enclosing_function_name_for_alias_resolution(&self, function_name: &str) -> Option<String> {
        self.user_functions()
            .into_iter()
            .filter(|candidate| candidate.name != function_name)
            .find(|candidate| {
                self.resolve_registered_function_declaration(&candidate.name)
                    .is_some_and(|function| {
                        self.function_references_nested_function(function, function_name)
                    })
            })
            .map(|candidate| candidate.name.clone())
    }

    fn statement_capture_source_value(
        statement: &Statement,
        source_name: &str,
    ) -> Option<Expression> {
        match statement {
            Statement::Var { name, value }
            | Statement::Let { name, value, .. }
            | Statement::Assign { name, value } => {
                let binding_source = scoped_binding_source_name(name).unwrap_or(name);
                (binding_source == source_name).then_some(value.clone())
            }
            Statement::Declaration { body }
            | Statement::Block { body }
            | Statement::Labeled { body, .. } => body
                .iter()
                .filter_map(|statement| {
                    Self::statement_capture_source_value(statement, source_name)
                })
                .last(),
            _ => None,
        }
    }

    pub(in crate::backend::direct_wasm) fn resolve_captured_alias_expression(
        &self,
        function_name: &str,
        source_name: &str,
        visited: &mut HashSet<(String, String)>,
    ) -> Option<Expression> {
        if !visited.insert((function_name.to_string(), source_name.to_string())) {
            return None;
        }
        if source_name == "this" {
            return Some(Expression::This);
        }
        let enclosing_name = self.enclosing_function_name_for_alias_resolution(function_name)?;
        let enclosing_function = self.resolve_registered_function_declaration(&enclosing_name)?;
        let source_value = enclosing_function
            .body
            .iter()
            .filter_map(|statement| Self::statement_capture_source_value(statement, source_name))
            .last()?;
        match source_value {
            Expression::Identifier(identifier) => {
                if identifier == "this" {
                    Some(Expression::This)
                } else {
                    self.resolve_captured_alias_expression(&enclosing_name, &identifier, visited)
                        .or(Some(Expression::Identifier(identifier)))
                }
            }
            other => Some(other),
        }
    }

    pub(in crate::backend::direct_wasm) fn resolve_bound_alias_expression_with_state(
        &self,
        expression: &Expression,
        environment: &StaticResolutionEnvironment,
    ) -> Option<Expression> {
        resolve_bound_alias_expression_in_environment(
            expression,
            environment,
            &|name| self.with_scope_blocks_static_identifier_resolution(name),
            &|name| {
                self.state
                    .runtime
                    .locals
                    .runtime_dynamic_bindings
                    .contains(name)
            },
            &|name, environment| environment.binding(name).cloned(),
        )
    }

    pub(in crate::backend::direct_wasm) fn resolve_bound_alias_expression(
        &self,
        expression: &Expression,
    ) -> Option<Expression> {
        let _guard = BoundAliasResolutionGuard::enter(expression);
        let trace_private = std::env::var_os("AYY_TRACE_PRIVATE_MEMBER_LOOKUP").is_some();
        let mut current = expression;
        let mut current_owned = None;
        let mut visited = HashSet::new();
        loop {
            let Expression::Identifier(name) = current else {
                return Some(current.clone());
            };
            if self.with_scope_blocks_static_identifier_resolution(name) {
                return Some(current.clone());
            }
            if self
                .state
                .runtime
                .locals
                .runtime_dynamic_bindings
                .contains(name)
            {
                return Some(current.clone());
            }
            if !visited.insert(name.clone()) {
                return None;
            }
            if let Some((resolved_name, _)) = self.resolve_current_local_binding(name) {
                if self
                    .state
                    .runtime
                    .locals
                    .runtime_dynamic_bindings
                    .contains(&resolved_name)
                {
                    return Some(Expression::Identifier(resolved_name));
                }
                if let Some(value) = self
                    .state
                    .speculation
                    .static_semantics
                    .local_value_binding(&resolved_name)
                {
                    current = value;
                    continue;
                }
                return Some(Expression::Identifier(resolved_name));
            }
            if let Some(value) = self
                .state
                .speculation
                .static_semantics
                .local_value_binding(name)
            {
                current = value;
                continue;
            }
            if let Some(hidden_name) = self.resolve_user_function_capture_hidden_name(name)
                && let Some(value) = self.global_value_binding(&hidden_name)
            {
                if trace_private {
                    eprintln!(
                        "private_lookup alias current_fn={:?} name={} source=hidden-global hidden={} value={:?}",
                        self.current_function_name(),
                        name,
                        hidden_name,
                        value,
                    );
                }
                current = value;
                continue;
            }
            if let Some(function_name) = self.current_function_name()
                && self
                    .resolve_user_function_capture_hidden_name(name)
                    .is_some()
                && let Some(value) =
                    self.resolve_captured_alias_expression(function_name, name, &mut HashSet::new())
            {
                if trace_private {
                    eprintln!(
                        "private_lookup alias current_fn={:?} name={} source=captured-enclosing value={:?}",
                        self.current_function_name(),
                        name,
                        value,
                    );
                }
                current_owned = Some(value);
                current = current_owned.as_ref().expect("owned alias expression");
                continue;
            }
            if let Some(value) = self.backend.global_value_binding(name) {
                if trace_private {
                    eprintln!(
                        "private_lookup alias current_fn={:?} name={} source=global value={:?}",
                        self.current_function_name(),
                        name,
                        value,
                    );
                }
                current = value;
                continue;
            }
            return Some(current.clone());
        }
    }

    pub(in crate::backend::direct_wasm) fn expression_aliases_captured_top_level_this(
        &self,
        expression: &Expression,
    ) -> bool {
        let Expression::Identifier(name) = expression else {
            return false;
        };
        if self.resolve_current_local_binding(name).is_none()
            && matches!(self.global_value_binding(name), Some(Expression::This))
        {
            return true;
        }
        let Some(function_name) = self.current_function_name() else {
            return false;
        };
        if self
            .resolve_user_function_capture_hidden_name(name)
            .is_none()
        {
            return false;
        }
        matches!(
            self.resolve_captured_alias_expression(function_name, name, &mut HashSet::new()),
            Some(Expression::This)
        )
    }

    pub(in crate::backend::direct_wasm) fn resolve_symbol_identity_expression(
        &self,
        expression: &Expression,
    ) -> Option<Expression> {
        if let Expression::Call { callee, arguments } = expression
            && arguments.is_empty()
            && let Expression::Member { object, property } = callee.as_ref()
            && matches!(property.as_ref(), Expression::String(name) if name == "valueOf")
            && self.infer_value_kind(object) == Some(StaticValueKind::Symbol)
        {
            return self.resolve_symbol_identity_expression(object);
        }

        let Expression::Identifier(name) = expression else {
            return None;
        };
        if let Some((resolved_name, _)) = self.resolve_current_local_binding(name)
            && resolved_name != *name
            && let Some(resolved) =
                self.resolve_symbol_identity_expression(&Expression::Identifier(resolved_name))
        {
            return Some(resolved);
        }
        if self.lookup_identifier_kind(name) != Some(StaticValueKind::Symbol) {
            if let Some(resolved) = self.resolve_bound_alias_expression(expression)
                && !static_expression_matches(&resolved, expression)
            {
                if self.well_known_symbol_name(&resolved).is_some() {
                    return Some(resolved);
                }
                if let Expression::Identifier(resolved_name) = &resolved
                    && self.lookup_identifier_kind(resolved_name) == Some(StaticValueKind::Symbol)
                {
                    return Some(resolved);
                }
            }
            return None;
        }

        let mut current_name = name.clone();
        let mut visited = HashSet::new();
        loop {
            if !visited.insert(current_name.clone()) {
                return None;
            }
            let next = if let Some((resolved_name, _)) =
                self.resolve_current_local_binding(&current_name)
            {
                if resolved_name != current_name
                    && self.lookup_identifier_kind(&resolved_name) == Some(StaticValueKind::Symbol)
                {
                    current_name = resolved_name;
                    continue;
                }
                self.state
                    .speculation
                    .static_semantics
                    .local_value_binding(&resolved_name)
            } else {
                self.state
                    .speculation
                    .static_semantics
                    .local_value_binding(&current_name)
                    .or_else(|| self.backend.global_value_binding(&current_name))
            };
            match next {
                Some(Expression::Identifier(next_name))
                    if self.lookup_identifier_kind(next_name) == Some(StaticValueKind::Symbol) =>
                {
                    current_name = next_name.clone();
                }
                _ => return Some(Expression::Identifier(current_name)),
            }
        }
    }

    pub(in crate::backend::direct_wasm) fn resolve_global_value_expression(
        &self,
        expression: &Expression,
    ) -> Option<Expression> {
        let mut visited = HashSet::new();
        self.resolve_global_value_expression_with_visited(expression, &mut visited)
    }

    pub(in crate::backend::direct_wasm) fn resolve_global_value_expression_with_visited(
        &self,
        expression: &Expression,
        visited: &mut HashSet<String>,
    ) -> Option<Expression> {
        let Expression::Identifier(name) = expression else {
            return Some(expression.clone());
        };
        if self.with_scope_blocks_static_identifier_resolution(name) {
            return None;
        }
        if self.resolve_current_local_binding(name).is_some()
            && self.current_function_name().is_some()
        {
            return None;
        }
        if !visited.insert(name.clone()) {
            return None;
        }
        let value = self.backend.global_value_binding(name)?.clone();
        self.resolve_global_identifiers_in_expression(&value, visited)
    }

    pub(in crate::backend::direct_wasm) fn resolve_global_identifiers_in_expression(
        &self,
        expression: &Expression,
        visited: &mut HashSet<String>,
    ) -> Option<Expression> {
        match expression {
            Expression::Identifier(name) if self.backend.global_value_binding(name).is_some() => {
                self.resolve_global_value_expression_with_visited(expression, visited)
            }
            Expression::Unary { op, expression } => Some(Expression::Unary {
                op: *op,
                expression: Box::new(
                    self.resolve_global_identifiers_in_expression(expression, visited)?,
                ),
            }),
            Expression::Binary { op, left, right } => Some(Expression::Binary {
                op: *op,
                left: Box::new(self.resolve_global_identifiers_in_expression(left, visited)?),
                right: Box::new(self.resolve_global_identifiers_in_expression(right, visited)?),
            }),
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => Some(Expression::Conditional {
                condition: Box::new(
                    self.resolve_global_identifiers_in_expression(condition, visited)?,
                ),
                then_expression: Box::new(
                    self.resolve_global_identifiers_in_expression(then_expression, visited)?,
                ),
                else_expression: Box::new(
                    self.resolve_global_identifiers_in_expression(else_expression, visited)?,
                ),
            }),
            Expression::Sequence(expressions) => Some(Expression::Sequence(
                expressions
                    .iter()
                    .map(|expression| {
                        self.resolve_global_identifiers_in_expression(expression, visited)
                    })
                    .collect::<Option<Vec<_>>>()?,
            )),
            Expression::Member { object, property } => Some(Expression::Member {
                object: Box::new(self.resolve_global_identifiers_in_expression(object, visited)?),
                property: Box::new(
                    self.resolve_global_identifiers_in_expression(property, visited)?,
                ),
            }),
            Expression::Call { callee, arguments } => Some(Expression::Call {
                callee: Box::new(self.resolve_global_identifiers_in_expression(callee, visited)?),
                arguments: arguments
                    .iter()
                    .map(|argument| match argument {
                        CallArgument::Expression(expression) => Some(CallArgument::Expression(
                            self.resolve_global_identifiers_in_expression(expression, visited)?,
                        )),
                        CallArgument::Spread(expression) => Some(CallArgument::Spread(
                            self.resolve_global_identifiers_in_expression(expression, visited)?,
                        )),
                    })
                    .collect::<Option<Vec<_>>>()?,
            }),
            _ => Some(expression.clone()),
        }
    }
}
