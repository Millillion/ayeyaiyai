use super::*;

const RUNTIME_CHECK_FAST_STATIC_DEPTH_LIMIT: usize = 16;

fn global_identifier_call_requires_runtime_value(
    compiler: &FunctionCompiler<'_>,
    callee: &Expression,
    callee_name: &str,
    function_binding: &LocalFunctionBinding,
) -> bool {
    let LocalFunctionBinding::User(function_name) = function_binding else {
        return false;
    };
    if compiler
        .user_function(function_name)
        .is_some_and(|user_function| {
            user_function.has_parameter_defaults() || user_function.has_lowered_pattern_parameters()
        })
    {
        return false;
    }
    if callee_name == function_name
        || !(function_name.starts_with("__ayy_fnexpr_")
            || function_name.starts_with("__ayy_arrow_"))
    {
        return false;
    }
    if compiler
        .resolve_function_expression_capture_slots(callee)
        .is_some()
    {
        return false;
    }
    if compiler.current_function_name().is_none()
        && compiler.global_binding_index(callee_name).is_some()
        && !compiler
            .state
            .emission
            .emitted_value_bindings
            .contains(callee_name)
    {
        return true;
    }

    let static_global_binding = compiler
        .global_value_binding(callee_name)
        .and_then(|value| compiler.resolve_function_binding_from_expression(value));
    static_global_binding.as_ref() != Some(function_binding)
}

fn captured_identifier_user_function(
    compiler: &FunctionCompiler<'_>,
    name: &str,
    capture_slots: &BTreeMap<String, String>,
) -> Option<UserFunction> {
    fn internal_name_hint(function_name: &str) -> Option<&str> {
        function_name
            .rsplit_once("__name_")
            .map(|(_, hinted_name)| hinted_name)
            .filter(|hinted_name| !hinted_name.is_empty())
    }

    let source_name = scoped_binding_source_name(name).unwrap_or(name);
    compiler.user_functions().into_iter().find(|user_function| {
        internal_name_hint(&user_function.name)
            .map(|hint| scoped_binding_source_name(hint).unwrap_or(hint) == source_name)
            .unwrap_or(false)
            && compiler
                .user_function_capture_bindings(&user_function.name)
                .is_some_and(|capture_bindings| {
                    !capture_bindings.is_empty()
                        && capture_bindings
                            .keys()
                            .all(|capture_name| capture_slots.contains_key(capture_name))
                })
    })
}

fn user_function_body_contains_throw(compiler: &FunctionCompiler<'_>, function_name: &str) -> bool {
    struct ThrowFinder {
        found: bool,
    }

    impl crate::ir::visit::Visitor for ThrowFinder {
        fn visit_statement(&mut self, statement: &Statement) {
            if self.found {
                return;
            }
            if matches!(statement, Statement::Throw(_)) {
                self.found = true;
                return;
            }
            crate::ir::visit::walk_statement(self, statement);
        }
    }

    let Some(function) = compiler.resolve_registered_function_declaration(function_name) else {
        return false;
    };
    let mut finder = ThrowFinder { found: false };
    for statement in &function.body {
        crate::ir::visit::Visitor::visit_statement(&mut finder, statement);
        if finder.found {
            return true;
        }
    }
    false
}

#[derive(Clone)]
struct DirectRuntimeCheckHelperPlan {
    condition_param_index: usize,
    update_name: String,
    update_value: Expression,
    throw_value: Expression,
    return_value: Expression,
}

#[derive(Clone)]
struct FastNumericRuntimeCheckUpdate {
    global_index: u32,
    update_name: String,
    op: BinaryOp,
    delta: i32,
    tracked_value: Expression,
}

struct SimpleArrayAppendReturnArgumentPlan {
    array_name: String,
    append_param_index: usize,
    return_param_index: usize,
}

fn primitive_side_effect_free_expression(expression: &Expression) -> bool {
    matches!(
        expression,
        Expression::Number(_)
            | Expression::BigInt(_)
            | Expression::String(_)
            | Expression::Bool(_)
            | Expression::Null
            | Expression::Undefined
    )
}

fn direct_runtime_check_helper_plan(
    compiler: &FunctionCompiler<'_>,
    user_function: &UserFunction,
) -> Option<DirectRuntimeCheckHelperPlan> {
    fn single_statement_or_block_statement(statements: &[Statement]) -> Option<&Statement> {
        let [statement] = statements else {
            return None;
        };
        match statement {
            Statement::Block { body } => {
                let [statement] = body.as_slice() else {
                    return None;
                };
                Some(statement)
            }
            _ => Some(statement),
        }
    }

    let function = compiler.resolve_registered_function_declaration(&user_function.name)?;
    if function.params.is_empty() {
        return None;
    };
    let (condition_statement, update_statement, return_value) = match function.body.as_slice() {
        [condition_statement, update_statement] => {
            (condition_statement, update_statement, Expression::Undefined)
        }
        [
            condition_statement,
            update_statement,
            Statement::Return(return_value),
        ] if primitive_side_effect_free_expression(return_value) => {
            (condition_statement, update_statement, return_value.clone())
        }
        _ => return None,
    };

    let Statement::If {
        condition,
        then_branch,
        else_branch,
    } = condition_statement
    else {
        return None;
    };
    if !else_branch.is_empty() || then_branch.len() != 1 {
        return None;
    }
    let then_statement = single_statement_or_block_statement(then_branch)?;
    let Expression::Unary {
        op: UnaryOp::Not,
        expression,
    } = condition
    else {
        return None;
    };
    let Expression::Identifier(condition_name) = expression.as_ref() else {
        return None;
    };
    let condition_param_index = function
        .params
        .iter()
        .position(|parameter| parameter.name == *condition_name)?;

    let Statement::Throw(throw_value) = then_statement else {
        return None;
    };

    let Statement::Assign { name, value } = update_statement else {
        return None;
    };
    let Expression::Binary { op, left, right } = value else {
        return None;
    };
    if !matches!(op, BinaryOp::Add | BinaryOp::Subtract)
        || !matches!(left.as_ref(), Expression::Identifier(left_name) if left_name == name)
        || !primitive_side_effect_free_expression(right)
        || function
            .params
            .iter()
            .any(|parameter| parameter.name == *name)
    {
        return None;
    }

    Some(DirectRuntimeCheckHelperPlan {
        condition_param_index,
        update_name: name.clone(),
        update_value: value.clone(),
        throw_value: throw_value.clone(),
        return_value,
    })
}

impl<'a> FunctionCompiler<'a> {
    fn simple_array_append_return_argument_plan(
        &self,
        user_function: &UserFunction,
    ) -> Option<SimpleArrayAppendReturnArgumentPlan> {
        if user_function.lexical_this
            || user_function.is_async()
            || user_function.is_generator()
            || user_function.has_parameter_defaults()
            || user_function.has_lowered_pattern_parameters()
            || self.user_function_mentions_private_member_access(user_function)
            || self.user_function_mentions_direct_eval(user_function)
        {
            return None;
        }

        let function = self.resolve_registered_function_declaration(&user_function.name)?;
        let [
            Statement::AssignMember {
                object,
                property,
                value,
            },
            Statement::Return(return_value),
        ] = function.body.as_slice()
        else {
            return None;
        };

        let Expression::Identifier(array_name) = object else {
            return None;
        };
        if array_name == "this"
            || array_name == "arguments"
            || user_function.params.iter().any(|param| param == array_name)
            || user_function.scope_bindings.contains(array_name)
        {
            return None;
        }
        let Expression::Member {
            object: length_object,
            property: length_property,
        } = property
        else {
            return None;
        };
        if !matches!(length_object.as_ref(), Expression::Identifier(name) if name == array_name)
            || !matches!(length_property.as_ref(), Expression::String(name) if name == "length")
        {
            return None;
        }
        let Expression::Identifier(append_param_name) = value else {
            return None;
        };
        let Expression::Identifier(return_param_name) = return_value else {
            return None;
        };
        let append_param_index = function
            .params
            .iter()
            .position(|parameter| parameter.name == *append_param_name)?;
        let return_param_index = function
            .params
            .iter()
            .position(|parameter| parameter.name == *return_param_name)?;
        if user_function.params.get(append_param_index) != Some(append_param_name)
            || user_function.params.get(return_param_index) != Some(return_param_name)
        {
            return None;
        }
        if !self
            .state
            .speculation
            .static_semantics
            .has_local_array_binding(array_name)
            && !self
                .backend
                .global_semantics
                .values
                .array_bindings
                .contains_key(array_name)
        {
            return None;
        }

        Some(SimpleArrayAppendReturnArgumentPlan {
            array_name: array_name.clone(),
            append_param_index,
            return_param_index,
        })
    }

    fn emit_simple_array_append_return_argument_call(
        &mut self,
        user_function: &UserFunction,
        arguments: &[CallArgument],
    ) -> DirectResult<bool> {
        let trace = crate::ayy_env_flag!("AYY_TRACE_SIMPLE_ARRAY_APPEND");
        let trace_start = trace.then(std::time::Instant::now);
        let Some(plan) = self.simple_array_append_return_argument_plan(user_function) else {
            if trace {
                eprintln!(
                    "simple_array_append:reject_plan function={}",
                    user_function.name
                );
            }
            return Ok(false);
        };
        if arguments.len() < user_function.params.len()
            || arguments
                .iter()
                .take(user_function.params.len())
                .any(|argument| matches!(argument, CallArgument::Spread(_)))
        {
            if trace {
                eprintln!(
                    "simple_array_append:reject_arguments function={} args={} params={}",
                    user_function.name,
                    arguments.len(),
                    user_function.params.len()
                );
            }
            return Ok(false);
        }
        let Some(CallArgument::Expression(append_source_argument)) =
            arguments.get(plan.append_param_index)
        else {
            return Ok(false);
        };
        let Some(append_static_value) =
            self.simple_array_append_static_primitive_argument_value(append_source_argument)
        else {
            if trace {
                eprintln!(
                    "simple_array_append:reject_append_value function={} argument={append_source_argument:?}",
                    user_function.name
                );
            }
            return Ok(false);
        };
        if arguments.len() == user_function.params.len()
            && arguments.iter().all(|argument| match argument {
                CallArgument::Expression(expression) => {
                    inline_summary_side_effect_free_expression(expression)
                }
                CallArgument::Spread(_) => false,
            })
        {
            let return_argument = match arguments.get(plan.return_param_index) {
                Some(CallArgument::Expression(expression)) => expression.clone(),
                _ => return Ok(false),
            };
            let return_resolution_start = trace.then(std::time::Instant::now);
            let return_static_value =
                self.simple_array_append_static_primitive_argument_value(&return_argument);
            let update_start = trace.then(std::time::Instant::now);
            if !self.emit_simple_array_append_return_argument_static_array_update(
                &plan.array_name,
                &append_static_value,
            )? {
                return Ok(false);
            }
            let return_emit_start = trace.then(std::time::Instant::now);
            if let Some(return_static_value) = return_static_value {
                self.emit_numeric_expression(&return_static_value)?;
            } else {
                self.emit_numeric_expression(&return_argument)?;
            }
            if trace {
                let now = std::time::Instant::now();
                eprintln!(
                    "simple_array_append:accepted_static function={} array={} append={append_static_value:?} total_ms={} return_resolve_ms={} update_ms={} return_emit_ms={}",
                    user_function.name,
                    plan.array_name,
                    trace_start
                        .map(|start| start.elapsed().as_millis())
                        .unwrap_or(0),
                    return_resolution_start
                        .map(|start| update_start
                            .unwrap_or(now)
                            .duration_since(start)
                            .as_millis())
                        .unwrap_or(0),
                    update_start
                        .map(|start| return_emit_start
                            .unwrap_or(now)
                            .duration_since(start)
                            .as_millis())
                        .unwrap_or(0),
                    return_emit_start
                        .map(|start| now.duration_since(start).as_millis())
                        .unwrap_or(0)
                );
            }
            return Ok(true);
        }

        let mut argument_locals = Vec::with_capacity(user_function.params.len());
        for (index, argument) in arguments
            .iter()
            .take(user_function.params.len())
            .enumerate()
        {
            let CallArgument::Expression(argument) = argument else {
                return Ok(false);
            };
            let hidden_name = self.allocate_named_hidden_local(
                &format!("simple_array_append_arg_{index}"),
                self.infer_value_kind(argument)
                    .unwrap_or(StaticValueKind::Unknown),
            );
            let hidden_local = self
                .state
                .runtime
                .locals
                .get(&hidden_name)
                .copied()
                .expect("fresh simple array append argument hidden local must exist");
            self.emit_numeric_expression(argument)?;
            self.push_local_set(hidden_local);
            self.update_capture_slot_binding_from_expression(&hidden_name, argument)?;
            argument_locals.push(hidden_local);
        }
        self.discard_call_arguments(&arguments[user_function.params.len()..])?;

        if !self.emit_simple_array_append_return_argument_array_update(
            &plan.array_name,
            *argument_locals
                .get(plan.append_param_index)
                .expect("validated append argument index"),
            &append_static_value,
        )? {
            return Ok(false);
        }
        self.push_local_get(
            *argument_locals
                .get(plan.return_param_index)
                .expect("validated return argument index"),
        );
        if trace {
            eprintln!(
                "simple_array_append:accepted_locals function={} array={} append={append_static_value:?}",
                user_function.name, plan.array_name
            );
        }
        Ok(true)
    }

