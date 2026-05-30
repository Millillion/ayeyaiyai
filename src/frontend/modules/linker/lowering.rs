use super::super::*;

impl ModuleLinker {
    fn static_module_seed_value(value: &Expression) -> bool {
        matches!(
            value,
            Expression::Number(_)
                | Expression::BigInt(_)
                | Expression::String(_)
                | Expression::Bool(_)
                | Expression::Null
                | Expression::Undefined
        ) || matches!(value, Expression::Identifier(name) if name.starts_with("__ayy_fnstmt_"))
    }

    fn static_module_seed_expression(value: &Expression) -> Option<Expression> {
        if Self::static_module_seed_value(value) {
            return Some(value.clone());
        }

        if let Expression::Await(awaited) = value
            && let Some(resolved) = Self::static_module_promise_resolve_value(awaited)
            && Self::static_module_seed_value(&resolved)
        {
            return Some(resolved);
        }

        None
    }

    fn static_module_promise_resolve_value(expression: &Expression) -> Option<Expression> {
        let Expression::Call { callee, arguments } = expression else {
            return None;
        };
        let Expression::Member { object, property } = callee.as_ref() else {
            return None;
        };
        if !matches!(object.as_ref(), Expression::Identifier(name) if name == "Promise")
            || !matches!(property.as_ref(), Expression::String(name) if name == "resolve")
            || arguments.len() > 1
        {
            return None;
        }
        match arguments.first() {
            Some(CallArgument::Expression(value)) => Some(value.clone()),
            Some(CallArgument::Spread(_)) => None,
            None => Some(Expression::Undefined),
        }
    }

    fn static_module_promise_then_reaction(
        statement: &Statement,
    ) -> Option<(Expression, Expression)> {
        let Statement::Expression(Expression::Call { callee, arguments }) = statement else {
            return None;
        };
        let Expression::Member { object, property } = callee.as_ref() else {
            return None;
        };
        if !matches!(property.as_ref(), Expression::String(name) if name == "then") {
            return None;
        }
        let value = Self::static_module_promise_resolve_value(object)?;
        let Some(CallArgument::Expression(handler)) = arguments.first() else {
            return None;
        };
        Some((handler.clone(), value))
    }

    fn substitute_static_module_reaction_argument(
        expression: &Expression,
        parameter: Option<&str>,
        argument: &Expression,
    ) -> Expression {
        match expression {
            Expression::Identifier(name) if parameter == Some(name.as_str()) => argument.clone(),
            Expression::Assign { name, value } => Expression::Assign {
                name: name.clone(),
                value: Box::new(Self::substitute_static_module_reaction_argument(
                    value, parameter, argument,
                )),
            },
            Expression::Member { object, property } => Expression::Member {
                object: Box::new(Self::substitute_static_module_reaction_argument(
                    object, parameter, argument,
                )),
                property: Box::new(Self::substitute_static_module_reaction_argument(
                    property, parameter, argument,
                )),
            },
            Expression::AssignMember {
                object,
                property,
                value,
            } => Expression::AssignMember {
                object: Box::new(Self::substitute_static_module_reaction_argument(
                    object, parameter, argument,
                )),
                property: Box::new(Self::substitute_static_module_reaction_argument(
                    property, parameter, argument,
                )),
                value: Box::new(Self::substitute_static_module_reaction_argument(
                    value, parameter, argument,
                )),
            },
            Expression::Await(value)
            | Expression::EnumerateKeys(value)
            | Expression::GetIterator(value)
            | Expression::IteratorClose(value)
            | Expression::Unary {
                expression: value, ..
            } => {
                let value =
                    Self::substitute_static_module_reaction_argument(value, parameter, argument);
                match expression {
                    Expression::Await(_) => Expression::Await(Box::new(value)),
                    Expression::EnumerateKeys(_) => Expression::EnumerateKeys(Box::new(value)),
                    Expression::GetIterator(_) => Expression::GetIterator(Box::new(value)),
                    Expression::IteratorClose(_) => Expression::IteratorClose(Box::new(value)),
                    Expression::Unary { op, .. } => Expression::Unary {
                        op: *op,
                        expression: Box::new(value),
                    },
                    _ => unreachable!("filtered above"),
                }
            }
            Expression::Binary { op, left, right } => Expression::Binary {
                op: *op,
                left: Box::new(Self::substitute_static_module_reaction_argument(
                    left, parameter, argument,
                )),
                right: Box::new(Self::substitute_static_module_reaction_argument(
                    right, parameter, argument,
                )),
            },
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => Expression::Conditional {
                condition: Box::new(Self::substitute_static_module_reaction_argument(
                    condition, parameter, argument,
                )),
                then_expression: Box::new(Self::substitute_static_module_reaction_argument(
                    then_expression,
                    parameter,
                    argument,
                )),
                else_expression: Box::new(Self::substitute_static_module_reaction_argument(
                    else_expression,
                    parameter,
                    argument,
                )),
            },
            Expression::Sequence(expressions) => Expression::Sequence(
                expressions
                    .iter()
                    .map(|expression| {
                        Self::substitute_static_module_reaction_argument(
                            expression, parameter, argument,
                        )
                    })
                    .collect(),
            ),
            Expression::Call { callee, arguments } => Expression::Call {
                callee: Box::new(Self::substitute_static_module_reaction_argument(
                    callee, parameter, argument,
                )),
                arguments: arguments
                    .iter()
                    .map(|call_argument| match call_argument {
                        CallArgument::Expression(expression) => CallArgument::Expression(
                            Self::substitute_static_module_reaction_argument(
                                expression, parameter, argument,
                            ),
                        ),
                        CallArgument::Spread(expression) => {
                            CallArgument::Spread(Self::substitute_static_module_reaction_argument(
                                expression, parameter, argument,
                            ))
                        }
                    })
                    .collect(),
            },
            _ => expression.clone(),
        }
    }

