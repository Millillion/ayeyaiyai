use super::*;

impl<'a> FunctionCompiler<'a> {
    pub(in crate::backend::direct_wasm) fn emit_literal_expression(
        &mut self,
        expression: &Expression,
    ) -> DirectResult<()> {
        match expression {
            Expression::Number(value) => {
                if value.is_nan() {
                    self.push_i32_const(JS_NAN_TAG);
                } else {
                    self.push_i32_const(f64_to_i32(*value)?);
                }
                Ok(())
            }
            Expression::BigInt(value) => {
                self.push_i32_const(parse_bigint_to_i32(value)?);
                Ok(())
            }
            Expression::String(text) => {
                match parse_string_to_i32(text) {
                    Ok(parsed) => self.push_i32_const(parsed),
                    Err(Unsupported("string literal collides with reserved JS tag")) => {
                        return Err(Unsupported("string literal collides with reserved JS tag"));
                    }
                    Err(_) => {
                        self.emit_static_string_literal(text)?;
                    }
                }
                Ok(())
            }
            Expression::Null => {
                self.push_i32_const(JS_NULL_TAG);
                Ok(())
            }
            Expression::Undefined => {
                self.push_i32_const(JS_UNDEFINED_TAG);
                Ok(())
            }
            Expression::Bool(value) => {
                self.push_i32_const(if *value { 1 } else { 0 });
                Ok(())
            }
            Expression::Array(elements) => self.emit_array_literal_expression(elements),
            Expression::Object(entries) => self.emit_object_literal_expression(entries),
            _ => unreachable!("literal expression expected"),
        }
    }

    fn emit_array_spread_expression(&mut self, expression: &Expression) -> DirectResult<bool> {
        let Some((_, steps, completion_effects, _)) =
            self.simple_generator_source_metadata(expression)
        else {
            return Ok(false);
        };

        for step in steps {
            for effect in step.effects {
                self.emit_statement(&effect)?;
            }
            match step.outcome {
                SimpleGeneratorStepOutcome::Yield(value) => {
                    self.emit_numeric_expression(&value)?;
                    self.state.emission.output.instructions.push(0x1a);
                }
                SimpleGeneratorStepOutcome::YieldResult(result) => {
                    let value =
                        self.simple_generator_yield_result_value(&result, &Expression::Undefined);
                    self.emit_numeric_expression(&value)?;
                    self.state.emission.output.instructions.push(0x1a);
                }
                SimpleGeneratorStepOutcome::Throw(value) => {
                    self.emit_static_throw_value(&StaticThrowValue::Value(value))?;
                    return Ok(true);
                }
            }
        }
        for effect in completion_effects {
            self.emit_statement(&effect)?;
        }
        Ok(true)
    }

