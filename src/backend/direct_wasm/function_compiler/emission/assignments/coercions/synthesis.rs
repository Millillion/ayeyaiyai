use super::*;

const MS_PER_SECOND: f64 = 1_000.0;
const MS_PER_MINUTE: f64 = 60.0 * MS_PER_SECOND;
const MS_PER_HOUR: f64 = 60.0 * MS_PER_MINUTE;
const MS_PER_DAY: f64 = 24.0 * MS_PER_HOUR;

fn days_from_civil(year: i32, month: u32, day: u32) -> i64 {
    let year = year - i32::from(month <= 2);
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let year_of_era = year - era * 400;
    let month = month as i32;
    let day = day as i32;
    let day_of_year = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    (era * 146097 + day_of_era - 719468) as i64
}

fn civil_from_days(days: i64) -> (i32, u32, u32) {
    let days = days + 719468;
    let era = if days >= 0 { days } else { days - 146096 } / 146097;
    let day_of_era = days - era * 146097;
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36524 - day_of_era / 146096) / 365;
    let mut year = year_of_era as i32 + era as i32 * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    year += i32::from(month <= 2);
    (year, month as u32, day as u32)
}

impl<'a> FunctionCompiler<'a> {
    pub(in crate::backend::direct_wasm) fn constructor_callee_inherits_from_builtin_prototype(
        &self,
        callee: &Expression,
        arguments: &[CallArgument],
        constructor_name: &str,
    ) -> bool {
        let target_prototype = Self::prototype_member_expression(constructor_name);
        if self.expression_inherits_from_prototype_for_instanceof(
            &Expression::New {
                callee: Box::new(callee.clone()),
                arguments: arguments.to_vec(),
            },
            &target_prototype,
        ) {
            return true;
        }
        let Expression::Identifier(name) = callee else {
            return false;
        };
        self.expression_inherits_from_prototype_for_instanceof(
            &Self::prototype_member_expression(name),
            &target_prototype,
        )
    }

