use super::*;

fn is_using_completion_binding(name: &str) -> bool {
    name.starts_with("__ayy_using_error_")
}

enum StaticCatchScanOutcome {
    CatchValue(Expression),
    Continue,
    Unsupported,
}

impl<'a> FunctionCompiler<'a> {
    fn merge_possible_throw_kind(
        current: &mut Option<StaticValueKind>,
        candidate: Option<StaticValueKind>,
    ) {
        let Some(candidate) = candidate else {
            return;
        };
        match current {
            None => *current = Some(candidate),
            Some(existing) if *existing == candidate => {}
            Some(existing) => *existing = StaticValueKind::Unknown,
        }
    }

    pub(in crate::backend::direct_wasm) fn resolve_possible_throw_kind_from_try_body(
        &self,
        body: &[Statement],
    ) -> Option<StaticValueKind> {
        let mut visited_functions = HashSet::new();
        self.resolve_possible_throw_kind_from_statements(body, &mut visited_functions)
    }

    fn resolve_possible_throw_kind_from_statements(
        &self,
        statements: &[Statement],
        visited_functions: &mut HashSet<String>,
    ) -> Option<StaticValueKind> {
        let mut kind = None;
        for statement in statements {
            Self::merge_possible_throw_kind(
                &mut kind,
                self.resolve_possible_throw_kind_from_statement(statement, visited_functions),
            );
        }
        kind
    }

    fn resolve_possible_throw_kind_from_statement(
        &self,
        statement: &Statement,
        visited_functions: &mut HashSet<String>,
    ) -> Option<StaticValueKind> {
        match statement {
            Statement::Declaration { body }
            | Statement::Block { body }
            | Statement::Labeled { body, .. }
            | Statement::With { body, .. }
            | Statement::DoWhile { body, .. }
            | Statement::While { body, .. } => {
                self.resolve_possible_throw_kind_from_statements(body, visited_functions)
            }
            Statement::Throw(expression) => self.infer_value_kind(expression),
            Statement::Var { value, .. }
            | Statement::Let { value, .. }
            | Statement::Assign { value, .. }
            | Statement::Expression(value)
            | Statement::Return(value)
            | Statement::Yield { value }
            | Statement::YieldDelegate { value } => {
                self.resolve_possible_throw_kind_from_expression(value, visited_functions)
            }
            Statement::AssignMember {
                object,
                property,
                value,
            } => {
                let mut kind = None;
                for expression in [object, property, value] {
                    Self::merge_possible_throw_kind(
                        &mut kind,
                        self.resolve_possible_throw_kind_from_expression(
                            expression,
                            visited_functions,
                        ),
                    );
                }
                kind
            }
            Statement::If {
                condition,
                then_branch,
                else_branch,
            } => {
                let mut kind =
                    self.resolve_possible_throw_kind_from_expression(condition, visited_functions);
                Self::merge_possible_throw_kind(
                    &mut kind,
                    self.resolve_possible_throw_kind_from_statements(
                        then_branch,
                        visited_functions,
                    ),
                );
                Self::merge_possible_throw_kind(
                    &mut kind,
                    self.resolve_possible_throw_kind_from_statements(
                        else_branch,
                        visited_functions,
                    ),
                );
                kind
            }
            Statement::Try {
                body,
                catch_setup,
                catch_body,
                ..
            } => {
                let mut kind =
                    self.resolve_possible_throw_kind_from_statements(body, visited_functions);
                Self::merge_possible_throw_kind(
                    &mut kind,
                    self.resolve_possible_throw_kind_from_statements(
                        catch_setup,
                        visited_functions,
                    ),
                );
                Self::merge_possible_throw_kind(
                    &mut kind,
                    self.resolve_possible_throw_kind_from_statements(catch_body, visited_functions),
                );
                kind
            }
            Statement::Switch {
                discriminant,
                cases,
                ..
            } => {
                let mut kind = self
                    .resolve_possible_throw_kind_from_expression(discriminant, visited_functions);
                for case in cases {
                    if let Some(test) = &case.test {
                        Self::merge_possible_throw_kind(
                            &mut kind,
                            self.resolve_possible_throw_kind_from_expression(
                                test,
                                visited_functions,
                            ),
                        );
                    }
                    Self::merge_possible_throw_kind(
                        &mut kind,
                        self.resolve_possible_throw_kind_from_statements(
                            &case.body,
                            visited_functions,
                        ),
                    );
                }
                kind
            }
            Statement::For {
                init,
                condition,
                update,
                body,
                break_hook,
                ..
            } => {
                let mut kind =
                    self.resolve_possible_throw_kind_from_statements(init, visited_functions);
                if let Some(condition) = condition {
                    Self::merge_possible_throw_kind(
                        &mut kind,
                        self.resolve_possible_throw_kind_from_expression(
                            condition,
                            visited_functions,
                        ),
                    );
                }
                if let Some(update) = update {
                    Self::merge_possible_throw_kind(
                        &mut kind,
                        self.resolve_possible_throw_kind_from_expression(update, visited_functions),
                    );
                }
                if let Some(break_hook) = break_hook {
                    Self::merge_possible_throw_kind(
                        &mut kind,
                        self.resolve_possible_throw_kind_from_expression(
                            break_hook,
                            visited_functions,
                        ),
                    );
                }
                Self::merge_possible_throw_kind(
                    &mut kind,
                    self.resolve_possible_throw_kind_from_statements(body, visited_functions),
                );
                kind
            }
            Statement::Print { values } => {
                let mut kind = None;
                for value in values {
                    Self::merge_possible_throw_kind(
                        &mut kind,
                        self.resolve_possible_throw_kind_from_expression(value, visited_functions),
                    );
                }
                kind
            }
            Statement::Break { .. } | Statement::Continue { .. } => None,
        }
    }