    pub(in crate::backend::direct_wasm) fn resolve_simple_array_append_return_argument_static_call_value(
        &self,
        expression: &Expression,
    ) -> Option<Expression> {
        let Expression::Call { callee, arguments } = expression else {
            return None;
        };
        let user_function = self.resolve_user_function_from_expression(callee)?;
        let plan = self.simple_array_append_return_argument_plan(&user_function)?;
        if arguments.len() != user_function.params.len()
            || arguments
                .iter()
                .any(|argument| matches!(argument, CallArgument::Spread(_)))
            || arguments
                .iter()
                .any(|argument| !inline_summary_side_effect_free_expression(argument.expression()))
        {
            return None;
        }
        let return_argument = arguments.get(plan.return_param_index)?.expression();
        self.simple_array_append_static_primitive_argument_value(return_argument)
    }

    fn simple_array_append_static_primitive_argument_value(
        &self,
        expression: &Expression,
    ) -> Option<Expression> {
        if !inline_summary_side_effect_free_expression(expression) {
            return None;
        }
        if let Some(value) = self.runtime_check_fast_static_primitive_expression(expression, 0)
            && primitive_side_effect_free_expression(&value)
        {
            return Some(value);
        }
        let value = self.materialize_static_expression(expression);
        primitive_side_effect_free_expression(&value).then_some(value)
    }

    fn emit_simple_array_append_return_argument_static_array_update(
        &mut self,
        array_name: &str,
        static_value: &Expression,
    ) -> DirectResult<bool> {
        let Some(index) = self.append_simple_array_static_binding_value(array_name, static_value)
        else {
            return Ok(false);
        };

        let use_global_runtime_array = self.is_named_global_array_binding(array_name)
            && (!self.state.speculation.execution_context.top_level_function
                || self.uses_global_runtime_array_state(array_name));
        if use_global_runtime_array {
            if !self.is_named_global_array_binding(array_name) {
                return Ok(false);
            }
            self.backend
                .mark_global_array_with_runtime_state(array_name);
            self.backend
                .shared_global_semantics
                .values
                .mark_array_with_runtime_state(array_name);
            let binding = self.global_runtime_array_slot_binding(array_name, index);
            self.emit_numeric_expression(static_value)?;
            self.push_global_set(binding.value_index);
            self.push_i32_const(1);
            self.push_global_set(binding.present_index);

            let length_binding = self.global_runtime_array_length_binding(array_name);
            let next_length = index as i32 + 1;
            self.push_global_get(length_binding.value_index);
            self.push_i32_const(next_length);
            self.push_binary_op(BinaryOp::LessThan)?;
            self.state.emission.output.instructions.push(0x04);
            self.state
                .emission
                .output
                .instructions
                .push(EMPTY_BLOCK_TYPE);
            self.push_control_frame();
            self.push_i32_const(next_length);
            self.push_global_set(length_binding.value_index);
            self.push_i32_const(1);
            self.push_global_set(length_binding.present_index);
            self.state.emission.output.instructions.push(0x0b);
            self.pop_control_frame();
        } else {
            let slot = self.ensure_runtime_array_slot_entry(array_name, index);
            self.emit_numeric_expression(static_value)?;
            self.push_local_set(slot.value_local);
            self.push_i32_const(1);
            self.push_local_set(slot.present_local);
            if let Some(length_local) = self
                .state
                .speculation
                .static_semantics
                .runtime_array_length_local(array_name)
            {
                let next_length = index as i32 + 1;
                self.push_local_get(length_local);
                self.push_i32_const(next_length);
                self.push_binary_op(BinaryOp::LessThan)?;
                self.state.emission.output.instructions.push(0x04);
                self.state
                    .emission
                    .output
                    .instructions
                    .push(EMPTY_BLOCK_TYPE);
                self.push_control_frame();
                self.push_i32_const(next_length);
                self.push_local_set(length_local);
                self.state.emission.output.instructions.push(0x0b);
                self.pop_control_frame();
            }
        }
        Ok(true)
    }

    fn emit_simple_array_append_return_argument_array_update(
        &mut self,
        array_name: &str,
        value_local: u32,
        static_value: &Expression,
    ) -> DirectResult<bool> {
        let Some(index) = self.append_simple_array_static_binding_value(array_name, static_value)
        else {
            return Ok(false);
        };
        let use_global_runtime_array = self.is_named_global_array_binding(array_name)
            && (!self.state.speculation.execution_context.top_level_function
                || self.uses_global_runtime_array_state(array_name));
        if use_global_runtime_array {
            if self.emit_global_runtime_array_slot_write_from_local(
                array_name,
                index,
                value_local,
            )? {
                self.state.emission.output.instructions.push(0x1a);
            }
        } else {
            self.ensure_runtime_array_slot_entry(array_name, index);
            if self.emit_runtime_array_slot_write_from_local(array_name, index, value_local)? {
                self.state.emission.output.instructions.push(0x1a);
            }
        }
        Ok(true)
    }

    fn append_simple_array_static_binding_value(
        &mut self,
        array_name: &str,
        static_value: &Expression,
    ) -> Option<u32> {
        if let Some(array_binding) = self
            .state
            .speculation
            .static_semantics
            .local_array_binding_mut(array_name)
        {
            let index = array_binding.values.len() as u32;
            array_binding.values.push(Some(static_value.clone()));
            crate::backend::direct_wasm::memo::bump_static_state_generation();
            return Some(index);
        }
        if let Some(array_binding) = self
            .backend
            .global_semantics
            .values
            .array_bindings
            .get_mut(array_name)
        {
            let index = array_binding.values.len() as u32;
            array_binding.values.push(Some(static_value.clone()));
            crate::backend::direct_wasm::memo::bump_static_state_generation();
            return Some(index);
        }
        None
    }

    fn runtime_check_condition_is_static_true_without_observable_effects(
        &self,
        condition: &Expression,
    ) -> bool {
        if !self.runtime_check_static_true_probe_is_cheap(condition, 0) {
            return false;
        }
        if self.runtime_check_fast_static_truthy_condition(condition, 0) == Some(true) {
            return true;
        }
        if !self.runtime_check_expression_is_observably_side_effect_free(condition, 0) {
            return false;
        }
        if self.resolve_static_boolean_expression(condition) == Some(true) {
            return true;
        }
        false
    }

    fn runtime_check_fast_static_truthy_condition(
        &self,
        expression: &Expression,
        depth: usize,
    ) -> Option<bool> {
        if depth > RUNTIME_CHECK_FAST_STATIC_DEPTH_LIMIT {
            return None;
        }
        match expression {
            Expression::Bool(value) => Some(*value),
            Expression::Binary {
                op: BinaryOp::LogicalAnd,
                left,
                right,
            } => {
                let left_truthy =
                    self.runtime_check_fast_static_truthy_condition(left, depth + 1)?;
                if !left_truthy {
                    return Some(false);
                }
                self.runtime_check_fast_static_truthy_condition(right, depth + 1)
            }
            Expression::Binary {
                op: BinaryOp::LogicalOr,
                left,
                right,
            } => {
                let left_truthy =
                    self.runtime_check_fast_static_truthy_condition(left, depth + 1)?;
                if left_truthy {
                    return Some(true);
                }
                self.runtime_check_fast_static_truthy_condition(right, depth + 1)
            }
            Expression::Binary {
                op: op @ (BinaryOp::Equal | BinaryOp::NotEqual),
                left,
                right,
            } => self.runtime_check_fast_static_equality(op, left, right, depth + 1),
            _ => self
                .runtime_check_fast_static_primitive_expression(expression, depth + 1)
                .and_then(|value| self.runtime_check_fast_static_primitive_truthy(&value)),
        }
    }

    fn runtime_check_fast_static_equality(
        &self,
        op: &BinaryOp,
        left: &Expression,
        right: &Expression,
        depth: usize,
    ) -> Option<bool> {
        if let (Some(left_value), Some(right_value)) = (
            self.runtime_check_fast_static_primitive_expression(left, depth + 1),
            self.runtime_check_fast_static_primitive_expression(right, depth + 1),
        ) {
            let equal = match (&left_value, &right_value) {
                (Expression::Number(left), Expression::Number(right)) => left == right,
                (Expression::String(left), Expression::String(right)) => left == right,
                (Expression::Bool(left), Expression::Bool(right)) => left == right,
                (Expression::Null, Expression::Null)
                | (Expression::Undefined, Expression::Undefined) => true,
                _ => false,
            };
            return Some(equal ^ matches!(op, BinaryOp::NotEqual));
        }

        let left_key = self.runtime_check_fast_object_identity_key(left, depth + 1);
        let right_key = self.runtime_check_fast_object_identity_key(right, depth + 1);
        if let (Some(left_key), Some(right_key)) = (left_key, right_key) {
            return Some((left_key == right_key) ^ matches!(op, BinaryOp::NotEqual));
        }
        None
    }

    fn runtime_check_fast_static_primitive_truthy(&self, value: &Expression) -> Option<bool> {
        match value {
            Expression::Bool(value) => Some(*value),
            Expression::Null | Expression::Undefined => Some(false),
            Expression::Number(value) => Some(*value != 0.0 && !value.is_nan()),
            Expression::String(value) => Some(!value.is_empty()),
            _ => None,
        }
    }

    fn runtime_check_fast_static_primitive_expression(
        &self,
        expression: &Expression,
        depth: usize,
    ) -> Option<Expression> {
        if depth > RUNTIME_CHECK_FAST_STATIC_DEPTH_LIMIT {
            return None;
        }
        match expression {
            Expression::Number(_)
            | Expression::String(_)
            | Expression::Bool(_)
            | Expression::Null
            | Expression::Undefined => Some(expression.clone()),
            Expression::Identifier(name)
                if name == "undefined" && self.is_unshadowed_builtin_identifier(name) =>
            {
                Some(Expression::Undefined)
            }
            Expression::Identifier(name) => {
                self.runtime_check_fast_identifier_primitive_expression(name, depth + 1)
            }
            Expression::Binary {
                op: BinaryOp::Add | BinaryOp::Subtract,
                left,
                right,
            } => {
                let Expression::Number(left) =
                    self.runtime_check_fast_static_primitive_expression(left, depth + 1)?
                else {
                    return None;
                };
                let Expression::Number(right) =
                    self.runtime_check_fast_static_primitive_expression(right, depth + 1)?
                else {
                    return None;
                };
                Some(Expression::Number(match expression {
                    Expression::Binary {
                        op: BinaryOp::Add, ..
                    } => left + right,
                    _ => left - right,
                }))
            }
            Expression::Member { object, property } => self
                .runtime_check_fast_member_value(object, property, depth + 1)
                .and_then(|value| {
                    self.runtime_check_fast_static_primitive_expression(&value, depth + 1)
                }),
            Expression::Call { callee, arguments } => self
                .resolve_effectful_call_return_metadata_value(callee, arguments)
                .and_then(|value| {
                    self.runtime_check_fast_static_primitive_expression(&value, depth + 1)
                })
                .or_else(|| {
                    self.call_expression_static_member_number_shortcut_value(expression)
                        .map(Expression::Number)
                }),
            _ => None,
        }
    }

