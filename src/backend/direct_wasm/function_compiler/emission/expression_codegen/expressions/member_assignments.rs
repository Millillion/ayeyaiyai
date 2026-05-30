use super::*;

mod arguments_objects;
mod named_objects;
mod setter_calls;
mod super_members;

fn assign_member_expression_references_internal_iterator_step(expression: &Expression) -> bool {
    match expression {
        Expression::Identifier(name) => {
            name.starts_with("__ayy_array_step_") || name.starts_with("__ayy_for_of_step_")
        }
        Expression::Array(elements) => elements.iter().any(|element| match element {
            ArrayElement::Expression(value) | ArrayElement::Spread(value) => {
                assign_member_expression_references_internal_iterator_step(value)
            }
        }),
        Expression::Object(entries) => entries.iter().any(|entry| match entry {
            ObjectEntry::Data { key, value } => {
                assign_member_expression_references_internal_iterator_step(key)
                    || assign_member_expression_references_internal_iterator_step(value)
            }
            ObjectEntry::Getter { key, getter } => {
                assign_member_expression_references_internal_iterator_step(key)
                    || assign_member_expression_references_internal_iterator_step(getter)
            }
            ObjectEntry::Setter { key, setter } => {
                assign_member_expression_references_internal_iterator_step(key)
                    || assign_member_expression_references_internal_iterator_step(setter)
            }
            ObjectEntry::Spread(value) => {
                assign_member_expression_references_internal_iterator_step(value)
            }
        }),
        Expression::Binary { left, right, .. } => {
            assign_member_expression_references_internal_iterator_step(left)
                || assign_member_expression_references_internal_iterator_step(right)
        }
        Expression::Conditional {
            condition,
            then_expression,
            else_expression,
        } => {
            assign_member_expression_references_internal_iterator_step(condition)
                || assign_member_expression_references_internal_iterator_step(then_expression)
                || assign_member_expression_references_internal_iterator_step(else_expression)
        }
        Expression::Member { object, property } => {
            assign_member_expression_references_internal_iterator_step(object)
                || assign_member_expression_references_internal_iterator_step(property)
        }
        Expression::Unary { expression, .. }
        | Expression::Await(expression)
        | Expression::EnumerateKeys(expression)
        | Expression::GetIterator(expression)
        | Expression::IteratorClose(expression) => {
            assign_member_expression_references_internal_iterator_step(expression)
        }
        Expression::Assign { value, .. } => {
            assign_member_expression_references_internal_iterator_step(value)
        }
        Expression::AssignMember {
            object,
            property,
            value,
        } => {
            assign_member_expression_references_internal_iterator_step(object)
                || assign_member_expression_references_internal_iterator_step(property)
                || assign_member_expression_references_internal_iterator_step(value)
        }
        Expression::AssignSuperMember { property, value } => {
            assign_member_expression_references_internal_iterator_step(property)
                || assign_member_expression_references_internal_iterator_step(value)
        }
        Expression::Call { callee, arguments }
        | Expression::New { callee, arguments }
        | Expression::SuperCall { callee, arguments } => {
            assign_member_expression_references_internal_iterator_step(callee)
                || arguments.iter().any(|argument| match argument {
                    CallArgument::Expression(value) | CallArgument::Spread(value) => {
                        assign_member_expression_references_internal_iterator_step(value)
                    }
                })
        }
        Expression::SuperMember { property } => {
            assign_member_expression_references_internal_iterator_step(property)
        }
        _ => false,
    }
}

