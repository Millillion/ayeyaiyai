use super::*;

impl<'a> FunctionCompiler<'a> {
    fn user_function_runtime_value_from_expression(&self, expression: &Expression) -> Option<i32> {
        let LocalFunctionBinding::User(function_name) =
            self.resolve_function_binding_from_expression(expression)?
        else {
            return None;
        };
        self.user_function(&function_name)
            .map(user_function_runtime_value)
    }

    fn expression_is_static_constructible_reflect_new_target(
        &self,
        expression: &Expression,
    ) -> bool {
        self.resolve_function_binding_from_expression(expression)
            .is_some()
            || self
                .resolve_object_binding_from_expression(expression)
                .is_some_and(|binding| {
                    object_binding_lookup_value(
                        &binding,
                        &function_constructor_realm_id_property_expression(),
                    )
                    .is_some()
                        || object_binding_lookup_value(
                            &binding,
                            &function_constructor_source_property_expression(),
                        )
                        .is_some()
                })
            || self
                .resolve_static_constructed_function_metadata_object_binding(expression)
                .is_some()
            || self
                .resolve_static_constructed_function_source_expression(expression)
                .is_some()
            || self.infer_value_kind(expression) == Some(StaticValueKind::Function)
    }

    pub(in crate::backend::direct_wasm) fn emit_reflect_apply_call(
        &mut self,
        arguments: &[CallArgument],
    ) -> DirectResult<bool> {
        let expanded_arguments = self.expand_call_arguments(arguments);
        let target_expression = expanded_arguments
            .first()
            .cloned()
            .unwrap_or(Expression::Undefined);
        let Some(LocalFunctionBinding::User(function_name)) =
            self.resolve_function_binding_from_expression(&target_expression)
        else {
            return Ok(false);
        };
        let Some(user_function) = self.user_function(&function_name).cloned() else {
            return Ok(false);
        };
        let raw_this_expression = expanded_arguments
            .get(1)
            .cloned()
            .unwrap_or(Expression::Undefined);
        let apply_expression = expanded_arguments
            .get(2)
            .cloned()
            .unwrap_or(Expression::Undefined);
        let Some(call_arguments) =
            self.expand_apply_call_arguments_from_expression(&apply_expression)
        else {
            return Ok(false);
        };
        let capture_slots = self.resolve_function_expression_capture_slots(&target_expression);

        self.emit_numeric_expression(&target_expression)?;
        self.state.emission.output.instructions.push(0x1a);

        let this_hidden_name = self.allocate_named_hidden_local(
            "reflect_apply_this",
            self.infer_value_kind(&raw_this_expression)
                .unwrap_or(StaticValueKind::Unknown),
        );
        let this_hidden_local = self
            .state
            .runtime
            .locals
            .get(&this_hidden_name)
            .copied()
            .expect("fresh Reflect.apply hidden local must exist");
        self.emit_numeric_expression(&raw_this_expression)?;
        self.push_local_set(this_hidden_local);

        self.emit_numeric_expression(&apply_expression)?;
        self.state.emission.output.instructions.push(0x1a);
        for extra_argument in expanded_arguments.iter().skip(3) {
            self.emit_numeric_expression(extra_argument)?;
            self.state.emission.output.instructions.push(0x1a);
        }

        self.emit_user_function_call_with_function_this_binding(
            &user_function,
            &call_arguments,
            &Expression::Identifier(this_hidden_name),
            capture_slots.as_ref(),
        )?;
        let call_argument_expressions = call_arguments
            .iter()
            .map(|argument| argument.expression().clone())
            .collect::<Vec<_>>();
        self.sync_direct_arguments_assignments_from_static_user_call(
            &user_function,
            &call_argument_expressions,
        );
        Ok(true)
    }

