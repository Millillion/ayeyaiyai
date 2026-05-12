use super::*;

impl<'a> FunctionCompiler<'a> {
    fn resolve_constructed_generator_function_source(
        &self,
        expression: &Expression,
    ) -> Option<(Vec<SimpleGeneratorStep>, Vec<Statement>, Expression)> {
        let Expression::Call { callee, arguments } = expression else {
            return None;
        };
        let source_expression =
            self.resolve_static_constructed_function_source_from_expression(callee)?;
        if self.resolve_static_constructed_function_source_constructor_name(&source_expression)
            != Some("GeneratorFunction")
        {
            return None;
        }
        let (parameters, body) =
            self.resolve_static_constructed_function_parts(&source_expression)?;
        self.parse_constructed_generator_function_body(&body, &parameters, arguments)
    }

    fn parse_constructed_generator_function_body(
        &self,
        body: &str,
        parameters: &[String],
        arguments: &[CallArgument],
    ) -> Option<(Vec<SimpleGeneratorStep>, Vec<Statement>, Expression)> {
        let mut steps = Vec::new();
        let mut completion_value = Expression::Undefined;
        let mut saw_return = false;

        for statement_text in body
            .split(';')
            .map(str::trim)
            .filter(|text| !text.is_empty())
        {
            if saw_return {
                return None;
            }
            if let Some(value_text) = statement_text.strip_prefix("yield") {
                let value_text = value_text.trim();
                if value_text.starts_with('*') {
                    return None;
                }
                let value = if value_text.is_empty() {
                    Expression::Undefined
                } else {
                    Self::parse_constructed_function_expression_text(
                        value_text, parameters, arguments,
                    )?
                };
                steps.push(SimpleGeneratorStep {
                    effects: Vec::new(),
                    close_effects: Vec::new(),
                    outcome: SimpleGeneratorStepOutcome::Yield(value),
                });
                continue;
            }
            if let Some(value_text) = statement_text.strip_prefix("return") {
                completion_value = Self::parse_constructed_function_expression_text(
                    value_text.trim(),
                    parameters,
                    arguments,
                )?;
                saw_return = true;
                continue;
            }
            return None;
        }

        Some((steps, Vec::new(), completion_value))
    }

    pub(in crate::backend::direct_wasm) fn simple_generator_call_time_prefix_effects(
        &self,
        expression: &Expression,
    ) -> Option<Vec<Statement>> {
        let Expression::Call { callee, .. } = expression else {
            return None;
        };
        let Some(LocalFunctionBinding::User(function_name)) =
            self.resolve_function_binding_from_expression(callee)
        else {
            return None;
        };
        if !self
            .user_function(&function_name)
            .is_some_and(|user_function| user_function.is_generator())
        {
            return None;
        }
        let (prefix_effects, _, _, _) = self.resolve_simple_generator_source_parts(expression)?;
        Some(prefix_effects)
    }

    pub(in crate::backend::direct_wasm) fn emit_simple_generator_call_time_prefix_effects(
        &mut self,
        expression: &Expression,
    ) -> DirectResult<()> {
        let Some(prefix_effects) = self.simple_generator_call_time_prefix_effects(expression)
        else {
            return Ok(());
        };
        if prefix_effects.is_empty() {
            return Ok(());
        }
        let prefix_effects =
            self.simple_generator_prefix_effects_before_terminal_throw(&prefix_effects);
        self.register_bindings(prefix_effects)?;
        self.sync_visible_runtime_bindings_for_statements(prefix_effects)?;
        for statement in prefix_effects {
            self.emit_statement(statement)?;
        }
        Ok(())
    }