    fn resolve_possible_throw_kind_from_expression(
        &self,
        expression: &Expression,
        visited_functions: &mut HashSet<String>,
    ) -> Option<StaticValueKind> {
        if let Some(throw_value) = self.resolve_terminal_expression_throw_value(expression) {
            return self.infer_value_kind(&throw_value);
        }

        match expression {
            Expression::Call { callee, arguments } | Expression::New { callee, arguments } => {
                let mut kind =
                    self.resolve_possible_throw_kind_from_expression(callee, visited_functions);
                for argument in arguments {
                    Self::merge_possible_throw_kind(
                        &mut kind,
                        self.resolve_possible_throw_kind_from_expression(
                            argument.expression(),
                            visited_functions,
                        ),
                    );
                }
                if matches!(
                    callee.as_ref(),
                    Expression::Member { property, .. }
                        if matches!(property.as_ref(), Expression::String(name) if name == "call" || name == "apply")
                ) {
                    return kind;
                }
                if let Some(LocalFunctionBinding::User(function_name)) =
                    self.resolve_function_binding_from_expression(callee)
                    && visited_functions.insert(function_name.clone())
                {
                    if let Some(function) =
                        self.resolve_registered_function_declaration(&function_name)
                    {
                        Self::merge_possible_throw_kind(
                            &mut kind,
                            self.resolve_possible_throw_kind_from_statements(
                                &function.body,
                                visited_functions,
                            ),
                        );
                    }
                    visited_functions.remove(&function_name);
                }
                kind
            }
            Expression::Assign { value, .. }
            | Expression::Unary {
                expression: value, ..
            }
            | Expression::Await(value)
            | Expression::EnumerateKeys(value)
            | Expression::GetIterator(value)
            | Expression::IteratorClose(value) => {
                self.resolve_possible_throw_kind_from_expression(value, visited_functions)
            }
            Expression::Member { object, property }
            | Expression::Binary {
                left: object,
                right: property,
                ..
            } => {
                let mut kind =
                    self.resolve_possible_throw_kind_from_expression(object, visited_functions);
                Self::merge_possible_throw_kind(
                    &mut kind,
                    self.resolve_possible_throw_kind_from_expression(property, visited_functions),
                );
                kind
            }
            Expression::SuperMember { property } => {
                self.resolve_possible_throw_kind_from_expression(property, visited_functions)
            }
            Expression::AssignMember {
                object,
                property,
                value,
            } => {
                let mut kind =
                    self.resolve_possible_throw_kind_from_expression(object, visited_functions);
                Self::merge_possible_throw_kind(
                    &mut kind,
                    self.resolve_possible_throw_kind_from_expression(property, visited_functions),
                );
                Self::merge_possible_throw_kind(
                    &mut kind,
                    self.resolve_possible_throw_kind_from_expression(value, visited_functions),
                );
                kind
            }
            Expression::AssignSuperMember { property, value } => {
                let mut kind =
                    self.resolve_possible_throw_kind_from_expression(property, visited_functions);
                Self::merge_possible_throw_kind(
                    &mut kind,
                    self.resolve_possible_throw_kind_from_expression(value, visited_functions),
                );
                kind
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                let mut kind =
                    self.resolve_possible_throw_kind_from_expression(condition, visited_functions);
                Self::merge_possible_throw_kind(
                    &mut kind,
                    self.resolve_possible_throw_kind_from_expression(
                        then_expression,
                        visited_functions,
                    ),
                );
                Self::merge_possible_throw_kind(
                    &mut kind,
                    self.resolve_possible_throw_kind_from_expression(
                        else_expression,
                        visited_functions,
                    ),
                );
                kind
            }
            Expression::Sequence(expressions) => {
                let mut kind = None;
                for expression in expressions {
                    Self::merge_possible_throw_kind(
                        &mut kind,
                        self.resolve_possible_throw_kind_from_expression(
                            expression,
                            visited_functions,
                        ),
                    );
                }
                kind
            }
            Expression::Array(elements) => {
                let mut kind = None;
                for element in elements {
                    let expression = match element {
                        ArrayElement::Expression(expression) | ArrayElement::Spread(expression) => {
                            expression
                        }
                    };
                    Self::merge_possible_throw_kind(
                        &mut kind,
                        self.resolve_possible_throw_kind_from_expression(
                            expression,
                            visited_functions,
                        ),
                    );
                }
                kind
            }
            Expression::Object(entries) => {
                let mut kind = None;
                for entry in entries {
                    match entry {
                        ObjectEntry::Data { key, value } => {
                            Self::merge_possible_throw_kind(
                                &mut kind,
                                self.resolve_possible_throw_kind_from_expression(
                                    key,
                                    visited_functions,
                                ),
                            );
                            Self::merge_possible_throw_kind(
                                &mut kind,
                                self.resolve_possible_throw_kind_from_expression(
                                    value,
                                    visited_functions,
                                ),
                            );
                        }
                        ObjectEntry::Getter { key, getter }
                        | ObjectEntry::Setter {
                            key,
                            setter: getter,
                        } => {
                            Self::merge_possible_throw_kind(
                                &mut kind,
                                self.resolve_possible_throw_kind_from_expression(
                                    key,
                                    visited_functions,
                                ),
                            );
                            Self::merge_possible_throw_kind(
                                &mut kind,
                                self.resolve_possible_throw_kind_from_expression(
                                    getter,
                                    visited_functions,
                                ),
                            );
                        }
                        ObjectEntry::Spread(value) => {
                            Self::merge_possible_throw_kind(
                                &mut kind,
                                self.resolve_possible_throw_kind_from_expression(
                                    value,
                                    visited_functions,
                                ),
                            );
                        }
                    }
                }
                kind
            }
            Expression::SuperCall { callee, arguments } => {
                let mut kind =
                    self.resolve_possible_throw_kind_from_expression(callee, visited_functions);
                for argument in arguments {
                    Self::merge_possible_throw_kind(
                        &mut kind,
                        self.resolve_possible_throw_kind_from_expression(
                            argument.expression(),
                            visited_functions,
                        ),
                    );
                }
                kind
            }
            Expression::This
            | Expression::Sent
            | Expression::NewTarget
            | Expression::Update { .. }
            | Expression::Number(_)
            | Expression::BigInt(_)
            | Expression::String(_)
            | Expression::Bool(_)
            | Expression::Null
            | Expression::Undefined
            | Expression::Identifier(_) => None,
        }
    }

