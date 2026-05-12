use super::*;

impl<'a> FunctionCompiler<'a> {
    fn eval_internal_function_name_hint(function_name: &str) -> Option<&str> {
        function_name
            .rsplit_once("__name_")
            .map(|(_, hinted_name)| hinted_name)
            .filter(|hinted_name| !hinted_name.is_empty())
    }

    fn class_binding_name_for_function(&self, function_name: &str) -> Option<String> {
        if function_name.starts_with("__ayy_class_ctor_") {
            return self
                .resolve_registered_function_declaration(function_name)
                .and_then(|declaration| declaration.self_binding.clone())
                .or_else(|| {
                    Self::eval_internal_function_name_hint(function_name).map(str::to_string)
                });
        }

        let home_object_name = self.resolve_home_object_name_for_function(function_name)?;
        Some(
            home_object_name
                .strip_suffix(".prototype")
                .unwrap_or(home_object_name.as_str())
                .to_string(),
        )
    }

    fn class_private_names_for_function(
        &self,
        function_name: &str,
    ) -> Option<(String, Vec<String>)> {
        let class_binding_name = self.class_binding_name_for_function(function_name)?;
        let prefix = format!("__ayy$private${class_binding_name}$");
        let mut private_names = self
            .current_function_name()
            .filter(|current| *current == function_name)
            .and_then(|_| self.resolve_object_binding_from_expression(&Expression::This))
            .map(|this_binding| {
                this_binding
                    .string_properties
                    .iter()
                    .filter_map(|(property_name, _)| {
                        property_name.strip_prefix(&prefix).map(str::to_string)
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        Self::collect_private_names_from_identifier_binding(
            self,
            &class_binding_name,
            &prefix,
            &mut private_names,
        );
        let prototype_binding_name = format!("{class_binding_name}.prototype");
        Self::collect_private_names_from_identifier_binding(
            self,
            &prototype_binding_name,
            &prefix,
            &mut private_names,
        );
        for user_function in self.user_functions() {
            let belongs_to_class = user_function.home_object_binding.as_deref()
                == Some(class_binding_name.as_str())
                || user_function.home_object_binding.as_deref()
                    == Some(prototype_binding_name.as_str())
                || self
                    .resolve_registered_function_declaration(&user_function.name)
                    .and_then(|declaration| declaration.self_binding.as_deref())
                    == Some(class_binding_name.as_str());
            if !belongs_to_class {
                continue;
            }
            if let Some(declaration) =
                self.resolve_registered_function_declaration(&user_function.name)
            {
                for statement in &declaration.body {
                    Self::collect_private_names_from_statement(
                        statement,
                        &prefix,
                        &mut private_names,
                    );
                }
            }
        }
        private_names.sort();
        private_names.dedup();
        Some((class_binding_name, private_names))
    }

    fn collect_private_names_from_identifier_binding(
        &self,
        binding_name: &str,
        prefix: &str,
        names: &mut Vec<String>,
    ) {
        let Some(binding) = self.resolve_object_binding_from_expression(&Expression::Identifier(
            binding_name.to_string(),
        )) else {
            return;
        };
        names.extend(
            binding
                .string_properties
                .iter()
                .filter_map(|(property_name, _)| {
                    property_name.strip_prefix(prefix).map(str::to_string)
                }),
        );
    }

    fn collect_private_names_from_statement(
        statement: &Statement,
        prefix: &str,
        names: &mut Vec<String>,
    ) {
        match statement {
            Statement::Declaration { body }
            | Statement::Block { body }
            | Statement::Labeled { body, .. } => {
                for statement in body {
                    Self::collect_private_names_from_statement(statement, prefix, names);
                }
            }
            Statement::AssignMember {
                object, property, ..
            } if matches!(object, Expression::This) => {
                Self::collect_private_name_from_expression(property, prefix, names);
            }
            Statement::Expression(Expression::Call { callee, arguments })
                if matches!(
                    callee.as_ref(),
                    Expression::Member { object, property }
                        if matches!(object.as_ref(), Expression::Identifier(name) if name == "Object")
                            && matches!(property.as_ref(), Expression::String(name) if name == "defineProperty")
                ) =>
            {
                let [
                    CallArgument::Expression(target),
                    CallArgument::Expression(property),
                    ..,
                ] = arguments.as_slice()
                else {
                    return;
                };
                if matches!(target, Expression::This) {
                    Self::collect_private_name_from_expression(property, prefix, names);
                }
            }
            _ => {}
        }
    }

    fn collect_private_name_from_expression(
        expression: &Expression,
        prefix: &str,
        names: &mut Vec<String>,
    ) {
        if let Expression::String(property_name) = expression
            && let Some(name) = property_name.strip_prefix(prefix)
        {
            names.push(name.to_string());
        }
    }

    pub(in crate::backend::direct_wasm) fn parse_eval_program_in_class_field_initializer_context(
        &self,
        source: &str,
    ) -> Option<Program> {
        let current_function_name = self.current_function_name()?;
        self.parse_eval_program_in_class_field_initializer_context_for_function(
            current_function_name,
            source,
        )
    }

    pub(in crate::backend::direct_wasm) fn parse_eval_program_in_class_field_initializer_context_for_function(
        &self,
        function_name: &str,
        source: &str,
    ) -> Option<Program> {
        let (class_binding_name, private_names) =
            self.class_private_names_for_function(function_name)?;
        let wrapper_method_name = "__ayy_eval_wrapper__";
        let private_declarations = private_names
            .into_iter()
            .map(|name| format!("#{name};"))
            .collect::<Vec<_>>()
            .join("\n");
        let wrapped_source = if private_declarations.is_empty() {
            format!("class {class_binding_name} {{\n{wrapper_method_name}() {{\n{source}\n}}\n}}")
        } else {
            format!(
                "class {class_binding_name} {{\n{private_declarations}\n{wrapper_method_name}() {{\n{source}\n}}\n}}"
            )
        };
        let mut wrapped_program = frontend::parse(&wrapped_source).ok()?;
        let wrapper_function = wrapped_program
            .functions
            .iter()
            .find(|function| {
                Self::eval_internal_function_name_hint(&function.name) == Some(wrapper_method_name)
            })
            .cloned()
            .or_else(|| {
                let wrapper_methods = wrapped_program
                    .functions
                    .iter()
                    .filter(|function| function.name.starts_with("__ayy_class_method_"))
                    .cloned()
                    .collect::<Vec<_>>();
                (wrapper_methods.len() == 1).then(|| wrapper_methods[0].clone())
            })?;

        wrapped_program.functions.retain(|function| {
            if function.name == wrapper_function.name {
                return false;
            }
            if function.name.starts_with("__ayy_class_ctor_")
                && Self::eval_internal_function_name_hint(&function.name)
                    == Some(class_binding_name.as_str())
            {
                return false;
            }
            if function.name.starts_with("__ayy_class_init_")
                && function.body.iter().any(|statement| {
                    matches!(
                        statement,
                        Statement::Return(Expression::Identifier(name))
                            if name == &class_binding_name
                    )
                })
            {
                return false;
            }
            true
        });

        Some(Program {
            strict: wrapper_function.strict,
            functions: wrapped_program.functions,
            statements: wrapper_function.body,
        })
    }

    fn parse_eval_program_in_class_method_context_for_function(
        &self,
        function_name: &str,
        source: &str,
    ) -> Option<Program> {
        let (class_binding_name, private_names) =
            self.class_private_names_for_function(function_name)?;
        let user_function = self.user_function(function_name)?;
        let home_object_name = self.resolve_home_object_name_for_function(function_name)?;
        let wrapper_method_name = "__ayy_eval_wrapper__";
        let private_declarations = private_names
            .into_iter()
            .map(|name| format!("#{name};"))
            .collect::<Vec<_>>()
            .join("\n");
        let mut wrapper_method_prefix = String::new();
        if !home_object_name.ends_with(".prototype") {
            wrapper_method_prefix.push_str("static ");
        }
        if user_function.is_async() {
            wrapper_method_prefix.push_str("async ");
        }
        if user_function.is_generator() {
            wrapper_method_prefix.push('*');
        }
        let wrapped_source = if private_declarations.is_empty() {
            format!(
                "class {class_binding_name} {{\n{wrapper_method_prefix}{wrapper_method_name}() {{\n{source}\n}}\n}}"
            )
        } else {
            format!(
                "class {class_binding_name} {{\n{private_declarations}\n{wrapper_method_prefix}{wrapper_method_name}() {{\n{source}\n}}\n}}"
            )
        };
        let mut wrapped_program = frontend::parse(&wrapped_source).ok()?;
        let wrapper_function = wrapped_program
            .functions
            .iter()
            .find(|function| {
                Self::eval_internal_function_name_hint(&function.name) == Some(wrapper_method_name)
            })
            .cloned()
            .or_else(|| {
                let wrapper_methods = wrapped_program
                    .functions
                    .iter()
                    .filter(|function| function.name.starts_with("__ayy_class_method_"))
                    .cloned()
                    .collect::<Vec<_>>();
                (wrapper_methods.len() == 1).then(|| wrapper_methods[0].clone())
            })?;

        wrapped_program.functions.retain(|function| {
            if function.name == wrapper_function.name {
                return false;
            }
            if function.name.starts_with("__ayy_class_ctor_")
                && Self::eval_internal_function_name_hint(&function.name)
                    == Some(class_binding_name.as_str())
            {
                return false;
            }
            if function.name.starts_with("__ayy_class_init_")
                && function.body.iter().any(|statement| {
                    matches!(
                        statement,
                        Statement::Return(Expression::Identifier(name))
                            if name == &class_binding_name
                    )
                })
            {
                return false;
            }
            true
        });

        Some(Program {
            strict: wrapper_function.strict,
            functions: wrapped_program.functions,
            statements: wrapper_function.body,
        })
    }

    fn parse_eval_program_in_function_context(
        &self,
        source: &str,
        function_name: Option<&str>,
    ) -> Option<Program> {
        if frontend::script_goal_has_direct_using_declaration(source) {
            return None;
        }

        let Some(function_name) = function_name else {
            return self.parse_eval_program_in_current_function_context(source);
        };

        if function_name.starts_with("__ayy_class_method_")
            && let Some(program) =
                self.parse_eval_program_in_class_method_context_for_function(function_name, source)
        {
            return Some(program);
        }

        if self
            .resolve_home_object_name_for_function(function_name)
            .is_some()
            && source.contains("super")
            && let Some(program) = self.parse_eval_program_in_method_context(source)
        {
            return Some(program);
        }

        self.parse_eval_program_in_ordinary_function_context(source)
    }

    fn parse_validated_static_direct_eval_program_with_context(
        &self,
        arguments: &[CallArgument],
        eval_function_name: Option<&str>,
    ) -> Result<Option<Program>, StaticThrowValue> {
        let Some(argument) = arguments.first() else {
            return Ok(None);
        };
        let CallArgument::Expression(Expression::String(argument_source)) = argument else {
            return Ok(None);
        };

        let raw_source = argument_source.clone();
        let eval_context_strict = eval_function_name
            .and_then(|function_name| self.resolve_registered_function_declaration(function_name))
            .is_some_and(|function| function.strict)
            || (eval_function_name.is_none()
                && self.state.speculation.execution_context.strict_mode);
        let argument_source = if eval_context_strict {
            let mut strict_argument_source = String::from("\"use strict\";");
            strict_argument_source.push_str(argument_source);
            Cow::Owned(strict_argument_source)
        } else {
            Cow::Borrowed(argument_source.as_str())
        };

        let program = if let Some(program) =
            self.parse_eval_program_in_function_context(&argument_source, eval_function_name)
        {
            program
        } else if let Ok(program) = frontend::parse_script_goal(&argument_source) {
            program
        } else {
            return Err(StaticThrowValue::NamedError("SyntaxError"));
        };
        let mut program = lower_eval_static_function_constructors(program);

        namespace_eval_program_internal_function_names(
            &mut program,
            eval_function_name.or_else(|| self.current_function_name()),
            &raw_source,
        );
        self.normalize_eval_scoped_bindings_to_source_names(&mut program);

        if self.eval_arguments_initializer_conflict(&program)
            || self.eval_arguments_declaration_conflicts(&program)
            || self.eval_parameter_var_declaration_conflicts(&program)
            || self.eval_program_declares_var_collision_with_global_lexical(&program)
            || self.eval_program_declares_var_collision_with_active_lexical(&program)
        {
            return Err(StaticThrowValue::NamedError("SyntaxError"));
        }

        if self.eval_program_declares_non_definable_global_function(&program) {
            return Err(StaticThrowValue::NamedError("TypeError"));
        }

        Ok(Some(program))
    }

    fn parse_validated_static_direct_eval_program(
        &self,
        arguments: &[CallArgument],
    ) -> Result<Option<Program>, StaticThrowValue> {
        self.parse_validated_static_direct_eval_program_with_context(arguments, None)
    }

    pub(in crate::backend::direct_wasm) fn resolve_static_direct_eval_outcome(
        &self,
        arguments: &[CallArgument],
    ) -> Option<StaticEvalOutcome> {
        match self.parse_validated_static_direct_eval_program(arguments) {
            Ok(_) => None,
            Err(error) => Some(StaticEvalOutcome::Throw(error)),
        }
    }

    fn resolve_static_eval_statement_completion_expression(
        &self,
        statement: &Statement,
    ) -> Option<Expression> {
        match statement {
            Statement::Expression(expression) => Some(expression.clone()),
            Statement::Assign { value, .. } | Statement::AssignMember { value, .. } => {
                Some(value.clone())
            }
            Statement::Block { body } | Statement::Declaration { body } => {
                self.resolve_static_eval_statement_list_completion_expression(body)
            }
            Statement::With { body, .. } => self
                .resolve_static_eval_statement_list_completion_expression(body)
                .or(Some(Expression::Undefined)),
            Statement::Labeled { body, .. }
            | Statement::DoWhile { body, .. }
            | Statement::While { body, .. }
            | Statement::For { body, .. } => {
                self.resolve_static_eval_statement_list_completion_expression(body)
            }
            Statement::If {
                then_branch,
                else_branch,
                ..
            } => {
                let then_completion =
                    self.resolve_static_eval_statement_list_completion_expression(then_branch);
                let else_completion =
                    self.resolve_static_eval_statement_list_completion_expression(else_branch);
                match (then_completion, else_completion) {
                    (Some(then_completion), Some(else_completion))
                        if static_expression_matches(&then_completion, &else_completion) =>
                    {
                        Some(then_completion)
                    }
                    _ => None,
                }
            }
            _ => None,
        }
    }

    fn resolve_static_eval_statement_list_completion_expression(
        &self,
        statements: &[Statement],
    ) -> Option<Expression> {
        let mut completion = None;
        for statement in statements {
            if let Some(statement_completion) =
                self.resolve_static_eval_statement_completion_expression(statement)
            {
                completion = Some(statement_completion);
            }
        }
        Some(completion.unwrap_or(Expression::Undefined))
    }

    pub(in crate::backend::direct_wasm) fn resolve_static_direct_eval_completion_outcome_with_context(
        &self,
        arguments: &[CallArgument],
        eval_function_name: Option<&str>,
    ) -> Option<StaticEvalOutcome> {
        match self
            .parse_validated_static_direct_eval_program_with_context(arguments, eval_function_name)
        {
            Ok(Some(program)) => self
                .resolve_static_eval_statement_list_completion_expression(&program.statements)
                .map(StaticEvalOutcome::Value),
            Ok(None) => None,
            Err(error) => Some(StaticEvalOutcome::Throw(error)),
        }
    }

    fn static_eval_statement_inline_effects(
        &self,
        statement: &Statement,
    ) -> Option<Vec<InlineFunctionEffect>> {
        match statement {
            Statement::Assign { name, value } => Some(vec![InlineFunctionEffect::Assign {
                name: name.clone(),
                value: value.clone(),
            }]),
            Statement::Expression(Expression::Assign { name, value }) => {
                Some(vec![InlineFunctionEffect::Assign {
                    name: name.clone(),
                    value: value.as_ref().clone(),
                }])
            }
            Statement::Expression(Expression::Update { name, op, prefix }) => {
                Some(vec![InlineFunctionEffect::Update {
                    name: name.clone(),
                    op: *op,
                    prefix: *prefix,
                }])
            }
            Statement::Expression(expression) => {
                Some(vec![InlineFunctionEffect::Expression(expression.clone())])
            }
            Statement::Block { body } | Statement::Declaration { body } => {
                self.static_eval_statement_list_inline_effects(body)
            }
            _ => None,
        }
    }

    fn static_eval_statement_list_inline_effects(
        &self,
        statements: &[Statement],
    ) -> Option<Vec<InlineFunctionEffect>> {
        let mut effects = Vec::new();
        for statement in statements {
            effects.extend(self.static_eval_statement_inline_effects(statement)?);
        }
        Some(effects)
    }

    pub(in crate::backend::direct_wasm) fn static_direct_eval_inline_effects_with_context(
        &self,
        arguments: &[CallArgument],
        eval_function_name: Option<&str>,
    ) -> Option<Vec<InlineFunctionEffect>> {
        let program = match self
            .parse_validated_static_direct_eval_program_with_context(arguments, eval_function_name)
        {
            Ok(Some(program)) => program,
            Ok(None) => return Some(Vec::new()),
            Err(_) => return None,
        };
        self.static_eval_statement_list_inline_effects(&program.statements)
    }

    pub(in crate::backend::direct_wasm) fn resolve_static_direct_eval_return_outcome_from_user_function(
        &self,
        user_function: &UserFunction,
        function: &FunctionDeclaration,
        arguments: &[CallArgument],
        this_binding: &Expression,
    ) -> Option<StaticEvalOutcome> {
        let [Statement::Return(expression)] = function.body.as_slice() else {
            return None;
        };
        let arguments_binding = Expression::Array(
            arguments
                .iter()
                .map(|argument| match argument {
                    CallArgument::Expression(expression) => {
                        ArrayElement::Expression(expression.clone())
                    }
                    CallArgument::Spread(expression) => ArrayElement::Spread(expression.clone()),
                })
                .collect(),
        );
        let expression = self.substitute_user_function_call_frame_bindings(
            expression,
            user_function,
            arguments,
            this_binding,
            &arguments_binding,
        );
        let Expression::Call {
            callee,
            arguments: eval_arguments,
        } = expression
        else {
            return None;
        };
        if !matches!(callee.as_ref(), Expression::Identifier(name) if name == "eval") {
            return None;
        }
        self.resolve_static_direct_eval_completion_outcome_with_context(
            &eval_arguments,
            Some(&function.name),
        )
    }

    pub(in crate::backend::direct_wasm) fn infer_static_direct_eval_completion_kind(
        &self,
        arguments: &[CallArgument],
    ) -> Option<StaticValueKind> {
        let program = self
            .parse_validated_static_direct_eval_program(arguments)
            .ok()
            .flatten()?;
        self.infer_eval_statement_list_completion_kind(&program.statements)
    }

    fn infer_eval_statement_list_completion_kind(
        &self,
        statements: &[Statement],
    ) -> Option<StaticValueKind> {
        let mut completion_kind = None;
        for statement in statements {
            if let Some(kind) = self.infer_eval_statement_completion_kind(statement) {
                completion_kind = Some(kind);
            }
        }
        completion_kind
    }

    fn infer_eval_statement_completion_kind(
        &self,
        statement: &Statement,
    ) -> Option<StaticValueKind> {
        match statement {
            Statement::Expression(expression) => self.infer_value_kind(expression),
            Statement::Assign { value, .. } | Statement::AssignMember { value, .. } => {
                self.infer_value_kind(value)
            }
            Statement::Block { body } | Statement::Declaration { body } => {
                self.infer_eval_statement_list_completion_kind(body)
            }
            Statement::With { body, .. } => self
                .infer_eval_statement_list_completion_kind(body)
                .or(Some(StaticValueKind::Undefined)),
            Statement::DoWhile { body, .. }
            | Statement::While { body, .. }
            | Statement::For { body, .. } => self.infer_eval_statement_list_completion_kind(body),
            Statement::Labeled { body, .. } => self.infer_eval_statement_list_completion_kind(body),
            Statement::If {
                then_branch,
                else_branch,
                ..
            } => {
                let then_kind = self.infer_eval_statement_list_completion_kind(then_branch);
                let else_kind = self.infer_eval_statement_list_completion_kind(else_branch);
                if then_kind == else_kind {
                    then_kind
                } else {
                    Some(StaticValueKind::Unknown)
                }
            }
            _ => None,
        }
    }

    pub(in crate::backend::direct_wasm) fn eval_arguments_declaration_conflicts(
        &self,
        program: &Program,
    ) -> bool {
        if !eval_program_declares_var_arguments(program) {
            return false;
        }

        let Some(current_function) = self.current_user_function() else {
            return false;
        };

        self.state.parameters.in_parameter_default_initialization
            && (!current_function.lexical_this
                || current_function
                    .params
                    .iter()
                    .any(|parameter| parameter == "arguments"))
    }

    pub(in crate::backend::direct_wasm) fn eval_arguments_initializer_conflict(
        &self,
        program: &Program,
    ) -> bool {
        self.state
            .speculation
            .execution_context
            .direct_eval_in_class_field_initializer
            && eval_program_contains_arguments(program)
    }

    pub(in crate::backend::direct_wasm) fn eval_parameter_var_declaration_conflicts(
        &self,
        program: &Program,
    ) -> bool {
        if program.strict || !self.state.parameters.in_parameter_default_initialization {
            return false;
        }

        collect_eval_var_names(program).into_iter().any(|var_name| {
            self.state
                .parameters
                .parameter_names
                .iter()
                .any(|param_name| {
                    scoped_binding_source_name(param_name).unwrap_or(param_name.as_str())
                        == var_name
                })
        })
    }

    pub(in crate::backend::direct_wasm) fn eval_program_declares_var_collision_with_global_lexical(
        &self,
        program: &Program,
    ) -> bool {
        if !self.state.speculation.execution_context.top_level_function || program.strict {
            return false;
        }

        collect_eval_var_names(program)
            .into_iter()
            .any(|name| self.backend.global_has_lexical_binding(&name))
    }

    pub(in crate::backend::direct_wasm) fn eval_program_declares_var_collision_with_active_lexical(
        &self,
        program: &Program,
    ) -> bool {
        if program.strict {
            return false;
        }

        collect_eval_var_names(program).into_iter().any(|name| {
            self.state
                .emission
                .lexical_scopes
                .active_eval_lexical_binding_counts
                .contains_key(&name)
        })
    }

    pub(in crate::backend::direct_wasm) fn eval_program_declares_non_definable_global_function(
        &self,
        program: &Program,
    ) -> bool {
        if !self.state.speculation.execution_context.top_level_function {
            return false;
        }

        program
            .functions
            .iter()
            .filter(|function| function.register_global)
            .any(|function| is_non_definable_global_name(&function.name))
    }

    pub(in crate::backend::direct_wasm) fn parse_eval_program_in_current_function_context(
        &self,
        source: &str,
    ) -> Option<Program> {
        if frontend::script_goal_has_direct_using_declaration(source) {
            return None;
        }

        if self
            .state
            .speculation
            .execution_context
            .direct_eval_in_class_field_initializer
            && let Some(program) =
                self.parse_eval_program_in_class_field_initializer_context(source)
        {
            return Some(program);
        }

        let current_function_name = self.current_function_name();
        if let Some(current_function_name) = current_function_name {
            if current_function_name.starts_with("__ayy_class_method_")
                && let Some(program) = self.parse_eval_program_in_class_method_context_for_function(
                    current_function_name,
                    source,
                )
            {
                return Some(program);
            }

            if self
                .resolve_home_object_name_for_function(current_function_name)
                .is_some()
                && source.contains("super")
            {
                if let Some(program) = self.parse_eval_program_in_method_context(source) {
                    return Some(program);
                }
            }
        }

        if current_function_name.is_some()
            || self
                .state
                .speculation
                .execution_context
                .direct_eval_in_class_field_initializer
        {
            return self.parse_eval_program_in_ordinary_function_context(source);
        }

        None
    }

    pub(in crate::backend::direct_wasm) fn parse_eval_program_in_ordinary_function_context(
        &self,
        source: &str,
    ) -> Option<Program> {
        let wrapper_name = "__ayy_eval_new_target_context__";
        let wrapped_source = format!("function {wrapper_name}() {{\n{source}\n}}");
        let mut wrapped_program = match frontend::parse(&wrapped_source) {
            Ok(program) => program,
            Err(_) => return None,
        };
        let wrapper = wrapped_program
            .functions
            .iter()
            .find(|function| function.name == wrapper_name)
            .cloned()?;
        wrapped_program
            .functions
            .retain(|function| function.name != wrapper_name);

        Some(Program {
            strict: wrapper.strict,
            functions: wrapped_program.functions,
            statements: wrapper.body,
        })
    }

    pub(in crate::backend::direct_wasm) fn parse_eval_program_in_method_context(
        &self,
        source: &str,
    ) -> Option<Program> {
        let wrapper_property = "__ayy_eval_wrapper__";
        let wrapped_source = format!("({{{wrapper_property}() {{\n{source}\n}}}});");
        let mut wrapped_program = frontend::parse(&wrapped_source).ok()?;
        let wrapper_name = wrapped_program.statements.iter().find_map(|statement| {
            let Statement::Expression(Expression::Object(entries)) = statement else {
                return None;
            };
            entries.iter().find_map(|entry| match entry {
                crate::ir::hir::ObjectEntry::Data { key, value }
                    if matches!(key, Expression::String(name) if name == wrapper_property) =>
                {
                    let Expression::Identifier(name) = value else {
                        return None;
                    };
                    Some(name.clone())
                }
                _ => None,
            })
        })?;
        let wrapper = wrapped_program
            .functions
            .iter()
            .find(|function| function.name == wrapper_name)
            .cloned()?;
        wrapped_program
            .functions
            .retain(|function| function.name != wrapper_name);

        Some(Program {
            strict: wrapper.strict,
            functions: wrapped_program.functions,
            statements: wrapper.body,
        })
    }
}

pub(in crate::backend::direct_wasm) fn eval_program_contains_arguments(program: &Program) -> bool {
    program
        .statements
        .iter()
        .any(eval_statement_contains_arguments)
        || program
            .functions
            .iter()
            .any(|function| function.body.iter().any(eval_statement_contains_arguments))
}

fn eval_statement_contains_arguments(statement: &Statement) -> bool {
    match statement {
        Statement::Declaration { body }
        | Statement::Block { body }
        | Statement::Labeled { body, .. } => body.iter().any(eval_statement_contains_arguments),
        Statement::With { object, body } => {
            eval_expression_contains_arguments(object)
                || body.iter().any(eval_statement_contains_arguments)
        }
        Statement::Var { value, .. }
        | Statement::Let { value, .. }
        | Statement::Assign { value, .. }
        | Statement::Return(value)
        | Statement::Throw(value)
        | Statement::Expression(value) => eval_expression_contains_arguments(value),
        Statement::AssignMember {
            object,
            property,
            value,
        } => {
            eval_expression_contains_arguments(object)
                || eval_expression_contains_arguments(property)
                || eval_expression_contains_arguments(value)
        }
        Statement::Print { values } => values.iter().any(eval_expression_contains_arguments),
        Statement::If {
            condition,
            then_branch,
            else_branch,
        } => {
            eval_expression_contains_arguments(condition)
                || then_branch.iter().any(eval_statement_contains_arguments)
                || else_branch.iter().any(eval_statement_contains_arguments)
        }
        Statement::While {
            condition, body, ..
        }
        | Statement::DoWhile {
            condition, body, ..
        } => {
            eval_expression_contains_arguments(condition)
                || body.iter().any(eval_statement_contains_arguments)
        }
        Statement::For {
            init,
            condition,
            update,
            body,
            ..
        } => {
            init.iter().any(eval_statement_contains_arguments)
                || condition
                    .as_ref()
                    .is_some_and(eval_expression_contains_arguments)
                || update
                    .as_ref()
                    .is_some_and(eval_expression_contains_arguments)
                || body.iter().any(eval_statement_contains_arguments)
        }
        Statement::Break { .. } | Statement::Continue { .. } => false,
        Statement::Try {
            body,
            catch_binding: _,
            catch_setup,
            catch_body,
        } => {
            body.iter().any(eval_statement_contains_arguments)
                || catch_setup.iter().any(eval_statement_contains_arguments)
                || catch_body.iter().any(eval_statement_contains_arguments)
        }
        Statement::Switch {
            discriminant,
            cases,
            ..
        } => {
            eval_expression_contains_arguments(discriminant)
                || cases.iter().any(|case| {
                    case.test
                        .as_ref()
                        .is_some_and(eval_expression_contains_arguments)
                        || case.body.iter().any(eval_statement_contains_arguments)
                })
        }
        Statement::Yield { value } | Statement::YieldDelegate { value } => {
            eval_expression_contains_arguments(value)
        }
    }
}

fn eval_expression_contains_arguments(expression: &Expression) -> bool {
    match expression {
        Expression::Identifier(name) => name == "arguments",
        Expression::Null
        | Expression::Undefined
        | Expression::Bool(_)
        | Expression::Number(_)
        | Expression::BigInt(_)
        | Expression::String(_)
        | Expression::NewTarget
        | Expression::This
        | Expression::Sent => false,
        Expression::Array(elements) => elements.iter().any(|element| match element {
            ArrayElement::Expression(expression) | ArrayElement::Spread(expression) => {
                eval_expression_contains_arguments(expression)
            }
        }),
        Expression::Object(entries) => entries.iter().any(|entry| match entry {
            ObjectEntry::Data { key, value } => {
                eval_expression_contains_arguments(key) || eval_expression_contains_arguments(value)
            }
            ObjectEntry::Getter { key, getter }
            | ObjectEntry::Setter {
                key,
                setter: getter,
            } => {
                eval_expression_contains_arguments(key)
                    || eval_expression_contains_arguments(getter)
            }
            ObjectEntry::Spread(expression) => eval_expression_contains_arguments(expression),
        }),
        Expression::Member { object, property } => {
            eval_expression_contains_arguments(object)
                || eval_expression_contains_arguments(property)
        }
        Expression::SuperMember { property }
        | Expression::Await(property)
        | Expression::EnumerateKeys(property)
        | Expression::GetIterator(property)
        | Expression::IteratorClose(property) => eval_expression_contains_arguments(property),
        Expression::Unary { expression, .. } => eval_expression_contains_arguments(expression),
        Expression::Binary { left, right, .. } => {
            eval_expression_contains_arguments(left) || eval_expression_contains_arguments(right)
        }
        Expression::Conditional {
            condition,
            then_expression,
            else_expression,
        } => {
            eval_expression_contains_arguments(condition)
                || eval_expression_contains_arguments(then_expression)
                || eval_expression_contains_arguments(else_expression)
        }
        Expression::Assign { value, .. } => eval_expression_contains_arguments(value),
        Expression::AssignMember {
            object,
            property,
            value,
        } => {
            eval_expression_contains_arguments(object)
                || eval_expression_contains_arguments(property)
                || eval_expression_contains_arguments(value)
        }
        Expression::AssignSuperMember { property, value } => {
            eval_expression_contains_arguments(property)
                || eval_expression_contains_arguments(value)
        }
        Expression::Call { callee, arguments }
        | Expression::New { callee, arguments }
        | Expression::SuperCall { callee, arguments } => {
            eval_expression_contains_arguments(callee)
                || arguments.iter().any(|argument| match argument {
                    CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                        eval_expression_contains_arguments(expression)
                    }
                })
        }
        Expression::Sequence(expressions) => {
            expressions.iter().any(eval_expression_contains_arguments)
        }
        Expression::Update { .. } => false,
    }
}