    fn runtime_check_fast_identifier_primitive_expression(
        &self,
        name: &str,
        depth: usize,
    ) -> Option<Expression> {
        if depth > RUNTIME_CHECK_FAST_STATIC_DEPTH_LIMIT
            || self.with_scope_blocks_static_identifier_resolution(name)
        {
            return None;
        }

        let identifier = Expression::Identifier(name.to_string());
        let resolved_name = self
            .resolve_current_local_binding(name)
            .map(|(resolved_name, _)| resolved_name);
        if self.local_lexical_initialized_local(name).is_some()
            || resolved_name.as_deref().is_some_and(|resolved_name| {
                self.local_lexical_initialized_local(resolved_name)
                    .is_some()
            })
            || (resolved_name.is_none() && self.backend.lexical_global_binding(name).is_some())
        {
            return None;
        }

        let value = resolved_name
            .as_deref()
            .and_then(|resolved_name| {
                self.state
                    .speculation
                    .static_semantics
                    .local_value_binding(resolved_name)
            })
            .or_else(|| {
                self.state
                    .speculation
                    .static_semantics
                    .local_value_binding(name)
            })
            .or_else(|| self.global_value_binding(name))?;

        if static_expression_matches(value, &identifier) {
            return None;
        }

        match value {
            Expression::Number(_)
            | Expression::String(_)
            | Expression::Bool(_)
            | Expression::BigInt(_)
            | Expression::Null
            | Expression::Undefined => Some(value.clone()),
            Expression::Identifier(alias) if alias != name => {
                self.runtime_check_fast_identifier_primitive_expression(alias, depth + 1)
            }
            Expression::Unary { .. }
            | Expression::Binary { .. }
            | Expression::Conditional { .. }
            | Expression::Sequence(_) => {
                self.runtime_check_fast_static_primitive_expression(value, depth + 1)
            }
            _ => None,
        }
    }

