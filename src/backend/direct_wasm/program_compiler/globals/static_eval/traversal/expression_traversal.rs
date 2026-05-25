use super::*;

impl DirectWasmCompiler {
    fn resolve_static_eval_discovery_alias(&self, expression: &Expression) -> Expression {
        let mut current = expression.clone();
        let mut visited = HashSet::new();
        while let Expression::Identifier(name) = &current {
            if !visited.insert(name.clone()) {
                break;
            }
            let Some(next) = self.state.global_semantics.values.value_binding(name) else {
                break;
            };
            if static_expression_matches(next, &current) {
                break;
            }
            current = next.clone();
        }
        current
    }

    fn static_eval_source_from_expression(&self, expression: &Expression) -> Option<String> {
        match self.resolve_static_eval_discovery_alias(expression) {
            Expression::String(source) => Some(source.clone()),
            Expression::Identifier(name) => self
                .state
                .global_semantics
                .values
                .value_binding(&name)
                .and_then(|value| match value {
                    Expression::String(source) => Some(source.clone()),
                    _ => None,
                }),
            Expression::Binary {
                op: BinaryOp::Add,
                left,
                right,
            } => Some(format!(
                "{}{}",
                self.static_eval_source_from_expression(&left)?,
                self.static_eval_source_from_expression(&right)?
            )),
            _ => None,
        }
    }

    fn static_eval_source_from_argument(&self, argument: Option<&CallArgument>) -> Option<String> {
        let Some(CallArgument::Expression(expression)) = argument else {
            return None;
        };
        self.static_eval_source_from_expression(expression)
    }

    fn expression_is_test262_eval_script_callee(expression: &Expression) -> bool {
        matches!(
            expression,
            Expression::Member { object, property }
                if matches!(object.as_ref(), Expression::Identifier(name) if name == "$262")
                    && matches!(property.as_ref(), Expression::String(name) if name == "evalScript")
        )
    }

    fn expression_callee_may_be_static_eval(
        &self,
        callee: &Expression,
        current_function_name: Option<&str>,
    ) -> bool {
        match callee {
            Expression::Identifier(name) if name == "eval" => true,
            Expression::Identifier(name)
                if name.contains("eval")
                    && current_function_name
                        .and_then(|function_name| self.registered_function(function_name))
                        .is_some_and(|function| {
                            function
                                .params
                                .iter()
                                .any(|parameter| parameter.name == *name)
                        }) =>
            {
                true
            }
            Expression::Sequence(expressions) => matches!(
                expressions.last(),
                Some(Expression::Identifier(name)) if name == "eval"
            ),
            _ => false,
        }
    }

    fn static_eval_realm_builtin_from_expression(&self, expression: &Expression) -> Option<String> {
        match self.resolve_static_eval_discovery_alias(expression) {
            Expression::Identifier(name) if parse_test262_realm_eval_builtin(&name).is_some() => {
                Some(name)
            }
            Expression::Member { object, property } if matches!(property.as_ref(), Expression::String(property_name) if property_name == "eval") => {
                match object.as_ref() {
                    Expression::Identifier(global_name) => self
                        .state
                        .global_semantics
                        .values
                        .value_binding(global_name)
                        .and_then(|value| match value {
                            Expression::Identifier(realm_global_name) => {
                                parse_test262_realm_global_identifier(realm_global_name)
                            }
                            _ => None,
                        })
                        .or_else(|| parse_test262_realm_global_identifier(global_name))
                        .map(test262_realm_eval_builtin_name),
                    Expression::Member { object, property } if matches!(property.as_ref(), Expression::String(property_name) if property_name == "global") => {
                        match self.resolve_static_eval_discovery_alias(object) {
                            Expression::Identifier(realm_name) => {
                                parse_test262_realm_identifier(&realm_name)
                                    .map(test262_realm_eval_builtin_name)
                            }
                            _ => None,
                        }
                    }
                    _ => None,
                }
            }
            _ => None,
        }
    }

    fn static_eval_realm_builtin_from_argument(
        &self,
        argument: Option<&CallArgument>,
    ) -> Option<String> {
        let Some(CallArgument::Expression(expression)) = argument else {
            return None;
        };
        self.static_eval_realm_builtin_from_expression(expression)
    }