    fn emit_static_array_spread_iterator_throw(
        &mut self,
        expression: &Expression,
    ) -> DirectResult<bool> {
        let trace_array_spread = std::env::var_os("AYY_TRACE_ARRAY_SPREAD").is_some();
        let Some(iterator_target) = self.resolve_static_get_iterator_value(expression, &[]) else {
            if trace_array_spread {
                eprintln!("array_spread_static: no iterator target for {expression:?}");
            }
            return Ok(false);
        };
        if trace_array_spread {
            eprintln!(
                "array_spread_static: iterator_target={iterator_target:?} expression={expression:?}"
            );
        }
        if matches!(
            self.infer_value_kind(&iterator_target),
            Some(
                StaticValueKind::Undefined
                    | StaticValueKind::Null
                    | StaticValueKind::Bool
                    | StaticValueKind::Number
                    | StaticValueKind::String
                    | StaticValueKind::BigInt
                    | StaticValueKind::Symbol
            )
        ) {
            self.emit_named_error_throw("TypeError")?;
            return Ok(true);
        }
        let current_function_name = self.current_function_name().map(str::to_string);
        let next_property = Expression::String("next".to_string());
        let mut next_target = iterator_target.clone();
        let mut next_binding = self.resolve_member_function_binding(&next_target, &next_property);
        if next_binding.is_none() {
            let iterator_property =
                self.materialize_static_expression(&symbol_iterator_expression());
            if let Some(iterator_binding) =
                self.resolve_member_function_binding(&iterator_target, &iterator_property)
            {
                let Some(iterator_outcome) = self
                    .resolve_static_function_outcome_from_binding_with_context(
                        &iterator_binding,
                        &[],
                        current_function_name.as_deref(),
                    )
                else {
                    return Ok(false);
                };
                match iterator_outcome {
                    StaticEvalOutcome::Throw(throw_value) => {
                        self.emit_static_throw_value(&throw_value)?;
                        return Ok(true);
                    }
                    StaticEvalOutcome::Value(value) => {
                        next_target = self.materialize_static_expression(&value);
                        next_binding =
                            self.resolve_member_function_binding(&next_target, &next_property);
                    }
                }
            }
        }
        let Some(next_binding) = next_binding else {
            if trace_array_spread {
                eprintln!("array_spread_static: no next binding target={next_target:?}");
            }
            return Ok(false);
        };
        let Some(next_outcome) = self.resolve_static_function_outcome_from_binding_with_context(
            &next_binding,
            &[],
            current_function_name.as_deref(),
        ) else {
            if trace_array_spread {
                eprintln!("array_spread_static: no next outcome binding={next_binding:?}");
            }
            return Ok(false);
        };
        let step_result = match next_outcome {
            StaticEvalOutcome::Throw(throw_value) => {
                if trace_array_spread {
                    eprintln!("array_spread_static: next throws");
                }
                self.emit_static_throw_value(&throw_value)?;
                return Ok(true);
            }
            StaticEvalOutcome::Value(value) => self.materialize_static_expression(&value),
        };
        if trace_array_spread {
            eprintln!("array_spread_static: step_result={step_result:?}");
        }
        let Some(step_binding) = self.resolve_object_binding_from_expression(&step_result) else {
            if trace_array_spread {
                eprintln!("array_spread_static: no step binding, emitting member read");
            }
            let value_read = Expression::Member {
                object: Box::new(step_result),
                property: Box::new(Expression::String("value".to_string())),
            };
            self.emit_numeric_expression(&value_read)?;
            self.state.emission.output.instructions.push(0x1a);
            return Ok(true);
        };

        let done_property = Expression::String("done".to_string());
        if let Some(getter_binding) =
            self.resolve_member_getter_binding(&step_result, &done_property)
        {
            if trace_array_spread {
                eprintln!("array_spread_static: done getter binding={getter_binding:?}");
            }
            let Some(done_outcome) = self
                .resolve_static_function_outcome_from_binding_with_context(
                    &getter_binding,
                    &[],
                    current_function_name.as_deref(),
                )
            else {
                return Ok(false);
            };
            match done_outcome {
                StaticEvalOutcome::Throw(throw_value) => {
                    self.emit_static_throw_value(&throw_value)?;
                    return Ok(true);
                }
                StaticEvalOutcome::Value(value)
                    if self.resolve_static_boolean_expression(&value) == Some(true) =>
                {
                    return Ok(false);
                }
                StaticEvalOutcome::Value(_) => {}
            }
        }
        if let Some(descriptor) = object_binding_lookup_descriptor(&step_binding, &done_property)
            && let Some(getter) = &descriptor.getter
        {
            let Some(getter_binding) = self.resolve_function_binding_from_expression(getter) else {
                return Ok(false);
            };
            let Some(done_outcome) = self
                .resolve_static_function_outcome_from_binding_with_context(
                    &getter_binding,
                    &[],
                    current_function_name.as_deref(),
                )
            else {
                return Ok(false);
            };
            match done_outcome {
                StaticEvalOutcome::Throw(throw_value) => {
                    self.emit_static_throw_value(&throw_value)?;
                    return Ok(true);
                }
                StaticEvalOutcome::Value(value)
                    if self.resolve_static_boolean_expression(&value) == Some(true) =>
                {
                    return Ok(false);
                }
                StaticEvalOutcome::Value(_) => {}
            }
        }
        let done_value = object_binding_lookup_value(&step_binding, &done_property)
            .cloned()
            .unwrap_or(Expression::Bool(false));
        if self.resolve_static_boolean_expression(&done_value) == Some(true) {
            return Ok(false);
        }

        let value_property = Expression::String("value".to_string());
        if let Some(getter_binding) =
            self.resolve_member_getter_binding(&step_result, &value_property)
        {
            if trace_array_spread {
                eprintln!("array_spread_static: value getter binding={getter_binding:?}");
            }
            let Some(value_outcome) = self
                .resolve_static_function_outcome_from_binding_with_context(
                    &getter_binding,
                    &[],
                    current_function_name.as_deref(),
                )
            else {
                return Ok(false);
            };
            if let StaticEvalOutcome::Throw(throw_value) = value_outcome {
                self.emit_static_throw_value(&throw_value)?;
                return Ok(true);
            }
        }
        if let Some(descriptor) = object_binding_lookup_descriptor(&step_binding, &value_property)
            && let Some(getter) = &descriptor.getter
        {
            if trace_array_spread {
                eprintln!("array_spread_static: value descriptor getter={getter:?}");
            }
            let Some(getter_binding) = self.resolve_function_binding_from_expression(getter) else {
                return Ok(false);
            };
            let Some(value_outcome) = self
                .resolve_static_function_outcome_from_binding_with_context(
                    &getter_binding,
                    &[],
                    current_function_name.as_deref(),
                )
            else {
                return Ok(false);
            };
            if let StaticEvalOutcome::Throw(throw_value) = value_outcome {
                self.emit_static_throw_value(&throw_value)?;
                return Ok(true);
            }
        }

        if trace_array_spread {
            eprintln!("array_spread_static: emitting fallback value read");
        }
        let value_read = Expression::Member {
            object: Box::new(step_result),
            property: Box::new(value_property),
        };
        self.emit_numeric_expression(&value_read)?;
        self.state.emission.output.instructions.push(0x1a);
        Ok(true)
    }