    pub(in crate::backend::direct_wasm) fn resolve_terminal_throw_value_from_try_body(
        &self,
        body: &[Statement],
    ) -> Option<Expression> {
        if let Some(value) = self.resolve_using_scope_terminal_throw_value(body) {
            return Some(value);
        }

        let Some((last, prefix)) = body.split_last() else {
            return None;
        };
        if !prefix
            .iter()
            .all(|statement| self.statement_preserves_try_metadata_before_terminal_throw(statement))
            || !self.statement_has_deterministic_terminal_throw(last)
        {
            return None;
        }
        self.resolve_terminal_throw_value_from_statement(last)
    }

    fn new_expression_produces_default_constructor_instance(
        &self,
        callee: &Expression,
        arguments: &[CallArgument],
    ) -> bool {
        let Some(LocalFunctionBinding::User(function_name)) =
            self.resolve_function_binding_from_expression(callee)
        else {
            return false;
        };
        let Some(user_function) = self.user_function(&function_name) else {
            return false;
        };
        if self.user_function_is_derived_constructor(user_function) {
            return false;
        }

        let capture_source_bindings =
            self.resolve_constructor_capture_source_bindings_from_expression(callee);
        if let Some(return_expression) = self
            .resolve_user_constructor_explicit_return_expression_for_function(
                user_function,
                arguments,
                capture_source_bindings.as_ref(),
            )
        {
            return matches!(
                self.infer_value_kind(&return_expression),
                Some(
                    StaticValueKind::Number
                        | StaticValueKind::Bool
                        | StaticValueKind::String
                        | StaticValueKind::BigInt
                        | StaticValueKind::Null
                        | StaticValueKind::Undefined
                        | StaticValueKind::Symbol
                )
            );
        }

        self.resolve_user_constructor_object_binding_from_new(callee, arguments)
            .is_some()
    }

    fn thrown_expression_can_bind_static_catch_value(&self, expression: &Expression) -> bool {
        if inline_summary_side_effect_free_expression(expression) {
            return true;
        }
        match expression {
            Expression::New { callee, arguments } => {
                self.new_expression_produces_default_constructor_instance(callee, arguments)
            }
            _ => false,
        }
    }

    fn resolve_static_binary_outcome_for_catch_scan_with_state(
        &self,
        op: BinaryOp,
        left: &Expression,
        right: &Expression,
        environment: &mut StaticResolutionEnvironment,
    ) -> Option<StaticEvalOutcome> {
        let left_value = self.resolve_static_expression_value_with_state(left, environment);
        let right_value = self.resolve_static_expression_value_with_state(right, environment);
        let current_function_name = self.current_function_name();
        match op {
            BinaryOp::Add => self.resolve_static_addition_outcome_with_context(
                &left_value,
                &right_value,
                current_function_name,
            ),
            BinaryOp::Subtract
            | BinaryOp::Multiply
            | BinaryOp::Divide
            | BinaryOp::Modulo
            | BinaryOp::Exponentiate
            | BinaryOp::BitwiseAnd
            | BinaryOp::BitwiseOr
            | BinaryOp::BitwiseXor
            | BinaryOp::LeftShift
            | BinaryOp::RightShift
            | BinaryOp::UnsignedRightShift => self
                .resolve_static_numeric_binary_outcome_with_context(
                    op,
                    &left_value,
                    &right_value,
                    current_function_name,
                ),
            BinaryOp::LessThan
            | BinaryOp::LessThanOrEqual
            | BinaryOp::GreaterThan
            | BinaryOp::GreaterThanOrEqual => self.resolve_static_relational_outcome_with_context(
                op,
                &left_value,
                &right_value,
                current_function_name,
            ),
            _ => None,
        }
    }

