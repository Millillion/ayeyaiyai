use super::*;

impl DirectWasmCompiler {
    pub(in crate::backend::direct_wasm) fn collect_user_function_parameter_analysis(
        &self,
        program: &Program,
    ) -> UserFunctionParameterAnalysis {
        let value_bindings = self.collect_user_function_parameter_value_bindings(program);
        let mut function_bindings_by_function = HashMap::new();
        let mut array_bindings_by_function = HashMap::new();
        let mut object_bindings_by_function = HashMap::new();
        for function in &program.functions {
            function_bindings_by_function.insert(function.name.clone(), HashMap::new());
            array_bindings_by_function.insert(function.name.clone(), HashMap::new());
            object_bindings_by_function.insert(function.name.clone(), HashMap::new());
        }
        for _ in 0..8 {
            let previous_function_bindings = function_bindings_by_function.clone();
            let previous_array_bindings = array_bindings_by_function.clone();
            let previous_object_bindings = object_bindings_by_function.clone();

            let mut top_level_aliases = HashMap::new();
            let (mut top_level_value_bindings, mut top_level_object_state) =
                self.snapshot_top_level_static_state();
            for statement in &program.statements {
                let aliases_before_statement = top_level_aliases.clone();
                let value_bindings_before_statement = top_level_value_bindings.clone();
                let object_state_before_statement = top_level_object_state.clone();
                self.collect_parameter_bindings_from_statement(
                    statement,
                    &mut top_level_aliases,
                    &mut function_bindings_by_function,
                    &mut array_bindings_by_function,
                    &mut object_bindings_by_function,
                );
                self.collect_stateful_callback_bindings_from_statement(
                    statement,
                    &aliases_before_statement,
                    &mut function_bindings_by_function,
                    &mut array_bindings_by_function,
                    &mut object_bindings_by_function,
                    &value_bindings_before_statement,
                    &object_state_before_statement,
                    true,
                );
                self.update_parameter_binding_state_from_statement(
                    statement,
                    &mut top_level_value_bindings,
                    &mut top_level_object_state,
                );
            }
            for function in &program.functions {
                let mut aliases = top_level_aliases.clone();
                for parameter in &function.params {
                    aliases.entry(parameter.name.clone()).or_insert(None);
                }
                self.collect_parameter_bindings_from_statements_in_function(
                    &function.body,
                    &mut aliases,
                    &mut function_bindings_by_function,
                    &mut array_bindings_by_function,
                    &mut object_bindings_by_function,
                    Some(&function.name),
                );
            }
            self.seed_proxy_define_property_handler_parameter_bindings(
                program,
                &mut object_bindings_by_function,
            );

            if function_bindings_by_function == previous_function_bindings
                && array_bindings_by_function == previous_array_bindings
                && object_bindings_by_function == previous_object_bindings
            {
                break;
            }
        }

        UserFunctionParameterAnalysis {
            function_bindings_by_function,
            value_bindings_by_function: value_bindings,
            array_bindings_by_function,
            object_bindings_by_function,
        }
    }

    #[cfg(test)]
    pub(in crate::backend::direct_wasm) fn collect_user_function_parameter_bindings(
        &self,
        program: &Program,
    ) -> (
        HashMap<String, HashMap<String, Option<LocalFunctionBinding>>>,
        HashMap<String, HashMap<String, Option<Expression>>>,
        HashMap<String, HashMap<String, Option<ArrayValueBinding>>>,
        HashMap<String, HashMap<String, Option<ObjectValueBinding>>>,
    ) {
        let analysis = self.collect_user_function_parameter_analysis(program);
        (
            analysis.function_bindings_by_function,
            analysis.value_bindings_by_function,
            analysis.array_bindings_by_function,
            analysis.object_bindings_by_function,
        )
    }

    pub(in crate::backend::direct_wasm) fn collect_user_function_parameter_value_bindings(
        &self,
        program: &Program,
    ) -> HashMap<String, HashMap<String, Option<Expression>>> {
        let mut previous = HashMap::new();
        for function in &program.functions {
            previous.insert(function.name.clone(), HashMap::new());
        }

        for _ in 0..8 {
            let mut bindings = HashMap::new();
            for function in &program.functions {
                bindings.insert(function.name.clone(), HashMap::new());
            }

            let mut top_level_aliases = HashMap::new();
            for statement in &program.statements {
                self.collect_parameter_value_bindings_from_statement_in_function(
                    statement,
                    &mut top_level_aliases,
                    &mut bindings,
                    &previous,
                    None,
                );
            }

            for function in &program.functions {
                let mut aliases = top_level_aliases.clone();
                for parameter in &function.params {
                    aliases.entry(parameter.name.clone()).or_insert(None);
                }
                self.collect_parameter_value_bindings_from_statements_in_function(
                    &function.body,
                    &mut aliases,
                    &mut bindings,
                    &previous,
                    Some(&function.name),
                );
            }

            if bindings == previous {
                return bindings;
            }
            previous = bindings;
        }

        previous
    }

