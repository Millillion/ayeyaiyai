use super::*;

fn class_initializer_sets_static_name(function: &FunctionDeclaration, expected_name: &str) -> bool {
    function.name.starts_with("__ayy_class_init_")
        && function
            .body
            .iter()
            .any(|statement| statement_defines_static_name_property(statement, expected_name))
}

fn statement_defines_static_name_property(statement: &Statement, expected_name: &str) -> bool {
    let Statement::Expression(Expression::Call { callee, arguments }) = statement else {
        return false;
    };
    let Expression::Member { object, property } = callee.as_ref() else {
        return false;
    };
    if !matches!(object.as_ref(), Expression::Identifier(name) if name == "Object")
        || !matches!(property.as_ref(), Expression::String(name) if name == "defineProperty")
    {
        return false;
    }
    let [
        CallArgument::Expression(_target),
        CallArgument::Expression(Expression::String(property_name)),
        CallArgument::Expression(descriptor_expression),
    ] = arguments.as_slice()
    else {
        return false;
    };
    property_name == "name"
        && resolve_property_descriptor_definition(descriptor_expression)
            .and_then(|descriptor| descriptor.value)
            .is_some_and(|value| {
                static_expression_matches(&value, &Expression::String(expected_name.to_string()))
            })
}

impl<'a> FunctionCompiler<'a> {
    fn runtime_shadow_member_matches_expected_value(
        &self,
        actual: &Expression,
        expected: &Expression,
    ) -> bool {
        let Expression::Member { object, property } = actual else {
            return false;
        };
        let Some(shadow_binding_name) =
            self.runtime_object_property_shadow_binding_name_for_expression(object, property)
        else {
            return false;
        };
        self.global_value_binding(&shadow_binding_name)
            .or_else(|| {
                self.backend
                    .shared_global_semantics
                    .values
                    .value_bindings
                    .get(&shadow_binding_name)
            })
            .is_some_and(|value| {
                static_expression_matches(value, expected)
                    || static_expression_matches(
                        &self.materialize_static_expression(value),
                        expected,
                    )
            })
    }

    fn descriptor_value_matches_expected(
        &self,
        actual: &Expression,
        expected: Option<&Expression>,
    ) -> bool {
        let Some(expected) = expected else {
            return true;
        };
        static_expression_matches(actual, expected)
            || static_expression_matches(&self.materialize_static_expression(actual), expected)
            || self
                .resolve_static_reference_identity_key(actual)
                .zip(self.resolve_static_reference_identity_key(expected))
                .is_some_and(|(actual_key, expected_key)| actual_key == expected_key)
            || self.runtime_shadow_member_matches_expected_value(actual, expected)
    }