impl<'a> FunctionCompiler<'a> {
    fn template_object_raw_member_base<'b>(object: &'b Expression) -> Option<&'b Expression> {
        let Expression::Member {
            object: base_object,
            property,
        } = object
        else {
            return None;
        };
        matches!(property.as_ref(), Expression::String(name) if name == "raw")
            .then_some(base_object.as_ref())
    }

    fn emit_template_object_frozen_absent_member_assignment(
        &mut self,
        object: &Expression,
        property: &Expression,
        value: &Expression,
    ) -> DirectResult<bool> {
        if !Self::template_object_absent_static_own_property(property) {
            return Ok(false);
        }

        let raw_base_object = Self::template_object_raw_member_base(object);
        if raw_base_object.is_some_and(|base| !inline_summary_side_effect_free_expression(base)) {
            return Ok(false);
        }

        let mut runtime_values = self
            .backend
            .template_object_array_bindings
            .keys()
            .copied()
            .collect::<Vec<_>>();
        if runtime_values.is_empty() {
            return Ok(false);
        }
        runtime_values.sort_unstable();

        let object_kind = self.infer_value_kind(object);
        let object_value_local = self.allocate_temp_local();
        self.emit_numeric_expression(object)?;
        self.push_local_set(object_value_local);
        self.emit_throw_if_member_base_nullish_local(object_value_local)?;
        self.emit_property_key_expression_effects(property)?;

        let value_local = self.allocate_temp_local();
        self.emit_numeric_expression(value)?;
        self.push_local_set(value_local);

        let comparison_local = if let Some(raw_base_object) = raw_base_object {
            let comparison_local = self.allocate_temp_local();
            self.emit_numeric_expression(raw_base_object)?;
            self.push_local_set(comparison_local);
            comparison_local
        } else {
            object_value_local
        };
        let strict_mode = self.state.speculation.execution_context.strict_mode;

        fn emit_branch<'a>(
            compiler: &mut FunctionCompiler<'a>,
            comparison_local: u32,
            value_local: u32,
            runtime_values: &[i32],
            object_kind: Option<StaticValueKind>,
            strict_mode: bool,
            index: usize,
        ) -> DirectResult<()> {
            let Some(runtime_value) = runtime_values.get(index) else {
                if matches!(
                    object_kind,
                    Some(StaticValueKind::Null | StaticValueKind::Undefined)
                ) {
                    compiler.emit_named_error_throw("TypeError")?;
                } else {
                    compiler.push_local_get(value_local);
                }
                return Ok(());
            };

            compiler.push_local_get(comparison_local);
            compiler.push_i32_const(*runtime_value);
            compiler.push_binary_op(BinaryOp::Equal)?;
            compiler.state.emission.output.instructions.push(0x04);
            compiler.state.emission.output.instructions.push(I32_TYPE);
            compiler.push_control_frame();
            if strict_mode {
                compiler.emit_named_error_throw("TypeError")?;
            } else {
                compiler.push_local_get(value_local);
            }
            compiler.state.emission.output.instructions.push(0x05);
            emit_branch(
                compiler,
                comparison_local,
                value_local,
                runtime_values,
                object_kind,
                strict_mode,
                index.saturating_add(1),
            )?;
            compiler.state.emission.output.instructions.push(0x0b);
            compiler.pop_control_frame();
            Ok(())
        }

        emit_branch(
            self,
            comparison_local,
            value_local,
            &runtime_values,
            object_kind,
            strict_mode,
            0,
        )?;
        Ok(true)
    }

    fn expression_is_static_function_constructor_global_this_call(
        &self,
        expression: &Expression,
    ) -> bool {
        let Expression::Call { callee, arguments } = expression else {
            return false;
        };
        if !arguments.is_empty() {
            return false;
        }
        let function_name = match callee.as_ref() {
            Expression::Identifier(function_name) => function_name.clone(),
            _ => {
                let Some(Expression::Identifier(function_name)) =
                    self.resolve_bound_alias_expression(callee)
                else {
                    return false;
                };
                function_name
            }
        };
        if !function_name.starts_with("__ayy_function_ctor_") {
            return false;
        }
        self.resolve_registered_function_declaration(&function_name)
            .is_some_and(|function| {
                matches!(
                    function.body.as_slice(),
                    [Statement::Return(Expression::This)]
                )
            })
    }

    pub(in crate::backend::direct_wasm) fn resolve_static_global_object_alias_expression(
        &self,
        expression: &Expression,
    ) -> Option<Expression> {
        if matches!(
            expression,
            Expression::Member { object, .. }
                if matches!(
                    object.as_ref(),
                    Expression::Identifier(name)
                        if name.starts_with("__ayy_module_dep_")
                            || name.starts_with("__ayy_module_namespace_")
                )
        ) {
            return None;
        }

        if self.expression_is_static_function_constructor_global_this_call(expression) {
            return Some(Expression::Identifier("globalThis".to_string()));
        }

        let materialized = self.materialize_static_expression(expression);
        match materialized {
            Expression::Identifier(name) if name == "globalThis" => {
                Some(Expression::Identifier(name))
            }
            Expression::This
                if self.expression_is_static_function_constructor_global_this_call(expression) =>
            {
                Some(Expression::Identifier("globalThis".to_string()))
            }
            _ => None,
        }
    }

    fn static_function_constructor_global_this_update_value(
        &self,
        property: &Expression,
        value: &Expression,
    ) -> Option<Expression> {
        let Expression::Binary {
            op: BinaryOp::Add,
            left,
            right,
        } = value
        else {
            return None;
        };
        let Expression::Member {
            object: left_object,
            property: left_property,
        } = left.as_ref()
        else {
            return None;
        };
        if !self.expression_is_static_function_constructor_global_this_call(left_object) {
            return None;
        }
        let left_property = self.canonical_object_property_expression(left_property);
        if !static_expression_matches(&left_property, property) {
            return None;
        }

        let global_this = Expression::Identifier("globalThis".to_string());
        let previous_value = self
            .runtime_object_property_shadow_binding_name_for_expression(&global_this, property)
            .and_then(|shadow_binding_name| self.global_value_binding(&shadow_binding_name))
            .or_else(|| {
                self.backend
                    .global_object_binding("globalThis")
                    .and_then(|binding| object_binding_lookup_value(binding, property))
            })?;
        let resolved = self.resolve_static_string_addition_value_with_context(
            previous_value,
            right,
            self.current_function_name(),
        );
        if std::env::var_os("AYY_TRACE_MEMBER_ASSIGNMENT").is_some() {
            eprintln!(
                "function_constructor_global_this_update property={property:?} previous={previous_value:?} right={right:?} resolved={resolved:?}"
            );
        }
        resolved.map(Expression::String)
    }

    fn emit_static_function_constructor_global_this_member_assignment(
        &mut self,
        object: &Expression,
        property: &Expression,
        value: &Expression,
    ) -> DirectResult<bool> {
        if !self.expression_is_static_function_constructor_global_this_call(object) {
            return Ok(false);
        }
        let materialized_property = self.canonical_object_property_expression(property);
        let Expression::String(property_name) = &materialized_property else {
            return Ok(false);
        };

        let global_this = Expression::Identifier("globalThis".to_string());
        if self.global_object_binding("globalThis").is_none() {
            self.backend
                .sync_global_object_binding("globalThis", Some(empty_object_value_binding()));
            self.backend
                .set_global_binding_kind("globalThis", StaticValueKind::Object);
        }

        let emitted_value = self
            .static_function_constructor_global_this_update_value(&materialized_property, value)
            .unwrap_or_else(|| self.member_assignment_emission_value(value));
        if std::env::var_os("AYY_TRACE_MEMBER_ASSIGNMENT").is_some() {
            eprintln!(
                "function_constructor_global_this_emit property={materialized_property:?} value={value:?} emitted={emitted_value:?}"
            );
        }
        let value_local = self.allocate_temp_local();
        if self
            .resolve_property_key_coercion_binding(property)
            .is_some()
        {
            self.emit_property_key_expression_effects(property)?;
        }
        self.emit_numeric_expression(&emitted_value)?;
        self.push_local_set(value_local);
        self.emit_scoped_property_store_from_local(
            &global_this,
            property_name,
            value_local,
            &emitted_value,
        )?;
        Ok(true)
    }

    fn primitive_assignment_constructor_name(&self, object: &Expression) -> Option<&'static str> {
        match object {
            Expression::Number(_) => Some("Number"),
            Expression::String(_) => Some("String"),
            Expression::Bool(_) => Some("Boolean"),
            Expression::BigInt(_) => Some("BigInt"),
            Expression::Identifier(name)
                if self.lookup_identifier_kind(name) == Some(StaticValueKind::Symbol) =>
            {
                Some("Symbol")
            }
            Expression::Call { callee, arguments }
                if arguments.is_empty()
                    && matches!(callee.as_ref(), Expression::Identifier(name) if name == "Symbol") =>
            {
                Some("Symbol")
            }
            _ => {
                let materialized = self.materialize_static_expression(object);
                if static_expression_matches(&materialized, object) {
                    None
                } else {
                    self.primitive_assignment_constructor_name(&materialized)
                }
            }
        }
    }

    fn primitive_assignment_prototype_expression(
        &self,
        object: &Expression,
        realm_id: Option<u32>,
    ) -> Option<Expression> {
        let constructor_name = self.primitive_assignment_constructor_name(object)?;
        if let Some(realm_id) = realm_id {
            return Some(Expression::Member {
                object: Box::new(Expression::Member {
                    object: Box::new(Expression::Identifier(test262_realm_global_identifier(
                        realm_id,
                    ))),
                    property: Box::new(Expression::String(constructor_name.to_string())),
                }),
                property: Box::new(Expression::String("prototype".to_string())),
            });
        }
        Some(Self::prototype_member_expression(constructor_name))
    }

    fn primitive_assignment_base_is_effect_free(&self, object: &Expression) -> bool {
        if inline_summary_side_effect_free_expression(object) {
            return true;
        }

        match object {
            Expression::Call { callee, arguments }
                if arguments.is_empty()
                    && matches!(callee.as_ref(), Expression::Identifier(name) if name == "Symbol" && self.is_unshadowed_builtin_identifier(name)) =>
            {
                true
            }
            _ => {
                let materialized = self.materialize_static_expression(object);
                !static_expression_matches(&materialized, object)
                    && self.primitive_assignment_base_is_effect_free(&materialized)
            }
        }
    }

    fn resolve_primitive_assignment_proxy_binding(
        &self,
        object: &Expression,
        realm_id: Option<u32>,
    ) -> Option<ProxyValueBinding> {
        let mut prototype = self.primitive_assignment_prototype_expression(object, realm_id)?;
        for _ in 0..32 {
            if let Some(proxy_binding) = self.resolve_proxy_binding_from_expression(&prototype) {
                return Some(proxy_binding);
            }
            let materialized_prototype = self.materialize_static_expression(&prototype);
            if !static_expression_matches(&materialized_prototype, &prototype)
                && let Some(proxy_binding) =
                    self.resolve_proxy_binding_from_expression(&materialized_prototype)
            {
                return Some(proxy_binding);
            }
            let Some(next_prototype) = self
                .resolve_static_object_prototype_expression(&materialized_prototype)
                .or_else(|| self.resolve_static_object_prototype_expression(&prototype))
            else {
                return None;
            };
            if static_expression_matches(&next_prototype, &prototype)
                || static_expression_matches(&next_prototype, &materialized_prototype)
                || matches!(next_prototype, Expression::Null)
            {
                return None;
            }
            prototype = next_prototype;
        }
        None
    }

    pub(in crate::backend::direct_wasm) fn emit_primitive_prototype_proxy_set_assignment(
        &mut self,
        object: &Expression,
        property: &Expression,
        value: &Expression,
        realm_id: Option<u32>,
    ) -> DirectResult<bool> {
        let Some(proxy_binding) = self.resolve_primitive_assignment_proxy_binding(object, realm_id)
        else {
            return Ok(false);
        };
        let Some(set_binding) = proxy_binding.set_binding.clone() else {
            return Ok(false);
        };
        let property = self.canonical_object_property_expression(property);

        if !self.primitive_assignment_base_is_effect_free(object) {
            self.emit_numeric_expression(object)?;
            self.state.emission.output.instructions.push(0x1a);
        }
        self.emit_numeric_expression(&property)?;
        self.state.emission.output.instructions.push(0x1a);

        let emitted_value = self.member_assignment_emission_value(value);
        let value_local = self.allocate_temp_local();
        self.emit_numeric_expression(&emitted_value)?;
        self.push_local_set(value_local);

        let value_arg_name = self.allocate_named_hidden_local(
            "primitive_proxy_set_value",
            self.infer_value_kind(&emitted_value)
                .unwrap_or(StaticValueKind::Unknown),
        );
        let value_arg_local = self
            .state
            .runtime
            .locals
            .get(&value_arg_name)
            .copied()
            .expect("fresh primitive proxy set value hidden local must exist");
        self.push_local_get(value_local);
        self.push_local_set(value_arg_local);

        let arguments = [
            proxy_binding.target.clone(),
            property,
            Expression::Identifier(value_arg_name),
            object.clone(),
        ];
        self.emit_function_binding_effect_statements_with_arguments(&set_binding, &arguments)?;
        self.push_local_get(value_local);
        Ok(true)
    }

    fn expression_aliases_named_member_property(
        &self,
        expression: &Expression,
        object_name: &str,
        property_name: &str,
        depth: usize,
    ) -> bool {
        if depth > 8 {
            return false;
        }

        match expression {
            Expression::Identifier(name) => {
                let resolved_name = self
                    .resolve_current_local_binding(name)
                    .map(|(resolved_name, _)| resolved_name);
                let value = resolved_name
                    .as_deref()
                    .and_then(|resolved_name| {
                        self.state
                            .speculation
                            .static_semantics
                            .local_value_binding(resolved_name)
                    })
                    .or_else(|| {
                        self.state
                            .speculation
                            .static_semantics
                            .local_value_binding(name)
                    })
                    .or_else(|| self.global_value_binding(name));
                value.is_some_and(|value| {
                    !static_expression_matches(value, expression)
                        && self.expression_aliases_named_member_property(
                            value,
                            object_name,
                            property_name,
                            depth + 1,
                        )
                })
            }
            Expression::Member { object, property } => {
                let object_matches = match object.as_ref() {
                    Expression::Identifier(name) => {
                        let name_source = scoped_binding_source_name(name).unwrap_or(name);
                        let object_source =
                            scoped_binding_source_name(object_name).unwrap_or(object_name);
                        name_source == object_source
                    }
                    _ => false,
                };
                let property = self.canonical_object_property_expression(property);
                object_matches
                    && static_property_name_from_expression(&property).as_deref()
                        == Some(property_name)
            }
            Expression::Assign { value, .. }
            | Expression::AssignMember { value, .. }
            | Expression::AssignSuperMember { value, .. } => self
                .expression_aliases_named_member_property(
                    value,
                    object_name,
                    property_name,
                    depth + 1,
                ),
            Expression::Sequence(expressions) => expressions.last().is_some_and(|expression| {
                self.expression_aliases_named_member_property(
                    expression,
                    object_name,
                    property_name,
                    depth + 1,
                )
            }),
            _ => false,
        }
    }

    pub(in crate::backend::direct_wasm) fn emit_assign_member_expression(
        &mut self,
        object: &Expression,
        property: &Expression,
        value: &Expression,
    ) -> DirectResult<()> {
        let trace_member_assignment = std::env::var_os("AYY_TRACE_MEMBER_ASSIGNMENT").is_some();
        if trace_member_assignment {
            eprintln!(
                "member_assignment:start object={object:?} property={property:?} value={value:?}"
            );
        }
        let value_references_internal_iterator_step =
            assign_member_expression_references_internal_iterator_step(value);
        if self.is_array_prototype_symbol_iterator_member(object, property) {
            self.emit_array_prototype_symbol_iterator_deleted_marker(false)?;
        }

        if trace_member_assignment {
            eprintln!("member_assignment:arguments_or_restricted:start");
        }
        if self.emit_arguments_or_restricted_member_assignment(object, property, value)? {
            if trace_member_assignment {
                eprintln!("member_assignment:arguments_or_restricted:hit");
            }
            return Ok(());
        }
        if trace_member_assignment {
            eprintln!("member_assignment:arguments_or_restricted:done");
        }

        if let Expression::Identifier(name) = object
            && matches!(property, Expression::String(property_name) if property_name == "prototype")
            && !self.expression_aliases_named_member_property(value, name, "prototype", 0)
        {
            self.update_prototype_object_binding(name, value);
        }

        if self
            .state
            .speculation
            .execution_context
            .private_field_initializer_block
        {
            let materialized_property = self.canonical_object_property_expression(property);
            let private_brand_marker_assignment = self
                .current_private_brand_binding_name()
                .is_some_and(|brand_name| {
                    matches!(value, Expression::Identifier(value_name) if value_name == &brand_name)
                });
            if private_brand_marker_assignment
                && is_private_property_name_expression(&materialized_property)
            {
                let initializer_owner_name = match object {
                    Expression::Identifier(name) => Some(name.as_str()),
                    Expression::This => Some("this"),
                    _ => None,
                };
                if let Some(name) = initializer_owner_name
                    && self.emit_private_field_initializer_add(
                        name,
                        object,
                        &materialized_property,
                        value,
                    )?
                {
                    if trace_member_assignment {
                        eprintln!("member_assignment:private_brand_initializer:hit");
                    }
                    return Ok(());
                }
            }
        }

        if trace_member_assignment {
            eprintln!("member_assignment:setter:start");
        }
        if self.emit_setter_member_assignment(object, property, value)? {
            if trace_member_assignment {
                eprintln!("member_assignment:setter:hit");
            }
            return Ok(());
        }
        if trace_member_assignment {
            eprintln!("member_assignment:setter:done");
        }

        if trace_member_assignment {
            eprintln!("member_assignment:named:start");
        }
        if self.emit_named_object_member_assignment(object, property, value)? {
            if trace_member_assignment {
                eprintln!("member_assignment:named:hit");
            }
            return Ok(());
        }
        if trace_member_assignment {
            eprintln!("member_assignment:named:done");
        }

        if self.emit_static_function_constructor_global_this_member_assignment(
            object, property, value,
        )? {
            if trace_member_assignment {
                eprintln!("member_assignment:function_constructor_global_this:hit");
            }
            return Ok(());
        }

        if let Some(global_alias) = self.resolve_static_global_object_alias_expression(object)
            && !static_expression_matches(&global_alias, object)
        {
            if let Expression::Identifier(name) = &global_alias
                && name == "globalThis"
                && self.global_object_binding(name).is_none()
            {
                self.backend
                    .sync_global_object_binding(name, Some(empty_object_value_binding()));
                self.backend
                    .set_global_binding_kind(name, StaticValueKind::Object);
            }
            if self.emit_named_object_member_assignment(&global_alias, property, value)? {
                return Ok(());
            }
        }

        if trace_member_assignment {
            eprintln!("member_assignment:canonical_property:start");
        }
        let materialized_property = self.canonical_object_property_expression(property);
        if trace_member_assignment {
            eprintln!(
                "member_assignment:canonical_property:done property={materialized_property:?}"
            );
        }
        if self.emit_primitive_prototype_proxy_set_assignment(
            object,
            &materialized_property,
            value,
            None,
        )? {
            if trace_member_assignment {
                eprintln!("member_assignment:primitive_proxy_set:hit");
            }
            return Ok(());
        }
        if !value_references_internal_iterator_step
            && let Expression::String(property_name) = &materialized_property
            && self
                .runtime_object_property_shadow_binding_name_for_expression(
                    object,
                    &materialized_property,
                )
                .is_some()
        {
            let emitted_value = self.member_assignment_emission_value(value);
            let value_local = self.allocate_temp_local();
            if self
                .resolve_property_key_coercion_binding(property)
                .is_some()
            {
                self.emit_property_key_expression_effects(property)?;
            }
            self.emit_numeric_expression(&emitted_value)?;
            self.push_local_set(value_local);
            self.emit_scoped_property_store_from_local(
                object,
                property_name,
                value_local,
                &emitted_value,
            )?;
            return Ok(());
        }

        if inline_summary_side_effect_free_expression(object) {
            if trace_member_assignment {
                eprintln!("member_assignment:materialized_object:start");
            }
            let materialized_object = self.materialize_static_expression(object);
            if trace_member_assignment {
                eprintln!(
                    "member_assignment:materialized_object:done object={materialized_object:?}"
                );
            }
            if !static_expression_matches(&materialized_object, object)
                && self.emit_named_object_member_assignment(
                    &materialized_object,
                    property,
                    value,
                )?
            {
                return Ok(());
            }
            if !static_expression_matches(&materialized_object, object)
                && self.emit_materialized_module_capture_member_assignment(
                    object,
                    &materialized_object,
                    property,
                    value,
                )?
            {
                return Ok(());
            }
        }

        if self.emit_template_object_frozen_absent_member_assignment(object, property, value)? {
            if trace_member_assignment {
                eprintln!("member_assignment:template_object_frozen_absent:hit");
            }
            return Ok(());
        }

        if trace_member_assignment {
            eprintln!("member_assignment:fallback:object");
        }
        let object_kind = self.infer_value_kind(object);
        self.emit_numeric_expression(object)?;
        self.state.emission.output.instructions.push(0x1a);
        if trace_member_assignment {
            eprintln!("member_assignment:fallback:property");
        }
        self.emit_numeric_expression(property)?;
        self.state.emission.output.instructions.push(0x1a);
        if trace_member_assignment {
            eprintln!("member_assignment:fallback:value");
        }
        self.emit_numeric_expression(value)?;
        self.state.emission.output.instructions.push(0x1a);
        if matches!(
            object_kind,
            Some(StaticValueKind::Null | StaticValueKind::Undefined)
        ) {
            self.emit_named_error_throw("TypeError")?;
            return Ok(());
        }
        self.push_i32_const(JS_TYPEOF_OBJECT_TAG);
        if trace_member_assignment {
            eprintln!("member_assignment:done");
        }
        Ok(())
    }
}
