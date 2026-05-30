use super::*;

fn expression_calls_user_function(expression: &Expression, names: &HashSet<String>) -> bool {
    match expression {
        Expression::Call { callee, arguments }
        | Expression::SuperCall { callee, arguments }
        | Expression::New { callee, arguments } => {
            expression_references_user_function(callee, names)
                || expression_calls_user_function(callee, names)
                || arguments.iter().any(|argument| match argument {
                    CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                        expression_calls_user_function(expression, names)
                    }
                })
        }
        Expression::Array(elements) => elements.iter().any(|element| match element {
            ArrayElement::Expression(expression) | ArrayElement::Spread(expression) => {
                expression_calls_user_function(expression, names)
            }
        }),
        Expression::Object(entries) => entries.iter().any(|entry| match entry {
            ObjectEntry::Data { key, value } => {
                expression_calls_user_function(key, names)
                    || expression_calls_user_function(value, names)
            }
            ObjectEntry::Getter { key, getter } => {
                expression_calls_user_function(key, names)
                    || expression_calls_user_function(getter, names)
            }
            ObjectEntry::Setter { key, setter } => {
                expression_calls_user_function(key, names)
                    || expression_calls_user_function(setter, names)
            }
            ObjectEntry::Spread(expression) => expression_calls_user_function(expression, names),
        }),
        Expression::Member { object, property } => {
            expression_calls_user_function(object, names)
                || expression_calls_user_function(property, names)
        }
        Expression::SuperMember { property } => expression_calls_user_function(property, names),
        Expression::Assign { value, .. }
        | Expression::Await(value)
        | Expression::EnumerateKeys(value)
        | Expression::GetIterator(value)
        | Expression::IteratorClose(value)
        | Expression::Unary {
            expression: value, ..
        } => expression_calls_user_function(value, names),
        Expression::AssignMember {
            object,
            property,
            value,
        } => {
            expression_calls_user_function(object, names)
                || expression_calls_user_function(property, names)
                || expression_calls_user_function(value, names)
        }
        Expression::AssignSuperMember { property, value } => {
            expression_calls_user_function(property, names)
                || expression_calls_user_function(value, names)
        }
        Expression::Binary { left, right, .. } => {
            expression_calls_user_function(left, names)
                || expression_calls_user_function(right, names)
        }
        Expression::Conditional {
            condition,
            then_expression,
            else_expression,
        } => {
            expression_calls_user_function(condition, names)
                || expression_calls_user_function(then_expression, names)
                || expression_calls_user_function(else_expression, names)
        }
        Expression::Sequence(expressions) => expressions
            .iter()
            .any(|expression| expression_calls_user_function(expression, names)),
        Expression::Update { .. }
        | Expression::Identifier(_)
        | Expression::Number(_)
        | Expression::BigInt(_)
        | Expression::String(_)
        | Expression::Bool(_)
        | Expression::Null
        | Expression::Undefined
        | Expression::NewTarget
        | Expression::This
        | Expression::Sent => false,
    }
}

