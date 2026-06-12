use super::*;

impl<'a> FunctionCompiler<'a> {
    fn lowered_pattern_inline_argument_is_generator_call(&self, expression: &Expression) -> bool {
        let Expression::Call { callee, arguments } = expression else {
            return false;
        };
        if !arguments.is_empty() {
            return false;
        }
        let Expression::Identifier(name) = callee.as_ref() else {
            return false;
        };
        self.resolve_registered_function_declaration(name)
            .is_some_and(|function| function.kind.is_generator() && !function.kind.is_async())
    }

    fn lowered_pattern_inline_argument_is_safe(&self, expression: &Expression) -> bool {
        if self.inline_safe_argument_expression(expression) {
            return true;
        }
        if self.lowered_pattern_inline_argument_is_generator_call(expression) {
            return true;
        }
        if let Expression::Identifier(name) = expression
            && let Some(binding_name) = self.resolve_local_array_iterator_binding_name(name)
        {
            return self
                .state
                .speculation
                .static_semantics
                .local_array_iterator_binding(&binding_name)
                .is_some();
        }
        false
    }

    fn lowered_pattern_inline_captures_are_safe(&self, user_function: &UserFunction) -> bool {
        self.backend
            .function_registry
            .analysis
            .user_function_capture_bindings
            .get(&user_function.name)
            .is_none_or(|captures| {
                captures.keys().all(|name| {
                    let safe = name == "assert"
                        // verifyProperty is dispatched by name at every call
                        // site (emit_verify_property_call), never through a
                        // binding, so capturing it cannot change resolution.
                        || name == "verifyProperty"
                        || name.starts_with("__ayy_class_brand_")
                        // Captured user-function bindings (e.g. harness
                        // helpers) resolve to the same program function at
                        // any unshadowed call site.
                        || (self.resolve_current_local_binding(name).is_none()
                            && (self.user_function(name).is_some()
                                || self.backend.global_function_binding(name).is_some()
                                || self
                                    .resolve_function_binding_from_expression(
                                        &Expression::Identifier(name.clone())
                                    )
                                    .is_some()));
                    if !safe && crate::ayy_env_flag!("AYY_TRACE_USER_CALLS") {
                        eprintln!(
                            "lowered_pattern_inline:capture-unsafe target={} capture={name} local={} user_fn={}",
                            user_function.name,
                            self.resolve_current_local_binding(name).is_some(),
                            self.user_function(name).is_some()
                        );
                    }
                    safe
                })
            })
    }

    /// Bound capture slots are inline-safe when every captured binding either
    /// is a class brand/assert helper or its resolved slot is the same-named
    /// live global at the call site, so inline emission reads and writes the
    /// exact binding the standalone closure call would.
    pub(in crate::backend::direct_wasm) fn bound_capture_slots_are_inline_lowered_pattern_safe(
        &self,
        user_function: &UserFunction,
        capture_slots: &BTreeMap<String, String>,
    ) -> bool {
        let trace_user_calls = crate::ayy_env_flag!("AYY_TRACE_USER_CALLS");
        self.backend
            .function_registry
            .analysis
            .user_function_capture_bindings
            .get(&user_function.name)
            .is_none_or(|captures| {
                captures.keys().all(|name| {
                    if name == "assert"
                        || name == "verifyProperty"
                        || name.starts_with("__ayy_class_brand_")
                    {
                        return true;
                    }
                    if self.resolve_current_local_binding(name).is_some() {
                        if trace_user_calls {
                            eprintln!(
                                "bound_capture_inline_safe:reject-local target={} capture={name}",
                                user_function.name
                            );
                        }
                        return false;
                    }
                    // Captured user-function bindings (e.g. harness helpers)
                    // resolve to the same program function at any call site.
                    if matches!(
                        self.resolve_function_binding_from_expression(&Expression::Identifier(
                            name.clone()
                        )),
                        Some(LocalFunctionBinding::User(_))
                    ) {
                        return true;
                    }
                    let slot_safe = capture_slots.get(name).is_some_and(|slot| {
                        if slot == name {
                            return true;
                        }
                        // Closure-slot snapshots remember the live binding
                        // they were taken from; the capture stays inline-safe
                        // when that source is the same-named binding.
                        let source = self
                            .state
                            .speculation
                            .static_semantics
                            .capture_slot_source_bindings
                            .get(slot)
                            .cloned()
                            .unwrap_or_else(|| slot.clone());
                        self.capture_slot_live_source_binding_name(&source) == *name
                    }) && self.resolve_global_binding_index(name).is_some();
                    if !slot_safe && trace_user_calls {
                        eprintln!(
                            "bound_capture_inline_safe:reject-slot target={} capture={name} slot={:?} global={}",
                            user_function.name,
                            capture_slots.get(name),
                            self.resolve_global_binding_index(name).is_some()
                        );
                    }
                    slot_safe
                })
            })
    }

