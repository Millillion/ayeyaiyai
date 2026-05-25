use super::*;

impl<'a> FunctionCompiler<'a> {
    fn is_import_meta_call(callee: &Expression, arguments: &[CallArgument]) -> bool {
        matches!(callee, Expression::Identifier(function_name) if function_name == "__ayyImportMeta")
            && matches!(
                arguments,
                [] | [CallArgument::Expression(Expression::Number(_))]
                    | [CallArgument::Spread(Expression::Number(_))]
            )
    }

    fn optional_member_call_sequence_parts(
        &self,
        callee: &Expression,
    ) -> Option<(Expression, Expression)> {
        let Expression::Sequence(expressions) = callee else {
            return None;
        };
        let [
            Expression::Assign { name, value },
            Expression::Conditional {
                then_expression,
                else_expression,
                ..
            },
        ] = expressions.as_slice()
        else {
            return None;
        };
        if !matches!(then_expression.as_ref(), Expression::Undefined) {
            return None;
        }
        let Expression::Member { object, property } = else_expression.as_ref() else {
            return None;
        };
        if !matches!(object.as_ref(), Expression::Identifier(object_name) if object_name == name)
            || !Self::expression_references_internal_assignment_temp(object)
        {
            return None;
        }
        Some((value.as_ref().clone(), property.as_ref().clone()))
    }

    fn optional_call_base_is_statically_nullish(&self, value: &Expression) -> Option<bool> {
        let materialized = self.materialize_static_expression(value);
        match &materialized {
            Expression::Null | Expression::Undefined => return Some(true),
            Expression::Array(_)
            | Expression::Object(_)
            | Expression::This
            | Expression::New { .. } => {
                return Some(false);
            }
            _ => {}
        }

        self.infer_value_kind(&materialized)
            .or_else(|| self.infer_value_kind(value))
            .and_then(|kind| match kind {
                StaticValueKind::Null | StaticValueKind::Undefined => Some(true),
                StaticValueKind::Unknown => None,
                _ => Some(false),
            })
            .or_else(|| {
                self.resolve_object_binding_from_expression(value)
                    .map(|_| false)
            })
    }

    fn resolve_optional_member_call_sequence_result_expression(
        &self,
        callee: &Expression,
        arguments: &[CallArgument],
    ) -> Option<Expression> {
        let (object, property) = self.optional_member_call_sequence_parts(callee)?;
        if !inline_summary_side_effect_free_expression(&object)
            || !inline_summary_side_effect_free_expression(&property)
            || arguments
                .iter()
                .any(|argument| !inline_summary_side_effect_free_expression(argument.expression()))
        {
            return None;
        }
        if self.optional_call_base_is_statically_nullish(&object)? {
            return Some(Expression::Undefined);
        }

        let member_callee = Expression::Member {
            object: Box::new(object.clone()),
            property: Box::new(property.clone()),
        };
        self.resolve_static_call_result_expression_with_context(
            &member_callee,
            arguments,
            self.current_function_name(),
        )
        .map(|(value, _)| self.materialize_static_expression(&value))
        .or_else(|| {
            if !arguments.is_empty() {
                return None;
            }
            let Expression::String(property_name) = property else {
                return None;
            };
            match self.resolve_static_member_call_outcome_with_context(
                &object,
                &property_name,
                self.current_function_name(),
            )? {
                StaticEvalOutcome::Value(value) => Some(self.materialize_static_expression(&value)),
                StaticEvalOutcome::Throw(_) => None,
            }
        })
    }

    fn resolve_side_effect_free_static_call_object_binding(
        &self,
        callee: &Expression,
        arguments: &[CallArgument],
    ) -> Option<ObjectValueBinding> {
        if !inline_summary_side_effect_free_expression(callee)
            || arguments
                .iter()
                .any(|argument| !inline_summary_side_effect_free_expression(argument.expression()))
        {
            return None;
        }

        let (result_expression, _) = self.resolve_static_call_result_expression_with_context(
            callee,
            arguments,
            self.current_function_name(),
        )?;
        self.resolve_object_binding_from_expression(&result_expression)
    }

    pub(super) fn resolve_call_or_new_object_binding(
        &self,
        expression: &Expression,
    ) -> Option<ObjectValueBinding> {
        match expression {
            Expression::Call { callee, arguments } => {
                self.resolve_call_object_binding(callee, arguments)
            }
            Expression::New { callee, arguments } => {
                self.resolve_new_object_binding(expression, callee, arguments)
            }
            _ => None,
        }
    }

