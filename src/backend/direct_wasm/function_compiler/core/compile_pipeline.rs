use super::*;

impl<'a> FunctionCompiler<'a> {
    pub(in crate::backend::direct_wasm) fn compile(
        self,
        statements: &[Statement],
    ) -> DirectResult<CompiledFunction> {
        let mut compiler = self;
        compiler.compile_in_current_global_scope(statements, None)
    }

    pub(in crate::backend::direct_wasm) fn compile_with_initial_named_error(
        self,
        statements: &[Statement],
        error_name: &str,
    ) -> DirectResult<CompiledFunction> {
        let mut compiler = self;
        compiler.compile_in_current_global_scope(statements, Some(error_name))
    }

    fn compile_in_current_global_scope(
        &mut self,
        statements: &[Statement],
        initial_named_error: Option<&str>,
    ) -> DirectResult<CompiledFunction> {
        let trace = std::env::var_os("AYY_TRACE_FUNCTION_COMPILE").is_some();
        if trace {
            eprintln!("function_compile=register_statements");
        }
        self.bindings_domain().register_statements(statements)?;
        let mut declared_local_indices = self
            .state
            .runtime
            .locals
            .bindings
            .values()
            .copied()
            .filter(|local_index| *local_index >= self.state.parameters.param_count)
            .collect::<Vec<_>>();
        declared_local_indices.sort_unstable();
        declared_local_indices.dedup();
        for local_index in declared_local_indices {
            self.push_i32_const(JS_UNDEFINED_TAG);
            self.push_local_set(local_index);
        }
        if self.current_function_name().is_some() && !self.current_function_is_derived_constructor()
        {
            self.seed_local_this_object_binding();
        }
        self.push_global_get(THROW_TAG_GLOBAL_INDEX);
        self.push_local_set(self.state.runtime.throws.throw_tag_local);
        self.push_global_get(THROW_VALUE_GLOBAL_INDEX);
        self.push_local_set(self.state.runtime.throws.throw_value_local);
        if let Some(parameter_scope_arguments_local) =
            self.state.parameters.parameter_scope_arguments_local
        {
            self.push_i32_const(JS_UNDEFINED_TAG);
            self.push_local_set(parameter_scope_arguments_local);
        }
        if let Some(local_index) = self.state.speculation.execution_context.self_binding_local
            && let Some(runtime_value) = self
                .state
                .speculation
                .execution_context
                .self_binding_runtime_value
        {
            self.push_i32_const(runtime_value);
            self.push_local_set(local_index);
        }
        let parameter_initialized_locals = self
            .state
            .parameters
            .parameter_initialized_locals
            .values()
            .copied()
            .collect::<Vec<_>>();
        for initialized_local in parameter_initialized_locals {
            self.push_i32_const(0);
            self.push_local_set(initialized_local);
        }
        if trace {
            eprintln!("function_compile=initialize_arguments_object");
        }
        self.initialize_arguments_object(statements)?;
        self.initialize_function_scope_arguments_binding_local();
        if trace {
            eprintln!("function_compile=initialize_parameter_defaults");
        }
        self.initialize_parameter_defaults()?;
        self.initialize_rest_parameter_runtime_array()?;
        if trace {
            eprintln!("function_compile=emit_direct_scope");
        }
        if let Some(error_name) = initial_named_error {
            self.emit_named_error_throw(error_name)?;
        } else if self
            .current_async_function_static_promise_outcome(statements)
            .is_some()
            || self.current_async_function_static_tick_order_shape(statements)
        {
            if trace {
                eprintln!("function_compile=skip_static_async_promise_body");
            }
        } else if self.should_enable_tail_restart_for_current_function(statements) {
            self.emit_function_body_with_tail_restart(statements)?;
        } else {
            self.control_flow_domain().emit_direct_scope(statements)?;
        }

        if trace {
            eprintln!("function_compile=finalize");
        }
        self.exception_domain().clear_throw_state();
        if self.state.runtime.behavior.allow_return {
            if self.current_function_is_derived_constructor() {
                self.sync_current_derived_constructor_runtime_this_shadow_to_static_owner()?;
                let this_local = self.allocate_temp_local();
                self.push_global_get(CURRENT_THIS_GLOBAL_INDEX);
                self.push_local_tee(this_local);
                self.push_i32_const(JS_UNDEFINED_TAG);
                self.push_binary_op(BinaryOp::Equal)?;
                self.state.emission.output.instructions.push(0x04);
                self.state.emission.output.instructions.push(I32_TYPE);
                self.push_control_frame();
                self.emit_named_error_throw("ReferenceError")?;
                self.push_i32_const(JS_UNDEFINED_TAG);
                self.state.emission.output.instructions.push(0x05);
                self.push_local_get(this_local);
                self.state.emission.output.instructions.push(0x0b);
                self.pop_control_frame();
            } else {
                self.push_i32_const(JS_UNDEFINED_TAG);
            }
        }

        let instructions = std::mem::take(&mut self.state.emission.output.instructions);
        Ok(CompiledFunction {
            local_count: self.state.runtime.locals.next_local_index
                - self.state.parameters.param_count,
            instructions,
        })
    }

