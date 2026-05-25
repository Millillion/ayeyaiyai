use super::*;

impl<'a> FunctionCompiler<'a> {
    fn direct_iterator_result_member_value(
        &self,
        result: &Expression,
        property_name: &str,
    ) -> Option<Expression> {
        let property = Expression::String(property_name.to_string());
        let Expression::Object(entries) = result else {
            return None;
        };
        entries.iter().find_map(|entry| match entry {
            ObjectEntry::Data { key, value }
                if self.materialize_static_expression(key) == property =>
            {
                Some(value.clone())
            }
            _ => None,
        })
    }

    pub(in crate::backend::direct_wasm) fn emit_direct_iterator_step_member_read(
        &mut self,
        object: &Expression,
        property: &Expression,
    ) -> DirectResult<bool> {
        let Expression::String(property_name) = property else {
            return Ok(false);
        };
        if property_name != "done" && property_name != "value" {
            return Ok(false);
        }
        let Expression::Call { callee, arguments } = object else {
            return Ok(false);
        };
        if !arguments.is_empty() {
            return Ok(false);
        }
        let Expression::Member {
            object: iterator_object,
            property: next_property,
        } = callee.as_ref()
        else {
            return Ok(false);
        };
        if !matches!(next_property.as_ref(), Expression::String(name) if name == "next") {
            return Ok(false);
        }
        if let Some(predicted_result) =
            self.resolve_fresh_simple_generator_next_result_expression(iterator_object, arguments)
            && let Some(predicted_value) =
                self.direct_iterator_result_member_value(&predicted_result, property_name)
            && self.emit_fresh_simple_generator_next_call(iterator_object, arguments)?
        {
            self.state.emission.output.instructions.push(0x1a);
            let result = self
                .resolve_call_snapshot_result_expression(object)
                .unwrap_or(predicted_result);
            let value = self
                .direct_iterator_result_member_value(&result, property_name)
                .unwrap_or(predicted_value);
            self.emit_numeric_expression(&value)?;
            return Ok(true);
        }
        let hidden_name =
            self.allocate_named_hidden_local("direct_iterator_step", StaticValueKind::Object);
        self.update_local_iterator_step_binding(&hidden_name, object);
        let Some(IteratorStepBinding::Runtime {
            done_local,
            value_local,
            ..
        }) = self
            .state
            .speculation
            .static_semantics
            .local_iterator_step_binding(&hidden_name)
            .cloned()
        else {
            return Ok(false);
        };
        self.emit_numeric_expression(iterator_object)?;
        self.state.emission.output.instructions.push(0x1a);
        match property_name.as_str() {
            "done" => self.push_local_get(done_local),
            "value" => self.push_local_get(value_local),
            _ => unreachable!("filtered above"),
        }
        Ok(true)
    }

    pub(in crate::backend::direct_wasm) fn direct_iterator_binding_source_expression<'b>(
        &self,
        value: &'b Expression,
    ) -> Option<&'b Expression> {
        let iterated = match value {
            Expression::GetIterator(iterated) => iterated.as_ref(),
            Expression::Call { callee, arguments }
                if arguments.is_empty()
                    && matches!(
                        callee.as_ref(),
                        Expression::Member { property, .. }
                            if is_symbol_iterator_expression(property)
                    ) =>
            {
                let Expression::Member { object, .. } = callee.as_ref() else {
                    unreachable!("filtered above");
                };
                object.as_ref()
            }
            _ => return None,
        };
        let next_property = Expression::String("next".to_string());
        let has_next_property = self
            .resolve_object_binding_from_expression(iterated)
            .is_some_and(|object_binding| {
                object_binding_has_property(&object_binding, &next_property)
            });
        if has_next_property {
            return Some(iterated);
        }
        None
    }
}