    fn emit_array_literal_expression(
        &mut self,
        elements: &[crate::ir::hir::ArrayElement],
    ) -> DirectResult<()> {
        for element in elements {
            match element {
                crate::ir::hir::ArrayElement::Expression(expression) => {
                    self.emit_numeric_expression(expression)?;
                    self.state.emission.output.instructions.push(0x1a);
                }
                crate::ir::hir::ArrayElement::Spread(expression) => {
                    if !self.emit_array_spread_expression(expression)?
                        && !self.emit_static_array_spread_iterator_throw(expression)?
                    {
                        self.emit_numeric_expression(&Expression::GetIterator(Box::new(
                            expression.clone(),
                        )))?;
                        self.state.emission.output.instructions.push(0x1a);
                    }
                }
            }
        }
        self.push_i32_const(JS_TYPEOF_OBJECT_TAG);
        Ok(())
    }

    fn emit_object_literal_expression(
        &mut self,
        entries: &[crate::ir::hir::ObjectEntry],
    ) -> DirectResult<()> {
        for entry in entries {
            match entry {
                crate::ir::hir::ObjectEntry::Data { key, value } => {
                    self.emit_property_key_expression_effects(key)?;
                    self.emit_numeric_expression(value)?;
                    self.state.emission.output.instructions.push(0x1a);
                }
                crate::ir::hir::ObjectEntry::Getter { key, getter } => {
                    self.emit_property_key_expression_effects(key)?;
                    self.emit_numeric_expression(getter)?;
                    self.state.emission.output.instructions.push(0x1a);
                }
                crate::ir::hir::ObjectEntry::Setter { key, setter } => {
                    self.emit_property_key_expression_effects(key)?;
                    self.emit_numeric_expression(setter)?;
                    self.state.emission.output.instructions.push(0x1a);
                }
                crate::ir::hir::ObjectEntry::Spread(expression) => {
                    self.emit_numeric_expression(expression)?;
                    self.state.emission.output.instructions.push(0x1a);
                    self.emit_object_spread_copy_data_properties_effects(expression)?;
                }
            }
        }
        self.push_i32_const(JS_TYPEOF_OBJECT_TAG);
        Ok(())
    }
}
