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
        let materialized_target_object = if let Expression::Identifier(name) = object
            && name.starts_with("__ayy_target_object_")
        {
            let candidate = self.materialize_static_expression(object);
            matches!(candidate, Expression::Identifier(_) | Expression::This).then_some(candidate)
        } else {
            None
        };
        let object = materialized_target_object.as_ref().unwrap_or(object);
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

        if trace_member_assignment {
            eprintln!("member_assignment:canonical_property:start");
        }
        let materialized_property = self.canonical_object_property_expression(property);
        if trace_member_assignment {
            eprintln!(
                "member_assignment:canonical_property:done property={materialized_property:?}"
            );
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
            let value_local = self.allocate_temp_local();
            self.emit_numeric_expression(value)?;
            self.push_local_set(value_local);
            self.emit_scoped_property_store_from_local(object, property_name, value_local, value)?;
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