    fn resolve_call_object_binding(
        &self,
        callee: &Expression,
        arguments: &[CallArgument],
    ) -> Option<ObjectValueBinding> {
        if let Some(result_expression) =
            self.resolve_optional_member_call_sequence_result_expression(callee, arguments)
            && let Some(object_binding) =
                self.resolve_object_binding_from_expression(&result_expression)
        {
            return Some(object_binding);
        }
        if let Some(object_binding) =
            self.resolve_side_effect_free_static_call_object_binding(callee, arguments)
        {
            return Some(object_binding);
        }
        if Self::is_import_meta_call(callee, arguments) {
            let mut object_binding = empty_object_value_binding();
            object_binding_set_property(
                &mut object_binding,
                Expression::String("toString".to_string()),
                Expression::Identifier("String".to_string()),
            );
            return Some(object_binding);
        }
        if arguments.is_empty()
            && let Expression::Identifier(function_name) = callee
            && function_name.starts_with("__ayy_class_init_")
            && let Some(object_binding) =
                self.infer_static_class_init_constructor_object_binding(function_name)
        {
            return Some(object_binding);
        }
        if matches!(
            callee,
            Expression::Member { object, property }
                if matches!(object.as_ref(), Expression::Identifier(name) if name == "Object")
                    && matches!(property.as_ref(), Expression::String(name) if name == "defineProperty")
        ) {
            return self.resolve_object_define_property_call_binding(arguments);
        }
        if matches!(
            callee,
            Expression::Member { object, property }
                if matches!(object.as_ref(), Expression::Identifier(name) if name == "Object")
                    && matches!(property.as_ref(), Expression::String(name) if name == "create")
        ) {
            return Some(empty_object_value_binding());
        }
        if matches!(
            callee,
            Expression::Member { object, property }
                if matches!(object.as_ref(), Expression::Identifier(name) if name == "Object")
                    && matches!(property.as_ref(), Expression::String(name) if name == "getPrototypeOf")
        ) {
            let Some(CallArgument::Expression(target) | CallArgument::Spread(target)) =
                arguments.first()
            else {
                return None;
            };
            let prototype = self.resolve_static_object_prototype_expression(target)?;
            return self.resolve_object_binding_from_expression(&prototype);
        }
        if arguments.is_empty()
            && matches!(
                callee,
                Expression::Member { property, .. } if is_symbol_iterator_expression(property)
            )
        {
            let Expression::Member { object, property } = callee else {
                unreachable!("filtered above");
            };
            if let Some(object_binding) = self.resolve_object_binding_from_expression(object) {
                let has_next_method = object_binding_lookup_value(
                    &object_binding,
                    &Expression::String("next".to_string()),
                )
                .and_then(|value| self.resolve_function_binding_from_expression(value))
                .is_some();
                if has_next_method {
                    return Some(object_binding);
                }
                if self
                    .resolve_member_function_binding(object, property)
                    .is_some()
                    || self
                        .resolve_member_getter_binding(object, property)
                        .is_some()
                {
                    if let Some((result_expression, _)) = self
                        .resolve_static_call_result_expression_with_context(
                            callee,
                            arguments,
                            self.current_function_name(),
                        )
                        && let Some(result_binding) =
                            self.resolve_object_binding_from_expression(&result_expression)
                    {
                        return Some(result_binding);
                    }
                    if let Some(returned_binding) =
                        self.resolve_returned_object_binding_from_call(callee, arguments)
                    {
                        return Some(returned_binding);
                    }
                    return Some(empty_object_value_binding());
                }
                if self.resolve_iterator_source_kind(object).is_some() {
                    return Some(empty_object_value_binding());
                }
            }
            return self
                .resolve_iterator_source_kind(object)
                .map(|_| empty_object_value_binding());
        }
        if arguments.is_empty()
            && matches!(
                callee,
                Expression::Member { object, property }
                    if matches!(object.as_ref(), Expression::Identifier(name) if name == "$262")
                        && matches!(property.as_ref(), Expression::String(name) if name == "createRealm")
            )
        {
            return Some(empty_object_value_binding());
        }
        if let Some(LocalFunctionBinding::User(function_name)) = self
            .resolve_function_binding_from_expression_with_context(
                callee,
                self.current_function_name(),
            )
            && self
                .user_function(&function_name)
                .is_some_and(|function| matches!(function.kind, FunctionKind::Async))
        {
            return None;
        }
        self.resolve_native_error_object_binding(callee, arguments)
            .or_else(|| {
                self.resolve_static_call_result_expression_with_context(
                    callee,
                    arguments,
                    self.current_function_name(),
                )
                .filter(|_| matches!(callee, Expression::Identifier(_)))
                .and_then(|(result_expression, _)| {
                    self.resolve_object_binding_from_expression(&result_expression)
                })
            })
            .or_else(|| self.resolve_returned_object_binding_from_call(callee, arguments))
            .or_else(|| {
                if !arguments.is_empty() {
                    return None;
                }
                let LocalFunctionBinding::User(function_name) = self
                    .resolve_function_binding_from_expression_with_context(
                        callee,
                        self.current_function_name(),
                    )?
                else {
                    return None;
                };
                let (result_expression, _) = self
                    .execute_simple_static_user_function_with_bindings(
                        &function_name,
                        &HashMap::new(),
                    )?;
                self.resolve_object_binding_from_expression(&result_expression)
            })
    }