    fn static_module_reaction_assignment_effect(
        &self,
        handler: &Expression,
        argument: &Expression,
    ) -> Option<(String, Expression)> {
        let Expression::Identifier(function_name) = handler else {
            return None;
        };
        let function = self
            .lowerer
            .functions
            .iter()
            .rev()
            .find(|function| function.name == *function_name)?;
        let parameter = match function.params.as_slice() {
            [] => None,
            [parameter] if !parameter.rest && parameter.default.is_none() => {
                Some(parameter.name.as_str())
            }
            _ => return None,
        };
        let assignment = function.body.iter().find_map(|statement| match statement {
            Statement::Assign { name, value } => Some((name.clone(), value.clone())),
            Statement::Expression(Expression::Assign { name, value })
            | Statement::Return(Expression::Assign { name, value }) => {
                Some((name.clone(), value.as_ref().clone()))
            }
            _ => None,
        })?;
        Some((
            assignment.0,
            Self::substitute_static_module_reaction_argument(&assignment.1, parameter, argument),
        ))
    }

    fn apply_static_module_reaction_seed_effects(
        &self,
        seeds: &mut BTreeMap<String, Statement>,
        pending_reactions: &mut Vec<(Expression, Expression)>,
    ) {
        for (handler, argument) in pending_reactions.drain(..) {
            let Some((name, value)) =
                self.static_module_reaction_assignment_effect(&handler, &argument)
            else {
                continue;
            };
            if !seeds.contains_key(&name) {
                continue;
            }
            if Self::static_module_seed_value(&value) {
                seeds.insert(
                    name.clone(),
                    Statement::Let {
                        name,
                        mutable: true,
                        value,
                    },
                );
            } else {
                seeds.remove(&name);
            }
        }
    }

    fn static_module_seed_binding(statement: &Statement) -> Option<(String, Statement)> {
        match statement {
            Statement::Var { name, value } => {
                let value = Self::static_module_seed_expression(value)?;
                Some((
                    name.clone(),
                    Statement::Var {
                        name: name.clone(),
                        value,
                    },
                ))
            }
            Statement::Let {
                name,
                mutable,
                value,
            } => {
                let value = Self::static_module_seed_expression(value)?;
                Some((
                    name.clone(),
                    Statement::Let {
                        name: name.clone(),
                        mutable: *mutable,
                        value,
                    },
                ))
            }
            _ => None,
        }
    }

    fn refresh_static_module_seed_assignment(
        seeds: &mut BTreeMap<String, Statement>,
        name: &str,
        value: &Expression,
    ) {
        if !seeds.contains_key(name) {
            return;
        }

        if let Some(value) = Self::static_module_seed_expression(value) {
            seeds.insert(
                name.to_string(),
                Statement::Let {
                    name: name.to_string(),
                    mutable: true,
                    value,
                },
            );
        } else {
            seeds.remove(name);
        }
    }

    fn remove_static_module_seed_expression_assignments(
        expression: &Expression,
        seeds: &mut BTreeMap<String, Statement>,
    ) {
        match expression {
            Expression::Assign { name, value } => {
                Self::remove_static_module_seed_expression_assignments(value, seeds);
                Self::refresh_static_module_seed_assignment(seeds, name, value);
            }
            Expression::Update { name, .. } => {
                seeds.remove(name);
            }
            Expression::AssignMember {
                object,
                property,
                value,
            } => {
                Self::remove_static_module_seed_expression_assignments(object, seeds);
                Self::remove_static_module_seed_expression_assignments(property, seeds);
                Self::remove_static_module_seed_expression_assignments(value, seeds);
            }
            Expression::Member { object, property } => {
                Self::remove_static_module_seed_expression_assignments(object, seeds);
                Self::remove_static_module_seed_expression_assignments(property, seeds);
            }
            Expression::SuperMember { property } => {
                Self::remove_static_module_seed_expression_assignments(property, seeds);
            }
            Expression::AssignSuperMember { property, value } => {
                Self::remove_static_module_seed_expression_assignments(property, seeds);
                Self::remove_static_module_seed_expression_assignments(value, seeds);
            }
            Expression::Await(value)
            | Expression::EnumerateKeys(value)
            | Expression::GetIterator(value)
            | Expression::IteratorClose(value)
            | Expression::Unary {
                expression: value, ..
            } => Self::remove_static_module_seed_expression_assignments(value, seeds),
            Expression::Binary { left, right, .. } => {
                Self::remove_static_module_seed_expression_assignments(left, seeds);
                Self::remove_static_module_seed_expression_assignments(right, seeds);
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                Self::remove_static_module_seed_expression_assignments(condition, seeds);
                Self::remove_static_module_seed_expression_assignments(then_expression, seeds);
                Self::remove_static_module_seed_expression_assignments(else_expression, seeds);
            }
            Expression::Sequence(expressions) => {
                for expression in expressions {
                    Self::remove_static_module_seed_expression_assignments(expression, seeds);
                }
            }
            Expression::Call { callee, arguments }
            | Expression::SuperCall { callee, arguments }
            | Expression::New { callee, arguments } => {
                Self::remove_static_module_seed_expression_assignments(callee, seeds);
                for argument in arguments {
                    Self::remove_static_module_seed_expression_assignments(
                        argument.expression(),
                        seeds,
                    );
                }
            }
            Expression::Array(elements) => {
                for element in elements {
                    match element {
                        ArrayElement::Expression(expression) | ArrayElement::Spread(expression) => {
                            Self::remove_static_module_seed_expression_assignments(
                                expression, seeds,
                            );
                        }
                    }
                }
            }
            Expression::Object(entries) => {
                for entry in entries {
                    match entry {
                        ObjectEntry::Data { key, value } => {
                            Self::remove_static_module_seed_expression_assignments(key, seeds);
                            Self::remove_static_module_seed_expression_assignments(value, seeds);
                        }
                        ObjectEntry::Getter { key, getter } => {
                            Self::remove_static_module_seed_expression_assignments(key, seeds);
                            Self::remove_static_module_seed_expression_assignments(getter, seeds);
                        }
                        ObjectEntry::Setter { key, setter } => {
                            Self::remove_static_module_seed_expression_assignments(key, seeds);
                            Self::remove_static_module_seed_expression_assignments(setter, seeds);
                        }
                        ObjectEntry::Spread(expression) => {
                            Self::remove_static_module_seed_expression_assignments(
                                expression, seeds,
                            );
                        }
                    }
                }
            }
            Expression::Identifier(_)
            | Expression::Number(_)
            | Expression::BigInt(_)
            | Expression::String(_)
            | Expression::Bool(_)
            | Expression::Null
            | Expression::Undefined
            | Expression::This
            | Expression::NewTarget
            | Expression::Sent => {}
        }
    }