    fn static_eval_registered_callee_name(&self, callee: &Expression) -> Option<String> {
        match self.resolve_static_eval_discovery_alias(callee) {
            Expression::Identifier(name) if self.registered_function(&name).is_some() => Some(name),
            _ => None,
        }
    }

    fn register_realm_static_eval_program(
        &mut self,
        source: &str,
        eval_builtin_name: &str,
    ) -> DirectResult<()> {
        let Ok(eval_program) = frontend::parse_script_goal(source) else {
            return Ok(());
        };
        let mut eval_program = lower_static_eval_function_constructors(eval_program);
        namespace_eval_program_internal_function_names(
            &mut eval_program,
            Some(eval_builtin_name),
            source,
        );
        let new_functions = eval_program
            .functions
            .iter()
            .filter(|function| !self.contains_user_function(&function.name))
            .cloned()
            .collect::<Vec<_>>();
        if !new_functions.is_empty() {
            self.register_functions(&new_functions)?;
        }
        self.register_local_class_member_bindings(&eval_program.functions);
        self.register_static_eval_functions(&eval_program)
    }

    fn register_realm_static_eval_functions_for_bound_parameter_in_expression(
        &mut self,
        expression: &Expression,
        parameter_name: &str,
        eval_builtin_name: &str,
    ) -> DirectResult<()> {
        match expression {
            Expression::Call { callee, arguments } => {
                if matches!(callee.as_ref(), Expression::Identifier(name) if name == parameter_name)
                    && let Some(source) = self.static_eval_source_from_argument(arguments.first())
                {
                    self.register_realm_static_eval_program(&source, eval_builtin_name)?;
                }
                self.register_realm_static_eval_functions_for_bound_parameter_in_expression(
                    callee,
                    parameter_name,
                    eval_builtin_name,
                )?;
                for argument in arguments {
                    match argument {
                        CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                            self.register_realm_static_eval_functions_for_bound_parameter_in_expression(
                                expression,
                                parameter_name,
                                eval_builtin_name,
                            )?;
                        }
                    }
                }
            }
            Expression::Array(elements) => {
                for element in elements {
                    match element {
                        ArrayElement::Expression(expression) | ArrayElement::Spread(expression) => {
                            self.register_realm_static_eval_functions_for_bound_parameter_in_expression(
                                expression,
                                parameter_name,
                                eval_builtin_name,
                            )?;
                        }
                    }
                }
            }
            Expression::Object(entries) => {
                for entry in entries {
                    match entry {
                        ObjectEntry::Data { key, value } => {
                            self.register_realm_static_eval_functions_for_bound_parameter_in_expression(
                                key,
                                parameter_name,
                                eval_builtin_name,
                            )?;
                            self.register_realm_static_eval_functions_for_bound_parameter_in_expression(
                                value,
                                parameter_name,
                                eval_builtin_name,
                            )?;
                        }
                        ObjectEntry::Getter { key, getter } => {
                            self.register_realm_static_eval_functions_for_bound_parameter_in_expression(
                                key,
                                parameter_name,
                                eval_builtin_name,
                            )?;
                            self.register_realm_static_eval_functions_for_bound_parameter_in_expression(
                                getter,
                                parameter_name,
                                eval_builtin_name,
                            )?;
                        }
                        ObjectEntry::Setter { key, setter } => {
                            self.register_realm_static_eval_functions_for_bound_parameter_in_expression(
                                key,
                                parameter_name,
                                eval_builtin_name,
                            )?;
                            self.register_realm_static_eval_functions_for_bound_parameter_in_expression(
                                setter,
                                parameter_name,
                                eval_builtin_name,
                            )?;
                        }
                        ObjectEntry::Spread(expression) => {
                            self.register_realm_static_eval_functions_for_bound_parameter_in_expression(
                                expression,
                                parameter_name,
                                eval_builtin_name,
                            )?;
                        }
                    }
                }
            }
            Expression::Member { object, property }
            | Expression::Binary {
                left: object,
                right: property,
                ..
            } => {
                self.register_realm_static_eval_functions_for_bound_parameter_in_expression(
                    object,
                    parameter_name,
                    eval_builtin_name,
                )?;
                self.register_realm_static_eval_functions_for_bound_parameter_in_expression(
                    property,
                    parameter_name,
                    eval_builtin_name,
                )?;
            }
            Expression::SuperMember { property } => {
                self.register_realm_static_eval_functions_for_bound_parameter_in_expression(
                    property,
                    parameter_name,
                    eval_builtin_name,
                )?;
            }
            Expression::Assign { value, .. }
            | Expression::AssignMember { value, .. }
            | Expression::AssignSuperMember { value, .. }
            | Expression::Await(value)
            | Expression::EnumerateKeys(value)
            | Expression::GetIterator(value)
            | Expression::IteratorClose(value)
            | Expression::Unary {
                expression: value, ..
            } => {
                self.register_realm_static_eval_functions_for_bound_parameter_in_expression(
                    value,
                    parameter_name,
                    eval_builtin_name,
                )?;
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                self.register_realm_static_eval_functions_for_bound_parameter_in_expression(
                    condition,
                    parameter_name,
                    eval_builtin_name,
                )?;
                self.register_realm_static_eval_functions_for_bound_parameter_in_expression(
                    then_expression,
                    parameter_name,
                    eval_builtin_name,
                )?;
                self.register_realm_static_eval_functions_for_bound_parameter_in_expression(
                    else_expression,
                    parameter_name,
                    eval_builtin_name,
                )?;
            }
            Expression::Sequence(expressions) => {
                for expression in expressions {
                    self.register_realm_static_eval_functions_for_bound_parameter_in_expression(
                        expression,
                        parameter_name,
                        eval_builtin_name,
                    )?;
                }
            }
            Expression::SuperCall { callee, arguments } | Expression::New { callee, arguments } => {
                self.register_realm_static_eval_functions_for_bound_parameter_in_expression(
                    callee,
                    parameter_name,
                    eval_builtin_name,
                )?;
                for argument in arguments {
                    match argument {
                        CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                            self.register_realm_static_eval_functions_for_bound_parameter_in_expression(
                                expression,
                                parameter_name,
                                eval_builtin_name,
                            )?;
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
            | Expression::NewTarget
            | Expression::Identifier(_)
            | Expression::This
            | Expression::Sent
            | Expression::Update { .. } => {}
        }
        Ok(())
    }

    fn register_realm_static_eval_functions_for_bound_parameter_in_statements(
        &mut self,
        statements: &[Statement],
        parameter_name: &str,
        eval_builtin_name: &str,
    ) -> DirectResult<()> {
        for statement in statements {
            match statement {
                Statement::Declaration { body }
                | Statement::Block { body }
                | Statement::Labeled { body, .. }
                | Statement::With { body, .. } => {
                    self.register_realm_static_eval_functions_for_bound_parameter_in_statements(
                        body,
                        parameter_name,
                        eval_builtin_name,
                    )?;
                }
                Statement::Var { value, .. }
                | Statement::Let { value, .. }
                | Statement::Assign { value, .. }
                | Statement::Throw(value)
                | Statement::Return(value)
                | Statement::Yield { value }
                | Statement::YieldDelegate { value }
                | Statement::Expression(value) => {
                    self.register_realm_static_eval_functions_for_bound_parameter_in_expression(
                        value,
                        parameter_name,
                        eval_builtin_name,
                    )?;
                }
                Statement::AssignMember {
                    object,
                    property,
                    value,
                } => {
                    self.register_realm_static_eval_functions_for_bound_parameter_in_expression(
                        object,
                        parameter_name,
                        eval_builtin_name,
                    )?;
                    self.register_realm_static_eval_functions_for_bound_parameter_in_expression(
                        property,
                        parameter_name,
                        eval_builtin_name,
                    )?;
                    self.register_realm_static_eval_functions_for_bound_parameter_in_expression(
                        value,
                        parameter_name,
                        eval_builtin_name,
                    )?;
                }
                Statement::Print { values } => {
                    for value in values {
                        self.register_realm_static_eval_functions_for_bound_parameter_in_expression(
                            value,
                            parameter_name,
                            eval_builtin_name,
                        )?;
                    }
                }
                Statement::If {
                    condition,
                    then_branch,
                    else_branch,
                } => {
                    self.register_realm_static_eval_functions_for_bound_parameter_in_expression(
                        condition,
                        parameter_name,
                        eval_builtin_name,
                    )?;
                    self.register_realm_static_eval_functions_for_bound_parameter_in_statements(
                        then_branch,
                        parameter_name,
                        eval_builtin_name,
                    )?;
                    self.register_realm_static_eval_functions_for_bound_parameter_in_statements(
                        else_branch,
                        parameter_name,
                        eval_builtin_name,
                    )?;
                }
                Statement::Try {
                    body,
                    catch_setup,
                    catch_body,
                    ..
                } => {
                    self.register_realm_static_eval_functions_for_bound_parameter_in_statements(
                        body,
                        parameter_name,
                        eval_builtin_name,
                    )?;
                    self.register_realm_static_eval_functions_for_bound_parameter_in_statements(
                        catch_setup,
                        parameter_name,
                        eval_builtin_name,
                    )?;
                    self.register_realm_static_eval_functions_for_bound_parameter_in_statements(
                        catch_body,
                        parameter_name,
                        eval_builtin_name,
                    )?;
                }
                Statement::Switch {
                    discriminant,
                    cases,
                    ..
                } => {
                    self.register_realm_static_eval_functions_for_bound_parameter_in_expression(
                        discriminant,
                        parameter_name,
                        eval_builtin_name,
                    )?;
                    for case in cases {
                        if let Some(test) = &case.test {
                            self.register_realm_static_eval_functions_for_bound_parameter_in_expression(
                                test,
                                parameter_name,
                                eval_builtin_name,
                            )?;
                        }
                        self.register_realm_static_eval_functions_for_bound_parameter_in_statements(
                            &case.body,
                            parameter_name,
                            eval_builtin_name,
                        )?;
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
                    self.register_realm_static_eval_functions_for_bound_parameter_in_statements(
                        init,
                        parameter_name,
                        eval_builtin_name,
                    )?;
                    if let Some(condition) = condition {
                        self.register_realm_static_eval_functions_for_bound_parameter_in_expression(
                            condition,
                            parameter_name,
                            eval_builtin_name,
                        )?;
                    }
                    if let Some(update) = update {
                        self.register_realm_static_eval_functions_for_bound_parameter_in_expression(
                            update,
                            parameter_name,
                            eval_builtin_name,
                        )?;
                    }
                    if let Some(break_hook) = break_hook {
                        self.register_realm_static_eval_functions_for_bound_parameter_in_expression(
                            break_hook,
                            parameter_name,
                            eval_builtin_name,
                        )?;
                    }
                    self.register_realm_static_eval_functions_for_bound_parameter_in_statements(
                        body,
                        parameter_name,
                        eval_builtin_name,
                    )?;
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
                    self.register_realm_static_eval_functions_for_bound_parameter_in_expression(
                        condition,
                        parameter_name,
                        eval_builtin_name,
                    )?;
                    if let Some(break_hook) = break_hook {
                        self.register_realm_static_eval_functions_for_bound_parameter_in_expression(
                            break_hook,
                            parameter_name,
                            eval_builtin_name,
                        )?;
                    }
                    self.register_realm_static_eval_functions_for_bound_parameter_in_statements(
                        body,
                        parameter_name,
                        eval_builtin_name,
                    )?;
                }
                Statement::Break { .. } | Statement::Continue { .. } => {}
            }
        }
        Ok(())
    }

    fn register_realm_static_eval_functions_for_call_arguments(
        &mut self,
        callee: &Expression,
        arguments: &[CallArgument],
    ) -> DirectResult<()> {
        let Some(function_name) = self.static_eval_registered_callee_name(callee) else {
            return Ok(());
        };
        let Some(function) = self.registered_function(&function_name).cloned() else {
            return Ok(());
        };
        for (parameter, argument) in function.params.iter().zip(arguments) {
            let Some(eval_builtin_name) =
                self.static_eval_realm_builtin_from_argument(Some(argument))
            else {
                continue;
            };
            self.register_realm_static_eval_functions_for_bound_parameter_in_statements(
                &function.body,
                &parameter.name,
                &eval_builtin_name,
            )?;
        }
        Ok(())
    }

    pub(in crate::backend::direct_wasm) fn register_static_eval_functions_in_expression(
        &mut self,
        expression: &Expression,
        current_function_name: Option<&str>,
    ) -> DirectResult<()> {
        match expression {
            Expression::Call { callee, arguments } => {
                self.register_realm_static_eval_functions_for_call_arguments(callee, arguments)?;
                if let Some(source) = self.static_eval_source_from_argument(arguments.first()) {
                    if let Some(eval_builtin_name) =
                        self.static_eval_realm_builtin_from_expression(callee)
                    {
                        self.register_realm_static_eval_program(&source, &eval_builtin_name)?;
                    } else if Self::expression_is_test262_eval_script_callee(callee)
                        && let Ok(eval_program) = frontend::parse_script_goal(&source)
                    {
                        let mut eval_program =
                            lower_static_eval_function_constructors(eval_program);
                        namespace_eval_program_internal_function_names(
                            &mut eval_program,
                            current_function_name,
                            &source,
                        );
                        if !eval_program.strict {
                            for name in collect_eval_var_names(&eval_program) {
                                if self.global_has_binding(&name) {
                                    continue;
                                }
                                self.ensure_implicit_global_binding(&name);
                            }
                        }
                        let new_functions = eval_program
                            .functions
                            .iter()
                            .filter(|function| !self.contains_user_function(&function.name))
                            .cloned()
                            .collect::<Vec<_>>();
                        if !new_functions.is_empty() {
                            self.register_functions(&new_functions)?;
                        }
                        let global_functions = eval_program
                            .functions
                            .iter()
                            .filter(|function| function.register_global)
                            .cloned()
                            .collect::<Vec<_>>();
                        for function in &global_functions {
                            self.ensure_implicit_global_binding(&function.name);
                            self.set_global_user_function_reference(&function.name);
                        }
                        self.register_local_class_member_bindings(&eval_program.functions);
                        self.register_static_eval_functions(&eval_program)?;
                    } else if matches!(callee.as_ref(), Expression::Identifier(name) if name == "eval")
                        && let Some(eval_program) = self
                            .parse_static_eval_program_in_context(&source, current_function_name)
                    {
                        let mut eval_program =
                            lower_static_eval_function_constructors(eval_program);
                        namespace_eval_program_internal_function_names(
                            &mut eval_program,
                            current_function_name,
                            &source,
                        );
                        self.register_eval_local_function_bindings(
                            current_function_name,
                            &eval_program,
                        );
                        let new_functions = eval_program
                            .functions
                            .iter()
                            .filter(|function| !self.contains_user_function(&function.name))
                            .cloned()
                            .collect::<Vec<_>>();
                        if !new_functions.is_empty() {
                            self.register_functions(&new_functions)?;
                        }
                        let global_functions = eval_program
                            .functions
                            .iter()
                            .filter(|function| function.register_global)
                            .cloned()
                            .collect::<Vec<_>>();
                        if !global_functions.is_empty() {
                            self.register_functions(&global_functions)?;
                            for function in &global_functions {
                                self.ensure_implicit_global_binding(&function.name);
                                self.set_global_user_function_reference(&function.name);
                            }
                        }
                        self.register_local_class_member_bindings(&eval_program.functions);
                        self.register_static_eval_functions(&eval_program)?;
                    } else if matches!(
                        callee.as_ref(),
                        Expression::Sequence(expressions)
                            if matches!(expressions.last(), Some(Expression::Identifier(name)) if name == "eval")
                    ) && let Ok(eval_program) = frontend::parse(&source)
                    {
                        let mut eval_program =
                            lower_static_eval_function_constructors(eval_program);
                        namespace_eval_program_internal_function_names(
                            &mut eval_program,
                            current_function_name,
                            &source,
                        );
                        if !eval_program.strict {
                            for name in collect_eval_var_names(&eval_program) {
                                if self.global_has_binding(&name) {
                                    continue;
                                }
                                self.ensure_implicit_global_binding(&name);
                            }
                        }
                        let new_functions = eval_program
                            .functions
                            .iter()
                            .filter(|function| !self.contains_user_function(&function.name))
                            .cloned()
                            .collect::<Vec<_>>();
                        if !new_functions.is_empty() {
                            self.register_functions(&new_functions)?;
                        }
                        self.register_local_class_member_bindings(&eval_program.functions);
                        self.register_static_eval_functions(&eval_program)?;
                    } else if self
                        .expression_callee_may_be_static_eval(callee, current_function_name)
                        && let Ok(eval_program) = frontend::parse(&source)
                    {
                        let mut eval_program =
                            lower_static_eval_function_constructors(eval_program);
                        namespace_eval_program_internal_function_names(
                            &mut eval_program,
                            current_function_name,
                            &source,
                        );
                        let new_functions = eval_program
                            .functions
                            .iter()
                            .filter(|function| !self.contains_user_function(&function.name))
                            .cloned()
                            .collect::<Vec<_>>();
                        if !new_functions.is_empty() {
                            self.register_functions(&new_functions)?;
                        }
                        self.register_local_class_member_bindings(&eval_program.functions);
                        self.register_static_eval_functions(&eval_program)?;
                    }
                }
                self.register_static_eval_functions_in_expression(callee, current_function_name)?;
                for argument in arguments {
                    match argument {
                        CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                            self.register_static_eval_functions_in_expression(
                                expression,
                                current_function_name,
                            )?;
                        }
                    }
                }
            }
            Expression::Array(elements) => {
                for element in elements {
                    match element {
                        ArrayElement::Expression(expression) | ArrayElement::Spread(expression) => {
                            self.register_static_eval_functions_in_expression(
                                expression,
                                current_function_name,
                            )?;
                        }
                    }
                }
            }
            Expression::Object(entries) => {
                for entry in entries {
                    match entry {
                        ObjectEntry::Data { key, value } => {
                            self.register_static_eval_functions_in_expression(
                                key,
                                current_function_name,
                            )?;
                            self.register_static_eval_functions_in_expression(
                                value,
                                current_function_name,
                            )?;
                        }
                        ObjectEntry::Getter { key, getter } => {
                            self.register_static_eval_functions_in_expression(
                                key,
                                current_function_name,
                            )?;
                            self.register_static_eval_functions_in_expression(
                                getter,
                                current_function_name,
                            )?;
                        }
                        ObjectEntry::Setter { key, setter } => {
                            self.register_static_eval_functions_in_expression(
                                key,
                                current_function_name,
                            )?;
                            self.register_static_eval_functions_in_expression(
                                setter,
                                current_function_name,
                            )?;
                        }
                        ObjectEntry::Spread(expression) => {
                            self.register_static_eval_functions_in_expression(
                                expression,
                                current_function_name,
                            )?;
                        }
                    }
                }
            }
            Expression::Member { object, property } => {
                self.register_static_eval_functions_in_expression(object, current_function_name)?;
                self.register_static_eval_functions_in_expression(property, current_function_name)?;
            }
            Expression::Assign { value, .. }
            | Expression::Await(value)
            | Expression::EnumerateKeys(value)
            | Expression::GetIterator(value)
            | Expression::IteratorClose(value)
            | Expression::Unary {
                expression: value, ..
            } => {
                self.register_static_eval_functions_in_expression(value, current_function_name)?;
            }
            Expression::AssignMember {
                object,
                property,
                value,
            } => {
                self.register_static_eval_functions_in_expression(object, current_function_name)?;
                self.register_static_eval_functions_in_expression(property, current_function_name)?;
                self.register_static_eval_functions_in_expression(value, current_function_name)?;
            }
            Expression::AssignSuperMember { property, value } => {
                self.register_static_eval_functions_in_expression(property, current_function_name)?;
                self.register_static_eval_functions_in_expression(value, current_function_name)?;
            }
            Expression::Binary { left, right, .. } => {
                self.register_static_eval_functions_in_expression(left, current_function_name)?;
                self.register_static_eval_functions_in_expression(right, current_function_name)?;
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                self.register_static_eval_functions_in_expression(
                    condition,
                    current_function_name,
                )?;
                self.register_static_eval_functions_in_expression(
                    then_expression,
                    current_function_name,
                )?;
                self.register_static_eval_functions_in_expression(
                    else_expression,
                    current_function_name,
                )?;
            }
            Expression::Sequence(expressions) => {
                for expression in expressions {
                    self.register_static_eval_functions_in_expression(
                        expression,
                        current_function_name,
                    )?;
                }
            }
            Expression::SuperCall { callee, arguments } | Expression::New { callee, arguments } => {
                self.register_static_eval_functions_in_expression(callee, current_function_name)?;
                for argument in arguments {
                    match argument {
                        CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                            self.register_static_eval_functions_in_expression(
                                expression,
                                current_function_name,
                            )?;
                        }
                    }
                }
            }
            Expression::SuperMember { property } => {
                self.register_static_eval_functions_in_expression(property, current_function_name)?;
            }
            Expression::Number(_)
            | Expression::BigInt(_)
            | Expression::String(_)
            | Expression::Bool(_)
            | Expression::Null
            | Expression::Undefined
            | Expression::Identifier(_)
            | Expression::Update { .. }
            | Expression::NewTarget
            | Expression::This
            | Expression::Sent => {}
        }
        Ok(())
    }
}