    fn resolve_object_define_property_call_binding(
        &self,
        arguments: &[CallArgument],
    ) -> Option<ObjectValueBinding> {
        let [
            CallArgument::Expression(target),
            CallArgument::Expression(property),
            CallArgument::Expression(descriptor_expression),
            ..,
        ] = arguments
        else {
            return None;
        };
        let descriptor = resolve_property_descriptor_definition(descriptor_expression)?;
        let property = self.canonical_object_property_expression(property);
        let mut object_binding = self.resolve_object_binding_from_expression(target)?;
        if !object_binding_can_define_property(&object_binding, &property) {
            return Some(object_binding);
        }

        let property_name = static_property_name_from_expression(&property);
        let existing_value = object_binding_lookup_value(&object_binding, &property).cloned();
        let existing_descriptor =
            object_binding_lookup_descriptor(&object_binding, &property).cloned();
        let current_enumerable = property_name.as_ref().is_some_and(|property_name| {
            !object_binding
                .non_enumerable_string_properties
                .iter()
                .any(|hidden_name| hidden_name == property_name)
        });
        let enumerable = descriptor.enumerable.unwrap_or_else(|| {
            existing_descriptor
                .as_ref()
                .map(|descriptor| descriptor.enumerable)
                .unwrap_or(current_enumerable)
        });
        let configurable = descriptor.configurable.unwrap_or_else(|| {
            existing_descriptor
                .as_ref()
                .map(|descriptor| descriptor.configurable)
                .unwrap_or(false)
        });
        let (value, writable, getter, setter, has_get, has_set) = if descriptor.is_accessor() {
            (
                None,
                None,
                descriptor.getter.as_ref().map(|expression| {
                    self.materialize_define_property_value_expression(expression)
                }),
                descriptor.setter.as_ref().map(|expression| {
                    self.materialize_define_property_value_expression(expression)
                }),
                descriptor.getter.is_some(),
                descriptor.setter.is_some(),
            )
        } else {
            let value = descriptor
                .value
                .as_ref()
                .map(|expression| self.materialize_define_property_value_expression(expression))
                .or(existing_value)
                .or_else(|| {
                    existing_descriptor
                        .as_ref()
                        .and_then(|descriptor| descriptor.value.clone())
                })
                .unwrap_or(Expression::Undefined);
            let writable = descriptor.writable.or_else(|| {
                existing_descriptor
                    .as_ref()
                    .and_then(|descriptor| descriptor.writable)
            });
            (
                Some(value),
                Some(writable.unwrap_or(false)),
                None,
                None,
                false,
                false,
            )
        };
        object_binding_define_property_descriptor(
            &mut object_binding,
            property,
            PropertyDescriptorBinding {
                value,
                configurable,
                enumerable,
                writable,
                getter,
                setter,
                has_get,
                has_set,
            },
        );
        Some(object_binding)
    }

    fn resolve_new_object_binding(
        &self,
        expression: &Expression,
        callee: &Expression,
        arguments: &[CallArgument],
    ) -> Option<ObjectValueBinding> {
        let typed_array_binding = self.resolve_static_typed_array_object_binding(callee, arguments);
        let object_binding = self
            .resolve_static_collection_object_binding(callee, arguments)
            .or_else(|| self.resolve_static_weak_collection_object_binding(callee, arguments))
            .or_else(|| self.resolve_native_error_object_binding(callee, arguments))
            .or_else(|| self.resolve_user_constructor_new_binding(expression, callee, arguments))
            .or_else(|| {
                (arguments.is_empty()
                    && matches!(callee, Expression::Identifier(name) if name == "Object"))
                .then(empty_object_value_binding)
            })
            .or_else(|| {
                matches!(callee, Expression::Identifier(name) if name == "WeakRef")
                    .then(empty_object_value_binding)
            });

        match (object_binding, typed_array_binding) {
            (Some(mut object_binding), Some(typed_array_binding)) => {
                self.merge_static_typed_array_object_binding(
                    &mut object_binding,
                    &typed_array_binding,
                );
                Some(object_binding)
            }
            (Some(object_binding), None) => Some(object_binding),
            (None, Some(typed_array_binding)) => Some(typed_array_binding),
            (None, None) => None,
        }
    }

    pub(in crate::backend::direct_wasm) fn object_binding_is_static_map(
        &self,
        object_binding: &ObjectValueBinding,
    ) -> bool {
        matches!(
            object_binding_lookup_value(
                object_binding,
                &map_collection_marker_property_expression()
            ),
            Some(Expression::Bool(true))
        )
    }

    pub(in crate::backend::direct_wasm) fn static_map_size_from_binding(
        &self,
        object_binding: &ObjectValueBinding,
    ) -> Option<f64> {
        object_binding_lookup_value(object_binding, &map_collection_size_property_expression())
            .and_then(|value| match value {
                Expression::Number(size) => Some(*size),
                _ => None,
            })
    }

    pub(in crate::backend::direct_wasm) fn define_static_map_size(
        &self,
        object_binding: &mut ObjectValueBinding,
        size: f64,
    ) {
        object_binding_define_property(
            object_binding,
            map_collection_size_property_expression(),
            Expression::Number(size),
            false,
        );
    }

    fn typed_array_constructor_names() -> [&'static str; 11] {
        [
            "Uint8Array",
            "Int8Array",
            "Uint16Array",
            "Int16Array",
            "Uint32Array",
            "Int32Array",
            "Float32Array",
            "Float64Array",
            "Uint8ClampedArray",
            "BigInt64Array",
            "BigUint64Array",
        ]
    }

    pub(in crate::backend::direct_wasm) fn resolve_static_typed_array_object_binding_from_expression(
        &self,
        expression: &Expression,
    ) -> Option<ObjectValueBinding> {
        let Expression::New { callee, arguments } = expression else {
            return None;
        };
        self.resolve_static_typed_array_object_binding(callee, arguments)
    }

    pub(in crate::backend::direct_wasm) fn resolve_static_typed_array_object_binding(
        &self,
        callee: &Expression,
        arguments: &[CallArgument],
    ) -> Option<ObjectValueBinding> {
        if let Expression::Identifier(name) = callee
            && typed_array_builtin_bytes_per_element(name).is_some()
            && self.is_unshadowed_builtin_identifier(name)
        {
            return self.synthesize_static_typed_array_object_binding(name, arguments);
        }

        if let Some(binding) = self.resolve_function_binding_from_expression(callee) {
            match binding {
                LocalFunctionBinding::Builtin(function_name)
                    if typed_array_builtin_bytes_per_element(&function_name).is_some() =>
                {
                    return self
                        .synthesize_static_typed_array_object_binding(&function_name, arguments);
                }
                LocalFunctionBinding::User(function_name) => {
                    if let Some(user_function) = self.user_function(&function_name) {
                        return self
                            .resolve_static_typed_array_object_binding_from_derived_constructor(
                                user_function,
                                arguments,
                                0,
                            );
                    }
                }
                _ => {}
            }
        }

        for constructor_name in Self::typed_array_constructor_names() {
            if self.constructor_callee_inherits_from_builtin_prototype(
                callee,
                arguments,
                constructor_name,
            ) {
                return self
                    .synthesize_static_typed_array_object_binding(constructor_name, arguments);
            }
        }
        None
    }