    fn initialize_function_scope_arguments_binding_local(&mut self) {
        let Some(function) = self.current_user_function() else {
            return;
        };
        if !function.body_declares_arguments_binding
            || function.params.iter().any(|param| {
                param == "arguments"
                    || scoped_binding_source_name(param)
                        .is_some_and(|source_name| source_name == "arguments")
            })
        {
            return;
        }
        let Some((resolved_name, local_index)) = self.resolve_current_local_binding("arguments")
        else {
            return;
        };
        if local_index < self.state.parameters.param_count {
            return;
        }

        self.push_i32_const(JS_TYPEOF_OBJECT_TAG);
        self.push_local_set(local_index);
        self.state
            .parameters
            .direct_arguments_aliases
            .insert(resolved_name.clone());
        self.state
            .speculation
            .static_semantics
            .set_local_kind(&resolved_name, StaticValueKind::Object);
    }

    fn initialize_rest_parameter_runtime_array(&mut self) -> DirectResult<()> {
        let Some((rest_index, rest_name)) = self.current_rest_parameter_binding() else {
            return Ok(());
        };
        let Some(actual_argument_count_local) = self.state.parameters.actual_argument_count_local
        else {
            return Ok(());
        };
        let rest_index = rest_index as u32;
        let length_local = self.ensure_runtime_array_length_local(&rest_name);

        self.push_local_get(actual_argument_count_local);
        self.push_i32_const(rest_index as i32);
        self.push_binary_op(BinaryOp::GreaterThan)?;
        self.state.emission.output.instructions.push(0x04);
        self.state.emission.output.instructions.push(I32_TYPE);
        self.push_control_frame();
        self.push_local_get(actual_argument_count_local);
        self.push_i32_const(rest_index as i32);
        self.push_binary_op(BinaryOp::Subtract)?;
        self.state.emission.output.instructions.push(0x05);
        self.push_i32_const(0);
        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();
        self.push_local_set(length_local);

        for rest_slot_index in 0..TRACKED_ARRAY_SLOT_LIMIT {
            let source_argument_index = rest_index + rest_slot_index;
            let source_local = if source_argument_index < self.state.parameters.visible_param_count
            {
                Some(source_argument_index)
            } else {
                self.state
                    .parameters
                    .extra_argument_param_locals
                    .get(&source_argument_index)
                    .copied()
            };
            let slot = self.ensure_runtime_array_slot_entry(&rest_name, rest_slot_index);

            if let Some(source_local) = source_local {
                self.push_local_get(source_local);
            } else {
                self.push_i32_const(JS_UNDEFINED_TAG);
            }
            self.push_local_set(slot.value_local);

            if source_local.is_some() {
                self.push_local_get(actual_argument_count_local);
                self.push_i32_const(source_argument_index as i32);
                self.push_binary_op(BinaryOp::GreaterThan)?;
            } else {
                self.push_i32_const(0);
            }
            self.push_local_set(slot.present_local);
        }

        Ok(())
    }