    fn refresh_static_module_seed_bindings(
        seeds: &mut BTreeMap<String, Statement>,
        statement: &Statement,
    ) {
        if let Some((name, seed)) = Self::static_module_seed_binding(statement) {
            seeds.insert(name, seed);
            return;
        }

        match statement {
            Statement::Var { name, .. } | Statement::Let { name, .. } => {
                seeds.remove(name);
            }
            Statement::Assign { name, value } => {
                Self::refresh_static_module_seed_assignment(seeds, name, value);
            }
            Statement::Expression(expression)
            | Statement::Throw(expression)
            | Statement::Return(expression)
            | Statement::Yield { value: expression }
            | Statement::YieldDelegate { value: expression } => {
                Self::remove_static_module_seed_expression_assignments(expression, seeds);
            }
            Statement::AssignMember {
                object,
                property,
                value,
            } => {
                Self::remove_static_module_seed_expression_assignments(object, seeds);
                Self::remove_static_module_seed_expression_assignments(property, seeds);
                Self::remove_static_module_seed_expression_assignments(value, seeds);
            }
            Statement::Print { values } => {
                for value in values {
                    Self::remove_static_module_seed_expression_assignments(value, seeds);
                }
            }
            Statement::Declaration { body }
            | Statement::Block { body }
            | Statement::Labeled { body, .. }
            | Statement::With { body, .. } => {
                for statement in body {
                    Self::refresh_static_module_seed_bindings(seeds, statement);
                }
            }
            Statement::If {
                condition,
                then_branch,
                else_branch,
            } => {
                Self::remove_static_module_seed_expression_assignments(condition, seeds);
                for statement in then_branch {
                    Self::refresh_static_module_seed_bindings(seeds, statement);
                }
                for statement in else_branch {
                    Self::refresh_static_module_seed_bindings(seeds, statement);
                }
            }
            Statement::Try {
                body,
                catch_setup,
                catch_body,
                ..
            } => {
                for statement in body {
                    Self::refresh_static_module_seed_bindings(seeds, statement);
                }
                for statement in catch_setup {
                    Self::refresh_static_module_seed_bindings(seeds, statement);
                }
                for statement in catch_body {
                    Self::refresh_static_module_seed_bindings(seeds, statement);
                }
            }
            Statement::Switch {
                discriminant,
                cases,
                ..
            } => {
                Self::remove_static_module_seed_expression_assignments(discriminant, seeds);
                for case in cases {
                    if let Some(test) = &case.test {
                        Self::remove_static_module_seed_expression_assignments(test, seeds);
                    }
                    for statement in &case.body {
                        Self::refresh_static_module_seed_bindings(seeds, statement);
                    }
                }
            }
            Statement::For {
                init,
                condition,
                update,
                break_hook,
                body,
                ..
            } => {
                for statement in init {
                    Self::refresh_static_module_seed_bindings(seeds, statement);
                }
                if let Some(condition) = condition {
                    Self::remove_static_module_seed_expression_assignments(condition, seeds);
                }
                if let Some(update) = update {
                    Self::remove_static_module_seed_expression_assignments(update, seeds);
                }
                if let Some(break_hook) = break_hook {
                    Self::remove_static_module_seed_expression_assignments(break_hook, seeds);
                }
                for statement in body {
                    Self::refresh_static_module_seed_bindings(seeds, statement);
                }
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
                Self::remove_static_module_seed_expression_assignments(condition, seeds);
                if let Some(break_hook) = break_hook {
                    Self::remove_static_module_seed_expression_assignments(break_hook, seeds);
                }
                for statement in body {
                    Self::refresh_static_module_seed_bindings(seeds, statement);
                }
            }
            Statement::Break { .. } | Statement::Continue { .. } => {}
        }
    }

    fn collect_static_module_referenced_names_from_expression(
        expression: &Expression,
        names: &mut HashSet<String>,
    ) {
        match expression {
            Expression::Identifier(name) | Expression::Update { name, .. } => {
                names.insert(name.clone());
            }
            Expression::Assign { name, value } => {
                names.insert(name.clone());
                Self::collect_static_module_referenced_names_from_expression(value, names);
            }
            Expression::Member { object, property }
            | Expression::AssignMember {
                object,
                property,
                value: _,
            } => {
                Self::collect_static_module_referenced_names_from_expression(object, names);
                Self::collect_static_module_referenced_names_from_expression(property, names);
                if let Expression::AssignMember { value, .. } = expression {
                    Self::collect_static_module_referenced_names_from_expression(value, names);
                }
            }
            Expression::SuperMember { property } => {
                Self::collect_static_module_referenced_names_from_expression(property, names);
            }
            Expression::AssignSuperMember { property, value } => {
                Self::collect_static_module_referenced_names_from_expression(property, names);
                Self::collect_static_module_referenced_names_from_expression(value, names);
            }
            Expression::Await(value)
            | Expression::EnumerateKeys(value)
            | Expression::GetIterator(value)
            | Expression::IteratorClose(value)
            | Expression::Unary {
                expression: value, ..
            } => Self::collect_static_module_referenced_names_from_expression(value, names),
            Expression::Binary { left, right, .. } => {
                Self::collect_static_module_referenced_names_from_expression(left, names);
                Self::collect_static_module_referenced_names_from_expression(right, names);
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                Self::collect_static_module_referenced_names_from_expression(condition, names);
                Self::collect_static_module_referenced_names_from_expression(
                    then_expression,
                    names,
                );
                Self::collect_static_module_referenced_names_from_expression(
                    else_expression,
                    names,
                );
            }
            Expression::Sequence(expressions) => {
                for expression in expressions {
                    Self::collect_static_module_referenced_names_from_expression(expression, names);
                }
            }
            Expression::Call { callee, arguments }
            | Expression::SuperCall { callee, arguments }
            | Expression::New { callee, arguments } => {
                Self::collect_static_module_referenced_names_from_expression(callee, names);
                for argument in arguments {
                    Self::collect_static_module_referenced_names_from_expression(
                        argument.expression(),
                        names,
                    );
                }
            }
            Expression::Array(elements) => {
                for element in elements {
                    match element {
                        ArrayElement::Expression(expression) | ArrayElement::Spread(expression) => {
                            Self::collect_static_module_referenced_names_from_expression(
                                expression, names,
                            );
                        }
                    }
                }
            }
            Expression::Object(entries) => {
                for entry in entries {
                    match entry {
                        ObjectEntry::Data { key, value } => {
                            Self::collect_static_module_referenced_names_from_expression(
                                key, names,
                            );
                            Self::collect_static_module_referenced_names_from_expression(
                                value, names,
                            );
                        }
                        ObjectEntry::Getter { key, getter } => {
                            Self::collect_static_module_referenced_names_from_expression(
                                key, names,
                            );
                            Self::collect_static_module_referenced_names_from_expression(
                                getter, names,
                            );
                        }
                        ObjectEntry::Setter { key, setter } => {
                            Self::collect_static_module_referenced_names_from_expression(
                                key, names,
                            );
                            Self::collect_static_module_referenced_names_from_expression(
                                setter, names,
                            );
                        }
                        ObjectEntry::Spread(expression) => {
                            Self::collect_static_module_referenced_names_from_expression(
                                expression, names,
                            );
                        }
                    }
                }
            }
            Expression::Number(_)
            | Expression::BigInt(_)
            | Expression::String(_)
            | Expression::Bool(_)
            | Expression::Null
            | Expression::Undefined
            | Expression::This
            | Expression::NewTarget
            | Expression::Sent => {}
        }
    }