    fn resolve_static_typed_array_object_binding_from_derived_constructor(
        &self,
        user_function: &UserFunction,
        arguments: &[CallArgument],
        depth: usize,
    ) -> Option<ObjectValueBinding> {
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

        if let Expression::Identifier(name) = &resolved_callee
            && typed_array_builtin_bytes_per_element(name).is_some()
        {
            return self.synthesize_static_typed_array_object_binding(name, &substituted_arguments);
        }

        match self.resolve_function_binding_from_expression(&resolved_callee)? {
            LocalFunctionBinding::Builtin(function_name)
                if typed_array_builtin_bytes_per_element(&function_name).is_some() =>
            {
                self.synthesize_static_typed_array_object_binding(
                    &function_name,
                    &substituted_arguments,
                )
            }
            LocalFunctionBinding::User(function_name) => {
                let super_function = self.user_function(&function_name)?;
                self.resolve_static_typed_array_object_binding_from_derived_constructor(
                    super_function,
                    &substituted_arguments,
                    depth + 1,
                )
            }
            _ => {
                for constructor_name in Self::typed_array_constructor_names() {
                    if self.constructor_callee_inherits_from_builtin_prototype(
                        &resolved_callee,
                        &substituted_arguments,
                        constructor_name,
                    ) {
                        return self.synthesize_static_typed_array_object_binding(
                            constructor_name,
                            &substituted_arguments,
                        );
                    }
                }
                None
            }
        }
    }

    fn synthesize_static_typed_array_object_binding(
        &self,
        constructor_name: &str,
        arguments: &[CallArgument],
    ) -> Option<ObjectValueBinding> {
        let length = self.static_typed_array_initial_length(constructor_name, arguments)?;
        let bytes_per_element = typed_array_builtin_bytes_per_element(constructor_name)? as usize;
        let byte_length = length.checked_mul(bytes_per_element)?;
        let mut object_binding = empty_object_value_binding();
        object_binding_define_property(
            &mut object_binding,
            typed_array_name_property_expression(),
            Expression::String(constructor_name.to_string()),
            false,
        );
        object_binding_define_property(
            &mut object_binding,
            typed_array_length_property_expression(),
            Expression::Number(length as f64),
            false,
        );
        object_binding_define_property(
            &mut object_binding,
            viewed_array_buffer_property_expression(),
            Expression::New {
                callee: Box::new(Expression::Identifier("ArrayBuffer".to_string())),
                arguments: vec![CallArgument::Expression(Expression::Number(
                    byte_length as f64,
                ))],
            },
            false,
        );
        Some(object_binding)
    }

    fn static_typed_array_initial_length(
        &self,
        constructor_name: &str,
        arguments: &[CallArgument],
    ) -> Option<usize> {
        let expanded_arguments = self.expand_call_arguments(arguments);
        let Some(source) = expanded_arguments.first() else {
            return Some(0);
        };
        let materialized_source = self.materialize_static_expression(source);
        if matches!(materialized_source, Expression::Undefined)
            || matches!(&materialized_source, Expression::Identifier(name)
                if name == "undefined" && self.is_unshadowed_builtin_identifier(name))
        {
            return Some(0);
        }
        if let Some(length) = extract_typed_array_element_count(source) {
            return Some(length);
        }
        if let Some(length) = extract_typed_array_element_count(&materialized_source) {
            return Some(length);
        }
        if let Some((byte_length, _)) = self.resolve_array_buffer_binding_from_expression(source) {
            let bytes_per_element =
                typed_array_builtin_bytes_per_element(constructor_name)? as usize;
            return Some(byte_length / bytes_per_element);
        }
        self.resolve_array_binding_from_expression(source)
            .map(|binding| binding.values.len())
    }

    pub(in crate::backend::direct_wasm) fn merge_static_typed_array_object_binding(
        &self,
        object_binding: &mut ObjectValueBinding,
        typed_array_binding: &ObjectValueBinding,
    ) {
        for property in [
            typed_array_name_property_expression(),
            typed_array_length_property_expression(),
            viewed_array_buffer_property_expression(),
        ] {
            if let Some(value) = object_binding_lookup_value(typed_array_binding, &property) {
                object_binding_define_property(object_binding, property, value.clone(), false);
            }
        }
    }

    pub(in crate::backend::direct_wasm) fn static_typed_array_name_from_binding(
        &self,
        object_binding: &ObjectValueBinding,
    ) -> Option<String> {
        object_binding_lookup_value(object_binding, &typed_array_name_property_expression())
            .and_then(|value| match value {
                Expression::String(name) => Some(name.clone()),
                _ => None,
            })
    }

