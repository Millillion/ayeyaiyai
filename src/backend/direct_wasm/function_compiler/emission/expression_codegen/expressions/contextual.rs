use super::*;

fn is_internal_assignment_temp(name: &str) -> bool {
    name.starts_with("__ayy_optional_base_")
        || name.starts_with("__ayy_target_object_")
        || name.starts_with("__ayy_target_property_")
        || name.starts_with("__ayy_postfix_previous_")
}

fn is_internal_target_property_temp(name: &str) -> bool {
    name.starts_with("__ayy_target_property_")
}

#[derive(Clone, Copy)]
enum TemplateObjectMemberRead {
    Index(u32),
    Length,
    RawArray,
    AbsentFrozenOwnProperty,
}

impl<'a> FunctionCompiler<'a> {
    pub(in crate::backend::direct_wasm) fn template_object_absent_static_own_property(
        property: &Expression,
    ) -> bool {
        let Some(property_name) = static_property_name_from_expression(property) else {
            return false;
        };
        if argument_index_from_expression(property).is_some() {
            return false;
        }
        !matches!(
            property_name.as_str(),
            "length"
                | "raw"
                | "__proto__"
                | "constructor"
                | "toString"
                | "toLocaleString"
                | "valueOf"
                | "hasOwnProperty"
                | "isPrototypeOf"
                | "propertyIsEnumerable"
                | "at"
                | "concat"
                | "copyWithin"
                | "entries"
                | "every"
                | "fill"
                | "filter"
                | "find"
                | "findIndex"
                | "findLast"
                | "findLastIndex"
                | "flat"
                | "flatMap"
                | "forEach"
                | "includes"
                | "indexOf"
                | "join"
                | "keys"
                | "lastIndexOf"
                | "map"
                | "pop"
                | "push"
                | "reduce"
                | "reduceRight"
                | "reverse"
                | "shift"
                | "slice"
                | "some"
                | "sort"
                | "splice"
                | "toReversed"
                | "toSorted"
                | "toSpliced"
                | "unshift"
                | "values"
                | "with"
        )
    }

    pub(in crate::backend::direct_wasm) fn current_function_declares_non_eval_binding_source(
        &self,
        name: &str,
    ) -> bool {
        let Some(function) = self.current_user_function_declaration() else {
            return false;
        };
        let source_name = scoped_binding_source_name(name).unwrap_or(name);
        collect_declared_bindings_from_statements_recursive(&function.body)
            .into_iter()
            .any(|binding| scoped_binding_source_name(&binding).unwrap_or(&binding) == source_name)
            || function.params.iter().any(|parameter| {
                scoped_binding_source_name(&parameter.name).unwrap_or(&parameter.name)
                    == source_name
            })
            || function.self_binding.as_ref().is_some_and(|self_binding| {
                scoped_binding_source_name(self_binding).unwrap_or(self_binding) == source_name
            })
            || source_name == "arguments"
    }

    pub(in crate::backend::direct_wasm) fn assignment_value_declares_static_direct_eval_var_binding(
        &self,
        name: &str,
        value: &Expression,
    ) -> bool {
        let mut bindings = HashSet::new();
        let caller_strict = self
            .current_user_function_declaration()
            .map(|function| function.strict)
            .unwrap_or(self.state.speculation.execution_context.strict_mode);
        collect_static_direct_eval_var_bindings_from_expression(
            value,
            caller_strict,
            &mut bindings,
        );
        let source_name = scoped_binding_source_name(name).unwrap_or(name);
        bindings
            .into_iter()
            .any(|binding| scoped_binding_source_name(&binding).unwrap_or(&binding) == source_name)
    }

    pub(in crate::backend::direct_wasm) fn emit_identifier_expression_value(
        &mut self,
        name: &str,
    ) -> DirectResult<()> {
        let trace_identifier_dispatch = std::env::var_os("AYY_TRACE_IDENTIFIER_DISPATCH").is_some();
        if trace_identifier_dispatch {
            eprintln!("identifier_dispatch:start name={name}");
        }
        if let Some(scope_object) = self.resolve_with_scope_binding(name)? {
            if trace_identifier_dispatch {
                eprintln!("identifier_dispatch:path scoped name={name}");
            }
            self.emit_scoped_property_read(&scope_object, name)?;
        } else {
            if trace_identifier_dispatch {
                eprintln!("identifier_dispatch:path plain name={name}");
            }
            self.with_suspended_with_scopes(|compiler| compiler.emit_plain_identifier_read(name))?;
        }
        if trace_identifier_dispatch {
            eprintln!("identifier_dispatch:done name={name}");
        }
        Ok(())
    }