    fn expression_statement_is_safe_for_static_catch_scan_with_state(
        &self,
        expression: &Expression,
        environment: &mut StaticResolutionEnvironment,
    ) -> bool {
        if !inline_summary_side_effect_free_expression(expression) {
            return false;
        }
        match expression {
            Expression::Binary { op, left, right }
                if matches!(
                    op,
                    BinaryOp::Add
                        | BinaryOp::Subtract
                        | BinaryOp::Multiply
                        | BinaryOp::Divide
                        | BinaryOp::Modulo
                        | BinaryOp::Exponentiate
                        | BinaryOp::BitwiseAnd
                        | BinaryOp::BitwiseOr
                        | BinaryOp::BitwiseXor
                        | BinaryOp::LeftShift
                        | BinaryOp::RightShift
                        | BinaryOp::UnsignedRightShift
                        | BinaryOp::LessThan
                        | BinaryOp::LessThanOrEqual
                        | BinaryOp::GreaterThan
                        | BinaryOp::GreaterThanOrEqual
                ) =>
            {
                let mut referenced_names = HashSet::new();
                collect_referenced_binding_names_from_expression(left, &mut referenced_names);
                collect_referenced_binding_names_from_expression(right, &mut referenced_names);
                if referenced_names.iter().any(|name| {
                    let source_name = scoped_binding_source_name(name).unwrap_or(name);
                    environment.contains_object_binding(name)
                        || environment.contains_object_binding(source_name)
                }) {
                    return false;
                }
                matches!(
                    self.resolve_static_binary_outcome_for_catch_scan_with_state(
                        *op,
                        left,
                        right,
                        environment,
                    ),
                    Some(StaticEvalOutcome::Value(_))
                )
            }
            _ => true,
        }
    }

    pub(in crate::backend::direct_wasm) fn resolve_static_catch_value_from_try_body(
        &self,
        body: &[Statement],
    ) -> Option<Expression> {
        let mut environment = self.snapshot_static_resolution_environment();
        for statement in body {
            match self
                .resolve_static_catch_value_from_statement_with_state(statement, &mut environment)
            {
                StaticCatchScanOutcome::CatchValue(value) => return Some(value),
                StaticCatchScanOutcome::Continue => {}
                StaticCatchScanOutcome::Unsupported => return None,
            }
        }
        None
    }

    fn resolve_static_catch_value_from_statement_with_state(
        &self,
        statement: &Statement,
        environment: &mut StaticResolutionEnvironment,
    ) -> StaticCatchScanOutcome {
        match statement {
            Statement::Declaration { body }
            | Statement::Block { body }
            | Statement::Labeled { body, .. } => {
                self.resolve_static_catch_value_from_statements_with_state(body, environment)
            }
            Statement::With { object, body }
                if inline_summary_side_effect_free_expression(object) =>
            {
                if self
                    .resolve_terminal_expression_throw_value_with_state(object, environment)
                    .is_some()
                {
                    return StaticCatchScanOutcome::Unsupported;
                }
                self.resolve_static_catch_value_from_statements_with_state(body, environment)
            }
            Statement::Var { name, value }
            | Statement::Let { name, value, .. }
            | Statement::Assign { name, value } => {
                if let Some(throw_value) =
                    self.resolve_terminal_expression_throw_value_with_state(value, environment)
                {
                    return StaticCatchScanOutcome::CatchValue(throw_value);
                }
                if !inline_summary_side_effect_free_expression(value) {
                    return StaticCatchScanOutcome::Unsupported;
                }
                let value = self.resolve_static_expression_value_with_state(value, environment);
                environment.assign_binding_value(name.clone(), value.clone());
                let object_binding =
                    self.resolve_object_binding_from_expression_with_state(&value, environment);
                environment.sync_object_binding(name, object_binding);
                StaticCatchScanOutcome::Continue
            }
            Statement::Expression(expression) => self
                .resolve_terminal_expression_throw_value_with_state(expression, environment)
                .map(StaticCatchScanOutcome::CatchValue)
                .unwrap_or_else(|| {
                    if self.expression_statement_is_safe_for_static_catch_scan_with_state(
                        expression,
                        environment,
                    ) {
                        StaticCatchScanOutcome::Continue
                    } else {
                        StaticCatchScanOutcome::Unsupported
                    }
                }),
            Statement::Throw(expression)
                if self.thrown_expression_can_bind_static_catch_value(expression) =>
            {
                StaticCatchScanOutcome::CatchValue(
                    self.resolve_static_expression_value_with_state(expression, environment),
                )
            }
            Statement::If {
                condition,
                then_branch,
                else_branch,
            } => match self.resolve_static_expression_value_with_state(condition, environment) {
                Expression::Bool(true) => self
                    .resolve_static_catch_value_from_statements_with_state(
                        then_branch,
                        environment,
                    ),
                Expression::Bool(false) => self
                    .resolve_static_catch_value_from_statements_with_state(
                        else_branch,
                        environment,
                    ),
                _ => StaticCatchScanOutcome::Unsupported,
            },
            _ => StaticCatchScanOutcome::Unsupported,
        }
    }