    /// Resolves parameter defaults for the lowered-pattern inline path.
    /// Returns `None` when the defaults cannot be replayed faithfully at the
    /// call site; otherwise returns the `(parameter, default)` pairs that must
    /// be bound through prepended `Let` statements so each default evaluates
    /// exactly once, in order, at function entry.
    fn lowered_pattern_inline_defaulted_parameter_lets(
        &self,
        user_function: &UserFunction,
        arguments: &[Expression],
    ) -> Option<Vec<(String, Expression)>> {
        if !user_function.has_parameter_defaults() {
            return Some(Vec::new());
        }
        let function = self.resolve_registered_function_declaration(&user_function.name)?;
        let mut defaulted_parameter_lets = Vec::new();
        for (index, parameter) in function.params.iter().enumerate() {
            let Some(default) = parameter.default.as_ref() else {
                continue;
            };
            let use_default = match arguments.get(index) {
                None => true,
                Some(argument) => {
                    match self.materialize_static_expression(argument) {
                        Expression::Undefined => true,
                        materialized => {
                            // The default must be skipped only when the
                            // argument is statically known to not be
                            // undefined; otherwise the choice is a runtime
                            // decision this path cannot represent.
                            let kind = self.infer_value_kind(&materialized);
                            match kind {
                                Some(StaticValueKind::Unknown) | None => return None,
                                Some(StaticValueKind::Undefined) => true,
                                Some(_) => false,
                            }
                        }
                    }
                }
            };
            if !use_default {
                continue;
            }
            // Only compiler-generated parameter names can be rebound by a
            // prepended `Let` without colliding with call-site bindings, and
            // `this` inside a default would resolve to the caller's receiver.
            if !parameter.name.starts_with("__ayy_param_")
                || expression_references_this(default)
                || self.inline_argument_mentions_shadowed_implicit_global(default)
            {
                return None;
            }
            defaulted_parameter_lets.push((parameter.name.clone(), default.clone()));
        }
        Some(defaulted_parameter_lets)
    }

    /// Every nonlocal binding referenced by the function body (and by any
    /// user function the body references, transitively) must resolve at the
    /// call site to the same-named global so inline emission dispatches
    /// identifier callees and nonlocal reads exactly as the standalone
    /// closure call would.
    fn lowered_pattern_inline_nonlocal_references_resolve_at_call_site(
        &self,
        function_name: &str,
        visited: &mut HashSet<String>,
    ) -> bool {
        if !visited.insert(function_name.to_string()) {
            return true;
        }
        let Some(function) = self.resolve_registered_function_declaration(function_name) else {
            return false;
        };
        let Some(user_function) = self.user_function(function_name) else {
            return false;
        };
        collect_referenced_binding_names_from_statements(&function.body)
            .iter()
            .all(|name| {
                let source_name = scoped_binding_source_name(name).unwrap_or(name);
                if source_name == function.name
                    || user_function
                        .params
                        .iter()
                        .any(|param| param == source_name)
                    || user_function.scope_bindings.iter().any(|binding| {
                        scoped_binding_source_name(binding).unwrap_or(binding) == source_name
                    })
                {
                    return true;
                }
                let is_nonlocal = self.global_has_binding(source_name)
                    || self.global_has_implicit_binding(source_name)
                    || self.user_function(source_name).is_some();
                if !is_nonlocal {
                    return true;
                }
                if self.resolve_current_local_binding(source_name).is_some()
                    || self
                        .resolve_user_function_capture_hidden_name(source_name)
                        .is_some()
                {
                    return false;
                }
                if self.user_function(source_name).is_some() {
                    return self.lowered_pattern_inline_nonlocal_references_resolve_at_call_site(
                        source_name,
                        visited,
                    );
                }
                true
            })
    }