    fn seed_proxy_define_property_handler_parameter_bindings(
        &self,
        program: &Program,
        object_bindings_by_function: &mut HashMap<
            String,
            HashMap<String, Option<ObjectValueBinding>>,
        >,
    ) {
        for statement in &program.statements {
            self.seed_proxy_define_property_handler_bindings_from_statement(
                statement,
                object_bindings_by_function,
            );
        }
        for function in &program.functions {
            for statement in &function.body {
                self.seed_proxy_define_property_handler_bindings_from_statement(
                    statement,
                    object_bindings_by_function,
                );
            }
        }
    }

    fn seed_proxy_define_property_handler_bindings_from_statement(
        &self,
        statement: &Statement,
        object_bindings_by_function: &mut HashMap<
            String,
            HashMap<String, Option<ObjectValueBinding>>,
        >,
    ) {
        match statement {
            Statement::Expression(expression)
            | Statement::Return(expression)
            | Statement::Throw(expression)
            | Statement::Yield { value: expression }
            | Statement::YieldDelegate { value: expression } => self
                .seed_proxy_define_property_handler_bindings_from_expression(
                    expression,
                    object_bindings_by_function,
                ),
            Statement::Var { value, .. }
            | Statement::Let { value, .. }
            | Statement::Assign { value, .. } => self
                .seed_proxy_define_property_handler_bindings_from_expression(
                    value,
                    object_bindings_by_function,
                ),
            Statement::AssignMember {
                object,
                property,
                value,
            } => {
                self.seed_proxy_define_property_handler_bindings_from_expression(
                    object,
                    object_bindings_by_function,
                );
                self.seed_proxy_define_property_handler_bindings_from_expression(
                    property,
                    object_bindings_by_function,
                );
                self.seed_proxy_define_property_handler_bindings_from_expression(
                    value,
                    object_bindings_by_function,
                );
            }
            Statement::Print { values } => {
                for value in values {
                    self.seed_proxy_define_property_handler_bindings_from_expression(
                        value,
                        object_bindings_by_function,
                    );
                }
            }
            Statement::Block { body: statements }
            | Statement::Try {
                body: statements, ..
            }
            | Statement::Labeled {
                body: statements, ..
            } => {
                for statement in statements {
                    self.seed_proxy_define_property_handler_bindings_from_statement(
                        statement,
                        object_bindings_by_function,
                    );
                }
            }
            Statement::If {
                condition,
                then_branch,
                else_branch,
            } => {
                self.seed_proxy_define_property_handler_bindings_from_expression(
                    condition,
                    object_bindings_by_function,
                );
                for statement in then_branch {
                    self.seed_proxy_define_property_handler_bindings_from_statement(
                        statement,
                        object_bindings_by_function,
                    );
                }
                for statement in else_branch {
                    self.seed_proxy_define_property_handler_bindings_from_statement(
                        statement,
                        object_bindings_by_function,
                    );
                }
            }
            Statement::While {
                condition, body, ..
            }
            | Statement::DoWhile {
                condition, body, ..
            } => {
                self.seed_proxy_define_property_handler_bindings_from_expression(
                    condition,
                    object_bindings_by_function,
                );
                for statement in body {
                    self.seed_proxy_define_property_handler_bindings_from_statement(
                        statement,
                        object_bindings_by_function,
                    );
                }
            }
            Statement::For {
                init,
                condition,
                update,
                body,
                ..
            } => {
                for statement in init {
                    self.seed_proxy_define_property_handler_bindings_from_statement(
                        statement,
                        object_bindings_by_function,
                    );
                }
                if let Some(condition) = condition {
                    self.seed_proxy_define_property_handler_bindings_from_expression(
                        condition,
                        object_bindings_by_function,
                    );
                }
                if let Some(update) = update {
                    self.seed_proxy_define_property_handler_bindings_from_expression(
                        update,
                        object_bindings_by_function,
                    );
                }
                for statement in body {
                    self.seed_proxy_define_property_handler_bindings_from_statement(
                        statement,
                        object_bindings_by_function,
                    );
                }
            }
            Statement::Switch {
                discriminant,
                cases,
                ..
            } => {
                self.seed_proxy_define_property_handler_bindings_from_expression(
                    discriminant,
                    object_bindings_by_function,
                );
                for case in cases {
                    if let Some(test) = &case.test {
                        self.seed_proxy_define_property_handler_bindings_from_expression(
                            test,
                            object_bindings_by_function,
                        );
                    }
                    for statement in &case.body {
                        self.seed_proxy_define_property_handler_bindings_from_statement(
                            statement,
                            object_bindings_by_function,
                        );
                    }
                }
            }
            Statement::Declaration { body } => {
                for statement in body {
                    self.seed_proxy_define_property_handler_bindings_from_statement(
                        statement,
                        object_bindings_by_function,
                    );
                }
            }
            Statement::With { object, body } => {
                self.seed_proxy_define_property_handler_bindings_from_expression(
                    object,
                    object_bindings_by_function,
                );
                for statement in body {
                    self.seed_proxy_define_property_handler_bindings_from_statement(
                        statement,
                        object_bindings_by_function,
                    );
                }
            }
            Statement::Break { .. } | Statement::Continue { .. } => {}
        }
    }

