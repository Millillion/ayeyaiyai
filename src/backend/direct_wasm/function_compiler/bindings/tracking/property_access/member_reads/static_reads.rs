use super::*;

impl<'a> FunctionCompiler<'a> {
    fn emit_global_object_property_descriptor_value(
        &mut self,
        property_name: &str,
        value: &Expression,
    ) -> DirectResult<()> {
        if matches!(value, Expression::Identifier(source_name) if source_name == property_name) {
            if let Some(global_index) = self.resolve_global_binding_index(property_name) {
                self.emit_declared_global_binding_read(property_name, global_index)?;
                return Ok(());
            }
            if let Some(binding) = self.implicit_global_binding(property_name) {
                self.push_global_get(binding.present_index);
                self.state.emission.output.instructions.push(0x04);
                self.state.emission.output.instructions.push(I32_TYPE);
                self.push_control_frame();
                self.push_global_get(binding.value_index);
                self.state.emission.output.instructions.push(0x05);
                self.push_i32_const(JS_UNDEFINED_TAG);
                self.state.emission.output.instructions.push(0x0b);
                self.pop_control_frame();
                return Ok(());
            }
        }

        self.emit_numeric_expression(value)
    }

    fn emit_global_this_member_read_without_prelude(
        &mut self,
        object: &Expression,
        property: &Expression,
    ) -> DirectResult<bool> {
        if !matches!(object, Expression::Identifier(name) if name == "globalThis" && self.is_unshadowed_builtin_identifier(name))
        {
            return Ok(false);
        }

        let property = self
            .resolve_property_key_expression(property)
            .unwrap_or_else(|| self.materialize_static_expression(property));
        let Expression::String(property_name) = property else {
            return Ok(false);
        };

        if let Some(state) = self
            .backend
            .global_property_descriptor(&property_name)
            .cloned()
        {
            self.emit_global_object_property_descriptor_value(&property_name, &state.value)?;
            return Ok(true);
        }
        if property_name == "NaN" {
            self.push_i32_const(JS_NAN_TAG);
            return Ok(true);
        }
        if property_name == "Infinity" {
            self.emit_numeric_expression(&Expression::Number(f64::INFINITY))?;
            return Ok(true);
        }
        if property_name == "undefined" {
            self.push_i32_const(JS_UNDEFINED_TAG);
            return Ok(true);
        }
        if let Some(kind) = builtin_identifier_kind(&property_name) {
            match kind {
                StaticValueKind::Function => {
                    self.push_i32_const(
                        builtin_function_runtime_value(&property_name)
                            .unwrap_or(JS_TYPEOF_FUNCTION_TAG),
                    );
                    return Ok(true);
                }
                StaticValueKind::Object => {
                    self.push_i32_const(JS_TYPEOF_OBJECT_TAG);
                    return Ok(true);
                }
                _ => {}
            }
        }

        let property_expression = Expression::String(property_name.clone());
        if self
            .runtime_object_property_shadow_binding_name_for_expression(
                object,
                &property_expression,
            )
            .is_some_and(|shadow_name| self.global_has_implicit_binding(&shadow_name))
            || self.runtime_object_property_shadow_deletion_may_affect_property(
                object,
                &property_expression,
            )
        {
            return Ok(false);
        }

        Ok(false)
    }