    fn lowered_pattern_inline_argument_reads_nonlocal_binding(
        &self,
        expression: &Expression,
    ) -> bool {
        if self.lowered_pattern_inline_argument_is_generator_call(expression) {
            return false;
        }
        if let Expression::Identifier(name) = expression
            && self
                .resolve_local_array_iterator_binding_name(name)
                .is_some()
        {
            return false;
        }
        let mut referenced_names = HashSet::new();
        collect_referenced_binding_names_from_expression(expression, &mut referenced_names);
        referenced_names.iter().any(|name| {
            let source_name = scoped_binding_source_name(name).unwrap_or(name);
            self.resolve_current_local_binding(source_name).is_none()
                && (self.global_has_binding(source_name)
                    || self.global_has_implicit_binding(source_name)
                    || self
                        .resolve_user_function_capture_hidden_name(source_name)
                        .is_some())
        })
    }

    fn lowered_pattern_inline_user_function_reads_nonlocal_binding(
        &self,
        function_name: &str,
    ) -> bool {
        let Some(function) = self.resolve_registered_function_declaration(function_name) else {
            return false;
        };
        let Some(user_function) = self.user_function(function_name) else {
            return false;
        };
        collect_referenced_binding_names_from_statements(&function.body)
            .iter()
            .any(|name| {
                let source_name = scoped_binding_source_name(name).unwrap_or(name);
                source_name != function.name
                    && !user_function
                        .params
                        .iter()
                        .any(|param| param == source_name)
                    && !user_function.scope_bindings.iter().any(|binding| {
                        scoped_binding_source_name(binding).unwrap_or(binding) == source_name
                    })
                    && (self.global_has_binding(source_name)
                        || self.global_has_implicit_binding(source_name)
                        || self
                            .resolve_user_function_capture_hidden_name(source_name)
                            .is_some())
            })
    }

    fn lowered_pattern_inline_body_references_nonlocal_user_function(
        &self,
        statements: &[Statement],
    ) -> bool {
        collect_referenced_binding_names_from_statements(statements)
            .iter()
            .any(|name| {
                if matches!(
                    name.as_str(),
                    "__assert"
                        | "__assertSameValue"
                        | "__assertNotSameValue"
                        | "__ayyAssertCompareArray"
                ) {
                    return false;
                }
                self.user_function(name).is_some()
                    && self.lowered_pattern_inline_user_function_reads_nonlocal_binding(name)
            })
    }

    fn lowered_pattern_inline_body_references_call_frame_arguments(
        &self,
        user_function: &UserFunction,
        statements: &[Statement],
    ) -> bool {
        if user_function.lexical_this
            || user_function.params.iter().any(|param| {
                param == "arguments"
                    || scoped_binding_source_name(param)
                        .is_some_and(|source_name| source_name == "arguments")
            })
            || user_function.body_declares_arguments_binding
        {
            return false;
        }
        collect_referenced_binding_names_from_statements(statements)
            .iter()
            .any(|name| {
                name == "arguments"
                    || scoped_binding_source_name(name)
                        .is_some_and(|source_name| source_name == "arguments")
            })
    }

    fn lowered_pattern_inline_statement_is_supported(statement: &Statement) -> bool {
        match statement {
            Statement::Var { .. }
            | Statement::Let { .. }
            | Statement::Assign { .. }
            | Statement::AssignMember { .. }
            | Statement::Expression(_)
            | Statement::Print { .. }
            | Statement::Throw(_) => true,
            Statement::Block { body }
            | Statement::Declaration { body }
            | Statement::Labeled { body, .. } => body
                .iter()
                .all(Self::lowered_pattern_inline_statement_is_supported),
            Statement::If {
                then_branch,
                else_branch,
                ..
            } => then_branch
                .iter()
                .chain(else_branch)
                .all(Self::lowered_pattern_inline_statement_is_supported),
            Statement::While {
                labels,
                body,
                break_hook,
                ..
            } => {
                labels.is_empty()
                    && break_hook.is_none()
                    && body
                        .iter()
                        .all(Self::lowered_pattern_inline_statement_is_supported)
            }
            Statement::Return(_)
            | Statement::Yield { .. }
            | Statement::YieldDelegate { .. }
            | Statement::Break { .. }
            | Statement::Continue { .. }
            | Statement::With { .. }
            | Statement::DoWhile { .. }
            | Statement::For { .. }
            | Statement::Try { .. }
            | Statement::Switch { .. } => false,
        }
    }

