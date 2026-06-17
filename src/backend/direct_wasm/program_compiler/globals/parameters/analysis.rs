use super::*;

impl DirectWasmCompiler {
    pub(in crate::backend::direct_wasm) fn collect_user_function_parameter_analysis(
        &self,
        program: &Program,
    ) -> UserFunctionParameterAnalysis {
        let trace_timing = crate::ayy_env_flag!("AYY_TRACE_COMPILE_TIMING");
        let timing_start = trace_timing.then(std::time::Instant::now);
        let mut timing_last = timing_start;
        let mut trace_step = |step: &str| {
            if let Some(previous) = timing_last {
                let now = std::time::Instant::now();
                let total_ms = timing_start
                    .map(|start| now.duration_since(start).as_millis())
                    .unwrap_or(0);
                eprintln!(
                    "parameter_analysis_timing step={step} elapsed_ms={} total_ms={total_ms}",
                    now.duration_since(previous).as_millis()
                );
                timing_last = Some(now);
            }
        };
        let value_bindings = self.collect_user_function_parameter_value_bindings(program);
        trace_step("value_bindings");
        let mut function_bindings_by_function = HashMap::new();
        let mut array_bindings_by_function = HashMap::new();
        let mut object_bindings_by_function = HashMap::new();
        for function in &program.functions {
            function_bindings_by_function.insert(function.name.clone(), HashMap::new());
            array_bindings_by_function.insert(function.name.clone(), HashMap::new());
            object_bindings_by_function.insert(function.name.clone(), HashMap::new());
        }
        trace_step("init_maps");
        for iteration in 0..8 {
            let previous_function_bindings = function_bindings_by_function.clone();
            let previous_array_bindings = array_bindings_by_function.clone();
            let previous_object_bindings = object_bindings_by_function.clone();
            trace_step(&format!("iter{iteration}:clone_previous"));

            let mut top_level_aliases = HashMap::new();
            let (mut top_level_value_bindings, mut top_level_object_state) =
                self.snapshot_top_level_static_state();
            trace_step(&format!("iter{iteration}:snapshot_top_level"));
            let mut plain_scan_ms = 0;
            let mut stateful_scan_ms = 0;
            let mut alias_snapshot_ms = 0;
            let mut state_update_ms = 0;
            for statement in &program.statements {
                let needs_stateful_scan =
                    self.statement_needs_stateful_callback_parameter_analysis(statement);
                let aliases_before_statement = if needs_stateful_scan
                    && Self::statement_may_update_parameter_aliases(statement)
                {
                    let step_start = trace_timing.then(std::time::Instant::now);
                    let snapshot = top_level_aliases.clone();
                    if let Some(step_start) = step_start {
                        alias_snapshot_ms += step_start.elapsed().as_millis();
                    }
                    Some(snapshot)
                } else {
                    None
                };
                let step_start = trace_timing.then(std::time::Instant::now);
                self.collect_parameter_bindings_from_statement(
                    statement,
                    &mut top_level_aliases,
                    &mut function_bindings_by_function,
                    &mut array_bindings_by_function,
                    &mut object_bindings_by_function,
                );
                if let Some(step_start) = step_start {
                    plain_scan_ms += step_start.elapsed().as_millis();
                }
                if needs_stateful_scan {
                    let step_start = trace_timing.then(std::time::Instant::now);
                    self.collect_stateful_callback_bindings_from_statement(
                        statement,
                        aliases_before_statement
                            .as_ref()
                            .unwrap_or(&top_level_aliases),
                        &mut function_bindings_by_function,
                        &mut array_bindings_by_function,
                        &mut object_bindings_by_function,
                        &top_level_value_bindings,
                        &top_level_object_state,
                        true,
                    );
                    if let Some(step_start) = step_start {
                        stateful_scan_ms += step_start.elapsed().as_millis();
                    }
                }
                let step_start = trace_timing.then(std::time::Instant::now);
                self.update_parameter_binding_state_from_statement(
                    statement,
                    &mut top_level_value_bindings,
                    &mut top_level_object_state,
                );
                if let Some(step_start) = step_start {
                    state_update_ms += step_start.elapsed().as_millis();
                }
            }
            if trace_timing {
                eprintln!(
                    "parameter_analysis_top_level_timing iter={iteration} alias_snapshot_ms={alias_snapshot_ms} plain_ms={plain_scan_ms} stateful_ms={stateful_scan_ms} update_ms={state_update_ms}"
                );
            }
            trace_step(&format!("iter{iteration}:top_level"));
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
            trace_step(&format!("iter{iteration}:functions"));
            self.seed_proxy_define_property_handler_parameter_bindings(
                program,
                &mut object_bindings_by_function,
            );
            trace_step(&format!("iter{iteration}:proxy_seed"));

            if function_bindings_by_function == previous_function_bindings
                && array_bindings_by_function == previous_array_bindings
                && object_bindings_by_function == previous_object_bindings
            {
                trace_step(&format!("iter{iteration}:stable"));
                break;
            }
            trace_step(&format!("iter{iteration}:compare_changed"));
        }

        UserFunctionParameterAnalysis {
            function_bindings_by_function,
            value_bindings_by_function: value_bindings,
            array_bindings_by_function,
            object_bindings_by_function,
        }
    }