    pub(in crate::backend::direct_wasm) fn static_typed_array_length_from_binding(
        &self,
        object_binding: &ObjectValueBinding,
    ) -> Option<usize> {
        object_binding_lookup_value(object_binding, &typed_array_length_property_expression())
            .and_then(|value| match value {
                Expression::Number(length)
                    if length.is_finite() && *length >= 0.0 && length.fract() == 0.0 =>
                {
                    Some(*length as usize)
                }
                _ => None,
            })
    }

    pub(in crate::backend::direct_wasm) fn static_typed_array_member_value_from_binding(
        &self,
        object_binding: &ObjectValueBinding,
        property: &Expression,
    ) -> Option<Expression> {
        let constructor_name = self.static_typed_array_name_from_binding(object_binding)?;
        let length = self.static_typed_array_length_from_binding(object_binding)?;
        match property {
            Expression::String(name) if name == "length" => Some(Expression::Number(length as f64)),
            Expression::String(name) if name == "byteLength" => {
                let bytes_per_element =
                    typed_array_builtin_bytes_per_element(&constructor_name)? as usize;
                Some(Expression::Number(
                    length.checked_mul(bytes_per_element)? as f64
                ))
            }
            Expression::String(name) if name == "buffer" => object_binding_lookup_value(
                object_binding,
                &viewed_array_buffer_property_expression(),
            )
            .cloned(),
            _ => {
                let index = argument_index_from_expression(property)? as usize;
                if index >= length {
                    return Some(Expression::Undefined);
                }
                let value = object_binding_lookup_value(object_binding, property)
                    .cloned()
                    .unwrap_or(Expression::Number(0.0));
                Some(self.coerce_static_typed_array_element_value(&constructor_name, &value))
            }
        }
    }

    pub(in crate::backend::direct_wasm) fn resolve_static_typed_array_to_string_tag(
        &self,
        receiver: &Expression,
    ) -> Option<String> {
        let object_binding = self.resolve_object_binding_from_expression(receiver)?;
        let constructor_name = self.static_typed_array_name_from_binding(&object_binding)?;
        Some(format!("[object {constructor_name}]"))
    }

    fn coerce_static_typed_array_element_value(
        &self,
        constructor_name: &str,
        value: &Expression,
    ) -> Expression {
        let materialized = self.materialize_static_expression(value);
        let Some(number) = self
            .resolve_static_number_value(&materialized)
            .or_else(|| self.resolve_static_number_value(value))
        else {
            return materialized;
        };
        let coerced = match constructor_name {
            "Uint8Array" => Self::coerce_integer_typed_array_value(number, 8, false),
            "Int8Array" => Self::coerce_integer_typed_array_value(number, 8, true),
            "Uint16Array" => Self::coerce_integer_typed_array_value(number, 16, false),
            "Int16Array" => Self::coerce_integer_typed_array_value(number, 16, true),
            "Uint32Array" => Self::coerce_integer_typed_array_value(number, 32, false),
            "Int32Array" => Self::coerce_integer_typed_array_value(number, 32, true),
            "Uint8ClampedArray" => number.clamp(0.0, 255.0).round(),
            "Float32Array" => (number as f32) as f64,
            "Float64Array" => number,
            _ => number,
        };
        Expression::Number(coerced)
    }

    fn coerce_integer_typed_array_value(value: f64, bits: u32, signed: bool) -> f64 {
        let integer = if value.is_finite() {
            value.trunc() as i128
        } else {
            0
        };
        let modulus = 1_i128 << bits;
        let mut wrapped = integer % modulus;
        if wrapped < 0 {
            wrapped += modulus;
        }
        if signed {
            let sign_bit = 1_i128 << (bits - 1);
            if wrapped >= sign_bit {
                wrapped -= modulus;
            }
        }
        wrapped as f64
    }

    fn resolve_static_collection_object_binding(
        &self,
        callee: &Expression,
        arguments: &[CallArgument],
    ) -> Option<ObjectValueBinding> {
        if matches!(
            callee,
            Expression::Identifier(name)
                if matches!(name.as_str(), "Map" | "Set") && self.is_unshadowed_builtin_identifier(name)
        ) {
            if let Expression::Identifier(name) = callee {
                return self.synthesize_static_map_object_binding(name, arguments);
            }
        }

        if let Some(binding) = self.resolve_function_binding_from_expression(callee) {
            match binding {
                LocalFunctionBinding::Builtin(function_name)
                    if matches!(function_name.as_str(), "Map" | "Set") =>
                {
                    return self.synthesize_static_map_object_binding(&function_name, arguments);
                }
                LocalFunctionBinding::User(function_name) => {
                    if let Some(user_function) = self.user_function(&function_name) {
                        return self
                            .resolve_static_collection_object_binding_from_derived_constructor(
                                user_function,
                                arguments,
                                0,
                            );
                    }
                }
                _ => {}
            }
        }

        if self.constructor_callee_inherits_from_builtin_prototype(callee, arguments, "Map") {
            return self.synthesize_static_map_object_binding("Map", arguments);
        }
        if self.constructor_callee_inherits_from_builtin_prototype(callee, arguments, "Set") {
            return self.synthesize_static_map_object_binding("Set", arguments);
        }

        None
    }

    fn synthesize_static_map_object_binding(
        &self,
        kind: &str,
        arguments: &[CallArgument],
    ) -> Option<ObjectValueBinding> {
        let entries = self.static_map_initial_entries(kind, arguments)?;
        let size = entries.iter().filter(|entry| entry.is_some()).count() as f64;
        let mut object_binding = empty_object_value_binding();
        object_binding_define_property(
            &mut object_binding,
            map_collection_marker_property_expression(),
            Expression::Bool(true),
            false,
        );
        object_binding_define_property(
            &mut object_binding,
            map_collection_kind_property_expression(),
            Expression::String(kind.to_string()),
            false,
        );
        self.define_static_map_entries(&mut object_binding, entries);
        self.define_static_map_size(&mut object_binding, size);
        Some(object_binding)
    }