    pub(in crate::backend::direct_wasm) fn emit_assign_expression_value(
        &mut self,
        name: &str,
        value: &Expression,
    ) -> DirectResult<()> {
        let scoped_target = self.resolve_with_scope_binding(name)?;
        let resolved_reference_local = scoped_target
            .is_none()
            .then(|| self.resolve_current_local_binding(name))
            .flatten();
        let resolved_reference_local = if resolved_reference_local.is_some()
            && self.assignment_value_declares_static_direct_eval_var_binding(name, value)
            && !self.current_function_declares_non_eval_binding_source(name)
        {
            None
        } else {
            resolved_reference_local
        };
        let reference_targets_capture = scoped_target.is_none()
            && resolved_reference_local.is_none()
            && !is_internal_assignment_temp(name)
            && self
                .resolve_user_function_capture_hidden_name(name)
                .is_some();
        let reference_global_index = (scoped_target.is_none()
            && resolved_reference_local.is_none()
            && !reference_targets_capture)
            .then(|| self.resolve_global_binding_index(name))
            .flatten();
        let reference_targets_eval_local = scoped_target.is_none()
            && resolved_reference_local.is_none()
            && !reference_targets_capture
            && reference_global_index.is_none()
            && self.resolve_eval_local_function_hidden_name(name).is_some();
        let reference_implicit_global = (scoped_target.is_none()
            && resolved_reference_local.is_none()
            && !reference_targets_capture
            && reference_global_index.is_none()
            && !reference_targets_eval_local)
            .then(|| self.backend.implicit_global_binding(name))
            .flatten();
        let reference_is_unresolvable = scoped_target.is_none()
            && resolved_reference_local.is_none()
            && !reference_targets_capture
            && reference_global_index.is_none()
            && !reference_targets_eval_local
            && reference_implicit_global.is_none();
        self.emit_numeric_expression(value)?;
        if let Some(scope_object) = scoped_target {
            let value_local = self.allocate_temp_local();
            self.push_local_set(value_local);
            self.emit_scoped_property_store_from_local(&scope_object, name, value_local, value)?;
        } else {
            let value_local = self.allocate_temp_local();
            self.push_local_set(value_local);
            self.emit_store_identifier_value_local_with_reference_target(
                name,
                value,
                value_local,
                resolved_reference_local,
                reference_targets_capture,
                reference_global_index,
                reference_targets_eval_local,
                reference_implicit_global,
                reference_is_unresolvable,
            )?;
            self.push_local_get(value_local);
        }
        Ok(())
    }

    fn template_object_member_read_kind(
        &self,
        property: &Expression,
    ) -> Option<TemplateObjectMemberRead> {
        if let Some(index) = argument_index_from_expression(property) {
            return Some(TemplateObjectMemberRead::Index(index));
        }
        match property {
            Expression::String(name) if name == "length" => Some(TemplateObjectMemberRead::Length),
            Expression::String(name) if name == "raw" => Some(TemplateObjectMemberRead::RawArray),
            _ if Self::template_object_absent_static_own_property(property) => {
                Some(TemplateObjectMemberRead::AbsentFrozenOwnProperty)
            }
            _ => None,
        }
    }

    fn emit_template_object_member_value(
        &mut self,
        binding: &ArrayValueBinding,
        read_kind: TemplateObjectMemberRead,
    ) -> DirectResult<()> {
        match read_kind {
            TemplateObjectMemberRead::Index(index) => {
                if let Some(Some(value)) = binding.values.get(index as usize) {
                    self.emit_numeric_expression(value)?;
                } else {
                    self.push_i32_const(JS_UNDEFINED_TAG);
                }
            }
            TemplateObjectMemberRead::Length => {
                self.push_i32_const(binding.values.len() as i32);
            }
            TemplateObjectMemberRead::RawArray => {
                self.push_i32_const(JS_TYPEOF_OBJECT_TAG);
            }
            TemplateObjectMemberRead::AbsentFrozenOwnProperty => {
                self.push_i32_const(JS_UNDEFINED_TAG);
            }
        }
        Ok(())
    }