    fn collect_static_module_referenced_names_from_statement(
        statement: &Statement,
        names: &mut HashSet<String>,
    ) {
        match statement {
            Statement::Declaration { body }
            | Statement::Block { body }
            | Statement::Labeled { body, .. } => {
                for statement in body {
                    Self::collect_static_module_referenced_names_from_statement(statement, names);
                }
            }
            Statement::Var { value, .. }
            | Statement::Let { value, .. }
            | Statement::Expression(value)
            | Statement::Throw(value)
            | Statement::Return(value)
            | Statement::Yield { value }
            | Statement::YieldDelegate { value } => {
                Self::collect_static_module_referenced_names_from_expression(value, names);
            }
            Statement::Assign { name, value } => {
                names.insert(name.clone());
                Self::collect_static_module_referenced_names_from_expression(value, names);
            }
            Statement::AssignMember {
                object,
                property,
                value,
            } => {
                Self::collect_static_module_referenced_names_from_expression(object, names);
                Self::collect_static_module_referenced_names_from_expression(property, names);
                Self::collect_static_module_referenced_names_from_expression(value, names);
            }
            Statement::Print { values } => {
                for value in values {
                    Self::collect_static_module_referenced_names_from_expression(value, names);
                }
            }
            Statement::With { object, body } => {
                Self::collect_static_module_referenced_names_from_expression(object, names);
                for statement in body {
                    Self::collect_static_module_referenced_names_from_statement(statement, names);
                }
            }
            Statement::If {
                condition,
                then_branch,
                else_branch,
            } => {
                Self::collect_static_module_referenced_names_from_expression(condition, names);
                for statement in then_branch {
                    Self::collect_static_module_referenced_names_from_statement(statement, names);
                }
                for statement in else_branch {
                    Self::collect_static_module_referenced_names_from_statement(statement, names);
                }
            }
            Statement::Try {
                body,
                catch_setup,
                catch_body,
                ..
            } => {
                for statement in body {
                    Self::collect_static_module_referenced_names_from_statement(statement, names);
                }
                for statement in catch_setup {
                    Self::collect_static_module_referenced_names_from_statement(statement, names);
                }
                for statement in catch_body {
                    Self::collect_static_module_referenced_names_from_statement(statement, names);
                }
            }
            Statement::Switch {
                discriminant,
                cases,
                ..
            } => {
                Self::collect_static_module_referenced_names_from_expression(discriminant, names);
                for case in cases {
                    if let Some(test) = &case.test {
                        Self::collect_static_module_referenced_names_from_expression(test, names);
                    }
                    for statement in &case.body {
                        Self::collect_static_module_referenced_names_from_statement(
                            statement, names,
                        );
                    }
                }
            }
            Statement::For {
                init,
                condition,
                update,
                break_hook,
                body,
                ..
            } => {
                for statement in init {
                    Self::collect_static_module_referenced_names_from_statement(statement, names);
                }
                if let Some(condition) = condition {
                    Self::collect_static_module_referenced_names_from_expression(condition, names);
                }
                if let Some(update) = update {
                    Self::collect_static_module_referenced_names_from_expression(update, names);
                }
                if let Some(break_hook) = break_hook {
                    Self::collect_static_module_referenced_names_from_expression(break_hook, names);
                }
                for statement in body {
                    Self::collect_static_module_referenced_names_from_statement(statement, names);
                }
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
                Self::collect_static_module_referenced_names_from_expression(condition, names);
                if let Some(break_hook) = break_hook {
                    Self::collect_static_module_referenced_names_from_expression(break_hook, names);
                }
                for statement in body {
                    Self::collect_static_module_referenced_names_from_statement(statement, names);
                }
            }
            Statement::Break { .. } | Statement::Continue { .. } => {}
        }
    }

    fn static_module_referenced_names(statements: &[Statement]) -> HashSet<String> {
        let mut names = HashSet::new();
        for statement in statements {
            Self::collect_static_module_referenced_names_from_statement(statement, &mut names);
        }
        names
    }