    fn resolve_static_collection_object_binding_from_derived_constructor(
        &self,
        user_function: &UserFunction,
        arguments: &[CallArgument],
        depth: usize,
    ) -> Option<ObjectValueBinding> {
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

        if let Expression::Identifier(name) = &resolved_callee
            && matches!(name.as_str(), "Map" | "Set")
        {
            return self.synthesize_static_map_object_binding(name, &substituted_arguments);
        }

        match self.resolve_function_binding_from_expression(&resolved_callee)? {
            LocalFunctionBinding::Builtin(function_name)
                if matches!(function_name.as_str(), "Map" | "Set") =>
            {
                self.synthesize_static_map_object_binding(&function_name, &substituted_arguments)
            }
            LocalFunctionBinding::User(function_name) => {
                let super_function = self.user_function(&function_name)?;
                self.resolve_static_collection_object_binding_from_derived_constructor(
                    super_function,
                    &substituted_arguments,
                    depth + 1,
                )
            }
            _ => None,
        }
    }

    fn static_map_initial_entries(
        &self,
        kind: &str,
        arguments: &[CallArgument],
    ) -> Option<Vec<Option<Expression>>> {
        let expanded_arguments = self.expand_call_arguments(arguments);
        let Some(source) = expanded_arguments.first() else {
            return Some(Vec::new());
        };
        let materialized_source = self.materialize_static_expression(source);
        if matches!(materialized_source, Expression::Undefined)
            || matches!(&materialized_source, Expression::Identifier(name)
                if name == "undefined" && self.is_unshadowed_builtin_identifier(name))
        {
            return Some(Vec::new());
        }
        let binding = self
            .resolve_array_binding_from_expression(source)
            .or_else(|| self.resolve_static_iterable_binding_from_expression(source))
            .or_else(|| {
                self.resolve_object_binding_from_expression(source)
                    .and_then(|binding| {
                        self.static_map_entries_from_binding(&binding)
                            .map(|values| ArrayValueBinding { values })
                    })
            })?;
        if kind == "Set" {
            return Some(binding.values);
        }
        binding
            .values
            .into_iter()
            .map(|entry| {
                let entry = entry?;
                let entry_binding = self.resolve_array_binding_from_expression(&entry)?;
                let key = entry_binding
                    .values
                    .first()
                    .and_then(|value| value.clone())?;
                let value = entry_binding
                    .values
                    .get(1)
                    .and_then(|value| value.clone())
                    .unwrap_or(Expression::Undefined);
                Some(Some(Expression::Array(vec![
                    ArrayElement::Expression(key),
                    ArrayElement::Expression(value),
                ])))
            })
            .collect()
    }

    pub(in crate::backend::direct_wasm) fn static_map_kind_from_binding(
        &self,
        object_binding: &ObjectValueBinding,
    ) -> Option<String> {
        object_binding_lookup_value(object_binding, &map_collection_kind_property_expression())
            .and_then(|value| match value {
                Expression::String(kind) => Some(kind.clone()),
                _ => None,
            })
    }

    pub(in crate::backend::direct_wasm) fn apply_static_map_mutation_metadata(
        &mut self,
        object: &Expression,
        property_name: &str,
        arguments: &[CallArgument],
    ) -> bool {
        if !matches!(property_name, "set" | "add" | "delete") {
            return false;
        }
        let Some(mut object_binding) = self.resolve_object_binding_from_expression(object) else {
            return false;
        };
        if !self.object_binding_is_static_map(&object_binding) {
            return false;
        }
        let Some(object_name) = self.static_collection_identifier_name(object) else {
            return false;
        };
        let Some(collection_kind) = self.static_map_kind_from_binding(&object_binding) else {
            return false;
        };
        let mut entries = self
            .static_map_entries_from_binding(&object_binding)
            .unwrap_or_default();
        match (collection_kind.as_str(), property_name, arguments) {
            (
                "Map",
                "set",
                [
                    CallArgument::Expression(key) | CallArgument::Spread(key),
                    CallArgument::Expression(value) | CallArgument::Spread(value),
                    ..,
                ],
            ) => {
                let entry = Expression::Array(vec![
                    ArrayElement::Expression(self.materialize_static_expression(key)),
                    ArrayElement::Expression(self.materialize_static_expression(value)),
                ]);
                if let Some(existing) = self.static_map_entry_index(&entries, key) {
                    entries[existing] = Some(entry);
                } else {
                    entries.push(Some(entry));
                }
            }
            (
                "Set",
                "add",
                [
                    CallArgument::Expression(value) | CallArgument::Spread(value),
                    ..,
                ],
            ) => {
                if self.static_map_entry_index(&entries, value).is_none() {
                    entries.push(Some(self.materialize_static_expression(value)));
                }
            }
            (
                "Map" | "Set",
                "delete",
                [
                    CallArgument::Expression(key) | CallArgument::Spread(key),
                    ..,
                ],
            ) => {
                if let Some(existing) = self.static_map_entry_index(&entries, key) {
                    entries.remove(existing);
                }
            }
            _ => return false,
        }
        let runtime_entries = entries.clone();
        let current_size = entries.iter().filter(|entry| entry.is_some()).count() as f64;
        self.define_static_map_entries(&mut object_binding, entries);
        self.define_static_map_size(&mut object_binding, current_size);
        self.sync_static_map_runtime_entries(&object_name, &collection_kind, &runtime_entries);
        self.state
            .speculation
            .static_semantics
            .set_local_object_binding(&object_name, object_binding.clone());
        if self.binding_name_is_global(&object_name) {
            self.backend
                .sync_global_object_binding(&object_name, Some(object_binding));
        }
        true
    }