    pub(in crate::backend::direct_wasm) fn emit_reflect_construct_call(
        &mut self,
        arguments: &[CallArgument],
    ) -> DirectResult<bool> {
        let trace = std::env::var_os("AYY_TRACE_REFLECT_CONSTRUCT_CALL").is_some();
        let expanded_arguments = self.expand_call_arguments(arguments);
        let target_expression = expanded_arguments
            .first()
            .cloned()
            .unwrap_or(Expression::Undefined);
        let Some(LocalFunctionBinding::User(function_name)) =
            self.resolve_function_binding_from_expression(&target_expression)
        else {
            return Ok(false);
        };
        let Some(user_function) = self.user_function(&function_name).cloned() else {
            return Ok(false);
        };
        if !user_function.is_constructible() {
            return Ok(false);
        }
        let apply_expression = expanded_arguments
            .get(1)
            .cloned()
            .unwrap_or(Expression::Undefined);
        let Some(call_arguments) =
            self.expand_apply_call_arguments_from_expression(&apply_expression)
        else {
            return Ok(false);
        };
        let new_target_expression = expanded_arguments.get(2).cloned();
        let new_target_value = if let Some(new_target_expression) = new_target_expression.as_ref() {
            if let Some(value) =
                self.user_function_runtime_value_from_expression(new_target_expression)
            {
                if trace {
                    eprintln!(
                        "reflect_construct_call:new_target_user expression={new_target_expression:?} value={value}"
                    );
                }
                value
            } else if self
                .expression_is_static_constructible_reflect_new_target(new_target_expression)
            {
                if trace {
                    eprintln!(
                        "reflect_construct_call:new_target_static expression={new_target_expression:?} fallback_value={}",
                        user_function_runtime_value(&user_function)
                    );
                }
                user_function_runtime_value(&user_function)
            } else {
                if trace {
                    eprintln!(
                        "reflect_construct_call:unsupported_new_target expression={new_target_expression:?}"
                    );
                }
                return Ok(false);
            }
        } else {
            user_function_runtime_value(&user_function)
        };
        if trace {
            eprintln!(
                "reflect_construct_call:specialized target={target_expression:?} function={} new_target_value={new_target_value} call_arguments={call_arguments:?}",
                user_function.name
            );
        }
        let capture_slots = self.initialize_user_function_capture_slots_from_expression(
            &target_expression,
            &user_function,
        )?;
        let ordinary_this_expression = Expression::Object(Vec::new());
        let construct_this_expression = if self.user_function_is_derived_constructor(&user_function)
        {
            &Expression::Undefined
        } else {
            &ordinary_this_expression
        };

        self.emit_numeric_expression(&target_expression)?;
        self.state.emission.output.instructions.push(0x1a);
        self.emit_numeric_expression(&apply_expression)?;
        self.state.emission.output.instructions.push(0x1a);
        if let Some(new_target_expression) = new_target_expression.as_ref() {
            self.emit_numeric_expression(new_target_expression)?;
            self.state.emission.output.instructions.push(0x1a);
        }
        for extra_argument in expanded_arguments.iter().skip(3) {
            self.emit_numeric_expression(extra_argument)?;
            self.state.emission.output.instructions.push(0x1a);
        }

        if let Some(capture_slots) = capture_slots.as_ref() {
            self.emit_user_function_call_with_new_target_and_this_expression_and_bound_captures_without_static_snapshot(
                &user_function,
                &call_arguments,
                new_target_value,
                construct_this_expression,
                capture_slots,
            )?;
        } else {
            self.emit_user_function_call_with_new_target_and_this_expression_without_static_snapshot(
                &user_function,
                &call_arguments,
                new_target_value,
                construct_this_expression,
            )?;
        }
        let constructor_return_local = self.allocate_temp_local();
        self.push_local_set(constructor_return_local);
        let call_effect_nonlocal_bindings =
            self.collect_user_function_call_effect_nonlocal_bindings(&user_function);
        self.sync_current_function_capture_runtime_values_for_call_effects(
            &call_effect_nonlocal_bindings,
        )?;
        if !call_effect_nonlocal_bindings.is_empty() {
            self.invalidate_static_binding_metadata_for_names(&call_effect_nonlocal_bindings);
        }
        let call_argument_expressions = call_arguments
            .iter()
            .map(|argument| argument.expression().clone())
            .collect::<Vec<_>>();
        self.sync_direct_arguments_assignments_from_static_user_call(
            &user_function,
            &call_argument_expressions,
        );
        self.push_local_get(constructor_return_local);
        self.state.emission.output.instructions.push(0x1a);
        self.push_i32_const(JS_TYPEOF_OBJECT_TAG);
        Ok(true)
    }

    pub(in crate::backend::direct_wasm) fn emit_function_prototype_call_or_apply(
        &mut self,
        object: &Expression,
        property: &Expression,
        arguments: &[CallArgument],
    ) -> DirectResult<bool> {
        let Expression::String(property_name) = property else {
            return Ok(false);
        };
        if property_name != "call" && property_name != "apply" {
            return Ok(false);
        }
        if property_name == "call"
            && matches!(
                object,
                Expression::Member {
                    object: prototype_object,
                    property: to_string_property,
                } if matches!(
                    prototype_object.as_ref(),
                    Expression::Member {
                        object: object_constructor,
                        property: prototype_property,
                    } if matches!(object_constructor.as_ref(), Expression::Identifier(name) if name == "Object")
                        && matches!(prototype_property.as_ref(), Expression::String(name) if name == "prototype")
                ) && matches!(to_string_property.as_ref(), Expression::String(name) if name == "toString")
            )
        {
            let static_tag = arguments.first().and_then(|argument| match argument {
                CallArgument::Expression(receiver) | CallArgument::Spread(receiver) => {
                    self.resolve_static_typed_array_to_string_tag(receiver)
                }
            });
            self.emit_numeric_expression(object)?;
            self.state.emission.output.instructions.push(0x1a);
            for argument in arguments {
                match argument {
                    CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                        self.emit_numeric_expression(expression)?;
                        self.state.emission.output.instructions.push(0x1a);
                    }
                }
            }
            if let Some(tag) = static_tag {
                self.emit_static_string_literal(&tag)?;
            } else {
                self.push_i32_const(JS_TYPEOF_STRING_TAG);
            }
            return Ok(true);
        }
        if property_name == "call" && self.emit_has_own_property_call(object, arguments)? {
            return Ok(true);
        }
        if property_name == "call" && self.emit_property_is_enumerable_call(object, arguments)? {
            return Ok(true);
        }

