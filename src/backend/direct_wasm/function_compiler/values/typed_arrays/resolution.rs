use super::*;

impl<'a> FunctionCompiler<'a> {
    pub(in crate::backend::direct_wasm) fn resolve_local_resizable_array_buffer_binding_name(
        &self,
        name: &str,
    ) -> Option<String> {
        if self
            .state
            .speculation
            .static_semantics
            .local_resizable_array_buffer_binding(name)
            .is_some()
        {
            return Some(name.to_string());
        }
        let (resolved_name, _) = self.resolve_current_local_binding(name)?;
        self.state
            .speculation
            .static_semantics
            .local_resizable_array_buffer_binding(&resolved_name)
            .is_some()
            .then_some(resolved_name)
    }

    fn is_viewed_array_buffer_constructor_name(name: &str) -> bool {
        matches!(
            name,
            "DataView"
                | "Uint8Array"
                | "Int8Array"
                | "Uint16Array"
                | "Int16Array"
                | "Uint32Array"
                | "Int32Array"
                | "Float32Array"
                | "Float64Array"
                | "Uint8ClampedArray"
                | "BigInt64Array"
                | "BigUint64Array"
        )
    }

    pub(in crate::backend::direct_wasm) fn resolve_typed_array_element_count(
        &self,
        expression: &Expression,
    ) -> Option<usize> {
        if let Expression::Identifier(name) = expression {
            let mut candidates = vec![name.clone()];
            if let Some((resolved_name, _)) = self.resolve_current_local_binding(name) {
                candidates.push(resolved_name);
            }
            for candidate in candidates {
                if let Some(value) = self
                    .state
                    .speculation
                    .static_semantics
                    .local_value_binding(&candidate)
                    .cloned()
                    .or_else(|| self.global_value_binding(&candidate).cloned())
                    .filter(|value| {
                        !static_expression_matches(
                            value,
                            &Expression::Identifier(candidate.clone()),
                        )
                    })
                    && let Some(length) = self.resolve_typed_array_element_count(&value)
                {
                    return Some(length);
                }
            }
        }
        if let Expression::Binary { op, left, right } = expression {
            let left = self
                .resolve_typed_array_element_count(left)
                .map(|value| value as f64)
                .or_else(|| self.resolve_static_number_value(left))?;
            let right = self
                .resolve_typed_array_element_count(right)
                .map(|value| value as f64)
                .or_else(|| self.resolve_static_number_value(right))?;
            let number = match op {
                BinaryOp::Add => left + right,
                BinaryOp::Subtract => left - right,
                BinaryOp::Multiply => left * right,
                BinaryOp::Divide => left / right,
                BinaryOp::Modulo => left % right,
                BinaryOp::Exponentiate => left.powf(right),
                _ => return None,
            };
            if number.is_finite() && number.fract() == 0.0 && number >= 0.0 {
                return Some(number as usize);
            }
        }
        if let Some(number) = self.resolve_static_number_value(expression)
            && number.is_finite()
            && number.fract() == 0.0
            && number >= 0.0
        {
            return Some(number as usize);
        }
        extract_typed_array_element_count(expression).or_else(|| {
            let materialized = self.materialize_static_expression(expression);
            extract_typed_array_element_count(&materialized)
        })
    }

    fn resolve_typed_array_constructor_bytes_per_element(
        &self,
        callee: &Expression,
    ) -> Option<usize> {
        if let Expression::Identifier(name) = callee {
            let mut candidates = vec![name.clone()];
            if let Some((resolved_name, _)) = self.resolve_current_local_binding(name) {
                candidates.push(resolved_name);
            }
            for candidate in candidates {
                if let Some(bytes_per_element) = typed_array_builtin_bytes_per_element(&candidate) {
                    return Some(bytes_per_element as usize);
                }
                if let Some(value) = self
                    .state
                    .speculation
                    .static_semantics
                    .local_value_binding(&candidate)
                    .cloned()
                    .or_else(|| self.global_value_binding(&candidate).cloned())
                    .filter(|value| {
                        !static_expression_matches(
                            value,
                            &Expression::Identifier(candidate.clone()),
                        )
                    })
                    && let Some(bytes_per_element) =
                        self.resolve_typed_array_constructor_bytes_per_element(&value)
                {
                    return Some(bytes_per_element);
                }
                if let Some(LocalFunctionBinding::Builtin(function_name)) = self
                    .state
                    .speculation
                    .static_semantics
                    .local_function_binding(&candidate)
                    .cloned()
                    .or_else(|| self.backend.global_function_binding(&candidate).cloned())
                    && let Some(bytes_per_element) =
                        typed_array_builtin_bytes_per_element(&function_name)
                {
                    return Some(bytes_per_element as usize);
                }
            }
        }
        if let Some(LocalFunctionBinding::Builtin(function_name)) =
            self.resolve_function_binding_from_expression(callee)
            && let Some(bytes_per_element) = typed_array_builtin_bytes_per_element(&function_name)
        {
            return Some(bytes_per_element as usize);
        }
        if let Expression::Identifier(name) = self.materialize_static_expression(callee)
            && let Some(bytes_per_element) = typed_array_builtin_bytes_per_element(&name)
        {
            return Some(bytes_per_element as usize);
        }
        for constructor_name in [
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
        ] {
            if self.constructor_callee_inherits_from_builtin_prototype(
                callee,
                &[],
                constructor_name,
            ) {
                return typed_array_builtin_bytes_per_element(constructor_name)
                    .map(|bytes| bytes as usize);
            }
        }
        None
    }