    fn template_object_verify_target<'b>(
        object_expression: &'b Expression,
    ) -> Option<(&'b Expression, bool)> {
        if let Expression::Member { object, property } = object_expression
            && matches!(property.as_ref(), Expression::String(name) if name == "raw")
        {
            return Some((object.as_ref(), true));
        }
        Some((object_expression, false))
    }

    fn template_object_property_descriptor_matches(
        &self,
        raw_array: bool,
        binding: &ArrayValueBinding,
        property_name: &str,
        descriptor: &PropertyDescriptorDefinition,
        expected_value: Option<&Expression>,
    ) -> bool {
        if descriptor.is_accessor() {
            return false;
        }

        let matches_expected_value = |actual: &Expression| {
            expected_value.is_none_or(|expected| {
                self.descriptor_value_matches_expected(actual, Some(expected))
            })
        };
        let matches_bool =
            |actual: bool, expected: Option<bool>| expected.is_none_or(|value| value == actual);

        if !raw_array && property_name == "raw" {
            return expected_value.is_none()
                && matches_bool(false, descriptor.writable)
                && matches_bool(false, descriptor.enumerable)
                && matches_bool(false, descriptor.configurable);
        }

        if property_name == "length" {
            return matches_expected_value(&Expression::Number(binding.values.len() as f64))
                && matches_bool(false, descriptor.writable)
                && matches_bool(false, descriptor.enumerable)
                && matches_bool(false, descriptor.configurable);
        }

        let Some(index) = canonical_array_index_from_property_name(property_name) else {
            return false;
        };
        let Some(value) = binding.values.get(index as usize) else {
            return false;
        };
        let actual = value.as_ref().unwrap_or(&Expression::Undefined);
        matches_expected_value(actual)
            && matches_bool(false, descriptor.writable)
            && matches_bool(true, descriptor.enumerable)
            && matches_bool(false, descriptor.configurable)
    }

    fn emit_template_object_verify_property_call(
        &mut self,
        object_expression: &Expression,
        property_name: &str,
        property_expression: &Expression,
        descriptor: &PropertyDescriptorDefinition,
        expected_value: Option<&Expression>,
        arguments: &[CallArgument],
    ) -> DirectResult<bool> {
        let Some((comparison_expression, raw_array)) =
            Self::template_object_verify_target(object_expression)
        else {
            return Ok(false);
        };
        if !inline_summary_side_effect_free_expression(comparison_expression)
            || !inline_summary_side_effect_free_expression(property_expression)
        {
            return Ok(false);
        }

        let mut templates = if raw_array {
            self.backend
                .template_object_raw_array_bindings
                .iter()
                .map(|(runtime_value, binding)| (*runtime_value, binding.clone()))
                .collect::<Vec<_>>()
        } else {
            self.backend
                .template_object_array_bindings
                .iter()
                .map(|(runtime_value, binding)| (*runtime_value, binding.clone()))
                .collect::<Vec<_>>()
        };
        if templates.is_empty() {
            return Ok(false);
        }
        templates.sort_by_key(|(runtime_value, _)| *runtime_value);
        if !templates.iter().any(|(_, binding)| {
            self.template_object_property_descriptor_matches(
                raw_array,
                binding,
                property_name,
                descriptor,
                expected_value,
            )
        }) {
            return Ok(false);
        }

        let comparison_local = self.allocate_temp_local();
        self.emit_numeric_expression(comparison_expression)?;
        self.push_local_set(comparison_local);

        fn emit_branch<'a>(
            compiler: &mut FunctionCompiler<'a>,
            comparison_local: u32,
            raw_array: bool,
            property_name: &str,
            descriptor: &PropertyDescriptorDefinition,
            expected_value: Option<&Expression>,
            templates: &[(i32, ArrayValueBinding)],
            index: usize,
        ) -> DirectResult<()> {
            let Some((runtime_value, binding)) = templates.get(index) else {
                return compiler.emit_error_throw();
            };

            compiler.push_local_get(comparison_local);
            compiler.push_i32_const(*runtime_value);
            compiler.push_binary_op(BinaryOp::Equal)?;
            compiler.state.emission.output.instructions.push(0x04);
            compiler.state.emission.output.instructions.push(I32_TYPE);
            compiler.push_control_frame();
            if compiler.template_object_property_descriptor_matches(
                raw_array,
                binding,
                property_name,
                descriptor,
                expected_value,
            ) {
                compiler.push_i32_const(JS_UNDEFINED_TAG);
            } else {
                compiler.emit_error_throw()?;
            }
            compiler.state.emission.output.instructions.push(0x05);
            emit_branch(
                compiler,
                comparison_local,
                raw_array,
                property_name,
                descriptor,
                expected_value,
                templates,
                index.saturating_add(1),
            )?;
            compiler.state.emission.output.instructions.push(0x0b);
            compiler.pop_control_frame();
            Ok(())
        }

        emit_branch(
            self,
            comparison_local,
            raw_array,
            property_name,
            descriptor,
            expected_value,
            &templates,
            0,
        )?;

        for argument in arguments.iter().skip(3) {
            match argument {
                CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                    self.emit_numeric_expression(expression)?;
                    self.state.emission.output.instructions.push(0x1a);
                }
            }
        }
        Ok(true)
    }

    pub(in crate::backend::direct_wasm) fn emit_verify_property_call(
        &mut self,
        arguments: &[CallArgument],
    ) -> DirectResult<bool> {
        let [object_argument, property_argument, descriptor_argument, ..] = arguments else {
            return Ok(false);
        };
        let (
            CallArgument::Expression(object_expression),
            CallArgument::Expression(property_expression),
            CallArgument::Expression(descriptor_expression),
        ) = (object_argument, property_argument, descriptor_argument)
        else {
            return Ok(false);
        };

        let Some(descriptor) = resolve_property_descriptor_definition(descriptor_expression) else {
            return Ok(false);
        };
        let trace_verify_property = std::env::var_os("AYY_TRACE_VERIFY_PROPERTY").is_some();
        let expected_value = descriptor.value.as_ref().map(|value| {
            let materialized = self.materialize_static_expression(value);
            match materialized {
                Expression::Identifier(name)
                    if name == "undefined" && self.is_unshadowed_builtin_identifier(&name) =>
                {
                    Expression::Undefined
                }
                _ => materialized,
            }
        });
        let expected_writable = descriptor.writable;
        let expected_enumerable = descriptor.enumerable;
        let expected_configurable = descriptor.configurable;
        let matches_bool =
            |actual: bool, expected: Option<bool>| expected.is_none_or(|value| value == actual);
        let matches_missing_bool = |expected: Option<bool>| expected.is_none();

        let direct_arguments = self.is_direct_arguments_object(object_expression);
        let arguments_binding = self.resolve_arguments_binding_from_expression(object_expression);
        let object_binding = self
            .resolve_object_binding_from_expression(object_expression)
            .or_else(|| match object_expression {
                Expression::Identifier(name) => self
                    .resolve_identifier_object_binding_fallback(name)
                    .or_else(|| self.resolve_runtime_shadow_object_binding(name)),
                Expression::This => self.resolve_runtime_shadow_object_binding("this"),
                _ => None,
            });
        let resolved_property = self
            .resolve_property_key_expression(property_expression)
            .unwrap_or_else(|| self.materialize_static_expression(property_expression));
        let symbol_property = [&resolved_property, property_expression]
            .into_iter()
            .find_map(|candidate| {
                if self.well_known_symbol_name(candidate).is_some() {
                    Some(candidate.clone())
                } else {
                    self.resolve_symbol_identity_expression(candidate)
                }
            });

        if direct_arguments
            && symbol_property
                .as_ref()
                .is_some_and(is_symbol_iterator_expression)
        {
            if expected_value
                .as_ref()
                .is_some_and(|value| *value == arguments_symbol_iterator_expression())
                && matches_bool(true, expected_writable)
                && matches_bool(false, expected_enumerable)
                && matches_bool(true, expected_configurable)
            {
                for argument in arguments.iter().skip(3) {
                    match argument {
                        CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                            self.emit_numeric_expression(expression)?;
                            self.state.emission.output.instructions.push(0x1a);
                        }
                    }
                }
                self.push_i32_const(JS_UNDEFINED_TAG);
                return Ok(true);
            }
            return Ok(false);
        }

        let property_name = static_property_name_from_expression(&resolved_property);
        if object_binding.is_none()
            && let Some(property_name) = property_name.as_deref()
            && self.emit_template_object_verify_property_call(
                object_expression,
                property_name,
                property_expression,
                &descriptor,
                expected_value.as_ref(),
                arguments,
            )?
        {
            return Ok(true);
        }
        let matches_value = |actual: &Expression| {
            expected_value.as_ref().is_none_or(|expected| {
                self.descriptor_value_matches_expected(actual, Some(expected))
            })
        };
        let global_property_descriptor =
            (self.state.speculation.execution_context.top_level_function
                && matches!(object_expression, Expression::This))
            .then(|| {
                property_name.as_ref().and_then(|property_name| {
                    self.backend
                        .global_property_descriptor(property_name)
                        .cloned()
                })
            })
            .flatten();

        let function_metadata_property_matches_expected =
            property_name.as_ref().is_some_and(|property_name| {
                if !matches!(property_name.as_str(), "name" | "length") {
                    return false;
                }
                let Some(expected) = expected_value.as_ref() else {
                    return false;
                };
                let property_expression = Expression::String(property_name.clone());
                let resolved_value_matches = match property_name.as_str() {
                    "name" => self
                        .resolve_function_name_value(object_expression, &property_expression)
                        .is_some_and(|name| {
                            static_expression_matches(&Expression::String(name), expected)
                        }),
                    "length" => self
                        .resolve_user_function_length(object_expression, &property_expression)
                        .is_some_and(|length| {
                            static_expression_matches(&Expression::Number(length as f64), expected)
                        }),
                    _ => false,
                };
                if resolved_value_matches {
                    return true;
                }
                let Expression::Identifier(object_name) = object_expression else {
                    return false;
                };
                let source_name = scoped_binding_source_name(object_name).unwrap_or(object_name);
                match (property_name.as_str(), expected) {
                    ("name", Expression::String(expected_name)) if expected_name == source_name => {
                        self.user_functions()
                            .into_iter()
                            .filter(|function| {
                                self.resolve_user_function_display_name(&function.name)
                                    .as_deref()
                                    == Some(source_name)
                            })
                            .count()
                            == 1
                            || self
                                .user_functions()
                                .into_iter()
                                .filter_map(|function| {
                                    self.resolve_registered_function_declaration(&function.name)
                                })
                                .filter(|function| {
                                    class_initializer_sets_static_name(function, expected_name)
                                })
                                .count()
                                == 1
                    }
                    ("length", Expression::Number(expected_length)) => self
                        .user_functions()
                        .into_iter()
                        .find(|function| {
                            self.resolve_user_function_display_name(&function.name)
                                .as_deref()
                                == Some(source_name)
                        })
                        .is_some_and(|function| function.length as f64 == *expected_length),
                    _ => false,
                }
            });

        if let Some(property_name) = property_name.as_ref()
            && matches!(property_name.as_str(), "name" | "length")
            && expected_value.is_some()
            && expected_writable == Some(false)
            && expected_enumerable == Some(false)
            && expected_configurable == Some(true)
            && (self
                .resolve_function_binding_from_expression(object_expression)
                .is_some()
                || self.infer_value_kind(object_expression) == Some(StaticValueKind::Function)
                || function_metadata_property_matches_expected)
        {
            let actual_property = Expression::Member {
                object: Box::new(object_expression.clone()),
                property: Box::new(Expression::String(property_name.clone())),
            };
            let actual_local = self.allocate_temp_local();
            let expected_local = self.allocate_temp_local();
            self.emit_numeric_expression(&actual_property)?;
            self.push_local_set(actual_local);
            self.emit_numeric_expression(expected_value.as_ref().expect("checked above"))?;
            self.push_local_set(expected_local);
            self.push_local_get(actual_local);
            self.push_local_get(expected_local);
            self.push_binary_op(BinaryOp::NotEqual)?;
            self.state.emission.output.instructions.push(0x04);
            self.state
                .emission
                .output
                .instructions
                .push(EMPTY_BLOCK_TYPE);
            self.push_control_frame();
            self.emit_error_throw()?;
            self.state.emission.output.instructions.push(0x0b);
            self.pop_control_frame();
            for argument in arguments.iter().skip(3) {
                match argument {
                    CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                        self.emit_numeric_expression(expression)?;
                        self.state.emission.output.instructions.push(0x1a);
                    }
                }
            }
            self.push_i32_const(JS_UNDEFINED_TAG);
            return Ok(true);
        }
        if direct_arguments
            && let Some(index) = property_name
                .as_ref()
                .and_then(|property_name| canonical_array_index_from_property_name(property_name))
        {
            let Some(slot) = self.state.parameters.arguments_slots.get(&index).cloned() else {
                return Ok(false);
            };
            let matches_descriptor = slot.state.present
                && matches_bool(slot.state.enumerable, expected_enumerable)
                && matches_bool(slot.state.configurable, expected_configurable)
                && if slot.state.is_accessor() {
                    matches_missing_bool(expected_writable) && expected_value.is_none()
                } else {
                    matches_bool(slot.state.writable, expected_writable)
                };
            if !matches_descriptor {
                return Ok(false);
            }
            if let Some(expected_value) = expected_value.as_ref() {
                let actual_local = self.allocate_temp_local();
                let expected_local = self.allocate_temp_local();
                self.emit_arguments_slot_read(index)?;
                self.push_local_set(actual_local);
                self.emit_numeric_expression(expected_value)?;
                self.push_local_set(expected_local);
                self.push_local_get(actual_local);
                self.push_local_get(expected_local);
                self.push_binary_op(BinaryOp::NotEqual)?;
                self.state.emission.output.instructions.push(0x04);
                self.state
                    .emission
                    .output
                    .instructions
                    .push(EMPTY_BLOCK_TYPE);
                self.push_control_frame();
                self.emit_error_throw()?;
                self.state.emission.output.instructions.push(0x0b);
                self.pop_control_frame();
            }
            for argument in arguments.iter().skip(3) {
                match argument {
                    CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                        self.emit_numeric_expression(expression)?;
                        self.state.emission.output.instructions.push(0x1a);
                    }
                }
            }
            self.push_i32_const(JS_UNDEFINED_TAG);
            return Ok(true);
        }

        let matches_property = if let Some(symbol_property) = symbol_property.as_ref() {
            object_binding
                .as_ref()
                .and_then(|object_binding| {
                    self.resolve_object_binding_property_value_for_object(
                        object_expression,
                        object_binding,
                        symbol_property,
                    )
                })
                .is_some_and(|actual| matches_value(&actual))
                && matches_bool(true, expected_writable)
                && matches_bool(true, expected_enumerable)
                && matches_bool(true, expected_configurable)
        } else if property_name.as_deref() == Some("length") {
            if direct_arguments {
                self.state
                    .speculation
                    .execution_context
                    .current_arguments_length_present
                    && self
                        .state
                        .speculation
                        .execution_context
                        .current_arguments_length_override
                        .as_ref()
                        .is_none_or(matches_value)
                    && matches_bool(true, expected_writable)
                    && matches_bool(false, expected_enumerable)
                    && matches_bool(true, expected_configurable)
            } else if let Some(arguments_binding) = arguments_binding.as_ref() {
                arguments_binding.length_present
                    && matches_value(&arguments_binding.length_value)
                    && matches_bool(true, expected_writable)
                    && matches_bool(false, expected_enumerable)
                    && matches_bool(true, expected_configurable)
            } else if let Some(function_length) =
                self.resolve_user_function_length(object_expression, &resolved_property)
            {
                matches_value(&Expression::Number(function_length as f64))
                    && matches_bool(false, expected_writable)
                    && matches_bool(false, expected_enumerable)
                    && matches_bool(true, expected_configurable)
            } else if let Some(object_descriptor) = self.resolve_object_property_descriptor_binding(
                object_expression,
                self.resolve_bound_alias_expression(object_expression)
                    .filter(|resolved| !static_expression_matches(resolved, object_expression))
                    .as_ref(),
                &self.materialize_static_expression(object_expression),
                &Expression::String("length".to_string()),
                Some("length"),
            ) {
                object_descriptor.value.as_ref().is_none_or(|value| {
                    self.descriptor_value_matches_expected(value, expected_value.as_ref())
                }) && match object_descriptor.writable {
                    Some(writable) => matches_bool(writable, expected_writable),
                    None => matches_missing_bool(expected_writable),
                } && matches_bool(object_descriptor.enumerable, expected_enumerable)
                    && matches_bool(object_descriptor.configurable, expected_configurable)
            } else {
                false
            }
        } else if property_name.as_deref() == Some("callee") {
            let strict = if direct_arguments {
                Some(self.state.speculation.execution_context.strict_mode)
            } else {
                arguments_binding.as_ref().map(|binding| binding.strict)
            };
            if let Some(strict) = strict {
                if strict {
                    expected_value.is_none()
                        && matches_missing_bool(expected_writable)
                        && matches_bool(false, expected_enumerable)
                        && matches_bool(false, expected_configurable)
                } else {
                    let actual_value = if direct_arguments {
                        self.direct_arguments_callee_expression()
                    } else {
                        arguments_binding
                            .as_ref()
                            .and_then(|binding| binding.callee_value.clone())
                    };
                    let present = if direct_arguments {
                        self.state
                            .speculation
                            .execution_context
                            .current_arguments_callee_present
                    } else {
                        arguments_binding
                            .as_ref()
                            .is_some_and(|binding| binding.callee_present)
                    };
                    present
                        && actual_value.as_ref().is_none_or(matches_value)
                        && matches_bool(true, expected_writable)
                        && matches_bool(false, expected_enumerable)
                        && matches_bool(true, expected_configurable)
                }
            } else {
                false
            }
        } else if let Some(property_name) = property_name.as_ref() {
            if let Some(arguments_binding) = arguments_binding.as_ref() {
                if let Ok(index) = property_name.parse::<usize>() {
                    arguments_binding
                        .values
                        .get(index)
                        .is_some_and(matches_value)
                        && matches_bool(true, expected_writable)
                        && matches_bool(true, expected_enumerable)
                        && matches_bool(true, expected_configurable)
                } else {
                    false
                }
            } else if let Some(global_property_descriptor) = global_property_descriptor.as_ref() {
                matches_value(&global_property_descriptor.value)
                    && match global_property_descriptor.writable {
                        Some(writable) => matches_bool(writable, expected_writable),
                        None => matches_missing_bool(expected_writable),
                    }
                    && matches_bool(global_property_descriptor.enumerable, expected_enumerable)
                    && matches_bool(
                        global_property_descriptor.configurable,
                        expected_configurable,
                    )
            } else if let Some(object_descriptor) = self.resolve_object_property_descriptor_binding(
                object_expression,
                self.resolve_bound_alias_expression(object_expression)
                    .filter(|resolved| !static_expression_matches(resolved, object_expression))
                    .as_ref(),
                &self.materialize_static_expression(object_expression),
                &Expression::String(property_name.clone()),
                Some(property_name),
            ) {
                object_descriptor.value.as_ref().is_none_or(|value| {
                    self.descriptor_value_matches_expected(value, expected_value.as_ref())
                }) && match object_descriptor.writable {
                    Some(writable) => matches_bool(writable, expected_writable),
                    None => matches_missing_bool(expected_writable),
                } && matches_bool(object_descriptor.enumerable, expected_enumerable)
                    && matches_bool(object_descriptor.configurable, expected_configurable)
            } else if let Some(function_descriptor) = self
                .resolve_function_property_descriptor_binding(
                    object_expression,
                    self.resolve_bound_alias_expression(object_expression)
                        .filter(|resolved| !static_expression_matches(resolved, object_expression))
                        .as_ref(),
                    &self.materialize_static_expression(object_expression),
                    property_name,
                )
            {
                let property = Expression::String(property_name.clone());
                let tracked_value_matches_descriptor = object_binding
                    .as_ref()
                    .and_then(|object_binding| {
                        object_binding_lookup_value(object_binding, &property)
                    })
                    .is_none_or(|tracked_value| {
                        function_descriptor
                            .value
                            .as_ref()
                            .is_none_or(|descriptor_value| tracked_value == descriptor_value)
                    });
                tracked_value_matches_descriptor
                    && function_descriptor.value.as_ref().is_none_or(|value| {
                        self.descriptor_value_matches_expected(value, expected_value.as_ref())
                    })
                    && match function_descriptor.writable {
                        Some(writable) => matches_bool(writable, expected_writable),
                        None => matches_missing_bool(expected_writable),
                    }
                    && matches_bool(function_descriptor.enumerable, expected_enumerable)
                    && matches_bool(function_descriptor.configurable, expected_configurable)
            } else if let Some(object_binding) = object_binding.as_ref() {
                let property = Expression::String(property_name.clone());
                object_binding_lookup_value(object_binding, &property).is_some_and(matches_value)
                    && matches_bool(true, expected_writable)
                    && matches_bool(
                        !object_binding
                            .non_enumerable_string_properties
                            .iter()
                            .any(|name| name == property_name),
                        expected_enumerable,
                    )
                    && matches_bool(true, expected_configurable)
            } else if matches!(
                object_expression,
                Expression::Member { property, .. }
                    if matches!(property.as_ref(), Expression::String(name) if name == "prototype")
            ) {
                let member_expression = Expression::Member {
                    object: Box::new(object_expression.clone()),
                    property: Box::new(Expression::String(property_name.clone())),
                };
                let member_function_binding =
                    self.resolve_function_binding_from_expression(&member_expression);
                member_function_binding.is_some()
                    && expected_value.as_ref().is_none_or(|expected| {
                        self.resolve_function_binding_from_expression(expected)
                            == member_function_binding
                    })
                    && matches_bool(true, expected_writable)
                    && matches_bool(false, expected_enumerable)
                    && matches_bool(true, expected_configurable)
            } else {
                false
            }
        } else {
            false
        };

        if trace_verify_property {
            let object_binding_summary = object_binding.as_ref().map(|binding| {
                let descriptors = binding
                    .property_descriptors
                    .iter()
                    .map(|(property, descriptor)| {
                        (
                            property.clone(),
                            descriptor.value.clone(),
                            descriptor.writable,
                            descriptor.enumerable,
                            descriptor.configurable,
                            descriptor.getter.clone(),
                            descriptor.setter.clone(),
                            descriptor.has_get,
                            descriptor.has_set,
                        )
                    })
                    .collect::<Vec<_>>();
                (
                    ordered_object_property_names(binding),
                    binding.string_properties.clone(),
                    binding.symbol_properties.clone(),
                    binding.non_enumerable_string_properties.clone(),
                    descriptors,
                )
            });
            eprintln!(
                "verify_property: object={object_expression:?} property={property_expression:?} resolved_property={resolved_property:?} symbol_property={symbol_property:?} expected_value={expected_value:?} expected_writable={expected_writable:?} expected_enumerable={expected_enumerable:?} expected_configurable={expected_configurable:?} object_binding={object_binding_summary:?} matches={matches_property}"
            );
        }

        if !matches_property {
            return Ok(false);
        }

        for argument in arguments.iter().skip(3) {
            match argument {
                CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                    self.emit_numeric_expression(expression)?;
                    self.state.emission.output.instructions.push(0x1a);
                }
            }
        }

        self.push_i32_const(JS_UNDEFINED_TAG);
        Ok(true)
    }
}