    fn sync_static_map_runtime_entries(
        &mut self,
        object_name: &str,
        collection_kind: &str,
        entries: &[Option<Expression>],
    ) {
        let length_local = self.ensure_runtime_array_length_local(object_name);
        self.push_i32_const(entries.len() as i32);
        self.push_local_set(length_local);

        if collection_kind == "Map" {
            let mut keys = Vec::with_capacity(entries.len());
            let mut values = Vec::with_capacity(entries.len());
            for entry in entries {
                let (key, value) = entry
                    .as_ref()
                    .map(Self::static_map_runtime_entry_parts)
                    .unwrap_or((None, None));
                keys.push(key);
                values.push(value);
            }

            let key_name = Self::static_map_key_runtime_name(object_name);
            let key_length_local = self.ensure_runtime_array_length_local(&key_name);
            self.ensure_runtime_array_slots_for_binding(
                &key_name,
                &ArrayValueBinding { values: keys },
            );
            self.push_i32_const(entries.len() as i32);
            self.push_local_set(key_length_local);

            let value_name = Self::static_map_value_runtime_name(object_name);
            let value_length_local = self.ensure_runtime_array_length_local(&value_name);
            self.ensure_runtime_array_slots_for_binding(&value_name, &ArrayValueBinding { values });
            self.push_i32_const(entries.len() as i32);
            self.push_local_set(value_length_local);
        } else {
            self.ensure_runtime_array_slots_for_binding(
                object_name,
                &ArrayValueBinding {
                    values: entries.to_vec(),
                },
            );
        }
    }

    fn static_map_runtime_entry_parts(
        entry: &Expression,
    ) -> (Option<Expression>, Option<Expression>) {
        let Expression::Array(elements) = entry else {
            return (Some(entry.clone()), Some(entry.clone()));
        };
        let key = elements.first().and_then(|element| match element {
            ArrayElement::Expression(expression) => Some(expression.clone()),
            ArrayElement::Spread(_) => None,
        });
        let value = elements
            .get(1)
            .and_then(|element| match element {
                ArrayElement::Expression(expression) => Some(expression.clone()),
                ArrayElement::Spread(_) => None,
            })
            .or(Some(Expression::Undefined));
        (key, value)
    }

    pub(in crate::backend::direct_wasm) fn static_map_key_runtime_name(
        object_name: &str,
    ) -> String {
        format!("__ayy_map_keys_{object_name}")
    }

    pub(in crate::backend::direct_wasm) fn static_map_value_runtime_name(
        object_name: &str,
    ) -> String {
        format!("__ayy_map_values_{object_name}")
    }

    pub(in crate::backend::direct_wasm) fn static_map_entry_index(
        &self,
        entries: &[Option<Expression>],
        key: &Expression,
    ) -> Option<usize> {
        let materialized_key = self.materialize_static_expression(key);
        entries.iter().position(|entry| {
            let Some(entry) = entry else {
                return false;
            };
            let entry_key = match entry {
                Expression::Array(elements) => elements.first().and_then(|element| match element {
                    ArrayElement::Expression(expression) => Some(expression),
                    ArrayElement::Spread(_) => None,
                }),
                _ => Some(entry),
            };
            entry_key.is_some_and(|entry_key| {
                [key, &materialized_key].into_iter().any(|candidate| {
                    self.resolve_static_same_value_result_with_context(
                        entry_key,
                        candidate,
                        self.current_function_name(),
                    )
                    .unwrap_or(false)
                })
            })
        })
    }

    pub(in crate::backend::direct_wasm) fn static_collection_identifier_name(
        &self,
        object: &Expression,
    ) -> Option<String> {
        let Expression::Identifier(name) = object else {
            return None;
        };
        Some(
            self.resolve_current_local_binding(name)
                .map(|(resolved_name, _)| resolved_name)
                .unwrap_or_else(|| name.clone()),
        )
    }

    pub(in crate::backend::direct_wasm) fn static_map_entries_from_binding(
        &self,
        object_binding: &ObjectValueBinding,
    ) -> Option<Vec<Option<Expression>>> {
        let Expression::Array(entries) = object_binding_lookup_value(
            object_binding,
            &map_collection_entries_property_expression(),
        )?
        else {
            return None;
        };
        entries
            .iter()
            .map(|entry| match entry {
                ArrayElement::Expression(expression) => Some(Some(expression.clone())),
                ArrayElement::Spread(_) => None,
            })
            .collect()
    }

    pub(in crate::backend::direct_wasm) fn define_static_map_entries(
        &self,
        object_binding: &mut ObjectValueBinding,
        entries: Vec<Option<Expression>>,
    ) {
        object_binding_define_property(
            object_binding,
            map_collection_entries_property_expression(),
            Expression::Array(
                entries
                    .into_iter()
                    .map(|entry| ArrayElement::Expression(entry.unwrap_or(Expression::Undefined)))
                    .collect(),
            ),
            false,
        );
    }