    fn emit_function_body_with_tail_restart(
        &mut self,
        statements: &[Statement],
    ) -> DirectResult<()> {
        let parameter_names = self.state.parameters.parameter_names.clone();
        for parameter_name in parameter_names {
            self.state
                .clear_local_static_binding_metadata(&parameter_name);
        }

        self.state.emission.output.instructions.push(0x02);
        self.state
            .emission
            .output
            .instructions
            .push(EMPTY_BLOCK_TYPE);
        let break_target = self.push_control_frame();

        self.state.emission.output.instructions.push(0x03);
        self.state
            .emission
            .output
            .instructions
            .push(EMPTY_BLOCK_TYPE);
        let restart_target = self.push_control_frame();
        self.state.emission.control_flow.tail_call_restart_target = Some(restart_target);

        self.emit_tail_restart_activation_prelude(statements);
        let emit_result = self.control_flow_domain().emit_direct_scope(statements);
        self.state.emission.control_flow.tail_call_restart_target = None;
        emit_result?;

        self.push_br(self.relative_depth(break_target));
        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();
        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();
        Ok(())
    }

    fn emit_tail_restart_activation_prelude(&mut self, statements: &[Statement]) {
        let mut reset_bindings = std::collections::HashSet::new();
        collect_tail_restart_var_bindings(statements, &mut reset_bindings);

        let parameter_names = self.state.parameters.parameter_names.clone();
        for parameter_name in parameter_names {
            reset_bindings.remove(&parameter_name);
        }
        reset_bindings.remove("arguments");

        if let Some(declaration) = self.current_user_function_declaration() {
            if let Some(self_binding) = declaration.self_binding.as_deref() {
                reset_bindings.remove(self_binding);
            }
            if let Some(top_level_binding) = declaration.top_level_binding.as_deref() {
                reset_bindings.remove(top_level_binding);
            }
        }

        let mut reset_bindings = reset_bindings.into_iter().collect::<Vec<_>>();
        reset_bindings.sort();
        for binding in reset_bindings {
            let Some((resolved_name, local_index)) = self.resolve_current_local_binding(&binding)
            else {
                continue;
            };
            if local_index < self.state.parameters.param_count {
                continue;
            }

            self.push_i32_const(JS_UNDEFINED_TAG);
            self.push_local_set(local_index);
            self.state
                .clear_local_static_binding_metadata(&resolved_name);
            self.update_local_value_binding(&resolved_name, &Expression::Undefined);
            self.state
                .speculation
                .static_semantics
                .set_local_kind(&resolved_name, StaticValueKind::Undefined);
        }
    }

    fn should_enable_tail_restart_for_current_function(&self, statements: &[Statement]) -> bool {
        let trace = std::env::var_os("AYY_TRACE_FUNCTION_COMPILE").is_some();
        if !self.state.runtime.behavior.allow_return
            || !self.state.parameters.arguments_slots.is_empty()
        {
            if trace {
                eprintln!(
                    "function_compile=tail_restart_disabled precheck allow_return={} arguments_slots={}",
                    self.state.runtime.behavior.allow_return,
                    self.state.parameters.arguments_slots.len()
                );
            }
            return false;
        }

        let Some(function) = self.current_user_function() else {
            if trace {
                eprintln!("function_compile=tail_restart_disabled no_current_function");
            }
            return false;
        };
        if function.is_async()
            || function.is_generator()
            || function.has_parameter_defaults()
            || function.has_lowered_pattern_parameters()
            || !function.extra_argument_indices.is_empty()
            || statements_reference_this_or_new_target_for_tail_restart(statements)
        {
            if trace {
                eprintln!(
                    "function_compile=tail_restart_disabled function={} async={} generator={} defaults={} patterns={} extra_args={} this_or_new={}",
                    function.name,
                    function.is_async(),
                    function.is_generator(),
                    function.has_parameter_defaults(),
                    function.has_lowered_pattern_parameters(),
                    function.extra_argument_indices.len(),
                    statements_reference_this_or_new_target_for_tail_restart(statements)
                );
            }
            return false;
        }

        let callee_names = self.current_tail_restart_callee_names(function);

        let enabled = self.statements_contain_current_self_tail_call(
            statements,
            &callee_names,
            function.params.len(),
        );
        if trace {
            eprintln!(
                "function_compile=tail_restart_candidate function={} enabled={} callee_names={:?} arity={}",
                function.name,
                enabled,
                callee_names,
                function.params.len()
            );
        }
        enabled
    }