    fn runtime_check_fast_object_identity_key(
        &self,
        expression: &Expression,
        depth: usize,
    ) -> Option<String> {
        if depth > RUNTIME_CHECK_FAST_STATIC_DEPTH_LIMIT {
            return None;
        }
        match expression {
            Expression::Identifier(name) => self
                .runtime_check_direct_object_binding_for_identifier(name)
                .map(|_| format!("identifier:{name}")),
            Expression::Member { object, property } => {
                let property_name =
                    Self::runtime_check_fast_side_effect_free_property_name(property)?;
                let value = self.runtime_check_fast_member_value(object, property, depth + 1)?;
                match value {
                    Expression::Object(_) => self
                        .runtime_check_fast_object_identity_key(object, depth + 1)
                        .map(|object_key| format!("{object_key}.{property_name}")),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    fn runtime_check_direct_object_binding_for_identifier(
        &self,
        name: &str,
    ) -> Option<ObjectValueBinding> {
        let resolved_name = self
            .resolve_current_local_binding(name)
            .map(|(resolved_name, _)| resolved_name)
            .unwrap_or_else(|| name.to_string());
        self.state
            .speculation
            .static_semantics
            .local_object_binding(&resolved_name)
            .or_else(|| {
                self.state
                    .speculation
                    .static_semantics
                    .local_object_binding(name)
            })
            .cloned()
            .or_else(|| self.global_object_binding(name).cloned())
    }

    fn runtime_check_fast_member_value(
        &self,
        object: &Expression,
        property: &Expression,
        depth: usize,
    ) -> Option<Expression> {
        if depth > RUNTIME_CHECK_FAST_STATIC_DEPTH_LIMIT {
            return None;
        }
        let property = self.runtime_check_fast_member_property_expression(property, depth + 1)?;
        let trace_member = crate::ayy_env_flag!("AYY_TRACE_RUNTIME_CHECK_MEMBER");
        if trace_member {
            eprintln!("runtime_check_member:start object={object:?} property={property:?}");
        }
        if self.runtime_object_property_shadow_deletion_may_affect_property(object, &property) {
            if trace_member {
                eprintln!(
                    "runtime_check_member:reject_shadow_delete object={object:?} property={property:?}"
                );
            }
            return None;
        }
        match object {
            Expression::Identifier(name) => {
                let result = self
                    .runtime_check_direct_object_binding_for_identifier(name)
                    .and_then(|binding| {
                        if let Some(descriptor) =
                            object_binding_lookup_descriptor(&binding, &property)
                        {
                            if descriptor.getter.is_some() || descriptor.has_get {
                                return self.runtime_check_fast_getter_member_value(
                                    object,
                                    &property,
                                    depth + 1,
                                );
                            }
                            if let Some(value) = descriptor.value.as_ref() {
                                return Some(value.clone());
                            }
                            if descriptor.setter.is_some() || descriptor.has_set {
                                return Some(Expression::Undefined);
                            }
                        }
                        object_binding_lookup_value(&binding, &property)
                            .cloned()
                            .or_else(|| {
                                self.runtime_check_fast_inherited_member_value(
                                    object,
                                    &property,
                                    depth + 1,
                                )
                            })
                            .or_else(|| {
                                self.runtime_check_fast_getter_member_value(
                                    object,
                                    &property,
                                    depth + 1,
                                )
                            })
                    });
                if trace_member {
                    eprintln!(
                        "runtime_check_member:identifier object={object:?} property={property:?} result={result:?}"
                    );
                }
                result
            }
            Expression::Object(entries) => {
                Self::runtime_check_fast_object_literal_property_value(entries, &property)
            }
            Expression::Member {
                object: parent_object,
                property: parent_property,
            } => {
                let parent_value = self.runtime_check_fast_member_value(
                    parent_object,
                    parent_property,
                    depth + 1,
                )?;
                match parent_value {
                    Expression::Object(entries) => {
                        Self::runtime_check_fast_object_literal_property_value(&entries, &property)
                    }
                    _ => None,
                }
            }
            _ => None,
        }
    }

    fn runtime_check_fast_member_property_expression(
        &self,
        property: &Expression,
        depth: usize,
    ) -> Option<Expression> {
        if depth > RUNTIME_CHECK_FAST_STATIC_DEPTH_LIMIT {
            return None;
        }
        if let Some(property_name) = static_property_name_from_expression(property) {
            return Some(Expression::String(property_name));
        }
        if let Expression::Identifier(name) = property {
            let resolved_name = self
                .resolve_current_local_binding(name)
                .map(|(resolved_name, _)| resolved_name);
            let value = resolved_name
                .as_deref()
                .and_then(|resolved_name| {
                    self.state
                        .speculation
                        .static_semantics
                        .local_value_binding(resolved_name)
                })
                .or_else(|| {
                    self.state
                        .speculation
                        .static_semantics
                        .local_value_binding(name)
                })
                .or_else(|| self.global_value_binding(name));
            if let Some(value) = value
                && !static_expression_matches(value, property)
            {
                return self.runtime_check_fast_member_property_expression(value, depth + 1);
            }
        }
        if let Expression::Call { callee, arguments } = property
            && let Some(value) =
                self.resolve_effectful_call_return_metadata_value(callee, arguments)
            && !static_expression_matches(&value, property)
        {
            return self.runtime_check_fast_member_property_expression(&value, depth + 1);
        }
        if let Some(resolved) = self
            .resolve_bound_alias_expression(property)
            .filter(|resolved| !static_expression_matches(resolved, property))
        {
            return self.runtime_check_fast_member_property_expression(&resolved, depth + 1);
        }
        let materialized = self.materialize_static_expression(property);
        if static_expression_matches(&materialized, property) {
            return None;
        }
        self.runtime_check_fast_member_property_expression(&materialized, depth + 1)
    }

    fn runtime_check_fast_inherited_member_value(
        &self,
        object: &Expression,
        property: &Expression,
        depth: usize,
    ) -> Option<Expression> {
        if depth > RUNTIME_CHECK_FAST_STATIC_DEPTH_LIMIT {
            return None;
        }
        let mut prototype = self.resolve_static_object_prototype_expression(object)?;
        for _ in 0..8 {
            if matches!(prototype, Expression::Null | Expression::Undefined) {
                return None;
            }
            if self
                .runtime_object_property_shadow_deletion_may_affect_property(&prototype, property)
            {
                return None;
            }
            if let Some(binding) = self.resolve_object_binding_from_expression(&prototype) {
                if let Some(value) = object_binding_lookup_value(&binding, property) {
                    return Some(value.clone());
                }
                if object_binding_lookup_descriptor(&binding, property).is_some() {
                    return None;
                }
            }
            let materialized_prototype = self.materialize_static_expression(&prototype);
            let next_prototype = self
                .resolve_static_object_prototype_expression(&materialized_prototype)
                .or_else(|| self.resolve_static_object_prototype_expression(&prototype))?;
            if static_expression_matches(&next_prototype, &prototype)
                || static_expression_matches(&next_prototype, &materialized_prototype)
            {
                return None;
            }
            prototype = next_prototype;
        }
        None
    }

    fn runtime_check_fast_getter_member_value(
        &self,
        object: &Expression,
        property: &Expression,
        depth: usize,
    ) -> Option<Expression> {
        if depth > RUNTIME_CHECK_FAST_STATIC_DEPTH_LIMIT {
            return None;
        }
        let getter_binding = self.resolve_member_getter_binding(object, property)?;
        let static_this_expression = self.resolve_static_snapshot_this_expression(object);
        if let Some(value) =
            self.runtime_check_fast_simple_getter_member_value(&getter_binding, object, depth + 1)
        {
            return Some(value);
        }
        self.resolve_static_getter_value_from_binding_with_context(
            &getter_binding,
            &static_this_expression,
            self.current_function_name(),
        )
    }

    fn runtime_check_fast_simple_getter_member_value(
        &self,
        getter_binding: &LocalFunctionBinding,
        this_binding: &Expression,
        depth: usize,
    ) -> Option<Expression> {
        if depth > RUNTIME_CHECK_FAST_STATIC_DEPTH_LIMIT {
            return None;
        }
        let LocalFunctionBinding::User(function_name) = getter_binding else {
            return None;
        };
        let user_function = self.user_function(function_name)?;
        if user_function.is_async()
            || user_function.is_generator()
            || user_function.has_parameter_defaults()
            || user_function.has_lowered_pattern_parameters()
            || self.user_function_mentions_private_member_access(user_function)
            || self.user_function_mentions_direct_eval(user_function)
            || !user_function.params.is_empty()
        {
            return None;
        }
        let function = self.resolve_registered_function_declaration(function_name)?;
        let [Statement::Return(return_value)] = function.body.as_slice() else {
            return None;
        };
        let arguments_binding = Expression::Array(Vec::new());
        let substituted = self.substitute_user_function_call_frame_bindings(
            return_value,
            user_function,
            &[],
            this_binding,
            &arguments_binding,
        );
        let side_effect_free =
            self.runtime_check_expression_is_observably_side_effect_free(&substituted, depth + 1);
        if crate::ayy_env_flag!("AYY_TRACE_RUNTIME_CHECK_GETTER") {
            eprintln!(
                "runtime_check_getter function={function_name} this={this_binding:?} return={return_value:?} substituted={substituted:?} side_effect_free={side_effect_free}"
            );
        }
        side_effect_free.then_some(substituted)
    }

    fn runtime_check_fast_object_literal_property_value(
        entries: &[ObjectEntry],
        property: &Expression,
    ) -> Option<Expression> {
        let property_name = Self::runtime_check_fast_side_effect_free_property_name(property)?;
        entries.iter().rev().find_map(|entry| {
            let ObjectEntry::Data { key, value } = entry else {
                return None;
            };
            let key_name = Self::runtime_check_fast_side_effect_free_property_name(key)?;
            (key_name == property_name).then(|| value.clone())
        })
    }

    fn runtime_check_fast_side_effect_free_property_name(
        expression: &Expression,
    ) -> Option<String> {
        match expression {
            Expression::String(text) => Some(text.clone()),
            Expression::Bool(value) => Some(value.to_string()),
            Expression::BigInt(value) => Some(value.clone()),
            Expression::Null => Some("null".to_string()),
            Expression::Undefined => Some("undefined".to_string()),
            Expression::Number(value) => Some(js_number_property_name(*value)),
            _ => None,
        }
    }

    fn runtime_check_static_true_probe_is_cheap(
        &self,
        expression: &Expression,
        depth: usize,
    ) -> bool {
        if depth > RUNTIME_CHECK_FAST_STATIC_DEPTH_LIMIT {
            return false;
        }
        match expression {
            Expression::Number(_)
            | Expression::BigInt(_)
            | Expression::String(_)
            | Expression::Bool(_)
            | Expression::Null
            | Expression::Undefined
            | Expression::Identifier(_)
            | Expression::This
            | Expression::NewTarget
            | Expression::Sent => true,
            Expression::Unary {
                op: UnaryOp::Delete,
                ..
            }
            | Expression::SuperMember { .. }
            | Expression::Assign { .. }
            | Expression::AssignMember { .. }
            | Expression::AssignSuperMember { .. }
            | Expression::SuperCall { .. }
            | Expression::New { .. }
            | Expression::Update { .. }
            | Expression::Await(_)
            | Expression::EnumerateKeys(_)
            | Expression::GetIterator(_)
            | Expression::IteratorClose(_)
            | Expression::Array(_)
            | Expression::Object(_) => false,
            Expression::Member { object, property } => self
                .runtime_check_fast_member_value(object, property, depth + 1)
                .is_some(),
            Expression::Call { .. } => self
                .call_expression_static_member_number_shortcut_value(expression)
                .is_some(),
            Expression::Unary { expression, .. } => {
                self.runtime_check_static_true_probe_is_cheap(expression, depth + 1)
            }
            Expression::Binary { left, right, .. } => {
                self.runtime_check_static_true_probe_is_cheap(left, depth + 1)
                    && self.runtime_check_static_true_probe_is_cheap(right, depth + 1)
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                self.runtime_check_static_true_probe_is_cheap(condition, depth + 1)
                    && self.runtime_check_static_true_probe_is_cheap(then_expression, depth + 1)
                    && self.runtime_check_static_true_probe_is_cheap(else_expression, depth + 1)
            }
            Expression::Sequence(expressions) => expressions.iter().all(|expression| {
                self.runtime_check_static_true_probe_is_cheap(expression, depth + 1)
            }),
        }
    }

    fn runtime_check_expression_is_observably_side_effect_free(
        &self,
        expression: &Expression,
        depth: usize,
    ) -> bool {
        if depth > RUNTIME_CHECK_FAST_STATIC_DEPTH_LIMIT {
            return false;
        }
        match expression {
            Expression::Number(_)
            | Expression::BigInt(_)
            | Expression::String(_)
            | Expression::Bool(_)
            | Expression::Null
            | Expression::Undefined
            | Expression::Identifier(_)
            | Expression::This
            | Expression::NewTarget
            | Expression::Sent => true,
            Expression::Member { object, property } => {
                if self
                    .runtime_check_fast_member_value(object, property, depth + 1)
                    .is_some()
                {
                    return true;
                }
                self.runtime_check_expression_is_observably_side_effect_free(object, depth + 1)
                    && self.runtime_check_expression_is_observably_side_effect_free(
                        property,
                        depth + 1,
                    )
                    && self
                        .resolve_member_getter_binding(object, property)
                        .is_none()
            }
            Expression::Unary {
                op: UnaryOp::Delete,
                ..
            }
            | Expression::Assign { .. }
            | Expression::AssignMember { .. }
            | Expression::AssignSuperMember { .. }
            | Expression::SuperCall { .. }
            | Expression::New { .. }
            | Expression::Update { .. }
            | Expression::Await(_)
            | Expression::EnumerateKeys(_)
            | Expression::GetIterator(_)
            | Expression::IteratorClose(_) => false,
            Expression::Unary { expression, .. }
            | Expression::SuperMember {
                property: expression,
            } => {
                self.runtime_check_expression_is_observably_side_effect_free(expression, depth + 1)
            }
            Expression::Binary { left, right, .. } => {
                self.runtime_check_expression_is_observably_side_effect_free(left, depth + 1)
                    && self
                        .runtime_check_expression_is_observably_side_effect_free(right, depth + 1)
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                self.runtime_check_expression_is_observably_side_effect_free(condition, depth + 1)
                    && self.runtime_check_expression_is_observably_side_effect_free(
                        then_expression,
                        depth + 1,
                    )
                    && self.runtime_check_expression_is_observably_side_effect_free(
                        else_expression,
                        depth + 1,
                    )
            }
            Expression::Sequence(expressions) => expressions.iter().all(|expression| {
                self.runtime_check_expression_is_observably_side_effect_free(expression, depth + 1)
            }),
            Expression::Array(elements) => elements.iter().all(|element| {
                let expression = match element {
                    ArrayElement::Expression(expression) | ArrayElement::Spread(expression) => {
                        expression
                    }
                };
                self.runtime_check_expression_is_observably_side_effect_free(expression, depth + 1)
            }),
            Expression::Object(entries) => entries.iter().all(|entry| match entry {
                ObjectEntry::Data { key, value } => {
                    self.runtime_check_expression_is_observably_side_effect_free(key, depth + 1)
                        && self.runtime_check_expression_is_observably_side_effect_free(
                            value,
                            depth + 1,
                        )
                }
                ObjectEntry::Getter { .. } | ObjectEntry::Setter { .. } => false,
                ObjectEntry::Spread(expression) => self
                    .runtime_check_expression_is_observably_side_effect_free(expression, depth + 1),
            }),
            Expression::Call { callee, arguments } => {
                self.runtime_check_call_is_observably_side_effect_free(callee, arguments, depth + 1)
            }
        }
    }

    fn runtime_check_call_is_observably_side_effect_free(
        &self,
        callee: &Expression,
        arguments: &[CallArgument],
        depth: usize,
    ) -> bool {
        if arguments.iter().any(|argument| {
            !self.runtime_check_expression_is_observably_side_effect_free(
                argument.expression(),
                depth + 1,
            )
        }) {
            return false;
        }

        if let Expression::Member { object, property } = callee
            && matches!(property.as_ref(), Expression::String(name) if name == "call" || name == "apply")
        {
            return self
                .resolve_user_function_from_expression(object)
                .is_some_and(|user_function| {
                    self.runtime_check_user_function_call_is_observably_side_effect_free(
                        &user_function,
                    )
                });
        }

        self.resolve_user_function_from_expression(callee)
            .is_some_and(|user_function| {
                self.runtime_check_user_function_call_is_observably_side_effect_free(&user_function)
            })
    }

    fn runtime_check_user_function_call_is_observably_side_effect_free(
        &self,
        user_function: &UserFunction,
    ) -> bool {
        if user_function.is_async()
            || user_function.is_generator()
            || user_function.has_parameter_defaults()
            || user_function.has_lowered_pattern_parameters()
            || self.user_function_mentions_private_member_access(user_function)
            || self.user_function_mentions_direct_eval(user_function)
        {
            return false;
        }
        self.resolve_registered_function_declaration(&user_function.name)
            .is_some_and(|function| {
                matches!(
                    function.body.as_slice(),
                    [Statement::Return(Expression::This)]
                        | [Statement::Return(Expression::Undefined)]
                        | [Statement::Return(Expression::Null)]
                        | [Statement::Return(Expression::Bool(_))]
                        | [Statement::Return(Expression::Number(_))]
                        | [Statement::Return(Expression::String(_))]
                )
            })
    }

    fn runtime_check_helper_numeric_self_update_value(
        &self,
        name: &str,
        update_value: &Expression,
    ) -> Option<Expression> {
        let Expression::Binary { op, left, right } = update_value else {
            return None;
        };
        let Expression::Identifier(left_name) = left.as_ref() else {
            return None;
        };
        if left_name != name {
            return None;
        }
        let Expression::Number(delta) = right.as_ref() else {
            return None;
        };
        let current = self.static_number_value_for_runtime_check_helper_binding(name)?;
        let next = match op {
            BinaryOp::Add => current + delta,
            BinaryOp::Subtract => current - delta,
            _ => return None,
        };
        Some(Expression::Number(next))
    }

    fn static_number_value_for_runtime_check_helper_binding(&self, name: &str) -> Option<f64> {
        let resolved_name = self
            .resolve_current_local_binding(name)
            .map(|(resolved_name, _)| resolved_name);
        resolved_name
            .as_deref()
            .and_then(|resolved_name| {
                self.state
                    .speculation
                    .static_semantics
                    .local_value_binding(resolved_name)
            })
            .or_else(|| {
                self.state
                    .speculation
                    .static_semantics
                    .local_value_binding(name)
            })
            .or_else(|| self.global_value_binding(name))
            .and_then(|value| match value {
                Expression::Number(value) => Some(*value),
                _ => None,
            })
    }

    fn sync_runtime_check_helper_update_metadata(&mut self, name: &str, value: Option<Expression>) {
        let Some(value) = value else {
            self.clear_global_binding_state(name);
            self.backend
                .shared_global_semantics
                .clear_global_binding_state(name);
            self.state
                .speculation
                .static_semantics
                .clear_local_value_binding(name);
            return;
        };

        if let Some((resolved_name, _)) = self.resolve_current_local_binding(name) {
            self.update_local_value_binding(&resolved_name, &value);
            self.state
                .speculation
                .static_semantics
                .set_local_kind(&resolved_name, StaticValueKind::Number);
            if resolved_name != name {
                self.update_local_value_binding(name, &value);
                self.state
                    .speculation
                    .static_semantics
                    .set_local_kind(name, StaticValueKind::Number);
            }
        } else if let Expression::Number(number) = value {
            self.sync_runtime_check_helper_global_number_metadata(name, number);
        } else {
            self.update_static_global_assignment_metadata(name, &value);
        }
    }

    fn sync_runtime_check_helper_global_number_metadata(&mut self, name: &str, number: f64) {
        let value = Expression::Number(number);
        self.backend
            .set_global_binding_kind(name, StaticValueKind::Number);
        self.backend
            .shared_global_semantics
            .set_global_binding_kind(name, StaticValueKind::Number);
        self.backend
            .sync_global_expression_binding(name, Some(value.clone()));
        self.backend
            .shared_global_semantics
            .values
            .set_value_binding(name.to_string(), value);
        if self.backend.global_array_binding(name).is_some() {
            self.backend.sync_global_array_binding(name, None);
            self.backend
                .shared_global_semantics
                .values
                .sync_array_binding(name, None);
        }
        if self.backend.global_object_binding(name).is_some() {
            self.backend.sync_global_object_binding(name, None);
            self.backend
                .shared_global_semantics
                .values
                .sync_object_binding(name, None);
        }
        if self.backend.global_arguments_binding(name).is_some()
            || self
                .backend
                .shared_global_semantics
                .values
                .arguments_binding(name)
                .is_some()
        {
            self.backend.sync_global_arguments_binding(name, None);
            self.backend
                .shared_global_semantics
                .values
                .sync_arguments_binding(name, None);
        }
        if self.backend.global_function_binding(name).is_some()
            || self
                .backend
                .shared_global_semantics
                .global_functions()
                .function_binding(name)
                .is_some()
        {
            self.backend.sync_global_function_binding(name, None);
            self.backend
                .shared_global_semantics
                .clear_global_function_binding(name);
        }
    }

    fn emit_runtime_check_helper_update(
        &mut self,
        plan: &DirectRuntimeCheckHelperPlan,
        fast_numeric_update: Option<&FastNumericRuntimeCheckUpdate>,
    ) -> DirectResult<()> {
        if let Some(fast_numeric_update) = fast_numeric_update {
            self.emit_runtime_check_helper_fast_numeric_global_update(fast_numeric_update)?;
            return Ok(());
        }

        let tracked_update_value = self
            .runtime_check_helper_numeric_self_update_value(&plan.update_name, &plan.update_value);
        let value_local = self.allocate_temp_local();
        self.emit_numeric_expression(&plan.update_value)?;
        self.push_local_set(value_local);
        self.emit_store_identifier_from_local(&plan.update_name, value_local)?;
        self.sync_runtime_check_helper_update_metadata(&plan.update_name, tracked_update_value);
        Ok(())
    }

    fn emit_runtime_check_helper_fast_numeric_global_update(
        &mut self,
        fast_update: &FastNumericRuntimeCheckUpdate,
    ) -> DirectResult<()> {
        self.push_global_get(fast_update.global_index);
        self.push_i32_const(fast_update.delta);
        self.push_binary_op(fast_update.op.clone())?;
        self.push_global_set(fast_update.global_index);
        self.state
            .emission
            .emitted_value_bindings
            .insert(fast_update.update_name.clone());
        self.sync_runtime_check_helper_update_metadata(
            &fast_update.update_name,
            Some(fast_update.tracked_value.clone()),
        );
        Ok(())
    }

    fn plan_runtime_check_helper_fast_numeric_global_update(
        &self,
        plan: &DirectRuntimeCheckHelperPlan,
        arguments: &[CallArgument],
    ) -> Option<FastNumericRuntimeCheckUpdate> {
        if !self.runtime_check_arguments_preserve_update_binding(arguments, &plan.update_name) {
            return None;
        }
        self.plan_runtime_check_helper_fast_numeric_global_update_without_argument_scan(plan)
    }

    fn plan_runtime_check_helper_fast_numeric_global_update_without_argument_scan(
        &self,
        plan: &DirectRuntimeCheckHelperPlan,
    ) -> Option<FastNumericRuntimeCheckUpdate> {
        let Expression::Binary { op, left, right } = &plan.update_value else {
            return None;
        };
        let Expression::Identifier(left_name) = left.as_ref() else {
            return None;
        };
        if left_name != &plan.update_name {
            return None;
        }
        let Expression::Number(delta) = right.as_ref() else {
            return None;
        };
        let Some(current) =
            self.static_number_value_for_runtime_check_helper_binding(&plan.update_name)
        else {
            return None;
        };
        let next = match op {
            BinaryOp::Add => current + delta,
            BinaryOp::Subtract => current - delta,
            _ => return None,
        };
        let finite_integer = |value: f64| {
            value.is_finite()
                && value.fract() == 0.0
                && value >= i32::MIN as f64
                && value <= i32::MAX as f64
        };
        if !finite_integer(current) || !finite_integer(*delta) || !finite_integer(next) {
            return None;
        }
        if self
            .resolve_current_local_binding(&plan.update_name)
            .is_some()
            || self
                .backend
                .lexical_global_binding(&plan.update_name)
                .is_some()
        {
            return None;
        }
        let Some(global_index) = self.backend.global_binding_index(&plan.update_name) else {
            return None;
        };

        Some(FastNumericRuntimeCheckUpdate {
            global_index,
            update_name: plan.update_name.clone(),
            op: op.clone(),
            delta: *delta as i32,
            tracked_value: Expression::Number(next),
        })
    }

    fn runtime_check_arguments_preserve_update_binding(
        &self,
        arguments: &[CallArgument],
        update_name: &str,
    ) -> bool {
        arguments.iter().all(|argument| {
            !self.runtime_check_expression_may_write_binding(argument.expression(), update_name, 0)
        })
    }

    fn runtime_check_expression_may_write_binding(
        &self,
        expression: &Expression,
        name: &str,
        depth: usize,
    ) -> bool {
        if depth > 12 {
            return true;
        }

        let source_name = scoped_binding_source_name(name).unwrap_or(name);
        let mut assigned_names = HashSet::new();
        collect_assigned_binding_names_from_expression(expression, &mut assigned_names);
        if assigned_names.iter().any(|assigned| {
            let assigned_source = scoped_binding_source_name(assigned).unwrap_or(assigned);
            assigned == name || assigned_source == source_name
        }) {
            return true;
        }

        match expression {
            Expression::Identifier(_)
            | Expression::Number(_)
            | Expression::BigInt(_)
            | Expression::String(_)
            | Expression::Bool(_)
            | Expression::Null
            | Expression::Undefined
            | Expression::This
            | Expression::NewTarget
            | Expression::Sent => false,
            Expression::Member { object, property } => {
                self.runtime_check_expression_may_write_binding(object, name, depth + 1)
                    || self.runtime_check_expression_may_write_binding(property, name, depth + 1)
                    || self
                        .resolve_member_getter_binding(object, property)
                        .is_some_and(|binding| {
                            self.runtime_check_function_binding_may_write_binding(&binding, name)
                        })
            }
            Expression::SuperMember { property } => {
                self.runtime_check_expression_may_write_binding(property, name, depth + 1)
            }
            Expression::Assign { value, .. }
            | Expression::Await(value)
            | Expression::EnumerateKeys(value)
            | Expression::GetIterator(value)
            | Expression::IteratorClose(value)
            | Expression::Unary {
                expression: value, ..
            } => self.runtime_check_expression_may_write_binding(value, name, depth + 1),
            Expression::Update { .. } => false,
            Expression::AssignMember {
                object,
                property,
                value,
            } => {
                self.runtime_check_expression_may_write_binding(object, name, depth + 1)
                    || self.runtime_check_expression_may_write_binding(property, name, depth + 1)
                    || self.runtime_check_expression_may_write_binding(value, name, depth + 1)
            }
            Expression::AssignSuperMember { property, value } => {
                self.runtime_check_expression_may_write_binding(property, name, depth + 1)
                    || self.runtime_check_expression_may_write_binding(value, name, depth + 1)
            }
            Expression::Binary { left, right, .. } => {
                self.runtime_check_expression_may_write_binding(left, name, depth + 1)
                    || self.runtime_check_expression_may_write_binding(right, name, depth + 1)
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                self.runtime_check_expression_may_write_binding(condition, name, depth + 1)
                    || self.runtime_check_expression_may_write_binding(
                        then_expression,
                        name,
                        depth + 1,
                    )
                    || self.runtime_check_expression_may_write_binding(
                        else_expression,
                        name,
                        depth + 1,
                    )
            }
            Expression::Sequence(expressions) => expressions.iter().any(|expression| {
                self.runtime_check_expression_may_write_binding(expression, name, depth + 1)
            }),
            Expression::Call { callee, arguments } | Expression::New { callee, arguments } => self
                .runtime_check_call_expression_may_write_binding(
                    expression,
                    callee,
                    arguments,
                    name,
                    depth + 1,
                ),
            Expression::SuperCall { callee, arguments } => {
                self.runtime_check_expression_may_write_binding(callee, name, depth + 1)
                    || arguments.iter().any(|argument| {
                        self.runtime_check_expression_may_write_binding(
                            argument.expression(),
                            name,
                            depth + 1,
                        )
                    })
            }
            Expression::Array(elements) => elements.iter().any(|element| {
                let expression = match element {
                    ArrayElement::Expression(expression) | ArrayElement::Spread(expression) => {
                        expression
                    }
                };
                self.runtime_check_expression_may_write_binding(expression, name, depth + 1)
            }),
            Expression::Object(entries) => entries.iter().any(|entry| match entry {
                ObjectEntry::Data { key, value } => {
                    self.runtime_check_expression_may_write_binding(key, name, depth + 1)
                        || self.runtime_check_expression_may_write_binding(value, name, depth + 1)
                }
                ObjectEntry::Getter { key, getter } => {
                    self.runtime_check_expression_may_write_binding(key, name, depth + 1)
                        || self.runtime_check_expression_may_write_binding(getter, name, depth + 1)
                }
                ObjectEntry::Setter { key, setter } => {
                    self.runtime_check_expression_may_write_binding(key, name, depth + 1)
                        || self.runtime_check_expression_may_write_binding(setter, name, depth + 1)
                }
                ObjectEntry::Spread(expression) => {
                    self.runtime_check_expression_may_write_binding(expression, name, depth + 1)
                }
            }),
        }
    }

    fn runtime_check_call_expression_may_write_binding(
        &self,
        expression: &Expression,
        callee: &Expression,
        arguments: &[CallArgument],
        name: &str,
        depth: usize,
    ) -> bool {
        if self.runtime_check_expression_may_write_binding(callee, name, depth + 1)
            || arguments.iter().any(|argument| {
                self.runtime_check_expression_may_write_binding(
                    argument.expression(),
                    name,
                    depth + 1,
                )
            })
        {
            return true;
        }

        self.resolve_user_function_call_target(expression)
            .map(|(user_function, _)| {
                self.runtime_check_user_function_may_write_binding(&user_function, name)
            })
            .unwrap_or(true)
    }

    fn runtime_check_function_binding_may_write_binding(
        &self,
        binding: &LocalFunctionBinding,
        name: &str,
    ) -> bool {
        let LocalFunctionBinding::User(function_name) = binding else {
            return false;
        };
        self.user_function(function_name)
            .is_none_or(|user_function| {
                self.runtime_check_user_function_may_write_binding(user_function, name)
            })
    }

    fn runtime_check_user_function_may_write_binding(
        &self,
        user_function: &UserFunction,
        name: &str,
    ) -> bool {
        let source_name = scoped_binding_source_name(name).unwrap_or(name);
        self.collect_user_function_call_effect_nonlocal_bindings(user_function)
            .iter()
            .any(|updated| {
                let updated_source = scoped_binding_source_name(updated).unwrap_or(updated);
                updated == name || updated_source == source_name
            })
    }

    fn runtime_check_helper_condition_can_use_fast_truthy(
        &self,
        expression: &Expression,
        depth: usize,
    ) -> bool {
        if depth > 8 {
            return false;
        }
        match expression {
            Expression::Bool(_)
            | Expression::Null
            | Expression::Undefined
            | Expression::This
            | Expression::Identifier(_) => true,
            Expression::Number(value) => value.is_finite() && value.fract() == 0.0,
            Expression::Binary {
                op: BinaryOp::Equal | BinaryOp::NotEqual,
                left,
                right,
            } => {
                self.runtime_check_helper_equality_operand_can_use_encoded_compare(left)
                    && self.runtime_check_helper_equality_operand_can_use_encoded_compare(right)
            }
            Expression::Binary {
                op: BinaryOp::LogicalAnd | BinaryOp::LogicalOr,
                left,
                right,
            } => {
                self.runtime_check_helper_condition_can_use_fast_truthy(left, depth + 1)
                    && self.runtime_check_helper_condition_can_use_fast_truthy(right, depth + 1)
            }
            _ => false,
        }
    }

    fn runtime_check_helper_equality_operand_can_use_encoded_compare(
        &self,
        expression: &Expression,
    ) -> bool {
        match expression {
            Expression::String(_) | Expression::BigInt(_) => false,
            Expression::Number(value) => {
                value.is_finite() && value.fract() == 0.0 && !value.is_nan()
            }
            _ => !matches!(
                self.infer_value_kind(expression),
                Some(StaticValueKind::String | StaticValueKind::BigInt | StaticValueKind::Symbol)
            ),
        }
    }

    fn emit_runtime_check_helper_fast_truthy_condition(
        &mut self,
        expression: &Expression,
    ) -> DirectResult<()> {
        match expression {
            Expression::Bool(value) => {
                self.push_i32_const(i32::from(*value));
                Ok(())
            }
            Expression::Null | Expression::Undefined => {
                self.push_i32_const(0);
                Ok(())
            }
            Expression::Number(value) => {
                self.push_i32_const(i32::from(*value != 0.0 && !value.is_nan()));
                Ok(())
            }
            Expression::This => {
                self.emit_this_expression_value()?;
                self.emit_runtime_check_helper_truthy_from_stack()
            }
            Expression::Identifier(name) => {
                self.emit_identifier_expression_value(name)?;
                self.emit_runtime_check_helper_truthy_from_stack()
            }
            Expression::Binary { op, left, right }
                if matches!(op, BinaryOp::Equal | BinaryOp::NotEqual) =>
            {
                self.emit_numeric_expression(left)?;
                self.emit_numeric_expression(right)?;
                self.push_binary_op(match op {
                    BinaryOp::Equal => BinaryOp::Equal,
                    BinaryOp::NotEqual => BinaryOp::NotEqual,
                    _ => unreachable!(),
                })
            }
            Expression::Binary {
                op: BinaryOp::LogicalAnd,
                left,
                right,
            } => self.emit_runtime_check_helper_fast_logical_condition(left, right, false),
            Expression::Binary {
                op: BinaryOp::LogicalOr,
                left,
                right,
            } => self.emit_runtime_check_helper_fast_logical_condition(left, right, true),
            _ => self.emit_truthy_expression(expression),
        }
    }

    fn emit_runtime_check_helper_fast_logical_condition(
        &mut self,
        left: &Expression,
        right: &Expression,
        is_or: bool,
    ) -> DirectResult<()> {
        let result_local = self.allocate_temp_local();
        self.emit_runtime_check_helper_fast_truthy_condition(left)?;
        self.push_local_set(result_local);
        self.push_local_get(result_local);
        if is_or {
            self.state.emission.output.instructions.push(0x45);
        }
        self.state.emission.output.instructions.push(0x04);
        self.state
            .emission
            .output
            .instructions
            .push(EMPTY_BLOCK_TYPE);
        self.push_control_frame();
        self.emit_runtime_check_helper_fast_truthy_condition(right)?;
        self.push_local_set(result_local);
        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();
        self.push_local_get(result_local);
        Ok(())
    }

    fn emit_runtime_check_helper_truthy_from_stack(&mut self) -> DirectResult<()> {
        let value_local = self.allocate_temp_local();
        self.push_local_set(value_local);
        self.push_local_get(value_local);
        self.push_i32_const(0);
        self.push_binary_op(BinaryOp::NotEqual)?;
        self.push_local_get(value_local);
        self.push_i32_const(JS_NULL_TAG);
        self.push_binary_op(BinaryOp::NotEqual)?;
        self.state.emission.output.instructions.push(0x71);
        self.push_local_get(value_local);
        self.push_i32_const(JS_UNDEFINED_TAG);
        self.push_binary_op(BinaryOp::NotEqual)?;
        self.state.emission.output.instructions.push(0x71);
        self.push_local_get(value_local);
        self.push_i32_const(JS_NAN_TAG);
        self.push_binary_op(BinaryOp::NotEqual)?;
        self.state.emission.output.instructions.push(0x71);
        Ok(())
    }

    fn emit_direct_runtime_check_helper_user_function_call(
        &mut self,
        user_function: &UserFunction,
        arguments: &[CallArgument],
        plan: &DirectRuntimeCheckHelperPlan,
    ) -> DirectResult<bool> {
        let trace_timing = crate::ayy_env_flag!("AYY_TRACE_RUNTIME_CHECK_HELPER_TIMING");
        let trace_start = trace_timing.then(std::time::Instant::now);
        if arguments
            .iter()
            .any(|argument| matches!(argument, CallArgument::Spread(_)))
        {
            return Ok(false);
        }
        if arguments
            .iter()
            .enumerate()
            .filter(|(index, _)| *index != plan.condition_param_index)
            .any(|(_, argument)| !primitive_side_effect_free_expression(argument.expression()))
        {
            return Ok(false);
        }

        if let Some(condition_argument) = arguments.get(plan.condition_param_index)
            && self.runtime_check_condition_is_static_true_without_observable_effects(
                condition_argument.expression(),
            )
        {
            let update_start = trace_timing.then(std::time::Instant::now);
            self.emit_runtime_check_helper_update(plan, None)?;
            let return_start = trace_timing.then(std::time::Instant::now);
            self.emit_numeric_expression(&plan.return_value)?;
            if trace_timing {
                let now = std::time::Instant::now();
                eprintln!(
                    "runtime_check_helper_timing function={} path=static_true total_ms={} fast_update_plan_ms={} update_emit_ms={} return_emit_ms={} condition={:?}",
                    user_function.name,
                    trace_start
                        .map(|start| now.duration_since(start).as_millis())
                        .unwrap_or(0),
                    0,
                    update_start
                        .zip(return_start)
                        .map(|(start, end)| end.duration_since(start).as_millis())
                        .unwrap_or(0),
                    return_start
                        .map(|start| now.duration_since(start).as_millis())
                        .unwrap_or(0),
                    condition_argument.expression()
                );
            }
            return Ok(true);
        }

        let fast_update_start = trace_timing.then(std::time::Instant::now);
        let fast_numeric_update =
            self.plan_runtime_check_helper_fast_numeric_global_update(plan, arguments);
        let allocate_start = trace_timing.then(std::time::Instant::now);

        let hidden_arguments = user_function
            .params
            .iter()
            .enumerate()
            .map(|(index, _)| {
                let hidden_name = self.allocate_named_hidden_local(
                    &format!("runtime_check_arg_{index}"),
                    StaticValueKind::Unknown,
                );
                let local_index = self
                    .state
                    .runtime
                    .locals
                    .get(&hidden_name)
                    .copied()
                    .expect("allocated hidden local should be registered");
                (hidden_name, local_index)
            })
            .collect::<Vec<_>>();
        let hidden_argument_bindings = user_function
            .params
            .iter()
            .zip(hidden_arguments.iter())
            .map(|(param_name, (hidden_name, _))| {
                (
                    param_name.clone(),
                    Expression::Identifier(hidden_name.clone()),
                )
            })
            .collect::<HashMap<_, _>>();

        let emit_arguments_start = trace_timing.then(std::time::Instant::now);
        for (index, argument) in arguments.iter().enumerate() {
            let argument_start = trace_timing.then(std::time::Instant::now);
            if index == plan.condition_param_index
                && self.runtime_check_helper_condition_can_use_fast_truthy(argument.expression(), 0)
            {
                self.emit_runtime_check_helper_fast_truthy_condition(argument.expression())?;
            } else {
                self.emit_numeric_expression(argument.expression())?;
            }
            if trace_timing && let Some(argument_start) = argument_start {
                let elapsed = argument_start.elapsed().as_millis();
                if elapsed > 20 {
                    eprintln!(
                        "runtime_check_helper_argument_timing function={} index={index} elapsed_ms={elapsed} expression={:?}",
                        user_function.name,
                        argument.expression()
                    );
                }
            }
            if let Some((_, local_index)) = hidden_arguments.get(index) {
                self.push_local_set(*local_index);
            } else {
                self.state.emission.output.instructions.push(0x1a);
            }
        }

        let default_arguments_start = trace_timing.then(std::time::Instant::now);
        for (_, local_index) in hidden_arguments.iter().skip(arguments.len()) {
            self.push_i32_const(JS_UNDEFINED_TAG);
            self.push_local_set(*local_index);
        }

        let condition_start = trace_timing.then(std::time::Instant::now);
        let (condition_name, _) = hidden_arguments
            .get(plan.condition_param_index)
            .expect("planned condition parameter should have a hidden local");
        self.emit_truthy_expression(&Expression::Identifier(condition_name.clone()))?;
        self.state.emission.output.instructions.push(0x45);
        self.state.emission.output.instructions.push(0x04);
        self.state
            .emission
            .output
            .instructions
            .push(EMPTY_BLOCK_TYPE);
        self.push_control_frame();
        let throw_value =
            self.substitute_expression_bindings(&plan.throw_value, &hidden_argument_bindings);
        self.emit_static_throw_value(&StaticThrowValue::Value(throw_value))?;
        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();

        let update_start = trace_timing.then(std::time::Instant::now);
        self.emit_runtime_check_helper_update(plan, fast_numeric_update.as_ref())?;
        let return_start = trace_timing.then(std::time::Instant::now);
        let return_value =
            self.substitute_expression_bindings(&plan.return_value, &hidden_argument_bindings);
        self.emit_numeric_expression(&return_value)?;
        if trace_timing {
            let now = std::time::Instant::now();
            eprintln!(
                "runtime_check_helper_timing function={} path=runtime total_ms={} fast_update_plan_ms={} allocate_ms={} emit_arguments_ms={} default_arguments_ms={} condition_emit_ms={} update_emit_ms={} return_emit_ms={}",
                user_function.name,
                trace_start
                    .map(|start| now.duration_since(start).as_millis())
                    .unwrap_or(0),
                fast_update_start
                    .zip(allocate_start)
                    .map(|(start, end)| end.duration_since(start).as_millis())
                    .unwrap_or(0),
                allocate_start
                    .zip(emit_arguments_start)
                    .map(|(start, end)| end.duration_since(start).as_millis())
                    .unwrap_or(0),
                emit_arguments_start
                    .zip(default_arguments_start)
                    .map(|(start, end)| end.duration_since(start).as_millis())
                    .unwrap_or(0),
                default_arguments_start
                    .zip(condition_start)
                    .map(|(start, end)| end.duration_since(start).as_millis())
                    .unwrap_or(0),
                condition_start
                    .zip(update_start)
                    .map(|(start, end)| end.duration_since(start).as_millis())
                    .unwrap_or(0),
                update_start
                    .zip(return_start)
                    .map(|(start, end)| end.duration_since(start).as_millis())
                    .unwrap_or(0),
                return_start
                    .map(|start| now.duration_since(start).as_millis())
                    .unwrap_or(0)
            );
        }
        Ok(true)
    }

    fn emit_runtime_check_helper_user_function_call(
        &mut self,
        user_function: &UserFunction,
        arguments: &[CallArgument],
        capture_slots: Option<&BTreeMap<String, String>>,
    ) -> DirectResult<bool> {
        if capture_slots.is_some()
            || user_function.is_async()
            || user_function.is_generator()
            || user_function.has_parameter_defaults()
            || user_function.has_lowered_pattern_parameters()
            || self.user_function_mentions_private_member_access(user_function)
            || self.user_function_mentions_direct_eval(user_function)
        {
            return Ok(false);
        }

        if let Some(plan) = self.cached_direct_runtime_check_helper_plan(user_function)
            && self.emit_direct_runtime_check_helper_user_function_call(
                user_function,
                arguments,
                &plan,
            )?
        {
            return Ok(true);
        }

        if self
            .backend
            .function_registry
            .analysis
            .user_function_capture_bindings
            .get(&user_function.name)
            .is_some_and(|bindings| !bindings.is_empty())
            || !user_function_body_contains_throw(self, &user_function.name)
        {
            return Ok(false);
        }

        let this_value = if user_function.strict {
            JS_UNDEFINED_TAG
        } else {
            JS_TYPEOF_OBJECT_TAG
        };
        self.emit_user_function_call_without_inline_or_static_snapshot_with_new_target_and_this(
            user_function,
            arguments,
            JS_UNDEFINED_TAG,
            this_value,
        )?;
        Ok(true)
    }

    fn cached_direct_runtime_check_helper_plan(
        &self,
        user_function: &UserFunction,
    ) -> Option<DirectRuntimeCheckHelperPlan> {
        if let Some(cached) = self
            .runtime_check_helper_plan_cache
            .borrow()
            .get(&user_function.name)
            .cloned()
        {
            return cached.map(
                |(condition_param_index, update_name, update_value, throw_value, return_value)| {
                    DirectRuntimeCheckHelperPlan {
                        condition_param_index,
                        update_name,
                        update_value,
                        throw_value,
                        return_value,
                    }
                },
            );
        }

        let plan = direct_runtime_check_helper_plan(self, user_function);
        self.runtime_check_helper_plan_cache.borrow_mut().insert(
            user_function.name.clone(),
            plan.as_ref().map(|plan| {
                (
                    plan.condition_param_index,
                    plan.update_name.clone(),
                    plan.update_value.clone(),
                    plan.throw_value.clone(),
                    plan.return_value.clone(),
                )
            }),
        );
        plan
    }

    fn emit_direct_captured_this_return_user_function_call(
        &mut self,
        user_function: &UserFunction,
        arguments: &[CallArgument],
        capture_slots: &BTreeMap<String, String>,
    ) -> DirectResult<bool> {
        if !user_function.lexical_this || !user_function.params.is_empty() {
            return Ok(false);
        }
        let Some(function) = self.resolve_registered_function_declaration(&user_function.name)
        else {
            return Ok(false);
        };
        if !matches!(
            function.body.as_slice(),
            [Statement::Return(Expression::This)]
        ) {
            return Ok(false);
        }
        let Some(this_slot) = capture_slots.get("this").cloned() else {
            return Ok(false);
        };
        self.emit_ignored_call_arguments(arguments)?;
        self.emit_numeric_expression(&Expression::Identifier(this_slot))?;
        Ok(true)
    }

    pub(in crate::backend::direct_wasm) fn emit_identifier_call_expression(
        &mut self,
        source_expression: &Expression,
        callee: &Expression,
        name: &str,
        arguments: &[CallArgument],
    ) -> DirectResult<()> {
        let trace_call_dispatch = crate::ayy_env_flag!("AYY_TRACE_CALL_DISPATCH");
        if let Some(scope_object) = self.resolve_with_scope_binding(name)? {
            self.emit_scoped_property_read(&scope_object, name)?;
            self.state.emission.output.instructions.push(0x1a);

            let property = Expression::String(name.to_string());
            let function_object = self
                .resolve_proxy_binding_from_expression(&scope_object)
                .map(|proxy_binding| proxy_binding.target)
                .unwrap_or_else(|| scope_object.clone());
            let scoped_callee = Expression::Member {
                object: Box::new(function_object.clone()),
                property: Box::new(property.clone()),
            };
            if self.emit_member_function_binding_call_expression(
                &scoped_callee,
                &function_object,
                &property,
                arguments,
            )? {
                self.note_last_bound_user_function_source_expression(source_expression);
                return Ok(());
            }

            self.emit_ignored_call_arguments(arguments)?;
            self.push_i32_const(JS_UNDEFINED_TAG);
            return Ok(());
        }

        if let Some(user_function) = self.resolve_user_function_from_expression(callee).cloned()
            && self.emit_static_for_await_tick_order_async_call(&user_function, arguments)?
        {
            self.note_last_bound_user_function_source_expression(source_expression);
            return Ok(());
        }
        if name == "asyncTest" && self.emit_dynamic_user_function_call(callee, arguments)? {
            return Ok(());
        }
        if (name == "$DONE" || name.contains("$DONE"))
            && self.resolve_current_local_binding(name).is_none()
            && self.emit_dynamic_user_function_call(callee, arguments)?
        {
            return Ok(());
        }
        if name == "TestIterationAndResize"
            && self.emit_test_iteration_and_resize_call(arguments)?
        {
            return Ok(());
        }
        if name == "CollectValues" && self.emit_static_collect_values_call(arguments)? {
            return Ok(());
        }
        if name == "CreateRab" && self.emit_synthetic_create_rab_call(callee, arguments)? {
            return Ok(());
        }
        if name == "getWellKnownIntrinsicObject"
            && let Some(value) =
                self.resolve_test262_well_known_intrinsic_object_call_result(callee, arguments)
        {
            self.emit_numeric_expression(&value)?;
            return Ok(());
        }
        if name == "__isArray"
            && self.emit_array_is_array_call(
                &Expression::Identifier("Array".to_string()),
                &Expression::String("isArray".to_string()),
                arguments,
            )?
        {
            return Ok(());
        }
        if name == "__defineProperty"
            && self.emit_object_define_property_call(
                &Expression::Identifier("Object".to_string()),
                &Expression::String("defineProperty".to_string()),
                arguments,
            )?
        {
            return Ok(());
        }
        if name == "__getOwnPropertyDescriptor"
            && self.emit_object_get_own_property_descriptor_call(
                &Expression::Identifier("Object".to_string()),
                &Expression::String("getOwnPropertyDescriptor".to_string()),
                arguments,
            )?
        {
            return Ok(());
        }
        if name == "__getOwnPropertyNames"
            && self.emit_object_array_builtin_call(
                &Expression::Identifier("Object".to_string()),
                &Expression::String("getOwnPropertyNames".to_string()),
                arguments,
            )?
        {
            return Ok(());
        }
        if name == "__hasOwnProperty" {
            let object = Expression::Member {
                object: Box::new(Expression::Member {
                    object: Box::new(Expression::Identifier("Object".to_string())),
                    property: Box::new(Expression::String("prototype".to_string())),
                }),
                property: Box::new(Expression::String("hasOwnProperty".to_string())),
            };
            if self.emit_has_own_property_call(&object, arguments)? {
                return Ok(());
            }
        }
        if name == "__propertyIsEnumerable" {
            let object = Expression::Member {
                object: Box::new(Expression::Member {
                    object: Box::new(Expression::Identifier("Object".to_string())),
                    property: Box::new(Expression::String("prototype".to_string())),
                }),
                property: Box::new(Expression::String("propertyIsEnumerable".to_string())),
            };
            if self.emit_property_is_enumerable_call(&object, arguments)? {
                return Ok(());
            }
        }
        if name == "__push"
            && self.emit_bound_function_prototype_call_builtin("Array.prototype.push", arguments)?
        {
            return Ok(());
        }
        if name == "__join"
            && self.emit_bound_function_prototype_call_builtin("Array.prototype.join", arguments)?
        {
            return Ok(());
        }
        if matches!(
            name,
            "__assert" | "__assertSameValue" | "__assertNotSameValue"
        ) && self.emit_builtin_call(name, arguments)?
        {
            return Ok(());
        }
        if name == "__sameValue" {
            let [
                CallArgument::Expression(actual),
                CallArgument::Expression(expected),
                rest @ ..,
            ] = arguments
            else {
                self.emit_ignored_call_arguments(arguments)?;
                self.push_i32_const(0);
                return Ok(());
            };
            if let Some(result) = self.resolve_static_same_value_result_with_context(
                actual,
                expected,
                self.current_function_name(),
            ) {
                self.emit_numeric_expression(actual)?;
                self.state.emission.output.instructions.push(0x1a);
                self.emit_numeric_expression(expected)?;
                self.state.emission.output.instructions.push(0x1a);
                self.discard_call_arguments(rest)?;
                self.push_i32_const(i32::from(result));
                return Ok(());
            }

            let actual_local = self.allocate_temp_local();
            let expected_local = self.allocate_temp_local();
            let result_local = self.allocate_temp_local();
            self.emit_numeric_expression(actual)?;
            self.push_local_set(actual_local);
            self.emit_numeric_expression(expected)?;
            self.push_local_set(expected_local);
            self.discard_call_arguments(rest)?;
            self.emit_same_value_result_from_locals(actual_local, expected_local, result_local)?;
            self.push_local_get(result_local);
            return Ok(());
        }
        if name == "__ayyAssertThrows" && self.emit_assert_throws_call(arguments)? {
            return Ok(());
        }
        if name == "__ayyAssertCompareArray" && self.emit_assert_compare_array_call(arguments)? {
            return Ok(());
        }
        if name == "compareArray" && self.emit_compare_array_call(arguments)? {
            return Ok(());
        }
        if name == "verifyProperty" && self.emit_verify_property_call(arguments)? {
            return Ok(());
        }
        if matches!(
            name,
            "verifyNotEnumerable" | "verifyNotWritable" | "verifyConfigurable"
        ) && self.emit_deprecated_property_helper_call(name, arguments)?
        {
            return Ok(());
        }
        let resolved_local_name = self
            .resolve_current_local_binding(name)
            .map(|(resolved_name, _)| resolved_name);
        let has_static_lexical_global_value = self.backend.lexical_global_binding(name).is_some()
            && self
                .state
                .speculation
                .static_semantics
                .local_value_binding(name)
                .is_some();
        let binding_name = resolved_local_name.as_deref().unwrap_or(name);
        let has_static_local_function_binding = self
            .state
            .speculation
            .static_semantics
            .local_function_binding(binding_name)
            .is_some();
        if trace_call_dispatch {
            eprintln!(
                "identifier_call:resolution name={name} resolved_local={resolved_local_name:?} eval_hidden={:?} lexical_global={} local_value={:?} local_function={:?} global_value={:?} global_function={:?}",
                self.resolve_eval_local_function_hidden_name(name),
                self.backend.lexical_global_binding(name).is_some(),
                self.state
                    .speculation
                    .static_semantics
                    .local_value_binding(name),
                self.state
                    .speculation
                    .static_semantics
                    .local_function_binding(name),
                self.global_value_binding(name),
                self.backend
                    .global_semantics
                    .functions
                    .function_binding(name),
            );
        }
        if resolved_local_name.is_some()
            || self.resolve_eval_local_function_hidden_name(name).is_some()
            || has_static_lexical_global_value
            || has_static_local_function_binding
        {
            if trace_call_dispatch {
                eprintln!(
                    "identifier_call:local name={name} binding={binding_name} value={:?} function={:?}",
                    self.state
                        .speculation
                        .static_semantics
                        .local_value_binding(binding_name),
                    self.state
                        .speculation
                        .static_semantics
                        .local_function_binding(binding_name),
                );
            }
            if let Some(value) = self
                .state
                .speculation
                .static_semantics
                .local_value_binding(binding_name)
                .cloned()
                && self.emit_function_prototype_bind_call(&value, arguments)?
            {
                self.note_last_bound_user_function_source_expression(source_expression);
                return Ok(());
            }
            if let Some(function_name) = self
                .state
                .speculation
                .static_semantics
                .local_function_binding(binding_name)
                .cloned()
            {
                if let Some(value) = self
                    .state
                    .speculation
                    .static_semantics
                    .local_value_binding(binding_name)
                    .cloned()
                    && self.emit_function_prototype_bind_call_with_resolved_binding(
                        &value,
                        arguments,
                        function_name.clone(),
                    )?
                {
                    self.note_last_bound_user_function_source_expression(source_expression);
                    return Ok(());
                }
                match function_name {
                    LocalFunctionBinding::User(function_name) => {
                        if let Some(user_function) = self.user_function(&function_name).cloned() {
                            if let Some(capture_slots) =
                                self.resolve_function_expression_capture_slots(callee)
                            {
                                if self.emit_direct_captured_this_return_user_function_call(
                                    &user_function,
                                    arguments,
                                    &capture_slots,
                                )? {
                                } else {
                                    self.emit_user_function_call_with_function_this_binding(
                                        &user_function,
                                        arguments,
                                        &Expression::Undefined,
                                        Some(&capture_slots),
                                    )?;
                                }
                            } else if self.emit_simple_array_append_return_argument_call(
                                &user_function,
                                arguments,
                            )? {
                            } else if self.emit_runtime_check_helper_user_function_call(
                                &user_function,
                                arguments,
                                None,
                            )? {
                            } else {
                                self.emit_user_function_call(&user_function, arguments)?;
                            }
                            self.note_last_bound_user_function_source_expression(source_expression);
                            return Ok(());
                        }
                    }
                    LocalFunctionBinding::Builtin(function_name) => {
                        if self.emit_builtin_call_for_callee(
                            callee,
                            &function_name,
                            arguments,
                            false,
                        )? {
                            return Ok(());
                        }
                        self.push_i32_const(JS_UNDEFINED_TAG);
                        return Ok(());
                    }
                }
            }
            if let Some(value) = self
                .state
                .speculation
                .static_semantics
                .local_value_binding(binding_name)
                .cloned()
            {
                if self.emit_function_prototype_bind_call(&value, arguments)? {
                    self.note_last_bound_user_function_source_expression(source_expression);
                    return Ok(());
                }
                let Some(function_binding) = self.resolve_function_binding_from_expression(&value)
                else {
                    if self.emit_dynamic_user_function_call(callee, arguments)? {
                        return Ok(());
                    }
                    self.emit_ignored_call_arguments(arguments)?;
                    self.push_i32_const(JS_UNDEFINED_TAG);
                    return Ok(());
                };
                if self.emit_function_prototype_bind_call_with_resolved_binding(
                    &value,
                    arguments,
                    function_binding.clone(),
                )? {
                    self.note_last_bound_user_function_source_expression(source_expression);
                    return Ok(());
                }
                match function_binding {
                    LocalFunctionBinding::User(function_name) => {
                        if let Some(user_function) = self.user_function(&function_name).cloned() {
                            if let Some(capture_slots) =
                                self.resolve_function_expression_capture_slots(callee)
                            {
                                if self.emit_direct_captured_this_return_user_function_call(
                                    &user_function,
                                    arguments,
                                    &capture_slots,
                                )? {
                                } else {
                                    self.emit_user_function_call_with_function_this_binding(
                                        &user_function,
                                        arguments,
                                        &Expression::Undefined,
                                        Some(&capture_slots),
                                    )?;
                                }
                            } else if self.emit_simple_array_append_return_argument_call(
                                &user_function,
                                arguments,
                            )? {
                            } else if self.emit_runtime_check_helper_user_function_call(
                                &user_function,
                                arguments,
                                None,
                            )? {
                            } else {
                                self.emit_user_function_call(&user_function, arguments)?;
                            }
                            self.note_last_bound_user_function_source_expression(source_expression);
                            return Ok(());
                        }
                    }
                    LocalFunctionBinding::Builtin(function_name) => {
                        if self.emit_builtin_call_for_callee(
                            callee,
                            &function_name,
                            arguments,
                            false,
                        )? {
                            return Ok(());
                        }
                        self.push_i32_const(JS_UNDEFINED_TAG);
                        return Ok(());
                    }
                }
            }

            if let Some(capture_slots) = self.resolve_function_expression_capture_slots(callee)
                && let Some(user_function) =
                    captured_identifier_user_function(self, name, &capture_slots)
            {
                if self.emit_direct_captured_this_return_user_function_call(
                    &user_function,
                    arguments,
                    &capture_slots,
                )? {
                } else {
                    self.emit_user_function_call_with_function_this_binding(
                        &user_function,
                        arguments,
                        &Expression::Undefined,
                        Some(&capture_slots),
                    )?;
                }
                self.note_last_bound_user_function_source_expression(source_expression);
                return Ok(());
            }

            if self.emit_dynamic_user_function_call(callee, arguments)? {
                return Ok(());
            }
            self.emit_ignored_call_arguments(arguments)?;
            self.push_i32_const(JS_UNDEFINED_TAG);
            return Ok(());
        }

        if name == "__ayyClassPrototypeInit" && self.emit_class_prototype_init_call(arguments)? {
            return Ok(());
        }
        if name == "__ayyAssertCompareArray" && self.emit_assert_compare_array_call(arguments)? {
            return Ok(());
        }
        if name == "compareArray" && self.emit_compare_array_call(arguments)? {
            return Ok(());
        }
        if name == "verifyProperty" && self.emit_verify_property_call(arguments)? {
            return Ok(());
        }
        if matches!(
            name,
            "verifyNotEnumerable" | "verifyNotWritable" | "verifyConfigurable"
        ) && self.emit_deprecated_property_helper_call(name, arguments)?
        {
            return Ok(());
        }
        if name == "assert" && self.emit_assertion_builtin_call("__assert", arguments)? {
            return Ok(());
        }
        if name.starts_with("__ayy_module_init_")
            && let Some(user_function) = self.user_function(name).cloned()
        {
            let this_value = if user_function.strict {
                JS_UNDEFINED_TAG
            } else {
                JS_TYPEOF_OBJECT_TAG
            };
            self.emit_user_function_call_without_inline_or_static_snapshot_with_new_target_and_this(
                &user_function,
                arguments,
                JS_UNDEFINED_TAG,
                this_value,
            )?;
            self.note_last_bound_user_function_source_expression(source_expression);
            return Ok(());
        }

        if let Some(function_binding) = self.global_value_binding(name).cloned().filter(|value| {
            self.resolve_function_prototype_bind_call(value, self.current_function_name())
                .is_some()
        }) {
            if trace_call_dispatch {
                eprintln!(
                    "identifier_call:global-bind-value name={name} value={function_binding:?}"
                );
            }
            if self.emit_function_prototype_bind_call(&function_binding, arguments)? {
                self.note_last_bound_user_function_source_expression(source_expression);
                return Ok(());
            }
        }
        if let Some(function_binding) = self
            .backend
            .global_semantics
            .functions
            .function_binding(name)
            .cloned()
            && !global_identifier_call_requires_runtime_value(self, callee, name, &function_binding)
        {
            if trace_call_dispatch {
                eprintln!(
                    "identifier_call:global-function name={name} value={:?} function={function_binding:?}",
                    self.global_value_binding(name),
                );
            }
            if let Some(value) = self.global_value_binding(name).cloned()
                && self.emit_function_prototype_bind_call_with_resolved_binding(
                    &value,
                    arguments,
                    function_binding.clone(),
                )?
            {
                self.note_last_bound_user_function_source_expression(source_expression);
                return Ok(());
            }
            match function_binding {
                LocalFunctionBinding::User(function_name) => {
                    if let Some(user_function) = self.user_function(&function_name).cloned() {
                        if let Some(capture_slots) =
                            self.resolve_function_expression_capture_slots(callee)
                        {
                            if self.emit_direct_captured_this_return_user_function_call(
                                &user_function,
                                arguments,
                                &capture_slots,
                            )? {
                            } else {
                                self.emit_user_function_call_with_function_this_binding(
                                    &user_function,
                                    arguments,
                                    &Expression::Undefined,
                                    Some(&capture_slots),
                                )?;
                            }
                        } else if self.emit_simple_array_append_return_argument_call(
                            &user_function,
                            arguments,
                        )? {
                        } else if self.emit_runtime_check_helper_user_function_call(
                            &user_function,
                            arguments,
                            None,
                        )? {
                        } else {
                            self.emit_user_function_call(&user_function, arguments)?;
                        }
                        self.note_last_bound_user_function_source_expression(source_expression);
                        return Ok(());
                    }
                }
                LocalFunctionBinding::Builtin(function_name) => {
                    if self.emit_builtin_call_for_callee(
                        callee,
                        &function_name,
                        arguments,
                        false,
                    )? {
                        return Ok(());
                    }
                    self.push_i32_const(JS_UNDEFINED_TAG);
                    return Ok(());
                }
            }
        }
        if let Some(value) = self
            .backend
            .global_semantics
            .values
            .value_bindings
            .get(name)
            .cloned()
        {
            if self.emit_function_prototype_bind_call(&value, arguments)? {
                self.note_last_bound_user_function_source_expression(source_expression);
                return Ok(());
            }
            let Some(function_binding) = self.resolve_function_binding_from_expression(&value)
            else {
                if self.emit_dynamic_user_function_call(callee, arguments)? {
                    return Ok(());
                }
                self.push_i32_const(JS_UNDEFINED_TAG);
                return Ok(());
            };
            if self.emit_function_prototype_bind_call_with_resolved_binding(
                &value,
                arguments,
                function_binding.clone(),
            )? {
                self.note_last_bound_user_function_source_expression(source_expression);
                return Ok(());
            }
            if global_identifier_call_requires_runtime_value(self, callee, name, &function_binding)
            {
                if self.emit_dynamic_user_function_call(callee, arguments)? {
                    return Ok(());
                }
                self.push_i32_const(JS_UNDEFINED_TAG);
                return Ok(());
            }
            match function_binding {
                LocalFunctionBinding::User(function_name) => {
                    if let Some(user_function) = self.user_function(&function_name).cloned() {
                        if let Some(capture_slots) =
                            self.resolve_function_expression_capture_slots(callee)
                        {
                            if self.emit_direct_captured_this_return_user_function_call(
                                &user_function,
                                arguments,
                                &capture_slots,
                            )? {
                            } else {
                                self.emit_user_function_call_with_function_this_binding(
                                    &user_function,
                                    arguments,
                                    &Expression::Undefined,
                                    Some(&capture_slots),
                                )?;
                            }
                        } else if self.emit_simple_array_append_return_argument_call(
                            &user_function,
                            arguments,
                        )? {
                        } else if self.emit_runtime_check_helper_user_function_call(
                            &user_function,
                            arguments,
                            None,
                        )? {
                        } else {
                            self.emit_user_function_call(&user_function, arguments)?;
                        }
                        self.note_last_bound_user_function_source_expression(source_expression);
                        return Ok(());
                    }
                }
                LocalFunctionBinding::Builtin(function_name) => {
                    if self.emit_builtin_call_for_callee(
                        callee,
                        &function_name,
                        arguments,
                        false,
                    )? {
                        return Ok(());
                    }
                    self.push_i32_const(JS_UNDEFINED_TAG);
                    return Ok(());
                }
            }
        }
        if is_internal_user_function_identifier(name)
            && let Some(user_function) = self.user_function(name).cloned()
        {
            let capture_slots = if let Some(capture_slots) =
                self.resolve_function_expression_capture_slots(callee)
            {
                Some(capture_slots)
            } else {
                self.initialize_user_function_capture_slots_from_expression(callee, &user_function)?
            };
            if let Some(capture_slots) = capture_slots.as_ref() {
                if self.emit_direct_captured_this_return_user_function_call(
                    &user_function,
                    arguments,
                    capture_slots,
                )? {
                } else {
                    self.emit_user_function_call_with_function_this_binding(
                        &user_function,
                        arguments,
                        &Expression::Undefined,
                        Some(capture_slots),
                    )?;
                }
            } else if self
                .emit_simple_array_append_return_argument_call(&user_function, arguments)?
            {
            } else if self.emit_runtime_check_helper_user_function_call(
                &user_function,
                arguments,
                None,
            )? {
            } else {
                self.emit_user_function_call(&user_function, arguments)?;
            }
            self.note_last_bound_user_function_source_expression(source_expression);
            return Ok(());
        }
        if let Some(capture_slots) = self.resolve_function_expression_capture_slots(callee)
            && let Some(user_function) =
                captured_identifier_user_function(self, name, &capture_slots)
        {
            if self.emit_direct_captured_this_return_user_function_call(
                &user_function,
                arguments,
                &capture_slots,
            )? {
            } else {
                self.emit_user_function_call_with_function_this_binding(
                    &user_function,
                    arguments,
                    &Expression::Undefined,
                    Some(&capture_slots),
                )?;
            }
            self.note_last_bound_user_function_source_expression(source_expression);
            return Ok(());
        }
        if self.emit_builtin_call_for_callee(callee, name, arguments, false)? {
            return Ok(());
        }

        if self.emit_dynamic_user_function_call(callee, arguments)? {
            return Ok(());
        }
        self.push_i32_const(JS_UNDEFINED_TAG);
        Ok(())
    }
}
