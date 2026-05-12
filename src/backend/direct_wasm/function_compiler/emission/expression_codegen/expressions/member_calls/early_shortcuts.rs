use super::*;

impl<'a> FunctionCompiler<'a> {
    pub(in crate::backend::direct_wasm) fn emit_early_member_call_shortcuts(
        &mut self,
        object: &Expression,
        property: &Expression,
        arguments: &[CallArgument],
    ) -> DirectResult<bool> {
        if self.emit_immediate_promise_member_call(object, property, arguments)? {
            return Ok(true);
        }
        if self.emit_function_prototype_call_or_apply(object, property, arguments)? {
            return Ok(true);
        }
        if matches!(object, Expression::Identifier(name) if name == "assert")
            && matches!(property, Expression::String(name) if name == "sameValue")
            && self.emit_assertion_builtin_call("__assertSameValue", arguments)?
        {
            return Ok(true);
        }
        if matches!(object, Expression::Identifier(name) if name == "assert")
            && matches!(property, Expression::String(name) if name == "notSameValue")
            && self.emit_assertion_builtin_call("__assertNotSameValue", arguments)?
        {
            return Ok(true);
        }
        if matches!(object, Expression::Identifier(name) if name == "assert")
            && matches!(property, Expression::String(name) if name == "throws")
            && self.emit_assert_throws_call(arguments)?
        {
            return Ok(true);
        }
        if self.emit_array_is_array_call(object, property, arguments)? {
            return Ok(true);
        }
        if self.emit_object_is_call(object, property, arguments)? {
            return Ok(true);
        }
        if self.emit_object_create_call(object, property, arguments)? {
            return Ok(true);
        }
        if self.emit_object_get_prototype_of_call(object, property, arguments)? {
            return Ok(true);
        }
        if self.emit_object_is_extensible_call(object, property, arguments)? {
            return Ok(true);
        }
        if self.emit_object_prevent_extensions_call(object, property, arguments)? {
            return Ok(true);
        }
        if self.emit_object_set_prototype_of_call(object, property, arguments)? {
            return Ok(true);
        }
        if self.emit_static_map_set_call(object, property, arguments)? {
            return Ok(true);
        }
        if self.emit_static_weak_collection_mutation_call(object, property, arguments)? {
            return Ok(true);
        }
        if self.emit_static_member_builtin_call_result(object, property, arguments)? {
            return Ok(true);
        }
        if matches!(property, Expression::String(property_name) if property_name == "resize")
            && let (
                Expression::Identifier(buffer_name),
                Some(
                    CallArgument::Expression(length_expression)
                    | CallArgument::Spread(length_expression),
                ),
            ) = (object, arguments.first())
            && let Some(new_length) = extract_typed_array_element_count(length_expression)
        {
            self.emit_numeric_expression(object)?;
            self.state.emission.output.instructions.push(0x1a);
            self.emit_numeric_expression(length_expression)?;
            self.state.emission.output.instructions.push(0x1a);
            for argument in arguments.iter().skip(1) {
                match argument {
                    CallArgument::Expression(expression) | CallArgument::Spread(expression) => {
                        self.emit_numeric_expression(expression)?;
                        self.state.emission.output.instructions.push(0x1a);
                    }
                }
            }
            if self.apply_resizable_array_buffer_resize(buffer_name, new_length)? {
                self.push_i32_const(JS_UNDEFINED_TAG);
                return Ok(true);
            }
        }
        if matches!(property, Expression::String(property_name) if property_name == "resize") {
            self.emit_numeric_expression(object)?;
            self.state.emission.output.instructions.push(0x1a);
            self.emit_ignored_call_arguments(arguments)?;
            self.push_i32_const(JS_UNDEFINED_TAG);
            return Ok(true);
        }
        Ok(false)
    }

    fn emit_static_member_builtin_call_result(
        &mut self,
        object: &Expression,
        property: &Expression,
        arguments: &[CallArgument],
    ) -> DirectResult<bool> {
        if !inline_summary_side_effect_free_expression(object)
            || !inline_summary_side_effect_free_expression(property)
            || arguments
                .iter()
                .any(|argument| !inline_summary_side_effect_free_expression(argument.expression()))
        {
            return Ok(false);
        }
        if matches!(property, Expression::String(property_name) if property_name == "next")
            && let Expression::Identifier(iterator_name) = object
            && self
                .resolve_local_array_iterator_binding_name(iterator_name)
                .is_some()
        {
            return Ok(false);
        }
        let callee = Expression::Member {
            object: Box::new(object.clone()),
            property: Box::new(property.clone()),
        };
        let Some((value, _)) = self.resolve_static_call_result_expression_with_context(
            &callee,
            arguments,
            self.current_function_name(),
        ) else {
            return Ok(false);
        };
        self.emit_numeric_expression(&value)?;
        Ok(true)
    }