    fn current_tail_restart_callee_names(
        &self,
        function: &UserFunction,
    ) -> std::collections::HashSet<String> {
        let mut callee_names = std::collections::HashSet::new();
        callee_names.insert(function.name.clone());
        if let Some(source_name) = Self::generated_function_statement_source_name(&function.name) {
            callee_names.insert(source_name.to_string());
        }
        if let Some(current_name) = self.current_function_name() {
            callee_names.insert(current_name.to_string());
            if let Some(source_name) = Self::generated_function_statement_source_name(current_name)
            {
                callee_names.insert(source_name.to_string());
            }
        }
        if let Some(self_binding) = self
            .current_user_function_declaration()
            .and_then(|declaration| declaration.self_binding.clone())
        {
            callee_names.insert(self_binding);
        }
        if let Some(top_level_binding) = self
            .current_user_function_declaration()
            .and_then(|declaration| declaration.top_level_binding.clone())
        {
            callee_names.insert(top_level_binding);
        }
        callee_names
    }

    fn statements_contain_current_self_tail_call(
        &self,
        statements: &[Statement],
        callee_names: &std::collections::HashSet<String>,
        arity: usize,
    ) -> bool {
        statements.iter().any(|statement| {
            self.statement_contains_current_self_tail_call(statement, callee_names, arity)
        })
    }

    fn statement_contains_current_self_tail_call(
        &self,
        statement: &Statement,
        callee_names: &std::collections::HashSet<String>,
        arity: usize,
    ) -> bool {
        match statement {
            Statement::Return(expression) => {
                self.expression_is_current_self_tail_call(expression, callee_names, arity)
            }
            Statement::Declaration { body }
            | Statement::Block { body }
            | Statement::Labeled { body, .. }
            | Statement::With { body, .. }
            | Statement::DoWhile { body, .. }
            | Statement::While { body, .. } => {
                self.statements_contain_current_self_tail_call(body, callee_names, arity)
            }
            Statement::If {
                then_branch,
                else_branch,
                ..
            } => {
                self.statements_contain_current_self_tail_call(then_branch, callee_names, arity)
                    || self.statements_contain_current_self_tail_call(
                        else_branch,
                        callee_names,
                        arity,
                    )
            }
            Statement::Try {
                body,
                catch_setup,
                catch_body,
                ..
            } => {
                self.statements_contain_current_self_tail_call(body, callee_names, arity)
                    || self.statements_contain_current_self_tail_call(
                        catch_setup,
                        callee_names,
                        arity,
                    )
                    || self.statements_contain_current_self_tail_call(
                        catch_body,
                        callee_names,
                        arity,
                    )
            }
            Statement::Switch { cases, .. } => cases.iter().any(|case| {
                self.statements_contain_current_self_tail_call(&case.body, callee_names, arity)
            }),
            Statement::For { init, body, .. } => {
                self.statements_contain_current_self_tail_call(init, callee_names, arity)
                    || self.statements_contain_current_self_tail_call(body, callee_names, arity)
            }
            Statement::Var { .. }
            | Statement::Let { .. }
            | Statement::Assign { .. }
            | Statement::AssignMember { .. }
            | Statement::Print { .. }
            | Statement::Expression(_)
            | Statement::Throw(_)
            | Statement::Break { .. }
            | Statement::Continue { .. }
            | Statement::Yield { .. }
            | Statement::YieldDelegate { .. } => false,
        }
    }