    fn simple_generator_prefix_effects_before_terminal_throw<'b>(
        &self,
        prefix_effects: &'b [Statement],
    ) -> &'b [Statement] {
        let Some(terminal_index) = prefix_effects.iter().position(|statement| {
            self.simple_generator_prefix_statement_definitely_throws(statement)
        }) else {
            return prefix_effects;
        };
        &prefix_effects[..=terminal_index]
    }

    fn simple_generator_prefix_statement_definitely_throws(&self, statement: &Statement) -> bool {
        match statement {
            Statement::Throw(_) => true,
            Statement::Let { value, .. }
            | Statement::Var { value, .. }
            | Statement::Assign { value, .. }
            | Statement::Expression(value) => {
                self.simple_generator_prefix_expression_definitely_throws(value)
            }
            _ => false,
        }
    }

    fn simple_generator_prefix_expression_definitely_throws(
        &self,
        expression: &Expression,
    ) -> bool {
        if self
            .resolve_terminal_expression_throw_value(expression)
            .is_some()
        {
            return true;
        }
        let Expression::GetIterator(source) = expression else {
            return false;
        };
        let materialized_source = self.materialize_static_expression(source);
        let iterator_target = if !static_expression_matches(&materialized_source, source) {
            &materialized_source
        } else {
            source.as_ref()
        };
        self.array_prototype_symbol_iterator_deleted_affects(iterator_target)
            || self
                .resolve_static_get_iterator_throw_value(iterator_target, &[])
                .is_some()
    }

    pub(in crate::backend::direct_wasm) fn resolve_array_prototype_simple_generator_source(
        &self,
        expression: &Expression,
    ) -> Option<(Vec<SimpleGeneratorStep>, Vec<Statement>, Expression)> {
        let materialized = self.materialize_static_expression(expression);
        if !static_expression_matches(&materialized, expression) {
            return self.resolve_array_prototype_simple_generator_source(&materialized);
        }
        let array_binding = self.resolve_array_binding_from_expression(expression)?;

        let iterator_property = self.materialize_static_expression(&symbol_iterator_expression());
        let array_prototype = Expression::Member {
            object: Box::new(Expression::Identifier("Array".to_string())),
            property: Box::new(Expression::String("prototype".to_string())),
        };
        let LocalFunctionBinding::User(function_name) =
            self.resolve_member_function_binding(&array_prototype, &iterator_property)?
        else {
            return None;
        };
        let user_function = self.user_function(&function_name)?;
        if !user_function.is_generator()
            || !user_function.params.is_empty()
            || user_function.has_parameter_defaults()
            || !user_function.extra_argument_indices.is_empty()
        {
            return None;
        }

        let function = self.resolve_registered_function_declaration(&function_name)?;
        let cache_key = simple_generator_source_cache_key("array-prototype", function, expression);
        if let Some(cached) = lookup_simple_generator_source_cache(&cache_key) {
            return cached
                .map(|(_, steps, effects, completion_value)| (steps, effects, completion_value));
        }
        let _guard = SimpleGeneratorSourceGuard::enter_key(&cache_key)?;
        let mut call_argument_values = user_function
            .params
            .iter()
            .map(|_| Expression::Undefined)
            .collect::<Vec<_>>();
        let mut arguments_values = Vec::new();
        let analysis_this_binding = if self
            .runtime_array_length_local_for_expression(expression)
            .is_some()
            || !matches!(expression, Expression::Array(_))
        {
            Expression::Array(
                array_binding
                    .values
                    .clone()
                    .into_iter()
                    .map(|value| {
                        crate::ir::hir::ArrayElement::Expression(
                            value.unwrap_or(Expression::Undefined),
                        )
                    })
                    .collect(),
            )
        } else {
            expression.clone()
        };
        let substituted_body = self
            .substitute_simple_generator_statements_with_call_frame_bindings(
                &function.body,
                user_function,
                function.mapped_arguments && !function.strict,
                &mut call_argument_values,
                &mut arguments_values,
                &analysis_this_binding,
            )?;
        let (substituted_body, completion_value) =
            self.split_simple_generator_completion(substituted_body)?;
        let mut steps = Vec::new();
        let mut effects = Vec::new();
        let result = self
            .analyze_simple_generator_statements(
                &substituted_body,
                matches!(user_function.kind, FunctionKind::AsyncGenerator),
                &mut steps,
                &mut effects,
            )
            .map(|_| (Vec::new(), steps, effects, completion_value));
        store_simple_generator_source_cache(cache_key, result.clone());
        result.map(|(_, steps, effects, completion_value)| (steps, effects, completion_value))
    }

    fn resolve_simple_generator_source_parts(
        &self,
        expression: &Expression,
    ) -> Option<(
        Vec<Statement>,
        Vec<SimpleGeneratorStep>,
        Vec<Statement>,
        Expression,
    )> {
        let trace_source = std::env::var_os("AYY_TRACE_SIMPLE_GENERATOR_SOURCE").is_some();
        if trace_source {
            eprintln!("simple_generator_source:start expression={expression:?}");
        }
        if let Expression::Call { callee, .. } = expression
            && let Expression::Member { object, property } = callee.as_ref()
            && matches!(
                property.as_ref(),
                Expression::String(name) if matches!(name.as_str(), "next" | "return" | "throw")
            )
            && let Expression::Identifier(iterator_name) = object.as_ref()
            && let Some(binding_name) =
                self.resolve_local_array_iterator_binding_name(iterator_name)
            && self
                .state
                .speculation
                .static_semantics
                .local_array_iterator_binding(&binding_name)
                .is_some()
        {
            return None;
        }
        if let Expression::Call { callee, .. } = expression {
            let is_user_generator_call = matches!(
                self.resolve_function_binding_from_expression(callee),
                Some(LocalFunctionBinding::User(ref function_name))
                    if self
                        .user_function(function_name)
                        .is_some_and(|user_function| user_function.is_generator())
            );
            if !is_user_generator_call {
                return None;
            }
        }
        if let Expression::Call { callee, arguments } = expression
            && let Some(LocalFunctionBinding::User(function_name)) =
                self.resolve_function_binding_from_expression(callee)
            && let Some(user_function) = self.user_function(&function_name)
        {
            if trace_source {
                eprintln!(
                    "simple_generator_source:user function={function_name} kind={:?}",
                    user_function.kind
                );
            }
            if !user_function.is_generator() {
                if trace_source {
                    eprintln!("simple_generator_source:not-generator function={function_name}");
                }
                return None;
            }
            let function = self.resolve_registered_function_declaration(&function_name)?;
            let cache_key = simple_generator_source_cache_key("call", function, expression);
            if let Some(cached) = lookup_simple_generator_source_cache(&cache_key) {
                return cached;
            }
            let _guard = SimpleGeneratorSourceGuard::enter_key(&cache_key)?;
            let expanded_arguments = self.expand_call_arguments(arguments);
            let mut call_argument_values = expanded_arguments.clone();
            if call_argument_values.len() < user_function.params.len() {
                call_argument_values.resize(user_function.params.len(), Expression::Undefined);
            }
            let mut arguments_values = expanded_arguments;
            let raw_this_binding = self.resolve_generator_call_this_binding(callee);
            let analysis_this_binding =
                if self.should_box_sloppy_function_this(user_function, &raw_this_binding) {
                    Expression::This
                } else {
                    raw_this_binding
                };
            let parameter_default_prefix_effects = self.apply_simple_generator_parameter_defaults(
                user_function,
                &mut call_argument_values,
                &arguments_values,
                &analysis_this_binding,
            )?;
            let (prefix_statements, body_statements) = self
                .partition_simple_generator_call_time_prefix_effects(user_function, &function.body);
            let body_var_scope_bindings =
                self.simple_generator_body_var_scope_bindings(user_function, &body_statements);
            let parameter_scope_bindings = self.simple_generator_captured_parameter_scope_bindings(
                user_function,
                &body_statements,
                &call_argument_values,
                &body_var_scope_bindings,
            );
            let mut prefix_effects = parameter_default_prefix_effects;
            let substituted_prefix = self
                .substitute_simple_generator_statements_with_call_frame_bindings(
                    &prefix_statements,
                    user_function,
                    function.mapped_arguments && !function.strict,
                    &mut call_argument_values,
                    &mut arguments_values,
                    &analysis_this_binding,
                )?;
            prefix_effects.extend(substituted_prefix);
            let substituted_body = self
                .substitute_simple_generator_statements_with_call_frame_bindings(
                    &body_statements,
                    user_function,
                    function.mapped_arguments && !function.strict,
                    &mut call_argument_values,
                    &mut arguments_values,
                    &analysis_this_binding,
                )?;
            let substituted_body = substituted_body
                .iter()
                .map(|statement| {
                    self.substitute_statement_bindings(statement, &body_var_scope_bindings)
                })
                .collect::<Vec<_>>();
            let substituted_body = if parameter_scope_bindings.is_empty() {
                substituted_body
            } else {
                parameter_scope_bindings
                    .iter()
                    .map(|(name, value)| Statement::Let {
                        name: name.clone(),
                        mutable: true,
                        value: value.clone(),
                    })
                    .chain(substituted_body)
                    .collect()
            };

            let substituted_body =
                self.expand_static_lowered_for_of_completion_effects(&substituted_body);
            let (substituted_body, completion_value) =
                self.split_simple_generator_completion(substituted_body)?;
            if trace_source {
                eprintln!(
                    "simple_generator_source:analyze function={function_name} body={substituted_body:#?} completion={completion_value:?}"
                );
            }
            let mut steps = Vec::new();
            let mut effects = Vec::new();
            if self
                .analyze_simple_generator_statements(
                    &substituted_body,
                    matches!(user_function.kind, FunctionKind::AsyncGenerator),
                    &mut steps,
                    &mut effects,
                )
                .is_none()
            {
                if trace_source {
                    eprintln!("simple_generator_source:analyze-failed function={function_name}");
                }
                store_simple_generator_source_cache(cache_key, None);
                return None;
            }
            if trace_source {
                eprintln!(
                    "simple_generator_source:ok function={function_name} steps={} effects={}",
                    steps.len(),
                    effects.len()
                );
            }
            let result = Some((prefix_effects, steps, effects, completion_value));
            store_simple_generator_source_cache(cache_key, result.clone());
            return result;
        }

        let materialized = self.materialize_static_expression(expression);
        if !static_expression_matches(&materialized, expression) {
            return self.resolve_simple_generator_source_parts(&materialized);
        }

        None
    }

    fn simple_generator_body_var_scope_bindings(
        &self,
        user_function: &UserFunction,
        statements: &[Statement],
    ) -> HashMap<String, Expression> {
        let mut names = Vec::new();
        Self::collect_simple_generator_body_var_names(statements, &mut names);
        names
            .into_iter()
            .filter(|name| {
                !name.starts_with("__ayy_") && scoped_binding_source_name(name).is_none()
            })
            .map(|name| {
                let scoped_name =
                    Self::simple_generator_body_scoped_binding_name(&user_function.name, &name);
                (name, Expression::Identifier(scoped_name))
            })
            .collect()
    }

    fn collect_simple_generator_body_var_names(statements: &[Statement], names: &mut Vec<String>) {
        for statement in statements {
            match statement {
                Statement::Var { name, .. } => {
                    if !names.contains(name) {
                        names.push(name.clone());
                    }
                }
                Statement::Block { body }
                | Statement::Declaration { body }
                | Statement::Labeled { body, .. }
                | Statement::With { body, .. }
                | Statement::While { body, .. }
                | Statement::DoWhile { body, .. } => {
                    Self::collect_simple_generator_body_var_names(body, names);
                }
                Statement::If {
                    then_branch,
                    else_branch,
                    ..
                } => {
                    Self::collect_simple_generator_body_var_names(then_branch, names);
                    Self::collect_simple_generator_body_var_names(else_branch, names);
                }
                Statement::Try {
                    body,
                    catch_setup,
                    catch_body,
                    ..
                } => {
                    Self::collect_simple_generator_body_var_names(body, names);
                    Self::collect_simple_generator_body_var_names(catch_setup, names);
                    Self::collect_simple_generator_body_var_names(catch_body, names);
                }
                Statement::Switch { cases, .. } => {
                    for case in cases {
                        Self::collect_simple_generator_body_var_names(&case.body, names);
                    }
                }
                Statement::For { init, body, .. } => {
                    Self::collect_simple_generator_body_var_names(init, names);
                    Self::collect_simple_generator_body_var_names(body, names);
                }
                Statement::Let { .. }
                | Statement::Assign { .. }
                | Statement::AssignMember { .. }
                | Statement::Print { .. }
                | Statement::Expression(_)
                | Statement::Throw(_)
                | Statement::Return(_)
                | Statement::Yield { .. }
                | Statement::YieldDelegate { .. }
                | Statement::Break { .. }
                | Statement::Continue { .. } => {}
            }
        }
    }

    fn simple_generator_body_scoped_binding_name(function_name: &str, source_name: &str) -> String {
        let mut hash = 14_695_981_039_346_656_037_u64;
        for byte in function_name
            .bytes()
            .chain(std::iter::once(b':'))
            .chain(source_name.bytes())
        {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(1_099_511_628_211);
        }
        format!("__ayy_scope${source_name}${hash}")
    }

    fn simple_generator_captured_parameter_scope_bindings(
        &self,
        user_function: &UserFunction,
        statements: &[Statement],
        call_argument_values: &[Expression],
        body_var_scope_bindings: &HashMap<String, Expression>,
    ) -> Vec<(String, Expression)> {
        let referenced = collect_referenced_binding_names_from_statements(statements);
        let mut captured_parameters = HashSet::new();
        for nested_function in self.user_functions() {
            if nested_function.name == user_function.name
                || !referenced.contains(&nested_function.name)
            {
                continue;
            }
            let Some(capture_bindings) = self.user_function_capture_bindings(&nested_function.name)
            else {
                continue;
            };
            captured_parameters.extend(capture_bindings.keys().filter_map(|capture_name| {
                user_function
                    .params
                    .iter()
                    .position(|param| {
                        scoped_binding_source_name(param).unwrap_or(param) == capture_name
                    })
                    .map(|index| (capture_name.clone(), index))
            }));
        }

        let mut captured_parameters = captured_parameters.into_iter().collect::<Vec<_>>();
        captured_parameters.sort_by(|(_, left), (_, right)| left.cmp(right));
        captured_parameters.dedup_by(|left, right| left.0 == right.0);
        captured_parameters
            .into_iter()
            .filter(|(source_name, _)| !body_var_scope_bindings.contains_key(source_name))
            .map(|(source_name, index)| {
                let scoped_name = Self::simple_generator_body_scoped_binding_name(
                    &format!("{}:param", user_function.name),
                    &source_name,
                );
                let value = call_argument_values
                    .get(index)
                    .cloned()
                    .unwrap_or(Expression::Undefined);
                (scoped_name, value)
            })
            .collect()
    }

    fn apply_simple_generator_parameter_defaults(
        &self,
        user_function: &UserFunction,
        call_argument_values: &mut Vec<Expression>,
        arguments_values: &[Expression],
        this_binding: &Expression,
    ) -> Option<Vec<Statement>> {
        let mut prefix_effects = Vec::new();
        if !user_function.has_parameter_defaults() {
            return Some(prefix_effects);
        }

        if call_argument_values.len() < user_function.params.len() {
            call_argument_values.resize(user_function.params.len(), Expression::Undefined);
        }

        let parameter_indices = user_function
            .params
            .iter()
            .enumerate()
            .map(|(index, name)| (name.as_str(), index))
            .collect::<HashMap<_, _>>();

        for (index, default) in user_function.parameter_defaults.iter().enumerate() {
            let current_value = self.materialize_static_expression(&call_argument_values[index]);
            if !self.simple_generator_argument_is_undefined_for_default(
                &call_argument_values[index],
                &current_value,
            ) {
                call_argument_values[index] = current_value;
                continue;
            }

            let Some(default) = default else {
                call_argument_values[index] = Expression::Undefined;
                continue;
            };

            if self.simple_generator_parameter_default_eval_var_conflicts(default, user_function) {
                prefix_effects.push(Self::simple_generator_syntax_error_throw_statement());
                call_argument_values[index] = Expression::Undefined;
                return Some(prefix_effects);
            }

            let mut referenced = HashSet::new();
            collect_referenced_binding_names_from_expression(default, &mut referenced);
            if referenced.iter().any(|name| {
                parameter_indices
                    .get(name.as_str())
                    .is_some_and(|referenced_index| *referenced_index >= index)
            }) {
                prefix_effects.push(Self::simple_generator_reference_error_throw_statement());
                call_argument_values[index] = Expression::Undefined;
                return Some(prefix_effects);
            }

            let call_arguments = self.simple_generator_call_arguments(call_argument_values);
            let arguments_binding =
                self.simple_generator_arguments_binding_expression(arguments_values);
            let mut default_scope_function = user_function.clone();
            default_scope_function.body_declares_arguments_binding = false;
            let substituted_default = self.substitute_user_function_call_frame_bindings(
                default,
                &default_scope_function,
                &call_arguments,
                this_binding,
                &arguments_binding,
            );
            let materialized_default = self.materialize_static_expression(&substituted_default);
            if self.simple_generator_default_creates_sync_generator_iterator(&materialized_default)
            {
                call_argument_values[index] = materialized_default;
                continue;
            }
            if !inline_summary_side_effect_free_expression(&materialized_default) {
                let (mut default_effects, default_value) = self
                    .lower_simple_generator_parameter_default_prefix_effects(
                        &substituted_default,
                    )?;
                prefix_effects.append(&mut default_effects);
                call_argument_values[index] = default_value;
                continue;
            }
            call_argument_values[index] = materialized_default;
        }

        Some(prefix_effects)
    }

    fn simple_generator_parameter_default_eval_var_conflicts(
        &self,
        default: &Expression,
        user_function: &UserFunction,
    ) -> bool {
        let Some(program) = self.simple_generator_direct_eval_program(default) else {
            return false;
        };
        if program.strict {
            return false;
        }

        collect_eval_var_names(&program).into_iter().any(|var_name| {
            user_function.params.iter().any(|param_name| {
                scoped_binding_source_name(param_name).unwrap_or(param_name.as_str()) == var_name
            })
        })
    }

    fn simple_generator_direct_eval_program(&self, expression: &Expression) -> Option<Program> {
        let Expression::Call { callee, arguments } = expression else {
            return None;
        };
        if !matches!(callee.as_ref(), Expression::Identifier(name) if name == "eval") {
            return None;
        }
        let Some(CallArgument::Expression(Expression::String(source))) = arguments.first() else {
            return None;
        };
        frontend::parse_script_goal(source).ok()
    }

    fn simple_generator_syntax_error_throw_statement() -> Statement {
        Statement::Throw(Expression::Call {
            callee: Box::new(Expression::Identifier("SyntaxError".to_string())),
            arguments: Vec::new(),
        })
    }

    fn simple_generator_reference_error_throw_statement() -> Statement {
        Statement::Throw(Expression::Call {
            callee: Box::new(Expression::Identifier("ReferenceError".to_string())),
            arguments: Vec::new(),
        })
    }

    fn simple_generator_default_creates_sync_generator_iterator(
        &self,
        expression: &Expression,
    ) -> bool {
        let Expression::Call { callee, arguments } = expression else {
            return false;
        };
        if !inline_summary_side_effect_free_expression(callee) {
            return false;
        }
        if arguments.iter().any(|argument| {
            matches!(argument, CallArgument::Spread(_))
                || !inline_summary_side_effect_free_expression(argument.expression())
        }) {
            return false;
        }
        let Some(LocalFunctionBinding::User(function_name)) =
            self.resolve_function_binding_from_expression(callee)
        else {
            return false;
        };
        self.user_function(&function_name)
            .is_some_and(|function| matches!(function.kind, FunctionKind::Generator))
    }

    fn lower_simple_generator_parameter_default_prefix_effects(
        &self,
        expression: &Expression,
    ) -> Option<(Vec<Statement>, Expression)> {
        if self
            .resolve_terminal_expression_throw_value(expression)
            .is_some()
        {
            return Some((
                vec![Statement::Expression(expression.clone())],
                Expression::Undefined,
            ));
        }
        match expression {
            Expression::Assign { name, value } => {
                if !inline_summary_side_effect_free_expression(value) {
                    return None;
                }
                let value = value.as_ref().clone();
                Some((
                    vec![Statement::Assign {
                        name: name.clone(),
                        value: value.clone(),
                    }],
                    value,
                ))
            }
            Expression::AssignMember {
                object,
                property,
                value,
            } => {
                if !inline_summary_side_effect_free_expression(object)
                    || !inline_summary_side_effect_free_expression(property)
                    || !inline_summary_side_effect_free_expression(value)
                {
                    return None;
                }
                let value = value.as_ref().clone();
                Some((
                    vec![Statement::AssignMember {
                        object: object.as_ref().clone(),
                        property: property.as_ref().clone(),
                        value: value.clone(),
                    }],
                    value,
                ))
            }
            Expression::Sequence(expressions) => {
                let (last, leading) = expressions.split_last()?;
                let mut effects = Vec::new();
                for expression in leading {
                    if inline_summary_side_effect_free_expression(expression) {
                        continue;
                    }
                    if self
                        .simple_generator_parameter_default_discard_call_effect(expression)
                        .is_some()
                    {
                        effects.push(Statement::Expression(expression.clone()));
                        continue;
                    }
                    let (mut expression_effects, _) =
                        self.lower_simple_generator_parameter_default_prefix_effects(expression)?;
                    effects.append(&mut expression_effects);
                }
                if inline_summary_side_effect_free_expression(last) {
                    return Some((effects, last.clone()));
                }
                let (mut last_effects, value) =
                    self.lower_simple_generator_parameter_default_prefix_effects(last)?;
                effects.append(&mut last_effects);
                Some((effects, value))
            }
            _ => None,
        }
    }

    fn simple_generator_parameter_default_discard_call_effect(
        &self,
        expression: &Expression,
    ) -> Option<()> {
        let Expression::Call { callee, arguments } = expression else {
            return None;
        };
        if !matches!(callee.as_ref(), Expression::Identifier(name) if name == "eval") {
            return None;
        }
        let Some(CallArgument::Expression(source)) = arguments.first() else {
            return None;
        };
        matches!(source, Expression::String(_)).then_some(())
    }

    fn simple_generator_argument_is_undefined_for_default(
        &self,
        original: &Expression,
        materialized: &Expression,
    ) -> bool {
        static_expression_matches(materialized, &Expression::Undefined)
            || matches!(
                materialized,
                Expression::Unary {
                    op: UnaryOp::Void,
                    ..
                }
            )
            || matches!(
                original,
                Expression::Unary {
                    op: UnaryOp::Void,
                    ..
                }
            )
            || matches!(
                materialized,
                Expression::Identifier(name)
                    if name == "undefined" && self.is_unshadowed_builtin_identifier(name)
            )
            || matches!(
                original,
                Expression::Identifier(name)
                    if name == "undefined" && self.is_unshadowed_builtin_identifier(name)
            )
    }

    pub(in crate::backend::direct_wasm) fn resolve_simple_generator_source(
        &self,
        expression: &Expression,
    ) -> Option<(Vec<SimpleGeneratorStep>, Vec<Statement>, Expression)> {
        if let Some(source) = self.resolve_constructed_generator_function_source(expression) {
            return Some(source);
        }
        let Expression::Call { callee, .. } = expression else {
            return None;
        };
        let Some(LocalFunctionBinding::User(function_name)) =
            self.resolve_function_binding_from_expression(callee)
        else {
            return None;
        };
        if !self
            .user_function(&function_name)
            .is_some_and(|user_function| user_function.is_generator())
        {
            return None;
        }
        let (_, steps, effects, completion_value) =
            self.resolve_simple_generator_source_parts(expression)?;
        Some((steps, effects, completion_value))
    }

    pub(in crate::backend::direct_wasm) fn analyze_effectful_iterator_source_call(
        &self,
        expression: &Expression,
    ) -> Option<(String, Expression, Vec<Statement>)> {
        if !matches!(expression, Expression::Call { .. }) {
            let materialized = self.materialize_static_expression(expression);
            if !static_expression_matches(&materialized, expression) {
                return self.analyze_effectful_iterator_source_call(&materialized);
            }
        }

        let Expression::Call { callee, arguments } = expression else {
            return None;
        };
        let LocalFunctionBinding::User(function_name) =
            self.resolve_function_binding_from_expression(callee)?
        else {
            return None;
        };
        let user_function = self.user_function(&function_name)?;
        if user_function.is_async()
            || user_function.is_generator()
            || user_function.has_parameter_defaults()
            || !user_function.extra_argument_indices.is_empty()
        {
            return None;
        }

        let function = self.resolve_registered_function_declaration(&function_name)?;
        let (terminal_statement, effect_statements) = function.body.split_last()?;
        let mut substituted_effects = Vec::new();
        for statement in effect_statements {
            match statement {
                Statement::Assign { name, value } => {
                    let substituted = self.substitute_user_function_argument_bindings(
                        value,
                        user_function,
                        arguments,
                    );
                    if expression_mentions_call_frame_state(&substituted) {
                        return None;
                    }
                    substituted_effects.push(Statement::Assign {
                        name: name.clone(),
                        value: substituted,
                    });
                }
                Statement::Expression(Expression::Update { name, op, prefix }) => {
                    substituted_effects.push(Statement::Expression(Expression::Update {
                        name: name.clone(),
                        op: *op,
                        prefix: *prefix,
                    }));
                }
                Statement::Expression(effect_expression) => {
                    let substituted = self.substitute_user_function_argument_bindings(
                        effect_expression,
                        user_function,
                        arguments,
                    );
                    if expression_mentions_call_frame_state(&substituted) {
                        return None;
                    }
                    substituted_effects.push(Statement::Expression(substituted));
                }
                Statement::Block { body } if body.is_empty() => {}
                _ => return None,
            }
        }

        let Statement::Return(return_value) = terminal_statement else {
            return None;
        };
        let returned_expression =
            self.substitute_user_function_argument_bindings(return_value, user_function, arguments);
        if expression_mentions_call_frame_state(&returned_expression)
            || static_expression_matches(&returned_expression, expression)
        {
            return None;
        }

        Some((function_name, returned_expression, substituted_effects))
    }

    fn partition_simple_generator_call_time_prefix_effects(
        &self,
        user_function: &UserFunction,
        statements: &[Statement],
    ) -> (Vec<Statement>, Vec<Statement>) {
        if !user_function.has_lowered_pattern_parameters() {
            return (Vec::new(), statements.to_vec());
        }

        let mut tracked_names = user_function
            .params
            .iter()
            .filter(|name| name.starts_with("__ayy_param_") || name.starts_with("__ayy_rest_"))
            .cloned()
            .collect::<HashSet<_>>();
        let mut prefix_started = false;
        let mut prefix_effects = Vec::new();
        let mut remaining = Vec::new();

        for (index, statement) in statements.iter().enumerate() {
            let mut referenced_names = HashSet::new();
            collect_referenced_binding_names_from_statement(statement, &mut referenced_names);
            let mut assigned_names = HashSet::new();
            collect_assigned_binding_names_from_statement(statement, &mut assigned_names);

            let touches_tracked = referenced_names
                .iter()
                .chain(assigned_names.iter())
                .any(|name| tracked_names.contains(name));
            let assigns_generated_name =
                assigned_names.iter().any(|name| name.starts_with("__ayy_"));
            let is_destructuring_default_placeholder = prefix_started
                && !touches_tracked
                && !assigns_generated_name
                && Self::is_simple_generator_destructuring_default_placeholder(statement)
                && statements.get(index + 1).is_some_and(|next_statement| {
                    let mut next_referenced_names = HashSet::new();
                    collect_referenced_binding_names_from_statement(
                        next_statement,
                        &mut next_referenced_names,
                    );
                    let mut next_assigned_names = HashSet::new();
                    collect_assigned_binding_names_from_statement(
                        next_statement,
                        &mut next_assigned_names,
                    );
                    next_referenced_names
                        .iter()
                        .chain(next_assigned_names.iter())
                        .any(|name| tracked_names.contains(name))
                });

            if !prefix_started {
                if !touches_tracked {
                    remaining.push(statement.clone());
                    continue;
                }
                prefix_started = true;
            } else if !(touches_tracked
                || assigns_generated_name
                || is_destructuring_default_placeholder)
            {
                remaining.push(statement.clone());
                continue;
            }

            tracked_names.extend(
                assigned_names
                    .into_iter()
                    .filter(|name| name.starts_with("__ayy_")),
            );
            prefix_effects.push(statement.clone());
        }

        (prefix_effects, remaining)
    }

    fn is_simple_generator_destructuring_default_placeholder(statement: &Statement) -> bool {
        matches!(
            statement,
            Statement::Let {
                value: Expression::Undefined,
                ..
            } | Statement::Var {
                value: Expression::Undefined,
                ..
            } | Statement::Assign {
                value: Expression::Undefined,
                ..
            }
        )
    }
}