    fn seed_static_module_continuation_bindings(
        &self,
        init_body: &[Statement],
        continuation_bodies: Vec<Vec<Statement>>,
    ) -> Vec<Vec<Statement>> {
        let mut seeds = BTreeMap::<String, Statement>::new();
        let mut pending_reactions = Vec::<(Expression, Expression)>::new();
        for statement in init_body {
            if let Some(reaction) = Self::static_module_promise_then_reaction(statement) {
                pending_reactions.push(reaction);
            }
            Self::refresh_static_module_seed_bindings(&mut seeds, statement);
            if Self::static_module_await_boundary_effect(statement).is_some() {
                self.apply_static_module_reaction_seed_effects(&mut seeds, &mut pending_reactions);
            }
        }

        continuation_bodies
            .into_iter()
            .map(|body| {
                let referenced = Self::static_module_referenced_names(&body);
                let mut seeded_body = seeds
                    .iter()
                    .filter_map(|(name, statement)| referenced.contains(name).then_some(statement))
                    .cloned()
                    .collect::<Vec<_>>();

                for statement in &body {
                    if let Some(reaction) = Self::static_module_promise_then_reaction(statement) {
                        pending_reactions.push(reaction);
                    }
                    Self::refresh_static_module_seed_bindings(&mut seeds, statement);
                    if Self::static_module_await_boundary_effect(statement).is_some() {
                        self.apply_static_module_reaction_seed_effects(
                            &mut seeds,
                            &mut pending_reactions,
                        );
                    }
                }

                seeded_body.extend(body);
                seeded_body
            })
            .collect()
    }

    fn static_module_await_boundary_effect(statement: &Statement) -> Option<Statement> {
        match statement {
            Statement::Expression(Expression::Await(_))
            | Statement::Var {
                value: Expression::Await(_),
                ..
            }
            | Statement::Let {
                value: Expression::Await(_),
                ..
            }
            | Statement::Assign {
                value: Expression::Await(_),
                ..
            }
            | Statement::Return(Expression::Await(_))
            | Statement::If {
                condition: Expression::Await(_),
                ..
            } => Some(statement.clone()),
            _ => None,
        }
    }

    fn split_static_module_await_segments(
        statements: Vec<Statement>,
    ) -> (Vec<Statement>, Vec<Vec<Statement>>) {
        let mut segments = vec![Vec::new()];
        for statement in statements {
            if let Some(effect) = Self::static_module_await_boundary_effect(&statement) {
                if let Some(segment) = segments.last_mut() {
                    segment.push(effect);
                }
                segments.push(Vec::new());
            } else if let Some(segment) = segments.last_mut() {
                segment.push(statement);
            }
        }

        let mut segments = segments.into_iter();
        let init_body = segments.next().unwrap_or_default();
        let continuations = segments
            .filter(|segment| !segment.is_empty())
            .collect::<Vec<_>>();
        (init_body, continuations)
    }

    fn retarget_self_import_binding(
        import_bindings: &mut HashMap<String, ImportBinding>,
        module_index: usize,
        export_name: &str,
        binding_name: &str,
    ) {
        for binding in import_bindings.values_mut() {
            let ImportBinding::Named {
                module_index: imported_module_index,
                export_name: imported_export_name,
                self_local_binding,
                ..
            } = binding
            else {
                continue;
            };
            if *imported_module_index == module_index && imported_export_name == export_name {
                *self_local_binding = Some(binding_name.to_string());
            }
        }
    }