fn statement_calls_user_function(statement: &Statement, names: &HashSet<String>) -> bool {
    match statement {
        Statement::Declaration { body }
        | Statement::Block { body }
        | Statement::Labeled { body, .. } => body
            .iter()
            .any(|statement| statement_calls_user_function(statement, names)),
        Statement::Var { value, .. }
        | Statement::Let { value, .. }
        | Statement::Assign { value, .. }
        | Statement::Expression(value)
        | Statement::Throw(value)
        | Statement::Return(value)
        | Statement::Yield { value }
        | Statement::YieldDelegate { value } => expression_calls_user_function(value, names),
        Statement::AssignMember {
            object,
            property,
            value,
        } => {
            expression_calls_user_function(object, names)
                || expression_calls_user_function(property, names)
                || expression_calls_user_function(value, names)
        }
        Statement::Print { values } => values
            .iter()
            .any(|value| expression_calls_user_function(value, names)),
        Statement::With { object, body } => {
            expression_calls_user_function(object, names)
                || body
                    .iter()
                    .any(|statement| statement_calls_user_function(statement, names))
        }
        Statement::If {
            condition,
            then_branch,
            else_branch,
        } => {
            expression_calls_user_function(condition, names)
                || then_branch
                    .iter()
                    .any(|statement| statement_calls_user_function(statement, names))
                || else_branch
                    .iter()
                    .any(|statement| statement_calls_user_function(statement, names))
        }
        Statement::Try {
            body,
            catch_setup,
            catch_body,
            ..
        } => body
            .iter()
            .chain(catch_setup.iter())
            .chain(catch_body.iter())
            .any(|statement| statement_calls_user_function(statement, names)),
        Statement::Switch {
            discriminant,
            cases,
            ..
        } => {
            expression_calls_user_function(discriminant, names)
                || cases.iter().any(|case| {
                    case.test
                        .as_ref()
                        .is_some_and(|test| expression_calls_user_function(test, names))
                        || case
                            .body
                            .iter()
                            .any(|statement| statement_calls_user_function(statement, names))
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
                .any(|statement| statement_calls_user_function(statement, names))
                || condition
                    .as_ref()
                    .is_some_and(|condition| expression_calls_user_function(condition, names))
                || update
                    .as_ref()
                    .is_some_and(|update| expression_calls_user_function(update, names))
                || break_hook
                    .as_ref()
                    .is_some_and(|break_hook| expression_calls_user_function(break_hook, names))
                || body
                    .iter()
                    .any(|statement| statement_calls_user_function(statement, names))
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
            expression_calls_user_function(condition, names)
                || break_hook
                    .as_ref()
                    .is_some_and(|break_hook| expression_calls_user_function(break_hook, names))
                || body
                    .iter()
                    .any(|statement| statement_calls_user_function(statement, names))
        }
        Statement::Break { .. } | Statement::Continue { .. } => false,
    }
}

impl<'a> FunctionCompiler<'a> {
    pub(in crate::backend::direct_wasm) fn merge_bound_snapshot_updated_bindings(
        bindings: &mut HashMap<String, Expression>,
        updated_bindings: HashMap<String, Expression>,
    ) {
        for (name, value) in updated_bindings {
            bindings.insert(name, value);
        }
    }

    fn collect_bound_snapshot_returned_capture_names_from_function_expression(
        &self,
        expression: &Expression,
        current_function_name: Option<&str>,
        names: &mut HashSet<String>,
    ) {
        let Some(LocalFunctionBinding::User(function_name)) = self
            .resolve_function_binding_from_expression_with_context(
                expression,
                current_function_name,
            )
        else {
            return;
        };
        if let Some(captures) = self
            .backend
            .function_registry
            .analysis
            .user_function_capture_bindings
            .get(&function_name)
        {
            names.extend(captures.keys().cloned());
        }
        let mut visited_functions = HashSet::new();
        self.collect_bound_snapshot_returned_capture_names_from_function_returns(
            &function_name,
            names,
            &mut visited_functions,
        );
    }

    fn collect_bound_snapshot_returned_capture_names_from_function_returns(
        &self,
        function_name: &str,
        names: &mut HashSet<String>,
        visited_functions: &mut HashSet<String>,
    ) {
        if !visited_functions.insert(function_name.to_string()) {
            return;
        }
        let Some(function) = self.resolve_registered_function_declaration(function_name) else {
            return;
        };
        for statement in &function.body {
            self.collect_bound_snapshot_returned_capture_names_from_statement_returns(
                statement,
                Some(function_name),
                names,
                visited_functions,
            );
        }
    }

    fn collect_bound_snapshot_returned_capture_names_from_statement_returns(
        &self,
        statement: &Statement,
        current_function_name: Option<&str>,
        names: &mut HashSet<String>,
        visited_functions: &mut HashSet<String>,
    ) {
        match statement {
            Statement::Return(expression) | Statement::Throw(expression) => self
                .collect_bound_snapshot_returned_capture_names_from_return_expression(
                    expression,
                    current_function_name,
                    names,
                    visited_functions,
                ),
            Statement::Declaration { body }
            | Statement::Block { body }
            | Statement::Labeled { body, .. }
            | Statement::With { body, .. } => {
                for statement in body {
                    self.collect_bound_snapshot_returned_capture_names_from_statement_returns(
                        statement,
                        current_function_name,
                        names,
                        visited_functions,
                    );
                }
            }
            Statement::If {
                then_branch,
                else_branch,
                ..
            } => {
                for statement in then_branch {
                    self.collect_bound_snapshot_returned_capture_names_from_statement_returns(
                        statement,
                        current_function_name,
                        names,
                        visited_functions,
                    );
                }
                for statement in else_branch {
                    self.collect_bound_snapshot_returned_capture_names_from_statement_returns(
                        statement,
                        current_function_name,
                        names,
                        visited_functions,
                    );
                }
            }
            Statement::Try {
                body, catch_body, ..
            } => {
                for statement in body {
                    self.collect_bound_snapshot_returned_capture_names_from_statement_returns(
                        statement,
                        current_function_name,
                        names,
                        visited_functions,
                    );
                }
                for statement in catch_body {
                    self.collect_bound_snapshot_returned_capture_names_from_statement_returns(
                        statement,
                        current_function_name,
                        names,
                        visited_functions,
                    );
                }
            }
            Statement::Switch { cases, .. } => {
                for case in cases {
                    for statement in &case.body {
                        self.collect_bound_snapshot_returned_capture_names_from_statement_returns(
                            statement,
                            current_function_name,
                            names,
                            visited_functions,
                        );
                    }
                }
            }
            Statement::For { init, body, .. } => {
                for statement in init {
                    self.collect_bound_snapshot_returned_capture_names_from_statement_returns(
                        statement,
                        current_function_name,
                        names,
                        visited_functions,
                    );
                }
                for statement in body {
                    self.collect_bound_snapshot_returned_capture_names_from_statement_returns(
                        statement,
                        current_function_name,
                        names,
                        visited_functions,
                    );
                }
            }
            Statement::While { body, .. } | Statement::DoWhile { body, .. } => {
                for statement in body {
                    self.collect_bound_snapshot_returned_capture_names_from_statement_returns(
                        statement,
                        current_function_name,
                        names,
                        visited_functions,
                    );
                }
            }
            Statement::Expression(_)
            | Statement::Var { .. }
            | Statement::Let { .. }
            | Statement::Assign { .. }
            | Statement::AssignMember { .. }
            | Statement::Print { .. }
            | Statement::Yield { .. }
            | Statement::YieldDelegate { .. }
            | Statement::Break { .. }
            | Statement::Continue { .. } => {}
        }
    }

    fn collect_bound_snapshot_returned_capture_names_from_return_expression(
        &self,
        expression: &Expression,
        current_function_name: Option<&str>,
        names: &mut HashSet<String>,
        visited_functions: &mut HashSet<String>,
    ) {
        if let Some(LocalFunctionBinding::User(function_name)) = self
            .resolve_function_binding_from_expression_with_context(
                expression,
                current_function_name,
            )
        {
            if let Some(captures) = self
                .backend
                .function_registry
                .analysis
                .user_function_capture_bindings
                .get(&function_name)
            {
                names.extend(captures.keys().cloned());
            }
            self.collect_bound_snapshot_returned_capture_names_from_function_returns(
                &function_name,
                names,
                visited_functions,
            );
        }
        match expression {
            Expression::Array(elements) => {
                for element in elements {
                    match element {
                        ArrayElement::Expression(expression)
                        | ArrayElement::Spread(expression) => self
                            .collect_bound_snapshot_returned_capture_names_from_return_expression(
                                expression,
                                current_function_name,
                                names,
                                visited_functions,
                            ),
                    }
                }
            }
            Expression::Object(entries) => {
                for entry in entries {
                    match entry {
                        ObjectEntry::Data { key, value } => {
                            self.collect_bound_snapshot_returned_capture_names_from_return_expression(
                                key,
                                current_function_name,
                                names,
                                visited_functions,
                            );
                            self.collect_bound_snapshot_returned_capture_names_from_return_expression(
                                value,
                                current_function_name,
                                names,
                                visited_functions,
                            );
                        }
                        ObjectEntry::Getter { key, getter } => {
                            self.collect_bound_snapshot_returned_capture_names_from_return_expression(
                                key,
                                current_function_name,
                                names,
                                visited_functions,
                            );
                            self.collect_bound_snapshot_returned_capture_names_from_return_expression(
                                getter,
                                current_function_name,
                                names,
                                visited_functions,
                            );
                        }
                        ObjectEntry::Setter { key, setter } => {
                            self.collect_bound_snapshot_returned_capture_names_from_return_expression(
                                key,
                                current_function_name,
                                names,
                                visited_functions,
                            );
                            self.collect_bound_snapshot_returned_capture_names_from_return_expression(
                                setter,
                                current_function_name,
                                names,
                                visited_functions,
                            );
                        }
                        ObjectEntry::Spread(expression) => self
                            .collect_bound_snapshot_returned_capture_names_from_return_expression(
                                expression,
                                current_function_name,
                                names,
                                visited_functions,
                            ),
                    }
                }
            }
            Expression::Call { callee, arguments }
            | Expression::SuperCall { callee, arguments }
            | Expression::New { callee, arguments } => {
                self.collect_bound_snapshot_returned_capture_names_from_return_expression(
                    callee,
                    current_function_name,
                    names,
                    visited_functions,
                );
                for argument in arguments {
                    match argument {
                        CallArgument::Expression(expression)
                        | CallArgument::Spread(expression) => self
                            .collect_bound_snapshot_returned_capture_names_from_return_expression(
                                expression,
                                current_function_name,
                                names,
                                visited_functions,
                            ),
                    }
                }
            }
            Expression::Member { object, property } => {
                self.collect_bound_snapshot_returned_capture_names_from_return_expression(
                    object,
                    current_function_name,
                    names,
                    visited_functions,
                );
                self.collect_bound_snapshot_returned_capture_names_from_return_expression(
                    property,
                    current_function_name,
                    names,
                    visited_functions,
                );
            }
            Expression::SuperMember { property } => {
                self.collect_bound_snapshot_returned_capture_names_from_return_expression(
                    property,
                    current_function_name,
                    names,
                    visited_functions,
                );
            }
            Expression::Assign { value, .. }
            | Expression::Await(value)
            | Expression::EnumerateKeys(value)
            | Expression::GetIterator(value)
            | Expression::IteratorClose(value)
            | Expression::Unary {
                expression: value, ..
            } => self.collect_bound_snapshot_returned_capture_names_from_return_expression(
                value,
                current_function_name,
                names,
                visited_functions,
            ),
            Expression::AssignMember {
                object,
                property,
                value,
            } => {
                self.collect_bound_snapshot_returned_capture_names_from_return_expression(
                    object,
                    current_function_name,
                    names,
                    visited_functions,
                );
                self.collect_bound_snapshot_returned_capture_names_from_return_expression(
                    property,
                    current_function_name,
                    names,
                    visited_functions,
                );
                self.collect_bound_snapshot_returned_capture_names_from_return_expression(
                    value,
                    current_function_name,
                    names,
                    visited_functions,
                );
            }
            Expression::AssignSuperMember { property, value } => {
                self.collect_bound_snapshot_returned_capture_names_from_return_expression(
                    property,
                    current_function_name,
                    names,
                    visited_functions,
                );
                self.collect_bound_snapshot_returned_capture_names_from_return_expression(
                    value,
                    current_function_name,
                    names,
                    visited_functions,
                );
            }
            Expression::Binary { left, right, .. } => {
                self.collect_bound_snapshot_returned_capture_names_from_return_expression(
                    left,
                    current_function_name,
                    names,
                    visited_functions,
                );
                self.collect_bound_snapshot_returned_capture_names_from_return_expression(
                    right,
                    current_function_name,
                    names,
                    visited_functions,
                );
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                self.collect_bound_snapshot_returned_capture_names_from_return_expression(
                    condition,
                    current_function_name,
                    names,
                    visited_functions,
                );
                self.collect_bound_snapshot_returned_capture_names_from_return_expression(
                    then_expression,
                    current_function_name,
                    names,
                    visited_functions,
                );
                self.collect_bound_snapshot_returned_capture_names_from_return_expression(
                    else_expression,
                    current_function_name,
                    names,
                    visited_functions,
                );
            }
            Expression::Sequence(expressions) => {
                for expression in expressions {
                    self.collect_bound_snapshot_returned_capture_names_from_return_expression(
                        expression,
                        current_function_name,
                        names,
                        visited_functions,
                    );
                }
            }
            Expression::Update { .. }
            | Expression::Identifier(_)
            | Expression::Number(_)
            | Expression::BigInt(_)
            | Expression::String(_)
            | Expression::Bool(_)
            | Expression::Null
            | Expression::Undefined
            | Expression::NewTarget
            | Expression::Sent
            | Expression::This => {}
        }
    }

    fn collect_bound_snapshot_returned_capture_names_from_call_arguments(
        &self,
        arguments: &[CallArgument],
        current_function_name: Option<&str>,
        names: &mut HashSet<String>,
    ) {
        for argument in arguments {
            match argument {
                CallArgument::Expression(expression) | CallArgument::Spread(expression) => self
                    .collect_bound_snapshot_returned_capture_names_from_expression(
                        expression,
                        current_function_name,
                        names,
                    ),
            }
        }
    }

    fn collect_bound_snapshot_returned_capture_names_from_expression(
        &self,
        expression: &Expression,
        current_function_name: Option<&str>,
        names: &mut HashSet<String>,
    ) {
        self.collect_bound_snapshot_returned_capture_names_from_function_expression(
            expression,
            current_function_name,
            names,
        );
        match expression {
            Expression::Array(elements) => {
                for element in elements {
                    match element {
                        ArrayElement::Expression(expression) | ArrayElement::Spread(expression) => {
                            self.collect_bound_snapshot_returned_capture_names_from_expression(
                                expression,
                                current_function_name,
                                names,
                            )
                        }
                    }
                }
            }
            Expression::Object(entries) => {
                for entry in entries {
                    match entry {
                        ObjectEntry::Data { key, value } => {
                            self.collect_bound_snapshot_returned_capture_names_from_expression(
                                key,
                                current_function_name,
                                names,
                            );
                            self.collect_bound_snapshot_returned_capture_names_from_expression(
                                value,
                                current_function_name,
                                names,
                            );
                        }
                        ObjectEntry::Getter { key, getter } => {
                            self.collect_bound_snapshot_returned_capture_names_from_expression(
                                key,
                                current_function_name,
                                names,
                            );
                            self.collect_bound_snapshot_returned_capture_names_from_expression(
                                getter,
                                current_function_name,
                                names,
                            );
                        }
                        ObjectEntry::Setter { key, setter } => {
                            self.collect_bound_snapshot_returned_capture_names_from_expression(
                                key,
                                current_function_name,
                                names,
                            );
                            self.collect_bound_snapshot_returned_capture_names_from_expression(
                                setter,
                                current_function_name,
                                names,
                            );
                        }
                        ObjectEntry::Spread(expression) => self
                            .collect_bound_snapshot_returned_capture_names_from_expression(
                                expression,
                                current_function_name,
                                names,
                            ),
                    }
                }
            }
            Expression::Member { object, property } => {
                self.collect_bound_snapshot_returned_capture_names_from_expression(
                    object,
                    current_function_name,
                    names,
                );
                self.collect_bound_snapshot_returned_capture_names_from_expression(
                    property,
                    current_function_name,
                    names,
                );
            }
            Expression::SuperMember { property } => {
                self.collect_bound_snapshot_returned_capture_names_from_expression(
                    property,
                    current_function_name,
                    names,
                );
            }
            Expression::Assign { value, .. }
            | Expression::Await(value)
            | Expression::EnumerateKeys(value)
            | Expression::GetIterator(value)
            | Expression::IteratorClose(value)
            | Expression::Unary {
                expression: value, ..
            } => self.collect_bound_snapshot_returned_capture_names_from_expression(
                value,
                current_function_name,
                names,
            ),
            Expression::AssignMember {
                object,
                property,
                value,
            } => {
                self.collect_bound_snapshot_returned_capture_names_from_expression(
                    object,
                    current_function_name,
                    names,
                );
                self.collect_bound_snapshot_returned_capture_names_from_expression(
                    property,
                    current_function_name,
                    names,
                );
                self.collect_bound_snapshot_returned_capture_names_from_expression(
                    value,
                    current_function_name,
                    names,
                );
            }
            Expression::AssignSuperMember { property, value } => {
                self.collect_bound_snapshot_returned_capture_names_from_expression(
                    property,
                    current_function_name,
                    names,
                );
                self.collect_bound_snapshot_returned_capture_names_from_expression(
                    value,
                    current_function_name,
                    names,
                );
            }
            Expression::Binary { left, right, .. } => {
                self.collect_bound_snapshot_returned_capture_names_from_expression(
                    left,
                    current_function_name,
                    names,
                );
                self.collect_bound_snapshot_returned_capture_names_from_expression(
                    right,
                    current_function_name,
                    names,
                );
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                self.collect_bound_snapshot_returned_capture_names_from_expression(
                    condition,
                    current_function_name,
                    names,
                );
                self.collect_bound_snapshot_returned_capture_names_from_expression(
                    then_expression,
                    current_function_name,
                    names,
                );
                self.collect_bound_snapshot_returned_capture_names_from_expression(
                    else_expression,
                    current_function_name,
                    names,
                );
            }
            Expression::Sequence(expressions) => {
                for expression in expressions {
                    self.collect_bound_snapshot_returned_capture_names_from_expression(
                        expression,
                        current_function_name,
                        names,
                    );
                }
            }
            Expression::Call { callee, arguments }
            | Expression::SuperCall { callee, arguments }
            | Expression::New { callee, arguments } => {
                self.collect_bound_snapshot_returned_capture_names_from_expression(
                    callee,
                    current_function_name,
                    names,
                );
                self.collect_bound_snapshot_returned_capture_names_from_call_arguments(
                    arguments,
                    current_function_name,
                    names,
                );
            }
            Expression::Update { .. }
            | Expression::Identifier(_)
            | Expression::Number(_)
            | Expression::BigInt(_)
            | Expression::String(_)
            | Expression::Bool(_)
            | Expression::Null
            | Expression::Undefined
            | Expression::NewTarget
            | Expression::Sent
            | Expression::This => {}
        }
    }

    fn bound_snapshot_binding_value_for_capture_name(
        name: &str,
        bindings: &HashMap<String, Expression>,
    ) -> Option<Expression> {
        bindings
            .iter()
            .find(|(binding_name, _)| {
                scoped_binding_source_name(binding_name).is_some_and(|source| source == name)
            })
            .map(|(_, value)| value.clone())
            .or_else(|| bindings.get(name).cloned())
    }

    fn bound_snapshot_capture_alias_is_retained(
        alias: &str,
        capture_name: &str,
        capture_names: &HashSet<String>,
    ) -> bool {
        let alias_source = scoped_binding_source_name(alias).unwrap_or(alias);
        alias_source != capture_name && capture_names.contains(alias_source)
    }

    fn preserve_bound_snapshot_returned_capture_bindings(
        &self,
        outcome: &BoundSnapshotControlFlow,
        local_bindings: &HashMap<String, Expression>,
        materialized_bindings: &mut HashMap<String, Expression>,
        retained_names: &mut HashSet<String>,
        current_function_name: Option<&str>,
    ) {
        let value = match outcome {
            BoundSnapshotControlFlow::Return(value) | BoundSnapshotControlFlow::Throw(value) => {
                value
            }
            BoundSnapshotControlFlow::None | BoundSnapshotControlFlow::Break(_) => return,
        };
        let mut capture_names = HashSet::new();
        self.collect_bound_snapshot_returned_capture_names_from_expression(
            value,
            current_function_name,
            &mut capture_names,
        );
        for capture_name in &capture_names {
            if capture_name == "this" || capture_name == "arguments" {
                retained_names.insert(capture_name.clone());
                continue;
            }
            retained_names.insert(capture_name.clone());
            if let Some(value) =
                Self::bound_snapshot_binding_value_for_capture_name(capture_name, local_bindings)
            {
                if let Expression::Identifier(alias) = &value
                    && Self::bound_snapshot_capture_alias_is_retained(
                        alias,
                        capture_name,
                        &capture_names,
                    )
                {
                    materialized_bindings.insert(capture_name.clone(), value);
                    continue;
                }
            }
            if materialized_bindings.contains_key(capture_name) {
                continue;
            }
            let Some(value) =
                Self::bound_snapshot_binding_value_for_capture_name(&capture_name, local_bindings)
            else {
                continue;
            };
            let materialized = self
                .evaluate_bound_snapshot_expression(
                    &value,
                    &mut materialized_bindings.clone(),
                    current_function_name,
                )
                .unwrap_or(value);
            materialized_bindings.insert(capture_name.clone(), materialized);
        }
    }

    fn user_function_calls_captured_user_function(&self, user_function: &UserFunction) -> bool {
        if self
            .backend
            .function_registry
            .analysis
            .user_function_capture_bindings
            .is_empty()
        {
            return false;
        }
        let captured_user_function_names = self
            .backend
            .function_registry
            .analysis
            .user_function_capture_bindings
            .keys()
            .cloned()
            .collect::<HashSet<_>>();
        self.resolve_registered_function_declaration(&user_function.name)
            .is_some_and(|function| {
                function.body.iter().any(|statement| {
                    statement_calls_user_function(statement, &captured_user_function_names)
                })
            })
    }

    fn materialize_bound_snapshot_bindings(
        &self,
        bindings: &HashMap<String, Expression>,
        current_function_name: Option<&str>,
    ) -> HashMap<String, Expression> {
        let mut materialized = bindings.clone();
        let binding_names = materialized.keys().cloned().collect::<Vec<_>>();
        for name in binding_names {
            let Some(current_value) = materialized.get(&name).cloned() else {
                continue;
            };
            if matches!(current_value, Expression::Identifier(_))
                && self
                    .resolve_static_reference_identity_key(&current_value)
                    .is_some()
            {
                continue;
            }
            if let Expression::Identifier(value_name) = &current_value
                && let Some(target_value) = materialized.get(value_name)
                && value_name != "arguments"
                && !matches!(
                    target_value,
                    Expression::Number(_)
                        | Expression::BigInt(_)
                        | Expression::String(_)
                        | Expression::Bool(_)
                        | Expression::Null
                        | Expression::Undefined
                )
            {
                continue;
            }
            let resolved = self
                .evaluate_bound_snapshot_expression(
                    &current_value,
                    &mut materialized,
                    current_function_name,
                )
                .or(Some(current_value));
            if let Some(resolved) = resolved {
                materialized.insert(name, resolved);
            }
        }
        materialized
    }

    fn should_preserve_bound_snapshot_control_value_identity(
        &self,
        value: &Expression,
        current_function_name: Option<&str>,
    ) -> bool {
        matches!(value, Expression::Identifier(_))
            && (self.resolve_static_reference_identity_key(value).is_some()
                || self.resolve_object_binding_from_expression(value).is_some()
                || self.resolve_array_binding_from_expression(value).is_some()
                || self
                    .resolve_function_binding_from_expression_with_context(
                        value,
                        current_function_name,
                    )
                    .is_some())
    }

    pub(in crate::backend::direct_wasm) fn resolve_bound_snapshot_user_function_result(
        &self,
        function_name: &str,
        bindings: &HashMap<String, Expression>,
    ) -> Option<(Expression, HashMap<String, Expression>)> {
        self.resolve_bound_snapshot_user_function_result_with_arguments_and_this(
            function_name,
            bindings,
            &[],
            &Expression::Undefined,
        )
    }

    pub(in crate::backend::direct_wasm) fn resolve_bound_snapshot_user_function_result_with_arguments(
        &self,
        function_name: &str,
        bindings: &HashMap<String, Expression>,
        arguments: &[Expression],
    ) -> Option<(Expression, HashMap<String, Expression>)> {
        self.resolve_bound_snapshot_user_function_result_with_arguments_and_this(
            function_name,
            bindings,
            arguments,
            &Expression::Undefined,
        )
    }

    pub(in crate::backend::direct_wasm) fn resolve_bound_snapshot_user_function_result_with_arguments_and_this(
        &self,
        function_name: &str,
        bindings: &HashMap<String, Expression>,
        arguments: &[Expression],
        this_binding: &Expression,
    ) -> Option<(Expression, HashMap<String, Expression>)> {
        let (outcome, local_bindings) = self
            .resolve_bound_snapshot_user_function_outcome_with_arguments_and_this(
                function_name,
                bindings,
                arguments,
                this_binding,
            )?;
        Some((
            match outcome {
                StaticEvalOutcome::Value(value) => value,
                StaticEvalOutcome::Throw(_) => Expression::Undefined,
            },
            local_bindings,
        ))
    }

    pub(in crate::backend::direct_wasm) fn resolve_bound_snapshot_user_function_outcome_with_arguments_and_this(
        &self,
        function_name: &str,
        bindings: &HashMap<String, Expression>,
        arguments: &[Expression],
        this_binding: &Expression,
    ) -> Option<(StaticEvalOutcome, HashMap<String, Expression>)> {
        let trace = std::env::var_os("AYY_TRACE_BOUND_SNAPSHOT").is_some();
        let function = self.resolve_registered_function_declaration(function_name)?;
        let user_function = self.user_function(function_name)?;
        if trace {
            eprintln!(
                "bound_snapshot_user_function:start function={function_name} arguments={arguments:?} this={this_binding:?} binding_keys={:?}",
                bindings.keys().collect::<Vec<_>>()
            );
        }
        if user_function.is_generator() {
            if trace {
                eprintln!(
                    "bound_snapshot_user_function:none function={function_name} reason=generator"
                );
            }
            return None;
        }
        if let Some(captures) = self
            .backend
            .function_registry
            .analysis
            .user_function_capture_bindings
            .get(&user_function.name)
        {
            let missing = captures
                .keys()
                .filter(|name| !bindings.contains_key(*name))
                .cloned()
                .collect::<Vec<_>>();
            if !missing.is_empty() {
                if trace {
                    eprintln!(
                        "bound_snapshot_user_function:none function={function_name} reason=missing_captures missing={missing:?} captures={:?}",
                        captures.keys().collect::<Vec<_>>()
                    );
                }
                return None;
            }
        }
        if self.user_function_calls_captured_user_function(user_function) {
            if trace {
                eprintln!(
                    "bound_snapshot_user_function:none function={function_name} reason=calls_captured_user_function"
                );
            }
            return None;
        }
        if user_function.has_parameter_defaults()
            && !user_function
                .parameter_defaults
                .iter()
                .flatten()
                .all(inline_summary_side_effect_free_expression)
        {
            if trace {
                eprintln!(
                    "bound_snapshot_user_function:none function={function_name} reason=effectful_parameter_defaults"
                );
            }
            return None;
        }
        if self.user_function_deletes_call_frame_arguments_member(user_function) {
            if trace {
                eprintln!(
                    "bound_snapshot_user_function:none function={function_name} reason=deletes_arguments_member"
                );
            }
            return None;
        }
        if user_function.has_lowered_pattern_parameters() {
            if trace {
                eprintln!(
                    "bound_snapshot_user_function:none function={function_name} reason=lowered_pattern_parameters"
                );
            }
            return None;
        }
        let iterator_consumption_indices =
            self.user_function_parameter_iterator_consumption_indices(user_function);
        if !iterator_consumption_indices.is_empty() {
            if trace {
                eprintln!(
                    "bound_snapshot_user_function:none function={function_name} reason=parameter_iterator_consumption indices={iterator_consumption_indices:?}"
                );
            }
            return None;
        }
        if !user_function.params.is_empty() && !user_function.extra_argument_indices.is_empty() {
            if trace {
                eprintln!(
                    "bound_snapshot_user_function:none function={function_name} reason=extra_argument_indices params={:?} extra={:?}",
                    user_function.params, user_function.extra_argument_indices
                );
            }
            return None;
        }
        let materialized_arguments = arguments
            .iter()
            .map(|argument| self.materialize_static_expression(argument))
            .collect::<Vec<_>>();
        let mut local_bindings = bindings.clone();
        for binding in &user_function.scope_bindings {
            local_bindings
                .entry(binding.clone())
                .or_insert(Expression::Undefined);
        }
        local_bindings.insert("this".to_string(), this_binding.clone());
        let arguments_shadowed = user_function.lexical_this
            || user_function.body_declares_arguments_binding
            || user_function.params.iter().any(|param| {
                param == "arguments"
                    || scoped_binding_source_name(param)
                        .is_some_and(|source_name| source_name == "arguments")
            });
        if !arguments_shadowed {
            local_bindings.insert(
                "arguments".to_string(),
                Expression::Array(
                    materialized_arguments
                        .iter()
                        .cloned()
                        .map(ArrayElement::Expression)
                        .collect(),
                ),
            );
        }
        for (index, parameter_name) in user_function.params.iter().enumerate() {
            let mut parameter_value = materialized_arguments
                .get(index)
                .cloned()
                .unwrap_or(Expression::Undefined);
            if matches!(parameter_value, Expression::Undefined)
                && let Some(default) = user_function
                    .parameter_defaults
                    .get(index)
                    .and_then(Option::as_ref)
            {
                parameter_value = self
                    .evaluate_bound_snapshot_expression(
                        default,
                        &mut local_bindings,
                        Some(function_name),
                    )
                    .or_else(|| {
                        self.resolve_static_primitive_expression_with_context(
                            default,
                            Some(function_name),
                        )
                    })
                    .unwrap_or_else(|| self.materialize_static_expression(default));
            }
            local_bindings.insert(parameter_name.clone(), parameter_value);
        }
        if let Expression::Identifier(this_name) = this_binding
            && !local_bindings.contains_key(this_name)
        {
            if let Some(object_binding) = self.resolve_object_binding_from_expression(this_binding)
            {
                local_bindings.insert(
                    this_name.clone(),
                    object_binding_to_expression(&object_binding),
                );
            } else if let Some(value) = self
                .state
                .speculation
                .static_semantics
                .local_value_binding(this_name)
                .cloned()
                .or_else(|| {
                    self.backend
                        .global_semantics
                        .values
                        .value_bindings
                        .get(this_name)
                        .cloned()
                })
            {
                local_bindings.insert(this_name.clone(), value);
            }
        }
        let result = match self.execute_bound_snapshot_statements(
            &function.body,
            &mut local_bindings,
            Some(function_name),
        ) {
            Some(result) => result,
            None => {
                if trace {
                    eprintln!(
                        "bound_snapshot_user_function:none function={function_name} reason=statement_execution local_keys={:?}",
                        local_bindings.keys().collect::<Vec<_>>()
                    );
                }
                return None;
            }
        };
        let mut materialized_bindings =
            self.materialize_bound_snapshot_bindings(&local_bindings, Some(function_name));
        let mut updated_nonlocal_names =
            self.collect_user_function_assigned_nonlocal_bindings(user_function);
        updated_nonlocal_names
            .extend(self.collect_user_function_call_effect_nonlocal_bindings(user_function));
        updated_nonlocal_names.insert(SNAPSHOT_AWAIT_RESOLUTION_VALUE.to_string());
        updated_nonlocal_names.insert(SNAPSHOT_AWAIT_REJECTION_VALUE.to_string());
        let materialized_outcome = match result {
            BoundSnapshotControlFlow::None => BoundSnapshotControlFlow::None,
            BoundSnapshotControlFlow::Return(value) => BoundSnapshotControlFlow::Return(
                if self.should_preserve_bound_snapshot_control_value_identity(
                    &value,
                    Some(function_name),
                ) {
                    value
                } else {
                    self.evaluate_bound_snapshot_expression(
                        &value,
                        &mut materialized_bindings,
                        Some(function_name),
                    )
                    .unwrap_or(value)
                },
            ),
            BoundSnapshotControlFlow::Throw(value) => BoundSnapshotControlFlow::Throw(
                if self.should_preserve_bound_snapshot_control_value_identity(
                    &value,
                    Some(function_name),
                ) {
                    value
                } else {
                    self.evaluate_bound_snapshot_expression(
                        &value,
                        &mut materialized_bindings,
                        Some(function_name),
                    )
                    .unwrap_or(value)
                },
            ),
            BoundSnapshotControlFlow::Break(_) => return None,
        };
        self.preserve_bound_snapshot_returned_capture_bindings(
            &materialized_outcome,
            &local_bindings,
            &mut materialized_bindings,
            &mut updated_nonlocal_names,
            Some(function_name),
        );
        materialized_bindings.retain(|name, _| {
            let source_name = scoped_binding_source_name(name).unwrap_or(name);
            updated_nonlocal_names.contains(source_name)
        });
        Some((
            match materialized_outcome {
                BoundSnapshotControlFlow::None => StaticEvalOutcome::Value(Expression::Undefined),
                BoundSnapshotControlFlow::Return(value) => StaticEvalOutcome::Value(value),
                BoundSnapshotControlFlow::Throw(value) => {
                    StaticEvalOutcome::Throw(StaticThrowValue::Value(value))
                }
                BoundSnapshotControlFlow::Break(_) => return None,
            },
            materialized_bindings,
        ))
    }

    pub(in crate::backend::direct_wasm) fn resolve_bound_snapshot_function_outcome_with_arguments_and_this(
        &self,
        binding: &LocalFunctionBinding,
        bindings: &HashMap<String, Expression>,
        arguments: &[Expression],
        this_binding: &Expression,
    ) -> Option<(StaticEvalOutcome, HashMap<String, Expression>)> {
        let LocalFunctionBinding::User(function_name) = binding else {
            return None;
        };
        self.resolve_bound_snapshot_user_function_outcome_with_arguments_and_this(
            function_name,
            bindings,
            arguments,
            this_binding,
        )
    }

    pub(in crate::backend::direct_wasm) fn apply_bound_snapshot_user_function_call_effects(
        &self,
        function_name: &str,
        arguments: &[Expression],
        this_binding: &Expression,
        bindings: &mut HashMap<String, Expression>,
        current_function_name: Option<&str>,
    ) -> Option<Expression> {
        let user_function = self.user_function(function_name)?;
        if user_function.is_async() || user_function.is_generator() {
            return None;
        }
        let evaluated_arguments = arguments
            .iter()
            .map(|argument| {
                self.evaluate_bound_snapshot_expression(argument, bindings, current_function_name)
            })
            .collect::<Option<Vec<_>>>()?;
        let (result, updated_bindings) = self
            .resolve_bound_snapshot_user_function_result_with_arguments_and_this(
                function_name,
                bindings,
                &evaluated_arguments,
                this_binding,
            )?;
        for (name, value) in updated_bindings {
            let source_name = scoped_binding_source_name(&name)
                .unwrap_or(&name)
                .to_string();
            if user_function.scope_bindings.contains(&source_name) {
                continue;
            }
            bindings.insert(source_name, value);
        }
        Some(result)
    }

    pub(in crate::backend::direct_wasm) fn resolve_bound_snapshot_function_result_with_arguments(
        &self,
        binding: &LocalFunctionBinding,
        bindings: &HashMap<String, Expression>,
        arguments: &[Expression],
    ) -> Option<(Expression, HashMap<String, Expression>)> {
        self.resolve_bound_snapshot_function_result_with_arguments_and_this(
            binding,
            bindings,
            arguments,
            &Expression::Undefined,
        )
    }

    pub(in crate::backend::direct_wasm) fn resolve_bound_snapshot_function_result_with_arguments_and_this(
        &self,
        binding: &LocalFunctionBinding,
        bindings: &HashMap<String, Expression>,
        arguments: &[Expression],
        this_binding: &Expression,
    ) -> Option<(Expression, HashMap<String, Expression>)> {
        let LocalFunctionBinding::User(function_name) = binding else {
            return None;
        };
        self.resolve_bound_snapshot_user_function_result_with_arguments_and_this(
            function_name,
            bindings,
            arguments,
            this_binding,
        )
    }

    pub(in crate::backend::direct_wasm) fn resolve_bound_snapshot_thenable_outcome(
        &self,
        binding: &LocalFunctionBinding,
        this_binding: &Expression,
        bindings: &mut HashMap<String, Expression>,
        _current_function_name: Option<&str>,
    ) -> Option<StaticEvalOutcome> {
        let (result, updated_bindings) = self
            .resolve_bound_snapshot_function_result_with_arguments_and_this(
                binding,
                bindings,
                &[
                    Expression::Identifier(SNAPSHOT_AWAIT_RESOLVE_BINDING.to_string()),
                    Expression::Identifier(SNAPSHOT_AWAIT_REJECT_BINDING.to_string()),
                ],
                this_binding,
            )?;
        Self::merge_bound_snapshot_updated_bindings(bindings, updated_bindings);
        let resolution = bindings
            .get(SNAPSHOT_AWAIT_RESOLUTION_VALUE)
            .cloned()
            .map(|value| self.sanitize_snapshot_await_marker_expression(&value));
        let rejection = bindings
            .get(SNAPSHOT_AWAIT_REJECTION_VALUE)
            .cloned()
            .map(|value| self.sanitize_snapshot_await_marker_expression(&value));
        for value in bindings.values_mut() {
            *value = self.sanitize_snapshot_await_marker_expression(value);
        }
        bindings.retain(|name, value| {
            name != SNAPSHOT_AWAIT_RESOLUTION_VALUE
                && name != SNAPSHOT_AWAIT_REJECTION_VALUE
                && name != SNAPSHOT_AWAIT_RESOLVE_BINDING
                && name != SNAPSHOT_AWAIT_REJECT_BINDING
                && !matches!(
                    value,
                    Expression::Identifier(marker)
                        if marker == SNAPSHOT_AWAIT_RESOLVE_BINDING
                            || marker == SNAPSHOT_AWAIT_REJECT_BINDING
                )
        });
        if let Some(resolution) = resolution {
            return self
                .resolve_static_await_resolution_outcome(&resolution)
                .or(Some(StaticEvalOutcome::Value(resolution)));
        }
        if let Some(rejection) = rejection {
            return Some(StaticEvalOutcome::Throw(StaticThrowValue::Value(rejection)));
        }
        match result {
            Expression::Undefined => None,
            _ => self.resolve_static_await_resolution_outcome(&result),
        }
    }
}