    fn statement_needs_stateful_callback_parameter_analysis(&self, statement: &Statement) -> bool {
        match statement {
            Statement::Declaration { body }
            | Statement::Block { body }
            | Statement::Labeled { body, .. } => body.iter().any(|statement| {
                self.statement_needs_stateful_callback_parameter_analysis(statement)
            }),
            Statement::Var { value, .. }
            | Statement::Let { value, .. }
            | Statement::Assign { value, .. }
            | Statement::Expression(value)
            | Statement::Throw(value)
            | Statement::Return(value)
            | Statement::Yield { value }
            | Statement::YieldDelegate { value } => {
                self.expression_needs_stateful_callback_parameter_analysis(value)
            }
            Statement::AssignMember {
                object,
                property,
                value,
            } => {
                self.expression_needs_stateful_callback_parameter_analysis(object)
                    || self.expression_needs_stateful_callback_parameter_analysis(property)
                    || self.expression_needs_stateful_callback_parameter_analysis(value)
            }
            Statement::Print { values } => values
                .iter()
                .any(|value| self.expression_needs_stateful_callback_parameter_analysis(value)),
            Statement::With { object, body } => {
                self.expression_needs_stateful_callback_parameter_analysis(object)
                    || body.iter().any(|statement| {
                        self.statement_needs_stateful_callback_parameter_analysis(statement)
                    })
            }
            Statement::If {
                condition,
                then_branch,
                else_branch,
            } => {
                self.expression_needs_stateful_callback_parameter_analysis(condition)
                    || then_branch.iter().any(|statement| {
                        self.statement_needs_stateful_callback_parameter_analysis(statement)
                    })
                    || else_branch.iter().any(|statement| {
                        self.statement_needs_stateful_callback_parameter_analysis(statement)
                    })
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
                .any(|statement| {
                    self.statement_needs_stateful_callback_parameter_analysis(statement)
                }),
            Statement::Switch {
                discriminant,
                cases,
                ..
            } => {
                self.expression_needs_stateful_callback_parameter_analysis(discriminant)
                    || cases.iter().any(|case| {
                        case.test.as_ref().is_some_and(|test| {
                            self.expression_needs_stateful_callback_parameter_analysis(test)
                        }) || case.body.iter().any(|statement| {
                            self.statement_needs_stateful_callback_parameter_analysis(statement)
                        })
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
                init.iter().any(|statement| {
                    self.statement_needs_stateful_callback_parameter_analysis(statement)
                }) || condition.as_ref().is_some_and(|condition| {
                    self.expression_needs_stateful_callback_parameter_analysis(condition)
                }) || update.as_ref().is_some_and(|update| {
                    self.expression_needs_stateful_callback_parameter_analysis(update)
                }) || break_hook.as_ref().is_some_and(|break_hook| {
                    self.expression_needs_stateful_callback_parameter_analysis(break_hook)
                }) || body.iter().any(|statement| {
                    self.statement_needs_stateful_callback_parameter_analysis(statement)
                })
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
                self.expression_needs_stateful_callback_parameter_analysis(condition)
                    || break_hook.as_ref().is_some_and(|break_hook| {
                        self.expression_needs_stateful_callback_parameter_analysis(break_hook)
                    })
                    || body.iter().any(|statement| {
                        self.statement_needs_stateful_callback_parameter_analysis(statement)
                    })
            }
            Statement::Break { .. } | Statement::Continue { .. } => false,
        }
    }

    fn expression_needs_stateful_callback_parameter_analysis(
        &self,
        expression: &Expression,
    ) -> bool {
        match expression {
            Expression::Call { callee, arguments }
            | Expression::New { callee, arguments }
            | Expression::SuperCall { callee, arguments } => {
                self.expression_needs_stateful_callback_parameter_analysis(callee)
                    || self.call_needs_stateful_callback_parameter_analysis(callee, arguments)
                    || arguments.iter().any(|argument| {
                        self.expression_needs_stateful_callback_parameter_analysis(
                            argument.expression(),
                        )
                    })
            }
            Expression::Assign { value, .. }
            | Expression::Unary {
                expression: value, ..
            }
            | Expression::Await(value)
            | Expression::EnumerateKeys(value)
            | Expression::GetIterator(value)
            | Expression::IteratorClose(value) => {
                self.expression_needs_stateful_callback_parameter_analysis(value)
            }
            Expression::Member { object, property }
            | Expression::AssignMember {
                object,
                property,
                value: _,
            } => {
                self.expression_needs_stateful_callback_parameter_analysis(object)
                    || self.expression_needs_stateful_callback_parameter_analysis(property)
                    || matches!(expression, Expression::AssignMember { value, .. }
                        if self.expression_needs_stateful_callback_parameter_analysis(value))
            }
            Expression::SuperMember { property } => {
                self.expression_needs_stateful_callback_parameter_analysis(property)
            }
            Expression::AssignSuperMember { property, value } => {
                self.expression_needs_stateful_callback_parameter_analysis(property)
                    || self.expression_needs_stateful_callback_parameter_analysis(value)
            }
            Expression::Array(elements) => elements.iter().any(|element| {
                let expression = match element {
                    ArrayElement::Expression(expression) | ArrayElement::Spread(expression) => {
                        expression
                    }
                };
                self.expression_needs_stateful_callback_parameter_analysis(expression)
            }),
            Expression::Object(entries) => entries.iter().any(|entry| match entry {
                ObjectEntry::Data { key, value } => {
                    self.expression_needs_stateful_callback_parameter_analysis(key)
                        || self.expression_needs_stateful_callback_parameter_analysis(value)
                }
                ObjectEntry::Getter { key, getter } => {
                    self.expression_needs_stateful_callback_parameter_analysis(key)
                        || self.expression_needs_stateful_callback_parameter_analysis(getter)
                }
                ObjectEntry::Setter { key, setter } => {
                    self.expression_needs_stateful_callback_parameter_analysis(key)
                        || self.expression_needs_stateful_callback_parameter_analysis(setter)
                }
                ObjectEntry::Spread(value) => {
                    self.expression_needs_stateful_callback_parameter_analysis(value)
                }
            }),
            Expression::Binary { left, right, .. } => {
                self.expression_needs_stateful_callback_parameter_analysis(left)
                    || self.expression_needs_stateful_callback_parameter_analysis(right)
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                self.expression_needs_stateful_callback_parameter_analysis(condition)
                    || self.expression_needs_stateful_callback_parameter_analysis(then_expression)
                    || self.expression_needs_stateful_callback_parameter_analysis(else_expression)
            }
            Expression::Sequence(expressions) => expressions.iter().any(|expression| {
                self.expression_needs_stateful_callback_parameter_analysis(expression)
            }),
            Expression::Number(_)
            | Expression::BigInt(_)
            | Expression::String(_)
            | Expression::Bool(_)
            | Expression::Null
            | Expression::Undefined
            | Expression::Identifier(_)
            | Expression::This
            | Expression::Sent
            | Expression::NewTarget
            | Expression::Update { .. } => false,
        }
    }

    fn call_needs_stateful_callback_parameter_analysis(
        &self,
        callee: &Expression,
        arguments: &[CallArgument],
    ) -> bool {
        if matches!(
            callee,
            Expression::Member { property, .. }
                if matches!(property.as_ref(), Expression::String(name) if name == "apply")
        ) {
            return true;
        }

        arguments.iter().any(|argument| {
            Self::argument_may_need_stateful_callback_parameter_analysis(argument.expression())
        })
    }

    fn argument_may_need_stateful_callback_parameter_analysis(argument: &Expression) -> bool {
        match argument {
            Expression::Identifier(_)
            | Expression::This
            | Expression::Sent
            | Expression::NewTarget
            | Expression::Member { .. }
            | Expression::SuperMember { .. }
            | Expression::Call { .. }
            | Expression::New { .. }
            | Expression::SuperCall { .. } => true,
            Expression::Array(elements) => elements.iter().any(|element| match element {
                ArrayElement::Expression(expression) => {
                    Self::argument_may_need_stateful_callback_parameter_analysis(expression)
                }
                ArrayElement::Spread(_) => true,
            }),
            Expression::Object(entries) => entries.iter().any(|entry| match entry {
                ObjectEntry::Data { key, value } => {
                    Self::argument_may_need_stateful_callback_parameter_analysis(key)
                        || Self::argument_may_need_stateful_callback_parameter_analysis(value)
                }
                ObjectEntry::Getter { key, getter } => {
                    Self::argument_may_need_stateful_callback_parameter_analysis(key)
                        || Self::argument_may_need_stateful_callback_parameter_analysis(getter)
                }
                ObjectEntry::Setter { key, setter } => {
                    Self::argument_may_need_stateful_callback_parameter_analysis(key)
                        || Self::argument_may_need_stateful_callback_parameter_analysis(setter)
                }
                ObjectEntry::Spread(_) => true,
            }),
            Expression::Assign { value, .. }
            | Expression::Await(value)
            | Expression::EnumerateKeys(value)
            | Expression::GetIterator(value)
            | Expression::IteratorClose(value) => {
                Self::argument_may_need_stateful_callback_parameter_analysis(value)
            }
            Expression::AssignMember { .. } | Expression::AssignSuperMember { .. } => true,
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                Self::argument_may_need_stateful_callback_parameter_analysis(condition)
                    || Self::argument_may_need_stateful_callback_parameter_analysis(then_expression)
                    || Self::argument_may_need_stateful_callback_parameter_analysis(else_expression)
            }
            Expression::Sequence(expressions) => expressions
                .iter()
                .any(Self::argument_may_need_stateful_callback_parameter_analysis),
            Expression::Binary {
                op: BinaryOp::LogicalAnd | BinaryOp::LogicalOr | BinaryOp::NullishCoalescing,
                left,
                right,
            } => {
                Self::argument_may_need_stateful_callback_parameter_analysis(left)
                    || Self::argument_may_need_stateful_callback_parameter_analysis(right)
            }
            Expression::Unary { .. }
            | Expression::Binary { .. }
            | Expression::Update { .. }
            | Expression::Number(_)
            | Expression::BigInt(_)
            | Expression::String(_)
            | Expression::Bool(_)
            | Expression::Null
            | Expression::Undefined => false,
        }
    }

    fn statement_may_update_parameter_aliases(statement: &Statement) -> bool {
        match statement {
            Statement::Declaration { body }
            | Statement::Block { body }
            | Statement::Labeled { body, .. } => body
                .iter()
                .any(Self::statement_may_update_parameter_aliases),
            Statement::Var { .. } | Statement::Let { .. } | Statement::Assign { .. } => true,
            Statement::Expression(value)
            | Statement::Throw(value)
            | Statement::Return(value)
            | Statement::Yield { value }
            | Statement::YieldDelegate { value } => {
                Self::expression_may_update_parameter_aliases(value)
            }
            Statement::AssignMember {
                object,
                property,
                value,
            } => {
                Self::expression_may_update_parameter_aliases(object)
                    || Self::expression_may_update_parameter_aliases(property)
                    || Self::expression_may_update_parameter_aliases(value)
            }
            Statement::Print { values } => values
                .iter()
                .any(Self::expression_may_update_parameter_aliases),
            Statement::With { object, body } => {
                Self::expression_may_update_parameter_aliases(object)
                    || body
                        .iter()
                        .any(Self::statement_may_update_parameter_aliases)
            }
            Statement::If {
                condition,
                then_branch,
                else_branch,
            } => {
                Self::expression_may_update_parameter_aliases(condition)
                    || then_branch
                        .iter()
                        .any(Self::statement_may_update_parameter_aliases)
                    || else_branch
                        .iter()
                        .any(Self::statement_may_update_parameter_aliases)
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
                .any(Self::statement_may_update_parameter_aliases),
            Statement::Switch {
                discriminant,
                cases,
                ..
            } => {
                Self::expression_may_update_parameter_aliases(discriminant)
                    || cases.iter().any(|case| {
                        case.test
                            .as_ref()
                            .is_some_and(Self::expression_may_update_parameter_aliases)
                            || case
                                .body
                                .iter()
                                .any(Self::statement_may_update_parameter_aliases)
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
                    .any(Self::statement_may_update_parameter_aliases)
                    || condition
                        .as_ref()
                        .is_some_and(Self::expression_may_update_parameter_aliases)
                    || update
                        .as_ref()
                        .is_some_and(Self::expression_may_update_parameter_aliases)
                    || break_hook
                        .as_ref()
                        .is_some_and(Self::expression_may_update_parameter_aliases)
                    || body
                        .iter()
                        .any(Self::statement_may_update_parameter_aliases)
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
                Self::expression_may_update_parameter_aliases(condition)
                    || break_hook
                        .as_ref()
                        .is_some_and(Self::expression_may_update_parameter_aliases)
                    || body
                        .iter()
                        .any(Self::statement_may_update_parameter_aliases)
            }
            Statement::Break { .. } | Statement::Continue { .. } => false,
        }
    }

    fn expression_may_update_parameter_aliases(expression: &Expression) -> bool {
        match expression {
            Expression::Assign { .. } | Expression::Update { .. } => true,
            Expression::AssignMember {
                object,
                property,
                value,
            } => {
                Self::expression_may_update_parameter_aliases(object)
                    || Self::expression_may_update_parameter_aliases(property)
                    || Self::expression_may_update_parameter_aliases(value)
            }
            Expression::AssignSuperMember { property, value } => {
                Self::expression_may_update_parameter_aliases(property)
                    || Self::expression_may_update_parameter_aliases(value)
            }
            Expression::Call { callee, arguments }
            | Expression::New { callee, arguments }
            | Expression::SuperCall { callee, arguments } => {
                Self::expression_may_update_parameter_aliases(callee)
                    || arguments.iter().any(|argument| {
                        Self::expression_may_update_parameter_aliases(argument.expression())
                    })
            }
            Expression::Member { object, property } => {
                Self::expression_may_update_parameter_aliases(object)
                    || Self::expression_may_update_parameter_aliases(property)
            }
            Expression::SuperMember { property }
            | Expression::Unary {
                expression: property,
                ..
            }
            | Expression::Await(property)
            | Expression::EnumerateKeys(property)
            | Expression::GetIterator(property)
            | Expression::IteratorClose(property) => {
                Self::expression_may_update_parameter_aliases(property)
            }
            Expression::Array(elements) => elements.iter().any(|element| {
                let expression = match element {
                    ArrayElement::Expression(expression) | ArrayElement::Spread(expression) => {
                        expression
                    }
                };
                Self::expression_may_update_parameter_aliases(expression)
            }),
            Expression::Object(entries) => entries.iter().any(|entry| match entry {
                ObjectEntry::Data { key, value } => {
                    Self::expression_may_update_parameter_aliases(key)
                        || Self::expression_may_update_parameter_aliases(value)
                }
                ObjectEntry::Getter { key, getter } => {
                    Self::expression_may_update_parameter_aliases(key)
                        || Self::expression_may_update_parameter_aliases(getter)
                }
                ObjectEntry::Setter { key, setter } => {
                    Self::expression_may_update_parameter_aliases(key)
                        || Self::expression_may_update_parameter_aliases(setter)
                }
                ObjectEntry::Spread(value) => Self::expression_may_update_parameter_aliases(value),
            }),
            Expression::Binary { left, right, .. } => {
                Self::expression_may_update_parameter_aliases(left)
                    || Self::expression_may_update_parameter_aliases(right)
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                Self::expression_may_update_parameter_aliases(condition)
                    || Self::expression_may_update_parameter_aliases(then_expression)
                    || Self::expression_may_update_parameter_aliases(else_expression)
            }
            Expression::Sequence(expressions) => expressions
                .iter()
                .any(Self::expression_may_update_parameter_aliases),
            Expression::Number(_)
            | Expression::BigInt(_)
            | Expression::String(_)
            | Expression::Bool(_)
            | Expression::Null
            | Expression::Undefined
            | Expression::Identifier(_)
            | Expression::This
            | Expression::Sent
            | Expression::NewTarget => false,
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
        let trace_timing = crate::ayy_env_flag!("AYY_TRACE_COMPILE_TIMING");
        let timing_start = trace_timing.then(std::time::Instant::now);
        let mut timing_last = timing_start;
        let mut trace_step = |step: &str| {
            if let Some(previous) = timing_last {
                let now = std::time::Instant::now();
                let total_ms = timing_start
                    .map(|start| now.duration_since(start).as_millis())
                    .unwrap_or(0);
                eprintln!(
                    "parameter_value_timing step={step} elapsed_ms={} total_ms={total_ms}",
                    now.duration_since(previous).as_millis()
                );
                timing_last = Some(now);
            }
        };
        super::value_bindings::rest_array_aliases::collect_stable_rest_array_aliases(program);
        trace_step("rest_aliases");
        let mut previous = HashMap::new();
        for function in &program.functions {
            previous.insert(function.name.clone(), HashMap::new());
        }
        trace_step("init_previous");

        for iteration in 0..8 {
            let mut bindings = HashMap::new();
            for function in &program.functions {
                bindings.insert(function.name.clone(), HashMap::new());
            }
            trace_step(&format!("iter{iteration}:init_bindings"));

            let mut top_level_aliases = HashMap::new();
            let mut value_top_level_scan_ms = 0;
            for statement in &program.statements {
                let step_start = trace_timing.then(std::time::Instant::now);
                self.collect_parameter_value_bindings_from_statement_in_function(
                    statement,
                    &mut top_level_aliases,
                    &mut bindings,
                    &previous,
                    None,
                );
                if let Some(step_start) = step_start {
                    value_top_level_scan_ms += step_start.elapsed().as_millis();
                }
            }
            if trace_timing {
                eprintln!(
                    "parameter_value_top_level_timing iter={iteration} scan_ms={value_top_level_scan_ms}"
                );
            }
            trace_step(&format!("iter{iteration}:top_level"));

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
            trace_step(&format!("iter{iteration}:functions"));

            if bindings == previous {
                trace_step(&format!("iter{iteration}:stable"));
                return bindings;
            }
            previous = bindings;
            trace_step(&format!("iter{iteration}:changed"));
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