    fn expression_is_current_self_tail_call(
        &self,
        expression: &Expression,
        callee_names: &std::collections::HashSet<String>,
        arity: usize,
    ) -> bool {
        match expression {
            Expression::Call { callee, arguments } => {
                self.expression_resolves_to_current_tail_callee(callee, callee_names)
                    && arguments.len() == arity
                    && arguments
                        .iter()
                        .all(|argument| matches!(argument, CallArgument::Expression(_)))
            }
            Expression::Binary { op, left, right } => match op {
                BinaryOp::LogicalAnd if matches!(left.as_ref(), Expression::Bool(true)) => {
                    self.expression_is_current_self_tail_call(right, callee_names, arity)
                }
                BinaryOp::LogicalOr if matches!(left.as_ref(), Expression::Bool(false)) => {
                    self.expression_is_current_self_tail_call(right, callee_names, arity)
                }
                BinaryOp::NullishCoalescing
                    if matches!(left.as_ref(), Expression::Null | Expression::Undefined)
                        || matches!(left.as_ref(), Expression::Identifier(name) if name == "undefined") =>
                {
                    self.expression_is_current_self_tail_call(right, callee_names, arity)
                }
                _ => false,
            },
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => match condition.as_ref() {
                Expression::Bool(true) => {
                    self.expression_is_current_self_tail_call(then_expression, callee_names, arity)
                }
                Expression::Bool(false) => {
                    self.expression_is_current_self_tail_call(else_expression, callee_names, arity)
                }
                _ => false,
            },
            Expression::Sequence(expressions) => expressions.last().is_some_and(|expression| {
                self.expression_is_current_self_tail_call(expression, callee_names, arity)
            }),
            _ => false,
        }
    }

    fn expression_resolves_to_current_tail_callee(
        &self,
        expression: &Expression,
        callee_names: &std::collections::HashSet<String>,
    ) -> bool {
        if let Expression::Identifier(name) = expression {
            return identifier_matches_tail_callee_name(name, callee_names);
        }
        if self.zero_arg_function_call_returns_current_tail_callee(expression, callee_names) {
            return true;
        }
        matches!(
            self.resolve_function_binding_from_expression(expression),
            Some(LocalFunctionBinding::User(function_name))
                if identifier_matches_tail_callee_name(&function_name, callee_names)
        )
    }

    fn zero_arg_function_call_returns_current_tail_callee(
        &self,
        expression: &Expression,
        callee_names: &std::collections::HashSet<String>,
    ) -> bool {
        let Expression::Call { callee, arguments } = expression else {
            return false;
        };
        if !arguments.is_empty() {
            return false;
        }
        let Expression::Identifier(name) = callee.as_ref() else {
            return false;
        };
        let Some(returned_expression) =
            self.simple_zero_arg_function_statement_return_expression(name)
        else {
            return false;
        };
        self.returned_expression_matches_current_tail_callee(returned_expression, callee_names)
    }

    fn returned_expression_matches_current_tail_callee(
        &self,
        expression: &Expression,
        callee_names: &std::collections::HashSet<String>,
    ) -> bool {
        if let Expression::Identifier(name) = expression {
            if identifier_matches_tail_callee_name(name, callee_names) {
                return true;
            }
            let source_name = scoped_binding_source_name(name).unwrap_or(name);
            if let Some(function_name) =
                self.function_statement_binding_name_for_source(source_name)
                && identifier_matches_tail_callee_name(&function_name, callee_names)
            {
                return true;
            }
        }
        matches!(
            self.resolve_function_binding_from_expression(expression),
            Some(LocalFunctionBinding::User(function_name))
                if identifier_matches_tail_callee_name(&function_name, callee_names)
        )
    }
}

fn identifier_matches_tail_callee_name(
    name: &str,
    callee_names: &std::collections::HashSet<String>,
) -> bool {
    callee_names.contains(name)
        || scoped_binding_source_name(name).is_some_and(|source_name| {
            callee_names
                .iter()
                .any(|callee_name| callee_name.as_str() == source_name)
        })
}