    pub(super) fn emit_special_member_read_without_prelude(
        &mut self,
        object: &Expression,
        property: &Expression,
        static_array_property: &Expression,
    ) -> DirectResult<bool> {
        if matches!(property, Expression::String(property_name) if property_name == "global")
            && matches!(
                object,
                Expression::Call { callee, arguments }
                    if arguments.is_empty()
                        && matches!(
                            callee.as_ref(),
                            Expression::Member { object, property }
                                if matches!(object.as_ref(), Expression::Identifier(name) if name == "$262")
                                    && matches!(property.as_ref(), Expression::String(name) if name == "createRealm")
                        )
            )
        {
            self.push_i32_const(JS_TYPEOF_OBJECT_TAG);
            return Ok(true);
        }
        if self.emit_global_this_member_read_without_prelude(object, property)? {
            return Ok(true);
        }
        if self.state.speculation.execution_context.top_level_function
            && matches!(object, Expression::This)
        {
            let property = self
                .resolve_property_key_expression(property)
                .unwrap_or_else(|| self.materialize_static_expression(property));
            if let Expression::String(ref property_name) = property {
                if let Some(state) = self.backend.global_property_descriptor(&property_name) {
                    if let Some(value) = state.writable.map(|_| state.value.clone()) {
                        self.emit_numeric_expression(&value)?;
                    } else {
                        self.push_i32_const(JS_UNDEFINED_TAG);
                    }
                    return Ok(true);
                }
                if property_name == "NaN" {
                    self.push_i32_const(JS_NAN_TAG);
                    return Ok(true);
                }
                if property_name == "Infinity" {
                    self.emit_numeric_expression(&Expression::Number(f64::INFINITY))?;
                    return Ok(true);
                }
                if property_name == "undefined" {
                    self.push_i32_const(JS_UNDEFINED_TAG);
                    return Ok(true);
                }
                if let Some(kind) = builtin_identifier_kind(&property_name) {
                    match kind {
                        StaticValueKind::Function => {
                            self.push_i32_const(
                                builtin_function_runtime_value(&property_name)
                                    .unwrap_or(JS_TYPEOF_FUNCTION_TAG),
                            );
                            return Ok(true);
                        }
                        StaticValueKind::Object => {
                            self.push_i32_const(JS_TYPEOF_OBJECT_TAG);
                            return Ok(true);
                        }
                        _ => {}
                    }
                }
            }
        }
        let private_receiver_requires_runtime_brand_check = self
            .is_private_member_read_property(property)
            && (matches!(object, Expression::This)
                || self
                    .resolve_bound_alias_expression(object)
                    .is_some_and(|resolved| {
                        !static_expression_matches(&resolved, object)
                            && matches!(resolved, Expression::This)
                    })
                || self.expression_uses_runtime_dynamic_binding(object));
        if matches!(property, Expression::String(property_name) if property_name == "length")
            && self
                .resolve_function_binding_from_expression(object)
                .is_none()
            && self
                .resolve_member_getter_binding(object, property)
                .is_none()
            && self
                .resolve_member_function_binding(object, property)
                .is_none()
            && self
                .resolve_member_setter_binding(object, property)
                .is_none()
            && let Expression::String(text) = self.materialize_static_expression(object)
        {
            self.push_i32_const(text.encode_utf16().count() as i32);
            return Ok(true);
        }
        if matches!(object, Expression::Identifier(name) if name == "Number" && self.is_unshadowed_builtin_identifier(name))
            && matches!(property, Expression::String(property_name) if property_name == "NaN")
        {
            self.push_i32_const(JS_NAN_TAG);
            return Ok(true);
        }
        if let Some(step_binding) = self.resolve_iterator_step_binding_from_expression(object) {
            if let Expression::String(property_name) = property {
                match property_name.as_str() {
                    "done" => {
                        if std::env::var_os("AYY_TRACE_ITERATOR_CACHED_SLOTS").is_some() {
                            match &step_binding {
                                IteratorStepBinding::Runtime {
                                    done_local,
                                    value_local,
                                    static_done,
                                    static_value,
                                    ..
                                } => eprintln!(
                                    "iterator_step_member_read object={object:?} property=done done_local={done_local} value_local={value_local} static_done={static_done:?} static_value={static_value:?}"
                                ),
                            }
                        }
                        match step_binding {
                            IteratorStepBinding::Runtime { done_local, .. } => {
                                self.push_local_get(done_local);
                            }
                        }
                        return Ok(true);
                    }
                    "value" => {
                        if std::env::var_os("AYY_TRACE_ITERATOR_CACHED_SLOTS").is_some() {
                            match &step_binding {
                                IteratorStepBinding::Runtime {
                                    done_local,
                                    value_local,
                                    static_done,
                                    static_value,
                                    ..
                                } => eprintln!(
                                    "iterator_step_member_read object={object:?} property=value done_local={done_local} value_local={value_local} static_done={static_done:?} static_value={static_value:?}"
                                ),
                            }
                        }
                        match step_binding {
                            IteratorStepBinding::Runtime { value_local, .. } => {
                                self.push_local_get(value_local);
                            }
                        }
                        return Ok(true);
                    }
                    _ => {}
                }
            }
        }
        let has_static_property_key = matches!(
            static_array_property,
            Expression::String(_) | Expression::Number(_)
        );
        if !has_static_property_key {
            return Ok(false);
        }
        let has_runtime_shadow_binding = self
            .resolve_runtime_object_property_shadow_binding(object, property)
            .is_some()
            || self
                .resolve_runtime_object_property_shadow_deleted_binding(object, property)
                .is_some();
        if !has_runtime_shadow_binding
            && !self.is_private_member_read_property(property)
            && !private_receiver_requires_runtime_brand_check
        {
            if let Some(value) =
                self.resolve_static_iterator_step_assignment_value(&Expression::Member {
                    object: Box::new(object.clone()),
                    property: Box::new(property.clone()),
                })
            {
                self.emit_numeric_expression(&value)?;
                return Ok(true);
            }
            if let Some(text) = self.resolve_static_string_value(&Expression::Member {
                object: Box::new(object.clone()),
                property: Box::new(property.clone()),
            }) {
                if std::env::var_os("AYY_TRACE_RUNTIME_SHADOWS").is_some() {
                    eprintln!(
                        "runtime_shadow_member_static_string object={object:?} property={property:?} text={text:?}"
                    );
                }
                self.emit_static_string_literal(&text)?;
                return Ok(true);
            }
            if let Some(value) =
                self.resolve_primitive_prototype_property_value(object, static_array_property)
            {
                self.emit_numeric_expression(&value)?;
                return Ok(true);
            }
        }
        if let Expression::Identifier(name) = object {
            let resolved_view_name = self
                .resolve_current_local_binding(name)
                .map(|(resolved_name, _)| resolved_name)
                .filter(|resolved_name| {
                    self.state
                        .speculation
                        .static_semantics
                        .has_local_typed_array_view_binding(resolved_name)
                });
            let view_name = resolved_view_name.as_deref().unwrap_or(name);
            if self
                .state
                .speculation
                .static_semantics
                .has_local_typed_array_view_binding(view_name)
            {
                if matches!(static_array_property, Expression::String(text) if text == "buffer")
                    && let Some(buffer_expression) =
                        self.resolve_static_viewed_array_buffer_expression(object)
                {
                    self.emit_numeric_expression(&buffer_expression)?;
                    return Ok(true);
                }
                if matches!(static_array_property, Expression::String(text) if text == "length") {
                    if let Some(length_local) = self
                        .state
                        .speculation
                        .static_semantics
                        .runtime_array_length_local(view_name)
                    {
                        self.push_local_get(length_local);
                    } else {
                        self.push_i32_const(0);
                    }
                    return Ok(true);
                }
                if let Some(index) = argument_index_from_expression(static_array_property) {
                    if let Some(oob_local) = self
                        .state
                        .speculation
                        .static_semantics
                        .runtime_typed_array_oob_local(view_name)
                    {
                        self.push_local_get(oob_local);
                        self.state.emission.output.instructions.push(0x04);
                        self.state.emission.output.instructions.push(I32_TYPE);
                        self.push_control_frame();
                        self.push_i32_const(JS_UNDEFINED_TAG);
                        self.state.emission.output.instructions.push(0x05);
                        if !self.emit_runtime_array_slot_read(view_name, index)? {
                            self.push_i32_const(JS_UNDEFINED_TAG);
                        }
                        self.state.emission.output.instructions.push(0x0b);
                        self.pop_control_frame();
                    } else if !self.emit_runtime_array_slot_read(view_name, index)? {
                        self.push_i32_const(JS_UNDEFINED_TAG);
                    }
                    return Ok(true);
                }
            }
        }
        if let Some(object_binding) = self.resolve_object_binding_from_expression(object)
            && let Some(value) = self.static_typed_array_member_value_from_binding(
                &object_binding,
                static_array_property,
            )
        {
            self.emit_numeric_expression(&value)?;
            return Ok(true);
        }
        if matches!(static_array_property, Expression::String(text) if text == "buffer")
            && let Some(buffer_expression) =
                self.resolve_static_viewed_array_buffer_expression(object)
        {
            self.emit_numeric_expression(&buffer_expression)?;
            return Ok(true);
        }
        if let Some(bytes_per_element) =
            self.resolve_typed_array_builtin_bytes_per_element(object, property)
        {
            self.push_i32_const(bytes_per_element as i32);
            return Ok(true);
        }
        if let Expression::Member {
            object: prototype_owner,
            property: prototype_property,
        } = self.materialize_static_expression(object)
            && matches!(prototype_property.as_ref(), Expression::String(name) if name == "prototype")
            && let Expression::Identifier(object_name) = prototype_owner.as_ref()
            && self.is_unshadowed_builtin_identifier(object_name)
            && let Expression::String(property_name) = self.materialize_static_expression(property)
            && let Some(value) = builtin_prototype_number_value(object_name, &property_name)
        {
            self.emit_numeric_expression(&Expression::Number(value))?;
            return Ok(true);
        }
        if let Some(function_name) = self.resolve_function_name_value(object, property) {
            self.emit_static_string_literal(&function_name)?;
            return Ok(true);
        }
        if let Some(function_length) = self.resolve_user_function_length(object, property) {
            self.push_i32_const(function_length as i32);
            return Ok(true);
        }
        if matches!(property, Expression::String(property_name) if property_name == "prototype")
            && matches!(
                object,
                Expression::Identifier(name)
                    if matches!(name.as_str(), "GeneratorFunction" | "AsyncGeneratorFunction")
                        && self.is_unshadowed_builtin_identifier(name)
            )
        {
            self.push_i32_const(JS_TYPEOF_OBJECT_TAG);
            return Ok(true);
        }
        if matches!(property, Expression::String(property_name) if property_name == "prototype") {
            if let Expression::Identifier(object_name) = self.materialize_static_expression(object)
                && self.is_unshadowed_builtin_identifier(&object_name)
                && let Some(kind) = builtin_constructor_prototype_kind(&object_name)
                && let Some(tag) = kind.as_typeof_tag()
            {
                self.push_i32_const(tag);
                return Ok(true);
            }
            let resolved_object = self
                .resolve_bound_alias_expression(object)
                .filter(|resolved| !static_expression_matches(resolved, object));
            let materialized_object = self.materialize_static_expression(object);
            if let Some(descriptor) = self.resolve_function_property_descriptor_binding(
                object,
                resolved_object.as_ref(),
                &materialized_object,
                "prototype",
            ) {
                let original_member = Expression::Member {
                    object: Box::new(object.clone()),
                    property: Box::new(Expression::String("prototype".to_string())),
                };
                match descriptor.value {
                    Some(value) if static_expression_matches(&value, &original_member) => {
                        self.push_i32_const(JS_TYPEOF_OBJECT_TAG);
                    }
                    Some(value) => {
                        self.emit_numeric_expression(&value)?;
                    }
                    None => {
                        self.push_i32_const(JS_UNDEFINED_TAG);
                    }
                }
                return Ok(true);
            }
        }
        Ok(false)
    }
}