    fn function_constructor_builtin_name(name: &str) -> Option<&'static str> {
        match name {
            "Function" => Some("Function"),
            "GeneratorFunction" => Some("GeneratorFunction"),
            "AsyncFunction" => Some("AsyncFunction"),
            "AsyncGeneratorFunction" => Some("AsyncGeneratorFunction"),
            _ => None,
        }
    }

    fn function_constructor_builtin_name_from_binding(
        binding: &LocalFunctionBinding,
    ) -> Option<&'static str> {
        let LocalFunctionBinding::Builtin(name) = binding else {
            return None;
        };
        Self::function_constructor_builtin_name(name)
    }

    fn expression_resolves_to_function_constructor_builtin(
        &self,
        expression: &Expression,
    ) -> Option<&'static str> {
        let trace = std::env::var_os("AYY_TRACE_CONSTRUCTED_FUNCTIONS").is_some();
        if let Expression::Identifier(name) = expression
            && let Some(constructor_name) = Self::function_constructor_builtin_name(name)
            && (self.is_unshadowed_builtin_identifier(name)
                || self.infer_value_kind(expression) == Some(StaticValueKind::Function))
        {
            if trace {
                eprintln!(
                    "constructed_function_ctor:builtin_direct expression={expression:?} constructor={constructor_name}"
                );
            }
            return Some(constructor_name);
        }
        if let Expression::Identifier(name) = expression {
            let local_binding = self
                .resolve_current_local_binding(name)
                .and_then(|(resolved_name, _)| {
                    self.state
                        .speculation
                        .static_semantics
                        .local_function_binding(&resolved_name)
                })
                .or_else(|| {
                    self.state
                        .speculation
                        .static_semantics
                        .local_function_binding(name)
                });
            if let Some(constructor_name) = local_binding
                .and_then(Self::function_constructor_builtin_name_from_binding)
                .or_else(|| {
                    self.backend
                        .global_function_binding(name)
                        .and_then(Self::function_constructor_builtin_name_from_binding)
                })
            {
                if trace {
                    eprintln!(
                        "constructed_function_ctor:builtin_function_binding expression={expression:?} constructor={constructor_name}"
                    );
                }
                return Some(constructor_name);
            }
        }
        if trace && let Expression::Identifier(name) = expression {
            let global_kind = self
                .global_binding_kind(name)
                .and_then(StaticValueKind::as_typeof_str)
                .unwrap_or("unknown");
            eprintln!(
                "constructed_function_ctor:builtin_probe expression={expression:?} local_value={:?} global_value={:?} global_kind={global_kind} global_function={:?} capture_source={:?}",
                self.state
                    .speculation
                    .static_semantics
                    .local_value_binding(name),
                self.global_value_binding(name),
                self.backend.global_function_binding(name),
                self.resolve_capture_hidden_source_binding_name(name)
            );
        }
        let resolved = self
            .resolve_bound_alias_expression(expression)
            .filter(|resolved| !static_expression_matches(resolved, expression))
            .or_else(|| {
                let materialized = self.materialize_static_expression(expression);
                (!static_expression_matches(&materialized, expression)).then_some(materialized)
            })?;
        match resolved {
            Expression::Identifier(name) => {
                let constructor_name =
                    Self::function_constructor_builtin_name(&name).filter(|_| {
                        self.is_unshadowed_builtin_identifier(&name)
                            || self.infer_value_kind(&Expression::Identifier(name.clone()))
                                == Some(StaticValueKind::Function)
                            || static_expression_matches(
                                &self.materialize_static_expression(expression),
                                &Expression::Identifier(name.clone()),
                            )
                    });
                if trace {
                    eprintln!(
                        "constructed_function_ctor:builtin_resolved expression={expression:?} resolved=Identifier({name:?}) constructor={constructor_name:?}"
                    );
                }
                constructor_name
            }
            _ => None,
        }
    }

    fn resolve_derived_constructed_function_constructor_name(
        &self,
        function_name: &str,
        arguments: &[CallArgument],
        depth: usize,
    ) -> Option<&'static str> {
        let trace = std::env::var_os("AYY_TRACE_CONSTRUCTED_FUNCTIONS").is_some();
        if depth > 16 {
            return None;
        }
        let user_function = self.user_function(function_name).or_else(|| {
            self.backend
                .function_registry
                .catalog
                .user_function(function_name)
        })?;
        if !self.user_function_is_derived_constructor(user_function) {
            if trace {
                eprintln!(
                    "constructed_function_ctor:derived_candidate_not_derived function={function_name}"
                );
            }
            return None;
        }
        let Some((super_callee, super_arguments)) =
            self.resolve_derived_constructor_super_call(user_function)
        else {
            if trace {
                eprintln!(
                    "constructed_function_ctor:derived_candidate_no_super function={function_name}"
                );
            }
            return None;
        };
        let expanded_arguments = self.expand_call_arguments(arguments);
        let this_binding = Expression::Identifier(Self::STATIC_NEW_THIS_BINDING.to_string());
        let arguments_binding = Expression::Array(
            expanded_arguments
                .iter()
                .cloned()
                .map(crate::ir::hir::ArrayElement::Expression)
                .collect(),
        );
        let substituted_callee = self.substitute_constructor_call_frame_bindings_with_rest(
            super_callee,
            user_function,
            arguments,
            &this_binding,
            &arguments_binding,
        );
        let resolved_callee = self
            .resolve_bound_alias_expression(&substituted_callee)
            .or_else(|| match &substituted_callee {
                Expression::Identifier(name) => self
                    .state
                    .speculation
                    .static_semantics
                    .local_value_binding(name)
                    .cloned()
                    .or_else(|| self.global_value_binding(name).cloned()),
                _ => None,
            })
            .unwrap_or_else(|| substituted_callee.clone());
        if trace {
            eprintln!(
                "constructed_function_ctor:derived_candidate function={function_name} super={super_callee:?} substituted={substituted_callee:?} resolved={resolved_callee:?}"
            );
        }
        if let Some(constructor_name) =
            self.expression_resolves_to_function_constructor_builtin(&resolved_callee)
        {
            return Some(constructor_name);
        }
        let substituted_arguments = super_arguments
            .iter()
            .map(|argument| match argument {
                CallArgument::Expression(expression) => CallArgument::Expression(
                    self.substitute_constructor_call_frame_bindings_with_rest(
                        expression,
                        user_function,
                        arguments,
                        &this_binding,
                        &arguments_binding,
                    ),
                ),
                CallArgument::Spread(expression) => {
                    CallArgument::Spread(self.substitute_constructor_call_frame_bindings_with_rest(
                        expression,
                        user_function,
                        arguments,
                        &this_binding,
                        &arguments_binding,
                    ))
                }
            })
            .collect::<Vec<_>>();
        let LocalFunctionBinding::User(super_function_name) =
            self.resolve_function_binding_from_expression(&resolved_callee)?
        else {
            return None;
        };
        self.resolve_derived_constructed_function_constructor_name(
            &super_function_name,
            &substituted_arguments,
            depth + 1,
        )
    }

    fn resolve_static_date_argument_number(
        &self,
        arguments: &[CallArgument],
        index: usize,
    ) -> Option<f64> {
        let argument = match arguments.get(index)? {
            CallArgument::Expression(expression) | CallArgument::Spread(expression) => expression,
        };
        self.resolve_static_number_value(argument).or_else(|| {
            let text =
                self.resolve_static_string_concat_value(argument, self.current_function_name())?;
            let trimmed = text.trim();
            if trimmed.is_empty() {
                Some(0.0)
            } else {
                trimmed.parse::<f64>().ok()
            }
        })
    }

    fn resolve_static_date_constructor_timestamp(
        &self,
        callee: &Expression,
        arguments: &[CallArgument],
    ) -> Option<f64> {
        let Expression::Identifier(name) = callee else {
            return None;
        };
        let constructs_date = if self.is_unshadowed_builtin_identifier(name) {
            name == "Date"
        } else {
            self.constructor_callee_inherits_from_builtin_prototype(callee, arguments, "Date")
        };
        if !constructs_date {
            return None;
        }

        if arguments.is_empty() {
            return Some(0.0);
        }
        if arguments.len() == 1 {
            return self.resolve_static_date_argument_number(arguments, 0);
        }

        let mut year = self.resolve_static_date_argument_number(arguments, 0)? as i32;
        if (0..=99).contains(&year) {
            year += 1900;
        }
        let month_index = self.resolve_static_date_argument_number(arguments, 1)? as i32;
        let date = self
            .resolve_static_date_argument_number(arguments, 2)
            .unwrap_or(1.0) as i32;
        let hours = self
            .resolve_static_date_argument_number(arguments, 3)
            .unwrap_or(0.0) as i64;
        let minutes = self
            .resolve_static_date_argument_number(arguments, 4)
            .unwrap_or(0.0) as i64;
        let seconds = self
            .resolve_static_date_argument_number(arguments, 5)
            .unwrap_or(0.0) as i64;
        let milliseconds = self
            .resolve_static_date_argument_number(arguments, 6)
            .unwrap_or(0.0) as i64;

        year += month_index.div_euclid(12);
        let month = month_index.rem_euclid(12) + 1;
        let day = days_from_civil(year, month as u32, 1) + i64::from(date - 1);
        Some(
            day as f64 * MS_PER_DAY
                + hours as f64 * MS_PER_HOUR
                + minutes as f64 * MS_PER_MINUTE
                + seconds as f64 * MS_PER_SECOND
                + milliseconds as f64,
        )
    }

    pub(in crate::backend::direct_wasm) fn resolve_static_date_timestamp(
        &self,
        expression: &Expression,
    ) -> Option<f64> {
        if let Expression::Identifier(name) = expression {
            if let Some(object_binding) = self
                .state
                .speculation
                .static_semantics
                .local_object_binding(name)
                .or_else(|| self.backend.global_object_binding(name))
                && let Some(Expression::Number(timestamp)) =
                    object_binding_lookup_value(object_binding, &date_value_property_expression())
            {
                return Some(*timestamp);
            }
            if let Some(resolved) = self
                .resolve_bound_alias_expression(expression)
                .filter(|resolved| !static_expression_matches(resolved, expression))
            {
                return self.resolve_static_date_timestamp(&resolved);
            }
        }
        let Expression::New { callee, arguments } = expression else {
            return None;
        };
        self.resolve_static_date_constructor_timestamp(callee.as_ref(), &arguments)
    }

    pub(in crate::backend::direct_wasm) fn synthesize_static_date_string(
        &self,
        timestamp: f64,
    ) -> String {
        if timestamp.fract() == 0.0 {
            format!("Date({})", timestamp as i64)
        } else {
            format!("Date({timestamp})")
        }
    }

    pub(in crate::backend::direct_wasm) fn resolve_static_date_component(
        &self,
        timestamp: f64,
        property_name: &str,
    ) -> Option<f64> {
        let day = (timestamp / MS_PER_DAY).floor() as i64;
        let (year, month, date) = civil_from_days(day);
        match property_name {
            "getFullYear" | "getUTCFullYear" => Some(year as f64),
            "getMonth" | "getUTCMonth" => Some(month as f64 - 1.0),
            "getDate" | "getUTCDate" => Some(date as f64),
            _ => None,
        }
    }

    pub(in crate::backend::direct_wasm) fn seed_date_value_property(
        &self,
        source_expression: &Expression,
        object_binding: &mut ObjectValueBinding,
    ) {
        let Some(timestamp) = self.resolve_static_date_timestamp(source_expression) else {
            return;
        };
        object_binding_define_property(
            object_binding,
            date_value_property_expression(),
            Expression::Number(timestamp),
            false,
        );
    }

    pub(in crate::backend::direct_wasm) fn seed_local_date_object_binding(
        &mut self,
        name: &str,
        source_expression: &Expression,
    ) {
        let Some(timestamp) = self.resolve_static_date_timestamp(source_expression) else {
            return;
        };
        let mut object_binding = self
            .state
            .speculation
            .static_semantics
            .local_object_binding(name)
            .cloned()
            .or_else(|| self.backend.global_object_binding(name).cloned())
            .unwrap_or_else(empty_object_value_binding);
        object_binding_define_property(
            &mut object_binding,
            date_value_property_expression(),
            Expression::Number(timestamp),
            false,
        );
        self.state
            .speculation
            .static_semantics
            .set_local_object_binding(name, object_binding.clone());
        if self.binding_name_is_global(name) {
            self.backend
                .sync_global_object_binding(name, Some(object_binding));
        }
        self.state
            .speculation
            .static_semantics
            .set_local_kind(name, StaticValueKind::Object);
    }

    pub(in crate::backend::direct_wasm) fn seed_global_date_object_binding(
        &mut self,
        name: &str,
        source_expression: &Expression,
    ) {
        let Some(timestamp) = self.resolve_static_date_timestamp(source_expression) else {
            return;
        };
        let mut object_binding = self
            .backend
            .global_object_binding(name)
            .cloned()
            .unwrap_or_else(empty_object_value_binding);
        object_binding_define_property(
            &mut object_binding,
            date_value_property_expression(),
            Expression::Number(timestamp),
            false,
        );
        self.backend
            .sync_global_object_binding(name, Some(object_binding));
    }

    fn expression_resolves_to_native_error_builtin(
        &self,
        expression: &Expression,
    ) -> Option<&'static str> {
        let native_error_builtin_name = |name: &str| {
            NATIVE_ERROR_NAMES
                .iter()
                .copied()
                .find(|candidate| candidate == &name)
        };
        if let Expression::Identifier(name) = expression
            && let Some(error_name) = native_error_builtin_name(name)
            && self.native_error_identifier_refers_to_builtin(name)
        {
            return Some(error_name);
        }
        let resolved = self
            .resolve_bound_alias_expression(expression)
            .filter(|resolved| !static_expression_matches(resolved, expression))
            .or_else(|| {
                let materialized = self.materialize_static_expression(expression);
                (!static_expression_matches(&materialized, expression)).then_some(materialized)
            })?;
        if let Expression::Identifier(name) = resolved {
            return native_error_builtin_name(&name)
                .filter(|_| self.native_error_identifier_refers_to_builtin(&name));
        }
        None
    }

    fn native_error_identifier_refers_to_builtin(&self, name: &str) -> bool {
        if self.resolve_current_local_binding(name).is_some()
            || self
                .state
                .speculation
                .static_semantics
                .local_value_binding(name)
                .is_some()
            || self.backend.lexical_global_binding(name).is_some()
        {
            return false;
        }

        if let Some(binding) = self
            .state
            .speculation
            .static_semantics
            .local_function_binding(name)
        {
            return matches!(binding, LocalFunctionBinding::Builtin(builtin) if builtin == name);
        }

        if let Some(binding) = self.backend.global_function_binding(name) {
            return matches!(binding, LocalFunctionBinding::Builtin(builtin) if builtin == name);
        }

        if let Some(value) = self.global_value_binding(name)
            && !static_expression_matches(value, &Expression::Identifier(name.to_string()))
        {
            return false;
        }

        self.global_object_binding(name).is_none()
            && self.global_array_binding(name).is_none()
            && self.backend.global_arguments_binding(name).is_none()
    }

    fn resolve_derived_constructed_native_error_details(
        &self,
        function_name: &str,
        arguments: &[CallArgument],
        depth: usize,
    ) -> Option<(&'static str, Vec<CallArgument>)> {
        if depth > 16 {
            return None;
        }
        let user_function = self.user_function(function_name).or_else(|| {
            self.backend
                .function_registry
                .catalog
                .user_function(function_name)
        })?;
        if !self.user_function_is_derived_constructor(user_function) {
            return None;
        }
        let (super_callee, super_arguments) =
            self.resolve_derived_constructor_super_call(user_function)?;
        let expanded_arguments = self.expand_call_arguments(arguments);
        let this_binding = Expression::Identifier(Self::STATIC_NEW_THIS_BINDING.to_string());
        let arguments_binding = Expression::Array(
            expanded_arguments
                .iter()
                .cloned()
                .map(crate::ir::hir::ArrayElement::Expression)
                .collect(),
        );
        let substituted_callee = self.substitute_constructor_call_frame_bindings_with_rest(
            super_callee,
            user_function,
            arguments,
            &this_binding,
            &arguments_binding,
        );
        let resolved_callee = self
            .resolve_bound_alias_expression(&substituted_callee)
            .or_else(|| match &substituted_callee {
                Expression::Identifier(name) => self
                    .state
                    .speculation
                    .static_semantics
                    .local_value_binding(name)
                    .cloned()
                    .or_else(|| self.global_value_binding(name).cloned()),
                _ => None,
            })
            .unwrap_or_else(|| substituted_callee.clone());
        let substituted_arguments = super_arguments
            .iter()
            .map(|argument| match argument {
                CallArgument::Expression(expression) => CallArgument::Expression(
                    self.substitute_constructor_call_frame_bindings_with_rest(
                        expression,
                        user_function,
                        arguments,
                        &this_binding,
                        &arguments_binding,
                    ),
                ),
                CallArgument::Spread(expression) => {
                    CallArgument::Spread(self.substitute_constructor_call_frame_bindings_with_rest(
                        expression,
                        user_function,
                        arguments,
                        &this_binding,
                        &arguments_binding,
                    ))
                }
            })
            .collect::<Vec<_>>();
        if let Some(error_name) = self.expression_resolves_to_native_error_builtin(&resolved_callee)
        {
            return Some((error_name, substituted_arguments));
        }
        let LocalFunctionBinding::User(super_function_name) =
            self.resolve_function_binding_from_expression(&resolved_callee)?
        else {
            return None;
        };
        self.resolve_derived_constructed_native_error_details(
            &super_function_name,
            &substituted_arguments,
            depth + 1,
        )
    }

    fn resolve_static_constructed_native_error_details(
        &self,
        expression: &Expression,
    ) -> Option<(&'static str, Vec<CallArgument>)> {
        let Expression::New { callee, arguments } = expression else {
            return None;
        };
        if let Some(error_name) = self.expression_resolves_to_native_error_builtin(callee) {
            return Some((error_name, arguments.clone()));
        }
        if let Some(LocalFunctionBinding::User(function_name)) =
            self.resolve_function_binding_from_expression(callee)
            && let Some(details) =
                self.resolve_derived_constructed_native_error_details(&function_name, arguments, 0)
        {
            return Some(details);
        }
        NATIVE_ERROR_NAMES.iter().copied().find_map(|candidate| {
            self.constructor_callee_inherits_from_builtin_prototype(callee, arguments, candidate)
                .then(|| (candidate, arguments.clone()))
        })
    }

    fn resolve_static_constructed_native_error_message_from_arguments(
        &self,
        arguments: &[CallArgument],
    ) -> Option<String> {
        let expanded_arguments = self.expand_call_arguments(arguments);
        let argument = expanded_arguments.first()?;
        if matches!(
            self.resolve_static_primitive_expression_with_context(
                argument,
                self.current_function_name(),
            ),
            Some(Expression::Undefined)
        ) || matches!(
            argument,
            Expression::Identifier(name)
                if name == "undefined" && self.is_unshadowed_builtin_identifier(name)
        ) {
            return None;
        }
        self.resolve_static_string_concat_value(argument, self.current_function_name())
    }

    pub(in crate::backend::direct_wasm) fn resolve_static_constructed_native_error_object_binding(
        &self,
        source_expression: &Expression,
    ) -> Option<ObjectValueBinding> {
        let (error_name, error_arguments) =
            self.resolve_static_constructed_native_error_details(source_expression)?;
        let mut object_binding = empty_object_value_binding();
        object_binding_define_property(
            &mut object_binding,
            Expression::String("name".to_string()),
            Expression::String(error_name.to_string()),
            false,
        );
        object_binding_define_property(
            &mut object_binding,
            Expression::String("constructor".to_string()),
            Expression::Identifier(error_name.to_string()),
            false,
        );
        if let Some(message) =
            self.resolve_static_constructed_native_error_message_from_arguments(&error_arguments)
        {
            object_binding_define_property(
                &mut object_binding,
                Expression::String("message".to_string()),
                Expression::String(message),
                false,
            );
        }
        Some(object_binding)
    }

    pub(in crate::backend::direct_wasm) fn seed_native_error_object_binding(
        &self,
        source_expression: &Expression,
        object_binding: &mut ObjectValueBinding,
    ) {
        let Some(native_error_binding) =
            self.resolve_static_constructed_native_error_object_binding(source_expression)
        else {
            return;
        };
        for (property, value) in native_error_binding.string_properties {
            let enumerable = !native_error_binding
                .non_enumerable_string_properties
                .iter()
                .any(|hidden_name| hidden_name == &property);
            object_binding_define_property(
                object_binding,
                Expression::String(property),
                value,
                enumerable,
            );
        }
    }

    pub(in crate::backend::direct_wasm) fn seed_local_native_error_object_binding(
        &mut self,
        name: &str,
        source_expression: &Expression,
    ) {
        let Some(native_error_binding) =
            self.resolve_static_constructed_native_error_object_binding(source_expression)
        else {
            return;
        };
        let mut object_binding = self
            .state
            .speculation
            .static_semantics
            .local_object_binding(name)
            .cloned()
            .or_else(|| self.backend.global_object_binding(name).cloned())
            .unwrap_or_else(empty_object_value_binding);
        for (property, value) in native_error_binding.string_properties {
            let enumerable = !native_error_binding
                .non_enumerable_string_properties
                .iter()
                .any(|hidden_name| hidden_name == &property);
            object_binding_define_property(
                &mut object_binding,
                Expression::String(property),
                value,
                enumerable,
            );
        }
        self.state
            .speculation
            .static_semantics
            .set_local_object_binding(name, object_binding.clone());
        if self.binding_name_is_global(name) {
            self.backend
                .sync_global_object_binding(name, Some(object_binding));
        }
        self.state
            .speculation
            .static_semantics
            .set_local_kind(name, StaticValueKind::Object);
    }

    pub(in crate::backend::direct_wasm) fn seed_global_native_error_object_binding(
        &mut self,
        name: &str,
        source_expression: &Expression,
    ) {
        let Some(native_error_binding) =
            self.resolve_static_constructed_native_error_object_binding(source_expression)
        else {
            return;
        };
        let mut object_binding = self
            .backend
            .global_object_binding(name)
            .cloned()
            .unwrap_or_else(empty_object_value_binding);
        for (property, value) in native_error_binding.string_properties {
            let enumerable = !native_error_binding
                .non_enumerable_string_properties
                .iter()
                .any(|hidden_name| hidden_name == &property);
            object_binding_define_property(
                &mut object_binding,
                Expression::String(property),
                value,
                enumerable,
            );
        }
        self.backend
            .sync_global_object_binding(name, Some(object_binding));
    }

    pub(in crate::backend::direct_wasm) fn resolve_static_constructed_function_constructor_name(
        &self,
        callee: &Expression,
        arguments: &[CallArgument],
    ) -> Option<&'static str> {
        let trace = std::env::var_os("AYY_TRACE_CONSTRUCTED_FUNCTIONS").is_some();
        let Expression::Identifier(_name) = callee else {
            if trace {
                eprintln!("constructed_function_ctor:non_identifier callee={callee:?}");
            }
            return None;
        };
        if let Some(constructor_name) =
            self.expression_resolves_to_function_constructor_builtin(callee)
        {
            if trace {
                eprintln!(
                    "constructed_function_ctor:builtin_alias callee={callee:?} constructor={constructor_name}"
                );
            }
            return Some(constructor_name);
        }
        let mut callee_candidates = vec![callee.clone()];
        if let Some(resolved) = self
            .resolve_bound_alias_expression(callee)
            .filter(|resolved| !static_expression_matches(resolved, callee))
        {
            callee_candidates.push(resolved);
        }
        let materialized_callee = self.materialize_static_expression(callee);
        if !static_expression_matches(&materialized_callee, callee) {
            callee_candidates.push(materialized_callee);
        }
        if let Expression::Identifier(name) = callee
            && let Some(value) = self
                .state
                .speculation
                .static_semantics
                .local_value_binding(name)
                .or_else(|| self.global_value_binding(name))
            && !static_expression_matches(value, callee)
        {
            callee_candidates.push(value.clone());
        }
        if let Expression::Identifier(name) = callee {
            if let Some(alias) = self.resolve_static_class_init_constructor_alias(name) {
                callee_candidates.push(Expression::Identifier(alias));
            }
            if let Some(alias) = self.resolve_static_class_init_local_alias_expression(name) {
                callee_candidates.push(alias);
            }
        }
        if trace {
            eprintln!(
                "constructed_function_ctor:candidates callee={callee:?} candidates={callee_candidates:?}"
            );
        }
        for candidate in &callee_candidates {
            let candidate_function_name = self
                .resolve_function_binding_from_expression(candidate)
                .and_then(|binding| match binding {
                    LocalFunctionBinding::User(function_name) => Some(function_name),
                    LocalFunctionBinding::Builtin(_) => None,
                })
                .or_else(|| match candidate {
                    Expression::Identifier(name)
                        if self.user_function(name).is_some()
                            || self
                                .backend
                                .function_registry
                                .catalog
                                .user_function(name)
                                .is_some() =>
                    {
                        Some(name.clone())
                    }
                    Expression::Identifier(name) => self
                        .resolve_user_function_by_binding_name(name)
                        .map(|function| function.name.clone()),
                    _ => None,
                });
            if let Some(function_name) = candidate_function_name
                && let Some(constructor_name) = self
                    .resolve_derived_constructed_function_constructor_name(
                        &function_name,
                        arguments,
                        0,
                    )
            {
                if trace {
                    eprintln!(
                        "constructed_function_ctor:derived callee={callee:?} candidate={candidate:?} function={function_name} constructor={constructor_name}"
                    );
                }
                return Some(constructor_name);
            }
        }
        let callee_is_ordinary_user_constructor = callee_candidates.iter().any(|candidate| {
            self.resolve_function_binding_from_expression(candidate)
                .is_some_and(|binding| matches!(binding, LocalFunctionBinding::User(_)))
                || matches!(
                    candidate,
                    Expression::Identifier(name)
                        if self.user_function(name).is_some()
                            || self
                                .backend
                                .function_registry
                                .catalog
                                .user_function(name)
                                .is_some()
                            || self.resolve_user_function_by_binding_name(name).is_some()
                )
        });
        let inherited = (!callee_is_ordinary_user_constructor)
            .then(|| {
                [
                    "GeneratorFunction",
                    "AsyncGeneratorFunction",
                    "AsyncFunction",
                    "Function",
                ]
                .into_iter()
                .find(|constructor_name| {
                    self.constructor_callee_inherits_from_builtin_prototype(
                        callee,
                        arguments,
                        constructor_name,
                    )
                })
            })
            .flatten();
        if trace {
            eprintln!(
                "constructed_function_ctor:inheritance callee={callee:?} constructor={inherited:?}"
            );
        }
        inherited
    }

    pub(in crate::backend::direct_wasm) fn resolve_static_constructed_function_source_expression(
        &self,
        expression: &Expression,
    ) -> Option<Expression> {
        let resolved = self
            .resolve_bound_alias_expression(expression)
            .filter(|resolved| !static_expression_matches(resolved, expression))
            .unwrap_or_else(|| expression.clone());
        let Expression::New { callee, arguments } = &resolved else {
            return None;
        };
        self.resolve_static_constructed_function_constructor_name(callee, arguments)?;
        Some(resolved)
    }

    pub(in crate::backend::direct_wasm) fn resolve_static_constructed_function_parts(
        &self,
        source_expression: &Expression,
    ) -> Option<(Vec<String>, String)> {
        let Expression::New { arguments, .. } = source_expression else {
            return None;
        };
        if arguments.is_empty() {
            return Some((Vec::new(), String::new()));
        }

        let mut source_parts = Vec::new();
        for argument in arguments {
            source_parts.push(self.resolve_static_string_concat_value(
                argument.expression(),
                self.current_function_name(),
            )?);
        }

        let body = source_parts.pop().unwrap_or_default();
        let parameters = source_parts
            .into_iter()
            .flat_map(|part| {
                part.split(',')
                    .map(str::trim)
                    .filter(|parameter| !parameter.is_empty())
                    .map(str::to_string)
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        Some((parameters, body))
    }

    pub(in crate::backend::direct_wasm) fn resolve_static_constructed_function_metadata_object_binding(
        &self,
        source_expression: &Expression,
    ) -> Option<ObjectValueBinding> {
        let source_expression =
            self.resolve_static_constructed_function_source_expression(source_expression)?;
        let Expression::New { callee, arguments } = &source_expression else {
            return None;
        };
        let constructor_name =
            self.resolve_static_constructed_function_constructor_name(callee, arguments)?;
        let (parameters, _) = self.resolve_static_constructed_function_parts(&source_expression)?;
        let mut object_binding = empty_object_value_binding();
        object_binding_define_property(
            &mut object_binding,
            function_constructor_source_property_expression(),
            source_expression,
            false,
        );
        object_binding_define_property(
            &mut object_binding,
            Expression::String("length".to_string()),
            Expression::Number(parameters.len() as f64),
            false,
        );
        object_binding_define_property(
            &mut object_binding,
            Expression::String("name".to_string()),
            Expression::String("anonymous".to_string()),
            false,
        );
        if constructor_name == "GeneratorFunction" {
            object_binding_define_property(
                &mut object_binding,
                Expression::String("prototype".to_string()),
                Expression::Object(Vec::new()),
                false,
            );
        }
        Some(object_binding)
    }

    pub(in crate::backend::direct_wasm) fn seed_constructed_function_object_binding(
        &self,
        source_expression: &Expression,
        object_binding: &mut ObjectValueBinding,
    ) {
        let Some(function_binding) =
            self.resolve_static_constructed_function_metadata_object_binding(source_expression)
        else {
            return;
        };
        for (property, value) in function_binding.string_properties {
            let enumerable = !function_binding
                .non_enumerable_string_properties
                .iter()
                .any(|hidden_name| hidden_name == &property);
            object_binding_define_property(
                object_binding,
                Expression::String(property),
                value,
                enumerable,
            );
        }
    }

    pub(in crate::backend::direct_wasm) fn seed_local_constructed_function_object_binding(
        &mut self,
        name: &str,
        source_expression: &Expression,
    ) {
        let Some(function_binding) =
            self.resolve_static_constructed_function_metadata_object_binding(source_expression)
        else {
            return;
        };
        let mut object_binding = self
            .state
            .speculation
            .static_semantics
            .local_object_binding(name)
            .cloned()
            .or_else(|| self.backend.global_object_binding(name).cloned())
            .unwrap_or_else(empty_object_value_binding);
        for (property, value) in function_binding.string_properties {
            let enumerable = !function_binding
                .non_enumerable_string_properties
                .iter()
                .any(|hidden_name| hidden_name == &property);
            object_binding_define_property(
                &mut object_binding,
                Expression::String(property),
                value,
                enumerable,
            );
        }
        self.state
            .speculation
            .static_semantics
            .set_local_object_binding(name, object_binding.clone());
        if self.binding_name_is_global(name) {
            self.backend
                .sync_global_object_binding(name, Some(object_binding));
        }
        self.state
            .speculation
            .static_semantics
            .set_local_kind(name, StaticValueKind::Object);
    }

    pub(in crate::backend::direct_wasm) fn seed_global_constructed_function_object_binding(
        &mut self,
        name: &str,
        source_expression: &Expression,
    ) {
        let Some(function_binding) =
            self.resolve_static_constructed_function_metadata_object_binding(source_expression)
        else {
            return;
        };
        let mut object_binding = self
            .backend
            .global_object_binding(name)
            .cloned()
            .unwrap_or_else(empty_object_value_binding);
        for (property, value) in function_binding.string_properties {
            let enumerable = !function_binding
                .non_enumerable_string_properties
                .iter()
                .any(|hidden_name| hidden_name == &property);
            object_binding_define_property(
                &mut object_binding,
                Expression::String(property),
                value,
                enumerable,
            );
        }
        self.backend
            .sync_global_object_binding(name, Some(object_binding));
    }

    pub(in crate::backend::direct_wasm) fn resolve_static_constructed_function_source_from_expression(
        &self,
        expression: &Expression,
    ) -> Option<Expression> {
        if let Some(object_binding) = self.resolve_object_binding_from_expression(expression)
            && let Some(source_expression) = object_binding_lookup_value(
                &object_binding,
                &function_constructor_source_property_expression(),
            )
        {
            return Some(source_expression.clone());
        }
        if let Expression::Identifier(name) = expression
            && let Some(object_binding) = self.backend.global_object_binding(name)
            && let Some(source_expression) = object_binding_lookup_value(
                object_binding,
                &function_constructor_source_property_expression(),
            )
        {
            return Some(source_expression.clone());
        }
        self.resolve_static_constructed_function_source_expression(expression)
    }

    pub(in crate::backend::direct_wasm) fn resolve_static_constructed_function_source_constructor_name(
        &self,
        source_expression: &Expression,
    ) -> Option<&'static str> {
        let Expression::New { callee, arguments } = source_expression else {
            return None;
        };
        self.resolve_static_constructed_function_constructor_name(callee, arguments)
    }

    pub(in crate::backend::direct_wasm) fn constructed_function_call_creates_generator_iterator(
        &self,
        callee: &Expression,
    ) -> bool {
        self.resolve_static_constructed_function_source_from_expression(callee)
            .as_ref()
            .and_then(|source_expression| {
                self.resolve_static_constructed_function_source_constructor_name(source_expression)
            })
            == Some("GeneratorFunction")
    }

    fn constructed_function_call_argument_expression(
        arguments: &[CallArgument],
        index: usize,
    ) -> Expression {
        arguments
            .get(index)
            .map(|argument| argument.expression().clone())
            .unwrap_or(Expression::Undefined)
    }

    fn constructed_function_binary_operator(symbol: char) -> Option<BinaryOp> {
        match symbol {
            '+' => Some(BinaryOp::Add),
            '-' => Some(BinaryOp::Subtract),
            '*' => Some(BinaryOp::Multiply),
            '/' => Some(BinaryOp::Divide),
            _ => None,
        }
    }

    fn find_constructed_function_binary_operator(
        expression_text: &str,
        operators: &[char],
    ) -> Option<(usize, char)> {
        expression_text
            .char_indices()
            .rev()
            .find_map(|(index, symbol)| {
                if !operators.contains(&symbol) {
                    return None;
                }
                let left = expression_text[..index].trim();
                let right = expression_text[index + symbol.len_utf8()..].trim();
                (!left.is_empty() && !right.is_empty()).then_some((index, symbol))
            })
    }

    fn parse_constructed_function_operand(
        token: &str,
        parameters: &[String],
        arguments: &[CallArgument],
    ) -> Option<Expression> {
        let token = token.trim();
        if let Some(index) = parameters.iter().position(|parameter| parameter == token) {
            return Some(Self::constructed_function_call_argument_expression(
                arguments, index,
            ));
        }
        if token == "undefined" {
            return Some(Expression::Undefined);
        }
        if let Ok(value) = token.parse::<f64>() {
            return Some(Expression::Number(value));
        }
        if let Some(text) = token
            .strip_prefix('\'')
            .and_then(|text| text.strip_suffix('\''))
            .or_else(|| {
                token
                    .strip_prefix('"')
                    .and_then(|text| text.strip_suffix('"'))
            })
        {
            return Some(Expression::String(text.to_string()));
        }
        None
    }

    pub(in crate::backend::direct_wasm) fn parse_constructed_function_expression_text(
        expression_text: &str,
        parameters: &[String],
        arguments: &[CallArgument],
    ) -> Option<Expression> {
        let expression_text = expression_text.trim();
        if expression_text.starts_with('(') && expression_text.ends_with(')') {
            let inner = &expression_text[1..expression_text.len() - 1];
            if let Some(parsed) =
                Self::parse_constructed_function_expression_text(inner, parameters, arguments)
            {
                return Some(parsed);
            }
        }
        for operators in [['+', '-'].as_slice(), ['*', '/'].as_slice()] {
            if let Some((index, symbol)) =
                Self::find_constructed_function_binary_operator(expression_text, operators)
            {
                let left = Self::parse_constructed_function_expression_text(
                    &expression_text[..index],
                    parameters,
                    arguments,
                )?;
                let right = Self::parse_constructed_function_expression_text(
                    &expression_text[index + symbol.len_utf8()..],
                    parameters,
                    arguments,
                )?;
                return Some(Expression::Binary {
                    op: Self::constructed_function_binary_operator(symbol)?,
                    left: Box::new(left),
                    right: Box::new(right),
                });
            }
        }
        Self::parse_constructed_function_operand(expression_text, parameters, arguments)
    }

    fn parse_constructed_function_body_result(
        body: &str,
        parameters: &[String],
        arguments: &[CallArgument],
    ) -> Option<Expression> {
        let body = body.trim();
        if body.is_empty() {
            return Some(Expression::Undefined);
        }
        let return_expression = body.strip_prefix("return")?.trim();
        let return_expression = return_expression
            .strip_suffix(';')
            .unwrap_or(return_expression)
            .trim();
        Self::parse_constructed_function_expression_text(return_expression, parameters, arguments)
    }

    pub(in crate::backend::direct_wasm) fn resolve_static_constructed_function_call_result(
        &self,
        callee: &Expression,
        arguments: &[CallArgument],
    ) -> Option<Expression> {
        let source_expression =
            self.resolve_static_constructed_function_source_from_expression(callee)?;
        if self.resolve_static_constructed_function_source_constructor_name(&source_expression)
            == Some("GeneratorFunction")
        {
            return None;
        }
        let (parameters, body) =
            self.resolve_static_constructed_function_parts(&source_expression)?;
        Self::parse_constructed_function_body_result(&body, &parameters, arguments)
    }

    pub(in crate::backend::direct_wasm) fn synthesize_static_function_to_string(
        &self,
        function_name: &str,
    ) -> String {
        let Some(function) = self.resolve_registered_function_declaration(function_name) else {
            return format!("function {function_name}() {{}}");
        };
        let params = function
            .params
            .iter()
            .map(|param| param.name.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        let prefix = match function.kind {
            FunctionKind::Ordinary => "function",
            FunctionKind::Generator => "function*",
            FunctionKind::Async => "async function",
            FunctionKind::AsyncGenerator => "async function*",
        };
        let display_name = self.resolve_user_function_display_name(function_name);
        match display_name {
            Some(name) if !name.is_empty() => format!("{prefix} {name}({params}) {{}}"),
            _ => format!("{prefix}({params}) {{}}"),
        }
    }

    pub(in crate::backend::direct_wasm) fn synthesize_static_function_binding_to_string(
        &self,
        binding: &LocalFunctionBinding,
    ) -> String {
        match binding {
            LocalFunctionBinding::User(function_name) => {
                self.synthesize_static_function_to_string(function_name)
            }
            LocalFunctionBinding::Builtin(function_name) => format!(
                "function {}() {{}}",
                builtin_function_display_name(function_name)
            ),
        }
    }

    fn resolve_static_boxed_primitive_argument_value(
        &self,
        boxed_constructor_name: &str,
        arguments: &[CallArgument],
    ) -> Option<Expression> {
        match boxed_constructor_name {
            "Boolean" => {
                let value = match arguments.first() {
                    Some(CallArgument::Expression(argument))
                    | Some(CallArgument::Spread(argument)) => {
                        self.resolve_static_boolean_expression(argument)?
                    }
                    None => false,
                };
                Some(Expression::Bool(value))
            }
            "Number" => {
                let value = match arguments.first() {
                    Some(CallArgument::Expression(argument))
                    | Some(CallArgument::Spread(argument)) => {
                        self.resolve_static_number_value(argument)?
                    }
                    None => 0.0,
                };
                Some(Expression::Number(value))
            }
            "String" => {
                let value = match arguments.first() {
                    Some(CallArgument::Expression(argument))
                    | Some(CallArgument::Spread(argument)) => self
                        .resolve_static_symbol_to_string_value_with_context(
                            argument,
                            self.current_function_name(),
                        )
                        .or_else(|| {
                            self.resolve_static_string_concat_value(
                                argument,
                                self.current_function_name(),
                            )
                        })?,
                    None => String::new(),
                };
                Some(Expression::String(value))
            }
            "Object" => match arguments.first() {
                Some(CallArgument::Expression(argument)) | Some(CallArgument::Spread(argument)) => {
                    self.resolve_static_primitive_expression_with_context(
                        argument,
                        self.current_function_name(),
                    )
                    .filter(|value| {
                        matches!(
                            value,
                            Expression::Number(_)
                                | Expression::BigInt(_)
                                | Expression::String(_)
                                | Expression::Bool(_)
                        ) || self.infer_value_kind(value) == Some(StaticValueKind::Symbol)
                    })
                }
                None => None,
            },
            _ => None,
        }
    }

    fn resolve_static_constructed_boxed_primitive_constructor_name(
        &self,
        callee: &Expression,
        arguments: &[CallArgument],
    ) -> Option<&'static str> {
        if self.constructor_callee_inherits_from_boxed_prototype(callee, arguments, "Boolean") {
            return Some("Boolean");
        }
        if self.constructor_callee_inherits_from_boxed_prototype(callee, arguments, "Number") {
            return Some("Number");
        }
        if self.constructor_callee_inherits_from_boxed_prototype(callee, arguments, "String") {
            return Some("String");
        }
        None
    }

    fn constructor_callee_inherits_from_boxed_prototype(
        &self,
        callee: &Expression,
        arguments: &[CallArgument],
        boxed_constructor_name: &str,
    ) -> bool {
        let target_prototype = Self::prototype_member_expression(boxed_constructor_name);
        if self.expression_inherits_from_prototype_for_instanceof(
            &Expression::New {
                callee: Box::new(callee.clone()),
                arguments: arguments.to_vec(),
            },
            &target_prototype,
        ) {
            return true;
        }
        let Expression::Identifier(name) = callee else {
            return false;
        };
        self.expression_inherits_from_prototype_for_instanceof(
            &Self::prototype_member_expression(name),
            &target_prototype,
        )
    }

    pub(in crate::backend::direct_wasm) fn resolve_static_constructed_boxed_primitive_value(
        &self,
        expression: &Expression,
    ) -> Option<Expression> {
        let Expression::New { callee, arguments } = expression else {
            return None;
        };
        let Expression::Identifier(name) = callee.as_ref() else {
            return None;
        };
        let boxed_constructor_name = if self.is_unshadowed_builtin_identifier(name) {
            match name.as_str() {
                "Boolean" | "Number" | "String" | "Object" => name.as_str(),
                _ => return None,
            }
        } else {
            self.resolve_static_constructed_boxed_primitive_constructor_name(callee, arguments)?
        };
        self.resolve_static_boxed_primitive_argument_value(boxed_constructor_name, arguments)
    }

    pub(in crate::backend::direct_wasm) fn seed_boxed_primitive_value_property(
        &self,
        source_expression: &Expression,
        object_binding: &mut ObjectValueBinding,
    ) {
        let Some(value) = self.resolve_static_constructed_boxed_primitive_value(source_expression)
        else {
            if std::env::var_os("AYY_TRACE_BOXED_PRIMITIVES").is_some() {
                eprintln!("boxed_primitive:seed_local none source={source_expression:?}");
            }
            return;
        };
        if std::env::var_os("AYY_TRACE_BOXED_PRIMITIVES").is_some() {
            eprintln!("boxed_primitive:seed_local source={source_expression:?} value={value:?}");
        }
        object_binding_define_property(
            object_binding,
            boxed_primitive_value_property_expression(),
            value,
            false,
        );
    }

    pub(in crate::backend::direct_wasm) fn seed_global_boxed_primitive_object_binding(
        &mut self,
        name: &str,
        source_expression: &Expression,
    ) {
        let Some(value) = self.resolve_static_constructed_boxed_primitive_value(source_expression)
        else {
            if std::env::var_os("AYY_TRACE_BOXED_PRIMITIVES").is_some() {
                eprintln!(
                    "boxed_primitive:seed_global none name={name} source={source_expression:?}"
                );
            }
            return;
        };
        if std::env::var_os("AYY_TRACE_BOXED_PRIMITIVES").is_some() {
            eprintln!(
                "boxed_primitive:seed_global name={name} source={source_expression:?} value={value:?}"
            );
        }
        let mut object_binding = self
            .backend
            .global_object_binding(name)
            .cloned()
            .unwrap_or_else(empty_object_value_binding);
        object_binding_define_property(
            &mut object_binding,
            boxed_primitive_value_property_expression(),
            value,
            false,
        );
        self.backend
            .sync_global_object_binding(name, Some(object_binding));
    }

    pub(in crate::backend::direct_wasm) fn resolve_static_symbol_to_string_value_with_context(
        &self,
        expression: &Expression,
        current_function_name: Option<&str>,
    ) -> Option<String> {
        if let Expression::Member { object, property } = expression {
            let materialized_property = self
                .resolve_property_key_expression(property)
                .unwrap_or_else(|| self.materialize_static_expression(property));
            let object_binding =
                self.resolve_object_binding_from_expression(object)
                    .or_else(|| {
                        let materialized_object = self.materialize_static_expression(object);
                        (!static_expression_matches(&materialized_object, object))
                            .then(|| {
                                self.resolve_object_binding_from_expression(&materialized_object)
                            })
                            .flatten()
                    });
            if let Some(value) = object_binding
                .as_ref()
                .and_then(|binding| object_binding_lookup_value(binding, &materialized_property))
                && !static_expression_matches(value, expression)
            {
                return self.resolve_static_symbol_to_string_value_with_context(
                    value,
                    current_function_name,
                );
            }
        }

        if let Some(resolved) = self.resolve_bound_alias_expression(expression)
            && !static_expression_matches(&resolved, expression)
        {
            return self.resolve_static_symbol_to_string_value_with_context(
                &resolved,
                current_function_name,
            );
        }

        let materialized = self.materialize_static_expression(expression);
        if !static_expression_matches(&materialized, expression) {
            return self.resolve_static_symbol_to_string_value_with_context(
                &materialized,
                current_function_name,
            );
        }

        if let Some(symbol_name) = self.well_known_symbol_name(expression) {
            return Some(format!("Symbol({symbol_name})"));
        }

        let Expression::Call { callee, arguments } = expression else {
            return None;
        };
        if !matches!(callee.as_ref(), Expression::Identifier(name) if name == "Symbol" && self.is_unshadowed_builtin_identifier(name))
        {
            return None;
        }

        let description = match arguments.first() {
            None => String::new(),
            Some(CallArgument::Expression(argument) | CallArgument::Spread(argument)) => {
                if matches!(
                    self.resolve_static_primitive_expression_with_context(
                        argument,
                        current_function_name,
                    ),
                    Some(Expression::Undefined)
                ) {
                    String::new()
                } else {
                    self.resolve_static_string_concat_value(argument, current_function_name)?
                }
            }
        };

        Some(format!("Symbol({description})"))
    }

    pub(in crate::backend::direct_wasm) fn resolve_static_boxed_primitive_value(
        &self,
        expression: &Expression,
    ) -> Option<Expression> {
        let trace_boxed = std::env::var_os("AYY_TRACE_BOXED_PRIMITIVES").is_some();
        if let Some(object_binding) = self.resolve_object_binding_from_expression(expression)
            && let Some(value) = object_binding_lookup_value(
                &object_binding,
                &boxed_primitive_value_property_expression(),
            )
        {
            if trace_boxed {
                eprintln!(
                    "boxed_primitive:object_binding expression={expression:?} value={value:?}"
                );
            }
            return Some(value.clone());
        }
        if let Expression::Identifier(name) = expression
            && let Some(object_binding) = self.backend.global_object_binding(name)
            && let Some(value) = object_binding_lookup_value(
                object_binding,
                &boxed_primitive_value_property_expression(),
            )
        {
            if trace_boxed {
                eprintln!(
                    "boxed_primitive:global_object_binding expression={expression:?} value={value:?}"
                );
            }
            return Some(value.clone());
        }
        let resolved = self
            .resolve_bound_alias_expression(expression)
            .unwrap_or_else(|| expression.clone());
        if trace_boxed {
            eprintln!("boxed_primitive:start expression={expression:?} resolved={resolved:?}");
        }
        let (callee, arguments, is_construct) = match resolved {
            Expression::New { callee, arguments } | Expression::Call { callee, arguments } => {
                let is_construct = matches!(expression, Expression::New { .. })
                    || matches!(
                        self.resolve_bound_alias_expression(expression),
                        Some(Expression::New { .. })
                    );
                (callee, arguments, is_construct)
            }
            _ => return None,
        };
        let Expression::Identifier(name) = callee.as_ref() else {
            if trace_boxed {
                eprintln!("boxed_primitive:none non_identifier_callee={callee:?}");
            }
            return None;
        };
        let inherits_boolean = is_construct
            && self.constructor_callee_inherits_from_boxed_prototype(
                callee.as_ref(),
                &arguments,
                "Boolean",
            );
        let inherits_number = is_construct
            && self.constructor_callee_inherits_from_boxed_prototype(
                callee.as_ref(),
                &arguments,
                "Number",
            );
        let inherits_string = is_construct
            && self.constructor_callee_inherits_from_boxed_prototype(
                callee.as_ref(),
                &arguments,
                "String",
            );
        if trace_boxed {
            eprintln!(
                "boxed_primitive:callee name={name} is_construct={is_construct} unshadowed={} inherits_boolean={inherits_boolean} inherits_number={inherits_number} inherits_string={inherits_string}",
                self.is_unshadowed_builtin_identifier(name)
            );
        }
        let boxed_constructor_name = if self.is_unshadowed_builtin_identifier(name) {
            name.as_str()
        } else if inherits_boolean {
            "Boolean"
        } else if inherits_number {
            "Number"
        } else if inherits_string {
            "String"
        } else {
            if trace_boxed {
                eprintln!("boxed_primitive:none no_boxed_constructor");
            }
            return None;
        };
        self.resolve_static_boxed_primitive_argument_value(boxed_constructor_name, &arguments)
    }

    pub(in crate::backend::direct_wasm) fn resolve_static_symbol_to_primitive_outcome_with_context(
        &self,
        expression: &Expression,
        current_function_name: Option<&str>,
    ) -> Option<StaticEvalOutcome> {
        let symbol_property = symbol_to_primitive_expression();
        let default_argument = [CallArgument::Expression(Expression::String(
            "default".to_string(),
        ))];
        let call_result = if let Some(getter_binding) =
            self.resolve_member_getter_binding(expression, &symbol_property)
        {
            match self.resolve_static_function_outcome_from_binding_with_context(
                &getter_binding,
                &[],
                current_function_name,
            )? {
                StaticEvalOutcome::Throw(throw_value) => {
                    return Some(StaticEvalOutcome::Throw(throw_value));
                }
                StaticEvalOutcome::Value(method_value) => {
                    if let Some(primitive) = self.resolve_static_primitive_expression_with_context(
                        &method_value,
                        current_function_name,
                    ) {
                        return match primitive {
                            Expression::Null | Expression::Undefined => None,
                            _ => Some(StaticEvalOutcome::Throw(StaticThrowValue::NamedError(
                                "TypeError",
                            ))),
                        };
                    }
                    let Some(binding) = self.resolve_function_binding_from_expression_with_context(
                        &method_value,
                        current_function_name,
                    ) else {
                        return Some(StaticEvalOutcome::Throw(StaticThrowValue::NamedError(
                            "TypeError",
                        )));
                    };
                    self.resolve_static_function_outcome_from_binding_with_context(
                        &binding,
                        &default_argument,
                        current_function_name,
                    )?
                }
            }
        } else if let Some(function_binding) =
            self.resolve_member_function_binding(expression, &symbol_property)
        {
            let capture_slots =
                self.resolve_member_function_capture_slots(expression, &symbol_property);
            match self.resolve_static_function_outcome_from_binding_with_call_frame_and_context(
                &function_binding,
                &default_argument,
                expression,
                current_function_name,
            )? {
                StaticEvalOutcome::Value(value) => {
                    let value = capture_slots
                        .as_ref()
                        .map(|capture_slots| {
                            self.substitute_capture_slot_bindings(&value, capture_slots)
                        })
                        .unwrap_or(value);
                    StaticEvalOutcome::Value(self.materialize_static_expression(&value))
                }
                StaticEvalOutcome::Throw(throw_value) => StaticEvalOutcome::Throw(throw_value),
            }
        } else {
            let object_binding = self.resolve_object_binding_from_expression(expression)?;
            let method_value =
                self.resolve_object_binding_property_value(&object_binding, &symbol_property)?;
            if let Some(primitive) = self.resolve_static_primitive_expression_with_context(
                &method_value,
                current_function_name,
            ) {
                return match primitive {
                    Expression::Null | Expression::Undefined => None,
                    _ => Some(StaticEvalOutcome::Throw(StaticThrowValue::NamedError(
                        "TypeError",
                    ))),
                };
            }
            let Some(binding) = self.resolve_function_binding_from_expression_with_context(
                &method_value,
                current_function_name,
            ) else {
                return Some(StaticEvalOutcome::Throw(StaticThrowValue::NamedError(
                    "TypeError",
                )));
            };
            self.resolve_static_function_outcome_from_binding_with_context(
                &binding,
                &default_argument,
                current_function_name,
            )?
        };

        match call_result {
            StaticEvalOutcome::Throw(throw_value) => Some(StaticEvalOutcome::Throw(throw_value)),
            StaticEvalOutcome::Value(value) => {
                let value = if let Expression::Identifier(name) = &value
                    && let Some(global_value) = self.global_value_binding(name)
                    && let Some(primitive) = self.resolve_static_primitive_expression_with_context(
                        global_value,
                        current_function_name,
                    ) {
                    primitive
                } else {
                    value
                };
                if let Some(primitive) = self
                    .resolve_static_primitive_expression_with_context(&value, current_function_name)
                {
                    return Some(StaticEvalOutcome::Value(primitive));
                }
                match self.infer_value_kind(&value) {
                    Some(StaticValueKind::Object) | Some(StaticValueKind::Function) => Some(
                        StaticEvalOutcome::Throw(StaticThrowValue::NamedError("TypeError")),
                    ),
                    _ => None,
                }
            }
        }
    }
}