    pub(crate) fn lower_module(
        &mut self,
        module_index: usize,
        module: &Module,
        source_text: String,
    ) -> Result<()> {
        let exports_param = "exports".to_string();
        let module_path = self.modules[module_index].path.clone();
        ensure_module_lexical_names_are_unique(module)?;
        let module_declared_names = collect_module_declared_names(module)?;
        self.modules[module_index]
            .pending_import_resolutions
            .clear();
        let mut dependency_params = Vec::new();
        let mut dependency_param_by_index = HashMap::new();
        let mut import_bindings = HashMap::new();
        let mut export_expressions = BTreeMap::<String, Expression>::new();
        let mut namespace_export_module_indices = BTreeMap::<String, usize>::new();
        let mut reexport_sources = BTreeMap::<String, (usize, String)>::new();
        let mut export_resolutions = self.modules[module_index].export_resolutions.clone();
        let mut late_export_getters = HashSet::<String>::new();
        let mut star_export_expressions = BTreeMap::<String, Expression>::new();
        let mut star_export_resolutions = BTreeMap::<String, ExportResolution>::new();
        let mut star_reexport_sources = BTreeMap::<String, (usize, String)>::new();
        let mut ambiguous_star_exports = HashSet::<String>::new();
        let mut pending_self_reexports = Vec::<(String, String)>::new();
        let mut hoisted_statements = Vec::new();
        let mut body_statements = Vec::new();

        let dynamic_import_sources =
            self.dynamic_import_specifier_sources_for_module(module, &source_text);
        for source in &dynamic_import_sources {
            if let Ok(dependency_path) = resolve_module_specifier(&module_path, source) {
                self.load_dynamic_module_with_type(&dependency_path, None)?;
            }
        }

        for item in &module.body {
            let ModuleItem::ModuleDecl(module_declaration) = item else {
                continue;
            };
            match module_declaration {
                ModuleDecl::Import(import) => self.register_import_declaration(
                    module_index,
                    &module_path,
                    import,
                    &mut dependency_params,
                    &mut dependency_param_by_index,
                    &mut import_bindings,
                )?,
                ModuleDecl::ExportNamed(export_named) => {
                    if let Some(source) = &export_named.src {
                        self.dependency_param_for_source(
                            &module_path,
                            &source.value.to_string_lossy(),
                            import_attribute_type(export_named.with.as_deref())?.as_deref(),
                            true,
                            &mut dependency_params,
                            &mut dependency_param_by_index,
                        )?;
                    }
                }
                ModuleDecl::ExportAll(export_all) => {
                    self.dependency_param_for_source(
                        &module_path,
                        &export_all.src.value.to_string_lossy(),
                        import_attribute_type(export_all.with.as_deref())?.as_deref(),
                        true,
                        &mut dependency_params,
                        &mut dependency_param_by_index,
                    )?;
                }
                _ => {}
            }
        }

        self.lowerer.strict_modes.push(true);
        self.lowerer.module_mode = true;
        self.lowerer.source_text = Some(source_text);
        self.lowerer.current_module_path = Some(module_path.clone());
        self.lowerer.module_index_lookup = self.module_indices.clone();
        self.lowerer.dynamic_import_specifier_lookup =
            self.dynamic_import_specifier_index_lookup(&module_path, &dynamic_import_sources);
        let function_start = self.lowerer.functions.len();

        for item in &module.body {
            match item {
                ModuleItem::Stmt(statement) => match statement {
                    Stmt::Decl(Decl::Fn(function_declaration)) => hoisted_statements.extend(
                        self.lowerer
                            .lower_nested_function_declaration(function_declaration)?,
                    ),
                    other => {
                        body_statements.extend(self.lowerer.lower_statement(other, false, false)?)
                    }
                },
                ModuleItem::ModuleDecl(module_declaration) => match module_declaration {
                    ModuleDecl::Import(_) => {}
                    ModuleDecl::ExportDecl(export) => match &export.decl {
                        Decl::Fn(function_declaration) => {
                            hoisted_statements.extend(
                                self.lowerer
                                    .lower_nested_function_declaration(function_declaration)?,
                            );
                            let export_name = function_declaration.ident.sym.to_string();
                            export_expressions.insert(
                                export_name.clone(),
                                Expression::Identifier(export_name.clone()),
                            );
                            late_export_getters.insert(export_name.clone());
                            export_resolutions.insert(
                                export_name.clone(),
                                ExportResolution::Binding {
                                    module_index,
                                    binding_name: export_name,
                                    local: true,
                                },
                            );
                        }
                        Decl::Var(variable_declaration) => {
                            body_statements.extend(
                                self.lowerer
                                    .lower_variable_declaration(variable_declaration)?,
                            );
                            for name in collect_var_decl_bound_names(variable_declaration)? {
                                export_expressions
                                    .insert(name.clone(), Expression::Identifier(name.clone()));
                                export_resolutions.insert(
                                    name.clone(),
                                    ExportResolution::Binding {
                                        module_index,
                                        binding_name: name,
                                        local: true,
                                    },
                                );
                            }
                        }
                        Decl::Class(class_declaration) => {
                            body_statements
                                .extend(self.lowerer.lower_class_declaration(class_declaration)?);
                            let export_name = class_declaration.ident.sym.to_string();
                            export_expressions.insert(
                                export_name.clone(),
                                Expression::Identifier(export_name.clone()),
                            );
                            export_resolutions.insert(
                                export_name.clone(),
                                ExportResolution::Binding {
                                    module_index,
                                    binding_name: export_name,
                                    local: true,
                                },
                            );
                        }
                        other => bail!("unsupported export declaration: {other:?}"),
                    },
                    ModuleDecl::ExportDefaultDecl(export_default) => {
                        let default_uses_hoisted_binding =
                            matches!(export_default.decl, DefaultDecl::Fn(_));
                        let default_is_class = matches!(export_default.decl, DefaultDecl::Class(_));
                        let expression = self.lower_default_export_declaration(
                            export_default,
                            &mut hoisted_statements,
                            &mut body_statements,
                        )?;
                        export_expressions.insert("default".to_string(), expression.clone());
                        if default_is_class || default_uses_hoisted_binding {
                            late_export_getters.insert("default".to_string());
                        }
                        export_resolutions.insert(
                            "default".to_string(),
                            ExportResolution::Binding {
                                module_index,
                                binding_name: match expression {
                                    Expression::Identifier(name) => name,
                                    _ => "default".to_string(),
                                },
                                local: true,
                            },
                        );
                        if let Some(Expression::Identifier(binding_name)) =
                            export_expressions.get("default")
                        {
                            Self::retarget_self_import_binding(
                                &mut import_bindings,
                                module_index,
                                "default",
                                binding_name,
                            );
                        }
                    }
                    ModuleDecl::ExportDefaultExpr(export_default) => {
                        let local_name = self.lowerer.fresh_temporary_name("module_default");
                        body_statements.push(Statement::Let {
                            name: local_name.clone(),
                            mutable: false,
                            value: self.lowerer.lower_expression_with_name_hint(
                                &export_default.expr,
                                Some("default"),
                            )?,
                        });
                        export_expressions.insert(
                            "default".to_string(),
                            Expression::Identifier(local_name.clone()),
                        );
                        late_export_getters.insert("default".to_string());
                        export_resolutions.insert(
                            "default".to_string(),
                            ExportResolution::Binding {
                                module_index,
                                binding_name: local_name.clone(),
                                local: true,
                            },
                        );
                        Self::retarget_self_import_binding(
                            &mut import_bindings,
                            module_index,
                            "default",
                            &local_name,
                        );
                    }
                    ModuleDecl::ExportNamed(export_named) if export_named.src.is_none() => {
                        for specifier in &export_named.specifiers {
                            match specifier {
                                ExportSpecifier::Named(named) => {
                                    let local_name = module_export_name_string(&named.orig)?;
                                    ensure!(
                                        import_bindings.contains_key(&local_name)
                                            || module_declared_names.contains(&local_name),
                                        "unresolvable export `{local_name}`"
                                    );
                                    let export_name = named
                                        .exported
                                        .as_ref()
                                        .map(module_export_name_string)
                                        .transpose()?
                                        .unwrap_or_else(|| local_name.clone());
                                    export_expressions.insert(
                                        export_name.clone(),
                                        Expression::Identifier(local_name.clone()),
                                    );
                                    let resolution = import_bindings
                                        .get(&local_name)
                                        .map(|binding| {
                                            self.export_resolution_for_import_binding(binding)
                                        })
                                        .transpose()?
                                        .unwrap_or_else(|| ExportResolution::Binding {
                                            module_index,
                                            binding_name: local_name,
                                            local: true,
                                        });
                                    export_resolutions.insert(export_name, resolution);
                                }
                                other => bail!("unsupported local export specifier: {other:?}"),
                            }
                        }
                    }
                    ModuleDecl::ExportNamed(export_named) => {
                        let source = export_named
                            .src
                            .as_ref()
                            .context("re-export must have a source")?;
                        let namespace_param = self.dependency_param_for_source(
                            &module_path,
                            &source.value.to_string_lossy(),
                            import_attribute_type(export_named.with.as_deref())?.as_deref(),
                            true,
                            &mut dependency_params,
                            &mut dependency_param_by_index,
                        )?;
                        let dependency_index = dependency_params
                            .iter()
                            .find(|dependency| dependency.param_name == namespace_param)
                            .map(|dependency| dependency.module_index)
                            .context("re-export dependency must be registered")?;
                        let dependency_finalized = self.load_order.contains(&dependency_index);
                        let self_reexport = dependency_index == module_index;
                        for specifier in &export_named.specifiers {
                            match specifier {
                                ExportSpecifier::Named(named) => {
                                    let imported_name = module_export_name_string(&named.orig)?;
                                    let export_name = named
                                        .exported
                                        .as_ref()
                                        .map(module_export_name_string)
                                        .transpose()?
                                        .unwrap_or_else(|| imported_name.clone());
                                    export_expressions.insert(
                                        export_name.clone(),
                                        Expression::Member {
                                            object: Box::new(Expression::Identifier(
                                                namespace_param.clone(),
                                            )),
                                            property: Box::new(Expression::String(
                                                imported_name.clone(),
                                            )),
                                        },
                                    );
                                    if !self_reexport {
                                        reexport_sources.insert(
                                            export_name.clone(),
                                            (dependency_index, imported_name.clone()),
                                        );
                                    }
                                    if self_reexport {
                                        pending_self_reexports.push((export_name, imported_name));
                                    } else if !dependency_finalized {
                                        export_resolutions.insert(
                                            export_name,
                                            ExportResolution::Binding {
                                                module_index: dependency_index,
                                                binding_name: imported_name,
                                                local: false,
                                            },
                                        );
                                    } else {
                                        export_resolutions.insert(
                                            export_name,
                                            self.require_export_resolution_for_dependency(
                                                dependency_index,
                                                &imported_name,
                                            )?,
                                        );
                                    }
                                }
                                ExportSpecifier::Namespace(namespace) => {
                                    let export_name = module_export_name_string(&namespace.name)?;
                                    export_expressions.insert(
                                        export_name.clone(),
                                        Expression::Identifier(namespace_param.clone()),
                                    );
                                    namespace_export_module_indices
                                        .insert(export_name.clone(), dependency_index);
                                    export_resolutions.insert(
                                        export_name,
                                        ExportResolution::Namespace {
                                            module_index: dependency_index,
                                        },
                                    );
                                }
                                ExportSpecifier::Default(default) => {
                                    let export_name = default.exported.sym.to_string();
                                    export_expressions.insert(
                                        export_name.clone(),
                                        Expression::Member {
                                            object: Box::new(Expression::Identifier(
                                                namespace_param.clone(),
                                            )),
                                            property: Box::new(Expression::String(
                                                "default".to_string(),
                                            )),
                                        },
                                    );
                                    if !self_reexport {
                                        reexport_sources.insert(
                                            export_name.clone(),
                                            (dependency_index, "default".to_string()),
                                        );
                                    }
                                    if self_reexport {
                                        pending_self_reexports
                                            .push((export_name, "default".to_string()));
                                    } else if !dependency_finalized {
                                        export_resolutions.insert(
                                            export_name,
                                            ExportResolution::Binding {
                                                module_index: dependency_index,
                                                binding_name: "default".to_string(),
                                                local: false,
                                            },
                                        );
                                    } else {
                                        export_resolutions.insert(
                                            export_name,
                                            self.require_export_resolution_for_dependency(
                                                dependency_index,
                                                "default",
                                            )?,
                                        );
                                    }
                                }
                            }
                        }
                    }
                    ModuleDecl::ExportAll(export_all) => {
                        let namespace_param = self.dependency_param_for_source(
                            &module_path,
                            &export_all.src.value.to_string_lossy(),
                            import_attribute_type(export_all.with.as_deref())?.as_deref(),
                            true,
                            &mut dependency_params,
                            &mut dependency_param_by_index,
                        )?;
                        let dependency_index = dependency_params
                            .iter()
                            .find(|dependency| dependency.param_name == namespace_param)
                            .map(|dependency| dependency.module_index)
                            .context("export-all dependency must be registered")?;
                        let dependency_export_resolutions =
                            self.modules[dependency_index].export_resolutions.clone();
                        for (export_name, resolution) in dependency_export_resolutions {
                            if export_name == "default" {
                                continue;
                            }
                            if export_resolutions.contains_key(export_name.as_str())
                                || ambiguous_star_exports.contains(export_name.as_str())
                            {
                                continue;
                            }
                            let resolution = self.canonicalize_export_resolution(resolution)?;

                            let expression = Expression::Member {
                                object: Box::new(Expression::Identifier(namespace_param.clone())),
                                property: Box::new(Expression::String(export_name.clone())),
                            };
                            if let Some(previous_resolution) =
                                star_export_resolutions.get(&export_name)
                            {
                                if previous_resolution != &resolution {
                                    star_export_expressions.remove(&export_name);
                                    star_export_resolutions.remove(&export_name);
                                    star_reexport_sources.remove(&export_name);
                                    ambiguous_star_exports.insert(export_name);
                                }
                            } else {
                                star_export_expressions.insert(export_name.clone(), expression);
                                star_export_resolutions.insert(export_name.clone(), resolution);
                                star_reexport_sources
                                    .insert(export_name.clone(), (dependency_index, export_name));
                            }
                        }
                    }
                    other => bail!("unsupported module declaration: {other:?}"),
                },
            }
        }

        for (export_name, expression) in star_export_expressions {
            if !export_expressions.contains_key(&export_name) {
                if let Some(source) = star_reexport_sources.get(&export_name).cloned() {
                    reexport_sources.insert(export_name.clone(), source);
                }
                export_expressions.insert(export_name, expression);
            }
        }
        for (export_name, resolution) in star_export_resolutions {
            if !export_resolutions.contains_key(&export_name) {
                export_resolutions.insert(export_name, resolution);
            }
        }
        for (export_name, imported_name) in pending_self_reexports {
            if let Some(resolution) = export_resolutions.get(&imported_name).cloned() {
                export_resolutions.insert(export_name, resolution);
            } else if let Some(Expression::Identifier(binding_name)) =
                export_expressions.get(&imported_name)
            {
                export_resolutions.insert(
                    export_name,
                    ExportResolution::Binding {
                        module_index,
                        binding_name: binding_name.clone(),
                        local: true,
                    },
                );
            } else if ambiguous_star_exports.contains(&imported_name) {
                bail!(
                    "ambiguous export `{imported_name}` in `{}`",
                    self.modules[module_index].path.display()
                );
            } else {
                bail!(
                    "missing export `{imported_name}` in `{}`",
                    self.modules[module_index].path.display()
                );
            }
        }

        self.lowerer.strict_modes.pop();
        self.lowerer.module_mode = false;
        self.lowerer.source_text = None;
        self.lowerer.current_module_path = None;
        self.lowerer.module_index_lookup.clear();
        self.lowerer.dynamic_import_specifier_lookup.clear();

        self.rewrite_module_import_bindings_in_statements(
            module_index,
            &mut hoisted_statements,
            &import_bindings,
        )?;
        self.rewrite_module_import_bindings_in_statements(
            module_index,
            &mut body_statements,
            &import_bindings,
        )?;
        for function in &mut self.lowerer.functions[function_start..] {
            rewrite_module_import_bindings_in_function(function, &import_bindings, module_index)?;
        }

        let early_export_expressions = export_expressions
            .iter()
            .filter(|(name, _)| !late_export_getters.contains(*name))
            .map(|(name, expression)| (name.clone(), expression.clone()))
            .collect::<BTreeMap<_, _>>();
        let late_export_expressions = export_expressions
            .iter()
            .filter(|(name, _)| late_export_getters.contains(*name))
            .map(|(name, expression)| (name.clone(), expression.clone()))
            .collect::<BTreeMap<_, _>>();

        let deferred_exports = self.modules[module_index].deferred_namespace_name.clone();
        let mut init_body = self.build_module_namespace_prelude(&exports_param);
        init_body.extend(
            self.build_module_namespace_prelude_with_tag(&deferred_exports, "Deferred Module"),
        );
        init_body.extend(self.build_export_getter_statements(
            module_index,
            &exports_param,
            &early_export_expressions,
            &namespace_export_module_indices,
            &reexport_sources,
            &import_bindings,
        )?);
        init_body.extend(self.build_export_getter_statements(
            module_index,
            &deferred_exports,
            &early_export_expressions,
            &namespace_export_module_indices,
            &reexport_sources,
            &import_bindings,
        )?);
        init_body.extend(hoisted_statements);
        init_body.extend(body_statements);
        init_body.extend(self.build_export_getter_statements(
            module_index,
            &exports_param,
            &late_export_expressions,
            &namespace_export_module_indices,
            &reexport_sources,
            &import_bindings,
        )?);
        init_body.extend(self.build_export_getter_statements(
            module_index,
            &deferred_exports,
            &late_export_expressions,
            &namespace_export_module_indices,
            &reexport_sources,
            &import_bindings,
        )?);
        self.mark_module_init_body_status(module_index, &mut init_body);
        let (init_body, continuation_bodies) = Self::split_static_module_await_segments(init_body);
        let continuation_bodies =
            self.seed_static_module_continuation_bindings(&init_body, continuation_bodies);
        let (init_body, init_async_lowered) = asyncify_statements(init_body);

        let mut params = vec![Parameter {
            name: exports_param,
            default: None,
            rest: false,
        }];
        params.extend(dependency_params.iter().map(|dependency| Parameter {
            name: dependency.param_name.clone(),
            default: None,
            rest: false,
        }));
        let async_continuation_names = continuation_bodies
            .iter()
            .enumerate()
            .map(|(index, _)| format!("__ayy_module_async_continuation_{module_index}_{index}"))
            .collect::<Vec<_>>();

        self.lowerer.functions.push(FunctionDeclaration {
            name: self.modules[module_index].init_name.clone(),
            top_level_binding: None,
            params: params.clone(),
            body: init_body,
            register_global: false,
            kind: FunctionKind::from_flags(
                false,
                init_async_lowered || !async_continuation_names.is_empty(),
            ),
            self_binding: None,
            mapped_arguments: false,
            strict: true,
            lexical_this: false,
            constructible: true,
            derived_constructor: false,
            direct_eval_in_class_field_initializer: false,
            length: dependency_params.len() + 1,
            synthetic_capture_bindings: Vec::new(),
            immutable_class_bindings: Vec::new(),
            private_brand_binding: None,
        });

        for (continuation_name, continuation_body) in async_continuation_names
            .iter()
            .zip(continuation_bodies.into_iter())
        {
            let (continuation_body, continuation_async) = asyncify_statements(continuation_body);
            self.lowerer.functions.push(FunctionDeclaration {
                name: continuation_name.clone(),
                top_level_binding: None,
                params: params.clone(),
                body: continuation_body,
                register_global: false,
                kind: FunctionKind::from_flags(false, continuation_async),
                self_binding: None,
                mapped_arguments: false,
                strict: true,
                lexical_this: false,
                constructible: true,
                derived_constructor: false,
                direct_eval_in_class_field_initializer: false,
                length: dependency_params.len() + 1,
                synthetic_capture_bindings: Vec::new(),
                immutable_class_bindings: Vec::new(),
                private_brand_binding: None,
            });
        }

        self.modules[module_index].async_continuation_names = async_continuation_names;
        self.modules[module_index].init_async = init_async_lowered
            || !self.modules[module_index]
                .async_continuation_names
                .is_empty();
        self.modules[module_index].dependency_params = dependency_params;
        self.modules[module_index].export_names = export_expressions.keys().cloned().collect();
        self.modules[module_index].export_resolutions = export_resolutions;
        self.modules[module_index].ambiguous_export_names = ambiguous_star_exports;

        Ok(())
    }
}