    fn lowered_pattern_inline_expression_reads_static_member_getter(
        &self,
        expression: &Expression,
    ) -> bool {
        match expression {
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
            | Expression::Update { .. } => false,
            Expression::Array(elements) => elements.iter().any(|element| match element {
                ArrayElement::Expression(expression) | ArrayElement::Spread(expression) => {
                    self.lowered_pattern_inline_expression_reads_static_member_getter(expression)
                }
            }),
            Expression::Object(entries) => entries.iter().any(|entry| match entry {
                ObjectEntry::Data { key, value } => {
                    self.lowered_pattern_inline_expression_reads_static_member_getter(key)
                        || self.lowered_pattern_inline_expression_reads_static_member_getter(value)
                }
                ObjectEntry::Getter { key, getter } => {
                    self.lowered_pattern_inline_expression_reads_static_member_getter(key)
                        || self.lowered_pattern_inline_expression_reads_static_member_getter(getter)
                }
                ObjectEntry::Setter { key, setter } => {
                    self.lowered_pattern_inline_expression_reads_static_member_getter(key)
                        || self.lowered_pattern_inline_expression_reads_static_member_getter(setter)
                }
                ObjectEntry::Spread(expression) => {
                    self.lowered_pattern_inline_expression_reads_static_member_getter(expression)
                }
            }),
            Expression::Member { object, property } => {
                self.resolve_member_getter_binding(object, property)
                    .is_some()
                    || self.lowered_pattern_inline_expression_reads_static_member_getter(object)
                    || self.lowered_pattern_inline_expression_reads_static_member_getter(property)
            }
            Expression::SuperMember { property } => {
                self.lowered_pattern_inline_expression_reads_static_member_getter(property)
            }
            Expression::Assign { value, .. } | Expression::Await(value) => {
                self.lowered_pattern_inline_expression_reads_static_member_getter(value)
            }
            Expression::AssignMember {
                object,
                property,
                value,
            } => {
                self.resolve_member_getter_binding(object, property)
                    .is_some()
                    || self.lowered_pattern_inline_expression_reads_static_member_getter(object)
                    || self.lowered_pattern_inline_expression_reads_static_member_getter(property)
                    || self.lowered_pattern_inline_expression_reads_static_member_getter(value)
            }
            Expression::AssignSuperMember { property, value } => {
                self.lowered_pattern_inline_expression_reads_static_member_getter(property)
                    || self.lowered_pattern_inline_expression_reads_static_member_getter(value)
            }
            Expression::EnumerateKeys(value)
            | Expression::GetIterator(value)
            | Expression::IteratorClose(value)
            | Expression::Unary {
                expression: value, ..
            } => self.lowered_pattern_inline_expression_reads_static_member_getter(value),
            Expression::Binary { left, right, .. } => {
                self.lowered_pattern_inline_expression_reads_static_member_getter(left)
                    || self.lowered_pattern_inline_expression_reads_static_member_getter(right)
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                self.lowered_pattern_inline_expression_reads_static_member_getter(condition)
                    || self.lowered_pattern_inline_expression_reads_static_member_getter(
                        then_expression,
                    )
                    || self.lowered_pattern_inline_expression_reads_static_member_getter(
                        else_expression,
                    )
            }
            Expression::Sequence(expressions) => expressions.iter().any(|expression| {
                self.lowered_pattern_inline_expression_reads_static_member_getter(expression)
            }),
            Expression::Call { callee, arguments }
            | Expression::SuperCall { callee, arguments }
            | Expression::New { callee, arguments } => {
                self.lowered_pattern_inline_expression_reads_static_member_getter(callee)
                    || arguments.iter().any(|argument| {
                        self.lowered_pattern_inline_expression_reads_static_member_getter(
                            argument.expression(),
                        )
                    })
            }
        }
    }

