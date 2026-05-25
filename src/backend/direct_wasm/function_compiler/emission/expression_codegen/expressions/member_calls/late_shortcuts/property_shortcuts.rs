use super::*;

impl<'a> FunctionCompiler<'a> {
    fn static_object_reference_matches(
        &self,
        candidate: &Expression,
        target: &Expression,
        target_materialized: &Expression,
        target_binding: Option<&LocalFunctionBinding>,
        target_identity: Option<&Expression>,
    ) -> bool {
        if static_expression_matches(candidate, target)
            || static_expression_matches(candidate, target_materialized)
            || target_binding.is_some_and(|target_binding| {
                self.resolve_function_binding_from_expression(candidate)
                    .as_ref()
                    == Some(target_binding)
            })
        {
            return true;
        }

        let candidate_materialized = self.materialize_static_expression(candidate);
        if !static_expression_matches(&candidate_materialized, candidate)
            && (static_expression_matches(&candidate_materialized, target)
                || static_expression_matches(&candidate_materialized, target_materialized))
        {
            return true;
        }

        if let Some(target_identity) = target_identity
            && let Some(candidate_identity) = self
                .resolve_static_object_identity_expression(candidate)
                .or_else(|| self.resolve_static_object_identity_expression(&candidate_materialized))
        {
            return static_expression_matches(&candidate_identity, target_identity);
        }

        false
    }

    fn expression_static_prototype_chain_contains_object_result(
        &self,
        expression: &Expression,
        target: &Expression,
    ) -> Option<bool> {
        let Some(mut prototype) = self.resolve_static_object_prototype_expression(expression)
        else {
            return None;
        };
        let target_materialized = self.materialize_static_expression(target);
        let target_binding = self.resolve_function_binding_from_expression(target);
        let target_identity = self
            .resolve_static_object_identity_expression(target)
            .or_else(|| self.resolve_static_object_identity_expression(&target_materialized));
        if target_identity.is_none()
            && target_binding.is_none()
            && !matches!(
                target_materialized,
                Expression::Object(_) | Expression::Array(_)
            )
        {
            return None;
        }
        let mut visited = Vec::new();

        for _ in 0..32 {
            let materialized_prototype = self.materialize_static_expression(&prototype);
            for candidate in [&prototype, &materialized_prototype] {
                if self.static_object_reference_matches(
                    candidate,
                    target,
                    &target_materialized,
                    target_binding.as_ref(),
                    target_identity.as_ref(),
                ) {
                    return Some(true);
                }
            }
            if matches!(materialized_prototype, Expression::Null)
                || visited.iter().any(|visited| {
                    static_expression_matches(visited, &prototype)
                        || static_expression_matches(visited, &materialized_prototype)
                })
            {
                return Some(false);
            }
            visited.push(prototype.clone());

            let Some(next_prototype) = self
                .resolve_static_object_prototype_expression(&materialized_prototype)
                .or_else(|| self.resolve_static_object_prototype_expression(&prototype))
            else {
                return None;
            };
            if static_expression_matches(&next_prototype, &prototype)
                || static_expression_matches(&next_prototype, &materialized_prototype)
            {
                return Some(false);
            }
            prototype = next_prototype;
        }

        None
    }