    fn emit_static_map_set_call(
        &mut self,
        object: &Expression,
        property: &Expression,
        arguments: &[CallArgument],
    ) -> DirectResult<bool> {
        let Expression::String(property_name) = property else {
            return Ok(false);
        };
        let Some(object_binding) = self.resolve_object_binding_from_expression(object) else {
            return Ok(false);
        };
        if !self.object_binding_is_static_map(&object_binding) {
            return Ok(false);
        }
        let Some(collection_kind) = self.static_map_kind_from_binding(&object_binding) else {
            return Ok(false);
        };
        let supported = matches!(
            (collection_kind.as_str(), property_name.as_str(), arguments),
            (
                "Map",
                "set",
                [
                    CallArgument::Expression(_) | CallArgument::Spread(_),
                    CallArgument::Expression(_) | CallArgument::Spread(_),
                    ..,
                ],
            ) | (
                "Set",
                "add",
                [CallArgument::Expression(_) | CallArgument::Spread(_), ..],
            ) | (
                "Map" | "Set",
                "delete",
                [CallArgument::Expression(_) | CallArgument::Spread(_), ..],
            )
        );
        if !supported {
            return Ok(false);
        }

        if property_name == "delete" {
            self.emit_numeric_expression(object)?;
            self.state.emission.output.instructions.push(0x1a);
            self.emit_ignored_call_arguments(arguments)?;
            self.apply_static_map_mutation_metadata(object, property_name, arguments);
            self.push_i32_const(JS_TYPEOF_BOOLEAN_TAG);
        } else {
            self.emit_numeric_expression(object)?;
            self.emit_ignored_call_arguments(arguments)?;
            self.apply_static_map_mutation_metadata(object, property_name, arguments);
        }
        Ok(true)
    }

    fn static_map_identifier_name(&self, object: &Expression) -> Option<String> {
        let Expression::Identifier(name) = object else {
            return None;
        };
        Some(
            self.resolve_current_local_binding(name)
                .map(|(resolved_name, _)| resolved_name)
                .unwrap_or_else(|| name.clone()),
        )
    }

    fn emit_static_weak_collection_mutation_call(
        &mut self,
        object: &Expression,
        property: &Expression,
        arguments: &[CallArgument],
    ) -> DirectResult<bool> {
        let Expression::String(property_name) = property else {
            return Ok(false);
        };
        let Some(mut object_binding) = self.resolve_object_binding_from_expression(object) else {
            return Ok(false);
        };
        let Some(collection_kind) = self.static_weak_collection_kind_from_binding(&object_binding)
        else {
            return Ok(false);
        };
        let value = match (collection_kind.as_str(), property_name.as_str(), arguments) {
            (
                "WeakMap",
                "set",
                [
                    CallArgument::Expression(key) | CallArgument::Spread(key),
                    CallArgument::Expression(value) | CallArgument::Spread(value),
                    ..,
                ],
            ) => {
                self.define_static_weak_collection_entry(
                    &mut object_binding,
                    key,
                    self.materialize_static_expression(value),
                );
                Some(object.clone())
            }
            (
                "WeakSet",
                "add",
                [
                    CallArgument::Expression(key) | CallArgument::Spread(key),
                    ..,
                ],
            ) => {
                self.define_static_weak_collection_entry(
                    &mut object_binding,
                    key,
                    Expression::Bool(true),
                );
                Some(object.clone())
            }
            _ => None,
        };
        let Some(result) = value else {
            return Ok(false);
        };
        let Some(object_name) = self.static_map_identifier_name(object) else {
            return Ok(false);
        };
        self.state
            .speculation
            .static_semantics
            .set_local_object_binding(&object_name, object_binding.clone());
        if self.binding_name_is_global(&object_name) {
            self.backend
                .sync_global_object_binding(&object_name, Some(object_binding));
        }

        self.emit_numeric_expression(object)?;
        self.state.emission.output.instructions.push(0x1a);
        self.emit_ignored_call_arguments(arguments)?;
        self.emit_numeric_expression(&result)?;
        Ok(true)
    }
}