    fn resolve_static_catch_value_from_statements_with_state(
        &self,
        body: &[Statement],
        environment: &mut StaticResolutionEnvironment,
    ) -> StaticCatchScanOutcome {
        for statement in body {
            match self.resolve_static_catch_value_from_statement_with_state(statement, environment)
            {
                StaticCatchScanOutcome::Continue => {}
                outcome => return outcome,
            }
        }
        StaticCatchScanOutcome::Continue
    }

    fn substitute_expression_identifier(
        expression: &Expression,
        name: &str,
        replacement: &Expression,
    ) -> Expression {
        match expression {
            Expression::Identifier(identifier) if identifier == name => replacement.clone(),
            Expression::Member { object, property } => Expression::Member {
                object: Box::new(Self::substitute_expression_identifier(
                    object,
                    name,
                    replacement,
                )),
                property: Box::new(Self::substitute_expression_identifier(
                    property,
                    name,
                    replacement,
                )),
            },
            Expression::SuperMember { property } => Expression::SuperMember {
                property: Box::new(Self::substitute_expression_identifier(
                    property,
                    name,
                    replacement,
                )),
            },
            Expression::Assign {
                name: target,
                value,
            } => Expression::Assign {
                name: target.clone(),
                value: Box::new(Self::substitute_expression_identifier(
                    value,
                    name,
                    replacement,
                )),
            },
            Expression::AssignMember {
                object,
                property,
                value,
            } => Expression::AssignMember {
                object: Box::new(Self::substitute_expression_identifier(
                    object,
                    name,
                    replacement,
                )),
                property: Box::new(Self::substitute_expression_identifier(
                    property,
                    name,
                    replacement,
                )),
                value: Box::new(Self::substitute_expression_identifier(
                    value,
                    name,
                    replacement,
                )),
            },
            Expression::AssignSuperMember { property, value } => Expression::AssignSuperMember {
                property: Box::new(Self::substitute_expression_identifier(
                    property,
                    name,
                    replacement,
                )),
                value: Box::new(Self::substitute_expression_identifier(
                    value,
                    name,
                    replacement,
                )),
            },
            Expression::Unary { op, expression } => Expression::Unary {
                op: *op,
                expression: Box::new(Self::substitute_expression_identifier(
                    expression,
                    name,
                    replacement,
                )),
            },
            Expression::Binary { op, left, right } => Expression::Binary {
                op: *op,
                left: Box::new(Self::substitute_expression_identifier(
                    left,
                    name,
                    replacement,
                )),
                right: Box::new(Self::substitute_expression_identifier(
                    right,
                    name,
                    replacement,
                )),
            },
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => Expression::Conditional {
                condition: Box::new(Self::substitute_expression_identifier(
                    condition,
                    name,
                    replacement,
                )),
                then_expression: Box::new(Self::substitute_expression_identifier(
                    then_expression,
                    name,
                    replacement,
                )),
                else_expression: Box::new(Self::substitute_expression_identifier(
                    else_expression,
                    name,
                    replacement,
                )),
            },
            Expression::Sequence(expressions) => Expression::Sequence(
                expressions
                    .iter()
                    .map(|expression| {
                        Self::substitute_expression_identifier(expression, name, replacement)
                    })
                    .collect(),
            ),
            Expression::Call { callee, arguments } => Expression::Call {
                callee: Box::new(Self::substitute_expression_identifier(
                    callee,
                    name,
                    replacement,
                )),
                arguments: arguments
                    .iter()
                    .map(|argument| match argument {
                        CallArgument::Expression(expression) => CallArgument::Expression(
                            Self::substitute_expression_identifier(expression, name, replacement),
                        ),
                        CallArgument::Spread(expression) => CallArgument::Spread(
                            Self::substitute_expression_identifier(expression, name, replacement),
                        ),
                    })
                    .collect(),
            },
            Expression::SuperCall { callee, arguments } => Expression::SuperCall {
                callee: Box::new(Self::substitute_expression_identifier(
                    callee,
                    name,
                    replacement,
                )),
                arguments: arguments
                    .iter()
                    .map(|argument| match argument {
                        CallArgument::Expression(expression) => CallArgument::Expression(
                            Self::substitute_expression_identifier(expression, name, replacement),
                        ),
                        CallArgument::Spread(expression) => CallArgument::Spread(
                            Self::substitute_expression_identifier(expression, name, replacement),
                        ),
                    })
                    .collect(),
            },
            Expression::New { callee, arguments } => Expression::New {
                callee: Box::new(Self::substitute_expression_identifier(
                    callee,
                    name,
                    replacement,
                )),
                arguments: arguments
                    .iter()
                    .map(|argument| match argument {
                        CallArgument::Expression(expression) => CallArgument::Expression(
                            Self::substitute_expression_identifier(expression, name, replacement),
                        ),
                        CallArgument::Spread(expression) => CallArgument::Spread(
                            Self::substitute_expression_identifier(expression, name, replacement),
                        ),
                    })
                    .collect(),
            },
            Expression::Object(entries) => Expression::Object(
                entries
                    .iter()
                    .map(|entry| match entry {
                        ObjectEntry::Data { key, value } => ObjectEntry::Data {
                            key: Self::substitute_expression_identifier(key, name, replacement),
                            value: Self::substitute_expression_identifier(value, name, replacement),
                        },
                        ObjectEntry::Getter { key, getter } => ObjectEntry::Getter {
                            key: Self::substitute_expression_identifier(key, name, replacement),
                            getter: Self::substitute_expression_identifier(
                                getter,
                                name,
                                replacement,
                            ),
                        },
                        ObjectEntry::Setter { key, setter } => ObjectEntry::Setter {
                            key: Self::substitute_expression_identifier(key, name, replacement),
                            setter: Self::substitute_expression_identifier(
                                setter,
                                name,
                                replacement,
                            ),
                        },
                        ObjectEntry::Spread(value) => ObjectEntry::Spread(
                            Self::substitute_expression_identifier(value, name, replacement),
                        ),
                    })
                    .collect(),
            ),
            Expression::Array(elements) => Expression::Array(
                elements
                    .iter()
                    .map(|element| match element {
                        ArrayElement::Expression(expression) => ArrayElement::Expression(
                            Self::substitute_expression_identifier(expression, name, replacement),
                        ),
                        ArrayElement::Spread(expression) => ArrayElement::Spread(
                            Self::substitute_expression_identifier(expression, name, replacement),
                        ),
                    })
                    .collect(),
            ),
            Expression::Await(expression) => Expression::Await(Box::new(
                Self::substitute_expression_identifier(expression, name, replacement),
            )),
            Expression::EnumerateKeys(expression) => Expression::EnumerateKeys(Box::new(
                Self::substitute_expression_identifier(expression, name, replacement),
            )),
            Expression::GetIterator(expression) => Expression::GetIterator(Box::new(
                Self::substitute_expression_identifier(expression, name, replacement),
            )),
            Expression::IteratorClose(expression) => Expression::IteratorClose(Box::new(
                Self::substitute_expression_identifier(expression, name, replacement),
            )),
            _ => expression.clone(),
        }
    }