fn collect_tail_restart_var_bindings(
    statements: &[Statement],
    bindings: &mut std::collections::HashSet<String>,
) {
    for statement in statements {
        collect_tail_restart_var_bindings_from_statement(statement, bindings);
    }
}

fn collect_tail_restart_var_bindings_from_statement(
    statement: &Statement,
    bindings: &mut std::collections::HashSet<String>,
) {
    match statement {
        Statement::Var { name, .. } => {
            bindings.insert(name.clone());
        }
        Statement::Declaration { body }
        | Statement::Block { body }
        | Statement::Labeled { body, .. }
        | Statement::With { body, .. }
        | Statement::DoWhile { body, .. }
        | Statement::While { body, .. } => {
            collect_tail_restart_var_bindings(body, bindings);
        }
        Statement::If {
            then_branch,
            else_branch,
            ..
        } => {
            collect_tail_restart_var_bindings(then_branch, bindings);
            collect_tail_restart_var_bindings(else_branch, bindings);
        }
        Statement::Try {
            body,
            catch_setup,
            catch_body,
            ..
        } => {
            collect_tail_restart_var_bindings(body, bindings);
            collect_tail_restart_var_bindings(catch_setup, bindings);
            collect_tail_restart_var_bindings(catch_body, bindings);
        }
        Statement::Switch { cases, .. } => {
            for case in cases {
                collect_tail_restart_var_bindings(&case.body, bindings);
            }
        }
        Statement::For { init, body, .. } => {
            collect_tail_restart_var_bindings(init, bindings);
            collect_tail_restart_var_bindings(body, bindings);
        }
        Statement::Let { .. }
        | Statement::Assign { .. }
        | Statement::AssignMember { .. }
        | Statement::Print { .. }
        | Statement::Expression(_)
        | Statement::Throw(_)
        | Statement::Return(_)
        | Statement::Break { .. }
        | Statement::Continue { .. }
        | Statement::Yield { .. }
        | Statement::YieldDelegate { .. } => {}
    }
}

fn statements_reference_this_or_new_target_for_tail_restart(statements: &[Statement]) -> bool {
    statements
        .iter()
        .any(statement_references_this_or_new_target_for_tail_restart)
}