        let Some(function_binding) = self.resolve_function_binding_from_expression(object) else {
            return Ok(false);
        };
        if property_name == "call"
            && matches!(&function_binding, LocalFunctionBinding::Builtin(name) if name == "Promise.prototype.then")
        {
            let Some(then_call) = self.promise_prototype_then_call_expression(object, arguments)
            else {
                return Ok(false);
            };
            let Some(_) = self.consume_immediate_promise_outcome(&then_call)? else {
                return Ok(false);
            };
            self.push_i32_const(JS_TYPEOF_OBJECT_TAG);
            return Ok(true);
        }
        let LocalFunctionBinding::User(function_name) = function_binding else {
            return Ok(false);
        };
        let Some(user_function) = self.user_function(&function_name).cloned() else {
            return Ok(false);
        };
        let capture_slots = self.resolve_function_expression_capture_slots(object);

        let expanded_arguments = self.expand_call_arguments(arguments);
        let raw_this_expression = expanded_arguments
            .first()
            .cloned()
            .unwrap_or(Expression::Undefined);
        let (call_arguments, apply_expression) = if property_name == "call" {
            (
                expanded_arguments
                    .iter()
                    .skip(1)
                    .cloned()
                    .map(CallArgument::Expression)
                    .collect::<Vec<_>>(),
                None,
            )
        } else {
            let apply_expression = expanded_arguments
                .get(1)
                .cloned()
                .unwrap_or(Expression::Undefined);
            let Some(call_arguments) =
                self.expand_apply_call_arguments_from_expression(&apply_expression)
            else {
                return Ok(false);
            };
            (call_arguments, Some(apply_expression))
        };
        let materialized_this_expression = self.materialize_static_expression(&raw_this_expression);
        let materialized_call_arguments = call_arguments
            .iter()
            .map(|argument| match argument {
                CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                    self.materialize_static_expression(expression)
                }
            })
            .collect::<Vec<_>>();
        let call_argument_expressions = call_arguments
            .iter()
            .map(|argument| match argument {
                CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                    expression.clone()
                }
            })
            .collect::<Vec<_>>();

        self.emit_numeric_expression(object)?;
        self.state.emission.output.instructions.push(0x1a);

        if capture_slots.is_none()
            && (user_function.strict || user_function.lexical_this)
            && self.can_inline_user_function_call_with_explicit_call_frame(
                &user_function,
                &materialized_call_arguments,
                &materialized_this_expression,
            )
        {
            if let Some(apply_expression) = &apply_expression {
                self.emit_numeric_expression(apply_expression)?;
                self.state.emission.output.instructions.push(0x1a);
                for extra_argument in expanded_arguments.iter().skip(2) {
                    self.emit_numeric_expression(extra_argument)?;
                    self.state.emission.output.instructions.push(0x1a);
                }
            }
            let result_local = self.allocate_temp_local();
            if self.emit_inline_user_function_summary_with_explicit_call_frame(
                &user_function,
                &call_argument_expressions,
                &materialized_this_expression,
                result_local,
            )? {
                self.push_local_get(result_local);
                return Ok(true);
            }
        }

        let this_hidden_name = if apply_expression.is_some() {
            let this_hidden_name = self.allocate_named_hidden_local(
                "call_apply_this",
                self.infer_value_kind(&raw_this_expression)
                    .unwrap_or(StaticValueKind::Unknown),
            );
            let this_hidden_local = self
                .state
                .runtime
                .locals
                .get(&this_hidden_name)
                .copied()
                .expect("fresh call/apply hidden local must exist");
            self.emit_numeric_expression(&raw_this_expression)?;
            self.push_local_set(this_hidden_local);
            Some(this_hidden_name)
        } else {
            None
        };
        if let Some(apply_expression) = &apply_expression {
            self.emit_numeric_expression(apply_expression)?;
            self.state.emission.output.instructions.push(0x1a);
            for extra_argument in expanded_arguments.iter().skip(2) {
                self.emit_numeric_expression(extra_argument)?;
                self.state.emission.output.instructions.push(0x1a);
            }
        }
        let this_expression = this_hidden_name
            .map(Expression::Identifier)
            .unwrap_or_else(|| raw_this_expression.clone());
        self.emit_user_function_call_with_function_this_binding(
            &user_function,
            &call_arguments,
            &this_expression,
            capture_slots.as_ref(),
        )?;
        Ok(true)
    }
}