    fn constructor_callee_inherits_from_viewed_array_buffer_prototype(
        &self,
        callee: &Expression,
        arguments: &[CallArgument],
    ) -> bool {
        let candidate_instance = Expression::New {
            callee: Box::new(callee.clone()),
            arguments: arguments.to_vec(),
        };
        for constructor_name in [
            "DataView",
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
        ] {
            let target_prototype = Self::prototype_member_expression(constructor_name);
            if self.expression_inherits_from_prototype_for_instanceof(
                &candidate_instance,
                &target_prototype,
            ) {
                return true;
            }
        }

        let Expression::Identifier(name) = callee else {
            return false;
        };
        let callee_prototype = Self::prototype_member_expression(name);
        [
            "DataView",
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
        .into_iter()
        .any(|constructor_name| {
            self.expression_inherits_from_prototype_for_instanceof(
                &callee_prototype,
                &Self::prototype_member_expression(constructor_name),
            )
        })
    }

    pub(in crate::backend::direct_wasm) fn resolve_static_constructed_viewed_array_buffer_expression(
        &self,
        expression: &Expression,
    ) -> Option<Expression> {
        let Expression::New { callee, arguments } = expression else {
            return None;
        };
        let Expression::Identifier(name) = callee.as_ref() else {
            return None;
        };
        let constructs_view = if self.is_unshadowed_builtin_identifier(name) {
            Self::is_viewed_array_buffer_constructor_name(name)
        } else {
            self.constructor_callee_inherits_from_viewed_array_buffer_prototype(callee, arguments)
        };
        if !constructs_view {
            return None;
        }

        let buffer_expression = match arguments.first()? {
            CallArgument::Expression(expression) | CallArgument::Spread(expression) => expression,
        };
        let materialized_buffer = self.materialize_static_expression(buffer_expression);
        if let Expression::Identifier(buffer_name) = &materialized_buffer
            && let Some(resolved_buffer_name) =
                self.resolve_local_resizable_array_buffer_binding_name(buffer_name)
        {
            return Some(Expression::Identifier(resolved_buffer_name));
        }
        if let Expression::Identifier(buffer_name) = &materialized_buffer
            && self
                .state
                .speculation
                .static_semantics
                .local_resizable_array_buffer_binding(buffer_name)
                .is_some()
        {
            return Some(materialized_buffer);
        }
        if let Expression::Identifier(buffer_name) = buffer_expression
            && let Some(resolved_buffer_name) =
                self.resolve_local_resizable_array_buffer_binding_name(buffer_name)
        {
            return Some(Expression::Identifier(resolved_buffer_name));
        }
        if let Expression::Identifier(buffer_name) = buffer_expression
            && self
                .state
                .speculation
                .static_semantics
                .local_resizable_array_buffer_binding(buffer_name)
                .is_some()
        {
            return Some(buffer_expression.clone());
        }
        self.resolve_array_buffer_binding_from_expression(buffer_expression)
            .map(|_| materialized_buffer)
    }

    pub(in crate::backend::direct_wasm) fn resolve_static_viewed_array_buffer_expression(
        &self,
        expression: &Expression,
    ) -> Option<Expression> {
        if let Some(object_binding) = self.resolve_object_binding_from_expression(expression)
            && let Some(value) = object_binding_lookup_value(
                &object_binding,
                &viewed_array_buffer_property_expression(),
            )
        {
            return Some(value.clone());
        }
        if let Expression::Identifier(name) = expression {
            if let Some(object_binding) = self.backend.global_object_binding(name)
                && let Some(value) = object_binding_lookup_value(
                    object_binding,
                    &viewed_array_buffer_property_expression(),
                )
            {
                return Some(value.clone());
            }
            let resolved_name = self
                .resolve_current_local_binding(name)
                .map(|(resolved_name, _)| resolved_name)
                .unwrap_or_else(|| name.clone());
            if let Some(value) = self
                .state
                .speculation
                .static_semantics
                .local_value_binding(&resolved_name)
                .filter(|value| !static_expression_matches(value, expression))
                && let Some(buffer) =
                    self.resolve_static_constructed_viewed_array_buffer_expression(value)
            {
                return Some(buffer);
            }
            if let Some(value) = self
                .global_value_binding(name)
                .filter(|value| !static_expression_matches(value, expression))
                && let Some(buffer) =
                    self.resolve_static_constructed_viewed_array_buffer_expression(value)
            {
                return Some(buffer);
            }
        }
        self.resolve_static_constructed_viewed_array_buffer_expression(expression)
    }

    fn resolve_synthetic_create_rab_binding_from_call(
        &self,
        callee: &Expression,
        arguments: &[CallArgument],
    ) -> Option<ResizableArrayBufferBinding> {
        let LocalFunctionBinding::User(function_name) =
            self.resolve_function_binding_from_expression(callee)?
        else {
            return None;
        };
        if function_name != "CreateRab" {
            return None;
        }
        let byte_length_expression = match arguments.first()? {
            CallArgument::Expression(expression) | CallArgument::Spread(expression) => expression,
        };
        let ctor_expression = match arguments.get(1)? {
            CallArgument::Expression(expression) | CallArgument::Spread(expression) => expression,
        };
        let bytes_per_element =
            self.resolve_typed_array_constructor_bytes_per_element(ctor_expression)?;
        let byte_length = self.resolve_typed_array_element_count(byte_length_expression)?;
        if bytes_per_element == 0 || byte_length % bytes_per_element != 0 {
            return None;
        }
        let length = byte_length / bytes_per_element;
        let max_length = length.checked_mul(2)?;
        Some(ResizableArrayBufferBinding {
            values: (0..length)
                .map(|index| Some(Expression::Number((index % 128) as f64)))
                .collect(),
            max_length,
            bytes_per_element,
        })
    }

    pub(in crate::backend::direct_wasm) fn resolve_resizable_array_buffer_binding_from_expression(
        &self,
        expression: &Expression,
    ) -> Option<ResizableArrayBufferBinding> {
        if let Expression::Identifier(name) = expression {
            if let Some(resolved_name) =
                self.resolve_local_resizable_array_buffer_binding_name(name)
            {
                return self
                    .state
                    .speculation
                    .static_semantics
                    .local_resizable_array_buffer_binding(&resolved_name)
                    .cloned();
            }
            if let Some(value) = self
                .state
                .speculation
                .static_semantics
                .last_bound_user_function_call
                .as_ref()
                .and_then(|snapshot| snapshot.updated_bindings.get(name))
                .filter(|value| !static_expression_matches(value, expression))
            {
                return self.resolve_resizable_array_buffer_binding_from_expression(value);
            }
        }

        let (callee, arguments) = match expression {
            Expression::New { callee, arguments } => (callee.as_ref(), arguments.as_slice()),
            Expression::Call { callee, arguments } => {
                if let Some(binding) =
                    self.resolve_synthetic_create_rab_binding_from_call(callee, arguments)
                {
                    return Some(binding);
                }
                if !matches!(callee.as_ref(), Expression::Identifier(_)) {
                    return None;
                }
                let resolved = self.resolve_static_call_result_expression(callee, arguments)?;
                return self.resolve_resizable_array_buffer_binding_from_expression(&resolved);
            }
            _ => return None,
        };

        if !matches!(callee, Expression::Identifier(name) if name == "ArrayBuffer") {
            return None;
        }

        let length = self.resolve_typed_array_element_count(match arguments.first()? {
            CallArgument::Expression(expression) | CallArgument::Spread(expression) => expression,
        })?;

        let max_length = arguments
            .get(1)
            .and_then(|argument| match argument {
                CallArgument::Expression(Expression::Object(entries))
                | CallArgument::Spread(Expression::Object(entries)) => {
                    entries.iter().find_map(|entry| {
                        let crate::ir::hir::ObjectEntry::Data { key, value } = entry else {
                            return None;
                        };
                        if !matches!(key, Expression::String(name) if name == "maxByteLength") {
                            return None;
                        }
                        self.resolve_typed_array_element_count(value)
                    })
                }
                _ => None,
            })
            .unwrap_or(length);

        Some(ResizableArrayBufferBinding {
            values: (0..length)
                .map(|index| Some(Expression::Number((index % 128) as f64)))
                .collect(),
            max_length,
            bytes_per_element: 1,
        })
    }

    pub(in crate::backend::direct_wasm) fn resolve_array_buffer_binding_from_expression(
        &self,
        expression: &Expression,
    ) -> Option<(usize, usize)> {
        let binding = self.resolve_resizable_array_buffer_binding_from_expression(expression)?;
        Some((
            binding
                .values
                .len()
                .checked_mul(binding.bytes_per_element)?,
            binding.max_length.checked_mul(binding.bytes_per_element)?,
        ))
    }

    pub(in crate::backend::direct_wasm) fn resolve_typed_array_view_binding_from_expression(
        &self,
        expression: &Expression,
    ) -> Option<TypedArrayViewBinding> {
        if let Expression::Identifier(name) = expression {
            if let Some(binding) = self
                .state
                .speculation
                .static_semantics
                .local_typed_array_view_binding(name)
                .cloned()
            {
                return Some(binding);
            }
            if let Some((resolved_name, _)) = self.resolve_current_local_binding(name) {
                return self
                    .state
                    .speculation
                    .static_semantics
                    .local_typed_array_view_binding(&resolved_name)
                    .cloned();
            }
        }

        let (callee, arguments) = match expression {
            Expression::New { callee, arguments } => (callee.as_ref(), arguments.as_slice()),
            Expression::Call { callee, arguments } => {
                if !matches!(callee.as_ref(), Expression::Identifier(_)) {
                    return None;
                }
                let resolved = self.resolve_static_call_result_expression(callee, arguments)?;
                return self.resolve_typed_array_view_binding_from_expression(&resolved);
            }
            _ => return None,
        };
        let bytes_per_element = self
            .resolve_typed_array_constructor_bytes_per_element(callee)
            .unwrap_or(1);
        let buffer_expression = match arguments.first()? {
            CallArgument::Expression(expression) | CallArgument::Spread(expression) => expression,
        };
        let Expression::Identifier(buffer_name) = buffer_expression else {
            return None;
        };
        let buffer_name = self.resolve_local_resizable_array_buffer_binding_name(buffer_name)?;

        let byte_offset = arguments
            .get(1)
            .and_then(|argument| match argument {
                CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                    self.resolve_typed_array_element_count(expression)
                }
            })
            .unwrap_or(0);
        if byte_offset % bytes_per_element != 0 {
            return None;
        }
        let offset = byte_offset / bytes_per_element;
        let fixed_length = arguments.get(2).and_then(|argument| match argument {
            CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                self.resolve_typed_array_element_count(expression)
            }
        });

