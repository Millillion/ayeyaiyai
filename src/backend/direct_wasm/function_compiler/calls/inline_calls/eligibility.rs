use super::*;

impl<'a> FunctionCompiler<'a> {
    fn expression_mentions_direct_eval(expression: &Expression) -> bool {
        match expression {
            Expression::Identifier(_) => false,
            Expression::Member { object, property } => {
                Self::expression_mentions_direct_eval(object)
                    || Self::expression_mentions_direct_eval(property)
            }
            Expression::Assign { value, .. } => Self::expression_mentions_direct_eval(value),
            Expression::AssignMember {
                object,
                property,
                value,
            } => {
                Self::expression_mentions_direct_eval(object)
                    || Self::expression_mentions_direct_eval(property)
                    || Self::expression_mentions_direct_eval(value)
            }
            Expression::AssignSuperMember { property, value } => {
                Self::expression_mentions_direct_eval(property)
                    || Self::expression_mentions_direct_eval(value)
            }
            Expression::Await(expression)
            | Expression::EnumerateKeys(expression)
            | Expression::GetIterator(expression)
            | Expression::IteratorClose(expression)
            | Expression::Unary { expression, .. } => {
                Self::expression_mentions_direct_eval(expression)
            }
            Expression::Binary { left, right, .. } => {
                Self::expression_mentions_direct_eval(left)
                    || Self::expression_mentions_direct_eval(right)
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                Self::expression_mentions_direct_eval(condition)
                    || Self::expression_mentions_direct_eval(then_expression)
                    || Self::expression_mentions_direct_eval(else_expression)
            }
            Expression::Sequence(expressions) => expressions
                .iter()
                .any(Self::expression_mentions_direct_eval),
            Expression::Array(elements) => elements.iter().any(|element| match element {
                ArrayElement::Expression(expression) | ArrayElement::Spread(expression) => {
                    Self::expression_mentions_direct_eval(expression)
                }
            }),
            Expression::Call { callee, arguments } => {
                matches!(callee.as_ref(), Expression::Identifier(name) if name == "eval")
                    || Self::expression_mentions_direct_eval(callee)
                    || arguments.iter().any(|argument| match argument {
                        CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                            Self::expression_mentions_direct_eval(expression)
                        }
                    })
            }
            Expression::SuperCall { callee, arguments } | Expression::New { callee, arguments } => {
                Self::expression_mentions_direct_eval(callee)
                    || arguments.iter().any(|argument| match argument {
                        CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                            Self::expression_mentions_direct_eval(expression)
                        }
                    })
            }
            Expression::Object(entries) => entries.iter().any(|entry| match entry {
                ObjectEntry::Data { key, value } => {
                    Self::expression_mentions_direct_eval(key)
                        || Self::expression_mentions_direct_eval(value)
                }
                ObjectEntry::Getter { key, getter } => {
                    Self::expression_mentions_direct_eval(key)
                        || Self::expression_mentions_direct_eval(getter)
                }
                ObjectEntry::Setter { key, setter } => {
                    Self::expression_mentions_direct_eval(key)
                        || Self::expression_mentions_direct_eval(setter)
                }
                ObjectEntry::Spread(expression) => {
                    Self::expression_mentions_direct_eval(expression)
                }
            }),
            Expression::This
            | Expression::SuperMember { .. }
            | Expression::Update { .. }
            | Expression::Number(_)
            | Expression::BigInt(_)
            | Expression::String(_)
            | Expression::Bool(_)
            | Expression::Null
            | Expression::Undefined
            | Expression::NewTarget
            | Expression::Sent => false,
        }
    }

    fn expression_mentions_private_member_access(expression: &Expression) -> bool {
        match expression {
            Expression::Identifier(_) => false,
            Expression::Member { object, property } => {
                matches!(property.as_ref(), Expression::String(name) if name.starts_with("__ayy$private$"))
                    || Self::expression_mentions_private_member_access(object)
                    || Self::expression_mentions_private_member_access(property)
            }
            Expression::Assign { value, .. } => {
                Self::expression_mentions_private_member_access(value)
            }
            Expression::AssignMember {
                object,
                property,
                value,
            } => {
                matches!(property.as_ref(), Expression::String(name) if name.starts_with("__ayy$private$"))
                    || Self::expression_mentions_private_member_access(object)
                    || Self::expression_mentions_private_member_access(property)
                    || Self::expression_mentions_private_member_access(value)
            }
            Expression::AssignSuperMember { property, value } => {
                Self::expression_mentions_private_member_access(property)
                    || Self::expression_mentions_private_member_access(value)
            }
            Expression::Await(expression)
            | Expression::EnumerateKeys(expression)
            | Expression::GetIterator(expression)
            | Expression::IteratorClose(expression)
            | Expression::Unary { expression, .. } => {
                Self::expression_mentions_private_member_access(expression)
            }
            Expression::Binary { left, right, .. } => {
                Self::expression_mentions_private_member_access(left)
                    || Self::expression_mentions_private_member_access(right)
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                Self::expression_mentions_private_member_access(condition)
                    || Self::expression_mentions_private_member_access(then_expression)
                    || Self::expression_mentions_private_member_access(else_expression)
            }
            Expression::Sequence(expressions) => expressions
                .iter()
                .any(Self::expression_mentions_private_member_access),
            Expression::Array(elements) => elements.iter().any(|element| match element {
                ArrayElement::Expression(expression) | ArrayElement::Spread(expression) => {
                    Self::expression_mentions_private_member_access(expression)
                }
            }),
            Expression::Call { callee, arguments }
            | Expression::SuperCall { callee, arguments }
            | Expression::New { callee, arguments } => {
                Self::expression_mentions_private_member_access(callee)
                    || arguments.iter().any(|argument| match argument {
                        CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                            Self::expression_mentions_private_member_access(expression)
                        }
                    })
            }
            Expression::Object(entries) => entries.iter().any(|entry| match entry {
                ObjectEntry::Data { key, value } => {
                    Self::expression_mentions_private_member_access(key)
                        || Self::expression_mentions_private_member_access(value)
                }
                ObjectEntry::Getter { key, getter } => {
                    Self::expression_mentions_private_member_access(key)
                        || Self::expression_mentions_private_member_access(getter)
                }
                ObjectEntry::Setter { key, setter } => {
                    Self::expression_mentions_private_member_access(key)
                        || Self::expression_mentions_private_member_access(setter)
                }
                ObjectEntry::Spread(expression) => {
                    Self::expression_mentions_private_member_access(expression)
                }
            }),
            Expression::This
            | Expression::SuperMember { .. }
            | Expression::Update { .. }
            | Expression::Number(_)
            | Expression::BigInt(_)
            | Expression::String(_)
            | Expression::Bool(_)
            | Expression::Null
            | Expression::Undefined
            | Expression::NewTarget
            | Expression::Sent => false,
        }
    }

    fn statement_mentions_private_member_access(statement: &Statement) -> bool {
        match statement {
            Statement::Declaration { body }
            | Statement::Block { body }
            | Statement::Labeled { body, .. }
            | Statement::With { body, .. } => body
                .iter()
                .any(Self::statement_mentions_private_member_access),
            Statement::If {
                condition,
                then_branch,
                else_branch,
            } => {
                Self::expression_mentions_private_member_access(condition)
                    || then_branch
                        .iter()
                        .any(Self::statement_mentions_private_member_access)
                    || else_branch
                        .iter()
                        .any(Self::statement_mentions_private_member_access)
            }
            Statement::Switch {
                discriminant,
                cases,
                ..
            } => {
                Self::expression_mentions_private_member_access(discriminant)
                    || cases.iter().any(|case| {
                        case.test
                            .as_ref()
                            .is_some_and(Self::expression_mentions_private_member_access)
                            || case
                                .body
                                .iter()
                                .any(Self::statement_mentions_private_member_access)
                    })
            }
            Statement::Try {
                body,
                catch_binding: _,
                catch_setup,
                catch_body,
                ..
            } => {
                body.iter()
                    .any(Self::statement_mentions_private_member_access)
                    || catch_setup
                        .iter()
                        .any(Self::statement_mentions_private_member_access)
                    || catch_body
                        .iter()
                        .any(Self::statement_mentions_private_member_access)
            }
            Statement::While {
                condition, body, ..
            }
            | Statement::DoWhile {
                condition, body, ..
            } => {
                Self::expression_mentions_private_member_access(condition)
                    || body
                        .iter()
                        .any(Self::statement_mentions_private_member_access)
            }
            Statement::For {
                init,
                condition,
                update,
                body,
                ..
            } => {
                init.iter()
                    .any(Self::statement_mentions_private_member_access)
                    || condition
                        .as_ref()
                        .is_some_and(Self::expression_mentions_private_member_access)
                    || update
                        .as_ref()
                        .is_some_and(Self::expression_mentions_private_member_access)
                    || body
                        .iter()
                        .any(Self::statement_mentions_private_member_access)
            }
            Statement::Var { value, .. }
            | Statement::Let { value, .. }
            | Statement::Assign { value, .. }
            | Statement::Return(value)
            | Statement::Throw(value)
            | Statement::Expression(value) => {
                Self::expression_mentions_private_member_access(value)
            }
            Statement::AssignMember {
                object,
                property,
                value,
            } => {
                Self::expression_mentions_private_member_access(object)
                    || Self::expression_mentions_private_member_access(property)
                    || Self::expression_mentions_private_member_access(value)
                    || matches!(property, Expression::String(name) if name.starts_with("__ayy$private$"))
            }
            Statement::Print { values } => values
                .iter()
                .any(Self::expression_mentions_private_member_access),
            Statement::Break { .. }
            | Statement::Continue { .. }
            | Statement::Yield { .. }
            | Statement::YieldDelegate { .. } => false,
        }
    }

    fn expression_reaches_private_member_access(
        &self,
        expression: &Expression,
        visited_functions: &mut HashSet<String>,
    ) -> bool {
        if Self::expression_mentions_private_member_access(expression) {
            return true;
        }
        if Self::expression_mentions_direct_eval(expression) {
            return true;
        }

        match expression {
            Expression::Call { callee, arguments }
            | Expression::SuperCall { callee, arguments }
            | Expression::New { callee, arguments } => {
                self.callee_reaches_private_member_access(callee, visited_functions)
                    || self.expression_reaches_private_member_access(callee, visited_functions)
                    || arguments.iter().any(|argument| match argument {
                        CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                            self.expression_reaches_private_member_access(
                                expression,
                                visited_functions,
                            )
                        }
                    })
            }
            Expression::Member { object, property } => {
                self.expression_reaches_private_member_access(object, visited_functions)
                    || self.expression_reaches_private_member_access(property, visited_functions)
            }
            Expression::SuperMember { property } => {
                self.expression_reaches_private_member_access(property, visited_functions)
            }
            Expression::Assign { value, .. }
            | Expression::Await(value)
            | Expression::EnumerateKeys(value)
            | Expression::GetIterator(value)
            | Expression::IteratorClose(value)
            | Expression::Unary {
                expression: value, ..
            } => self.expression_reaches_private_member_access(value, visited_functions),
            Expression::AssignMember {
                object,
                property,
                value,
            } => {
                self.expression_reaches_private_member_access(object, visited_functions)
                    || self.expression_reaches_private_member_access(property, visited_functions)
                    || self.expression_reaches_private_member_access(value, visited_functions)
            }
            Expression::AssignSuperMember { property, value } => {
                self.expression_reaches_private_member_access(property, visited_functions)
                    || self.expression_reaches_private_member_access(value, visited_functions)
            }
            Expression::Binary { left, right, .. } => {
                self.expression_reaches_private_member_access(left, visited_functions)
                    || self.expression_reaches_private_member_access(right, visited_functions)
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                self.expression_reaches_private_member_access(condition, visited_functions)
                    || self.expression_reaches_private_member_access(
                        then_expression,
                        visited_functions,
                    )
                    || self.expression_reaches_private_member_access(
                        else_expression,
                        visited_functions,
                    )
            }
            Expression::Sequence(expressions) => expressions.iter().any(|expression| {
                self.expression_reaches_private_member_access(expression, visited_functions)
            }),
            Expression::Array(elements) => elements.iter().any(|element| match element {
                ArrayElement::Expression(expression) | ArrayElement::Spread(expression) => {
                    self.expression_reaches_private_member_access(expression, visited_functions)
                }
            }),
            Expression::Object(entries) => entries.iter().any(|entry| match entry {
                ObjectEntry::Data { key, value } => {
                    self.expression_reaches_private_member_access(key, visited_functions)
                        || self.expression_reaches_private_member_access(value, visited_functions)
                }
                ObjectEntry::Getter { key, getter } => {
                    self.expression_reaches_private_member_access(key, visited_functions)
                        || self.expression_reaches_private_member_access(getter, visited_functions)
                }
                ObjectEntry::Setter { key, setter } => {
                    self.expression_reaches_private_member_access(key, visited_functions)
                        || self.expression_reaches_private_member_access(setter, visited_functions)
                }
                ObjectEntry::Spread(expression) => {
                    self.expression_reaches_private_member_access(expression, visited_functions)
                }
            }),
            Expression::This
            | Expression::Identifier(_)
            | Expression::Update { .. }
            | Expression::Number(_)
            | Expression::BigInt(_)
            | Expression::String(_)
            | Expression::Bool(_)
            | Expression::Null
            | Expression::Undefined
            | Expression::NewTarget
            | Expression::Sent => false,
        }
    }

    fn callee_reaches_private_member_access(
        &self,
        callee: &Expression,
        visited_functions: &mut HashSet<String>,
    ) -> bool {
        if matches!(
            callee,
            Expression::Member { object, property }
                if matches!(object.as_ref(), Expression::Identifier(name) if name == "assert")
                    && matches!(
                        property.as_ref(),
                        Expression::String(name)
                            if matches!(name.as_str(), "sameValue" | "notSameValue")
                    )
        ) {
            return false;
        }

        let binding = match callee {
            Expression::Member { object, property } => self
                .resolve_member_function_binding_shallow_without_runtime_public_this_guard(
                    object, property,
                )
                .or_else(|| {
                    self.resolve_syntactic_builtin_member_function_binding(object, property)
                }),
            _ => self.resolve_function_binding_from_expression(callee),
        };
        match binding {
            Some(LocalFunctionBinding::User(function_name)) => self
                .user_function_name_reaches_private_member_access(
                    &function_name,
                    visited_functions,
                ),
            Some(LocalFunctionBinding::Builtin(_)) => false,
            None => matches!(callee, Expression::Member { .. }),
        }
    }

    fn resolve_syntactic_builtin_member_function_binding(
        &self,
        object: &Expression,
        property: &Expression,
    ) -> Option<LocalFunctionBinding> {
        let Expression::String(property_name) = property else {
            return None;
        };
        match object {
            Expression::Member {
                object: prototype_owner,
                property: prototype_property,
            } if matches!(prototype_property.as_ref(), Expression::String(name) if name == "prototype") =>
            {
                let Expression::Identifier(object_name) = prototype_owner.as_ref() else {
                    return None;
                };
                if !self.is_unshadowed_builtin_identifier(object_name) {
                    return None;
                }
                builtin_prototype_function_name(object_name, property_name)
                    .map(|name| LocalFunctionBinding::Builtin(name.to_string()))
            }
            Expression::Identifier(object_name) => {
                if !self.is_unshadowed_builtin_identifier(object_name) {
                    return None;
                }
                builtin_member_function_name(object_name, property_name)
                    .map(|name| LocalFunctionBinding::Builtin(name.to_string()))
            }
            _ => None,
        }
    }

    fn statement_reaches_private_member_access(
        &self,
        statement: &Statement,
        visited_functions: &mut HashSet<String>,
    ) -> bool {
        if Self::statement_mentions_private_member_access(statement) {
            return true;
        }

        match statement {
            Statement::Declaration { body }
            | Statement::Block { body }
            | Statement::Labeled { body, .. }
            | Statement::With { body, .. } => body.iter().any(|statement| {
                self.statement_reaches_private_member_access(statement, visited_functions)
            }),
            Statement::If {
                condition,
                then_branch,
                else_branch,
            } => {
                self.expression_reaches_private_member_access(condition, visited_functions)
                    || then_branch.iter().any(|statement| {
                        self.statement_reaches_private_member_access(statement, visited_functions)
                    })
                    || else_branch.iter().any(|statement| {
                        self.statement_reaches_private_member_access(statement, visited_functions)
                    })
            }
            Statement::Switch {
                discriminant,
                cases,
                ..
            } => {
                self.expression_reaches_private_member_access(discriminant, visited_functions)
                    || cases.iter().any(|case| {
                        case.test.as_ref().is_some_and(|test| {
                            self.expression_reaches_private_member_access(test, visited_functions)
                        }) || case.body.iter().any(|statement| {
                            self.statement_reaches_private_member_access(
                                statement,
                                visited_functions,
                            )
                        })
                    })
            }
            Statement::Try {
                body,
                catch_binding: _,
                catch_setup,
                catch_body,
                ..
            } => {
                body.iter().any(|statement| {
                    self.statement_reaches_private_member_access(statement, visited_functions)
                }) || catch_setup.iter().any(|statement| {
                    self.statement_reaches_private_member_access(statement, visited_functions)
                }) || catch_body.iter().any(|statement| {
                    self.statement_reaches_private_member_access(statement, visited_functions)
                })
            }
            Statement::While {
                condition, body, ..
            }
            | Statement::DoWhile {
                condition, body, ..
            } => {
                self.expression_reaches_private_member_access(condition, visited_functions)
                    || body.iter().any(|statement| {
                        self.statement_reaches_private_member_access(statement, visited_functions)
                    })
            }
            Statement::For {
                init,
                condition,
                update,
                body,
                ..
            } => {
                init.iter().any(|statement| {
                    self.statement_reaches_private_member_access(statement, visited_functions)
                }) || condition.as_ref().is_some_and(|condition| {
                    self.expression_reaches_private_member_access(condition, visited_functions)
                }) || update.as_ref().is_some_and(|update| {
                    self.expression_reaches_private_member_access(update, visited_functions)
                }) || body.iter().any(|statement| {
                    self.statement_reaches_private_member_access(statement, visited_functions)
                })
            }
            Statement::Var { value, .. }
            | Statement::Let { value, .. }
            | Statement::Assign { value, .. }
            | Statement::Return(value)
            | Statement::Throw(value)
            | Statement::Expression(value) => {
                self.expression_reaches_private_member_access(value, visited_functions)
            }
            Statement::AssignMember {
                object,
                property,
                value,
            } => {
                self.expression_reaches_private_member_access(object, visited_functions)
                    || self.expression_reaches_private_member_access(property, visited_functions)
                    || self.expression_reaches_private_member_access(value, visited_functions)
            }
            Statement::Print { values } => values.iter().any(|value| {
                self.expression_reaches_private_member_access(value, visited_functions)
            }),
            Statement::Break { .. }
            | Statement::Continue { .. }
            | Statement::Yield { .. }
            | Statement::YieldDelegate { .. } => false,
        }
    }

    fn collect_local_function_aliases_from_expression(
        &self,
        expression: &Expression,
        aliases: &mut HashMap<String, String>,
    ) {
        match expression {
            Expression::Assign { name, value } => {
                if let Expression::Identifier(function_name) = value.as_ref()
                    && self
                        .resolve_registered_function_declaration(function_name)
                        .is_some()
                {
                    aliases.insert(name.clone(), function_name.clone());
                }
                self.collect_local_function_aliases_from_expression(value, aliases);
            }
            Expression::Member { object, property } => {
                self.collect_local_function_aliases_from_expression(object, aliases);
                self.collect_local_function_aliases_from_expression(property, aliases);
            }
            Expression::AssignMember {
                object,
                property,
                value,
            } => {
                self.collect_local_function_aliases_from_expression(object, aliases);
                self.collect_local_function_aliases_from_expression(property, aliases);
                self.collect_local_function_aliases_from_expression(value, aliases);
            }
            Expression::AssignSuperMember { property, value } => {
                self.collect_local_function_aliases_from_expression(property, aliases);
                self.collect_local_function_aliases_from_expression(value, aliases);
            }
            Expression::Await(expression)
            | Expression::EnumerateKeys(expression)
            | Expression::GetIterator(expression)
            | Expression::IteratorClose(expression)
            | Expression::Unary { expression, .. } => {
                self.collect_local_function_aliases_from_expression(expression, aliases);
            }
            Expression::Binary { left, right, .. } => {
                self.collect_local_function_aliases_from_expression(left, aliases);
                self.collect_local_function_aliases_from_expression(right, aliases);
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                self.collect_local_function_aliases_from_expression(condition, aliases);
                self.collect_local_function_aliases_from_expression(then_expression, aliases);
                self.collect_local_function_aliases_from_expression(else_expression, aliases);
            }
            Expression::Sequence(expressions) => {
                for expression in expressions {
                    self.collect_local_function_aliases_from_expression(expression, aliases);
                }
            }
            Expression::Array(elements) => {
                for element in elements {
                    match element {
                        ArrayElement::Expression(expression) | ArrayElement::Spread(expression) => {
                            self.collect_local_function_aliases_from_expression(
                                expression, aliases,
                            );
                        }
                    }
                }
            }
            Expression::Call { callee, arguments }
            | Expression::SuperCall { callee, arguments }
            | Expression::New { callee, arguments } => {
                self.collect_local_function_aliases_from_expression(callee, aliases);
                for argument in arguments {
                    self.collect_local_function_aliases_from_expression(
                        argument.expression(),
                        aliases,
                    );
                }
            }
            Expression::Object(entries) => {
                for entry in entries {
                    match entry {
                        ObjectEntry::Data { key, value } => {
                            self.collect_local_function_aliases_from_expression(key, aliases);
                            self.collect_local_function_aliases_from_expression(value, aliases);
                        }
                        ObjectEntry::Getter { key, getter } => {
                            self.collect_local_function_aliases_from_expression(key, aliases);
                            self.collect_local_function_aliases_from_expression(getter, aliases);
                        }
                        ObjectEntry::Setter { key, setter } => {
                            self.collect_local_function_aliases_from_expression(key, aliases);
                            self.collect_local_function_aliases_from_expression(setter, aliases);
                        }
                        ObjectEntry::Spread(expression) => {
                            self.collect_local_function_aliases_from_expression(
                                expression, aliases,
                            );
                        }
                    }
                }
            }
            Expression::Identifier(_)
            | Expression::This
            | Expression::SuperMember { .. }
            | Expression::Update { .. }
            | Expression::Number(_)
            | Expression::BigInt(_)
            | Expression::String(_)
            | Expression::Bool(_)
            | Expression::Null
            | Expression::Undefined
            | Expression::NewTarget
            | Expression::Sent => {}
        }
    }

    fn collect_local_function_aliases_from_statement(
        &self,
        statement: &Statement,
        aliases: &mut HashMap<String, String>,
    ) {
        match statement {
            Statement::Var { name, value }
            | Statement::Let { name, value, .. }
            | Statement::Assign { name, value } => {
                if let Expression::Identifier(function_name) = value
                    && self
                        .resolve_registered_function_declaration(function_name)
                        .is_some()
                {
                    aliases.insert(name.clone(), function_name.clone());
                }
                self.collect_local_function_aliases_from_expression(value, aliases);
            }
            Statement::Declaration { body }
            | Statement::Block { body }
            | Statement::Labeled { body, .. }
            | Statement::With { body, .. }
            | Statement::While { body, .. }
            | Statement::DoWhile { body, .. } => {
                for statement in body {
                    self.collect_local_function_aliases_from_statement(statement, aliases);
                }
            }
            Statement::If {
                condition,
                then_branch,
                else_branch,
            } => {
                self.collect_local_function_aliases_from_expression(condition, aliases);
                for statement in then_branch.iter().chain(else_branch.iter()) {
                    self.collect_local_function_aliases_from_statement(statement, aliases);
                }
            }
            Statement::Try {
                body,
                catch_setup,
                catch_body,
                ..
            } => {
                for statement in body
                    .iter()
                    .chain(catch_setup.iter())
                    .chain(catch_body.iter())
                {
                    self.collect_local_function_aliases_from_statement(statement, aliases);
                }
            }
            Statement::Switch {
                discriminant,
                cases,
                ..
            } => {
                self.collect_local_function_aliases_from_expression(discriminant, aliases);
                for case in cases {
                    if let Some(test) = &case.test {
                        self.collect_local_function_aliases_from_expression(test, aliases);
                    }
                    for statement in &case.body {
                        self.collect_local_function_aliases_from_statement(statement, aliases);
                    }
                }
            }
            Statement::For {
                init,
                condition,
                update,
                body,
                ..
            } => {
                for statement in init.iter().chain(body.iter()) {
                    self.collect_local_function_aliases_from_statement(statement, aliases);
                }
                if let Some(condition) = condition {
                    self.collect_local_function_aliases_from_expression(condition, aliases);
                }
                if let Some(update) = update {
                    self.collect_local_function_aliases_from_expression(update, aliases);
                }
            }
            Statement::Expression(value)
            | Statement::Throw(value)
            | Statement::Return(value)
            | Statement::Yield { value }
            | Statement::YieldDelegate { value } => {
                self.collect_local_function_aliases_from_expression(value, aliases);
            }
            Statement::AssignMember {
                object,
                property,
                value,
            } => {
                self.collect_local_function_aliases_from_expression(object, aliases);
                self.collect_local_function_aliases_from_expression(property, aliases);
                self.collect_local_function_aliases_from_expression(value, aliases);
            }
            Statement::Print { values } => {
                for value in values {
                    self.collect_local_function_aliases_from_expression(value, aliases);
                }
            }
            Statement::Break { .. } | Statement::Continue { .. } => {}
        }
    }

    fn expression_calls_private_reaching_local_function(
        &self,
        expression: &Expression,
        local_function_aliases: &HashMap<String, String>,
        visited_functions: &mut HashSet<String>,
    ) -> bool {
        match expression {
            Expression::Call { callee, arguments }
            | Expression::SuperCall { callee, arguments }
            | Expression::New { callee, arguments } => {
                if let Expression::Identifier(callee_name) = callee.as_ref() {
                    let function_name = local_function_aliases
                        .get(callee_name)
                        .map(String::as_str)
                        .unwrap_or(callee_name);
                    if self
                        .resolve_registered_function_declaration(function_name)
                        .is_some()
                        && self.user_function_name_reaches_private_member_access(
                            function_name,
                            visited_functions,
                        )
                    {
                        return true;
                    }
                }
                self.expression_calls_private_reaching_local_function(
                    callee,
                    local_function_aliases,
                    visited_functions,
                ) || arguments.iter().any(|argument| {
                    self.expression_calls_private_reaching_local_function(
                        argument.expression(),
                        local_function_aliases,
                        visited_functions,
                    )
                })
            }
            Expression::Member { object, property } => {
                self.expression_calls_private_reaching_local_function(
                    object,
                    local_function_aliases,
                    visited_functions,
                ) || self.expression_calls_private_reaching_local_function(
                    property,
                    local_function_aliases,
                    visited_functions,
                )
            }
            Expression::Assign { value, .. }
            | Expression::Await(value)
            | Expression::EnumerateKeys(value)
            | Expression::GetIterator(value)
            | Expression::IteratorClose(value)
            | Expression::Unary {
                expression: value, ..
            } => self.expression_calls_private_reaching_local_function(
                value,
                local_function_aliases,
                visited_functions,
            ),
            Expression::AssignMember {
                object,
                property,
                value,
            } => {
                self.expression_calls_private_reaching_local_function(
                    object,
                    local_function_aliases,
                    visited_functions,
                ) || self.expression_calls_private_reaching_local_function(
                    property,
                    local_function_aliases,
                    visited_functions,
                ) || self.expression_calls_private_reaching_local_function(
                    value,
                    local_function_aliases,
                    visited_functions,
                )
            }
            Expression::SuperMember { property } => self
                .expression_calls_private_reaching_local_function(
                    property,
                    local_function_aliases,
                    visited_functions,
                ),
            Expression::AssignSuperMember { property, value } => {
                self.expression_calls_private_reaching_local_function(
                    property,
                    local_function_aliases,
                    visited_functions,
                ) || self.expression_calls_private_reaching_local_function(
                    value,
                    local_function_aliases,
                    visited_functions,
                )
            }
            Expression::Binary { left, right, .. } => {
                self.expression_calls_private_reaching_local_function(
                    left,
                    local_function_aliases,
                    visited_functions,
                ) || self.expression_calls_private_reaching_local_function(
                    right,
                    local_function_aliases,
                    visited_functions,
                )
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                self.expression_calls_private_reaching_local_function(
                    condition,
                    local_function_aliases,
                    visited_functions,
                ) || self.expression_calls_private_reaching_local_function(
                    then_expression,
                    local_function_aliases,
                    visited_functions,
                ) || self.expression_calls_private_reaching_local_function(
                    else_expression,
                    local_function_aliases,
                    visited_functions,
                )
            }
            Expression::Sequence(expressions) => expressions.iter().any(|expression| {
                self.expression_calls_private_reaching_local_function(
                    expression,
                    local_function_aliases,
                    visited_functions,
                )
            }),
            Expression::Array(elements) => elements.iter().any(|element| match element {
                ArrayElement::Expression(expression) | ArrayElement::Spread(expression) => self
                    .expression_calls_private_reaching_local_function(
                        expression,
                        local_function_aliases,
                        visited_functions,
                    ),
            }),
            Expression::Object(entries) => entries.iter().any(|entry| match entry {
                ObjectEntry::Data { key, value } => {
                    self.expression_calls_private_reaching_local_function(
                        key,
                        local_function_aliases,
                        visited_functions,
                    ) || self.expression_calls_private_reaching_local_function(
                        value,
                        local_function_aliases,
                        visited_functions,
                    )
                }
                ObjectEntry::Getter { key, getter } => {
                    self.expression_calls_private_reaching_local_function(
                        key,
                        local_function_aliases,
                        visited_functions,
                    ) || self.expression_calls_private_reaching_local_function(
                        getter,
                        local_function_aliases,
                        visited_functions,
                    )
                }
                ObjectEntry::Setter { key, setter } => {
                    self.expression_calls_private_reaching_local_function(
                        key,
                        local_function_aliases,
                        visited_functions,
                    ) || self.expression_calls_private_reaching_local_function(
                        setter,
                        local_function_aliases,
                        visited_functions,
                    )
                }
                ObjectEntry::Spread(expression) => self
                    .expression_calls_private_reaching_local_function(
                        expression,
                        local_function_aliases,
                        visited_functions,
                    ),
            }),
            Expression::Identifier(_)
            | Expression::This
            | Expression::Update { .. }
            | Expression::Number(_)
            | Expression::BigInt(_)
            | Expression::String(_)
            | Expression::Bool(_)
            | Expression::Null
            | Expression::Undefined
            | Expression::NewTarget
            | Expression::Sent => false,
        }
    }

    fn statement_calls_private_reaching_local_function(
        &self,
        statement: &Statement,
        local_function_aliases: &HashMap<String, String>,
        visited_functions: &mut HashSet<String>,
    ) -> bool {
        match statement {
            Statement::Declaration { body }
            | Statement::Block { body }
            | Statement::Labeled { body, .. }
            | Statement::With { body, .. }
            | Statement::While { body, .. }
            | Statement::DoWhile { body, .. } => body.iter().any(|statement| {
                self.statement_calls_private_reaching_local_function(
                    statement,
                    local_function_aliases,
                    visited_functions,
                )
            }),
            Statement::If {
                condition,
                then_branch,
                else_branch,
            } => {
                self.expression_calls_private_reaching_local_function(
                    condition,
                    local_function_aliases,
                    visited_functions,
                ) || then_branch.iter().any(|statement| {
                    self.statement_calls_private_reaching_local_function(
                        statement,
                        local_function_aliases,
                        visited_functions,
                    )
                }) || else_branch.iter().any(|statement| {
                    self.statement_calls_private_reaching_local_function(
                        statement,
                        local_function_aliases,
                        visited_functions,
                    )
                })
            }
            Statement::Switch {
                discriminant,
                cases,
                ..
            } => {
                self.expression_calls_private_reaching_local_function(
                    discriminant,
                    local_function_aliases,
                    visited_functions,
                ) || cases.iter().any(|case| {
                    case.test.as_ref().is_some_and(|test| {
                        self.expression_calls_private_reaching_local_function(
                            test,
                            local_function_aliases,
                            visited_functions,
                        )
                    }) || case.body.iter().any(|statement| {
                        self.statement_calls_private_reaching_local_function(
                            statement,
                            local_function_aliases,
                            visited_functions,
                        )
                    })
                })
            }
            Statement::Try {
                body,
                catch_setup,
                catch_body,
                ..
            } => {
                body.iter().any(|statement| {
                    self.statement_calls_private_reaching_local_function(
                        statement,
                        local_function_aliases,
                        visited_functions,
                    )
                }) || catch_setup.iter().any(|statement| {
                    self.statement_calls_private_reaching_local_function(
                        statement,
                        local_function_aliases,
                        visited_functions,
                    )
                }) || catch_body.iter().any(|statement| {
                    self.statement_calls_private_reaching_local_function(
                        statement,
                        local_function_aliases,
                        visited_functions,
                    )
                })
            }
            Statement::For {
                init,
                condition,
                update,
                body,
                ..
            } => {
                init.iter().any(|statement| {
                    self.statement_calls_private_reaching_local_function(
                        statement,
                        local_function_aliases,
                        visited_functions,
                    )
                }) || condition.as_ref().is_some_and(|condition| {
                    self.expression_calls_private_reaching_local_function(
                        condition,
                        local_function_aliases,
                        visited_functions,
                    )
                }) || update.as_ref().is_some_and(|update| {
                    self.expression_calls_private_reaching_local_function(
                        update,
                        local_function_aliases,
                        visited_functions,
                    )
                }) || body.iter().any(|statement| {
                    self.statement_calls_private_reaching_local_function(
                        statement,
                        local_function_aliases,
                        visited_functions,
                    )
                })
            }
            Statement::Var { value, .. }
            | Statement::Let { value, .. }
            | Statement::Assign { value, .. }
            | Statement::Return(value)
            | Statement::Throw(value)
            | Statement::Expression(value)
            | Statement::Yield { value }
            | Statement::YieldDelegate { value } => self
                .expression_calls_private_reaching_local_function(
                    value,
                    local_function_aliases,
                    visited_functions,
                ),
            Statement::AssignMember {
                object,
                property,
                value,
            } => {
                self.expression_calls_private_reaching_local_function(
                    object,
                    local_function_aliases,
                    visited_functions,
                ) || self.expression_calls_private_reaching_local_function(
                    property,
                    local_function_aliases,
                    visited_functions,
                ) || self.expression_calls_private_reaching_local_function(
                    value,
                    local_function_aliases,
                    visited_functions,
                )
            }
            Statement::Print { values } => values.iter().any(|value| {
                self.expression_calls_private_reaching_local_function(
                    value,
                    local_function_aliases,
                    visited_functions,
                )
            }),
            Statement::Break { .. } | Statement::Continue { .. } => false,
        }
    }

    fn user_function_name_reaches_private_member_access(
        &self,
        function_name: &str,
        visited_functions: &mut HashSet<String>,
    ) -> bool {
        if !visited_functions.insert(function_name.to_string()) {
            return false;
        }

        self.resolve_registered_function_declaration(function_name)
            .is_some_and(|function| {
                let mut local_function_aliases = HashMap::new();
                for statement in &function.body {
                    self.collect_local_function_aliases_from_statement(
                        statement,
                        &mut local_function_aliases,
                    );
                }
                function.body.iter().any(|statement| {
                    self.statement_reaches_private_member_access(statement, visited_functions)
                        || self.statement_calls_private_reaching_local_function(
                            statement,
                            &local_function_aliases,
                            visited_functions,
                        )
                })
            })
    }

    fn statement_mentions_direct_eval(statement: &Statement) -> bool {
        match statement {
            Statement::Declaration { body }
            | Statement::Block { body }
            | Statement::Labeled { body, .. }
            | Statement::With { body, .. } => body.iter().any(Self::statement_mentions_direct_eval),
            Statement::If {
                condition,
                then_branch,
                else_branch,
            } => {
                Self::expression_mentions_direct_eval(condition)
                    || then_branch.iter().any(Self::statement_mentions_direct_eval)
                    || else_branch.iter().any(Self::statement_mentions_direct_eval)
            }
            Statement::Switch {
                discriminant,
                cases,
                ..
            } => {
                Self::expression_mentions_direct_eval(discriminant)
                    || cases.iter().any(|case| {
                        case.test
                            .as_ref()
                            .is_some_and(Self::expression_mentions_direct_eval)
                            || case.body.iter().any(Self::statement_mentions_direct_eval)
                    })
            }
            Statement::Try {
                body,
                catch_setup,
                catch_body,
                ..
            } => {
                body.iter().any(Self::statement_mentions_direct_eval)
                    || catch_setup.iter().any(Self::statement_mentions_direct_eval)
                    || catch_body.iter().any(Self::statement_mentions_direct_eval)
            }
            Statement::While {
                condition, body, ..
            }
            | Statement::DoWhile {
                condition, body, ..
            } => {
                Self::expression_mentions_direct_eval(condition)
                    || body.iter().any(Self::statement_mentions_direct_eval)
            }
            Statement::For {
                init,
                condition,
                update,
                body,
                ..
            } => {
                init.iter().any(Self::statement_mentions_direct_eval)
                    || condition
                        .as_ref()
                        .is_some_and(Self::expression_mentions_direct_eval)
                    || update
                        .as_ref()
                        .is_some_and(Self::expression_mentions_direct_eval)
                    || body.iter().any(Self::statement_mentions_direct_eval)
            }
            Statement::Var { value, .. }
            | Statement::Let { value, .. }
            | Statement::Assign { value, .. }
            | Statement::Return(value)
            | Statement::Throw(value)
            | Statement::Expression(value) => Self::expression_mentions_direct_eval(value),
            Statement::AssignMember {
                object,
                property,
                value,
            } => {
                Self::expression_mentions_direct_eval(object)
                    || Self::expression_mentions_direct_eval(property)
                    || Self::expression_mentions_direct_eval(value)
            }
            Statement::Print { values } => values.iter().any(Self::expression_mentions_direct_eval),
            Statement::Break { .. }
            | Statement::Continue { .. }
            | Statement::Yield { .. }
            | Statement::YieldDelegate { .. } => false,
        }
    }

    pub(in crate::backend::direct_wasm) fn user_function_mentions_private_member_access(
        &self,
        user_function: &UserFunction,
    ) -> bool {
        self.user_function_name_reaches_private_member_access(
            &user_function.name,
            &mut HashSet::new(),
        )
    }

    pub(in crate::backend::direct_wasm) fn user_function_mentions_direct_eval(
        &self,
        user_function: &UserFunction,
    ) -> bool {
        self.resolve_registered_function_declaration(&user_function.name)
            .is_some_and(|function| {
                function
                    .body
                    .iter()
                    .any(Self::statement_mentions_direct_eval)
            })
    }

    pub(in crate::backend::direct_wasm) fn inline_safe_argument_expression(
        &self,
        expression: &Expression,
    ) -> bool {
        if self.expression_reads_local_descriptor_binding_member(expression) {
            return false;
        }
        let materialized = self.materialize_static_expression(expression);
        matches!(
            materialized,
            Expression::Number(_)
                | Expression::BigInt(_)
                | Expression::String(_)
                | Expression::Bool(_)
                | Expression::Null
                | Expression::Undefined
                | Expression::This
                | Expression::Array(_)
        ) || matches!(materialized, Expression::Object(ref entries)
            if entries.iter().all(|entry| matches!(entry, ObjectEntry::Data { .. })))
            || matches!(
                materialized,
                Expression::Member { ref object, ref property }
                    if matches!(property.as_ref(), Expression::String(name) if name == "prototype")
                        && !matches!(object.as_ref(), Expression::SuperMember { .. })
            )
            || self
                .resolve_object_binding_from_expression(expression)
                .is_some()
            || self
                .resolve_array_binding_from_expression(expression)
                .is_some()
            || self
                .resolve_function_binding_from_expression(expression)
                .is_some()
            || self
                .resolve_user_function_from_expression(expression)
                .is_some()
            || self
                .resolve_symbol_identity_expression(&materialized)
                .is_some()
            || self
                .resolve_symbol_identity_expression(expression)
                .is_some()
    }

    pub(in crate::backend::direct_wasm) fn can_inline_user_function_call(
        &self,
        user_function: &UserFunction,
        arguments: &[Expression],
    ) -> bool {
        if !user_function.lexical_this
            && self
                .resolve_registered_function_declaration(&user_function.name)
                .is_some_and(|function| {
                    function
                        .body
                        .iter()
                        .any(Self::statement_mentions_call_frame_state)
                })
        {
            return false;
        }
        self.state.emission.control_flow.try_stack.is_empty()
            && !self.current_function_contains_try_statement()
            && arguments.iter().all(|argument| {
                let materialized = self.materialize_static_expression(argument);
                static_expression_matches(&materialized, argument)
                    && self.inline_safe_argument_expression(argument)
            })
            && !arguments
                .iter()
                .any(|argument| self.inline_argument_mentions_shadowed_implicit_global(argument))
            && !user_function.is_async()
            && !user_function.is_generator()
            && !user_function
                .inline_summary
                .as_ref()
                .is_some_and(inline_summary_mentions_assertion_builtin)
            && !self.user_function_mentions_private_member_access(user_function)
            && !self.user_function_mentions_direct_eval(user_function)
            && !self.user_function_contains_identifier_callee_call(user_function)
            && !self.user_function_may_read_restricted_function_property(user_function)
            && !self
                .backend
                .function_registry
                .analysis
                .user_function_capture_bindings
                .contains_key(&user_function.name)
            && !self.user_function_references_captured_user_function(user_function)
            && user_function.extra_argument_indices.is_empty()
            && !user_function.has_parameter_defaults()
            && !user_function.has_lowered_pattern_parameters()
            && (user_function
                .inline_summary
                .as_ref()
                .is_some_and(|summary| {
                    !inline_summary_mentions_assertion_builtin(summary)
                        && (user_function.lexical_this
                            || !inline_summary_mentions_call_frame_state(summary))
                })
                || self.user_function_has_inlineable_terminal_body(user_function))
    }

    pub(in crate::backend::direct_wasm) fn can_inline_user_function_call_with_explicit_call_frame(
        &self,
        user_function: &UserFunction,
        arguments: &[Expression],
        this_expression: &Expression,
    ) -> bool {
        self.state.emission.control_flow.try_stack.is_empty()
            && !self.current_function_contains_try_statement()
            && (user_function.lexical_this || !matches!(this_expression, Expression::This))
            && !self.expression_reads_local_descriptor_binding_member(this_expression)
            && self.inline_safe_argument_expression(this_expression)
            && !self.inline_argument_mentions_shadowed_implicit_global(this_expression)
            && arguments
                .iter()
                .all(|argument| self.inline_safe_argument_expression(argument))
            && !arguments
                .iter()
                .any(|argument| self.inline_argument_mentions_shadowed_implicit_global(argument))
            && !user_function.is_async()
            && !user_function.is_generator()
            && !user_function
                .inline_summary
                .as_ref()
                .is_some_and(inline_summary_mentions_assertion_builtin)
            && !self.user_function_mentions_private_member_access(user_function)
            && !self.user_function_mentions_direct_eval(user_function)
            && !self.user_function_contains_identifier_callee_call(user_function)
            && !self.user_function_may_read_restricted_function_property(user_function)
            && !self
                .backend
                .function_registry
                .analysis
                .user_function_capture_bindings
                .contains_key(&user_function.name)
            && !self.user_function_references_captured_user_function(user_function)
            && user_function.extra_argument_indices.is_empty()
            && !user_function.has_parameter_defaults()
            && (user_function
                .inline_summary
                .as_ref()
                .is_some_and(|summary| {
                    !inline_summary_mentions_assertion_builtin(summary)
                        && !inline_summary_mentions_unsupported_explicit_call_frame_state(summary)
                })
                || self
                    .user_function_has_explicit_call_frame_inlineable_terminal_body(user_function))
    }

    pub(in crate::backend::direct_wasm) fn can_inline_primitive_effect_user_function_call_with_explicit_call_frame(
        &self,
        user_function: &UserFunction,
        arguments: &[Expression],
        this_expression: &Expression,
    ) -> bool {
        !self.expression_reads_local_descriptor_binding_member(this_expression)
            && (user_function.lexical_this || !matches!(this_expression, Expression::This))
            && self.inline_safe_argument_expression(this_expression)
            && !self.inline_argument_mentions_shadowed_implicit_global(this_expression)
            && arguments
                .iter()
                .all(|argument| self.inline_safe_argument_expression(argument))
            && !arguments
                .iter()
                .any(|argument| self.inline_argument_mentions_shadowed_implicit_global(argument))
            && !user_function.is_async()
            && !user_function.is_generator()
            && !user_function
                .inline_summary
                .as_ref()
                .is_some_and(inline_summary_mentions_assertion_builtin)
            && !self.user_function_mentions_private_member_access(user_function)
            && !self.user_function_mentions_direct_eval(user_function)
            && !self.user_function_contains_identifier_callee_call(user_function)
            && !self.user_function_may_read_restricted_function_property(user_function)
            && !self
                .backend
                .function_registry
                .analysis
                .user_function_capture_bindings
                .contains_key(&user_function.name)
            && !self.user_function_references_captured_user_function(user_function)
            && user_function.extra_argument_indices.is_empty()
            && !user_function.has_parameter_defaults()
            && (user_function
                .inline_summary
                .as_ref()
                .is_some_and(|summary| {
                    !inline_summary_mentions_assertion_builtin(summary)
                        && !inline_summary_mentions_unsupported_explicit_call_frame_state(summary)
                })
                || self
                    .user_function_has_explicit_call_frame_inlineable_terminal_body(user_function))
    }

    pub(in crate::backend::direct_wasm) fn can_inline_immediate_promise_assertion_callback_with_explicit_call_frame(
        &self,
        user_function: &UserFunction,
        arguments: &[Expression],
        this_expression: &Expression,
    ) -> bool {
        let result = matches!(arguments, [_])
            && !self.current_function_contains_try_statement()
            && !self.expression_reads_local_descriptor_binding_member(this_expression)
            && self.inline_safe_argument_expression(this_expression)
            && !self.inline_argument_mentions_shadowed_implicit_global(this_expression)
            && arguments
                .iter()
                .all(|argument| self.inline_safe_argument_expression(argument))
            && !arguments
                .iter()
                .any(|argument| self.inline_argument_mentions_shadowed_implicit_global(argument))
            && !user_function.is_async()
            && !user_function.is_generator()
            && !self
                .resolve_registered_function_declaration(&user_function.name)
                .is_some_and(|function| {
                    function
                        .body
                        .iter()
                        .any(Self::statement_mentions_private_member_access)
                })
            && !self.user_function_mentions_direct_eval(user_function)
            && !self.user_function_contains_identifier_callee_call(user_function)
            && !self
                .backend
                .function_registry
                .analysis
                .user_function_capture_bindings
                .contains_key(&user_function.name)
            && !self.user_function_references_captured_user_function(user_function)
            && user_function.extra_argument_indices.is_empty()
            && !user_function.has_parameter_defaults()
            && self.user_function_has_explicit_call_frame_inlineable_terminal_body(user_function);
        result
    }
}