fn statement_references_this_or_new_target_for_tail_restart(statement: &Statement) -> bool {
    match statement {
        Statement::Declaration { body }
        | Statement::Block { body }
        | Statement::Labeled { body, .. }
        | Statement::With { body, .. }
        | Statement::DoWhile { body, .. }
        | Statement::While { body, .. } => {
            statements_reference_this_or_new_target_for_tail_restart(body)
        }
        Statement::Var { value, .. }
        | Statement::Let { value, .. }
        | Statement::Assign { value, .. }
        | Statement::Expression(value)
        | Statement::Throw(value)
        | Statement::Return(value)
        | Statement::Yield { value }
        | Statement::YieldDelegate { value } => {
            expression_references_this_or_new_target_for_tail_restart(value)
        }
        Statement::AssignMember {
            object,
            property,
            value,
        } => {
            expression_references_this_or_new_target_for_tail_restart(object)
                || expression_references_this_or_new_target_for_tail_restart(property)
                || expression_references_this_or_new_target_for_tail_restart(value)
        }
        Statement::Print { values } => values
            .iter()
            .any(expression_references_this_or_new_target_for_tail_restart),
        Statement::If {
            condition,
            then_branch,
            else_branch,
        } => {
            expression_references_this_or_new_target_for_tail_restart(condition)
                || statements_reference_this_or_new_target_for_tail_restart(then_branch)
                || statements_reference_this_or_new_target_for_tail_restart(else_branch)
        }
        Statement::Try {
            body,
            catch_setup,
            catch_body,
            ..
        } => {
            statements_reference_this_or_new_target_for_tail_restart(body)
                || statements_reference_this_or_new_target_for_tail_restart(catch_setup)
                || statements_reference_this_or_new_target_for_tail_restart(catch_body)
        }
        Statement::Switch {
            discriminant,
            cases,
            ..
        } => {
            expression_references_this_or_new_target_for_tail_restart(discriminant)
                || cases.iter().any(|case| {
                    statements_reference_this_or_new_target_for_tail_restart(&case.body)
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
            statements_reference_this_or_new_target_for_tail_restart(init)
                || condition
                    .as_ref()
                    .is_some_and(expression_references_this_or_new_target_for_tail_restart)
                || update
                    .as_ref()
                    .is_some_and(expression_references_this_or_new_target_for_tail_restart)
                || break_hook
                    .as_ref()
                    .is_some_and(expression_references_this_or_new_target_for_tail_restart)
                || statements_reference_this_or_new_target_for_tail_restart(body)
        }
        Statement::Break { .. } | Statement::Continue { .. } => false,
    }
}

fn expression_references_this_or_new_target_for_tail_restart(expression: &Expression) -> bool {
    match expression {
        Expression::This | Expression::NewTarget | Expression::SuperMember { .. } => true,
        Expression::Array(elements) => elements.iter().any(|element| match element {
            ArrayElement::Expression(expression) | ArrayElement::Spread(expression) => {
                expression_references_this_or_new_target_for_tail_restart(expression)
            }
        }),
        Expression::Object(entries) => entries.iter().any(|entry| match entry {
            ObjectEntry::Data { key, value } => {
                expression_references_this_or_new_target_for_tail_restart(key)
                    || expression_references_this_or_new_target_for_tail_restart(value)
            }
            ObjectEntry::Getter { key, getter } => {
                expression_references_this_or_new_target_for_tail_restart(key)
                    || expression_references_this_or_new_target_for_tail_restart(getter)
            }
            ObjectEntry::Setter { key, setter } => {
                expression_references_this_or_new_target_for_tail_restart(key)
                    || expression_references_this_or_new_target_for_tail_restart(setter)
            }
            ObjectEntry::Spread(expression) => {
                expression_references_this_or_new_target_for_tail_restart(expression)
            }
        }),
        Expression::Member { object, property } => {
            expression_references_this_or_new_target_for_tail_restart(object)
                || expression_references_this_or_new_target_for_tail_restart(property)
        }
        Expression::Assign { value, .. }
        | Expression::Await(value)
        | Expression::EnumerateKeys(value)
        | Expression::GetIterator(value)
        | Expression::IteratorClose(value)
        | Expression::Unary {
            expression: value, ..
        } => expression_references_this_or_new_target_for_tail_restart(value),
        Expression::AssignMember {
            object,
            property,
            value,
        } => {
            expression_references_this_or_new_target_for_tail_restart(object)
                || expression_references_this_or_new_target_for_tail_restart(property)
                || expression_references_this_or_new_target_for_tail_restart(value)
        }
        Expression::AssignSuperMember { .. } | Expression::SuperCall { .. } => true,
        Expression::Binary { left, right, .. } => {
            expression_references_this_or_new_target_for_tail_restart(left)
                || expression_references_this_or_new_target_for_tail_restart(right)
        }
        Expression::Conditional {
            condition,
            then_expression,
            else_expression,
        } => {
            expression_references_this_or_new_target_for_tail_restart(condition)
                || expression_references_this_or_new_target_for_tail_restart(then_expression)
                || expression_references_this_or_new_target_for_tail_restart(else_expression)
        }
        Expression::Sequence(expressions) => expressions
            .iter()
            .any(expression_references_this_or_new_target_for_tail_restart),
        Expression::Call { callee, arguments } | Expression::New { callee, arguments } => {
            expression_references_this_or_new_target_for_tail_restart(callee)
                || arguments.iter().any(|argument| {
                    expression_references_this_or_new_target_for_tail_restart(argument.expression())
                })
        }
        Expression::Number(_)
        | Expression::BigInt(_)
        | Expression::String(_)
        | Expression::Bool(_)
        | Expression::Null
        | Expression::Undefined
        | Expression::Identifier(_)
        | Expression::Sent
        | Expression::Update { .. } => false,
    }
}