    fn collect_using_try_body_static_assignments(
        statements: &[Statement],
    ) -> HashMap<String, Expression> {
        let mut bindings = HashMap::new();
        for statement in statements {
            match statement {
                Statement::Assign { name, value } | Statement::Let { name, value, .. } => {
                    bindings.insert(name.clone(), value.clone());
                }
                Statement::Throw(_) => break,
                _ => {}
            }
        }
        bindings
    }

    fn resolve_using_static_local_expression(
        expression: &Expression,
        bindings: &HashMap<String, Expression>,
    ) -> Expression {
        match expression {
            Expression::Identifier(name) => bindings
                .get(name)
                .cloned()
                .unwrap_or_else(|| expression.clone()),
            _ => expression.clone(),
        }
    }

    fn resolve_using_member_function_binding(
        &self,
        object: &Expression,
        property: &Expression,
        bindings: &HashMap<String, Expression>,
    ) -> Option<LocalFunctionBinding> {
        if let Some(binding) = self.resolve_member_function_binding(object, property) {
            return Some(binding);
        }

        let resolved_object = Self::resolve_using_static_local_expression(object, bindings);
        if !static_expression_matches(&resolved_object, object)
            && let Some(binding) = self.resolve_member_function_binding(&resolved_object, property)
        {
            return Some(binding);
        }

        let object_binding = self.resolve_object_binding_from_expression(&resolved_object)?;
        let value = self.resolve_object_binding_property_value(&object_binding, property)?;
        self.resolve_function_binding_from_expression(&value)
    }

    fn resolve_using_call_throw_value(
        &self,
        expression: &Expression,
        bindings: &HashMap<String, Expression>,
    ) -> Option<Expression> {
        let Expression::Call { callee, arguments } = expression else {
            return self.resolve_terminal_expression_throw_value(expression);
        };
        let argument_expressions = arguments
            .iter()
            .map(|argument| match argument {
                CallArgument::Expression(expression) => Some(expression.clone()),
                CallArgument::Spread(_) => None,
            })
            .collect::<Option<Vec<_>>>()?;
        let binding = match callee.as_ref() {
            Expression::Member { object, property } => {
                self.resolve_using_member_function_binding(object, property, bindings)?
            }
            _ => self.resolve_function_binding_from_expression(callee)?,
        };
        match self
            .resolve_terminal_function_outcome_from_binding(&binding, &argument_expressions)?
        {
            StaticEvalOutcome::Throw(throw_value) => {
                self.resolve_static_throw_value_expression(&throw_value)
            }
            _ => None,
        }
    }

    fn resolve_using_dispose_throw_value(
        &self,
        body: &[Statement],
        bindings: &HashMap<String, Expression>,
    ) -> Option<Expression> {
        if let [Statement::If { then_branch, .. }] = body {
            if let [Statement::Expression(expression)] = then_branch.as_slice() {
                return self.resolve_using_call_throw_value(expression, bindings);
            }
            return self.resolve_terminal_throw_value_from_try_body(then_branch);
        }
        self.resolve_terminal_throw_value_from_try_body(body)
    }