    fn seed_proxy_define_property_handler_bindings_from_expression(
        &self,
        expression: &Expression,
        object_bindings_by_function: &mut HashMap<
            String,
            HashMap<String, Option<ObjectValueBinding>>,
        >,
    ) {
        match expression {
            Expression::New { callee, arguments } if matches!(callee.as_ref(), Expression::Identifier(name) if name == "Proxy") =>
            {
                if let [target, handler, ..] = self
                    .expanded_global_static_call_arguments(arguments)
                    .as_slice()
                {
                    self.register_proxy_define_property_handler_bindings(
                        target,
                        handler,
                        object_bindings_by_function,
                    );
                }
                for argument in arguments {
                    self.seed_proxy_define_property_handler_bindings_from_expression(
                        argument.expression(),
                        object_bindings_by_function,
                    );
                }
            }
            Expression::Call { callee, arguments }
            | Expression::SuperCall { callee, arguments } => {
                self.seed_proxy_define_property_handler_bindings_from_expression(
                    callee,
                    object_bindings_by_function,
                );
                for argument in arguments {
                    self.seed_proxy_define_property_handler_bindings_from_expression(
                        argument.expression(),
                        object_bindings_by_function,
                    );
                }
            }
            Expression::New { callee, arguments } => {
                self.seed_proxy_define_property_handler_bindings_from_expression(
                    callee,
                    object_bindings_by_function,
                );
                for argument in arguments {
                    self.seed_proxy_define_property_handler_bindings_from_expression(
                        argument.expression(),
                        object_bindings_by_function,
                    );
                }
            }
            Expression::Member { object, property }
            | Expression::AssignMember {
                object,
                property,
                value: _,
            } => {
                self.seed_proxy_define_property_handler_bindings_from_expression(
                    object,
                    object_bindings_by_function,
                );
                self.seed_proxy_define_property_handler_bindings_from_expression(
                    property,
                    object_bindings_by_function,
                );
                if let Expression::AssignMember { value, .. } = expression {
                    self.seed_proxy_define_property_handler_bindings_from_expression(
                        value,
                        object_bindings_by_function,
                    );
                }
            }
            Expression::Assign { value, .. }
            | Expression::Await(value)
            | Expression::EnumerateKeys(value)
            | Expression::GetIterator(value)
            | Expression::IteratorClose(value)
            | Expression::Unary {
                expression: value, ..
            } => self.seed_proxy_define_property_handler_bindings_from_expression(
                value,
                object_bindings_by_function,
            ),
            Expression::SuperMember { property } => {
                self.seed_proxy_define_property_handler_bindings_from_expression(
                    property,
                    object_bindings_by_function,
                );
            }
            Expression::Binary { left, right, .. } => {
                self.seed_proxy_define_property_handler_bindings_from_expression(
                    left,
                    object_bindings_by_function,
                );
                self.seed_proxy_define_property_handler_bindings_from_expression(
                    right,
                    object_bindings_by_function,
                );
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                self.seed_proxy_define_property_handler_bindings_from_expression(
                    condition,
                    object_bindings_by_function,
                );
                self.seed_proxy_define_property_handler_bindings_from_expression(
                    then_expression,
                    object_bindings_by_function,
                );
                self.seed_proxy_define_property_handler_bindings_from_expression(
                    else_expression,
                    object_bindings_by_function,
                );
            }
            Expression::Sequence(expressions) => {
                for expression in expressions {
                    self.seed_proxy_define_property_handler_bindings_from_expression(
                        expression,
                        object_bindings_by_function,
                    );
                }
            }
            Expression::Array(expressions) => {
                for expression in expressions {
                    let expression = match expression {
                        crate::ir::hir::ArrayElement::Expression(expression)
                        | crate::ir::hir::ArrayElement::Spread(expression) => expression,
                    };
                    self.seed_proxy_define_property_handler_bindings_from_expression(
                        expression,
                        object_bindings_by_function,
                    );
                }
            }
            Expression::Object(entries) => {
                for entry in entries {
                    match entry {
                        ObjectEntry::Data { key, value } => {
                            self.seed_proxy_define_property_handler_bindings_from_expression(
                                key,
                                object_bindings_by_function,
                            );
                            self.seed_proxy_define_property_handler_bindings_from_expression(
                                value,
                                object_bindings_by_function,
                            );
                        }
                        ObjectEntry::Getter { key, getter } => {
                            self.seed_proxy_define_property_handler_bindings_from_expression(
                                key,
                                object_bindings_by_function,
                            );
                            self.seed_proxy_define_property_handler_bindings_from_expression(
                                getter,
                                object_bindings_by_function,
                            );
                        }
                        ObjectEntry::Setter { key, setter } => {
                            self.seed_proxy_define_property_handler_bindings_from_expression(
                                key,
                                object_bindings_by_function,
                            );
                            self.seed_proxy_define_property_handler_bindings_from_expression(
                                setter,
                                object_bindings_by_function,
                            );
                        }
                        ObjectEntry::Spread(expression) => {
                            self.seed_proxy_define_property_handler_bindings_from_expression(
                                expression,
                                object_bindings_by_function,
                            );
                        }
                    }
                }
            }
            Expression::AssignSuperMember { property, value } => {
                self.seed_proxy_define_property_handler_bindings_from_expression(
                    property,
                    object_bindings_by_function,
                );
                self.seed_proxy_define_property_handler_bindings_from_expression(
                    value,
                    object_bindings_by_function,
                );
            }
            Expression::Update { .. }
            | Expression::Number(_)
            | Expression::BigInt(_)
            | Expression::String(_)
            | Expression::Bool(_)
            | Expression::Null
            | Expression::Undefined
            | Expression::NewTarget
            | Expression::Identifier(_)
            | Expression::This
            | Expression::Sent => {}
        }
    }

