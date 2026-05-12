use super::*;

impl<'a> FunctionCompiler<'a> {
    pub(in crate::backend::direct_wasm) fn compile(
        self,
        statements: &[Statement],
    ) -> DirectResult<CompiledFunction> {
        let mut compiler = self;
        compiler.compile_in_current_global_scope(statements)
    }

    fn compile_in_current_global_scope(
        &mut self,
        statements: &[Statement],
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
        if trace {
            eprintln!("function_compile=emit_direct_scope");
        }
        if self
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

    fn should_enable_tail_restart_for_current_function(&self, statements: &[Statement]) -> bool {
        if !self.state.runtime.behavior.allow_return
            || !self.state.parameters.arguments_slots.is_empty()
        {
            return false;
        }

        let Some(function) = self.current_user_function() else {
            return false;
        };
        if function.is_async()
            || function.is_generator()
            || function.has_parameter_defaults()
            || function.has_lowered_pattern_parameters()
            || !function.extra_argument_indices.is_empty()
            || statements_reference_this_or_new_target_for_tail_restart(statements)
        {
            return false;
        }

        let mut callee_names = std::collections::HashSet::new();
        callee_names.insert(function.name.clone());
        if let Some(current_name) = self.current_function_name() {
            callee_names.insert(current_name.to_string());
        }
        if let Some(self_binding) = self
            .current_user_function_declaration()
            .and_then(|declaration| declaration.self_binding.clone())
        {
            callee_names.insert(self_binding);
        }

        statements_contain_self_tail_call(statements, &callee_names, function.params.len())
    }
}

fn statements_contain_self_tail_call(
    statements: &[Statement],
    callee_names: &std::collections::HashSet<String>,
    arity: usize,
) -> bool {
    statements
        .iter()
        .any(|statement| statement_contains_self_tail_call(statement, callee_names, arity))
}

fn statement_contains_self_tail_call(
    statement: &Statement,
    callee_names: &std::collections::HashSet<String>,
    arity: usize,
) -> bool {
    match statement {
        Statement::Return(expression) => {
            expression_is_self_tail_call(expression, callee_names, arity)
        }
        Statement::Declaration { body }
        | Statement::Block { body }
        | Statement::Labeled { body, .. }
        | Statement::With { body, .. }
        | Statement::DoWhile { body, .. }
        | Statement::While { body, .. } => {
            statements_contain_self_tail_call(body, callee_names, arity)
        }
        Statement::If {
            then_branch,
            else_branch,
            ..
        } => {
            statements_contain_self_tail_call(then_branch, callee_names, arity)
                || statements_contain_self_tail_call(else_branch, callee_names, arity)
        }
        Statement::Try {
            body,
            catch_setup,
            catch_body,
            ..
        } => {
            statements_contain_self_tail_call(body, callee_names, arity)
                || statements_contain_self_tail_call(catch_setup, callee_names, arity)
                || statements_contain_self_tail_call(catch_body, callee_names, arity)
        }
        Statement::Switch { cases, .. } => cases
            .iter()
            .any(|case| statements_contain_self_tail_call(&case.body, callee_names, arity)),
        Statement::For { init, body, .. } => {
            statements_contain_self_tail_call(init, callee_names, arity)
                || statements_contain_self_tail_call(body, callee_names, arity)
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

fn expression_is_self_tail_call(
    expression: &Expression,
    callee_names: &std::collections::HashSet<String>,
    arity: usize,
) -> bool {
    let Expression::Call { callee, arguments } = expression else {
        return false;
    };
    matches!(callee.as_ref(), Expression::Identifier(name) if callee_names.contains(name))
        && arguments.len() == arity
        && arguments
            .iter()
            .all(|argument| matches!(argument, CallArgument::Expression(_)))
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