    pub(in crate::backend::direct_wasm) fn static_weak_collection_kind_from_binding(
        &self,
        object_binding: &ObjectValueBinding,
    ) -> Option<String> {
        object_binding_lookup_value(object_binding, &weak_collection_kind_property_expression())
            .and_then(|value| match value {
                Expression::String(kind) => Some(kind.clone()),
                _ => None,
            })
    }

    pub(in crate::backend::direct_wasm) fn object_binding_is_static_weak_collection_kind(
        &self,
        object_binding: &ObjectValueBinding,
        expected_kind: &str,
    ) -> bool {
        self.static_weak_collection_kind_from_binding(object_binding)
            .is_some_and(|kind| kind == expected_kind)
    }

    pub(in crate::backend::direct_wasm) fn weak_collection_entry_property_for_key(
        &self,
        key: &Expression,
    ) -> Option<Expression> {
        let resolved = self
            .resolve_bound_alias_expression(key)
            .filter(|resolved| !static_expression_matches(resolved, key));
        let materialized = self.materialize_static_expression(key);
        [Some(key), resolved.as_ref(), Some(&materialized)]
            .into_iter()
            .flatten()
            .find_map(|candidate| match candidate {
                Expression::Identifier(name) => {
                    Some(weak_collection_entry_property_expression(name))
                }
                _ => None,
            })
    }

    pub(in crate::backend::direct_wasm) fn define_static_weak_collection_entry(
        &self,
        object_binding: &mut ObjectValueBinding,
        key: &Expression,
        value: Expression,
    ) -> Option<()> {
        let property = self.weak_collection_entry_property_for_key(key)?;
        object_binding_define_property(object_binding, property, value, false);
        Some(())
    }

    pub(in crate::backend::direct_wasm) fn static_weak_collection_entry_value(
        &self,
        object_binding: &ObjectValueBinding,
        key: &Expression,
    ) -> Option<Expression> {
        let property = self.weak_collection_entry_property_for_key(key)?;
        object_binding_lookup_value(object_binding, &property).cloned()
    }

    fn resolve_static_weak_collection_object_binding(
        &self,
        callee: &Expression,
        _arguments: &[CallArgument],
    ) -> Option<ObjectValueBinding> {
        let kind = match callee {
            Expression::Identifier(name)
                if matches!(name.as_str(), "WeakMap" | "WeakSet")
                    && self.is_unshadowed_builtin_identifier(name) =>
            {
                name.as_str()
            }
            _ => return None,
        };
        let mut object_binding = empty_object_value_binding();
        object_binding_define_property(
            &mut object_binding,
            weak_collection_kind_property_expression(),
            Expression::String(kind.to_string()),
            false,
        );
        Some(object_binding)
    }

    fn resolve_user_constructor_new_binding(
        &self,
        expression: &Expression,
        callee: &Expression,
        arguments: &[CallArgument],
    ) -> Option<ObjectValueBinding> {
        self.resolve_user_constructor_object_binding_from_new(callee, arguments)
            .map(|binding| {
                self.rewrite_static_new_this_object_binding_for_expression(&binding, expression)
            })
            .or_else(|| {
                let Expression::Call {
                    callee: init_callee,
                    arguments: init_arguments,
                } = callee
                else {
                    return None;
                };
                if !init_arguments.is_empty() {
                    return None;
                }
                let Expression::Identifier(function_name) = init_callee.as_ref() else {
                    return None;
                };
                let constructor_expression = self
                    .resolve_static_class_init_call_constructor_alias(function_name)
                    .map(Expression::Identifier)
                    .or_else(|| {
                        self.infer_static_class_init_call_result_expression(function_name)
                    })?;
                self.resolve_user_constructor_object_binding_from_new(
                    &constructor_expression,
                    arguments,
                )
                .map(|binding| {
                    self.rewrite_static_new_this_object_binding_for_expression(&binding, expression)
                })
            })
            .or_else(|| {
                self.state
                    .speculation
                    .static_semantics
                    .last_bound_user_function_call
                    .as_ref()
                    .filter(|snapshot| {
                        snapshot
                            .source_expression
                            .as_ref()
                            .is_some_and(|source| static_expression_matches(source, expression))
                    })
                    .and_then(|snapshot| snapshot.result_expression.as_ref())
                    .and_then(|result| self.resolve_object_binding_from_expression(result))
            })
    }

    fn resolve_native_error_object_binding(
        &self,
        callee: &Expression,
        arguments: &[CallArgument],
    ) -> Option<ObjectValueBinding> {
        if !matches!(
            callee,
            Expression::Identifier(name) if native_error_runtime_value(name).is_some()
        ) {
            return None;
        }

        let mut object_binding = empty_object_value_binding();
        if let Expression::Identifier(name) = callee {
            object_binding_set_property(
                &mut object_binding,
                Expression::String("constructor".to_string()),
                Expression::Identifier(name.clone()),
            );
            object_binding_set_property(
                &mut object_binding,
                Expression::String("name".to_string()),
                Expression::String(name.clone()),
            );
        }
        if let Some(
            CallArgument::Expression(message_expression) | CallArgument::Spread(message_expression),
        ) = arguments.first()
        {
            let materialized_message = self.materialize_static_expression(message_expression);
            if !matches!(materialized_message, Expression::Undefined)
                && !matches!(&materialized_message, Expression::Identifier(name)
                    if name == "undefined" && self.is_unshadowed_builtin_identifier(name))
            {
                object_binding_set_property(
                    &mut object_binding,
                    Expression::String("message".to_string()),
                    materialized_message,
                );
            }
        }
        Some(object_binding)
    }
}