    fn emit_template_object_member_read_from_local(
        &mut self,
        object_value_local: u32,
        fallback_object: &Expression,
        property: &Expression,
    ) -> DirectResult<bool> {
        let Some(read_kind) = self.template_object_member_read_kind(property) else {
            return Ok(false);
        };
        let mut templates = self
            .backend
            .template_object_array_bindings
            .iter()
            .map(|(runtime_value, binding)| (*runtime_value, binding.clone()))
            .collect::<Vec<_>>();
        let trace_template_objects = std::env::var_os("AYY_TRACE_TEMPLATE_OBJECTS").is_some();
        if trace_template_objects {
            eprintln!(
                "template_member:property={property:?} entries={}",
                templates.len()
            );
        }
        if templates.is_empty() {
            return Ok(false);
        }
        templates.sort_by_key(|(runtime_value, _)| *runtime_value);

        fn emit_branch<'a>(
            compiler: &mut FunctionCompiler<'a>,
            object_value_local: u32,
            fallback_object: &Expression,
            property: &Expression,
            templates: &[(i32, ArrayValueBinding)],
            read_kind: TemplateObjectMemberRead,
            index: usize,
        ) -> DirectResult<()> {
            let Some((runtime_value, binding)) = templates.get(index) else {
                return compiler.emit_member_read_without_prelude(fallback_object, property);
            };
            compiler.push_local_get(object_value_local);
            compiler.push_i32_const(*runtime_value);
            compiler.push_binary_op(BinaryOp::Equal)?;
            compiler.state.emission.output.instructions.push(0x04);
            compiler.state.emission.output.instructions.push(I32_TYPE);
            compiler.push_control_frame();
            compiler.emit_template_object_member_value(binding, read_kind)?;
            compiler.state.emission.output.instructions.push(0x05);
            emit_branch(
                compiler,
                object_value_local,
                fallback_object,
                property,
                templates,
                read_kind,
                index.saturating_add(1),
            )?;
            compiler.state.emission.output.instructions.push(0x0b);
            compiler.pop_control_frame();
            Ok(())
        }