    fn lowered_pattern_inline_statement_reads_static_member_getter(
        &self,
        statement: &Statement,
    ) -> bool {
        match statement {
            Statement::Declaration { body }
            | Statement::Block { body }
            | Statement::Labeled { body, .. } => body.iter().any(|statement| {
                self.lowered_pattern_inline_statement_reads_static_member_getter(statement)
            }),
            Statement::Var { value, .. }
            | Statement::Let { value, .. }
            | Statement::Assign { value, .. }
            | Statement::Expression(value)
            | Statement::Throw(value)
            | Statement::Return(value)
            | Statement::Yield { value }
            | Statement::YieldDelegate { value } => {
                self.lowered_pattern_inline_expression_reads_static_member_getter(value)
            }
            Statement::AssignMember {
                object,
                property,
                value,
            } => {
                self.resolve_member_getter_binding(object, property)
                    .is_some()
                    || self.lowered_pattern_inline_expression_reads_static_member_getter(object)
                    || self.lowered_pattern_inline_expression_reads_static_member_getter(property)
                    || self.lowered_pattern_inline_expression_reads_static_member_getter(value)
            }
            Statement::Print { values } => values.iter().any(|value| {
                self.lowered_pattern_inline_expression_reads_static_member_getter(value)
            }),
            Statement::Break { .. } | Statement::Continue { .. } => false,
            Statement::With { object, body } => {
                self.lowered_pattern_inline_expression_reads_static_member_getter(object)
                    || body.iter().any(|statement| {
                        self.lowered_pattern_inline_statement_reads_static_member_getter(statement)
                    })
            }
            Statement::If {
                condition,
                then_branch,
                else_branch,
            } => {
                self.lowered_pattern_inline_expression_reads_static_member_getter(condition)
                    || then_branch.iter().any(|statement| {
                        self.lowered_pattern_inline_statement_reads_static_member_getter(statement)
                    })
                    || else_branch.iter().any(|statement| {
                        self.lowered_pattern_inline_statement_reads_static_member_getter(statement)
                    })
            }
            Statement::Try {
                body,
                catch_setup,
                catch_body,
                ..
            } => body
                .iter()
                .chain(catch_setup)
                .chain(catch_body)
                .any(|statement| {
                    self.lowered_pattern_inline_statement_reads_static_member_getter(statement)
                }),
            Statement::Switch {
                discriminant,
                cases,
                ..
            } => {
                self.lowered_pattern_inline_expression_reads_static_member_getter(discriminant)
                    || cases.iter().any(|case| {
                        case.test.as_ref().is_some_and(|test| {
                            self.lowered_pattern_inline_expression_reads_static_member_getter(test)
                        }) || case.body.iter().any(|statement| {
                            self.lowered_pattern_inline_statement_reads_static_member_getter(
                                statement,
                            )
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
                    self.lowered_pattern_inline_statement_reads_static_member_getter(statement)
                }) || condition.as_ref().is_some_and(|condition| {
                    self.lowered_pattern_inline_expression_reads_static_member_getter(condition)
                }) || update.as_ref().is_some_and(|update| {
                    self.lowered_pattern_inline_expression_reads_static_member_getter(update)
                }) || break_hook.as_ref().is_some_and(|break_hook| {
                    self.lowered_pattern_inline_expression_reads_static_member_getter(break_hook)
                }) || body.iter().any(|statement| {
                    self.lowered_pattern_inline_statement_reads_static_member_getter(statement)
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
                self.lowered_pattern_inline_expression_reads_static_member_getter(condition)
                    || break_hook.as_ref().is_some_and(|break_hook| {
                        self.lowered_pattern_inline_expression_reads_static_member_getter(
                            break_hook,
                        )
                    })
                    || body.iter().any(|statement| {
                        self.lowered_pattern_inline_statement_reads_static_member_getter(statement)
                    })
            }
        }
    }

    pub(in crate::backend::direct_wasm) fn emit_inline_lowered_pattern_user_function_with_arguments(
        &mut self,
        user_function: &UserFunction,
        arguments: &[Expression],
        this_expression: &Expression,
    ) -> DirectResult<bool> {
        self.emit_inline_lowered_pattern_user_function_with_arguments_impl(
            user_function,
            arguments,
            this_expression,
            false,
        )
    }

    /// Variant for call sites that have already proven the function's bound
    /// capture slots alias call-site-visible bindings (see
    /// `bound_capture_slots_are_inline_lowered_pattern_safe`), bypassing the
    /// name-based capture safety gate.
    pub(in crate::backend::direct_wasm) fn emit_inline_lowered_pattern_user_function_with_validated_captures(
        &mut self,
        user_function: &UserFunction,
        arguments: &[Expression],
        this_expression: &Expression,
    ) -> DirectResult<bool> {
        self.emit_inline_lowered_pattern_user_function_with_arguments_impl(
            user_function,
            arguments,
            this_expression,
            true,
        )
    }

    fn emit_inline_lowered_pattern_user_function_with_arguments_impl(
        &mut self,
        user_function: &UserFunction,
        arguments: &[Expression],
        this_expression: &Expression,
        captures_validated: bool,
    ) -> DirectResult<bool> {
        let trace_user_calls = crate::ayy_env_flag!("AYY_TRACE_USER_CALLS");
        let consumes_parameter_iterator = !self
            .user_function_parameter_iterator_consumption_indices(user_function)
            .is_empty();
        if trace_user_calls {
            eprintln!(
                "lowered_pattern_inline:check target={} lowered={} consumes_iterator={} args={arguments:?}",
                user_function.name,
                user_function.has_lowered_pattern_parameters(),
                consumes_parameter_iterator
            );
        }
        let defaulted_parameter_lets =
            self.lowered_pattern_inline_defaulted_parameter_lets(user_function, arguments);
        // Identifier callees and references to captured user functions are
        // safe to emit inline when every nonlocal name involved resolves at
        // the call site to the same-named global.
        let nonlocal_references_resolve = self
            .lowered_pattern_inline_nonlocal_references_resolve_at_call_site(
                &user_function.name,
                &mut HashSet::new(),
            );
        if !(user_function.has_lowered_pattern_parameters() || consumes_parameter_iterator)
            || user_function.is_async()
            || user_function.is_generator()
            || defaulted_parameter_lets.is_none()
            || self.user_function_mentions_direct_eval(user_function)
            || (self.user_function_contains_identifier_callee_call(user_function)
                && !nonlocal_references_resolve)
            || self.user_function_may_read_restricted_function_property(user_function)
            || !(captures_validated || self.lowered_pattern_inline_captures_are_safe(user_function))
            || (self.user_function_references_captured_user_function(user_function)
                && !nonlocal_references_resolve)
            || !user_function.extra_argument_indices.is_empty()
            || !self.inline_safe_argument_expression(this_expression)
            || !arguments
                .iter()
                .all(|argument| self.lowered_pattern_inline_argument_is_safe(argument))
            || self.inline_argument_mentions_shadowed_implicit_global(this_expression)
            || arguments
                .iter()
                .any(|argument| self.inline_argument_mentions_shadowed_implicit_global(argument))
            || arguments.iter().any(|argument| {
                self.lowered_pattern_inline_argument_reads_nonlocal_binding(argument)
            })
        {
            if trace_user_calls {
                eprintln!(
                    "lowered_pattern_inline:reject target={} async={} generator={} defaults={} private={} eval={} identifier_callee={} restricted={} captures={} captured_ref={} extra_args={} this_safe={} args_safe={} this_shadow={} args_shadow={} args_nonlocal={}",
                    user_function.name,
                    user_function.is_async(),
                    user_function.is_generator(),
                    user_function.has_parameter_defaults(),
                    self.user_function_mentions_private_member_access(user_function),
                    self.user_function_mentions_direct_eval(user_function),
                    self.user_function_contains_identifier_callee_call(user_function),
                    self.user_function_may_read_restricted_function_property(user_function),
                    !self.lowered_pattern_inline_captures_are_safe(user_function),
                    self.user_function_references_captured_user_function(user_function),
                    !user_function.extra_argument_indices.is_empty(),
                    self.inline_safe_argument_expression(this_expression),
                    arguments
                        .iter()
                        .all(|argument| self.lowered_pattern_inline_argument_is_safe(argument)),
                    self.inline_argument_mentions_shadowed_implicit_global(this_expression),
                    arguments
                        .iter()
                        .any(|argument| self
                            .inline_argument_mentions_shadowed_implicit_global(argument)),
                    arguments.iter().any(|argument| self
                        .lowered_pattern_inline_argument_reads_nonlocal_binding(argument))
                );
            }
            return Ok(false);
        }
        let Some(function) = self
            .resolve_registered_function_declaration(&user_function.name)
            .cloned()
        else {
            return Ok(false);
        };
        if self.lowered_pattern_inline_body_references_call_frame_arguments(
            user_function,
            &function.body,
        ) {
            if trace_user_calls {
                eprintln!(
                    "lowered_pattern_inline:reject-arguments target={}",
                    user_function.name
                );
            }
            return Ok(false);
        }
        if self.lowered_pattern_inline_body_references_nonlocal_user_function(&function.body)
            && !nonlocal_references_resolve
        {
            if trace_user_calls {
                eprintln!(
                    "lowered_pattern_inline:reject-nonlocal-user-function target={}",
                    user_function.name
                );
            }
            return Ok(false);
        }
        if !function
            .body
            .iter()
            .all(Self::lowered_pattern_inline_statement_is_supported)
        {
            if trace_user_calls {
                eprintln!(
                    "lowered_pattern_inline:reject-body target={}",
                    user_function.name
                );
            }
            return Ok(false);
        }

        let defaulted_parameter_lets =
            defaulted_parameter_lets.expect("defaulted parameter lets were validated above");
        let mut bindings = HashMap::new();
        for (index, parameter) in function.params.iter().enumerate() {
            if defaulted_parameter_lets
                .iter()
                .any(|(name, _)| name == &parameter.name)
            {
                // Bound through a prepended `Let` so the default expression
                // evaluates exactly once at function entry.
                continue;
            }
            let value = if parameter.rest {
                Expression::Array(
                    arguments
                        .iter()
                        .skip(index)
                        .cloned()
                        .map(ArrayElement::Expression)
                        .collect(),
                )
            } else {
                arguments
                    .get(index)
                    .cloned()
                    .unwrap_or(Expression::Undefined)
            };
            bindings.insert(parameter.name.clone(), value);
        }
        let body = defaulted_parameter_lets
            .into_iter()
            .map(|(name, default)| Statement::Let {
                name,
                mutable: true,
                value: default,
            })
            .chain(
                function
                    .body
                    .iter()
                    .map(|statement| self.substitute_statement_bindings(statement, &bindings)),
            )
            .collect::<Vec<_>>();
        if body.iter().any(|statement| {
            self.lowered_pattern_inline_statement_reads_static_member_getter(statement)
        }) {
            if trace_user_calls {
                eprintln!(
                    "lowered_pattern_inline:reject-static-getter target={}",
                    user_function.name
                );
            }
            return Ok(false);
        }

        self.emit_numeric_expression(this_expression)?;
        self.state.emission.output.instructions.push(0x1a);

        self.with_user_function_execution_context(user_function, |compiler| {
            if trace_user_calls {
                eprintln!(
                    "lowered_pattern_inline:emit target={} statements={}",
                    user_function.name,
                    body.len()
                );
            }
            if compiler.emit_static_lowered_pattern_inline_body(&body)? {
                return Ok(true);
            }
            compiler.push_i32_const(JS_UNDEFINED_TAG);
            Ok(true)
        })
    }

    pub(in crate::backend::direct_wasm) fn emit_inline_user_function_summary_with_arguments(
        &mut self,
        user_function: &UserFunction,
        arguments: &[Expression],
    ) -> DirectResult<bool> {
        let call_arguments = arguments
            .iter()
            .cloned()
            .map(CallArgument::Expression)
            .collect::<Vec<_>>();

        if let Some(summary) = user_function.inline_summary.as_ref()
            && !self.user_function_contains_local_declaration(user_function)
            && !self
                .user_function_creates_descriptor_binding_with_arguments(user_function, arguments)
        {
            self.emit_inline_summary_with_call_arguments(user_function, summary, &call_arguments)?;
            return Ok(true);
        }

        let Some(function) = self
            .resolve_registered_function_declaration(&user_function.name)
            .cloned()
        else {
            return Ok(false);
        };
        let Some((terminal_statement, effect_statements)) = function.body.split_last() else {
            return Ok(false);
        };

        self.with_user_function_execution_context(user_function, |compiler| {
            for statement in effect_statements {
                if !compiler.emit_inline_user_function_effect_statement(
                    statement,
                    user_function,
                    &call_arguments,
                )? {
                    return Ok(false);
                }
            }
            compiler.emit_inline_user_function_terminal_statement(
                terminal_statement,
                user_function,
                &call_arguments,
            )
        })
    }
}