    pub(super) fn emit_property_member_call_shortcuts(
        &mut self,
        source_expression: &Expression,
        object: &Expression,
        property: &Expression,
        arguments: &[CallArgument],
    ) -> DirectResult<bool> {
        if matches!(
            property,
            Expression::String(property_name)
                if property_name == "__lookupGetter__" || property_name == "__lookupSetter__"
        ) && let [CallArgument::Expression(argument_property)] = arguments
        {
            self.emit_numeric_expression(object)?;
            self.state.emission.output.instructions.push(0x1a);
            self.emit_numeric_expression(argument_property)?;
            self.state.emission.output.instructions.push(0x1a);
            let accessor = if matches!(property, Expression::String(property_name) if property_name == "__lookupGetter__")
            {
                self.resolve_member_getter_binding(object, argument_property)
            } else {
                self.resolve_member_setter_binding(object, argument_property)
            };
            match accessor {
                Some(LocalFunctionBinding::User(function_name)) => {
                    if let Some(runtime_value) = self.user_function_runtime_value(&function_name) {
                        self.push_i32_const(runtime_value);
                    } else {
                        self.push_i32_const(JS_UNDEFINED_TAG);
                    }
                }
                Some(LocalFunctionBinding::Builtin(function_name)) => {
                    if let Some(runtime_value) = builtin_function_runtime_value(&function_name) {
                        self.push_i32_const(runtime_value);
                    } else {
                        self.push_i32_const(JS_UNDEFINED_TAG);
                    }
                }
                None => self.push_i32_const(JS_UNDEFINED_TAG),
            }
            return Ok(true);
        }

        if matches!(property, Expression::String(property_name) if property_name == "isPrototypeOf")
            && let [CallArgument::Expression(candidate)] = arguments
            && let Some(result) = self
                .expression_static_prototype_chain_contains_object_result(candidate, object)
                .or_else(|| {
                    self.expression_inherits_from_prototype_for_instanceof(candidate, object)
                        .then_some(true)
                })
        {
            self.emit_numeric_expression(object)?;
            self.state.emission.output.instructions.push(0x1a);
            self.emit_numeric_expression(candidate)?;
            self.state.emission.output.instructions.push(0x1a);
            self.push_i32_const(if result { 1 } else { 0 });
            return Ok(true);
        }

        if matches!(property, Expression::String(property_name) if property_name == "hasOwnProperty")
            && let [CallArgument::Expression(argument_property)] = arguments
        {
            let trace_has_own_shortcut = std::env::var_os("AYY_TRACE_HAS_OWN_SHORTCUT").is_some();
            if trace_has_own_shortcut {
                eprintln!(
                    "has_own_shortcut:start fn={:?} object={object:?} property={argument_property:?} object_binding={} runtime_owner={:?} function_binding={}",
                    self.current_function_name(),
                    self.resolve_object_binding_from_expression(object)
                        .is_some(),
                    match object {
                        Expression::Identifier(name) => {
                            self.runtime_object_property_shadow_owner_name_for_identifier(name)
                        }
                        Expression::This => {
                            self.runtime_object_property_shadow_owner_name_for_identifier("this")
                        }
                        _ => None,
                    },
                    self.resolve_function_binding_from_expression(object)
                        .is_some()
                );
            }
            if let Some(has_property) = self
                .resolve_top_level_global_object_has_own_property_result(object, argument_property)
            {
                self.emit_numeric_expression(object)?;
                self.state.emission.output.instructions.push(0x1a);
                self.emit_numeric_expression(argument_property)?;
                self.state.emission.output.instructions.push(0x1a);
                self.push_i32_const(if has_property { 1 } else { 0 });
                return Ok(true);
            }
            if let Some(array_binding) = self.resolve_array_binding_from_expression(object) {
                let has_property = matches!(argument_property, Expression::String(property_name) if property_name == "length")
                    || argument_index_from_expression(argument_property).is_some_and(|index| {
                        array_binding
                            .values
                            .get(index as usize)
                            .is_some_and(|value| value.is_some())
                    });
                self.emit_numeric_expression(object)?;
                self.state.emission.output.instructions.push(0x1a);
                self.emit_numeric_expression(argument_property)?;
                self.state.emission.output.instructions.push(0x1a);
                self.push_i32_const(if has_property { 1 } else { 0 });
                return Ok(true);
            }
            if self.current_function_requires_runtime_public_this_resolution()
                && self.expression_is_current_this_reference(object)
                && !is_private_property_name_expression(argument_property)
            {
                self.emit_numeric_expression(object)?;
                self.state.emission.output.instructions.push(0x1a);
                self.emit_numeric_expression(argument_property)?;
                self.state.emission.output.instructions.push(0x1a);
                if self.emit_runtime_known_object_has_property_check(object, argument_property)? {
                    return Ok(true);
                }
            }
            if let Some(object_binding) = self.resolve_object_binding_from_expression(object) {
                self.emit_numeric_expression(object)?;
                self.state.emission.output.instructions.push(0x1a);
                if self.runtime_object_property_shadow_deletion_may_affect_property(
                    object,
                    argument_property,
                ) {
                    self.emit_numeric_expression(argument_property)?;
                    self.state.emission.output.instructions.push(0x1a);
                    self.emit_object_get_own_property_descriptor_result(object, argument_property)?;
                    self.push_i32_const(JS_UNDEFINED_TAG);
                    self.push_binary_op(BinaryOp::NotEqual)?;
                    return Ok(true);
                }
                match self.resolve_static_object_has_own_property_result(object, argument_property)
                {
                    Some(Some(has_property)) => {
                        self.emit_numeric_expression(argument_property)?;
                        self.state.emission.output.instructions.push(0x1a);
                        self.push_i32_const(if has_property { 1 } else { 0 });
                    }
                    Some(None) => {
                        if !self.emit_runtime_known_object_has_property_check(
                            object,
                            argument_property,
                        )? {
                            self.emit_numeric_expression(argument_property)?;
                            self.state.emission.output.instructions.push(0x1a);
                            self.push_i32_const(0);
                        }
                    }
                    None => {
                        let has_property = self
                            .resolve_object_binding_property_value(
                                &object_binding,
                                argument_property,
                            )
                            .is_some();
                        self.emit_numeric_expression(argument_property)?;
                        self.state.emission.output.instructions.push(0x1a);
                        self.push_i32_const(if has_property { 1 } else { 0 });
                    }
                }
                return Ok(true);
            }
            if let Some(has_property) =
                self.resolve_function_object_has_own_property(object, argument_property)
            {
                self.emit_numeric_expression(object)?;
                self.state.emission.output.instructions.push(0x1a);
                self.emit_numeric_expression(argument_property)?;
                self.state.emission.output.instructions.push(0x1a);
                self.push_i32_const(if has_property { 1 } else { 0 });
                return Ok(true);
            }
            if self
                .resolve_function_binding_from_expression(object)
                .is_some()
            {
                self.emit_numeric_expression(object)?;
                self.state.emission.output.instructions.push(0x1a);
                self.emit_numeric_expression(argument_property)?;
                self.state.emission.output.instructions.push(0x1a);
                if self.emit_runtime_known_object_has_property_check(object, argument_property)? {
                    return Ok(true);
                }
                self.push_i32_const(0);
                return Ok(true);
            }
            if matches!(
                argument_property,
                Expression::String(property_name)
                    if property_name == "caller" || property_name == "arguments"
            ) && matches!(
                object,
                Expression::Member { object: receiver, .. }
                    if matches!(receiver.as_ref(), Expression::This)
            ) {
                self.emit_numeric_expression(object)?;
                self.state.emission.output.instructions.push(0x1a);
                self.emit_numeric_expression(argument_property)?;
                self.state.emission.output.instructions.push(0x1a);
                self.push_i32_const(0);
                return Ok(true);
            }
        }

        if matches!(object, Expression::Identifier(name) if name == "Object")
            && matches!(property, Expression::String(property_name) if property_name == "defineProperty")
            && let [
                CallArgument::Expression(target),
                CallArgument::Expression(property_name_expression),
                CallArgument::Expression(descriptor),
                ..,
            ] = arguments
        {
            if self.is_direct_arguments_object(target)
                && let Some(index) = argument_index_from_expression(property_name_expression)
                && let Some(descriptor) = resolve_property_descriptor_definition(descriptor)
                && self.apply_direct_arguments_define_property(index, &descriptor)?
            {
                self.push_i32_const(JS_TYPEOF_OBJECT_TAG);
                return Ok(true);
            }

            if let Some(accepted_without_mutation) = self
                .static_define_property_accepts_without_mutation(
                    target,
                    property_name_expression,
                    descriptor,
                )
            {
                if accepted_without_mutation {
                    self.emit_define_property_target_result_with_argument_effects(
                        target,
                        property_name_expression,
                        descriptor,
                    )?;
                    return Ok(true);
                }
                self.emit_define_property_argument_effects(
                    target,
                    property_name_expression,
                    descriptor,
                )?;
                return self.emit_named_error_throw("TypeError").map(|_| true);
            }

            let resolved_property_name = self
                .resolve_property_key_expression(property_name_expression)
                .unwrap_or_else(|| self.materialize_static_expression(property_name_expression));
            if let Some(target_binding) = self.resolve_object_binding_from_expression(target)
                && !object_binding_can_define_property(&target_binding, &resolved_property_name)
            {
                return self.emit_named_error_throw("TypeError").map(|_| true);
            }

            let trace_proxy_define_property =
                std::env::var_os("AYY_TRACE_PROXY_DEFINE_PROPERTY").is_some();
            if let Some(proxy_binding) = self.resolve_proxy_binding_from_expression(target)
                && let Some(define_property_binding) =
                    proxy_binding.define_property_binding.as_ref()
            {
                if trace_proxy_define_property {
                    eprintln!(
                        "object_define_property_shortcut:proxy target={target:?} proxy_target={:?} binding={define_property_binding:?}",
                        proxy_binding.target
                    );
                }
                let trap_arguments = [
                    proxy_binding.target.clone(),
                    property_name_expression.clone(),
                    descriptor.clone(),
                ];
                let trap_result = self.resolve_function_binding_static_return_bool(
                    define_property_binding,
                    &trap_arguments,
                );
                match define_property_binding {
                    LocalFunctionBinding::User(function_name) => {
                        if trace_proxy_define_property {
                            eprintln!(
                                "object_define_property_shortcut:emit-user binding={function_name}"
                            );
                        }
                        if let Some(user_function) = self.user_function(function_name).cloned() {
                            let call_arguments = trap_arguments
                                .iter()
                                .cloned()
                                .map(CallArgument::Expression)
                                .collect::<Vec<_>>();
                            self.emit_user_function_call(&user_function, &call_arguments)?;
                            self.state.emission.output.instructions.push(0x1a);
                            if self.user_function_is_direct_reflect_define_property_forwarder(
                                &user_function,
                            ) {
                                let reflect_arguments = vec![
                                    CallArgument::Expression(proxy_binding.target.clone()),
                                    CallArgument::Expression(property_name_expression.clone()),
                                    CallArgument::Expression(descriptor.clone()),
                                ];
                                if self.emit_reflect_define_property_call(
                                    &Expression::Identifier("Reflect".to_string()),
                                    &Expression::String("defineProperty".to_string()),
                                    &reflect_arguments,
                                )? {
                                    self.state.emission.output.instructions.push(0x1a);
                                }
                            }
                        } else if trace_proxy_define_property {
                            eprintln!(
                                "object_define_property_shortcut:missing-user binding={function_name}"
                            );
                        }
                    }
                    LocalFunctionBinding::Builtin(function_name) => {
                        if trace_proxy_define_property {
                            eprintln!(
                                "object_define_property_shortcut:emit-builtin binding={function_name}"
                            );
                        }
                        let call_arguments = trap_arguments
                            .iter()
                            .cloned()
                            .map(CallArgument::Expression)
                            .collect::<Vec<_>>();
                        if self.emit_builtin_call(function_name, &call_arguments)? {
                            self.state.emission.output.instructions.push(0x1a);
                        }
                    }
                }
                if matches!(trap_result, Some(false)) {
                    return self.emit_named_error_throw("TypeError").map(|_| true);
                }
                self.emit_numeric_expression(target)?;
                return Ok(true);
            }
            if trace_proxy_define_property {
                eprintln!("object_define_property_shortcut:fallback target={target:?}");
            }

            self.sync_static_define_property_descriptor_metadata_from_expression(
                target,
                property_name_expression,
                descriptor,
            );
            self.emit_numeric_expression(target)?;
            self.state.emission.output.instructions.push(0x1a);
            self.emit_property_key_expression_effects(property_name_expression)?;
            self.emit_numeric_expression(descriptor)?;
            self.state.emission.output.instructions.push(0x1a);
            self.push_i32_const(JS_TYPEOF_OBJECT_TAG);
            return Ok(true);
        }

        if matches!(property, Expression::String(property_name) if property_name == "hasOwnProperty")
            && let [CallArgument::Expression(argument_property)] = arguments
        {
            let direct_arguments = self.is_direct_arguments_object(object);
            let arguments_binding = self.resolve_arguments_binding_from_expression(object);
            if direct_arguments && let Expression::String(owned_property_name) = argument_property {
                match owned_property_name.as_str() {
                    "callee" | "length" => {
                        self.push_i32_const(
                            if self.direct_arguments_has_property(owned_property_name) {
                                1
                            } else {
                                0
                            },
                        );
                        return Ok(true);
                    }
                    _ => {
                        if let Some(index) =
                            canonical_array_index_from_property_name(owned_property_name)
                        {
                            if let Some(slot) = self.state.parameters.arguments_slots.get(&index) {
                                self.push_local_get(slot.present_local);
                            } else {
                                self.push_i32_const(0);
                            }
                            return Ok(true);
                        }
                    }
                }
            }
            if let Some(arguments_binding) = arguments_binding.as_ref()
                && let Expression::String(owned_property_name) = argument_property
            {
                let has_property = match owned_property_name.as_str() {
                    "callee" => arguments_binding.callee_present,
                    "length" => arguments_binding.length_present,
                    _ => owned_property_name
                        .parse::<usize>()
                        .ok()
                        .is_some_and(|index| index < arguments_binding.values.len()),
                };
                self.push_i32_const(if has_property { 1 } else { 0 });
                return Ok(true);
            }
            if let Some(has_property) =
                self.resolve_function_object_has_own_property(object, argument_property)
            {
                self.push_i32_const(if has_property { 1 } else { 0 });
                return Ok(true);
            }
            if self
                .resolve_function_binding_from_expression(object)
                .is_some()
            {
                if self.emit_runtime_known_object_has_property_check(object, argument_property)? {
                    return Ok(true);
                }
                self.push_i32_const(0);
                return Ok(true);
            }
        }

        if let Expression::Identifier(name) = object {
            let resolved_name = self
                .resolve_current_local_binding(name)
                .map(|(resolved_name, _)| resolved_name)
                .unwrap_or_else(|| name.clone());
            if let Some(descriptor) = self
                .state
                .speculation
                .static_semantics
                .objects
                .local_descriptor_bindings
                .get(&resolved_name)
                && matches!(property, Expression::String(property_name) if property_name == "hasOwnProperty")
                && let [CallArgument::Expression(Expression::String(owned_property_name))] =
                    arguments
            {
                let has_property = match owned_property_name.as_str() {
                    "configurable" | "enumerable" => true,
                    "value" => descriptor.value.is_some(),
                    "writable" => descriptor.writable.is_some(),
                    "get" => descriptor.has_get,
                    "set" => descriptor.has_set,
                    _ => false,
                };
                self.push_i32_const(if has_property { 1 } else { 0 });
                return Ok(true);
            }
        }

        if self
            .resolve_descriptor_binding_from_expression(source_expression)
            .is_some()
        {
            self.emit_ignored_call_arguments(arguments)?;
            self.push_i32_const(JS_TYPEOF_OBJECT_TAG);
            return Ok(true);
        }

        Ok(false)
    }

    fn user_function_is_direct_reflect_define_property_forwarder(
        &self,
        user_function: &UserFunction,
    ) -> bool {
        let Some(function) = self.resolve_registered_function_declaration(&user_function.name)
        else {
            return false;
        };
        let [target_param, key_param, descriptor_param, ..] = user_function.params.as_slice()
        else {
            return false;
        };
        function.body.iter().any(|statement| {
            let Statement::Return(Expression::Call { callee, arguments }) = statement else {
                return false;
            };
            matches!(
                callee.as_ref(),
                Expression::Member { object, property }
                    if matches!(object.as_ref(), Expression::Identifier(name) if name == "Reflect")
                        && matches!(property.as_ref(), Expression::String(name) if name == "defineProperty")
            ) && matches!(
                arguments.as_slice(),
                [
                    CallArgument::Expression(Expression::Identifier(target)),
                    CallArgument::Expression(Expression::Identifier(key)),
                    CallArgument::Expression(Expression::Identifier(descriptor)),
                ] if target == target_param && key == key_param && descriptor == descriptor_param
            )
        })
    }
}
