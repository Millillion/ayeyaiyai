use super::*;

impl<'a> FunctionCompiler<'a> {
    fn ordinary_to_primitive_outcome_is_terminal(
        &self,
        outcome: &StaticEvalOutcome,
    ) -> Option<bool> {
        match outcome {
            StaticEvalOutcome::Throw(_) => Some(true),
            StaticEvalOutcome::Value(value) => {
                self.static_expression_is_non_object_primitive(value)
            }
        }
    }

    fn ordinary_to_primitive_target_expression<'b>(expression: &'b Expression) -> &'b Expression {
        match expression {
            Expression::Sequence(expressions) => expressions
                .last()
                .map(Self::ordinary_to_primitive_target_expression)
                .unwrap_or(expression),
            _ => expression,
        }
    }

    fn resolve_ordinary_to_primitive_step_outcome(
        &self,
        binding: &LocalFunctionBinding,
        this_expression: &Expression,
    ) -> Option<StaticEvalOutcome> {
        self.resolve_terminal_function_outcome_from_binding(binding, &[])
            .or_else(|| {
                self.resolve_function_binding_static_return_expression_with_call_frame(
                    binding,
                    &[],
                    this_expression,
                )
                .or_else(|| self.resolve_function_binding_static_return_expression(binding, &[]))
                .map(StaticEvalOutcome::Value)
            })
    }

    pub(in crate::backend::direct_wasm) fn raw_object_literal_ordinary_to_primitive_method<'b>(
        entries: &'b [ObjectEntry],
        method_name: &str,
    ) -> Option<&'b Expression> {
        entries.iter().rev().find_map(|entry| {
            let ObjectEntry::Data { key, value } = entry else {
                return None;
            };
            matches!(key, Expression::String(name) if name == method_name).then_some(value)
        })
    }

    pub(in crate::backend::direct_wasm) fn resolve_raw_object_literal_ordinary_to_primitive_plan(
        &self,
        expression: &Expression,
        entries: &[ObjectEntry],
    ) -> Option<OrdinaryToPrimitivePlan> {
        let mut steps = Vec::new();
        let mut own_non_callable_methods = 0;
        for method_name in ["valueOf", "toString"] {
            let Some(method_value) =
                Self::raw_object_literal_ordinary_to_primitive_method(entries, method_name)
            else {
                continue;
            };
            let binding = self
                .resolve_function_binding_from_expression(method_value)
                .or_else(|| {
                    let materialized = self.materialize_static_expression(method_value);
                    self.resolve_function_binding_from_expression(&materialized)
                });
            let Some(binding) = binding else {
                own_non_callable_methods += 1;
                continue;
            };
            let outcome = self
                .resolve_ordinary_to_primitive_step_outcome(&binding, expression)
                .or_else(|| {
                    self.resolve_static_member_call_outcome_with_context(
                        expression,
                        method_name,
                        self.current_function_name(),
                    )
                })?;
            let terminal = self.ordinary_to_primitive_outcome_is_terminal(&outcome)?;
            steps.push(OrdinaryToPrimitiveStep { binding, outcome });
            if terminal {
                return Some(OrdinaryToPrimitivePlan { steps });
            }
        }
        (!steps.is_empty() || own_non_callable_methods == 2)
            .then_some(OrdinaryToPrimitivePlan { steps })
    }

    pub(in crate::backend::direct_wasm) fn resolve_ordinary_to_primitive_plan(
        &self,
        expression: &Expression,
    ) -> Option<OrdinaryToPrimitivePlan> {
        if self.expression_depends_on_active_loop_assignment(expression) {
            return None;
        }
        let expression = Self::ordinary_to_primitive_target_expression(expression);
        if let Expression::Object(entries) = expression
            && let Some(plan) =
                self.resolve_raw_object_literal_ordinary_to_primitive_plan(expression, entries)
        {
            return Some(plan);
        }
        let materialized = if self.binary_expression_calls_user_function(expression) {
            expression.clone()
        } else {
            self.materialize_static_expression(expression)
        };
        let object_binding = self
            .resolve_object_binding_from_expression(expression)
            .or_else(|| {
                (!static_expression_matches(&materialized, expression))
                    .then(|| self.resolve_object_binding_from_expression(&materialized))
                    .flatten()
            })
            .or_else(|| self.resolve_effectful_returned_object_binding(expression));
        let mut steps = Vec::new();
        let mut own_non_callable_methods = 0;
        for method_name in ["valueOf", "toString"] {
            let property = Expression::String(method_name.to_string());
            if let Some(object_binding) = object_binding.as_ref()
                && let Some(method_value) = object_binding_lookup_value(object_binding, &property)
            {
                let Some(binding) = self.resolve_function_binding_from_expression(method_value)
                else {
                    own_non_callable_methods += 1;
                    continue;
                };
                let outcome = self
                    .resolve_ordinary_to_primitive_step_outcome(&binding, expression)
                    .or_else(|| {
                        self.resolve_static_member_call_outcome_with_context(
                            expression,
                            method_name,
                            self.current_function_name(),
                        )
                    })?;
                let terminal = self.ordinary_to_primitive_outcome_is_terminal(&outcome)?;
                steps.push(OrdinaryToPrimitiveStep { binding, outcome });
                if terminal {
                    return Some(OrdinaryToPrimitivePlan { steps });
                }
                continue;
            }
            if let Some(binding) = self.resolve_member_function_binding(expression, &property) {
                let outcome = self
                    .resolve_ordinary_to_primitive_step_outcome(&binding, expression)
                    .or_else(|| {
                        self.resolve_static_member_call_outcome_with_context(
                            expression,
                            method_name,
                            self.current_function_name(),
                        )
                    })?;
                let terminal = self.ordinary_to_primitive_outcome_is_terminal(&outcome)?;
                steps.push(OrdinaryToPrimitiveStep { binding, outcome });
                if terminal {
                    return Some(OrdinaryToPrimitivePlan { steps });
                }
                continue;
            }
        }
        (!steps.is_empty() || own_non_callable_methods == 2)
            .then_some(OrdinaryToPrimitivePlan { steps })
    }

    pub(in crate::backend::direct_wasm) fn resolve_ordinary_to_primitive_plan_with_state(
        &self,
        expression: &Expression,
        environment: &mut StaticResolutionEnvironment,
    ) -> Option<OrdinaryToPrimitivePlan> {
        if self.expression_depends_on_active_loop_assignment(expression) {
            return None;
        }
        let expression = Self::ordinary_to_primitive_target_expression(expression);
        if let Expression::Object(entries) = expression
            && let Some(plan) =
                self.resolve_raw_object_literal_ordinary_to_primitive_plan(expression, entries)
        {
            return Some(plan);
        }

        let materialized = if self.binary_expression_calls_user_function(expression) {
            expression.clone()
        } else {
            self.resolve_static_expression_value_with_state(expression, environment)
        };
        if let Expression::Object(entries) = &materialized
            && let Some(plan) =
                self.resolve_raw_object_literal_ordinary_to_primitive_plan(&materialized, entries)
        {
            return Some(plan);
        }

        let mut object_binding =
            self.resolve_object_binding_from_expression_with_state(expression, environment);
        if object_binding.is_none() && !static_expression_matches(&materialized, expression) {
            object_binding =
                self.resolve_object_binding_from_expression_with_state(&materialized, environment);
        }
        if object_binding.is_none()
            && let Expression::Identifier(name) = expression
        {
            object_binding = environment.object_binding(name).cloned();
        }

        let this_expression = if static_expression_matches(&materialized, expression) {
            expression
        } else {
            &materialized
        };
        let mut steps = Vec::new();
        let mut own_non_callable_methods = 0;
        for method_name in ["valueOf", "toString"] {
            let property = Expression::String(method_name.to_string());
            if let Some(object_binding) = object_binding.as_ref()
                && let Some(method_value) = object_binding_lookup_value(object_binding, &property)
            {
                let Some(binding) = self.resolve_function_binding_from_expression(method_value)
                else {
                    own_non_callable_methods += 1;
                    continue;
                };
                let outcome = self
                    .resolve_ordinary_to_primitive_step_outcome(&binding, this_expression)
                    .or_else(|| {
                        self.resolve_static_member_call_outcome_with_context(
                            expression,
                            method_name,
                            self.current_function_name(),
                        )
                    })
                    .or_else(|| {
                        (!static_expression_matches(&materialized, expression))
                            .then(|| {
                                self.resolve_static_member_call_outcome_with_context(
                                    &materialized,
                                    method_name,
                                    self.current_function_name(),
                                )
                            })
                            .flatten()
                    })?;
                let terminal = self.ordinary_to_primitive_outcome_is_terminal(&outcome)?;
                steps.push(OrdinaryToPrimitiveStep { binding, outcome });
                if terminal {
                    return Some(OrdinaryToPrimitivePlan { steps });
                }
                continue;
            }
            if let Some(binding) = self.resolve_member_function_binding(expression, &property) {
                let outcome = self
                    .resolve_ordinary_to_primitive_step_outcome(&binding, this_expression)
                    .or_else(|| {
                        self.resolve_static_member_call_outcome_with_context(
                            expression,
                            method_name,
                            self.current_function_name(),
                        )
                    })?;
                let terminal = self.ordinary_to_primitive_outcome_is_terminal(&outcome)?;
                steps.push(OrdinaryToPrimitiveStep { binding, outcome });
                if terminal {
                    return Some(OrdinaryToPrimitivePlan { steps });
                }
                continue;
            }
        }
        (!steps.is_empty() || own_non_callable_methods == 2)
            .then_some(OrdinaryToPrimitivePlan { steps })
    }

    pub(in crate::backend::direct_wasm) fn static_expression_is_non_object_primitive(
        &self,
        expression: &Expression,
    ) -> Option<bool> {
        match self.infer_value_kind(expression)? {
            StaticValueKind::Number
            | StaticValueKind::BigInt
            | StaticValueKind::String
            | StaticValueKind::Bool
            | StaticValueKind::Null
            | StaticValueKind::Undefined
            | StaticValueKind::Symbol => Some(true),
            StaticValueKind::Object | StaticValueKind::Function => Some(false),
            StaticValueKind::Unknown => None,
        }
    }

    pub(in crate::backend::direct_wasm) fn analyze_ordinary_to_primitive_plan(
        &self,
        plan: &OrdinaryToPrimitivePlan,
    ) -> OrdinaryToPrimitiveAnalysis {
        for step in &plan.steps {
            match &step.outcome {
                StaticEvalOutcome::Throw(_) => return OrdinaryToPrimitiveAnalysis::Throw,
                StaticEvalOutcome::Value(value) => {
                    match self.static_expression_is_non_object_primitive(value) {
                        Some(true) => {
                            if let Some(kind) = self.infer_value_kind(value) {
                                return OrdinaryToPrimitiveAnalysis::Primitive(kind);
                            }
                            return OrdinaryToPrimitiveAnalysis::Unknown;
                        }
                        Some(false) => continue,
                        None => return OrdinaryToPrimitiveAnalysis::Unknown,
                    }
                }
            }
        }
        OrdinaryToPrimitiveAnalysis::TypeError
    }

    pub(in crate::backend::direct_wasm) fn emit_ordinary_to_primitive_from_plan(
        &mut self,
        expression: &Expression,
        plan: &OrdinaryToPrimitivePlan,
        result_local: u32,
    ) -> DirectResult<SymbolToPrimitiveHandling> {
        for step in &plan.steps {
            if let Some(handling) = self.emit_ordinary_to_primitive_step_inline_effects(
                expression,
                &step.binding,
                &step.outcome,
                result_local,
            )? {
                match handling {
                    SymbolToPrimitiveHandling::Handled => return Ok(handling),
                    SymbolToPrimitiveHandling::AlwaysThrows => return Ok(handling),
                    SymbolToPrimitiveHandling::NotHandled => continue,
                }
            }
            if matches!(step.binding, LocalFunctionBinding::Builtin(_)) {
                match &step.outcome {
                    StaticEvalOutcome::Throw(_) => {
                        self.emit_static_eval_outcome(&step.outcome)?;
                        return Ok(SymbolToPrimitiveHandling::AlwaysThrows);
                    }
                    StaticEvalOutcome::Value(value) => {
                        match self.static_expression_is_non_object_primitive(value) {
                            Some(true) => return Ok(SymbolToPrimitiveHandling::Handled),
                            Some(false) => continue,
                            None => return Ok(SymbolToPrimitiveHandling::NotHandled),
                        }
                    }
                }
            }

            if !self.emit_binding_call_result_to_local_with_explicit_this(
                &step.binding,
                &[],
                expression,
                JS_TYPEOF_OBJECT_TAG,
                result_local,
            )? {
                return Ok(SymbolToPrimitiveHandling::NotHandled);
            }
            match &step.outcome {
                StaticEvalOutcome::Throw(_) => {
                    self.emit_check_global_throw_for_user_call()?;
                    return Ok(SymbolToPrimitiveHandling::AlwaysThrows);
                }
                StaticEvalOutcome::Value(value) => {
                    match self.static_expression_is_non_object_primitive(value) {
                        Some(true) => return Ok(SymbolToPrimitiveHandling::Handled),
                        Some(false) => continue,
                        None => return Ok(SymbolToPrimitiveHandling::NotHandled),
                    }
                }
            }
        }
        self.emit_named_error_throw("TypeError")?;
        Ok(SymbolToPrimitiveHandling::AlwaysThrows)
    }

    fn emit_ordinary_to_primitive_step_inline_effects(
        &mut self,
        expression: &Expression,
        binding: &LocalFunctionBinding,
        outcome: &StaticEvalOutcome,
        result_local: u32,
    ) -> DirectResult<Option<SymbolToPrimitiveHandling>> {
        let LocalFunctionBinding::User(function_name) = binding else {
            return Ok(None);
        };
        let Some(user_function) = self.user_function(function_name).cloned() else {
            return Ok(None);
        };
        let rejects = [
            ("params", user_function.visible_param_count() != 0),
            (
                "extra_args",
                !user_function.extra_argument_indices.is_empty(),
            ),
            ("defaults", user_function.has_parameter_defaults()),
            ("patterns", user_function.has_lowered_pattern_parameters()),
            ("async", user_function.is_async()),
            ("generator", user_function.is_generator()),
            (
                "arguments_delete",
                self.user_function_deletes_call_frame_arguments_member(&user_function),
            ),
            (
                "eval",
                self.user_function_mentions_direct_eval(&user_function),
            ),
            (
                "private",
                self.user_function_mentions_private_member_access(&user_function),
            ),
            (
                "captures",
                !self.ordinary_to_primitive_inline_captures_are_safe(&user_function),
            ),
        ];
        if rejects.iter().any(|(_, rejected)| *rejected) {
            return Ok(None);
        }
        let Some(function) = self
            .resolve_registered_function_declaration(&user_function.name)
            .cloned()
        else {
            return Ok(None);
        };
        if !collect_declared_bindings_from_statements_recursive(&function.body).is_empty() {
            return Ok(None);
        }
        let Some((_terminal_statement, effect_statements)) = function.body.split_last() else {
            return Ok(None);
        };
        let value_primitive_status = match outcome {
            StaticEvalOutcome::Value(value) => {
                let Some(status) = self.static_expression_is_non_object_primitive(value) else {
                    return Ok(None);
                };
                Some(status)
            }
            StaticEvalOutcome::Throw(_) => None,
        };

        let call_arguments = Vec::new();
        let arguments_binding = Expression::Array(Vec::new());
        for statement in effect_statements {
            if !self.emit_inline_user_function_effect_statement_with_explicit_call_frame(
                statement,
                &user_function,
                &call_arguments,
                expression,
                &arguments_binding,
                &[],
            )? {
                return Ok(None);
            }
        }

        match outcome {
            StaticEvalOutcome::Throw(throw_value) => {
                self.emit_static_throw_value(throw_value)?;
                Ok(Some(SymbolToPrimitiveHandling::AlwaysThrows))
            }
            StaticEvalOutcome::Value(value) => {
                self.emit_numeric_expression(value)?;
                self.push_local_set(result_local);
                match value_primitive_status {
                    Some(true) => Ok(Some(SymbolToPrimitiveHandling::Handled)),
                    Some(false) => Ok(Some(SymbolToPrimitiveHandling::NotHandled)),
                    None => unreachable!("value primitive status checked before effect emission"),
                }
            }
        }
    }

    fn ordinary_to_primitive_inline_captures_are_safe(&self, user_function: &UserFunction) -> bool {
        self.backend
            .function_registry
            .analysis
            .user_function_capture_bindings
            .get(&user_function.name)
            .is_none_or(|captures| {
                captures.keys().all(|name| {
                    let locally_shadowed_without_capture_slot = name == "this"
                        || name == "new.target"
                        || self.parameter_scope_arguments_local_for(name).is_some()
                        || (self.is_current_arguments_binding_name(name)
                            && self.has_arguments_object())
                        || self.resolve_current_local_binding(name).is_some()
                        || self
                            .state
                            .speculation
                            .static_semantics
                            .has_local_function_binding(name)
                        || (is_internal_user_function_identifier(name)
                            && self.contains_user_function(name))
                        || self.resolve_eval_local_function_hidden_name(name).is_some();
                    name == "assert"
                        || name == "verifyProperty"
                        || name.starts_with("__ayy_class_brand_")
                        || (!locally_shadowed_without_capture_slot
                            && (self.resolve_global_binding_index(name).is_some()
                                || self.global_has_binding(name)
                                || self.global_has_implicit_binding(name)
                                || self.backend.global_has_lexical_binding(name)
                                || self.user_function(name).is_some()
                                || self.backend.global_function_binding(name).is_some()
                                || matches!(
                                    self.resolve_function_binding_from_expression(
                                        &Expression::Identifier(name.clone())
                                    ),
                                    Some(LocalFunctionBinding::User(_))
                                )))
                })
            })
    }

    pub(in crate::backend::direct_wasm) fn emit_effectful_ordinary_to_primitive_addition(
        &mut self,
        left: &Expression,
        right: &Expression,
    ) -> DirectResult<bool> {
        if self.expression_depends_on_active_loop_assignment(left)
            || self.expression_depends_on_active_loop_assignment(right)
        {
            return Ok(false);
        }
        let current_function_name = self.current_function_name();
        let left_symbol_preempts_ordinary =
            self.symbol_to_primitive_preempts_ordinary_to_primitive(left, current_function_name);
        let right_symbol_preempts_ordinary =
            self.symbol_to_primitive_preempts_ordinary_to_primitive(right, current_function_name);
        let left_plan = if left_symbol_preempts_ordinary {
            None
        } else {
            self.resolve_ordinary_to_primitive_plan(left)
        };
        let right_plan = if right_symbol_preempts_ordinary {
            None
        } else {
            self.resolve_ordinary_to_primitive_plan(right)
        };
        let left_eval_throw = matches!(
            self.resolve_terminal_call_expression_outcome(left),
            Some(StaticEvalOutcome::Throw(_))
        );
        let right_eval_throw = matches!(
            self.resolve_terminal_call_expression_outcome(right),
            Some(StaticEvalOutcome::Throw(_))
        );
        let left_analysis = left_plan
            .as_ref()
            .map(|plan| self.analyze_ordinary_to_primitive_plan(plan))
            .unwrap_or(OrdinaryToPrimitiveAnalysis::Unknown);
        let right_analysis = right_plan
            .as_ref()
            .map(|plan| self.analyze_ordinary_to_primitive_plan(plan))
            .unwrap_or(OrdinaryToPrimitiveAnalysis::Unknown);
        let left_symbol_type_error = matches!(
            left_analysis,
            OrdinaryToPrimitiveAnalysis::Primitive(StaticValueKind::Symbol)
        );
        let right_symbol_type_error = matches!(
            right_analysis,
            OrdinaryToPrimitiveAnalysis::Primitive(StaticValueKind::Symbol)
        );
        let left_type_error = left_symbol_type_error
            || matches!(left_analysis, OrdinaryToPrimitiveAnalysis::TypeError);
        let right_type_error = right_symbol_type_error
            || matches!(right_analysis, OrdinaryToPrimitiveAnalysis::TypeError);
        let final_type_error = left_type_error || right_type_error;

        if !(left_eval_throw
            || right_eval_throw
            || matches!(left_analysis, OrdinaryToPrimitiveAnalysis::Throw)
            || matches!(right_analysis, OrdinaryToPrimitiveAnalysis::Throw)
            || final_type_error)
        {
            return Ok(false);
        }

        let left_local = self.allocate_temp_local();
        self.emit_numeric_expression(left)?;
        self.push_local_set(left_local);
        if left_eval_throw {
            self.push_i32_const(JS_UNDEFINED_TAG);
            return Ok(true);
        }

        let right_local = self.allocate_temp_local();
        self.emit_numeric_expression(right)?;
        self.push_local_set(right_local);
        if right_eval_throw {
            self.push_i32_const(JS_UNDEFINED_TAG);
            return Ok(true);
        }

        if let Some(plan) = left_plan.as_ref() {
            match self.emit_ordinary_to_primitive_from_plan(left, plan, left_local)? {
                SymbolToPrimitiveHandling::AlwaysThrows => {
                    self.push_i32_const(JS_UNDEFINED_TAG);
                    return Ok(true);
                }
                SymbolToPrimitiveHandling::Handled => {}
                SymbolToPrimitiveHandling::NotHandled => return Ok(false),
            }
        }

        if let Some(plan) = right_plan.as_ref() {
            match self.emit_ordinary_to_primitive_from_plan(right, plan, right_local)? {
                SymbolToPrimitiveHandling::AlwaysThrows => {
                    self.push_i32_const(JS_UNDEFINED_TAG);
                    return Ok(true);
                }
                SymbolToPrimitiveHandling::Handled => {}
                SymbolToPrimitiveHandling::NotHandled => return Ok(false),
            }
        }

        if left_symbol_type_error || right_symbol_type_error {
            self.emit_named_error_throw("TypeError")?;
            self.push_i32_const(JS_UNDEFINED_TAG);
            return Ok(true);
        }

        Ok(true)
    }

    pub(in crate::backend::direct_wasm) fn emit_effectful_ordinary_to_primitive_numeric(
        &mut self,
        left: &Expression,
        right: &Expression,
    ) -> DirectResult<bool> {
        if self.expression_depends_on_active_loop_assignment(left)
            || self.expression_depends_on_active_loop_assignment(right)
        {
            return Ok(false);
        }

        let current_function_name = self.current_function_name();
        let left_symbol_preempts_ordinary =
            self.symbol_to_primitive_preempts_ordinary_to_primitive(left, current_function_name);
        let right_symbol_preempts_ordinary =
            self.symbol_to_primitive_preempts_ordinary_to_primitive(right, current_function_name);
        let left_plan = if left_symbol_preempts_ordinary {
            None
        } else {
            self.resolve_ordinary_to_primitive_plan(left)
        };
        let right_plan = if right_symbol_preempts_ordinary {
            None
        } else {
            self.resolve_ordinary_to_primitive_plan(right)
        };
        let numeric_hint_argument = Expression::String("number".to_string());
        let left_symbol_type_error = self.symbol_to_primitive_non_callable_type_error(left);
        let right_symbol_type_error = self.symbol_to_primitive_non_callable_type_error(right);
        let left_symbol_terminal =
            self.symbol_to_primitive_callable_terminal_effect(left, &numeric_hint_argument);
        let right_symbol_terminal =
            self.symbol_to_primitive_callable_terminal_effect(right, &numeric_hint_argument);
        let left_analysis = left_plan
            .as_ref()
            .map(|plan| self.analyze_ordinary_to_primitive_plan(plan))
            .unwrap_or(OrdinaryToPrimitiveAnalysis::Unknown);
        let right_analysis = right_plan
            .as_ref()
            .map(|plan| self.analyze_ordinary_to_primitive_plan(plan))
            .unwrap_or(OrdinaryToPrimitiveAnalysis::Unknown);

        let left_type_error = matches!(
            left_analysis,
            OrdinaryToPrimitiveAnalysis::Primitive(StaticValueKind::Symbol)
                | OrdinaryToPrimitiveAnalysis::TypeError
        );
        let right_type_error = matches!(
            right_analysis,
            OrdinaryToPrimitiveAnalysis::Primitive(StaticValueKind::Symbol)
                | OrdinaryToPrimitiveAnalysis::TypeError
        );
        let final_type_error = left_type_error || right_type_error;

        if !(matches!(left_analysis, OrdinaryToPrimitiveAnalysis::Throw)
            || matches!(right_analysis, OrdinaryToPrimitiveAnalysis::Throw)
            || left_symbol_type_error
            || right_symbol_type_error
            || left_symbol_terminal
            || right_symbol_terminal
            || final_type_error)
        {
            return Ok(false);
        }

        let left_local = self.allocate_temp_local();
        self.emit_numeric_expression(left)?;
        self.push_local_set(left_local);
        self.emit_check_global_throw_for_user_call()?;
        let right_local = self.allocate_temp_local();
        self.emit_conditionally_reachable_numeric_expression_to_local(right, right_local)?;
        self.emit_check_global_throw_for_user_call()?;

        if left_symbol_type_error {
            self.emit_named_error_throw("TypeError")?;
            self.push_i32_const(JS_UNDEFINED_TAG);
            return Ok(true);
        }

        if left_symbol_terminal {
            match self
                .emit_effectful_symbol_to_primitive_for_operand(left, &numeric_hint_argument)?
            {
                SymbolToPrimitiveHandling::AlwaysThrows => {
                    self.push_i32_const(JS_UNDEFINED_TAG);
                    return Ok(true);
                }
                SymbolToPrimitiveHandling::Handled | SymbolToPrimitiveHandling::NotHandled => {}
            }
        }

        if let Some(plan) = left_plan.as_ref() {
            match self.emit_ordinary_to_primitive_from_plan(left, plan, left_local)? {
                SymbolToPrimitiveHandling::AlwaysThrows => {
                    self.push_i32_const(JS_UNDEFINED_TAG);
                    return Ok(true);
                }
                SymbolToPrimitiveHandling::Handled => {}
                SymbolToPrimitiveHandling::NotHandled => return Ok(false),
            }
        }

        if left_type_error {
            self.emit_named_error_throw("TypeError")?;
            self.push_i32_const(JS_UNDEFINED_TAG);
            return Ok(true);
        }

        if right_symbol_type_error {
            self.emit_named_error_throw("TypeError")?;
            self.push_i32_const(JS_UNDEFINED_TAG);
            return Ok(true);
        }

        if right_symbol_terminal {
            match self
                .emit_effectful_symbol_to_primitive_for_operand(right, &numeric_hint_argument)?
            {
                SymbolToPrimitiveHandling::AlwaysThrows => {
                    self.push_i32_const(JS_UNDEFINED_TAG);
                    return Ok(true);
                }
                SymbolToPrimitiveHandling::Handled | SymbolToPrimitiveHandling::NotHandled => {}
            }
        }

        if let Some(plan) = right_plan.as_ref() {
            match self.emit_ordinary_to_primitive_from_plan(right, plan, right_local)? {
                SymbolToPrimitiveHandling::AlwaysThrows => {
                    self.push_i32_const(JS_UNDEFINED_TAG);
                    return Ok(true);
                }
                SymbolToPrimitiveHandling::Handled => {}
                SymbolToPrimitiveHandling::NotHandled => return Ok(false),
            }
        }

        if right_type_error {
            self.emit_named_error_throw("TypeError")?;
            self.push_i32_const(JS_UNDEFINED_TAG);
            return Ok(true);
        }

        Ok(false)
    }
}