    fn resolve_using_scope_terminal_throw_value(&self, body: &[Statement]) -> Option<Expression> {
        for statement in body.iter().rev() {
            let Statement::Try {
                body: try_body,
                catch_binding: Some(completion_name),
                catch_body,
                ..
            } = statement
            else {
                continue;
            };
            if !is_using_completion_binding(completion_name) {
                continue;
            }
            if !matches!(
                catch_body.last(),
                Some(Statement::Throw(Expression::Identifier(name))) if name == completion_name
            ) {
                continue;
            }

            let using_body_bindings = Self::collect_using_try_body_static_assignments(try_body);
            let mut completion = self.resolve_terminal_throw_value_from_try_body(try_body)?;
            for catch_statement in catch_body {
                let Statement::If { then_branch, .. } = catch_statement else {
                    continue;
                };
                for finalizer_statement in then_branch {
                    let Statement::Try {
                        body: dispose_body,
                        catch_binding: Some(disposal_error_name),
                        catch_body: dispose_catch_body,
                        ..
                    } = finalizer_statement
                    else {
                        continue;
                    };
                    let disposal_error = self
                        .resolve_using_dispose_throw_value(dispose_body, &using_body_bindings)
                        .unwrap_or_else(|| Expression::Identifier(disposal_error_name.clone()));
                    let Some(Statement::Assign { name, value }) = dispose_catch_body.first() else {
                        continue;
                    };
                    if name != completion_name {
                        continue;
                    }
                    let with_completion =
                        Self::substitute_expression_identifier(value, completion_name, &completion);
                    completion = Self::substitute_expression_identifier(
                        &with_completion,
                        disposal_error_name,
                        &disposal_error,
                    );
                }
            }
            return Some(completion);
        }

        None
    }

    fn resolve_terminal_throw_value_from_statement(
        &self,
        statement: &Statement,
    ) -> Option<Expression> {
        match statement {
            Statement::Declaration { body }
            | Statement::Block { body }
            | Statement::Labeled { body, .. } => {
                self.resolve_terminal_throw_value_from_try_body(body)
            }
            Statement::With { object, body }
                if inline_summary_side_effect_free_expression(object) =>
            {
                self.resolve_terminal_throw_value_from_try_body(body)
            }
            Statement::Throw(expression) => Some(expression.clone()),
            Statement::Expression(expression) => {
                self.resolve_terminal_expression_throw_value(expression)
            }
            Statement::For {
                init,
                condition,
                update,
                body,
                ..
            } => {
                for statement in init {
                    if let Some(throw_value) =
                        self.resolve_terminal_throw_value_from_statement(statement)
                    {
                        return Some(throw_value);
                    }
                }
                if let Some(condition) = condition
                    && let Some(throw_value) =
                        self.resolve_terminal_expression_throw_value(condition)
                {
                    return Some(throw_value);
                }
                for statement in body {
                    if let Some(throw_value) =
                        self.resolve_terminal_throw_value_from_statement(statement)
                    {
                        return Some(throw_value);
                    }
                }
                update
                    .as_ref()
                    .and_then(|expression| self.resolve_terminal_expression_throw_value(expression))
            }
            _ => None,
        }
    }

    #[track_caller]
    pub(in crate::backend::direct_wasm) fn emit_throw_from_locals(&mut self) -> DirectResult<()> {
        if std::env::var_os("AYY_TRACE_THROW_SITES").is_some() {
            let caller = std::panic::Location::caller();
            self.emit_print(&[Expression::String(format!(
                "throw_from_locals fn={:?} allow_return={} try_depth={} instruction={} caller={}:{}",
                self.current_function_name(),
                self.state.runtime.behavior.allow_return,
                self.state.emission.control_flow.try_stack.len(),
                self.state.emission.output.instructions.len(),
                caller.file(),
                caller.line()
            ))])?;
        }
        self.push_local_get(self.state.runtime.throws.throw_value_local);
        self.push_global_set(THROW_VALUE_GLOBAL_INDEX);
        self.push_local_get(self.state.runtime.throws.throw_tag_local);
        self.push_global_set(THROW_TAG_GLOBAL_INDEX);

        let Some(try_context) = self.state.emission.control_flow.try_stack.last() else {
            if self.state.runtime.behavior.allow_return {
                self.push_i32_const(JS_UNDEFINED_TAG);
                self.clear_local_throw_state();
                self.state.emission.output.instructions.push(0x0f);
                return Ok(());
            }
            self.emit_uncaught_throw_report_from_locals()?;
            self.state.emission.output.instructions.push(0x00);
            return Ok(());
        };

        self.push_br(self.relative_depth(try_context.catch_target));
        Ok(())
    }