        emit_branch(
            self,
            object_value_local,
            fallback_object,
            property,
            &templates,
            read_kind,
            0,
        )?;
        Ok(true)
    }

    fn emit_template_object_raw_array_member_read_from_local(
        &mut self,
        object_value_local: u32,
    ) -> DirectResult<bool> {
        let mut templates = self
            .backend
            .template_object_raw_array_bindings
            .iter()
            .map(|(runtime_value, binding)| (*runtime_value, binding.clone()))
            .collect::<Vec<_>>();
        if templates.is_empty() {
            return Ok(false);
        }
        templates.sort_by_key(|(runtime_value, _)| *runtime_value);

        fn emit_branch<'a>(
            compiler: &mut FunctionCompiler<'a>,
            object_value_local: u32,
            templates: &[(i32, ArrayValueBinding)],
            index: usize,
        ) -> DirectResult<()> {
            let Some((runtime_value, binding)) = templates.get(index) else {
                compiler.push_i32_const(JS_TYPEOF_OBJECT_TAG);
                return Ok(());
            };
            compiler.push_local_get(object_value_local);
            compiler.push_i32_const(*runtime_value);
            compiler.push_binary_op(BinaryOp::Equal)?;
            compiler.state.emission.output.instructions.push(0x04);
            compiler.state.emission.output.instructions.push(I32_TYPE);
            compiler.push_control_frame();
            let array = Expression::Array(
                binding
                    .values
                    .iter()
                    .map(|value| {
                        ArrayElement::Expression(value.clone().unwrap_or(Expression::Undefined))
                    })
                    .collect(),
            );
            compiler.emit_numeric_expression(&array)?;
            compiler.state.emission.output.instructions.push(0x05);
            emit_branch(
                compiler,
                object_value_local,
                templates,
                index.saturating_add(1),
            )?;
            compiler.state.emission.output.instructions.push(0x0b);
            compiler.pop_control_frame();
            Ok(())
        }

        emit_branch(self, object_value_local, &templates, 0)?;
        Ok(true)
    }

    fn emit_template_object_raw_array_absent_member_read(
        &mut self,
        object: &Expression,
        property: &Expression,
    ) -> DirectResult<bool> {
        if !Self::template_object_absent_static_own_property(property) {
            return Ok(false);
        }
        let Expression::Member {
            object: base_object,
            property: raw_property,
        } = object
        else {
            return Ok(false);
        };
        if !matches!(raw_property.as_ref(), Expression::String(name) if name == "raw") {
            return Ok(false);
        }
        if !inline_summary_side_effect_free_expression(base_object) {
            return Ok(false);
        }

        let mut runtime_values = self
            .backend
            .template_object_raw_array_bindings
            .keys()
            .copied()
            .collect::<Vec<_>>();
        if runtime_values.is_empty() {
            return Ok(false);
        }
        runtime_values.sort_unstable();

        let base_object_local = self.allocate_temp_local();
        self.emit_numeric_expression(base_object)?;
        self.push_local_set(base_object_local);

        fn emit_branch<'a>(
            compiler: &mut FunctionCompiler<'a>,
            base_object_local: u32,
            object: &Expression,
            property: &Expression,
            runtime_values: &[i32],
            index: usize,
        ) -> DirectResult<()> {
            let Some(runtime_value) = runtime_values.get(index) else {
                return compiler.emit_member_read_without_prelude(object, property);
            };
            compiler.push_local_get(base_object_local);
            compiler.push_i32_const(*runtime_value);
            compiler.push_binary_op(BinaryOp::Equal)?;
            compiler.state.emission.output.instructions.push(0x04);
            compiler.state.emission.output.instructions.push(I32_TYPE);
            compiler.push_control_frame();
            compiler.push_i32_const(JS_UNDEFINED_TAG);
            compiler.state.emission.output.instructions.push(0x05);
            emit_branch(
                compiler,
                base_object_local,
                object,
                property,
                runtime_values,
                index.saturating_add(1),
            )?;
            compiler.state.emission.output.instructions.push(0x0b);
            compiler.pop_control_frame();
            Ok(())
        }

        emit_branch(
            self,
            base_object_local,
            object,
            property,
            &runtime_values,
            0,
        )?;
        Ok(true)
    }

    pub(in crate::backend::direct_wasm) fn emit_member_expression_value(
        &mut self,
        object: &Expression,
        property: &Expression,
    ) -> DirectResult<()> {
        let trace_member_reads = std::env::var_os("AYY_TRACE_MEMBER_READS").is_some();
        if trace_member_reads {
            eprintln!(
                "member_expr:start current_fn={:?} object={object:?} property={property:?}",
                self.current_function_name(),
            );
        }
        if std::env::var_os("AYY_TRACE_PRIVATE_MEMBER_LOOKUP").is_some()
            && matches!(property, Expression::String(name) if name.starts_with("__ayy$private$"))
        {
            eprintln!(
                "private_emit_member current_fn={:?} object={object:?} property={property:?}",
                self.current_function_name(),
            );
        }
        let object_is_internal_assignment_temp =
            matches!(object, Expression::Identifier(name) if is_internal_assignment_temp(name));
        if !object_is_internal_assignment_temp
            && self.emit_direct_iterator_step_member_read(object, property)?
        {
            if trace_member_reads {
                eprintln!("member_expr:direct_iterator object={object:?} property={property:?}");
            }
            return Ok(());
        }
        let original_member = Expression::Member {
            object: Box::new(object.clone()),
            property: Box::new(property.clone()),
        };
        if !object_is_internal_assignment_temp
            && let Some(value) =
                self.resolve_module_namespace_live_binding_member_value(object, property)
        {
            self.emit_numeric_expression(&value)?;
            return Ok(());
        }
        if !object_is_internal_assignment_temp
            && !matches!(property, Expression::Member { .. })
            && let Some(function_binding) =
                self.resolve_function_binding_from_expression(&original_member)
        {
            match function_binding {
                LocalFunctionBinding::User(function_name) => {
                    if let Some(runtime_value) = self.user_function_runtime_value(&function_name) {
                        if trace_member_reads {
                            eprintln!("member_expr:user_function_value member={original_member:?}");
                        }
                        self.push_i32_const(runtime_value);
                        return Ok(());
                    }
                }
                LocalFunctionBinding::Builtin(function_name) => {
                    if trace_member_reads {
                        eprintln!("member_expr:builtin_function_value member={original_member:?}");
                    }
                    self.push_i32_const(
                        builtin_function_runtime_value(&function_name)
                            .unwrap_or(JS_TYPEOF_FUNCTION_TAG),
                    );
                    return Ok(());
                }
            }
        }
        if !object_is_internal_assignment_temp
            && matches!(property, Expression::String(property_name) if property_name == "prototype")
        {
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
                return Ok(());
            }
        }
        let object_value_local = self.allocate_temp_local();
        self.emit_numeric_expression(object)?;
        self.push_local_set(object_value_local);
        self.emit_throw_if_member_base_nullish_local(object_value_local)?;
        self.push_local_get(object_value_local);
        if trace_member_reads {
            eprintln!("member_expr:object_done object={object:?} property={property:?}");
        }
        self.state.emission.output.instructions.push(0x1a);
        let resolved_property = self.emit_property_key_expression_effects(property)?;
        if trace_member_reads {
            eprintln!(
                "member_expr:property_done object={object:?} property={property:?} resolved={resolved_property:?}"
            );
        }
        if let Some(resolved_property) = resolved_property.as_ref()
            && let Expression::Identifier(property_name) = property
            && is_internal_target_property_temp(property_name)
        {
            let property_key_local = self.allocate_temp_local();
            self.emit_numeric_expression(resolved_property)?;
            self.push_local_set(property_key_local);
            self.emit_store_identifier_value_local(
                property_name,
                resolved_property,
                property_key_local,
            )?;
        }
        let effective_property = resolved_property.as_ref().unwrap_or(property);
        let read_object = self
            .private_member_read_receiver_after_evaluation(object, effective_property)
            .unwrap_or_else(|| object.clone());
        if matches!(effective_property, Expression::String(name) if name == "raw")
            && self.emit_template_object_raw_array_member_read_from_local(object_value_local)?
        {
            if trace_member_reads {
                eprintln!("member_expr:template_object_raw object={read_object:?}");
            }
            return Ok(());
        }
        if self
            .emit_template_object_raw_array_absent_member_read(&read_object, effective_property)?
        {
            if trace_member_reads {
                eprintln!(
                    "member_expr:template_object_raw_absent object={read_object:?} property={effective_property:?}"
                );
            }
            return Ok(());
        }
        if self.emit_template_object_member_read_from_local(
            object_value_local,
            &read_object,
            effective_property,
        )? {
            if trace_member_reads {
                eprintln!(
                    "member_expr:template_object object={read_object:?} property={effective_property:?}"
                );
            }
            return Ok(());
        }
        let result = self.emit_member_read_without_prelude(&read_object, effective_property);
        if trace_member_reads {
            eprintln!(
                "member_expr:done object={read_object:?} property={effective_property:?} ok={}",
                result.is_ok()
            );
        }
        result
    }

    pub(in crate::backend::direct_wasm) fn resolve_module_namespace_live_binding_member_value(
        &self,
        object: &Expression,
        property: &Expression,
    ) -> Option<Expression> {
        let object_binding = self.direct_module_namespace_object_binding(object)?;
        let namespace_marker = object_binding_lookup_value(
            &object_binding,
            &Expression::String("__ayy$module$namespace".to_string()),
        )?;
        if !matches!(namespace_marker, Expression::Bool(true)) {
            return None;
        }
        let property = self
            .resolve_property_key_expression(property)
            .unwrap_or_else(|| self.materialize_static_expression(property));
        let value = object_binding_lookup_value(&object_binding, &property)?.clone();
        match &value {
            Expression::Identifier(name)
                if name.starts_with("__ayy_capture_binding__")
                    && self
                        .implicit_global_binding(name)
                        .or_else(|| self.hidden_implicit_global_binding(name))
                        .is_some() =>
            {
                Some(value)
            }
            _ => {
                let module_index = object_binding_lookup_value(
                    &object_binding,
                    &Expression::String("__ayy$module$namespace$moduleIndex".to_string()),
                )
                .and_then(|value| match value {
                    Expression::Number(index)
                        if index.is_finite() && *index >= 0.0 && index.fract() == 0.0 =>
                    {
                        Some(*index as usize)
                    }
                    _ => None,
                })?;
                self.resolve_static_dynamic_import_namespace_live_binding_member_value(
                    module_index,
                    &property,
                )
            }
        }
    }

    fn direct_module_namespace_object_binding(
        &self,
        object: &Expression,
    ) -> Option<ObjectValueBinding> {
        let Expression::Identifier(name) = object else {
            return None;
        };
        let resolved_local_name = self
            .resolve_current_local_binding(name)
            .map(|(resolved_name, _)| resolved_name);
        let mut candidate_names = Vec::new();
        if let Some(resolved_name) = resolved_local_name.as_ref() {
            candidate_names.push(resolved_name.as_str());
        }
        candidate_names.push(name.as_str());
        candidate_names.sort_unstable();
        candidate_names.dedup();

        for candidate_name in &candidate_names {
            if let Some(binding) = self
                .state
                .speculation
                .static_semantics
                .local_object_binding(candidate_name)
                .filter(|binding| Self::object_binding_has_module_namespace_marker(binding))
            {
                return Some(binding.clone());
            }
        }
        if resolved_local_name.is_some() {
            return None;
        }
        for candidate_name in candidate_names {
            if let Some(binding) = self
                .global_object_binding(candidate_name)
                .filter(|binding| Self::object_binding_has_module_namespace_marker(binding))
            {
                return Some(binding.clone());
            }
        }
        None
    }

    fn private_member_read_receiver_after_evaluation(
        &self,
        object: &Expression,
        property: &Expression,
    ) -> Option<Expression> {
        if !self.is_private_member_read_property(property) {
            return None;
        }
        let Expression::Call { callee, arguments } = object else {
            return None;
        };
        if !arguments.is_empty() {
            return None;
        }
        let LocalFunctionBinding::User(function_name) =
            self.resolve_function_binding_from_expression(callee)?
        else {
            return None;
        };
        let user_function = self.user_function(&function_name)?;
        if !user_function.lexical_this {
            return None;
        }
        let function = self.resolve_registered_function_declaration(&function_name)?;
        matches!(
            function.body.as_slice(),
            [Statement::Return(Expression::This)]
        )
        .then_some(Expression::This)
    }

    pub(in crate::backend::direct_wasm) fn emit_throw_if_member_base_nullish_local(
        &mut self,
        object_value_local: u32,
    ) -> DirectResult<()> {
        self.push_local_get(object_value_local);
        self.push_i32_const(JS_NULL_TAG);
        self.push_binary_op(BinaryOp::Equal)?;

        self.push_local_get(object_value_local);
        self.push_i32_const(JS_UNDEFINED_TAG);
        self.push_binary_op(BinaryOp::Equal)?;

        self.state.emission.output.instructions.push(0x72);
        self.state.emission.output.instructions.push(0x04);
        self.state
            .emission
            .output
            .instructions
            .push(EMPTY_BLOCK_TYPE);
        self.push_control_frame();
        self.emit_named_error_throw("TypeError")?;
        self.state.emission.output.instructions.push(0x0b);
        self.pop_control_frame();
        Ok(())
    }

    pub(in crate::backend::direct_wasm) fn emit_super_member_expression_value(
        &mut self,
        property: &Expression,
    ) -> DirectResult<()> {
        let resolved_property = self.resolve_property_key_expression_with_coercion(property);
        self.emit_numeric_expression(property)?;
        self.state.emission.output.instructions.push(0x1a);
        if let Some(coercion) = resolved_property
            .as_ref()
            .and_then(|resolved| resolved.coercion.clone())
            .or_else(|| self.resolve_property_key_coercion_binding(property))
        {
            self.emit_super_property_key_coercion_effect(&coercion)?;
        }
        let property = resolved_property
            .as_ref()
            .map(|resolved| &resolved.key)
            .unwrap_or(property);

        if self.current_function_is_derived_constructor() {
            self.push_global_get(CURRENT_THIS_GLOBAL_INDEX);
            self.push_i32_const(JS_UNDEFINED_TAG);
            self.push_binary_op(BinaryOp::Equal)?;
            self.state.emission.output.instructions.push(0x04);
            self.state
                .emission
                .output
                .instructions
                .push(EMPTY_BLOCK_TYPE);
            self.push_control_frame();
            self.emit_named_error_throw("ReferenceError")?;
            self.state.emission.output.instructions.push(0x0b);
            self.pop_control_frame();
        }

        let super_base =
            self.resolve_super_base_expression_with_context(self.current_function_name());
        if self.super_base_is_statically_nullish(super_base.as_ref()) {
            self.emit_named_error_throw("TypeError")?;
            return Ok(());
        }
        if let Some(function_binding) = self.resolve_super_function_binding(property) {
            match function_binding {
                LocalFunctionBinding::User(function_name) => {
                    if let Some(user_function) = self.user_function(&function_name) {
                        self.push_i32_const(user_function_runtime_value(user_function));
                    } else {
                        self.push_i32_const(JS_UNDEFINED_TAG);
                    }
                }
                LocalFunctionBinding::Builtin(_) => {
                    self.push_i32_const(JS_TYPEOF_FUNCTION_TAG);
                }
            }
            return Ok(());
        }
        if let Some(function_binding) = self.resolve_super_getter_binding(property) {
            match function_binding {
                LocalFunctionBinding::User(function_name) => {
                    let capture_slots = self
                        .resolve_super_base_expression_with_context(self.current_function_name())
                        .and_then(|base| {
                            self.resolve_member_function_capture_slots(&base, property)
                        });
                    self.emit_member_getter_call_with_bound_this(
                        &function_name,
                        &Expression::This,
                        capture_slots.as_ref(),
                    )?;
                }
                LocalFunctionBinding::Builtin(function_name) => {
                    let callee = Expression::Identifier(function_name);
                    if !self.emit_arguments_slot_accessor_call(&callee, &[], 0, Some(&[]))? {
                        self.push_i32_const(JS_UNDEFINED_TAG);
                    }
                }
            }
            return Ok(());
        }
        if let Some(value) = self.resolve_super_value_expression(property) {
            self.emit_numeric_expression(&value)?;
            return Ok(());
        }
        if self.emit_super_member_read_via_runtime_prototype_binding(property)? {
            return Ok(());
        }
        self.push_i32_const(JS_UNDEFINED_TAG);
        Ok(())
    }

    pub(in crate::backend::direct_wasm) fn emit_this_expression_value(
        &mut self,
    ) -> DirectResult<()> {
        if self.current_function_is_derived_constructor() {
            self.push_global_get(CURRENT_THIS_GLOBAL_INDEX);
            self.push_i32_const(JS_UNDEFINED_TAG);
            self.push_binary_op(BinaryOp::Equal)?;
            self.state.emission.output.instructions.push(0x04);
            self.state.emission.output.instructions.push(I32_TYPE);
            self.push_control_frame();
            self.emit_named_error_throw("ReferenceError")?;
            self.push_i32_const(JS_UNDEFINED_TAG);
            self.state.emission.output.instructions.push(0x05);
            self.push_global_get(CURRENT_THIS_GLOBAL_INDEX);
            self.state.emission.output.instructions.push(0x0b);
            self.pop_control_frame();
            return Ok(());
        }
        if self
            .current_user_function()
            .is_some_and(|function| function.lexical_this)
            && let Some(hidden_name) = self.resolve_user_function_capture_hidden_name("this")
        {
            let binding = self
                .implicit_global_binding(&hidden_name)
                .unwrap_or_else(|| self.ensure_implicit_global_binding(&hidden_name));
            self.push_global_get(binding.value_index);
            return Ok(());
        }
        self.push_global_get(CURRENT_THIS_GLOBAL_INDEX);
        Ok(())
    }
}