        Some(TypedArrayViewBinding {
            buffer_name,
            offset,
            fixed_length,
            bytes_per_element,
        })
    }

    pub(in crate::backend::direct_wasm) fn typed_array_view_static_length(
        &self,
        view: &TypedArrayViewBinding,
    ) -> Option<usize> {
        let buffer = self
            .state
            .speculation
            .static_semantics
            .local_resizable_array_buffer_binding(&view.buffer_name)?;
        let scale = if buffer.bytes_per_element == view.bytes_per_element {
            1
        } else if buffer.bytes_per_element == 1 {
            view.bytes_per_element
        } else {
            return None;
        };
        let start = view.offset.checked_mul(scale)?;
        match view.fixed_length {
            Some(length) => {
                let scaled_length = length.checked_mul(scale)?;
                if start + scaled_length > buffer.values.len() {
                    None
                } else {
                    Some(length)
                }
            }
            None => {
                if start > buffer.values.len() {
                    None
                } else {
                    Some(buffer.values.len().saturating_sub(start) / scale)
                }
            }
        }
    }

    pub(in crate::backend::direct_wasm) fn typed_array_view_static_values(
        &self,
        view: &TypedArrayViewBinding,
    ) -> Option<ArrayValueBinding> {
        let buffer = self
            .state
            .speculation
            .static_semantics
            .local_resizable_array_buffer_binding(&view.buffer_name)?;
        let length = self.typed_array_view_static_length(view)?;
        let scale = if buffer.bytes_per_element == view.bytes_per_element {
            1
        } else if buffer.bytes_per_element == 1 {
            view.bytes_per_element
        } else {
            return None;
        };
        let start = view.offset.checked_mul(scale)?;
        let values = if scale == 1 {
            buffer.values[start..start + length].to_vec()
        } else {
            (0..length)
                .map(|index| {
                    buffer
                        .values
                        .get(start + index * scale)
                        .cloned()
                        .unwrap_or(Some(Expression::Number(0.0)))
                })
                .collect()
        };
        Some(ArrayValueBinding { values })
    }
}