    fn register_proxy_define_property_handler_bindings(
        &self,
        target: &Expression,
        handler: &Expression,
        object_bindings_by_function: &mut HashMap<
            String,
            HashMap<String, Option<ObjectValueBinding>>,
        >,
    ) {
        let Expression::Object(entries) = handler else {
            return;
        };
        let Some(handler_function_name) = entries.iter().find_map(|entry| match entry {
            ObjectEntry::Data { key, value }
                if matches!(key, Expression::String(name) if name == "defineProperty") =>
            {
                let Expression::Identifier(name) = value else {
                    return None;
                };
                self.user_function(name).map(|_| name.clone())
            }
            _ => None,
        }) else {
            return;
        };
        let Some(user_function) = self.user_function(&handler_function_name) else {
            return;
        };
        let Some(parameter_object_bindings) =
            object_bindings_by_function.get_mut(&handler_function_name)
        else {
            return;
        };

        if let Some(param_name) = user_function.params.first() {
            let target_binding = self
                .infer_global_object_binding(target)
                .unwrap_or_else(empty_object_value_binding);
            Self::merge_parameter_object_binding_candidate(
                parameter_object_bindings,
                param_name,
                Some(target_binding),
            );
        }

        if let Some(param_name) = user_function.params.get(2) {
            Self::merge_parameter_object_binding_candidate(
                parameter_object_bindings,
                param_name,
                Some(Self::proxy_define_property_descriptor_binding()),
            );
        }
    }

    fn merge_parameter_object_binding_candidate(
        parameter_object_bindings: &mut HashMap<String, Option<ObjectValueBinding>>,
        param_name: &str,
        candidate: Option<ObjectValueBinding>,
    ) {
        match candidate {
            None => {
                parameter_object_bindings.insert(param_name.to_string(), None);
            }
            Some(binding) => match parameter_object_bindings.get(param_name) {
                Some(None) => {}
                Some(Some(existing)) if *existing == binding => {}
                Some(Some(_)) => {
                    parameter_object_bindings.insert(param_name.to_string(), None);
                }
                None => {
                    parameter_object_bindings.insert(param_name.to_string(), Some(binding));
                }
            },
        }
    }

    fn proxy_define_property_descriptor_binding() -> ObjectValueBinding {
        let mut binding = empty_object_value_binding();
        for property_name in ["value", "writable", "enumerable", "configurable"] {
            object_binding_set_property(
                &mut binding,
                Expression::String(property_name.to_string()),
                Expression::Undefined,
            );
        }
        binding
    }
}