    pub(in crate::backend::direct_wasm) fn emit_check_global_throw_for_user_call(
        &mut self,
    ) -> DirectResult<()> {
        self.push_global_get(THROW_TAG_GLOBAL_INDEX);
        self.push_i32_const(0);
        self.push_binary_op(BinaryOp::NotEqual)?;
        self.state.emission.output.instructions.push(0x04);
        self.state
            .emission
            .output
            .instructions
            .push(EMPTY_BLOCK_TYPE);
        self.push_control_frame();

        if std::env::var_os("AYY_TRACE_THROW_SITES").is_some() {
            self.emit_print(&[Expression::String(format!(
                "check_global_throw fn={:?} allow_return={} try_depth={} instruction={}",
                self.current_function_name(),
                self.state.runtime.behavior.allow_return,
                self.state.emission.control_flow.try_stack.len(),
                self.state.emission.output.instructions.len()
            ))])?;
        }

        self.push_global_get(THROW_VALUE_GLOBAL_INDEX);
        self.push_local_set(self.state.runtime.throws.throw_value_local);
        self.push_global_get(THROW_TAG_GLOBAL_INDEX);
        self.push_local_set(self.state.runtime.throws.throw_tag_local);

        let Some(catch_target) = self
            .state
            .emission
            .control_flow
            .try_stack
            .last()
            .map(|try_context| try_context.catch_target)
        else {
            if self.state.runtime.behavior.allow_return {
                self.push_i32_const(JS_UNDEFINED_TAG);
                self.state.emission.output.instructions.push(0x0f);
            } else {
                self.emit_uncaught_throw_report_from_locals()?;
                self.state.emission.output.instructions.push(0x00);
            }
            self.state.emission.output.instructions.push(0x0b);
            self.pop_control_frame();
            return Ok(());
        };

        self.clear_global_throw_state();
        self.push_br(self.relative_depth(catch_target));
        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();
        Ok(())
    }

    pub(in crate::backend::direct_wasm) fn clear_local_throw_state(&mut self) {
        self.push_i32_const(0);
        self.push_local_set(self.state.runtime.throws.throw_tag_local);
        self.push_i32_const(0);
        self.push_local_set(self.state.runtime.throws.throw_value_local);
    }

    pub(in crate::backend::direct_wasm) fn clear_global_throw_state(&mut self) {
        self.push_i32_const(0);
        self.push_global_set(THROW_TAG_GLOBAL_INDEX);
        self.push_i32_const(0);
        self.push_global_set(THROW_VALUE_GLOBAL_INDEX);
    }

    #[track_caller]
    pub(in crate::backend::direct_wasm) fn emit_error_throw(&mut self) -> DirectResult<()> {
        if std::env::var_os("AYY_TRACE_THROW_SITES").is_some() {
            let caller = std::panic::Location::caller();
            self.emit_print(&[Expression::String(format!(
                "throw_site name=Error fn={:?} instruction={} caller={}:{}",
                self.current_function_name(),
                self.state.emission.output.instructions.len(),
                caller.file(),
                caller.line()
            ))])?;
        }
        self.push_i32_const(JS_TYPEOF_OBJECT_TAG);
        self.push_local_set(self.state.runtime.throws.throw_value_local);
        self.push_i32_const(1);
        self.push_local_set(self.state.runtime.throws.throw_tag_local);
        self.emit_throw_from_locals()
    }

    #[track_caller]
    pub(in crate::backend::direct_wasm) fn emit_named_error_throw(
        &mut self,
        name: &str,
    ) -> DirectResult<()> {
        if std::env::var_os("AYY_TRACE_THROW_SITES").is_some() {
            let caller = std::panic::Location::caller();
            self.emit_print(&[Expression::String(format!(
                "throw_site name={name} fn={:?} instruction={} caller={}:{}",
                self.current_function_name(),
                self.state.emission.output.instructions.len(),
                caller.file(),
                caller.line()
            ))])?;
        }
        if let Some(value) = native_error_runtime_value(name) {
            self.push_i32_const(value);
            self.push_local_set(self.state.runtime.throws.throw_value_local);
            self.push_i32_const(1);
            self.push_local_set(self.state.runtime.throws.throw_tag_local);
            return self.emit_throw_from_locals();
        }
        if name == "Test262Error" {
            self.push_i32_const(TEST262_ERROR_RUNTIME_VALUE);
            self.push_local_set(self.state.runtime.throws.throw_value_local);
            self.push_i32_const(1);
            self.push_local_set(self.state.runtime.throws.throw_tag_local);
            return self.emit_throw_from_locals();
        }

        self.emit_error_throw()
    }

    pub(in crate::backend::direct_wasm) fn emit_static_throw_value(
        &mut self,
        throw_value: &StaticThrowValue,
    ) -> DirectResult<()> {
        match throw_value {
            StaticThrowValue::Value(expression) => {
                self.emit_numeric_expression(expression)?;
                self.push_local_set(self.state.runtime.throws.throw_value_local);
                self.push_i32_const(1);
                self.push_local_set(self.state.runtime.throws.throw_tag_local);
                self.emit_throw_from_locals()
            }
            StaticThrowValue::NamedError(name) => self.emit_named_error_throw(name),
        }
    }

    pub(in crate::backend::direct_wasm) fn emit_static_eval_outcome(
        &mut self,
        outcome: &StaticEvalOutcome,
    ) -> DirectResult<()> {
        match outcome {
            StaticEvalOutcome::Value(expression) => self.emit_numeric_expression(expression),
            StaticEvalOutcome::Throw(throw_value) => self.emit_static_throw_value(throw_value),
        }
    }
}
