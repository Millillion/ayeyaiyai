use super::*;

thread_local! {
    static ARRAY_BINDING_RESOLUTION_STACK: std::cell::RefCell<Vec<Expression>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

impl<'a> FunctionCompiler<'a> {
    pub(in crate::backend::direct_wasm) fn resolve_test262_to_numbers_call_binding(
        &self,
        callee: &Expression,
        arguments: &[CallArgument],
    ) -> Option<ArrayValueBinding> {
        let LocalFunctionBinding::User(function_name) =
            self.resolve_function_binding_from_expression(callee)?
        else {
            return None;
        };
        let declaration = self.prepared_function_declaration(&function_name)?;
        if declaration.name != "ToNumbers" || declaration.params.len() != 1 {
            return None;
        }

        let CallArgument::Expression(source) = arguments.first()? else {
            return None;
        };
        let source_binding = self.resolve_array_binding_from_expression(source)?;
        Some(ArrayValueBinding {
            values: source_binding
                .values
                .into_iter()
                .map(|value| {
                    value.map(|expression| self.materialize_static_expression(&expression))
                })
                .collect(),
        })
    }

    fn push_for_in_key_candidate(
        values: &mut Vec<Option<Expression>>,
        seen: &mut std::collections::HashSet<String>,
        name: &str,
    ) {
        if seen.insert(name.to_string()) {
            values.push(Some(Expression::String(name.to_string())));
        }
    }

    fn append_for_in_keys_from_object_binding(
        values: &mut Vec<Option<Expression>>,
        seen: &mut std::collections::HashSet<String>,
        object_binding: &ObjectValueBinding,
    ) {
        for name in ordered_object_property_names(object_binding) {
            if object_binding
                .non_enumerable_string_properties
                .iter()
                .any(|hidden_name| hidden_name == &name)
            {
                continue;
            }
            Self::push_for_in_key_candidate(values, seen, &name);
        }
    }

    fn expression_is_top_level_global_object_for_enumeration(
        &self,
        expression: &Expression,
    ) -> bool {
        self.expression_is_top_level_global_object_for_enumeration_inner(expression, 0)
    }

    fn expression_is_top_level_global_object_for_enumeration_inner(
        &self,
        expression: &Expression,
        depth: usize,
    ) -> bool {
        if matches!(expression, Expression::Identifier(name) if name == "globalThis" && self.is_unshadowed_builtin_identifier(name))
            || (self.state.speculation.execution_context.top_level_function
                && matches!(expression, Expression::This))
        {
            return true;
        }

        if depth >= 8 {
            return false;
        }

        if let Some(resolved) = self
            .resolve_bound_alias_expression(expression)
            .filter(|resolved| !static_expression_matches(resolved, expression))
        {
            return self
                .expression_is_top_level_global_object_for_enumeration_inner(&resolved, depth + 1);
        }

        let materialized = self.materialize_static_expression(expression);
        if static_expression_matches(&materialized, expression) {
            return false;
        }

        self.expression_is_top_level_global_object_for_enumeration_inner(&materialized, depth + 1)
    }

    fn append_for_in_keys_from_global_property_descriptors(
        &self,
        values: &mut Vec<Option<Expression>>,
        seen: &mut std::collections::HashSet<String>,
    ) {
        let mut enumerable_names = self
            .backend
            .global_semantics
            .values
            .property_descriptors
            .iter()
            .filter_map(|(name, descriptor)| descriptor.enumerable.then_some(name.clone()))
            .collect::<Vec<_>>();
        enumerable_names.sort();
        for name in enumerable_names {
            Self::push_for_in_key_candidate(values, seen, &name);
        }
    }

    fn static_for_in_enumerated_keys_binding(
        &self,
        expression: &Expression,
    ) -> Option<ArrayValueBinding> {
        let trace_for_in_keys = std::env::var_os("AYY_TRACE_FOR_IN_KEYS").is_some();
        if let Some(module_index) = self.module_namespace_index_from_expression(expression) {
            let result = self
                .resolve_static_dynamic_import_namespace_own_property_names_binding(module_index);
            if trace_for_in_keys {
                eprintln!(
                    "for_in_keys:module_namespace expression={expression:?} module_index={module_index} values={:?}",
                    result.as_ref().map(|binding| &binding.values)
                );
            }
            return result;
        }

        if let Some(array_binding) = self.resolve_array_binding_from_expression(expression) {
            if trace_for_in_keys {
                eprintln!(
                    "for_in_keys:array expression={expression:?} values={:?}",
                    array_binding.values
                );
            }
            return Some(enumerated_keys_from_array_binding(&array_binding));
        }

        let mut values = Vec::new();
        let mut seen = std::collections::HashSet::new();
        let is_top_level_global_object =
            self.expression_is_top_level_global_object_for_enumeration(expression);
        if is_top_level_global_object {
            self.append_for_in_keys_from_global_property_descriptors(&mut values, &mut seen);
            if trace_for_in_keys {
                eprintln!("for_in_keys:global expression={expression:?} values={values:?}");
            }
        }

        if let Some(object_binding) = self.resolve_object_binding_from_expression(expression) {
            if trace_for_in_keys {
                eprintln!(
                    "for_in_keys:object expression={expression:?} props={:?} hidden={:?}",
                    ordered_object_property_names(&object_binding),
                    object_binding.non_enumerable_string_properties
                );
            }
            Self::append_for_in_keys_from_object_binding(&mut values, &mut seen, &object_binding);
        } else if is_top_level_global_object {
            if trace_for_in_keys {
                eprintln!("for_in_keys:result expression={expression:?} values={values:?}");
            }
            return Some(ArrayValueBinding { values });
        } else {
            return None;
        }

        let mut prototype = self.resolve_static_object_prototype_expression(expression);
        for _ in 0..32 {
            let Some(current_prototype) = prototype else {
                break;
            };
            let materialized_prototype = self.materialize_static_expression(&current_prototype);
            if matches!(materialized_prototype, Expression::Null) {
                break;
            }

            for candidate in [&current_prototype, &materialized_prototype] {
                let Some(prototype_binding) =
                    self.resolve_object_binding_from_expression(candidate)
                else {
                    continue;
                };
                if trace_for_in_keys {
                    eprintln!(
                        "for_in_keys:prototype candidate={candidate:?} props={:?} hidden={:?}",
                        ordered_object_property_names(&prototype_binding),
                        prototype_binding.non_enumerable_string_properties
                    );
                }
                Self::append_for_in_keys_from_object_binding(
                    &mut values,
                    &mut seen,
                    &prototype_binding,
                );
                break;
            }

            let next_prototype = self
                .resolve_static_object_prototype_expression(&materialized_prototype)
                .or_else(|| self.resolve_static_object_prototype_expression(&current_prototype));
            if let Some(next_prototype) = next_prototype.as_ref()
                && (static_expression_matches(next_prototype, &current_prototype)
                    || static_expression_matches(next_prototype, &materialized_prototype))
            {
                break;
            }
            prototype = next_prototype;
        }

        if trace_for_in_keys {
            eprintln!("for_in_keys:result expression={expression:?} values={values:?}");
        }
        Some(ArrayValueBinding { values })
    }

    fn array_constructor_binding_from_arguments(
        &self,
        expanded_arguments: Vec<Expression>,
    ) -> ArrayValueBinding {
        if expanded_arguments.is_empty() {
            return ArrayValueBinding { values: Vec::new() };
        }
        if expanded_arguments.len() == 1
            && let Some(length) = self.resolve_static_number_value(&expanded_arguments[0])
            && length.is_finite()
            && length >= 0.0
            && length.fract() == 0.0
        {
            return ArrayValueBinding {
                values: vec![None; length as usize],
            };
        }
        ArrayValueBinding {
            values: expanded_arguments.into_iter().map(Some).collect(),
        }
    }

    fn resolve_array_concat_binding(
        &self,
        receiver: &Expression,
        arguments: &[CallArgument],
    ) -> Option<ArrayValueBinding> {
        let mut values = self
            .resolve_array_binding_from_expression(receiver)?
            .values
            .clone();
        for argument in self.expand_call_arguments(arguments) {
            if let Some(array_binding) = self.resolve_array_binding_from_expression(&argument) {
                values.extend(array_binding.values);
            } else {
                values.push(Some(self.materialize_static_expression(&argument)));
            }
        }
        Some(ArrayValueBinding { values })
    }

    pub(in crate::backend::direct_wasm) fn substitute_constructor_call_frame_bindings_with_rest(
        &self,
        expression: &Expression,
        user_function: &UserFunction,
        arguments: &[CallArgument],
        this_binding: &Expression,
        arguments_binding: &Expression,
    ) -> Expression {
        let Some(declaration) = self.resolve_registered_function_declaration(&user_function.name)
        else {
            return self.substitute_user_function_call_frame_bindings(
                expression,
                user_function,
                arguments,
                this_binding,
                arguments_binding,
            );
        };

        let expanded_arguments = self.expand_call_arguments(arguments);
        let mut bindings = HashMap::new();
        for (index, parameter) in declaration.params.iter().enumerate() {
            let value = if parameter.rest {
                Expression::Array(
                    expanded_arguments
                        .iter()
                        .skip(index)
                        .cloned()
                        .map(crate::ir::hir::ArrayElement::Expression)
                        .collect(),
                )
            } else {
                expanded_arguments
                    .get(index)
                    .cloned()
                    .unwrap_or(Expression::Undefined)
            };
            bindings.insert(parameter.name.clone(), value);
        }

        let substituted = self.substitute_expression_bindings(expression, &bindings);
        self.substitute_call_frame_special_bindings(
            &substituted,
            user_function,
            this_binding,
            arguments_binding,
        )
    }

    fn resolve_array_binding_from_derived_array_constructor_new(
        &self,
        constructed_callee: &Expression,
        user_function: &UserFunction,
        arguments: &[CallArgument],
        depth: usize,
    ) -> Option<ArrayValueBinding> {
        if depth > 16 || !self.user_function_is_derived_constructor(user_function) {
            return None;
        }
        let declaration = self.resolve_registered_function_declaration(&user_function.name)?;
        if declaration
            .body
            .iter()
            .any(|statement| matches!(statement, Statement::Return(_)))
        {
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
        let capture_resolved_callee = match &substituted_callee {
            Expression::Identifier(name) => self
                .resolve_constructor_capture_source_bindings_from_expression(constructed_callee)
                .and_then(|bindings| bindings.get(name).cloned()),
            _ => None,
        };
        let resolved_callee = capture_resolved_callee
            .or_else(|| {
                self.resolve_bound_alias_expression(&substituted_callee)
                    .filter(|resolved| !static_expression_matches(resolved, &substituted_callee))
            })
            .or_else(|| match &substituted_callee {
                Expression::Identifier(name) => self
                    .state
                    .speculation
                    .static_semantics
                    .local_value_binding(name)
                    .cloned()
                    .or_else(|| {
                        self.backend
                            .global_semantics
                            .values
                            .value_bindings
                            .get(name)
                            .cloned()
                    }),
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

        match self.resolve_function_binding_from_expression(&resolved_callee)? {
            LocalFunctionBinding::Builtin(function_name) if function_name == "Array" => {
                Some(self.array_constructor_binding_from_arguments(
                    self.expand_call_arguments(&substituted_arguments),
                ))
            }
            LocalFunctionBinding::User(function_name) => {
                let super_function = self.user_function(&function_name)?;
                self.resolve_array_binding_from_derived_array_constructor_new(
                    &resolved_callee,
                    super_function,
                    &substituted_arguments,
                    depth + 1,
                )
            }
            _ => None,
        }
    }

    fn push_static_array_constructor_callee_candidate(
        candidates: &mut Vec<Expression>,
        candidate: Expression,
    ) {
        if candidates
            .iter()
            .any(|existing| static_expression_matches(existing, &candidate))
        {
            return;
        }
        candidates.push(candidate);
    }

    fn static_array_constructor_callee_candidates(&self, callee: &Expression) -> Vec<Expression> {
        let mut candidates = vec![callee.clone()];
        if let Some(resolved) = self
            .resolve_bound_alias_expression(callee)
            .filter(|resolved| !static_expression_matches(resolved, callee))
        {
            Self::push_static_array_constructor_callee_candidate(&mut candidates, resolved);
        }
        let materialized_callee = self.materialize_static_expression(callee);
        if !static_expression_matches(&materialized_callee, callee) {
            Self::push_static_array_constructor_callee_candidate(
                &mut candidates,
                materialized_callee,
            );
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
            Self::push_static_array_constructor_callee_candidate(&mut candidates, value.clone());
        }
        if let Expression::Identifier(name) = callee {
            if let Some(alias) = self.resolve_static_class_init_constructor_alias(name) {
                Self::push_static_array_constructor_callee_candidate(
                    &mut candidates,
                    Expression::Identifier(alias),
                );
            }
            if let Some(alias) = self.resolve_static_class_init_local_alias_expression(name) {
                Self::push_static_array_constructor_callee_candidate(&mut candidates, alias);
            }
        }
        candidates
    }

    fn static_array_constructor_candidate_is_builtin_array(&self, candidate: &Expression) -> bool {
        if matches!(candidate, Expression::Identifier(name) if name == "Array" && self.is_unshadowed_builtin_identifier(name))
        {
            return true;
        }
        self.resolve_function_binding_from_expression(candidate)
            .is_some_and(
                |binding| matches!(binding, LocalFunctionBinding::Builtin(name) if name == "Array"),
            )
    }

    fn user_function_from_static_array_constructor_candidate(
        &self,
        candidate: &Expression,
    ) -> Option<&UserFunction> {
        let candidate_function_name = match candidate {
            Expression::Call { callee, arguments } if arguments.is_empty() => {
                let Expression::Identifier(function_name) = callee.as_ref() else {
                    return None;
                };
                self.resolve_static_class_init_call_constructor_alias(function_name)
            }
            _ => self
                .resolve_function_binding_from_expression(candidate)
                .and_then(|binding| match binding {
                    LocalFunctionBinding::User(function_name) => Some(function_name),
                    LocalFunctionBinding::Builtin(_) => None,
                }),
        }
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
        })?;

        self.user_function(&candidate_function_name).or_else(|| {
            self.backend
                .function_registry
                .catalog
                .user_function(&candidate_function_name)
        })
    }

    pub(in crate::backend::direct_wasm) fn resolve_array_slice_binding(
        &self,
        object: &Expression,
        arguments: &[CallArgument],
    ) -> Option<ArrayValueBinding> {
        let array_binding = self.resolve_array_binding_from_expression(object)?;
        let start = match arguments.first() {
            None => 0usize,
            Some(CallArgument::Expression(expression)) | Some(CallArgument::Spread(expression)) => {
                self.resolve_static_number_value(expression)?.max(0.0) as usize
            }
        };
        let end = match arguments.get(1) {
            None => array_binding.values.len(),
            Some(CallArgument::Expression(expression)) | Some(CallArgument::Spread(expression)) => {
                self.resolve_static_number_value(expression)?.max(0.0) as usize
            }
        };
        let start = start.min(array_binding.values.len());
        let end = end.min(array_binding.values.len()).max(start);
        Some(ArrayValueBinding {
            values: array_binding.values[start..end].to_vec(),
        })
    }

    pub(in crate::backend::direct_wasm) fn resolve_array_binding_from_expression(
        &self,
        expression: &Expression,
    ) -> Option<ArrayValueBinding> {
        let reentered = ARRAY_BINDING_RESOLUTION_STACK.with(|stack| {
            stack
                .borrow()
                .iter()
                .any(|visited| static_expression_matches(visited, expression))
        });
        if reentered {
            return None;
        }

        ARRAY_BINDING_RESOLUTION_STACK.with(|stack| {
            stack.borrow_mut().push(expression.clone());
        });
        let result = self.resolve_array_binding_from_expression_inner(expression);
        ARRAY_BINDING_RESOLUTION_STACK.with(|stack| {
            stack.borrow_mut().pop();
        });
        result
    }

    fn resolve_array_binding_from_expression_inner(
        &self,
        expression: &Expression,
    ) -> Option<ArrayValueBinding> {
        if std::env::var_os("AYY_TRACE_FOR_IN_KEYS").is_some()
            && matches!(expression, Expression::EnumerateKeys(_))
        {
            eprintln!("for_in_keys:resolve_array expression={expression:?}");
        }
        if let Expression::Identifier(name) = expression {
            if let Some(binding) = self
                .state
                .speculation
                .static_semantics
                .local_typed_array_view_binding(name)
                .and_then(|view| self.typed_array_view_static_values(view))
                .or_else(|| {
                    self.state
                        .speculation
                        .static_semantics
                        .local_array_binding(name)
                        .cloned()
                })
                .or_else(|| {
                    let hidden_name = self.resolve_user_function_capture_hidden_name(name)?;
                    self.global_array_binding(&hidden_name).cloned()
                })
                .or_else(|| self.global_array_binding(name).cloned())
            {
                return Some(binding);
            }
        }

        if let Some(resolved) = self
            .resolve_bound_alias_expression(expression)
            .filter(|resolved| !static_expression_matches(resolved, expression))
        {
            return self.resolve_array_binding_from_expression(&resolved);
        }

        let binding = match expression {
            Expression::Assign { value, .. }
            | Expression::AssignMember { value, .. }
            | Expression::AssignSuperMember { value, .. } => {
                self.resolve_array_binding_from_expression(value)
            }
            Expression::Sequence(expressions) => expressions
                .last()
                .and_then(|expression| self.resolve_array_binding_from_expression(expression)),
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                let branch = if self.resolve_static_if_condition_value(condition)? {
                    then_expression.as_ref()
                } else {
                    else_expression.as_ref()
                };
                self.resolve_array_binding_from_expression(branch)
            }
            Expression::Identifier(name) => self
                .state
                .speculation
                .static_semantics
                .local_typed_array_view_binding(name)
                .and_then(|view| self.typed_array_view_static_values(view))
                .or_else(|| {
                    self.state
                        .speculation
                        .static_semantics
                        .local_array_binding(name)
                        .cloned()
                })
                .or_else(|| {
                    let hidden_name = self.resolve_user_function_capture_hidden_name(name)?;
                    self.global_array_binding(&hidden_name).cloned()
                })
                .or_else(|| self.global_array_binding(name).cloned()),
            Expression::EnumerateKeys(value) => self.static_for_in_enumerated_keys_binding(value),
            Expression::Member { object, property } => {
                let property = self
                    .resolve_property_key_expression(property)
                    .unwrap_or_else(|| self.materialize_static_expression(property));
                if let Some(value) =
                    self.resolve_module_namespace_live_binding_member_value(object, &property)
                {
                    if let Some(binding) = self.resolve_array_binding_from_expression(&value) {
                        return Some(binding);
                    }
                }
                if let Expression::Identifier(name) = object.as_ref()
                    && let Some(module_index) = Self::module_index_from_namespace_like_identifier(name)
                    && let Some(initializer) = self
                        .resolve_static_dynamic_import_namespace_live_binding_member_initializer_value(
                            module_index,
                            &property,
                        )
                    && let Some(binding) = self.resolve_array_binding_from_expression(&initializer)
                {
                    return Some(binding);
                }
                if self.runtime_object_property_shadow_deletion_is_statically_present(
                    object, &property,
                ) {
                    return None;
                }
                if let Some(shadow_binding_name) = self
                    .runtime_object_property_shadow_binding_name_for_expression(object, &property)
                    && let Some(value) = self
                        .global_value_binding(&shadow_binding_name)
                        .cloned()
                        .or_else(|| {
                            self.backend
                                .shared_global_semantics
                                .values
                                .value_bindings
                                .get(&shadow_binding_name)
                                .cloned()
                        })
                    && let Some(array_binding) = self.resolve_array_binding_from_expression(&value)
                {
                    return Some(array_binding);
                }
                if let Some(IteratorStepBinding::Runtime {
                    static_value: Some(value),
                    ..
                }) = self.resolve_iterator_step_binding_from_expression(object)
                    && matches!(property, Expression::String(ref name) if name == "value")
                    && let Some(array_binding) = self.resolve_array_binding_from_expression(&value)
                {
                    return Some(array_binding);
                }
                if let Some(object_binding) = self.resolve_object_binding_from_expression(object)
                    && let Some(value) =
                        self.resolve_object_binding_property_value(&object_binding, &property)
                    && let Some(array_binding) = self.resolve_array_binding_from_expression(&value)
                {
                    return Some(array_binding);
                }
                let array_binding = self.resolve_array_binding_from_expression(object)?;
                let index = argument_index_from_expression(&property)?;
                let value = array_binding.values.get(index as usize)?.clone()?;
                self.resolve_array_binding_from_expression(&value)
            }
            Expression::Call { callee, arguments } => {
                if matches!(callee.as_ref(), Expression::Identifier(name) if name == "__ayyTemplateObject")
                    && let Some(CallArgument::Expression(cooked) | CallArgument::Spread(cooked)) =
                        arguments.get(1)
                {
                    return self.resolve_array_binding_from_expression(cooked);
                }
                if matches!(
                    callee.as_ref(),
                    Expression::Member { object, property }
                        if matches!(object.as_ref(), Expression::Identifier(name) if name == "Array")
                            && matches!(property.as_ref(), Expression::String(name) if name == "from")
                ) && let Some(CallArgument::Expression(target) | CallArgument::Spread(target)) =
                    arguments.first()
                    && let Some(binding) = self.static_typed_array_values_from_expression(target)
                {
                    return Some(binding);
                }
                if let Some(binding) =
                    self.static_builtin_object_array_call_binding(callee, arguments)
                {
                    return Some(binding);
                }
                if let Some(binding) =
                    self.resolve_test262_to_numbers_call_binding(callee, arguments)
                {
                    return Some(binding);
                }
                if matches!(callee.as_ref(), Expression::Identifier(name) if name == "CollectValues")
                    && let Some(binding) =
                        self.resolve_static_collect_values_call_binding(arguments)
                {
                    return Some(binding);
                }
                if let Expression::Member { object, property } = callee.as_ref() {
                    if matches!(property.as_ref(), Expression::String(name) if name == "concat") {
                        return self.resolve_array_concat_binding(object, arguments);
                    }
                    if matches!(property.as_ref(), Expression::String(name) if name == "slice") {
                        return self.resolve_array_slice_binding(object, arguments);
                    }
                }
                let Expression::Identifier(name) = callee.as_ref() else {
                    return None;
                };
                let user_function = self.resolve_user_function_from_callee_name(name)?;
                let param_index = user_function.enumerated_keys_param_index?;
                let argument = match arguments.get(param_index) {
                    Some(CallArgument::Expression(expression))
                    | Some(CallArgument::Spread(expression)) => expression,
                    None => return Some(ArrayValueBinding { values: Vec::new() }),
                };
                self.static_enumerated_keys_binding(argument)
            }
            Expression::New { callee, arguments } => {
                for candidate in self.static_array_constructor_callee_candidates(callee) {
                    if self.static_array_constructor_candidate_is_builtin_array(&candidate) {
                        return Some(self.array_constructor_binding_from_arguments(
                            self.expand_call_arguments(arguments),
                        ));
                    }
                    let Some(user_function) =
                        self.user_function_from_static_array_constructor_candidate(&candidate)
                    else {
                        continue;
                    };
                    if let Some(binding) = self
                        .resolve_array_binding_from_derived_array_constructor_new(
                            &candidate,
                            user_function,
                            arguments,
                            0,
                        )
                    {
                        return Some(binding);
                    }
                    let Some(param_index) = user_function.enumerated_keys_param_index else {
                        continue;
                    };
                    let argument = match arguments.get(param_index) {
                        Some(CallArgument::Expression(expression))
                        | Some(CallArgument::Spread(expression)) => expression,
                        None => return Some(ArrayValueBinding { values: Vec::new() }),
                    };
                    if let Some(binding) = self.static_enumerated_keys_binding(argument) {
                        return Some(binding);
                    }
                }
                None
            }
            Expression::Array(elements) => {
                let mut values = Vec::new();
                for element in elements {
                    match element {
                        crate::ir::hir::ArrayElement::Expression(expression) => {
                            values.push(Some(self.materialize_static_expression(expression)));
                        }
                        crate::ir::hir::ArrayElement::Spread(expression) => {
                            if let Some(binding) =
                                self.resolve_array_binding_from_expression(expression)
                            {
                                values.extend(binding.values);
                            } else if let Some(binding) =
                                self.resolve_static_iterable_binding_from_expression(expression)
                            {
                                values.extend(binding.values);
                            } else {
                                values.push(Some(self.materialize_static_expression(expression)));
                            }
                        }
                    }
                }
                Some(ArrayValueBinding { values })
            }
            _ => None,
        };
        if binding.is_some() {
            return binding;
        }

        let materialized = self.materialize_static_expression(expression);
        if !static_expression_matches(&materialized, expression) {
            return self.resolve_array_binding_from_expression(&materialized);
        }
        None
    }
}
